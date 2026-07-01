//! The project/session registry — the in-process store of live [`Workspace`]s keyed by id, plus the
//! GUI-agnostic command logic (returns DTOs, so the Tauri wrappers in [`crate::commands`] are
//! one-liners). Lives behind a `Mutex` so it can be a single Tauri-managed state.
//!
//! Each entry is a [`Workspace`]: a canonical `nhlng.db` edit session plus an optional mirrored
//! roster-save session. Reads go to the canonical session; writes fan out to both (see
//! `studio-domain`'s `workspace`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use studio_domain::{
    EditSession, FieldValue, Platform, Project, ProjectProfile, ProjectRole, Workspace,
};

use crate::convert;
use crate::dto::{
    ArenaSummaryDto, ArenaViewDto, CareerStatsDto, CreatePlayerDto, InstallReportDto,
    IntegrityReportDto, LinesViewDto, OpOutcomeDto, PlayerSummaryDto, PlayerViewDto,
    ProjectSummaryDto, TeamSummaryDto, TeamViewDto,
};
use crate::error::ApiError;

#[derive(Default)]
struct Inner {
    next: u32,
    sessions: HashMap<u32, Workspace>,
}

/// The session registry. Construct with `Registry::default()` and hand to Tauri via `.manage(...)`.
#[derive(Default)]
pub struct Registry {
    inner: Mutex<Inner>,
}

impl Registry {
    /// Load a single file as a project (no roster mirror) — the legacy / PS4 / Recomp / Advanced path.
    pub fn load_project(
        &self,
        db_path: &str,
        platform: Platform,
        role: ProjectRole,
        meta_xml: &str,
        defs_root: &str,
    ) -> Result<ProjectSummaryDto, ApiError> {
        let session = build_session(db_path, platform, role, meta_xml, defs_root)?;
        self.register(Workspace::single(session, db_path.to_string()))
    }

    /// Walk a root folder for everything a project needs (databases + save inventory) and report it,
    /// including whether a `.nhlmodproj` sidecar already exists in the resolved install dir.
    pub fn detect_install(
        &self,
        game_id: &str,
        platform: Platform,
        root: &str,
        xenia_root: Option<&str>,
    ) -> InstallReportDto {
        let report = studio_domain::detect_install(
            game_id,
            platform,
            Path::new(root),
            xenia_root.map(Path::new),
        );
        let has_existing_profile = report
            .install_dir
            .as_ref()
            .map(|d| ProjectProfile::sidecar_path(Path::new(d)).exists())
            .unwrap_or(false);
        convert::install_report_dto(&report, has_existing_profile)
    }

    /// Open a project from a root folder: resolve the file set, enforce the modding minimum
    /// (`nhlng.db` + a roster save), persist the per-project profile sidecar, and begin a workspace
    /// over the canonical `nhlng.db` — mirroring edits into the roster save when its container is
    /// writable. Returns the same summary as `load_project` (plus roster status fields).
    pub fn open_workspace(
        &self,
        game_id: &str,
        platform: Platform,
        root: &str,
        xenia_root: Option<&str>,
        big_files_extracted: bool,
        defs_root: &str,
    ) -> Result<ProjectSummaryDto, ApiError> {
        let report = studio_domain::detect_install(
            game_id,
            platform,
            Path::new(root),
            xenia_root.map(Path::new),
        );
        if !report.minimum_met {
            return Err(ApiError::Workspace(format!(
                "this folder is missing the modding minimum (an nhlng.db and a roster save). \
                 Found nhlng.db: {}, roster save: {}.",
                report.db_paths.nhlng.is_some(),
                report.saves.iter().any(|s| s.kind == "roster"),
            )));
        }
        // Safe: minimum_met guarantees nhlng is present.
        let nhlng = report.db_paths.nhlng.clone().unwrap();
        // Prefer the discovered metadata; fall back to the sibling `nhlng-meta.xml`.
        let meta = report
            .db_paths
            .meta_xml
            .clone()
            .or_else(|| {
                Path::new(&nhlng)
                    .parent()
                    .map(|d| d.join("nhlng-meta.xml").to_string_lossy().to_string())
            })
            .filter(|p| Path::new(p).exists())
            .ok_or_else(|| {
                ApiError::Workspace(
                    "no nhlng-meta.xml found beside the database; cannot open the project."
                        .to_string(),
                )
            })?;

        // Persist the profile sidecar (best-effort — a read-only install shouldn't block opening).
        if let Some(profile) = ProjectProfile::from_report(&report, big_files_extracted) {
            let _ = profile.save();
        }

        // Build the canonical session and, when a writable roster save is present, the mirror.
        // Only PS3/Xbox mirror: there nhlng.db *is* the roster (record-index aligned). On PS4
        // nhl_game.db is a template and the live roster is a separate save — never mirror it.
        let nhlng_session = build_session(&nhlng, platform, ProjectRole::Nhlng, &meta, defs_root)?;
        let roster_slot = matches!(platform, Platform::Ps3 | Platform::Xbox)
            .then(|| report.saves.iter().find(|s| s.kind == "roster" && s.writable))
            .flatten();
        let ws = match roster_slot {
            Some(slot) => {
                let roster_session =
                    build_session(&slot.path, platform, ProjectRole::Roster, &meta, defs_root)?;
                Workspace::with_roster(nhlng_session, nhlng.clone(), roster_session, slot.path.clone())?
            }
            None => Workspace::single(nhlng_session, nhlng.clone()),
        };
        self.register(ws)
    }

    /// Drop a workspace, freeing its working bytes. Returns `false` if it was not registered.
    pub fn close_project(&self, id: u32) -> bool {
        self.inner.lock().unwrap().sessions.remove(&id).is_some()
    }

    /// A fresh summary of the project (reflects pending edits / growth).
    pub fn summary(&self, id: u32) -> Result<ProjectSummaryDto, ApiError> {
        self.with_workspace(id, |ws| {
            convert::project_summary_dto(id, ws).map_err(ApiError::Domain)
        })
    }

    /// Run the Layer 4 audit over the current working state.
    pub fn validate(&self, id: u32) -> Result<IntegrityReportDto, ApiError> {
        self.with_workspace(id, |ws| Ok(convert::report_dto(&ws.primary().validate()?)))
    }

    /// The player master list (view-models).
    pub fn list_players(&self, id: u32) -> Result<Vec<PlayerSummaryDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .primary()
                .player_summaries()?
                .iter()
                .map(convert::player_summary_dto)
                .collect())
        })
    }

    /// The full Player detail for one `wBIz`, or `None` if absent.
    pub fn player_view(&self, id: u32, player_id: u64) -> Result<Option<PlayerViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .primary()
                .player_view(player_id)?
                .as_ref()
                .map(convert::player_view_dto))
        })
    }

    /// The team master list (view-models).
    pub fn list_teams(&self, id: u32) -> Result<Vec<TeamSummaryDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .primary()
                .team_summaries()?
                .iter()
                .map(convert::team_summary_dto)
                .collect())
        })
    }

    /// The full Team detail for one `kOtt.vfEq`, or `None` if absent.
    pub fn team_view(&self, id: u32, team_id: u64) -> Result<Option<TeamViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws.primary().team_view(team_id)?.as_ref().map(convert::team_view_dto))
        })
    }

    /// The arena master list (view-models).
    pub fn list_arenas(&self, id: u32) -> Result<Vec<ArenaSummaryDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .primary()
                .arena_summaries()?
                .iter()
                .map(convert::arena_summary_dto)
                .collect())
        })
    }

    /// The full Arena detail for one `StEO.vfEq`, or `None` if absent.
    pub fn arena_view(&self, id: u32, arena_id: u64) -> Result<Option<ArenaViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws.primary().arena_view(arena_id)?.as_ref().map(convert::arena_view_dto))
        })
    }

    /// A player's read-only stock career stats.
    pub fn career_stats(&self, id: u32, player_id: u64) -> Result<CareerStatsDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::career_stats_dto(&ws.primary().career_stats(player_id)?))
        })
    }

    /// The depth chart (lines) for one team, or `None` if the team is absent.
    pub fn lines_view(&self, id: u32, team_id: u64) -> Result<Option<LinesViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws.primary().lines_view(team_id)?.as_ref().map(convert::lines_view_dto))
        })
    }

    /// Stage a generic validated field edit (studio ids); the value is integer or string text.
    /// Mirrored into the roster save when the target table is shared and the files are aligned.
    pub fn set_field(
        &self,
        id: u32,
        table: &str,
        record: u32,
        field: &str,
        value: FieldValue,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.set_field(table, record, field, value)?))
        })
    }

    pub fn set_captaincy(
        &self,
        id: u32,
        team: u64,
        player: u64,
        role: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.set_captaincy(team, player, role)?))
        })
    }

    pub fn create_player(
        &self,
        id: u32,
        team: u64,
        position: u64,
    ) -> Result<CreatePlayerDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (pid, outcome) = ws.create_player(team, position)?;
            Ok(convert::create_player_dto(pid, outcome))
        })
    }

    pub fn delete_player(&self, id: u32, player: u64) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| Ok(convert::outcome_dto(ws.delete_player(player)?)))
    }

    pub fn move_player(
        &self,
        id: u32,
        player: u64,
        from_team: u64,
        to_team: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.move_player(player, from_team, to_team)?))
        })
    }

    pub fn set_jersey_number(
        &self,
        id: u32,
        team: u64,
        player: u64,
        number: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.set_jersey_number(team, player, number)?))
        })
    }

    pub fn set_roster_status(
        &self,
        id: u32,
        team: u64,
        player: u64,
        status: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.set_roster_status(team, player, status)?))
        })
    }

    /// Revert the last committed operation across both files; `false` if nothing to undo.
    pub fn undo(&self, id: u32) -> Result<bool, ApiError> {
        self.with_workspace(id, |ws| Ok(ws.undo()))
    }

    /// Re-apply the last undone operation across both files; `false` if nothing to redo.
    pub fn redo(&self, id: u32) -> Result<bool, ApiError> {
        self.with_workspace(id, |ws| Ok(ws.redo()))
    }

    /// In-place dual save: write `nhlng.db` and (if attached) the roster save back to their original
    /// paths. Returns the number of files written.
    pub fn save_workspace(&self, id: u32) -> Result<u64, ApiError> {
        self.with_workspace(id, |ws| ws.save_in_place().map_err(ApiError::Domain))
    }

    /// Write the canonical db to an explicit path ("Save As" / legacy single-file). Returns bytes.
    pub fn save(&self, id: u32, out_path: &str) -> Result<u64, ApiError> {
        self.with_workspace(id, |ws| ws.save_primary_to(out_path).map_err(ApiError::Domain))
    }

    /// Insert a workspace under a fresh id and return its summary.
    fn register(&self, ws: Workspace) -> Result<ProjectSummaryDto, ApiError> {
        let mut guard = self.inner.lock().unwrap();
        let id = guard.next;
        guard.next += 1;
        guard.sessions.insert(id, ws);
        // Safe: just inserted.
        let ws = guard.sessions.get(&id).unwrap();
        convert::project_summary_dto(id, ws).map_err(ApiError::Domain)
    }

    fn with_workspace<R>(
        &self,
        id: u32,
        f: impl FnOnce(&mut Workspace) -> Result<R, ApiError>,
    ) -> Result<R, ApiError> {
        let mut guard = self.inner.lock().unwrap();
        let ws = guard
            .sessions
            .get_mut(&id)
            .ok_or(ApiError::UnknownProject(id))?;
        f(ws)
    }
}

/// Load one file as an edit session (raw db or save container).
fn build_session(
    db_path: &str,
    platform: Platform,
    role: ProjectRole,
    meta_xml: &str,
    defs_root: &str,
) -> Result<EditSession, ApiError> {
    let project = Project::load(db_path, platform, role, meta_xml, defs_root)?;
    Ok(EditSession::begin(project))
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
