//! The Tauri command surface — one thin wrapper per command, forwarding to the [`Registry`] and
//! stringifying errors (Tauri serializes `Err(String)` to the frontend). Register these in the
//! Tauri builder via `tauri::generate_handler!` (see `src-tauri`).

use std::path::{Path, PathBuf};

use tauri::{Manager, State};

use studio_domain::FieldValue;

use crate::dto::{
    AssetExportSummaryDto, AssetRootsArgsDto, AssetRootsDto, BrowseListingDto, ExportOptionsDto,
    FileExportOptionsDto, FileExportSummaryDto, HeadPreviewDto, ImportTextureSummaryDto,
    KitPieceInputDto, KitPreviewDto, OutfitExportSummaryDto, PlayerAssetsDto,
    AddCareerSeasonDto, ArenaSummaryDto, ArenaViewDto, AutocorrectSummaryDto, CareerStatsDto,
    CreatePlayerDto, EaComputedRatingsDto, ExtractSummaryDto, FormulaCatalogDto, FormulaProfileDto,
    HistoricalCareerRatingsDto, HistoricalFiltersDto, HistoricalPersonDto, HistoricalPersonSummaryDto,
    HistoricalTeamDto, HistoricalTeamFiltersDto, HistoricalTeamFranchiseDto,
    HistoricalTeamIndexEntryDto, HistoricalTeamSummaryDto, InstallReportDto, IntegrityReportDto,
    LinesViewDto, OpOutcomeDto, PlayerSummaryDto, PlayerViewDto, ProjectSummaryDto,
    RepairBlocksSummaryDto, TeamSummaryDto, TeamViewDto,
    ChangeSetPreviewDto, ImportItemDto, ImportResultDto, MatchCandidateDto,
};
use crate::registry::{parse_platform, parse_role, Registry};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn load_project(
    registry: State<'_, Registry>,
    db_path: String,
    platform: String,
    role: String,
    meta_xml: String,
    defs_root: String,
) -> CmdResult<ProjectSummaryDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let role = parse_role(&role).map_err(|e| e.to_string())?;
    registry
        .load_project(&db_path, platform, role, &meta_xml, &defs_root)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_project(registry: State<'_, Registry>, project_id: u32) -> bool {
    registry.close_project(project_id)
}

/// Onboarding step 1: point at a root folder (RPCS3 root for PS3, shadPS4 root for PS4, the
/// `default.xex` install dir for Xbox, the game folder for Recomp) and report the resolved databases
/// + save inventory. `save_path` locates the roster/profile saves for platforms whose emulator saves
/// aren't in a consistent place (Xbox / Recomp); ignored for PS3/PS4.
#[tauri::command]
pub fn detect_install(
    registry: State<'_, Registry>,
    game_id: String,
    platform: String,
    root: String,
    save_path: Option<String>,
) -> CmdResult<InstallReportDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    Ok(registry.detect_install(&game_id, platform, &root, save_path.as_deref()))
}

/// Onboarding step 2: open the project from the chosen root — enforces the modding minimum
/// (`nhlng.db` + a roster save), writes the `.nhlmodproj` profile sidecar, and begins an edit
/// session over the canonical `nhlng.db`.
#[tauri::command]
pub fn open_workspace(
    app: tauri::AppHandle,
    registry: State<'_, Registry>,
    game_id: String,
    platform: String,
    root: String,
    save_path: Option<String>,
    big_files_extracted: bool,
) -> CmdResult<ProjectSummaryDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let defs = resolve_defs(&app);
    registry
        .open_workspace(
            &game_id,
            platform,
            &root,
            save_path.as_deref(),
            big_files_extracted,
            &defs.to_string_lossy(),
        )
        .map_err(|e| e.to_string())
}

/// Extract a game's `.big` archives in place, given the install root the user pointed at. Walks the
/// root for EA archives — Xbox/PS3/Recomp `BIGF`/`BIG4` and PS4 `EB\0\3` (the latter streamed, since
/// PS4 archives can be tens of GB) — and writes every entry out preserving its internal folder
/// hierarchy (chunkzip payloads are inflated; chunklzx entries are skipped). Returns a summary of
/// what was written. Runs at onboarding (before a project is opened), so it takes the install
/// context rather than a project id.
#[tauri::command]
pub fn extract_big_files(
    registry: State<'_, Registry>,
    _game_id: String,
    _platform: String,
    root: String,
) -> CmdResult<ExtractSummaryDto> {
    registry.extract_install(&root).map_err(|e| e.to_string())
}

/// Resolve the bundled `defs/` directory: a packaged resource in release, falling back to the
/// in-repo `defs/` during development.
fn resolve_defs(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(res) = app.path().resource_dir() {
        let p = res.join("defs");
        if p.join("enums").is_dir() {
            return p;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../defs")
}

/// One-step project open for onboarding: the user picks only the database (or save container); the
/// metadata XML is derived as its sibling `{db-stem}-meta.xml` (so `nhlng.db`→`nhlng-meta.xml`,
/// PS4 `nhl_game.db`→`nhl_game-meta.xml`, `eng_us.db`→`eng_us-meta.xml`) and `defs/` is resolved
/// automatically. Use `load_project` (with explicit meta/defs) for the Advanced override path.
#[tauri::command]
pub fn open_project(
    app: tauri::AppHandle,
    registry: State<'_, Registry>,
    db_path: String,
    platform: String,
    role: String,
) -> CmdResult<ProjectSummaryDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let role = parse_role(&role).map_err(|e| e.to_string())?;
    let db = Path::new(&db_path);
    let stem = db
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "could not read the database filename".to_string())?;
    let meta = db
        .parent()
        .map(|d| d.join(format!("{stem}-meta.xml")))
        .ok_or_else(|| "could not derive the database folder".to_string())?;
    if !meta.exists() {
        return Err(format!(
            "no {stem}-meta.xml found beside the database ({}). Use Advanced to pick it manually.",
            meta.display()
        ));
    }
    let defs = resolve_defs(&app);
    registry
        .load_project(
            &db_path,
            platform,
            role,
            &meta.to_string_lossy(),
            &defs.to_string_lossy(),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn project_summary(
    registry: State<'_, Registry>,
    project_id: u32,
) -> CmdResult<ProjectSummaryDto> {
    registry.summary(project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_project(
    registry: State<'_, Registry>,
    project_id: u32,
) -> CmdResult<IntegrityReportDto> {
    registry.validate(project_id).map_err(|e| e.to_string())
}

/// Enable/disable the acknowledged corruption-override edit mode for a project. With it on, edits are
/// permitted on a database that fails the structural audit (unsupported — surfaced by the on-load
/// corruption prompt after the user acknowledges the risk).
#[tauri::command]
pub fn set_edit_override(
    registry: State<'_, Registry>,
    project_id: u32,
    allow: bool,
) -> CmdResult<()> {
    registry
        .set_edit_override(project_id, allow)
        .map_err(|e| e.to_string())
}

/// Switch which file the project's read views display: `nhlng.db` (default) or the mirrored roster
/// save. Requires an attached roster mirror. On an aligned pair writes are unaffected (they fan out
/// to both files); on a misaligned pair the workspace enters roster-only mode and edits apply
/// strictly to the roster file. Returns the refreshed project summary reflecting the new source.
#[tauri::command]
pub fn set_display_roster(
    registry: State<'_, Registry>,
    project_id: u32,
    on: bool,
) -> CmdResult<ProjectSummaryDto> {
    registry
        .set_display_roster(project_id, on)
        .map_err(|e| e.to_string())
}

/// Auto-correct duplicate jersey numbers on load (default reassignment logic); returns a summary of
/// the corrections applied (empty when none were needed). Corrections are staged as a pending edit.
#[tauri::command]
pub fn autocorrect_jerseys(
    registry: State<'_, Registry>,
    project_id: u32,
) -> CmdResult<AutocorrectSummaryDto> {
    registry
        .autocorrect_jerseys(project_id)
        .map_err(|e| e.to_string())
}

/// Repair a corrupted roster-block layout (the "Corrupted roster block" audit errors) — an
/// order-only permutation of the roster slot table back into contiguous per-team blocks, staged as
/// a pending, undoable edit. Returns how many slots moved and which teams were affected.
#[tauri::command]
pub fn repair_roster_blocks(
    registry: State<'_, Registry>,
    project_id: u32,
) -> CmdResult<RepairBlocksSummaryDto> {
    registry
        .repair_roster_blocks(project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_players(
    registry: State<'_, Registry>,
    project_id: u32,
) -> CmdResult<Vec<PlayerSummaryDto>> {
    registry.list_players(project_id).map_err(|e| e.to_string())
}

/// The free-agent pool (players occupying no roster slot — the S1 occupancy model), in the
/// master-list row shape.
#[tauri::command]
pub fn list_free_agents(
    registry: State<'_, Registry>,
    project_id: u32,
) -> CmdResult<Vec<PlayerSummaryDto>> {
    registry.list_free_agents(project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn player_view(
    registry: State<'_, Registry>,
    project_id: u32,
    player_id: u64,
) -> CmdResult<Option<PlayerViewDto>> {
    registry
        .player_view(project_id, player_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn career_stats(
    registry: State<'_, Registry>,
    project_id: u32,
    player_id: u64,
) -> CmdResult<CareerStatsDto> {
    registry.career_stats(project_id, player_id).map_err(|e| e.to_string())
}

/// Add a stock career season row (`table` = one of the four career tables); stat cells start zero.
#[tauri::command]
pub fn add_career_season(
    registry: State<'_, Registry>,
    project_id: u32,
    table: String,
    player_id: u64,
    year_end: u64,
    league: u64,
    team: String,
) -> CmdResult<AddCareerSeasonDto> {
    registry
        .add_career_season(project_id, &table, player_id, year_end, league, &team)
        .map_err(|e| e.to_string())
}

/// Delete one stock career row by record index (ordered shift).
#[tauri::command]
pub fn delete_career_row(
    registry: State<'_, Registry>,
    project_id: u32,
    table: String,
    record: u32,
) -> CmdResult<OpOutcomeDto> {
    registry
        .delete_career_row(project_id, &table, record)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_teams(registry: State<'_, Registry>, project_id: u32) -> CmdResult<Vec<TeamSummaryDto>> {
    registry.list_teams(project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn team_view(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Option<TeamViewDto>> {
    registry.team_view(project_id, team_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn lines_view(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Option<LinesViewDto>> {
    registry.lines_view(project_id, team_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn team_strategies(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Option<crate::dto::TeamStrategiesDto>> {
    registry.team_strategies(project_id, team_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn team_draft_picks(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Vec<crate::dto::DraftPickDto>> {
    registry.team_draft_picks(project_id, team_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_team_jerseys(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Vec<crate::dto::TeamJerseyDto>> {
    registry.list_team_jerseys(project_id, team_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_team_equipment(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Vec<crate::dto::TeamEquipmentDto>> {
    registry.list_team_equipment(project_id, team_id).map_err(|e| e.to_string())
}

/// Trade a future draft pick (`qHav` row) to another team. The original-owner field is left
/// untouched so provenance ("via trade from …") stays visible.
#[tauri::command]
pub fn trade_draft_pick(
    registry: State<'_, Registry>,
    project_id: u32,
    record: u32,
    to_team: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .trade_draft_pick(project_id, record, to_team)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_arenas(
    registry: State<'_, Registry>,
    project_id: u32,
) -> CmdResult<Vec<ArenaSummaryDto>> {
    registry.list_arenas(project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn arena_view(
    registry: State<'_, Registry>,
    project_id: u32,
    arena_id: u64,
) -> CmdResult<Option<ArenaViewDto>> {
    registry.arena_view(project_id, arena_id).map_err(|e| e.to_string())
}

/// Stage a validated field edit. Provide exactly one of `text_value` (string fields) or `int_value`
/// (integer fields).
#[tauri::command]
pub fn set_field(
    registry: State<'_, Registry>,
    project_id: u32,
    table: String,
    record: u32,
    field: String,
    int_value: Option<i64>,
    text_value: Option<String>,
) -> CmdResult<OpOutcomeDto> {
    let value = match (text_value, int_value) {
        (Some(s), _) => FieldValue::Text(s),
        (None, Some(i)) => FieldValue::Int(i),
        (None, None) => return Err("set_field requires int_value or text_value".to_string()),
    };
    registry
        .set_field(project_id, &table, record, &field, value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_captaincy(
    registry: State<'_, Registry>,
    project_id: u32,
    team: u64,
    player: u64,
    role: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .set_captaincy(project_id, team, player, role)
        .map_err(|e| e.to_string())
}

/// Activate (`active = true`) or deactivate a trait on a player's ratings record. `table` is the
/// ratings companion (`dSvy`/`mHuy`), `record` its record index, `value` the trait number.
#[tauri::command]
pub fn set_player_trait(
    registry: State<'_, Registry>,
    project_id: u32,
    table: String,
    record: u32,
    value: i64,
    active: bool,
) -> CmdResult<OpOutcomeDto> {
    registry
        .set_player_trait(project_id, &table, record, value, active)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_player(
    registry: State<'_, Registry>,
    project_id: u32,
    team: u64,
    position: u64,
) -> CmdResult<CreatePlayerDto> {
    registry
        .create_player(project_id, team, position)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clone_player(
    registry: State<'_, Registry>,
    project_id: u32,
    source_player: u64,
    to_team: u64,
) -> CmdResult<CreatePlayerDto> {
    registry
        .clone_player(project_id, source_player, to_team)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_player(
    registry: State<'_, Registry>,
    project_id: u32,
    player: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .delete_player(project_id, player)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_player(
    registry: State<'_, Registry>,
    project_id: u32,
    player: u64,
    from_team: u64,
    to_team: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .move_player(project_id, player, from_team, to_team)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trade_slots(
    registry: State<'_, Registry>,
    project_id: u32,
    a_team: u64,
    a_player: u64,
    b_team: u64,
    b_player: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .trade_slots(project_id, a_team, a_player, b_team, b_player)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn release_to_fa(
    registry: State<'_, Registry>,
    project_id: u32,
    from_team: u64,
    player: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .release_to_fa(project_id, from_team, player)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sign_from_fa(
    registry: State<'_, Registry>,
    project_id: u32,
    player: u64,
    to_team: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .sign_from_fa(project_id, player, to_team)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_membership(
    registry: State<'_, Registry>,
    project_id: u32,
    player: u64,
    to_team: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .add_membership(project_id, player, to_team)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_membership(
    registry: State<'_, Registry>,
    project_id: u32,
    from_team: u64,
    player: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .remove_membership(project_id, from_team, player)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_jersey_number(
    registry: State<'_, Registry>,
    project_id: u32,
    team: u64,
    player: u64,
    number: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .set_jersey_number(project_id, team, player, number)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn retire_jersey(
    registry: State<'_, Registry>,
    project_id: u32,
    team: u64,
    number: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .retire_jersey(project_id, team, number)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unretire_jersey(
    registry: State<'_, Registry>,
    project_id: u32,
    team: u64,
    number: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .unretire_jersey(project_id, team, number)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_dressed(
    registry: State<'_, Registry>,
    project_id: u32,
    team: u64,
    player: u64,
    dressed: bool,
) -> CmdResult<OpOutcomeDto> {
    registry
        .set_dressed(project_id, team, player, dressed)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn undo(registry: State<'_, Registry>, project_id: u32) -> CmdResult<bool> {
    registry.undo(project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn redo(registry: State<'_, Registry>, project_id: u32) -> CmdResult<bool> {
    registry.redo(project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_project(
    registry: State<'_, Registry>,
    project_id: u32,
    out_path: String,
) -> CmdResult<u64> {
    registry
        .save(project_id, &out_path)
        .map_err(|e| e.to_string())
}

/// In-place dual save: write `nhlng.db` and (when attached) the roster save back to the paths the
/// project was opened from. Returns the number of files written.
#[tauri::command]
pub fn save_workspace(registry: State<'_, Registry>, project_id: u32) -> CmdResult<u64> {
    registry.save_workspace(project_id).map_err(|e| e.to_string())
}

// ---- Historical (FHM) corpus ----------------------------------------------

/// Open an FHM historical corpus data pack (read-only). Returns the person count. Call once before
/// any other historical command; a different path replaces the open corpus.
#[tauri::command]
pub fn open_corpus(registry: State<'_, Registry>, path: String) -> CmdResult<u64> {
    registry.open_corpus(&path).map_err(|e| e.to_string())
}

/// Whether a historical corpus is currently open.
#[tauri::command]
pub fn corpus_open(registry: State<'_, Registry>) -> bool {
    registry.corpus_open()
}

/// The season + league filter options for the Historical browse.
#[tauri::command]
pub fn historical_filters(registry: State<'_, Registry>) -> CmdResult<HistoricalFiltersDto> {
    registry.historical_filters().map_err(|e| e.to_string())
}

/// The Historical Players master list (person-centric). `goalieFilter`: `true` goalies only, `false`
/// skaters only, `null` all.
#[tauri::command]
pub fn list_historical_players(
    registry: State<'_, Registry>,
    search: String,
    goalie_filter: Option<bool>,
    limit: i64,
) -> CmdResult<Vec<HistoricalPersonSummaryDto>> {
    registry
        .list_historical_players(&search, goalie_filter, limit)
        .map_err(|e| e.to_string())
}

/// One historical person's full detail, or `null` if absent.
#[tauri::command]
pub fn historical_player(
    registry: State<'_, Registry>,
    person_id: i64,
) -> CmdResult<Option<HistoricalPersonDto>> {
    registry.historical_player(person_id).map_err(|e| e.to_string())
}

/// The Historical Teams master list for one season (season-scoped).
#[tauri::command]
pub fn list_historical_teams(
    registry: State<'_, Registry>,
    season_year: i64,
    search: String,
    limit: i64,
) -> CmdResult<Vec<HistoricalTeamSummaryDto>> {
    registry
        .list_historical_teams(season_year, &search, limit)
        .map_err(|e| e.to_string())
}

/// One team-season's detail (identity + roster), or `null` if absent.
#[tauri::command]
pub fn historical_team(
    registry: State<'_, Registry>,
    season_year: i64,
    src_team_id: i64,
) -> CmdResult<Option<HistoricalTeamDto>> {
    registry
        .historical_team(season_year, src_team_id)
        .map_err(|e| e.to_string())
}

/// The league filter options for the franchise-consolidated team browse.
#[tauri::command]
pub fn historical_team_filters(
    registry: State<'_, Registry>,
) -> CmdResult<HistoricalTeamFiltersDto> {
    registry.historical_team_filters().map_err(|e| e.to_string())
}

/// The franchise-consolidated team index (NHL franchises + standalone non-NHL teams). `league` filters
/// by league abbr (`"NHL"` → the franchises); `null` → everything.
#[tauri::command]
pub fn list_historical_franchises(
    registry: State<'_, Registry>,
    league: Option<String>,
    search: String,
    limit: i64,
) -> CmdResult<Vec<HistoricalTeamIndexEntryDto>> {
    registry
        .list_historical_franchises(league.as_deref(), &search, limit)
        .map_err(|e| e.to_string())
}

/// One franchise/standalone-team's detail for a season (`seasonYear` null → newest), or `null` if the
/// key is unknown. `kind` is `"franchise"` (key = franchise id) or `"team"` (key = `team_key`).
#[tauri::command]
pub fn historical_franchise(
    registry: State<'_, Registry>,
    kind: String,
    key: String,
    season_year: Option<i64>,
) -> CmdResult<Option<HistoricalTeamFranchiseDto>> {
    registry
        .historical_franchise(&kind, &key, season_year)
        .map_err(|e| e.to_string())
}

// ---- Formula engine (Stage 2) ---------------------------------------------

/// The packaged default formula profile.
#[tauri::command]
pub fn formula_default(registry: State<'_, Registry>) -> FormulaProfileDto {
    registry.formula_default()
}

/// The static formula catalog (EA attrs, FHM vocabulary, prebuilts) for the Formulas UI.
#[tauri::command]
pub fn formula_catalog(registry: State<'_, Registry>) -> FormulaCatalogDto {
    registry.formula_catalog()
}

/// Compute a person-season's EA ratings under the supplied profile; `null` if the person has no
/// derivable position.
#[tauri::command]
pub fn compute_ratings(
    registry: State<'_, Registry>,
    person_id: i64,
    season_year: i64,
    profile: FormulaProfileDto,
) -> CmdResult<Option<EaComputedRatingsDto>> {
    registry
        .compute_ratings(person_id, season_year, &profile)
        .map_err(|e| e.to_string())
}

/// Compute a player's cross-career EA rating compilations (Full Career + Peak 3 + per-season
/// overalls + best season) under the supplied profile; `null` if the person has no playing seasons.
#[tauri::command]
pub fn compute_career_ratings(
    registry: State<'_, Registry>,
    person_id: i64,
    profile: FormulaProfileDto,
) -> CmdResult<Option<HistoricalCareerRatingsDto>> {
    registry
        .compute_career_ratings(person_id, &profile)
        .map_err(|e| e.to_string())
}

/// The active (working) formula profile — persisted next to the corpus, falling back to the
/// packaged default when unset.
#[tauri::command]
pub fn get_active_formula(registry: State<'_, Registry>) -> CmdResult<FormulaProfileDto> {
    registry.get_active_formula().map_err(|e| e.to_string())
}

/// Persist the active (working) formula profile.
#[tauri::command]
pub fn save_active_formula(
    registry: State<'_, Registry>,
    profile: FormulaProfileDto,
) -> CmdResult<()> {
    registry.save_active_formula(&profile).map_err(|e| e.to_string())
}

/// Saved formula profile names.
#[tauri::command]
pub fn list_formula_profiles(registry: State<'_, Registry>) -> CmdResult<Vec<String>> {
    registry.list_formula_profiles().map_err(|e| e.to_string())
}

/// Load a saved formula profile by name.
#[tauri::command]
pub fn load_formula_profile(
    registry: State<'_, Registry>,
    name: String,
) -> CmdResult<FormulaProfileDto> {
    registry.load_formula_profile(&name).map_err(|e| e.to_string())
}

/// Save a formula profile under `name`.
#[tauri::command]
pub fn save_formula_profile(
    registry: State<'_, Registry>,
    name: String,
    profile: FormulaProfileDto,
) -> CmdResult<()> {
    registry
        .save_formula_profile(&name, &profile)
        .map_err(|e| e.to_string())
}

/// Delete a saved formula profile.
#[tauri::command]
pub fn delete_formula_profile(registry: State<'_, Registry>, name: String) -> CmdResult<()> {
    registry.delete_formula_profile(&name).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- assets

/// Report a title's art roots (`<install>/rendering`, `<install>/fe`) and whether both are
/// present. `complete: false` means the install is still packed — extract first.
#[tauri::command]
pub fn asset_roots(registry: State<'_, Registry>, roots: AssetRootsArgsDto) -> AssetRootsDto {
    registry.asset_roots(&roots)
}

/// Every player's art, resolved against the install.
#[tauri::command]
pub fn list_player_assets(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
) -> CmdResult<Vec<PlayerAssetsDto>> {
    registry
        .list_player_assets(project_id, &roots)
        .map_err(|e| e.to_string())
}

/// A player portrait framed for the bottom-anchored header tile (cropped to content + down to the
/// lowest solid row, dropping any faded edge). Returns a PNG data URL.
#[tauri::command]
pub fn portrait_header_preview(
    registry: State<'_, Registry>,
    path: String,
) -> CmdResult<String> {
    registry.portrait_header_preview(&path).map_err(|e| e.to_string())
}

/// The team's logo as a cropped PNG data URL for the Team header window (`null` when the title ships
/// no logo file or the team has no art id). Resolved + cropped server-side.
#[tauri::command]
pub fn team_logo(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
    team_id: u64,
) -> CmdResult<Option<String>> {
    registry
        .team_logo(project_id, &roots, team_id)
        .map_err(|e| e.to_string())
}

/// One team's outfit textures (jersey / pants / socks) resolved from the install art tree, for the
/// Uniforms surface. Each entry carries an install-relative path the texture preview/replace commands
/// take. Empty when the title ships no team-keyed outfit textures.
#[tauri::command]
pub fn team_outfits(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
    team_id: u64,
) -> CmdResult<Vec<crate::dto::OutfitTextureDto>> {
    registry
        .team_outfits(project_id, &roots, team_id)
        .map_err(|e| e.to_string())
}

/// Decode a player's head mesh (by head id) for the 3D viewer.
#[tauri::command]
pub fn head_preview(
    registry: State<'_, Registry>,
    roots: AssetRootsArgsDto,
    head_id: u64,
) -> CmdResult<HeadPreviewDto> {
    registry.head_preview(&roots, head_id).map_err(|e| e.to_string())
}

/// Decode a head mesh for the 3D viewer from already-resolved paths (the PS4
/// bespoke path — name-slug heads that a numeric id can't resolve). Works for
/// any platform; the decode dispatches on `.mcd` vs `.rx2`.
#[tauri::command]
pub fn head_preview_paths(
    registry: State<'_, Registry>,
    mesh_paths: Vec<String>,
    texture_paths: Vec<String>,
) -> CmdResult<HeadPreviewDto> {
    registry
        .head_preview_paths(mesh_paths, texture_paths)
        .map_err(|e| e.to_string())
}

/// Assemble a full player kit (head + helmet + jersey + gloves + pants + socks
/// + skates) for the 3D viewer from install-relative piece paths.
#[tauri::command]
pub fn kit_preview_paths(
    registry: State<'_, Registry>,
    install_dir: String,
    pieces: Vec<KitPieceInputDto>,
) -> CmdResult<KitPreviewDto> {
    registry
        .kit_preview_paths(&install_dir, pieces)
        .map_err(|e| e.to_string())
}

/// Assemble a team's full uniform kit (selected jersey/pants/socks colorway on
/// the shared body + a representative player's head + glove/helmet) for the 3D
/// viewer. `playerId` picks whose head/pose is shown; absent = top-overall.
#[tauri::command]
pub fn team_kit_preview(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
    team_id: u64,
    style: u32,
    variant: u32,
    player_id: Option<u64>,
    // When true, gear pieces resolve from the previewed player's own `xmja`/`fMVl`
    // selected equipment ids rather than the team defaults (player Equipment tab).
    player_gear: Option<bool>,
) -> CmdResult<KitPreviewDto> {
    registry
        .team_kit_preview(
            project_id,
            &roots,
            team_id,
            style,
            variant,
            player_id,
            player_gear.unwrap_or(false),
        )
        .map_err(|e| e.to_string())
}

/// Decode one portrait to a PNG data URL for preview.
#[tauri::command]
pub fn portrait_preview(
    registry: State<'_, Registry>,
    path: String,
    crop: bool,
) -> CmdResult<String> {
    registry.portrait_preview(&path, crop).map_err(|e| e.to_string())
}

/// Export art for the selected players (empty `bio_records` = all), named by player slug.
#[tauri::command]
pub fn export_assets(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
    out_dir: String,
    bio_records: Vec<u32>,
    options: ExportOptionsDto,
) -> CmdResult<AssetExportSummaryDto> {
    registry
        .export_assets(project_id, &roots, &out_dir, bio_records, &options)
        .map_err(|e| e.to_string())
}

/// List one directory level of the extracted install for the file browser, rooted at the install
/// dir. `rel` is a forward-slash path relative to that root (`""` = the root); resolved safely so
/// the UI can never walk outside the install.
#[tauri::command]
pub fn browse_install_files(
    registry: State<'_, Registry>,
    install_dir: String,
    rel: String,
) -> CmdResult<BrowseListingDto> {
    registry
        .browse_files(&install_dir, &rel)
        .map_err(|e| e.to_string())
}

/// Export the selected files/folders from the extracted install into `out_dir`, decoding recognized
/// assets to glTF/PNG/DDS and mirroring each source's path relative to the install.
#[tauri::command]
pub fn export_install_files(
    registry: State<'_, Registry>,
    install_dir: String,
    rel_paths: Vec<String>,
    out_dir: String,
    options: FileExportOptionsDto,
) -> CmdResult<FileExportSummaryDto> {
    registry
        .export_files(&install_dir, rel_paths, &out_dir, &options)
        .map_err(|e| e.to_string())
}

/// Write an edited texture (`source_image_path`) back into the extracted asset file at `rel_path`
/// (resolved safely under `install_dir`), dispatching on the file's container (PS4 GNF / BIGF portrait
/// / RW4 texlib). Writes a one-time `.bak` when `write_backup` is set. Pure filesystem — no project
/// needed.
#[tauri::command]
pub fn import_texture(
    registry: State<'_, Registry>,
    install_dir: String,
    rel_path: String,
    source_image_path: String,
    write_backup: bool,
    texture_index: Option<u32>,
) -> CmdResult<ImportTextureSummaryDto> {
    registry
        .import_texture(&install_dir, &rel_path, &source_image_path, write_backup, texture_index)
        .map_err(|e| e.to_string())
}

/// Write an in-memory PNG (base64) back into the extracted texture at `rel_path` — the bytes twin of
/// `import_texture`, for a UI-baked texture (e.g. the nameplate editor). One-time `.bak` on
/// `write_backup`.
#[tauri::command]
pub fn import_texture_bytes(
    registry: State<'_, Registry>,
    install_dir: String,
    rel_path: String,
    png_base64: String,
    write_backup: bool,
    texture_index: Option<u32>,
) -> CmdResult<ImportTextureSummaryDto> {
    registry
        .import_texture_bytes(&install_dir, &rel_path, &png_base64, write_backup, texture_index)
        .map_err(|e| e.to_string())
}

/// Decode the current on-disk texture at `rel_path` (under `install_dir`) to a PNG data URL for the
/// Replace preview / row thumbnail. Reports the texture's real dimensions + container; `max_edge > 0`
/// downscales for a thumbnail (`0` = full size).
#[tauri::command]
pub fn texture_preview(
    registry: State<'_, Registry>,
    install_dir: String,
    rel_path: String,
    max_edge: u32,
    texture_index: Option<u32>,
) -> CmdResult<crate::dto::TexturePreviewDto> {
    registry
        .texture_preview(&install_dir, &rel_path, max_edge, texture_index)
        .map_err(|e| e.to_string())
}

/// Decode an operator-picked source image to a PNG data URL (with its dimensions) for the Replace
/// preview's "after" side, so a size mismatch is visible before the write.
#[tauri::command]
pub fn source_image_preview(
    registry: State<'_, Registry>,
    source_image_path: String,
) -> CmdResult<crate::dto::TexturePreviewDto> {
    registry
        .source_image_preview(&source_image_path)
        .map_err(|e| e.to_string())
}

/// Export every team's jersey/pants/socks as PNGs named by the team, into `out_dir`.
#[tauri::command]
pub fn export_team_outfits(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
    out_dir: String,
) -> CmdResult<OutfitExportSummaryDto> {
    registry
        .export_team_outfits(project_id, &roots, &out_dir)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- roster builder (Phase 5.4)

/// Auto-detect EA identity matches for a corpus person against the open project — the Roster Builder
/// Match/Create toggle. Needs both an open corpus and project `projectId`.
#[tauri::command]
pub fn match_ea_player(
    registry: State<'_, Registry>,
    project_id: u32,
    person_id: i64,
) -> CmdResult<Vec<MatchCandidateDto>> {
    registry
        .match_ea_player(project_id, person_id)
        .map_err(|e| e.to_string())
}

/// Dry-run a Roster Builder cart against `projectId`: per-item outcomes, player-count delta, audit,
/// blockers, and ledger dedup flags — without mutating the session.
#[tauri::command]
pub fn import_preview(
    registry: State<'_, Registry>,
    project_id: u32,
    items: Vec<ImportItemDto>,
) -> CmdResult<ChangeSetPreviewDto> {
    registry
        .import_preview(project_id, &items)
        .map_err(|e| e.to_string())
}

/// Commit a Roster Builder cart against `projectId` — the same lowering as preview, applied as one
/// transaction / one undo entry, then ledgered.
#[tauri::command]
pub fn import_commit(
    registry: State<'_, Registry>,
    project_id: u32,
    items: Vec<ImportItemDto>,
) -> CmdResult<ImportResultDto> {
    registry
        .import_commit(project_id, &items)
        .map_err(|e| e.to_string())
}
