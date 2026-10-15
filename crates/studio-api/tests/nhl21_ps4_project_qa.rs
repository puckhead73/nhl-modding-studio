//! Create the owned PS4 review project, with independent original inputs copied by Studio.
use std::path::Path;
use studio_api::{Registry,ImportSourcesDto};
use studio_domain::Platform;
#[test]
#[ignore="creates one new NHL21 PS4 project; never replaces previous evidence"]
fn create_ps4_project() {
    let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let a=std::path::PathBuf::from("H:/Emulators/ShadPS4/Games/CUSA18182/databigs/Extracted/nhl/data/common/gamemodes/database/t3db");
    let out=repo.join("target/overnight-20261003/nhl21/ps4 desktop project");
    assert!(!out.exists());
    let sources=ImportSourcesDto{
        main_db:Some(a.join("nhl_game.db").to_string_lossy().into()),
        meta:Some(a.join("nhl_game-meta.xml").to_string_lossy().into()),
        dictionary:Some("H:/Emulators/ShadPS4/Games/CUSA18182/databigs/Extracted/nhl/data/kettle/loc/eng_us.db".into()),
        roster_save:None,
        assets_dir:Some("F:/TEMP/nhl-studio-overnight-20261003/nhl21/pkg-source-routes-1/CUSA18182".into()),
    };
    Registry::default().project_create_import(&out.to_string_lossy(),"nhl21",Platform::Ps4,&sources,"",None,None,None,&repo.join("defs").to_string_lossy()).unwrap();
    assert!(out.join("nhl-studio-project.json").is_file());
}
