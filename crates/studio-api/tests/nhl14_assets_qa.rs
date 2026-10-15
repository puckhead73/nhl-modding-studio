//! Opt-in NHL14 QA; input images are read-only and outputs require a fresh folder.
use std::path::{Path, PathBuf};

use studio_api::Registry;
use studio_domain::{discovery, Platform};

fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
#[ignore = "read-only observations of NHL14_QA_ASSETS, writes a new NHL14_QA_OBSERVATIONS receipt"]
fn xbox_asset_and_editing_surfaces() {
    let root = PathBuf::from(std::env::var("NHL14_QA_ASSETS").unwrap());
    let out = PathBuf::from(std::env::var("NHL14_QA_OBSERVATIONS").unwrap());
    assert!(!out.exists());
    let ps3 = std::env::var("NHL14_QA_PLATFORM").as_deref() == Ok("ps3");
    let platform = if ps3 { Platform::Ps3 } else { Platform::Xbox };
    let meta = repo(if ps3 { "anchors/ps3/nhl14/databases/nhlng-meta.xml" } else { "anchors/xbox360/nhl14/databases/nhlng-meta.xml" });
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl14"),platform,
        &s(&root.join("db/nhlng.db")),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    let found = discovery::detect_install("nhl14",platform,&root,None);
    reg.attach_dictionary(id,found.db_paths.localization.as_ref().unwrap()).unwrap();
    let args = studio_api::AssetRootsArgsDto { install_dir:s(&root), platform:Some(platform.as_str().into()), rendering_override:None, fe_override:None };
    let teams = reg.list_teams(id).unwrap();
    let team = teams.iter().find(|t| t.name.contains("Anaheim")).unwrap();
    let player = reg.list_players(id).unwrap().into_iter().find(|p| p.team_id == Some(team.team_id)).unwrap();
    fn obs<T:serde::Serialize,E:std::fmt::Display>(r:Result<T,E>)->serde_json::Value {
        match r {Ok(v)=>serde_json::json!({"value":v}),Err(e)=>serde_json::json!({"error":e.to_string()})}
    }
    let leagues = reg.list_leagues(id).unwrap();
    let logos:Vec<_> = leagues.leagues.iter().filter(|l| l.code>=0).map(|l|obs(reg.league_logo(id,l.code as u64,Some(&s(&root))))).collect();
    let arenas=reg.list_arenas(id).unwrap();
    let arena_art:Vec<_>=arenas.iter().take(2).map(|a|obs(reg.arena_assets(id,&args,a.arena_id))).collect();
    let outfits = reg.team_outfits(id,&args,team.team_id).unwrap();
    let equipment = reg.player_equipment_files(id,player.player_id,&args,&repo("defs")).unwrap();
    let gloves = equipment.iter().find(|e| e.slot == "gloves").unwrap();
    assert!(gloves.file_count > 0, "stock gloves must resolve past the style prefix");
    assert_eq!(gloves.set_contract, "measured");
    assert!(!outfits.is_empty(), "extracted native uniforms must be discoverable");
    assert!(outfits.iter().any(|t| t.piece == "jersey" && t.label.contains("color map")), "jersey color must not be hidden behind the number font");
    assert!(outfits.iter().any(|t| t.rel_path.replace('\\', "/") == if ps3 { "rendering/jersey/texlib_0_0_0.rpsgl" } else { "rendering/jersey/texlib_0_0_0.rx2" }
        && t.piece == "jersey" && t.label.contains("color map") && t.texture_index == if ps3 { 3 } else { 2 }));
    if ps3 {
        for index in 0..6 {
            let (_, decoded) = studio_domain::decode_texture_file(&root.join("rendering/jersey/texlib_0_0_0.rpsgl"),Some(index)).unwrap();
            let preview = out.with_extension(format!("slot-{index}.png"));
            assert!(!preview.exists());
            std::fs::write(preview, decoded.to_png().unwrap()).unwrap();
        }
    }
    let receipt=serde_json::json!({
        "team":obs(reg.team_view(id,team.team_id)),"player":obs(reg.player_view(id,player.player_id)),
        "leagues":leagues,"league_logos":logos,"arenas":arenas,"arena_art":arena_art,
        "team_logos":obs(reg.team_logo_set(id,team.team_id,Some(&s(&root)))),
        "kits":obs(reg.team_kits(id,&args,team.team_id)),"outfits":{"value":outfits},
        "lines":obs(reg.lines_view(id,team.team_id)),"equipment":{"value":equipment},
        "player_assets":obs(reg.list_player_assets(id,&args)),"game_verified":false
    });
    std::fs::write(out,serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}

#[test]
#[ignore = "copies native textures into a fresh NHL14_QA_TEXTURES directory"]
fn nhl14_native_texture_replace_roundtrip() {
    let source = PathBuf::from(std::env::var("NHL14_QA_ASSETS").expect("copied extracted disc"));
    let out = PathBuf::from(std::env::var("NHL14_QA_TEXTURES").expect("fresh texture output"));
    assert!(!out.exists(), "preserve earlier evidence");
    std::fs::create_dir_all(&out).unwrap();
    let reg = Registry::default();
    let mut results = Vec::new();
    for rel in ["fe/ion/artassets/teamlogos/t0.big", "fe/ion/artassets/playerheads/p0_4000/p1.big", "fe/ion/artassets/leaguelogos/l0.big",
        "rendering/jersey/texlib_0_0_0.rx2",
        "rendering/playerstick/texlib_24.rx2", "rendering/icesurface/centerlogo_0_cm.rx2"] {
        // PS3 uses the corresponding native RenderWare PSGL package suffix.
        let native = if std::env::var("NHL14_QA_PLATFORM").as_deref() == Ok("ps3") {
            rel.replace(".rx2", ".rpsgl").replace("/playerheads/", "/playerheadssmall/")
        } else { rel.to_string() };
        // Assert the discovered color slot separately; use the title-native raster here.
        let index = if rel == "rendering/jersey/texlib_0_0_0.rx2" {
            if std::env::var("NHL14_QA_PLATFORM").as_deref() == Ok("ps3") { 3 } else { 2 }
        } else { 0 };
        let rel = native.as_str();
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















