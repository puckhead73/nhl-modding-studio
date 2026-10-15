//! Receipt 51 — G3 through the Registry: genuine NHL 20 (private fixture database) with its install attached
//! (`H:\Emulators\ShadPS4\Games\CUSA15439`, only `boot.big` read): a goalie's pair, the team paints' helmet types, the
//! cache, and an NHL 19 project refused against the NHL 20 install.
use std::path::{Path, PathBuf};

use studio_api::dto::AssetRootsArgsDto;
use studio_api::{parse_platform, Registry};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

#[test]
#[ignore = "genuine NHL 20 private fixture (NHL_STUDIO_FIXTURES) and the PS4 install under H:/Emulators/ShadPS4/Games: select with --ignored"]
fn goalie_pair_and_team_paints_through_the_registry() {
    let fx = PathBuf::from(std::env::var("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES (fails, never skips)")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    let s = |p: &Path| p.to_string_lossy().into_owned();
    let defs = repo("defs");
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl20"), parse_platform("ps4").unwrap(), &s(&fx.join("nhl_game.db")), None, &s(&fx.join("nhl_game-meta.xml")), &s(&defs)).unwrap().project_id;
    let args = AssetRootsArgsDto { install_dir: r"H:\Emulators\ShadPS4\Games\CUSA15439".into(), platform: Some("ps4".into()), rendering_override: None, fe_override: None };
    // Every goalie: applies, classified; at least one unique and (R135 §3) type-99 goalies reported, never an error.
    let goalies: Vec<u64> = reg.list_players(id).unwrap().into_iter().filter(|p| p.is_goalie).map(|p| p.player_id).take(400).collect();
    assert!(!goalies.is_empty());
    let t0 = std::time::Instant::now();
    let mut classes = std::collections::BTreeMap::new();
    let mut named = 0;
    for g in &goalies {
        let d = reg.ps4_goalie_mask_pair(id, &args, *g, &defs).unwrap();
        assert!(d.applies && d.catalogue_problem.is_none(), "{:?}", d.catalogue_problem);
        // Receipt 57: a list row's name is the title's text, never a key, template or placeholder.
        if let Some(n) = &d.mask_name {
            assert!(!n.starts_with('$') && !n.contains('%') && !n.contains("GoalieMask"), "{n}");
            named += 1;
        }
        *classes.entry(d.class.unwrap()).or_insert(0) += 1;
    }
    eprintln!("{} goalies in {:.1} s (one vault read, then cached): {classes:?}", goalies.len(), t0.elapsed().as_secs_f32());
    assert!(classes.get("unique").copied().unwrap_or(0) > 0);
    assert!(named > 0, "some goalie's pair is a named list row");
    // Team paints: PS4 values listed with helmet types; PRSA's 248 has no record.
    let teams = reg.list_teams(id).unwrap();
    let prsa = teams.iter().find(|t| t.abbrev == "TPR").map(|t| t.team_id).expect("TPR (PRSA)");
    let m = reg.team_goalie_masks_with_defs(id, prsa, &args, Some(&defs)).unwrap();
    let p248 = m.paints.iter().find(|p| p.value == Some(248)).expect("paint 248");
    assert_eq!(p248.helmet_types.as_deref(), Some(&[][..]), "no catalogue record");
    assert!(m.notes.iter().any(|n| n.contains("vault catalogue")), "{:?}", m.notes);
    // An NHL 19 project against the NHL 20 install: refused as a source mismatch (numbers still shown).
    let d19 = repo("anchors/ps4/nhl19/databases");
    let id19 = reg.open_workspace_from_paths(Some("nhl19"), parse_platform("ps4").unwrap(), &s(&d19.join("nhl_game.db")), None, &s(&d19.join("nhl_game-meta.xml")), &s(&defs)).unwrap().project_id;
    let g19 = reg.list_players(id19).unwrap().into_iter().find(|p| p.is_goalie).unwrap().player_id;
    let d = reg.ps4_goalie_mask_pair(id19, &args, g19, &defs).unwrap();
    assert!(d.class.is_none() && d.catalogue_problem.as_deref().unwrap().contains("source mismatch"), "{:?}", d.catalogue_problem);
    assert!(d.text.starts_with("Mask "));
}

/// Receipt 52 — the skater-gear check and a Creation Zone thumbnail through the Registry (genuine NHL 20 + its install).
#[test]
#[ignore = "genuine NHL 20 private fixture (NHL_STUDIO_FIXTURES) and the PS4 install under H:/Emulators/ShadPS4/Games: select with --ignored"]
fn skater_gear_and_thumbnails_through_the_registry() {
    let fx = PathBuf::from(std::env::var("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES (fails, never skips)")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    let s = |p: &Path| p.to_string_lossy().into_owned();
    let defs = repo("defs");
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl20"), parse_platform("ps4").unwrap(), &s(&fx.join("nhl_game.db")), None, &s(&fx.join("nhl_game-meta.xml")), &s(&defs)).unwrap().project_id;
    let args = AssetRootsArgsDto { install_dir: r"H:\Emulators\ShadPS4\Games\CUSA15439".into(), platform: Some("ps4".into()), rendering_override: None, fe_override: None };
    let skaters: Vec<u64> = reg.list_players(id).unwrap().into_iter().filter(|p| !p.is_goalie).map(|p| p.player_id).take(60).collect();
    let mut classes = std::collections::BTreeMap::new();
    let mut thumb = None;
    let mut names = 0;
    for p in &skaters {
        let d = reg.ps4_skater_gear(id, &args, *p, &defs).unwrap();
        assert!(d.applies && d.problem.is_none(), "{:?}", d.problem);
        assert_eq!(d.items.len(), 5);
        assert!(d.notes.iter().any(|n| n.contains("English dictionary in inxp_00.big")), "{:?}", d.notes);
        for it in d.items {
            if it.class.as_deref() == Some("row") {
                // Receipt 57: every list row of genuine NHL 20 resolves (R141: only mask placeholders do not).
                let n = it.name.as_deref().unwrap_or_else(|| panic!("{} {}: no name for {:?}", it.family, it.value, it.key));
                assert!(!n.starts_with('$'), "{n}");
                names += 1;
            }
            *classes.entry((it.family.clone(), it.class.clone().unwrap())).or_insert(0) += 1;
            if thumb.is_none() {
                if let (Some(a), Some(m)) = (it.thumbnail_archive, it.thumbnail_member) {
                    thumb = Some((a, m));
                }
            }
        }
    }
    eprintln!("{} skaters: {classes:?}; {names} named items", skaters.len());
    assert!(names > 0);
    let (a, m) = thumb.expect("a thumbnail");
    let pv = reg.ps4_gear_thumbnail(&args, &a, &m).unwrap();
    assert!(pv.png_data_url.starts_with("data:image/png;base64,") && pv.width > 0);
    // Only Creation Zone equipment members of the install's own archives.
    assert!(reg.ps4_gear_thumbnail(&args, "../boot.big", &m).is_err());
    assert!(reg.ps4_gear_thumbnail(&args, &a, "nhl/data/common/database/attrib/gccollections.vlt").is_err());
}
