//! S2-C: the copied-target Workspace/API integration through the [`Registry`] seam (no Tauri).
//! A real workspace (the nhl14 PS3 `nhlng.db` + aligned ROSTER pair on temp copies, in the
//! header-indexed content layout) is reviewed, applied, reopened and restored through the
//! server-owned handle contract; every refusal keeps the user's edits; a fresh process discovers an
//! interrupted transaction through the same recovery API.
//!
//! The content layout is a fixture shape for the header planner; nothing here is evidence that a
//! game accepts the result (every artifact is unqualified and the DTOs say so).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier};

use studio_api::{parse_platform, Registry};
use studio_domain::tdb_txn::{self, Event, FileIdentity, Hooks};
use studio_domain::FieldValue;

const NHLNG: &str = "anchors/ps3/nhl14/databases/nhlng.db";
const META: &str = "anchors/ps3/nhl14/databases/nhlng-meta.xml";
const DEFS: &str = "defs";
const ROSTER: &str = "anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA";
const PROFILE: &str = "B13EBABEBABEBABE";
const TITLE: &str = "454109EC";
const SAVE_NAME: &str = "ROSTER 20260723000649";

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The error text of a refused call (panics on success).
trait ErrText {
    fn err_text(self) -> String;
}
impl<T> ErrText for Result<T, studio_api::ApiError> {
    fn err_text(self) -> String {
        match self {
            Ok(_) => panic!("expected a refusal"),
            Err(e) => e.to_string(),
        }
    }
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-s2c-api-{tag}-{}", std::process::id()));
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

struct Tree {
    root: PathBuf,
    nhlng: PathBuf,
    roster: PathBuf,
    headers: PathBuf,
}

impl Tree {
    fn header(&self) -> PathBuf {
        self.headers.join(format!("{SAVE_NAME}.header"))
    }
}

fn copied_tree(base: &Path) -> Tree {
    let root = base.join("copy");
    tdb_txn::engine::create_copied_target_root(&root).unwrap();
    std::fs::create_dir_all(root.join("USRDIR")).unwrap();
    let nhlng = root.join("USRDIR/nhlng.db");
    std::fs::copy(repo(NHLNG), &nhlng).unwrap();
    let save_dir = root
        .join(PROFILE)
        .join(TITLE)
        .join("00000001")
        .join(SAVE_NAME);
    std::fs::create_dir_all(&save_dir).unwrap();
    let roster = save_dir.join(SAVE_NAME);
    std::fs::copy(repo(ROSTER), &roster).unwrap();
    let headers = root.join(PROFILE).join(TITLE).join("Headers/00000001");
    std::fs::create_dir_all(&headers).unwrap();
    Tree {
        root,
        nhlng,
        roster,
        headers,
    }
}

fn open(reg: &Registry, t: &Tree) -> u32 {
    let summary = reg
        .open_workspace_from_paths(
            Some("nhl14"),
            parse_platform("ps3").unwrap(),
            &s(&t.nhlng),
            Some(&s(&t.roster)),
            &s(&repo(META)),
            &s(&repo(DEFS)),
        )
        .unwrap();
    assert!(summary.roster_attached && summary.roster_aligned);
    summary.project_id
}

fn edit(reg: &Registry, id: u32, value: i64) {
    reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(value))
        .unwrap();
    assert!(reg.summary(id).unwrap().dirty);
}

fn dirty(reg: &Registry, id: u32) -> bool {
    reg.summary(id).unwrap().dirty
}

fn player_position(reg: &Registry, id: u32) -> String {
    let players = reg.list_players(id).unwrap();
    let view = reg.player_view(id, players[0].player_id).unwrap().unwrap();
    view.identity
        .iter()
        .find(|f| f.name == "position")
        .map(|f| f.display.clone())
        .unwrap_or_default()
}

#[test]
fn two_saves_reopen_and_ordered_restore_through_the_api() {
    let tmp = Tmp::new("cycle");
    let t = copied_tree(&tmp.0);
    let orig_nhlng = FileIdentity::of_file(&t.nhlng).unwrap();
    let orig_roster = FileIdentity::of_file(&t.roster).unwrap();
    let reg = Registry::default();
    let id = open(&reg, &t);
    let root = s(&t.root);

    // A clean root.
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.apply_allowed && !st.blocked && st.transactions.is_empty());
    assert!(st.guidance.contains("accepts"));

    // ---- cycle 1
    edit(&reg, id, 2);
    let pos_after_edit = player_position(&reg, id);
    let review = reg.copied_target_review(id, &root).unwrap();
    assert_eq!(review.project_id, id);
    assert_eq!(review.mode, "copied-target");
    assert_eq!(review.qualification_state, "unqualified");
    assert!(!review.live_install_allowed);
    let labels: Vec<(&str, &str, &str)> = review
        .artifacts
        .iter()
        .map(|a| (a.key.as_str(), a.kind.as_str(), a.label.as_str()))
        .collect();
    assert_eq!(
        labels,
        vec![
            ("primary", "database", "Main database (nhlng.db)"),
            ("roster", "roster-save", "Roster save (ROSTER 20260723000649)"),
            (
                "roster.header",
                "content-header",
                "Save index header for ROSTER 20260723000649 (B13EBABEBABEBABE/454109EC/Headers/00000001/ROSTER 20260723000649.header)"
            ),
        ]
    );
    let hdr = &review.artifacts[2];
    assert!(hdr.prior.is_none(), "planned as a new file");
    assert_eq!(hdr.dependencies, vec!["roster".to_owned()]);
    assert_eq!(hdr.qualification_items[0].status, "unresolved");
    assert_eq!(
        review.artifacts[0].prior.as_ref().unwrap().sha256,
        orig_nhlng.sha256
    );
    assert!(!review.artifacts[0].destination_path.starts_with(r"\\?\"));
    assert!(review.bound_inputs.is_empty());
    assert_eq!(review.status.pending_handles.len(), 1);
    assert!(
        !review.status.apply_allowed,
        "the prepared set holds the root"
    );
    assert!(review
        .status
        .guidance
        .contains(&format!("review handle {}", review.handle)));
    assert!(dirty(&reg, id), "review changes nothing in the session");

    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_eq!(applied.txn_id, review.txn_id);
    assert!(applied.applied_unqualified);
    assert_eq!(applied.project.primary, "rebound");
    assert_eq!(applied.project.roster.as_deref(), Some("rebound"));
    assert!(!applied.project.dirty);
    assert!(applied.log.iter().any(|l| l == "AfterCommitRecord"));
    assert!(applied.status.apply_allowed);
    assert_eq!(applied.destinations.len(), 3);
    let installed_nhlng = FileIdentity::of_file(&t.nhlng).unwrap();
    assert_eq!(
        applied
            .destinations
            .iter()
            .find(|d| d.key == "primary")
            .unwrap()
            .identity
            .as_ref()
            .unwrap()
            .sha256,
        installed_nhlng.sha256
    );
    assert_eq!(std::fs::read(t.header()).unwrap().len(), 0x148);
    let header_bytes = std::fs::read(t.header()).unwrap();
    assert!(!dirty(&reg, id), "the installed file is the baseline now");
    assert_eq!(player_position(&reg, id), pos_after_edit);

    // Reopen through a second registry: the edit is what loads.
    let reg2 = Registry::default();
    let id2 = open(&reg2, &t);
    assert_eq!(player_position(&reg2, id2), pos_after_edit);
    drop(reg2);

    // ---- cycle 2: no header needed (preserved byte-for-byte); the prior is the installed file.
    edit(&reg, id, 3);
    let review2 = reg.copied_target_review(id, &root).unwrap();
    assert_eq!(review2.artifacts.len(), 2);
    assert_eq!(
        review2.artifacts[0].prior.as_ref().unwrap().sha256,
        installed_nhlng.sha256
    );
    let applied2 = reg.copied_target_apply(id, review2.handle).unwrap();
    assert_eq!(applied2.state, "committed");
    assert_eq!(std::fs::read(t.header()).unwrap(), header_bytes);
    assert!(!dirty(&reg, id));
    let st = reg.copied_target_status(&root).unwrap();
    assert_eq!(st.transactions.len(), 2);
    assert!(st.transactions.iter().all(|x| x.state == "committed"));
    assert!(!st.transactions[0].restorable, "superseded by the second");
    assert_eq!(st.transactions[0].superseded[0].by_txn_id, applied2.txn_id);
    assert!(st.transactions[1].restorable);

    // ---- ordered restore.
    let e = reg.copied_target_restore(&root, &applied.txn_id).err_text();
    assert!(e.contains("restore that first"), "{e}");
    let r2 = reg.copied_target_restore(&root, &applied2.txn_id).unwrap();
    assert_eq!(r2.state, "restored");
    assert_eq!(r2.affected.len(), 1);
    assert_eq!(r2.affected[0].primary, "reloaded");
    assert_eq!(r2.affected[0].roster.as_deref(), Some("reloaded"));
    assert!(!r2.affected[0].dirty);
    assert_eq!(
        player_position(&reg, id),
        pos_after_edit,
        "cycle 1's bytes are shown"
    );
    assert_eq!(FileIdentity::of_file(&t.nhlng).unwrap(), installed_nhlng);
    let r1 = reg.copied_target_restore(&root, &applied.txn_id).unwrap();
    assert_eq!(r1.state, "restored");
    assert_eq!(FileIdentity::of_file(&t.nhlng).unwrap(), orig_nhlng);
    assert_eq!(FileIdentity::of_file(&t.roster).unwrap(), orig_roster);
    assert!(!t.header().exists());
    assert_eq!(
        std::fs::read(&t.nhlng).unwrap(),
        std::fs::read(repo(NHLNG)).unwrap()
    );
    assert!(!dirty(&reg, id));
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.transactions.iter().all(|x| x.state == "restored"));
    assert!(st.apply_allowed);
}

/// Sets `flag` (and optionally cancels through the registry) when `at` fires.
struct At<'a> {
    at: String,
    action: Box<dyn FnMut() + 'a>,
    done: bool,
}
impl Hooks for At<'_> {
    fn event(&mut self, e: &Event) -> std::io::Result<()> {
        if !self.done && format!("{e:?}") == self.at {
            self.done = true;
            (self.action)();
        }
        Ok(())
    }
}

#[test]
fn every_refusal_keeps_the_edits_and_frees_the_root() {
    let tmp = Tmp::new("refuse");
    let t = copied_tree(&tmp.0);
    let reg = Registry::default();
    let id = open(&reg, &t);
    let root = s(&t.root);
    let orig = std::fs::read(&t.nhlng).unwrap();
    let assert_untouched = |msg: &str| {
        assert!(dirty(&reg, id), "edits kept: {msg}");
        assert_eq!(
            std::fs::read(&t.nhlng).unwrap(),
            orig,
            "nothing written: {msg}"
        );
        assert!(!t.header().exists(), "no header written: {msg}");
        let st = reg.copied_target_status(&root).unwrap();
        assert!(st.apply_allowed, "root free: {msg} — {}", st.guidance);
        assert!(st.pending_handles.is_empty(), "no handle left: {msg}");
    };

    // Session edit after review → refused at apply, set aborted, handle consumed.
    // (Record 0 is a defenceman in the anchor; the edits below stay within skater codes.)
    edit(&reg, id, 2);
    let r = reg.copied_target_review(id, &root).unwrap();
    edit(&reg, id, 1);
    let e = reg.copied_target_apply(id, r.handle).err_text();
    assert!(e.contains("edited after the review"), "{e}");
    assert_untouched("edit after review");
    let e = reg.copied_target_apply(id, r.handle).err_text();
    assert!(e.contains("unknown, already applied"), "{e}");

    // Undo back to the reviewed bytes is the same revision: a fresh review applies fine later;
    // here we only show the revision is content-based.
    let r = reg.copied_target_review(id, &root).unwrap();
    edit(&reg, id, 0);
    assert!(reg.undo(id).unwrap());
    let rev_now = reg.copied_target_review(id, &root).unwrap();
    assert_eq!(rev_now.workspace_revision, r.workspace_revision);
    reg.copied_target_discard(rev_now.handle).unwrap();
    assert_untouched("discard");

    // External destination change → refused before mutation.
    let r = reg.copied_target_review(id, &root).unwrap();
    let mut bytes = orig.clone();
    bytes[100] ^= 1;
    std::fs::write(&t.nhlng, &bytes).unwrap();
    let e = reg.copied_target_apply(id, r.handle).err_text();
    assert!(
        e.contains("changed on disk after the session opened"),
        "{e}"
    );
    std::fs::write(&t.nhlng, &orig).unwrap();
    assert_untouched("external db change");

    // A header appearing after review → refused; the foreign header is left alone.
    let r = reg.copied_target_review(id, &root).unwrap();
    std::fs::write(t.header(), vec![0xEE; 0x148]).unwrap();
    let e = reg.copied_target_apply(id, r.handle).err_text();
    assert!(e.contains("appeared on disk"), "{e}");
    assert_eq!(std::fs::read(t.header()).unwrap(), vec![0xEE; 0x148]);
    std::fs::remove_file(t.header()).unwrap();
    assert_untouched("header appeared");

    // Missing required parent (the Headers index vanished) → reported by name, no mutation.
    let r = reg.copied_target_review(id, &root).unwrap();
    std::fs::remove_dir_all(&t.headers).unwrap();
    let e = reg.copied_target_apply(id, r.handle).err_text();
    assert!(e.contains("does not exist in the copied target"), "{e}");
    std::fs::create_dir_all(&t.headers).unwrap();
    assert_untouched("missing parent");

    // Missing staged output → the engine aborts at stage; nothing mutated.
    let r = reg.copied_target_review(id, &root).unwrap();
    let staged: PathBuf = {
        // The staged file lives in the process temp staging dir; find it through the engine's
        // persisted change set.
        let cs_path = t
            .root
            .join(".tdb-txn")
            .join(&r.txn_id)
            .join("changeset.json");
        let cs: serde_json::Value =
            serde_json::from_slice(&std::fs::read(cs_path).unwrap()).unwrap();
        PathBuf::from(cs["artifacts"][0]["replacement"].as_str().unwrap())
    };
    std::fs::remove_file(&staged).unwrap();
    let e = reg.copied_target_apply(id, r.handle).err_text();
    assert!(e.contains("missing") || e.contains("staged"), "{e}");
    assert_untouched("missing staged output");

    // Handle from another project → refused, handle kept for its owner.
    let tmp_b = Tmp::new("other");
    let t_b = copied_tree(&tmp_b.0);
    let id_b = open(&reg, &t_b);
    let r = reg.copied_target_review(id, &root).unwrap();
    let e = reg.copied_target_apply(id_b, r.handle).err_text();
    assert!(e.contains(&format!("belongs to project {id}")), "{e}");
    assert_eq!(
        reg.copied_target_status(&root)
            .unwrap()
            .pending_handles
            .len(),
        1
    );
    // Re-review supersedes the pending handle (its set is aborted).
    let r2 = reg.copied_target_review(id, &root).unwrap();
    assert_ne!(r2.handle, r.handle);
    let e = reg.copied_target_apply(id, r.handle).err_text();
    assert!(e.contains("superseded") || e.contains("unknown"), "{e}");
    reg.copied_target_discard(r2.handle).unwrap();
    assert_untouched("cross-project");

    // Cancellation: requested through the registry while the apply runs → Aborted, edits kept.
    let r = reg.copied_target_review(id, &root).unwrap();
    let handle = r.handle;
    let mut hook = At {
        at: "AfterStageReplacement(\"primary\")".into(),
        action: Box::new(|| {
            assert!(reg.copied_target_cancel(handle).unwrap());
        }),
        done: false,
    };
    let e = reg
        .copied_target_apply_with_hooks(id, handle, Some(&mut hook))
        .err_text();
    assert!(e.to_lowercase().contains("cancel"), "{e}");
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st
        .transactions
        .iter()
        .any(|x| x.txn_id == r.txn_id && x.state == "aborted"));
    assert_untouched("cancel");

    // Cancellation after mutation began → rolled back to the pre-images.
    let r = reg.copied_target_review(id, &root).unwrap();
    let handle = r.handle;
    let mut hook = At {
        at: "AfterReplaced(\"primary\")".into(),
        action: Box::new(|| {
            assert!(reg.copied_target_cancel(handle).unwrap());
        }),
        done: false,
    };
    let e = reg
        .copied_target_apply_with_hooks(id, handle, Some(&mut hook))
        .err_text();
    assert!(e.contains("RolledBack"), "{e}");
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st
        .transactions
        .iter()
        .any(|x| x.txn_id == r.txn_id && x.state == "rolled-back"));
    assert_untouched("cancel after mutation");

    // Concurrent request: while an apply is inside the engine, another thread's edit, save,
    // review and apply of the same handle are refused; the apply then commits.
    let r = reg.copied_target_review(id, &root).unwrap();
    let handle = r.handle;
    let gate = Arc::new(Barrier::new(2));
    let (tx, rx) = std::sync::mpsc::channel::<Vec<String>>();
    std::thread::scope(|sc| {
        let g = gate.clone();
        let reg_ref = &reg;
        let root = &root;
        sc.spawn(move || {
            g.wait(); // the apply is inside the engine now
            let errs = vec![
                reg_ref
                    .set_field(id, "ubPc", 0, "vjla", FieldValue::Int(0))
                    .err_text(),
                reg_ref.save_workspace(id).err_text(),
                reg_ref.copied_target_review(id, root).err_text(),
                reg_ref.copied_target_apply(id, handle).err_text(),
                reg_ref.copied_target_recover(root).err_text(),
            ];
            let st = reg_ref.copied_target_status(root).unwrap();
            assert_eq!(st.busy_projects, vec![id]);
            assert!(st
                .pending_handles
                .iter()
                .any(|h| h.handle == handle && h.in_progress));
            tx.send(errs).unwrap();
            g.wait(); // let the apply continue
        });
        let g = gate.clone();
        let mut hook = At {
            at: "BeforeBackup(\"primary\")".into(),
            action: Box::new(move || {
                g.wait();
                g.wait();
            }),
            done: false,
        };
        let applied = reg
            .copied_target_apply_with_hooks(id, handle, Some(&mut hook))
            .unwrap();
        assert_eq!(applied.state, "committed");
    });
    let errs = rx.recv().unwrap();
    assert!(
        errs[0].contains("locked by a copied-target operation"),
        "{}",
        errs[0]
    );
    assert!(
        errs[1].contains("locked by a copied-target operation"),
        "{}",
        errs[1]
    );
    assert!(errs[2].contains("apply in progress"), "{}", errs[2]);
    assert!(errs[3].contains("unknown, already applied"), "{}", errs[3]);
    assert!(errs[4].contains("apply is running"), "{}", errs[4]);
    assert!(!dirty(&reg, id));
    assert_ne!(std::fs::read(&t.nhlng).unwrap(), orig);

    // Stale baseline: a further edit, then a restore underneath it → edits kept, saves refuse,
    // an explicit reload (with consent) clears it.
    let st = reg.copied_target_status(&root).unwrap();
    let committed = st
        .transactions
        .iter()
        .find(|x| x.state == "committed")
        .unwrap();
    edit(&reg, id, 2);
    let restored = reg.copied_target_restore(&root, &committed.txn_id).unwrap();
    assert_eq!(restored.affected[0].primary, "stale-kept-edits");
    assert!(restored.affected[0].dirty);
    assert!(restored.affected[0].stale_baseline.is_some());
    let e = reg.copied_target_review(id, &root).err_text();
    assert!(e.contains("edits were kept"), "{e}");
    let e = reg.copied_target_reload(id, false).err_text();
    assert!(e.contains("unsaved edits"), "{e}");
    assert!(dirty(&reg, id));
    let b = reg.copied_target_reload(id, true).unwrap();
    assert_eq!(b.primary, "reloaded");
    assert!(!b.dirty);
    assert_eq!(std::fs::read(&t.nhlng).unwrap(), orig);
    // And the root accepts a new save again.
    edit(&reg, id, 2);
    let r = reg.copied_target_review(id, &root).unwrap();
    reg.copied_target_apply(id, r.handle).unwrap();
    assert!(!dirty(&reg, id));
}

// ---------------------------------------------------------------- fresh-process interruption

/// The child entry point: review + apply, exiting the process at the named event. Only acts when
/// `S2C_CHILD` is set (the parent re-invokes this test binary).
#[test]
fn child() {
    let Ok(at) = std::env::var("S2C_CHILD") else {
        return;
    };
    let base = PathBuf::from(std::env::var("S2C_BASE").unwrap());
    let t = Tree {
        root: base.join("copy"),
        nhlng: base.join("copy/USRDIR/nhlng.db"),
        roster: base
            .join("copy")
            .join(PROFILE)
            .join(TITLE)
            .join("00000001")
            .join(SAVE_NAME)
            .join(SAVE_NAME),
        headers: base
            .join("copy")
            .join(PROFILE)
            .join(TITLE)
            .join("Headers/00000001"),
    };
    let reg = Registry::default();
    let id = open(&reg, &t);
    edit(&reg, id, 2);
    let r = reg.copied_target_review(id, &s(&t.root)).unwrap();
    struct ExitAt(String);
    impl Hooks for ExitAt {
        fn event(&mut self, e: &Event) -> std::io::Result<()> {
            if format!("{e:?}") == self.0 {
                std::process::exit(9);
            }
            Ok(())
        }
    }
    let _ = reg.copied_target_apply_with_hooks(id, r.handle, Some(&mut ExitAt(at)));
    panic!("the child must have exited inside the engine");
}

#[test]
fn a_restarted_registry_discovers_and_recovers_an_interrupted_apply() {
    let tmp = Tmp::new("restart");
    let t = copied_tree(&tmp.0);
    let orig = std::fs::read(&t.nhlng).unwrap();
    let root = s(&t.root);

    // The child dies after the first destination was replaced and journaled.
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("child")
        .arg("--exact")
        .arg("--nocapture")
        .env("S2C_CHILD", "AfterReplaced(\"primary\")")
        .env("S2C_BASE", &tmp.0)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(9), "the child exited at the hook");
    assert_ne!(
        std::fs::read(&t.nhlng).unwrap(),
        orig,
        "the first replacement landed"
    );

    // A fresh registry (no handle) sees the interrupted set through the same API.
    let reg = Registry::default();
    let st = reg.copied_target_status(&root).unwrap();
    assert_eq!(st.transactions.len(), 1);
    assert_eq!(st.transactions[0].state, "applying");
    assert!(st.blocked && !st.apply_allowed);
    assert!(
        st.guidance.contains("run recovery inspection"),
        "{}",
        st.guidance
    );
    assert!(st.pending_handles.is_empty());
    let id = open(&reg, &t);
    edit(&reg, id, 3);
    let e = reg.copied_target_review(id, &root).err_text();
    assert!(e.contains("cannot take a new save"), "{e}");
    assert!(dirty(&reg, id));

    // Recovery rolls the set back to its pre-images and refreshes the workspace: the session was
    // opened on the half-applied file and is dirty, so it keeps its edits and is marked stale.
    let rec = reg.copied_target_recover(&root).unwrap();
    assert_eq!(rec.status.transactions[0].state, "rolled-back");
    assert!(rec.status.apply_allowed);
    assert_eq!(std::fs::read(&t.nhlng).unwrap(), orig);
    // Forced termination is the one case no teardown covers: the child's staging directory is
    // still there (S2-C-R2 documents it as unreferenced scratch once the set is resolved). Its
    // exact path is what the engine's persisted change set names as the replacement source; it
    // is removed here by that path — a test-owned cleanup, not an orphan sweep.
    let cs_path =
        Path::new(&rec.status.transactions[0].recovery_dir).join(tdb_txn::journal::CHANGESET_FILE);
    let cs: tdb_txn::ChangeSet = serde_json::from_slice(&std::fs::read(&cs_path).unwrap()).unwrap();
    let child_staging = cs.artifacts[0].replacement.parent().unwrap().to_path_buf();
    assert!(
        child_staging.is_dir(),
        "the killed child left its staging: {}",
        child_staging.display()
    );
    assert!(child_staging
        .parent()
        .unwrap()
        .ends_with("nhl-modding-studio/copied-target-staging"));
    std::fs::remove_dir_all(&child_staging).unwrap();
    assert_eq!(rec.affected.len(), 1);
    assert_eq!(rec.affected[0].primary, "stale-kept-edits");
    assert!(dirty(&reg, id));
    reg.copied_target_reload(id, true).unwrap();
    edit(&reg, id, 1);
    let r = reg.copied_target_review(id, &root).unwrap();
    let applied = reg.copied_target_apply(id, r.handle).unwrap();
    assert_eq!(applied.state, "committed");
    // Recovery of a committed set is a no-op; a foreign edit afterwards blocks the root.
    let rec = reg.copied_target_recover(&root).unwrap();
    assert_eq!(rec.status.transactions[1].state, "committed");
    assert_eq!(rec.affected[0].primary, "unchanged");
    let mut foreign = std::fs::read(&t.nhlng).unwrap();
    foreign[200] ^= 1;
    std::fs::write(&t.nhlng, &foreign).unwrap();
    let rec = reg.copied_target_recover(&root).unwrap();
    assert_eq!(rec.status.transactions[1].state, "blocked");
    assert!(rec.status.blocked);
    assert!(
        rec.status.guidance.contains("blocked"),
        "{}",
        rec.status.guidance
    );
    assert_eq!(
        std::fs::read(&t.nhlng).unwrap(),
        foreign,
        "foreign bytes retained"
    );
    edit(&reg, id, 0);
    let e = reg.copied_target_review(id, &root).err_text();
    assert!(e.contains("blocked"), "{e}");
    assert!(t
        .root
        .join(".tdb-txn")
        .join(&applied.txn_id)
        .join("backups")
        .is_dir());
}

// ---------------------------------------------------------------- archives share the service

/// A synthetic EB archive in the copied tree, edited through the controlled S2-A adapter, joins the
/// workspace's change set through the same review handle: one prepare, one apply, one journal,
/// one restore. No archive-specific command or writer exists.
#[test]
fn a_staged_archive_and_the_workspace_share_one_prepared_service() {
    use studio_domain::txn::stage_archive;
    use tdb_archive_big::stage::{
        plan, synth::build_archive, BoundSource, ExpectedDecoded, Replacement,
    };
    use tdb_archive_codec::compress_chunkzip;

    let tmp = Tmp::new("archive");
    let t = copied_tree(&tmp.0);
    let reg = Registry::default();
    let id = open(&reg, &t);
    let root = s(&t.root);

    let plain0: Vec<u8> = (0..3000u32).map(|i| (i * 7 % 251) as u8).collect();
    let archive = build_archive(
        &["rendering/blocker", "fe/loc"],
        &[
            (0, "blocker_0_82_0_dm.rpsgl", compress_chunkzip(&plain0)),
            (1, "nhl_eng_us-meta.xml", b"<meta/>original".to_vec()),
        ],
    );
    let archive_dest = t.root.join("USRDIR/patch.big");
    std::fs::write(&archive_dest, &archive).unwrap();
    let orig_archive = FileIdentity::of_file(&archive_dest).unwrap();
    let bound = BoundSource::bind(&mut std::fs::File::open(&archive_dest).unwrap()).unwrap();
    let small = b"<meta/>".to_vec();
    let big: Vec<u8> = (0..20_000u32).map(|i| (i * 13 % 241) as u8).collect();
    let p = plan(
        &bound,
        vec![
            Replacement {
                entry_index: 1,
                expected_path: bound.entries[1].path.clone(),
                expected_stored_sha256: bound.entries[1].stored_sha256,
                new_stored: small.clone(),
                expected_decoded: ExpectedDecoded::of(&small),
            },
            Replacement {
                entry_index: 0,
                expected_path: bound.entries[0].path.clone(),
                expected_stored_sha256: bound.entries[0].stored_sha256,
                new_stored: compress_chunkzip(&big),
                expected_decoded: ExpectedDecoded::of(&big),
            },
        ],
    )
    .unwrap();
    let staging = tmp.0.join("archive-staging");
    std::fs::create_dir_all(&staging).unwrap();
    let staged = stage_archive(bound, p, &archive_dest, &staging.join("patch.big.staged")).unwrap();
    let new_eof = staged.report().new_eof;

    edit(&reg, id, 2);
    let review = reg
        .copied_target_review_with(id, &root, |plan, _| {
            plan.add_archive("patch", &archive_dest, &staged)
                .map_err(studio_api::ApiError::Domain)
        })
        .unwrap();
    let kinds: Vec<(&str, &str)> = review
        .artifacts
        .iter()
        .map(|a| (a.key.as_str(), a.kind.as_str()))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ("primary", "database"),
            ("roster", "roster-save"),
            ("roster.header", "content-header"),
            ("patch", "archive"),
        ]
    );
    let arch = &review.artifacts[3];
    assert_eq!(arch.label, "Archive (patch.big)");
    assert_eq!(arch.producer, "tdb-archive-big::stage");
    assert_eq!(arch.prior.as_ref().unwrap().sha256, orig_archive.sha256);
    assert_eq!(arch.qualification_state, "unqualified");
    assert!(arch
        .qualification_items
        .iter()
        .any(|i| i.subject == "native loader acceptance" && i.status == "unresolved"));

    // An archive outside the root is refused at the same seam.
    let outside = tmp.0.join("outside.big");
    std::fs::write(&outside, &archive).unwrap();
    let e = reg
        .copied_target_review_with(id, &root, |plan, _| {
            plan.add_archive("patch", &outside, &staged)
                .map_err(studio_api::ApiError::Domain)
        })
        .err_text();
    assert!(
        e.contains("the archive") && e.contains("not inside the selected copied-target root"),
        "{e}"
    );
    // (that review superseded the first handle and then failed: the root is free again)
    assert!(reg.copied_target_status(&root).unwrap().apply_allowed);

    let review = reg
        .copied_target_review_with(id, &root, |plan, _| {
            plan.add_archive("patch", &archive_dest, &staged)
                .map_err(studio_api::ApiError::Domain)
        })
        .unwrap();
    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_eq!(applied.destinations.len(), 4);
    assert!(!dirty(&reg, id));
    let reopened = BoundSource::bind(&mut std::fs::File::open(&archive_dest).unwrap()).unwrap();
    assert_eq!(reopened.identity.len, new_eof);
    assert_eq!(reopened.entries[1].size as usize, small.len());
    assert!(
        reopened.entries[0].offset >= orig_archive.len,
        "grown entry appended"
    );

    let restored = reg.copied_target_restore(&root, &applied.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive_dest).unwrap(), archive);
    assert_eq!(
        std::fs::read(&t.nhlng).unwrap(),
        std::fs::read(repo(NHLNG)).unwrap()
    );
    assert!(!t.header().exists());
    assert_eq!(restored.affected[0].primary, "reloaded");
}

// ---------------------------------------------------------------- S2-C-R1 regressions

fn open_single(reg: &Registry, db: &Path) -> u32 {
    reg.load_project(
        &s(db),
        parse_platform("ps3").unwrap(),
        studio_api::parse_role("nhlng").unwrap(),
        &s(&repo(META)),
        &s(&repo(DEFS)),
    )
    .unwrap()
    .project_id
}

/// R1: an existing save index header is the reason no header is planned, so it is a reviewed,
/// identity-bound dependency: gone, changed, or not a regular file → refused before mutation and
/// never regenerated; unchanged → the save commits and the header is preserved byte-for-byte.
#[test]
fn r1_existing_headers_are_bound_dependencies() {
    let tmp = Tmp::new("r1-header");
    let t = copied_tree(&tmp.0);
    let header_v1 = vec![0xA5u8; 0x148];
    std::fs::write(t.header(), &header_v1).unwrap();
    let reg = Registry::default();
    let id = open(&reg, &t);
    let root = s(&t.root);

    // Unchanged header: bound, listed, preserved.
    edit(&reg, id, 2);
    let r = reg.copied_target_review(id, &root).unwrap();
    assert_eq!(
        r.artifacts.len(),
        2,
        "no header artifact: the header exists"
    );
    assert_eq!(r.bound_inputs.len(), 1);
    assert_eq!(r.bound_inputs[0].kind, "content-header");
    assert!(r.bound_inputs[0].path.ends_with(".header"));
    assert_eq!(
        r.bound_inputs[0].identity.sha256,
        FileIdentity::of_bytes(&header_v1).sha256
    );
    assert!(r.bound_inputs[0].purpose.contains("preserved as-is"));
    let applied = reg.copied_target_apply(id, r.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_eq!(std::fs::read(t.header()).unwrap(), header_v1);
    assert!(!dirty(&reg, id));

    // Deleted after review: refused before any mutation, not regenerated, edits kept.
    edit(&reg, id, 1);
    let before = std::fs::read(&t.nhlng).unwrap();
    let r = reg.copied_target_review(id, &root).unwrap();
    std::fs::remove_file(t.header()).unwrap();
    let e = reg.copied_target_apply(id, r.handle).err_text();
    assert!(e.contains("content-header") && e.contains("is gone"), "{e}");
    assert!(!t.header().exists(), "never regenerated silently");
    assert_eq!(std::fs::read(&t.nhlng).unwrap(), before);
    assert!(dirty(&reg, id));
    assert!(reg.copied_target_status(&root).unwrap().apply_allowed);

    // Modified after review: refused; the modified bytes are left alone.
    let header_v2 = vec![0x5Au8; 0x148];
    std::fs::write(t.header(), &header_v2).unwrap();
    let r = reg.copied_target_review(id, &root).unwrap();
    assert_eq!(
        r.bound_inputs[0].identity.sha256,
        FileIdentity::of_bytes(&header_v2).sha256
    );
    let header_v3 = vec![0x11u8; 0x148];
    std::fs::write(t.header(), &header_v3).unwrap();
    let e = reg.copied_target_apply(id, r.handle).err_text();
    assert!(e.contains("changed after planning"), "{e}");
    assert_eq!(std::fs::read(t.header()).unwrap(), header_v3);
    assert!(dirty(&reg, id));

    // A directory where the header should be is not a companion: review refuses.
    std::fs::remove_file(t.header()).unwrap();
    std::fs::create_dir(t.header()).unwrap();
    let e = reg.copied_target_review(id, &root).err_text();
    assert!(e.contains("not a regular file"), "{e}");
    assert!(dirty(&reg, id));
    assert!(reg.copied_target_status(&root).unwrap().apply_allowed);
}

/// R2: the reservation covers the files, so a second workspace on the same files (an alias) is
/// blocked through its own project id — the exact competing-save probe — as are opening a new
/// alias, closing either project and a Save-As into a held file; unrelated projects keep working.
/// After the commit the alias is refreshed (clean → reloaded, dirty → edits kept and stale).
#[test]
fn r2_reservations_cover_aliasing_workspaces() {
    let tmp = Tmp::new("r2-alias");
    let t = copied_tree(&tmp.0);
    let tmp_c = Tmp::new("r2-unrelated");
    let t_c = copied_tree(&tmp_c.0);
    let reg = Registry::default();
    let a = open(&reg, &t);
    let b = open(&reg, &t);
    let c = open(&reg, &t_c);
    let root = s(&t.root);
    let orig = std::fs::read(&t.nhlng).unwrap();

    edit(&reg, a, 2);
    let r = reg.copied_target_review(a, &root).unwrap();
    let handle = r.handle;
    let mut probe = At {
        at: "AfterReplaced(\"primary\")".into(),
        action: Box::new(|| {
            // The exact alias-save probe: refused by the reservation, not discovered by the engine.
            let e = reg.save_workspace(b).err_text();
            assert!(e.contains("locked by a copied-target operation"), "{e}");
            let e = reg
                .set_field(b, "ubPc", 0, "vjla", FieldValue::Int(1))
                .err_text();
            assert!(e.contains("locked by a copied-target operation"), "{e}");
            let e = reg.copied_target_reload(b, true).err_text();
            assert!(e.contains("locked by a copied-target operation"), "{e}");
            // A new alias cannot be opened on held files.
            let e = reg
                .open_workspace_from_paths(
                    Some("nhl14"),
                    parse_platform("ps3").unwrap(),
                    &s(&t.nhlng),
                    Some(&s(&t.roster)),
                    &s(&repo(META)),
                    &s(&repo(DEFS)),
                )
                .err_text();
            assert!(e.contains("held by a copied-target operation"), "{e}");
            // Neither project can be closed while its state is needed — and the typed outcome
            // says *why* (LEAGUE-SMOKE-R1: the shell keeps its owner on `busy`, names the holder).
            assert!(!reg.close_project(a));
            assert!(!reg.close_project(b));
            for id in [a, b] {
                let o = reg.close_project_outcome(id);
                assert_eq!(o.outcome, "busy");
                let why = o.reason.unwrap();
                assert!(
                    why.contains("apply of review handle") && why.contains("cancel it"),
                    "{why}"
                );
            }
            // A Save-As from an unrelated project into a held file is refused; the unrelated
            // project itself keeps working.
            let e = reg.save(c, &s(&t.nhlng)).err_text();
            assert!(e.contains("held by a copied-target operation"), "{e}");
            reg.set_field(c, "ubPc", 0, "vjla", FieldValue::Int(2))
                .unwrap();
            let out = tmp_c.0.join("save-as.db");
            reg.save(c, &s(&out)).unwrap();
            // Recovery and restore of the root are refused while the apply runs.
            assert!(reg
                .copied_target_recover(&root)
                .err_text()
                .contains("apply is running"));
            assert!(reg
                .copied_target_restore(&root, "000001-x")
                .err_text()
                .contains("apply is running"));
            // Status names the reservation and every project it holds.
            let st = reg.copied_target_status(&root).unwrap();
            assert_eq!(st.busy_projects, vec![a, b]);
            assert_eq!(st.reservations.len(), 1);
            assert_eq!(st.reservations[0].projects, vec![a, b]);
            assert!(!st.reservations[0].whole_root);
            assert_eq!(st.reservations[0].files.len(), 3);
            assert!(st.reservations[0]
                .holder
                .contains(&format!("handle {handle}")));
        }),
        done: false,
    };
    let applied = reg
        .copied_target_apply_with_hooks(a, handle, Some(&mut probe))
        .unwrap();
    assert!(probe.done, "the probe ran inside the engine");
    assert_eq!(applied.state, "committed");
    assert_eq!(applied.aliases.len(), 1);
    assert_eq!(applied.aliases[0].project_id, b);
    assert_eq!(applied.aliases[0].primary, "reloaded");
    assert_eq!(applied.aliases[0].roster.as_deref(), Some("reloaded"));
    assert!(applied.aliases[0].refresh_error.is_none());
    assert_eq!(player_position(&reg, b), player_position(&reg, a));
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.reservations.is_empty() && st.busy_projects.is_empty());
    assert_ne!(std::fs::read(&t.nhlng).unwrap(), orig);
    assert!(reg.close_project(b), "closable again after the apply");
    // Idempotent: a second close of the same id is `already_closed`, never `busy`, and closing
    // an id that was never opened reads the same way.
    assert_eq!(reg.close_project_outcome(b).outcome, "already_closed");
    assert_eq!(reg.close_project_outcome(9_999).outcome, "already_closed");
    let c2 = open(&reg, &t);
    assert_eq!(reg.close_project_outcome(c2).outcome, "closed");
    assert_eq!(reg.close_project_outcome(c2).outcome, "already_closed");

    // A dirty alias keeps its edits and is marked stale.
    let b2 = open(&reg, &t);
    edit(&reg, b2, 1);
    let b2_pos = player_position(&reg, b2);
    edit(&reg, a, 0);
    let r = reg.copied_target_review(a, &root).unwrap();
    let applied = reg.copied_target_apply(a, r.handle).unwrap();
    assert_eq!(applied.aliases[0].project_id, b2);
    assert_eq!(applied.aliases[0].primary, "stale-kept-edits");
    assert!(applied.aliases[0].dirty);
    assert!(applied.aliases[0].stale_baseline.is_some());
    assert_eq!(player_position(&reg, b2), b2_pos, "edits kept");
    assert!(reg
        .copied_target_review(b2, &root)
        .err_text()
        .contains("edits were kept"));
    assert!(reg
        .copied_target_reload(b2, false)
        .err_text()
        .contains("unsaved edits"));
    let reloaded = reg.copied_target_reload(b2, true).unwrap();
    assert_eq!(reloaded.primary, "reloaded");
    assert_eq!(player_position(&reg, b2), player_position(&reg, a));
}

/// R2: preflight and reservation are one critical section. A burst of edits racing the apply
/// either lands before it (the apply refuses: revision changed) or is refused by the reservation
/// (the apply commits with zero competing edits accepted). No third outcome.
#[test]
fn r2_same_project_edits_cannot_slip_between_preflight_and_reservation() {
    let mut outcomes = (0usize, 0usize);
    for round in 0..4u32 {
        // A fresh tree and registry per round: the baseline is always the anchor.
        let tmp = Tmp::new(&format!("r2-race-{round}"));
        let t = copied_tree(&tmp.0);
        let reg = Registry::default();
        let id = open(&reg, &t);
        let root = s(&t.root);
        edit(&reg, id, 2);
        let on_disk = std::fs::read(&t.nhlng).unwrap();
        let r = reg.copied_target_review(id, &root).unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel::<()>();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<usize>();
        // Odd rounds: the burst starts only once the apply holds the reservation (the hook fires
        // inside the engine), so the "committed, zero accepted" branch is exercised
        // deterministically; even rounds race freely from the start.
        let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
        let gated = round % 2 == 1;
        if !gated {
            go_tx.send(()).unwrap();
        }
        std::thread::scope(|sc| {
            let reg_ref = &reg;
            sc.spawn(move || {
                started_tx.send(()).unwrap();
                go_rx.recv().unwrap();
                let mut accepted = 0usize;
                for i in 0..40u32 {
                    let v = if (i + round) % 2 == 0 { 0 } else { 1 };
                    if reg_ref
                        .set_field(id, "ubPc", 0, "vjla", FieldValue::Int(v))
                        .is_ok()
                    {
                        accepted += 1;
                    }
                }
                done_tx.send(accepted).unwrap();
            });
            started_rx.recv().unwrap();
            // Hold the apply inside the engine until the burst is over, so every competing edit
            // is decided while the reservation exists.
            let done_rx = &done_rx;
            let go_tx = &go_tx;
            let mut hold = At {
                at: "BeforeStageReplacement(\"primary\")".into(),
                action: Box::new(move || {
                    if gated {
                        go_tx.send(()).unwrap();
                    }
                    let _ = done_rx.recv();
                }),
                done: false,
            };
            let result = reg.copied_target_apply_with_hooks(id, r.handle, Some(&mut hold));
            let held = hold.done;
            drop(hold);
            if !held && gated {
                // The apply was refused before the hook could release the burst.
                go_tx.send(()).unwrap();
            }
            let accepted = if held { 0 } else { done_rx.recv().unwrap() };
            match result {
                Ok(applied) => {
                    assert_eq!(applied.state, "committed");
                    assert_eq!(accepted, 0, "no edit slipped past the reservation");
                    assert!(!dirty(&reg, id));
                    outcomes.0 += 1;
                }
                Err(e) => {
                    assert!(
                        !gated,
                        "a gated round cannot be refused: nothing edited first"
                    );
                    let e = e.to_string();
                    assert!(e.contains("edited after the review"), "{e}");
                    assert!(accepted >= 1, "refused only because an edit landed first");
                    assert_eq!(std::fs::read(&t.nhlng).unwrap(), on_disk, "nothing written");
                    assert!(reg.copied_target_status(&root).unwrap().apply_allowed);
                    outcomes.1 += 1;
                }
            }
        });
    }
    eprintln!(
        "race rounds: {} committed, {} refused",
        outcomes.0, outcomes.1
    );
    assert!(
        outcomes.0 >= 2,
        "the gated rounds committed with zero accepted edits"
    );
}

/// R2: overlapping roots. An apply holding files inside a nested copied-target root blocks a
/// restore/recovery reservation over the enclosing root — reservations are compared on files and
/// roots, not on project ids alone.
#[test]
fn r2_overlapping_roots_contend() {
    let tmp = Tmp::new("r2-roots");
    let outer = copied_tree(&tmp.0);
    // A nested copied-target root inside the outer one, with its own single-file project.
    let nested_root = outer.root.join("nested");
    tdb_txn::engine::create_copied_target_root(&nested_root).unwrap();
    let nested_db = nested_root.join("nhlng.db");
    std::fs::copy(repo(NHLNG), &nested_db).unwrap();
    let reg = Registry::default();
    let outer_id = open(&reg, &outer);
    let nested_id = open_single(&reg, &nested_db);
    // Something restorable in the outer root.
    edit(&reg, outer_id, 2);
    let r = reg.copied_target_review(outer_id, &s(&outer.root)).unwrap();
    let outer_txn = reg.copied_target_apply(outer_id, r.handle).unwrap().txn_id;

    edit(&reg, nested_id, 2);
    let r = reg
        .copied_target_review(nested_id, &s(&nested_root))
        .unwrap();
    let mut probe = At {
        at: "AfterReplaced(\"primary\")".into(),
        action: Box::new(|| {
            let e = reg
                .copied_target_restore(&s(&outer.root), &outer_txn)
                .err_text();
            assert!(
                e.contains("overlaps") && e.contains("apply of review handle"),
                "{e}"
            );
            let e = reg.copied_target_recover(&s(&outer.root)).err_text();
            assert!(e.contains("overlaps"), "{e}");
            // The outer project itself is not held by the nested apply.
            reg.set_field(outer_id, "ubPc", 0, "vjla", FieldValue::Int(1))
                .unwrap();
            let st = reg.copied_target_status(&s(&outer.root)).unwrap();
            assert_eq!(
                st.reservations.len(),
                1,
                "the nested reservation is visible from the outer root"
            );
            assert_eq!(st.reservations[0].projects, vec![nested_id]);
        }),
        done: false,
    };
    let applied = reg
        .copied_target_apply_with_hooks(nested_id, r.handle, Some(&mut probe))
        .unwrap();
    assert!(probe.done);
    assert_eq!(applied.state, "committed");
    // Afterwards the outer restore proceeds; the whole-root reservation covers the nested project
    // too, and both are refreshed (outer: dirty → stale; nested: unchanged by this restore).
    let restored = reg
        .copied_target_restore(&s(&outer.root), &outer_txn)
        .unwrap();
    assert_eq!(restored.state, "restored");
    let by_id = |id: u32| {
        restored
            .affected
            .iter()
            .find(|p| p.project_id == id)
            .unwrap()
    };
    assert_eq!(by_id(outer_id).primary, "stale-kept-edits");
    assert_eq!(by_id(nested_id).primary, "unchanged");
}

/// R3: an alias whose files cannot be re-read after a commit or restore is reported as
/// `refresh-failed` (never "unchanged"), is left exactly as it was — no partial adoption of the
/// changed primary — keeps its edits, and refuses copied-target saves until a reload succeeds.
#[test]
fn r3_refresh_failures_are_reported_and_leave_the_alias_untouched() {
    let tmp = Tmp::new("r3");
    let t = copied_tree(&tmp.0);
    let reg = Registry::default();
    // The initiator is a single-file project on the main db; the alias also holds the roster.
    let a = open_single(&reg, &t.nhlng);
    let b = open(&reg, &t);
    let root = s(&t.root);
    let b_pos_before = player_position(&reg, b);
    let roster_bytes = std::fs::read(&t.roster).unwrap();

    // Clean alias, roster missing at refresh time.
    edit(&reg, a, 2);
    let r = reg.copied_target_review(a, &root).unwrap();
    assert_eq!(r.artifacts.len(), 1);
    std::fs::remove_file(&t.roster).unwrap();
    let applied = reg.copied_target_apply(a, r.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let alias = &applied.aliases[0];
    assert_eq!(alias.project_id, b);
    assert_eq!(alias.primary, "refresh-failed");
    assert_eq!(alias.roster.as_deref(), Some("refresh-failed"));
    assert!(
        alias.refresh_error.as_deref().unwrap().contains(SAVE_NAME),
        "{:?}",
        alias.refresh_error
    );
    assert!(alias
        .stale_baseline
        .as_deref()
        .unwrap()
        .contains("could not be re-read"));
    assert!(!alias.dirty);
    assert_eq!(
        player_position(&reg, b),
        b_pos_before,
        "the changed primary was not adopted"
    );
    let e = reg.copied_target_review(b, &root).err_text();
    assert!(e.contains("could not be re-read"), "{e}");
    // Reload while the file is still missing: refused, nothing changes.
    let e = reg.copied_target_reload(b, false).err_text();
    assert!(e.contains(SAVE_NAME), "{e}");
    assert_eq!(player_position(&reg, b), b_pos_before);
    // The file is back: a plain reload succeeds and adopts the committed primary.
    std::fs::write(&t.roster, &roster_bytes).unwrap();
    let reloaded = reg.copied_target_reload(b, false).unwrap();
    assert_eq!(reloaded.primary, "reloaded");
    assert!(reloaded.stale_baseline.is_none());
    assert_eq!(player_position(&reg, b), player_position(&reg, a));

    // Clean alias, roster corrupt at refresh time (a restore this time).
    std::fs::write(&t.roster, b"garbage where a save container was").unwrap();
    let restored = reg.copied_target_restore(&root, &applied.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    let alias = restored
        .affected
        .iter()
        .find(|p| p.project_id == b)
        .unwrap();
    assert_eq!(alias.primary, "refresh-failed");
    assert!(alias.refresh_error.is_some());
    let initiator = restored
        .affected
        .iter()
        .find(|p| p.project_id == a)
        .unwrap();
    assert_eq!(initiator.primary, "reloaded");
    assert_eq!(
        player_position(&reg, a),
        b_pos_before,
        "the initiator reloaded the original"
    );
    assert_ne!(
        player_position(&reg, b),
        b_pos_before,
        "the alias still shows the pre-restore bytes: nothing was adopted"
    );
    std::fs::write(&t.roster, &roster_bytes).unwrap();
    reg.copied_target_reload(b, false).unwrap();
    assert_eq!(player_position(&reg, b), b_pos_before);

    // Dirty alias, roster missing: the edits survive the failed refresh and the failed reload;
    // once the file is back, reloading without consent is still refused (the primary differs),
    // reloading with consent works.
    edit(&reg, b, 1);
    let b_dirty_pos = player_position(&reg, b);
    edit(&reg, a, 0);
    let r = reg.copied_target_review(a, &root).unwrap();
    std::fs::remove_file(&t.roster).unwrap();
    let applied = reg.copied_target_apply(a, r.handle).unwrap();
    let alias = &applied.aliases[0];
    assert_eq!(alias.primary, "refresh-failed");
    assert!(alias.dirty);
    assert_eq!(player_position(&reg, b), b_dirty_pos);
    assert!(reg
        .copied_target_reload(b, false)
        .err_text()
        .contains(SAVE_NAME));
    assert_eq!(player_position(&reg, b), b_dirty_pos);
    std::fs::write(&t.roster, &roster_bytes).unwrap();
    let e = reg.copied_target_reload(b, false).err_text();
    assert!(e.contains("unsaved edits"), "{e}");
    assert_eq!(player_position(&reg, b), b_dirty_pos);
    let reloaded = reg.copied_target_reload(b, true).unwrap();
    assert_eq!(reloaded.primary, "reloaded");
    assert!(!reloaded.dirty);
    assert_eq!(player_position(&reg, b), player_position(&reg, a));
}

// ---------------------------------------------------------------- S2-C-R2

fn leftover_debug(v: &[studio_api::StagingLeftoverDto]) -> Vec<(String, String, String)> {
    v.iter()
        .map(|l| (l.path.clone(), l.kind.clone(), l.error.clone()))
        .collect()
}

/// Whether `dir` matches names case-insensitively (a scratch file looked up under another
/// spelling), so the alias assertions below say what the file system actually does.
fn folds_case(dir: &Path) -> bool {
    let probe = dir.join("case-probe.tmp");
    std::fs::write(&probe, b"x").unwrap();
    let same = std::fs::canonicalize(dir.join("CASE-PROBE.TMP")).ok()
        == std::fs::canonicalize(&probe).ok();
    std::fs::remove_file(&probe).unwrap();
    same
}

/// R1 (S2-C-R2): the reviewer's reproduction — an unrelated workspace's "Save As" to `NHLNG.DB`
/// while `nhlng.db` is reserved. Refused before any byte changes, for an existing destination
/// (the primary) and a planned absent one (the header), for a new open under the other spelling,
/// and under the whole-root guard of a restore; the initiating transaction commits, sessions are
/// intact, reservations are released and the unrelated workspace keeps working throughout.
#[test]
fn r2_save_as_under_an_equivalent_spelling_is_refused_before_any_byte_changes() {
    let tmp = Tmp::new("r2-case");
    let t = copied_tree(&tmp.0);
    let tmp_c = Tmp::new("r2-case-unrelated");
    let t_c = copied_tree(&tmp_c.0);
    let folds = folds_case(&t.root.join("USRDIR"));
    assert!(
        folds || !cfg!(windows),
        "the reviewer's reproduction needs a case-folding directory on Windows"
    );
    let reg = Registry::default();
    let a = open(&reg, &t);
    let c = open(&reg, &t_c);
    let root = s(&t.root);
    let orig = std::fs::read(&t.nhlng).unwrap();

    edit(&reg, a, 2);
    let pos_a = player_position(&reg, a);
    // The unrelated workspace carries a distinguishable value.
    edit(&reg, c, 1);
    let upper_primary = t.root.join("USRDIR").join("NHLNG.DB");
    let upper_header = t
        .headers
        .join(format!("{}.HEADER", SAVE_NAME.to_uppercase()));
    let r = reg.copied_target_review(a, &root).unwrap();
    let handle = r.handle;
    let mut probe = At {
        at: "AfterReplaced(\"primary\")".into(),
        action: Box::new(|| {
            let before = std::fs::read(&t.nhlng).unwrap();
            let saved = reg.save(c, &s(&upper_primary));
            if folds {
                let e = saved.err_text();
                assert!(e.contains("held by a copied-target operation"), "{e}");
            } else {
                // A case-sensitive directory: a distinct file, which is not held.
                saved.unwrap();
                assert!(upper_primary.exists());
            }
            assert_eq!(
                std::fs::read(&t.nhlng).unwrap(),
                before,
                "the transaction's file is untouched"
            );
            // A planned, still absent destination under another spelling.
            assert!(
                !t.header().exists(),
                "the header is written later in the apply"
            );
            let saved = reg.save(c, &s(&upper_header));
            if folds {
                let e = saved.err_text();
                assert!(e.contains("held by a copied-target operation"), "{e}");
                assert!(!t.header().exists() && !upper_header.exists());
            } else {
                saved.unwrap();
                std::fs::remove_file(&upper_header).unwrap();
            }
            // Opening a workspace under the other spelling is refused just the same.
            let opened = reg.open_workspace_from_paths(
                Some("nhl14"),
                parse_platform("ps3").unwrap(),
                &s(&upper_primary),
                Some(&s(&t.roster)),
                &s(&repo(META)),
                &s(&repo(DEFS)),
            );
            if folds {
                let e = opened.err_text();
                assert!(e.contains("held by a copied-target operation"), "{e}");
            }
            // A target whose identity cannot be established is refused, not written.
            let e = reg
                .save(c, &s(&t.root.join("no-such-dir").join("x.db")))
                .err_text();
            assert!(e.contains("could not be resolved"), "{e}");
            // The unrelated workspace itself keeps working.
            reg.set_field(c, "ubPc", 0, "vjla", FieldValue::Int(1))
                .unwrap();
            reg.save(c, &s(&tmp_c.0.join("elsewhere.db"))).unwrap();
            let st = reg.copied_target_status(&root).unwrap();
            assert_eq!(st.reservations.len(), 1);
            assert_eq!(st.reservations[0].files.len(), 3);
            assert_eq!(st.busy_projects, vec![a]);
        }),
        done: false,
    };
    let applied = reg
        .copied_target_apply_with_hooks(a, handle, Some(&mut probe))
        .unwrap();
    assert!(probe.done, "the probe ran inside the engine");
    assert_eq!(applied.state, "committed");
    assert_eq!(applied.project.primary, "rebound");
    assert!(applied.aliases.is_empty());
    assert!(!dirty(&reg, a));
    assert_eq!(player_position(&reg, a), pos_a);
    assert_ne!(std::fs::read(&t.nhlng).unwrap(), orig);
    if folds {
        assert_eq!(
            std::fs::read(&upper_primary).unwrap(),
            std::fs::read(&t.nhlng).unwrap(),
            "one file under two spellings"
        );
    }
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.reservations.is_empty() && st.busy_projects.is_empty());
    assert!(dirty(&reg, c), "the unrelated workspace kept its edits");
    reg.save(c, &s(&tmp_c.0.join("after.db"))).unwrap();

    // The whole-root guard of a restore uses the same identity: the other spelling is refused.
    let mut probe = At {
        at: "BeforeRevert(\"primary\")".into(),
        action: Box::new(|| {
            let before = std::fs::read(&t.nhlng).unwrap();
            let e = reg.save(c, &s(&upper_primary)).err_text();
            assert!(e.contains("held by a copied-target operation"), "{e}");
            assert_eq!(std::fs::read(&t.nhlng).unwrap(), before);
            reg.save(c, &s(&tmp_c.0.join("during-restore.db"))).unwrap();
        }),
        done: false,
    };
    let restored = reg
        .copied_target_restore_with_hooks(&root, &applied.txn_id, &mut probe)
        .unwrap();
    assert!(probe.done);
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&t.nhlng).unwrap(), orig);
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.reservations.is_empty());
    // After everything, the other spelling is writable again (the file is not held).
    reg.save(c, &s(&upper_primary)).unwrap();
}

/// R2 (S2-C-R2): a prepared handle's staging is owned by the registry and torn down on every
/// normal disposal — closing its project, explicit teardown, a normally dropped registry — while
/// another registry's prepared staging is preserved; a removal that fails is a reported leftover,
/// retried and cleared later, never a silent success.
#[test]
fn r2_prepared_staging_is_torn_down_on_normal_disposal_and_leftovers_are_reported() {
    let tmp = Tmp::new("r2-staging");
    let t = copied_tree(&tmp.0);
    let tmp2 = Tmp::new("r2-staging-other");
    let t2 = copied_tree(&tmp2.0);
    let root = s(&t.root);
    let root2 = s(&t2.root);
    let reg = Registry::default();
    let other = Registry::default();

    // Another registry holds its own prepared handle throughout.
    let b = open(&other, &t2);
    edit(&other, b, 2);
    let rb = other.copied_target_review(b, &root2).unwrap();
    let staging_b = other.copied_target_staging_dir(rb.handle).unwrap();
    assert!(staging_b.is_dir());

    // Closing the project discards its prepared handle: engine set aborted, staging gone.
    let a = open(&reg, &t);
    edit(&reg, a, 2);
    let r = reg.copied_target_review(a, &root).unwrap();
    let staging = reg.copied_target_staging_dir(r.handle).unwrap();
    assert!(staging.is_dir());
    assert!(reg.close_project(a));
    assert!(!staging.exists(), "staging removed with the handle");
    assert!(reg.copied_target_staging_dir(r.handle).is_none());
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.pending_handles.is_empty());
    assert!(st.apply_allowed, "the root is free: the set was aborted");
    assert!(st.staging_leftovers.is_empty());
    assert!(
        staging_b.is_dir(),
        "the other registry's staging is untouched"
    );

    // Explicit teardown: every prepared handle goes; the sessions and their edits stay.
    let a = open(&reg, &t);
    edit(&reg, a, 1);
    let r = reg.copied_target_review(a, &root).unwrap();
    let staging = reg.copied_target_staging_dir(r.handle).unwrap();
    assert!(staging.is_dir());
    let left = reg.teardown_copied_target();
    assert!(left.is_empty(), "{:?}", leftover_debug(&left));
    assert!(!staging.exists());
    let e = reg.copied_target_apply(a, r.handle).err_text();
    assert!(e.contains("unknown, already applied, discarded"), "{e}");
    assert!(dirty(&reg, a), "edits kept");
    assert!(reg.copied_target_status(&root).unwrap().apply_allowed);
    assert!(
        staging_b.is_dir(),
        "the other registry's staging is untouched"
    );

    // A removal that fails is reported and retried, not claimed.
    let r = reg.copied_target_review(a, &root).unwrap();
    let staging = reg.copied_target_staging_dir(r.handle).unwrap();
    let pinned = std::fs::read_dir(&staging)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    // Pinned without delete sharing, so the directory genuinely cannot go yet on Windows.
    #[cfg(windows)]
    let hold = {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1) // FILE_SHARE_READ only
            .open(&pinned)
            .unwrap()
    };
    #[cfg(not(windows))]
    let hold = std::fs::File::open(&pinned).unwrap();
    let st = reg.copied_target_discard(r.handle).unwrap();
    assert!(st.apply_allowed, "the engine set is aborted regardless");
    if cfg!(windows) {
        // An open handle keeps the file: the directory stays and is reported.
        assert!(staging.exists());
        assert_eq!(
            st.staging_leftovers.len(),
            1,
            "{:?}",
            leftover_debug(&st.staging_leftovers)
        );
        assert_eq!(st.staging_leftovers[0].kind, "removal-failed");
        assert!(st.staging_leftovers[0].txn_id.is_none());
        assert!(
            Path::new(&st.staging_leftovers[0].path).file_name() == staging.file_name(),
            "{}",
            st.staging_leftovers[0].path
        );
        assert!(!st.staging_leftovers[0].error.is_empty());
        drop(hold);
        let left = reg.teardown_copied_target();
        assert!(left.is_empty(), "{:?}", leftover_debug(&left));
    } else {
        drop(hold);
        assert!(st.staging_leftovers.is_empty());
    }
    assert!(!staging.exists());
    assert!(reg
        .copied_target_status(&root)
        .unwrap()
        .staging_leftovers
        .is_empty());

    // A normally dropped registry with a prepared handle: the backstop aborts and removes.
    let r = reg.copied_target_review(a, &root).unwrap();
    let staging = reg.copied_target_staging_dir(r.handle).unwrap();
    drop(reg);
    assert!(
        !staging.exists(),
        "staging removed when the registry dropped"
    );
    let fresh = Registry::default();
    let st = fresh.copied_target_status(&root).unwrap();
    assert!(st.apply_allowed, "the dropped registry aborted its set");
    assert!(
        st.transactions.iter().all(|x| x.state == "aborted"),
        "{:?}",
        st.transactions.iter().map(|x| &x.state).collect::<Vec<_>>()
    );

    // The other registry's handle is still whole: it applies.
    assert!(staging_b.is_dir());
    let applied = other.copied_target_apply(b, rb.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert!(!staging_b.exists(), "staging removed after the apply");
}

/// The persisted journal of a transaction, as the status DTO locates it.
fn journal_path(reg: &Registry, root: &str, txn_id: &str) -> PathBuf {
    let st = reg.copied_target_status(root).unwrap();
    let t = st
        .transactions
        .iter()
        .find(|t| t.txn_id == txn_id)
        .expect("the transaction is journaled");
    Path::new(&t.recovery_dir).join(tdb_txn::journal::JOURNAL_FILE)
}

fn txn_state(reg: &Registry, root: &str, txn_id: &str) -> String {
    reg.copied_target_status(root)
        .unwrap()
        .transactions
        .iter()
        .find(|t| t.txn_id == txn_id)
        .map(|t| t.state.clone())
        .unwrap_or_else(|| "gone".into())
}

/// Pin a file so it can be read but not replaced (Windows: `FILE_SHARE_READ` only). Elsewhere a
/// plain open, which does not block replacement — the test branches on `cfg!(windows)`.
fn pin(path: &Path) -> std::fs::File {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(path)
            .unwrap()
    }
    #[cfg(not(windows))]
    {
        std::fs::File::open(path).unwrap()
    }
}

/// R3 (S2-C-R3): the reviewer's locked-journal reproduction. When a prepared set cannot be
/// aborted (its journal is pinned), its staging is **retained** with an `unresolved-transaction`
/// obligation and the persisted state stays `prepared`; repeated teardown and a normal registry
/// Drop re-check but never delete it; every other handle is still settled (a second registry's
/// set is aborted and its staging removed regardless of failure order); once the journal is free,
/// the next teardown — or a recovery of the root — aborts the set and only then removes the
/// staging and clears the obligation.
#[test]
fn r3_abort_failure_retains_staging_until_the_set_is_resolved() {
    let tmp = Tmp::new("r3-pinned");
    let t = copied_tree(&tmp.0);
    let tmp2 = Tmp::new("r3-pinned-other");
    let t2 = copied_tree(&tmp2.0);
    let root = s(&t.root);
    let root2 = s(&t2.root);
    let reg = Registry::default();
    let a = open(&reg, &t);
    let b = open(&reg, &t2);
    edit(&reg, a, 2);
    edit(&reg, b, 2);
    let ra = reg.copied_target_review(a, &root).unwrap();
    let rb = reg.copied_target_review(b, &root2).unwrap();
    let staging_a = reg.copied_target_staging_dir(ra.handle).unwrap();
    let staging_b = reg.copied_target_staging_dir(rb.handle).unwrap();
    assert!(staging_a.is_dir() && staging_b.is_dir());
    let journal_a = journal_path(&reg, &root, &ra.txn_id);

    // ---- teardown while a's journal is pinned: b settles, a is retained.
    let hold = pin(&journal_a);
    let left = reg.teardown_copied_target();
    assert!(
        !staging_b.exists(),
        "the other handle was settled regardless"
    );
    assert_eq!(txn_state(&reg, &root2, &rb.txn_id), "aborted");
    assert!(reg.copied_target_staging_dir(ra.handle).is_none());
    assert!(reg.copied_target_staging_dir(rb.handle).is_none());
    if cfg!(windows) {
        assert!(staging_a.is_dir(), "retained: the set still references it");
        assert_eq!(txn_state(&reg, &root, &ra.txn_id), "prepared");
        assert_eq!(left.len(), 1, "{:?}", leftover_debug(&left));
        assert_eq!(left[0].kind, "unresolved-transaction");
        assert_eq!(left[0].txn_id.as_deref(), Some(ra.txn_id.as_str()));
        assert_eq!(Path::new(&left[0].path).file_name(), staging_a.file_name());
        assert!(
            left[0].error.contains("could not be aborted"),
            "{}",
            left[0].error
        );
        let st = reg.copied_target_status(&root).unwrap();
        assert!(st.pending_handles.is_empty());
        assert!(!st.apply_allowed, "the prepared set still holds the root");
        assert_eq!(st.staging_leftovers.len(), 1);
        assert_eq!(st.staging_leftovers[0].kind, "unresolved-transaction");

        // Repeated teardown re-checks and keeps it.
        for _ in 0..2 {
            let again = reg.teardown_copied_target();
            assert_eq!(again.len(), 1, "{:?}", leftover_debug(&again));
            assert!(staging_a.is_dir());
            assert_eq!(txn_state(&reg, &root, &ra.txn_id), "prepared");
        }
        // Reviewing the same project again is refused (the root is held by the prepared set) and
        // the retained staging is untouched by that refusal.
        let e = reg.copied_target_review(a, &root).err_text();
        assert!(e.contains("cannot take a new save"), "{e}");
        assert!(staging_a.is_dir());

        // ---- the journal is freed: the next teardown aborts the set, then removes the staging.
        drop(hold);
        let left = reg.teardown_copied_target();
        assert!(left.is_empty(), "{:?}", leftover_debug(&left));
        assert!(!staging_a.exists());
        assert_eq!(txn_state(&reg, &root, &ra.txn_id), "aborted");
        let st = reg.copied_target_status(&root).unwrap();
        assert!(st.staging_leftovers.is_empty() && st.apply_allowed);
        assert!(
            dirty(&reg, a),
            "the session and its edits were never touched"
        );

        // ---- the same, resolved by recovery of the root instead of a retry.
        let rc = reg.copied_target_review(a, &root).unwrap();
        let staging_c = reg.copied_target_staging_dir(rc.handle).unwrap();
        let journal_c = journal_path(&reg, &root, &rc.txn_id);
        let hold = pin(&journal_c);
        let left = reg.teardown_copied_target();
        assert_eq!(left.len(), 1);
        assert!(staging_c.is_dir());
        drop(hold);
        let rec = reg.copied_target_recover(&root).unwrap();
        assert_eq!(txn_state(&reg, &root, &rc.txn_id), "aborted");
        assert!(rec.status.staging_leftovers.is_empty());
        assert!(
            !staging_c.exists(),
            "settled once recovery resolved the set"
        );

        // ---- a normal Drop cannot delete an unresolved set's staging either.
        let rd = reg.copied_target_review(a, &root).unwrap();
        let staging_d = reg.copied_target_staging_dir(rd.handle).unwrap();
        let journal_d = journal_path(&reg, &root, &rd.txn_id);
        let hold = pin(&journal_d);
        drop(reg);
        assert!(staging_d.is_dir(), "Drop retained it");
        drop(hold);
        let fresh = Registry::default();
        assert_eq!(txn_state(&fresh, &root, &rd.txn_id), "prepared");
        assert!(!fresh.copied_target_status(&root).unwrap().apply_allowed);
        // The obligation died with the registry; the set is resolved by recovery and the staging
        // is then unreferenced scratch at the path the change set records (test-owned cleanup).
        let rec = fresh.copied_target_recover(&root).unwrap();
        assert!(rec.status.apply_allowed);
        assert_eq!(txn_state(&fresh, &root, &rd.txn_id), "aborted");
        assert!(staging_d.is_dir(), "a fresh registry does not know it");
        let cs_path =
            Path::new(&journal_d.parent().unwrap()).join(tdb_txn::journal::CHANGESET_FILE);
        let cs: tdb_txn::ChangeSet =
            serde_json::from_slice(&std::fs::read(&cs_path).unwrap()).unwrap();
        assert_eq!(cs.artifacts[0].replacement.parent().unwrap(), staging_d);
        std::fs::remove_dir_all(&staging_d).unwrap();
    } else {
        // Without a blocking pin both settle at once.
        drop(hold);
        assert!(left.is_empty(), "{:?}", leftover_debug(&left));
        assert!(!staging_a.exists());
        assert_eq!(txn_state(&reg, &root, &ra.txn_id), "aborted");
    }
}

/// Every file under `dir` with its bytes, for "staged bytes untouched" assertions.
fn dir_bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// R4 finding 1: a recovery refused by an overlapping reservation must leave every prepared
/// handle on that root exactly as it was — still listed, still applicable, its staged bytes
/// untouched — and the review is then resolved normally by apply (first round) or discard
/// (second round).
#[test]
fn r4_refused_recovery_keeps_the_prepared_handle_and_its_staging() {
    let tmp = Tmp::new("r4-recover");
    let outer = copied_tree(&tmp.0);
    let nested_root = outer.root.join("nested");
    tdb_txn::engine::create_copied_target_root(&nested_root).unwrap();
    let nested_db = nested_root.join("nhlng.db");
    std::fs::copy(repo(NHLNG), &nested_db).unwrap();
    let reg = Registry::default();
    let outer_id = open(&reg, &outer);
    let nested_id = open_single(&reg, &nested_db);
    let outer_root = s(&outer.root);
    let orig_outer = std::fs::read(&outer.nhlng).unwrap();

    for round in 0..2 {
        edit(&reg, outer_id, 2 - round);
        let ro = reg.copied_target_review(outer_id, &outer_root).unwrap();
        let staging_o = reg.copied_target_staging_dir(ro.handle).unwrap();
        let staged_before = dir_bytes(&staging_o);

        edit(&reg, nested_id, 2 - round);
        let rn = reg
            .copied_target_review(nested_id, &s(&nested_root))
            .unwrap();
        let mut probe = At {
            at: "AfterReplaced(\"primary\")".into(),
            action: Box::new(|| {
                let e = reg.copied_target_recover(&outer_root).err_text();
                assert!(e.contains("overlaps"), "{e}");
                // The refusal consumed nothing.
                assert_eq!(
                    reg.copied_target_staging_dir(ro.handle).as_deref(),
                    Some(staging_o.as_path())
                );
                assert_eq!(
                    dir_bytes(&staging_o),
                    staged_before,
                    "staged bytes untouched"
                );
                let st = reg.copied_target_status(&outer_root).unwrap();
                assert!(st.pending_handles.iter().any(|h| h.handle == ro.handle));
                assert!(st.staging_leftovers.is_empty());
            }),
            done: false,
        };
        let applied = reg
            .copied_target_apply_with_hooks(nested_id, rn.handle, Some(&mut probe))
            .unwrap();
        assert!(probe.done);
        assert_eq!(applied.state, "committed");

        // The outer review is still whole and resolves normally.
        assert_eq!(dir_bytes(&staging_o), staged_before);
        let st = reg.copied_target_status(&outer_root).unwrap();
        assert!(st.pending_handles.iter().any(|h| h.handle == ro.handle));
        assert!(st.reservations.is_empty());
        if round == 0 {
            let applied = reg.copied_target_apply(outer_id, ro.handle).unwrap();
            assert_eq!(applied.state, "committed");
            assert_ne!(std::fs::read(&outer.nhlng).unwrap(), orig_outer);
            assert!(!dirty(&reg, outer_id));
        } else {
            let st = reg.copied_target_discard(ro.handle).unwrap();
            assert!(st.pending_handles.is_empty() && st.apply_allowed);
            assert!(dirty(&reg, outer_id), "discard keeps the edits");
        }
        assert!(!staging_o.exists(), "settled with the handle");
        let st = reg.copied_target_status(&outer_root).unwrap();
        assert!(st.staging_leftovers.is_empty());
        assert!(st.pending_handles.is_empty());
    }
    let left = reg.teardown_copied_target();
    assert!(left.is_empty(), "{:?}", leftover_debug(&left));
}

/// R4 finding 2: an apply refused at preflight (the workspace was edited after the review) whose
/// abort then fails (journal pinned) must retain the staging as a reported obligation with the
/// journal still `prepared`, touch no destination and keep the edits; repeated teardown and a
/// normal Drop preserve it; unpinning plus settlement or recovery removes it. The same guard
/// releases staging on the ordinary refusal (abort succeeds) and when a hook panics inside the
/// engine before mutation (the set is aborted by the settle, staging released).
#[test]
fn r4_apply_refusal_with_a_failed_abort_retains_staging() {
    let tmp = Tmp::new("r4-apply");
    let t = copied_tree(&tmp.0);
    let root = s(&t.root);
    let orig = std::fs::read(&t.nhlng).unwrap();
    let orig_roster = std::fs::read(&t.roster).unwrap();
    let reg = Registry::default();
    let a = open(&reg, &t);

    // ---- ordinary refusal: abort succeeds, staging released, nothing retained.
    edit(&reg, a, 2);
    let r = reg.copied_target_review(a, &root).unwrap();
    let staging = reg.copied_target_staging_dir(r.handle).unwrap();
    edit(&reg, a, 1);
    let e = reg.copied_target_apply(a, r.handle).err_text();
    assert!(
        e.contains("edited after the review") && e.contains("root is free"),
        "{e}"
    );
    assert!(!staging.exists());
    assert_eq!(txn_state(&reg, &root, &r.txn_id), "aborted");
    assert!(reg
        .copied_target_status(&root)
        .unwrap()
        .staging_leftovers
        .is_empty());
    assert!(dirty(&reg, a));

    // ---- refusal with a pinned journal: abort fails, staging retained and reported.
    let r = reg.copied_target_review(a, &root).unwrap();
    let staging = reg.copied_target_staging_dir(r.handle).unwrap();
    let staged = dir_bytes(&staging);
    let journal = journal_path(&reg, &root, &r.txn_id);
    edit(&reg, a, 2);
    let pos = player_position(&reg, a);
    let hold = pin(&journal);
    let e = reg.copied_target_apply(a, r.handle).err_text();
    assert!(e.contains("edited after the review"), "{e}");
    assert_eq!(
        std::fs::read(&t.nhlng).unwrap(),
        orig,
        "no destination touched"
    );
    assert_eq!(std::fs::read(&t.roster).unwrap(), orig_roster);
    assert!(!t.header().exists());
    assert!(
        dirty(&reg, a) && player_position(&reg, a) == pos,
        "edits kept"
    );
    assert!(
        reg.copied_target_staging_dir(r.handle).is_none(),
        "the handle is consumed"
    );
    if cfg!(windows) {
        assert!(
            e.contains("could not be aborted cleanly") && e.contains("retained"),
            "{e}"
        );
        assert!(staging.is_dir(), "retained");
        assert_eq!(dir_bytes(&staging), staged);
        assert_eq!(txn_state(&reg, &root, &r.txn_id), "prepared");
        let st = reg.copied_target_status(&root).unwrap();
        assert!(st.pending_handles.is_empty() && !st.apply_allowed);
        assert_eq!(
            st.staging_leftovers.len(),
            1,
            "{:?}",
            leftover_debug(&st.staging_leftovers)
        );
        assert_eq!(st.staging_leftovers[0].kind, "unresolved-transaction");
        assert_eq!(
            st.staging_leftovers[0].txn_id.as_deref(),
            Some(r.txn_id.as_str())
        );
        // Repeated teardown keeps it.
        for _ in 0..2 {
            let left = reg.teardown_copied_target();
            assert_eq!(left.len(), 1);
            assert!(staging.is_dir());
            assert_eq!(txn_state(&reg, &root, &r.txn_id), "prepared");
        }
        // Unpinned: settlement aborts the set, then removes the staging.
        drop(hold);
        let left = reg.teardown_copied_target();
        assert!(left.is_empty(), "{:?}", leftover_debug(&left));
        assert!(!staging.exists());
        assert_eq!(txn_state(&reg, &root, &r.txn_id), "aborted");
        assert!(reg.copied_target_status(&root).unwrap().apply_allowed);

        // ---- the same, resolved by recovery instead.
        let r = reg.copied_target_review(a, &root).unwrap();
        let staging = reg.copied_target_staging_dir(r.handle).unwrap();
        let journal = journal_path(&reg, &root, &r.txn_id);
        edit(&reg, a, 1);
        let hold = pin(&journal);
        let _ = reg.copied_target_apply(a, r.handle).err_text();
        assert!(staging.is_dir());
        drop(hold);
        let rec = reg.copied_target_recover(&root).unwrap();
        assert_eq!(txn_state(&reg, &root, &r.txn_id), "aborted");
        assert!(rec.status.staging_leftovers.is_empty());
        assert!(!staging.exists());
    } else {
        drop(hold);
        assert!(!staging.exists());
        assert_eq!(txn_state(&reg, &root, &r.txn_id), "aborted");
    }

    // ---- a hook panic before any mutation: the guard settles (abort) and releases staging.
    let r = reg.copied_target_review(a, &root).unwrap();
    let staging = reg.copied_target_staging_dir(r.handle).unwrap();
    struct PanicAt(String);
    impl Hooks for PanicAt {
        fn event(&mut self, e: &Event) -> std::io::Result<()> {
            if format!("{e:?}") == self.0 {
                panic!("hook panic for the test");
            }
            Ok(())
        }
    }
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reg.copied_target_apply_with_hooks(
            a,
            r.handle,
            Some(&mut PanicAt("BeforeStageReplacement(\"primary\")".into())),
        )
    }));
    assert!(outcome.is_err(), "the panic propagated");
    assert!(!staging.exists(), "settled on unwind");
    assert_eq!(txn_state(&reg, &root, &r.txn_id), "aborted");
    assert_eq!(std::fs::read(&t.nhlng).unwrap(), orig);
    let st = reg.copied_target_status(&root).unwrap();
    assert!(st.staging_leftovers.is_empty() && st.pending_handles.is_empty());
    assert!(st.reservations.is_empty(), "the reservation guard released");
    assert!(dirty(&reg, a), "edits kept");

    // ---- a hook panic after mutation began: recovery material is the engine's; the staging
    // is retained (journal `applying`) until recovery rolls the set back, then released.
    let r = reg.copied_target_review(a, &root).unwrap();
    let staging = reg.copied_target_staging_dir(r.handle).unwrap();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reg.copied_target_apply_with_hooks(
            a,
            r.handle,
            Some(&mut PanicAt("AfterReplaced(\"primary\")".into())),
        )
    }));
    assert!(outcome.is_err());
    assert_eq!(txn_state(&reg, &root, &r.txn_id), "applying");
    assert!(staging.is_dir(), "retained while the set is unresolved");
    let st = reg.copied_target_status(&root).unwrap();
    assert_eq!(
        st.staging_leftovers.len(),
        1,
        "{:?}",
        leftover_debug(&st.staging_leftovers)
    );
    assert_eq!(st.staging_leftovers[0].kind, "unresolved-transaction");
    assert!(st.reservations.is_empty());
    let rec = reg.copied_target_recover(&root).unwrap();
    assert_eq!(rec.status.transactions.last().unwrap().state, "rolled-back");
    assert_eq!(
        std::fs::read(&t.nhlng).unwrap(),
        orig,
        "rolled back to the original"
    );
    assert!(!staging.exists(), "released once recovery resolved the set");
    assert!(rec.status.staging_leftovers.is_empty());
    let left = reg.teardown_copied_target();
    assert!(left.is_empty(), "{:?}", leftover_debug(&left));
}
