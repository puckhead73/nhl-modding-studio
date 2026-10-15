//! The Tauri command surface — one thin wrapper per command, forwarding to the [`Registry`] and
//! stringifying errors (Tauri serializes `Err(String)` to the frontend). Register these in the
//! Tauri builder via `tauri::generate_handler!` (see `src-tauri`).

use std::path::{Path, PathBuf};

use tauri::{Manager, State};

use studio_domain::FieldValue;

use crate::dto::{
    AddCareerSeasonDto, ArenaSummaryDto, ArenaViewDto, AssetExportSummaryDto, AssetRootsArgsDto,
    AssetRootsDto, AutocorrectSummaryDto, BrowseListingDto, CareerStatsDto, ChangeSetPreviewDto,
    CreatePlayerDto, EaComputedRatingsDto, EquipmentSlotAssetDto, EquipmentSlotDto,
    ExportOptionsDto, FileExportOptionsDto, FileExportSummaryDto,
    FormulaCatalogDto, FormulaProfileDto, HeadPreviewDto, HistoricalCareerRatingsDto,
    HistoricalFiltersDto, HistoricalPersonDto, HistoricalPersonSummaryDto, HistoricalTeamDto,
    HistoricalTeamFiltersDto, HistoricalTeamFranchiseDto, HistoricalTeamIndexEntryDto,
    HistoricalTeamSummaryDto, HockeyShopItemDto, HockeyShopSyncSummaryDto, ImportItemDto,
    ImportResultDto, ImportTextureSummaryDto, InstallReportDto, InstallStoreItemApplyResponseDto,
    InstallStoreItemPlanDto, InstallStoreItemRequestDto, IntegrityReportDto, KitPieceInputDto,
    KitPreviewDto, LegendsComputeSummaryDto, LegendsMemberDto, LegendsStatusDto,
    LegendsTeamSummaryDto, LinesViewDto, MatchCandidateDto, OpOutcomeDto, OutfitExportSummaryDto,
    PlayerAssetsDto, PlayerManifestSummaryDto, PlayerSummaryDto, PlayerViewDto, ProjectSummaryDto,
    RepairBlocksSummaryDto, TeamSummaryDto, TeamViewDto,
};
use crate::registry::{parse_platform, parse_role, Registry};

type CmdResult<T> = Result<T, String>;

/// Run a registry call on Tauri's **blocking thread pool** and await its answer (LEAGUE-RESP-1).
///
/// Why not `#[tauri::command(async)]` on a sync body: that attribute makes the generated wrapper
/// call the body *inline inside the async task* it spawns on the runtime — the sync work then
/// pins one of the runtime's worker threads for its whole duration (minutes for a copied-target
/// apply), which is not a blocking pool. `spawn_blocking` is: the runtime's workers stay free for
/// IPC dispatch and other commands, and however many of these commands wait on the registry's
/// mutexes, the webview's main thread is never among the waiters.
///
/// The closure gets the managed [`Registry`] through the `'static` [`tauri::AppHandle`], so no
/// borrowed `State` has to cross into the pool. A worker that panics is reported as an error
/// string like every other command failure.
///
/// **Which commands, and why only these.** Every command the shell, the Leagues screens,
/// Pending Edits, the copied-target panel and the project picker dispatch is declared
/// `pub async fn` and forwards through here; the source guard
/// `tests/command_execution_context.rs` lists them. A plain `#[tauri::command]` on a sync `fn`
/// runs on the **main thread** and holds the window for as long as it waits on the registry's
/// `inner`/`copied` mutexes — during the apply's ~32 s preflight on a real PS4 archive that was
/// the measured freeze. The remaining commands (player, team, asset, Hockey Shop, historical and
/// formula surfaces) still run sync on the main thread; each waits behind whichever registry lock
/// it needs and blocks the window for that long. They are outside this bounded package and
/// listed as remaining work in its receipt.
async fn off_main<T, F>(app: tauri::AppHandle, f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce(&Registry) -> CmdResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let registry = app.state::<Registry>();
        f(&registry)
    })
    .await
    .map_err(|e| format!("the command's worker thread ended before answering: {e}"))?
}

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

/// Release a project's workspace. Returns the acknowledged outcome — `closed`, `already_closed`,
/// `busy` (a copied-target operation still holds the session) or `failed` — so the shell only
/// disposes of its owner once the server has actually let go (LEAGUE-SMOKE-R1).
#[tauri::command]
pub async fn close_project(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<crate::dto::CloseProjectDto> {
    off_main(app, move |registry| {
        Ok(registry.close_project_outcome(project_id))
    })
    .await
}

/// Onboarding step 1: point at a root folder (RPCS3 root for PS3, the game folder — or a library
/// holding it — for PS4, the `default.xex` install dir for Xbox, the game folder for Recomp) and
/// report the resolved databases + save inventory. `save_path` locates the roster/profile saves for
/// platforms whose saves aren't beside the game (Xbox / Recomp / PS4's shadPS4 savedata); ignored
/// for PS3.
#[tauri::command]
pub async fn detect_install(
    app: tauri::AppHandle,
    game_id: String,
    platform: String,
    root: String,
    save_path: Option<String>,
) -> CmdResult<InstallReportDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    off_main(app, move |registry| {
        Ok(registry.detect_install(&game_id, platform, &root, save_path.as_deref()))
    })
    .await
}

/// Onboarding step 2: open the project from the chosen root — enforces the modding minimum
/// (`nhlng.db` + a roster save), writes the `.nhlmodproj` profile sidecar, and begins an edit
/// session over the canonical `nhlng.db`. PS4: `roster_save` is the roster save chosen from the
/// inventory (required when several are recognised).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn open_workspace(
    app: tauri::AppHandle,
    game_id: String,
    platform: String,
    root: String,
    save_path: Option<String>,
    roster_save: Option<String>,
    big_files_extracted: bool,
) -> CmdResult<ProjectSummaryDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let defs = resolve_defs(&app);
    off_main(app, move |registry| {
        registry
            .open_workspace(
                &game_id,
                platform,
                &root,
                save_path.as_deref(),
                roster_save.as_deref(),
                big_files_extracted,
                &defs.to_string_lossy(),
            )
            .map_err(|e| e.to_string())
    })
    .await
}

/// The PS4 save-data folder to suggest at onboarding: the current Windows user's
/// `%APPDATA%\shadPS4\savedata`, and whether it exists. Read-only; never scanned unless chosen.
#[tauri::command]
pub fn ps4_save_root_suggestion() -> crate::dto::SaveRootSuggestionDto {
    let path = studio_domain::ps4_default_save_root();
    crate::dto::SaveRootSuggestionDto {
        exists: path.as_deref().is_some_and(|p| p.is_dir()),
        path: path.map(|p| p.to_string_lossy().into_owned()),
    }
}

/// The bundled `defs/` under a resource directory — for the no-installer release archive that is
/// the executable's own folder — when it holds `enums/`. Receipt 43: exercised on the extracted
/// release archive (`tests/release_archive_layout.rs`).
pub fn bundled_defs(resource_dir: &Path) -> Option<PathBuf> {
    let p = resource_dir.join("defs");
    p.join("enums").is_dir().then_some(p)
}

/// About / licences (receipt 46): the packaged licence, notices and release status, read beside the
/// executable (Tauri's resource directory is the executable's folder in the archive layout).
#[tauri::command]
pub fn about_studio(app: tauri::AppHandle) -> crate::dto::AboutDto {
    let root = app.path().resource_dir().ok();
    crate::about::read(root.as_deref(), &app.package_info().version.to_string())
}

/// One licence or notice text shipped in the archive, by its archive-relative path.
#[tauri::command]
pub fn about_licence_file(app: tauri::AppHandle, path: String) -> CmdResult<String> {
    let root = app.path().resource_dir().map_err(|e| format!("Studio could not resolve the folder it runs from: {e}"))?;
    crate::about::read_file(&root, &path)
}

/// Resolve the bundled `defs/` directory: a packaged resource in release, falling back to the
/// in-repo `defs/` during development.
fn resolve_defs(app: &tauri::AppHandle) -> PathBuf {
    if let Some(p) = app.path().resource_dir().ok().and_then(|res| bundled_defs(&res)) {
        return p;
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
pub async fn project_summary(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<ProjectSummaryDto> {
    off_main(app, move |registry| {
        registry.summary(project_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn validate_project(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<IntegrityReportDto> {
    off_main(app, move |registry| {
        registry.validate(project_id).map_err(|e| e.to_string())
    })
    .await
}

/// Enable/disable the acknowledged corruption-override edit mode for a project. With it on, edits are
/// permitted on a database that fails the structural audit (unsupported — surfaced by the on-load
/// corruption prompt after the user acknowledges the risk).
#[tauri::command]
pub async fn set_edit_override(
    app: tauri::AppHandle,
    project_id: u32,
    allow: bool,
) -> CmdResult<()> {
    off_main(app, move |registry| {
        registry
            .set_edit_override(project_id, allow)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Switch which file the project's read views display: `nhlng.db` (default) or the mirrored roster
/// save. Requires an attached roster mirror. On an aligned pair writes are unaffected (they fan out
/// to both files); on a misaligned pair the workspace enters roster-only mode and edits apply
/// strictly to the roster file. Returns the refreshed project summary reflecting the new source.
#[tauri::command]
pub async fn set_display_roster(
    app: tauri::AppHandle,
    project_id: u32,
    on: bool,
) -> CmdResult<ProjectSummaryDto> {
    off_main(app, move |registry| {
        registry
            .set_display_roster(project_id, on)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Auto-correct duplicate jersey numbers on load (default reassignment logic); returns a summary of
/// the corrections applied (empty when none were needed). Corrections are staged as a pending edit.
#[tauri::command]
pub async fn autocorrect_jerseys(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<AutocorrectSummaryDto> {
    off_main(app, move |registry| {
        registry
            .autocorrect_jerseys(project_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Repair a corrupted roster-block layout (the "Corrupted roster block" audit errors) — an
/// order-only permutation of the roster slot table back into contiguous per-team blocks, staged as
/// a pending, undoable edit. Returns how many slots moved and which teams were affected.
#[tauri::command]
pub async fn repair_roster_blocks(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<RepairBlocksSummaryDto> {
    off_main(app, move |registry| {
        registry
            .repair_roster_blocks(project_id)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn list_players(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<Vec<PlayerSummaryDto>> {
    off_main(app, move |registry| registry.list_players(project_id).map_err(|e| e.to_string())).await
}

/// The free-agent pool (players occupying no roster slot — the S1 occupancy model), in the
/// master-list row shape.
#[tauri::command]
pub async fn list_free_agents(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<Vec<PlayerSummaryDto>> {
    off_main(app, move |registry| registry
        .list_free_agents(project_id)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn player_view(
    app: tauri::AppHandle,
    project_id: u32,
    player_id: u64,
) -> CmdResult<Option<PlayerViewDto>> {
    off_main(app, move |registry| registry
        .player_view(project_id, player_id)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn player_assignments(app: tauri::AppHandle, project_id: u32, player_id: u64) -> CmdResult<Vec<crate::registry::player_assignments::PlayerAssignment>> {
    off_main(app, move |registry| registry.player_assignments(project_id, player_id).map_err(|e| e.to_string())).await
}
#[tauri::command]
pub async fn set_player_assignment(app: tauri::AppHandle, project_id: u32, player_id: u64, assignment: String, choice: String) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry.set_player_assignment(project_id, player_id, &assignment, &choice).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub fn career_stats(
    registry: State<'_, Registry>,
    project_id: u32,
    player_id: u64,
) -> CmdResult<CareerStatsDto> {
    registry
        .career_stats(project_id, player_id)
        .map_err(|e| e.to_string())
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
pub async fn list_teams(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<Vec<TeamSummaryDto>> {
    off_main(app, move |registry| registry.list_teams(project_id).map_err(|e| e.to_string())).await
}

/// The pending review (baseline ↔ working), cells capped at `limit` with an exact count.
#[tauri::command]
pub async fn pending_changes(
    app: tauri::AppHandle,
    project_id: u32,
    limit: usize,
) -> CmdResult<crate::dto::PendingReviewDto> {
    off_main(app, move |registry| {
        registry
            .pending_changes(project_id, limit)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Attach the target's localization dictionary for league label resolution.
#[tauri::command]
pub async fn attach_dictionary(
    app: tauri::AppHandle,
    project_id: u32,
    path: String,
) -> CmdResult<crate::dto::DictionaryRefDto> {
    off_main(app, move |registry| {
        registry
            .attach_dictionary(project_id, &path)
            .map_err(|e| e.to_string())
    })
    .await
}

/// The loaded target's league catalog.
#[tauri::command]
pub async fn list_leagues(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<crate::dto::LeagueCatalogDto> {
    off_main(app, move |registry| {
        registry.list_leagues(project_id).map_err(|e| e.to_string())
    })
    .await
}

/// One league's detail by native code.
#[tauri::command]
pub async fn league_view(
    app: tauri::AppHandle,
    project_id: u32,
    code: u64,
) -> CmdResult<Option<crate::dto::LeagueViewDto>> {
    off_main(app, move |registry| {
        registry
            .league_view(project_id, code)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn team_view(
    app: tauri::AppHandle,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Option<TeamViewDto>> {
    off_main(app, move |registry| registry
        .team_view(project_id, team_id)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn lines_view(
    app: tauri::AppHandle,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Option<LinesViewDto>> {
    off_main(app, move |registry| registry
        .lines_view(project_id, team_id)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn team_strategies(
    app: tauri::AppHandle,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Option<crate::dto::TeamStrategiesDto>> {
    off_main(app, move |registry| registry
        .team_strategies(project_id, team_id)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub fn team_draft_picks(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Vec<crate::dto::DraftPickDto>> {
    registry
        .team_draft_picks(project_id, team_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_team_jerseys(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Vec<crate::dto::TeamJerseyDto>> {
    registry
        .list_team_jerseys(project_id, team_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_team_equipment(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Vec<crate::dto::TeamEquipmentDto>> {
    registry
        .list_team_equipment(project_id, team_id)
        .map_err(|e| e.to_string())
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
pub async fn list_arenas(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<Vec<ArenaSummaryDto>> {
    off_main(app, move |registry| {
        registry.list_arenas(project_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub fn arena_view(
    registry: State<'_, Registry>,
    project_id: u32,
    arena_id: u64,
) -> CmdResult<Option<ArenaViewDto>> {
    registry
        .arena_view(project_id, arena_id)
        .map_err(|e| e.to_string())
}

/// Stage a validated field edit. Provide exactly one of `text_value` (string fields) or `int_value`
/// (integer fields).
/// Teams 2.2 — what reassigning `team_id`'s home arena to `to_arena` would change (the cell, who
/// stays, who is already there), before the write. Read-only.
#[tauri::command]
pub fn arena_assignment_preview(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
    to_arena: u64,
) -> CmdResult<crate::dto::ArenaAssignmentPreviewDto> {
    registry
        .arena_assignment_preview(project_id, team_id, to_arena)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_field(
    app: tauri::AppHandle,
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
    off_main(app, move |registry| {
        registry
            .set_field(project_id, &table, record, &field, value)
            .map_err(|e| e.to_string())
    })
    .await
}

/// C8: write several integer cells of one record as one operation (team colours: `_r/_g/_b`).
#[tauri::command]
pub async fn set_team_roster(app: tauri::AppHandle, project_id: u32, team: u64, writes: Vec<(u32, String, i64)>) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry.set_team_roster(project_id, team, &writes).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn set_cells_int(
    app: tauri::AppHandle,
    project_id: u32,
    table: String,
    record: u32,
    writes: Vec<crate::dto::CellWriteDto>,
) -> CmdResult<OpOutcomeDto> {
    let w: Vec<(String, i64)> = writes.into_iter().map(|c| (c.field, c.value)).collect();
    off_main(app, move |registry| registry.set_cells_int(project_id, &table, record, &w).map_err(|e| e.to_string())).await
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
pub async fn move_player(
    app: tauri::AppHandle,
    project_id: u32,
    player: u64,
    from_team: u64,
    to_team: u64,
) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry
        .move_player(project_id, player, from_team, to_team)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn trade_slots(
    app: tauri::AppHandle,
    project_id: u32,
    a_team: u64,
    a_player: u64,
    b_team: u64,
    b_player: u64,
) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry
        .trade_slots(project_id, a_team, a_player, b_team, b_player)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn release_to_fa(
    app: tauri::AppHandle,
    project_id: u32,
    from_team: u64,
    player: u64,
) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry
        .release_to_fa(project_id, from_team, player)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn sign_from_fa(
    app: tauri::AppHandle,
    project_id: u32,
    player: u64,
    to_team: u64,
) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry
        .sign_from_fa(project_id, player, to_team)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn add_membership(
    app: tauri::AppHandle,
    project_id: u32,
    player: u64,
    to_team: u64,
) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry
        .add_membership(project_id, player, to_team)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn remove_membership(
    app: tauri::AppHandle,
    project_id: u32,
    from_team: u64,
    player: u64,
) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry
        .remove_membership(project_id, from_team, player)
        .map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn set_jersey_number(
    app: tauri::AppHandle,
    project_id: u32,
    team: u64,
    player: u64,
    number: u64,
) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry
        .set_jersey_number(project_id, team, player, number)
        .map_err(|e| e.to_string())).await
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
pub async fn set_dressed(
    app: tauri::AppHandle,
    project_id: u32,
    team: u64,
    player: u64,
    dressed: bool,
) -> CmdResult<OpOutcomeDto> {
    off_main(app, move |registry| registry.set_dressed(project_id, team, player, dressed).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn undo(app: tauri::AppHandle, project_id: u32) -> CmdResult<bool> {
    off_main(app, move |registry| {
        registry.undo(project_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn redo(app: tauri::AppHandle, project_id: u32) -> CmdResult<bool> {
    off_main(app, move |registry| {
        registry.redo(project_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn save_project(
    app: tauri::AppHandle,
    project_id: u32,
    out_path: String,
) -> CmdResult<u64> {
    off_main(app, move |registry| {
        registry
            .save(project_id, &out_path)
            .map_err(|e| e.to_string())
    })
    .await
}

/// In-place dual save: write `nhlng.db` and (when attached) the roster save back to the paths the
/// project was opened from. Returns the number of files written.
#[tauri::command]
pub async fn save_workspace(app: tauri::AppHandle, project_id: u32) -> CmdResult<u64> {
    off_main(app, move |registry| {
        registry
            .save_workspace(project_id)
            .map_err(|e| e.to_string())
    })
    .await
}

// ---- Historical (FHM) corpus ----------------------------------------------

/// Open an FHM historical corpus data pack (read-only). Returns the person count. Call once before
/// any other historical command; a different path replaces the open corpus.
#[tauri::command]
pub fn open_corpus(app: tauri::AppHandle, registry: State<'_, Registry>, path: String) -> CmdResult<u64> {
    let root = app.path().resource_dir().map_err(|e| e.to_string())?;
    let path = resolve_corpus_path(&root, &path)?;
    registry.open_corpus(&path.to_string_lossy()).map_err(|e| e.to_string())
}

/// Empty selection uses the portable data pack beside Studio, independent of the working directory.
pub fn resolve_corpus_path(resource_dir: &Path, selected: &str) -> CmdResult<PathBuf> {
    if !selected.trim().is_empty() {
        return Ok(PathBuf::from(selected.trim()));
    }
    let path = resource_dir.join("corpus").join("historical.db");
    if !path.is_file() {
        return Err("The bundled historical database is missing. Extract the whole playtester archive, keeping corpus/historical.db beside Studio, or select another historical database.".into());
    }
    Ok(path)
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
    registry
        .historical_player(person_id)
        .map_err(|e| e.to_string())
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
    registry
        .historical_team_filters()
        .map_err(|e| e.to_string())
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
    registry
        .save_active_formula(&profile)
        .map_err(|e| e.to_string())
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
    registry
        .load_formula_profile(&name)
        .map_err(|e| e.to_string())
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
    registry
        .delete_formula_profile(&name)
        .map_err(|e| e.to_string())
}

/// One player's Legends composite computed live under `profile` — the Formulas live preview's
/// "Legends" option. Bypasses the rating cache so unsaved weight edits show through.
#[tauri::command]
pub fn compute_legends_preview(
    registry: State<'_, Registry>,
    person_id: i64,
    profile: FormulaProfileDto,
) -> CmdResult<Option<EaComputedRatingsDto>> {
    registry
        .compute_legends_preview(person_id, &profile)
        .map_err(|e| e.to_string())
}

/// Write a CSV of every player installed in the loaded database — identity, export slug, team,
/// league, and whether a portrait / head mesh resolves on disk. Needs the project's install tree.
#[tauri::command]
pub fn export_player_manifest(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
    out_path: String,
) -> CmdResult<PlayerManifestSummaryDto> {
    registry
        .export_player_manifest(project_id, &roots, &out_path)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- legends

/// Where the Legends precompute stands for the active formula profile (drives the Formulas tab and
/// the app-level pending-calculation banner).
#[tauri::command]
pub fn legends_status(registry: State<'_, Registry>) -> CmdResult<LegendsStatusDto> {
    registry.legends_status().map_err(|e| e.to_string())
}

/// Switch the active formula profile. A profile whose Legends run is missing or stale comes back
/// with the calculation still owed.
#[tauri::command]
pub fn set_active_profile(registry: State<'_, Registry>, name: String) -> CmdResult<()> {
    registry
        .set_active_profile(&name)
        .map_err(|e| e.to_string())
}

/// Record that a Legends calculation is owed (`pending`) or that the user has acknowledged going
/// without it (`dismissed`).
#[tauri::command]
pub fn set_legends_pending(
    registry: State<'_, Registry>,
    pending: bool,
    dismissed: bool,
) -> CmdResult<()> {
    registry
        .set_legends_pending(pending, dismissed)
        .map_err(|e| e.to_string())
}

/// Run the Legends precompute for the active profile — a full-corpus pass, fanned out across cores,
/// whose results land in the `legends_cache.db` sidecar. Clears the pending flag on success.
#[tauri::command]
pub fn compute_legends_ratings(
    registry: State<'_, Registry>,
) -> CmdResult<LegendsComputeSummaryDto> {
    registry
        .compute_legends_ratings()
        .map_err(|e| e.to_string())
}

/// Every built Legends team for the active profile.
#[tauri::command]
pub fn legends_teams(registry: State<'_, Registry>) -> CmdResult<Vec<LegendsTeamSummaryDto>> {
    registry.legends_teams().map_err(|e| e.to_string())
}

/// One Legends team's roster. `teamKey` is `franchise:<id>` or a residual-team key.
#[tauri::command]
pub fn legends_roster(
    registry: State<'_, Registry>,
    team_key: String,
) -> CmdResult<Vec<LegendsMemberDto>> {
    registry
        .legends_roster(&team_key)
        .map_err(|e| e.to_string())
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
pub fn portrait_header_preview(registry: State<'_, Registry>, path: String) -> CmdResult<String> {
    registry
        .portrait_header_preview(&path)
        .map_err(|e| e.to_string())
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

/// Phase 2.4 (C9): a team's logo families on the open title (loose paths + the archive the console
/// reads), for the Team logo card's preview / replace actions.
#[tauri::command]
pub fn team_logo_set(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
    install_root: Option<String>,
) -> CmdResult<Option<crate::dto::TeamLogoSetDto>> {
    registry
        .team_logo_set(project_id, team_id, install_root.as_deref())
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
    include_ambiguous: Option<bool>,
) -> CmdResult<Vec<crate::dto::OutfitTextureDto>> {
    registry
        .team_outfits_with_candidates(project_id, &roots, team_id, include_ambiguous.unwrap_or(false))
        .map_err(|e| e.to_string())
}

/// Phase 2.6 (22 Sep 2026): a team's kits joined to their textures under the title's measured
/// contract — per-component evidence, the shipped spare candidates, and why re-pointing is not
/// offered. Read-only.
#[tauri::command]
pub fn team_kits(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
    team_id: u64,
) -> CmdResult<crate::dto::TeamKitsDto> {
    registry
        .team_kits(project_id, &roots, team_id)
        .map_err(|e| e.to_string())
}

/// Phase 2.5 (22 Sep 2026): an arena's shipped assets — joined where measured, listed elsewhere.
/// Read-only.
#[tauri::command]
pub fn arena_assets(
    app: tauri::AppHandle,
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
    arena_id: u64,
) -> CmdResult<Option<crate::dto::ArenaAssetsDto>> {
    let defs = resolve_defs(&app);
    registry
        .arena_assets_with_defs(project_id, &roots, arena_id, Some(&defs))
        .map_err(|e| e.to_string())
}

/// Presentation 5.2 (receipt 49): the arena's venue art (read-only gallery).
#[tauri::command]
pub async fn arena_venue_art(app: tauri::AppHandle, project_id: u32, roots: AssetRootsArgsDto, arena_id: u64) -> CmdResult<Option<crate::dto::VenueArtDto>> {
    off_main(app, move |registry| registry.arena_venue_art(project_id, &roots, arena_id).map_err(|e| e.to_string())).await
}

/// One venue raster as a PNG preview (the listed file, unchanged).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn arena_venue_raster(
    app: tauri::AppHandle,
    project_id: u32,
    roots: AssetRootsArgsDto,
    arena_id: u64,
    file: String,
    sha256: String,
    ordinal: u32,
    max_edge: u32,
) -> CmdResult<crate::dto::VenueRasterPreviewDto> {
    off_main(app, move |registry| registry.arena_venue_raster(project_id, &roots, arena_id, &file, &sha256, ordinal, max_edge).map_err(|e| e.to_string())).await
}

/// Phase 3 (22 Sep 2026): every player's portrait / head containment — ids, the `hasportrait` flag
/// as stored, what the install ships, the unreferenced pools. Read-only.
#[tauri::command]
pub fn player_containment(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
) -> CmdResult<crate::dto::PlayerContainmentViewDto> {
    registry
        .player_containment(project_id, &roots)
        .map_err(|e| e.to_string())
}

/// Players 3.4 (receipt 38): plan (`apply` false) or perform (`apply` true) the reassignment of a
/// player's portrait / head (and, receipt 66, PS4 NHL 17–21 hair) to another asset the attached tree ships; refused plans
/// write nothing. `target` is signed so a negative id is refused by the plan, not by deserialization.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn player_reassign(app: tauri::AppHandle, project_id: u32, roots: AssetRootsArgsDto, bio_record: u32, kind: String, target: i64, apply: bool, reviewed: Option<String>) -> CmdResult<crate::dto::PlayerReassignPlanDto> {
    // Receipt 63: PS4 heads read the vault (off the main thread); an apply names the fingerprint it reviewed.
    let defs = resolve_defs(&app);
    off_main(app, move |registry| registry.player_reassign(project_id, &roots, bio_record, &kind, target, apply, reviewed.as_deref(), &defs).map_err(|e| e.to_string())).await
}

/// Misc 5.4 (22 Sep 2026): the PS4 song catalogue listing (selectable songs, not the team's song).
#[tauri::command]
pub fn song_catalogue(registry: State<'_, Registry>, project_id: u32, install_root: String, query: Option<String>, limit: Option<usize>) -> CmdResult<crate::dto::SongCatalogueDto> {
    registry
        .song_catalogue(project_id, &install_root, query.as_deref().unwrap_or(""), limit.unwrap_or(200))
        .map_err(|e| e.to_string())
}

/// Player ids occupying a team's `eGlu` roster block — the roster for membership-only teams
/// (Winter Classic / all-star / national), whose players keep another main team.
#[tauri::command]
pub fn team_roster_ids(
    registry: State<'_, Registry>,
    project_id: u32,
    team_id: u64,
) -> CmdResult<Vec<u64>> {
    registry
        .team_roster_ids(project_id, team_id)
        .map_err(|e| e.to_string())
}

/// Clone an `xjgw` jersey row into a new kit on a team — the DB half of "Add New Jersey"
/// (staged as a pending, undoable edit). `jersey_index: None` picks the smallest unused per-team
/// index.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn clone_jersey(
    registry: State<'_, Registry>,
    project_id: u32,
    source_record: u32,
    team: u64,
    style: u64,
    art_id: u64,
    variant: u64,
    jersey_index: Option<u64>,
) -> CmdResult<crate::dto::CloneJerseyResultDto> {
    registry
        .clone_jersey(
            project_id,
            source_record,
            team,
            style,
            art_id,
            variant,
            jersey_index,
        )
        .map_err(|e| e.to_string())
}

/// The in-game jersey-select thumbnail for a kit (FE tree, with the game's custom fallbacks).
#[tauri::command]
pub fn team_jersey_thumbnail(
    registry: State<'_, Registry>,
    roots: AssetRootsArgsDto,
    style: u32,
    art_id: u32,
    variant: u32,
) -> CmdResult<crate::dto::JerseyThumbnailDto> {
    registry
        .team_jersey_thumbnail(&roots, style, art_id, variant)
        .map_err(|e| e.to_string())
}

/// Clone a kit's texture files onto a new asset id — the file half of "Add New Jersey".
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn clone_kit_textures(
    registry: State<'_, Registry>,
    roots: AssetRootsArgsDto,
    src_style: u32,
    src_art_id: u32,
    src_variant: u32,
    dst_style: u32,
    dst_art_id: u32,
    dst_variant: u32,
) -> CmdResult<crate::dto::CloneKitTexturesSummaryDto> {
    registry
        .clone_kit_textures(
            &roots,
            src_style,
            src_art_id,
            src_variant,
            dst_style,
            dst_art_id,
            dst_variant,
        )
        .map_err(|e| e.to_string())
}

/// Decode a player's head mesh (by head id) for the 3D viewer.
#[tauri::command]
pub fn head_preview(
    registry: State<'_, Registry>,
    roots: AssetRootsArgsDto,
    head_id: u64,
) -> CmdResult<HeadPreviewDto> {
    registry
        .head_preview(&roots, head_id)
        .map_err(|e| e.to_string())
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
// The parameter list is the IPC contract the frontend calls with; collapsing it into a struct
// would change the command's public shape and its generated bindings.
#[allow(clippy::too_many_arguments)]
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

/// The Hockey Shop slot catalog (Stick / Gloves / Helmet / Skates + goalie gear).
#[tauri::command]
pub fn equipment_slots(registry: State<'_, Registry>) -> CmdResult<Vec<EquipmentSlotDto>> {
    Ok(registry.equipment_slots())
}

/// Scan one Hockey Shop slot's render folder → equipment ids and their files, with each id's model
/// name resolved from the bundled enum vocabularies.
#[tauri::command]
pub fn list_equipment_slot_assets(
    app: tauri::AppHandle,
    registry: State<'_, Registry>,
    roots: AssetRootsArgsDto,
    slot: String,
    game: Option<String>,
) -> CmdResult<Vec<EquipmentSlotAssetDto>> {
    let defs = resolve_defs(&app);
    registry
        .list_equipment_slot_assets(&roots, &slot, game.as_deref(), &defs)
        .map_err(|e| e.to_string())
}

/// Phase 4: a player's equipment ids checked against the install's rendering tree.
#[tauri::command]
pub fn player_equipment_files(app: tauri::AppHandle, registry: State<'_, Registry>, project_id: u32, player_id: u64, roots: AssetRootsArgsDto) -> CmdResult<Vec<crate::dto::EquipmentFileCheckDto>> {
    let defs = resolve_defs(&app);
    registry.player_equipment_files(project_id, player_id, &roots, &defs).map_err(|e| e.to_string())
}

/// Equipment 4.5 G2 (receipt 42): a team's goalie mask paints with the gen7 file check.
#[tauri::command]
pub fn team_goalie_masks(app: tauri::AppHandle, registry: State<'_, Registry>, project_id: u32, team_id: u64, roots: AssetRootsArgsDto) -> CmdResult<crate::dto::TeamGoalieMasksDto> {
    let defs = resolve_defs(&app);
    registry.team_goalie_masks_with_defs(project_id, team_id, &roots, Some(&defs)).map_err(|e| e.to_string())
}

/// Equipment 4.1–4.4 (receipt 52): a PS4 skater's gear against the title's own Creation Zone lists (read-only).
#[tauri::command]
pub async fn ps4_skater_gear(app: tauri::AppHandle, project_id: u32, player_id: u64, roots: AssetRootsArgsDto) -> CmdResult<crate::dto::Ps4SkaterGearDto> {
    let defs = resolve_defs(&app);
    off_main(app, move |registry| registry.ps4_skater_gear(project_id, &roots, player_id, &defs).map_err(|e| e.to_string())).await
}

/// A Creation Zone equipment thumbnail as a PNG preview.
#[tauri::command]
pub async fn ps4_gear_thumbnail(app: tauri::AppHandle, roots: AssetRootsArgsDto, archive: String, member: String) -> CmdResult<crate::dto::VenueRasterPreviewDto> {
    off_main(app, move |registry| registry.ps4_gear_thumbnail(&roots, &archive, &member).map_err(|e| e.to_string())).await
}

/// Equipment 4.5 G3 (receipt 51): a PS4 goalie's mask pair against the title's own vault catalogue (read-only).
#[tauri::command]
pub async fn ps4_goalie_mask_pair(app: tauri::AppHandle, project_id: u32, player_id: u64, roots: AssetRootsArgsDto) -> CmdResult<crate::dto::Ps4GoalieMaskPairDto> {
    let defs = resolve_defs(&app);
    off_main(app, move |registry| registry.ps4_goalie_mask_pair(project_id, &roots, player_id, &defs).map_err(|e| e.to_string())).await
}


/// Phase 2.5 — the arena's venue-parameter inventory.
#[tauri::command]
pub fn arena_venue_view(app: tauri::AppHandle, registry: State<'_, Registry>, project_id: u32, arena_id: u64, install_root: Option<String>) -> CmdResult<Option<crate::dto::ArenaVenueDto>> {
    let defs = resolve_defs(&app);
    let root = install_root.as_deref().map(std::path::Path::new);
    registry.arena_venue_view(project_id, arena_id, &defs, root).map_err(|e| e.to_string())
}

/// Assemble the 3D preview for one Hockey Shop equipment item (mesh + colour map).
#[tauri::command]
pub fn equipment_item_preview(
    registry: State<'_, Registry>,
    roots: AssetRootsArgsDto,
    slot: String,
    id: u32,
) -> CmdResult<KitPreviewDto> {
    registry
        .equipment_item_preview(&roots, &slot, id)
        .map_err(|e| e.to_string())
}

/// List a slot's Hockey Shop store items (disk-backed PNG library).
#[tauri::command]
pub fn hockey_shop_list(
    registry: State<'_, Registry>,
    store_root: String,
    slot: String,
) -> CmdResult<Vec<HockeyShopItemDto>> {
    registry
        .hockey_shop_list(&store_root, &slot)
        .map_err(|e| e.to_string())
}

/// Extract a representative texture per game equipment id into the store (idempotent).
#[tauri::command]
pub fn hockey_shop_sync(
    app: tauri::AppHandle,
    registry: State<'_, Registry>,
    project_id: Option<u32>,
    roots: AssetRootsArgsDto,
    store_root: String,
    slot: String,
    game: Option<String>,
    game_id: Option<String>,
) -> CmdResult<HockeyShopSyncSummaryDto> {
    let defs = resolve_defs(&app);
    registry
        .hockey_shop_sync(project_id, &roots, &store_root, &slot, &defs, game.as_deref(), game_id.as_deref())
        .map_err(|e| e.to_string())
}

/// Ingest an operator PNG as a new store item.
#[tauri::command]
pub fn hockey_shop_import(
    registry: State<'_, Registry>,
    store_root: String,
    slot: String,
    name: String,
    author: String,
    image_base64: String,
) -> CmdResult<HockeyShopItemDto> {
    registry
        .hockey_shop_import(&store_root, &slot, &name, &author, &image_base64)
        .map_err(|e| e.to_string())
}

/// Remove a store item (manifest entry + its PNG).
#[tauri::command]
pub fn hockey_shop_delete(
    registry: State<'_, Registry>,
    store_root: String,
    slot: String,
    entry_id: String,
) -> CmdResult<()> {
    registry
        .hockey_shop_delete(&store_root, &slot, &entry_id)
        .map_err(|e| e.to_string())
}

/// One store item's PNG as a data URL for the preview tile.
#[tauri::command]
pub fn hockey_shop_load_texture(
    registry: State<'_, Registry>,
    store_root: String,
    slot: String,
    entry_id: String,
) -> CmdResult<String> {
    registry
        .hockey_shop_load_texture(&store_root, &slot, &entry_id)
        .map_err(|e| e.to_string())
}

/// Read-only plan for installing a store item into the game at a target id.
#[tauri::command]
pub fn hockey_shop_install_plan(
    registry: State<'_, Registry>,
    project_id: Option<u32>,
    roots: AssetRootsArgsDto,
    store_root: String,
    request: InstallStoreItemRequestDto,
) -> CmdResult<InstallStoreItemPlanDto> {
    registry
        .hockey_shop_install_plan(project_id, &roots, &store_root, &request)
        .map_err(|e| e.to_string())
}

/// Apply an install — clones a donor (create mode) + splices the store texture into
/// the target colour map. Writes `.rx2` files into the install (a `.bak` first).
#[tauri::command]
pub fn hockey_shop_install_apply(
    registry: State<'_, Registry>,
    project_id: Option<u32>,
    roots: AssetRootsArgsDto,
    store_root: String,
    request: InstallStoreItemRequestDto,
) -> CmdResult<InstallStoreItemApplyResponseDto> {
    registry
        .hockey_shop_install_apply(project_id, &roots, &store_root, &request)
        .map_err(|e| e.to_string())
}

/// Decode one portrait to a PNG data URL for preview.
#[tauri::command]
pub fn portrait_preview(
    registry: State<'_, Registry>,
    path: String,
    crop: bool,
) -> CmdResult<String> {
    registry
        .portrait_preview(&path, crop)
        .map_err(|e| e.to_string())
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

/// Build the appearance catalog (every head + hair + facial hair, players and
/// non-players) with a `catalog.json` manifest, into `out_dir`.
#[tauri::command]
pub fn export_head_catalog(
    registry: State<'_, Registry>,
    project_id: u32,
    roots: AssetRootsArgsDto,
    out_dir: String,
    textures: bool,
) -> CmdResult<crate::dto::HeadCatalogSummaryDto> {
    registry
        .export_head_catalog(project_id, &roots, &out_dir, textures)
        .map_err(|e| e.to_string())
}

/// Read a built appearance catalog's `catalog.json` back for the browser.
#[tauri::command]
pub fn read_catalog_manifest(
    registry: State<'_, Registry>,
    dir: String,
) -> CmdResult<crate::dto::CatalogManifestDto> {
    registry
        .read_catalog_manifest(&dir)
        .map_err(|e| e.to_string())
}

/// Decode one catalog asset (its source container filtered to the entry's
/// submeshes) for the 3D viewer.
#[tauri::command]
pub fn catalog_entry_preview(
    registry: State<'_, Registry>,
    source: String,
    submeshes: Vec<String>,
    texture_sources: Option<Vec<String>>,
) -> CmdResult<HeadPreviewDto> {
    registry
        .catalog_entry_preview(source, submeshes, texture_sources)
        .map_err(|e| e.to_string())
}

/// Read a PNG off disk as a `data:image/png;base64,…` URL (catalog texture thumbs).
#[tauri::command]
pub fn read_png_data_url(registry: State<'_, Registry>, path: String) -> CmdResult<String> {
    registry.read_png_data_url(&path).map_err(|e| e.to_string())
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

/// Write the reviewed source image back into the extracted asset file at `rel_path` (resolved
/// safely under `install_dir`), dispatching on the file's container (PS4 GNF / BIGF portrait /
/// RW4 texlib). `review_id` names the `texture_replace_plan` the operator saw; the write refuses
/// before any backup or mutation when the target's bytes, the source, the raster, the selection or
/// the policy differ from that review. Writes a one-time `.bak` when `write_backup` is set. Pure
/// filesystem — no project needed.
#[tauri::command]
pub fn import_texture(
    registry: State<'_, Registry>,
    install_dir: String,
    rel_path: String,
    source_image_path: String,
    write_backup: bool,
    texture_index: Option<u32>,
    review_id: u64,
) -> CmdResult<ImportTextureSummaryDto> {
    registry
        .import_texture(
            &install_dir,
            &rel_path,
            &source_image_path,
            write_backup,
            texture_index,
            review_id,
        )
        .map_err(|e| e.to_string())
}

/// Write the UI-baked PNG held by review `review_id` (from `texture_replace_plan_bytes`) back into
/// the extracted texture at `rel_path` — the bytes twin of `import_texture`, for the nameplate
/// editor. Same drift refusals; one-time `.bak` on `write_backup`.
#[tauri::command]
pub fn import_texture_bytes(
    registry: State<'_, Registry>,
    install_dir: String,
    rel_path: String,
    write_backup: bool,
    texture_index: Option<u32>,
    review_id: u64,
) -> CmdResult<ImportTextureSummaryDto> {
    registry
        .import_texture_bytes(&install_dir, &rel_path, write_backup, texture_index, review_id)
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

/// Preview a texture that lives inside an `EB\0\3` render archive (PS3 NHL 12 / 13 kit members):
/// the archive at `archive_rel_path` (under `install_dir`), the member by exact path. Read-only.
#[tauri::command]
pub fn archive_member_texture_preview(
    registry: State<'_, Registry>,
    install_dir: String,
    archive_rel_path: String,
    member_path: String,
    max_edge: u32,
    texture_index: Option<u32>,
) -> CmdResult<crate::dto::TexturePreviewDto> {
    registry
        .archive_member_texture_preview(&install_dir, &archive_rel_path, &member_path, max_edge, texture_index)
        .map_err(|e| e.to_string())
}

/// What a Replace… of `rel_path` with the image at `source_image_path` would do — the refusal or
/// the partial-replacement disclosure, before Apply. Reads the target and the source; writes
/// nothing; holds the answer as a review the write must name.
#[tauri::command]
pub fn texture_replace_plan(
    registry: State<'_, Registry>,
    install_dir: String,
    rel_path: String,
    source_image_path: String,
    texture_index: Option<u32>,
) -> CmdResult<crate::dto::TextureReplacePlanDto> {
    registry
        .texture_replace_plan(&install_dir, &rel_path, &source_image_path, texture_index)
        .map_err(|e| e.to_string())
}

/// `texture_replace_plan` for a PNG the UI baked (base64): the decoded image is held in the review,
/// and `import_texture_bytes` writes exactly that image.
#[tauri::command]
pub fn texture_replace_plan_bytes(
    registry: State<'_, Registry>,
    install_dir: String,
    rel_path: String,
    png_base64: String,
    texture_index: Option<u32>,
) -> CmdResult<crate::dto::TextureReplacePlanDto> {
    registry
        .texture_replace_plan_bytes(&install_dir, &rel_path, &png_base64, texture_index)
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

// ---- S2-C copied-target transactions ---------------------------------------------------------
//
// Save a project's sessions into an explicitly selected, marked **copied-target** root through the
// shared transaction engine: review (prepare) → apply, with status / recover / restore / discard /
// cancel / reload. Never writes a live game tree; the existing `save_workspace` is unchanged.

/// The marked copied-target root the project's files sit under, if any.
#[tauri::command]
pub async fn copied_target_root_for(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<crate::dto::CopiedTargetRootDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_root_for(project_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Create a new, empty, marked copied-target root (refused if the path exists).
#[tauri::command]
pub async fn copied_target_create_root(app: tauri::AppHandle, path: String) -> CmdResult<String> {
    off_main(app, move |registry| {
        registry
            .copied_target_create_root(&path)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Package the project's files (and the save-index headers the copied tree newly requires) and
/// prepare them for `root`. Returns a server-owned handle for `copied_target_apply`.
///
/// **Execution context (LEAGUE-SMOKE-1, reworked by LEAGUE-RESP-1).** The copied-target
/// review/apply/restore/recover/reload commands stage, hash and rename whole files — minutes on a
/// real PS4 archive. LEAGUE-SMOKE-1 declared them `#[tauri::command(async)]` so a cancel no
/// longer queued behind the apply on the main thread; that attribute, on a sync body, runs the
/// body inline on a runtime worker. They now go through [`off_main`] (the blocking pool), as do
/// the status, cancel, close and ordinary league/pending commands, so **no** command on these
/// screens waits for a registry lock on the main thread.
///
/// **Lock constraints (what the registry now guarantees).** The apply's binding checks and
/// reservation are one short `inner` critical section; its whole-file identity verification and
/// the engine run hold only the reservation. The review packages the sessions under `inner` (a
/// few seconds, the same order as an ordinary edit) and then stages the archive and prepares the
/// set under a *review reservation*, never `inner`. Status, cancel and close therefore answer
/// promptly during both; a competing edit/save/close on a reserved project is refused at once
/// with the holder named rather than waiting. See the *Lock scope* section of the registry's
/// copied-target module.
#[tauri::command]
pub async fn copied_target_review(
    app: tauri::AppHandle,
    project_id: u32,
    root: String,
) -> CmdResult<crate::dto::CopiedTargetReviewDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_review(project_id, &root)
            .map_err(|e| e.to_string())
    })
    .await
}

/// As `copied_target_review`, also replacing one existing raw member of a copied archive with
/// the packaged primary database (PS4 `inxp_00.big` / `nhl_game.db`).
#[tauri::command]
pub async fn copied_target_review_archive_member(
    app: tauri::AppHandle,
    project_id: u32,
    root: String,
    archive_relative: String,
    member_path: String,
) -> CmdResult<crate::dto::CopiedTargetReviewDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_review_archive_member(project_id, &root, &archive_relative, &member_path)
            .map_err(|e| e.to_string())
    })
    .await
}

/// As `copied_target_review`, replacing one or more existing raw archive members with plan
/// artifacts (`primary` database, or the pending `dictionary` rename).
#[tauri::command]
pub async fn copied_target_review_archive_members(
    app: tauri::AppHandle,
    project_id: u32,
    root: String,
    specs: Vec<crate::dto::ArchiveMemberSpecDto>,
) -> CmdResult<crate::dto::CopiedTargetReviewDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_review_archive_members(project_id, &root, specs)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn dictionary_texts(app: tauri::AppHandle, project_id: u32, keys: Vec<String>) -> CmdResult<Vec<Option<String>>> {
    off_main(app, move |registry| registry.dictionary_texts(project_id, &keys).map_err(|e| e.to_string())).await
}

/// Preview a dictionary rename (LOC-W1): `key` would show `donor`'s text. Read-only.
#[tauri::command]
pub async fn dictionary_rename_preview(
    app: tauri::AppHandle,
    project_id: u32,
    key: String,
    donor: String,
) -> CmdResult<crate::dto::DictionaryRepointPreviewDto> {
    off_main(app, move |registry| {
        registry
            .dictionary_rename_preview(project_id, &key, &donor)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Hold a dictionary rename as the project's pending dictionary change.
#[tauri::command]
pub async fn dictionary_rename_set(
    app: tauri::AppHandle,
    project_id: u32,
    key: String,
    donor: String,
) -> CmdResult<crate::dto::DictionaryRepointPreviewDto> {
    off_main(app, move |registry| {
        registry
            .dictionary_rename_set(project_id, &key, &donor)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Drop the pending dictionary rename.
#[tauri::command]
pub async fn dictionary_rename_clear(app: tauri::AppHandle, project_id: u32) -> CmdResult<bool> {
    off_main(app, move |registry| {
        registry
            .dictionary_rename_clear(project_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Deliberately detach the attached dictionary and any pending rename (R1 F2).
#[tauri::command]
pub async fn detach_dictionary(app: tauri::AppHandle, project_id: u32) -> CmdResult<bool> {
    off_main(app, move |registry| {
        registry
            .detach_dictionary(project_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Apply a reviewed handle. Refuses an edited workspace, a changed destination or input, a stale
/// or foreign handle; on commit the sessions adopt the installed files as their baselines.
#[tauri::command]
pub async fn copied_target_apply(
    app: tauri::AppHandle,
    project_id: u32,
    handle: u32,
) -> CmdResult<crate::dto::CopiedTargetApplyDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_apply(project_id, handle)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Ask a prepared or running apply to stop at its next safe point.
#[tauri::command]
pub async fn copied_target_cancel(app: tauri::AppHandle, handle: u32) -> CmdResult<bool> {
    off_main(app, move |registry| {
        registry
            .copied_target_cancel(handle)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Drop a prepared handle without applying it (frees the root).
#[tauri::command]
pub async fn copied_target_discard(
    app: tauri::AppHandle,
    handle: u32,
) -> CmdResult<crate::dto::CopiedTargetStatusDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_discard(handle)
            .map_err(|e| e.to_string())
    })
    .await
}

/// The root's transactions (from its journals), pending handles, and what to do next.
#[tauri::command]
pub async fn copied_target_status(
    app: tauri::AppHandle,
    root: String,
) -> CmdResult<crate::dto::CopiedTargetStatusDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_status(&root)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Deterministic recovery of every interrupted transaction under the root; refreshes open
/// workspaces whose files changed (clean sessions reload, dirty ones keep their edits).
#[tauri::command]
pub async fn copied_target_recover(
    app: tauri::AppHandle,
    root: String,
) -> CmdResult<crate::dto::CopiedTargetRecoveryDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_recover(&root)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Put a committed transaction's exact originals back; refreshes open workspaces likewise.
#[tauri::command]
pub async fn copied_target_restore(
    app: tauri::AppHandle,
    root: String,
    txn_id: String,
) -> CmdResult<crate::dto::CopiedTargetRestoreDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_restore(&root, &txn_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Reload a project's sessions from disk after a restore/recovery left the baseline stale.
/// Refuses unsaved edits unless `discard_edits` is set.
#[tauri::command]
pub async fn copied_target_reload(
    app: tauri::AppHandle,
    project_id: u32,
    discard_edits: bool,
) -> CmdResult<crate::dto::ProjectBaselineDto> {
    off_main(app, move |registry| {
        registry
            .copied_target_reload(project_id, discard_edits)
            .map_err(|e| e.to_string())
    })
    .await
}

// ---------------------------------------------------------------------------------------------
// Xbox 360 disc images (p1.1-xbox360-iso-member-route)
// ---------------------------------------------------------------------------------------------

/// Resolve the title's actual metadata name from the baseline inventory. This handles
/// NHL07NG-meta.xml and the packaged baselines/files layout as well as development anchors.
/// A caller-supplied path always wins.
fn resolve_iso_meta(
    app: &tauri::AppHandle,
    game_id: &str,
    platform: &str,
    given: Option<String>,
) -> Result<String, String> {
    if let Some(p) = given.filter(|p| !p.trim().is_empty()) {
        return Ok(p);
    }
    iso_meta_from_store(&resolve_baselines(app)?, game_id, platform)
}

fn iso_meta_from_store(store: &studio_domain::BaselineStore, game_id: &str, platform: &str) -> Result<String, String> {
    let platform = parse_platform(platform).map_err(|e| e.to_string())?;
    let target = store.target(game_id, platform).ok_or_else(|| format!("no bundled metadata target for {game_id} on {platform:?}"))?;
    let meta = target.component(studio_domain::baselines::ROLE_META)
        .ok_or_else(|| format!("no bundled metadata for {game_id}; supply its metadata XML explicitly"))?;
    // Refuse missing or changed schemas instead of silently opening with a different title's schema.
    store.read_verified(meta).map_err(|e| e.to_string())?;
    Ok(store.component_path(meta).to_string_lossy().into_owned())
}

// ---- Save Editor ------------------------------------------------------------------------------

/// Where Save Editor keeps the originals it copies before a session first overwrites a file:
/// under the app data directory, never beside the save (an extra file in a console save folder
/// can upset the emulator or console that owns it).
fn resolve_save_backups(app: &tauri::AppHandle) -> CmdResult<PathBuf> {
    let base = app.path().app_data_dir().map_err(|e| format!("no application data directory: {e}"))?;
    Ok(base.join("save-editor-backups"))
}

/// Save Editor: identify a chosen save container or extracted database — its format and kind,
/// the bundled game targets that describe it, and whether it may be opened. Reads only.
#[tauri::command]
pub async fn save_editor_identify(
    app: tauri::AppHandle,
    path: String,
) -> CmdResult<crate::dto::SaveFileIdentityDto> {
    let store = resolve_baselines(&app)?;
    off_main(app, move |registry| {
        registry.save_editor_identify(&path, &store).map_err(|e| e.to_string())
    })
    .await
}

/// Save Editor: open a file as its own edit session under the chosen game and platform. A file
/// that needs acknowledgement (a franchise or season save) opens only with `acknowledged`;
/// `meta_override` is the advanced route for a file no bundled definition describes.
#[tauri::command]
pub async fn save_editor_open(
    app: tauri::AppHandle,
    path: String,
    game_id: String,
    platform: String,
    acknowledged: bool,
    meta_override: Option<String>,
) -> CmdResult<crate::dto::SaveEditorOpenDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let store = resolve_baselines(&app)?;
    let defs = resolve_defs(&app);
    let backups = resolve_save_backups(&app)?;
    off_main(app, move |registry| {
        registry
            .save_editor_open(
                &path,
                &game_id,
                platform,
                acknowledged,
                meta_override.as_deref(),
                &store,
                &defs.to_string_lossy(),
                &backups,
            )
            .map_err(|e| e.to_string())
    })
    .await
}

/// Save Editor: write the session back to its file (backing up the original on the first save).
#[tauri::command]
pub async fn save_editor_save(
    app: tauri::AppHandle,
    project_id: u32,
) -> CmdResult<crate::dto::SaveEditorSaveDto> {
    off_main(app, move |registry| {
        registry.save_editor_save(project_id).map_err(|e| e.to_string())
    })
    .await
}

#[cfg(test)]
mod iso_metadata_tests {
    use super::*;

    #[test]
    fn iso_metadata_uses_inventory_filename_and_files_root() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let store = studio_domain::BaselineStore::open(&repo.join("baselines/INVENTORY.json"), &repo.join("anchors")).unwrap();
        let path = iso_meta_from_store(&store, "nhl07", "xbox").unwrap();
        assert!(path.ends_with("NHL07NG-meta.xml"));
        assert!(Path::new(&path).is_file());
        assert!(iso_meta_from_store(&store, "nhl14", "xbox").unwrap().ends_with("nhlng-meta.xml"));
        assert!(iso_meta_from_store(&store, "not-a-game", "xbox").is_err());

        // A release ships baselines/files, not the repository's anchors directory.
        let temp = std::env::temp_dir().join(format!("nhlms-iso-schema-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
        let _cleanup = Cleanup(temp.clone());
        let mut packaged = store.clone();
        packaged.files_root = temp.join("baselines/files");
        let component = store.target("nhl07", studio_domain::Platform::Xbox).unwrap()
            .component(studio_domain::baselines::ROLE_META).unwrap();
        let destination = packaged.component_path(component);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(&path, &destination).unwrap();
        assert_eq!(PathBuf::from(iso_meta_from_store(&packaged, "nhl07", "xbox").unwrap()), destination);
        std::fs::write(&destination, b"changed schema").unwrap();
        assert!(iso_meta_from_store(&packaged, "nhl07", "xbox").is_err());
    }
}

/// Read-only: what a disc image carries (layout, member count, database/dictionary locations
/// and identities). Never writes.
#[tauri::command]
pub async fn iso_inspect(app: tauri::AppHandle, image: String) -> CmdResult<crate::dto::IsoInspectDto> {
    off_main(app, move |registry| registry.iso_inspect(&image).map_err(|e| e.to_string())).await
}

/// Copy the complete disc to a fresh folder using the cancellable extraction job service.
#[tauri::command]
pub async fn disc_extract_start(app: tauri::AppHandle, image: String, root: String) -> CmdResult<crate::dto::ExtractJobDto> {
    off_main(app, move |registry| registry.disc_extract_start(&image,&root).map_err(|e| e.to_string())).await
}

/// Prepare a PS4 base PKG. A selected helper overrides the optional release sidecar.
#[tauri::command]
pub async fn pkg_extract_start(app: tauri::AppHandle, image:String, root:String, game_id:String, extractor:Option<String>) -> CmdResult<crate::dto::ExtractJobDto> {
    let tool=match extractor.filter(|p|!p.trim().is_empty()) {
        Some(p)=>p,
        None=>app.path().resource_dir().map_err(|e|e.to_string())?.join("tools/pkg-extractor/pkg_extractor.exe").to_string_lossy().into_owned(),
    };
    off_main(app,move|registry|registry.pkg_extract_start(&image,&root,&game_id,&tool).map_err(|e|e.to_string())).await
}

/// Stage the image's database and dictionary into `staging_root` (reusing an existing staging of
/// the same image) and open the staged database as a workspace with the dictionary attached.
/// Returns the staging description and the project summary.
#[tauri::command]
pub async fn iso_open(
    app: tauri::AppHandle,
    image: String,
    game_id: String,
    platform: String,
    staging_root: String,
    meta_xml: Option<String>,
) -> CmdResult<(crate::dto::IsoStagedDto, ProjectSummaryDto)> {
    let meta = resolve_iso_meta(&app, &game_id, &platform, meta_xml)?;
    let defs = resolve_defs(&app);
    off_main(app, move |registry| {
        registry
            .iso_open(
                &image,
                &game_id,
                &platform,
                &meta,
                &staging_root,
                &defs.to_string_lossy(),
            )
            .map_err(|e| e.to_string())
    })
    .await
}

/// Review: the apply plan for the project's staging folder — which staged files changed and the
/// exact image ranges they would write — verified against the image now, held by the server
/// under `review_id`. Nothing written.
#[tauri::command]
pub async fn iso_plan(app: tauri::AppHandle, project_id: u32) -> CmdResult<crate::dto::IsoPlanDto> {
    off_main(app, move |registry| registry.iso_plan(project_id).map_err(|e| e.to_string())).await
}

/// Where the image route stands for the project (binding, in-flight operation, reviews).
#[tauri::command]
pub async fn iso_status(app: tauri::AppHandle, project_id: u32) -> CmdResult<crate::dto::IsoStatusDto> {
    off_main(app, move |registry| registry.iso_status(project_id).map_err(|e| e.to_string())).await
}

/// **Writes to the disc image** the project's staging manifest names: exactly the reviewed
/// plan `review_id`, as same-length range patches with hash-bound records under
/// `<staging>/records/<n>/`. Refused before any byte changes when the review is stale, the
/// project is dirty or edited since, the staged bytes changed, the emulator is running or the
/// image is held. A live image is a user-gated action.
#[tauri::command]
pub async fn iso_apply(app: tauri::AppHandle, project_id: u32, review_id: u64) -> CmdResult<crate::dto::IsoApplyDto> {
    off_main(app, move |registry| registry.iso_apply(project_id, review_id).map_err(|e| e.to_string())).await
}

/// Put one apply's records back (newest first), verified and idempotent; the records must be
/// this project's staging's own.
#[tauri::command]
pub async fn iso_restore(app: tauri::AppHandle, project_id: u32, records_dir: String) -> CmdResult<crate::dto::IsoRestoreDto> {
    off_main(app, move |registry| registry.iso_restore(project_id, &records_dir).map_err(|e| e.to_string())).await
}

/// Every record under the project's staging folder and what the image holds at each range now.
#[tauri::command]
pub async fn iso_records(app: tauri::AppHandle, project_id: u32) -> CmdResult<Vec<crate::dto::IsoRecordDto>> {
    off_main(app, move |registry| registry.iso_records(project_id).map_err(|e| e.to_string())).await
}

// ---------------------------------------------------------------------------------------------
// Phase 1.2 — league membership
// ---------------------------------------------------------------------------------------------

/// The membership choices for a team and how this title models them.
#[tauri::command]
pub async fn team_membership_options(
    app: tauri::AppHandle,
    project_id: u32,
    team_id: u64,
) -> CmdResult<crate::dto::MembershipOptionsDto> {
    off_main(app, move |registry| {
        registry
            .team_membership_options(project_id, team_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Move a team to a league with validated conference/division group ids (one transaction).
#[tauri::command]
pub async fn set_team_membership(
    app: tauri::AppHandle,
    project_id: u32,
    team_id: u64,
    league_code: u64,
    conference: Option<u64>,
    division: Option<u64>,
) -> CmdResult<crate::dto::MembershipResultDto> {
    off_main(app, move |registry| {
        registry
            .set_team_membership(project_id, team_id, league_code, conference, division)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn swap_team_memberships(app: tauri::AppHandle, project_id:u32, first:u64, second:u64) -> CmdResult<crate::dto::OpOutcomeDto> {
    off_main(app, move |registry| registry.swap_team_memberships(project_id,first,second).map_err(|e|e.to_string())).await
}

/// D2: the read-only settings catalogue (NHL 15 PS4 tables / NHL 16–21 setting_csv), or the blocker.
#[tauri::command]
pub async fn settings_catalogue(
    app: tauri::AppHandle,
    project_id: u32,
    install_root: Option<String>,
) -> CmdResult<crate::dto::SettingsCatalogueResultDto> {
    off_main(app, move |registry| {
        registry
            .settings_catalogue(project_id, install_root.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

/// D3: preview one stored-default edit (held plan) or the refusal.
#[tauri::command]
pub async fn settings_default_preview(
    app: tauri::AppHandle,
    project_id: u32,
    legacy_id: i64,
    context: Option<i64>,
    code: i64,
    install_root: Option<String>,
) -> CmdResult<crate::dto::SettingsEditPreviewDto> {
    off_main(app, move |registry| {
        registry
            .settings_default_preview(project_id, legacy_id, context, code, install_root.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

/// D4: drop the most recent pending NHL 16–21 catalogue edit.
#[tauri::command]
pub async fn settings_csv_undo(app: tauri::AppHandle, project_id: u32) -> CmdResult<bool> {
    off_main(app, move |registry| registry.settings_csv_undo(project_id).map_err(|e| e.to_string())).await
}

/// D8: preview a texture-member replacement inside a packed archive of the install.
#[tauri::command]
pub async fn asset_member_preview(
    app: tauri::AppHandle,
    project_id: u32,
    install_root: String,
    archive_relative: String,
    member_path: String,
    source_image_path: String,
    whole_group: Option<bool>,
) -> CmdResult<crate::dto::AssetMemberPreviewDto> {
    off_main(app, move |registry| {
        registry
            .asset_member_preview_scoped(project_id, &install_root, &archive_relative, &member_path, &source_image_path, whole_group.unwrap_or(false))
            .map_err(|e| e.to_string())
    })
    .await
}

/// Receipt 67 (C37): preview a PS3 board-ad raster replacement (NHL 11–15 / Legacy; same size, DXT1 / DXT3, every declared
/// level). The plan joins the pending replacements through `asset_member_apply_review`; the copied-target save writes it.
#[tauri::command]
pub async fn ps3_board_replacement_preview(
    app: tauri::AppHandle,
    project_id: u32,
    install_root: String,
    package: String,
    descriptor: u32,
    source_image_path: String,
) -> CmdResult<crate::dto::BoardReplacementPreviewDto> {
    off_main(app, move |registry| registry.ps3_board_replacement_preview(project_id, &install_root, &package, descriptor, &source_image_path).map_err(|e| e.to_string())).await
}

/// D8: apply a held texture-member review (pending until the next copied-target save).
#[tauri::command]
pub async fn asset_member_apply_review(app: tauri::AppHandle, project_id: u32, ticket: String) -> CmdResult<crate::dto::AssetMemberApplyDto> {
    off_main(app, move |registry| registry.asset_member_apply_review(project_id, &ticket).map_err(|e| e.to_string())).await
}

/// D8: drop the most recent pending texture-member replacement.
#[tauri::command]
pub async fn asset_member_undo(app: tauri::AppHandle, project_id: u32) -> CmdResult<bool> {
    off_main(app, move |registry| registry.asset_member_undo(project_id).map_err(|e| e.to_string())).await
}

/// D3: apply a held stored-default review by ticket.
#[tauri::command]
pub async fn settings_apply_review(
    app: tauri::AppHandle,
    project_id: u32,
    ticket: String,
) -> CmdResult<crate::dto::SettingsEditApplyDto> {
    off_main(app, move |registry| {
        registry
            .settings_apply_review(project_id, &ticket)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Every schedule table of the open database with the league it serves.
#[tauri::command]
pub async fn team_schedule_edit(app: tauri::AppHandle, project_id:u32, studio_table:String, team:u64, fingerprint:String,
    record:Option<u32>, remove:bool, opponent:Option<u64>, home:bool, month:u64, day:u64, expected_games:Vec<studio_domain::schedule_edit::GameRow>) -> CmdResult<crate::dto::ScheduleViewDto> {
    off_main(app,move |registry| registry.team_schedule_edit(project_id,&studio_table,team,&fingerprint,record,remove,opponent,home,month,day,&expected_games).map_err(|e|e.to_string())).await
}

#[tauri::command]
pub async fn schedule_tables(app: tauri::AppHandle, project_id: u32) -> CmdResult<Vec<crate::dto::ScheduleTableDto>> {
    off_main(app, move |registry| registry.schedule_tables(project_id).map_err(|e| e.to_string())).await
}

/// One schedule table's games (slots resolved to teams) and validation findings.
#[tauri::command]
pub async fn schedule_view(
    app: tauri::AppHandle,
    project_id: u32,
    studio_table: String,
) -> CmdResult<Option<crate::dto::ScheduleViewDto>> {
    off_main(app, move |registry| {
        registry
            .schedule_view(project_id, &studio_table)
            .map_err(|e| e.to_string())
    })
    .await
}

/// P1-A: preview one schedule game edit (home/away/date) with whole-table validation.
#[tauri::command]
pub async fn schedule_edit_preview(
    app: tauri::AppHandle,
    project_id: u32,
    studio_table: String,
    edit: crate::dto::ScheduleGameEditDto,
) -> CmdResult<crate::dto::ScheduleChangePlanDto> {
    off_main(app, move |registry| {
        registry
            .schedule_edit_preview(project_id, &studio_table, &edit)
            .map_err(|e| e.to_string())
    })
    .await
}

/// P1-A: apply a held schedule review (edit / import / generation) as one undoable transaction;
/// refused when the project changed since the preview or the source file changed.
#[tauri::command]
pub async fn schedule_apply_review(
    app: tauri::AppHandle,
    project_id: u32,
    ticket: String,
) -> CmdResult<crate::dto::ScheduleApplyResultDto> {
    off_main(app, move |registry| {
        registry
            .schedule_apply_review(project_id, &ticket)
            .map_err(|e| e.to_string())
    })
    .await
}

/// P1-A: export a schedule table as `csv` or `json` text.
#[tauri::command]
pub async fn schedule_export(
    app: tauri::AppHandle,
    project_id: u32,
    studio_table: String,
    format: String,
    out_path: Option<String>,
) -> CmdResult<crate::dto::ScheduleExportDto> {
    off_main(app, move |registry| {
        registry
            .schedule_export(project_id, &studio_table, &format, out_path.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

/// P1-A: preview a schedule import (parse, map keys, validate the whole proposal).
#[tauri::command]
pub async fn schedule_import_preview(
    app: tauri::AppHandle,
    project_id: u32,
    studio_table: String,
    format: String,
    text: Option<String>,
    path: Option<String>,
    mapping: Vec<crate::dto::ScheduleTeamMappingDto>,
) -> CmdResult<crate::dto::ScheduleImportPlanDto> {
    off_main(app, move |registry| {
        registry
            .schedule_import_preview(project_id, &studio_table, &format, text.as_deref(), path.as_deref(), &mapping)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Receipt 48: the table's shipped single-phase groups by exact source identity, or why not.
#[tauri::command]
pub async fn schedule_stock_groups(
    app: tauri::AppHandle,
    project_id: u32,
    studio_table: String,
) -> CmdResult<crate::dto::ScheduleStockGroupsDto> {
    off_main(app, move |registry| registry.schedule_stock_groups(project_id, &studio_table).map_err(|e| e.to_string())).await
}

/// P1-A: the generation topology a schedule table supports, or the blocker.
#[tauri::command]
pub async fn schedule_generation_options(
    app: tauri::AppHandle,
    project_id: u32,
    studio_table: String,
) -> CmdResult<crate::dto::ScheduleGenerationOptionsDto> {
    off_main(app, move |registry| {
        registry
            .schedule_generation_options(project_id, &studio_table)
            .map_err(|e| e.to_string())
    })
    .await
}

/// P1-A: preview a deterministic schedule generation.
#[tauri::command]
pub async fn schedule_generate_preview(
    app: tauri::AppHandle,
    project_id: u32,
    studio_table: String,
    config: crate::dto::ScheduleGenerationConfigDto,
) -> CmdResult<crate::dto::ScheduleGenerationPlanDto> {
    off_main(app, move |registry| {
        registry
            .schedule_generate_preview(project_id, &studio_table, &config)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Phase 1.4: a league's logo binding and package files on this title.
#[tauri::command]
pub async fn league_logo(
    app: tauri::AppHandle,
    project_id: u32,
    code: u64,
    install_root: Option<String>,
) -> CmdResult<Option<crate::dto::LeagueLogoDto>> {
    off_main(app, move |registry| {
        registry
            .league_logo(project_id, code, install_root.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

/// P1-G: the title's league-keyed presentation imagery (beyond the four logo families).
#[tauri::command]
pub async fn league_imagery(
    app: tauri::AppHandle,
    project_id: u32,
    install_root: Option<String>,
) -> CmdResult<Option<crate::dto::LeagueImageryDto>> {
    off_main(app, move |registry| {
        registry
            .league_imagery(project_id, install_root.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

/// Phase 2.1: a team's identity dependencies (index / art id consumers) and the contract's
/// disposition of changing each.
#[tauri::command]
pub async fn team_identity_dependencies(app: tauri::AppHandle, project_id: u32, team_id: u64) -> CmdResult<crate::dto::TeamIdentityDependenciesDto> {
    off_main(app, move |registry| registry.team_identity_dependencies(project_id, team_id).map_err(|e| e.to_string())).await
}

/// Phase 2.1: swap the art ids of two teams as one transaction (the jersey / award rows of both
/// follow); refused per the re-identification contract otherwise.
#[tauri::command]
pub async fn team_artid_swap(app: tauri::AppHandle, project_id: u32, team_a: u64, team_b: u64) -> CmdResult<crate::dto::ArtIdSwapDto> {
    off_main(app, move |registry| registry.team_artid_swap(project_id, team_a, team_b).map_err(|e| e.to_string())).await
}

/// Phase 2.7: the read-only name-call listing of one team from the loose gen7 speech files.
#[tauri::command]
pub async fn team_name_calls(app: tauri::AppHandle, project_id: u32, team_id: u64, install_root: String) -> CmdResult<crate::dto::TeamNameCallsDto> {
    off_main(app, move |registry| registry.team_name_calls(project_id, team_id, &install_root).map_err(|e| e.to_string())).await
}

/// P4.6 — list a measured gen7 bank (`pbp_eng` | `ann_eng` | `music_samples`) with each sample's SNR
/// header; `query` filters on the sample id text. Listing only.
#[tauri::command]
pub async fn gen7_bank_samples(app: tauri::AppHandle, project_id: u32, install_root: String, bank_stem: String, query: Option<String>, limit: Option<u32>) -> CmdResult<crate::dto::Gen7BankListingDto> {
    off_main(app, move |registry| registry.gen7_bank_samples(project_id, &install_root, &bank_stem, query.as_deref().unwrap_or(""), limit.unwrap_or(500) as usize).map_err(|e| e.to_string())).await
}

/// P4.3 — the **diagnostic** gen7 EALayer3 v1 preview of one speech sample (`bank_stem`: `pbp_eng` |
/// `ann_eng`; measured tuples only). Never game-verified; no replacement route.
#[tauri::command]
pub async fn gen7_speech_preview(app: tauri::AppHandle, project_id: u32, install_root: String, bank_stem: String, sample_id: u32) -> CmdResult<crate::dto::Gen7SpeechPreviewDto> {
    off_main(app, move |registry| registry.gen7_speech_preview(project_id, &install_root, &bank_stem, sample_id).map_err(|e| e.to_string())).await
}

/// Receipt 33 — cancel the project's in-flight gen7 sample preview (the EA-XMA helper is stopped);
/// answers whether one was running.
#[tauri::command]
pub fn gen7_speech_preview_cancel(registry: State<'_, Registry>, project_id: u32) -> CmdResult<bool> {
    Ok(registry.gen7_speech_preview_cancel(project_id))
}

/// 25 Sep 2026 — the gen7 `.abk` sound banks of the loose `audio/sfx` (listing only; measured
/// populations R89 / R91).
#[tauri::command]
pub async fn gen7_sfx_banks(app: tauri::AppHandle, project_id: u32, install_root: String) -> CmdResult<crate::dto::Gen7SfxBankListDto> {
    off_main(app, move |registry| registry.gen7_sfx_banks(project_id, &install_root).map_err(|e| e.to_string())).await
}

/// 25 Sep 2026 — one `.abk` bank's named picker tree, entries framed in the sibling `.ast`.
#[tauri::command]
pub async fn gen7_sfx_bank_tree(app: tauri::AppHandle, project_id: u32, install_root: String, bank: String) -> CmdResult<crate::dto::Gen7SfxBankTreeDto> {
    off_main(app, move |registry| registry.gen7_sfx_bank_tree(project_id, &install_root, &bank).map_err(|e| e.to_string())).await
}

/// 25 Sep 2026 — the **diagnostic** decode of one `.abk` entry (PS3 EALayer3 v1, one sub-stream).
/// Never game-verified; no replacement route.
#[tauri::command]
pub async fn gen7_sfx_preview(app: tauri::AppHandle, project_id: u32, install_root: String, bank: String, record: u32, sub_stream: Option<u32>) -> CmdResult<crate::dto::Gen7SfxPreviewDto> {
    off_main(app, move |registry| registry.gen7_sfx_preview(project_id, &install_root, &bank, record, sub_stream).map_err(|e| e.to_string())).await
}

/// P4 — the PS4 NHL 15–21 speech banks of the open title (listing only; read from the extracted
/// kettle tree under `install_root`).
#[tauri::command]
pub async fn ps4_speech_banks(app: tauri::AppHandle, project_id: u32, install_root: String) -> CmdResult<crate::dto::Ps4SpeechBanksDto> {
    off_main(app, move |registry| registry.ps4_speech_banks(project_id, &install_root).map_err(|e| e.to_string())).await
}

/// P4 — one bank's samples with their measured SNR headers (codec census); `query` filters on the
/// sample id text.
#[tauri::command]
pub async fn ps4_speech_samples(app: tauri::AppHandle, project_id: u32, install_root: String, bank_file: String, query: Option<String>, limit: Option<u32>) -> CmdResult<crate::dto::Ps4SpeechSamplesDto> {
    off_main(app, move |registry| registry.ps4_speech_samples(project_id, &install_root, &bank_file, query.as_deref().unwrap_or(""), limit.unwrap_or(500) as usize).map_err(|e| e.to_string())).await
}

/// P4 — the **diagnostic** EA-MP3 preview of one sample (`rendering`: `mpeg_only` | `candidate_a`).
/// Never game-verified; no replacement route.
#[tauri::command]
pub async fn ps4_speech_preview(app: tauri::AppHandle, project_id: u32, install_root: String, bank_file: String, sid: u64, rendering: String) -> CmdResult<crate::dto::Ps4SpeechPreviewDto> {
    off_main(app, move |registry| registry.ps4_speech_preview(project_id, &install_root, &bank_file, sid, &rendering).map_err(|e| e.to_string())).await
}

/// Phase 1.5 / 2.7: the read-only context listing (trigger → conditions → items).
#[tauri::command]
pub async fn context_catalogue(app: tauri::AppHandle, project_id: u32, install_root: String, query: Option<String>, limit: Option<u32>) -> CmdResult<crate::dto::ContextCatalogueDto> {
    off_main(app, move |registry| registry.context_catalogue(project_id, &install_root, query.as_deref().unwrap_or(""), limit.unwrap_or(200) as usize).map_err(|e| e.to_string())).await
}

/// Phase 3.5: one player's recorded names (read-only) from the loose gen7 speech files.
#[tauri::command]
pub async fn player_name_calls(app: tauri::AppHandle, project_id: u32, player_id: u64, install_root: String) -> CmdResult<crate::dto::PlayerNameCallsDto> {
    off_main(app, move |registry| registry.player_name_calls(project_id, player_id, &install_root).map_err(|e| e.to_string())).await
}

/// Phase 1.5 / 2.7 / 3.5: the read-only gen7 commentary catalogue (events, sentences sharing an
/// event's name, phrases → candidate samples → bank), filtered and capped.
#[tauri::command]
pub async fn speech_catalogue(
    app: tauri::AppHandle,
    project_id: u32,
    install_root: String,
    query: Option<String>,
    league_only: Option<bool>,
    limit: Option<u32>,
) -> CmdResult<crate::dto::SpeechCatalogueDto> {
    off_main(app, move |registry| {
        registry
            .speech_catalogue(project_id, &install_root, query.as_deref().unwrap_or(""), league_only.unwrap_or(false), limit.unwrap_or(200) as usize)
            .map_err(|e| e.to_string())
    })
    .await
}

/// LOC-W2: preview an in-place free-text re-encode of a dictionary key's text.
#[tauri::command]
pub async fn dictionary_retext_preview(
    app: tauri::AppHandle,
    project_id: u32,
    key: String,
    text: String,
) -> CmdResult<crate::dto::DictionaryRetextPreviewDto> {
    off_main(app, move |registry| {
        registry
            .dictionary_retext_preview(project_id, &key, &text)
            .map_err(|e| e.to_string())
    })
    .await
}

/// LOC-W2: hold an in-place free-text re-encode as the pending dictionary change.
#[tauri::command]
pub async fn dictionary_retext_set(
    app: tauri::AppHandle,
    project_id: u32,
    key: String,
    text: String,
) -> CmdResult<crate::dto::DictionaryRetextPreviewDto> {
    off_main(app, move |registry| {
        registry
            .dictionary_retext_set(project_id, &key, &text)
            .map_err(|e| e.to_string())
    })
    .await
}

/// LOC-W3: preview a dictionary growth (appended texts, inserted keys, nulls).
#[tauri::command]
pub async fn dictionary_growth_preview(
    app: tauri::AppHandle,
    project_id: u32,
    ops: Vec<crate::dto::DictionaryGrowthOpDto>,
    options: Option<crate::dto::DictionaryGrowthOptionsDto>,
) -> CmdResult<crate::dto::DictionaryGrowthPreviewDto> {
    off_main(app, move |registry| registry.dictionary_growth_preview_with(project_id, &ops, options.as_ref()).map_err(|e| e.to_string())).await
}

/// LOC-W3: hold a dictionary growth as the pending dictionary change.
#[tauri::command]
pub async fn dictionary_growth_set(
    app: tauri::AppHandle,
    project_id: u32,
    ops: Vec<crate::dto::DictionaryGrowthOpDto>,
    options: Option<crate::dto::DictionaryGrowthOptionsDto>,
) -> CmdResult<crate::dto::DictionaryGrowthPreviewDto> {
    off_main(app, move |registry| registry.dictionary_growth_set_with(project_id, &ops, options.as_ref()).map_err(|e| e.to_string())).await
}

/// Phase 2.3: a team's dictionary key families (by `artabbr`) and their texts.
#[tauri::command]
pub async fn team_text(app: tauri::AppHandle, project_id: u32, team_id: u64) -> CmdResult<Option<crate::dto::TeamTextDto>> {
    off_main(app, move |registry| registry.team_text(project_id, team_id).map_err(|e| e.to_string())).await
}

// ======================================================================================
// ONB-002 — project folders, bundled baselines, extraction service, deployment
// ======================================================================================

/// The bundled `baselines/` directory under a resource directory (`INVENTORY.json` plus `files/`),
/// for the no-installer archive layout. `None` when the directory holds no inventory.
pub fn bundled_baselines(resource_dir: &Path) -> Option<PathBuf> {
    let p = resource_dir.join("baselines");
    p.join(studio_domain::baselines::INVENTORY_FILE).is_file().then_some(p)
}

/// Resolve the baseline store: the packaged `baselines/` beside the executable, falling back to
/// the repository's `baselines/INVENTORY.json` over `anchors/` in a development run. Never a
/// hard-coded absolute path.
fn resolve_baselines(app: &tauri::AppHandle) -> CmdResult<studio_domain::BaselineStore> {
    if let Some(dir) = app.path().resource_dir().ok().and_then(|res| bundled_baselines(&res)) {
        return studio_domain::BaselineStore::open_bundled(&dir)
            .expect("checked: the inventory exists")
            .map_err(|e| e.to_string());
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    studio_domain::BaselineStore::open(
        &repo.join("baselines").join(studio_domain::baselines::INVENTORY_FILE),
        &repo.join("anchors"),
    )
    .map_err(|e| e.to_string())
}

/// The bundled baseline inventory: per shipping target, what Start fresh can use and what it cannot.
#[tauri::command]
pub async fn baseline_inventory(app: tauri::AppHandle) -> CmdResult<crate::dto::BaselineInventoryDto> {
    let store = resolve_baselines(&app)?;
    off_main(app, move |registry| Ok(registry.baseline_inventory(&store))).await
}

/// Start fresh: create a project folder from the bundled baseline of `game_id`/`platform`, with
/// the chosen game installation and save location as its deployment destinations, and open it.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn project_create_fresh(
    app: tauri::AppHandle,
    project_dir: String,
    game_id: String,
    platform: String,
    root: String,
    save_path: Option<String>,
    roster_save: Option<String>,
) -> CmdResult<crate::dto::ProjectOpenDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let store = resolve_baselines(&app)?;
    let defs = resolve_defs(&app);
    off_main(app, move |registry| {
        registry
            .project_create_fresh(&project_dir, &game_id, platform, &root, save_path.as_deref(), roster_save.as_deref(), &store, &defs.to_string_lossy())
            .map_err(|e| e.to_string())
    })
    .await
}

/// Use my own files: validate the chosen source files without writing anything.
#[tauri::command]
pub async fn project_import_review(
    app: tauri::AppHandle,
    game_id: String,
    platform: String,
    sources: crate::dto::ImportSourcesDto,
) -> CmdResult<crate::dto::ImportReviewDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let defs = resolve_defs(&app);
    let store = resolve_baselines(&app).ok();
    off_main(app, move |registry| registry.project_import_review(&game_id, platform, &sources, store.as_ref(), &defs.to_string_lossy()).map_err(|e| e.to_string())).await
}

/// Use my own files: create a project folder from the validated sources and open it. The sources
/// are copied; they are never written.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn project_create_import(
    app: tauri::AppHandle,
    project_dir: String,
    game_id: String,
    platform: String,
    sources: crate::dto::ImportSourcesDto,
    root: String,
    save_path: Option<String>,
    roster_save: Option<String>,
) -> CmdResult<crate::dto::ProjectOpenDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let defs = resolve_defs(&app);
    let store = resolve_baselines(&app).ok();
    off_main(app, move |registry| {
        registry
            .project_create_import(&project_dir, &game_id, platform, &sources, &root, save_path.as_deref(), roster_save.as_deref(), store.as_ref(), &defs.to_string_lossy())
            .map_err(|e| e.to_string())
    })
    .await
}

/// ONB-003: open an existing project folder and resume it (same project, target, destinations,
/// edited files, pending state and last view; missing external paths become warnings).
#[tauri::command]
pub async fn project_open(app: tauri::AppHandle, project_dir: String) -> CmdResult<crate::dto::ProjectOpenDto> {
    let defs = resolve_defs(&app);
    off_main(app, move |registry| registry.project_open(&project_dir, &defs.to_string_lossy()).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_copy_to(app: tauri::AppHandle, project_id: u32, destination: String) -> CmdResult<String> {
    off_main(app, move |registry| registry.project_copy_to(project_id, &destination).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn mod_library_scan(app: tauri::AppHandle, root: String) -> CmdResult<crate::dto::ModLibraryDto> {
    off_main(app, move |registry| registry.mod_library_scan(&root).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn mod_manager_check_target(app: tauri::AppHandle, root: String, platform: String) -> CmdResult<String> {
    off_main(app, move |registry| registry.mod_manager_check_target(&root, &platform).map_err(|e| e.to_string())).await
}

/// ONB-003: remember the last meaningful Studio route of the open project.
#[tauri::command]
pub async fn project_set_view(app: tauri::AppHandle, project_id: u32, route: String) -> CmdResult<()> {
    off_main(app, move |registry| registry.project_set_view(project_id, &route).map_err(|e| e.to_string())).await
}

/// ONB-003: link or relink the open project's machine-local destinations (game installation, save
/// location, roster save, extracted-assets folder). Read-only discovery; nothing is written there.
#[tauri::command]
pub async fn project_relink(
    app: tauri::AppHandle,
    project_id: u32,
    root: Option<String>,
    save_path: Option<String>,
    roster_save: Option<String>,
    assets_dir: Option<String>,
) -> CmdResult<crate::dto::ProjectRelinkDto> {
    off_main(app, move |registry| {
        registry
            .project_relink(project_id, root.as_deref(), save_path.as_deref(), roster_save.as_deref(), assets_dir.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn project_info(app: tauri::AppHandle, project_id: u32) -> CmdResult<crate::dto::ProjectInfoDto> {
    off_main(app, move |registry| registry.project_info(project_id).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_overview(app: tauri::AppHandle, project_id: u32) -> CmdResult<crate::dto::ProjectOverviewDto> {
    off_main(app, move |r| r.project_overview(project_id).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_set_details(app: tauri::AppHandle, project_id: u32, details: crate::dto::ProjectDetailsDto) -> CmdResult<crate::dto::ProjectDetailsDto> {
    off_main(app, move |r| r.project_set_details(project_id, details).map_err(|e| e.to_string())).await
}

/// Save the project: every pending database, roster, dictionary, settings-CSV and archive-member
/// edit is written into the project's working root as one journaled transaction.
#[tauri::command]
pub async fn project_save(app: tauri::AppHandle, project_id: u32) -> CmdResult<crate::dto::ProjectSaveDto> {
    off_main(app, move |registry| registry.project_save(project_id).map_err(|e| e.to_string())).await
}

/// Record the user's extraction choice for the project (`skipped` | `extracted` | `not_needed`).
#[tauri::command]
pub async fn project_set_extraction(app: tauri::AppHandle, project_id: u32, choice: String) -> CmdResult<crate::dto::ProjectInfoDto> {
    off_main(app, move |registry| registry.project_set_extraction(project_id, &choice).map_err(|e| e.to_string())).await
}

/// Plan a deployment (read-only). `allow_changed` lets destinations changed outside Studio be
/// replaced; without it they block the plan.
#[tauri::command]
pub async fn project_deploy_plan(app: tauri::AppHandle, project_id: u32, allow_changed: bool) -> CmdResult<crate::dto::DeployPlanDto> {
    off_main(app, move |registry| registry.project_deploy_plan(project_id, allow_changed).map_err(|e| e.to_string())).await
}

/// Apply the reviewed deployment plan `handle`: the only route that writes the game and save
/// destinations (extraction aside).
#[tauri::command]
pub async fn project_deploy_apply(app: tauri::AppHandle, project_id: u32, handle: u32) -> CmdResult<crate::dto::DeployApplyDto> {
    off_main(app, move |registry| registry.project_deploy_apply(project_id, handle).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_deploy_restore(app: tauri::AppHandle, project_id: u32, seq: u64) -> CmdResult<crate::dto::DeployRestoreDto> {
    off_main(app, move |registry| registry.project_deploy_restore(project_id, seq).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_deployments(app: tauri::AppHandle, project_id: u32) -> CmdResult<Vec<crate::dto::DeploymentRecordDto>> {
    off_main(app, move |registry| registry.project_deployments(project_id).map_err(|e| e.to_string())).await
}

// ONB-003 — mod package builder and installer

/// The installer's record folder (installation records, backups, staging) under the app data
/// directory; tests pass their own. Never inside a game installation.
fn resolve_install_records(app: &tauri::AppHandle) -> CmdResult<PathBuf> {
    let base = app.path().app_data_dir().map_err(|e| format!("no application data directory: {e}"))?;
    Ok(base.join("installs"))
}

/// What the open project could package right now: per category the items and blockers, per
/// platform the measured support. Read-only.
#[tauri::command]
pub async fn package_inventory(app: tauri::AppHandle, project_id: u32) -> CmdResult<crate::dto::PackageInventoryDto> {
    let defs = resolve_defs(&app);
    let store = resolve_baselines(&app).ok();
    off_main(app, move |registry| registry.package_inventory(project_id, store.as_ref(), &defs.to_string_lossy()).map_err(|e| e.to_string())).await
}

/// Build one `.nhlmod` package from the open, saved project. Refuses rather than omitting.
#[tauri::command]
pub async fn package_build(app: tauri::AppHandle, project_id: u32, request: crate::dto::PackageBuildRequestDto) -> CmdResult<crate::dto::PackageBuildDto> {
    let defs = resolve_defs(&app);
    let store = resolve_baselines(&app).ok();
    off_main(app, move |registry| registry.package_build(project_id, &request, store.as_ref(), &defs.to_string_lossy()).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_build(app: tauri::AppHandle, project_id: u32, projects_root: String) -> CmdResult<crate::dto::ProjectBuildDto> {
    let defs = resolve_defs(&app);
    let store = resolve_baselines(&app).ok();
    off_main(app, move |r| r.project_build(project_id, &projects_root, store.as_ref(), &defs.to_string_lossy()).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_archive(app: tauri::AppHandle, project_id: u32, projects_root: String) -> CmdResult<crate::dto::ProjectArchiveDto> {
    off_main(app, move |r| r.project_archive(project_id, &projects_root).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_archive_preview(app: tauri::AppHandle, archive: String) -> CmdResult<crate::dto::ProjectArchivePreviewDto> {
    off_main(app, move |r| r.project_archive_preview(&archive).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_import_archive(app: tauri::AppHandle, archive: String, destination: String, source_root: Option<String>) -> CmdResult<String> {
    off_main(app, move |r| r.project_import_archive(&archive, &destination, source_root.as_deref()).map_err(|e| e.to_string())).await
}

/// Read and validate a package file; nothing is written.
#[tauri::command]
pub async fn package_inspect(app: tauri::AppHandle, path: String) -> CmdResult<crate::dto::PackageInspectDto> {
    off_main(app, move |registry| registry.package_inspect(&path).map_err(|e| e.to_string())).await
}

/// Plan an installation onto the chosen destination (read-only; staged in the installer's own
/// folder). The plan must be applied explicitly.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn package_preflight(
    app: tauri::AppHandle,
    package_path: String,
    game_id: String,
    platform: String,
    root: String,
    save_path: Option<String>,
    roster_save: Option<String>,
    allow_conflicts: bool,
) -> CmdResult<crate::dto::InstallPlanDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let defs = resolve_defs(&app);
    let records = resolve_install_records(&app)?;
    // R72-R1: a roster-only package takes its schema metadata from the shipped baseline store,
    // never from a game installation the recipient may not have.
    let store = resolve_baselines(&app).ok();
    off_main(app, move |registry| {
        registry
            .package_preflight_with(&package_path, &game_id, platform, &root, save_path.as_deref(), roster_save.as_deref(), allow_conflicts, &records.to_string_lossy(), &defs.to_string_lossy(), store.as_ref())
            .map_err(|e| e.to_string())
    })
    .await
}

/// Apply a reviewed installation plan (backups first, two-phase publish, record kept).
#[tauri::command]
pub async fn package_apply(app: tauri::AppHandle, handle: u32) -> CmdResult<crate::dto::InstallApplyDto> {
    off_main(app, move |registry| registry.package_apply(handle).map_err(|e| e.to_string())).await
}

/// Every installation record on this machine, newest first.
#[tauri::command]
pub async fn package_installs(app: tauri::AppHandle) -> CmdResult<Vec<crate::dto::InstallRecordDto>> {
    let records = resolve_install_records(&app)?;
    off_main(app, move |registry| registry.package_installs(&records.to_string_lossy()).map_err(|e| e.to_string())).await
}

/// Restore (uninstall) one installation from its record; refuses when a destination changed since.
#[tauri::command]
pub async fn package_restore(app: tauri::AppHandle, record_dir: String) -> CmdResult<crate::dto::InstallApplyDto> {
    off_main(app, move |registry| registry.package_restore(&record_dir).map_err(|e| e.to_string())).await
}

/// Extraction preflight: what the installation's archives hold and what is already extracted.
/// Read-only.
#[tauri::command]
pub async fn game_files_check(app: tauri::AppHandle, root: String, game_id: String, platform: String) -> CmdResult<studio_domain::game_manifest::Report> {
    off_main(app, move |_| {
        use studio_domain::game_manifest::{check, unverified, Manifest};
        if !matches!(game_id.as_str(), "nhl07" | "nhl08") || platform != "xbox" {
            return Ok(unverified("No reference file manifest is bundled for this game/platform yet."));
        }
        // Embed the reviewed anchor manifest: packaged builds never depend on a dev path.
        let source = match game_id.as_str() {
            "nhl08" => include_str!("../../../anchors/xbox360/nhl08/game-files.json"),
            _ => include_str!("../../../anchors/xbox360/nhl07/game-files.json"),
        };
        let manifest = serde_json::from_str::<Manifest>(source);
        Ok(match manifest {
            Ok(m) => check(Path::new(&root), &m, &game_id, &platform),
            Err(e) => unverified(format!("Reference manifest could not be loaded: {e}")),
        })
    }).await
}

#[tauri::command]
pub async fn extract_preflight(app: tauri::AppHandle, root: String, platform: String, destination: Option<String>) -> CmdResult<crate::dto::ExtractPreflightDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    off_main(app, move |registry| registry.extract_preflight_to(&root, platform, destination.as_deref()).map_err(|e| e.to_string())).await
}

/// Start extracting the installation's archives in place (the explicit installation-write
/// exception). Returns the job; poll `extract_status`.
#[tauri::command]
pub async fn extract_start(app: tauri::AppHandle, root: String, platform: String, destination: Option<String>) -> CmdResult<crate::dto::ExtractJobDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    off_main(app, move |registry| registry.extract_start_to(&root, platform, destination.as_deref()).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn extract_status(app: tauri::AppHandle, job: u32) -> CmdResult<crate::dto::ExtractJobDto> {
    off_main(app, move |registry| registry.extract_status(job).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn extract_cancel(app: tauri::AppHandle, job: u32) -> CmdResult<crate::dto::ExtractJobDto> {
    off_main(app, move |registry| registry.extract_cancel(job).map_err(|e| e.to_string())).await
}

/// The running extraction for a destination, if any (so onboarding and Settings share one job).
#[tauri::command]
pub async fn extract_running_for(app: tauri::AppHandle, root: String) -> CmdResult<Option<crate::dto::ExtractJobDto>> {
    off_main(app, move |registry| Ok(registry.extract_running_for(&root))).await
}

#[tauri::command]
pub async fn project_revert_change(app: tauri::AppHandle, project_id: u32, change_id: String) -> CmdResult<crate::dto::OpOutcomeDto> {
    off_main(app, move |r| r.project_revert_change(project_id, &change_id).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn project_delete(app: tauri::AppHandle, project_dir: String) -> CmdResult<()> {
    off_main(app, move |registry| registry.project_delete(&project_dir).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub async fn team_artid_options(app: tauri::AppHandle, project_id: u32) -> CmdResult<Vec<(u64,u64,String)>> {
    off_main(app, move |registry| registry.team_artid_options(project_id).map_err(|e| e.to_string())).await
}
