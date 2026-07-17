//! Phase 4.4 — roster-save capacity normalization (Spike S3).
//!
//! A game-rewritten roster save can arrive **packed** (`max_records == current_records`), so the
//! very first insert pays a per-record growth and the file drifts from the vanilla shape. The
//! normalization pass pre-allocates each roster-bearing table back up to its **vanilla ceiling** —
//! growing `max_records` only, never inventing population — exactly the shape EA ships
//! (`current < max`, roughly 2× slack).
//!
//! **Every ceiling below is a real byte read, verified in
//! [docs/audits/roster-capacity.md](../../../docs/audits/roster-capacity.md):**
//! * **GEN7 (PS3 ≡ Xbox360)** — the main `nhlng.db` is packed and cannot supply the ceiling; it
//!   lives only in the vanilla PS3 ROSTER save's `max_records`, so GEN7 carries the audited table
//!   as a `game_id → ceilings` constant here.
//! * **GEN8 (PS4)** — the main `nhl_game.db` already carries the slack, so the ceiling is derived
//!   at runtime from the attached main DB's `max_records` (no constants). That derivation is also a
//!   safe *floor* on GEN7 (a packed main's `max == cur ≤ ceiling`), so the pass always takes the
//!   larger of the two sources.

use tdb_relations::entity::ids;

/// The seven roster-bearing tables the ceilings apply to, in the audit's column order:
/// index, bio, skater AI, skater equip, goalie AI, goalie equip, roster slots.
pub const ROSTER_TABLES: [&str; 7] = [
    ids::INDEX,        // ZBac
    ids::BIO,          // ubPc
    ids::SKATER_AI,    // dSvy
    ids::SKATER_EQUIP, // xmja
    ids::GOALIE_AI,    // mHuy
    ids::GOALIE_EQUIP, // fMVl
    ids::ROSTER_LINES, // eGlu
];

/// GEN7 vanilla ROSTER-save `max_records` per studio game id — the ceiling table from
/// docs/audits/roster-capacity.md §"GEN7" (verified byte reads, PS3 ≡ Xbox360 inner TDB).
/// Column order matches [`ROSTER_TABLES`]. `None` for ids outside the GEN7 set (PS4 titles derive
/// their ceiling from the attached main DB instead).
pub fn gen7_ceilings(game_id: &str) -> Option<[u16; 7]> {
    Some(match game_id {
        "nhl10" => [5688, 4533, 3999, 3999, 534, 534, 6219],
        "nhl11" => [7093, 5910, 5260, 5260, 650, 650, 7728],
        "nhl12" => [8283, 6898, 6324, 6324, 874, 874, 7853],
        "nhl13" => [8907, 6946, 6407, 6407, 839, 839, 10457],
        "nhl14" => [8803, 6873, 6350, 6350, 823, 823, 10353],
        "nhl15" => [8948, 7034, 6479, 6479, 855, 855, 10619],
        "nhllegacy" => [9955, 7988, 7331, 7331, 957, 957, 10879],
        _ => return None,
    })
}

/// One table the normalization pass grew: allocation went `from` → `to` (`current_records`
/// untouched).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapacityGrowth {
    /// Canonical table id (e.g. `ZBac`).
    pub table: String,
    pub from: u16,
    pub to: u16,
}
