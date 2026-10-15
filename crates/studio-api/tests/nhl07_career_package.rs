use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }
#[test]
fn career_removals_build_install_restore_and_reject_changed_base() {
    let temp = tempfile::tempdir().unwrap(); let reg = Registry::default();
    let root = temp.path().join("project"); let defs = s(&repo("defs"));
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let id = reg.project_create_fresh(&s(&root), "nhl07", Platform::Xbox, "", None, None, &store, &defs).unwrap().summary.project_id;
    for table in ["tsSy", "OAqO"] { for row in (0..6).rev() { reg.delete_career_row(id, table, row).unwrap(); } }
    // A surviving row's signed statistic must replay through metadata decoding.
    reg.set_field(id, "tsSy", 0, "Uomt", studio_domain::FieldValue::Int(-12)).unwrap();
    reg.project_save(id).unwrap();
    let package = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto {
        name: "Career removal QA".into(), version: "1".into(), description: String::new(), include: vec!["cells".into()], exclude: vec![], platforms: vec!["xbox".into()], out_path: Some(s(&temp.path().join("test.nhlmod"))),
    }, Some(&store), &defs).unwrap();
    let review = reg.package_inspect(&package.path).unwrap();
    assert_eq!(review.changes.iter().filter(|c| c.after == "Removed").count(), 12);
    assert!(review.changes.iter().filter(|c| c.after == "Removed").all(|c| c.groups[0] == "Career statistics" && c.route.is_none()));
    let game = temp.path().join("game"); std::fs::create_dir_all(game.join("db")).unwrap();
    std::fs::write(game.join("default.xex"), b"XEX2").unwrap();
    for file in ["NHL07NG.db", "NHL07NG-meta.xml"] { std::fs::copy(repo(&format!("anchors/xbox360/nhl07/databases/{file}")), game.join("db").join(file)).unwrap(); }
    let db = game.join("db/NHL07NG.db"); let original = std::fs::read(&db).unwrap();
    let expected = std::fs::read(root.join("working/db/NHL07NG.db")).unwrap();
    let records = s(&temp.path().join("records"));
    let plan = reg.package_preflight(&package.path, "nhl07", Platform::Xbox, &s(&game), None, None, false, &records, &defs).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons.iter().take(5).collect::<Vec<_>>());
    let applied = reg.package_apply(plan.handle).unwrap(); assert_eq!(applied.state, "committed");
    assert!(std::fs::read(&db).unwrap() == expected);
    assert_eq!(reg.package_restore(&applied.record_dir).unwrap().state, "restored"); assert!(std::fs::read(&db).unwrap() == original);
    // A different row layout must fail, even with overwrite permission.
    std::fs::write(&db, &expected).unwrap();
    let plan = reg.package_preflight(&package.path, "nhl07", Platform::Xbox, &s(&game), None, None, true, &records, &defs).unwrap();
    assert!(!plan.installable); assert!(std::fs::read(&db).unwrap() == expected);
}

#[test]
fn extracted_assets_override_survives_reopening_with_a_linked_source() {
    let temp = tempfile::tempdir().unwrap(); let reg = Registry::default();
    let root = temp.path().join("project"); let defs = s(&repo("defs"));
    let game = temp.path().join("game"); let assets = temp.path().join("assets");
    std::fs::create_dir_all(game.join("db")).unwrap(); std::fs::create_dir_all(assets.join("rendering")).unwrap();
    std::fs::write(game.join("default.xex"), b"XEX2").unwrap();
    for file in ["NHL07NG.db", "NHL07NG-meta.xml"] { std::fs::copy(repo(&format!("anchors/xbox360/nhl07/databases/{file}")), game.join("db").join(file)).unwrap(); }
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let id = reg.project_create_fresh(&s(&root), "nhl07", Platform::Xbox, &s(&game), None, None, &store, &defs).unwrap().summary.project_id;
    let linked = reg.project_relink(id, None, None, None, Some(&s(&assets))).unwrap();
    assert_eq!(linked.discovery.install_dir, Some(s(&assets)));
    reg.close_project(id);
    let opened = reg.project_open(&s(&root), &defs).unwrap();
    assert_eq!(opened.discovery.install_dir, Some(s(&assets)));
    assert_eq!(opened.project.destinations.picked_root, s(&game));
}
