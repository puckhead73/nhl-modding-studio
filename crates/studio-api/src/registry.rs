//! The project/session registry — the in-process store of live [`Workspace`]s keyed by id, plus the
//! GUI-agnostic command logic (returns DTOs, so the Tauri wrappers in [`crate::commands`] are
//! one-liners). Lives behind a `Mutex` so it can be a single Tauri-managed state.
//!
//! Each entry is a [`Workspace`]: a canonical `nhlng.db` edit session plus an optional mirrored
//! roster-save session. Reads go to the workspace's display session (canonical by default, the
//! roster mirror when the user toggles it); writes fan out to both (see `studio-domain`'s
//! `workspace`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;

use studio_domain::{
    AssetRoots, compute_maxima_at, compute_person_career_ratings, compute_person_ratings, default_profile,
    AttributeMaxima, EditSession, FhmCorpus, FieldValue, ImportItem, NormalizationMode, Platform,
    Project, ProjectProfile, ProjectRole, Workspace,
};
use tdb_instance_registry::{InstanceRegistry, LedgerEntry};

use crate::convert;
use crate::dto::{
    AddCareerSeasonDto, ArenaSummaryDto, ArenaViewDto, AutocorrectSummaryDto, CareerStatsDto,
    AssetExportSummaryDto, AssetRootsArgsDto, AssetRootsDto, BrowseListingDto, ExportOptionsDto,
    FileExportOptionsDto, FileExportSummaryDto, HeadPreviewDto, OutfitExportSummaryDto,
    PlayerAssetsDto,
    CreatePlayerDto, EaComputedRatingsDto, ExtractSummaryDto, FormulaCatalogDto, FormulaProfileDto,
    HistoricalCareerRatingsDto, HistoricalFiltersDto, HistoricalPersonDto, HistoricalPersonSummaryDto,
    HistoricalTeamDto, HistoricalTeamFiltersDto, HistoricalTeamFranchiseDto,
    HistoricalTeamIndexEntryDto, HistoricalTeamSummaryDto, InstallReportDto, IntegrityReportDto,
    JerseyCorrectionDto, LinesViewDto, OpOutcomeDto, PlayerSummaryDto, PlayerViewDto,
    ProjectSummaryDto, RepairBlocksSummaryDto, TeamSummaryDto, TeamViewDto,
    ChangeSetPreviewDto, ImportItemDto, ImportResultDto, MatchCandidateDto,
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
    /// The non-blocking Roster Builder import ledger (`ea_import_ledger.db` beside the corpus). Opened
    /// when a corpus opens; `None`/disabled degrades to "no dedup warnings", never blocking an import.
    ledger: Mutex<Option<InstanceRegistry>>,
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

        // PS4: the live roster is a standalone LZ4 save; `nhl_game.db` is only a template (used here
        // for its metadata). When a writable roster save is present, edit **it** directly as a single
        // session (no mirror — it is not record-index-aligned with the template). Fall back to the
        // template db when no roster save is present (browse-only, as before).
        if platform == Platform::Ps4 {
            if let Some(slot) = report.saves.iter().find(|s| s.kind == "roster" && s.writable) {
                let roster_session =
                    build_session(&slot.path, platform, ProjectRole::Roster, &meta, defs_root)?;
                let mut ws = Workspace::single(roster_session, slot.path.clone());
                ws.set_game_id(game_id);
                ws.normalize_roster_capacity().map_err(ApiError::Domain)?;
                return self.register(ws);
            }
        }

        // Build the canonical session and, when a writable roster save is present, the mirror.
        // PS3/Xbox/Recomp mirror: there nhlng.db *is* the roster (record-index aligned, big-endian
        // — Recomp is Xbox-derived).
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
        // Phase 4.4: pre-allocate a packed roster save back to the vanilla ceilings
        // (docs/audits/roster-capacity.md) so edits never hit a premature capacity wall.
        // No-op on vanilla-shaped files; idempotent; never touches the main DB.
        ws.normalize_roster_capacity().map_err(ApiError::Domain)?;
        self.register(ws)
    }

    /// Open a workspace from **explicit file paths** rather than by walking an install root.
    ///
    /// This is the headless / test-harness analogue of [`Registry::open_workspace`]: it skips
    /// [`studio_domain::detect_install`] (which expects an emulator-root layout) and takes the
    /// `nhlng.db`/main-db path, an optional aligned roster-save container path, and the metadata
    /// directly. When `roster_path` is `Some` it builds the mirrored [`Workspace`] (edits fan out to
    /// both files); otherwise it opens the single main DB. Same normalization and summary as the
    /// onboarding path, so callers exercise the exact app edit seam.
    pub fn open_workspace_from_paths(
        &self,
        game_id: Option<&str>,
        platform: Platform,
        nhlng_path: &str,
        roster_path: Option<&str>,
        meta_xml: &str,
        defs_root: &str,
    ) -> Result<ProjectSummaryDto, ApiError> {
        let nhlng_session = build_session(nhlng_path, platform, ProjectRole::Nhlng, meta_xml, defs_root)?;
        let mut ws = match roster_path {
            Some(rp) => {
                let roster_session =
                    build_session(rp, platform, ProjectRole::Roster, meta_xml, defs_root)?;
                Workspace::with_roster(
                    nhlng_session,
                    nhlng_path.to_string(),
                    roster_session,
                    rp.to_string(),
                )?
            }
            None => Workspace::single(nhlng_session, nhlng_path.to_string()),
        };
        if let Some(g) = game_id {
            ws.set_game_id(g);
        }
        ws.normalize_roster_capacity().map_err(ApiError::Domain)?;
        self.register(ws)
    }

    /// Extract a game's `.big` archives in place (Xbox / Recomp / PS4 ship the db packed). Runs at
    /// onboarding, before a project is opened, so it takes the install root rather than a project id.
    pub fn extract_install(&self, root: &str) -> Result<ExtractSummaryDto, ApiError> {
        let summary =
            studio_domain::extract_install(Path::new(root)).map_err(ApiError::Domain)?;
        Ok(convert::extract_summary_dto(&summary))
    }

    // ------------------------------------------------------------ assets

    /// Infer a title's art roots from its install dir, with optional operator overrides.
    /// Takes no project id — the UI calls this to *report* whether art is available.
    pub fn asset_roots(&self, args: &AssetRootsArgsDto) -> AssetRootsDto {
        convert::asset_roots_dto(&resolve_roots(args))
    }

    /// Every player's art, resolved against the install. One `is_file` probe per candidate path,
    /// so this is a scan: call it when the page opens or the roots change, then filter in the UI.
    pub fn list_player_assets(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
    ) -> Result<Vec<PlayerAssetsDto>, ApiError> {
        let roots = resolve_roots(args);
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .player_assets(&roots)?
                .iter()
                .map(convert::player_assets_dto)
                .collect())
        })
    }

    /// Decode one portrait to a PNG data URL for preview. Takes the resolved path from
    /// [`Registry::list_player_assets`] rather than an id, so the UI never re-derives EA's
    /// bucket layout.
    pub fn portrait_preview(&self, path: &str, crop: bool) -> Result<String, ApiError> {
        let img = studio_domain::decode_portrait(Path::new(path)).map_err(ApiError::Domain)?;
        let img = if crop { img.cropped_to_opaque(0) } else { img };
        let png = img.to_png().map_err(ApiError::Domain)?;
        Ok(format!(
            "data:image/png;base64,{}",
            BASE64.encode(&png)
        ))
    }

    /// Export art for the given bio records into `out_dir`, named by player slug.
    ///
    /// `bio_records` selects the players (empty = every scanned player), so the UI's filter and
    /// selection decide the set rather than a flag matrix duplicated down here.
    pub fn export_assets(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        out_dir: &str,
        bio_records: Vec<u32>,
        opts: &ExportOptionsDto,
    ) -> Result<AssetExportSummaryDto, ApiError> {
        let roots = resolve_roots(args);
        let opts = convert::export_options(opts);
        let out_dir = out_dir.to_string();
        self.with_workspace(id, |ws| {
            let mut all = ws.display().player_assets(&roots)?;
            // Flag generic placeholder portraits across the whole roster before
            // filtering — sharing can only be seen with every player in hand.
            studio_domain::mark_generic_portraits(&mut all);
            let selected: Vec<_> = if bio_records.is_empty() {
                all
            } else {
                let wanted: std::collections::HashSet<u32> = bio_records.into_iter().collect();
                all.into_iter()
                    .filter(|p| wanted.contains(&p.bio_record))
                    .collect()
            };
            let summary =
                studio_domain::export_player_assets(&selected, Path::new(&out_dir), opts)
                    .map_err(ApiError::Domain)?;
            Ok(convert::asset_export_summary_dto(&summary))
        })
    }

    /// List one directory level of the extracted install for the file browser, rooted at the
    /// install dir (where the game's loose tree lives after onboarding extraction). `rel` is a
    /// forward-slash path relative to that root (`""` = the root); it's resolved safely so the UI
    /// can never walk outside the install. Pure filesystem — no project/workspace needed.
    pub fn browse_files(&self, install_dir: &str, rel: &str) -> Result<BrowseListingDto, ApiError> {
        let listing =
            studio_domain::browse_asset_dir(Path::new(install_dir), rel).map_err(ApiError::Domain)?;
        Ok(convert::browse_listing_dto(&listing))
    }

    /// Export the selected files/folders (forward-slash rel paths under `install_dir`) into
    /// `out_dir`, decoding recognized assets to glTF/PNG/DDS. Folders expand recursively; the output
    /// mirrors each source's path relative to the install so a title's own layout is preserved.
    pub fn export_files(
        &self,
        install_dir: &str,
        rel_paths: Vec<String>,
        out_dir: &str,
        opts: &FileExportOptionsDto,
    ) -> Result<FileExportSummaryDto, ApiError> {
        let opts = convert::file_export_options(opts);
        let summary = studio_domain::export_asset_files(
            Path::new(install_dir),
            &rel_paths,
            Path::new(out_dir),
            opts,
        )
        .map_err(ApiError::Domain)?;
        Ok(convert::file_export_summary_dto(&summary))
    }

    /// Export every team's jersey/pants/socks as PNGs named by the team, into `out_dir`. Resolves
    /// each team's id → human-readable name here (the human-readable-surfaces rule), slugs it for the
    /// filename, then hands the domain the `(id, slug)` list plus the install's `rendering/` root.
    pub fn export_team_outfits(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        out_dir: &str,
    ) -> Result<OutfitExportSummaryDto, ApiError> {
        let roots = resolve_roots(args);
        let out_dir = out_dir.to_string();
        self.with_workspace(id, |ws| {
            let teams: Vec<studio_domain::OutfitTeam> = ws
                .display()
                .team_summaries()?
                .into_iter()
                .filter_map(|t| {
                    let slug = convert::team_name_slug(&t.name);
                    (!slug.is_empty()).then_some(studio_domain::OutfitTeam {
                        team_id: t.team_id,
                        slug,
                        abbrev: t.abbrev,
                    })
                })
                .collect();
            let summary = studio_domain::export_team_outfits(
                &teams,
                Path::new(&roots.rendering),
                Path::new(&out_dir),
            )
            .map_err(ApiError::Domain)?;
            Ok(convert::outfit_export_summary_dto(&summary))
        })
    }

    /// Decode a head for the 3D viewer, resolved server-side from `head_id`
    /// (`player_<head_id>.rx2` + its texlib) — no path crosses the boundary.
    /// Works for any head with a mesh on disk, not just bespoke ones, so a
    /// generic/pool head can be inspected too. Uses the lenient parse, so NHL
    /// 10's per-submesh-rig heads render for identification.
    pub fn head_preview(
        &self,
        args: &AssetRootsArgsDto,
        head_id: u64,
    ) -> Result<HeadPreviewDto, ApiError> {
        let roots = resolve_roots(args);
        // Resolved by numeric id: RenderWare `player_{id}.rx2` or PS4's shared
        // `generic_player_{id}.mcd`. Bespoke PS4 heads are name-slug keyed and
        // surface through `list_player_assets` (which knows the player's name).
        let mesh = roots.mesh_path(head_id, None).ok_or_else(|| {
            ApiError::Workspace(format!(
                "no head mesh for id {head_id} in this install's rendering folder"
            ))
        })?;
        // Head first, then any companion meshes (NHL 09's separate hair) — merged
        // into one preview.
        let mut meshes = vec![mesh];
        meshes.extend(roots.extra_head_mesh_paths(head_id, None));
        let texs: Vec<String> = roots
            .texture_paths(head_id, None)
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let preview =
            studio_domain::decode_head_preview(&meshes, &texs).map_err(ApiError::Domain)?;
        Ok(convert::head_preview_dto(&preview))
    }

    /// Decode a head for the 3D viewer from **already-resolved paths**, rather
    /// than re-deriving them from a numeric id. This is the path PS4 bespoke
    /// heads take: they're keyed by player-name slug, so `list_player_assets`
    /// (which knows the name) resolves the `.mcd` + `.gnf`/texlib files, and the
    /// UI hands them straight here. Works for any platform — the mesh decode
    /// dispatches on `.mcd` vs `.rx2` by extension.
    pub fn head_preview_paths(
        &self,
        mesh_paths: Vec<String>,
        texture_paths: Vec<String>,
    ) -> Result<HeadPreviewDto, ApiError> {
        if mesh_paths.is_empty() {
            return Err(ApiError::Workspace("no head mesh path given".into()));
        }
        let meshes: Vec<PathBuf> = mesh_paths.iter().map(PathBuf::from).collect();
        let preview = studio_domain::decode_head_preview(&meshes, &texture_paths)
            .map_err(ApiError::Domain)?;
        Ok(convert::head_preview_dto(&preview))
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
        self.with_workspace(id, |ws| Ok(convert::report_dto(&ws.display().audit_report()?)))
    }

    /// Toggle the acknowledged corruption-override edit mode: when enabled, edits proceed on a DB that
    /// fails the structural audit (unsupported). Used by the on-load corruption prompt's override.
    pub fn set_edit_override(&self, id: u32, allow: bool) -> Result<(), ApiError> {
        self.with_workspace(id, |ws| {
            ws.set_edit_override(allow);
            Ok(())
        })
    }

    /// Switch which file the project's read views are served from: the canonical `nhlng.db`
    /// (default) or the mirrored roster save (for users who only play off the roster save). Errors
    /// only when no roster mirror is attached; on a misaligned pair the workspace runs in
    /// roster-only mode (edits hit strictly the roster file). Returns the refreshed summary so the
    /// UI can re-render off the new source in one round trip.
    pub fn set_display_roster(&self, id: u32, on: bool) -> Result<ProjectSummaryDto, ApiError> {
        self.with_workspace(id, |ws| {
            ws.set_display_roster(on).map_err(ApiError::Domain)?;
            convert::project_summary_dto(id, ws).map_err(ApiError::Domain)
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
                .display()
                .team_summaries()?
                .into_iter()
                .map(|t| (t.team_id, t.name))
                .collect();
            let players: std::collections::HashMap<u64, String> = ws
                .display()
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

    /// Repair a corrupted roster-block layout (the "Corrupted roster block" audit findings):
    /// permute the roster slot table back into contiguous per-team blocks. Order-only — no
    /// membership, jersey, or captaincy changes — staged as a pending, undoable edit and mirrored
    /// into an aligned roster save. Returns a display-ready summary with team names resolved
    /// (human-readable-surfaces rule); `moved_slots == 0` means the layout was already healthy.
    pub fn repair_roster_blocks(&self, id: u32) -> Result<RepairBlocksSummaryDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (_, summary) = ws.repair_roster_blocks().map_err(ApiError::Domain)?;
            if summary.moved_slots == 0 {
                return Ok(RepairBlocksSummaryDto { moved_slots: 0, teams: Vec::new() });
            }
            let teams: std::collections::HashMap<u64, String> = ws
                .display()
                .team_summaries()?
                .into_iter()
                .map(|t| (t.team_id, t.name))
                .collect();
            let names = summary
                .affected_teams
                .iter()
                .map(|tid| {
                    teams
                        .get(tid)
                        .filter(|n| !n.is_empty())
                        .cloned()
                        .unwrap_or_else(|| format!("Team #{tid}"))
                })
                .collect();
            Ok(RepairBlocksSummaryDto { moved_slots: summary.moved_slots, teams: names })
        })
    }

    /// The player master list (view-models).
    pub fn list_players(&self, id: u32) -> Result<Vec<PlayerSummaryDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .player_summaries()?
                .iter()
                .map(convert::player_summary_dto)
                .collect())
        })
    }

    /// The free-agent pool: every indexed player occupying **no** roster slot. Off-roster is
    /// defined by slot occupancy alone (Spike S1, docs/audits/roster-pools.md — no sentinel team,
    /// no hidden pool), so `team_id == None` on the master-list row ⇔ free agent. Same row shape
    /// as `list_players`, pre-filtered for the Transactions pool column.
    pub fn list_free_agents(&self, id: u32) -> Result<Vec<PlayerSummaryDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .player_summaries()?
                .iter()
                .filter(|p| p.team_id.is_none())
                .map(convert::player_summary_dto)
                .collect())
        })
    }

    /// The full Player detail for one `wBIz`, or `None` if absent.
    pub fn player_view(&self, id: u32, player_id: u64) -> Result<Option<PlayerViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .player_view(player_id)?
                .as_ref()
                .map(convert::player_view_dto))
        })
    }

    /// The team master list (view-models).
    pub fn list_teams(&self, id: u32) -> Result<Vec<TeamSummaryDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .team_summaries()?
                .iter()
                .map(convert::team_summary_dto)
                .collect())
        })
    }

    /// The full Team detail for one `kOtt.vfEq`, or `None` if absent.
    pub fn team_view(&self, id: u32, team_id: u64) -> Result<Option<TeamViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws.display().team_view(team_id)?.as_ref().map(convert::team_view_dto))
        })
    }

    /// The arena master list (view-models).
    pub fn list_arenas(&self, id: u32) -> Result<Vec<ArenaSummaryDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .arena_summaries()?
                .iter()
                .map(convert::arena_summary_dto)
                .collect())
        })
    }

    /// The full Arena detail for one `StEO.vfEq`, or `None` if absent.
    pub fn arena_view(&self, id: u32, arena_id: u64) -> Result<Option<ArenaViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws.display().arena_view(arena_id)?.as_ref().map(convert::arena_view_dto))
        })
    }

    /// A player's read-only stock career stats.
    pub fn career_stats(&self, id: u32, player_id: u64) -> Result<CareerStatsDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::career_stats_dto(&ws.display().career_stats(player_id)?))
        })
    }

    /// The depth chart (lines) for one team, or `None` if the team is absent.
    pub fn lines_view(&self, id: u32, team_id: u64) -> Result<Option<LinesViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws.display().lines_view(team_id)?.as_ref().map(convert::lines_view_dto))
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
                .display()
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
                .display()
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
                .display()
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
                .display()
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

    pub fn set_dressed(
        &self,
        id: u32,
        team: u64,
        player: u64,
        dressed: bool,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.set_dressed(team, player, dressed)?))
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
        let dir = Path::new(path).parent().map(|p| p.to_path_buf());
        // Open the import ledger beside the corpus (created if absent; disabled if unwritable/corrupt).
        *self.ledger.lock().unwrap() =
            dir.as_ref().map(|d| InstanceRegistry::open(d.join("ea_import_ledger.db")));
        *self.corpus_dir.lock().unwrap() = dir;
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

    // ---- Roster Builder import (Phase 5.4) — the first commands to bridge both databases ----

    /// Auto-detect EA identity matches for a corpus person against project `id` (the cart's
    /// Match/Create toggle). Needs both an open corpus and the target project.
    pub fn match_ea_player(
        &self,
        id: u32,
        person_id: i64,
    ) -> Result<Vec<MatchCandidateDto>, ApiError> {
        self.with_corpus(|corpus| {
            self.with_workspace(id, |ws| {
                Ok(ws
                    .match_ea_player(corpus, person_id)?
                    .iter()
                    .map(convert::match_candidate_dto)
                    .collect())
            })
        })
    }

    /// Dry-run a Roster Builder cart against project `id`: lower every item through the corpus adapter
    /// and report what commit would do — per-item outcomes, the player-count delta, the audit over the
    /// previewed state, blockers, and the ledger dedup flag — **without** mutating the session.
    pub fn import_preview(
        &self,
        id: u32,
        items: &[ImportItemDto],
    ) -> Result<ChangeSetPreviewDto, ApiError> {
        // Resolve the active profile + maxima *first* — both briefly lock the corpus/maxima, which must
        // be released before we hold the corpus lock across the bridge below.
        let profile = convert::formula_profile_from_dto(&self.get_active_formula()?);
        let maxima = self.maxima_for(&profile)?;
        let items: Vec<ImportItem> = items.iter().map(convert::import_item_from_dto).collect();

        self.with_corpus(|corpus| {
            self.with_workspace(id, |ws| {
                let lowered = ws.lower_change_set(corpus, &items, &profile, &maxima)?;
                let preview = ws.preview_import(&lowered.ops, lowered.skipped.clone())?;
                let target = ws.primary_path().to_string();

                let ledger = self.ledger.lock().unwrap();
                let rows: Vec<_> = lowered
                    .items
                    .iter()
                    .map(|it| {
                        let already = it.lowered() && self.was_imported(&ledger, it.person_id, it.season_year, &target);
                        convert::lowered_item_result_dto(it, already)
                    })
                    .collect();
                let blocked = lowered.items.iter().filter(|i| !i.lowered()).count();

                // S4 ceiling gate: project each roster table's fill after the cart's creates.
                let capacity = ws.roster_capacity(studio_domain::count_creates(&lowered.ops))?;
                let capacity_blocked = capacity.iter().any(|c| c.over);

                Ok(ChangeSetPreviewDto {
                    items: rows,
                    players_before: preview.players_before,
                    players_after: preview.players_after,
                    blocked,
                    capacity: capacity.iter().map(convert::table_capacity_dto).collect(),
                    capacity_blocked,
                    audit: convert::report_dto(&preview.audit),
                    skipped_fields: preview.skipped.iter().map(|s| s.field.clone()).collect(),
                })
            })
        })
    }

    /// Commit a Roster Builder cart against project `id` — the same lowering as preview, applied for
    /// real as one transaction / one undo entry (roster-mirror aware). On success, each created/matched
    /// item is recorded to the import ledger (best-effort; a ledger failure never fails the commit).
    pub fn import_commit(
        &self,
        id: u32,
        items: &[ImportItemDto],
    ) -> Result<ImportResultDto, ApiError> {
        let profile = convert::formula_profile_from_dto(&self.get_active_formula()?);
        let maxima = self.maxima_for(&profile)?;
        let items: Vec<ImportItem> = items.iter().map(convert::import_item_from_dto).collect();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        self.with_corpus(|corpus| {
            self.with_workspace(id, |ws| {
                let lowered = ws.lower_change_set(corpus, &items, &profile, &maxima)?;
                let target = ws.primary_path().to_string();

                // Refuse an over-capacity write — the game rejects a save past the vanilla ceiling, and
                // the write path would grow past it silently (S4). Preview surfaces this first.
                let capacity = ws.roster_capacity(studio_domain::count_creates(&lowered.ops))?;
                if let Some(full) = capacity.iter().find(|c| c.over) {
                    return Err(ApiError::Workspace(format!(
                        "roster capacity exceeded: {} would reach {} of {} — free space or split the import",
                        full.label,
                        full.current + full.adding,
                        full.ceiling
                    )));
                }
                let applied = ws.commit_import(&lowered.ops)?;
                // Created `wBIz`s come back in Create-op order — the same order the Created items appear.
                let mut created = applied.created.iter().copied();

                // Ledger each lowered item with its effective EA id (the new wBIz for a create, the
                // matched id for an update), and echo that id back in the result. Best-effort ledger.
                let ledger = self.ledger.lock().unwrap();
                let mut ledgered = 0usize;
                let mut items = Vec::with_capacity(lowered.items.len());
                for it in &lowered.items {
                    let ea = match it.outcome {
                        studio_domain::ItemOutcome::Created => created.next(),
                        studio_domain::ItemOutcome::Matched(pid) => Some(pid),
                        _ => None,
                    };
                    if it.lowered() {
                        if let Some(reg) = ledger.as_ref() {
                            let ok = reg
                                .record(&LedgerEntry {
                                    person_id: it.person_id,
                                    season_year: it.season_year,
                                    target: target.clone(),
                                    ea_player_id: ea.unwrap_or(0) as i64,
                                    imported_at: now,
                                })
                                .is_ok();
                            if ok {
                                ledgered += 1;
                            }
                        }
                    }
                    let mut dto = convert::lowered_item_result_dto(it, false);
                    dto.ea_player_id = ea; // the real created id for a create
                    items.push(dto);
                }
                drop(ledger);

                Ok(ImportResultDto {
                    items,
                    outcome: convert::outcome_dto(applied.outcome),
                    ledgered,
                })
            })
        })
    }

    /// The maxima a profile needs: the corpus PositionMax scan, or an empty set for Nominal.
    fn maxima_for(&self, profile: &studio_domain::FormulaProfile) -> Result<Arc<AttributeMaxima>, ApiError> {
        match profile.normalisation_mode {
            NormalizationMode::PositionMax => self.attribute_maxima(),
            NormalizationMode::Nominal => Ok(Arc::new(AttributeMaxima::default())),
        }
    }

    /// Ledger dedup probe (advisory): has this `(person, season)` already been imported to `target`?
    fn was_imported(
        &self,
        ledger: &Option<InstanceRegistry>,
        person_id: i64,
        season_year: i64,
        target: &str,
    ) -> bool {
        ledger
            .as_ref()
            .and_then(|l| l.lookup(person_id, season_year, target).ok().flatten())
            .is_some()
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

/// Infer a title's art roots from the install dir, then apply any operator overrides.
///
/// The platform drives the on-disk layout (RenderWare `rendering/`+`fe/` vs the
/// PS4 kettle tree). A missing/unrecognized platform string decodes as Xbox —
/// the pre-PS4 behavior — so older callers keep working.
fn resolve_roots(args: &AssetRootsArgsDto) -> AssetRoots {
    let platform = args
        .platform
        .as_deref()
        .and_then(|p| parse_platform(p).ok())
        .unwrap_or(Platform::Xbox);
    AssetRoots::resolve(Path::new(&args.install_dir), platform).with_overrides(
        args.rendering_override.as_deref(),
        args.fe_override.as_deref(),
    )
}
