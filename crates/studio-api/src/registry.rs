//! The project/session registry — the in-process store of live [`EditSession`]s keyed by id, plus
//! the GUI-agnostic command logic (returns DTOs, so the Tauri wrappers in [`crate::commands`] are
//! one-liners). Lives behind a `Mutex` so it can be a single Tauri-managed state.

use std::collections::HashMap;
use std::sync::Mutex;

use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole};

use crate::convert;
use crate::dto::{
    ArenaSummaryDto, ArenaViewDto, CreatePlayerDto, IntegrityReportDto, OpOutcomeDto,
    PlayerSummaryDto, PlayerViewDto, ProjectSummaryDto, TeamSummaryDto, TeamViewDto,
};
use crate::error::ApiError;

#[derive(Default)]
struct Inner {
    next: u32,
    sessions: HashMap<u32, EditSession>,
}

/// The session registry. Construct with `Registry::default()` and hand to Tauri via `.manage(...)`.
#[derive(Default)]
pub struct Registry {
    inner: Mutex<Inner>,
}

impl Registry {
    /// Load a project, begin an edit session, register it, and return its summary (with the new id).
    pub fn load_project(
        &self,
        db_path: &str,
        platform: Platform,
        role: ProjectRole,
        meta_xml: &str,
        defs_root: &str,
    ) -> Result<ProjectSummaryDto, ApiError> {
        let project = Project::load(db_path, platform, role, meta_xml, defs_root)?;
        let session = EditSession::begin(project);
        let mut guard = self.inner.lock().unwrap();
        let id = guard.next;
        guard.next += 1;
        guard.sessions.insert(id, session);
        // Safe: just inserted.
        let session = guard.sessions.get(&id).unwrap();
        convert::project_summary_dto(id, session).map_err(ApiError::Domain)
    }

    /// Drop a session, freeing its working bytes. Returns `false` if it was not registered.
    pub fn close_project(&self, id: u32) -> bool {
        self.inner.lock().unwrap().sessions.remove(&id).is_some()
    }

    /// A fresh summary of the project (reflects pending edits / growth).
    pub fn summary(&self, id: u32) -> Result<ProjectSummaryDto, ApiError> {
        self.with_session(id, |s| {
            convert::project_summary_dto(id, s).map_err(ApiError::Domain)
        })
    }

    /// Run the Layer 4 audit over the current working state.
    pub fn validate(&self, id: u32) -> Result<IntegrityReportDto, ApiError> {
        self.with_session(id, |s| Ok(convert::report_dto(&s.validate()?)))
    }

    /// The player master list (view-models).
    pub fn list_players(&self, id: u32) -> Result<Vec<PlayerSummaryDto>, ApiError> {
        self.with_session(id, |s| {
            Ok(s.player_summaries()?
                .iter()
                .map(convert::player_summary_dto)
                .collect())
        })
    }

    /// The full Player detail for one `wBIz`, or `None` if absent.
    pub fn player_view(&self, id: u32, player_id: u64) -> Result<Option<PlayerViewDto>, ApiError> {
        self.with_session(id, |s| {
            Ok(s.player_view(player_id)?.as_ref().map(convert::player_view_dto))
        })
    }

    /// The team master list (view-models).
    pub fn list_teams(&self, id: u32) -> Result<Vec<TeamSummaryDto>, ApiError> {
        self.with_session(id, |s| {
            Ok(s.team_summaries()?.iter().map(convert::team_summary_dto).collect())
        })
    }

    /// The full Team detail for one `kOtt.vfEq`, or `None` if absent.
    pub fn team_view(&self, id: u32, team_id: u64) -> Result<Option<TeamViewDto>, ApiError> {
        self.with_session(id, |s| {
            Ok(s.team_view(team_id)?.as_ref().map(convert::team_view_dto))
        })
    }

    /// The arena master list (view-models).
    pub fn list_arenas(&self, id: u32) -> Result<Vec<ArenaSummaryDto>, ApiError> {
        self.with_session(id, |s| {
            Ok(s.arena_summaries()?.iter().map(convert::arena_summary_dto).collect())
        })
    }

    /// The full Arena detail for one `StEO.vfEq`, or `None` if absent.
    pub fn arena_view(&self, id: u32, arena_id: u64) -> Result<Option<ArenaViewDto>, ApiError> {
        self.with_session(id, |s| {
            Ok(s.arena_view(arena_id)?.as_ref().map(convert::arena_view_dto))
        })
    }

    /// Stage a generic validated field edit (studio ids); the value is integer or string text.
    pub fn set_field(
        &self,
        id: u32,
        table: &str,
        record: u32,
        field: &str,
        value: FieldValue,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_session(id, |s| {
            Ok(convert::outcome_dto(s.set_field(table, record, field, value)?))
        })
    }

    pub fn set_captaincy(
        &self,
        id: u32,
        team: u64,
        player: u64,
        role: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_session(id, |s| {
            Ok(convert::outcome_dto(s.set_captaincy(team, player, role)?))
        })
    }

    pub fn create_player(
        &self,
        id: u32,
        team: u64,
        position: u64,
    ) -> Result<CreatePlayerDto, ApiError> {
        self.with_session(id, |s| {
            let (pid, outcome) = s.create_player(team, position)?;
            Ok(convert::create_player_dto(pid, outcome))
        })
    }

    pub fn delete_player(&self, id: u32, player: u64) -> Result<OpOutcomeDto, ApiError> {
        self.with_session(id, |s| Ok(convert::outcome_dto(s.delete_player(player)?)))
    }

    pub fn move_player(&self, id: u32, player: u64, to_team: u64) -> Result<OpOutcomeDto, ApiError> {
        self.with_session(id, |s| Ok(convert::outcome_dto(s.move_player(player, to_team)?)))
    }

    pub fn set_jersey_number(
        &self,
        id: u32,
        team: u64,
        player: u64,
        number: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_session(id, |s| {
            Ok(convert::outcome_dto(s.set_jersey_number(team, player, number)?))
        })
    }

    pub fn set_roster_status(
        &self,
        id: u32,
        team: u64,
        player: u64,
        status: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_session(id, |s| {
            Ok(convert::outcome_dto(s.set_roster_status(team, player, status)?))
        })
    }

    /// Revert the last committed operation; `false` if nothing to undo.
    pub fn undo(&self, id: u32) -> Result<bool, ApiError> {
        self.with_session(id, |s| Ok(s.undo()))
    }

    /// Re-apply the last undone operation; `false` if nothing to redo.
    pub fn redo(&self, id: u32) -> Result<bool, ApiError> {
        self.with_session(id, |s| Ok(s.redo()))
    }

    /// Package and write the project to `out_path`; returns the number of bytes written.
    pub fn save(&self, id: u32, out_path: &str) -> Result<u64, ApiError> {
        self.with_session(id, |s| {
            let bytes = s.save()?;
            std::fs::write(out_path, &bytes).map_err(|source| ApiError::Io {
                path: out_path.to_string(),
                source,
            })?;
            Ok(bytes.len() as u64)
        })
    }

    fn with_session<R>(
        &self,
        id: u32,
        f: impl FnOnce(&mut EditSession) -> Result<R, ApiError>,
    ) -> Result<R, ApiError> {
        let mut guard = self.inner.lock().unwrap();
        let session = guard
            .sessions
            .get_mut(&id)
            .ok_or(ApiError::UnknownProject(id))?;
        f(session)
    }
}

/// Parse a platform string from the UI (`ps3` / `xbox` / `ps4` / `recomp`).
pub fn parse_platform(s: &str) -> Result<Platform, ApiError> {
    match s.to_ascii_lowercase().as_str() {
        "ps3" => Ok(Platform::Ps3),
        "xbox" | "xbox360" => Ok(Platform::Xbox),
        "ps4" => Ok(Platform::Ps4),
        "recomp" | "legacy" | "legacy-recomp" => Ok(Platform::Recomp),
        _ => Err(ApiError::BadEnum {
            what: "platform",
            value: s.to_string(),
        }),
    }
}

/// Parse a project-role string from the UI (`nhlng` / `roster` / `localization`).
pub fn parse_role(s: &str) -> Result<ProjectRole, ApiError> {
    match s.to_ascii_lowercase().as_str() {
        "nhlng" | "db" => Ok(ProjectRole::Nhlng),
        "roster" => Ok(ProjectRole::Roster),
        "localization" | "loc" => Ok(ProjectRole::Localization),
        _ => Err(ApiError::BadEnum {
            what: "role",
            value: s.to_string(),
        }),
    }
}
