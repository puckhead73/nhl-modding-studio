//! NHL 08 free-agent changes survive the package build/install/restore pipeline. Anchors stay
//! read-only; the project and the install target are temporary copies.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
fn nhl08_ps3_free_agency_build_install_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let store = BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap();
    let reg = Registry::default();
    let defs = s(&repo("defs"));
    let root = tmp.path().join("project");
    let id = reg.project_create_fresh(&s(&root), "nhl08", Platform::Ps3, "", None, None, &store, &defs).unwrap().summary.project_id;
    let initial_pool: Vec<_> = reg.list_free_agents(id).unwrap().iter().map(|p| p.player_id).collect();
    assert!(!initial_pool.is_empty());
    // Release the first rostered club player (national memberships refuse a club release).
    let (team, released) = reg.list_players(id).unwrap().iter()
        .filter_map(|p| p.team_id.map(|t| (t, p.player_id)))
        .find(|&(t, p)| reg.release_to_fa(id, t, p).is_ok())
        .expect("a releasable club player");
    // NHL 08's shipped free agent 0 has only index key 0, which a roster slot reads as vacant;
    // sign an ordinary free agent.
    let signed = *initial_pool.iter().find(|&&p| p != 0).unwrap();
    reg.sign_from_fa(id, signed, team).unwrap();
    reg.project_save(id).unwrap();
    let pool: Vec<_> = reg.list_free_agents(id).unwrap().iter().map(|p| p.player_id).collect();
    assert!(pool.contains(&released) && !pool.contains(&signed));
    let package = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto {
        name: "NHL 08 free agency".into(), version: "1".into(), description: "Qualification".into(),
        include: vec!["cells".into()], exclude: vec![], platforms: vec!["ps3".into()],
        out_path: Some(s(&tmp.path().join("fa.nhlmod"))),
    }, Some(&store), &defs).unwrap();
    let inspected = studio_domain::mod_package::inspect_package(Path::new(&package.path), &Default::default()).unwrap();
    assert!(inspected.edits.free_agents.is_some());

    let game = tmp.path().join("game");
    std::fs::create_dir_all(game.join("db")).unwrap();
    for file in ["nhlng.db", "nhlng-meta.xml"] {
        std::fs::copy(repo(&format!("anchors/ps3/nhl08/databases/{file}")), game.join("db").join(file)).unwrap();
    }
    let db = game.join("db/nhlng.db");
    let original = std::fs::read(&db).unwrap();
    let plan = reg.package_preflight(&package.path, "nhl08", Platform::Ps3, &s(&game), None, None, false,
        &s(&tmp.path().join("records")), &defs).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let installed = reg.package_apply(plan.handle).unwrap();
    let actual = std::fs::read(&db).unwrap();
    let expected = std::fs::read(root.join("working/db/nhlng.db")).unwrap();
    assert_eq!(studio_domain::explicit_free_agents(&actual).unwrap(), studio_domain::explicit_free_agents(&expected).unwrap());
    // Allocation history may differ after structural edits; compare every live record.
    let a = tdb_core::TdbFile::parse(&actual).unwrap();
    let b = tdb_core::TdbFile::parse(&expected).unwrap();
    for at in &a.tables {
        let bt = b.table_canonical(&at.id.canonical(a.endian)).unwrap();
        assert_eq!(at.current_records, bt.current_records);
        let len = at.current_records as usize * at.record_len_bytes as usize;
        assert_eq!(studio_domain::baselines::sha256_hex(&actual[at.record_block..at.record_block + len]),
            studio_domain::baselines::sha256_hex(&expected[bt.record_block..bt.record_block + len]), "{}", at.id.canonical(a.endian));
    }
    reg.package_restore(&installed.record_dir).unwrap();
    assert_eq!(std::fs::read(&db).unwrap(), original);
    reg.close_project(id);
}
