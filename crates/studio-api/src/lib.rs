//! `studio-api` — the Tauri-facing surface: the session [`Registry`], the typed [`dto`]s (with
//! generated TS counterparts), the domain↔DTO [`convert`] bridge, and the thin [`commands`].
//!
//! It is the only crate that depends on Tauri; all real work lives in `studio-domain`. Register the
//! commands and managed state from the Tauri app like so:
//!
//! ```ignore
//! tauri::Builder::default()
//!     .manage(studio_api::Registry::default())
//!     .invoke_handler(studio_api::command_handler!())
//!     .run(tauri::generate_context!())
//! ```

pub mod commands;
pub mod convert;
pub mod dto;
mod error;
mod registry;

pub use convert::{parse_raw_id, raw_id_to_string};
pub use dto::*;
pub use error::ApiError;
pub use registry::{parse_platform, parse_role, Registry};

/// Expands to the `tauri::generate_handler!` invocation listing every studio command, so the command
/// set is maintained in one place (here) rather than duplicated in the Tauri app.
#[macro_export]
macro_rules! command_handler {
    () => {
        ::tauri::generate_handler![
            $crate::commands::load_project,
            $crate::commands::open_project,
            $crate::commands::detect_install,
            $crate::commands::open_workspace,
            $crate::commands::extract_big_files,
            $crate::commands::close_project,
            $crate::commands::project_summary,
            $crate::commands::validate_project,
            $crate::commands::set_edit_override,
            $crate::commands::autocorrect_jerseys,
            $crate::commands::list_players,
            $crate::commands::player_view,
            $crate::commands::career_stats,
            $crate::commands::list_teams,
            $crate::commands::team_view,
            $crate::commands::lines_view,
            $crate::commands::list_arenas,
            $crate::commands::arena_view,
            $crate::commands::set_field,
            $crate::commands::set_captaincy,
            $crate::commands::create_player,
            $crate::commands::clone_player,
            $crate::commands::delete_player,
            $crate::commands::move_player,
            $crate::commands::trade_slots,
            $crate::commands::release_to_fa,
            $crate::commands::sign_from_fa,
            $crate::commands::set_jersey_number,
            $crate::commands::retire_jersey,
            $crate::commands::unretire_jersey,
            $crate::commands::set_roster_status,
            $crate::commands::undo,
            $crate::commands::redo,
            $crate::commands::save_project,
            $crate::commands::save_workspace,
        ]
    };
}

#[cfg(test)]
mod bindings {
    use crate::dto::*;
    use ts_rs::TS;

    /// Generate the TypeScript DTO definitions into the frontend's bindings dir (the FE/BE contract).
    #[test]
    fn export_typescript() {
        std::env::set_var(
            "TS_RS_EXPORT_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/bindings"),
        );
        ProjectSummaryDto::export_all().expect("export ProjectSummaryDto + deps");
        InstallReportDto::export_all().expect("export InstallReportDto + deps");
        ExtractSummaryDto::export_all().expect("export ExtractSummaryDto");
        IntegrityReportDto::export_all().expect("export IntegrityReportDto + deps");
        AutocorrectSummaryDto::export_all().expect("export AutocorrectSummaryDto + deps");
        CreatePlayerDto::export_all().expect("export CreatePlayerDto + deps");
        OpOutcomeDto::export_all().expect("export OpOutcomeDto");
        PlayerViewDto::export_all().expect("export PlayerViewDto + deps");
        PlayerSummaryDto::export_all().expect("export PlayerSummaryDto");
        TeamViewDto::export_all().expect("export TeamViewDto + deps");
        TeamSummaryDto::export_all().expect("export TeamSummaryDto");
        ArenaViewDto::export_all().expect("export ArenaViewDto + deps");
        ArenaSummaryDto::export_all().expect("export ArenaSummaryDto");
        LinesViewDto::export_all().expect("export LinesViewDto + deps");
        CareerStatsDto::export_all().expect("export CareerStatsDto");
    }
}
