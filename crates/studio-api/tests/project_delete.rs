//! Project deletion only exercises disposable fixtures.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }
#[test]
fn deletes_only_closed_project_and_keeps_its_siblings() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    let sibling = tmp.path().join("game-source.xex");
    std::fs::write(&sibling, b"untouched").unwrap();
    let reg = Registry::default();
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let id = reg.project_create_fresh(&s(&root), "nhl07", Platform::Xbox, "", None, None, &store, &s(&repo("defs"))).unwrap().summary.project_id;
    assert!(reg.project_delete(&s(&root)).unwrap_err().to_string().contains("Close"));
    assert!(root.join("nhl-studio-project.json").is_file());
    reg.close_project(id);
    let lock = studio_domain::project_store::ProjectLock::acquire(&root).unwrap();
    #[cfg(windows)]
    assert!(reg.project_delete(&s(&root)).is_err());
    drop(lock);
    reg.project_delete(&s(&root)).unwrap();
    assert!(!root.exists());
    assert_eq!(std::fs::read(&sibling).unwrap(), b"untouched");
}
#[test]
fn refuses_unrecognized_folders_without_deleting_files() {
    let tmp = tempfile::tempdir().unwrap();
    let sentinel = tmp.path().join("keep.txt");
    std::fs::write(&sentinel, b"keep").unwrap();
    assert!(Registry::default().project_delete(&s(tmp.path())).is_err());
    assert!(sentinel.exists());
    assert!(Registry::default().project_delete(".").is_err());
}
