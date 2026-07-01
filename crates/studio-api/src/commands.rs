//! The Tauri command surface — one thin wrapper per command, forwarding to the [`Registry`] and
//! stringifying errors (Tauri serializes `Err(String)` to the frontend). Register these in the
//! Tauri builder via `tauri::generate_handler!` (see `src-tauri`).

use std::path::{Path, PathBuf};

use tauri::{Manager, State};

use studio_domain::FieldValue;

use crate::dto::{
    ArenaSummaryDto, ArenaViewDto, CareerStatsDto, CreatePlayerDto, InstallReportDto,
    IntegrityReportDto, LinesViewDto, OpOutcomeDto, PlayerSummaryDto, PlayerViewDto,
    ProjectSummaryDto, TeamSummaryDto, TeamViewDto,
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

/// Onboarding step 1: point at a root folder (RPCS3 root for PS3, the `default.xex` install dir for
/// Xbox) and report the resolved databases + save inventory. `xenia_root` optionally overrides the
/// default Xenia content directory (Xbox only).
#[tauri::command]
pub fn detect_install(
    registry: State<'_, Registry>,
    game_id: String,
    platform: String,
    root: String,
    xenia_root: Option<String>,
) -> CmdResult<InstallReportDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    Ok(registry.detect_install(&game_id, platform, &root, xenia_root.as_deref()))
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
    xenia_root: Option<String>,
    big_files_extracted: bool,
) -> CmdResult<ProjectSummaryDto> {
    let platform = parse_platform(&platform).map_err(|e| e.to_string())?;
    let defs = resolve_defs(&app);
    registry
        .open_workspace(
            &game_id,
            platform,
            &root,
            xenia_root.as_deref(),
            big_files_extracted,
            &defs.to_string_lossy(),
        )
        .map_err(|e| e.to_string())
}

/// Extract a game's "big files" for modding, given the install root the user pointed at. **Not yet
/// implemented** — the EA big-archive codec is a later, separately-verified subsystem; this surfaces
/// a clear message rather than a silent no-op. Runs at onboarding (before a project is opened), so it
/// takes the install context rather than a project id.
#[tauri::command]
pub fn extract_big_files(
    _game_id: String,
    _platform: String,
    _root: String,
) -> CmdResult<()> {
    Err("big-file extraction is not yet implemented".to_string())
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

#[tauri::command]
pub fn list_players(
    registry: State<'_, Registry>,
    project_id: u32,
) -> CmdResult<Vec<PlayerSummaryDto>> {
    registry.list_players(project_id).map_err(|e| e.to_string())
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
pub fn set_roster_status(
    registry: State<'_, Registry>,
    project_id: u32,
    team: u64,
    player: u64,
    status: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .set_roster_status(project_id, team, player, status)
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
