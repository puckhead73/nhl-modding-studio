//! Create the owned PS4 review project, with independent original inputs copied by Studio.
use std::path::Path;
use studio_api::{Registry,ImportSourcesDto};
use studio_domain::Platform;
#[test]
#[ignore="creates one new NHL16 PS4 project; never replaces previous evidence"]
fn create_ps4_project() {
    let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let a=repo.join("anchors/ps4/nhl16/databases");
    let out=repo.join("target/overnight-20261003/nhl16/ps4 desktop project");
    assert!(!out.exists());
    let sources=ImportSourcesDto{
        main_db:Some(a.join("nhl_game.db").to_string_lossy().into()),
        meta:Some(a.join("nhl_game-meta.xml").to_string_lossy().into()),
        dictionary:Some(a.join("eng_us.db").to_string_lossy().into()),
        roster_save:Some(repo.join("anchors/ps4/nhl16/savefiles/84f8102f - ROSTER/DATA").to_string_lossy().into()),
        assets_dir:Some("F:/TEMP/nhl-studio-overnight-20261003/nhl16/ps4-source-routes-1/NHL 16/CUSA01909".into()),
    };
    Registry::default().project_create_import(&out.to_string_lossy(),"nhl16",Platform::Ps4,&sources,"",None,None,None,&repo.join("defs").to_string_lossy()).unwrap();
    assert!(out.join("nhl-studio-project.json").is_file());
}
