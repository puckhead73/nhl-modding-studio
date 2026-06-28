//! The Tauri command surface — one thin wrapper per command, forwarding to the [`Registry`] and
//! stringifying errors (Tauri serializes `Err(String)` to the frontend). Register these in the
//! Tauri builder via `tauri::generate_handler!` (see `src-tauri`).

use tauri::State;

use crate::dto::{CreatePlayerDto, IntegrityReportDto, OpOutcomeDto, ProjectSummaryDto};
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
