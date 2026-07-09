//! The project/session registry — the in-process store of live [`Workspace`]s keyed by id, plus the
//! GUI-agnostic command logic (returns DTOs, so the Tauri wrappers in [`crate::commands`] are
//! one-liners). Lives behind a `Mutex` so it can be a single Tauri-managed state.
//!
//! Each entry is a [`Workspace`]: a canonical `nhlng.db` edit session plus an optional mirrored
//! roster-save session. Reads go to the canonical session; writes fan out to both (see
//! `studio-domain`'s `workspace`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use studio_domain::{
    compute_maxima_at, compute_person_career_ratings, compute_person_ratings, default_profile,
    AttributeMaxima, EditSession, FhmCorpus, FieldValue, NormalizationMode, Platform, Project,
    ProjectProfile, ProjectRole, Workspace,
};

use crate::convert;
use crate::dto::{
    AddCareerSeasonDto, ArenaSummaryDto, ArenaViewDto, AutocorrectSummaryDto, CareerStatsDto,
    CreatePlayerDto, EaComputedRatingsDto, ExtractSummaryDto, FormulaCatalogDto, FormulaProfileDto,
    HistoricalCareerRatingsDto, HistoricalFiltersDto, HistoricalPersonDto, HistoricalPersonSummaryDto,
    HistoricalTeamDto, HistoricalTeamFiltersDto, HistoricalTeamFranchiseDto,
    HistoricalTeamIndexEntryDto, HistoricalTeamSummaryDto, InstallReportDto, IntegrityReportDto,
    JerseyCorrectionDto, LinesViewDto, OpOutcomeDto, PlayerSummaryDto, PlayerViewDto,
    ProjectSummaryDto, TeamSummaryDto, TeamViewDto,
};
use crate::error::ApiError;

#[derive(Default)]
struct Inner {
    next: u32,
    sessions: HashMap<u32, Workspace>,
}

/// The session registry. Construct with `Registry::default()` and hand to Tauri via `.manage(...)`.
///
/// Holds the per-project edit sessions (`inner`) plus the single app-global, read-only FHM
/// historical corpus (`corpus`). The corpus is not tied to any project — it is a source you browse
/// and import *from* — so it lives beside the session map rather than inside a `Workspace`.
#[derive(Default)]
pub struct Registry {
    inner: Mutex<Inner>,
    corpus: Mutex<Option<FhmCorpus>>,
    /// Directory the open corpus lives in — where formula profiles + the maxima sidecar are persisted.
    corpus_dir: Mutex<Option<PathBuf>>,
    /// The open corpus's db file — used to compute maxima on an independent connection.
    corpus_db_path: Mutex<Option<PathBuf>>,
    /// Cached PositionMax attribute maxima for the open corpus (a full-scan; see `attribute_maxima`).
    maxima: Mutex<Option<Arc<AttributeMaxima>>>,
    /// The open corpus's person count — a cheap fingerprint that invalidates a stale maxima sidecar
    /// if the data pack is rebuilt in place.
    corpus_person_count: Mutex<Option<u64>>,
}

/// The on-disk maxima sidecar: the maxima plus the corpus fingerprint it was computed from.
#[derive(serde::Serialize, serde::Deserialize)]
struct MaximaCache {
    person_count: u64,
    maxima: AttributeMaxima,
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
        save_path: Option<&str>,
    ) -> InstallReportDto {
        let report = studio_domain::detect_install(
            game_id,
            platform,
            Path::new(root),
            save_path.map(Path::new),
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
        save_path: Option<&str>,
        big_files_extracted: bool,
        defs_root: &str,
    ) -> Result<ProjectSummaryDto, ApiError> {
        let report = studio_domain::detect_install(
            game_id,
            platform,
            Path::new(root),
            save_path.map(Path::new),
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
        // PS3/Xbox/Recomp mirror: there nhlng.db *is* the roster (record-index aligned, big-endian
        // — Recomp is Xbox-derived). On PS4 nhl_game.db is a template and the live roster is a
        // separate save — never mirror it.
        let nhlng_session = build_session(&nhlng, platform, ProjectRole::Nhlng, &meta, defs_root)?;
        let roster_slot = matches!(platform, Platform::Ps3 | Platform::Xbox | Platform::Recomp)
            .then(|| report.saves.iter().find(|s| s.kind == "roster" && s.writable))
            .flatten();
        let mut ws = match roster_slot {
            Some(slot) => {
                let roster_session =
                    build_session(&slot.path, platform, ProjectRole::Roster, &meta, defs_root)?;
                Workspace::with_roster(nhlng_session, nhlng.clone(), roster_session, slot.path.clone())?
            }
            None => Workspace::single(nhlng_session, nhlng.clone()),
        };
        ws.set_game_id(game_id);
        self.register(ws)
    }

    /// Extract a game's `.big` archives in place (Xbox / Recomp / PS4 ship the db packed). Runs at
    /// onboarding, before a project is opened, so it takes the install root rather than a project id.
    pub fn extract_install(&self, root: &str) -> Result<ExtractSummaryDto, ApiError> {
        let summary =
            studio_domain::extract_install(Path::new(root)).map_err(ApiError::Domain)?;
        Ok(convert::extract_summary_dto(&summary))
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

    /// Run the Layer 4 audit over the current working state, resolved to human-readable findings.
    pub fn validate(&self, id: u32) -> Result<IntegrityReportDto, ApiError> {
        self.with_workspace(id, |ws| Ok(convert::report_dto(&ws.primary().audit_report()?)))
    }

    /// Toggle the acknowledged corruption-override edit mode: when enabled, edits proceed on a DB that
    /// fails the structural audit (unsupported). Used by the on-load corruption prompt's override.
    pub fn set_edit_override(&self, id: u32, allow: bool) -> Result<(), ApiError> {
        self.with_workspace(id, |ws| {
            ws.set_edit_override(allow);
            Ok(())
        })
    }

    /// Auto-correct duplicate jersey numbers on load (default reassignment logic), returning a
    /// display-ready summary with team/player names resolved (human-readable-surfaces rule). The
    /// corrections are staged as a pending, undoable edit — the user reviews/saves them like any other.
    pub fn autocorrect_jerseys(&self, id: u32) -> Result<AutocorrectSummaryDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (_, corrections) =
                ws.autocorrect_duplicate_jerseys().map_err(ApiError::Domain)?;
            if corrections.is_empty() {
                return Ok(AutocorrectSummaryDto { corrections: Vec::new() });
            }
            // Resolve names off the (post-correction) working state — identities are unchanged.
            let teams: std::collections::HashMap<u64, String> = ws
                .primary()
                .team_summaries()?
                .into_iter()
                .map(|t| (t.team_id, t.name))
                .collect();
            let players: std::collections::HashMap<u64, String> = ws
                .primary()
                .player_summaries()?
                .into_iter()
                .map(|p| (p.player_id, format!("{} {}", p.first_name, p.last_name).trim().to_string()))
                .collect();
            let dtos = corrections
                .iter()
                .map(|c| JerseyCorrectionDto {
                    team: teams
                        .get(&c.team_id)
                        .filter(|n| !n.is_empty())
                        .cloned()
                        .unwrap_or_else(|| format!("Team #{}", c.team_id)),
                    player: c
                        .player_id
                        .and_then(|p| players.get(&p))
                        .filter(|n| !n.is_empty())
                        .cloned()
                        .unwrap_or_else(|| "an unknown player".to_string()),
                    old_number: c.old,
                    new_number: c.new,
                })
                .collect();
            Ok(AutocorrectSummaryDto { corrections: dtos })
        })
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

    /// A team's strategy sheet (`EeYo`), or `None` when the database lacks it.
    pub fn team_strategies(
        &self,
        id: u32,
        team_id: u64,
    ) -> Result<Option<crate::dto::TeamStrategiesDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .primary()
                .team_strategies(team_id)?
                .as_ref()
                .map(convert::team_strategies_dto))
        })
    }

    /// A team's future draft picks (`qHav`), possibly empty.
    pub fn team_draft_picks(
        &self,
        id: u32,
        team_id: u64,
    ) -> Result<Vec<crate::dto::DraftPickDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .primary()
                .team_draft_picks(team_id)?
                .iter()
                .map(convert::draft_pick_dto)
                .collect())
        })
    }

    /// A team's registered jerseys (`xjgw`), possibly empty.
    pub fn list_team_jerseys(
        &self,
        id: u32,
        team_id: u64,
    ) -> Result<Vec<crate::dto::TeamJerseyDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .primary()
                .team_jerseys(team_id)?
                .iter()
                .map(convert::team_jersey_dto)
                .collect())
        })
    }

    /// Trade a future draft pick (`qHav` row) to another team.
    pub fn trade_draft_pick(
        &self,
        id: u32,
        record: u32,
        to_team: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.trade_draft_pick(record, to_team)?))
        })
    }

    /// A team's stock equipment rows (`hHbv`), possibly empty.
    pub fn list_team_equipment(
        &self,
        id: u32,
        team_id: u64,
    ) -> Result<Vec<crate::dto::TeamEquipmentDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .primary()
                .team_equipment(team_id)?
                .iter()
                .map(convert::team_equipment_dto)
                .collect())
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

    /// Add a stock career season row for a player; returns the new record index. Mirrored into the
    /// roster save when the career table is shared and aligned.
    pub fn add_career_season(
        &self,
        id: u32,
        table: &str,
        player: u64,
        year_end: u64,
        league: u64,
        team: &str,
    ) -> Result<AddCareerSeasonDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (record, outcome) = ws.add_career_season(table, player, year_end, league, team)?;
            Ok(AddCareerSeasonDto {
                record,
                outcome: convert::outcome_dto(outcome),
            })
        })
    }

    /// Delete one stock career row by record index (ordered shift).
    pub fn delete_career_row(
        &self,
        id: u32,
        table: &str,
        record: u32,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.delete_career_row(table, record)?))
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

    /// Activate/deactivate a trait on a player's ratings record (`dSvy`/`mHuy`).
    pub fn set_player_trait(
        &self,
        id: u32,
        table: &str,
        record: u32,
        value: i64,
        active: bool,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(
                ws.set_player_trait(table, record, value, active)?,
            ))
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

    pub fn clone_player(
        &self,
        id: u32,
        source_player: u64,
        to_team: u64,
    ) -> Result<CreatePlayerDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (pid, outcome) = ws.clone_player(source_player, to_team)?;
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

    pub fn trade_slots(
        &self,
        id: u32,
        a_team: u64,
        a_player: u64,
        b_team: u64,
        b_player: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(
                ws.trade_slots(a_team, a_player, b_team, b_player)?,
            ))
        })
    }

    pub fn release_to_fa(&self, id: u32, from_team: u64, player: u64) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.release_to_fa(from_team, player)?))
        })
    }

    pub fn sign_from_fa(&self, id: u32, player: u64, to_team: u64) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.sign_from_fa(player, to_team)?))
        })
    }

    pub fn add_membership(&self, id: u32, player: u64, to_team: u64) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.add_membership(player, to_team)?))
        })
    }

    pub fn remove_membership(
        &self,
        id: u32,
        from_team: u64,
        player: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.remove_membership(from_team, player)?))
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

    pub fn retire_jersey(&self, id: u32, team: u64, number: u64) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| Ok(convert::outcome_dto(ws.retire_jersey(team, number)?)))
    }

    pub fn unretire_jersey(&self, id: u32, team: u64, number: u64) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| Ok(convert::outcome_dto(ws.unretire_jersey(team, number)?)))
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

    // ---- Historical (FHM) corpus -----------------------------------------

    /// Open an FHM historical corpus data pack (read-only), verifying its schema version. Replaces
    /// any previously-open corpus. Returns the person count for a quick sanity signal.
    pub fn open_corpus(&self, path: &str) -> Result<u64, ApiError> {
        let corpus = FhmCorpus::open(Path::new(path))?;
        let count = corpus
            .meta("person_count")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        *self.corpus_dir.lock().unwrap() =
            Path::new(path).parent().map(|p| p.to_path_buf());
        *self.corpus_db_path.lock().unwrap() = Some(PathBuf::from(path));
        // A new corpus invalidates any previously-cached maxima.
        *self.maxima.lock().unwrap() = None;
        *self.corpus_person_count.lock().unwrap() = Some(count);
        *self.corpus.lock().unwrap() = Some(corpus);
        Ok(count)
    }

    /// Whether a corpus is currently open.
    pub fn corpus_open(&self) -> bool {
        self.corpus.lock().unwrap().is_some()
    }

    /// The season + league filter options for the Historical browse dropdowns.
    pub fn historical_filters(&self) -> Result<HistoricalFiltersDto, ApiError> {
        self.with_corpus(|c| {
            Ok(convert::historical_filters_dto(&c.seasons()?, &c.leagues()?))
        })
    }

    /// The Historical Players master list (person-centric), server-side filtered.
    pub fn list_historical_players(
        &self,
        search: &str,
        goalie_filter: Option<bool>,
        limit: i64,
    ) -> Result<Vec<HistoricalPersonSummaryDto>, ApiError> {
        self.with_corpus(|c| {
            Ok(c.person_summaries(search, goalie_filter, limit)?
                .iter()
                .map(convert::historical_person_summary_dto)
                .collect())
        })
    }

    /// One person's full detail (bio timeline + career + draft + awards), or `None` if absent.
    pub fn historical_player(&self, person_id: i64) -> Result<Option<HistoricalPersonDto>, ApiError> {
        self.with_corpus(|c| {
            let current = c.latest_season()?;
            Ok(c.person_detail(person_id)?
                .as_ref()
                .map(|d| convert::historical_person_dto(d, current)))
        })
    }

    /// The Historical Teams master list for one season (season-scoped), server-side filtered.
    pub fn list_historical_teams(
        &self,
        season_year: i64,
        search: &str,
        limit: i64,
    ) -> Result<Vec<HistoricalTeamSummaryDto>, ApiError> {
        self.with_corpus(|c| {
            Ok(c.team_summaries(season_year, search, limit)?
                .iter()
                .map(convert::historical_team_summary_dto)
                .collect())
        })
    }

    /// One team-season's detail (identity + roster), or `None` if absent.
    pub fn historical_team(
        &self,
        season_year: i64,
        src_team_id: i64,
    ) -> Result<Option<HistoricalTeamDto>, ApiError> {
        self.with_corpus(|c| {
            Ok(c.team_detail(season_year, src_team_id)?
                .as_ref()
                .map(convert::historical_team_dto))
        })
    }

    // ---- Franchise-consolidated team browse (v5) -------------------------

    /// The league filter options for the franchise-consolidated team browse.
    pub fn historical_team_filters(&self) -> Result<HistoricalTeamFiltersDto, ApiError> {
        self.with_corpus(|c| Ok(convert::historical_team_filters_dto(&c.team_leagues()?)))
    }

    /// The franchise-consolidated team index (franchises + standalone teams), league-filtered.
    pub fn list_historical_franchises(
        &self,
        league: Option<&str>,
        search: &str,
        limit: i64,
    ) -> Result<Vec<HistoricalTeamIndexEntryDto>, ApiError> {
        self.with_corpus(|c| {
            Ok(c.team_index(league, search, limit)?
                .iter()
                .map(convert::historical_team_index_entry_dto)
                .collect())
        })
    }

    /// One franchise/standalone-team's detail for a season (`None` season → newest), or `None` if the
    /// key is unknown.
    pub fn historical_franchise(
        &self,
        kind: &str,
        key: &str,
        season_year: Option<i64>,
    ) -> Result<Option<HistoricalTeamFranchiseDto>, ApiError> {
        self.with_corpus(|c| {
            Ok(c.team_franchise_detail(kind, key, season_year)?
                .as_ref()
                .map(convert::historical_team_franchise_dto))
        })
    }

    // ---- Formula engine (Stage 2) ----------------------------------------

    /// The packaged default formula profile.
    pub fn formula_default(&self) -> FormulaProfileDto {
        convert::formula_profile_dto(&default_profile())
    }

    /// The static formula catalog (EA attrs, FHM vocabulary, prebuilts) for the Formulas UI.
    pub fn formula_catalog(&self) -> FormulaCatalogDto {
        convert::formula_catalog_dto()
    }

    /// Compute a person-season's EA ratings under the supplied profile. `None` when the person is
    /// absent or has no derivable position.
    pub fn compute_ratings(
        &self,
        person_id: i64,
        season_year: i64,
        profile: &FormulaProfileDto,
    ) -> Result<Option<EaComputedRatingsDto>, ApiError> {
        let profile = convert::formula_profile_from_dto(profile);
        // Only PositionMax needs the corpus maxima. Compute it *outside* `with_corpus` so the
        // (one-time, minute-long) scan never holds the corpus lock and freezes the rest of the app.
        let maxima = match profile.normalisation_mode {
            NormalizationMode::PositionMax => self.attribute_maxima()?,
            NormalizationMode::Nominal => Arc::new(AttributeMaxima::default()),
        };
        self.with_corpus(|c| {
            Ok(
                compute_person_ratings(c, person_id, season_year, &profile, &maxima)?
                    .as_ref()
                    .map(convert::ea_computed_ratings_dto),
            )
        })
    }

    /// Compute a player's cross-career EA rating compilations (Full Career + Peak 3 + per-season
    /// overalls + best season) under the supplied profile. `None` when the person has no derivable
    /// position or no playing seasons.
    pub fn compute_career_ratings(
        &self,
        person_id: i64,
        profile: &FormulaProfileDto,
    ) -> Result<Option<HistoricalCareerRatingsDto>, ApiError> {
        let profile = convert::formula_profile_from_dto(profile);
        let maxima = match profile.normalisation_mode {
            NormalizationMode::PositionMax => self.attribute_maxima()?,
            NormalizationMode::Nominal => Arc::new(AttributeMaxima::default()),
        };
        self.with_corpus(|c| {
            Ok(compute_person_career_ratings(c, person_id, &profile, &maxima)?
                .as_ref()
                .map(convert::historical_career_ratings_dto))
        })
    }

    /// The open corpus's per-position attribute maxima for PositionMax normalization. This is a full
    /// `ratings_season` scan, so it is cached three ways, cheapest first: an in-process `Arc`, a
    /// `ea_attribute_maxima.json` sidecar next to the corpus (survives restarts), and only failing
    /// both does it run the scan — on a fresh read-only connection, so it does not block other corpus
    /// queries — then persists the result.
    fn attribute_maxima(&self) -> Result<Arc<AttributeMaxima>, ApiError> {
        if let Some(m) = self.maxima.lock().unwrap().clone() {
            return Ok(m);
        }
        let fingerprint = self.corpus_person_count.lock().unwrap().unwrap_or(0);
        // Sidecar hit — but only if it was computed from this same data pack.
        if let Ok(path) = self.maxima_sidecar_path() {
            if let Some(cache) = std::fs::read_to_string(&path)
                .ok()
                .and_then(|raw| serde_json::from_str::<MaximaCache>(&raw).ok())
                .filter(|c| c.person_count == fingerprint)
            {
                let arc = Arc::new(cache.maxima);
                *self.maxima.lock().unwrap() = Some(arc.clone());
                return Ok(arc);
            }
        }
        let db_path = self
            .corpus_db_path
            .lock()
            .unwrap()
            .clone()
            .ok_or(ApiError::CorpusNotOpen)?;
        let computed = compute_maxima_at(&db_path)?;
        // Best-effort persist; a failed write just means we recompute next launch.
        if let Ok(path) = self.maxima_sidecar_path() {
            let cache = MaximaCache {
                person_count: fingerprint,
                maxima: computed.clone(),
            };
            if let Ok(json) = serde_json::to_string(&cache) {
                let _ = std::fs::write(&path, json);
            }
        }
        let arc = Arc::new(computed);
        *self.maxima.lock().unwrap() = Some(arc.clone());
        Ok(arc)
    }

    /// Path of the PositionMax maxima cache: `<corpus>/ea_attribute_maxima.json`.
    fn maxima_sidecar_path(&self) -> Result<PathBuf, ApiError> {
        let dir = self
            .corpus_dir
            .lock()
            .unwrap()
            .clone()
            .ok_or(ApiError::CorpusNotOpen)?;
        Ok(dir.join("ea_attribute_maxima.json"))
    }

    /// The active (working) formula profile — the operator's live edits, persisted next to the
    /// corpus as `ea_formulas.json`. Falls back to the packaged default when the file is absent or
    /// no corpus is open (so the Formulas UI always has something to render).
    pub fn get_active_formula(&self) -> Result<FormulaProfileDto, ApiError> {
        let Ok(path) = self.active_formula_path() else {
            return Ok(self.formula_default());
        };
        if !path.exists() {
            return Ok(self.formula_default());
        }
        let raw = std::fs::read_to_string(&path).map_err(|e| ApiError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        serde_json::from_str(&raw)
            .map_err(|e| ApiError::Workspace(format!("active formula is not valid: {e}")))
    }

    /// Persist the active (working) formula profile to `ea_formulas.json` next to the corpus.
    pub fn save_active_formula(&self, profile: &FormulaProfileDto) -> Result<(), ApiError> {
        let path = self.active_formula_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ApiError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        let json = serde_json::to_string_pretty(profile)
            .map_err(|e| ApiError::Workspace(format!("serialise active formula: {e}")))?;
        std::fs::write(&path, json).map_err(|e| ApiError::Io {
            path: path.display().to_string(),
            source: e,
        })
    }

    /// Saved formula profile names (in the corpus's `ea_formula_profiles` dir).
    pub fn list_formula_profiles(&self) -> Result<Vec<String>, ApiError> {
        let dir = self.profiles_dir()?;
        let mut names = Vec::new();
        if dir.is_dir() {
            for entry in std::fs::read_dir(&dir).map_err(|e| ApiError::Io {
                path: dir.display().to_string(),
                source: e,
            })? {
                let entry = entry.map_err(|e| ApiError::Io {
                    path: dir.display().to_string(),
                    source: e,
                })?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        names.push(stem.to_string());
                    }
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// Load a saved formula profile by name.
    pub fn load_formula_profile(&self, name: &str) -> Result<FormulaProfileDto, ApiError> {
        let path = self.profile_path(name)?;
        let raw = std::fs::read_to_string(&path).map_err(|e| ApiError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        serde_json::from_str(&raw)
            .map_err(|e| ApiError::Workspace(format!("profile {name:?} is not valid: {e}")))
    }

    /// Save a formula profile under `name` (sanitised; letters/digits/`_`/`-`).
    pub fn save_formula_profile(
        &self,
        name: &str,
        profile: &FormulaProfileDto,
    ) -> Result<(), ApiError> {
        let path = self.profile_path(name)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ApiError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        let json = serde_json::to_string_pretty(profile)
            .map_err(|e| ApiError::Workspace(format!("serialise profile: {e}")))?;
        std::fs::write(&path, json).map_err(|e| ApiError::Io {
            path: path.display().to_string(),
            source: e,
        })
    }

    /// Delete a saved formula profile.
    pub fn delete_formula_profile(&self, name: &str) -> Result<(), ApiError> {
        let path = self.profile_path(name)?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| ApiError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
        }
        Ok(())
    }

    fn profiles_dir(&self) -> Result<std::path::PathBuf, ApiError> {
        let dir = self
            .corpus_dir
            .lock()
            .unwrap()
            .clone()
            .ok_or(ApiError::CorpusNotOpen)?;
        Ok(dir.join("ea_formula_profiles"))
    }

    /// Path of the active (working) formula profile: `<corpus>/ea_formulas.json`.
    fn active_formula_path(&self) -> Result<std::path::PathBuf, ApiError> {
        let dir = self
            .corpus_dir
            .lock()
            .unwrap()
            .clone()
            .ok_or(ApiError::CorpusNotOpen)?;
        Ok(dir.join("ea_formulas.json"))
    }

    fn profile_path(&self, name: &str) -> Result<std::path::PathBuf, ApiError> {
        let mut slug = String::with_capacity(name.len());
        for ch in name.trim().chars() {
            match ch {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' => slug.push(ch),
                ' ' => slug.push('_'),
                _ => {}
            }
        }
        if slug.is_empty() {
            return Err(ApiError::Workspace("profile name is empty".into()));
        }
        Ok(self.profiles_dir()?.join(format!("{slug}.json")))
    }

    /// Run a read against the open corpus, or error if none is open.
    fn with_corpus<R>(
        &self,
        f: impl FnOnce(&FhmCorpus) -> Result<R, ApiError>,
    ) -> Result<R, ApiError> {
        let guard = self.corpus.lock().unwrap();
        let corpus = guard.as_ref().ok_or(ApiError::CorpusNotOpen)?;
        f(corpus)
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
