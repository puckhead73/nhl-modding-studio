//! PS4 NHL15 dumped-disc extraction through production services on an owned RAR copy.
use std::{path::{Path,PathBuf},sync::atomic::AtomicBool};
use studio_api::Registry;
use studio_domain::{extract_service,discovery,Platform};
fn repo(p:&str)->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)}
fn s(p:&Path)->String {p.to_string_lossy().into_owned()}
#[test]
#[ignore="extracts only the owned NHL15 PS4 QA dump; originals remain read-only"]
fn ps4_dump_extraction_and_existing_files() {
    let root=Path::new("S:/TEMP/nhl-studio-overnight-20261003/nhl15/ps4-source-routes-1/NHL 15/CUSA00561");
    assert!(root.join("eboot.bin").is_file());
    let out=repo("target/overnight-20261003/nhl15/ps4-source-receipt-1.json");
    assert!(!out.exists());
    let pre=extract_service::preflight(root,Platform::Ps4);
    eprintln!("Preflight {} archives / {} members / {} collisions: {}",pre.archives.len(),pre.members_total,pre.collisions.len(),pre.reason);
    let result=extract_service::extract(root,Platform::Ps4,&AtomicBool::new(false),&mut |_|{}).unwrap();
    let after=extract_service::preflight(root,Platform::Ps4);
    let found=discovery::detect_install("nhl15",Platform::Ps4,root,None);
    std::fs::write(&out,serde_json::to_vec_pretty(&serde_json::json!({"before":format!("{pre:?}"),"result":format!("{result:?}"),"after":format!("{after:?}"),"discovery":format!("{found:?}"),"game_verified":false})).unwrap()).unwrap();
    let reg=Registry::default();
    let db=found.db_paths.nhlng.as_ref().expect("readable database after extraction");
    let id=reg.open_workspace_from_paths(Some("nhl15"),Platform::Ps4,db,None,&s(&repo("anchors/ps4/nhl15/databases/nhl_game-meta.xml")),&s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id,found.db_paths.localization.as_deref().unwrap()).unwrap();
    eprintln!("PS4 extracted open: {} players / {} teams; state {}; failed {}; blocked {}",reg.list_players(id).unwrap().len(),reg.list_teams(id).unwrap().len(),after.state,result.archives_failed,result.files_blocked);
    // A partial run is recorded as partial; opening a database does not certify all art.
    assert!(reg.list_players(id).unwrap().len()>2000);
}
