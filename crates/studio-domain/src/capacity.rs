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

use tdb_core::TdbFile;
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

// ---------------------------------------------------------------------------
// Roster Builder capacity projection (Phase 5.4 / Spike S4)
// ---------------------------------------------------------------------------
//
// The write path grows a roster table past its vanilla ceiling freely (it only hard-errors at
// `u16::MAX`), so a large import would silently produce an over-capacity save the game rejects. The
// ceiling gate therefore lives in the **preview**: project each roster table's fill after the cart's
// creates and flag any that would cross its ceiling (docs/audits/capacity-full-policy.md).

/// Friendly labels for [`ROSTER_TABLES`], in the same column order — the human-readable surface for
/// the capacity bars (raw canonical id rides along in `TableCapacity::table`).
const ROSTER_TABLE_LABELS: [&str; 7] = [
    "Player index",
    "Player bios",
    "Skater ratings",
    "Skater equipment",
    "Goalie ratings",
    "Goalie equipment",
    "Roster slots",
];

/// New player clusters a cart adds, by kind. A create appends **one** row to the shared index/bio/slot
/// tables plus the **kind-correct** AI + equipment companions; an update (Match) adds nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingCreates {
    pub skaters: u32,
    pub goalies: u32,
}

impl PendingCreates {
    /// Rows added to the roster table at `ROSTER_TABLES[i]` by these creates.
    fn adding_for(&self, i: usize) -> u32 {
        match i {
            0 | 1 | 6 => self.skaters + self.goalies, // index, bio, slots — shared
            2 | 3 => self.skaters,                    // skater AI / equip
            4 | 5 => self.goalies,                    // goalie AI / equip
            _ => 0,
        }
    }
}

/// One roster table's projected fill vs its vanilla ceiling after the pending creates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableCapacity {
    /// Canonical table id (`ubPc`, …) — raw id for progressive disclosure.
    pub table: String,
    /// Human label for the bar.
    pub label: String,
    pub current: u32,
    /// The vanilla ceiling the game runtime accepts (GEN7 constant, else the db's own `max_records`).
    pub ceiling: u32,
    /// Rows this cart would add to the table.
    pub adding: u32,
    /// `current + adding > ceiling` — the storage blocker (only when `enforced`).
    pub over: bool,
    /// Whether the ceiling is reliable enough to block on. `false` when there is no ceiling source (a
    /// packed file opened without a known title), so an unknown headroom never falsely blocks.
    pub enforced: bool,
}

/// Project each roster-bearing table's fill after `creates`, given the loaded db and its title. The
/// ceiling is the larger of the GEN7 constant for `game_id` (the real ceiling on packed GEN7 mains,
/// whose own `max_records == current`) and the db's own `max_records` (which carries the ceiling on
/// PS4 mains and pre-grown mirrors). A table absent from this title's schema is skipped. When no
/// reliable ceiling exists (unknown title **and** a packed table), the row is reported unenforced so
/// preview never blocks on a headroom it cannot know.
pub fn project_capacity(db: &TdbFile, game_id: Option<&str>, creates: PendingCreates) -> Vec<TableCapacity> {
    let gen7 = game_id.and_then(gen7_ceilings);
    let mut out = Vec::new();
    for (i, cid) in ROSTER_TABLES.iter().enumerate() {
        let Some(t) = db.table_canonical(cid) else {
            continue; // title lacks this table (e.g. gen6 has no equip tables)
        };
        let current = t.current_records as u32;
        let max_rec = t.max_records as u32;
        let gen7_ceiling = gen7.map(|c| c[i] as u32);
        // The larger of the (known) GEN7 constant and the file's own allocation.
        let ceiling = gen7_ceiling.unwrap_or(0).max(max_rec);
        // Reliable when a GEN7 constant applies, or the file itself carries slack (PS4 / pre-grown).
        let enforced = gen7_ceiling.is_some() || max_rec > current;
        let adding = creates.adding_for(i);
        let over = enforced && current + adding > ceiling;
        out.push(TableCapacity {
            table: (*cid).to_string(),
            label: ROSTER_TABLE_LABELS[i].to_string(),
            current,
            ceiling,
            adding,
            over,
            enforced,
        });
    }
    out
}
