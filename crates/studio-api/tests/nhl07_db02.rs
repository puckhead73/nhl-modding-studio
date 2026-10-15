//! DB-02 qualification using disposable NHL07 projects, never live user files.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform, FieldValue};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }
fn fresh(reg: &Registry, root: &Path) -> u32 {
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    reg.project_create_fresh(&s(root), "nhl07", Platform::Xbox, "", None, None, &store, &s(&repo("defs"))).unwrap().summary.project_id
}
#[test]
fn db02_inventory() {
    let tmp = tempfile::tempdir().unwrap();
    let reg = Registry::default();
    let id = fresh(&reg, &tmp.path().join("project"));
    let data = serde_json::json!({
        "team": reg.team_view(id, 14).unwrap(),
        "lines": reg.lines_view(id, 14).unwrap(),
        "strategies": reg.team_strategies(id, 14).unwrap(),
        "draft": reg.team_draft_picks(id, 14).unwrap(),
        "summary": reg.summary(id).unwrap(),
    });
    std::fs::write(repo("target/db02-inventory.json"), serde_json::to_vec_pretty(&data).unwrap()).unwrap();
    reg.close_project(id);
}

#[test]
fn db02_team_identity_venue_strategies_package_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    let reg = Registry::default();
    let id = fresh(&reg, &root);
    let view = reg.team_view(id, 14).unwrap().unwrap();
    for field in ["RREB", "mCXg"] {
        assert!(!view.all_fields.iter().find(|f| f.studio_field == field).unwrap().editable);
        assert!(reg.set_field(id, "kOtt", view.record, field, FieldValue::Int(1)).is_err());
    }
    for (name, value) in [("Full Name", "Minnesota QA"), ("Short Name", "Minnesota Test"), ("Abbreviation", "QAM"), ("Arena City", "QA City")] {
        let cell = view.profile.sections.iter().flat_map(|s| &s.cells).find(|c| c.label == name).unwrap();
        let f = cell.field.as_ref().unwrap();
        reg.set_field(id, &f.studio_table, cell.record, &f.studio_field, FieldValue::Text(value.into())).unwrap();
    }
    let venue = view.profile.sections.iter().flat_map(|s| &s.cells).find(|c| c.label == "Home Arena").unwrap();
    let f = venue.field.as_ref().unwrap();
    assert!(f.enum_options.len() >= 30);
    let absent = (0..=127).find(|key| !f.enum_options.iter().any(|option| option.raw == *key)).unwrap();
    assert!(reg.set_field(id, &f.studio_table, venue.record, &f.studio_field, FieldValue::Int(absent)).is_err(), "unknown stock arena must be refused");
    reg.set_field(id, &f.studio_table, venue.record, &f.studio_field, FieldValue::Int(28)).unwrap();
    assert_eq!(reg.team_view(id, 14).unwrap().unwrap().arena.unwrap().name, reg.team_view(id, 28).unwrap().unwrap().arena.unwrap().name);
    assert!(reg.undo(id).unwrap());
    assert_eq!(reg.team_view(id, 14).unwrap().unwrap().arena.unwrap().city, "QA City");
    assert!(reg.redo(id).unwrap());
    let strategy = reg.team_strategies(id, 14).unwrap().unwrap();
    assert_eq!(strategy.tactics.len(), 9);
    for tactic in strategy.tactics {
        let f = tactic.field;
        let value = f.enum_options.iter().find(|o| Some(o.raw) != f.decoded).unwrap().raw;
        reg.set_field(id, &f.studio_table, strategy.record, &f.studio_field, FieldValue::Int(value)).unwrap();
    }
    let overview = reg.project_overview(id).unwrap();
    assert!(overview.notices.is_empty(), "{:?}", overview.notices);
    assert!(overview.changes.iter().filter(|c| c.id.starts_with("EeYo:")).all(|c| c.route.as_deref() == Some("/app/database/teams/14")));
    assert!(reg.project_save(id).unwrap().saved);
    let expected_team = serde_json::to_value(reg.team_view(id, 14).unwrap()).unwrap();
    let expected_strategies = serde_json::to_value(reg.team_strategies(id, 14).unwrap()).unwrap();
    reg.close_project(id);
    let defs = s(&repo("defs"));
    let id = reg.project_open(&s(&root), &defs).unwrap().summary.project_id;
    assert_eq!(serde_json::to_value(reg.team_view(id, 14).unwrap()).unwrap(), expected_team);
    assert_eq!(serde_json::to_value(reg.team_strategies(id, 14).unwrap()).unwrap(), expected_strategies);
    package_roundtrip(&reg, id, &root, tmp.path());
    reg.close_project(id);
}

fn package_roundtrip(reg: &Registry, id: u32, root: &Path, temp: &Path) {
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let defs = s(&repo("defs"));
    let package = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto {
        name: "DB02 qualification".into(), version: "1".into(), description: "Disposable fixture".into(),
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

#[test]
fn db02_roster_operations_are_atomic_and_packageable() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    let reg = Registry::default();
    let id = fresh(&reg, &root);
    let original = reg.lines_view(id, 14).unwrap().unwrap();
    let demitra = original.entries.iter().find(|e| e.player_id == Some(613)).unwrap();
    let slot = original.slot_fields.iter().find(|f| f.name == "l1c").unwrap().studio_field.clone();
    let occupant = original.entries.iter().find(|e| e.slots.contains(&slot)).unwrap();
    let replacement = original.entries.iter().find(|e| e.position_label != "G" && e.dressed == Some(true) && e.slot_record != occupant.slot_record).unwrap();
    let writes = vec![(occupant.slot_record, slot.clone(), 0), (replacement.slot_record, slot.clone(), 1)];
    let depth = reg.summary(id).unwrap().undo_depth;
    reg.set_team_roster(id, 14, &writes).unwrap();
    assert_eq!(reg.summary(id).unwrap().undo_depth, depth + 1);
    reg.undo(id).unwrap();
    assert_eq!(serde_json::to_value(reg.lines_view(id, 14).unwrap().unwrap()).unwrap(), serde_json::to_value(&original).unwrap());
    reg.redo(id).unwrap();
    let before_refusal = serde_json::to_value(reg.lines_view(id, 14).unwrap()).unwrap();
    let occupied_number = original.entries.iter().find(|e| e.player_id != Some(613) && e.jersey.is_some_and(|n| n > 0)).unwrap().jersey.unwrap();
    assert!(reg.set_jersey_number(id, 14, 613, occupied_number).is_err());
    assert!(reg.set_jersey_number(id, 14, 613, 100).is_err());
    assert_eq!(serde_json::to_value(reg.lines_view(id, 14).unwrap()).unwrap(), before_refusal);
    assert!(reg.retire_jersey(id, 14, 99).is_err());
    let goalie = original.entries.iter().find(|e| e.position_label == "G").unwrap();
    let foreign = reg.lines_view(id, 28).unwrap().unwrap().entries[0].slot_record;
    for invalid in [vec![(goalie.slot_record, slot.clone(), 1)], vec![(demitra.slot_record, "XSWT".into(), 0)], vec![(foreign, slot.clone(), 0)], vec![(occupant.slot_record, slot.clone(), 1)], vec![(99999, slot.clone(), 1)], vec![(demitra.slot_record, slot.clone(), 2)]] {
        assert!(reg.set_team_roster(id, 14, &invalid).is_err());
        assert_eq!(serde_json::to_value(reg.lines_view(id, 14).unwrap()).unwrap(), before_refusal);
    }
    // Demoting/promoting captaincy and scratching are each one history entry.
    let mut captain: Vec<_> = original.entries.iter().filter(|e| e.captaincy == Some(1) && e.player_id != Some(613)).map(|e| (e.slot_record, "mCcl".into(), 0)).collect();
    captain.push((demitra.slot_record, "mCcl".into(), 1));
    reg.set_team_roster(id, 14, &captain).unwrap();
    let before_scratch = serde_json::to_value(reg.lines_view(id, 14).unwrap()).unwrap();
    let depth = reg.summary(id).unwrap().undo_depth;
    reg.set_dressed(id, 14, 613, false).unwrap();
    let after = reg.lines_view(id, 14).unwrap().unwrap();
    let d = after.entries.iter().find(|e| e.player_id == Some(613)).unwrap();
    assert_eq!(d.dressed, Some(false)); assert!(d.slots.is_empty()); assert_eq!(d.captaincy, Some(0));
    assert_eq!(reg.summary(id).unwrap().undo_depth, depth + 1);
    reg.undo(id).unwrap();
    assert_eq!(serde_json::to_value(reg.lines_view(id, 14).unwrap()).unwrap(), before_scratch);
    reg.redo(id).unwrap();
    assert!(reg.set_team_roster(id, 14, &[(demitra.slot_record, "mCcl".into(), 1)]).is_err());
    assert!(reg.trade_draft_pick(id, 0, 28).is_err());
    reg.project_save(id).unwrap();
    package_roundtrip(&reg, id, &root, tmp.path());
    reg.close_project(id);
}

#[test]
fn db02_overview_revert_survives_save_and_preserves_unrelated_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    let reg = Registry::default();
    let id = fresh(&reg, &root);
    reg.set_jersey_number(id, 14, 613, 91).unwrap();
    let jersey = reg.project_overview(id).unwrap().changes.into_iter().find(|c| c.id.starts_with("ubPc:") && c.id.ends_with(":sVRt")).unwrap();
    let strategy = reg.team_strategies(id, 14).unwrap().unwrap();
    let cell = strategy.tactics.iter().find(|c| c.field.editable && c.field.enum_options.len() > 1).unwrap();
    let value = cell.field.enum_options.iter().find(|v| Some(v.raw) != cell.field.decoded).unwrap().raw;
    reg.set_field(id, "EeYo", strategy.record, &cell.field.studio_field, FieldValue::Int(value)).unwrap();
    reg.project_save(id).unwrap();
    reg.close_project(id);
    let id = reg.project_open(&s(&root), &s(&repo("defs"))).unwrap().summary.project_id;
    let depth = reg.summary(id).unwrap().undo_depth;
    reg.project_revert_change(id, &jersey.id).unwrap();
    assert_eq!(reg.summary(id).unwrap().undo_depth, depth + 1);
    let changes = reg.project_overview(id).unwrap().changes;
    assert!(!changes.iter().any(|c| c.id.ends_with(":sVRt")));
    assert!(changes.iter().any(|c| c.id.starts_with("EeYo:")));
    reg.project_save(id).unwrap();
    package_roundtrip(&reg, id, &root, tmp.path());
    reg.undo(id).unwrap();
    assert!(reg.project_overview(id).unwrap().changes.iter().any(|c| c.id == jersey.id));
    reg.redo(id).unwrap();
    assert!(reg.project_revert_change(id, &jersey.id).is_err());
    let strategy_id = reg.project_overview(id).unwrap().changes.into_iter().find(|c| c.id.starts_with("EeYo:")).unwrap().id;
    reg.project_revert_change(id, &strategy_id).unwrap();
    assert!(reg.project_overview(id).unwrap().changes.is_empty());
    reg.close_project(id);
    let id = reg.project_open(&s(&root), &s(&repo("defs"))).unwrap().summary.project_id;
    assert!(reg.project_overview(id).unwrap().changes.iter().any(|c| c.id.starts_with("EeYo:")));
    reg.project_revert_change(id, &strategy_id).unwrap();
    reg.project_save(id).unwrap();
    reg.close_project(id);
    let id = reg.project_open(&s(&root), &s(&repo("defs"))).unwrap().summary.project_id;
    assert!(reg.project_overview(id).unwrap().changes.is_empty());
    reg.close_project(id);
}

#[test]
fn db02_overview_revert_restores_linked_roster_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    let reg = Registry::default();
    let id = fresh(&reg, &root);
    let original = reg.lines_view(id, 14).unwrap().unwrap();
    let demitra = original.entries.iter().find(|e| e.player_id == Some(613)).unwrap();
    let captain = original.entries.iter().find(|e| e.captaincy == Some(1)).unwrap();
    reg.set_team_roster(id, 14, &[(captain.slot_record, "mCcl".into(), 0), (demitra.slot_record, "mCcl".into(), 1)]).unwrap();
    reg.project_revert_change(id, &format!("eGlu:{}:mCcl", demitra.slot_record)).unwrap();
    assert!(reg.project_overview(id).unwrap().changes.is_empty());
    reg.release_to_fa(id, 14, 613).unwrap();
    reg.project_revert_change(id, "free-agent:613").unwrap();
    assert!(reg.project_overview(id).unwrap().changes.is_empty(), "{}", serde_json::to_string(&reg.project_overview(id).unwrap().changes).unwrap());
    assert_eq!(serde_json::to_value(reg.lines_view(id, 14).unwrap().unwrap()).unwrap(), serde_json::to_value(&original).unwrap());
    reg.undo(id).unwrap();
    assert!(reg.project_overview(id).unwrap().changes.iter().any(|c| c.id == "free-agent:613"));
    reg.redo(id).unwrap();
    reg.close_project(id);
}

#[test]
fn db02_revert_line_slot_and_scratch_without_losing_other_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    let reg = Registry::default();
    let id = fresh(&reg, &root);
    let original = reg.lines_view(id, 14).unwrap().unwrap();
    let d = original.entries.iter().find(|e| e.player_id == Some(613)).unwrap();
    let slot = original.slot_fields.iter().find(|f| f.name == "l1c").unwrap().studio_field.clone();
    let other = original.entries.iter().find(|e| e.player_id != Some(613) && e.position_label == "C" && e.dressed == Some(true)).unwrap();
    reg.set_team_roster(id, 14, &[(d.slot_record, slot.clone(), 0), (other.slot_record, slot.clone(), 1)]).unwrap();
    reg.project_revert_change(id, &format!("eGlu:{}:{slot}", d.slot_record)).unwrap();
    assert!(reg.project_overview(id).unwrap().changes.is_empty());
    reg.set_dressed(id, 14, 613, false).unwrap();
    reg.project_revert_change(id, &format!("eGlu:{}:hSZj", d.slot_record)).unwrap();
    assert!(reg.project_overview(id).unwrap().changes.is_empty());
    reg.move_player(id, 613, 14, 28).unwrap();
    let change = reg.project_overview(id).unwrap().changes.into_iter().find(|c| c.id.starts_with("ubPc:") && c.id.ends_with(":dXSB")).unwrap();
    reg.project_revert_change(id, &change.id).unwrap();
    assert!(reg.project_overview(id).unwrap().changes.is_empty());
    reg.close_project(id);
}

#[test]
fn db02_revert_preserves_secondary_jerseys_and_reverts_trades_atomically() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    let reg = Registry::default();
    let id = fresh(&reg, &root);
    reg.set_jersey_number(id, 14, 613, 91).unwrap();
    reg.set_jersey_number(id, 48, 613, 90).unwrap();
    let secondary = reg.lines_view(id, 48).unwrap().unwrap().entries.into_iter().find(|e| e.player_id == Some(613)).unwrap();
    reg.project_revert_change(id, &format!("eGlu:{}:sVRt", secondary.slot_record)).unwrap();
    assert_eq!(reg.lines_view(id, 14).unwrap().unwrap().entries.iter().find(|e| e.player_id == Some(613)).unwrap().jersey, Some(91));
    let primary = reg.project_overview(id).unwrap().changes.into_iter().find(|c| c.id.starts_with("ubPc:") && c.id.ends_with(":sVRt")).unwrap().id;
    reg.project_revert_change(id, &primary).unwrap();
    assert!(reg.project_overview(id).unwrap().changes.is_empty());
    let partner = reg.lines_view(id, 28).unwrap().unwrap().entries.into_iter().find(|e| e.position_label == "C" && e.player_id.is_some()).unwrap().player_id.unwrap();
    reg.trade_slots(id, 14, 613, 28, partner).unwrap();
    let change = reg.project_overview(id).unwrap().changes.into_iter().find(|c| c.id.starts_with("ubPc:") && c.id.ends_with(":dXSB")).unwrap();
    let depth = reg.summary(id).unwrap().undo_depth;
    reg.project_revert_change(id, &change.id).unwrap();
    assert_eq!(reg.summary(id).unwrap().undo_depth, depth + 1);
    assert!(reg.project_overview(id).unwrap().changes.is_empty());
    reg.close_project(id);
}
