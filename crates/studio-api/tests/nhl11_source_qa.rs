//! Opt-in NHL11 QA; input images are read-only and outputs require a fresh folder.
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use studio_api::Registry;
use studio_domain::{discovery, extract_service, FieldValue, Platform};

fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
#[ignore = "requires NHL11_QA_ISO and a fresh NHL11_QA_OUT; copies the real Xbox360 disc"]
fn xbox_iso_dump_and_relocated_files_open_and_preserve_staged_edits() {
    let iso = PathBuf::from(std::env::var("NHL11_QA_ISO").expect("read-only ISO"));
    let out = PathBuf::from(std::env::var("NHL11_QA_OUT").expect("fresh output"));
    assert!(!out.exists(), "never overwrite prior QA evidence");
    std::fs::create_dir_all(&out).unwrap();
    let meta = repo("anchors/xbox360/nhl11/databases/nhlng-meta.xml");
    let defs = repo("defs");
    let located = studio_domain::xiso_source::locate_databases(&iso).unwrap();
    let original = studio_domain::xiso_source::read_located(&iso, located.nhlng.as_ref().unwrap()).unwrap();
    let stage = out.join("arbitrary staging location");
    let reg = Registry::default();
    let (staged, project) = reg.iso_open(&s(&iso), "nhl11", "xbox360", &s(&meta), &s(&stage), &s(&defs)).unwrap();
    assert_eq!(std::fs::read(&staged.db_path).unwrap(), original);
    let players = reg.list_players(project.project_id).unwrap();
    let teams = reg.list_teams(project.project_id).unwrap();
    assert!(!players.is_empty() && !teams.is_empty());
    // Resolve the editable field from the loaded schema, not a later-title column guess.
    let player = reg.player_view(project.project_id, players[0].player_id).unwrap().unwrap();
    let first = player.all_fields.iter().find(|f| f.name == "firstname").unwrap();
    reg.set_field(project.project_id, &first.studio_table, player.bio_record, &first.studio_field, FieldValue::Text("QA".into())).unwrap();
    reg.save_workspace(project.project_id).unwrap();
    let again = Registry::default();
    let (_, reopened) = again.iso_open(&s(&iso), "nhl11", "xbox360", &s(&meta), &s(&stage), &s(&defs)).unwrap();
    assert_eq!(again.list_players(reopened.project_id).unwrap().iter().find(|p| p.player_id == players[0].player_id).unwrap().first_name, "QA");
    assert!(tdb_core::TdbFile::parse(&std::fs::read(&staged.db_path).unwrap()).unwrap().header_crc_ok());
    assert_eq!(studio_domain::xiso_source::read_located(&iso, located.nhlng.as_ref().unwrap()).unwrap(), original);
    eprintln!("NHL11 ISO edit/reopen passed: {} players, {} teams", players.len(), teams.len());

    // A materialized disc is the dumped-disc input; extract only this new copy.
    let dump = out.join("relocated disc dump");
    let copied = studio_domain::disc_extract::extract(&iso, &dump, &AtomicBool::new(false), &mut |_,_| {}).unwrap();
    eprintln!("Copied {} disc files / {} bytes", copied.members, copied.bytes);
    let extracted = extract_service::extract(&dump, Platform::Xbox, &AtomicBool::new(false), &mut |_| {}).unwrap();
    let found = discovery::detect_install("nhl11", Platform::Xbox, &dump, None);
    let db = found.db_paths.nhlng.as_ref().expect("extracted DB found");
    let schema = found.db_paths.meta_xml.as_deref().map(str::to_string).unwrap_or_else(|| s(&meta));
    let loose = Registry::default();
    let opened = loose.open_workspace_from_paths(Some("nhl11"), Platform::Xbox, db, None, &schema, &s(&defs)).unwrap();
    assert_eq!(loose.list_players(opened.project_id).unwrap().len(), players.len());
    assert_eq!(loose.list_teams(opened.project_id).unwrap().len(), teams.len());
    let dict = found.db_paths.localization.as_ref().expect("dictionary found");
    loose.attach_dictionary(opened.project_id, dict).unwrap();
    let rescan = extract_service::preflight(&dump, Platform::Xbox);
    std::fs::write(out.join("source-routes.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "iso":iso, "staging":stage, "dump":dump, "copy":copied,
        "players":players.len(), "teams":teams.len(), "metadata":schema,
        "extraction":format!("{extracted:?}"), "rescan_state":rescan.state,
        "rescan_archives":rescan.archives.len(), "remaining_members":rescan.members_to_write,
        "staged_edit_reopened":true, "source_db_unchanged":true, "game_verified":false
    })).unwrap()).unwrap();
    eprintln!("NHL11 source routes complete: {} archives, {} remaining", rescan.archives.len(), rescan.members_to_write);
}


#[test]
fn compressed_disc_metadata_opens_without_rewriting_source() {
    let tmp = std::env::temp_dir().join(format!("nhl11-meta-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir(&tmp).unwrap();
    let meta = tmp.join("nhlng-meta.xml");
    let xml = std::fs::read(repo("anchors/xbox360/nhl11/databases/nhlng-meta.xml")).unwrap();
    let packed = tdb_archive_codec::refpack_compress(&xml);
    std::fs::write(&meta, &packed).unwrap();
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl11"), Platform::Xbox,
        &s(&repo("anchors/xbox360/nhl11/databases/nhlng.db")), None,
        &s(&meta), &s(&repo("defs"))).unwrap().project_id;
    assert!(reg.list_players(id).unwrap().len() > 3000);
    assert_eq!(std::fs::read(&meta).unwrap(), packed);
    // Malformed compressed metadata must fail, never silently choose another schema.
    std::fs::write(&meta, &packed[..6]).unwrap();
    assert!(Registry::default().open_workspace_from_paths(Some("nhl11"), Platform::Xbox,
        &s(&repo("anchors/xbox360/nhl11/databases/nhlng.db")), None,
        &s(&meta), &s(&repo("defs"))).is_err());
}

#[test]
#[ignore = "read-only verification of the copied NHL11_QA_DUMP after extraction"]
fn existing_dump_opens_with_shipped_metadata() {
    let dump = PathBuf::from(std::env::var("NHL11_QA_DUMP").unwrap());
    let found = discovery::detect_install("nhl11", Platform::Xbox, &dump, None);
    let meta = found.db_paths.meta_xml.as_ref().unwrap();
    let before = std::fs::read(meta).unwrap();
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl11"), Platform::Xbox,
        found.db_paths.nhlng.as_ref().unwrap(), None, meta, &s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id, found.db_paths.localization.as_ref().unwrap()).unwrap();
    assert_eq!(reg.list_players(id).unwrap().len(), 5310);
    assert_eq!(reg.list_teams(id).unwrap().len(), 242);
    assert_eq!(std::fs::read(meta).unwrap(), before);
    let pre = extract_service::preflight(&dump, Platform::Xbox);
    assert_eq!(pre.members_to_write, 0);
    eprintln!("NHL11 existing dump opens: {} archives, {} unreadable, state {}", pre.archives.len(), pre.archives.iter().filter(|a| a.parse_error.is_some()).count(), pre.state);
}

#[test]
#[ignore = "resumes extraction only in the generated NHL11 source-routes-1 QA copy"]
fn resume_copied_disc_after_codec_fixes() {
    let dump = repo("target/overnight-20261003/nhl11/source-routes-1/relocated disc dump");
    assert!(dump.is_dir());
    let progress = extract_service::extract(&dump, Platform::Xbox, &AtomicBool::new(false), &mut |_| {}).unwrap();
    let pre = extract_service::preflight(&dump, Platform::Xbox);
    let receipt = repo("target/overnight-20261003/nhl11/extraction-resume-2.json");
    // Keep the original extraction receipt and refuse to overwrite a prior rerun.
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(receipt).unwrap();
    use std::io::Write;
    file.write_all(&serde_json::to_vec_pretty(&serde_json::json!({"progress":format!("{progress:?}"),"state":pre.state,"remaining":pre.members_to_write})).unwrap()).unwrap();
    assert_eq!(progress.files_unsupported, 0);
    assert_eq!(progress.files_conflicting, 0);
    assert_eq!(pre.members_to_write, 0);
    assert_eq!(pre.state, "complete");
}
#[test]
#[ignore = "requires decrypted NHL11_QA_PS3_ISO and fresh NHL11_QA_PS3_OUT; preserves image"]
fn ps3_iso_dump_and_relocated_files_open() {
    let iso=PathBuf::from(std::env::var("NHL11_QA_PS3_ISO").unwrap());
    let out=PathBuf::from(std::env::var("NHL11_QA_PS3_OUT").unwrap());
    assert!(!out.exists()); std::fs::create_dir(&out).unwrap();
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
    // NHL11 packs db/nhlng.db inside boot.big; extract before looking for loose DBs.
    finish_ps3_dump(&out, serde_json::to_value(&receipt).unwrap());
}

#[test]
#[ignore = "resumes only the already copied NHL11 PS3 QA disc after the loose-DB assumption failed"]
fn ps3_resume_packed_database_disc() {
    let out=repo("target/overnight-20261003/nhl11/ps3-source-routes-1");
    assert!(out.join("relocated PS3 disc/PS3_GAME/USRDIR/EBOOT.BIN").is_file());
    finish_ps3_dump(&out, serde_json::json!({"prior_copy_completed":true,"receipt_unavailable_after_test_assertion":true}));
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
    let independent=Path::new("X:/Games/Emulation/Roms/Sony Playstation 3/NHL 11/PS3_GAME/USRDIR/db/nhlng.db");
    assert_eq!(original,std::fs::read(independent).unwrap(),"extracted database matches independent dump");
    let found=discovery::detect_install("nhl11",Platform::Ps3,&disc,None);
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl11"),Platform::Ps3,
        found.db_paths.nhlng.as_deref().unwrap(),None,
        &s(&repo("anchors/ps3/nhl11/databases/nhlng-meta.xml")),&s(&repo("defs"))).unwrap().project_id;
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
}




