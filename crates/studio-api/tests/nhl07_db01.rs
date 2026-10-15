//! DB-01 qualification through the production API; anchors and user projects stay read-only.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform, FieldValue};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
fn db01_inventory() {
    let tmp = tempfile::tempdir().unwrap();
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let reg = Registry::default();
    let opened = reg.project_create_fresh(&s(tmp.path().join("project").as_path()), "nhl07", Platform::Xbox, "", None, None, &store, &s(&repo("defs"))).unwrap();
    let id = opened.summary.project_id;
    let players = reg.list_players(id).unwrap();
    let bytes = std::fs::read(repo("anchors/xbox360/nhl07/databases/NHL07NG.db")).unwrap();
    let pool = studio_domain::explicit_free_agents(&bytes).unwrap().unwrap();
    assert_eq!(pool.len(), 74);
    assert!(!pool.contains(&613));
    for pid in pool.iter().take(5) {
        let v = reg.player_view(id, *pid).unwrap().unwrap();
        assert_eq!(v.all_fields.iter().find(|f| f.studio_field == "dXSB").unwrap().decoded, Some(0));
        assert_eq!(v.header.primary_team_id, None);
    }    let player = reg.player_view(id, 613).unwrap().unwrap();
    std::fs::write(repo("target/db01-player.json"), serde_json::to_vec_pretty(&player).unwrap()).unwrap();
    std::fs::write(repo("target/db01-summary.json"), serde_json::to_vec_pretty(&opened.summary).unwrap()).unwrap();
    std::fs::write(repo("target/db01-players.json"), serde_json::to_vec_pretty(&players).unwrap()).unwrap();
    reg.close_project(id);
}

#[test]
fn db01_fields_ratings_and_persistence() {
    let tmp = tempfile::tempdir().unwrap();
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let reg = Registry::default();
    let root = tmp.path().join("project");
    let defs = s(&repo("defs"));
    let opened = reg.project_create_fresh(&s(&root), "nhl07", Platform::Xbox, "", None, None, &store, &defs).unwrap();
    let id = opened.summary.project_id;
    assert!(opened.summary.capabilities.draft_year && opened.summary.capabilities.draft_pick);
    let view = reg.player_view(id, 613).unwrap().unwrap();
    assert!(view.profile.sections.iter().any(|s| s.title == "Contract Data" && s.cells.iter().any(|c| c.label == "Contract Amount")));
    let names = ["firstname", "lastname", "height", "weight", "year", "month", "day", "birthplace", "state",
        "intlcountry", "position", "handedness", "contractdollars", "contractlength", "contractis2way", "rookie", "yeardrafted", "positiondrafted"];
    let mut expected = Vec::new();
    for name in names {
        let f = view.all_fields.iter().find(|f| f.name == name).unwrap();
        assert!(f.editable, "{name}");
        let value = if f.is_string { FieldValue::Text(format!("QA{}", name.chars().take(3).collect::<String>())) }
            else { FieldValue::Int(match name {
                "position" => 1, "handedness" | "rookie" | "contractis2way" => 1,
                "month" => 1, "day" => 15, "contractdollars" => 200, "contractlength" => 3,
                _ => f.decoded.unwrap() + 1,
            }) };
        reg.set_field(id, &f.studio_table, view.bio_record, &f.studio_field, value).unwrap_or_else(|e| panic!("{name}: {e}"));
        let after = reg.player_view(id, 613).unwrap().unwrap();
        let changed = after.all_fields.iter().find(|a| a.name == name).unwrap();
        assert_ne!(changed.display, f.display, "{name}");
        assert!(reg.undo(id).unwrap());
        assert_eq!(reg.player_view(id, 613).unwrap().unwrap().all_fields.iter().find(|a| a.name == name).unwrap().display, f.display);
        assert!(reg.redo(id).unwrap());
        expected.push((name.to_owned(), changed.display.clone()));
    }
    // A valid February birthday must build even though the original day (29) is
    // invalid in that month/year. Replay validates the complete resulting date.
    let month = view.all_fields.iter().find(|f| f.name == "month").unwrap();
    reg.set_field(id, "ubPc", view.bio_record, &month.studio_field, FieldValue::Int(2)).unwrap();
    let display = reg.player_view(id, 613).unwrap().unwrap().all_fields.iter().find(|f| f.name == "month").unwrap().display.clone();
    expected.iter_mut().find(|(n, _)| n == "month").unwrap().1 = display;
    let v = reg.player_view(id, 613).unwrap().unwrap();
    assert_eq!(v.data.cap_hit.as_deref(), Some("$5,000,000"));
    assert_eq!(v.all_fields.iter().find(|f| f.name == "contractdollars").unwrap().edit_scale, Some(25000.0));
    // Every exposed rating for a skater and a goalie uses the same field path as the UI.
    let goalie = reg.list_players(id).unwrap().into_iter().find(|p| p.position_label == "G").unwrap().player_id;
    let mut ratings = Vec::new();
    for pid in [613, goalie] {
        let v = reg.player_view(id, pid).unwrap().unwrap();
        for a in &v.attributes.flat {
            let Some(f) = &a.field else { continue };
            if !a.is_rating || !f.editable { continue }
            let raw = if f.decoded == Some(40) { 41 } else { 40 };
            reg.set_field(id, &f.studio_table, v.ratings_record.unwrap(), &f.studio_field, FieldValue::Int(raw)).unwrap();
            ratings.push((pid, f.name.clone(), raw));
        }
        if let Some(f) = v.attributes.style.as_ref().and_then(|a| a.field.as_ref()) {
            let alt = f.enum_options.iter().find(|o| Some(o.raw) != f.decoded).unwrap();
            reg.set_field(id, &f.studio_table, v.ratings_record.unwrap(), &f.studio_field, FieldValue::Int(alt.raw)).unwrap();
        }
    }
    assert!(ratings.len() >= 30);
    assert!(reg.project_save(id).unwrap().saved);
    assert!(!reg.project_overview(id).unwrap().changes.is_empty());
    reg.close_project(id);
    let id = reg.project_open(&s(&root), &defs).unwrap().summary.project_id;
    let after = reg.player_view(id, 613).unwrap().unwrap();
    for (name, display) in expected { assert_eq!(after.all_fields.iter().find(|f| f.name == name).unwrap().display, display, "{name}"); }
    for (pid, name, raw) in ratings {
        let v = reg.player_view(id, pid).unwrap().unwrap();
        assert_eq!(v.ratings.iter().find(|f| f.name == name).unwrap().decoded, Some(raw));
    }
    // All accepted edits must survive the actual build/install pipeline, not just saving.
    let package = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto {
        name: "DB01 qualification".into(), version: "1".into(), description: "Isolated verification".into(),
        include: vec!["cells".into()], exclude: vec![], platforms: vec!["xbox".into()],
        out_path: Some(s(&tmp.path().join("test.nhlmod"))),
    }, Some(&store), &defs).unwrap();
    let game = tmp.path().join("game");
    std::fs::create_dir_all(game.join("db")).unwrap();
    std::fs::write(game.join("default.xex"), b"XEX2").unwrap();
    for file in ["NHL07NG.db", "NHL07NG-meta.xml"] {
        std::fs::copy(repo(&format!("anchors/xbox360/nhl07/databases/{file}")), game.join("db").join(file)).unwrap();
    }
    let original = std::fs::read(game.join("db/NHL07NG.db")).unwrap();
    let plan = reg.package_preflight(&package.path, "nhl07", Platform::Xbox, &s(&game), None, None, false,
        &s(&tmp.path().join("records")), &defs).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let result = reg.package_apply(plan.handle).unwrap();
    assert_eq!(std::fs::read(game.join("db/NHL07NG.db")).unwrap(), std::fs::read(root.join("working/db/NHL07NG.db")).unwrap());
    reg.package_restore(&result.record_dir).unwrap();
    assert_eq!(std::fs::read(game.join("db/NHL07NG.db")).unwrap(), original);
    reg.close_project(id);
}

#[test]
fn db01_memberships_and_refusals_are_atomic() {
    let tmp = tempfile::tempdir().unwrap();
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let reg = Registry::default();
    let defs = s(&repo("defs"));
    let root = tmp.path().join("project");
    let id = reg.project_create_fresh(&s(&root), "nhl07", Platform::Xbox, "", None, None, &store, &defs).unwrap().summary.project_id;
    let original = serde_json::to_value(reg.player_view(id, 613).unwrap().unwrap()).unwrap();
    // The international row can be removed/restored without moving the primary club.
    reg.remove_membership(id, 48, 613).unwrap();
    let v = reg.player_view(id, 613).unwrap().unwrap();
    assert_eq!(v.header.primary_team_id, Some(14));
    assert!(!v.memberships.iter().any(|m| m.team_id == 48));
    reg.add_membership(id, 613, 48).unwrap();
    assert_eq!(reg.player_view(id, 613).unwrap().unwrap().header.primary_team_id, Some(14));
    reg.undo(id).unwrap(); reg.undo(id).unwrap();
    assert_eq!(serde_json::to_value(reg.player_view(id, 613).unwrap().unwrap()).unwrap(), original);
    let bio = reg.player_view(id, 613).unwrap().unwrap().bio_record;
    reg.set_field(id, "ubPc", bio, "dXSB", FieldValue::Int(28)).unwrap();
    reg.set_field(id, "ubPc", bio, "sVRt", FieldValue::Int(90)).unwrap();
    let v = reg.player_view(id, 613).unwrap().unwrap();
    assert_eq!(v.header.primary_team_id, Some(28));
    assert!(v.memberships.iter().any(|m| m.team_id == 48 && m.jersey == Some(38)));
    reg.project_save(id).unwrap();
    reg.close_project(id);
    let id = reg.project_open(&s(&root), &defs).unwrap().summary.project_id;
    let v = reg.player_view(id, 613).unwrap().unwrap();
    assert_eq!(v.header.primary_team_id, Some(28));
    assert_eq!(v.header.jersey_number, Some(90));
    let before = serde_json::to_value(&v).unwrap();
    let history = reg.summary(id).unwrap().undo_depth;
    for result in [reg.create_player(id, 28, 0).map(|_| ()), reg.clone_player(id, 613, 28).map(|_| ()),
        reg.delete_player(id, 613).map(|_| ()),
        reg.sign_from_fa(id, 613, 14).map(|_| ())] {
        assert!(result.is_err());
        assert!(!result.unwrap_err().to_string().contains("UnknownTable"));
    }
    let last = v.all_fields.iter().find(|f| f.name == "lastname").unwrap();
    assert!(reg.set_field(id, "ubPc", v.bio_record, &last.studio_field, FieldValue::Text("X".repeat(200))).is_err());
    let speed = v.ratings.iter().find(|f| f.name == "p_speed").unwrap();
    assert!(reg.set_field(id, &v.ratings_table, v.ratings_record.unwrap(), &speed.studio_field, FieldValue::Int(64)).is_err());
    assert!(reg.set_field(id, &v.ratings_table, v.ratings_record.unwrap(), &speed.studio_field, FieldValue::Int(50)).is_err());
    for (name, raw) in [("month", 0), ("month", 13), ("day", 0), ("month", 2)] {
        let f = v.all_fields.iter().find(|f| f.name == name).unwrap();
        assert!(reg.set_field(id, "ubPc", v.bio_record, &f.studio_field, FieldValue::Int(raw)).is_err(), "{name}: {raw}");
    }
    assert!(reg.add_membership(id, 613, 48).is_err());
    assert!(reg.move_player(id, 613, 28, 9999).is_err());
    assert_eq!(reg.summary(id).unwrap().undo_depth, history);
    assert_eq!(serde_json::to_value(reg.player_view(id, 613).unwrap().unwrap()).unwrap(), before);
    reg.close_project(id);
}

#[test]
fn db01_free_agency_build_install_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let reg = Registry::default();
    let defs = s(&repo("defs"));
    let root = tmp.path().join("project");
    let id = reg.project_create_fresh(&s(&root), "nhl07", Platform::Xbox, "", None, None, &store, &defs).unwrap().summary.project_id;
    let initial_pool: Vec<_> = reg.list_free_agents(id).unwrap().iter().map(|p| p.player_id).collect();
    assert_eq!(initial_pool.len(), 74);
    let original = serde_json::to_value(reg.player_view(id, 613).unwrap().unwrap()).unwrap();
    assert!(reg.release_to_fa(id, 48, 613).is_err()); // Never release a national membership as a club.
    reg.release_to_fa(id, 14, 613).unwrap();
    let released = reg.player_view(id, 613).unwrap().unwrap();
    assert_eq!(released.header.primary_team_id, None);
    assert!(released.memberships.iter().any(|m| m.team_id == 48 && m.jersey == Some(38)));
    assert!(!released.memberships.iter().any(|m| m.team_id == 14));
    assert_eq!(reg.list_free_agents(id).unwrap().len(), 75);
    let history = reg.summary(id).unwrap().undo_depth;
    assert!(reg.release_to_fa(id, 14, 613).is_err());
    assert!(reg.sign_from_fa(id, 613, 9999).is_err());
    assert_eq!(reg.summary(id).unwrap().undo_depth, history);
    reg.undo(id).unwrap();
    assert_eq!(serde_json::to_value(reg.player_view(id, 613).unwrap().unwrap()).unwrap(), original);
    reg.redo(id).unwrap();
    // An international membership must not prevent a club from signing a free agent.
    assert!(reg.sign_from_fa(id, 613, 28).is_err()); // A full destination must leave the pool unchanged.
    reg.sign_from_fa(id, 613, 14).unwrap();
    assert_eq!(reg.list_free_agents(id).unwrap().len(), 74);
    assert_eq!(reg.player_view(id, 613).unwrap().unwrap().header.primary_team_id, Some(14));
    reg.set_jersey_number(id, 14, 613, 90).unwrap();
    reg.release_to_fa(id, 14, 613).unwrap();
    reg.project_save(id).unwrap();
    let released_bytes = std::fs::read(root.join("working/db/NHL07NG.db")).unwrap();
    let release_package = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto {
        name: "DB01 release".into(), version: "1".into(), description: "Qualification".into(),
        include: vec!["cells".into()], exclude: vec![], platforms: vec!["xbox".into()],
        out_path: Some(s(&tmp.path().join("release.nhlmod"))),
    }, Some(&store), &defs).unwrap();
    // Sign an original free agent into Minnesota's newly available slot. Both insertion and
    // removal must survive packaging, including when the resulting row count is unchanged.
    let signed = initial_pool[0];
    reg.sign_from_fa(id, signed, 14).unwrap();
    reg.project_save(id).unwrap();
    reg.close_project(id);
    let id = reg.project_open(&s(&root), &defs).unwrap().summary.project_id;
    let pool = reg.list_free_agents(id).unwrap();
    assert_eq!(pool.len(), 74);
    assert!(pool.iter().any(|p| p.player_id == 613));
    assert!(!pool.iter().any(|p| p.player_id == signed));
    let package = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto {
        name: "DB01 free agency".into(), version: "1".into(), description: "Qualification".into(),
        include: vec!["cells".into()], exclude: vec![], platforms: vec!["xbox".into()],
        out_path: Some(s(&tmp.path().join("fa.nhlmod"))),
    }, Some(&store), &defs).unwrap();
    let game = tmp.path().join("game");
    std::fs::create_dir_all(game.join("db")).unwrap();
    std::fs::write(game.join("default.xex"), b"XEX2").unwrap();
    for file in ["NHL07NG.db", "NHL07NG-meta.xml"] {
        std::fs::copy(repo(&format!("anchors/xbox360/nhl07/databases/{file}")), game.join("db").join(file)).unwrap();
    }
    let original_bytes = std::fs::read(game.join("db/NHL07NG.db")).unwrap();
    let records = s(&tmp.path().join("records"));
    let final_bytes = std::fs::read(root.join("working/db/NHL07NG.db")).unwrap();
    for (path, expected) in [(&release_package.path, &released_bytes), (&package.path, &final_bytes)] {
    let plan = reg.package_preflight(path, "nhl07", Platform::Xbox, &s(&game), None, None, false, &records, &defs).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let installed = reg.package_apply(plan.handle).unwrap();
    let actual = std::fs::read(game.join("db/NHL07NG.db")).unwrap();
    assert_eq!(studio_domain::explicit_free_agents(&actual).unwrap(), studio_domain::explicit_free_agents(expected).unwrap());
    let a = tdb_core::TdbFile::parse(&actual).unwrap();
    let b = tdb_core::TdbFile::parse(&expected).unwrap();
    // Allocation history may differ after growing then shrinking a table. Compare every
    // live record, rather than requiring identical unused capacity and physical offsets.
    for at in &a.tables {
        let bt = b.table_canonical(&at.id.canonical(a.endian)).unwrap();
        assert_eq!(at.current_records, bt.current_records);
        assert_eq!(at.record_len_bytes, bt.record_len_bytes);
        let len = at.current_records as usize * at.record_len_bytes as usize;
        assert_eq!(studio_domain::baselines::sha256_hex(&actual[at.record_block..at.record_block+len]),
            studio_domain::baselines::sha256_hex(&expected[bt.record_block..bt.record_block+len]), "{}", at.id.canonical(a.endian));
    }
    reg.package_restore(&installed.record_dir).unwrap();
    assert_eq!(std::fs::read(game.join("db/NHL07NG.db")).unwrap(), original_bytes);
    }
    // A different recipient pool must never be silently overwritten, even when the
    // caller permits ordinary cell conflicts. Preflight cannot alter the live file.
    let project = studio_domain::Project::load(game.join("db/NHL07NG.db"), Platform::Xbox, studio_domain::ProjectRole::Nhlng,
        game.join("db/NHL07NG-meta.xml"), repo("defs")).unwrap();
    let mut session = studio_domain::EditSession::begin(project);
    let pool = studio_domain::explicit_free_agents(session.bytes()).unwrap().unwrap();
    session.set_free_agent_pool(&pool[1..]).unwrap();
    let altered = session.save().unwrap();
    std::fs::write(game.join("db/NHL07NG.db"), &altered).unwrap();
    let plan = reg.package_preflight(&release_package.path, "nhl07", Platform::Xbox, &s(&game), None, None, true, &records, &defs).unwrap();
    assert!(!plan.installable);
    assert!(plan.reasons.iter().any(|r| r.contains("free-agent list")));
    assert_eq!(std::fs::read(game.join("db/NHL07NG.db")).unwrap(), altered);
    reg.close_project(id);
}
