//! Equipment 4.5 G2 (receipt 42; research R125 / R125b, 2026-09-26 handoff 18 §1 / §3 / G2): a team's goalie
//! mask paints (`exhibitionteams.goaliemask<k>_{starter,backup}`) with the file check a goalie's own `maskid`
//! gets. Anchors for the slot shape per title; the real X360 loose trees (read only) for the file check;
//! the PS4 anchor and the private genuine NHL 20 database for the shipped exceptions, reported not refused.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::team_goalie_masks::{team_goalie_masks, MaskPaintStatus};
use studio_domain::{EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn open_at(db: &Path, meta: &Path, platform: Platform, game: &str) -> EditSession {
    let mut s = EditSession::begin(Project::load(db, platform, ProjectRole::Nhlng, meta, repo("defs")).unwrap());
    s.set_game_id(Some(game.to_string()));
    s
}

fn anchor(game: &str, platform: Platform) -> EditSession {
    let (dir, db, meta) = match platform {
        Platform::Ps4 => ("ps4", "nhl_game.db", "nhl_game-meta.xml"),
        Platform::Ps3 => ("ps3", "nhlng.db", "nhlng-meta.xml"),
        _ => ("xbox360", "nhlng.db", "nhlng-meta.xml"),
    };
    let (db, meta) = if game == "nhl07" { ("NHL07NG.db", "NHL07NG-meta.xml") } else { (db, meta) };
    let base = repo(&format!("anchors/{dir}/{game}/databases"));
    open_at(&base.join(db), &base.join(meta), platform, game)
}

/// Per status: how many stored paints across every team of the session; plus the slot count per team.
fn sweep(s: &EditSession, rendering: Option<&Path>, game: &str, platform: Platform) -> (BTreeMap<&'static str, usize>, usize, BTreeMap<u32, usize>, bool) {
    let mut by: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut values: BTreeMap<u32, usize> = BTreeMap::new();
    let mut slots = 0;
    let mut applies = false;
    for t in s.team_summaries().unwrap() {
        let v = s.team_view(t.team_id).unwrap().unwrap();
        let m = team_goalie_masks(&v.all_fields, rendering, Some(game), platform);
        applies |= m.applies;
        slots = slots.max(m.paints.iter().map(|p| p.slot + 1).max().unwrap_or(0) as usize);
        for p in &m.paints {
            *by.entry(p.status.as_str()).or_default() += 1;
            if let Some(v) = p.value.filter(|v| *v > 0) {
                *values.entry(v).or_default() += 1;
            }
            if p.status == MaskPaintStatus::Files {
                for f in &p.files {
                    let stem = f.rsplit('/').next().unwrap().split('.').next().unwrap();
                    let n: Vec<u32> = stem.trim_start_matches("goaliehelmet_").split('_').filter_map(|x| x.parse().ok()).collect();
                    assert_eq!(n.get(1).copied(), p.value, "{game}: {f} names paint {:?} at the second position", p.value);
                }
            }
        }
    }
    (by, slots, values, applies)
}

#[test]
fn slot_shape_per_title_on_the_anchors() {
    for (game, platform, want_slots) in [("nhl07", Platform::Xbox, 0usize), ("nhl08", Platform::Xbox, 0), ("nhl09", Platform::Xbox, 0), ("nhl10", Platform::Xbox, 6), ("nhl11", Platform::Xbox, 7), ("nhllegacy", Platform::Xbox, 7), ("nhl15", Platform::Ps4, 7), ("nhl21", Platform::Ps4, 9)] {
        let s = anchor(game, platform);
        let (by, slots, values, applies) = sweep(&s, None, game, platform);
        assert_eq!(applies, want_slots > 0, "{game}: team mask slots exist from NHL 10");
        assert_eq!(slots, want_slots, "{game}: slot count");
        eprintln!("{game} {platform:?}: {slots} slots; statuses {by:?}; {} distinct non-zero paints", values.len());
        if platform == Platform::Ps4 {
            let exc = by.get("shipped_exception").copied().unwrap_or(0);
            if game == "nhl21" {
                assert!(exc > 0, "NHL 21 ships 248 / 249 in team slots (R125b): reported as exceptions");
                assert!(values.contains_key(&248) || values.contains_key(&249));
            } else {
                assert_eq!(exc, 0, "NHL 15 has no shipped exception");
            }
        } else if applies {
            assert_eq!(by.get("no_folder").copied().unwrap_or(0) + by.get("empty").copied().unwrap_or(0), by.values().sum::<usize>(), "{game}: without a tree, every non-zero paint says there is no folder to check");
        }
    }
}

fn x360_rendering(game: &str) -> PathBuf {
    let n = &game[3..];
    let tree = if game == "nhllegacy" { r"NHL Legacy\NHL Legacy".to_string() } else { format!(r"NHL {n}\NHL {n}") };
    PathBuf::from(r"X:\Games\Emulation\Roms\XBOX360").join(tree).join("rendering")
}

#[test]
#[ignore = "real X360 NHL 10 → Legacy trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_stored_team_paint_names_files_on_the_x360_trees() {
    // Research measured "all paint ids" on NHL 10, 11, 13, 15 and Legacy; 12 and 14 were not run.
    for game in ["nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"] {
        let r = x360_rendering(game);
        assert!(r.join("goaliehelmet").is_dir(), "{}: missing (fails, never skips)", r.display());
        let s = anchor(game, Platform::Xbox);
        let (by, slots, values, _) = sweep(&s, Some(&r), game, Platform::Xbox);
        let no_files = by.get("no_files").copied().unwrap_or(0);
        eprintln!("{game}: {slots} slots; statuses {by:?}; {} distinct non-zero paints", values.len());
        assert!(by.get("files").copied().unwrap_or(0) > 0, "{game}: some team paint names files");
        assert_eq!(by.get("no_folder"), None);
        if matches!(game, "nhl10" | "nhl11" | "nhl13" | "nhl15" | "nhllegacy") {
            assert_eq!(no_files, 0, "{game}: every non-zero team paint is a paint file id (R125)");
        }
    }
}

#[test]
#[ignore = "private fixture databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn genuine_nhl20_reports_the_shipped_exceptions() {
    let root = PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    let s = open_at(&root.join("nhl_game.db"), &root.join("nhl_game-meta.xml"), Platform::Ps4, "nhl20");
    let (by, slots, values, applies) = sweep(&s, None, "nhl20", Platform::Ps4);
    eprintln!("nhl20 (genuine): {slots} slots; statuses {by:?}; 248 × {:?}, 249 × {:?}", values.get(&248), values.get(&249));
    assert!(applies && slots == 9);
    assert!(by.get("shipped_exception").copied().unwrap_or(0) > 0, "248 / 249 reported");
    assert_eq!(by.get("files"), None, "PS4 paints are catalogue records, not files");
}
