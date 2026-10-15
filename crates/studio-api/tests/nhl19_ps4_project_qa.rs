//! Create the owned PS4 review project, with independent original inputs copied by Studio.
use std::path::Path;
use studio_api::{Registry,ImportSourcesDto};
use studio_domain::Platform;
#[test]
#[ignore="creates one new NHL19 PS4 project; never replaces previous evidence"]
fn create_ps4_project() {
    let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let a=repo.join("anchors/ps4/nhl19/databases");
    let out=repo.join("target/overnight-20261003/nhl19/ps4 desktop project");
    assert!(!out.exists());
    let sources=ImportSourcesDto{
        main_db:Some(a.join("nhl_game.db").to_string_lossy().into()),
        meta:Some(a.join("nhl_game-meta.xml").to_string_lossy().into()),
        dictionary:Some("H:/Emulators/ShadPS4/Games/CUSA11126/databigs/Extracted/nhl/data/kettle/loc/eng_us.db".into()),
        roster_save:Some(repo.join("anchors/ps4/nhl19/savefiles/RosterCustom0/DATA").to_string_lossy().into()),
        assets_dir:Some("F:/TEMP/nhl-studio-overnight-20261003/nhl19/ps4-source-routes-1/NHL 19/CUSA11126".into()),
    };
    Registry::default().project_create_import(&out.to_string_lossy(),"nhl19",Platform::Ps4,&sources,"",None,None,None,&repo.join("defs").to_string_lossy()).unwrap();
    assert!(out.join("nhl-studio-project.json").is_file());
}




