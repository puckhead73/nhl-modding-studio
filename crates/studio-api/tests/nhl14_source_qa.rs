//! Opt-in NHL14 QA; input images are read-only and outputs require a fresh folder.
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use studio_api::Registry;
use studio_domain::{discovery, extract_service, FieldValue, Platform};

fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
#[ignore = "requires NHL14_QA_ISO and a fresh NHL14_QA_OUT; copies the real Xbox360 disc"]
fn xbox_iso_dump_and_relocated_files_open_and_preserve_staged_edits() {
    let iso = PathBuf::from(std::env::var("NHL14_QA_ISO").expect("read-only ISO"));
    let out = PathBuf::from(std::env::var("NHL14_QA_OUT").expect("fresh output"));
    assert!(!out.exists(), "never overwrite prior QA evidence");
    std::fs::create_dir_all(&out).unwrap();
    let meta = repo("anchors/xbox360/nhl14/databases/nhlng-meta.xml");
    let defs = repo("defs");
    let located = studio_domain::xiso_source::locate_databases(&iso).unwrap();
    let original = studio_domain::xiso_source::read_located(&iso, located.nhlng.as_ref().unwrap()).unwrap();
    let stage = out.join("arbitrary staging location");
    let reg = Registry::default();
    let (staged, project) = reg.iso_open(&s(&iso), "nhl14", "xbox360", &s(&meta), &s(&stage), &s(&defs)).unwrap();
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
    let (_, reopened) = again.iso_open(&s(&iso), "nhl14", "xbox360", &s(&meta), &s(&stage), &s(&defs)).unwrap();
    assert_eq!(again.list_players(reopened.project_id).unwrap().iter().find(|p| p.player_id == players[0].player_id).unwrap().first_name, "QA");
    assert!(tdb_core::TdbFile::parse(&std::fs::read(&staged.db_path).unwrap()).unwrap().header_crc_ok());
    assert_eq!(studio_domain::xiso_source::read_located(&iso, located.nhlng.as_ref().unwrap()).unwrap(), original);
    eprintln!("NHL14 ISO edit/reopen passed: {} players, {} teams", players.len(), teams.len());

    // A materialized disc is the dumped-disc input; extract only this new copy.
    let dump = out.join("relocated disc dump");
    let copied = studio_domain::disc_extract::extract(&iso, &dump, &AtomicBool::new(false), &mut |_,_| {}).unwrap();
    eprintln!("Copied {} disc files / {} bytes", copied.members, copied.bytes);
    let extracted = extract_service::extract(&dump, Platform::Xbox, &AtomicBool::new(false), &mut |_| {}).unwrap();
    let found = discovery::detect_install("nhl14", Platform::Xbox, &dump, None);
    let db = found.db_paths.nhlng.as_ref().expect("extracted DB found");
    let schema = found.db_paths.meta_xml.as_deref().map(str::to_string).unwrap_or_else(|| s(&meta));
    let loose = Registry::default();
    let opened = loose.open_workspace_from_paths(Some("nhl14"), Platform::Xbox, db, None, &schema, &s(&defs)).unwrap();
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
    // Zero known members remaining does not imply success if an archive could not be listed.
    assert_eq!(extracted.archives_failed, 0, "{}", format!("{extracted:?}"));
    assert_eq!(rescan.state, "complete");
    eprintln!("NHL14 source routes complete: {} archives, {} remaining", rescan.archives.len(), rescan.members_to_write);
}

#[test]
#[ignore = "resumes only the owned NHL14 QA disc after decoder repairs"]
fn resume_copied_disc_after_codec_fixes() {
    let out = PathBuf::from("H:/TEMP/nhl-studio-overnight-20261003/nhl14/source-routes-1");
    let dump = out.join("relocated disc dump");
    assert!(dump.is_dir());
    let progress = extract_service::extract(&dump, Platform::Xbox, &AtomicBool::new(false), &mut |_| {}).unwrap();
    let pre = extract_service::preflight(&dump, Platform::Xbox);
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(out.join("extraction-resume-1.json")).unwrap();
    use std::io::Write;
    file.write_all(&serde_json::to_vec_pretty(&serde_json::json!({"progress":format!("{progress:?}"),"state":pre.state,"remaining":pre.members_to_write})).unwrap()).unwrap();
    assert_eq!(progress.archives_failed, 0);
    assert_eq!(progress.files_unsupported, 0);
    assert_eq!(progress.files_conflicting, 0);
    assert_eq!(pre.members_to_write, 0);
    assert_eq!(pre.state, "complete");
}




