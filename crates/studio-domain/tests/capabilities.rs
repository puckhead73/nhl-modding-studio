//! Capability probing verified against the anchor databases across the title range: NHL Legacy
//! (PS3) carries the full optional set, NHL 10 gates off everything era-gated, NHL 12 sits in
//! between, and PS4 NHL 21 (little-endian) probes cleanly too. Presence expectations come from the
//! 21-anchor schema diff of 2026-07-03.
//!
//! Rating counts go through the era-spanning metadata classification (`p_`/`g_`/`c_` prefixes +
//! declared range), so the probe needs each title's `-meta.xml`. NHL 10 uses the Xbox 360 anchor —
//! the PS3 NHL 10 anchor ships no meta XML.

use std::path::{Path, PathBuf};

use studio_domain::Capabilities;
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn probe(rel_db: &str, rel_meta: &str) -> Capabilities {
    let bytes = std::fs::read(repo(rel_db)).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta =
        FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(rel_meta)).unwrap()).unwrap();
    Capabilities::probe(&db, &meta)
}

/// The proven era (Legacy): every optional feature present; the classic 26 skater / 21 goalie
/// ratings all classify and all carry labels.
#[test]
fn legacy_carries_the_full_optional_set() {
    let c = probe(
        "anchors/ps3/nhllegacy/databases/nhlng.db",
        "anchors/ps3/nhllegacy/databases/nhlng-meta.xml",
    );
    assert!(c.arena_link);
    assert!(c.player_style);
    assert!(c.year_ufa);
    assert!(c.junior_team);
    assert!(c.draft_team);
    assert!(c.draft_year);
    assert!(c.draft_round);
    assert!(c.draft_pick);
    assert!(c.rookie_ahl);
    assert!(c.rookie_chl);
    assert_eq!(c.skater_ratings_total, 26);
    assert_eq!(c.skater_ratings_labelled, 26);
    assert_eq!(c.goalie_ratings_total, 21);
    assert_eq!(c.goalie_ratings_labelled, 21);
}

/// NHL 10 (Xbox anchor): no arena link, no draft/junior/UFA fields, no style — but its `p_*`/`g_*`
/// ratings classify and label fully through the era-spanning vocabulary (23 skater incl. the
/// pre-split Checking/Deflections/Toughness; 20 goalie incl. Floppiness — its 2-bit `g_consistency`
/// correctly classifies as a non-rating).
#[test]
fn nhl10_gates_features_but_ratings_resolve_by_name() {
    let c = probe(
        "anchors/xbox360/nhl10/databases/nhlng.db",
        "anchors/xbox360/nhl10/databases/nhlng-meta.xml",
    );
    assert!(!c.arena_link);
    assert!(!c.player_style);
    assert!(!c.year_ufa);
    assert!(!c.junior_team);
    assert!(!c.draft_team);
    assert!(!c.draft_year);
    assert!(!c.draft_round);
    assert!(!c.draft_pick);
    assert!(!c.rookie_ahl);
    assert!(!c.rookie_chl);
    assert_eq!(c.skater_ratings_total, 23);
    assert_eq!(c.skater_ratings_labelled, 23);
    assert_eq!(c.goalie_ratings_total, 20);
    assert_eq!(c.goalie_ratings_labelled, 20);
}

/// NHL 12: draft fields have arrived (incl. round, new in 12) but the arena link, style, and the
/// AHL/CHL rookie flags haven't yet; every `p_*` rating labels.
#[test]
fn nhl12_is_the_in_between_shape() {
    let c = probe(
        "anchors/ps3/nhl12/databases/nhlng.db",
        "anchors/ps3/nhl12/databases/nhlng-meta.xml",
    );
    assert!(!c.arena_link);
    assert!(!c.player_style);
    assert!(c.draft_team);
    assert!(c.draft_round);
    assert!(c.year_ufa);
    assert!(c.junior_team);
    assert!(!c.rookie_ahl);
    assert!(!c.rookie_chl);
    assert_eq!(c.skater_ratings_labelled, c.skater_ratings_total);
    assert_eq!(c.goalie_ratings_labelled, c.goalie_ratings_total);
}

/// NHL07 (gen6 schema, `NHL07NG.db`): the oldest supported title. It carries no separate equipment
/// tables (`xmja`/`fMVl` absent), no arena link, and none of the draft/junior/UFA/rookie bio fields —
/// so every optional flag gates off. Its ratings still classify from the `p_*`/`g_*` metadata (16 of
/// 20 skater and 13 of 15 goalie carry labels; the rest classify but aren't yet named, so the
/// Attributes tab hedges). `dXSB` main-team is 0-indexed (raw = team id).
#[test]
fn nhl07_is_the_gen6_shape() {
    let c = probe(
        "anchors/xbox360/nhl07/databases/NHL07NG.db",
        "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml",
    );
    assert!(!c.has_skater_equip, "NHL07 ships no xmja equipment table");
    assert!(!c.has_goalie_equip, "NHL07 ships no fMVl equipment table");
    assert!(!c.arena_link);
    // NHL07 DOES carry player types — under the gen6 3-bit `p_style`/`g_style` (`mQoE`/`wJus`), not
    // `QgFs` (verified in-game: skater Playmaker/Sniper/…; goalie Butterfly/Stand-Up/Hybrid).
    assert!(c.player_style, "NHL07 has gen6 player types (mQoE/wJus)");
    assert!(!c.year_ufa);
    assert!(!c.junior_team);
    assert!(!c.draft_team);
    assert!(!c.draft_year);
    assert!(!c.draft_round);
    assert!(!c.draft_pick);
    assert!(!c.rookie_ahl);
    assert!(!c.rookie_chl);
    // 19 of 20 skater ratings now carry labels (Injury Resist / Penalty Resist / Intensity added);
    // only the internal `prestige` slider stays unnamed. Goalies: 14 of 15 (Intensity / Clutch added).
    assert_eq!(c.skater_ratings_total, 20);
    assert_eq!(c.skater_ratings_labelled, 19);
    assert_eq!(c.goalie_ratings_total, 15);
    assert_eq!(c.goalie_ratings_labelled, 14);
    assert_eq!(c.main_team_offset, 0, "raw dXSB = team id (verified against the roster)");
}

/// PS4 NHL 21 (little-endian): the full optional set probes present; the `c_*` ratings (7-bit
/// 0..99 storage, incl. 18+'s Backskating) all classify and label — the canonical-id and metadata
/// lookups are endian-correct on LE files too.
#[test]
fn ps4_nhl21_probes_clean_on_little_endian() {
    let c = probe(
        "anchors/ps4/nhl21/databases/nhl_game.db",
        "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
    );
    assert!(c.arena_link);
    assert!(c.player_style);
    assert!(c.draft_team);
    assert!(c.draft_year);
    assert!(c.rookie_ahl);
    assert!(c.rookie_chl);
    assert_eq!(c.skater_ratings_total, 27, "26 classic + backskating");
    assert_eq!(c.skater_ratings_labelled, 27);
    assert_eq!(c.goalie_ratings_labelled, c.goalie_ratings_total);
    assert!(c.goalie_ratings_total >= 21);
}
