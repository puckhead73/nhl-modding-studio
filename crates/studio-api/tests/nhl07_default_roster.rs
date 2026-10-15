//! NHL07 project edits and portable package installation through the production API.
use std::path::{Path, PathBuf};
use studio_api::{Registry, dto::PackageBuildRequestDto};
use studio_domain::{BaselineStore, Platform};

fn repo(path: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path) }
fn shown(path: &Path) -> String { path.to_string_lossy().into_owned() }

/// Pin fresh-project anchors to the independently recorded original disc, not to
/// the user's mutable installation or merely the inventory's own hashes.
#[test]
fn nhl07_anchor_identity_and_database_roundtrip() {
    use studio_domain::{EditSession, Project, ProjectRole};
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let target = store.target("nhl07", Platform::Xbox).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo("anchors/xbox360/nhl07/game-files.json")).unwrap()).unwrap();
    assert_eq!(manifest["game_id"], "nhl07");
    assert_eq!(manifest["platform"], "xbox");
    assert_eq!(manifest["region"], "USA");
    for (role, path) in [("main_db", "db/NHL07NG.db"), ("meta", "db/NHL07NG-meta.xml"),
        ("dictionary", "fe/loc/nhl_eng_us.db")] {
        let component = target.component(role).unwrap();
        let bytes = store.read_verified(component).unwrap();
        let disc = manifest["files"].as_array().unwrap().iter().find(|f| f["path"] == path).unwrap();
        assert_eq!(disc["sha256"].as_str().unwrap(), studio_domain::baselines::sha256_hex(&bytes), "{role}");
        assert_eq!(disc["size"].as_u64().unwrap(), bytes.len() as u64, "{role}");
    }
    // The dictionary's companion is bundled, but is not a loose file on this disc.
    // Validate its inventory identity and parser compatibility without claiming disc provenance.
    for (data, meta, role) in [("main_db", "meta", ProjectRole::Nhlng),
        ("dictionary", "dictionary_meta", ProjectRole::Localization)] {
        let db = target.component(data).unwrap();
        let schema = target.component(meta).unwrap();
        let original = store.read_verified(db).unwrap();
        store.read_verified(schema).unwrap();
        let project = Project::load(store.component_path(db), Platform::Xbox, role,
            store.component_path(schema), repo("defs")).unwrap();
        project.validate_metadata_schema().unwrap();
        assert_eq!(EditSession::begin(project).save().unwrap(), original, "no-op roundtrip: {data}");
    }
}
fn membership(reg: &Registry, id: u32, team: u64, number: u64) {
    let p = reg.player_view(id, 613).unwrap().unwrap();
    assert!(p.memberships.iter().any(|m| m.team_id == team && m.jersey == Some(number)));
    assert!(p.memberships.iter().any(|m| m.team_id == 48 && m.jersey == Some(38)));
}

#[test]
fn nhl07_import_metadata_and_optional_input_boundaries() {
    use studio_api::dto::ImportSourcesDto;
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input");
    std::fs::create_dir(&input).unwrap();
    let db = input.join("NHL07NG.db");
    let meta = input.join("NHL07NG-meta.xml");
    let original = std::fs::read(repo("anchors/xbox360/nhl07/databases/NHL07NG.db")).unwrap();
    std::fs::write(&db, &original).unwrap();
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let reg = Registry::default();
    let defs = shown(&repo("defs"));
    let mut sources = ImportSourcesDto { main_db: Some(shown(&db)), ..Default::default() };
    let review = reg.project_import_review("nhl07", Platform::Xbox, &sources, Some(&store), &defs).unwrap();
    assert!(review.ok);
    assert!(review.meta_source.unwrap().starts_with("bundled"));
    assert!(!reg.project_import_review("nhl07", Platform::Xbox, &sources, None, &defs).unwrap().ok);
    // A valid XML file for a different database must not silently admit the import.
    std::fs::copy(repo("anchors/xbox360/nhl07/databases/nhl_eng_us-meta.xml"), &meta).unwrap();
    let review = reg.project_import_review("nhl07", Platform::Xbox, &sources, Some(&store), &defs).unwrap();
    assert!(!review.ok);
    assert!(review.items.iter().any(|i| i.detail.contains("metadata does not match")));
    let dest = temp.path().join("project");
    assert!(reg.project_create_import(&shown(&dest), "nhl07", Platform::Xbox, &sources, "", None, None, Some(&store), &defs).is_err());
    assert!(!dest.exists());
    std::fs::write(&meta, b"not XML").unwrap();
    assert!(!reg.project_import_review("nhl07", Platform::Xbox, &sources, Some(&store), &defs).unwrap().ok);
    std::fs::remove_file(&meta).unwrap();
    let bad_dict = input.join("bad.db");
    std::fs::write(&bad_dict, b"not a database").unwrap();
    sources.dictionary = Some(shown(&bad_dict));
    assert!(!reg.project_import_review("nhl07", Platform::Xbox, &sources, Some(&store), &defs).unwrap().ok);
    sources.dictionary = None;
    let opened = reg.project_create_import(&shown(&dest), "nhl07", Platform::Xbox, &sources, "", None, None, Some(&store), &defs).unwrap();
    assert!(opened.project.working_dictionary.is_none());
    assert!(opened.project.working_roster_save.is_none());
    assert!(!reg.list_teams(opened.summary.project_id).unwrap().is_empty());
    membership(&reg, opened.summary.project_id, 14, 38);
    reg.set_jersey_number(opened.summary.project_id, 14, 613, 90).unwrap();
    assert!(reg.project_save(opened.summary.project_id).unwrap().saved);
    reg.close_project(opened.summary.project_id);
    let working_meta = dest.join("working/db/NHL07NG-meta.xml");
    let good_meta = std::fs::read(&working_meta).unwrap();
    std::fs::copy(repo("anchors/xbox360/nhl07/databases/nhl_eng_us-meta.xml"), &working_meta).unwrap();
    assert!(reg.project_open(&shown(&dest), &defs).is_err());
    std::fs::write(&working_meta, good_meta).unwrap();
    let reopened = reg.project_open(&shown(&dest), &defs).unwrap();
    membership(&reg, reopened.summary.project_id, 14, 90);
    reg.close_project(reopened.summary.project_id);
    assert_eq!(std::fs::read(&db).unwrap(), original);
    // A save container or corrupt database cannot masquerade as the primary database.
    for bytes in [b"bad database".to_vec(), std::fs::read(repo("anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1")).unwrap()] {
        std::fs::write(&db, bytes).unwrap();
        assert!(!reg.project_import_review("nhl07", Platform::Xbox, &sources, Some(&store), &defs).unwrap().ok);
    }
    sources.main_db = Some(shown(&repo("anchors/xbox360/nhl07/databases/nhl_eng_us.db")));
    sources.meta = Some(shown(&repo("anchors/xbox360/nhl07/databases/nhl_eng_us-meta.xml")));
    assert!(!reg.project_import_review("nhl07", Platform::Xbox, &sources, Some(&store), &defs).unwrap().ok);
}

/// Execute the six acceptance scenarios with disposable copies of the review inputs.
#[test]
fn ms03_six_step_verification_with_prepared_inputs() {
    use studio_api::dto::ImportSourcesDto;
    let temp = tempfile::tempdir().unwrap();
    let fixtures = temp.path().join("inputs");
    for dir in ["database-only", "matching-metadata", "wrong-metadata", "invalid-database", "optional-dictionary"] {
        std::fs::create_dir_all(fixtures.join(dir)).unwrap();
    }
    for dir in ["database-only", "matching-metadata", "wrong-metadata"] {
        std::fs::copy(repo("anchors/xbox360/nhl07/databases/NHL07NG.db"), fixtures.join(dir).join("NHL07NG.db")).unwrap();
    }
    for (source, dest) in [("NHL07NG-meta.xml", "matching-metadata/NHL07NG-meta.xml"),
        ("nhl_eng_us-meta.xml", "wrong-metadata/NHL07NG-meta.xml"), ("nhl_eng_us.db", "optional-dictionary/nhl_eng_us.db")] {
        std::fs::copy(repo(&format!("anchors/xbox360/nhl07/databases/{source}")), fixtures.join(dest)).unwrap();
    }
    std::fs::write(fixtures.join("invalid-database/NHL07NG.db"), b"Invalid database test fixture").unwrap();
    std::fs::write(fixtures.join("optional-dictionary/invalid.db"), b"Invalid dictionary test fixture").unwrap();
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let reg = Registry::default();
    let defs = shown(&repo("defs"));
    let mut sources = ImportSourcesDto { main_db: Some(shown(&fixtures.join("database-only/NHL07NG.db"))), ..Default::default() };
    let create = |name: &str, input: &ImportSourcesDto| reg.project_create_import(
        &shown(&temp.path().join(name)), "nhl07", Platform::Xbox, input, "", None, None, Some(&store), &defs);
    let review = |input: &ImportSourcesDto| reg.project_import_review("nhl07", Platform::Xbox, input, Some(&store), &defs).unwrap();
    let opened = create("DB Only", &sources).unwrap();
    let id = opened.summary.project_id;
    assert!(!reg.list_players(id).unwrap().is_empty());
    assert!(!reg.list_teams(id).unwrap().is_empty());
    membership(&reg, id, 14, 38);
    assert!(opened.project.working_dictionary.is_none());
    eprintln!("MS03-1 PASS: bundled metadata, no dictionary/assets/save required");
    reg.set_jersey_number(id, 14, 613, 90).unwrap();
    reg.project_save(id).unwrap();
    reg.close_project(id);
    let id = reg.project_open(&shown(&temp.path().join("DB Only")), &defs).unwrap().summary.project_id;
    membership(&reg, id, 14, 90);
    reg.set_jersey_number(id, 14, 613, 38).unwrap();
    reg.project_save(id).unwrap();
    reg.close_project(id);
    let id = reg.project_open(&shown(&temp.path().join("DB Only")), &defs).unwrap().summary.project_id;
    membership(&reg, id, 14, 38);
    reg.close_project(id);
    eprintln!("MS03-2 PASS: 38 -> 90 -> save/reopen -> 38 -> save/reopen");
    sources.main_db = Some(shown(&fixtures.join("wrong-metadata/NHL07NG.db")));
    assert!(!review(&sources).ok);
    assert!(create("Metadata", &sources).is_err());
    assert!(!temp.path().join("Metadata").exists());
    sources.main_db = Some(shown(&fixtures.join("matching-metadata/NHL07NG.db")));
    assert_eq!(review(&sources).meta_source.as_deref(), Some("sibling"));
    reg.close_project(create("Metadata", &sources).unwrap().summary.project_id);
    eprintln!("MS03-3 PASS: wrong metadata rejected, matching sibling retry succeeds");
    sources.main_db = Some(shown(&fixtures.join("invalid-database/NHL07NG.db")));
    assert!(!review(&sources).ok);
    assert!(create("Invalid Input", &sources).is_err());
    sources.main_db = Some(shown(&fixtures.join("database-only/NHL07NG.db")));
    assert!(review(&sources).ok);
    eprintln!("MS03-4 PASS: invalid database rejected, corrected input accepted");
    sources.dictionary = Some(shown(&fixtures.join("optional-dictionary/invalid.db")));
    assert!(review(&sources).items.iter().any(|i| i.role == "dictionary" && !i.ok));
    assert!(create("Invalid Input", &sources).is_err());
    sources.dictionary = None;
    let id = create("Invalid Input", &sources).unwrap().summary.project_id;
    assert!(!reg.list_players(id).unwrap().is_empty());
    assert!(!reg.list_teams(id).unwrap().is_empty());
    reg.close_project(id);
    eprintln!("MS03-5 PASS: invalid dictionary rejected, clearing it permits import");
    sources.dictionary = Some(shown(&fixtures.join("optional-dictionary/nhl_eng_us.db")));
    let opened = create("With Dictionary", &sources).unwrap();
    let dict_path = opened.project.working_dictionary.unwrap();
    let original_dict = std::fs::read(fixtures.join("optional-dictionary/nhl_eng_us.db")).unwrap();
    assert_eq!(std::fs::read(&dict_path).unwrap(), original_dict);
    reg.project_save(opened.summary.project_id).unwrap();
    reg.close_project(opened.summary.project_id);
    let opened = reg.project_open(&shown(&temp.path().join("With Dictionary")), &defs).unwrap();
    assert!(!reg.list_players(opened.summary.project_id).unwrap().is_empty());
    assert!(!reg.list_teams(opened.summary.project_id).unwrap().is_empty());
    assert_eq!(std::fs::read(opened.project.working_dictionary.unwrap()).unwrap(), original_dict);
    reg.close_project(opened.summary.project_id);
    eprintln!("MS03-6 PASS: dictionary attached, preserved, reopened; Players/Teams usable");
}

/// Exercise a real bulk release without touching the user's installation or install records.
#[test]
#[ignore = "set NHLMS_TEST_PACKAGE to an NHL07 Xbox database-only package"]
fn bulk_release_install_restore_on_isolated_target() {
    let package = std::env::var("NHLMS_TEST_PACKAGE").expect("NHLMS_TEST_PACKAGE");
    let temp = tempfile::tempdir().unwrap();
    let game = temp.path().join("game");
    std::fs::create_dir_all(game.join("db")).unwrap();
    std::fs::write(game.join("default.xex"), b"XEX2").unwrap();
    for name in ["NHL07NG.db", "NHL07NG-meta.xml"] {
        std::fs::copy(repo(&format!("anchors/xbox360/nhl07/databases/{name}")), game.join("db").join(name)).unwrap();
    }
    let dest = game.join("db/NHL07NG.db");
    // Optional read-only reproduction of a recipient's pre-install backup.
    if let Ok(source) = std::env::var("NHLMS_TEST_DATABASE") {
        std::fs::copy(source, &dest).unwrap();
    }
    let original = std::fs::read(&dest).unwrap();
    let reg = Registry::default();
    let start = std::time::Instant::now();
    let plan = reg.package_preflight(&package, "nhl07", Platform::Xbox, &shown(&game), None, None, false,
        &shown(&temp.path().join("records")), &shown(&repo("defs"))).unwrap();
    eprintln!("{} edits prepared in {:?}", plan.applied, start.elapsed());
    assert!(plan.installable, "{:?} {:?}", plan.reasons, plan.conflicts);
    assert!(plan.applied > 100, "use a bulk package");
    assert_eq!(std::fs::read(&dest).unwrap(), original, "preflight cannot write the game");
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_ne!(std::fs::read(&dest).unwrap(), original);
    let identity = studio_domain::tdb_txn::FileIdentity::of_bytes(&std::fs::read(&dest).unwrap());
    eprintln!("Installed database: {identity:?}");
    if let Ok(expected) = std::env::var("NHLMS_TEST_EXPECTED_SHA256") {
        assert_eq!(identity.sha256, expected, "optimized replay must preserve the previous output");
    }
    // Historical imports are ordinary database edits in Overview: keep every net field,
    // including changes to hundreds of attributes, through a saved-project reopen.
    let inv = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let project_dir = temp.path().join("overview");
    let opened = reg.project_create_fresh(&shown(&project_dir), "nhl07", Platform::Xbox, "", None, None, &inv, &shown(&repo("defs"))).unwrap();
    reg.close_project(opened.summary.project_id);
    std::fs::copy(&dest, opened.project.working_main_db.unwrap()).unwrap();
    let reopened = reg.project_open(&shown(&project_dir), &shown(&repo("defs"))).unwrap();
    let overview = reg.project_overview(reopened.summary.project_id).unwrap();
    assert!(overview.changes.len() >= plan.applied as usize);
    assert!(overview.changes.iter().any(|c| c.groups.iter().any(|g| g == "Players")));
    assert_eq!(overview.changes.len(), overview.changes.iter().map(|c| &c.id).collect::<std::collections::HashSet<_>>().len());
    reg.close_project(reopened.summary.project_id);
    // Rechecking an installed package must resolve every cell as already applied.
    let again = reg.package_preflight(&package, "nhl07", Platform::Xbox, &shown(&game), None, None, false,
        &shown(&temp.path().join("records")), &shown(&repo("defs"))).unwrap();
    assert_eq!(again.applied, 0);
    assert!(again.conflicts.is_empty());
    assert_eq!(again.reasons, vec!["every edit in the package is already present at the destination; nothing to install"]);
    assert_eq!(reg.package_restore(&applied.record_dir).unwrap().state, "restored");
    assert_eq!(std::fs::read(&dest).unwrap(), original);
}

#[test]
fn package_replay_keeps_field_guards_and_rolls_back_invalid_final_state() {
    use studio_domain::{EditSession, FieldValue, Project, ProjectRole};
    let path = repo("anchors/xbox360/nhl07/databases/NHL07NG.db");
    let original = std::fs::read(&path).unwrap();
    let mut session = EditSession::begin(Project::load(&path, Platform::Xbox, ProjectRole::Nhlng,
        repo("anchors/xbox360/nhl07/databases/NHL07NG-meta.xml"), repo("defs")).unwrap());
    // A text write preceding a forbidden primary-key write cannot partially apply.
    let name = ("ubPc".into(), 281, "QbMR".into(), FieldValue::Text("Test".into()));
    assert!(session.set_package_fields(&[name.clone(), ("ubPc".into(),281,"wBIz".into(),FieldValue::Int(999))]).is_err());
    assert_eq!(session.save().unwrap(), original);
    // Vancouver already has #38: this final state remains invalid even in a batch.
    let error = session.set_package_fields(&[name, ("eGlu".into(),303,"dXSB".into(),FieldValue::Int(28))]).unwrap_err();
    assert!(error.to_string().contains("dup_on_team"), "{error}");
    assert_eq!(session.save().unwrap(), original);
    assert!(session.set_package_fields(&[("ubPc".into(),281,"sVRt".into(),FieldValue::Int(1000))]).is_err());
    assert_eq!(session.save().unwrap(), original);
}

#[test]
fn transfer_undo_save_reopen_build_install_restore() {
    let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("nhl07-roster-workflow-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let defs = shown(&repo("defs"));
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let reg = Registry::default();
    let project = root.join("project");
    let opened = reg.project_create_fresh(&shown(&project), "nhl07", Platform::Xbox, "", None, None, &store, &defs).unwrap();
    let id = opened.summary.project_id;
    reg.move_player(id, 613, 14, 28).unwrap();
    reg.set_jersey_number(id, 28, 613, 91).unwrap();
    membership(&reg, id, 28, 91);
    assert!(reg.undo(id).unwrap());
    assert!(reg.undo(id).unwrap());
    membership(&reg, id, 14, 38);
    assert!(!reg.pending_changes(id, 100).unwrap().dirty);
    assert!(reg.redo(id).unwrap());
    assert!(reg.redo(id).unwrap());
    let pending = reg.pending_changes(id, 100).unwrap();
    assert!(pending.roster.is_none());
    assert_eq!(pending.primary.total_cells, 4);
    assert!(reg.project_save(id).unwrap().saved);
    reg.close_project(id);
    let opened = reg.project_open(&shown(&project), &defs).unwrap();
    let id = opened.summary.project_id;
    membership(&reg, id, 28, 91);
    let working = std::fs::read(opened.project.working_main_db.as_ref().unwrap()).unwrap();
    let built = reg.package_build(id, &PackageBuildRequestDto {
        name: "Demitra Vancouver 91".into(), version: "1".into(), description: "Exhibition-only table qualification".into(),
        include: vec!["cells".into()], exclude: vec![], platforms: vec!["xbox".into()], out_path: Some(shown(&root.join("test.nhlmod"))),
    }, Some(&store), &defs).unwrap();
    let inspected = reg.package_inspect(&built.path).unwrap();
    assert_eq!(inspected.cells.len(), 4);
    // A completely independent vanilla installation, with a save that must remain untouched.
    let game = root.join("recipient");
    std::fs::create_dir_all(game.join("db")).unwrap();
    std::fs::write(game.join("default.xex"), b"XEX2").unwrap();
    for name in ["NHL07NG.db", "NHL07NG-meta.xml"] {
        std::fs::copy(repo(&format!("anchors/xbox360/nhl07/databases/{name}")), game.join("db").join(name)).unwrap();
    }
    let dest = game.join("db/NHL07NG.db");
    let original = std::fs::read(&dest).unwrap();
    let save = game.join("ROSTER1");
    std::fs::copy(repo("anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1"), &save).unwrap();
    let save_before = std::fs::read(&save).unwrap();
    let plan = reg.package_preflight(&built.path, "nhl07", Platform::Xbox, &shown(&game), None, None, false, &shown(&root.join("installs")), &defs).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    assert_eq!(plan.applied, 4);
    // Current database-only install recovery: fail after publication, then repeat with
    // rollback also failing. Neither scenario may touch the user's roster save.
    use studio_domain::in_place_save::{Event, Hooks};
    struct FailPublication { rollback: bool }
    impl Hooks for FailPublication {
        fn event(&mut self, event: &Event) -> std::io::Result<()> {
            if matches!(event, Event::AfterRename(_)) || (self.rollback && matches!(event, Event::BeforeRevert(_))) {
                return Err(std::io::Error::other("test interruption"));
            }
            Ok(())
        }
    }
    assert!(reg.package_apply_with(plan.handle, &mut FailPublication { rollback: false }).is_err());
    assert_eq!(std::fs::read(&dest).unwrap(), original);
    let plan = reg.package_preflight(&built.path, "nhl07", Platform::Xbox, &shown(&game), None, None, false, &shown(&root.join("installs")), &defs).unwrap();
    assert!(reg.package_apply_with(plan.handle, &mut FailPublication { rollback: true }).is_err());
    let failed = reg.package_installs(&shown(&root.join("installs"))).unwrap().into_iter().find(|r| r.recovery).unwrap();
    assert_eq!(Registry::default().package_restore(&failed.record_dir).unwrap().state, "restored");
    assert_eq!(std::fs::read(&dest).unwrap(), original);
    let plan = reg.package_preflight(&built.path, "nhl07", Platform::Xbox, &shown(&game), None, None, false, &shown(&root.join("installs")), &defs).unwrap();
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_eq!(std::fs::read(&dest).unwrap(), working);
    assert_eq!(std::fs::read(&save).unwrap(), save_before);
    assert!(reg.package_restore_with(&applied.record_dir, &mut FailPublication { rollback: false }).is_err());
    assert_eq!(std::fs::read(&dest).unwrap(), working);
    assert_eq!(reg.package_restore(&applied.record_dir).unwrap().state, "restored");
    assert_eq!(std::fs::read(&dest).unwrap(), original);
    reg.close_project(id);
    // Only remove the uniquely named temporary tree created by this test.
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}
