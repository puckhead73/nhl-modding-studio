//! NHLLEGACY PS3 ISO, dumped-disc and relocated-file QA on owned copies.
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use studio_api::Registry;
use studio_domain::{discovery, extract_service, Platform};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }
#[test]
#[ignore = "requires decrypted NHLLEGACY_QA_PS3_ISO and fresh NHLLEGACY_QA_PS3_OUT; preserves image"]
fn ps3_iso_dump_and_relocated_files_open() {
    let iso=PathBuf::from(std::env::var("NHLLEGACY_QA_PS3_ISO").unwrap());
    let out=PathBuf::from(std::env::var("NHLLEGACY_QA_PS3_OUT").unwrap());
    assert!(!out.exists()); std::fs::create_dir_all(&out).unwrap();
    let disc=out.join("relocated PS3 disc");
    // Exercise the same asynchronous entry point as desktop onboarding.
    let jobs = Registry::default();
    let mut receipt = jobs.disc_extract_start(&s(&iso), &s(&disc)).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    while matches!(receipt.state.as_str(), "running" | "cancelling") {
        assert!(std::time::Instant::now() < deadline, "disc copy timed out");
        std::thread::sleep(std::time::Duration::from_millis(100));
        receipt = jobs.extract_status(receipt.job).unwrap();
    }
    assert_eq!(receipt.state, "done", "{:?}", receipt.error);
    // Extract packed containers before looking for loose database files.
    finish_ps3_dump(&out, serde_json::to_value(&receipt).unwrap());
}

fn finish_ps3_dump(out: &Path, receipt: serde_json::Value) {
    assert!(!out.join("receipt.json").exists(), "preserve previous receipt");
    let disc=out.join("relocated PS3 disc");
    let game=disc.join("PS3_GAME/USRDIR");
    assert!(game.join("EBOOT.BIN").is_file());
    let db=game.join("db/nhlng.db");
    let pre=extract_service::preflight(&game,Platform::Ps3);
    assert_ne!(pre.state,"not_applicable"); assert!(pre.members_to_write>0);
    let extracted=extract_service::extract(&game,Platform::Ps3,&AtomicBool::new(false),&mut |_|{}).unwrap();
    let original=std::fs::read(&db).unwrap();
    assert!(tdb_core::TdbFile::parse(&original).unwrap().header_crc_ok(),"requires decrypted game data");
    let independent=Path::new("X:/Games/Emulation/Roms/Sony Playstation 3/NHL Legacy - Vanilla/PS3_GAME/USRDIR/db/nhlng.db");
    assert_eq!(original,std::fs::read(independent).unwrap(),"extracted database matches independent dump");
    let found=discovery::detect_install("nhllegacy",Platform::Ps3,&disc,None);
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhllegacy"),Platform::Ps3,
        found.db_paths.nhlng.as_deref().unwrap(),None,
        &s(&repo("anchors/ps3/nhllegacy/databases/nhlng-meta.xml")),&s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id,found.db_paths.localization.as_deref().unwrap()).unwrap();
    let players=reg.list_players(id).unwrap(); let teams=reg.list_teams(id).unwrap();
    assert!(players.len()>2000 && teams.len()>100);
    assert_eq!(std::fs::read(&db).unwrap(),original);
    let after=extract_service::preflight(&game,Platform::Ps3);
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "disc_copy":receipt,"extraction":format!("{extracted:?}"),
        "players":players.len(),"teams":teams.len(),"state":after.state,
        "source_db_preserved":true,"game_verified":false
    })).unwrap()).unwrap();
    assert_eq!(extracted.archives_failed, 0, "{extracted:?}");
    assert_eq!(after.state, "complete");
}







