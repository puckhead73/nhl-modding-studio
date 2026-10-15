use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform, FieldValue};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }
fn fresh(reg: &Registry, root: &Path) -> u32 {
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    reg.project_create_fresh(&s(root), "nhl07", Platform::Xbox, "", None, None, &store, &s(&repo("defs"))).unwrap().summary.project_id
}
fn package_roundtrip(reg: &Registry, id: u32, root: &Path, temp: &Path) {
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let defs = s(&repo("defs"));
    let package = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto {
        name: "DB06 qualification".into(), version: "1".into(), description: "Disposable fixture".into(),
        include: vec!["cells".into()], exclude: vec![], platforms: vec!["xbox".into()], out_path: Some(s(&temp.join("test.nhlmod"))),
    }, Some(&store), &defs).unwrap();
    let game = temp.join("game");
    std::fs::create_dir_all(game.join("db")).unwrap();
    std::fs::write(game.join("default.xex"), b"XEX2").unwrap();
    for file in ["NHL07NG.db", "NHL07NG-meta.xml"] {
        std::fs::copy(repo(&format!("anchors/xbox360/nhl07/databases/{file}")), game.join("db").join(file)).unwrap();
    }
    let original = std::fs::read(game.join("db/NHL07NG.db")).unwrap();
    let plan = reg.package_preflight(&package.path, "nhl07", Platform::Xbox, &s(&game), None, None, false, &s(&temp.join("records")), &defs).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed", "{:?}", applied.detail);
    let installed = std::fs::read(game.join("db/NHL07NG.db")).unwrap();
    let expected = std::fs::read(root.join("working/db/NHL07NG.db")).unwrap();
    assert!(installed == expected, "installed database must exactly match saved edits");
    assert_eq!(reg.package_restore(&applied.record_dir).unwrap().state, "restored");
    assert!(std::fs::read(game.join("db/NHL07NG.db")).unwrap() == original);
}

fn edit(reg:&Registry,id:u32,arena:u64,name:&str,value:FieldValue) {
 let a=reg.arena_view(id,arena).unwrap().unwrap();
 let f=a.all_fields.iter().find(|f|f.name==name).unwrap();
 reg.set_field(id,&f.studio_table,a.record,&f.studio_field,value).unwrap();
}
#[test]
fn arena_shared_identity_assignments_history_archive_and_release() {
 let temp=tempfile::tempdir().unwrap();let reg=Registry::default();let root=temp.path().join("project");let id=fresh(&reg,&root);
 let original=reg.arena_view(id,14).unwrap().unwrap();
 assert_eq!(original.all_fields.len(),33);
 assert!(original.all_fields.iter().find(|f|f.name=="arenatype").unwrap().enum_options.iter().any(|v|v.label=="International ice"));
 for name in ["goalhorn","periodhorn","jumbotron","icelogo","cityaudioid"] {
  let f=original.all_fields.iter().find(|f|f.name==name).unwrap();assert!(f.enum_options.len()>1);assert!(f.enum_options.iter().any(|v|v.label.contains("option")));
 }
 // Give St. Louis Minnesota's venue: the actual stock row, not the selector number, is shared.
 let team=reg.team_view(id,24).unwrap().unwrap();let cell=team.profile.sections.iter().flat_map(|s|&s.cells).find(|c|c.label=="Home Arena").unwrap();let f=cell.field.as_ref().unwrap();
 reg.set_field(id,&f.studio_table,cell.record,&f.studio_field,FieldValue::Int(14)).unwrap();
 edit(&reg,id,14,"arenaname",FieldValue::Text("DB06 Review Arena".into()));
 edit(&reg,id,14,"cityname",FieldValue::Text("St. Paul QA".into()));
 edit(&reg,id,14,"maxattendance",FieldValue::Int(19000));
 edit(&reg,id,14,"avgattendance",FieldValue::Int(95));
 edit(&reg,id,14,"arenatype",FieldValue::Int(1));
 for f in &original.all_fields {
  if studio_domain::arena_edit::is_assignment(&f.name) {
   if let Some(option)=f.enum_options.iter().find(|o|Some(o.raw)!=f.decoded) {edit(&reg,id,14,&f.name,FieldValue::Int(option.raw));}
  }
 }
 for name in ["boardadshown","icelogoshown"] {edit(&reg,id,14,name,FieldValue::Int(1));}
 let a=reg.arena_view(id,14).unwrap().unwrap();assert_eq!(a.teams.len(),2);assert_eq!(reg.arena_view(id,24).unwrap().unwrap().record,a.record);
 assert_eq!(reg.list_arenas(id).unwrap().iter().find(|a|a.arena_id==14).unwrap().team_count,2);
 assert_eq!(reg.team_view(id,24).unwrap().unwrap().arena.unwrap().name,"DB06 Review Arena");
 let overview=reg.project_overview(id).unwrap();assert!(overview.notices.is_empty(),"{:?}",overview.notices);
 assert!(overview.changes.iter().filter(|c|c.id.starts_with("cubz:")).all(|c|c.route.as_deref()==Some("/app/database/arenas/14")));
 let change=overview.changes.iter().find(|c|c.id.ends_with(":bAFc")).unwrap();reg.project_revert_change(id,&change.id).unwrap();
 assert_ne!(reg.team_view(id,24).unwrap().unwrap().arena.unwrap().name,"DB06 Review Arena");assert!(reg.undo(id).unwrap());assert!(reg.redo(id).unwrap());assert!(reg.undo(id).unwrap());
 let expected=serde_json::to_value(reg.arena_view(id,14).unwrap()).unwrap();
 assert!(reg.project_save(id).unwrap().saved);reg.close_project(id);let id=reg.project_open(&s(&root),&s(&repo("defs"))).unwrap().summary.project_id;
 assert_eq!(serde_json::to_value(reg.arena_view(id,14).unwrap()).unwrap(),expected);
 package_roundtrip(&reg,id,&root,temp.path());
 let archive=reg.project_archive(id,&s(&temp.path().join("archives"))).unwrap();let imported=temp.path().join("imported");
 reg.project_import_archive(&archive.path,&s(&imported),None).unwrap();let imported_id=reg.project_open(&s(&imported),&s(&repo("defs"))).unwrap().summary.project_id;
 assert_eq!(serde_json::to_value(reg.arena_view(imported_id,14).unwrap()).unwrap(),expected);
 reg.close_project(imported_id);reg.close_project(id);
}
#[test]
fn invalid_arena_edits_are_atomic_and_keys_are_protected() {
 let temp=tempfile::tempdir().unwrap();let reg=Registry::default();let id=fresh(&reg,&temp.path().join("p"));let a=reg.arena_view(id,14).unwrap().unwrap();
 for (name,value) in [("index",1),("maxattendance",0),("avgattendance",101),("arenatype",2),("icequality",1),("crowdtype",1),("crease",1)] {
  let f=a.all_fields.iter().find(|f|f.name==name).unwrap();assert!(reg.set_field(id,&f.studio_table,a.record,&f.studio_field,FieldValue::Int(value)).is_err(),"{name}");
 }
 assert!(reg.set_field(id,"StEO",14,"RREB",FieldValue::Int(1)).is_err());
 let f=a.all_fields.iter().find(|f|f.name=="arenaname").unwrap();assert!(reg.set_field(id,&f.studio_table,a.record,&f.studio_field,FieldValue::Text("x".repeat(1000))).is_err());
 assert!(reg.project_overview(id).unwrap().changes.is_empty());assert_eq!(serde_json::to_value(reg.arena_view(id,14).unwrap().unwrap()).unwrap(),serde_json::to_value(a).unwrap());
 reg.close_project(id);
}
