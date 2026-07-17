//! The Tauri command surface — one thin wrapper per command, forwarding to the [`Registry`] and
//! stringifying errors (Tauri serializes `Err(String)` to the frontend). Register these in the
//! Tauri builder via `tauri::generate_handler!` (see `src-tauri`).

use std::path::{Path, PathBuf};

use tauri::{Manager, State};

use studio_domain::FieldValue;

use crate::dto::{
    AddCareerSeasonDto, ArenaSummaryDto, ArenaViewDto, AutocorrectSummaryDto, CareerStatsDto,
    CreatePlayerDto, EaComputedRatingsDto, ExtractSummaryDto, FormulaCatalogDto, FormulaProfileDto,
    HistoricalCareerRatingsDto, HistoricalFiltersDto, HistoricalPersonDto, HistoricalPersonSummaryDto,
    HistoricalTeamDto, HistoricalTeamFiltersDto, HistoricalTeamFranchiseDto,
    HistoricalTeamIndexEntryDto, HistoricalTeamSummaryDto, InstallReportDto, IntegrityReportDto,
    LinesViewDto, OpOutcomeDto, PlayerSummaryDto, PlayerViewDto, ProjectSummaryDto, TeamSummaryDto,
    TeamViewDto,
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
/// root for EA BIGF/BIG4 archives and writes every entry out preserving its internal folder
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
