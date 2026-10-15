//! Native PS4 PNG/GNF replacements on fresh copied files only.
use std::path::{Path,PathBuf};
use studio_api::Registry;
fn s(p:&Path)->String {p.to_string_lossy().into_owned()}
#[test]
#[ignore="uses read-only NHL15 PS4 extracted assets; outputs to owned E: QA"]
fn ps4_native_texture_replace_roundtrip() {
    let source=PathBuf::from("H:/Emulators/ShadPS4/Games/CUSA00561/databigs/Extracted");
    let out=PathBuf::from("E:/Repositories/nhl-modding-studio/target/overnight-20261003/nhl15/ps4-texture-roundtrip-1");
    assert!(!out.exists()); std::fs::create_dir_all(&out).unwrap();
    let reg=Registry::default(); let mut results=Vec::new();
    for rel in [
        "nhl/ui/assets/leaguelogos/size64/l0.png",
        "nhl/ui/assets/teamlogos/fullsizeflat/t0.png",
        "nhl/ui/assets/portraits/p1.png",
        "nhl/data/kettle/rendering/characters/player/jersey_nineties_anaheim_away_cm.gnf",
        "nhl/data/kettle/rendering/characters/player/goaliesticktape_6_cm.gnf",
        "nhl/data/kettle/rendering/stadiums/common/centerlogo_0_cm.gnf",
    ] {
        let index=0;
        let original = std::fs::read(source.join(rel)).unwrap();
        let target = out.join(rel);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, &original).unwrap();
        // Exercise the production preview/plan/write path, not just the inner codec.
        let result = (|| -> Result<serde_json::Value, String> {
            let before = reg.texture_preview(&s(&out), rel, 0, Some(index)).map_err(|e| e.to_string())?;
            let rgba = [240, 10, 180, 255].repeat((before.width * before.height) as usize);
            let png = studio_domain::DecodedImage { width:before.width, height:before.height, rgba }.to_png().unwrap();
            let input = target.with_extension("qa.png");
            std::fs::write(&input, png).unwrap();
            let plan = reg.texture_replace_plan(&s(&out), rel, &s(&input), Some(index)).map_err(|e| e.to_string())?;
            if !plan.allowed { return Err(format!("review refused: {:?}", plan.refusal)); }
            let written = reg.import_texture(&s(&out), rel, &s(&input), true, Some(index), plan.review_id).map_err(|e| e.to_string())?;
            let after = Registry::default().texture_preview(&s(&out), rel, 0, Some(index)).map_err(|e| e.to_string())?;
            assert_eq!((before.width, before.height), (after.width, after.height));
            assert_ne!(before.data_url, after.data_url, "decoded replacement must differ");
            assert_ne!(std::fs::read(&target).unwrap(), original);
            assert_eq!(std::fs::read(format!("{}.bak", s(&target))).unwrap(), original);
            Ok(serde_json::json!({"width":after.width,"height":after.height,"container":after.container,"write":written,"decoded_changed":true}))
        })();
        assert_eq!(std::fs::read(source.join(rel)).unwrap(), original, "source stays untouched");
        results.push(match result {
            Ok(value) => serde_json::json!({"path":rel,"texture_index":index,"roundtrip":value}),
            Err(error) => serde_json::json!({"path":rel,"blocker":error}),
        });
    }
    std::fs::write(out.join("receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({"textures":results,"game_verified":false})).unwrap()).unwrap();
    assert!(results.iter().all(|r| r.get("roundtrip").is_some()), "native texture replacement regression: {results:?}");
}

















