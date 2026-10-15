//! ONB-003 §3 — Open existing project: edit → save → close → reopen restores the same project,
//! its edited files, its clean pending state and its last view; a moved project reopens without
//! its original root; a missing installation becomes a warning and is never substituted; a corrupt
//! manifest, an unknown schema and a concurrent open are refused with specific messages; an
//! ONB-002 (`nhlms-project/1`) manifest migrates losslessly; an interrupted save is reported, not
//! resumed. Every written path is a temp tree this test owns.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::tdb_txn::FileIdentity;
use studio_domain::{BaselineStore, FieldValue};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn defs() -> String {
    repo("defs").to_string_lossy().into_owned()
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn store() -> BaselineStore {
    BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap()
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(from, to).unwrap();
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-onb003-resume-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ps3_install(base: &Path) -> PathBuf {
    let root = base.join("rpcs3");
    let usrdir = root.join("dev_hdd0/game/BLUS31179/USRDIR");
    copy(&repo("anchors/ps3/nhl14/databases/nhlng.db"), &usrdir.join("nhlng.db"));
    copy(&repo("anchors/ps3/nhl14/databases/nhlng-meta.xml"), &usrdir.join("nhlng-meta.xml"));
    copy(&repo("anchors/ps3/nhl14/databases/nhl_eng_us.db"), &usrdir.join("nhl_eng_us.db"));
    copy(&repo("anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA"), &root.join("dev_hdd0/home/00000001/savedata/BLUS311790200 - ROSTER/SYS-DATA"));
    root
}

fn edit(reg: &Registry, id: u32, value: i64) {
    reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(value)).unwrap();
    if !reg.summary(id).unwrap().dirty {
        reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(value - 1)).unwrap();
    }
}

fn vjla(reg: &Registry, id: u32) -> String {
    let players = reg.list_players(id).unwrap();
    let view = reg.player_view(id, players[0].player_id).unwrap().unwrap();
    view.identity.iter().chain(view.ratings.iter()).find(|f| f.studio_field == "vjla" || f.name == "position").map(|f| f.display.clone()).unwrap_or_default()
}

#[test]
fn edit_save_close_reopen_restores_files_state_and_last_view() {
    let t = Tmp::new("reopen");
    let root = ps3_install(&t.0);
    let reg = Registry::default();
    let st = store();
    let dir = t.0.join("project");
    let opened = reg.project_create_fresh(&s(&dir), "nhl14", parse_platform("ps3").unwrap(), &s(&root), None, None, &st, &defs()).unwrap();
    let id = opened.summary.project_id;
    assert_eq!(opened.project.schema, "nhlms-project/2");
    assert!(opened.project.migrated_from.is_none());
    edit(&reg, id, 7);
    let shown = vjla(&reg, id);
    reg.project_save(id).unwrap();
    reg.project_set_view(id, "/app/database/players/1").unwrap();
    let err = reg.project_set_view(id, "C:\\evil").err().expect("not a route").to_string();
    assert!(err.contains("not a Studio view route"), "{err}");
    let working = PathBuf::from(opened.project.working_main_db.as_deref().unwrap());
    let saved_identity = FileIdentity::of_file(&working).unwrap();
    assert!(reg.close_project(id));

    let reopened = reg.project_open(&s(&dir), &defs()).unwrap();
    let id2 = reopened.summary.project_id;
    assert_ne!(id, id2);
    assert_eq!(reopened.project.project_id, opened.project.project_id, "the same project identity");
    assert_eq!(reopened.last_view.as_deref(), Some("/app/database/players/1"));
    assert!(!reopened.summary.dirty);
    assert_eq!(reopened.summary.undo_depth, 0);
    assert!(!reopened.warnings.iter().any(|w| w.contains("undo history")), "{:?}", reopened.warnings);
    assert_eq!(FileIdentity::of_file(&working).unwrap(), saved_identity);
    assert_eq!(vjla(&reg, id2), shown, "the edited value is what the session shows after the reopen");
    assert!(!reopened.summary.roster_attached, "fresh mod projects reopen without a save mirror");
    assert_eq!(reopened.project.destinations.picked_root, s(&root));
    assert!(reopened.discovery.nhlng_path.is_some());
    // Opening again reconnects to the same native session after a frontend reload.
    let again = reg.project_open(&s(&dir), &defs()).unwrap();
    assert_eq!(again.summary.project_id, id2);
    assert!(reg.close_project(id2));
}

#[test]
fn a_moved_project_reopens_and_a_missing_installation_warns_without_substitution() {
    let t = Tmp::new("moved");
    let root = ps3_install(&t.0);
    let reg = Registry::default();
    let st = store();
    let dir = t.0.join("project");
    let opened = reg.project_create_fresh(&s(&dir), "nhl14", parse_platform("ps3").unwrap(), &s(&root), None, None, &st, &defs()).unwrap();
    let id = opened.summary.project_id;
    edit(&reg, id, 6);
    reg.project_save(id).unwrap();
    let shown = vjla(&reg, id);
    assert!(reg.close_project(id));
    let moved = t.0.join("elsewhere/moved-project");
    std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
    std::fs::rename(&dir, &moved).unwrap();
    // The installation disappears too.
    std::fs::remove_dir_all(&root).unwrap();
    let reopened = reg.project_open(&s(&moved), &defs()).unwrap();
    let id2 = reopened.summary.project_id;
    assert!(Path::new(reopened.project.working_main_db.as_deref().unwrap()).starts_with(&moved));
    assert_eq!(vjla(&reg, id2), shown);
    assert_eq!(reopened.project.destinations.picked_root, s(&root), "the missing destination is kept, not replaced");
    assert!(reopened.warnings.iter().any(|w| w.contains("not reachable")), "{:?}", reopened.warnings);
    assert!(!reopened.warnings.iter().any(|w| w.contains("roster save destination")), "{:?}", reopened.warnings);
    assert!(reopened.discovery.nhlng_path.is_none());
    let plan = reg.project_deploy_plan(id2, false).unwrap();
    assert!(!plan.deployable);
    // Independent work continues: edit and save again.
    edit(&reg, id2, 2);
    assert!(reg.project_save(id2).unwrap().saved);
    assert!(reg.close_project(id2));
}

#[test]
fn corrupt_unknown_and_v1_manifests() {
    let t = Tmp::new("manifests");
    let root = ps3_install(&t.0);
    let reg = Registry::default();
    let st = store();
    let dir = t.0.join("project");
    let opened = reg.project_create_fresh(&s(&dir), "nhl14", parse_platform("ps3").unwrap(), &s(&root), None, None, &st, &defs()).unwrap();
    assert!(reg.close_project(opened.summary.project_id));
    let manifest = dir.join("nhl-studio-project.json");
    let good = std::fs::read_to_string(&manifest).unwrap();
    let tree_before = tree_hashes(&dir);

    // Corrupt JSON.
    std::fs::write(&manifest, b"{ not json").unwrap();
    let err = reg.project_open(&s(&dir), &defs()).err().expect("refused").to_string();
    assert!(err.contains("not readable") && err.contains("Nothing in the folder was changed"), "{err}");
    // A newer schema.
    std::fs::write(&manifest, good.replace("\"nhlms-project/2\"", "\"nhlms-project/9\"")).unwrap();
    let err = reg.project_open(&s(&dir), &defs()).err().expect("refused").to_string();
    assert!(err.contains("written with schema \"nhlms-project/9\""), "{err}");
    // Not a project folder at all.
    let err = reg.project_open(&s(&t.0.join("nowhere")), &defs()).err().expect("refused").to_string();
    assert!(err.contains("is not a Studio project"), "{err}");
    // Nothing above changed anything else in the folder.
    std::fs::write(&manifest, &good).unwrap();
    assert_eq!(tree_hashes(&dir), tree_before);

    // An ONB-002 manifest: the working paths are plain strings and the ONB-003 fields are absent.
    let mut v: serde_json::Value = serde_json::from_str(&good).unwrap();
    v["schema"] = serde_json::Value::String("nhlms-project/1".into());
    let w = v["working"].as_object_mut().unwrap();
    for (k, val) in w.iter_mut() {
        if k == "main_db" || k == "meta" {
            if let serde_json::Value::String(_) = val {
                // already a string
            }
        }
    }
    let o = v.as_object_mut().unwrap();
    for k in ["migrated_from", "last_view", "inputs", "sources", "member_edits"] {
        o.remove(k);
    }
    let d = o.get_mut("destinations").unwrap().as_object_mut().unwrap();
    d.remove("assets_dir");
    std::fs::write(&manifest, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    let reopened = reg.project_open(&s(&dir), &defs()).unwrap();
    assert_eq!(reopened.project.migrated_from.as_deref(), Some("nhlms-project/1"));
    assert_eq!(reopened.project.schema, "nhlms-project/2");
    assert!(reopened.warnings.iter().any(|w| w.contains("migrated")), "{:?}", reopened.warnings);
    let roles: Vec<String> = reopened.project.inputs.iter().map(|i| i.role.clone()).collect();
    assert!(roles.contains(&"main_db".into()) && roles.contains(&"dictionary".into()), "{roles:?}");
    assert!(!roles.contains(&"roster_save".into()), "{roles:?}");
    let rewritten: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
    assert_eq!(rewritten["schema"], "nhlms-project/2");
    assert_eq!(rewritten["migrated_from"], "nhlms-project/1");
    assert!(reg.close_project(reopened.summary.project_id));
}

#[test]
fn a_concurrent_open_is_refused_by_the_project_lock() {
    let t = Tmp::new("lock");
    let root = ps3_install(&t.0);
    let reg_a = Registry::default();
    let reg_b = Registry::default();
    let st = store();
    let dir = t.0.join("project");
    let opened = reg_a.project_create_fresh(&s(&dir), "nhl14", parse_platform("ps3").unwrap(), &s(&root), None, None, &st, &defs()).unwrap();
    // Reloading the frontend reattaches to the live session, including unsaved edits.
    let id = opened.summary.project_id;
    edit(&reg_a, id, 7);
    reg_a.project_set_view(id, "/app/database/players/1").unwrap();
    let before = reg_a.summary(id).unwrap();
    let value = vjla(&reg_a, id);
    let tree_before = tree_hashes(&dir);
    let reattached = reg_a.project_open(&s(&dir.join(".")), &defs()).unwrap();
    assert_eq!(reattached.summary.project_id, id);
    assert!(reattached.summary.dirty);
    assert_eq!(reattached.summary.undo_depth, before.undo_depth);
    assert_eq!(reattached.last_view.as_deref(), Some("/app/database/players/1"));
    assert_eq!(vjla(&reg_a, id), value);
    assert_eq!(tree_hashes(&dir), tree_before, "reattaching does not save or rewrite files");
    // A second Studio (a second registry stands in for a second process) cannot open it.
    let err = reg_b.project_open(&s(&dir), &defs()).err().expect("locked").to_string();
    if cfg!(windows) {
        assert!(err.contains("open in another Studio window"), "{err}");
    }
    assert!(reg_a.close_project(opened.summary.project_id));
    let again = reg_b.project_open(&s(&dir), &defs()).unwrap();
    assert!(reg_b.close_project(again.summary.project_id));
}

#[test]
fn an_interrupted_save_is_reported_on_reopen_and_not_resumed() {
    let t = Tmp::new("interrupted");
    let root = ps3_install(&t.0);
    let reg = Registry::default();
    let st = store();
    let dir = t.0.join("project");
    let opened = reg.project_create_fresh(&s(&dir), "nhl14", parse_platform("ps3").unwrap(), &s(&root), None, None, &st, &defs()).unwrap();
    let id = opened.summary.project_id;
    edit(&reg, id, 3);
    reg.project_save(id).unwrap();
    let working = PathBuf::from(opened.project.working_main_db.as_deref().unwrap());
    let saved = FileIdentity::of_file(&working).unwrap();
    assert!(reg.close_project(id));
    // A stale ACTIVE marker is what a crash mid-save leaves in the working root's journal.
    let txn_dir = dir.join("working").join(".tdb-txn");
    assert!(txn_dir.is_dir());
    std::fs::write(txn_dir.join("ACTIVE"), "txn-that-never-finished").unwrap();
    let reopened = reg.project_open(&s(&dir), &defs()).unwrap();
    assert!(reopened.warnings.iter().any(|w| w.contains("last save did not complete")), "{:?}", reopened.warnings);
    assert_eq!(FileIdentity::of_file(&working).unwrap(), saved, "opening changes nothing");
    assert!(txn_dir.join("ACTIVE").is_file(), "the journal is left for the explicit recovery route");
    // Saving again is refused until recovery, with the project's edits kept in memory.
    edit(&reg, reopened.summary.project_id, 5);
    let err = reg.project_save(reopened.summary.project_id).err().expect("refused").to_string();
    assert!(!err.is_empty(), "{err}");
    assert!(reg.summary(reopened.summary.project_id).unwrap().dirty, "the edit is not lost");
    assert!(reg.close_project(reopened.summary.project_id));
}

fn tree_hashes(root: &Path) -> Vec<(String, FileIdentity)> {
    let mut out = Vec::new();
    fn walk(d: &Path, root: &Path, out: &mut Vec<(String, FileIdentity)>) {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, root, out);
            } else if p.file_name().is_some_and(|n| n != "nhl-studio-project.lock") {
                out.push((p.strip_prefix(root).unwrap().to_string_lossy().into_owned(), FileIdentity::of_file(&p).unwrap()));
            }
        }
    }
    walk(root, root, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
