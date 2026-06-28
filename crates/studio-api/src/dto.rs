//! Data-transfer objects — the typed FE/BE contract. Each derives `serde::Serialize` and
//! `ts_rs::TS`, so the TypeScript definitions are generated from Rust (no hand-maintained `dto.ts`).
//! The domain↔DTO mapping lives in [`crate::convert`].

use serde::Serialize;
use ts_rs::TS;

/// One table's at-a-glance shape.
#[derive(Serialize, TS)]
pub struct TableSummaryDto {
    /// Canonical (community) id — the big-endian reversal already applied.
    pub id: String,
    pub field_count: usize,
    pub records: u32,
}

/// A loaded project plus a structural summary of its current working database.
#[derive(Serialize, TS)]
pub struct ProjectSummaryDto {
    pub project_id: u32,
    pub platform: String,
    pub role: String,
    /// How the file was loaded / will be saved: `"raw_db"` or `"container"`.
    pub source: String,
    pub endian: String,
    pub table_count: usize,
    pub db_size: u32,
    pub header_crc_ok: bool,
    pub size_ok: bool,
    /// `true` once the working bytes differ from the loaded baseline.
    pub dirty: bool,
    /// Number of committed operations that can be reverted.
    pub undo_depth: usize,
    /// Number of reverted operations that can be re-applied.
    pub redo_depth: usize,
    pub tables: Vec<TableSummaryDto>,
}

/// One integrity finding from the Layer 4 audit.
#[derive(Serialize, TS)]
pub struct FindingDto {
    /// `"error"` | `"warning"` | `"info"`.
    pub severity: String,
    pub predicate: String,
    pub message: String,
    pub table: String,
    pub field: String,
    pub record: Option<u32>,
}

/// The result of a whole-DB audit.
#[derive(Serialize, TS)]
pub struct IntegrityReportDto {
    /// `true` when there are no `error`-severity findings (safe to write).
    pub clean: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub findings: Vec<FindingDto>,
}

/// What an edit operation did: cell-edit count plus a description of each auto-repair.
#[derive(Serialize, TS)]
pub struct OpOutcomeDto {
    pub edits: u32,
    pub repairs: Vec<String>,
}

/// The outcome of creating a player: the new `wBIz` id plus the op outcome.
#[derive(Serialize, TS)]
pub struct CreatePlayerDto {
    /// `u64` but always within JS safe-integer range; emitted as `number` to match Tauri's JSON IPC.
    #[ts(type = "number")]
    pub player_id: u64,
    pub outcome: OpOutcomeDto,
}

/// One field of one record, ready to display or edit.
#[derive(Serialize, TS)]
pub struct FieldViewDto {
    pub studio_table: String,
    pub studio_field: String,
    pub name: String,
    /// `"integer"` | `"string"` | `"real"` | `"other"`.
    pub kind: String,
    /// What to render: enum label, decoded number, or string text.
    pub display: String,
    /// Sign/sentinel-decoded value for integers; `null` for strings.
    #[ts(type = "number | null")]
    pub decoded: Option<i64>,
    pub enum_label: Option<String>,
    /// Raw stored bits for integer fields (`null` for strings).
    #[ts(type = "number | null")]
    pub raw_int: Option<u64>,
    /// Text for string fields (`null` for integers).
    pub text: Option<String>,
    #[ts(type = "number | null")]
    pub range_low: Option<i64>,
    #[ts(type = "number | null")]
    pub range_high: Option<i64>,
    pub editable: bool,
    pub is_string: bool,
}

/// A compact player row for the master list.
#[derive(Serialize, TS)]
pub struct PlayerSummaryDto {
    #[ts(type = "number")]
    pub player_id: u64,
    pub first_name: String,
    pub last_name: String,
    pub position_label: String,
    #[ts(type = "number | null")]
    pub team_id: Option<u64>,
}

/// A player's roster membership on one team.
#[derive(Serialize, TS)]
pub struct RosterMembershipDto {
    #[ts(type = "number")]
    pub team_id: u64,
    #[ts(type = "number | null")]
    pub jersey: Option<u64>,
    #[ts(type = "number | null")]
    pub captaincy: Option<u64>,
    #[ts(type = "number | null")]
    pub roster_status: Option<u64>,
}

/// The full Player detail: curated identity, ratings, memberships, and the complete bio field list.
#[derive(Serialize, TS)]
pub struct PlayerViewDto {
    #[ts(type = "number")]
    pub player_id: u64,
    pub bio_record: u32,
    /// `"skater"` | `"goalie"`.
    pub kind: String,
    pub identity: Vec<FieldViewDto>,
    pub ratings: Vec<FieldViewDto>,
    /// The companion record index the ratings belong to (needed to edit them); `null` if absent.
    pub ratings_record: Option<u32>,
    /// The companion table id the ratings belong to (`dSvy` or `mHuy`).
    pub ratings_table: String,
    pub memberships: Vec<RosterMembershipDto>,
    pub all_fields: Vec<FieldViewDto>,
}

/// A read-only career-stats table for one player (stock season data).
#[derive(Serialize, TS)]
pub struct CareerStatsDto {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// One roster entry on a team's depth chart.
#[derive(Serialize, TS)]
pub struct LineEntryDto {
    pub slot_record: u32,
    #[ts(type = "number | null")]
    pub player_id: Option<u64>,
    pub name: String,
    pub position_label: String,
    #[ts(type = "number | null")]
    pub jersey: Option<u64>,
    #[ts(type = "number | null")]
    pub captaincy: Option<u64>,
    #[ts(type = "number | null")]
    pub roster_status: Option<u64>,
    /// Labels of the set 1-bit situational flags (meta-derived; in-game semantics unverified).
    pub slots: Vec<String>,
}

/// A team's depth chart.
#[derive(Serialize, TS)]
pub struct LinesViewDto {
    #[ts(type = "number")]
    pub team_id: u64,
    pub entries: Vec<LineEntryDto>,
}

/// A compact team row for the master list.
#[derive(Serialize, TS)]
pub struct TeamSummaryDto {
    #[ts(type = "number")]
    pub team_id: u64,
    pub record: u32,
    pub name: String,
    pub abbrev: String,
}

/// A resolved arena reference (a team's home rink).
#[derive(Serialize, TS)]
pub struct ArenaRefDto {
    #[ts(type = "number")]
    pub arena_id: u64,
    pub name: String,
    pub city: String,
}

/// The full Team detail: curated identity, resolved arena, roster size, and all `kOtt` fields.
#[derive(Serialize, TS)]
pub struct TeamViewDto {
    #[ts(type = "number")]
    pub team_id: u64,
    pub record: u32,
    pub identity: Vec<FieldViewDto>,
    pub arena: Option<ArenaRefDto>,
    pub roster_count: usize,
    pub all_fields: Vec<FieldViewDto>,
}

/// A compact arena row for the master list.
#[derive(Serialize, TS)]
pub struct ArenaSummaryDto {
    #[ts(type = "number")]
    pub arena_id: u64,
    pub record: u32,
    pub name: String,
    pub city: String,
}

/// The full Arena detail: curated identity plus all `StEO` fields.
#[derive(Serialize, TS)]
pub struct ArenaViewDto {
    #[ts(type = "number")]
    pub arena_id: u64,
    pub record: u32,
    pub identity: Vec<FieldViewDto>,
    pub all_fields: Vec<FieldViewDto>,
}
