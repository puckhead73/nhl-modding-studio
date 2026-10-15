//! Cumulative overview regression: original → live survives saving and reopening.
use std::path::{Path, PathBuf};
use studio_api::{dto::ProjectDetailsDto, Registry};
use studio_domain::{BaselineStore, Platform};
fn repo(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into()
}

#[test]
fn overview_asset_and_archive_changes_are_net_differences() {
    use studio_domain::{project_store::{ProjectStore, MaterializedArchive}, tdb_txn::FileIdentity};
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("Project");
    let reg = Registry::default();
    let inventory = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let defs = s(&repo("defs"));
    let id = reg.project_create_fresh(&s(&dir), "nhl07", Platform::Xbox, "", None, None, &inventory, &defs).unwrap().summary.project_id;
    reg.close_project(id);
    let mut store = ProjectStore::open(&dir).unwrap();
    std::fs::create_dir_all(store.overlay_dir()).unwrap();
    std::fs::write(store.overlay_dir().join("logo.bin"), b"changed").unwrap();
    let original = FileIdentity::of_bytes(b"original");
    store.record_staged_file("logo.bin", "test", Some(&original)).unwrap();
    std::fs::write(store.working_dir().join("assets.big"), b"changed").unwrap();
    store.manifest.materialized_archives.push(MaterializedArchive { relative: "assets.big".into(), destination: "unused".into(), prior: (&original).into(), at: "test".into() });
    store.save_manifest().unwrap();
    let id = reg.project_open(&s(&dir), &defs).unwrap().summary.project_id;
    let overview = reg.project_overview(id).unwrap();
    assert_eq!(overview.changes.len(), 2);
    assert!(overview.changes.iter().all(|c| c.route.as_deref() == Some("/app/assets/files")));
    reg.close_project(id);
    std::fs::write(store.overlay_dir().join("logo.bin"), b"original").unwrap();
    std::fs::write(store.working_dir().join("assets.big"), b"original").unwrap();
    let id = reg.project_open(&s(&dir), &defs).unwrap().summary.project_id;
    assert!(reg.project_overview(id).unwrap().changes.is_empty());
    reg.close_project(id);
}

#[test]
fn overview_metadata_and_cumulative_edits_survive_save_reopen_and_revert() {
    let root = std::env::temp_dir().join(format!(
        "overview-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dir = root.join("Original folder name");
    let reg = Registry::default();
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let defs = s(&repo("defs"));
    let opened = reg
        .project_create_fresh(
            &s(&dir),
            "nhl07",
            Platform::Xbox,
            "",
            None,
            None,
            &store,
            &defs,
        )
        .unwrap();
    let id = opened.summary.project_id;
    let initial = reg.project_overview(id).unwrap();
    assert!(initial.changes.is_empty());
    assert_eq!(initial.details.name, "Original folder name");
    reg.project_set_details(
        id,
        ProjectDetailsDto {
            name: "Shared roster".into(),
            version: "0.2".into(),
            author: "QA".into(),
            description: "Portable project details".into(),
        },
    )
    .unwrap();
    reg.move_player(id, 613, 14, 28).unwrap();
    reg.set_jersey_number(id, 28, 613, 90).unwrap();
    let review = reg.project_overview(id).unwrap();
    assert!(review.dirty);
    assert_eq!(review.changes.len(), 4);
    assert!(review
        .changes
        .iter()
        .all(|c| c.route.as_deref() == Some("/app/database/players/613?tab=player-data")));
    assert!(review
        .changes
        .iter()
        .all(|c| c.groups.iter().any(|g| g.contains("Demitra"))));
    assert!(review
        .changes
        .iter()
        .all(|c| c.groups.iter().any(|g| g.contains("Vancouver"))));
    let ids: Vec<_> = review.changes.iter().map(|c| c.id.clone()).collect();
    reg.project_save(id).unwrap();
    assert!(!reg.project_overview(id).unwrap().dirty);
    assert_eq!(reg.project_overview(id).unwrap().changes.len(), 4);
    reg.close_project(id);
    let opened = reg.project_open(&s(&dir), &defs).unwrap();
    let id = opened.summary.project_id;
    let review = reg.project_overview(id).unwrap();
    assert_eq!(review.details.author, "QA");
    assert_eq!(review.details.name, "Shared roster");
    assert_eq!(
        review
            .changes
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    // Returning to vanilla removes cumulative edits even though the session is now dirty.
    reg.move_player(id, 613, 28, 14).unwrap();
    reg.set_jersey_number(id, 14, 613, 38).unwrap();
    let review = reg.project_overview(id).unwrap();
    assert!(review.dirty);
    assert!(review.changes.is_empty());
    assert!(dir.is_dir());
    reg.close_project(id);
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}
