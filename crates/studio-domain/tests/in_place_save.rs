//! LEAGUE-PARITY-COMPLETION-R1 F1/F2: the in-place save is a coordinated publication of the
//! main database, the roster save and the attached dictionary — preflighted as a set, published
//! all-or-nothing with a verified rollback, and never adopting a split result — and a reload that
//! cannot prepare every file (dictionary included) changes nothing. NHL Legacy PS3 anchors are
//! copied to a temp dir; the anchors are never written.

use std::path::{Path, PathBuf};

use studio_domain::in_place_save::{Event, Hooks, TEMP_SUFFIX};
use studio_domain::tdb_txn::FileIdentity;
use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole, Workspace};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DICT: &str = "anchors/ps3/nhllegacy/databases/nhl_eng_us.db";
const DICT_SHA: &str = "708145517ee4f58c603b8f381e180e7249daf2331ebca0d0a58fc851699f8581";

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-r1f1-{tag}-{}", std::process::id()));
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

fn sha(p: &Path) -> String {
    FileIdentity::of_file(p).unwrap().sha256
}

fn temp_of(p: &Path) -> PathBuf {
    let mut n = p.file_name().unwrap().to_os_string();
    n.push(TEMP_SUFFIX);
    p.with_file_name(n)
}

/// A workspace on temp copies with the dictionary attached, one DB edit and one pending rename.
fn dirty_workspace(tmp: &Tmp) -> (Workspace, PathBuf, PathBuf) {
    let db = tmp.0.join("nhlng.db");
    let dict = tmp.0.join("nhl_eng_us.db");
    std::fs::copy(repo(DB), &db).unwrap();
    std::fs::copy(repo(DICT), &dict).unwrap();
    let session = EditSession::begin(
        Project::load(
            &db,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(META),
            repo("defs"),
        )
        .unwrap(),
    );
    let mut ws = Workspace::single(session, db.to_string_lossy().into_owned());
    ws.attach_dictionary(dict.to_str().unwrap()).unwrap();
    ws.set_field("kOtt", 0, "xsnn", FieldValue::Text("ZZZ".into()))
        .unwrap();
    ws.set_dictionary_repoint("TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE")
        .unwrap();
    assert!(ws.is_dirty());
    (ws, db, dict)
}

/// Everything the workspace must still hold after a refused or rolled-back save.
struct Snapshot {
    working: Vec<u8>,
    loaded: Option<FileIdentity>,
    revision: studio_domain::WorkspaceRevision,
    undo_depth: usize,
    pending_cells: usize,
    rename: Option<(String, String)>,
    dict_baseline: String,
}

fn snapshot(ws: &Workspace) -> Snapshot {
    let (pending, _) = ws.pending_changes(100);
    Snapshot {
        working: ws.primary().bytes().to_vec(),
        loaded: ws.primary().project().loaded_identity().cloned(),
        revision: ws.revision(),
        undo_depth: ws.undo_depth(),
        pending_cells: pending.total_cells,
        rename: ws
            .pending_dictionary()
            .map(|p| (p.before.clone(), p.after.clone())),
        dict_baseline: ws.dictionary().unwrap().sha256.clone(),
    }
}

fn assert_unchanged(ws: &Workspace, before: &Snapshot) {
    let now = snapshot(ws);
    assert_eq!(now.working, before.working, "working bytes kept");
    assert_eq!(now.loaded, before.loaded, "loaded identity kept");
    assert_eq!(now.revision, before.revision, "revision kept");
    assert_eq!(now.undo_depth, before.undo_depth, "history kept");
    assert_eq!(now.pending_cells, before.pending_cells, "pending cells kept");
    assert_eq!(now.rename, before.rename, "pending rename kept");
    assert_eq!(now.dict_baseline, before.dict_baseline, "dictionary baseline kept");
    assert!(ws.is_dirty());
}

#[test]
fn f1_a_changed_dictionary_refuses_the_whole_save_before_the_database_is_written() {
    let tmp = Tmp::new("changed-dict");
    let (mut ws, db, dict) = dirty_workspace(&tmp);
    let db_sha = sha(&db);
    let before = snapshot(&ws);
    assert_eq!(before.pending_cells, 1);
    assert_eq!(
        before.rename.as_ref().map(|(b, a)| (b.as_str(), a.as_str())),
        Some(("Norway", "Rest Of World"))
    );

    // The dictionary changes underneath (one flipped byte in the pool).
    let original = std::fs::read(&dict).unwrap();
    let mut tampered = original.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    std::fs::write(&dict, &tampered).unwrap();
    let tampered_sha = sha(&dict);

    let err = ws.save_in_place().unwrap_err().to_string();
    assert!(err.contains("changed since it was attached"), "{err}");
    assert!(err.contains("nothing was written"), "{err}");
    // Every destination is exactly what it was before the attempt.
    assert_eq!(sha(&db), db_sha, "the database was NOT written");
    assert_eq!(sha(&dict), tampered_sha, "the dictionary was not touched");
    assert!(!temp_of(&db).exists() && !temp_of(&dict).exists());
    assert_unchanged(&ws, &before);
    assert!(ws.stale_baseline().is_none(), "a refusal is not a stale baseline");

    // Put the dictionary back: the same save now publishes both files.
    std::fs::write(&dict, &original).unwrap();
    assert_eq!(ws.save_in_place().unwrap(), 2);
    assert_ne!(sha(&db), db_sha);
    assert_ne!(sha(&dict), DICT_SHA);
    assert!(!ws.is_dirty());
    assert!(ws.pending_dictionary().is_none());
    assert_eq!(
        tdb_loc::LocDictionary::parse(&std::fs::read(&dict).unwrap())
            .unwrap()
            .text("TXT_LEAGUE_NORWAY"),
        Some("Rest Of World")
    );
}

#[test]
fn f1_b_a_changed_database_refuses_the_whole_save_before_the_dictionary_is_written() {
    let tmp = Tmp::new("changed-db");
    let (mut ws, db, dict) = dirty_workspace(&tmp);
    let before = snapshot(&ws);
    // Another valid database lands at the path (the workspace's own packaged edit, as a stand-in
    // for an external tool's write).
    let other = ws.primary().save().unwrap();
    std::fs::write(&db, &other).unwrap();
    let other_sha = sha(&db);
    let err = ws.save_in_place().unwrap_err().to_string();
    assert!(err.contains("the main database"), "{err}");
    assert!(err.contains("changed since it was opened"), "{err}");
    assert_eq!(sha(&db), other_sha);
    assert_eq!(sha(&dict), DICT_SHA, "the dictionary was NOT written");
    assert_unchanged(&ws, &before);
}

#[test]
fn f1_c_a_dictionary_write_failure_after_the_database_was_written_rolls_the_database_back() {
    let tmp = Tmp::new("rollback");
    let (mut ws, db, dict) = dirty_workspace(&tmp);
    let db_sha = sha(&db);
    let before = snapshot(&ws);
    // Make the dictionary's temporary name unusable: the database publishes, the dictionary fails.
    std::fs::create_dir(temp_of(&dict)).unwrap();
    let err = ws.save_in_place().unwrap_err().to_string();
    assert!(err.contains("could not write the dictionary"), "{err}");
    assert!(err.contains("nothing on disk changed"), "{err}");
    assert_eq!(sha(&db), db_sha, "the database was put back");
    assert_eq!(sha(&dict), DICT_SHA);
    assert!(!temp_of(&db).exists());
    assert_unchanged(&ws, &before);
    assert!(ws.stale_baseline().is_none(), "a verified rollback leaves no stale state");

    // The same rename and edit publish once the obstacle is gone.
    std::fs::remove_dir(temp_of(&dict)).unwrap();
    assert_eq!(ws.save_in_place().unwrap(), 2);
    assert!(!ws.is_dirty());
    assert_ne!(sha(&db), db_sha);
    assert_eq!(
        ws.primary().project().loaded_identity().unwrap().sha256,
        sha(&db),
        "the saved file is the new baseline"
    );
    assert_eq!(ws.dictionary().unwrap().sha256, sha(&dict));
}

/// The one case a rollback cannot fix is reported as exactly the split it is, nothing is adopted,
/// and the workspace refuses further saves until an explicit reload.
#[test]
fn f1_d_a_failed_rollback_is_an_explicit_split_state_and_blocks_saves_until_reload() {
    let tmp = Tmp::new("split");
    let (mut ws, db, dict) = dirty_workspace(&tmp);
    let db_sha = sha(&db);
    let before = snapshot(&ws);
    struct Split {
        db: PathBuf,
        dict: PathBuf,
    }
    impl Hooks for Split {
        fn event(&mut self, e: &Event) -> std::io::Result<()> {
            match e {
                Event::BeforeWrite(l) if *l == self.dict.display().to_string() => {
                    Err(std::io::Error::other("dictionary write refused (injected)"))
                }
                Event::BeforeRevert(l) if *l == self.db.display().to_string() => {
                    std::fs::create_dir(temp_of(&self.db)).unwrap();
                    Ok(())
                }
                _ => Ok(()),
            }
        }
    }
    let err = ws
        .save_in_place_with(&mut Split {
            db: db.clone(),
            dict: dict.clone(),
        })
        .unwrap_err()
        .to_string();
    assert!(err.contains("holding the NEW bytes: the main database"), "{err}");
    assert!(err.contains("marked stale"), "{err}");
    assert_ne!(sha(&db), db_sha, "the database holds the new bytes");
    assert_eq!(sha(&dict), DICT_SHA, "the dictionary holds the old bytes");
    // Nothing was adopted: the session still measures against the original baseline.
    assert_unchanged(&ws, &before);
    let reason = ws.stale_baseline().expect("stale after a split");
    assert!(reason.contains("holding the NEW bytes"));
    let again = ws.save_in_place().unwrap_err().to_string();
    assert!(again.contains("baseline is stale"), "{again}");

    // An explicit reload takes what is on disk (the published database, the unchanged
    // dictionary) and clears the state.
    std::fs::remove_dir(temp_of(&db)).unwrap();
    ws.reload_from_disk().unwrap();
    assert!(ws.stale_baseline().is_none());
    assert!(!ws.is_dirty());
    assert_eq!(ws.primary().project().loaded_identity().unwrap().sha256, sha(&db));
    assert_eq!(ws.dictionary().unwrap().sha256, DICT_SHA);
}

#[test]
fn f2_a_missing_dictionary_fails_the_reload_and_preserves_everything() {
    let tmp = Tmp::new("reload-missing");
    let (mut ws, db, dict) = dirty_workspace(&tmp);
    let db_sha = sha(&db);
    let before = snapshot(&ws);
    let original = std::fs::read(&dict).unwrap();
    std::fs::remove_file(&dict).unwrap();

    let err = ws.reload_from_disk().unwrap_err().to_string();
    assert!(err.contains("attached dictionary"), "{err}");
    assert!(err.contains("could not be reloaded"), "{err}");
    assert!(ws.dictionary().is_some(), "the dictionary stays attached");
    assert_unchanged(&ws, &before);
    assert_eq!(sha(&db), db_sha, "a reload never writes");
    let reason = ws.stale_baseline().unwrap();
    assert!(reason.contains("could not be reloaded"), "{reason}");
    let refused = ws.save_in_place().unwrap_err().to_string();
    assert!(refused.contains("baseline is stale"), "{refused}");

    // The file comes back: the reload adopts disk, drops the working state, clears the reason.
    std::fs::write(&dict, &original).unwrap();
    ws.reload_from_disk().unwrap();
    assert!(!ws.is_dirty());
    assert!(ws.pending_dictionary().is_none());
    assert_eq!(ws.undo_depth(), 0);
    assert!(ws.stale_baseline().is_none());
    assert_eq!(ws.dictionary().unwrap().sha256, DICT_SHA);
}

#[test]
fn f2_b_a_crc_invalid_dictionary_fails_the_reload_and_preserves_everything() {
    let tmp = Tmp::new("reload-crc");
    let (mut ws, _db, dict) = dirty_workspace(&tmp);
    let before = snapshot(&ws);
    let original = std::fs::read(&dict).unwrap();
    let mut bad = original.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    std::fs::write(&dict, &bad).unwrap();

    let err = ws.reload_from_disk().unwrap_err().to_string();
    assert!(err.contains("attached dictionary"), "{err}");
    assert!(ws.dictionary().is_some());
    assert_unchanged(&ws, &before);
    assert!(ws.stale_baseline().is_some());

    std::fs::write(&dict, &original).unwrap();
    ws.reload_from_disk().unwrap();
    assert!(!ws.is_dirty() && ws.stale_baseline().is_none());
}

#[test]
fn f2_c_detaching_is_a_deliberate_action_that_clears_only_dictionary_staleness() {
    let tmp = Tmp::new("detach");
    let (mut ws, _db, dict) = dirty_workspace(&tmp);
    std::fs::remove_file(&dict).unwrap();
    ws.reload_from_disk().unwrap_err();
    assert!(ws.stale_baseline().is_some());
    assert!(ws.detach_dictionary());
    assert!(ws.dictionary().is_none());
    assert!(ws.pending_dictionary().is_none());
    assert!(ws.stale_baseline().is_none(), "the reason was about the dictionary only");
    assert!(ws.is_dirty(), "the database edit is still pending");
    assert!(!ws.detach_dictionary(), "nothing left to detach");
    // The database edit alone now saves (one file).
    assert_eq!(ws.save_in_place().unwrap(), 1);
    assert!(!ws.is_dirty());
}
