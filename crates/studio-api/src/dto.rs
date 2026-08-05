//! Data-transfer objects — the typed FE/BE contract. Each derives `serde::Serialize` and
//! `ts_rs::TS`, so the TypeScript definitions are generated from Rust (no hand-maintained `dto.ts`).
//! The domain↔DTO mapping lives in [`crate::convert`].

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One table's at-a-glance shape.
#[derive(Serialize, TS)]
pub struct TableSummaryDto {
    /// Canonical (community) id — the big-endian reversal already applied.
    pub id: String,
    pub field_count: usize,
    pub records: u32,
}

/// Which optional schema features the loaded title's database actually carries — probed from the
/// file's own field descriptors at load (never assumed from the title id), so unproven titles gate
/// honestly by evidence. Drives the UI's hide/disable decisions for surfaces a game doesn't have,
/// instead of rendering blanks a user can't tell apart from bugs.
#[derive(Serialize, TS)]
pub struct CapabilitiesDto {
    /// The team→arena link exists (absent in NHL 10–12) — gates the Team detail's Arena card.
    pub arena_link: bool,
    /// The player/goalie style (archetype) enum exists (absent in NHL 10–13).
    pub player_style: bool,
    /// Year of UFA eligibility (absent in NHL 10).
    pub year_ufa: bool,
    /// Junior (rights) team ref (absent in NHL 10).
    pub junior_team: bool,
    /// Draft team ref (absent in NHL 10).
    pub draft_team: bool,
    /// Draft year (absent in NHL 10).
    pub draft_year: bool,
    /// Draft round (absent in NHL 10–11).
    pub draft_round: bool,
    /// Draft position/pick (absent in NHL 10).
    pub draft_pick: bool,
    /// AHL rookie flag (absent in NHL 10–13).
    pub rookie_ahl: bool,
    /// CHL rookie flag (absent in NHL 10–13).
    pub rookie_chl: bool,
    /// The separate skater equipment table exists (absent on gen6 titles like NHL07, which ship no
    /// equipment tables) — gates any per-player equipment surface instead of rendering it blank.
    pub has_skater_equip: bool,
    /// The separate goalie equipment table exists (see `has_skater_equip`).
    pub has_goalie_equip: bool,
    /// Skater ratings in this database that carry an operator label; `< skater_ratings_total`
    /// means the loaded schema has ratings we can't yet name (the Attributes tab hedges visibly).
    pub skater_ratings_labelled: usize,
    /// All skater rating fields this database carries (classified from its own metadata).
    pub skater_ratings_total: usize,
    /// Labelled goalie ratings (see `skater_ratings_labelled`).
    pub goalie_ratings_labelled: usize,
    /// All goalie rating fields this database carries.
    pub goalie_ratings_total: usize,
    /// How `ubPc.dXSB` (the "main team") encodes the team id: `0` on NHL 09/10 (raw = team id) or
    /// `1` on Legacy/11→21 (raw = team id + 1, 0 = free agent). The Main-Team picker uses it so it
    /// reads/writes the right value per title instead of assuming the modern 1-indexed convention.
    pub main_team_offset: u8,
}

/// One database surface's availability verdict (Phase 1 presentation-capability contract). `state`
/// is one of `not-applicable` | `readable` | `editable` | `disabled` | `unverified`; `reason` carries
/// the human explanation for the non-plain states. Drives the UI's omit/disable/caution decisions per
/// tab, so an absent feature is never rendered as an empty tab a user can't tell from a bug.
#[derive(Serialize, TS)]
pub struct SurfaceCapabilityDto {
    pub state: String,
    pub reason: Option<String>,
}

/// The per-surface availability contract for the loaded database — one verdict per user-facing
/// surface (Phase 1B). Probed from the file's own schema (table + key + join + fields), never the
/// title id.
#[derive(Serialize, TS)]
pub struct SurfaceCapabilitiesDto {
    pub players: SurfaceCapabilityDto,
    pub teams: SurfaceCapabilityDto,
    pub arenas: SurfaceCapabilityDto,
    pub career: SurfaceCapabilityDto,
    pub attributes: SurfaceCapabilityDto,
    pub traits: SurfaceCapabilityDto,
    pub lines: SurfaceCapabilityDto,
    pub strategies: SurfaceCapabilityDto,
    pub captaincy: SurfaceCapabilityDto,
    pub contracts: SurfaceCapabilityDto,
    pub draft_picks: SurfaceCapabilityDto,
    pub jerseys: SurfaceCapabilityDto,
    pub team_equipment: SurfaceCapabilityDto,
    pub player_equipment: SurfaceCapabilityDto,
    pub portraits: SurfaceCapabilityDto,
    pub logos: SurfaceCapabilityDto,
    pub heads: SurfaceCapabilityDto,
    pub audio: SurfaceCapabilityDto,
}

/// A loaded project plus a structural summary of its current working database.
#[derive(Serialize, TS)]
pub struct ProjectSummaryDto {
    pub project_id: u32,
    /// Studio game id (`nhl10`…`nhllegacy`) when opened through onboarding; `null` on the legacy
    /// file-pick path (the user never named a title there).
    pub game_id: Option<String>,
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
    /// `true` when read views (and this summary's overview) are served from the roster save instead
    /// of `nhlng.db` (the display toggle). On an aligned pair edits still fan out to both files;
    /// with `display_roster && !roster_aligned` the workspace is in roster-only mode — edits apply
    /// strictly to the roster file, and areas the roster save doesn't carry aren't editable.
    pub display_roster: bool,
    /// The optional schema features this title's database carries (probed at load).
    pub capabilities: CapabilitiesDto,
    /// The per-surface availability contract — which tabs the UI should show, disable, or omit.
    pub surfaces: SurfaceCapabilitiesDto,
    /// True when saving this project uses an **experimentally-supported** write path: NHL10/11 v1
    /// containers (the `tag_1c` integrity word is un-reversed, so an edited save carries a stale tag)
    /// or PS4 roster saves (repack is code-complete but on-console load is unverified). The bytes are
    /// structurally valid and round-trip, but in-game acceptance isn't confirmed — surface a caveat
    /// before the user relies on the save.
    pub write_experimental: bool,
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

/// Summary of the roster-block structural repair (an order-only permutation of the roster slot
/// table back into contiguous per-team blocks — no membership changes).
#[derive(Serialize, TS)]
pub struct RepairBlocksSummaryDto {
    /// How many roster slots changed physical position (0 = the layout was already healthy).
    pub moved_slots: u32,
    /// Resolved names of the teams whose blocks the audit had flagged as corrupt.
    pub teams: Vec<String>,
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

/// One value of an enum-bound field's vocabulary (for edit-time dropdowns).
#[derive(Serialize, TS)]
pub struct EnumOptionDto {
    #[ts(type = "number")]
    pub raw: i64,
    pub label: String,
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
    /// Full `(raw, label)` vocabulary when enum-bound; empty otherwise. Powers edit-time dropdowns.
    pub enum_options: Vec<EnumOptionDto>,
    /// Friendly-edit offset: front-facing value is `decoded + edit_offset` (type 2011, stage raw 111).
    #[ts(type = "number | null")]
    pub edit_offset: Option<i64>,
    /// When true the friendly value is presented/parsed as `ft'in"` (height).
    pub edit_height: bool,
    /// Fixed-point scale: front-facing value is `decoded * edit_scale` (SV% ×0.001, GAA ×0.01).
    #[ts(type = "number | null")]
    pub edit_scale: Option<f64>,
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

/// One curated cell of a server-driven field profile — a labelled control the UI renders directly.
#[derive(Serialize, TS)]
pub struct FieldCellDto {
    pub label: String,
    /// `"field"` | `"team"` | `"arena"` | `"derived"` — how the UI edits the cell.
    pub control: String,
    /// The resolved field to read/edit; `null` for derived cells.
    pub field: Option<FieldViewDto>,
    /// The record index the cell's edit must target (usually the entity record; for a NHL07 team
    /// name, the joined stock/custom row that actually holds the name).
    pub record: u32,
    /// Team-FK encoding offset (`raw - offset` = team id) for `"team"` cells.
    pub team_offset: Option<u8>,
    /// The empty-option label for `"team"` cells (e.g. "(undrafted)").
    pub none_label: Option<String>,
    /// Resolved read-only text for `"derived"` cells.
    pub derived_display: Option<String>,
    /// When set, this cell's value semantics are not yet confirmed for the loaded title — the UI
    /// shows the raw value plus this note.
    pub unverified: Option<String>,
}

/// One titled section of a field profile.
#[derive(Serialize, TS)]
pub struct FieldSectionDto {
    pub title: String,
    pub cells: Vec<FieldCellDto>,
}

/// The full server-driven curated layout for one entity (the sections the UI renders in order).
#[derive(Serialize, TS)]
pub struct EntityFieldProfileDto {
    pub sections: Vec<FieldSectionDto>,
}

/// A compact player row for the master list and the team depth chart.
#[derive(Serialize, TS)]
pub struct PlayerSummaryDto {
    #[ts(type = "number")]
    pub player_id: u64,
    pub first_name: String,
    pub last_name: String,
    pub position_label: String,
    #[ts(type = "number | null")]
    pub team_id: Option<u64>,
    #[ts(type = "number | null")]
    pub jersey_number: Option<i64>,
    /// Player/Goalie Style (archetype) label, when the vocabulary knows the value.
    pub style_label: Option<String>,
    /// The real bio active flag (`kKYN`).
    pub is_active: Option<bool>,
    /// Derived EA Overall on the display scale (36..99).
    #[ts(type = "number | null")]
    pub overall: Option<i64>,
    /// NHL-rights team (bio `proteam`), canonical team id — powers "in the system".
    #[ts(type = "number | null")]
    pub rights_team_id: Option<u64>,
    /// The bio record index (lets list-driven surfaces edit `ubPc` cells directly).
    pub bio_record: u32,
    #[ts(type = "number | null")]
    pub cap_hit_dollars: Option<u64>,
    pub contract_years: Option<u32>,
    pub contract_status: Option<String>,
    pub is_two_way: Option<bool>,
    /// `true` for goaltenders — the Uniforms tab swaps the skater glove/helmet
    /// equipment cards for the goalie set (blocker / trapper / pads) when the
    /// previewed player is a goalie.
    pub is_goalie: bool,
}

/// A player's roster membership on one team.
#[derive(Serialize, TS)]
pub struct RosterMembershipDto {
    #[ts(type = "number")]
    pub team_id: u64,
    /// The team's resolved display name, when it resolves.
    pub team_name: Option<String>,
    #[ts(type = "number | null")]
    pub jersey: Option<u64>,
    #[ts(type = "number | null")]
    pub captaincy: Option<u64>,
    /// Lineup status from `eGlu.hSZj`: `true` dressed, `false` scratched, `null` if absent (Spike S4).
    pub dressed: Option<bool>,
}

/// Resolved, display-ready banner values for the Player detail header. Every field is already
/// localized (enum labels, formatted height/weight/date, resolved team) — render verbatim.
#[derive(Serialize, TS)]
pub struct PlayerHeaderDto {
    pub full_name: String,
    pub position_label: Option<String>,
    pub handedness_label: Option<String>,
    pub nationality_label: Option<String>,
    /// "City, State/Province/Country".
    pub birthplace: Option<String>,
    /// Composed `YYYY-MM-DD`.
    pub birthdate: Option<String>,
    /// Formatted height, e.g. `6'1"`.
    pub height: Option<String>,
    /// Formatted weight, e.g. `190 lb`.
    pub weight: Option<String>,
    /// Player/goalie style (archetype) label.
    pub style_label: Option<String>,
    #[ts(type = "number | null")]
    pub primary_team_id: Option<u64>,
    pub primary_team_name: Option<String>,
    #[ts(type = "number | null")]
    pub jersey_number: Option<i64>,
}

/// Resolved contract/draft/rookie/junior fields for the Player Data tab (display-ready).
#[derive(Serialize, TS)]
pub struct PlayerDataDto {
    #[ts(type = "number | null")]
    pub year_ufa: Option<u32>,
    #[ts(type = "number | null")]
    pub pro_team_id: Option<u64>,
    pub pro_team_name: Option<String>,
    /// Formatted, e.g. `$5,000,000`.
    pub cap_hit: Option<String>,
    #[ts(type = "number | null")]
    pub contract_length_years: Option<u32>,
    pub contract_status: Option<String>,
    pub is_two_way: Option<bool>,
    pub is_multi_year: Option<bool>,
    #[ts(type = "number | null")]
    pub junior_team_id: Option<u64>,
    pub junior_team_name: Option<String>,
    #[ts(type = "number | null")]
    pub draft_team_id: Option<u64>,
    pub draft_team_name: Option<String>,
    #[ts(type = "number | null")]
    pub draft_round: Option<u32>,
    #[ts(type = "number | null")]
    pub draft_position: Option<u32>,
    #[ts(type = "number | null")]
    pub draft_year: Option<u32>,
    pub is_rookie_nhl: Option<bool>,
    pub is_rookie_ahl: Option<bool>,
    pub is_rookie_chl: Option<bool>,
}

/// One labelled attribute cell for the Attributes tab. `field` carries the editable field view
/// (dropdown vocab / friendly-edit offset already resolved); `null` marks an unmapped placeholder — a
/// known attribute whose source field hasn't been anchored (e.g. Deflections).
#[derive(Serialize, TS)]
pub struct AttributeViewDto {
    pub label: String,
    /// True for a 6-bit `+36` rating (drives tier coloring); false for biases/enums/derived.
    pub is_rating: bool,
    pub field: Option<FieldViewDto>,
}

/// One active trait decoded from a `Trait N` slot, with a confidence tag so the UI can hedge weak
/// labels.
#[derive(Serialize, TS)]
pub struct TraitViewDto {
    pub slot_field: String,
    pub slot_index: u32,
    #[ts(type = "number")]
    pub value: i64,
    pub label: String,
    /// `"confirmed"` | `"strong"` | `"weak"` | `"unknown"`.
    pub confidence: String,
    pub note: Option<String>,
}

/// One toggleable entry in the trait checkbox menu.
#[derive(Serialize, TS)]
pub struct TraitOptionDto {
    #[ts(type = "number")]
    pub value: i64,
    pub label: String,
    /// `"confirmed"` | `"strong"` | `"weak"`.
    pub confidence: String,
    pub note: Option<String>,
    /// True when the player currently has this trait (a slot holds it).
    pub active: bool,
}

/// The labelled, ordered ratings + traits for the Attributes tab (all resolution done in the domain).
#[derive(Serialize, TS)]
pub struct PlayerAttributesDto {
    /// `"skater"` | `"goalie"` | `null` (no ratings companion).
    pub kind: Option<String>,
    pub overall: Option<AttributeViewDto>,
    pub style: Option<AttributeViewDto>,
    /// Potential Range (`QoMA` = `growthtier`) — in-game star potential (provisional decode).
    pub potential_range: Option<AttributeViewDto>,
    /// Potential Accuracy (`mBef` = `growthletter`) — scouting certainty High/Medium/Low.
    pub potential_accuracy: Option<AttributeViewDto>,
    pub flat: Vec<AttributeViewDto>,
    pub traits: Vec<TraitViewDto>,
    /// The full trait vocabulary as toggleable options (checkbox menu).
    pub trait_options: Vec<TraitOptionDto>,
}

/// The full Player detail: resolved header, curated identity, ratings, memberships, and the complete
/// bio field list.
#[derive(Serialize, TS)]
pub struct PlayerViewDto {
    #[ts(type = "number")]
    pub player_id: u64,
    pub bio_record: u32,
    /// `"skater"` | `"goalie"`.
    pub kind: String,
    /// Resolved, display-ready header banner values.
    pub header: PlayerHeaderDto,
    /// Resolved contract/draft/rookie/junior fields for the Player Data tab.
    pub data: PlayerDataDto,
    /// Labelled, ordered ratings + traits for the Attributes tab.
    pub attributes: PlayerAttributesDto,
    pub identity: Vec<FieldViewDto>,
    pub ratings: Vec<FieldViewDto>,
    /// The companion record index the ratings belong to (needed to edit them); `null` if absent.
    pub ratings_record: Option<u32>,
    /// The companion table id the ratings belong to (`dSvy` or `mHuy`).
    pub ratings_table: String,
    /// The per-player equipment companion fields (`xmja` skater / `fMVl` goalie) — the Equipment tab.
    pub equipment: Vec<FieldViewDto>,
    /// The equipment companion record index (needed to edit the fields); `null` if absent.
    pub equipment_record: Option<u32>,
    /// The equipment companion table id the fields belong to (`xmja` or `fMVl`).
    pub equipment_table: String,
    pub memberships: Vec<RosterMembershipDto>,
    pub all_fields: Vec<FieldViewDto>,
    /// The server-driven curated section layout the UI renders (per-game field profile).
    pub profile: EntityFieldProfileDto,
}

/// One column of a career-stats section: the studio field id + the header label (`GP`, `SV%`, …).
#[derive(Serialize, TS)]
pub struct CareerColumnDto {
    pub field: String,
    pub label: String,
}

/// One career season row: resolved identity plus stat cells aligned with the section's columns
/// (`null` = the field is absent from this title's schema).
#[derive(Serialize, TS)]
pub struct CareerRowDto {
    /// Record index in the section's table — the `set_field` / delete address.
    pub record: u32,
    /// `"2006-07"` (the stored year is the season-end year).
    pub season_label: String,
    /// Resolved league label (`NHL`/`AHL`).
    pub league_label: String,
    /// Stored team-code string, as-is.
    pub team: String,
    pub cells: Vec<Option<FieldViewDto>>,
}

/// One career-stats section (one of the four stock tables).
#[derive(Serialize, TS)]
pub struct CareerSectionDto {
    /// `skater_season` | `skater_playoffs` | `goalie_season` | `goalie_playoffs`.
    pub kind: String,
    pub title: String,
    /// Studio table id — the edit/add/delete address.
    pub table: String,
    pub columns: Vec<CareerColumnDto>,
    pub rows: Vec<CareerRowDto>,
}

/// A player's stock career stats: the four skater/goalie × season/playoffs sections.
#[derive(Serialize, TS)]
pub struct CareerStatsDto {
    pub sections: Vec<CareerSectionDto>,
}

/// The result of adding a stock career season row.
#[derive(Serialize, TS)]
pub struct AddCareerSeasonDto {
    /// The new row's record index in its table.
    pub record: u32,
    pub outcome: OpOutcomeDto,
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
    /// Lineup status from `eGlu.hSZj`: `true` dressed, `false` scratched, `null` if absent (Spike S4).
    /// ("In minors" is *not* here — it is a depth-chart identifier derived from the player's main team.)
    pub dressed: Option<bool>,
    /// Studio field ids of the set 1-bit situational flags (see `LinesViewDto.slot_fields`).
    pub slots: Vec<String>,
    /// Per-slot archetype cell (`eGlu.QgFs`) — Player Style enum (S4-confirmed); edit through `set_field`.
    pub style: Option<FieldViewDto>,
}

/// One situational slot field on `eGlu`: studio id + metadata name (`l1c`, `pp1lw`, `s1`, …).
#[derive(Serialize, TS)]
pub struct SlotFieldDto {
    pub studio_field: String,
    pub name: String,
}

/// A team's depth chart.
#[derive(Serialize, TS)]
pub struct LinesViewDto {
    #[ts(type = "number")]
    pub team_id: u64,
    /// Every situational slot field on this title's `eGlu`, in schema order.
    pub slot_fields: Vec<SlotFieldDto>,
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
    /// League code (`kOtt.xMjj`) + resolved label — powers league-scoped team pickers.
    #[ts(type = "number | null")]
    pub league: Option<i64>,
    pub league_label: Option<String>,
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
    /// Retired jersey numbers (`wcSL`), sorted ascending.
    pub retired_numbers: Vec<u32>,
    pub all_fields: Vec<FieldViewDto>,
    /// The server-driven curated section layout the UI renders (per-game field profile).
    pub profile: EntityFieldProfileDto,
}

/// One future draft pick owned by a team (`qHav`).
#[derive(Serialize, TS)]
pub struct DraftPickDto {
    /// The `qHav` record index — the handle `trade_draft_pick` takes.
    pub record: u32,
    pub year: u32,
    pub round: u32,
    pub overall: u32,
    #[ts(type = "number | null")]
    pub original_owner_team_id: Option<u64>,
    pub original_owner_name: Option<String>,
    pub via_trade: bool,
}

/// One strategy cell: operator label + the ordinary editable field view.
#[derive(Serialize, TS)]
pub struct StrategyCellDto {
    pub label: String,
    pub field: FieldViewDto,
}

/// A team's strategy sheet (`EeYo`), bucketed for the GUI. Cell edits go through `set_field`
/// on `(EeYo, record, field)` — the backend mirrors each edit onto career-mode `JUfQ`.
#[derive(Serialize, TS)]
pub struct TeamStrategiesDto {
    #[ts(type = "number")]
    pub team_id: u64,
    pub record: u32,
    pub tactics: Vec<StrategyCellDto>,
    /// 4 lines × 6 biases, line-major.
    pub forward_biases: Vec<StrategyCellDto>,
    /// 3 pairs × 6 biases, pair-major.
    pub defense_biases: Vec<StrategyCellDto>,
    pub team_biases: Vec<StrategyCellDto>,
}

/// One `xjgw` jersey row for a team. Cells edit through `set_field` on this record.
#[derive(Serialize, TS)]
pub struct TeamJerseyDto {
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

/// One colour zone of a stock-equipment row, decoded to RGB.
#[derive(Serialize, TS)]
pub struct EquipmentZoneDto {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// One `hHbv` stock-equipment row for a team.
#[derive(Serialize, TS)]
pub struct TeamEquipmentDto {
    pub record: u32,
    pub equipment_id: u32,
    pub equipment_type_raw: u32,
    pub brand_raw: u32,
    pub era_raw: u32,
    pub variant_raw: u32,
    pub zones: Vec<EquipmentZoneDto>,
}

/// One equipment slot in the Hockey Shop catalog.
#[derive(Serialize, TS)]
pub struct EquipmentSlotDto {
    /// Stable key the frontend sends back (`"gloves"`, `"blocker"`, …).
    pub key: String,
    /// Operator-facing name ("Gloves").
    pub label: String,
    /// `"skater"` | `"goalie"` — the category strip the slot lives under.
    pub category: String,
    /// The player-companion DB field that selects this slot's id (`xmja.NmUR`, …),
    /// surfaced behind progressive disclosure for power users.
    pub field_ref: String,
    /// Inclusive ID ceiling from the field's bit width.
    pub max_id: u32,
}

/// One render file belonging to an equipment id (path relative to `<rendering>/`).
#[derive(Serialize, TS)]
pub struct EquipmentSlotFileDto {
    pub relative_path: String,
}

/// One equipment id and the render files it owns, from scanning the slot folder.
#[derive(Serialize, TS)]
pub struct EquipmentSlotAssetDto {
    pub id: u32,
    pub files: Vec<EquipmentSlotFileDto>,
}

/// A compact arena row for the master list.
#[derive(Serialize, TS)]
pub struct ArenaSummaryDto {
    #[ts(type = "number")]
    pub arena_id: u64,
    pub record: u32,
    pub name: String,
    pub city: String,
    /// How many teams call this arena home (`kOtt.buDa == arena_id`).
    pub team_count: u32,
}

/// The full Arena detail: curated identity, the resolved tenants, plus all `StEO` fields.
#[derive(Serialize, TS)]
pub struct ArenaViewDto {
    #[ts(type = "number")]
    pub arena_id: u64,
    pub record: u32,
    pub identity: Vec<FieldViewDto>,
    /// Teams whose home arena this is, resolved to names.
    pub teams: Vec<TeamSummaryDto>,
    pub all_fields: Vec<FieldViewDto>,
}

// ---- Historical (FHM corpus) ----------------------------------------------
//
// The read-only historical source database. Person-centric: `person_id` is a synthesized identity
// unifying one human across the per-season FHM exports (see the `tdb-fhm` slug rule). These mirror
// the `studio_domain::historical` model rows; ids are already resolved to names by the store.

/// The season / league filter options for the Historical browse dropdowns.
#[derive(Serialize, TS)]
pub struct HistoricalFiltersDto {
    pub seasons: Vec<HistoricalSeasonOptionDto>,
    pub leagues: Vec<HistoricalLeagueOptionDto>,
}

#[derive(Serialize, TS)]
pub struct HistoricalSeasonOptionDto {
    #[ts(type = "number")]
    pub year: i64,
    /// Display label, e.g. `"1985-1986"`.
    pub label: String,
}

#[derive(Serialize, TS)]
pub struct HistoricalLeagueOptionDto {
    pub abbr: String,
    pub name: String,
}

/// One human in the Historical Players master list (unified across seasons).
#[derive(Serialize, TS)]
pub struct HistoricalPersonSummaryDto {
    #[ts(type = "number")]
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    /// `YYYY-MM-DD` (or `?` when unknown).
    pub dob: String,
    pub nationality: Option<String>,
    pub is_goalie: bool,
    /// Primary position label (C/LW/RW/D/G), or null when the person has no ratings row.
    pub position: Option<String>,
    #[ts(type = "number | null")]
    pub first_year: Option<i64>,
    #[ts(type = "number | null")]
    pub last_year: Option<i64>,
    #[ts(type = "number")]
    pub season_count: i64,
    pub retired: bool,
    /// True when the player is still active in the corpus's current season — the list renders the
    /// span end as "Present".
    pub active: bool,
    /// Distinct seasons the player logged games — the list's headline count (replaces the raw span).
    #[ts(type = "number")]
    pub seasons_played: i64,
}

/// The full Historical person detail: canonical bio + season timeline + career + draft + awards.
#[derive(Serialize, TS)]
pub struct HistoricalPersonDto {
    #[ts(type = "number")]
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    pub nick_name: Option<String>,
    pub dob: String,
    #[ts(type = "number | null")]
    pub height: Option<i64>,
    #[ts(type = "number | null")]
    pub weight: Option<i64>,
    pub hand: Option<String>,
    pub nationality: Option<String>,
    pub birthcity: Option<String>,
    pub birthstate: Option<String>,
    pub retired: bool,
    pub is_goalie: bool,
    pub position: Option<String>,
    #[ts(type = "number | null")]
    pub first_year: Option<i64>,
    /// Final season the player actually logged games (not the raw last appearance). `null` only when
    /// no played season exists.
    #[ts(type = "number | null")]
    pub last_year: Option<i64>,
    /// True when the player is still active in the corpus's current season — the UI renders the
    /// career end as "Present" rather than that year.
    pub active: bool,
    pub seasons: Vec<HistoricalSeasonRowDto>,
    pub career: Vec<HistoricalCareerRowDto>,
    /// Every league draft the person was selected in, NHL/major drafts first (empty if never drafted).
    pub drafts: Vec<HistoricalDraftDto>,
    pub contract: Option<HistoricalContractDto>,
    pub awards: Vec<HistoricalAwardDto>,
}

#[derive(Serialize, TS)]
pub struct HistoricalContractDto {
    pub team_abbr: Option<String>,
    pub ntc: bool,
    pub nmc: bool,
    pub elc: bool,
    pub ufa: bool,
    pub scholarship: Option<String>,
    /// FHM "Average Salary" (thousands).
    #[ts(type = "number | null")]
    pub avg_salary: Option<i64>,
    /// Years remaining on the deal.
    #[ts(type = "number | null")]
    pub length: Option<i64>,
}

/// A person's roster snapshot in one season (team resolved within that season's namespace).
#[derive(Serialize, TS)]
pub struct HistoricalSeasonRowDto {
    #[ts(type = "number")]
    pub season_year: i64,
    pub season_label: String,
    pub team_name: Option<String>,
    pub team_abbr: Option<String>,
    pub league_abbr: Option<String>,
    #[ts(type = "number | null")]
    pub height: Option<i64>,
    #[ts(type = "number | null")]
    pub weight: Option<i64>,
    /// The number worn that season (lowest, if several) — pre-fills the Roster Builder jersey.
    #[ts(type = "number | null")]
    pub jersey: Option<i64>,
    pub ability: Option<f64>,
    pub potential: Option<f64>,
}

/// One career-stats line, with the stat cells pre-ordered for display.
#[derive(Serialize, TS)]
pub struct HistoricalCareerRowDto {
    /// `"skater"` | `"goalie"`.
    pub kind: String,
    /// `"rs"` (regular season) | `"po"` (playoffs).
    pub phase: String,
    #[ts(type = "number")]
    pub year: i64,
    pub team_abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub cells: Vec<HistoricalStatCellDto>,
}

#[derive(Serialize, TS)]
pub struct HistoricalStatCellDto {
    pub label: String,
    pub value: String,
}

#[derive(Serialize, TS)]
pub struct HistoricalDraftDto {
    #[ts(type = "number | null")]
    pub year: Option<i64>,
    #[ts(type = "number | null")]
    pub round: Option<i64>,
    #[ts(type = "number | null")]
    pub pick: Option<i64>,
    #[ts(type = "number | null")]
    pub overall: Option<i64>,
    /// Era-resolved drafting team (historical name where known, else the snapshot abbr).
    pub team: Option<String>,
    /// Draft label, e.g. "NHL Entry Draft" / "QMJHL Entry Draft".
    pub draft_name: Option<String>,
    /// NHL/major draft — the ones that lead the display.
    pub is_nhl: bool,
}

#[derive(Serialize, TS)]
pub struct HistoricalAwardDto {
    #[ts(type = "number | null")]
    pub year: Option<i64>,
    pub league_abbr: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// A team as it existed in one season — the Historical Teams master list (season-scoped).
#[derive(Serialize, TS)]
pub struct HistoricalTeamSummaryDto {
    #[ts(type = "number")]
    pub season_year: i64,
    #[ts(type = "number")]
    pub src_team_id: i64,
    pub league_abbr: Option<String>,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub abbr: Option<String>,
}

/// Full team-season detail: identity + that season's roster (each entry links back to a person).
#[derive(Serialize, TS)]
pub struct HistoricalTeamDto {
    #[ts(type = "number")]
    pub season_year: i64,
    #[ts(type = "number")]
    pub src_team_id: i64,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub conference_name: Option<String>,
    pub division_name: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub roster: Vec<HistoricalRosterEntryDto>,
}

#[derive(Serialize, TS)]
pub struct HistoricalRosterEntryDto {
    #[ts(type = "number")]
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    pub is_goalie: bool,
    pub ability: Option<f64>,
    pub potential: Option<f64>,
}

// ---- Franchise-consolidated team browse (v5) ------------------------------
// The team browse consolidates NHL relocations/renames into one franchise entry (per the v5
// `franchise`/`franchise_member` lineage); non-NHL clubs stay standalone. `kind`+`key` identify an
// entry: a franchise (`key` = franchise id) or a standalone team (`key` = cross-season `team_key`).

/// The league filter options for the team browse (distinct leagues across the cross-season `team`
/// identity table; `NHL` lists the franchises).
#[derive(Serialize, TS)]
pub struct HistoricalTeamFiltersDto {
    pub leagues: Vec<HistoricalLeagueOptionDto>,
}

/// One row in the franchise-consolidated team index.
#[derive(Serialize, TS)]
pub struct HistoricalTeamIndexEntryDto {
    /// `"franchise"` | `"team"`.
    pub kind: String,
    pub key: String,
    pub name: String,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
    #[ts(type = "number | null")]
    pub first_year: Option<i64>,
    #[ts(type = "number | null")]
    pub last_year: Option<i64>,
    pub is_defunct: bool,
}

/// The franchise/team detail for one selected season: a franchise-level header, the seasons it iced a
/// team (the dropdown), and the constituent club block(s) for the selected season.
#[derive(Serialize, TS)]
pub struct HistoricalTeamFranchiseDto {
    pub kind: String,
    pub key: String,
    pub display_name: String,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub is_defunct: bool,
    pub notes: Option<String>,
    #[ts(type = "number[]")]
    pub seasons: Vec<i64>,
    #[ts(type = "number")]
    pub season_year: i64,
    pub constituents: Vec<HistoricalTeamBlockDto>,
}

/// One club as it existed in one season — identity + colours + affiliates + roster + cap sheet.
#[derive(Serialize, TS)]
pub struct HistoricalTeamBlockDto {
    pub team_key: Option<String>,
    #[ts(type = "number")]
    pub src_team_id: i64,
    #[ts(type = "number")]
    pub season_year: i64,
    /// Era-appropriate name (a franchise member's era name, or the standalone team's own name).
    pub name: String,
    pub nickname: Option<String>,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub conference_name: Option<String>,
    pub division_name: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub text_color: Option<String>,
    pub affiliates: Vec<HistoricalTeamAffiliateDto>,
    pub roster: Vec<HistoricalTeamMemberDto>,
    pub contracts: Vec<HistoricalTeamContractDto>,
}

#[derive(Serialize, TS)]
pub struct HistoricalTeamAffiliateDto {
    /// `"affiliate"` (a farm team under this club) | `"parent"` (the club this reports up to).
    pub relation: String,
    #[ts(type = "number")]
    pub src_team_id: i64,
    pub name: Option<String>,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
}

/// A roster member enriched with position, jersey, captaincy, DOB and nationality.
#[derive(Serialize, TS)]
pub struct HistoricalTeamMemberDto {
    #[ts(type = "number")]
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    pub is_goalie: bool,
    pub position: Option<String>,
    pub dob: String,
    pub nationality: Option<String>,
    #[ts(type = "number | null")]
    pub jersey: Option<i64>,
    /// `"C"` (captain) | `"A"` (alternate) | null.
    pub captaincy: Option<String>,
    pub ability: Option<f64>,
    pub potential: Option<f64>,
}

/// One line of a team-season cap sheet (`season_contract`; `avg_salary` in dollars).
#[derive(Serialize, TS)]
pub struct HistoricalTeamContractDto {
    #[ts(type = "number")]
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    #[ts(type = "number | null")]
    pub avg_salary: Option<i64>,
    pub ntc: bool,
    pub nmc: bool,
    pub elc: bool,
    pub ufa: bool,
}

// ---- Formula engine (Stage 2) ---------------------------------------------

/// One computed EA rating, on its display scale.
#[derive(Serialize, TS)]
pub struct EaRatingDto {
    pub ea_key: String,
    /// Display value; null when no component contributed.
    pub value: Option<i32>,
    pub scale_min: i32,
    pub scale_max: i32,
}

/// A person-season's computed EA ratings under a formula profile.
#[derive(Serialize, TS)]
pub struct EaComputedRatingsDto {
    /// `forward` / `defenseman` / `goalie`.
    pub position: String,
    pub overall: Option<i32>,
    /// Archetype/style EA label (e.g. "Sniper") — null for goalies.
    pub style_label: Option<String>,
    /// Archetype/style EA raw code — null for goalies.
    #[ts(type = "number | null")]
    pub style_raw: Option<u32>,
    pub ratings: Vec<EaRatingDto>,
    /// The raw FHM attribute values this was computed from (includes position grades G/LD/RD/LW/C/RW).
    pub fhm_values: Vec<FhmValueDto>,
}

/// One raw FHM attribute value.
#[derive(Serialize, TS)]
pub struct FhmValueDto {
    pub key: String,
    pub value: f64,
}

/// One playing season's EA Overall, for the Attributes season dropdown (newest first).
#[derive(Serialize, TS)]
pub struct CareerSeasonOverallDto {
    #[ts(type = "number")]
    pub season_year: i64,
    pub overall: Option<i32>,
}

/// Cross-career EA rating compilations for a player: per-season overalls (with the best flagged for
/// the ★), plus the Full Career and Peak-3 averaged views (same shape as a single season's ratings).
#[derive(Serialize, TS)]
pub struct HistoricalCareerRatingsDto {
    /// `forward` / `defenseman` / `goalie`.
    pub position: String,
    pub seasons: Vec<CareerSeasonOverallDto>,
    /// Highest-Overall season (ties → earliest) — starred in the dropdown. Null for goalies/no data.
    #[ts(type = "number | null")]
    pub best_season_year: Option<i64>,
    /// Average over ALL playing seasons with rating data.
    pub career_average: Option<EaComputedRatingsDto>,
    /// Average over the top-3 Overall seasons.
    pub peak3: Option<EaComputedRatingsDto>,
}

/// The editable formula profile (shape of `docs/reference/default_ea_formulas.json`). A component is
/// `[fhm_key, weight]`; `fhm_key` may be a raw FHM attribute or a `prebuilt:<key>` reference.
#[derive(Serialize, Deserialize, TS)]
pub struct FormulaProfileDto {
    #[ts(type = "Record<string, [string, number][]>")]
    pub forward: std::collections::BTreeMap<String, Vec<(String, f64)>>,
    #[ts(type = "Record<string, [string, number][]>")]
    pub defenseman: std::collections::BTreeMap<String, Vec<(String, f64)>>,
    #[ts(type = "Record<string, [string, number][]>")]
    pub goalie: std::collections::BTreeMap<String, Vec<(String, f64)>>,
    /// `"nominal"` | `"position_max"`.
    pub normalisation_mode: String,
}

/// Static catalog for the Formulas UI: the EA attribute lists, the FHM vocabulary, and prebuilts.
#[derive(Serialize, TS)]
pub struct FormulaCatalogDto {
    pub skater_ea_attrs: Vec<String>,
    pub goalie_ea_attrs: Vec<String>,
    pub fhm_groups: Vec<FhmGroupDto>,
    pub prebuilts: Vec<PrebuiltDto>,
}

#[derive(Serialize, TS)]
pub struct FhmGroupDto {
    pub group: String,
    pub attrs: Vec<FhmAttrDto>,
}

#[derive(Serialize, TS)]
pub struct FhmAttrDto {
    /// The `attrs_json` key.
    pub key: String,
    pub label: String,
}

#[derive(Serialize, TS)]
pub struct PrebuiltDto {
    /// Stored form, e.g. `"prebuilt:gp_durability"`.
    pub key: String,
    pub label: String,
    #[ts(type = "number")]
    pub native_max: u32,
    /// When true the formula outputs on its own scale (bias/tier rows).
    pub direct_output: bool,
}

// ---------------------------------------------------------------- assets

/// Which install's art to work against. Every asset command needs the same trio, so they take
/// it as one value rather than three positional strings.
#[derive(Deserialize, TS)]
pub struct AssetRootsArgsDto {
    /// The resolved install dir. Art is anchored on the game executable found under it
    /// (RenderWare titles) or on the `nhl/data` tree (PS4).
    pub install_dir: String,
    /// The install's platform (`ps3`/`xbox`/`ps4`/`recomp`). Drives the on-disk art
    /// layout — RenderWare `rendering/`+`fe/` vs the PS4 kettle tree. Absent decodes
    /// as Xbox (the pre-PS4 behavior).
    #[ts(optional)]
    pub platform: Option<String>,
    /// Override the inferred `rendering/` root; `null`/empty keeps the inferred path.
    pub rendering_override: Option<String>,
    /// Override the inferred `fe/` root; `null`/empty keeps the inferred path.
    pub fe_override: Option<String>,
}

/// Where a title keeps its art, and whether both roots are actually present.
#[derive(Serialize, TS)]
pub struct AssetRootsDto {
    pub rendering: String,
    pub fe: String,
    /// Both roots exist on disk. `false` means the install is still packed —
    /// run extraction before scanning.
    pub complete: bool,
}

/// One player's art, resolved against a real install. `full_name` and `slug` are the
/// human-readable surfaces; `portrait_id` / `head_id` are the raw EA ids, for progressive
/// disclosure only.
#[derive(Serialize, TS)]
pub struct PlayerAssetsDto {
    pub bio_record: u32,
    pub full_name: String,
    /// `lastname_firstname_dd_mm_yyyy` — the export filename stem. `null` when the bio
    /// lacks a usable name or birthdate, in which case this player can't be exported.
    pub slug: Option<String>,
    #[ts(type = "number")]
    pub portrait_id: u64,
    #[ts(type = "number")]
    pub head_id: u64,
    /// This head model is this player's own (`pPaD == lOnr`) — the only case where naming
    /// a mesh after them is truthful.
    pub bespoke_head: bool,
    /// A shared generic head (id >= 60000) reused across many players.
    pub pool_head: bool,
    pub portrait_path: Option<String>,
    pub mesh_path: Option<String>,
    /// Every RX2 container holding one of this head's textures: a single bundled texlib on
    /// Legacy/14/15, or several loose per-raster files on NHL 10. Empty when none ship.
    pub texture_paths: Vec<String>,
}

/// What to include in an export run.
#[derive(Deserialize, TS)]
pub struct ExportOptionsDto {
    pub portraits: bool,
    pub meshes_glb: bool,
    pub raw_rx2: bool,
    pub textures: bool,
    /// Crop portraits to their opaque bounds (EA ships them on a mostly-empty 512x512 canvas).
    pub crop_portraits: bool,
}

/// What an export run produced. Partial results are normal — `failures` carries the
/// already-human-readable reasons.
#[derive(Serialize, TS)]
pub struct AssetExportSummaryDto {
    pub players_considered: usize,
    pub portraits_written: usize,
    pub meshes_written: usize,
    pub textures_written: usize,
    pub raw_copied: usize,
    /// Portraits skipped because they're a generic placeholder shared across many
    /// un-portraited players, not the player's own.
    pub portraits_skipped_generic: usize,
    pub skipped_no_slug: usize,
    pub failures: Vec<String>,
    #[ts(type = "number")]
    pub bytes_written: u64,
}

// ---------------------------------------------------------------- file browser

/// One entry in a browsed directory of the extracted install.
#[derive(Serialize, TS)]
pub struct BrowseEntryDto {
    pub name: String,
    /// Forward-slash path relative to the browse root — the token passed back to browse into a
    /// folder or to select it/the file for export.
    pub rel_path: String,
    pub is_dir: bool,
    #[ts(type = "number")]
    pub size: u64,
    /// `mesh` / `portrait` / `texture` when this file decodes to an asset; `null` for directories
    /// and files the exporter doesn't recognize.
    pub kind: Option<String>,
}

/// One directory level of the extracted install, for the file browser.
#[derive(Serialize, TS)]
pub struct BrowseListingDto {
    /// The absolute browse root (the install dir).
    pub root: String,
    /// Forward-slash path of the listed directory relative to the root (`""` = the root itself).
    pub rel_path: String,
    /// The parent's rel path, or `null` at the root. Drives the "up" affordance.
    pub parent: Option<String>,
    pub entries: Vec<BrowseEntryDto>,
}

/// What to include in a file-browser export run.
#[derive(Deserialize, TS)]
pub struct FileExportOptionsDto {
    pub meshes_glb: bool,
    pub portraits_png: bool,
    pub textures: bool,
    /// Copy files the exporter doesn't recognize through unchanged rather than skipping them (also
    /// the fallback when a recognized file fails to decode).
    pub copy_unrecognized: bool,
    pub crop_portraits: bool,
}

/// What a file-browser export run produced. Partial results are normal — `failures` carries the
/// already-human-readable reasons.
#[derive(Serialize, TS)]
pub struct FileExportSummaryDto {
    pub files_considered: usize,
    pub meshes_written: usize,
    pub portraits_written: usize,
    pub textures_written: usize,
    pub raw_copied: usize,
    pub skipped_unrecognized: usize,
    pub skipped_unsafe: usize,
    pub failures: Vec<String>,
    #[ts(type = "number")]
    pub bytes_written: u64,
}

// ---------------------------------------------------------------- team outfits

/// What a team-outfit export produced (jersey / pants / socks per team, as PNGs named by team).
/// Partial results are normal — `failures` carries the already-human-readable reasons.
#[derive(Serialize, TS)]
pub struct OutfitExportSummaryDto {
    pub teams_considered: usize,
    pub jerseys_written: usize,
    pub pants_written: usize,
    pub socks_written: usize,
    pub failures: Vec<String>,
    #[ts(type = "number")]
    pub bytes_written: u64,
    /// The art folder scanned for outfit textures — shown when nothing was found so the operator can
    /// see where the exporter looked.
    pub rendering_root: String,
}

// ---------------------------------------------------------------- texture import

/// Result of writing an edited texture back into an extracted asset file.
#[derive(Serialize, TS)]
pub struct ImportTextureSummaryDto {
    /// The extracted file that was overwritten.
    pub target_path: String,
    /// Which container it was: `"gnf"` (PS4), `"portrait"` (BIGF), or `"texlib"` (RW4 `.rx2`).
    pub container: String,
    pub width: u32,
    pub height: u32,
    #[ts(type = "number")]
    pub bytes_written: u64,
    /// The `.bak` written before the first overwrite of this file, when a backup was requested and
    /// none existed yet.
    pub backup_path: Option<String>,
}

/// One of a team's outfit textures (jersey / pants / socks) resolved from the install's art tree,
/// for the per-team Uniforms surface. `rel_path` is the install-relative file-browser path the
/// texture preview + replace commands take.
#[derive(Serialize, TS)]
pub struct OutfitTextureDto {
    pub rel_path: String,
    /// The raster within `rel_path` this entry refers to (a jersey texlib bundle exposes its body and
    /// its number `font` as separate entries) — passed to the preview/replace commands.
    pub texture_index: u32,
    /// `"jersey"`, `"font"`, `"pant"`, `"sock"`, or `"name"` (a nameplate sheet).
    pub piece: String,
    /// Human-readable descriptor (piece + set/variant/map), team-scoped so the raw id is dropped.
    pub label: String,
    /// The raw on-disk file name, kept for a details tooltip (progressive disclosure).
    pub file_name: String,
    /// A `name_*` nameplate template (marker zones for name/number/patch positions) rather than a
    /// paintable texture — the UI offers the location editor instead of a plain image replace.
    pub is_nameplate: bool,
}

/// A decoded texture preview for the Replace flow: the current on-disk texture (the "before"),
/// a picked source image (the "after"), or a small list-row thumbnail. `data_url` is a
/// ready-to-render `data:image/png;base64,…`.
#[derive(Serialize, TS)]
pub struct TexturePreviewDto {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    /// The container the texture decoded from — `"gnf"`, `"portrait"`, or `"texlib"` — or `null`
    /// for an operator-supplied source image (which isn't a game container).
    pub container: Option<String>,
}

// ---------------------------------------------------------------- head preview

/// One decoded texlib raster for the head viewer (RGBA8, base64).
#[derive(Serialize, TS)]
pub struct PreviewTextureDto {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba_base64: String,
}

/// One submesh of a head, as flat arrays for a three.js `BufferGeometry`, plus
/// its material binding. `name` is the label the visibility toggle shows.
#[derive(Serialize, TS)]
pub struct PreviewSubmeshDto {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uv1: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    /// Index into `HeadPreviewDto::textures`; `null` renders untextured.
    pub material_index: Option<u32>,
    pub transparent: bool,
    pub render_order: i32,
}

/// A player head decoded for the 3D viewer: every submesh with its texture, plus
/// camera framing. `lenient_rig` is `true` for NHL 10 heads whose per-submesh
/// bone palettes disagree — the geometry rendered but the rig is unresolved.
#[derive(Serialize, TS)]
pub struct HeadPreviewDto {
    pub submeshes: Vec<PreviewSubmeshDto>,
    pub textures: Vec<PreviewTextureDto>,
    pub center: [f32; 3],
    pub span: f32,
    pub source_path: String,
    pub lenient_rig: bool,
}

// ---------------------------------------------------------------- kit preview

/// One assembled uniform piece for the full-kit viewer: its submeshes (material
/// indices point into `KitPreviewDto::textures`) plus provenance. `kind` drives
/// the piece's toggle grouping; `label` is what the toggle shows.
#[derive(Serialize, TS)]
pub struct KitPieceDto {
    pub kind: String,
    pub label: String,
    pub source_path: String,
    pub submeshes: Vec<PreviewSubmeshDto>,
}

/// A full player kit decoded for the 3D viewer: every piece (head, helmet,
/// jersey, gloves, pants, socks, skates) assembled in the shared bind-pose model
/// space, a pooled texture set the submeshes reference, and camera framing over
/// the whole figure.
#[derive(Serialize, TS)]
pub struct KitPreviewDto {
    pub pieces: Vec<KitPieceDto>,
    pub textures: Vec<PreviewTextureDto>,
    pub center: [f32; 3],
    pub span: f32,
}

/// One piece the caller asks the viewer to assemble: its kind, the mesh file
/// (install-relative), and the texlib/raster files (install-relative) that
/// texture it.
#[derive(Deserialize, TS)]
pub struct KitPieceInputDto {
    pub kind: String,
    pub mesh_rel_path: String,
    pub texture_rel_paths: Vec<String>,
}

// ---------------------------------------------------------------- roster builder (Phase 5.4)

/// One Roster Builder cart line (FE → BE): a corpus person-season aimed at an EA team, with the
/// create-vs-match decision (`match_ea_player_id`: `null` ⇒ create a new player; a `wBIz` ⇒ update
/// that existing one) and an optional jersey override.
#[derive(Deserialize, TS)]
pub struct ImportItemDto {
    #[ts(type = "number")]
    pub person_id: i64,
    #[ts(type = "number")]
    pub season_year: i64,
    #[ts(type = "number")]
    pub target_team_id: u64,
    /// `null` ⇒ CreateNew; a `wBIz` ⇒ Match(id) (update the existing player, no allocation).
    #[ts(type = "number | null")]
    pub match_ea_player_id: Option<u64>,
    #[ts(type = "number | null")]
    pub jersey: Option<i64>,
}

/// An EA identity-match candidate for a corpus person — the cart's Match/Create toggle. Human-readable
/// surfaces (`full_name`/`team_name`); the raw `ea_player_id` (`wBIz`) is for progressive disclosure.
#[derive(Serialize, TS)]
pub struct MatchCandidateDto {
    #[ts(type = "number")]
    pub ea_player_id: u64,
    pub full_name: String,
    /// Composed `YYYY-MM-DD`, or `null` when the EA bio carries no usable DOB.
    pub dob: Option<String>,
    pub team_name: Option<String>,
    /// `"name_and_dob"` (high confidence) | `"name_only"` (review candidate).
    pub confidence: String,
    /// `true` when more than one candidate was returned — the cart must disambiguate.
    pub ambiguous: bool,
}

/// One cart item's outcome in a preview/commit, resolved for display.
#[derive(Serialize, TS)]
pub struct ImportItemResultDto {
    #[ts(type = "number")]
    pub person_id: i64,
    #[ts(type = "number")]
    pub season_year: i64,
    #[ts(type = "number")]
    pub target_team_id: u64,
    pub full_name: String,
    /// `"created"` | `"matched"` | `"person_missing"` | `"no_such_player"` | `"no_bio_table"`.
    pub outcome: String,
    /// The matched player's `wBIz` (for `"matched"`/`"no_such_player"`); `null` otherwise. A create's
    /// new id is not known until commit, so it stays `null` in preview.
    #[ts(type = "number | null")]
    pub ea_player_id: Option<u64>,
    /// Fields this item could not write (out of range / absent on this title) — resolved field names.
    pub skipped_fields: Vec<String>,
    /// `true` when this `(person, season)` was already imported to this target (the dedup warning —
    /// advisory only, never blocks).
    pub already_imported: bool,
    /// `true` when this item lowered to an op (i.e. it will appear in the commit).
    pub lowered: bool,
}

/// One roster table's projected fill vs its vanilla ceiling after the cart's creates (the S4 storage
/// gate). `label` is the human surface; `table` is the raw canonical id for progressive disclosure.
#[derive(Serialize, TS)]
pub struct TableCapacityDto {
    pub table: String,
    pub label: String,
    pub current: u32,
    pub ceiling: u32,
    pub adding: u32,
    /// `current + adding > ceiling` — this table would overflow (a storage blocker).
    pub over: bool,
    /// Whether the ceiling is reliable enough to block on (false ⇒ shown, never blocks).
    pub enforced: bool,
}

/// The dry-run result of a whole cart (preview): per-item rows, the player-count delta, the audit over
/// the previewed state, the capacity projection, and the blocker count. Commit re-runs the identical
/// lowered ops, so what preview shows is what commit writes.
#[derive(Serialize, TS)]
pub struct ChangeSetPreviewDto {
    pub items: Vec<ImportItemResultDto>,
    #[ts(type = "number")]
    pub players_before: usize,
    #[ts(type = "number")]
    pub players_after: usize,
    /// Cart items that could not lower — a blocker; the FE disables Commit while `> 0`.
    #[ts(type = "number")]
    pub blocked: usize,
    /// Per-table roster capacity after the cart's creates (the S4 ceiling gate).
    pub capacity: Vec<TableCapacityDto>,
    /// `true` when any enforced table would overflow — Commit is disabled.
    pub capacity_blocked: bool,
    /// The four db-only audit predicates over the previewed state, resolved to human text.
    pub audit: IntegrityReportDto,
    /// Every field dropped across the cart (out of range / absent on this title).
    pub skipped_fields: Vec<String>,
}

/// The result of committing a cart: the same per-item trail (creates now carry their new `wBIz`), the
/// op outcome, and how many rows were written to the import ledger.
#[derive(Serialize, TS)]
pub struct ImportResultDto {
    pub items: Vec<ImportItemResultDto>,
    pub outcome: OpOutcomeDto,
    /// How many `(person, season, target)` rows were recorded in the ledger (0 when it is disabled).
    #[ts(type = "number")]
    pub ledgered: usize,
}
