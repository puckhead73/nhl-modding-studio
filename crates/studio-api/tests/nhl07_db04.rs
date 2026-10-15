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

use studio_domain::schedule_edit::GameRow;
fn rows(v: &studio_api::dto::ScheduleViewDto) -> Vec<GameRow> {
    v.games
        .iter()
        .map(|r| GameRow {
            index: r.index,
            month: r.month,
            day: r.day,
            home_slot: r.home_slot,
            away_slot: r.away_slot,
            round: r.round,
            status: r.status,
        })
        .collect()
}
#[test]
fn calendar_add_remove_undo_save_build_install_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let reg = Registry::default();
    let root = tmp.path().join("project");
    let id = fresh(&reg, &root);
    let original = reg.schedule_view(id, "ksGb").unwrap().unwrap();
    let expected = rows(&original);
    let home = original.slot_map[0].team_id;
    let away = original.slot_map[1].team_id;
    let added = reg
        .team_schedule_edit(
            id,
            "ksGb",
            home,
            &original.fingerprint,
            None,
            false,
            Some(away),
            true,
            8,
            1,
            &expected,
        )
        .unwrap();
    assert_eq!(added.games.len(), original.games.len() + 1);
    assert!(
        reg.team_schedule_edit(
            id,
            "ksGb",
            home,
            &original.fingerprint,
            None,
            false,
            Some(away),
            true,
            8,
            2,
            &expected
        )
        .is_err(),
        "stale calendar refused"
    );
    assert!(reg.undo(id).unwrap());
    assert_eq!(
        rows(&reg.schedule_view(id, "ksGb").unwrap().unwrap()),
        expected
    );
    assert!(reg.redo(id).unwrap());
    let added_row = added
        .games
        .iter()
        .find(|r| r.month == 7 && r.day == 0)
        .unwrap()
        .record;
    let removed = reg
        .team_schedule_edit(
            id,
            "ksGb",
            home,
            &added.fingerprint,
            Some(added_row),
            true,
            None,
            true,
            8,
            1,
            &rows(&added),
        )
        .unwrap();
    assert_eq!(rows(&removed), expected);
    assert!(reg.undo(id).unwrap());
    let overview = reg.project_overview(id).unwrap();
    assert!(overview.notices.is_empty(), "{:?}", overview.notices);
    assert!(overview.changes.iter().any(|c| c.id == "schedule:ksGb"));
    reg.project_revert_change(id, "schedule:ksGb").unwrap();
    assert_eq!(
        rows(&reg.schedule_view(id, "ksGb").unwrap().unwrap()),
        expected
    );
    assert!(reg.undo(id).unwrap());
    assert!(reg.project_save(id).unwrap().saved);
    reg.close_project(id);
    let id = reg
        .project_open(&s(&root), &s(&repo("defs")))
        .unwrap()
        .summary
        .project_id;
    assert_eq!(
        rows(&reg.schedule_view(id, "ksGb").unwrap().unwrap()),
        rows(&added)
    );
    package_roundtrip(&reg, id, &root, tmp.path());
    reg.close_project(id);
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
fn membership_swap_preserves_other_opponents_and_packages() {
    let tmp = tempfile::tempdir().unwrap();
    let reg = Registry::default();
    let root = tmp.path().join("project");
    let id = fresh(&reg, &root);
    let nhl = reg.schedule_view(id, "ksGb").unwrap().unwrap();
    let del = reg.schedule_view(id, "vufv").unwrap().unwrap();
    let first = 14;
    let second = del.slot_map[0].team_id;
    let assignments = |v: &studio_api::dto::ScheduleViewDto| {
        v.games
            .iter()
            .map(|r| {
                (
                    r.month,
                    r.day,
                    v.slot_map
                        .iter()
                        .find(|s| s.slot == r.home_slot)
                        .unwrap()
                        .team_id,
                    v.slot_map
                        .iter()
                        .find(|s| s.slot == r.away_slot)
                        .unwrap()
                        .team_id,
                )
            })
            .collect::<Vec<_>>()
    };
    let original = assignments(&nhl);
    assert!(reg
        .set_team_membership(id, first, 4, Some(0), Some(0))
        .is_err());
    reg.swap_team_memberships(id, first, second).unwrap();
    let changed = reg.schedule_view(id, "ksGb").unwrap().unwrap();
    let intended: Vec<_> = original
        .iter()
        .map(|&(m, d, h, a)| {
            (
                m,
                d,
                if h == first { second } else { h },
                if a == first { second } else { a },
            )
        })
        .collect();
    assert_eq!(assignments(&changed), intended);
    assert!(reg.undo(id).unwrap());
    assert_eq!(
        assignments(&reg.schedule_view(id, "ksGb").unwrap().unwrap()),
        original
    );
    assert!(reg.redo(id).unwrap());
    reg.project_revert_change(id, "organization").unwrap();
    assert_eq!(
        assignments(&reg.schedule_view(id, "ksGb").unwrap().unwrap()),
        original
    );
    assert!(reg.undo(id).unwrap());
    reg.project_save(id).unwrap();
    package_roundtrip(&reg, id, &root, tmp.path());
    reg.close_project(id);
}
fn source_session() -> studio_domain::EditSession {
    let mut project = studio_domain::Project::load(
        repo("anchors/xbox360/nhl07/databases/NHL07NG.db"),
        Platform::Xbox,
        studio_domain::ProjectRole::Nhlng,
        repo("anchors/xbox360/nhl07/databases/NHL07NG-meta.xml"),
        repo("defs"),
    )
    .unwrap();
    project.set_game_id(Some("nhl07".into()));
    studio_domain::EditSession::begin(project)
}
#[test]
fn csv_identity_dates_conflicts_and_resizable_generation() {
    use studio_domain::schedule_edit::*;
    let session = source_session();
    let model = session.schedule_model("ksGb").unwrap();
    let mut quoted = model.clone();
    quoted.slots.entries[0].name = "Québec, \"Nordiques\"\nClub®".into();
    let text = export_csv(&quoted, &session.schedule_export_context());
    assert!(text.as_bytes().starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(text.contains("Club®"));
    assert!(!text.contains("Â®"));
    let doc = parse_import(
        &text.replace("\n", "\r\n"),
        ImportFormat::Csv,
    )
    .unwrap();
    let imported = plan_import(&model, &doc, &[], None).unwrap();
    assert!(
        imported.change.validation.ok(),
        "{:?}",
        imported.change.validation.errors
    );
    assert_eq!(imported.change.cells, 0);
    assert!(parse_import("month,day,home,away\n10,1,\"broken,MIN", ImportFormat::Csv).is_err());
    let other = session.schedule_model("qyyN").unwrap();
    assert!(!plan_import(&other, &doc, &[], None)
        .unwrap()
        .change
        .validation
        .ok());
    let mut duplicate = model.rows.clone();
    duplicate.push(duplicate[0].clone());
    assert!(!replacement_plan(&model, duplicate).unwrap().validation.ok());
    let mut leap = model.rows.clone();
    leap[0].month = 1;
    leap[0].day = 28;
    assert!(!replacement_plan(&model, leap).unwrap().validation.ok());
    let options = generation_options(&model);
    let ids = options
        .members
        .iter()
        .filter(|m| m.default)
        .map(|m| m.team_id)
        .collect();
    let config = GenerationConfig {
        participants: ids,
        rounds: 2,
        start_month: 9,
        start_day: 0,
        days_between_matchdays: 2,
        pair_quotas: None,
        odd_home: None,
        groups: None,
        odd_hosts: None,
    };
    let plan = plan_generation(&model, &config).unwrap();
    assert!(
        plan.change.validation.ok(),
        "{:?}",
        plan.change.validation.errors
    );
    assert_eq!(plan.generated_rows, 870);
    assert_eq!(plan.change.replacement.as_ref().unwrap().len(), 871);
    let mut quota = config.clone();
    let n = quota.participants.len();
    quota.pair_quotas = Some(
        (0..n)
            .map(|i| (0..n).map(|j| if i == j { 0 } else { 2 }).collect())
            .collect(),
    );
    let generated = plan_generation(&model, &quota).unwrap();
    assert!(
        generated.change.validation.ok(),
        "{:?}",
        generated.change.validation.errors
    );
    assert_eq!(generated.generated_rows, 870);
}

#[test]
fn removing_a_stock_game_survives_archive_and_release() {
    let tmp = tempfile::tempdir().unwrap();
    let reg = Registry::default();
    let root = tmp.path().join("project");
    let id = fresh(&reg, &root);
    let view = reg.schedule_view(id, "ksGb").unwrap().unwrap();
    let after = reg
        .team_schedule_edit(
            id,
            "ksGb",
            14,
            &view.fingerprint,
            Some(9),
            true,
            None,
            true,
            10,
            5,
            &rows(&view),
        )
        .unwrap();
    assert_eq!(after.games.len(), 1230);
    reg.project_save(id).unwrap();
    let archive = reg.project_archive(id, &s(tmp.path())).unwrap();
    let restored = tmp.path().join("imported");
    reg.project_import_archive(&archive.path, &s(&restored), None)
        .unwrap();
    let copy = reg
        .project_open(&s(&restored), &s(&repo("defs")))
        .unwrap()
        .summary
        .project_id;
    assert_eq!(
        rows(&reg.schedule_view(copy, "ksGb").unwrap().unwrap()),
        rows(&after)
    );
    reg.close_project(copy);
    package_roundtrip(&reg, id, &root, tmp.path());
    reg.close_project(id);
}
#[test]
fn every_source_variant_accepts_one_game_and_rejects_invalid_dates_without_partial_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let reg = Registry::default();
    let id = fresh(&reg, &tmp.path().join("project"));
    for table in ["vufv", "fBWa", "ADlP", "uRbu", "ksGb", "qyyN", "zcVp"] {
        let v = reg.schedule_view(id, table).unwrap().unwrap();
        let first = v.slot_map[0].team_id;
        let second = v.slot_map[1].team_id;
        assert!(reg
            .team_schedule_edit(
                id,
                table,
                first,
                &v.fingerprint,
                None,
                false,
                Some(second),
                true,
                2,
                30,
                &rows(&v)
            )
            .is_err());
        assert_eq!(
            rows(&reg.schedule_view(id, table).unwrap().unwrap()),
            rows(&v)
        );
        let changed = reg
            .team_schedule_edit(
                id,
                table,
                first,
                &v.fingerprint,
                None,
                false,
                Some(second),
                true,
                8,
                1,
                &rows(&v),
            )
            .unwrap();
        assert_eq!(changed.games.len(), v.games.len() + 1);
        assert!(reg.undo(id).unwrap());
    }
    reg.close_project(id);
}

#[test]
fn generated_season_applies_atomically_and_installs(){
 let tmp=tempfile::tempdir().unwrap();let reg=Registry::default();let root=tmp.path().join("project");let id=fresh(&reg,&root);
 let options=reg.schedule_generation_options(id,"ksGb").unwrap();
 let config=studio_api::dto::ScheduleGenerationConfigDto{participants:options.members.iter().filter(|m|m.default).map(|m|m.team_id).collect(),rounds:2,start_month:9,start_day:0,days_between_matchdays:2,pair_quotas:None,odd_home:None,groups:None,odd_hosts:None};
 let preview=reg.schedule_generate_preview(id,"ksGb",&config).unwrap();
 let result=reg.schedule_apply_review(id,&preview.change.ticket).unwrap();assert_eq!(result.view.games.len(),871);
 assert!(reg.undo(id).unwrap());assert_eq!(reg.schedule_view(id,"ksGb").unwrap().unwrap().games.len(),1231);assert!(reg.redo(id).unwrap());reg.project_save(id).unwrap();
 package_roundtrip(&reg,id,&root,tmp.path());reg.close_project(id);
}

#[test]
fn league_review_uses_title_dictionary_without_inventing_european_bindings() {
    let tmp = tempfile::tempdir().unwrap();
    let reg = Registry::default();
    let id = fresh(&reg, &tmp.path().join("project"));
    let catalog = reg.list_leagues(id).unwrap();
    assert!(catalog.dictionary.is_some());
    for (code, label, key) in [(0, "NHL", "NHL"), (1, "National", "NATIONAL")] {
        let league = catalog.leagues.iter().find(|l| l.code == code).unwrap();
        assert_eq!(league.display, label);
        assert_eq!(league.dictionary_key.as_deref(), Some(key));
        assert_eq!(league.dictionary_key_evidence.as_deref(), Some("hypothesis"));
        assert!(league.evidence.contains("dictionary text for key"));
    }
    for code in [2, 3, 4, 5] {
        let league = catalog.leagues.iter().find(|l| l.code == code).unwrap();
        assert!(league.dictionary_key.is_none());
    }
    for (key, text) in [("WesternConference", "Western Conference"), ("EasternConference", "Eastern Conference"), ("NortheastDivision", "Northeast Division"), ("AtlanticDivision", "Atlantic Division"), ("SoutheastDivision", "Southeast Division"), ("CentralDivision", "Central Division"), ("NorthwestDivision", "Northwest Division"), ("PacificDivision", "Pacific Division")] {
        let texts = reg.dictionary_texts(id, &[key.to_string()]).unwrap();
        assert_eq!(texts[0].as_deref(), Some(text));
    }
    let options = reg.team_membership_options(id, 1).unwrap();
    let nhl = options.groups_by_league.iter().find(|g| g.league_code == 0).unwrap();
    assert!(nhl.conferences.iter().any(|c| c.label == "Eastern Conference"));
    assert!(nhl.divisions.iter().any(|c| c.label == "Pacific Division"));
    assert!(!reg.undo(id).unwrap(), "read-only label lookups create no edits");
    reg.dictionary_rename_set(id, "NHL", "NATIONAL").unwrap();
    assert_eq!(reg.dictionary_texts(id, &["NHL".into()]).unwrap()[0].as_deref(), Some("National"));
    assert!(reg.dictionary_rename_clear(id).unwrap());
    assert_eq!(reg.dictionary_texts(id, &["NHL".into(), "CHTM_LEAGUE_SWEDISH".into()]).unwrap(), vec![Some("NHL".into()), None]);
}
