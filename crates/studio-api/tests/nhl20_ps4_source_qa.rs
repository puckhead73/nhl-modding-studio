//! PS4 NHL20 packed-install extraction through production services on an owned copy.
use std::{path::{Path,PathBuf},sync::atomic::AtomicBool};
use studio_api::Registry;
use studio_domain::{extract_service,discovery,Platform};
fn repo(p:&str)->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)}
fn s(p:&Path)->String {p.to_string_lossy().into_owned()}
#[test]
#[ignore="extracts only the owned NHL20 PS4 QA dump; originals remain read-only"]
fn ps4_dump_extraction_and_existing_files() {
    let root=Path::new("F:/TEMP/nhl-studio-overnight-20261003/nhl20/pkg-source-routes-1/CUSA15439");
    assert!(root.join("eboot.bin").is_file());
    let out=repo("target/overnight-20261003/nhl20/ps4-source-receipt-1.json");
    assert!(!out.exists());
    let pre=extract_service::preflight(root,Platform::Ps4);
    eprintln!("Preflight {} archives / {} members / {} collisions: {}",pre.archives.len(),pre.members_total,pre.collisions.len(),pre.reason);
    let mut checkpoint = std::time::Instant::now();
    let result=extract_service::extract(root,Platform::Ps4,&AtomicBool::new(false),&mut |p| {
        // Long game archives can take minutes; future runs retain visible progress.
        if checkpoint.elapsed().as_secs() >= 30 {
            eprintln!("{} / {} archives, {} files, {} bytes; {}",p.archives_done,p.archives_total,p.files_written,p.bytes_written,p.current);
            std::fs::write(out.with_extension("progress.json"),serde_json::to_vec_pretty(&serde_json::json!({"archives_done":p.archives_done,"archives_total":p.archives_total,"files_written":p.files_written,"bytes_written":p.bytes_written,"current":p.current})).unwrap()).unwrap();
            checkpoint = std::time::Instant::now();
        }
    }).unwrap();
    let after=extract_service::preflight(root,Platform::Ps4);
    let found=discovery::detect_install("nhl20",Platform::Ps4,root,None);
    std::fs::write(&out,serde_json::to_vec_pretty(&serde_json::json!({"archives":pre.archives.len(),"members":pre.members_total,"collisions":pre.collisions.len(),"result":format!("{result:?}"),"state":after.state,"remaining":after.members_to_write,"discovery":format!("{found:?}"),"game_verified":false})).unwrap()).unwrap();
    let reg=Registry::default();
    let db=found.db_paths.nhlng.as_ref().expect("readable database after extraction");
    let id=reg.open_workspace_from_paths(Some("nhl20"),Platform::Ps4,db,None,&s(&repo("H:/Emulators/ShadPS4/Games/CUSA15439/databigs/Extracted/nhl/data/common/gamemodes/database/t3db/nhl_game-meta.xml")),&s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id,found.db_paths.localization.as_deref().unwrap()).unwrap();
    eprintln!("PS4 extracted open: {} players / {} teams; state {}; failed {}; blocked {}",reg.list_players(id).unwrap().len(),reg.list_teams(id).unwrap().len(),after.state,result.archives_failed,result.files_blocked);
    assert!(reg.list_players(id).unwrap().len()>2000);
    assert_eq!(result.archives_failed,0);
    assert_eq!(after.state,"complete");
}
