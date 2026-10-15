//! Equipment 4.5 — which files a goalie's gear numbers name (receipt 40; research R125 = 2026-09-26
//! handoff 18 §1, R125b). The Player → Equipment file check listed NHL 12 pads from the **first**
//! number of `goaliepad_*` names; research measured the id in the **second** position on NHL 12
//! (`fegoaliepad_0_11_0_dm`) and in the first from NHL 13 (`fegoaliepad_11_0_dm`). And NHL 08 / 09 carry
//! the mask paint in `maskcolor`, the third number of `goaliehelmet_<a>_<b>_<paint>_<d>_cm`, which had
//! no check. Real loose trees, read only; a naming match, not what the game draws.
use std::path::{Path, PathBuf};

use studio_domain::{check_player_equipment_files, slot_spec, EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn open(game: &str) -> EditSession {
    let dir = repo(&format!("anchors/xbox360/{game}/databases"));
    let mut s = EditSession::begin(Project::load(dir.join("nhlng.db"), Platform::Xbox, ProjectRole::Nhlng, dir.join("nhlng-meta.xml"), repo("defs")).unwrap());
    s.set_game_id(Some(game.to_string()));
    s
}

fn rendering(game: &str) -> PathBuf {
    let n = &game[3..];
    let tree = match game {
        "nhl09" => r"NHL 09\NHL 09 (USA)".to_string(),
        "nhllegacy" => r"NHL Legacy\NHL Legacy".to_string(),
        _ => format!(r"NHL {n}\NHL {n}"),
    };
    PathBuf::from(r"X:\Games\Emulation\Roms\XBOX360").join(tree).join("rendering")
}

/// The numeric tokens of a file name after its prefix (`fegoaliepad_0_11_0_dm.rx2` → [0, 11, 0]).
fn numbers(file: &str, prefix: &str) -> Option<Vec<u32>> {
    let name = file.rsplit('/').next()?;
    let stem = name.split('.').next()?;
    let rest = stem.strip_prefix(prefix)?.strip_prefix('_')?;
    Some(rest.split('_').filter_map(|t| t.parse().ok()).collect())
}

#[test]
#[ignore = "real X360 NHL 09 / 12 / 13 / Legacy trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn pads_follow_the_measured_position_per_title_and_nhl09_mask_paint_is_checked() {
    for (game, position) in [("nhl12", 1usize), ("nhl13", 0), ("nhllegacy", 0)] {
        let r = rendering(game);
        assert!(r.is_dir(), "{}: missing (fails, never skips)", r.display());
        let s = open(game);
        let mut checked = 0;
        for p in s.player_summaries().unwrap().into_iter().filter(|p| p.is_goalie && p.team_id.is_some()).take(12) {
            let v = s.player_view(p.player_id).unwrap().unwrap();
            let checks = check_player_equipment_files(&r, true, &v.equipment, s.project().enums(), Some(game), Platform::Xbox);
            let pads = checks.iter().find(|c| c.slot == "pads").expect("pads slot");
            let Some(id) = pads.id.filter(|i| *i > 0) else { continue };
            assert!(!pads.files.is_empty(), "{game}: pads {id} lists no file ({})", pads.note);
            for f in pads.files.iter().filter(|f| !f.contains("texlib_")) {
                let prefix = if f.contains("fegoaliepad") { "fegoaliepad" } else { "goaliepad" };
                let n = numbers(f, prefix).unwrap();
                assert_eq!(n.get(position).copied(), Some(id), "{game}: {f} names pads {id} at position {}", position + 1);
            }
            checked += 1;
        }
        assert!(checked >= 3, "{game}: {checked} goalies with non-zero pads");
        eprintln!("{game}: pads files carry the id at position {} for {checked} goalies", position + 1);
    }
    // The pre-fix reading on NHL 12 (first position) names different files — the defect.
    let r12 = rendering("nhl12");
    let default = slot_spec("pads").unwrap();
    let s = open("nhl12");
    let g = s.player_summaries().unwrap().into_iter().filter(|p| p.is_goalie && p.team_id.is_some()).find_map(|p| {
        let v = s.player_view(p.player_id).unwrap().unwrap();
        let pads = check_player_equipment_files(&r12, true, &v.equipment, s.project().enums(), Some("nhl12"), Platform::Xbox).into_iter().find(|c| c.slot == "pads")?;
        pads.id.filter(|i| *i > 0).map(|i| (i, pads.files))
    });
    let (id, fixed_files) = g.expect("an NHL 12 goalie with pads");
    let old: Vec<String> = studio_domain::list_equipment_slot_assets(&r12, default).into_iter().find(|a| a.id == id).map(|a| a.files).unwrap_or_default();
    assert_ne!(old, fixed_files, "NHL 12 pads {id}: the first-position reading names other files");
    eprintln!("nhl12 pads {id}: first-position reading listed {} file(s), the measured position {}", old.len(), fixed_files.len());

    // NHL 09 mask paint (`maskcolor`): files carry the value at the third position.
    let r9 = rendering("nhl09");
    assert!(r9.is_dir(), "{}: missing", r9.display());
    let s = open("nhl09");
    let mut seen = 0;
    for p in s.player_summaries().unwrap().into_iter().filter(|p| p.is_goalie && p.team_id.is_some()).take(15) {
        let v = s.player_view(p.player_id).unwrap().unwrap();
        let checks = check_player_equipment_files(&r9, true, &v.equipment, s.project().enums(), Some("nhl09"), Platform::Xbox);
        let paint = checks.iter().find(|c| c.slot == "goalie-mask-paint").expect("NHL 09 mask paint check");
        let Some(id) = paint.id.filter(|i| *i > 0) else { continue };
        assert!(!paint.files.is_empty(), "nhl09 mask paint {id}: {}", paint.note);
        for f in &paint.files {
            assert_eq!(numbers(f, "goaliehelmet").unwrap().get(2).copied(), Some(id), "nhl09: {f} names paint {id}");
        }
        seen += 1;
    }
    assert!(seen >= 3, "nhl09: {seen} goalies with a mask paint");
    eprintln!("nhl09: mask paint files checked for {seen} goalies");
    // NHL 10 has no `maskcolor`: no paint slot (its paint is `maskid`, the Goalie Helmet slot).
    let s = open("nhl10");
    let p = s.player_summaries().unwrap().into_iter().find(|p| p.is_goalie && p.team_id.is_some()).unwrap();
    let v = s.player_view(p.player_id).unwrap().unwrap();
    assert!(check_player_equipment_files(&rendering("nhl10"), true, &v.equipment, s.project().enums(), Some("nhl10"), Platform::Xbox).iter().all(|c| c.slot != "goalie-mask-paint"));
}
