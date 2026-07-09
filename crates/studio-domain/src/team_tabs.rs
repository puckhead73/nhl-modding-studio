//! Team-detail subtab view-models beyond the core Team Data view: **Draft Picks** (`qHav`),
//! **Strategies** (`EeYo`), **Retired Numbers** (`wcSL`), **Jerseys** (`xjgw`), and **Equipment**
//! (`hHbv`). All layouts verified against the PS3 NHL Legacy anchor meta (2026-07-05); the
//! decodes are ported from the reference app's operator-anchored research and re-checked against
//! our own metadata names where the XML carries them (`qHav.year/round/owner`, `wcSL.team/
//! jerseynumber`, `xjgw.teamid/style/...`).
//!
//! Editing: everything here edits through the ordinary validated `set_field` path (each row's
//! cells are addressed by `(studio table, record, studio field)`), except retired numbers, which
//! are whole-row appends/removes (`Session::retire_jersey` / `unretire_jersey`).

use tdb_core::{Table, TdbFile};
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;

use crate::view::{read_field, read_u64_by_studio, FieldView};

fn table<'a>(db: &'a TdbFile, studio_id: &str) -> Option<&'a Table> {
    db.table_canonical(studio_id)
}

fn field_by_studio<'a>(
    db: &TdbFile,
    t: &'a Table,
    studio_field: &str,
) -> Option<&'a tdb_core::FieldDescriptor> {
    t.fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == studio_field)
}

// ---------------------------------------------------------------------------
// Draft Picks — `qHav` (exhibitiondraftpick)
// ---------------------------------------------------------------------------
//
// One row per future pick (Legacy anchor: 6 years × 7 rounds × 30 teams). Decode verified against
// the anchor meta's own field names: `qFnd`=year (+1900), `KWpX`=round, `oCey`=pickoverall (+1),
// `Uwdb`=owner and `GFcg`=origteamid (both 1-indexed team FKs, 0 = unassigned), `XSWT` =
// drafted-player ref (0 in stock rosters).

pub const DRAFT_PICKS_TABLE: &str = "qHav";

/// One future draft pick owned by a team.
#[derive(Clone, Debug)]
pub struct DraftPick {
    /// The `qHav` record index — the handle `trade_draft_pick` takes.
    pub record: u32,
    pub year: u32,
    pub round: u32,
    pub overall: u32,
    /// Original owner's canonical team id + resolved name (`None` = unassigned).
    pub original_owner_team_id: Option<u64>,
    pub original_owner_name: Option<String>,
    pub via_trade: bool,
}

/// Every `qHav` pick currently owned by `team_id`, sorted year → round → overall. Empty (not an
/// error) when the table is absent — the UI renders a clean "no picks" state.
pub fn team_draft_picks(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    model: &tdb_relations::RosterModel,
    team_id: u64,
) -> Vec<DraftPick> {
    let Some(t) = table(db, DRAFT_PICKS_TABLE) else {
        return Vec::new();
    };
    let read = |rec: u32, id: &str| read_u64_by_studio(db, t, rec, id);
    let owner = |raw: Option<u64>| raw.filter(|&v| v > 0).map(|v| v - 1);

    let mut picks = Vec::new();
    for rec in 0..t.current_records as u32 {
        let current = owner(read(rec, "Uwdb"));
        if current != Some(team_id) {
            continue;
        }
        let original = owner(read(rec, "GFcg"));
        picks.push(DraftPick {
            record: rec,
            year: read(rec, "qFnd").map(|v| v as u32 + 1900).unwrap_or(0),
            round: read(rec, "KWpX").map(|v| v as u32).unwrap_or(0),
            overall: read(rec, "oCey").map(|v| v as u32 + 1).unwrap_or(0),
            original_owner_team_id: original,
            original_owner_name: original
                .and_then(|id| crate::view::team_name_by_id(db, meta, model, id)),
            via_trade: current != original,
        });
    }
    picks.sort_by(|a, b| {
        a.year
            .cmp(&b.year)
            .then(a.round.cmp(&b.round))
            .then(a.overall.cmp(&b.overall))
    });
    picks
}

// ---------------------------------------------------------------------------
// Strategies — `EeYo` (exhibitionteamstrategies), keyed by `vfEq`
// ---------------------------------------------------------------------------

pub const STRATEGIES_TABLE: &str = "EeYo";
/// Career-mode mirror of `EeYo` — identical 62-field schema (verified against the Legacy anchor
/// meta, 2026-07-05). Ships with 0 rows in stock `nhlng.db`; only dynasty saves populate it.
/// `Session::set_field` mirrors every `EeYo` cell edit onto the matching `JUfQ` row.
pub const STRATEGIES_LEAGUE_TABLE: &str = "JUfQ";

/// Which GUI bucket a strategy cell belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrategyBucket {
    Tactic,
    ForwardBias,
    DefenseBias,
    TeamBias,
}

/// Every strategy cell, in render order, with its operator label + bucket. Ported from the
/// reference app's operator-anchored 60-entry table (ids sanity-checked against our anchor meta:
/// `EeYo` and `JUfQ` both carry all 60 + `vfEq`).
pub const STRATEGY_FIELDS: &[(&str, &str, StrategyBucket)] = &[
    // ---- Tactics (3-bit) ----
    ("RBGh", "Forecheck Formation", StrategyBucket::Tactic),
    ("uAeC", "Neutral Zone Formation", StrategyBucket::Tactic),
    ("asrd", "Neutral Zone Pressure", StrategyBucket::Tactic),
    ("xKvv", "Defensive Pressure", StrategyBucket::Tactic),
    ("TTVS", "Offensive Pressure", StrategyBucket::Tactic),
    ("rKer", "Team Defensive Strategy", StrategyBucket::Tactic),
    ("eRIF", "Line 1 Style", StrategyBucket::Tactic),
    ("LIlj", "Line 2 Style", StrategyBucket::Tactic),
    ("ALBZ", "Line 3 Style", StrategyBucket::Tactic),
    ("CPwO", "Line 4 Style", StrategyBucket::Tactic),
    ("bame", "Team Power Play Strategy", StrategyBucket::Tactic),
    ("NzPy", "Team Penalty Kill Strategy", StrategyBucket::Tactic),
    // ---- Forward biases (4 lines × 6, 5-bit; in-game UI exposes 0..8) ----
    ("CaPK", "Line 1 — Bias 1", StrategyBucket::ForwardBias),
    ("TTSs", "Line 1 — Bias 2", StrategyBucket::ForwardBias),
    ("IYiC", "Line 1 — Bias 3", StrategyBucket::ForwardBias),
    ("GcXB", "Line 1 — Bias 4", StrategyBucket::ForwardBias),
    ("Jfbj", "Line 1 — Bias 5", StrategyBucket::ForwardBias),
    ("YmCV", "Line 1 — Bias 6", StrategyBucket::ForwardBias),
    ("TypP", "Line 2 — Bias 1", StrategyBucket::ForwardBias),
    ("OHMx", "Line 2 — Bias 2", StrategyBucket::ForwardBias),
    ("NMIj", "Line 2 — Bias 3", StrategyBucket::ForwardBias),
    ("rARc", "Line 2 — Bias 4", StrategyBucket::ForwardBias),
    ("YDBq", "Line 2 — Bias 5", StrategyBucket::ForwardBias),
    ("VuwU", "Line 2 — Bias 6", StrategyBucket::ForwardBias),
    ("RNez", "Line 3 — Bias 1", StrategyBucket::ForwardBias),
    ("QUDF", "Line 3 — Bias 2", StrategyBucket::ForwardBias),
    ("PPzP", "Line 3 — Bias 3", StrategyBucket::ForwardBias),
    ("BbGY", "Line 3 — Bias 4", StrategyBucket::ForwardBias),
    ("KYqG", "Line 3 — Bias 5", StrategyBucket::ForwardBias),
    ("LRnw", "Line 3 — Bias 6", StrategyBucket::ForwardBias),
    ("cCAZ", "Line 4 — Bias 1", StrategyBucket::ForwardBias),
    ("ZLFn", "Line 4 — Bias 2", StrategyBucket::ForwardBias),
    ("CQJx", "Line 4 — Bias 3", StrategyBucket::ForwardBias),
    ("kgEy", "Line 4 — Bias 4", StrategyBucket::ForwardBias),
    ("fjUw", "Line 4 — Bias 5", StrategyBucket::ForwardBias),
    ("aaxO", "Line 4 — Bias 6", StrategyBucket::ForwardBias),
    // ---- Defense biases (3 pairs × 6, 5-bit) ----
    ("KhBQ", "Pair 1 — Bias 1", StrategyBucket::DefenseBias),
    ("LocO", "Pair 1 — Bias 2", StrategyBucket::DefenseBias),
    ("AjsI", "Pair 1 — Bias 3", StrategyBucket::DefenseBias),
    ("OfpD", "Pair 1 — Bias 4", StrategyBucket::DefenseBias),
    ("RctJ", "Pair 1 — Bias 5", StrategyBucket::DefenseBias),
    ("QVwX", "Pair 1 — Bias 6", StrategyBucket::DefenseBias),
    ("bFXp", "Pair 2 — Bias 1", StrategyBucket::DefenseBias),
    ("GwSz", "Pair 2 — Bias 2", StrategyBucket::DefenseBias),
    ("FrOJ", "Pair 2 — Bias 3", StrategyBucket::DefenseBias),
    ("zDfy", "Pair 2 — Bias 4", StrategyBucket::DefenseBias),
    ("gAPw", "Pair 2 — Bias 5", StrategyBucket::DefenseBias),
    ("NJmq", "Pair 2 — Bias 6", StrategyBucket::DefenseBias),
    ("ZWQF", "Pair 3 — Bias 1", StrategyBucket::DefenseBias),
    ("IPNb", "Pair 3 — Bias 2", StrategyBucket::DefenseBias),
    ("HUJV", "Pair 3 — Bias 3", StrategyBucket::DefenseBias),
    ("JIYa", "Pair 3 — Bias 4", StrategyBucket::DefenseBias),
    ("SLIg", "Pair 3 — Bias 5", StrategyBucket::DefenseBias),
    ("DShy", "Pair 3 — Bias 6", StrategyBucket::DefenseBias),
    // ---- Team biases (6 × 5-bit; only bias 1 = PP Carry/Dump is operator-anchored) ----
    ("JWjc", "Power Play Carry / Dump", StrategyBucket::TeamBias),
    ("YPGK", "Team — Bias 2", StrategyBucket::TeamBias),
    ("DUCw", "Team — Bias 3", StrategyBucket::TeamBias),
    ("NITr", "Team — Bias 4", StrategyBucket::TeamBias),
    ("CLDF", "Team — Bias 5", StrategyBucket::TeamBias),
    ("TSyj", "Team — Bias 6", StrategyBucket::TeamBias),
];

/// One strategy cell: the operator label plus the ordinary editable [`FieldView`] (enum labels
/// come from the strategy vocabularies in `defs/enums/`).
#[derive(Clone, Debug)]
pub struct StrategyCell {
    pub label: String,
    pub field: FieldView,
}

/// A team's strategy sheet: the `EeYo` row bucketed for the GUI.
#[derive(Clone, Debug)]
pub struct TeamStrategies {
    pub team_id: u64,
    pub record: u32,
    pub tactics: Vec<StrategyCell>,
    /// 4 lines × 6 biases, line-major.
    pub forward_biases: Vec<StrategyCell>,
    /// 3 pairs × 6 biases, pair-major.
    pub defense_biases: Vec<StrategyCell>,
    pub team_biases: Vec<StrategyCell>,
}

/// The strategy sheet for one team, or `None` when `EeYo` is absent or carries no row for it.
pub fn team_strategies(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    team_id: u64,
) -> Option<TeamStrategies> {
    let t = table(db, STRATEGIES_TABLE)?;
    let record = (0..t.current_records as u32)
        .find(|&rec| read_u64_by_studio(db, t, rec, "vfEq") == Some(team_id))?;

    let mut out = TeamStrategies {
        team_id,
        record,
        tactics: Vec::new(),
        forward_biases: Vec::new(),
        defense_biases: Vec::new(),
        team_biases: Vec::new(),
    };
    for (id, label, bucket) in STRATEGY_FIELDS {
        let Some(f) = field_by_studio(db, t, id) else {
            continue;
        };
        let cell = StrategyCell {
            label: (*label).to_string(),
            field: read_field(db, meta, enums, t, record, f),
        };
        match bucket {
            StrategyBucket::Tactic => out.tactics.push(cell),
            StrategyBucket::ForwardBias => out.forward_biases.push(cell),
            StrategyBucket::DefenseBias => out.defense_biases.push(cell),
            StrategyBucket::TeamBias => out.team_biases.push(cell),
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Retired numbers — `wcSL` (stockretirednumbertable)
// ---------------------------------------------------------------------------
//
// One row = one retired number for one team. `wcSL.dXSB` is the **0-indexed** team FK (same
// convention as `eGlu.dXSB`, NOT the 1-indexed `ubPc.dXSB`); `JFiY` = the number (1..99). The
// anchor meta names the fields `team` / `jerseynumber`.

pub const RETIRED_TABLE: &str = "wcSL";

/// A team's retired jersey numbers, sorted ascending (deduped). Empty when `wcSL` is absent.
pub fn retired_numbers(db: &TdbFile, team_id: u64) -> Vec<u32> {
    let Some(t) = table(db, RETIRED_TABLE) else {
        return Vec::new();
    };
    let mut out: Vec<u32> = (0..t.current_records as u32)
        .filter(|&rec| read_u64_by_studio(db, t, rec, "dXSB") == Some(team_id))
        .filter_map(|rec| read_u64_by_studio(db, t, rec, "JFiY").map(|v| v as u32))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

// ---------------------------------------------------------------------------
// Jerseys — `xjgw` (stockteamjerseys)
// ---------------------------------------------------------------------------
//
// **`mCXg` is 0-indexed** (verified in the reference app against default.db; our anchor meta
// names it `teamid`) — the opposite of `hHbv.dXSB`. The `{style}_{artid}_{variant}` triple is the
// asset id that keys the jersey's texture files (surfacing those arrives with the Assets phase).

pub const JERSEYS_TABLE: &str = "xjgw";

/// One `xjgw` jersey row, decoded. All cells edit through `set_field` on this record.
#[derive(Clone, Debug)]
pub struct TeamJersey {
    pub record: u32,
    pub jersey_index: u32,
    pub style_raw: u32,
    pub team_art_id: u32,
    pub variant_raw: u32,
    pub fe_variant_raw: u32,
    pub helmet_logo: u32,
    pub helmet_numbers: u32,
    pub is_light_color: u32,
    pub has_tie: u32,
    pub loc_string_raw: u32,
    pub is_default: bool,
    pub is_lockable: bool,
}

/// Every jersey registered to `team_id` (`mCXg == team_id`), in stored order.
pub fn team_jerseys(db: &TdbFile, team_id: u64) -> Vec<TeamJersey> {
    let Some(t) = table(db, JERSEYS_TABLE) else {
        return Vec::new();
    };
    let read = |rec: u32, id: &str| read_u64_by_studio(db, t, rec, id).unwrap_or(0) as u32;
    (0..t.current_records as u32)
        .filter(|&rec| read_u64_by_studio(db, t, rec, "mCXg") == Some(team_id))
        .map(|rec| TeamJersey {
            record: rec,
            jersey_index: read(rec, "tYnx"),
            style_raw: read(rec, "cVLR"),
            team_art_id: read(rec, "GDQL"),
            variant_raw: read(rec, "FccT"),
            fe_variant_raw: read(rec, "ddOX"),
            helmet_logo: read(rec, "JpWa"),
            helmet_numbers: read(rec, "aEfN"),
            is_light_color: read(rec, "Ughr"),
            has_tie: read(rec, "tWcJ"),
            loc_string_raw: read(rec, "UuZg"),
            is_default: read(rec, "yFSM") == 1,
            is_lockable: read(rec, "vInP") == 1,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Equipment — `hHbv` (stockteamequipmenttable)
// ---------------------------------------------------------------------------
//
// `dXSB` here is **1-indexed** (`dXSB - 1 == kOtt.vfEq`). Nine colour zones, each stored as
// three 8-bit channels in **BGR** field order on disk; we surface RGB plus each channel's studio
// field id so the UI can edit the raw cells directly.

pub const EQUIPMENT_TABLE: &str = "hHbv";

/// Per-zone `(blue, green, red)` studio field ids, zone 1..9 — the literal on-disk BGR order.
pub const EQUIPMENT_ZONE_FIELDS: [(&str, &str, &str); 9] = [
    ("IByh", "QnCG", "VaOV"),
    ("GHIQ", "OZgb", "Tssg"),
    ("HZib", "PLSw", "SCSn"),
    ("KuQC", "SwoV", "PVgG"),
    ("LIml", "TeyG", "OjCJ"),
    ("NOUA", "VQkn", "QBEw"),
    ("MgSz", "UCSg", "RLCb"),
    ("DYdG", "LmNh", "WbNC"),
    ("CiDJ", "KYzW", "Xtnl"),
];

/// One colour zone, decoded to RGB (fields absent from a title's schema read as 0).
#[derive(Clone, Copy, Debug, Default)]
pub struct EquipmentZone {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// One stock-equipment row for a team.
#[derive(Clone, Debug)]
pub struct TeamEquipment {
    pub record: u32,
    pub equipment_id: u32,
    pub equipment_type_raw: u32,
    pub brand_raw: u32,
    pub era_raw: u32,
    pub variant_raw: u32,
    pub zones: Vec<EquipmentZone>,
}

/// Every `hHbv` row for `team_id` (`dXSB - 1 == team_id`), in stored order.
pub fn team_equipment(db: &TdbFile, team_id: u64) -> Vec<TeamEquipment> {
    let Some(t) = table(db, EQUIPMENT_TABLE) else {
        return Vec::new();
    };
    let read = |rec: u32, id: &str| read_u64_by_studio(db, t, rec, id).unwrap_or(0) as u32;
    (0..t.current_records as u32)
        .filter(|&rec| read_u64_by_studio(db, t, rec, "dXSB") == Some(team_id + 1))
        .map(|rec| TeamEquipment {
            record: rec,
            equipment_id: read(rec, "vfEq"),
            equipment_type_raw: read(rec, "jJpM"),
            brand_raw: read(rec, "TKWL"),
            era_raw: read(rec, "VhbR"),
            variant_raw: read(rec, "FccT"),
            zones: EQUIPMENT_ZONE_FIELDS
                .iter()
                .map(|(b, g, r)| EquipmentZone {
                    red: read(rec, r) as u8,
                    green: read(rec, g) as u8,
                    blue: read(rec, b) as u8,
                })
                .collect(),
        })
        .collect()
}
