//! DB03 checks only disposable projects and game installations.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform, FieldValue};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }
fn fresh(reg: &Registry, root: &Path) -> u32 {
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    reg.project_create_fresh(&s(root), "nhl07", Platform::Xbox, "", None, None, &store, &s(&repo("defs"))).unwrap().summary.project_id
}
fn assignments(reg: &Registry, id: u32, player: u64) -> serde_json::Value {
    serde_json::to_value(reg.player_assignments(id, player).unwrap()).unwrap()
}
fn value(reg: &Registry, id: u32, player: u64, field: &str) -> i64 {
    reg.player_view(id, player).unwrap().unwrap().all_fields.iter().find(|f| f.studio_field == field).unwrap().decoded.unwrap()
}

#[test]
fn role_catalogue_refusal_and_unknown_preservation() {
    let tmp = tempfile::tempdir().unwrap(); let reg = Registry::default();
    let id = fresh(&reg, &tmp.path().join("project"));
    let skater = reg.player_assignments(id, 613).unwrap();
    let goalie = reg.player_assignments(id, 734).unwrap();
    assert!(skater.iter().any(|a| a.key == "visor"));
    assert!(!skater.iter().any(|a| a.key == "pads"));
    assert!(goalie.iter().any(|a| a.key == "pads"));
    assert!(!goalie.iter().any(|a| a.key == "visor"));
    assert!(skater.iter().all(|a| a.availability == "Not verified in game source"));
    let before = assignments(&reg, id, 613);
    assert!(reg.set_player_assignment(id, 613, "pads", "15:0").is_err());
    assert!(reg.set_player_assignment(id, 613, "stick", "9999").is_err());
    assert!(reg.set_player_assignment(id, 613, "head", "404").is_err());
    assert_eq!(before, assignments(&reg, id, 613));
    // Advanced/imported values survive cataloguing without being advertised as stock choices.
    let p = reg.player_view(id, 613).unwrap().unwrap();
    reg.set_field(id, "ubPc", p.bio_record, "pPaD", FieldValue::Int(16000)).unwrap();
    let catalog = reg.player_assignments(id, 613).unwrap();
    let head = catalog.iter().find(|a| a.key == "head").unwrap();
    assert!(head.options.iter().any(|o| o.key == head.current && o.users == 0));
    assert_eq!(value(&reg, id, 613, "pPaD"), 16000);
    reg.close_project(id);
}

#[test]
fn linked_assignments_undo_overview_and_selective_revert() {
    let tmp = tempfile::tempdir().unwrap(); let reg = Registry::default();
    let id = fresh(&reg, &tmp.path().join("project"));
    let before = assignments(&reg, id, 613);
    reg.set_player_assignment(id, 613, "head", "404:1:0:0").unwrap();
    assert_eq!(value(&reg, id, 613, "pPaD"), 404);
    assert_eq!(value(&reg, id, 613, "KTDQ"), 1);
    assert!(reg.undo(id).unwrap()); assert_eq!(assignments(&reg, id, 613), before);
    assert!(reg.redo(id).unwrap());
    reg.set_player_assignment(id, 613, "stick", "2").unwrap();
    let overview = reg.project_overview(id).unwrap();
    assert!(overview.notices.is_empty(), "{:?}", overview.notices);
    assert!(overview.changes.iter().all(|c| c.route.as_deref() == Some("/app/database/players/613?tab=equipment")));
    let head = overview.changes.iter().find(|c| c.label == "Head").unwrap();
    reg.project_revert_change(id, &head.id).unwrap();
    for (f,v) in [("pPaD",208),("KTDQ",4),("CBfK",2),("Zief",2),("wXFx",2)] { assert_eq!(value(&reg,id,613,f), v); }
    reg.set_player_assignment(id, 613, "audio", "404:370").unwrap();
    let overview = reg.project_overview(id).unwrap();
    let audio = overview.changes.iter().find(|c| c.label == "Default commentary name").unwrap();
    reg.project_revert_change(id, &audio.id).unwrap();
    assert_eq!(value(&reg,id,613,"oazN"),208); assert_eq!(value(&reg,id,613,"KqtL"),188);
    assert_eq!(value(&reg,id,613,"wXFx"),2);
    let goalie = reg.player_view(id,734).unwrap().unwrap();
    reg.set_field(id,"ubPc",goalie.bio_record,"gGPi",FieldValue::Int(1)).unwrap();
    reg.set_player_assignment(id,734,"pads","15:0").unwrap();
    assert_eq!(value(&reg,id,734,"gGPi"),0);
    assert_eq!(value(&reg,id,734,"OGav"),15);
    assert!(reg.undo(id).unwrap());
    assert_eq!(value(&reg,id,734,"gGPi"),1);
    assert_eq!(value(&reg,id,734,"OGav"),4);
    reg.close_project(id);
}

#[test]
fn every_exposed_assignment_roundtrips_without_changing_other_player_data() {
    let tmp = tempfile::tempdir().unwrap(); let reg = Registry::default();
    let id = fresh(&reg,&tmp.path().join("project"));
    for player in [613,734] {
        let original = serde_json::to_value(reg.player_view(id,player).unwrap()).unwrap();
        for assignment in reg.player_assignments(id,player).unwrap() {
            let Some(choice) = assignment.options.iter().find(|o| o.key != assignment.current && o.users > 0) else { continue; };
            reg.set_player_assignment(id,player,&assignment.key,&choice.key).unwrap();
            assert_eq!(reg.player_assignments(id,player).unwrap().iter().find(|a| a.key == assignment.key).unwrap().current,choice.key);
            assert!(reg.undo(id).unwrap());
            assert_eq!(original,serde_json::to_value(reg.player_view(id,player).unwrap()).unwrap(),"{}",assignment.key);
        }
    }
    assert!(reg.project_overview(id).unwrap().changes.is_empty()); reg.close_project(id);
}

#[test]
fn source_availability_distinguishes_unextracted_missing_and_present() {
    let tmp = tempfile::tempdir().unwrap(); let reg = Registry::default();
    let id = fresh(&reg, &tmp.path().join("project"));
    let assets = tmp.path().join("assets"); std::fs::create_dir_all(&assets).unwrap();
    reg.project_relink(id,None,None,None,Some(&s(&assets))).unwrap();
    let current = |key: &str| reg.player_assignments(id,613).unwrap().into_iter().find(|a| a.key == key).unwrap();
    assert_eq!(current("head").availability,"Not verified in game source");
    let relative = "rendering/player/player_404/head_404.rx2";
    std::fs::write(assets.join("data3.big.nhlms-extracted.json"), serde_json::to_vec(&serde_json::json!({"members":[{"name":relative,"outcome":"written"}]})).unwrap()).unwrap();
    assert!(reg.set_player_assignment(id,613,"head","404:1:0:0").err().unwrap().to_string().contains("missing"));
    assert_eq!(value(&reg,id,613,"pPaD"),208);
    std::fs::create_dir_all(assets.join(relative).parent().unwrap()).unwrap();
    std::fs::write(assets.join(relative), b"fixture").unwrap();
    reg.set_player_assignment(id,613,"head","404:1:0:0").unwrap();
    assert_eq!(current("head").availability,"Available in game source");
    std::fs::remove_file(assets.join(relative)).unwrap();
    assert_eq!(current("head").availability,"Missing from extracted game source");
    assert_eq!(current("audio").availability,"Not verified in game source");
    reg.close_project(id);
}

#[test]
fn save_archive_build_install_and_restore() {
    let tmp = tempfile::tempdir().unwrap(); let reg = Registry::default(); let root = tmp.path().join("project");
    let id = fresh(&reg, &root);
    let untouched = serde_json::to_value(reg.player_view(id,792).unwrap()).unwrap();
    for (player, field, choice) in [(613,"head","404:1:0:0"),(613,"portrait","404"),(613,"audio","404:370"),(613,"stick","2"),(734,"pads","15:0"),(734,"trapper","15:0"),(734,"blocker","15:0")] {
        reg.set_player_assignment(id,player,field,choice).unwrap();
    }
    assert_eq!(untouched, serde_json::to_value(reg.player_view(id,792).unwrap()).unwrap());
    assert!(reg.project_save(id).unwrap().saved);
    let expected = assignments(&reg,id,613); let expected_goalie = assignments(&reg,id,734);
    let archive = reg.project_archive(id,&s(tmp.path())).unwrap();
    let imported = tmp.path().join("imported"); reg.project_import_archive(&archive.path,&s(&imported),None).unwrap();
    let imported_id = reg.project_open(&s(&imported), &s(&repo("defs"))).unwrap().summary.project_id;
    assert_eq!(expected,assignments(&reg,imported_id,613)); assert_eq!(expected_goalie,assignments(&reg,imported_id,734));
    reg.close_project(imported_id); reg.close_project(id);
    let id = reg.project_open(&s(&root), &s(&repo("defs"))).unwrap().summary.project_id;
    assert_eq!(expected,assignments(&reg,id,613)); assert_eq!(expected_goalie,assignments(&reg,id,734));
    package_roundtrip(&reg,id,&root,tmp.path()); reg.close_project(id);
}
fn package_roundtrip(reg: &Registry, id: u32, root: &Path, temp: &Path) {
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let defs = s(&repo("defs"));
    let package = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto {
        name: "DB03 qualification".into(), version: "1".into(), description: "Disposable fixture".into(),
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



