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
    /// `true` when a roster save is mirrored alongside `nhlng.db` (edits fan out to both).
    pub roster_attached: bool,
    /// `true` when the roster mirror is record-index-aligned with `nhlng.db` (so generic field edits
    /// mirror). `roster_attached && !roster_aligned` means degraded — field edits hit `nhlng` only.
    pub roster_aligned: bool,
}

/// One discovered save container, classified by its content.
#[derive(Serialize, TS)]
pub struct SaveSlotDto {
    /// Stable kind tag: `settings` | `roster` | `beagm` | `beapro` | `hockeycard` | `season` | `unknown`.
    pub kind: String,
    /// Human label for the UI (platform-aware: Xbox's settings slot is the Profile save).
    pub label: String,
    pub path: String,
    /// Container variant: `v1` (read-only) | `v2` | `v4`.
    pub container_version: String,
    pub writable: bool,
}

/// The result of pointing onboarding at a root folder: the resolved files plus the save inventory.
#[derive(Serialize, TS)]
pub struct InstallReportDto {
    pub game_id: String,
    pub platform: String,
    pub picked_root: String,
    /// The resolved per-game install dir; `null` if the game install wasn't found under the root.
    pub install_dir: Option<String>,
    pub nhlng_path: Option<String>,
    pub localization_path: Option<String>,
    pub meta_xml_path: Option<String>,
    pub saves: Vec<SaveSlotDto>,
    /// The bare minimum to mod: an `nhlng.db` **and** a roster save are both present.
    pub minimum_met: bool,
    /// `true` when the database isn't loose yet but `.big` archives are present — the install needs
    /// extraction before it can be opened (Xbox / Recomp / PS4 ship the db packed).
    pub extract_needed: bool,
    /// `true` when a `.nhlmodproj` sidecar already exists in the install dir (settings carry over).
    pub has_existing_profile: bool,
}

/// The result of extracting a game's `.big` archives in place.
#[derive(Serialize, TS)]
pub struct ExtractSummaryDto {
    /// `.big` archives that parsed and were processed.
    pub archives: usize,
    /// Entries written to disk.
    pub files_written: usize,
    /// Total bytes written (decompressed).
    #[ts(type = "number")]
    pub bytes_written: u64,
    /// Entries skipped because they're `chunklzx`-wrapped or failed to decode.
    pub skipped_unsupported: usize,
    /// Entries skipped because their name would escape the archive's directory.
    pub skipped_unsafe: usize,
    /// `.big` files that failed to parse.
    pub archives_failed: usize,
}

/// One integrity finding from the Layer 4 audit, resolved for display (human-readable-surfaces rule).
/// `title`/`location`/`message` are what the UI shows; `predicate`/`technical`/`table`/`field`/
/// `record` are the raw internals, surfaced only behind progressive disclosure.
#[derive(Serialize, TS)]
pub struct FindingDto {
    /// `"error"` | `"warning"` | `"info"`.
    pub severity: String,
    /// Friendly category, e.g. "Duplicate jersey number".
    pub title: String,
    /// Resolved entity context (team / player names), or `null` when nothing resolves.
    pub location: Option<String>,
    /// Plain-language description (no raw ids).
    pub message: String,
    /// Stable predicate id (grouping/telemetry) — not primary text.
    pub predicate: String,
    /// The original id-precise message, for the details affordance.
    pub technical: String,
    /// Raw studio ids, kept for the details affordance.
    pub table: String,
    pub field: String,
    pub record: Option<u32>,
}

/// One duplicate-jersey correction the on-load auto-repair applied, with names resolved for display.
#[derive(Serialize, TS)]
pub struct JerseyCorrectionDto {
    /// Resolved team name.
    pub team: String,
    /// Resolved player name (the renumbered player).
    pub player: String,
    /// The duplicate number the player wore.
    #[ts(type = "number")]
    pub old_number: u64,
    /// The number they were reassigned to.
    #[ts(type = "number")]
    pub new_number: u64,
}

/// Summary of the on-load duplicate-jersey auto-correction (empty when nothing needed fixing).
#[derive(Serialize, TS)]
pub struct AutocorrectSummaryDto {
    pub corrections: Vec<JerseyCorrectionDto>,
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
