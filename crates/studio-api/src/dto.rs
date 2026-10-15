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
    /// Receipt 67: why the install's own localization dictionary could not be attached at open (names show as keys).
    #[ts(optional)]
    pub dictionary_problem: Option<String>,
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
    /// The workspace content revision (identity of every session's working bytes) — the same
    /// value a copied-target review is bound to, so a client can tell whether a prepared review
    /// still matches the session without inferring it from undo depths.
    pub workspace_revision: String,
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
    /// PS4: the shadPS4 user profile folder the save belongs to.
    pub profile: Option<String>,
    /// PS4: the title id directory the save belongs to (e.g. `CUSA18182`).
    pub title_id: Option<String>,
    /// Why the save can't be opened (unreadable, or a format the codec doesn't recognise).
    pub problem: Option<String>,
}

/// One PS4 game folder onboarding considered, identified from its own `param.sfo`.
#[derive(Serialize, TS)]
pub struct GameCandidateDto {
    pub path: String,
    pub title: Option<String>,
    pub title_id: Option<String>,
    /// Its prefix names the region (`UP` / `EP` …).
    pub content_id: Option<String>,
    pub app_version: Option<String>,
    /// Whether this folder is the requested title.
    pub matches: bool,
    /// Why it isn't the requested title, or why it can't be identified.
    pub reason: Option<String>,
}

/// Why onboarding couldn't resolve something, as a stable kind and a readable explanation.
#[derive(Serialize, TS)]
pub struct DiscoveryProblemDto {
    /// `game_location_unreadable` | `game_not_found` | `game_wrong_title` | `game_unidentified` |
    /// `game_ambiguous` | `game_location_incomplete` | `database_missing` | `metadata_missing` |
    /// `save_location_unreadable` | `save_inventory_incomplete` | `saves_not_found` |
    /// `saves_other_title`. An `…_incomplete` kind replaces "not found" when part of the location
    /// couldn't be listed.
    pub kind: String,
    pub message: String,
}

/// The suggested PS4 save-data folder for the current Windows user (`%APPDATA%\shadPS4\savedata`).
#[derive(Serialize, TS)]
pub struct SaveRootSuggestionDto {
    /// `null` when the environment has no roaming application-data folder.
    pub path: Option<String>,
    /// Whether that folder exists now.
    pub exists: bool,
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
    /// PS4: the matched game folder (database, archives, extraction and art resolve from it).
    pub game_dir: Option<String>,
    /// PS4: the save-data location searched.
    pub save_root: Option<String>,
    /// PS4: every game folder considered, matching or not.
    pub game_candidates: Vec<GameCandidateDto>,
    /// Why something wasn't resolved (missing game, wrong title, unreadable location, no saves …).
    pub problems: Vec<DiscoveryProblemDto>,
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
    /// Inline strings only: the byte width the value must fit (UTF-8 bytes); `null` for integers.
    pub text_max_bytes: Option<u32>,
    /// Title-scoped vocabularies: where the labels come from ("labels established for NHL Legacy —
    /// …") or why the field has none on this title ("…: no catalogue for NHL 10 — …"); `null` for
    /// unscoped vocabularies and for fields without one.
    pub vocabulary_note: Option<String>,
}

/// One curated cell of a server-driven field profile — a labelled control the UI renders directly.
#[derive(Serialize, TS)]
pub struct FieldCellDto {
    pub label: String,
    /// `"field"` | `"team"` | `"arena"` | `"derived"` | `"color"` — how the UI edits the cell.
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
    /// `"color"` cells: the fields that make up the colour — one packed 24-bit word
    /// (`(r << 16) | (g << 8) | b`) or the `_r`, `_g`, `_b` byte fields in that order; edited as
    /// one operation through `set_cells_int`.
    pub color_fields: Vec<FieldViewDto>,
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
    /// Overall recomputed from this player's ratings with the EA regression; `null` when the loaded
    /// schema resolves too few of the formula's inputs to trust the result.
    #[ts(type = "number | null")]
    pub computed_overall: Option<i64>,
    /// The Overall stored in the companion row (`eWUF`) on the display scale — shown as the delta
    /// against `computed_overall`, never as the headline number.
    #[ts(type = "number | null")]
    pub stored_overall: Option<i64>,
    pub style: Option<AttributeViewDto>,
    /// Potential Range (`QoMA` = `growthtier`) — in-game star potential (provisional decode).
    pub potential_range: Option<AttributeViewDto>,
    /// Potential Accuracy (`mBef` = `growthletter`) — scouting certainty High/Medium/Low.
    pub potential_accuracy: Option<AttributeViewDto>,
    pub flat: Vec<AttributeViewDto>,
    pub traits: Vec<TraitViewDto>,
    /// The full trait vocabulary as toggleable options (checkbox menu).
    pub trait_options: Vec<TraitOptionDto>,
    /// Companion-table fields no cell above already shows — the tab's raw "other fields" drawer.
    pub other_fields: Vec<FieldViewDto>,
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
    /// False when this schema has no inline team-code field.
    #[ts(optional)]
    pub team_code_supported: Option<bool>,
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

/// The copied-target root a project's files sit under, if any — resolved by walking up from the
/// primary file to the nearest directory carrying the engine's marker.
#[derive(Serialize, TS)]
pub struct CopiedTargetRootDto {
    pub project_id: u32,
    /// The marked root (canonical), or `None` when no ancestor is a copied-target root.
    pub root: Option<String>,
    /// Plain-language reason when `root` is `None`.
    pub note: Option<String>,
}

/// One changed cell in the pending review, labelled with the entity it belongs to.
#[derive(Serialize, TS)]
pub struct PendingCellDto {
    pub studio_table: String,
    pub table_name: String,
    pub record: u32,
    /// `Boston Bruins`, `Sidney Crosby`, `Allsvenskan` … or `<table> #<record>`.
    pub entity: String,
    pub studio_field: String,
    pub field_name: String,
    /// Resolved display text (labels, friendly units) before and after.
    pub before: String,
    pub after: String,
    /// The stored values as text (integer bits or the string), for progressive disclosure.
    pub before_raw: String,
    pub after_raw: String,
}

/// A table whose row count or shape differs between the baseline and the working state.
#[derive(Serialize, TS)]
pub struct PendingTableDto {
    pub studio_table: String,
    pub table_name: String,
    pub before_records: Option<u32>,
    pub after_records: Option<u32>,
    pub note: String,
    pub rows_only_in_working: Vec<u32>,
    pub rows_only_in_baseline: Vec<u32>,
}

/// Derived integrity bytes (headers / CRCs) that moved as a consequence of the edits.
#[derive(Serialize, TS)]
pub struct PendingIntegrityDto {
    pub file_header: bool,
    pub table_headers: Vec<String>,
    pub other_bytes: usize,
}

/// The pending review for one file role (the primary database or the attached roster save).
#[derive(Serialize, TS)]
pub struct PendingFileDto {
    /// `primary` or `roster`.
    pub role: String,
    /// Human file label (`Main database (nhlng.db)`, `Roster save (…)`).
    pub label: String,
    pub path: String,
    pub dirty: bool,
    pub cells: Vec<PendingCellDto>,
    pub tables: Vec<PendingTableDto>,
    pub integrity: PendingIntegrityDto,
    /// Exact changed-cell count (the list may be truncated to the requested limit).
    pub total_cells: usize,
    pub truncated: bool,
    /// Set when the two images could not be compared; the UI must show it, never "no changes".
    pub error: Option<String>,
}

/// The whole pending review: primary database plus the attached roster save when present, plus
/// the pending localization-dictionary rename (LOC-W1) when one is held.
#[derive(Serialize, TS)]
pub struct PendingReviewDto {
    pub dirty: bool,
    pub primary: PendingFileDto,
    pub roster: Option<PendingFileDto>,
    pub dictionary: Option<PendingDictionaryDto>,
    /// D4: pending NHL 16–21 catalogue member edits (oldest first), if any.
    #[ts(optional)]
    pub settings_csv: Option<Vec<PendingSettingsCsvDto>>,
    /// D4: the archive those edits will be written into, relative to the install/copied root.
    #[ts(optional)]
    pub settings_csv_archive: Option<String>,
    /// D8: pending texture-member replacements inside packed archives (oldest first), if any.
    #[ts(optional)]
    pub asset_members: Option<Vec<AssetMemberPlanDto>>,
}

/// D8: one held / pending texture-member replacement inside a packed archive.
#[derive(Serialize, TS)]
pub struct AssetMemberPlanDto {
    /// The review ticket (empty on a pending entry).
    pub ticket: String,
    /// The archive relative to the install / copied root (`databigs/inxp_10.big`).
    pub archive_relative: String,
    pub member_path: String,
    /// `gnf` / `png` / … and the (unchanged) dimensions.
    pub container: String,
    pub width: u32,
    pub height: u32,
    /// `chunkzip` / `raw` — the member keeps its stored form.
    pub stored_form: String,
    pub source_stored_len: u64,
    pub new_stored_len: u64,
    /// Other members sharing the stored bytes; non-empty means a private copy is appended and
    /// those keep the shared bytes — unless `scope` is `whole-group`.
    pub shared_with: Vec<String>,
    /// `this-member` (private copy for a shared block) or `whole-group` (every path of the
    /// deduplicated block follows the new image) — always the caller's explicit choice.
    pub scope: String,
    pub placement_prediction: String,
    pub description: String,
    /// Receipt 67 (C37): the root-relative loose package written together with the member (a PS3 board pair).
    pub paired_loose: Option<String>,
    /// Receipt 67 (C37): the root-relative loose path that must stay absent (a PS3 archive-only package).
    pub loose_must_be_absent: Option<String>,
    /// Further disclosures (the materials a board raster change affects, source identities, runtime caveats).
    pub notes: Vec<String>,
}

/// Receipt 67 (C37): the preview of a PS3 board-ad raster replacement (same size, same DXT1 / DXT3 format, every declared level),
/// or the refusal. Apply with `asset_member_apply_review` (the ticket of `plan`); the next copied-target save writes it.
#[derive(Serialize, TS)]
pub struct BoardReplacementPreviewDto {
    pub plan: Option<AssetMemberPlanDto>,
    pub refusal_kind: Option<String>,
    pub refusal: Option<String>,
    /// `pair` (loose + archive member, one transaction) or `archive-only`.
    pub topology: Option<String>,
    /// `DXT1` / `DXT3` and the declared level count.
    pub format: Option<String>,
    pub levels: Option<u32>,
    /// Every material whose slot names the raster (all of them change).
    pub materials: Vec<String>,
}

/// D8: the preview of a texture-member replacement, or the refusal.
#[derive(Serialize, TS)]
pub struct AssetMemberPreviewDto {
    pub plan: Option<AssetMemberPlanDto>,
    pub refusal_kind: Option<String>,
    pub refusal: Option<String>,
}

/// D8: the outcome of applying a held texture-member review.
#[derive(Serialize, TS)]
pub struct AssetMemberApplyDto {
    pub plan: AssetMemberPlanDto,
    /// How many replacements are pending after this apply.
    pub pending: u32,
    pub summary: ProjectSummaryDto,
}

/// One dictionary row named by a rename preview.
#[derive(Serialize, TS)]
pub struct DictionaryRowDto {
    pub record: usize,
    pub key: String,
    pub hash: u32,
    pub text: Option<String>,
}

/// The preview of a dictionary rename (LOC-W1): key A shows key B's text after the change. Every
/// consequence the dictionary itself can show is listed; which game screens read the key is not.
#[derive(Serialize, TS)]
pub struct DictionaryRepointPreviewDto {
    pub dictionary_path: String,
    pub dictionary_sha256: String,
    pub endian_big: bool,
    pub source: DictionaryRowDto,
    /// `exact` or `by-hash`.
    pub source_lookup: String,
    pub donor: DictionaryRowDto,
    pub donor_lookup: String,
    pub text_before: String,
    pub text_after: String,
    pub slot_offset: usize,
    pub old_pointer: u32,
    pub new_pointer: u32,
    pub hash_collisions: Vec<DictionaryRowDto>,
    pub sharing_old_text: Vec<DictionaryRowDto>,
    pub sharing_new_text: Vec<DictionaryRowDto>,
    pub warnings: Vec<String>,
}

/// One pending NHL 16–21 CSV stored-default edit (D4): held beside the database edits until the
/// next copied-target save installs the rewritten members.
#[derive(Serialize, TS)]
pub struct PendingSettingsCsvDto {
    pub setting_name: String,
    pub label_text: Option<String>,
    pub context: Option<i64>,
    pub context_name: Option<String>,
    pub before: i64,
    pub after: i64,
    pub before_label: Option<String>,
    pub after_label: Option<String>,
    pub paired: bool,
    pub members: Vec<SettingsCsvMemberDto>,
    pub description: String,
}

/// The pending dictionary rename in the review.
#[derive(Serialize, TS)]
pub struct PendingDictionaryDto {
    pub path: String,
    pub baseline_sha256: String,
    pub working_sha256: String,
    /// `repoint` (LOC-W1: pointer to another key's text) or `retext` (LOC-W2: in-place re-encode).
    pub kind: String,
    pub key: String,
    pub before: String,
    pub after: String,
    /// Empty for a `retext`.
    pub donor_key: String,
    /// Bytes that differ from the baseline (the pointer slot plus the CRC words that moved).
    pub changed_bytes: usize,
    pub changed_ranges: Vec<String>,
    pub crc_scopes_changed: Vec<String>,
    pub warnings: Vec<String>,
}

/// One archive member a copied-target review replaces with a plan artifact's bytes.
#[derive(Deserialize, Serialize, TS)]
pub struct ArchiveMemberSpecDto {
    /// Archive path relative to the copied root (e.g. `databigs/inxp_00.big`, `cacheboot.big`).
    pub archive_relative: String,
    /// Exact member path inside the archive.
    pub member_path: String,
    /// `primary` (the packaged main database) or `dictionary` (the pending rename's output).
    pub artifact_key: String,
}

/// One league identity as the loaded target stores it (see `studio_domain::leagues`).
#[derive(Serialize, TS)]
pub struct LeagueSummaryDto {
    /// The native code: `stockleagues.leagueid` on PS4, the team-side league code elsewhere. The
    /// stable selection key — labels may repeat (NHL 21 has three `CUSTOM_TEAMS` rows).
    #[ts(type = "number")]
    pub code: i64,
    /// `stockleagues` record index on PS4 — the handle field edits take; `None` on derived codes.
    pub record: Option<u32>,
    /// `stockleagues.name`: a localization key, shown as a key until a dictionary resolves it.
    pub name_key: Option<String>,
    /// Resolved display text when one is established for this target.
    pub label: Option<String>,
    /// What to print: `label`, else `name_key`, else `League code N`.
    pub display: String,
    /// `native` / `corroborated` / `candidate`, or `None` for an unlabelled code.
    pub confidence: Option<String>,
    /// gen7 only: the dictionary key the executable is believed to bind this code to, and how that
    /// is known (`traced` — read from an executable's static table; `hypothesis` — the key family
    /// exists in the title's own dictionary, the binding is untraced). The rename control's default.
    pub dictionary_key: Option<String>,
    pub dictionary_key_evidence: Option<String>,
    /// Plain-language provenance of the label / key.
    pub evidence: String,
    /// Teams whose league code is this league (every team-table row, active or not).
    pub member_teams: usize,
    /// Presentation ids on the PS4 row, surfaced raw (semantics not established).
    #[ts(type = "number | null")]
    pub art_id: Option<u64>,
    #[ts(type = "number | null")]
    pub render_id: Option<u64>,
    #[ts(type = "number | null")]
    pub order_id: Option<u64>,
    #[ts(type = "number | null")]
    pub primary_color: Option<u64>,
    #[ts(type = "number | null")]
    pub secondary_color: Option<u64>,
}

/// The loaded target's league catalog: every league identity plus how it was resolved.
#[derive(Serialize, TS)]
pub struct LeagueCatalogDto {
    /// `stockleagues` (PS4 rows) or `derived-codes` (per-team codes labelled by a vocabulary).
    pub source: String,
    /// Studio id of the table carrying the per-team league code (`kOtt`, `HqzU` on NHL 07).
    pub team_league_table: String,
    /// The `defs/leagues` vocabulary that labelled derived codes, when one was selected.
    pub vocabulary: Option<String>,
    /// Why derived codes are shown raw, when no vocabulary applies.
    pub unresolved_reason: Option<String>,
    /// Why league names cannot be edited on this target, when they cannot.
    pub name_edit_blocker: Option<String>,
    /// The target's own dictionary the labels were resolved through, when one is attached.
    pub dictionary: Option<DictionaryRefDto>,
    pub leagues: Vec<LeagueSummaryDto>,
}

/// The localization dictionary attached to a project.
#[derive(Serialize, TS)]
pub struct DictionaryRefDto {
    pub path: String,
    pub locale: String,
    pub sha256: String,
    pub rows: usize,
}

/// One league's detail: identity, the editable row (PS4) and the member teams.
#[derive(Serialize, TS)]
pub struct LeagueViewDto {
    pub summary: LeagueSummaryDto,
    /// The `stockleagues` row's fields (PS4 only; empty on derived-code titles).
    pub fields: Vec<FieldViewDto>,
    /// Studio id of the row's table when `fields` is populated.
    pub row_table: Option<String>,
    /// Byte limit of the `name` key field on this title (the edit constraint), when the row exists.
    pub name_max_bytes: Option<u32>,
    pub members: Vec<TeamSummaryDto>,
    /// Why the name cannot be edited on this title, when it cannot.
    pub name_edit_blocker: Option<String>,
    /// Phase 1.2: conferences/divisions from this title's competition table, when one applies.
    pub structure: Option<LeagueStructureDto>,
    /// P1-C: why no structure is shown although the team row carries group ids (no table for
    /// this target, no game id, or a table this database's rows contradict).
    pub structure_blocker: Option<String>,
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
#[derive(Clone, Debug, Serialize, TS)]
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
    pub name: String,
    pub record: u32,
    pub identity: Vec<FieldViewDto>,
    pub arena: Option<ArenaRefDto>,
    pub roster_count: usize,
    /// Retired jersey numbers (`wcSL`), sorted ascending.
    pub retired_numbers: Vec<u32>,
    /// The team's native league code and the league catalog's display text for it (a resolved
    /// label, else the localization key shown as a key, else `League code N`).
    #[ts(type = "number | null")]
    pub league: Option<i64>,
    pub league_label: Option<String>,
    /// Conference / division labels, only where anchored for this title under this league;
    /// `None` means the raw group id is the only truthful display.
    pub conference_label: Option<String>,
    pub division_label: Option<String>,
    pub all_fields: Vec<FieldViewDto>,
    /// The server-driven curated section layout the UI renders (per-game field profile).
    pub profile: EntityFieldProfileDto,
}

/// C8: one integer cell of a multi-cell write (`set_cells_int`).
#[derive(Deserialize, Serialize, TS, Clone)]
pub struct CellWriteDto {
    pub field: String,
    #[ts(type = "number")]
    pub value: i64,
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

/// The new `xjgw` row created by a jersey clone (the DB half of "Add New Jersey").
#[derive(Serialize, TS)]
pub struct CloneJerseyResultDto {
    pub record: u32,
    pub outcome: OpOutcomeDto,
}

/// The in-game **jersey-select thumbnail** for one kit, resolved from the FE tree through the
/// game's own fallback chain (stock `jersey_{style}_{art}_{variant}.big` → shared `custom*.big`).
#[derive(Serialize, TS)]
pub struct JerseyThumbnailDto {
    /// `false` when the FE tree ships neither the stock archive nor any custom fallback.
    pub found: bool,
    /// PNG data URL of the decoded thumbnail; `None` when not found.
    pub data_url: Option<String>,
    pub width: u32,
    pub height: u32,
    /// Install-relative path (forward slashes) of the archive for the texture replace flow — set
    /// only for the kit's **own stock** archive under the install root. The shared `custom*.big`
    /// fallbacks are shown but never offered as a write target (they back every custom kit).
    pub rel_path: Option<String>,
    /// `true` when the image came from the kit's own stock archive (vs. a shared custom fallback).
    pub stock: bool,
}

/// One texture file cloned onto a new kit's asset id.
#[derive(Serialize, TS)]
pub struct ClonedKitFileDto {
    pub src: String,
    pub dst: String,
    /// Embedded asset-id occurrences rewritten inside the copied bytes.
    pub rewrites: u32,
}

/// The file half of "Add New Jersey": which textures were cloned onto the new asset id.
#[derive(Serialize, TS)]
pub struct CloneKitTexturesSummaryDto {
    pub files: Vec<ClonedKitFileDto>,
    /// Destination files that already existed and were left untouched.
    pub skipped_existing: Vec<String>,
}

/// One colour zone of a stock-equipment row, decoded to RGB.
#[derive(Serialize, TS)]
pub struct EquipmentZoneDto {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub packed_field: Option<String>,
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

/// Phase 2.5 (22 Sep 2026): one venue field of the open title's arena table — its candidate class
/// (research 2026-09-21 pm handoff R51, transcribed into `defs/arena/venue-fields.toml`), whether
/// it is constant across every arena row of *this* file, its colour rendering, and whether Studio
/// offers it as an edit. A class is a naming candidate, never a traced runtime effect.
#[derive(Serialize, TS)]
pub struct VenueFieldDto {
    pub name: String,
    pub label: String,
    pub class: String,
    pub class_note: String,
    #[ts(type = "number")]
    pub value: i64,
    /// `#RRGGBB` when this field is a measured 24-bit colour zone and the value is non-zero.
    pub colour: Option<String>,
    pub distinct: u32,
    /// The value every arena row carries, when the field never varies on this title.
    #[ts(type = "number | null")]
    pub constant: Option<i64>,
    /// What this value names in a catalogue, when one resolves it (PS4 `goalhorn`).
    pub resolved: Option<String>,
    /// The rule that resolved it, or why nothing did.
    pub resolved_note: Option<String>,
    /// The dictionary key the resolved row names, kept raw beside the text.
    pub resolved_key: Option<String>,
    /// `exact` / `by-hash` / `absent` / `no-dictionary` — how that key resolved through this
    /// project's own dictionary.
    pub resolved_key_lookup: Option<String>,
    /// Gen7 goal / period horn: the `.abk` the sound-bank browser opens (`?bank=`); never set for
    /// chant / audio-id fields.
    pub bank_file: Option<String>,
    pub editable: bool,
    /// Why not, when `editable` is false.
    pub refusal: Option<String>,
}

/// Phase 2.5: the whole venue-parameter inventory for one arena.
#[derive(Serialize, TS)]
pub struct ArenaVenueDto {
    #[ts(type = "number")]
    pub arena_id: u64,
    pub record: u32,
    /// Arena rows in this file — the population the constants were measured over.
    pub rows: u32,
    pub fields: Vec<VenueFieldDto>,
    /// `(class, count)` in display order.
    pub class_counts: Vec<(String, usize)>,
    pub notes: Vec<String>,
}

/// Phase 4 (21 Sep 2026): one equipment slot of a player checked against the title's rendering
/// tree (research 2026-09-21 handoff 12): the assigned id's files, the family's catalogue on
/// this title, the note.
#[derive(Serialize, TS)]
pub struct EquipmentFileCheckDto {
    pub slot: String,
    pub label: String,
    pub field_ref: String,
    pub folder: String,
    pub id: Option<u32>,
    pub model_label: Option<String>,
    /// Capped at 24; `file_count` is the full count.
    pub files: Vec<String>,
    pub file_count: usize,
    pub catalogue_ids: usize,
    pub catalogue_min: Option<u32>,
    pub catalogue_max: Option<u32>,
    pub note: String,
    /// Phase 4 (22 Sep 2026): `measured` / `single_container` / `not_measured` — the component-set
    /// contract for this slot on the open title (research R52).
    pub set_contract: String,
    /// The contract in words.
    pub set_rule: String,
    /// Per-group check when the contract is measured and the id has files; empty otherwise.
    pub component_groups: Vec<ComponentGroupDto>,
}

/// Equipment 4.5 G2 (receipt 42; research R125 / R125b): a team's goalie mask paints — slot `k`, starter
/// and backup — with the file check a goalie's own `maskid` gets on the gen7 loose trees. Stored numbers;
/// nothing says which uniform uses a slot or what the game draws.
#[derive(Serialize, TS)]
pub struct TeamGoalieMasksDto {
    /// `false` on titles without team mask slots (NHL 07–09).
    pub applies: bool,
    pub paints: Vec<TeamMaskPaintDto>,
    /// Distinct paint ids with files in the title's `goaliehelmet` folder (0 on PS4 / without a tree).
    pub catalogue_ids: usize,
    pub notes: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct TeamMaskPaintDto {
    pub slot: u32,
    /// `starter` / `backup`.
    pub role: String,
    /// The metadata field name (`goaliemask0_starter`), for disclosure.
    pub field_name: String,
    pub field_ref: String,
    pub value: Option<u32>,
    /// Capped at 12; `file_count` is the full count.
    pub files: Vec<String>,
    pub file_count: usize,
    /// `empty` / `files` / `no_files` / `no_folder` / `shipped_exception` / `not_checked`.
    pub status: String,
    pub note: String,
    /// PS4 with the title's vault read (receipt 51, G3): the helmet types the paint's design exists for; empty = no
    /// catalogue record. `None` when no catalogue was read. A slot has no helmet type of its own.
    #[ts(optional)]
    pub helmet_types: Option<Vec<i64>>,
}

/// One component group of an equipment id's set (the in-game files or the front-end copy).
#[derive(Serialize, TS)]
pub struct ComponentGroupDto {
    /// `in-game` / `front-end`.
    pub group: String,
    /// `complete` / `incomplete`.
    pub status: String,
    pub present: Vec<String>,
    pub missing: Vec<String>,
}

/// One equipment id and the render files it owns, from scanning the slot folder.
#[derive(Serialize, TS)]
pub struct EquipmentSlotAssetDto {
    pub id: u32,
    /// Resolved model name from the slot's anchored enum vocabulary ("CCM Ribcore 40K"), or
    /// `null` for an uncatalogued id / a slot with no vocabulary — the UI then shows the raw id.
    pub label: Option<String>,
    pub files: Vec<EquipmentSlotFileDto>,
}

/// One Hockey Shop store item — a texture in the disk-backed library. Also the
/// on-disk `store.json` row shape, so it round-trips through Deserialize.
#[derive(Serialize, Deserialize, TS, Clone)]
pub struct HockeyShopItemDto {
    /// Stable slug id (manifest key + PNG file stem).
    pub id: String,
    pub name: String,
    pub author: String,
    /// `"game"` (extracted from the install) or `"import"` (operator PNG).
    pub source: String,
    /// The game equipment id a `"game"` item was extracted from; `null` for imports.
    #[serde(default)]
    #[ts(optional)]
    pub game_asset_id: Option<u32>,
    /// PNG filename inside the slot's store directory.
    pub file: String,
    pub width: u32,
    pub height: u32,
    #[ts(type = "number")]
    pub added_unix: u64,
}

/// Result of one store sync pass (extract game textures into the store).
#[derive(Serialize, TS)]
pub struct HockeyShopSyncSummaryDto {
    pub added: u32,
    pub already_present: u32,
    pub failed: Vec<String>,
    pub store_dir: String,
}

/// Request to install a store item into the game at a target equipment id
/// (existing id → overwrite its colour map; unused id → create by cloning a donor).
#[derive(Deserialize, TS)]
pub struct InstallStoreItemRequestDto {
    pub slot: String,
    pub entry_id: String,
    pub target_id: u32,
    /// For a CREATE install: which shipped id's container to clone as the template.
    /// `null` = auto (the item's source game id if shipped, else the lowest id).
    #[serde(default)]
    #[ts(optional)]
    pub donor_id: Option<u32>,
    /// Back up files before overwriting. Defaults to `true`.
    #[serde(default)]
    #[ts(optional)]
    pub write_backup: Option<bool>,
    /// The install's title id (`nhl12`, …): picks the title's file naming for the slot (NHL 12 pads keep
    /// the id at the second position — receipt 40 / 45). `null` = the default naming.
    #[serde(default)]
    #[ts(optional)]
    pub game_id: Option<String>,
    /// Apply only: the `plan_fingerprint` of the plan the operator reviewed. Apply refuses without it or when the
    /// recomputed plan differs (receipt 47).
    #[serde(default)]
    #[ts(optional)]
    pub reviewed_fingerprint: Option<String>,
}

/// Read-only plan for an install — drives the modal's overwrite/create preview and
/// gates Apply on `blocking_errors`. Touches no files.
#[derive(Serialize, TS, Clone)]
pub struct InstallStoreItemPlanDto {
    /// The title whose file naming the plan used (the open project's, receipt 47); `null` for a slot named the
    /// same on every title when no title is known.
    pub title: Option<String>,
    /// What was reviewed (title, files and their bytes, store item, target, donor); apply must name it.
    pub plan_fingerprint: String,
    pub slot: String,
    pub entry_id: String,
    pub entry_name: String,
    pub target_id: u32,
    /// `"overwrite"` (target id already ships assets) or `"create"` (empty id).
    pub mode: String,
    /// The label already occupying the target id (overwrite mode).
    #[ts(optional)]
    pub target_existing_label: Option<String>,
    /// Donor id whose container is cloned (create mode).
    #[ts(optional)]
    pub donor_id: Option<u32>,
    /// Rendering-relative file that receives the store texture.
    #[ts(optional)]
    pub target_color_file: Option<String>,
    pub target_texture_index: u32,
    pub target_width: u32,
    pub target_height: u32,
    /// Files cloned from the donor (create mode; dest rel paths).
    pub cloned_files: Vec<String>,
    pub warnings: Vec<String>,
    pub blocking_errors: Vec<String>,
}

/// Result of applying an install.
#[derive(Serialize, TS)]
pub struct InstallStoreItemApplyResponseDto {
    pub plan: InstallStoreItemPlanDto,
    /// Rendering-relative files written (cloned container files + the spliced file).
    pub files_written: Vec<String>,
    /// The game-side install ledger path (records what's installed at each id).
    #[ts(optional)]
    pub ledger_path: Option<String>,
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

/// Teams 2.2 (24 Sep 2026) — what assigning `team_id` to home arena `to` would change, shown
/// **before** the `kOtt` arena cell is written: the exact cell the write targets, the arena the
/// team leaves and who stays there (or that it becomes vacant), the arena it joins and who already
/// plays there. Names are resolved; a refusal (no home-arena link on this title, the same arena,
/// an arena that is not in the `StEO` catalogue, an unknown team) carries its reason and
/// `allowed == false`. The write itself stays the ordinary validated `set_field` on that cell.
#[derive(Debug, Serialize, TS)]
pub struct ArenaAssignmentPreviewDto {
    #[ts(type = "number")]
    pub team_id: u64,
    pub team_name: String,
    /// The cell the assignment writes (`kOtt`, the team's record, the arena FK field), or `null`
    /// when this title's team table carries no home-arena link.
    pub table: Option<String>,
    pub record: Option<u32>,
    pub field: Option<String>,
    /// The team's current arena (`null` = unset or uncatalogued; `from_raw` then carries the raw id).
    pub from: Option<ArenaRefDto>,
    #[ts(type = "number | null")]
    pub from_raw: Option<u64>,
    /// The other teams that keep playing at the current arena after the move.
    pub from_other_tenants: Vec<String>,
    /// `true` when no team is left at the current arena after the move.
    pub from_becomes_vacant: bool,
    /// The target arena as catalogued, or `null` when `to_raw` is not a `StEO` row.
    pub to: Option<ArenaRefDto>,
    #[ts(type = "number")]
    pub to_raw: u64,
    /// The teams already playing at the target arena.
    pub to_tenants: Vec<String>,
    pub allowed: bool,
    pub refusal: Option<String>,
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

/// Result of a player-manifest CSV export.
#[derive(Serialize, TS)]
pub struct PlayerManifestSummaryDto {
    /// Where the CSV was written.
    pub path: String,
    #[ts(type = "number")]
    pub players: i64,
    /// How many have a real (non-placeholder) portrait installed.
    #[ts(type = "number")]
    pub with_portrait: i64,
    /// How many have a head mesh installed.
    #[ts(type = "number")]
    pub with_head_mesh: i64,
    /// How many produced a usable export slug (needs a name + a plausible birthdate).
    #[ts(type = "number")]
    pub with_slug: i64,
}

// ---------------------------------------------------------------- legends

/// Where the Legends precompute stands for the active formula profile — drives both the Formulas
/// tab and the app-level pending-action banner.
#[derive(Serialize, TS)]
pub struct LegendsStatusDto {
    /// The formula profile the app is currently working under.
    pub active_profile: String,
    /// True when a completed run exists for `active_profile` AND matches the open corpus.
    pub computed: bool,
    /// True when a run exists but was built against a different corpus build — it can't be trusted,
    /// so it counts as not computed and the user is asked to recalculate.
    pub stale: bool,
    pub computed_at: Option<String>,
    #[ts(type = "number")]
    pub player_count: i64,
    /// The user saved a profile and declined the calculation — the banner persists until they run
    /// it or dismiss it.
    pub pending: bool,
    /// The user acknowledged that Legends is unavailable under this profile.
    pub dismissed: bool,
}

/// The result of one Legends precompute pass.
#[derive(Serialize, TS)]
pub struct LegendsComputeSummaryDto {
    pub profile: String,
    /// People examined (everyone in the corpus with a derivable position).
    #[ts(type = "number")]
    pub examined: i64,
    /// People that produced a Legends composite (the rest have no usable rating seasons).
    #[ts(type = "number")]
    pub rated: i64,
    #[ts(type = "number")]
    pub elapsed_ms: i64,
}

/// One built Legends team, as the Puck's Picks list shows it.
#[derive(Serialize, TS)]
pub struct LegendsTeamSummaryDto {
    /// `franchise:<id>`, or one of the two residual-team keys.
    pub key: String,
    pub name: String,
    pub abbr: Option<String>,
    /// True for European Elite / Windsor Legends — they have no franchise identity to wear.
    pub is_special: bool,
    #[ts(type = "number")]
    pub forwards: i64,
    #[ts(type = "number")]
    pub defense: i64,
    #[ts(type = "number")]
    pub goalies: i64,
}

/// One player placed onto a Legends roster.
#[derive(Serialize, TS)]
pub struct LegendsMemberDto {
    #[ts(type = "number")]
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    /// `C`/`LW`/`RW`/`D`/`G`.
    pub position: String,
    pub is_goalie: bool,
    pub dob: String,
    pub nationality: Option<String>,
    /// The Legends Overall on the EA 0..99 scale (composited attributes → EA Overall formula).
    #[ts(type = "number")]
    pub overall: i64,
    /// NHL regular-season games with this franchise (0 on a residual team).
    #[ts(type = "number")]
    pub team_gp: i64,
    /// True when they were crowded off their most-tenured franchise onto their second.
    pub second_choice: bool,
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
    /// This head model is this player's own — a uniquely-referenced, non-pool head that
    /// resolves on disk (== `mesh_path.is_some()`). The only case where naming a mesh after
    /// them is truthful.
    pub bespoke_head: bool,
    /// A shared generic head in the pool id range (>= 60000) — the Legacy/NHL 15 convention.
    pub pool_head: bool,
    /// This head id is referenced by more than one player — a shared generic head (the
    /// NHL 08–14 low-id convention), not this player's own.
    pub head_shared: bool,
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
    /// Heads not written because `pPaD` is a shared pool id (captured generically).
    pub meshes_skipped_pool: usize,
    /// Heads not written because `pPaD` is shared by several players (captured generically).
    pub meshes_skipped_shared: usize,
    /// Heads not written because the player has no head assigned (`pPaD == 0`).
    pub meshes_skipped_no_head: usize,
    /// Heads not written because a non-pool `pPaD` resolved no mesh file on disk.
    pub meshes_skipped_missing: usize,
    pub failures: Vec<String>,
    #[ts(type = "number")]
    pub bytes_written: u64,
}

/// What an appearance-catalog export produced (Phase 5). The full per-asset rows
/// live in the on-disk `catalog.json`; this carries the totals + failures for the UI.
#[derive(Serialize, TS)]
pub struct HeadCatalogSummaryDto {
    pub heads_written: usize,
    pub hair_written: usize,
    pub facial_hair_written: usize,
    pub textures_written: usize,
    /// Total catalog assets (rows in the manifest).
    pub entry_count: usize,
    /// Bespoke players skipped for lacking a usable name/birthdate (captured
    /// generically by the sweep instead).
    pub skipped_no_base: usize,
    /// Relative path of the written `catalog.json`, if written.
    pub manifest_file: Option<String>,
    pub failures: Vec<String>,
    #[ts(type = "number")]
    pub bytes_written: u64,
}

/// One asset row in a built appearance catalog's `catalog.json` — what the
/// Appearance-catalog browser lists and previews.
#[derive(Serialize, TS)]
pub struct CatalogEntryDto {
    /// File stem, e.g. `ovechkin_alexander_17_09_1985_hair_nhllegacy`.
    pub name: String,
    /// `player` (a bespoke player's own head) or a swept role: `generic` /
    /// `coach` / `referee` / `trainer` / `commissioner`.
    pub origin: String,
    /// `head` | `hair` | `facialhair`.
    pub class: String,
    /// Game token (`nhl14`, `nhllegacy`, …).
    pub game: String,
    /// Catalog-relative path of the written `.glb`.
    pub mesh_file: String,
    /// Submesh names that went into this asset (also the filter for its preview).
    pub submeshes: Vec<String>,
    /// Catalog-relative paths of the head/hair's shared PNG textures.
    pub textures: Vec<String>,
    /// Absolute source texture containers those PNGs came from — fed back to the
    /// viewer so the 3D preview is textured like the game.
    pub texture_sources: Vec<String>,
    /// The absolute source mesh container this asset was cut from (provenance;
    /// also what the preview decodes and filters to `submeshes`).
    pub source: String,
}

/// A built appearance catalog read back for browsing — the totals plus every
/// asset row. Loaded from `{dir}/catalog.json`.
#[derive(Serialize, TS)]
pub struct CatalogManifestDto {
    pub version: u32,
    pub game: String,
    pub head_count: usize,
    pub hair_count: usize,
    pub facial_hair_count: usize,
    pub entries: Vec<CatalogEntryDto>,
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
    /// Which container it was: `"gnf"` (PS4), `"portrait"` (BIGF), `"texlib"` (RW4 `.rx2` / `.rpsgl`) or `"png"` (PS4 NHL 15).
    pub container: String,
    pub width: u32,
    pub height: u32,
    #[ts(type = "number")]
    pub bytes_written: u64,
    /// The `.bak` written before the first overwrite of this file, when a backup was requested and
    /// none existed yet.
    pub backup_path: Option<String>,
    /// Writer disclosures to show with the result: empty on the ordinary path; a BIGF package
    /// whose shape the span-preserving re-plan did not recognise reports that it was rebuilt
    /// through the generic builder (a layout no shipped package exhibits).
    pub notes: Vec<String>,
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
#[derive(Debug, Serialize, TS)]
pub struct TexturePreviewDto {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    /// The container the texture decoded from — `"gnf"`, `"portrait"`, `"texlib"` or `"png"` — or `null`
    /// for an operator-supplied source image (which isn't a game container).
    pub container: Option<String>,
}

/// What a Replace… would do, decided **before Apply** from the target's bytes and the source's
/// dimensions (coordinator review of receipts 10–17, F1). For an Xbox 360 `RW4xb2` DXT raster it
/// is the writer's own policy: a refusal (changed size on a mipped raster) or the partial-
/// replacement disclosure the result will repeat word for word. For every other container
/// `checked` is `false`: no pre-check exists and the writer's rule decides at write time.
#[derive(Debug, Serialize, TS)]
pub struct TextureReplacePlanDto {
    /// The server-held review this plan is (24 Sep 2026 review F1). `import_texture` /
    /// `import_texture_bytes` name it; the write refuses when the target's bytes, the source, the
    /// raster, the selection or the recomputed policy differ from what was shown. Consumed by the
    /// write that names it.
    pub review_id: u64,
    /// `"gnf"`, `"portrait"`, `"texlib"`, `"png"` or `null` when the bytes are not a recognised container.
    pub container: Option<String>,
    pub checked: bool,
    pub allowed: bool,
    pub refusal: Option<String>,
    pub notes: Vec<String>,
    /// The reviewed source's dimensions, as the backend decoded them.
    pub source_width: u32,
    pub source_height: u32,
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

// ---- S2-C copied-target transactions ------------------------------------------------------------
//
// The review/apply/status/recover/restore contract over the shared `tdb-txn` engine. Everything here
// is **server-derived**: the frontend receives a handle it hands back; it never supplies identities,
// manifests, validation results or destinations. There is no live mode and no force option.

/// A file's identity as the transaction engine binds it (length + SHA-256).
#[derive(Serialize, TS)]
pub struct FileIdentityDto {
    #[ts(type = "number")]
    pub len: u64,
    pub sha256: String,
}

/// One item of a producer's qualification record, carried verbatim (never resolved here).
#[derive(Serialize, TS)]
pub struct QualificationItemDto {
    pub subject: String,
    /// `resolved` / `unresolved` / `blocked`.
    pub status: String,
    pub note: String,
}

/// One destination the prepared change set replaces (or creates).
#[derive(Serialize, TS)]
pub struct CopiedTargetArtifactDto {
    /// Engine key (`primary`, `roster`, `primary.header`, …) — for cross-reference, not display.
    pub key: String,
    /// What this is, for the user: "Main database (nhlng.db)", "Save index header for ROSTER …".
    pub label: String,
    /// `database` | `roster-save` | `content-header` | `archive`.
    pub kind: String,
    /// Destination relative to the copied-target root (forward slashes).
    pub destination: String,
    /// Destination as an absolute path in the copied tree.
    pub destination_path: String,
    /// The identity the destination must still carry at apply; `null` = must be absent (a new file).
    pub prior: Option<FileIdentityDto>,
    /// The identity the destination will carry after apply.
    pub output: FileIdentityDto,
    /// Keys applied before this one.
    pub dependencies: Vec<String>,
    pub producer: String,
    /// `qualified` | `unqualified` — every Studio-produced artifact is unqualified today.
    pub qualification_state: String,
    pub qualification_items: Vec<QualificationItemDto>,
}

/// A file the plan was derived from but does not replace; re-verified before apply.
#[derive(Serialize, TS)]
pub struct BoundInputDto {
    /// `content-header` (an existing save index header the plan relies on and preserves as-is) or
    /// `header-template` (the sibling a new header's bytes were cloned from).
    pub kind: String,
    pub purpose: String,
    pub path: String,
    pub identity: FileIdentityDto,
}

/// A destination an older transaction no longer owns, and the later transaction that took it.
#[derive(Serialize, TS)]
pub struct SupersededDestinationDto {
    pub destination: String,
    pub by_txn_id: String,
}

/// One transaction under a copied-target root, as the engine's journal records it.
#[derive(Serialize, TS)]
pub struct CopiedTargetTransactionDto {
    pub txn_id: String,
    #[ts(type = "number")]
    pub sequence: u64,
    /// `prepared` | `staged` | `applying` | `committed` | `restoring` | `restored` | `aborted` |
    /// `rolled-back` | `blocked`, or `unreadable` when the journal could not be read.
    pub state: String,
    /// Why the transaction blocks the root, when it does (unreadable journal, unresolved
    /// destination); the recovery material is retained and never deleted by Studio.
    pub problem: Option<String>,
    pub superseded: Vec<SupersededDestinationDto>,
    /// Where the journal, backups and staged files live.
    pub recovery_dir: String,
    pub applied_unqualified: bool,
    /// Destinations (root-relative) in apply order.
    pub destinations: Vec<String>,
    /// `true` when `restore` is the next sensible operation (state committed, not superseded).
    pub restorable: bool,
}

/// A prepared review the registry still holds.
#[derive(Serialize, TS)]
pub struct PendingHandleDto {
    pub handle: u32,
    pub project_id: u32,
    pub txn_id: String,
    /// `true` while an apply is running for it.
    pub in_progress: bool,
}

/// Files and workspaces held by one copied-target operation in progress.
#[derive(Serialize, TS)]
pub struct ReservationDto {
    /// "apply of review handle 3 (project 0)", "recovery of <root>", "restore of transaction … in <root>".
    pub holder: String,
    pub root: String,
    /// The whole root is held (recovery / restore) rather than named files (apply).
    pub whole_root: bool,
    /// Every open workspace on the held files: the initiator and its aliases.
    pub projects: Vec<u32>,
    /// The held files (an apply's destinations and bound inputs); empty when `whole_root`.
    pub files: Vec<String>,
}

/// A staging directory the registry created for a review and has not removed yet (S2-C-R2/R3).
#[derive(Debug, Serialize, TS)]
pub struct StagingLeftoverDto {
    pub path: String,
    /// `removal-failed` — the set is resolved, only the directory removal failed (retried at the
    /// next teardown); `unresolved-transaction` — the prepared set could not be aborted and still
    /// references these files, so they are deliberately retained until it is resolved.
    pub kind: String,
    /// The transaction an `unresolved-transaction` leftover belongs to.
    pub txn_id: Option<String>,
    /// Why, in words.
    pub error: String,
}

/// The root's transaction state: what is there, whether another apply may proceed, and what to do.
#[derive(Serialize, TS)]
pub struct CopiedTargetStatusDto {
    pub root: String,
    pub transactions: Vec<CopiedTargetTransactionDto>,
    /// The root refuses new transactions (an active, interrupted, blocked or unreadable set).
    pub blocked: bool,
    pub active_txn_id: Option<String>,
    pub pending_handles: Vec<PendingHandleDto>,
    /// Projects whose sessions are locked by an apply/recover/restore in progress (any root).
    pub busy_projects: Vec<u32>,
    /// The operations holding files under, over or equal to this root right now.
    pub reservations: Vec<ReservationDto>,
    /// Staging directories (any root) this registry could not remove yet: tracked cleanup
    /// obligations, retried at every later teardown, never silently dropped.
    pub staging_leftovers: Vec<StagingLeftoverDto>,
    /// A new review/apply may proceed right now.
    pub apply_allowed: bool,
    /// What the user should do next, in words.
    pub guidance: String,
}

/// The result of `copied_target_review`: a prepared, server-owned change set.
#[derive(Serialize, TS)]
pub struct CopiedTargetReviewDto {
    /// Hand this back to `copied_target_apply` / `copied_target_discard` / `copied_target_cancel`.
    pub handle: u32,
    pub project_id: u32,
    pub txn_id: String,
    pub root: String,
    /// The workspace content revision the preparation is bound to; an edit after review changes it
    /// and apply refuses.
    pub workspace_revision: String,
    /// Always `copied-target`.
    pub mode: String,
    /// `unqualified` unless every artifact is qualified (none is today).
    pub qualification_state: String,
    /// Always `false`: this surface never writes a live game tree.
    pub live_install_allowed: bool,
    pub artifacts: Vec<CopiedTargetArtifactDto>,
    pub bound_inputs: Vec<BoundInputDto>,
    pub status: CopiedTargetStatusDto,
}

/// A session's baseline after an apply, restore, recovery or reload.
#[derive(Serialize, TS)]
pub struct ProjectBaselineDto {
    pub project_id: u32,
    /// `unchanged` | `rebound` | `reloaded` | `stale-kept-edits` | `no-baseline` |
    /// `refresh-failed`.
    pub primary: String,
    pub roster: Option<String>,
    pub dirty: bool,
    /// Set when the baseline changed under unsaved edits, or a refresh/reload failed: copied-target
    /// saves refuse until an explicit reload succeeds.
    pub stale_baseline: Option<String>,
    /// The error when the workspace could not be refreshed after an apply, restore or recovery
    /// (file missing, unreadable, malformed, alignment not derivable). Nothing in the workspace
    /// changed and its edits are kept; `primary`/`roster` read `refresh-failed`.
    pub refresh_error: Option<String>,
}

/// The acknowledged result of closing a project (LEAGUE-SMOKE-R1). The frontend disposes of its
/// owner only on `closed` / `already_closed`; `busy` and `failed` keep the session open on the
/// server and the UI must keep its owner (and the running operation's controls) in place.
#[derive(Serialize, TS)]
pub struct CloseProjectDto {
    pub project_id: u32,
    /// `closed` — the workspace was released now. `already_closed` — no such project is open
    /// (idempotent). `busy` — a copied-target operation (apply / restore / recovery) still holds
    /// the session; wait for it or cancel it, then retry. `failed` — a prepared review could not
    /// be discarded, so the session stays open (its staging is a tracked leftover).
    pub outcome: String,
    /// Plain-language reason for `busy` / `failed`, naming the holder or the discard error.
    pub reason: Option<String>,
}

/// One destination's identity after an apply or restore.
#[derive(Serialize, TS)]
pub struct DestinationOutcomeDto {
    pub key: String,
    pub destination: String,
    /// `null` = absent.
    pub identity: Option<FileIdentityDto>,
}

/// The result of a successful `copied_target_apply`.
#[derive(Serialize, TS)]
pub struct CopiedTargetApplyDto {
    pub handle: u32,
    pub txn_id: String,
    /// `committed`.
    pub state: String,
    pub recovery_dir: String,
    pub destinations: Vec<DestinationOutcomeDto>,
    pub applied_unqualified: bool,
    /// The workspace's sessions now use the installed files as their baselines.
    pub project: ProjectBaselineDto,
    /// Other open workspaces on the same files, refreshed after the commit (clean → reloaded,
    /// dirty → edits kept and stale, unreadable → refresh-failed).
    pub aliases: Vec<ProjectBaselineDto>,
    /// The engine's protocol events, in order.
    pub log: Vec<String>,
    pub status: CopiedTargetStatusDto,
}

/// The result of `copied_target_restore`.
#[derive(Serialize, TS)]
pub struct CopiedTargetRestoreDto {
    pub txn_id: String,
    /// `restored`.
    pub state: String,
    pub destinations: Vec<DestinationOutcomeDto>,
    /// Open workspaces whose files lie under the root, and what happened to their baselines.
    pub affected: Vec<ProjectBaselineDto>,
    pub status: CopiedTargetStatusDto,
}

/// The result of `copied_target_recover`.
#[derive(Serialize, TS)]
pub struct CopiedTargetRecoveryDto {
    pub affected: Vec<ProjectBaselineDto>,
    pub status: CopiedTargetStatusDto,
}

// ---------------------------------------------------------------------------------------------
// Xbox 360 disc images (p1.1-xbox360-iso-member-route)
// ---------------------------------------------------------------------------------------------

/// Where a file was found inside a disc image: the disc member and, when packed, the archive
/// entry (raw bytes at an absolute offset of the archive).
#[derive(Debug, Serialize, TS)]
pub struct IsoLocatedDto {
    /// Loose member path, or the root archive's path when packed.
    pub member_path: String,
    pub member_sector: u32,
    pub member_size: u32,
    /// `None` for a loose member.
    pub entry_path: Option<String>,
    pub entry_index: Option<usize>,
    pub entry_offset: Option<u64>,
    pub stored_size: Option<u32>,
    /// Plain-language description.
    pub describe: String,
}

/// One root archive that was probed while locating the database.
#[derive(Debug, Serialize, TS)]
pub struct IsoProbedArchiveDto {
    pub path: String,
    pub size: u32,
    pub verdict: String,
}

/// The read-only inspection of a disc image: layout, member count, where the database and the
/// dictionary live, and their identities as read from the image.
#[derive(Debug, Serialize, TS)]
pub struct IsoInspectDto {
    pub image: String,
    pub layout: String,
    pub members: usize,
    pub nhlng: Option<IsoLocatedDto>,
    pub nhlng_sha256: Option<String>,
    pub nhlng_len: Option<u64>,
    pub nhlng_tables: Option<usize>,
    pub dictionary: Option<IsoLocatedDto>,
    pub dictionary_sha256: Option<String>,
    pub dictionary_len: Option<u64>,
    pub dictionary_rows: Option<usize>,
    pub probed: Vec<IsoProbedArchiveDto>,
}

/// A staging folder for a disc image: the loose copies Studio edits, and where they came from.
#[derive(Debug, Serialize, TS)]
pub struct IsoStagedDto {
    pub root: String,
    pub image: String,
    pub layout: String,
    pub game_id: String,
    pub platform: String,
    pub db_path: String,
    pub meta_xml: String,
    pub dictionary_path: Option<String>,
    /// Prior identities (as read from the image) per staged file role.
    pub files: Vec<IsoStagedFileDto>,
    pub reused: bool,
    pub staged_at: String,
    /// The project this staging was opened as (R2 F2: the route is project-bound); `None` from
    /// `iso_stage`, which binds nothing.
    pub project_id: Option<u32>,
}

#[derive(Debug, Serialize, TS)]
pub struct IsoStagedFileDto {
    pub role: String,
    pub staged: String,
    pub prior_sha256: String,
    pub prior_len: u64,
    pub located: IsoLocatedDto,
}

/// One staged file's part of an apply plan.
#[derive(Debug, Serialize, TS)]
pub struct IsoPlannedFileDto {
    pub role: String,
    pub staged: String,
    pub staged_sha256: String,
    /// `false` when the staged file still carries its prior identity (nothing to write).
    pub changed: bool,
    pub member_path: Option<String>,
    pub entry_path: Option<String>,
    pub start: Option<u64>,
    pub end: Option<u64>,
    pub prior_sha256: Option<String>,
    pub replacement_sha256: Option<String>,
}

/// The apply plan for a project's staging folder, verified against the image as it is now and
/// held by the server under `review_id` (R2 F2): `iso_apply` names the review, and the plan it
/// describes is exactly what is written — never a plan recomputed after the review.
#[derive(Debug, Serialize, TS)]
pub struct IsoPlanDto {
    pub review_id: u64,
    pub project_id: u32,
    /// The workspace revision the plan was reviewed at; an apply refuses when it moved.
    pub workspace_revision: String,
    /// The project had unsaved edits at review time (the plan describes the files on disk; an
    /// apply refuses such a review).
    pub dirty: bool,
    pub image: String,
    pub root: String,
    pub files: Vec<IsoPlannedFileDto>,
    /// Number of files that would be written.
    pub writes: usize,
}

/// Where the image route stands for a project.
#[derive(Debug, Serialize, TS)]
pub struct IsoStatusDto {
    pub project_id: u32,
    pub root: String,
    pub image: String,
    /// The operation holding the project/image right now (an apply or restore), if any.
    pub in_flight: Option<String>,
    pub pending_reviews: usize,
    /// Always `false`: a started range write completes or is put back; it is not cancellable.
    pub cancellable: bool,
    pub note: String,
}

/// One recorded range patch and its current state on the image.
#[derive(Debug, Serialize, TS)]
pub struct IsoRecordDto {
    pub records_dir: String,
    pub record_dir: String,
    pub image: String,
    pub member_path: String,
    pub entry_path: String,
    pub start: u64,
    pub end: u64,
    pub prior_sha256: String,
    pub replacement_sha256: String,
    /// `backed-up` / `writing` / `applied` / `restoring` / `restored` as recorded.
    pub status: String,
    /// `prior` / `replacement` / `partial` / `mixed` / `unknown`: what the image holds at the
    /// range right now (`partial` = an interrupted apply/restore on an unfinished record,
    /// recoverable; `mixed` = old/new bytes on a completed record — a later external change,
    /// refused).
    pub on_image: String,
}

/// One range an apply attempted before it failed, with its state after the rollback.
#[derive(Debug, Serialize, TS)]
pub struct IsoAttemptedDto {
    pub role: String,
    pub entry_path: String,
    pub record: IsoRecordDto,
    /// `prior` / `replacement` / `partial` / `mixed` / `unknown` after the rollback.
    pub state: String,
    /// Why the range could not be put back, when it could not.
    pub restore_error: Option<String>,
}

/// The result of an apply. `outcome` is `committed`, `rolled-back` (a range failed and every
/// attempted range verified back to its prior bytes — the image is as it was) or `split` (a
/// range failed and at least one attempted range could not be verified back; `attempted` says
/// which holds what). The records of every attempted range are returned in every case.
#[derive(Debug, Serialize, TS)]
pub struct IsoApplyDto {
    pub outcome: String,
    /// Plain-language account.
    pub message: String,
    pub records_dir: String,
    pub records: Vec<IsoRecordDto>,
    pub attempted: Vec<IsoAttemptedDto>,
    pub unchanged: Vec<String>,
    /// The staged file whose write failed (rolled-back / split).
    pub failed: Option<String>,
    pub cause: Option<String>,
}

/// One file's restore outcome.
#[derive(Debug, Serialize, TS)]
pub struct IsoRestoreItemDto {
    pub entry_path: String,
    /// `restored` / `restored-partial` / `already-prior` / `refused: …`.
    pub outcome: String,
}

#[derive(Debug, Serialize, TS)]
pub struct IsoRestoreDto {
    pub records_dir: String,
    pub items: Vec<IsoRestoreItemDto>,
    pub all_prior: bool,
}

// ---------------------------------------------------------------------------------------------
// Phase 1.2 — competition groups and league membership
// ---------------------------------------------------------------------------------------------

/// One competition group as this title's table lists it, with its resolved label.
#[derive(Serialize, TS)]
pub struct CompetitionGroupDto {
    pub id: u64,
    /// Localization key the executable binds the group to; empty when unknown.
    pub key: String,
    /// `league` / `conference` / `division` / `pool` / `tournament` / `unused`.
    pub kind: String,
    pub label: String,
    /// `dictionary` (attached dictionary text) / `observed` (research text) / `key` / `unknown`
    /// (no key: the label names the id only).
    pub label_source: String,
    /// Whether any witnessed team row carries the id.
    pub used: bool,
    /// Every league code whose rows carry this group's block.
    pub league_codes: Vec<u64>,
    /// `traced-static` (read from the executable in id order) / `inferred` (from database
    /// usage only) / `unknown` (no key found).
    pub evidence: String,
    /// Why the row is inferred or unknown, in the research's words.
    pub note: Option<String>,
    /// Unknown rows only: the research's candidate key — a hint, never the name.
    pub hypothesis_key: Option<String>,
}

/// A league's competition structure on this title (NHL 12 → Legacy family; PS4 15 → 21).
#[derive(Serialize, TS)]
pub struct LeagueStructureDto {
    /// `traced` on the witnessed target, `by-reference` elsewhere.
    pub evidence: String,
    pub league_group: CompetitionGroupDto,
    pub conferences: Vec<CompetitionGroupDto>,
    pub divisions: Vec<CompetitionGroupDto>,
    /// Tournament pools the league's rows carry in both slots (PS4 Spengler Cup, Champions
    /// Hockey League, NHL 17 World Cup).
    pub pools: Vec<CompetitionGroupDto>,
    pub other: Vec<CompetitionGroupDto>,
    /// Keys shared with other leagues' groups (a dictionary rename of the key renames them all):
    /// `(key, labels of the other groups)`.
    pub shared_keys: Vec<(String, Vec<String>)>,
    /// Other league codes sharing this block (PS4 Custom Teams rows, Alumni / All-Time), with
    /// their display names.
    pub shared_with: Vec<(u64, String)>,
    /// The witness's provenance caveats (patch databases, anchor identity).
    pub witness_notes: Vec<String>,
    /// The witnessed regular (conference, division) arrangements of the block: a move must be
    /// one of them (R1 F1).
    pub pairs: Vec<(u64, u64)>,
    /// Exceptional stock arrangements (an All-Star squad's conference id in the division slot),
    /// described with their rows; never offered.
    pub exceptional: Vec<String>,
    /// How the open database was tied to this title: recorded identity, structure, or
    /// ambiguous (R1 F2).
    pub provenance: Option<String>,
}

#[derive(Serialize, TS)]
pub struct MembershipChoiceDto {
    pub value: u64,
    pub label: String,
    /// `traced-static` / `inferred` / `unknown` when the value is a group of the title's table.
    pub evidence: Option<String>,
}

#[derive(Serialize, TS)]
pub struct MembershipStateDto {
    pub league_code: u64,
    pub league_label: String,
    pub league_group: Option<MembershipChoiceDto>,
    pub conference: Option<MembershipChoiceDto>,
    pub division: Option<MembershipChoiceDto>,
}

#[derive(Serialize, TS)]
pub struct LeagueGroupsDto {
    pub league_code: u64,
    pub league_group: u64,
    /// The league group's evidence (`traced-static` / `inferred` / `unknown`).
    pub league_group_evidence: String,
    pub conferences: Vec<MembershipChoiceDto>,
    /// Every choosable division over all conferences; the card filters by `pairs`.
    pub divisions: Vec<MembershipChoiceDto>,
    /// The witnessed regular (conference, division) arrangements; a move must be one of them.
    pub pairs: Vec<(u64, u64)>,
    /// Exceptional stock arrangements of the block, described (never offered).
    pub exceptional: Vec<String>,
}

/// The membership choices for one team and how this title models them.
#[derive(Serialize, TS)]
pub struct MembershipOptionsDto {
    pub team_id: u64,
    /// `global-groups` / `local-bits` / `ps4-foreign-key` / `none`.
    pub model: String,
    pub group_evidence: Option<String>,
    pub current: MembershipStateDto,
    pub leagues: Vec<MembershipChoiceDto>,
    pub groups_by_league: Vec<LeagueGroupsDto>,
    pub notes: Vec<String>,
    pub blocker: Option<String>,
}

/// The result of a membership move: what was written, resolved before and after.
#[derive(Serialize, TS)]
pub struct MembershipResultDto {
    pub team_id: u64,
    pub table: String,
    pub record: u32,
    /// `(studio field id, value)` cells written in one transaction.
    pub writes: Vec<(String, i64)>,
    pub before: MembershipStateDto,
    pub after: MembershipStateDto,
    pub notes: Vec<String>,
    pub outcome: OpOutcomeDto,
}

// ---------------------------------------------------------------------------------------------
// Phase 1.2 — source schedules
// ---------------------------------------------------------------------------------------------

/// One schedule table in the database and the league it is believed to serve.
#[derive(Serialize, TS)]
pub struct ScheduleTableDto {
    pub studio_table: String,
    pub name: String,
    pub rows: u32,
    pub max_records: u32,
    pub league_code: Option<u64>,
    pub league_label: Option<String>,
    pub league_match: String,
    /// `source` / `future` / `debug` / `favorite` / `mode-local` / `variant`.
    pub role: String,
    /// Tournament templates (R115): the league whose page shows this table — display grouping only,
    /// never a slot binding (the table stays browse-only).
    pub shown_with_league: Option<u64>,
}

/// One scheduled game with its slots resolved to team names where the league is known.
#[derive(Serialize, TS)]
pub struct ScheduledGameDto {
    pub record: u32,
    pub index: u64,
    pub month: u64,
    pub day: u64,
    pub date: String,
    pub round: u64,
    pub status: Option<u64>,
    pub home_slot: u64,
    pub away_slot: u64,
    pub home: Option<String>,
    pub away: Option<String>,
    /// Tournament templates only (R115): read-only round / slot labels — never a team binding.
    pub round_label: Option<String>,
    pub home_label: Option<String>,
    pub away_label: Option<String>,
}

#[derive(Serialize, TS)]
pub struct ScheduleSlotDto {
    pub slot: u64,
    pub team: Option<String>,
    pub team_id: Option<u64>,
    pub games: u32,
}

/// One league member at its schedule slot (rank within the league by the team table's global
/// `index`), with the stable key the export files use.
#[derive(Serialize, TS)]
pub struct ScheduleSlotEntryDto {
    pub slot: u64,
    pub team_id: u64,
    pub name: String,
    pub abbrev: String,
    pub key: String,
    pub global_index: u64,
}

/// A schedule table's games, slot resolution and validation findings.
#[derive(Serialize, TS)]
pub struct ScheduleViewDto {
    pub table: ScheduleTableDto,
    /// Studio field ids by metadata name (`home`, `away`, `day`, `month`, `index`, `round`, `status`).
    pub fields: Vec<(String, String)>,
    /// The same fields' storage widths in bits (5-bit slots on gen7, 6-bit on PS4 18+, …).
    pub field_widths: Vec<(String, u32)>,
    /// Slot ↔ team map of the matched league (empty when no league is matched) — the picker
    /// source for the editor.
    pub slot_map: Vec<ScheduleSlotEntryDto>,
    /// Changes whenever the league's membership/ordering changes; previews carry it and apply
    /// refuses a stale one.
    pub fingerprint: String,
    pub games: Vec<ScheduledGameDto>,
    pub slots: Vec<ScheduleSlotDto>,
    pub unresolved_slots: Vec<ScheduleSlotDto>,
    pub findings: Vec<String>,
    /// Tournament templates (R115): what the slot labels rest on; the table stays browse-only.
    pub tournament_note: Option<String>,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// P1-A — schedule editing / import / export / generation
// ---------------------------------------------------------------------------------------------

/// Change the home team, away team and/or date of one game row. Dates are stored encoding
/// (0-indexed month/day); `null` leaves the value as it is.
#[derive(Deserialize, TS)]
pub struct ScheduleGameEditDto {
    pub record: u32,
    #[serde(default)]
    #[ts(optional)]
    pub home_team_id: Option<u64>,
    #[serde(default)]
    #[ts(optional)]
    pub away_team_id: Option<u64>,
    #[serde(default)]
    #[ts(optional)]
    pub month: Option<u64>,
    #[serde(default)]
    #[ts(optional)]
    pub day: Option<u64>,
}

/// Games per slot after a proposal, with the venue split.
#[derive(Serialize, TS)]
pub struct ScheduleSlotGamesDto {
    pub slot: u64,
    pub team: Option<String>,
    pub team_id: Option<u64>,
    pub games: u32,
    pub home: u32,
    pub away: u32,
}

/// Whole-proposal validation: `errors` refuse the apply; `warnings` are disclosed and do not.
#[derive(Serialize, TS)]
pub struct ScheduleValidationDto {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub games_per_slot: Vec<ScheduleSlotGamesDto>,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

/// A validated, apply-ready change: how many rows/cells move and what they mean. Held server-side
/// under `ticket`, bound to the project revision it was computed against; `schedule_apply_review`
/// applies exactly this plan or refuses.
#[derive(Serialize, TS)]
pub struct ScheduleChangePlanDto {
    /// The held review this plan belongs to: an opaque ticket (`<workspace namespace>/<sequence>`)
    /// that only the issuing project instance accepts. Carried as a string end to end.
    pub ticket: String,
    pub studio_table: String,
    pub table_name: String,
    pub league_label: Option<String>,
    pub rows_changed: u32,
    pub cells: u32,
    /// The changed rows before/after, resolved (first 200).
    pub before: Vec<ScheduledGameDto>,
    pub after: Vec<ScheduledGameDto>,
    pub validation: ScheduleValidationDto,
    pub fingerprint: String,
    pub notes: Vec<String>,
}

/// The result of applying a held review: what was previewed (`edit` / `import` / `generate`),
/// what was written, and the table's fresh browse view.
#[derive(Serialize, TS)]
pub struct ScheduleApplyResultDto {
    pub ticket: String,
    /// `edit` / `import` / `generate`.
    pub kind: String,
    pub plan: ScheduleChangePlanDto,
    pub import: Option<ScheduleImportPlanDto>,
    pub generation: Option<ScheduleGenerationPlanDto>,
    pub outcome: OpOutcomeDto,
    pub view: ScheduleViewDto,
}

/// A schedule export (`csv` / `json`) as text, with a suggested file name.
#[derive(Serialize, TS)]
pub struct ScheduleExportDto {
    pub format: String,
    pub text: String,
    pub suggested_filename: String,
}

/// One explicit destination assignment for an import: file key → team on this title.
#[derive(Deserialize, TS)]
pub struct ScheduleTeamMappingDto {
    pub key: String,
    pub team_id: u64,
}

/// How a file key was resolved: `explicit`, `auto`, `ambiguous` or `unmapped`.
#[derive(Serialize, TS)]
pub struct ScheduleKeyResolutionDto {
    pub key: String,
    pub team_id: Option<u64>,
    pub team: Option<String>,
    pub slot: Option<u64>,
    pub how: String,
    pub games: u32,
}

#[derive(Serialize, TS)]
pub struct ScheduleImportPlanDto {
    pub change: ScheduleChangePlanDto,
    pub keys: Vec<ScheduleKeyResolutionDto>,
    pub rows_in_file: u32,
    pub rows_in_table: u32,
}

/// One league member as the generator sees it.
#[derive(Serialize, TS)]
pub struct ScheduleParticipantDto {
    pub team_id: u64,
    pub slot: u64,
    pub name: String,
    pub abbrev: String,
    pub games: u32,
    pub default: bool,
}

/// The topology the table supports for generation, or the precise blocker.
#[derive(Serialize, TS)]
pub struct ScheduleGenerationOptionsDto {
    pub members: Vec<ScheduleParticipantDto>,
    pub kept_rows: u32,
    pub rows_to_fill: u32,
    pub pairs: u32,
    pub suggested_rounds: Option<u32>,
    pub suggested_start_month: u64,
    pub suggested_start_day: u64,
    pub suggested_days_between_matchdays: u32,
    pub matchdays_per_round: u32,
    pub blocker: Option<String>,
    pub notes: Vec<String>,
}

/// What the generator is asked to build (`start_month`/`start_day` stored encoding).
#[derive(Deserialize, TS, Clone)]
pub struct ScheduleGenerationConfigDto {
    pub participants: Vec<u64>,
    pub rounds: u32,
    pub start_month: u64,
    pub start_day: u64,
    pub days_between_matchdays: u32,
    /// Receipt 34: games per pair, square in the order of `participants` (symmetric, zero
    /// diagonal). Present = the user-supplied pair-quota generator (`rounds` ignored).
    #[serde(default)]
    #[ts(optional)]
    pub pair_quotas: Option<Vec<Vec<u32>>>,
    /// Host of the extra game of an odd pair count: `lower_slot`, `higher_slot`, `balance` or `per_pair`
    /// (research R123 Q11 — required when a count is odd). Any other value is refused (receipt 48).
    #[serde(default)]
    #[ts(optional)]
    pub odd_home: Option<String>,
    /// Receipt 48: single-phase groups (R123 partition) instead of `pair_quotas`.
    #[serde(default)]
    #[ts(optional)]
    pub groups: Option<ScheduleGroupsDto>,
    /// `per_pair`: the host of every odd pair.
    #[serde(default)]
    #[ts(optional)]
    pub odd_hosts: Option<Vec<ScheduleOddHostDto>>,
}

/// Receipt 48 — groups of participants (team ids) with games per pair inside a group and across groups.
#[derive(Deserialize, Serialize, TS, Clone)]
pub struct ScheduleGroupsDto {
    pub groups: Vec<Vec<u64>>,
    pub within: u32,
    pub across: u32,
    /// Shown in the plan's notes (e.g. the shipped table's recorded source).
    #[serde(default)]
    #[ts(optional)]
    pub origin: Option<String>,
}

#[derive(Deserialize, Serialize, TS, Clone)]
pub struct ScheduleOddHostDto {
    pub home: u64,
    pub away: u64,
}

/// Receipt 48 — the shipped groups of this table, offered only when the opened file, the table and the league's
/// membership equal a recorded R113 source exactly; otherwise `reason` says why (enter groups instead).
#[derive(Serialize, TS)]
pub struct ScheduleStockGroupsDto {
    pub available: bool,
    #[ts(optional)]
    pub groups: Option<ScheduleGroupsDto>,
    /// Club names per group, for display.
    pub group_names: Vec<Vec<String>>,
    #[ts(optional)]
    pub source: Option<String>,
    #[ts(optional)]
    pub games: Option<u64>,
    #[ts(optional)]
    pub reason: Option<String>,
}

#[derive(Serialize, TS)]
pub struct ScheduleGenerationPlanDto {
    pub change: ScheduleChangePlanDto,
    pub participants: u32,
    pub rounds: u32,
    pub matchdays: u32,
    pub generated_rows: u32,
    pub kept_rows: u32,
    pub games_per_participant: u32,
}

// ---------------------------------------------------------------------------------------------
// Phase 1.4 — league logos
// ---------------------------------------------------------------------------------------------

#[derive(Serialize, TS)]
pub struct LeagueLogoVariantDto {
    pub family: String,
    /// Canonical archive member path. Use loose_rel_path for extracted-file operations.
    pub rel_path: String,
    /// Actual extracted file relative to the selected root; may include databigs/Extracted.
    #[ts(optional)]
    pub loose_rel_path: Option<String>,
    /// Whether the file exists loose under the given root; `None` when no root was given.
    pub exists: Option<bool>,
    /// The packed archive (relative to the install root) the member ships in on PS4 — the
    /// archive-member replace route's target; `None` for loose gen7 packages.
    pub archive: Option<String>,
}

/// A league's logo binding on this title: the family-wide art index, how it is established, the
/// other leagues sharing it, and the package files per family.
#[derive(Serialize, TS)]
pub struct LeagueLogoDto {
    pub code: u64,
    pub art_index: u64,
    /// `native` / `traced` / `by-reference` / `inferred`.
    pub evidence: String,
    pub witness: String,
    pub shared_with: Vec<(u64, String)>,
    pub variants: Vec<LeagueLogoVariantDto>,
    pub notes: Vec<String>,
}

/// Phase 2.4 (C9): a team's logo families (art-id keyed) on the open title.
#[derive(Serialize, TS)]
pub struct TeamLogoSetDto {
    pub artid: u64,
    /// `measured` — the family table was measured on this title's shipped archive / tree.
    pub evidence: String,
    pub variants: Vec<LeagueLogoVariantDto>,
    pub notes: Vec<String>,
}

/// P1-G: one member of a league-imagery family — the loose path the texture surface addresses.
#[derive(Serialize, TS)]
pub struct LeagueImageryMemberDto {
    pub id: u64,
    pub rel_path: String,
    pub exists: Option<bool>,
    /// The French twin (PS4 17–21) when the registry declares one.
    pub fr_rel_path: Option<String>,
    pub fr_exists: Option<bool>,
    /// The packed archive the member ships in (PS4), for the archive-member replace route.
    pub archive: Option<String>,
}

/// P1-G: one league-keyed imagery family (gen7 folder or PS4 registry type) with the handoff's
/// reading of its members and the observed consumer screens.
#[derive(Serialize, TS)]
pub struct LeagueImageryFamilyDto {
    pub family: String,
    pub label: String,
    pub reading: String,
    pub consumers: Vec<String>,
    /// `id:WxH:format` facts where the survey decoded the member.
    pub texture_facts: Vec<String>,
    pub members: Vec<LeagueImageryMemberDto>,
}

/// Phase 2.1 (21 Sep 2026): one consumer of a team identity (rows of `table.field`).
#[derive(Serialize, TS, Debug, Clone, PartialEq, Eq)]
pub struct DependencyCountDto {
    pub table: String,
    pub field: String,
    pub rows: usize,
}

/// A team's two identities (index, art id), who depends on them, and the contract's disposition
/// of changing each (research 2026-09-21 handoff 10).
#[derive(Serialize, TS)]
pub struct TeamIdentityDependenciesDto {
    pub team_id: u64,
    pub team_name: String,
    pub artid: Option<u64>,
    pub index_dependencies: Vec<DependencyCountDto>,
    pub artid_dependencies: Vec<DependencyCountDto>,
    pub teams_sharing_artid: Vec<String>,
    pub index_change: String,
    pub artid_change: String,
}

/// The art-id swap between two teams: the plan (tables × rows) and the applied outcome.
#[derive(Serialize, TS)]
pub struct ArtIdSwapDto {
    pub team_a: u64,
    pub team_b: u64,
    pub team_a_name: String,
    pub team_b_name: String,
    pub artid_a: u64,
    pub artid_b: u64,
    /// `(table name, rows written)` in order.
    pub tables: Vec<(String, usize)>,
    pub total_cells: usize,
    pub summary: String,
    pub outcome: OpOutcomeDto,
}

/// Phase 1.5 / 2.7 / 3.5 (21 Sep 2026): one input file of the read-only gen7 commentary catalogue.
#[derive(Serialize, TS)]
pub struct SpeechSourceFileDto {
    pub role: String,
    pub path: String,
    pub len: u64,
    pub sha256: String,
}

/// A speech event: id, name, parameter names, other names the id carries, and how many sentences
/// carry exactly this name (0 = "no sentence carries this event name").
#[derive(Serialize, TS)]
pub struct SpeechEventDto {
    pub id: u16,
    pub name: String,
    pub parameters: Vec<String>,
    pub other_names_for_id: Vec<String>,
    pub sentences: usize,
}

/// One phrase of a sentence body: its sample group (fixed tag / value, name where the file has
/// one), the event parameters that select among the group's candidates, the candidate sample ids
/// and how many of them the bank resolves.
#[derive(Serialize, TS)]
pub struct SpeechPhraseDto {
    pub pool_id: u32,
    pub group_tag: u32,
    pub group_value: u32,
    pub group_name: Option<String>,
    pub parameters: Vec<String>,
    pub table_found: bool,
    /// How many candidate samples the group's table lists.
    pub candidate_count: usize,
    /// The first candidates (at most 64 — the page shows a sample of the list; the count is exact).
    pub candidate_sample_ids: Vec<u32>,
    pub bank_resolved: usize,
    pub unresolved_sample_ids: Vec<u32>,
}

#[derive(Serialize, TS)]
pub struct SpeechBodyDto {
    pub id: u32,
    pub phrases: Vec<SpeechPhraseDto>,
}

/// A sentence sharing an event's name (never "the line the event speaks").
#[derive(Serialize, TS)]
pub struct SpeechSentenceDto {
    pub name: String,
    pub event_id: Option<u16>,
    pub slot: u32,
    pub slot_ok: bool,
    pub bodies: Vec<SpeechBodyDto>,
}

#[derive(Serialize, TS)]
pub struct SpeechStatsDto {
    pub events: usize,
    pub events_with_a_sentence: usize,
    pub events_without_a_sentence: usize,
    pub parameters: usize,
    pub sentences: usize,
    pub sentences_slot_ok: usize,
    pub bodies: usize,
    pub phrases: usize,
    pub phrases_with_table: usize,
    pub graffiti_tables: usize,
    pub bank_samples: usize,
    pub keywords: usize,
    pub ambiguous_event_ids: usize,
}

/// The bounded catalogue view a page shows: the title's speech sources, the totals, and the
/// events / sentences matching the query (capped; `truncated` says when).
#[derive(Serialize, TS)]
pub struct SpeechCatalogueDto {
    pub game_id: String,
    pub platform: String,
    pub speech_dir: String,
    pub sources: Vec<SpeechSourceFileDto>,
    pub stats: SpeechStatsDto,
    pub notes: Vec<String>,
    pub query: String,
    pub league_only: bool,
    pub events_matching: usize,
    pub sentences_matching: usize,
    pub events: Vec<SpeechEventDto>,
    pub sentences: Vec<SpeechSentenceDto>,
    pub truncated: bool,
}

/// Phase 2.7 (21 Sep 2026): the read-only name-call listing of one team — the play-by-play samples
/// tagged with the team's `audioid` under the `team_ID` parameter (research 2026-09-21 handoff 07).
#[derive(Serialize, TS)]
pub struct TeamNameCallsDto {
    pub team_id: u64,
    pub team_name: String,
    pub team_audio_id: Option<u64>,
    pub team_tag: Option<u32>,
    pub tables: Vec<NameCallTableDto>,
    pub sample_ids_distinct: usize,
    pub bank_resolved_distinct: usize,
    pub keyword_mentions: Option<usize>,
    pub notes: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct NameCallTableDto {
    pub num_tags: u32,
    /// Capped at 64 ids; `sample_count` is the full count.
    pub sample_ids: Vec<u32>,
    pub sample_count: usize,
    pub bank_resolved: usize,
}

// ---------------------------------------------------------------- PS4 speech banks (P4, 24 Sep 2026)

/// One SND10 `.sbr` bank of the PS4 speech directory (identity: length + sha256) and its `.sbs`.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Ps4SpeechBankDto {
    pub file: String,
    #[ts(type = "number")]
    pub sbr_len: u64,
    pub sbr_sha256: String,
    pub sbs_present: bool,
    #[ts(type = "number | null")]
    pub sbs_len: Option<u64>,
    pub dsets: u32,
    pub samples: u32,
    pub note: Option<String>,
}

/// The banks of a PS4 NHL 15–21 title's speech directory. Listing only.
#[derive(Debug, Serialize, TS)]
pub struct Ps4SpeechBanksDto {
    pub game_id: String,
    pub speech_dir: String,
    pub banks: Vec<Ps4SpeechBankDto>,
    pub notes: Vec<String>,
}

/// One sample record of a bank with the SNR header read from its block stream.
#[derive(Debug, Serialize, TS)]
pub struct Ps4SpeechSampleDto {
    #[ts(type = "number")]
    pub sid: u64,
    #[ts(type = "number")]
    pub offset: u64,
    pub duration_ms: Option<u32>,
    pub codec: Option<String>,
    pub codec_code: Option<u8>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub num_samples: Option<u32>,
    pub problem: Option<String>,
    /// `true` only for EA-MP3 (0x0b, 48 kHz mono) — the one codec with a diagnostic preview route.
    pub previewable: bool,
}

#[derive(Debug, Serialize, TS)]
pub struct Ps4SpeechCodecCountDto {
    pub codec: String,
    pub count: u32,
}

/// A bank's samples (filtered by sample id text, capped) with the measured codec census.
#[derive(Debug, Serialize, TS)]
pub struct Ps4SpeechSamplesDto {
    pub bank: Ps4SpeechBankDto,
    pub total: u32,
    pub matching: u32,
    pub truncated: bool,
    pub codec_census: Vec<Ps4SpeechCodecCountDto>,
    pub rows: Vec<Ps4SpeechSampleDto>,
    pub notes: Vec<String>,
}

/// A **diagnostic** EA-MP3 preview of one sample: the grammar / decoder diagnostics, the rendering
/// used for the first 24 ms (`"mpeg_only"` or `"candidate_a"`), the label the UI must show, and the
/// WAV as a `data:audio/wav;base64,…` URL. Never game-verified; never a replacement route.
#[derive(Debug, Serialize, TS)]
pub struct Ps4SpeechPreviewDto {
    pub bank: Ps4SpeechBankDto,
    #[ts(type = "number")]
    pub sid: u64,
    #[ts(type = "number")]
    pub offset: u64,
    #[ts(type = "number")]
    pub sns_len: u64,
    pub sns_sha256: String,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub num_samples: u32,
    pub blocks: u32,
    pub frames: u32,
    pub frames_per_block: Vec<u32>,
    pub trim: u32,
    pub embedded_pcm_samples: u32,
    pub first_sample_over_1_lsb: Option<u32>,
    pub rendering: String,
    pub label: String,
    pub wav_data_url: String,
    pub notes: Vec<String>,
}

/// P4.3 (24 Sep 2026) — the **diagnostic** gen7 EALayer3 v1 preview of one speech sample
/// (research R77 / R79 / R80): grammar + rebuild + decode diagnostics, the fixed label, the source
/// identities and the WAV as a `data:audio/wav;base64,…` URL. Never game-verified; no replacement.
#[derive(Debug, Serialize, TS)]
pub struct Gen7SpeechPreviewDto {
    pub game_id: String,
    pub platform: String,
    pub bank_stem: String,
    pub bank_path: String,
    #[ts(type = "number")]
    pub bank_len: u64,
    pub bank_sha256: String,
    pub stream_path: String,
    #[ts(type = "number")]
    pub stream_len: u64,
    pub sample_id: u32,
    #[ts(type = "number")]
    pub offset: u64,
    #[ts(type = "number")]
    pub sns_len: u64,
    pub sns_sha256: String,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub num_samples: u32,
    pub blocks: u32,
    pub granules: u32,
    pub granules_per_block: Vec<u32>,
    pub rebuilt_frames: u32,
    pub max_main_data_begin: u32,
    pub pcm_frame_samples: u32,
    #[ts(type = "number")]
    pub surplus_samples: i64,
    pub decoded_samples_per_channel: u32,
    pub first_sample_over_1_lsb: Option<u32>,
    pub label: String,
    /// The EA-Speex route's diagnostic line (receipt 29); `None` on the EALayer3 route, whose granule
    /// fields carry the diagnostics.
    pub route_detail: Option<String>,
    pub tuple_source: String,
    pub wav_data_url: String,
    pub notes: Vec<String>,
}

/// P4.6 (24 Sep 2026) — one sample of a gen7 bank listing with the SNR header read at its stream offset.
#[derive(Debug, Serialize, TS)]
pub struct Gen7SampleRowDto {
    pub sample_id: u32,
    #[ts(type = "number")]
    pub offset: u64,
    pub codec: Option<String>,
    pub codec_code: Option<u8>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub num_samples: Option<u32>,
    pub problem: Option<String>,
    pub previewable: bool,
}

/// P4.6 — a measured gen7 bank (`pbp_eng` / `ann_eng` / `music_samples`) listed from its SBKR table:
/// source identities, the codec census measured from the headers, and the capped rows. Listing only.
#[derive(Debug, Serialize, TS)]
pub struct Gen7BankListingDto {
    pub game_id: String,
    pub platform: String,
    pub bank_stem: String,
    pub tuple_source: String,
    pub bank: String,
    #[ts(type = "number")]
    pub bank_len: u64,
    pub bank_sha256: String,
    pub stream: String,
    pub total: u32,
    pub matching: u32,
    pub truncated: bool,
    pub codec_census: Vec<Ps4SpeechCodecCountDto>,
    pub rows: Vec<Gen7SampleRowDto>,
    /// Why no row can be previewed now although the tuple is measured (EA-XMA without an installed
    /// XMA2 helper); `null` when previews are available.
    pub preview_unavailable: Option<String>,
    pub notes: Vec<String>,
}

/// Phase 1.5 / 2.7 (21 Sep 2026): the read-only context listing (research 2026-09-21 handoff 01):
/// trigger → conditions → items per context, filtered and capped.
#[derive(Serialize, TS)]
pub struct ContextCatalogueDto {
    pub game_id: String,
    pub platform: String,
    pub context_dir: String,
    pub records: usize,
    pub triggers: usize,
    pub condition_names: usize,
    pub speech_items: usize,
    pub context_items: usize,
    pub query: String,
    pub matching: usize,
    pub truncated: bool,
    pub contexts: Vec<ContextRecordDto>,
    pub notes: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct ContextRecordDto {
    pub index: u32,
    pub name: String,
    pub trigger: Option<String>,
    pub conditions: Vec<String>,
    pub items: Vec<ContextItemDto>,
}

#[derive(Serialize, TS)]
pub struct ContextItemDto {
    pub kind: String,
    pub name: String,
    pub event_id: u16,
    pub params: Vec<String>,
}

/// Phase 3.5 (21 Sep 2026): one player's recorded names, read-only — the play-by-play
/// `playernames` rows matching the name text with their `player_ID` tables, and the announcer's
/// `playerid` tables of the bio audio id (research 2026-09-21 handoff 07).
#[derive(Serialize, TS)]
pub struct PlayerNameCallsDto {
    pub player_id: u64,
    pub first_name: String,
    pub last_name: String,
    pub pbp_names: Vec<PbpNameEntryDto>,
    pub pbp_player_tag: Option<u32>,
    pub announcer_audio_id: Option<u64>,
    pub announcer_tag: Option<u32>,
    pub announcer_tables: Vec<NameCallTableDto>,
    pub announcer_sample_ids_distinct: usize,
    pub announcer_bank_resolved_distinct: usize,
    pub notes: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct PbpNameEntryDto {
    pub name: String,
    pub is_first_name: bool,
    pub bank: u64,
    pub audio_id: u64,
    pub tables: Vec<NameCallTableDto>,
    pub sample_ids_distinct: usize,
    pub bank_resolved_distinct: usize,
}

/// P1-G: the league-keyed presentation imagery of the open title beyond the four logo families.
#[derive(Serialize, TS)]
pub struct LeagueImageryDto {
    pub game_id: String,
    pub platform: String,
    /// Where the family ↔ path binding was read.
    pub registry: String,
    pub families: Vec<LeagueImageryFamilyDto>,
    pub notes: Vec<String>,
}

/// LOC-W2: the preview of an in-place free-text re-encode — the new text encoded with the
/// dictionary's own compression tree into the bytes the old string occupies.
#[derive(Serialize, TS)]
pub struct DictionaryRetextPreviewDto {
    pub dictionary_path: String,
    pub dictionary_sha256: String,
    pub endian_big: bool,
    pub source: DictionaryRowDto,
    pub source_lookup: String,
    pub text_before: String,
    pub text_after: String,
    pub pool_offset: u32,
    pub old_bits: usize,
    pub old_extent_bytes: usize,
    pub new_bits: usize,
    pub new_extent_bytes: usize,
    /// Rows pointing at the same string: they read the new text too.
    pub sharing_text: Vec<DictionaryRowDto>,
    pub hash_collisions: Vec<DictionaryRowDto>,
    pub warnings: Vec<String>,
}

/// LOC-W3: one growth operation as the UI sends it. `kind` = `text` (existing key, appended
/// text; `all_sharers` decides whether rows sharing the old string follow), `insert` (new key with
/// `text`, or a null text when `text` is null) or `null` (existing key's text set to null).
#[derive(Deserialize, Serialize, TS, Clone)]
pub struct DictionaryGrowthOpDto {
    pub kind: String,
    pub key: String,
    pub text: Option<String>,
    #[serde(default)]
    pub all_sharers: bool,
}

#[derive(Serialize, TS)]
pub struct DictionaryGrowthInsertDto {
    pub key: String,
    pub hash: u32,
    pub text: Option<String>,
    pub position: usize,
    pub output_record: usize,
}

#[derive(Serialize, TS)]
pub struct DictionaryGrowthEditDto {
    pub row: DictionaryRowDto,
    pub lookup: String,
    pub text_before: Option<String>,
    pub text_after: Option<String>,
    pub sharers: Vec<DictionaryRowDto>,
    pub all_sharers: bool,
    pub records: Vec<usize>,
}

/// The preview of a dictionary growth (LOC-W3): rows inserted at their sorted positions and texts
/// appended to the pool; the file gets longer.
#[derive(Serialize, TS)]
pub struct DictionaryGrowthPreviewDto {
    pub dictionary_path: String,
    pub dictionary_sha256: String,
    pub endian_big: bool,
    /// `A` (insertion offered) / `B` (out-of-order rows: insertion refused, text growth offered).
    pub order_class: String,
    pub descents: Vec<usize>,
    pub studio_touched: bool,
    pub rows_before: usize,
    pub rows_after: usize,
    pub appended_bytes: usize,
    pub inserts: Vec<DictionaryGrowthInsertDto>,
    pub edits: Vec<DictionaryGrowthEditDto>,
    pub warnings: Vec<String>,
    /// `preserving` (appended strings; nothing existing moves) or `canonical-rebuild` (the pool is
    /// repacked from the decoded rows; the compression tree is rebuilt when a text needs a symbol
    /// it lacks, or when forced). The rebuild is only ever used when the caller asks for it.
    pub serializer: String,
    /// Canonical rebuild: the symbols the current tree lacks (why a rebuild is needed).
    pub new_symbols: Vec<String>,
    /// Canonical rebuild, after the hold: the accounting lines (tree kept/rebuilt, pool sizes,
    /// discarded ranges, moved strings, re-pointed records).
    pub rebuild_accounting: Vec<String>,
}

/// How a growth is serialized (`serializer` on the growth commands): `preserving` (default),
/// `rebuild` (canonical rebuild; tree rebuilt only when needed) or `rebuild-tree` (canonical
/// rebuild with the tree always rebuilt).
#[derive(Deserialize, Serialize, TS, Clone, Default)]
pub struct DictionaryGrowthOptionsDto {
    #[serde(default)]
    pub serializer: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// Phase 2.3 — team text in the dictionary
// ---------------------------------------------------------------------------------------------

#[derive(Serialize, TS)]
pub struct TeamTextKeyDto {
    pub family: String,
    pub note: String,
    pub key: String,
    pub row_key: Option<String>,
    /// `exact` / `by-hash` / `absent`.
    pub lookup: String,
    pub text: Option<String>,
}

/// The per-team dictionary key families (keyed by the team row's `artabbr`) and the database's
/// own name strings, side by side.
#[derive(Serialize, TS)]
pub struct TeamTextDto {
    pub team_id: u64,
    pub artabbr: Option<String>,
    pub db_fullname: Option<String>,
    pub db_shortname: Option<String>,
    pub db_abbrname: Option<String>,
    pub keys: Vec<TeamTextKeyDto>,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------------------------------
// Phase 1.2 (D2) — the read-only settings catalogue
// ---------------------------------------------------------------------------------------------

/// One choice of a setting: the stored code and the dictionary key it maps to (resolved text
/// when a dictionary is attached).
#[derive(Serialize, TS)]
pub struct SettingChoiceDto {
    pub code: i64,
    pub key: String,
    pub text: Option<String>,
}

/// A stored default: `code` is `None` for an empty (null) cell; `cells` names every stored copy
/// (record/field or member line/column); `paired` + `consistent` describe the NHL 18–21 global pair.
#[derive(Serialize, TS)]
pub struct StoredDefaultDto {
    pub code: Option<i64>,
    pub label: Option<String>,
    pub cells: Vec<String>,
    pub values: Vec<Option<i64>>,
    pub paired: bool,
    pub consistent: Option<bool>,
}

#[derive(Serialize, TS)]
pub struct SettingContextDefaultDto {
    pub context: i64,
    pub context_name: Option<String>,
    pub stored: StoredDefaultDto,
    /// The row's own choice list when it differs from the setting's.
    pub choices: Option<Vec<SettingChoiceDto>>,
    pub domain_note: Option<String>,
    pub enabled: Option<bool>,
    pub visible: Option<bool>,
    pub anomaly: Option<String>,
}

/// One catalogue row (identity = the legacy id).
#[derive(Serialize, TS)]
pub struct SettingRowDto {
    pub legacy_id: i64,
    pub name: String,
    pub label_key: Option<String>,
    pub label_text: Option<String>,
    pub setting_id: Option<i64>,
    pub siblings: Vec<i64>,
    pub is_choice: Option<bool>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub size: Option<i64>,
    pub global: Option<StoredDefaultDto>,
    pub context_defaults: Vec<SettingContextDefaultDto>,
    pub choices: Vec<SettingChoiceDto>,
    /// `explicit` / `lineage` / `inferred` / `none`.
    pub label_source: String,
    pub groups: Vec<i64>,
    pub dependents: Vec<i64>,
    /// `eligible` / `excluded` / `out-of-scope` (the stored-default contract).
    pub eligibility: String,
    pub role: Option<String>,
    pub eligibility_reason: Option<String>,
    pub anomalies: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct SettingContextDto {
    pub id: i64,
    pub name: Option<String>,
    pub name_source: String,
    pub defaults_rows: u32,
    pub layout_rows: u32,
    pub reserved: bool,
}

#[derive(Serialize, TS)]
pub struct SettingGroupDto {
    pub id: i64,
    pub name: Option<String>,
    pub name_source: String,
    pub parent: Option<i64>,
    pub members: u32,
}

#[derive(Serialize, TS)]
pub struct SettingsMemberDto {
    pub path: String,
    pub plain_len: u64,
    pub plain_sha256: String,
    pub stored_form: String,
}

#[derive(Serialize, TS)]
pub struct SettingsTableDto {
    pub name: String,
    pub raw_id: String,
    pub rows: u32,
    pub max_records: u32,
}

/// Where the catalogue came from: `database` (NHL 15) or `setting-csv` (NHL 16–21).
#[derive(Serialize, TS)]
pub struct SettingsSourceDto {
    pub kind: String,
    /// Database: `base 01.00` / `patch 01.06` / `unrecorded …`.
    pub copy: Option<String>,
    pub database_sha256: Option<String>,
    pub tables: Vec<SettingsTableDto>,
    /// CSV: the archive/directory, the schema variant and the members read.
    pub origin: Option<String>,
    pub variant: Option<String>,
    pub members: Vec<SettingsMemberDto>,
}

#[derive(Serialize, TS)]
pub struct SettingsCatalogueDto {
    pub game: String,
    pub platform: String,
    pub source: SettingsSourceDto,
    pub contexts: Vec<SettingContextDto>,
    pub groups: Vec<SettingGroupDto>,
    pub settings: Vec<SettingRowDto>,
    pub absent_eligible: Vec<i64>,
    pub anomalies: Vec<String>,
    pub notes: Vec<String>,
    pub dictionary: Option<String>,
}

/// The catalogue, or the reason none can be shown for this project.
#[derive(Serialize, TS)]
pub struct SettingsCatalogueResultDto {
    pub catalogue: Option<SettingsCatalogueDto>,
    pub blocker_kind: Option<String>,
    pub blocker: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// Phase 1.2 (D3) — one-cell stored-default editing (NHL 15 PS4)
// ---------------------------------------------------------------------------------------------

/// One member rewrite of an NHL 16–21 CSV stored-default edit (D4): the cell(s) spliced, the
/// plain/stored identities before and after, and the placement the archive directory predicts.
#[derive(Serialize, TS)]
pub struct SettingsCsvMemberDto {
    pub member: String,
    pub member_path: String,
    pub stored_form: String,
    pub source_plain_sha256: String,
    pub source_stored_len: Option<u64>,
    pub new_plain_sha256: String,
    pub new_stored_len: u64,
    pub new_stored_sha256: String,
    /// `[line, column, before, after]` per spliced cell.
    pub cells: Vec<(u32, String, String, String)>,
    pub placement_prediction: Option<String>,
}

/// The held plan of one stored-default edit, exactly as previewed; Apply consumes `ticket`.
/// `kind` is `database` (NHL 15, one cell of the open database) or `csv` (NHL 16–21, one or two
/// cells of the archive's CSV members, pending until the next copied-target save).
#[derive(Serialize, TS)]
pub struct SettingsEditPlanDto {
    pub kind: String,
    pub ticket: String,
    pub legacy_id: i64,
    /// `None` = the global default.
    pub context: Option<i64>,
    pub setting_name: String,
    pub label_text: Option<String>,
    pub role: String,
    pub context_name: Option<String>,
    pub table_name: String,
    pub record: u32,
    pub before: i64,
    pub before_label: Option<String>,
    pub after: i64,
    pub after_label: Option<String>,
    pub max: i64,
    pub label_source: String,
    pub siblings: Vec<i64>,
    /// `[what, code]` pairs of the setting's other stored defaults (unchanged).
    pub unchanged: Vec<(String, i64)>,
    pub copy: String,
    pub no_op: bool,
    pub stock_anomalies: Vec<String>,
    pub description: String,
    pub notes: Vec<String>,
    /// CSV: the paired global edit (two cells in two members).
    pub paired: bool,
    /// CSV: the member rewrites; empty for a database edit.
    pub members: Vec<SettingsCsvMemberDto>,
    /// CSV: the archive the members were read from.
    pub archive: Option<String>,
}

/// A preview: the held plan, or the refusal (nothing held, nothing written).
#[derive(Serialize, TS)]
pub struct SettingsEditPreviewDto {
    pub plan: Option<SettingsEditPlanDto>,
    pub refusal_kind: Option<String>,
    pub refusal: Option<String>,
}

#[derive(Serialize, TS)]
pub struct SettingsEditApplyDto {
    pub plan: SettingsEditPlanDto,
    pub outcome: OpOutcomeDto,
    pub summary: ProjectSummaryDto,
}

// ---------------------------------------------------------------------------
// Phase 2.6 (22 Sep 2026): a team's kits joined to their textures — read-only evidence
// ---------------------------------------------------------------------------

/// One required component of a kit (jersey / pants / one sock file) and what Studio found for it.
#[derive(Serialize, TS)]
pub struct KitComponentDto {
    /// `jersey` / `pant` / `sock`.
    pub part: String,
    /// Human label: `jersey`, `pants`, `socks`, `socks (file 0)`, `socks (file 1)`.
    pub label: String,
    /// The stem the title's contract expects (no extension).
    pub file_stem: String,
    /// Absolute path when present.
    pub path: Option<String>,
    /// Install-relative path when present and under the install dir — what `texture_preview` takes.
    pub rel_path: Option<String>,
    /// `<archive>:<member path>` when the component is a render-archive member (PS3 NHL 12 / 13):
    /// present by the archive's file table. Preview through `archive_member_texture_preview` with
    /// the two fields below.
    pub archive_member: Option<String>,
    /// The render archive's install-relative path (`cacherender.big` beside `rendering/`) and the
    /// member path inside it, when archived.
    pub archive_rel_path: Option<String>,
    pub member_path: Option<String>,
    /// `present` / `missing` / `unknown`.
    pub status: String,
    /// Why it is missing or unknown; `None` when present.
    pub reason: Option<String>,
}

/// One `stockteamjerseys` row with its flags and per-component evidence.
#[derive(Serialize, TS)]
pub struct KitDto {
    pub record: u32,
    pub jersey_index: u32,
    #[ts(type = "number")]
    pub style: u64,
    #[ts(type = "number")]
    pub team_art_id: u64,
    #[ts(type = "number")]
    pub variant: u64,
    pub is_default: bool,
    pub is_light_color: bool,
    pub has_tie: bool,
    pub helmet_logo: bool,
    pub helmet_numbers: bool,
    /// Empty when the row is refused or the title's contract is not measured.
    pub components: Vec<KitComponentDto>,
    /// The nameplate sheet the row's triple names (R101 / R103; measured titles only) —
    /// informational, not part of `completeness`.
    pub nameplate: Option<KitComponentDto>,
    /// `complete` / `incomplete` / `unknown` — the same rule the spare candidates use.
    pub completeness: String,
    pub refusal: Option<String>,
}

/// A `(style, variant)` some family names under the team's art id but no row points at.
#[derive(Serialize, TS)]
pub struct SpareKitDto {
    #[ts(type = "number")]
    pub style: u64,
    #[ts(type = "number")]
    pub variant: u64,
    pub components: Vec<KitComponentDto>,
    pub completeness: String,
}

/// How one component directory of the rendering tree read.
#[derive(Debug, Serialize, TS)]
pub struct KitDirectoryDto {
    /// `jersey` / `pant` / `sock`.
    pub part: String,
    /// `listed` / `absent` / `unreadable` / `archived`.
    pub status: String,
    /// Kit-container files seen, when listed (loose or archived).
    pub files: Option<usize>,
    pub reason: Option<String>,
    /// The render archive(s) the members were listed from, when `archived`.
    pub archive: Option<String>,
}

/// A team's kits, the shipped spare candidates, and every piece of evidence behind the verdicts.
#[derive(Serialize, TS)]
pub struct TeamKitsDto {
    #[ts(type = "number")]
    pub team_id: u64,
    #[ts(type = "number | null")]
    pub team_art_id: Option<u64>,
    /// `measured` / `no_selector_table` / `unmeasured`.
    pub contract_status: String,
    /// `single_file` / `two_file` when measured.
    pub sock_grammar: Option<String>,
    /// The contract in words (the templates and their measurement, or why nothing is measured).
    pub contract: String,
    pub rendering_root: Option<String>,
    pub directories: Vec<KitDirectoryDto>,
    pub kits: Vec<KitDto>,
    pub spares: Vec<SpareKitDto>,
    /// Always `false` today: re-pointing a row has no write contract.
    pub reassignment_supported: bool,
    pub reassignment_reason: String,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Phase 2.5 (22 Sep 2026): an arena's shipped assets — read-only evidence
// ---------------------------------------------------------------------------

/// One file an arena asset family resolved (or looked for).
#[derive(Serialize, TS)]
pub struct AssetFileDto {
    #[ts(type = "number")]
    pub key: u64,
    /// Where the key came from, in words (`arenaartid`, `home team … (art id N)`, …).
    pub key_source: String,
    pub file_stem: String,
    pub path: Option<String>,
    /// Install-relative path when present and under the install dir — what `texture_preview` takes.
    pub rel_path: Option<String>,
    /// `present` / `missing` / `unknown`.
    pub status: String,
    pub reason: Option<String>,
}

/// One asset family on the arena card.
#[derive(Serialize, TS)]
pub struct AssetFamilyDto {
    pub family: String,
    pub label: String,
    pub dir: String,
    /// `established` / `via_home_team` / `candidate` / `no_established_binding`.
    pub binding: String,
    pub rule: String,
    pub files: Vec<AssetFileDto>,
    pub shipped_files: Option<usize>,
    /// Keys the tree ships that no row names (established families only).
    #[ts(type = "number[]")]
    pub spare_keys: Vec<u64>,
    pub notes: Vec<String>,
}

/// An arena's shipped assets, joined where the research measured a join and listed elsewhere.
#[derive(Serialize, TS)]
pub struct ArenaAssetsDto {
    #[ts(type = "number")]
    pub arena_id: u64,
    #[ts(type = "number | null")]
    pub arena_art_id: Option<u64>,
    /// `gen7` / `ps4_vault_reference` / `unmeasured`.
    pub contract_status: String,
    pub contract: String,
    pub rendering_root: Option<String>,
    pub families: Vec<AssetFamilyDto>,
    /// PS4 only: `(directory, members)` under `stadiums/`.
    pub venue_directories: Vec<(String, usize)>,
    /// PS4 only: the vault-level listing for this arena's `arenaartid` (research R73) — every
    /// candidate level, classed, never chosen; `null` on gen7.
    pub ps4_venue: Option<Ps4VenueListingDto>,
    pub notes: Vec<String>,
}

/// A texpack key of a level, resolved into the title's measured set.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Ps4TexpackRefDto {
    /// `center_logo` / `center_line` / `ice_line`.
    pub family: String,
    #[ts(type = "number")]
    pub key: i64,
    pub in_set: bool,
}

/// One vault level record carrying the arena's `arenaartid`.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Ps4VenueCandidateDto {
    pub level_name: String,
    /// `stadiums/<dir>` relative to the rendering root, when the research names one.
    pub dir: Option<String>,
    /// `present` / `missing` / `no_folder_shipped` / `unknown`.
    pub folder: String,
    pub members: Option<usize>,
    pub texpacks: Vec<Ps4TexpackRefDto>,
    #[ts(type = "number | null")]
    pub rink_id: Option<i64>,
    /// The record's own `ArenaID` attribute — not a second key to the arena table.
    #[ts(type = "number | null")]
    pub arena_id_attr: Option<i64>,
}

/// The read-only PS4 venue listing for one arena row (research R73): every level whose id equals
/// `arenaartid`, classed `unambiguous` / `ambiguous` / `level_without_folder` / `no_level` /
/// `unmeasured`; shared directories once; the dependency cascade stated; no re-point offered.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Ps4VenueListingDto {
    pub key_label: String,
    pub class: String,
    pub reason: String,
    pub candidates: Vec<Ps4VenueCandidateDto>,
    /// `(directory, members)` — members `null` when the research names the directory but the
    /// attached tree does not show it.
    #[ts(type = "Array<[string, number | null]>")]
    pub shared_content: Vec<(String, Option<usize>)>,
    pub dependencies: String,
    #[ts(type = "Record<string, number>")]
    pub typed_refs: std::collections::BTreeMap<String, usize>,
    pub reassignment_supported: bool,
    pub reassignment_reason: String,
}

// ---------------------------------------------------------------------------
// Phase 3 (22 Sep 2026): player portrait / head containment — read-only evidence
// ---------------------------------------------------------------------------

/// One player's containment: the ids, the flag as stored, and what the install ships.
#[derive(Serialize, TS)]
pub struct PlayerContainmentDto {
    pub bio_record: u32,
    pub full_name: String,
    #[ts(type = "number")]
    pub portrait_id: u64,
    #[ts(type = "number")]
    pub head_id: u64,
    /// `hasportrait` as stored; `null` when the title has no such column.
    pub has_portrait_flag: Option<bool>,
    /// `bespoke` / `pool` / `other`.
    pub class: String,
    /// `present` / `missing` / `unknown`.
    pub portrait: String,
    pub head: String,
    pub note: Option<String>,
}

/// The title-level cross-tab.
#[derive(Serialize, TS)]
pub struct ContainmentSummaryDto {
    /// `gen7` / `ps4` / `unmeasured`.
    pub contract_status: String,
    pub contract: String,
    pub players: usize,
    pub bespoke: usize,
    pub bespoke_head_present: usize,
    pub bespoke_portrait_present: usize,
    pub pool_players: usize,
    pub pool_head_present: usize,
    #[ts(type = "number | null")]
    pub pool_floor: Option<u64>,
    pub flag1_file1: usize,
    pub flag1_file0: usize,
    pub flag0_file1: usize,
    pub flag0_file0: usize,
    pub has_flag_column: bool,
    pub shipped_portraits: Option<usize>,
    pub shipped_heads: Option<usize>,
    pub unused_portraits: Option<usize>,
    pub unused_heads: Option<usize>,
    pub bespoke_missing_head: Vec<String>,
    pub bespoke_missing_portrait: Vec<String>,
    pub flagged_without_file: Vec<String>,
    pub notes: Vec<String>,
}

/// Players 3.4 (receipt 38): the plan of reassigning a player's portrait or head to another
/// shipped asset (research R124's refusal list), and — after an apply — the edit's outcome.
#[derive(Serialize, TS)]
pub struct PlayerReassignPlanDto {
    /// `portrait` / `head` / `hair` (receipt 66).
    pub kind: String,
    pub bio_record: u32,
    pub player: String,
    /// The stored value, decoded (-1 = the field's "none" value, e.g. `hairid`).
    #[ts(type = "number")]
    pub current_id: i64,
    #[ts(type = "number")]
    pub target_id: i64,
    pub allowed: bool,
    pub refusals: Vec<String>,
    /// The shipped files (and registry row) that make the target valid.
    pub evidence: Vec<String>,
    pub notes: Vec<String>,
    /// Receipt 66: the exact source identity the check read and the fingerprint binds (PS4: each vault member's full
    /// SHA-256 and the identity definitions' SHA-256) — behind disclosure. Empty for presence-by-path evidence.
    pub source_identity: Vec<String>,
    /// Studio ids of the bio cells the apply writes as one change (`pPaD`, plus `headtexture` on PS4 NHL 17–21) — behind
    /// disclosure.
    pub written_fields: Vec<String>,
    /// Receipt 63: the plan's review digest; an apply must name it.
    pub fingerprint: String,
    /// Present after an apply: the edit's outcome (one reviewed, undoable database edit).
    pub applied: Option<OpOutcomeDto>,
}

/// The whole containment view: the summary and every player's row.
#[derive(Serialize, TS)]
pub struct PlayerContainmentViewDto {
    pub summary: ContainmentSummaryDto,
    pub players: Vec<PlayerContainmentDto>,
}

// ---------------------------------------------------------------------------
// Misc 5.4 (22 Sep 2026): the PS4 song catalogue listing — read-only
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, TS)]
pub struct SongRowDto {
    #[ts(type = "number")]
    pub index: i64,
    pub identifier: String,
    #[ts(type = "number")]
    pub artid: i64,
    #[ts(type = "number")]
    pub audioid: i64,
    pub composer: String,
    #[ts(type = "number")]
    pub genre: i64,
    pub genre_key: Option<String>,
    pub is_goal: bool,
    /// `null` on NHL 17 / 18 (no `isintrosong` column).
    pub is_intro: Option<bool>,
    pub is_powerplay: bool,
    pub is_win: bool,
    /// `$TXT_SONG_<IDENTIFIER>` — a dictionary key, kept raw.
    pub name: String,
    /// `name` resolved through this project's own attached dictionary; `null` when unresolved
    /// (show `name` then).
    pub title: Option<String>,
    /// `exact` / `by-hash` / `absent` / `no-dictionary` / `not-a-key`.
    pub title_lookup: String,
    pub genre_label: Option<String>,
    pub genre_lookup: Option<String>,
}

/// One catalogue genre: its id, its `$TXT_…` key, the key resolved through the project's
/// dictionary, and how many rows carry it.
#[derive(Debug, Clone, Serialize, TS)]
pub struct SongGenreCountDto {
    #[ts(type = "number")]
    pub genre: i64,
    pub key: String,
    pub label: Option<String>,
    pub lookup: String,
    pub rows: usize,
}

/// The dictionary a listing's keys were resolved through — this project's own attachment.
#[derive(Debug, Clone, Serialize, TS)]
pub struct DictionaryStampDto {
    pub locale: String,
    pub sha256: String,
    pub rows: usize,
}

/// The selectable-songs listing of a PS4 NHL 17–21 install, or why it is unavailable.
#[derive(Debug, Clone, Serialize, TS)]
pub struct SongCatalogueDto {
    pub available: bool,
    pub reason: Option<String>,
    pub source: Option<String>,
    pub rows_total: usize,
    pub composers: usize,
    /// Genres in descending row-count order, keys resolved through the project's dictionary.
    pub genre_counts: Vec<SongGenreCountDto>,
    pub query: String,
    pub matching: usize,
    pub truncated: bool,
    pub rows: Vec<SongRowDto>,
    pub notes: Vec<String>,
    /// The dictionary the keys resolved through, or `null` when none is attached — the reply's
    /// stamp, so a card never paints one attachment's labels as another's.
    pub dictionary: Option<DictionaryStampDto>,
}

/// Gen7 `.abk` sound banks (25 Sep 2026, R82 / R89 / R91 / R92 consumed): one bank of the loose
/// `audio/sfx` listing — header facts and the sibling `.ast` id check. Listing only.
#[derive(Debug, Serialize, TS)]
pub struct Gen7SfxBankSummaryDto {
    pub name: String,
    pub path: String,
    #[ts(type = "number")]
    pub len: u64,
    pub platform: Option<String>,
    pub records: u32,
    pub stream_records: u32,
    pub ram_records: u32,
    pub lists: u32,
    pub names: u32,
    pub ast: Option<String>,
    #[ts(type = "number | null")]
    pub ast_len: Option<u64>,
    pub ast_id_matches: Option<bool>,
    pub problem: Option<String>,
}

#[derive(Debug, Serialize, TS)]
pub struct Gen7SfxBankListDto {
    pub game_id: String,
    pub platform: String,
    pub population: String,
    pub dir: String,
    pub banks: Vec<Gen7SfxBankSummaryDto>,
    pub notes: Vec<String>,
}

/// One picker-list entry resolved to its record; stream entries carry their framing result.
#[derive(Debug, Serialize, TS)]
pub struct Gen7SfxEntryDto {
    pub record: u32,
    pub weight: u32,
    pub kind: String,
    pub codec: String,
    pub codec_code: u32,
    pub sample_rate: u32,
    pub channel_config: u32,
    pub sub_streams: Option<u32>,
    pub num_samples: u32,
    #[ts(type = "number | null")]
    pub stream_offset: Option<u64>,
    pub blocks: Option<u32>,
    #[ts(type = "number | null")]
    pub stream_bytes: Option<u64>,
    pub framing: Option<String>,
    /// Unknown words carried as read (R94) — disclosure only.
    pub x: u32,
    pub s: u32,
    pub previewable: bool,
    pub preview_refusal: Option<String>,
}

#[derive(Debug, Serialize, TS)]
pub struct Gen7SfxPickerDto {
    pub owners: Vec<String>,
    pub owner_class: Option<String>,
    pub list_offset: u32,
    pub empty_slots: u32,
    pub entries: Vec<Gen7SfxEntryDto>,
}

/// The named picker tree of one bank (R89): every sample list with its owner name(s) and entries.
#[derive(Debug, Serialize, TS)]
pub struct Gen7SfxBankTreeDto {
    pub game_id: String,
    pub platform: String,
    pub bank: String,
    pub bank_path: String,
    #[ts(type = "number")]
    pub bank_len: u64,
    pub bank_sha256: String,
    /// "loose", or the archive member's stored form ("stored" / "refpack" / "chunkzip").
    pub bank_wrapper: String,
    pub bank_stored_sha256: String,
    pub bank_copies: u32,
    pub bank_platform: String,
    pub ast_path: Option<String>,
    #[ts(type = "number | null")]
    pub ast_len: Option<u64>,
    pub ast_id: u32,
    pub ast_id_matches: Option<bool>,
    pub records: u32,
    pub stream_records: u32,
    pub ram_records: u32,
    pub lists: u32,
    pub names: u32,
    pub handler_nodes: u32,
    pub pickers: Vec<Gen7SfxPickerDto>,
    pub notes: Vec<String>,
}

/// The **diagnostic** decode of one `.abk` entry (PS3 EALayer3 v1, one sub-stream; R92 construction).
#[derive(Debug, Serialize, TS)]
pub struct Gen7SfxPreviewDto {
    pub game_id: String,
    pub platform: String,
    pub bank: String,
    pub bank_sha256: String,
    pub record: u32,
    pub kind: String,
    /// Channel config 3 / 5: the stereo sub-stream (0-based) this preview is, of `sub_streams`.
    pub sub_stream: Option<u32>,
    pub sub_streams: u32,
    /// Stream loops: the loop point (the stub's samples) inside the preview.
    pub loop_start: Option<u32>,
    pub owners: Vec<String>,
    pub source: String,
    pub source_sha256: String,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub num_samples: u32,
    pub blocks: u32,
    pub granules: u32,
    pub rebuilt_frames: u32,
    pub decoded_frames_per_channel: u32,
    pub label: String,
    pub wav_data_url: String,
    pub notes: Vec<String>,
}

/// About / licences (receipt 46): read from the files shipped beside the executable. `packaged` is false for a
/// development build run from the repository; `problems` names each expected file or folder that is missing.
#[derive(Serialize, TS)]
pub struct AboutDto {
    pub version: String,
    /// Studio-owned source code: MIT (decision Q46e). Vendored third-party code keeps its own licence.
    pub studio_licence_spdx: String,
    /// The packaged `LICENSE`, when the candidate carries one.
    pub studio_licence_text: Option<String>,
    pub packaged: bool,
    /// The folder the files were read from.
    pub root: String,
    /// The candidate's name from its build record.
    pub candidate: Option<String>,
    pub release_ready: bool,
    /// Why the candidate is not release-ready, in plain words (from its build record).
    pub release_blockers: Vec<String>,
    /// `THIRD-PARTY-NOTICES.txt`.
    pub notices: Option<String>,
    pub licence_files: Vec<AboutLicenceFileDto>,
    pub problems: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct AboutLicenceFileDto {
    /// Archive-relative, `/`-separated.
    pub path: String,
}

/// Presentation 5.2 (receipt 49): an arena's venue packages, read-only — their rasters and the board-ad / bench-ad /
/// jumbotron / power-ring / glass materials with the raster their parameter block names (parsed data, R127 §6). No
/// runtime-selection or replacement claim.
#[derive(Serialize, TS)]
pub struct VenueArtDto {
    #[ts(optional)]
    pub arena_art_id: Option<u64>,
    /// Whether this title / platform is one research measured (else nothing is parsed).
    pub measured: bool,
    pub scope: String,
    pub packages: Vec<VenuePackageDto>,
    pub notes: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct VenuePackageDto {
    /// `stadium/<file>` — what a preview request names.
    pub file: String,
    pub bytes: u64,
    /// The file's SHA-256 at listing; a preview must name it.
    pub sha256: String,
    pub platform: String,
    pub materials: Vec<VenueMaterialDto>,
    pub other_materials: u32,
    pub rasters: Vec<VenueRasterDto>,
    pub notes: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct VenueMaterialDto {
    pub name: String,
    pub class: String,
    #[ts(optional)]
    pub shader: Option<String>,
    pub slots: Vec<VenueSlotDto>,
}

#[derive(Serialize, TS)]
pub struct VenueSlotDto {
    #[ts(optional)]
    pub raster: Option<u32>,
    /// Hex of a stored word that names no raster.
    #[ts(optional)]
    pub raw: Option<String>,
    #[ts(optional)]
    pub unreadable: Option<String>,
}

#[derive(Serialize, TS)]
pub struct VenueRasterDto {
    pub ordinal: u32,
    /// The descriptor's file-table index: what a PS3 board replacement names (receipt 67).
    pub descriptor: u32,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub previewable: bool,
    #[ts(optional)]
    pub note: Option<String>,
    pub used_by: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct VenueRasterPreviewDto {
    pub width: u32,
    pub height: u32,
    /// `data:image/png;base64,…`, longest edge at most the requested size.
    pub png_data_url: String,
}

/// Equipment 4.5 G3 (receipt 51; research 2026-09-28 handoff 02): a PS4 goalie's stored (mask, helmet type) pair against
/// the title's own vault catalogue. Classes other than `unique` are reported, never refused; nothing says what the game
/// draws.
#[derive(Serialize, TS)]
pub struct Ps4GoalieMaskPairDto {
    /// `false` unless this is a PS4 goalie.
    pub applies: bool,
    #[ts(optional)]
    pub maskid: Option<i64>,
    #[ts(optional)]
    pub helmet_type: Option<i64>,
    /// `unique` / `not_a_catalogue_helmet_type` / `only_other_helmet_types` / `absent` / `ambiguous`; `None` when the
    /// catalogue could not be read (see `catalogue_problem`).
    #[ts(optional)]
    pub class: Option<String>,
    pub text: String,
    /// Behind disclosure: the pack's internal name and colour-map FileRef (unique pairs).
    #[ts(optional)]
    pub pack_debug_name: Option<String>,
    #[ts(optional)]
    pub colour_map: Option<String>,
    /// The vault the catalogue came from (title and member hashes).
    #[ts(optional)]
    pub catalogue: Option<String>,
    #[ts(optional)]
    pub catalogue_problem: Option<String>,
    /// Receipt 57: the Creation Zone list row's name in the title's own dictionary (list rows only).
    #[ts(optional)]
    pub mask_name: Option<String>,
    pub notes: Vec<String>,
    /// Receipt 54: the pair's Creation Zone thumbnail (archive, member) and how it was joined:
    /// `creation_zone_row` or `name_convention_candidate`.
    #[ts(optional)]
    pub thumbnail_archive: Option<String>,
    #[ts(optional)]
    pub thumbnail_member: Option<String>,
    #[ts(optional)]
    pub thumbnail_basis: Option<String>,
}

/// Equipment 4.1–4.4 (receipt 52; research 2026-09-28 handoff 02b): a PS4 skater's helmet / gloves / skates / stick /
/// visor numbers against the title's own Creation Zone lists and vault enums. Reported, never refused.
#[derive(Serialize, TS)]
pub struct Ps4SkaterGearDto {
    pub applies: bool,
    pub items: Vec<Ps4SkaterGearItemDto>,
    #[ts(optional)]
    pub catalogue: Option<String>,
    #[ts(optional)]
    pub problem: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct Ps4SkaterGearItemDto {
    /// `helmet` / `gloves` / `skates` / `stick` / `visor`.
    pub family: String,
    pub label: String,
    pub value: i64,
    /// `row` / `catalogue_only` / `outside` / `enum_only_title` / `not_in_catalogue`; `None` without a catalogue.
    #[ts(optional)]
    pub class: Option<String>,
    pub text: String,
    /// The Creation Zone row's `$` name key (a dictionary key, not display text) — behind disclosure.
    #[ts(optional)]
    pub key: Option<String>,
    /// Receipt 57: the key's text in the title's own dictionary (e.g. "BAUER 4500").
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub thumbnail_archive: Option<String>,
    #[ts(optional)]
    pub thumbnail_member: Option<String>,
}

// ======================================================================================
// ONB-002 — project folders, bundled baselines, deployment and extraction
// ======================================================================================

/// The bundled baseline inventory (`baselines/INVENTORY.json`) as the picker shows it.
#[derive(Serialize, TS)]
pub struct BaselineInventoryDto {
    pub schema: String,
    /// The repository commit the inventory was generated at.
    pub head: String,
    pub inventory_path: String,
    pub files_root: String,
    pub targets: Vec<BaselineTargetDto>,
}

#[derive(Serialize, TS)]
pub struct BaselineTargetDto {
    pub game_id: String,
    pub platform: String,
    pub title: String,
    /// `available` | `partial` | `unavailable`.
    pub status: String,
    pub components: Vec<BaselineComponentDto>,
    pub gaps: Vec<BaselineGapDto>,
}

#[derive(Serialize, TS)]
pub struct BaselineComponentDto {
    pub role: String,
    pub file_name: String,
    #[ts(optional)]
    pub container_dir: Option<String>,
    #[ts(type = "number")]
    pub len: u64,
    pub sha256: String,
    /// `verified-vanilla` | `cross-platform-schema` | `derived`.
    pub provenance: String,
    pub witness: String,
    #[ts(optional)]
    pub codec: Option<SaveCodecDto>,
}

#[derive(Serialize, TS)]
pub struct SaveCodecDto {
    pub container_version: String,
    pub kind: String,
    pub readable: bool,
    pub writable: bool,
    pub experimental: bool,
}

#[derive(Serialize, TS)]
pub struct BaselineGapDto {
    pub component: String,
    pub reason: String,
}

/// The import sources chosen for **Use my own files**. Read sources only — never destinations.
#[derive(Deserialize, Serialize, TS, Clone, Default)]
pub struct ImportSourcesDto {
    /// The main database; ONB-003: optional (roster-only / assets-only projects).
    #[serde(default)]
    #[ts(optional)]
    pub main_db: Option<String>,
    /// The schema metadata; `null` = the sibling `<db-stem>-meta.xml`, or the bundled baseline's
    /// metadata when no main database is given.
    #[serde(default)]
    #[ts(optional)]
    pub meta: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub dictionary: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub roster_save: Option<String>,
    /// ONB-003: a folder of extracted game assets (read root for asset routes; outputs are staged
    /// in the project and deployed back here when no installation is linked).
    #[serde(default)]
    #[ts(optional)]
    pub assets_dir: Option<String>,
}

#[derive(Serialize, TS)]
pub struct ImportReviewDto {
    pub ok: bool,
    pub items: Vec<ImportItemReviewDto>,
    /// Where the schema metadata comes from: `sibling` | `explicit` | `bundled (<provenance>)`.
    #[ts(optional)]
    pub meta_source: Option<String>,
}

/// ONB-003: one input of a project (present or explicitly absent).
#[derive(Serialize, TS)]
pub struct ProjectInputDto {
    /// `main_db` | `roster_save` | `dictionary` | `assets`.
    pub role: String,
    /// `bundled` | `import` | `install` | `absent`.
    pub origin: String,
    #[ts(optional)]
    pub note: Option<String>,
}

/// ONB-003: the result of relinking an open project's destinations.
#[derive(Serialize, TS)]
pub struct ProjectRelinkDto {
    pub project: ProjectInfoDto,
    pub discovery: InstallReportDto,
    pub warnings: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct ImportItemReviewDto {
    pub role: String,
    pub path: String,
    #[ts(type = "number")]
    pub len: u64,
    pub sha256: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Serialize, TS)]
pub struct ProjectDestinationsDto {
    pub picked_root: String,
    pub install_dir: Option<String>,
    pub game_dir: Option<String>,
    pub save_root: Option<String>,
    pub roster_save: Option<String>,
    pub main_db_path: Option<String>,
    pub meta_xml_path: Option<String>,
    pub localization_path: Option<String>,
    /// ONB-003: the linked extracted-assets folder, if any.
    #[ts(optional)]
    pub assets_dir: Option<String>,
}

#[derive(Serialize, TS)]
pub struct ProjectComponentDto {
    pub role: String,
    pub file_name: String,
    pub origin: String,
    #[ts(type = "number")]
    pub len: u64,
    pub sha256: String,
    pub provenance: String,
    pub witness: String,
}

#[derive(Serialize, TS)]
pub struct StagedFileDto {
    pub relative: String,
    pub origin: String,
    pub at: String,
    /// ONB-003: the project source the output was encoded from, when kept.
    #[ts(optional)]
    pub source_sha256: Option<String>,
}

#[derive(Serialize, TS)]
pub struct DeploymentRecordDto {
    #[ts(type = "number")]
    pub seq: u64,
    pub at: String,
    pub state: String,
    pub restorable: bool,
    /// An interrupted or split deployment / restore that Restore finishes.
    pub recovery: bool,
}

/// The open project folder: manifest view.
#[derive(Serialize, TS)]
pub struct ProjectInfoDto {
    pub dir: String,
    pub project_id: String,
    pub created: String,
    pub game_id: String,
    pub platform: String,
    pub title: String,
    /// `fresh` | `import`.
    pub source_kind: String,
    #[ts(optional)]
    pub inventory_head: Option<String>,
    pub components: Vec<ProjectComponentDto>,
    pub gaps: Vec<BaselineGapDto>,
    pub destinations: ProjectDestinationsDto,
    #[ts(optional)]
    pub legacy_sidecar: Option<String>,
    /// `pending` | `skipped` | `extracted` | `not_needed`.
    pub extraction_choice: String,
    pub working_dir: String,
    pub exports_dir: String,
    /// ONB-003: absent for roster-only and assets-only projects.
    #[ts(optional)]
    pub working_main_db: Option<String>,
    #[ts(optional)]
    pub working_roster_save: Option<String>,
    #[ts(optional)]
    pub working_dictionary: Option<String>,
    #[ts(type = "number")]
    pub saves: u64,
    #[ts(optional)]
    pub last_saved: Option<String>,
    pub deployments: Vec<DeploymentRecordDto>,
    pub staged_files: Vec<StagedFileDto>,
    /// ONB-003: `database` (a main database opens) | `roster` (roster save only) | `assets`.
    pub mode: String,
    pub inputs: Vec<ProjectInputDto>,
    #[ts(optional)]
    pub last_view: Option<String>,
    #[ts(optional)]
    pub migrated_from: Option<String>,
    pub schema: String,
    /// Project-owned editable sources kept under `sources/`.
    #[ts(type = "number")]
    pub sources: u64,
    /// Archive-member replacements committed by saves (semantic records).
    #[ts(type = "number")]
    pub member_edits: u64,
}

#[derive(Clone, Serialize, Deserialize, TS)]
pub struct ProjectDetailsDto {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

#[derive(Serialize, TS)]
pub struct ProjectBuildDto {
    pub packages: Vec<PackageBuildDto>,
    pub skipped: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct ProjectArchiveDto {
    pub path: String,
    #[ts(type = "number")]
    pub files: u64,
    #[ts(type = "number")]
    pub len: u64,
    pub sha256: String,
}

#[derive(Serialize, TS)]
pub struct ProjectArchivePreviewDto {
    pub name: String,
    pub game_id: String,
    pub platform: String,
}

#[derive(Serialize, TS)]
pub struct ProjectOverviewDto {
    pub details: ProjectDetailsDto,
    pub dirty: bool,
    pub changes: Vec<ProjectOverviewChangeDto>,
    pub notices: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct ProjectOverviewChangeDto {
    pub id: String,
    /// Collapsed hierarchy, excluding the changed field itself.
    pub groups: Vec<String>,
    pub label: String,
    pub before: String,
    pub after: String,
    pub route: Option<String>,
}

/// A project opened (created or reopened): the session summary, the folder view and the
/// destinations as discovery sees them now. ONB-003: `warnings` name every external resource that
/// is missing or changed (nothing is substituted) and `last_view` is the route to restore.
#[derive(Serialize, TS)]
pub struct ProjectOpenDto {
    pub summary: ProjectSummaryDto,
    pub project: ProjectInfoDto,
    pub discovery: InstallReportDto,
    pub warnings: Vec<String>,
    #[ts(optional)]
    pub last_view: Option<String>,
}

#[derive(Serialize, TS)]
pub struct ProjectSaveDto {
    pub saved: bool,
    pub nothing_to_save: bool,
    #[ts(optional)]
    pub txn_id: Option<String>,
    #[ts(optional, type = "number | null")]
    pub seq: Option<u64>,
    /// Installation archives copied into the project for this save.
    pub materialized: Vec<String>,
    pub summary: ProjectSummaryDto,
}

#[derive(Serialize, TS)]
pub struct DeployItemDto {
    pub key: String,
    pub role: String,
    pub label: String,
    pub source_path: String,
    pub source: FileIdentityDto,
    #[ts(optional)]
    pub destination_path: Option<String>,
    #[ts(optional)]
    pub destination: Option<FileIdentityDto>,
    /// `already-current` | `unchanged` | `changed-externally` | `absent` | `unresolved` | `unreadable`.
    pub state: String,
    #[ts(optional)]
    pub blocked: Option<String>,
}

#[derive(Serialize, TS)]
pub struct DeployPlanDto {
    pub handle: u32,
    #[ts(type = "number")]
    pub seq: u64,
    pub deployable: bool,
    pub dirty: bool,
    pub reasons: Vec<String>,
    #[ts(type = "number")]
    pub backup_bytes: u64,
    #[ts(type = "number")]
    pub replacement_bytes: u64,
    pub items: Vec<DeployItemDto>,
}

#[derive(Serialize, TS)]
pub struct DeployOutcomeDto {
    pub key: String,
    pub label: String,
    pub path: String,
    /// `deployed` | `prior` | `unknown`.
    pub state: String,
    #[ts(optional)]
    pub backup: Option<String>,
}

#[derive(Serialize, TS)]
pub struct DeployApplyDto {
    #[ts(type = "number")]
    pub seq: u64,
    /// `committed` | `rolled-back` | `split`.
    pub state: String,
    #[ts(optional)]
    pub detail: Option<String>,
    pub record_dir: String,
    pub destinations: Vec<DeployOutcomeDto>,
}

#[derive(Serialize, TS)]
pub struct DeployRestoreDto {
    #[ts(type = "number")]
    pub seq: u64,
    pub state: String,
    pub items: Vec<DeployOutcomeDto>,
}

// ---------------------------------------------------------------------------------------------
// ONB-003 — mod package builder and installer
// ---------------------------------------------------------------------------------------------

/// One packageable category of the open project with its items (labels) and blockers.
#[derive(Serialize, TS)]
pub struct PackageCategoryDto {
    /// `cells` | `dictionary` | `textures` | `members` | `native` | `structural`.
    pub category: String,
    #[ts(type = "number")]
    pub count: u64,
    /// `ready` | `empty` | `blocked`.
    pub status: String,
    pub items: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct PackageComponentDto {
    pub category: String,
    /// `supported` | `unsupported` | `empty`.
    pub status: String,
    pub reason: String,
}

#[derive(Serialize, TS)]
pub struct PackageTargetDto {
    pub platform: String,
    /// `supported` | `unsupported`.
    pub status: String,
    pub components: Vec<PackageComponentDto>,
}

#[derive(Serialize, TS)]
pub struct PackageBaseDto {
    pub role: String,
    pub file_name: String,
    #[ts(type = "number")]
    pub len: u64,
    pub sha256: String,
    pub provenance: String,
}

#[derive(Serialize, TS)]
pub struct PackageCategoryCountDto {
    pub category: String,
    #[ts(type = "number")]
    pub count: u64,
}

/// What the open project could package right now (read-only).
#[derive(Serialize, TS)]
pub struct PackageInventoryDto {
    pub dirty: bool,
    #[ts(type = "number")]
    pub saves: u64,
    pub categories: Vec<PackageCategoryDto>,
    pub targets: Vec<PackageTargetDto>,
    pub blockers: Vec<String>,
    pub default_name: String,
    pub default_version: String,
    pub exports_dir: String,
    pub base: Vec<PackageBaseDto>,
}

/// The build request: identity, the categories included, the categories explicitly excluded
/// (an acknowledged omission is never silent), the declared platforms and an optional output path.
#[derive(Deserialize, Serialize, TS, Clone, Default)]
pub struct PackageBuildRequestDto {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    #[ts(optional)]
    pub out_path: Option<String>,
}

#[derive(Serialize, TS)]
pub struct PackageBuildDto {
    pub path: String,
    pub package_id: String,
    #[ts(type = "number")]
    pub len: u64,
    pub sha256: String,
    #[ts(type = "number")]
    pub entries: u64,
    pub categories: Vec<PackageCategoryCountDto>,
    pub targets: Vec<PackageTargetDto>,
}

#[derive(Serialize, TS)]
pub struct PackageEntryDto {
    pub path: String,
    #[ts(type = "number")]
    pub len: u64,
    pub sha256: String,
    pub kind: String,
}

#[derive(Serialize, TS)]
pub struct PackageCellDto {
    pub file: String,
    pub table_name: String,
    pub record: u32,
    pub entity: String,
    pub field_name: String,
    pub before: String,
    pub after: String,
}

#[derive(Serialize, TS)]
pub struct PackageDictionaryDto {
    pub key: String,
    pub before: String,
    pub after: String,
}

#[derive(Serialize, TS)]
pub struct PackageTextureDto {
    pub rel_path: String,
    #[ts(optional)]
    pub texture_index: Option<u32>,
    pub width: u32,
    pub height: u32,
    pub platform: String,
    pub payload: String,
}

/// A validated package, as the installer shows it before any destination is chosen.
#[derive(Serialize, TS)]
pub struct PackageInspectDto {
    pub path: String,
    #[ts(type = "number")]
    pub len: u64,
    pub sha256: String,
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub created: String,
    pub studio_version: String,
    pub game_id: String,
    pub title: String,
    pub author_platform: String,
    pub targets: Vec<PackageTargetDto>,
    pub base: Vec<PackageBaseDto>,
    pub categories: Vec<PackageCategoryCountDto>,
    pub entries: Vec<PackageEntryDto>,
    pub cells: Vec<PackageCellDto>,
    pub dictionary: Vec<PackageDictionaryDto>,
    pub textures: Vec<PackageTextureDto>,
    /// Read-only hierarchy of every packaged operation, including structural changes.
    pub changes: Vec<ProjectOverviewChangeDto>,
    pub project_source_kind: String,
    /// The edits document schema (`nhlms-modpkg-edits/2` binds rows by identity; `/1` by position
    /// and is refused by the installer).
    pub edits_schema: String,
}

#[derive(Serialize, TS)]
pub struct ModLibraryEntryDto {
    pub package_id: String,
    pub path: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub game_id: String,
    pub platforms: Vec<String>,
    pub created: String,
}

#[derive(Serialize, TS)]
pub struct ModLibraryDto {
    pub mods: Vec<ModLibraryEntryDto>,
    pub warnings: Vec<String>,
}

#[derive(Serialize, TS)]
pub struct InstallRequirementDto {
    /// `main_db` | `roster_save` | `dictionary` | `assets`.
    pub role: String,
    /// `found` | `missing` | `not-needed`.
    pub status: String,
    pub detail: String,
}

#[derive(Serialize, TS)]
pub struct InstallItemDto {
    pub key: String,
    pub role: String,
    pub label: String,
    pub destination_path: String,
    #[ts(optional)]
    pub destination: Option<FileIdentityDto>,
    pub replacement: FileIdentityDto,
    /// `replace` | `create` | `unchanged`.
    pub state: String,
    pub notes: Vec<String>,
}

/// The reviewed installation plan (read-only until Apply).
#[derive(Serialize, TS)]
pub struct InstallPlanDto {
    pub handle: u32,
    pub installable: bool,
    pub reasons: Vec<String>,
    pub conflicts: Vec<String>,
    pub requirements: Vec<InstallRequirementDto>,
    pub items: Vec<InstallItemDto>,
    #[ts(type = "number")]
    pub applied: u64,
    #[ts(type = "number")]
    pub already_present: u64,
    #[ts(type = "number")]
    pub backup_bytes: u64,
    #[ts(type = "number")]
    pub replacement_bytes: u64,
    pub package_name: String,
    pub package_version: String,
    pub package_id: String,
    pub game_id: String,
    pub platform: String,
    pub records_dir: String,
}

#[derive(Serialize, TS)]
pub struct InstallApplyDto {
    #[ts(type = "number")]
    pub seq: u64,
    /// `committed` | `rolled-back` | `split` | `restored`.
    pub state: String,
    #[ts(optional)]
    pub detail: Option<String>,
    pub record_dir: String,
    pub destinations: Vec<DeployOutcomeDto>,
}

#[derive(Serialize, TS)]
pub struct InstallRecordDto {
    pub record_dir: String,
    #[ts(type = "number")]
    pub seq: u64,
    pub at: String,
    pub state: String,
    pub package_name: String,
    pub package_version: String,
    pub package_id: String,
    pub game_id: String,
    pub platform: String,
    pub picked_root: String,
    /// The record can be restored or recovered (`committed`, `restore-incomplete`, `split`,
    /// `publishing`); `recovery` says it is the latter three — an interrupted operation.
    pub restorable: bool,
    pub recovery: bool,
    pub items: Vec<DeployOutcomeDto>,
}

/// Read-only extraction preflight of a game installation.
#[derive(Serialize, TS)]
pub struct ExtractPreflightDto {
    pub root: String,
    /// `not_applicable` | `complete` | `needed` | `partial` | `blocked`.
    pub state: String,
    pub reason: String,
    pub archives: Vec<ExtractArchiveDto>,
    #[ts(type = "number")]
    pub members_total: u64,
    #[ts(type = "number")]
    pub members_present: u64,
    #[ts(type = "number")]
    pub members_to_write: u64,
    #[ts(type = "number")]
    pub bytes_to_write: u64,
    #[ts(type = "number | null")]
    pub free_bytes: Option<u64>,
    /// Member names claimed by more than one archive (precedence unknown: blocked, not guessed).
    pub collisions: Vec<ExtractCollisionDto>,
}

#[derive(Serialize, TS)]
pub struct ExtractArchiveDto {
    pub path: String,
    pub format: String,
    #[ts(type = "number")]
    pub members: u64,
    #[ts(type = "number")]
    pub present: u64,
    /// `complete` | `partial` | `pending` | `unsupported` | `failed`.
    pub state: String,
    #[ts(optional)]
    pub note: Option<String>,
}

#[derive(Serialize, TS)]
pub struct ExtractCollisionDto {
    pub member: String,
    pub archives: Vec<String>,
}

/// A running or finished extraction job.
#[derive(Serialize, TS, Clone)]
pub struct ExtractJobDto {
    pub destination: String,
    #[ts(type = "number")]
    pub current_files_done: u64,
    #[ts(type = "number")]
    pub current_files_total: u64,
    #[ts(type = "number")]
    pub files_done: u64,
    #[ts(type = "number")]
    pub files_total: u64,
    pub job: u32,
    pub root: String,
    /// `running` | `cancelling` | `done` | `cancelled` | `failed`.
    pub state: String,
    pub current: String,
    #[ts(type = "number")]
    pub archives_done: u64,
    #[ts(type = "number")]
    pub archives_total: u64,
    #[ts(type = "number")]
    pub files_written: u64,
    #[ts(type = "number")]
    pub files_retained: u64,
    #[ts(type = "number")]
    pub files_conflicting: u64,
    #[ts(type = "number")]
    pub files_blocked: u64,
    #[ts(type = "number")]
    pub files_unsupported: u64,
    #[ts(type = "number")]
    pub files_unsafe: u64,
    #[ts(type = "number")]
    pub archives_failed: u64,
    #[ts(type = "number")]
    pub bytes_written: u64,
    pub problems: Vec<String>,
    #[ts(optional)]
    pub error: Option<String>,
}

// ---- Save Editor ------------------------------------------------------------------------------

/// Save Editor: one bundled game target whose definitions describe an opened file.
#[derive(Serialize, TS)]
pub struct SaveCandidateDto {
    /// Studio game id (`nhl14`, `legacyrecomp` …) — pass back to `save_editor_open`.
    pub game_id: String,
    /// `ps3` | `xbox` | `ps4` | `recomp` — pass back to `save_editor_open`.
    pub platform: String,
    /// Display title, e.g. `NHL 14`.
    pub title: String,
    /// Display platform, e.g. `PlayStation 3`.
    pub platform_label: String,
    /// The file's database layout equals this target's own bundled database or roster save.
    pub exact_schema: bool,
    /// Plain-language reasons this target is listed.
    pub evidence: Vec<String>,
    /// Among the candidates with the strongest evidence (the ones offered by default).
    pub leading: bool,
}

/// Save Editor: what a chosen file is, which game it belongs to, and whether it may be opened.
#[derive(Serialize, TS)]
pub struct SaveFileIdentityDto {
    pub path: String,
    pub file_name: String,
    /// `database` | `container` | `ps4_save`.
    pub format: String,
    /// Plain-language format, e.g. `PS3 / Xbox 360 save container (compressed)`.
    pub format_label: String,
    /// `database` | `roster` | `beagm` | `season` | `beapro` | `threes` | `hockeycard` |
    /// `settings` | `unknown`.
    pub kind: String,
    pub kind_label: String,
    /// `v1` … `v4` | `ps4`; `null` for a bare database.
    pub container_version: Option<String>,
    /// Both stored container checksums matched on open (`null` where none are verified).
    pub checksums_valid: Option<bool>,
    pub writable: bool,
    /// Writing this container variant is not verified in-game.
    pub write_experimental: bool,
    /// An NHL 07/08-style roster change list (recognised, not editable).
    pub change_list: bool,
    /// The leading candidates agree on the game.
    pub game_confident: bool,
    /// Exactly one candidate leads (game and platform both settled).
    pub platform_confident: bool,
    /// Compatible targets, best evidence first.
    pub candidates: Vec<SaveCandidateDto>,
    /// `allowed` | `caution` (open after the user acknowledges `decision_reason`) | `refused`.
    pub decision: String,
    pub decision_reason: Option<String>,
    /// Non-blocking facts to show before saving (experimental writes, stale checksums).
    pub notes: Vec<String>,
    /// Title ids found in the file's path (diagnostic, for progressive disclosure).
    pub path_hints: Vec<String>,
    /// Database tables the file itself carries (editable).
    pub own_tables: u32,
    /// Tables a save lacks that are shown read-only from the game's default database.
    pub supplied_tables: u32,
}

/// Save Editor: an opened file's edit session.
#[derive(Serialize, TS)]
pub struct SaveEditorOpenDto {
    pub summary: ProjectSummaryDto,
    pub file: SaveFileIdentityDto,
    /// The target the session was opened as.
    pub game_id: String,
    pub platform: String,
    pub title: String,
    pub platform_label: String,
    /// The edit session is view-only (the container cannot be written back).
    pub read_only: bool,
    /// Tables shown read-only from the game's default database because the save lacks them.
    pub supplied_tables: u32,
}

/// Save Editor: the result of saving back to the opened file.
#[derive(Serialize, TS)]
pub struct SaveEditorSaveDto {
    pub files_written: u32,
    /// The copy of the original file taken before this session first overwrote it (`null` when
    /// an earlier save in this session already took it).
    pub backup_path: Option<String>,
    pub summary: ProjectSummaryDto,
}
