//! The Tauri command surface — one thin wrapper per command, forwarding to the [`Registry`] and
//! stringifying errors (Tauri serializes `Err(String)` to the frontend). Register these in the
//! Tauri builder via `tauri::generate_handler!` (see `src-tauri`).

use tauri::State;

use studio_domain::FieldValue;

use crate::dto::{
    ArenaSummaryDto, ArenaViewDto, CareerStatsDto, CreatePlayerDto, IntegrityReportDto,
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
    to_team: u64,
) -> CmdResult<OpOutcomeDto> {
    registry
        .move_player(project_id, player, to_team)
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
