//! Per-title **capability probing** — which optional schema features the *loaded* database actually
//! carries, so the UI can hide surfaces a game doesn't have instead of rendering blanks.
//!
//! The flags are probed from the file itself (does this table carry that field descriptor?), never
//! assumed from the title id — an unproven (title, platform) gates honestly by evidence, per the
//! version-aware hard rule. The probe list mirrors the optional studio ids the curated views read
//! (`view::build_player_data` / `build_player_header`, the team→arena join); universal fields
//! (verified present in every anchor database, NHL 10–21 × PS3/Xbox/PS4) are not probed.
//!
//! Cross-title provenance: the 21-anchor schema diff of 2026-07-03 — `kOtt.buDa` absent in NHL
//! 10–12; `ubPc` draft/junior/UFA fields absent in NHL 10 (round also 11, AHL/CHL rookie flags
//! through 13); `QgFs` (style) absent in NHL 10–13. Ratings are counted through the era-spanning
//! metadata classification in [`crate::attributes`] (`p_`/`g_`/`c_` name prefixes).

use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

use crate::attributes::{rating_label, rating_offset, rating_suffix};

/// Which optional schema features the loaded database carries. `true` = the backing field exists in
/// this title's schema (not whether any record has a meaningful value).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    /// The team→arena link exists (`kOtt.buDa`) — the Team detail's Arena card.
    pub arena_link: bool,
    /// The player/goalie style (archetype) enum exists (`QgFs` on either ratings companion).
    pub player_style: bool,
    /// Year of UFA eligibility (`ubPc.GKKz`).
    pub year_ufa: bool,
    /// Junior (rights) team ref (`ubPc.ZbNy`).
    pub junior_team: bool,
    /// Draft team ref (`ubPc.vgWu`).
    pub draft_team: bool,
    /// Draft year (`ubPc.YKzW`).
    pub draft_year: bool,
    /// Draft round (`ubPc.ccjU`).
    pub draft_round: bool,
    /// Draft position/pick (`ubPc.tTfW`).
    pub draft_pick: bool,
    /// AHL rookie flag (`ubPc.ywRO`).
    pub rookie_ahl: bool,
    /// CHL rookie flag (`ubPc.BQVU`).
    pub rookie_chl: bool,
    /// The separate skater *equipment* companion table (`xmja`) exists. Gen6 titles (NHL07) fold
    /// ratings onto the AI table and ship no equipment table, so its absence is a schema shape, not a
    /// broken cluster — a future per-player equipment surface gates on this instead of rendering blank.
    pub has_skater_equip: bool,
    /// The separate goalie *equipment* companion table (`fMVl`) exists (see `has_skater_equip`).
    pub has_goalie_equip: bool,
    /// Skater rating fields this database carries (classified from the metadata — see
    /// [`rating_offset`]) that have an operator label in our vocabulary. When
    /// `< skater_ratings_total` the loaded schema has ratings we can't yet name — the Attributes
    /// tab should hedge visibly.
    pub skater_ratings_labelled: usize,
    /// All skater rating fields classified in this database.
    pub skater_ratings_total: usize,
    /// Labelled goalie rating fields (see `skater_ratings_labelled`).
    pub goalie_ratings_labelled: usize,
    /// All goalie rating fields classified in this database.
    pub goalie_ratings_total: usize,
    /// How `ubPc.dXSB` (the "main team" pointer) encodes the team id in this title: `0` on NHL
    /// 09/10 (raw = team id) or `1` on NHL Legacy/11→21 (raw = team id + 1, 0 = free agent). Probed
    /// from the roster ground truth ([`crate::probe_main_team_encoding`]) so the UI's Main-Team
    /// picker reads and writes the field with the right offset instead of assuming the modern one.
    pub main_team_offset: u8,
}

impl Capabilities {
    /// Probe the loaded database for every optional feature the curated views depend on. The
    /// metadata index (the title's own `-meta.xml`) drives the rating classification; the boolean
    /// flags come from the file's field descriptors alone.
    pub fn probe(db: &TdbFile, meta: &FieldMetaIndex) -> Capabilities {
        let (skater_ratings_labelled, skater_ratings_total) = count_ratings(db, meta, "dSvy");
        let (goalie_ratings_labelled, goalie_ratings_total) = count_ratings(db, meta, "mHuy");
        // The main-team offset needs the roster graph (it's recovered from membership ground truth);
        // fall back to the modern 1-indexed convention if the entity model can't be built here.
        let main_team_offset = tdb_relations::RosterModel::build(db)
            .map(|m| crate::probe_main_team_encoding(db, &m).offset() as u8)
            .unwrap_or(1);
        Capabilities {
            arena_link: has_field(db, "kOtt", "buDa"),
            // The style enum arrives on both companions together; either counts (`QgFs` on NHL
            // 14+). NHL07 gen6 stores it under `mQoE`/`wJus` with a known 3-bit vocabulary; NHL
            // 10→13 reuse those ids with a wider, unverified code space, so they don't count here.
            player_style: has_field(db, "dSvy", "QgFs")
                || has_field(db, "mHuy", "QgFs")
                || crate::attributes::is_gen6_style_schema(db),
            year_ufa: has_field(db, "ubPc", "GKKz"),
            junior_team: has_field(db, "ubPc", "ZbNy"),
            draft_team: has_field(db, "ubPc", "vgWu"),
            draft_year: has_field(db, "ubPc", "YKzW"),
            draft_round: has_field(db, "ubPc", "ccjU"),
            draft_pick: has_field(db, "ubPc", "tTfW"),
            rookie_ahl: has_field(db, "ubPc", "ywRO"),
            rookie_chl: has_field(db, "ubPc", "BQVU"),
            has_skater_equip: has_table(db, "xmja"),
            has_goalie_equip: has_table(db, "fMVl"),
            skater_ratings_labelled,
            skater_ratings_total,
            goalie_ratings_labelled,
            goalie_ratings_total,
            main_team_offset,
        }
    }
}

/// `true` iff `studio_table` exists in this file (table presence, independent of any field).
fn has_table(db: &TdbFile, studio_table: &str) -> bool {
    db.table_canonical(studio_table).is_some()
}

/// `true` iff `studio_table` exists in this file and carries a field with the given studio id.
fn has_field(db: &TdbFile, studio_table: &str, studio_field: &str) -> bool {
    db.table_canonical(studio_table)
        .map(|t| t.fields.iter().any(|f| f.id.canonical(db.endian) == studio_field))
        .unwrap_or(false)
}

/// `(labelled, total)` rating fields on a companion table, classified from each field's metadata
/// (name prefix + declared range — the same rule the views render by).
fn count_ratings(db: &TdbFile, meta: &FieldMetaIndex, studio_table: &str) -> (usize, usize) {
    let Some(table) = db.table_canonical(studio_table) else {
        return (0, 0);
    };
    let mut labelled = 0;
    let mut total = 0;
    for f in &table.fields {
        let Some(m) = meta.field(&table.id.raw(), &f.id.raw()) else {
            continue;
        };
        if rating_offset(studio_table, &m.name, m.range_low, m.range_high).is_none() {
            continue;
        }
        total += 1;
        if rating_suffix(&m.name).and_then(|s| rating_label(studio_table, s)).is_some() {
            labelled += 1;
        }
    }
    (labelled, total)
}
