//! DB04 uses disposable project and installation fixtures only.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform};
fn repo(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn fresh(reg: &Registry, root: &Path) -> u32 {
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    reg.project_create_fresh(
        &s(root),
        "nhl07",
        Platform::Xbox,
        "",
        None,
        None,
        &store,
        &s(&repo("defs")),
    )
    .unwrap()
    .summary
    .project_id
}

fn package_roundtrip(reg: &Registry, id: u32, root: &Path, temp: &Path) {
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let defs = s(&repo("defs"));
    let package = reg
        .package_build(
            id,
            &studio_api::dto::PackageBuildRequestDto {
                name: "DB04 qualification".into(),
                version: "1".into(),
                description: "Disposable fixture".into(),
                include: vec!["cells".into()],
                exclude: vec![],
                platforms: vec!["xbox".into()],
                out_path: Some(s(&temp.join("test.nhlmod"))),
            },
            Some(&store),
            &defs,
        )
        .unwrap();
    let game = temp.join("game");
    std::fs::create_dir_all(game.join("db")).unwrap();
    std::fs::write(game.join("default.xex"), b"XEX2").unwrap();
    for file in ["NHL07NG.db", "NHL07NG-meta.xml"] {
        std::fs::copy(
            repo(&format!("anchors/xbox360/nhl07/databases/{file}")),
            game.join("db").join(file),
        )
        .unwrap();
    }
    let original = std::fs::read(game.join("db/NHL07NG.db")).unwrap();
    let plan = reg
        .package_preflight(
            &package.path,
            "nhl07",
            Platform::Xbox,
            &s(&game),
            None,
            None,
            false,
            &s(&temp.join("records")),
            &defs,
        )
        .unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed", "{:?}", applied.detail);
    let installed = std::fs::read(game.join("db/NHL07NG.db")).unwrap();
    let expected = std::fs::read(root.join("working/db/NHL07NG.db")).unwrap();
    assert!(
        installed == expected,
        "installed database must exactly match saved edits"
    );
    assert_eq!(
        reg.package_restore(&applied.record_dir).unwrap().state,
        "restored"
    );
    assert!(std::fs::read(game.join("db/NHL07NG.db")).unwrap() == original);
}

#[test]
fn nhl07_art_ids_swap_stock_rows_undo_and_install() {
 let tmp=tempfile::tempdir().unwrap(); let reg=Registry::default();let root=tmp.path().join("project");let id=fresh(&reg,&root);
 let original=reg.team_artid_options(id).unwrap();
 assert_eq!(original.len(),reg.list_teams(id).unwrap().len());
 let a=original.iter().find(|r| r.0==1).unwrap();let b=original.iter().find(|r| r.0==2).unwrap();
 assert_ne!(a.1,b.1);
 let before=std::fs::read(root.join("working/db/NHL07NG.db")).unwrap();
 let swap=reg.team_artid_swap(id,a.0,b.0).unwrap();
 assert!(swap.outcome.edits >= 2); assert!(swap.tables.iter().any(|(name,n)|name=="stockteamtable" && *n==2));
 assert_eq!(reg.summary(id).unwrap().undo_depth,1);
 let changed=reg.team_artid_options(id).unwrap();
 assert_eq!(changed.iter().find(|r|r.0==a.0).unwrap().1,b.1);
 assert_eq!(changed.iter().find(|r|r.0==b.0).unwrap().1,a.1);
 for row in original.iter().filter(|r|r.0!=a.0&&r.0!=b.0){assert!(changed.contains(row));}
 assert!(reg.undo(id).unwrap());assert_eq!(reg.team_artid_options(id).unwrap(),original);
 reg.project_save(id).unwrap();assert_eq!(std::fs::read(root.join("working/db/NHL07NG.db")).unwrap(),before);
 assert!(reg.redo(id).unwrap());reg.project_save(id).unwrap();
 package_roundtrip(&reg,id,&root,tmp.path());
 reg.close_project(id);
}

#[test]
fn art_id_package_rejects_unpaired_assignment_without_partial_edits() {
 use studio_domain::{EditSession,FieldValue,Project,ProjectRole};
 let tmp=tempfile::tempdir().unwrap();let reg=Registry::default();let id=fresh(&reg,&tmp.path().join("project"));
 let view=reg.team_view(id,1).unwrap().unwrap();
 let cell=view.profile.sections.iter().flat_map(|s|&s.cells).find(|c|c.label=="Art ID").unwrap();let field=cell.field.as_ref().unwrap();
 let options=reg.team_artid_options(id).unwrap();let other=options.iter().find(|r|r.0==2).unwrap().1;
 let path=repo("anchors/xbox360/nhl07/databases/NHL07NG.db");let original=std::fs::read(&path).unwrap();
 let mut session=EditSession::begin(Project::load(&path,Platform::Xbox,ProjectRole::Nhlng,repo("anchors/xbox360/nhl07/databases/NHL07NG-meta.xml"),repo("defs")).unwrap());
 let error=session.set_package_fields(&[(field.studio_table.clone(),cell.record,field.studio_field.clone(),FieldValue::Int(other as i64))]).unwrap_err();
 assert!(error.to_string().contains("both sides"),"{error}");assert_eq!(session.save().unwrap(),original);
 reg.close_project(id);
}
