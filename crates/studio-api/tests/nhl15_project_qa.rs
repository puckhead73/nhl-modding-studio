//! Create a reviewable desktop project from the owned NHL15 disc copy.
use std::path::{Path, PathBuf};
use studio_api::{ImportSourcesDto, Registry};
use studio_domain::Platform;

#[test]
#[ignore = "requires the owned NHL15 QA disc copy and a fresh project path"]
fn create_owned_desktop_project() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ps3 = std::env::var("NHL15_QA_PLATFORM").as_deref() == Ok("ps3");
    let platform = if ps3 { Platform::Ps3 } else { Platform::Xbox };
    let root = PathBuf::from(if ps3 {
        "S:/TEMP/nhl-studio-overnight-20261003/nhl15/ps3-source-routes-1/relocated PS3 disc/PS3_GAME/USRDIR"
    } else { "S:/TEMP/nhl-studio-overnight-20261003/nhl15/source-routes-1/relocated disc dump" });
    let out = PathBuf::from(if ps3 {
        "E:/Repositories/nhl-modding-studio/target/overnight-20261003/nhl15/ps3 desktop project"
    } else { "E:/Repositories/nhl-modding-studio/target/overnight-20261003/nhl15/desktop project" });
    assert!(!out.exists(), "preserve previous evidence");
    // Qualify the independent existing-files route while ISO assets unpack separately.
    let inputs = if ps3 {
        PathBuf::from("X:/Games/Emulation/Roms/Sony Playstation 3/NHL 15/PS3_GAME/USRDIR")
    } else {
        PathBuf::from("X:/Games/Emulation/Roms/XBOX360/NHL 15/NHL 15")
    };
    let db = inputs.join("db/nhlng.db");
    let original = std::fs::read(&db).unwrap();
    let sources = ImportSourcesDto {
        main_db: Some(db.to_string_lossy().into()),
        meta: Some(repo.join(if ps3 { "anchors/ps3/nhl15/databases/nhlng-meta.xml" } else {
            "anchors/xbox360/nhl15/databases/nhlng-meta.xml"
        }).to_string_lossy().into()),
        dictionary: Some(inputs.join("fe/loc/nhl_eng_us.db").to_string_lossy().into()),
        roster_save: None,
        assets_dir: Some(root.to_string_lossy().into()),
    };
    // Destinations are deliberately empty; source assets are an owned QA copy.
    Registry::default().project_create_import(&out.to_string_lossy(), "nhl15", platform,
        &sources, "", None, None, None, &repo.join("defs").to_string_lossy()).unwrap();
    assert_eq!(std::fs::read(db).unwrap(), original);
    assert!(out.join("nhl-studio-project.json").is_file());
}



