//! NHL14 PS3 supplied-installation route; no ISO was located, so none is claimed.
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use studio_api::Registry;
use studio_domain::{discovery, extract_service, Platform};

#[test]
#[ignore = "extracts only the owned NHL14 PS3 installation copy on S:"]
fn copied_installation_extracts_and_opens_at_relocated_path() {
    let out = Path::new("S:/TEMP/nhl-studio-overnight-20261003/nhl14/ps3-existing-routes-1");
    let disc = out.join("relocated PS3 disc");
    let game = disc.join("PS3_GAME/USRDIR");
    assert!(game.join("EBOOT.BIN").is_file());
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let receipt = repo.join("target/overnight-20261003/nhl14-ps3-existing-receipt.json");
    assert!(!receipt.exists(), "preserve prior evidence");
    let original = std::fs::read("X:/Games/Emulation/Roms/Sony Playstation 3/NHL 14/PS3_GAME/USRDIR/db/nhlng.db").unwrap();
    let before = extract_service::preflight(&game, Platform::Ps3);
    assert_ne!(before.state, "not_applicable");
    let extracted = extract_service::extract(&game, Platform::Ps3, &AtomicBool::new(false), &mut |_|{}).unwrap();
    let db = std::fs::read(game.join("db/nhlng.db")).unwrap();
    assert_eq!(db, original);
    assert!(tdb_core::TdbFile::parse(&db).unwrap().header_crc_ok());
    let mut counts = Vec::new();
    // Resolve from each user-facing folder level, without a fixed emulator path.
    for root in [&disc, &disc.join("PS3_GAME"), &game] {
        let found = discovery::detect_install("nhl14", Platform::Ps3, root, None);
        let reg = Registry::default();
        let meta: PathBuf = repo.join("anchors/ps3/nhl14/databases/nhlng-meta.xml");
        let id = reg.open_workspace_from_paths(Some("nhl14"), Platform::Ps3,
            found.db_paths.nhlng.as_deref().unwrap(), None,
            &meta.to_string_lossy(), &repo.join("defs").to_string_lossy()).unwrap().project_id;
        reg.attach_dictionary(id, found.db_paths.localization.as_deref().unwrap()).unwrap();
        let players = reg.list_players(id).unwrap().len();
        let teams = reg.list_teams(id).unwrap().len();
        assert!(players > 2000 && teams > 100);
        counts.push(serde_json::json!({"root":root,"players":players,"teams":teams}));
    }
    let after = extract_service::preflight(&game, Platform::Ps3);
    std::fs::write(receipt, serde_json::to_vec_pretty(&serde_json::json!({
        "route":"copied supplied installation; ISO unavailable",
        "extraction":format!("{extracted:?}"),"state":after.state,
        "remaining":after.members_to_write,"opens":counts,"game_verified":false
    })).unwrap()).unwrap();
    assert_eq!(extracted.archives_failed, 0, "{extracted:?}");
    assert_eq!(after.state, "complete");
}
