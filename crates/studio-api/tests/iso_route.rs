//! `p1.1-xbox360-iso-member-route`: the disc-image route through the real Registry seam —
//! inspect → stage + open → edit (DB field + dictionary re-point) → in-place save (staging only)
//! → plan → apply (range patches on the image) → records → restore → byte-identical image.
//! The image is synthetic (a BIGF root archive carrying the committed NHL Legacy Xbox 360 anchor
//! database and dictionary, packed like the real discs); anchors are read only.

use std::path::{Path, PathBuf};

use studio_api::Registry;
use tdb_xiso::synth::{image, SynthFile};
use tdb_xiso::Layout;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn s(p: &Path) -> String {
    p.to_str().unwrap().to_string()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-isoroute-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const DB: &str = "anchors/xbox360/nhllegacy/databases/nhlng.db";
const DICT: &str = "anchors/xbox360/nhllegacy/databases/nhl_eng_us.db";
const META: &str = "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml";
const DICT_SHA: &str = "f00926ceff27d46f1e5264385468369cd60f91d54d9944dd5c8584bf6d9b8f70";

fn packed_image(dir: &Path) -> PathBuf {
    let db = std::fs::read(repo(DB)).unwrap();
    let dict = std::fs::read(repo(DICT)).unwrap();
    let mut b = tdb_archive_big::ArchiveBuilder::new();
    b.add_entry("fe/loc/nhl_eng_us.db", dict);
    b.add_entry("db/nhlng.db", db);
    b.add_entry("rendering/blocker.rpsgl", vec![7u8; 300]);
    let big = b.build().unwrap();
    let files = [
        SynthFile {
            path: "cacheboot.big",
            bytes: &big,
        },
        SynthFile {
            path: "default.xex",
            bytes: b"XEX2",
        },
    ];
    let p = dir.join("NHL Legacy (synthetic).iso");
    std::fs::write(&p, image(Layout::Xgd3, &files, None)).unwrap();
    p
}

#[test]
fn inspect_open_edit_save_plan_apply_records_restore_through_the_registry() {
    let tmp = Tmp::new("flow");
    let iso = packed_image(&tmp.0);
    let before = std::fs::read(&iso).unwrap();
    let reg = Registry::default();
    // R3: the running-emulator gate uses the deterministic probe seam here; the live process
    // listing is exercised (and its unreadable case refused) in `r3_..._enumeration` below.
    reg.set_process_probe_for_tests(|| Ok(Vec::new()));

    // Inspect: read-only, both files located inside the archive member with their identities.
    let insp = reg.iso_inspect(&s(&iso)).unwrap();
    assert_eq!(insp.layout, "xgd3");
    let n = insp.nhlng.as_ref().expect("database located");
    assert_eq!(n.member_path, "cacheboot.big");
    assert_eq!(n.entry_path.as_deref(), Some("db/nhlng.db"));
    assert_eq!(insp.nhlng_tables, Some(134));
    assert_eq!(insp.dictionary_sha256.as_deref(), Some(DICT_SHA));
    assert!(insp.dictionary_rows.unwrap() > 20_000);
    assert_eq!(std::fs::read(&iso).unwrap(), before, "inspect never writes");

    // Stage + open: a single-file workspace over the staging folder with the dictionary attached.
    let staging = tmp.0.join("staging");
    let (staged, summary) = reg
        .iso_open(&s(&iso), "nhllegacy", "xbox360", &s(&repo(META)), &s(&staging), &s(&repo("defs")))
        .unwrap();
    assert!(!staged.reused);
    assert_eq!(staged.platform, "xbox");
    assert_eq!(staged.files.len(), 2);
    assert!(Path::new(&staged.db_path).starts_with(&staging));
    let id = summary.project_id;
    assert!(!summary.dirty);
    let cat = reg.list_leagues(id).unwrap();
    assert!(cat.dictionary.is_some(), "the staged dictionary is attached");
    let norway = cat.leagues.iter().find(|l| l.code == 7).unwrap();
    assert_eq!(norway.label.as_deref(), Some("Norway"));
    assert_eq!(norway.dictionary_key_evidence.as_deref(), Some("traced"), "Xbox 360 is the witnessed target");

    // Nothing changed yet: the plan writes nothing.
    let plan = reg.iso_plan(id).unwrap();
    assert_eq!(plan.writes, 0);
    assert!(plan.files.iter().all(|f| !f.changed));
    assert_eq!(plan.project_id, id);
    assert_eq!(staged.project_id, Some(id));
    let st = reg.iso_status(id).unwrap();
    assert!(st.in_flight.is_none() && !st.cancellable && st.pending_reviews == 1);

    // Edit: one team abbreviation + the Norway re-point; save in place → staging only.
    reg.set_field(id, "kOtt", 0, "xsnn", studio_domain::FieldValue::Text("ZZZ".into())).unwrap();
    reg.dictionary_rename_set(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
    assert!(reg.summary(id).unwrap().dirty);
    // With unsaved edits the plan still describes the files as saved (unchanged) and says so;
    // applying such a review is refused by the server, not only by the UI (R2 F2).
    let dirty_plan = reg.iso_plan(id).unwrap();
    assert_eq!(dirty_plan.writes, 0);
    assert!(dirty_plan.dirty);
    let err = reg.iso_apply(id, dirty_plan.review_id).unwrap_err().to_string();
    assert!(err.contains("unsaved edits"), "{err}");
    assert_eq!(reg.save_workspace(id).unwrap(), 2);
    assert!(!reg.summary(id).unwrap().dirty);
    assert_eq!(std::fs::read(&iso).unwrap(), before, "a Studio save never touches the image");

    let plan = reg.iso_plan(id).unwrap();
    assert_eq!(plan.writes, 2);
    assert!(!plan.dirty);
    assert_eq!(plan.workspace_revision, reg.summary(id).unwrap().workspace_revision);
    for f in &plan.files {
        assert!(f.changed, "{}", f.staged);
        assert_eq!(f.member_path.as_deref(), Some("cacheboot.big"));
        assert!(f.end.unwrap() > f.start.unwrap());
    }
    let dict_plan = plan.files.iter().find(|f| f.role == "dictionary").unwrap();
    assert_eq!(dict_plan.prior_sha256.as_deref(), Some(DICT_SHA));
    assert_eq!(dict_plan.end.unwrap() - dict_plan.start.unwrap(), 1_759_044);

    // A consumed or superseded review is refused: the plan reviewed is the plan applied.
    let superseded = plan.review_id;
    let plan = reg.iso_plan(id).unwrap();
    assert_ne!(plan.review_id, superseded);
    let err = reg.iso_apply(id, superseded).unwrap_err().to_string();
    assert!(err.contains("not current"), "{err}");
    assert_eq!(std::fs::read(&iso).unwrap(), before);

    // Apply: both ranges written and verified; records exist; the image's other bytes untouched.
    let applied = reg.iso_apply(id, plan.review_id).unwrap();
    assert_eq!(applied.outcome, "committed");
    assert_eq!(applied.records.len(), 2);
    let err = reg.iso_apply(id, plan.review_id).unwrap_err().to_string();
    assert!(err.contains("not current"), "a review applies once: {err}");
    assert!(applied.records.iter().all(|r| r.status == "applied" && r.on_image == "replacement"));
    let after = std::fs::read(&iso).unwrap();
    assert_eq!(after.len(), before.len());
    let differing = after.iter().zip(&before).filter(|(a, b)| a != b).count();
    assert!(differing > 0 && differing < 64, "{differing} bytes differ");
    // A fresh inspection reads the edited files straight out of the image.
    let insp2 = reg.iso_inspect(&s(&iso)).unwrap();
    assert_ne!(insp2.dictionary_sha256.as_deref(), Some(DICT_SHA));
    assert_eq!(insp2.dictionary_rows, insp.dictionary_rows);
    let located = studio_domain::xiso_source::locate_databases(&iso).unwrap();
    let new_dict = studio_domain::xiso_source::read_located(&iso, located.dictionary.as_ref().unwrap()).unwrap();
    assert_eq!(
        tdb_loc::LocDictionary::parse(&new_dict).unwrap().text("TXT_LEAGUE_NORWAY"),
        Some("Rest Of World")
    );
    // The archive still parses and the untouched entry is intact.
    let x = tdb_xiso::Xiso::open(&iso).unwrap();
    let m = x.member("cacheboot.big").unwrap().clone();
    let arc = tdb_archive_big::Archive::parse(x.read_member(&m).unwrap()).unwrap();
    assert_eq!(arc.read_entry("rendering/blocker.rpsgl").unwrap(), vec![7u8; 300]);

    // Records list, then restore: byte-identical image, idempotent.
    let recs = reg.iso_records(id).unwrap();
    assert_eq!(recs.len(), 2);
    assert!(recs.iter().all(|r| r.on_image == "replacement"));
    // A records directory that is not this staging's own is refused before anything is read.
    let err = reg.iso_restore(id, &s(&tmp.0)).unwrap_err().to_string();
    assert!(err.contains("not an apply directory"), "{err}");
    let restored = reg.iso_restore(id, &applied.records_dir).unwrap();
    assert!(restored.all_prior);
    assert!(restored.items.iter().all(|i| i.outcome == "restored"));
    assert_eq!(std::fs::read(&iso).unwrap(), before);
    let again = reg.iso_restore(id, &applied.records_dir).unwrap();
    assert!(again.items.iter().all(|i| i.outcome == "already-prior"));
    let recs = reg.iso_records(id).unwrap();
    assert!(recs.iter().all(|r| r.on_image == "prior" && r.status == "restored"));

    // Re-open reuses the staging (with the saved edits) and refuses another image.
    let (again, s2) = reg
        .iso_open(&s(&iso), "nhllegacy", "xbox360", &s(&repo(META)), &s(&staging), &s(&repo("defs")))
        .unwrap();
    assert!(again.reused);
    assert_ne!(s2.project_id, id);
    // Closing a project forgets its binding: the route no longer serves it.
    assert!(reg.close_project(s2.project_id));
    let err = reg.iso_plan(s2.project_id).unwrap_err().to_string();
    assert!(err.contains("not opened from a disc image"), "{err}");
    let other = tmp.0.join("other.iso");
    std::fs::copy(&iso, &other).unwrap();
    let err = match reg.iso_open(&s(&other), "nhllegacy", "xbox360", &s(&repo(META)), &s(&staging), &s(&repo("defs"))) {
        Ok(_) => panic!("another image must be refused for this staging folder"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("already stages"), "{err}");
}

/// Real image, opt-in and **read-only**: `NHL_STUDIO_XISO=<iso>` (+ `NHL_STUDIO_XISO_GAME`,
/// default `nhl14`). Stages into a temp folder, holds the Norway/Legend re-point, saves the staging
/// copy and plans — never applies. Proves the preflight on real bytes: the plan's prior identity is
/// the image's own and the range is the entry's.
#[test]
#[ignore]
fn real_image_plan_only_never_writes() {
    let iso = std::env::var("NHL_STUDIO_XISO").expect("NHL_STUDIO_XISO must name the image");
    let game = std::env::var("NHL_STUDIO_XISO_GAME").unwrap_or_else(|_| "nhl14".into());
    let key = std::env::var("NHL_STUDIO_XISO_KEY").unwrap_or_else(|_| "TXT_LEGEND_TEAMS".into());
    let donor = std::env::var("NHL_STUDIO_XISO_DONOR").unwrap_or_else(|_| "TXT_ROW_LEAGUE".into());
    let before_len = std::fs::metadata(&iso).unwrap().len();
    let tmp = Tmp::new("real");
    let reg = Registry::default();
    let meta = repo(&format!("anchors/xbox360/{game}/databases/nhlng-meta.xml"));
    let staging = tmp.0.join("staging");
    let (staged, summary) = reg
        .iso_open(&iso, &game, "xbox360", &s(&meta), &s(&staging), &s(&repo("defs")))
        .unwrap();
    eprintln!("{iso}: staged {} files, layout {}", staged.files.len(), staged.layout);
    for f in &staged.files {
        eprintln!("  {}: prior {} — {}", f.role, f.prior_sha256, f.located.describe);
    }
    let id = summary.project_id;
    reg.dictionary_rename_set(id, &key, &donor).unwrap();
    assert_eq!(reg.save_workspace(id).unwrap(), 2);
    let plan = reg.iso_plan(id).unwrap();
    assert_eq!(plan.writes, 1, "only the dictionary changed");
    let d = plan.files.iter().find(|f| f.changed).unwrap();
    eprintln!(
        "  plan: {} [{}, {}) {} -> {}",
        d.entry_path.as_deref().unwrap(),
        d.start.unwrap(),
        d.end.unwrap(),
        d.prior_sha256.as_deref().unwrap(),
        d.replacement_sha256.as_deref().unwrap()
    );
    assert_eq!(std::fs::metadata(&iso).unwrap().len(), before_len);
}

fn open_legacy(reg: &Registry, iso: &Path, staging: &Path) -> u32 {
    reg.iso_open(&s(iso), "nhllegacy", "xbox360", &s(&repo(META)), &s(staging), &s(&repo("defs")))
        .unwrap()
        .1
        .project_id
}

/// R2 F2: the plan reviewed is the plan applied. Staged bytes that change after the review —
/// a save after confirmation, or an external write to the staging folder — refuse the apply
/// before any image byte changes; so does a review taken on another project, and a running
/// emulator for the image's platform.
#[test]
fn r2_f2_a_stale_or_foreign_review_and_a_running_emulator_are_refused_before_mutation() {
    let tmp = Tmp::new("stale");
    let iso = packed_image(&tmp.0);
    let before = std::fs::read(&iso).unwrap();
    let reg = Registry::default();
    reg.set_process_probe_for_tests(|| Ok(vec!["explorer".into(), "code".into()]));
    let staging = tmp.0.join("staging");
    let id = open_legacy(&reg, &iso, &staging);

    // Edit + save, review, then edit again *after* the review.
    reg.set_field(id, "kOtt", 0, "xsnn", studio_domain::FieldValue::Text("AAA".into())).unwrap();
    reg.save_workspace(id).unwrap();
    let review = reg.iso_plan(id).unwrap();
    assert_eq!(review.writes, 1);
    reg.set_field(id, "kOtt", 0, "xsnn", studio_domain::FieldValue::Text("BBB".into())).unwrap();
    // Unsaved edit after the review: refused (dirty).
    let err = reg.iso_apply(id, review.review_id).unwrap_err().to_string();
    assert!(err.contains("unsaved edits"), "{err}");
    assert_eq!(std::fs::read(&iso).unwrap(), before);
    // The review was consumed by the refusal; take a fresh (dirty) one, then save behind it.
    let review = reg.iso_plan(id).unwrap();
    assert!(review.dirty);
    reg.save_workspace(id).unwrap();
    let err = reg.iso_apply(id, review.review_id).unwrap_err().to_string();
    assert!(err.contains("reviewed while the project had unsaved edits"), "{err}");
    assert_eq!(std::fs::read(&iso).unwrap(), before);

    // A clean review, then the staged file changes on disk behind the review.
    let review = reg.iso_plan(id).unwrap();
    assert!(!review.dirty && review.writes == 1);
    let staged_db = staging.join("db").join("nhlng.db");
    let mut bytes = std::fs::read(&staged_db).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    std::fs::write(&staged_db, &bytes).unwrap();
    let err = reg.iso_apply(id, review.review_id).unwrap_err().to_string();
    assert!(err.contains("staged bytes changed after the review"), "{err}");
    assert_eq!(std::fs::read(&iso).unwrap(), before);
    assert!(reg.iso_records(id).unwrap().is_empty(), "no record for a refused apply");
    bytes[last] ^= 1;
    std::fs::write(&staged_db, &bytes).unwrap();

    // A review that belongs to another project (a second staging of the same image).
    let staging2 = tmp.0.join("staging2");
    let id2 = open_legacy(&reg, &iso, &staging2);
    let foreign = reg.iso_plan(id2).unwrap();
    let err = reg.iso_apply(id, foreign.review_id).unwrap_err().to_string();
    assert!(err.contains(&format!("belongs to project {id2}")), "{err}");
    assert_eq!(std::fs::read(&iso).unwrap(), before);

    // The emulator for this platform is running: refused at the boundary, nothing written.
    for emulator in ["xenia", "xenia_canary", "xenia_edge"] {
        let review = reg.iso_plan(id).unwrap();
        reg.set_process_probe_for_tests(move || Ok(vec![emulator.into()]));
        let err = reg.iso_apply(id, review.review_id).unwrap_err().to_string();
        assert!(err.contains(&format!("{emulator}.exe is running")), "{err}");
        assert_eq!(std::fs::read(&iso).unwrap(), before);
    }
    // Another platform's emulator does not block an Xbox image.
    reg.set_process_probe_for_tests(|| Ok(vec!["rpcs3".into(), "shadps4".into()]));
    let review = reg.iso_plan(id).unwrap();
    let applied = reg.iso_apply(id, review.review_id).unwrap();
    assert_eq!(applied.outcome, "committed");
    assert_ne!(std::fs::read(&iso).unwrap(), before);
    // …and a restore is refused the same way while the platform's emulator runs.
    reg.set_process_probe_for_tests(|| Ok(vec!["xenia".into()]));
    let err = reg.iso_restore(id, &applied.records_dir).unwrap_err().to_string();
    assert!(err.contains("xenia.exe is running"), "{err}");
    reg.set_process_probe_for_tests(|| Ok(Vec::new()));
    assert!(reg.iso_restore(id, &applied.records_dir).unwrap().all_prior);
    assert_eq!(std::fs::read(&iso).unwrap(), before);
}

/// R2 F2: while an apply holds the image, every conflicting operation is refused with the
/// holder named — a second staging root/project of the same image (spelled differently), a
/// restore, a save, a new review and a close of the held project, and opening the held image
/// again — and all of them proceed once the apply finished.
#[test]
fn r2_f2_conflicting_operations_are_serialized_by_image_identity_while_an_apply_runs() {
    use std::sync::{mpsc, Arc};
    let tmp = Tmp::new("serial");
    let iso = packed_image(&tmp.0);
    let before = std::fs::read(&iso).unwrap();
    let reg = Arc::new(Registry::default());
    let staging = tmp.0.join("staging");
    let id = open_legacy(&reg, &iso, &staging);
    // A second project on a second staging root — a differently spelled path to the same image.
    let alias = {
        let mut p = iso.parent().unwrap().to_path_buf();
        p.push(".");
        p.push(iso.file_name().unwrap());
        p
    };
    let staging2 = tmp.0.join("staging2");
    let id2 = open_legacy(&reg, &alias, &staging2);
    for pid in [id, id2] {
        reg.set_field(pid, "kOtt", 0, "xsnn", studio_domain::FieldValue::Text("AAA".into())).unwrap();
        reg.save_workspace(pid).unwrap();
    }
    let review1 = reg.iso_plan(id).unwrap();
    let review2 = reg.iso_plan(id2).unwrap();

    // The process probe runs inside the reservation: park the first apply there.
    let (entered_tx, entered_rx) = mpsc::channel::<()>();
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let go_rx = std::sync::Mutex::new(go_rx);
    reg.set_process_probe_for_tests(move || {
        let _ = entered_tx.send(());
        let _ = go_rx.lock().unwrap().recv();
        Ok(Vec::new())
    });
    let worker = {
        let reg = Arc::clone(&reg);
        std::thread::spawn(move || reg.iso_apply(id, review1.review_id))
    };
    entered_rx.recv().unwrap();

    let st = reg.iso_status(id).unwrap();
    let holder = st.in_flight.clone().expect("the apply holds the project");
    assert!(holder.contains("disc-image write of review"), "{holder}");
    // The second project's apply overlaps on the image (canonical identity, not spelling).
    let err = reg.iso_apply(id2, review2.review_id).unwrap_err().to_string();
    assert!(err.contains("overlaps") && err.contains(&holder), "{err}");
    // A save, a new review and a close of the held project are refused with the holder.
    let err = reg.save_workspace(id).unwrap_err().to_string();
    assert!(err.contains(&holder), "{err}");
    let err = reg.iso_plan(id).unwrap_err().to_string();
    assert!(err.contains(&holder), "{err}");
    let closed = reg.close_project_outcome(id);
    assert_eq!(closed.outcome, "busy", "{:?}", closed.reason);
    // Opening a third staging of the held image is refused too (it would alias the held file).
    let err = match reg.iso_open(&s(&iso), "nhllegacy", "xbox360", &s(&repo(META)), &s(&tmp.0.join("staging3")), &s(&repo("defs"))) {
        Ok(_) => panic!("must be refused while held"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains(&holder), "{err}");

    go_tx.send(()).unwrap();
    let applied = worker.join().unwrap().unwrap();
    assert_eq!(applied.outcome, "committed");
    assert!(reg.iso_status(id).unwrap().in_flight.is_none());
    reg.set_process_probe_for_tests(|| Ok(Vec::new()));
    // A restore is refused while the *other* project applies; park an apply of project 2 on
    // the restored image first — its plan re-verifies the range the first apply changed.
    let err = reg.iso_plan(id2).unwrap_err().to_string();
    assert!(err.contains("nothing was written"), "the plan re-verifies the image: {err}");
    let restored = reg.iso_restore(id, &applied.records_dir).unwrap();
    assert!(restored.all_prior);
    assert_eq!(std::fs::read(&iso).unwrap(), before);
    let review2 = reg.iso_plan(id2).unwrap();
    let (entered_tx, entered_rx) = mpsc::channel::<()>();
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let go_rx = std::sync::Mutex::new(go_rx);
    reg.set_process_probe_for_tests(move || {
        let _ = entered_tx.send(());
        let _ = go_rx.lock().unwrap().recv();
        Ok(Vec::new())
    });
    let worker = {
        let reg = Arc::clone(&reg);
        std::thread::spawn(move || reg.iso_apply(id2, review2.review_id))
    };
    entered_rx.recv().unwrap();
    let holder2 = reg.iso_status(id2).unwrap().in_flight.unwrap();
    let err = reg.iso_restore(id, &applied.records_dir).unwrap_err().to_string();
    assert!(err.contains(&holder2), "a restore races the other apply: {err}");
    go_tx.send(()).unwrap();
    let applied2 = worker.join().unwrap().unwrap();
    assert_eq!(applied2.outcome, "committed");
    reg.set_process_probe_for_tests(|| Ok(Vec::new()));
    assert!(reg.iso_restore(id2, &applied2.records_dir).unwrap().all_prior);
    assert_eq!(std::fs::read(&iso).unwrap(), before);
    assert!(reg.close_project(id) && reg.close_project(id2));
}

/// R2 F1 through the registry: a failed write (a fault after the first range was written, an
/// interrupted second range write) returns an outcome — never an error that hides the records —
/// with every attempted range classified, the image byte-identical, and the records listed
/// afterwards by `iso_records` like any committed apply's.
#[test]
fn r2_f1_a_failed_apply_returns_its_records_and_the_image_is_as_it_was() {
    let tmp = Tmp::new("failed");
    let iso = packed_image(&tmp.0);
    let before = std::fs::read(&iso).unwrap();
    let reg = Registry::default();
    reg.set_process_probe_for_tests(|| Ok(Vec::new()));
    let staging = tmp.0.join("staging");
    let id = open_legacy(&reg, &iso, &staging);
    reg.set_field(id, "kOtt", 0, "xsnn", studio_domain::FieldValue::Text("ZZZ".into())).unwrap();
    reg.dictionary_rename_set(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
    reg.save_workspace(id).unwrap();

    // The dictionary (second range) write is cut short after 4096 bytes.
    let review = reg.iso_plan(id).unwrap();
    assert_eq!(review.writes, 2);
    reg.set_iso_fault_for_tests(studio_api::IsoFault::Truncate {
        entry: "fe/loc/nhl_eng_us.db".into(),
        after: 4096,
    });
    let out = reg.iso_apply(id, review.review_id).unwrap();
    assert_eq!(out.outcome, "rolled-back", "{}", out.message);
    assert_eq!(out.failed.as_deref(), Some("fe/loc/nhl_eng_us.db"));
    assert!(out.cause.as_deref().unwrap().contains("interrupted after 4096 bytes"), "{:?}", out.cause);
    assert_eq!(out.attempted.len(), 2, "the failing range and the earlier one");
    assert!(out.attempted.iter().all(|a| a.state == "prior" && a.restore_error.is_none()));
    assert!(out.records.iter().all(|r| r.status == "restored" && r.on_image == "prior"));
    assert!(out.message.contains("the image is as it was"), "{}", out.message);
    assert_eq!(std::fs::read(&iso).unwrap(), before);
    let recs = reg.iso_records(id).unwrap();
    assert_eq!(recs.len(), 2, "records of the failed apply are listed");
    assert!(recs.iter().all(|r| r.records_dir == out.records_dir));

    // A fault at the final record update of the first range: same contract.
    let review = reg.iso_plan(id).unwrap();
    reg.set_iso_fault_for_tests(studio_api::IsoFault::At(studio_domain::iso_patch::Event::BeforeFinalRecord(
        "db/nhlng.db".into(),
    )));
    let out = reg.iso_apply(id, review.review_id).unwrap();
    assert_eq!(out.outcome, "rolled-back", "{}", out.message);
    assert_eq!(out.attempted.len(), 1);
    assert_eq!(std::fs::read(&iso).unwrap(), before);
    assert_eq!(reg.iso_records(id).unwrap().len(), 3);

    // Without a fault the same review shape commits and restores.
    let review = reg.iso_plan(id).unwrap();
    let out = reg.iso_apply(id, review.review_id).unwrap();
    assert_eq!(out.outcome, "committed");
    assert!(reg.iso_restore(id, &out.records_dir).unwrap().all_prior);
    assert_eq!(std::fs::read(&iso).unwrap(), before);
}

/// R3: an unreadable process list fails the gate closed — the apply and the restore are refused
/// before any byte changes, with the reason, and a readable list lets them through.
#[test]
fn r3_an_unreadable_process_enumeration_refuses_writes_and_restores() {
    let tmp = Tmp::new("enum");
    let iso = packed_image(&tmp.0);
    let before = std::fs::read(&iso).unwrap();
    let reg = Registry::default();
    let staging = tmp.0.join("staging");
    let id = open_legacy(&reg, &iso, &staging);
    reg.set_field(id, "kOtt", 0, "xsnn", studio_domain::FieldValue::Text("AAA".into())).unwrap();
    reg.save_workspace(id).unwrap();
    reg.set_process_probe_for_tests(|| Err("the process list could not be read (simulated exit 1)".into()));
    let review = reg.iso_plan(id).unwrap();
    let err = reg.iso_apply(id, review.review_id).unwrap_err().to_string();
    assert!(err.contains("process list could not be read"), "{err}");
    assert_eq!(std::fs::read(&iso).unwrap(), before);
    assert!(reg.iso_records(id).unwrap().is_empty());
    assert!(reg.iso_status(id).unwrap().in_flight.is_none(), "the reservation is released");
    reg.set_process_probe_for_tests(|| Ok(Vec::new()));
    let review = reg.iso_plan(id).unwrap();
    let applied = reg.iso_apply(id, review.review_id).unwrap();
    assert_eq!(applied.outcome, "committed");
    reg.set_process_probe_for_tests(|| Err("unreadable again".into()));
    let err = reg.iso_restore(id, &applied.records_dir).unwrap_err().to_string();
    assert!(err.contains("unreadable again"), "{err}");
    assert_ne!(std::fs::read(&iso).unwrap(), before, "nothing restored behind a closed gate");
    reg.set_process_probe_for_tests(|| Ok(Vec::new()));
    assert!(reg.iso_restore(id, &applied.records_dir).unwrap().all_prior);
    assert_eq!(std::fs::read(&iso).unwrap(), before);
}
