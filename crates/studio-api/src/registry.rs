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

use studio_domain::formula::compute_person_legends_ratings;
use studio_domain::legends::{
    assign_rosters, Slot, EUROPEAN_ELITE_KEY, EUROPEAN_ELITE_NAME, WINDSOR_LEGENDS_NAME,
};
use studio_domain::{
    compute_maxima_at, compute_person_career_ratings, compute_person_ratings, default_profile,
    AssetRoots, AttributeMaxima, EditSession, FhmCorpus, FieldValue, ImportItem, NormalizationMode,
    Platform, Project, ProjectProfile, ProjectRole, Workspace,
};
use studio_domain::{
    CachedMember, CachedRating, CachedTeam, LegendsCache, LegendsCandidate, LegendsRun,
};
use tdb_instance_registry::{InstanceRegistry, LedgerEntry};

use crate::convert;
use crate::dto::{
    AddCareerSeasonDto, ArenaSummaryDto, ArenaViewDto, AssetExportSummaryDto, AssetRootsArgsDto,
    AssetRootsDto, AutocorrectSummaryDto, BrowseListingDto, CareerStatsDto, ChangeSetPreviewDto,
    CreatePlayerDto, EaComputedRatingsDto, EquipmentSlotAssetDto, EquipmentSlotDto,
    ExportOptionsDto, ExtractSummaryDto, FileExportOptionsDto, FileExportSummaryDto,
    FormulaCatalogDto, FormulaProfileDto, HeadPreviewDto, HistoricalCareerRatingsDto,
    HistoricalFiltersDto, HistoricalPersonDto, HistoricalPersonSummaryDto, HistoricalTeamDto,
    HistoricalTeamFiltersDto, HistoricalTeamFranchiseDto, HistoricalTeamIndexEntryDto,
    HistoricalTeamSummaryDto, HockeyShopItemDto, HockeyShopSyncSummaryDto, ImportItemDto,
    ImportResultDto, InstallReportDto, InstallStoreItemApplyResponseDto,
    InstallStoreItemPlanDto, InstallStoreItemRequestDto, IntegrityReportDto, JerseyCorrectionDto,
    KitPieceInputDto, KitPreviewDto, LegendsComputeSummaryDto, LegendsMemberDto, LegendsStatusDto,
    LegendsTeamSummaryDto, LinesViewDto, MatchCandidateDto, OpOutcomeDto, OutfitExportSummaryDto,
    PlayerAssetsDto, PlayerManifestSummaryDto, PlayerSummaryDto, PlayerViewDto, ProjectSummaryDto,
    RepairBlocksSummaryDto, TeamSummaryDto, TeamViewDto,
};
use crate::error::ApiError;

/// The name of the seeded starter profile.
pub const DEFAULT_PROFILE: &str = "Default";

/// Which formula profile the app is working under, and whether a Legends calculation is owed.
/// Persisted beside the corpus as `ea_formula_state.json` so the pending banner survives a restart.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FormulaState {
    pub active_profile: String,
    /// A Legends calculation is owed for `active_profile`.
    pub legends_pending: bool,
    /// The user acknowledged they can't use Legends under this profile until they calculate.
    pub legends_dismissed: bool,
}

impl Default for FormulaState {
    fn default() -> Self {
        Self {
            active_profile: DEFAULT_PROFILE.to_string(),
            legends_pending: true,
            legends_dismissed: false,
        }
    }
}

/// The single-letter roster slot stored in the Legends cache.
fn slot_code(slot: Slot) -> &'static str {
    match slot {
        Slot::Forward => "F",
        Slot::Defense => "D",
        Slot::Goalie => "G",
    }
}

/// Seconds-resolution UTC stamp (`YYYY-MM-DDTHH:MM:SSZ`) for cache bookkeeping — enough to show
/// "last computed" without pulling in a date crate.
fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Civil-from-days (Howard Hinnant's algorithm), epoch-shifted to 0000-03-01.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

#[derive(Default)]
pub(crate) struct Inner {
    next: u32,
    sessions: HashMap<u32, Workspace>,
    /// Files and workspaces held by a copied-target apply, recovery or restore in progress
    /// (S2-C-R1): every access through [`Registry::with_workspace`] to a reserved project is
    /// refused, opening a workspace on reserved files is refused, and a "Save As" into a reserved
    /// file is refused — without holding `inner` itself across the long-running file work.
    reservations: Vec<copied_target::Reservation>,
    next_reservation: u64,
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
    /// The confirmed equipment-model enum vocabularies (`defs/enums/*.toml`), loaded once and shared
    /// across Hockey Shop slot scans to resolve id → model name. DB-independent, so it lives here
    /// rather than inside a `Workspace`.
    shop_enums: Mutex<Option<Arc<studio_domain::EnumRegistry>>>,
    /// S2-C: prepared copied-target change sets (server-owned handles) and applies in flight.
    copied: Mutex<copied_target::CopiedState>,
    /// Disc-image route (R2 F2): which project is bound to which staging/image, and the
    /// server-held reviewed plans an apply must name.
    iso: Mutex<iso::IsoState>,
    /// Loose-file texture Replace (24 Sep 2026 review F1): the server-held reviews a write must
    /// name — target bytes, raster, source identity and the displayed policy.
    texture_reviews: Mutex<texture_review::TextureReviewState>,
    /// ONB-002: the project folders bound to open sessions and the deployment plans held for them.
    projects: Mutex<project::ProjectState>,
    /// ONB-002: extraction preflights and jobs (one job per destination).
    extractions: Mutex<extraction::ExtractionState>,
    /// ONB-003: installer plans held for a package installation (server-owned handles).
    packages: Mutex<package::PackageState>,
    /// The verified baseline inventory the installer takes bundled schema metadata from (R72-R1).
    /// The app sets the shipped store at startup; without one, the repository checkout is used.
    baselines: Mutex<Option<std::sync::Arc<studio_domain::BaselineStore>>>,
    /// The in-flight gen7 sample preview of each project and its cancellation flag (receipt 33): a
    /// new request for the same project cancels the previous one, and `gen7_speech_preview_cancel`
    /// raises the flag; the EA-XMA helper route polls it and stops the helper.
    gen7_preview_cancel: Mutex<HashMap<u32, Arc<std::sync::atomic::AtomicBool>>>,
    /// G3 (receipt 51): the last PS4 goalie-mask catalogue read, keyed by title, install and `boot.big` identity
    /// (length + modification time) — a vault read takes seconds.
    ps4_masks: Mutex<Option<(String, Arc<studio_domain::ps4_goalie_masks::MaskCatalogue>)>>,
    /// Receipt 52: the last PS4 skater-gear catalogue read (same key rule).
    ps4_gear: Mutex<Option<(String, Arc<studio_domain::ps4_skater_gear::SkaterGearCatalogue>)>>,
    /// Receipt 63 / 66: the PS4 head catalogue parsed from the last vault read, keyed by title and the read's **exact**
    /// source identity (member hashes and definitions hash). The members are re-read and re-identified on every call; only
    /// the parse is reused, so a cache hit never stands in for source validation.
    ps4_heads: Mutex<Option<(String, Arc<studio_domain::ps4_heads::HeadCatalogue>)>>,
    /// Receipt 66: the PS4 hair-style catalogue (same rule as `ps4_heads`).
    ps4_hair: Mutex<Option<(String, Arc<studio_domain::ps4_hair::HairCatalogue>)>>,
    /// Save Editor: the bundled targets' schema evidence (built on first identification).
    schema_index: Mutex<Option<Arc<studio_domain::save_identify::SchemaIndex>>>,
    /// Save Editor: the sessions opened by the standalone tool, with their file and backup.
    save_editor: Mutex<HashMap<u32, save_editor::SaveEditorFile>>,
}

mod copied_target;
mod iso;
mod project;
mod project_delete;
mod save_editor;
mod overview;
pub mod player_assignments;
mod team_schedule;
mod overview_revert;
mod release;
mod project_archive;
mod mod_manager;
mod team_editing;
mod package;
mod recovery;
pub mod extraction;
mod texture_review;

#[doc(hidden)]
pub use copied_target::ApplyPhase;
#[doc(hidden)]
pub use iso::IsoFault;

/// The on-disk maxima sidecar: the maxima plus the corpus fingerprint it was computed from.
#[derive(serde::Serialize, serde::Deserialize)]
struct MaximaCache {
    person_count: u64,
    maxima: AttributeMaxima,
}

/// Receipt 67: the identity definitions a PS4 display cache was built from — the defs root and the SHA-256 of its
/// `ps4/vault-identities.json` (unreadable = its own key, so the next read retries).
fn defs_key(defs_root: &Path) -> String {
    use sha2::Digest;
    match std::fs::read(defs_root.join("ps4/vault-identities.json")) {
        Ok(b) => format!("{}#{:x}", defs_root.display(), sha2::Sha256::digest(&b)),
        Err(e) => format!("{}#unreadable:{e}", defs_root.display()),
    }
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
    ///
    /// PS4 (N21-001): `save_path` is the independently chosen save-data location and `roster_save`
    /// the roster save the user chose from the inventory. With several recognised roster saves (other
    /// profiles, other regional title ids, several slots) and no choice, the open is refused rather
    /// than taking the first; a choice that is no longer in the inventory is refused too. Ignored on
    /// the other platforms.
    #[allow(clippy::too_many_arguments)]
    pub fn open_workspace(
        &self,
        game_id: &str,
        platform: Platform,
        root: &str,
        save_path: Option<&str>,
        roster_save: Option<&str>,
        big_files_extracted: bool,
        defs_root: &str,
    ) -> Result<ProjectSummaryDto, ApiError> {
        let report = studio_domain::detect_install(
            game_id,
            platform,
            Path::new(root),
            save_path.map(Path::new),
        );
        if platform == Platform::Ps4 && !report.minimum_met {
            let why: Vec<&str> = report
                .problems
                .iter()
                .filter(|p| !p.kind.starts_with("save"))
                .map(|p| p.message.as_str())
                .collect();
            return Err(ApiError::Workspace(if why.is_empty() {
                "the game's nhl_game.db and nhl_game-meta.xml weren't both found.".to_string()
            } else {
                why.join(" ")
            }));
        }
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

        // PS4: settle which roster save opens before anything is written (a refused choice leaves
        // the sidecar untouched).
        let ps4_roster = if platform == Platform::Ps4 { ps4_chosen_roster(&report, roster_save)? } else { None };

        // ONB-002: opening never writes the install-side `.nhlmodproj` any more. The project
        // folder holds the project's state; an existing sidecar is read-only legacy configuration.
        let _ = big_files_extracted;

        // PS4: the live roster is a standalone LZ4 save; `nhl_game.db` is only a template (used here
        // for its metadata). When a writable roster save is present, edit **it** directly as a single
        // session (no mirror — it is not record-index-aligned with the template). Fall back to the
        // template db when no roster save is present (browse-only, as before).
        if platform == Platform::Ps4 {
            if let Some(slot) = ps4_roster {
                let roster_session =
                    build_session(&slot.path, platform, ProjectRole::Roster, &meta, defs_root)?;
                let mut ws = Workspace::single(roster_session, slot.path.clone());
                ws.set_game_id(game_id);
                if let Some(loc) = &report.db_paths.localization {
                    ws.attach_dictionary_at_open(loc);
                }
                ws.normalize_roster_capacity().map_err(ApiError::Domain)?;
                return self.register(ws);
            }
        }

        // Build the canonical session and, when a writable roster save is present, the mirror.
        // PS3/Xbox/Recomp mirror: there nhlng.db *is* the roster (record-index aligned, big-endian
        // — Recomp is Xbox-derived).
        let nhlng_session = build_session(&nhlng, platform, ProjectRole::Nhlng, &meta, defs_root)?;
        let roster_slot = matches!(platform, Platform::Ps3 | Platform::Xbox | Platform::Recomp)
            .then(|| {
                report
                    .saves
                    .iter()
                    .find(|s| s.kind == "roster" && s.writable)
            })
            .flatten();
        let mut ws = match roster_slot {
            Some(slot) => {
                let roster_session =
                    build_session(&slot.path, platform, ProjectRole::Roster, &meta, defs_root)?;
                Workspace::with_roster(
                    nhlng_session,
                    nhlng.clone(),
                    roster_session,
                    slot.path.clone(),
                )?
            }
            None => Workspace::single(nhlng_session, nhlng.clone()),
        };
        ws.set_game_id(game_id);
        // The install's own dictionary (best effort: a missing or damaged dictionary leaves labels
        // as keys rather than blocking the open).
        if let Some(loc) = &report.db_paths.localization {
            ws.attach_dictionary_at_open(loc);
        }
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
        let nhlng_session = build_session(
            nhlng_path,
            platform,
            ProjectRole::Nhlng,
            meta_xml,
            defs_root,
        )?;
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
        let summary = studio_domain::extract_install(Path::new(root)).map_err(ApiError::Domain)?;
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
        let img = studio_domain::decode_portrait(&self.preview_source_path(Path::new(path))).map_err(ApiError::Domain)?;
        let img = if crop { img.cropped_to_opaque(0) } else { img };
        let png = img.to_png().map_err(ApiError::Domain)?;
        Ok(format!("data:image/png;base64,{}", BASE64.encode(&png)))
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
            let summary = studio_domain::export_player_assets(&selected, Path::new(&out_dir), opts)
                .map_err(ApiError::Domain)?;
            Ok(convert::asset_export_summary_dto(&summary))
        })
    }

    /// Build the appearance catalog: every player's own head + hair + facial hair,
    /// plus the non-player sweep (generics, referees, coaches, trainers), written
    /// as GLB + PNG with a `catalog.json` manifest. The game token comes from the
    /// workspace `game_id`.
    pub fn export_head_catalog(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        out_dir: &str,
        textures: bool,
    ) -> Result<crate::dto::HeadCatalogSummaryDto, ApiError> {
        let roots = resolve_roots(args);
        let out_dir = out_dir.to_string();
        self.with_workspace(id, |ws| {
            let game = ws.game_id().unwrap_or("game").to_string();
            let players = ws.display().player_assets(&roots)?;
            let opts = studio_domain::CatalogOptions {
                game_tag: game,
                textures,
            };
            let summary =
                studio_domain::export_full_catalog(&players, &roots, Path::new(&out_dir), &opts)
                    .map_err(ApiError::Domain)?;
            Ok(convert::head_catalog_summary_dto(&summary))
        })
    }

    /// Read a built appearance catalog's `catalog.json` back for the browser. Pure
    /// filesystem — no project/workspace needed; the catalog is self-describing.
    pub fn read_catalog_manifest(
        &self,
        dir: &str,
    ) -> Result<crate::dto::CatalogManifestDto, ApiError> {
        let manifest =
            studio_domain::read_catalog_manifest(Path::new(dir)).map_err(ApiError::Domain)?;
        Ok(convert::catalog_manifest_dto(&manifest))
    }

    /// Decode one catalog asset for the 3D viewer: parse its source container and
    /// keep only the submeshes the entry owns (a head, a merged hair, or facial
    /// hair), framed to that subset. `source`, `submeshes` and `texture_sources`
    /// come straight from a [`crate::dto::CatalogEntryDto`] — the texture
    /// containers are decoded and name-bound so the preview is textured like the
    /// game (falling back to flat colours when the sources are absent).
    pub fn catalog_entry_preview(
        &self,
        source: String,
        submeshes: Vec<String>,
        texture_sources: Option<Vec<String>>,
    ) -> Result<HeadPreviewDto, ApiError> {
        let src = PathBuf::from(&source);
        // Prefer the entry's recorded texture containers; when it has none — an
        // older catalog written before that field, or a caller that didn't send
        // it — re-resolve them from the source's own folder so the preview is
        // still textured without a catalog rebuild.
        let textures = match texture_sources {
            Some(t) if !t.is_empty() => t,
            _ => studio_domain::textures_for_source(&src),
        };
        let meshes = vec![self.preview_source_path(&src)];
        let textures: Vec<_> = textures.iter().map(|p| self.preview_source_path(Path::new(p)).to_string_lossy().into_owned()).collect();
        let eye = src.ancestors().find(|p| p.file_name().is_some_and(|n| n == "rendering"))
            .and_then(|root| studio_domain::decode_eye_texture(root, None));
        let preview = studio_domain::decode_head_preview_named(&meshes, &textures, &eye.into_iter().collect::<Vec<_>>(), &submeshes)
            .map_err(ApiError::Domain)?;
        Ok(convert::head_preview_dto(&preview))
    }

    /// Read a PNG file straight off disk as a `data:image/png;base64,…` URL — the
    /// exported catalog texture thumbnails, which are already PNG (no re-decode).
    pub fn read_png_data_url(&self, path: &str) -> Result<String, ApiError> {
        let bytes =
            std::fs::read(path).map_err(|e| ApiError::Workspace(format!("read {path}: {e}")))?;
        Ok(format!("data:image/png;base64,{}", BASE64.encode(&bytes)))
    }

    /// List one directory level of the extracted install for the file browser, rooted at the
    /// install dir (where the game's loose tree lives after onboarding extraction). `rel` is a
    /// forward-slash path relative to that root (`""` = the root); it's resolved safely so the UI
    /// can never walk outside the install. Pure filesystem — no project/workspace needed.
    pub fn browse_files(&self, install_dir: &str, rel: &str) -> Result<BrowseListingDto, ApiError> {
        let listing = studio_domain::browse_asset_dir(Path::new(install_dir), rel)
            .map_err(ApiError::Domain)?;
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
            let art_ids_map = ws.display().jersey_art_ids()?;
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
                        // Jersey art ids (`xjgw.GDQL`) — the outfit-filename token space; raw
                        // team-id matching mis-assigned e.g. Team Poland's files to the
                        // Portland Pirates (colliding id spaces).
                        art_ids: art_ids_map.get(&t.team_id).cloned().unwrap_or_default(),
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

    /// Decode the **current** on-disk texture at `rel_path` (safe-joined under
    /// `install_dir`) to a PNG data URL for the Replace preview — the "before"
    /// image, and the source of the file-browser row thumbnails. Reports the
    /// texture's own dimensions + container so the UI can flag a source that
    /// won't fit (the write path is same-size only). `max_edge > 0` downscales for
    /// a lightweight thumbnail (aspect preserved, never upscaled); `0` = full size.
    pub fn texture_preview(
        &self,
        install_dir: &str,
        rel_path: &str,
        max_edge: u32,
        texture_index: Option<u32>,
    ) -> Result<crate::dto::TexturePreviewDto, ApiError> {
        // ONB-002: a staged (project-owned) copy is what the preview shows; otherwise the install.
        let (target, _) = self.project_asset_target(install_dir, rel_path)?;
        let (container, img) =
            studio_domain::decode_texture_file(&target, texture_index.map(|i| i as usize))
                .map_err(ApiError::Domain)?;
        // Full-size dims describe the write target; the thumbnail is only a display scale.
        let (width, height) = (img.width, img.height);
        let png = img.thumbnail(max_edge).to_png().map_err(ApiError::Domain)?;
        Ok(crate::dto::TexturePreviewDto {
            data_url: format!("data:image/png;base64,{}", BASE64.encode(&png)),
            width,
            height,
            container: Some(container.label().to_string()),
        })
    }

    /// [`Registry::texture_preview`] for a texture inside an `EB\0\3` render archive (PS3 NHL 12 /
    /// 13 kits in `cacherender.big` / `nocacherender.big`): `archive_rel_path` is safe-joined under
    /// `install_dir`, `member_path` is matched exactly against the archive's file table, the member
    /// is decoded and the raster previewed like a loose file. Read-only; no write route exists for
    /// an archive member here.
    pub fn archive_member_texture_preview(
        &self,
        install_dir: &str,
        archive_rel_path: &str,
        member_path: &str,
        max_edge: u32,
        texture_index: Option<u32>,
    ) -> Result<crate::dto::TexturePreviewDto, ApiError> {
        let archive = studio_domain::resolve_install_path(Path::new(install_dir), archive_rel_path)
            .ok_or_else(|| ApiError::Workspace(format!("{archive_rel_path} escapes the install root")))?;
        let (container, img) = studio_domain::decode_archive_member_texture(&archive, member_path, texture_index.map(|i| i as usize)).map_err(ApiError::Domain)?;
        let (width, height) = (img.width, img.height);
        let png = img.thumbnail(max_edge).to_png().map_err(ApiError::Domain)?;
        Ok(crate::dto::TexturePreviewDto {
            data_url: format!("data:image/png;base64,{}", BASE64.encode(&png)),
            width,
            height,
            container: Some(container.label().to_string()),
        })
    }

    /// Decode an operator-supplied source image (PNG/JPG/…) to a PNG data URL for
    /// the Replace preview — the "after" image, shown beside the current texture
    /// with its dimensions so a size mismatch is caught before the write.
    pub fn source_image_preview(
        &self,
        source_image_path: &str,
    ) -> Result<crate::dto::TexturePreviewDto, ApiError> {
        let img = studio_domain::decode_source_image(Path::new(source_image_path))
            .map_err(ApiError::Domain)?;
        let (width, height) = (img.width, img.height);
        let png = img.to_png().map_err(ApiError::Domain)?;
        Ok(crate::dto::TexturePreviewDto {
            data_url: format!("data:image/png;base64,{}", BASE64.encode(&png)),
            width,
            height,
            container: None,
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
        let overlay = self.preview_overlay_root(&args.install_dir);
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
        for mesh in &mut meshes { *mesh = preview_asset_path(mesh, &args.install_dir, overlay.as_deref()); }
        let texs: Vec<String> = roots
            .texture_paths(head_id, None)
            .into_iter()
            .map(|p| preview_asset_path(&p, &args.install_dir, overlay.as_deref()).to_string_lossy().into_owned())
            .collect();
        // The eye texture ships separately (common `texlib_999`), so the eye
        // submeshes are bare without it. This id-keyed path has no player context,
        // so the default eye colour is used (per-player eyes drive the kit preview).
        let eye = studio_domain::decode_eye_texture(Path::new(&roots.rendering), None);
        let extra: Vec<_> = eye.into_iter().collect();
        let preview =
            studio_domain::decode_head_preview(&meshes, &texs, &extra).map_err(ApiError::Domain)?;
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
        let eye = mesh_paths.first().and_then(|p| Path::new(p).ancestors().find(|p| p.file_name().is_some_and(|n| n == "rendering")))
            .and_then(|root| studio_domain::decode_eye_texture(root, None));
        let meshes: Vec<PathBuf> = mesh_paths.iter().map(|p| self.preview_source_path(Path::new(p))).collect();
        let texture_paths: Vec<_> = texture_paths.iter().map(|p| self.preview_source_path(Path::new(p)).to_string_lossy().into_owned()).collect();
        let preview = studio_domain::decode_head_preview(&meshes, &texture_paths, &eye.into_iter().collect::<Vec<_>>())
            .map_err(ApiError::Domain)?;
        Ok(convert::head_preview_dto(&preview))
    }

    /// Assemble a full player **kit** for the 3D viewer from install-relative
    /// piece paths. Each piece names its mesh and texture files as forward-slash
    /// paths under `install_dir` (the same convention the Uniforms tab's outfit
    /// textures use); every path is safe-joined under the root, and any piece
    /// whose file escapes the root is rejected. Pieces are laid into one shared
    /// bind-pose model space — see `studio_domain::kit_preview`.
    pub fn kit_preview_paths(
        &self,
        install_dir: &str,
        pieces: Vec<KitPieceInputDto>,
    ) -> Result<KitPreviewDto, ApiError> {
        let root = Path::new(install_dir);
        let overlay = self.preview_overlay_root(install_dir);
        let mut inputs = Vec::with_capacity(pieces.len());
        for p in &pieces {
            let mesh_path = studio_domain::resolve_install_path(root, &p.mesh_rel_path)
                .ok_or_else(|| {
                    ApiError::Workspace(format!("{} escapes the install root", p.mesh_rel_path))
                })?;
            let mut texture_paths = Vec::with_capacity(p.texture_rel_paths.len());
            for rel in &p.texture_rel_paths {
                let t = studio_domain::resolve_install_path(root, rel).ok_or_else(|| {
                    ApiError::Workspace(format!("{rel} escapes the install root"))
                })?;
                texture_paths.push(preview_asset_path(&t, install_dir, overlay.as_deref()).to_string_lossy().into_owned());
            }
            inputs.push(studio_domain::KitPieceInput {
                kind: p.kind.clone(),
                mesh_path: preview_asset_path(&mesh_path, install_dir, overlay.as_deref()),
                texture_paths,
                prebuilt_textures: Vec::new(),
                mirror_x: false,
                raise_frac: 0.0,
            });
        }
        let preview = studio_domain::decode_kit_preview(&inputs).map_err(ApiError::Domain)?;
        Ok(convert::kit_preview_dto(&preview))
    }

    /// Assemble a team's full uniform kit for the 3D viewer: the selected
    /// jersey/pants/socks colorway (resolved server-side) on the shared body,
    /// with a representative player's head, plus the high-detail glove and a
    /// helmet. `player_id` chooses whose head is shown (the picker); when absent
    /// the top-overall active roster player stands in. `style`/`variant` select
    /// which of the team's kits to dress — the same tokens the Uniforms tab uses.
    // Mirrors the `team_kit_preview` command's parameter list one-for-one; see the note there.
    #[allow(clippy::too_many_arguments)]
    pub fn team_kit_preview(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        team_id: u64,
        style: u32,
        variant: u32,
        player_id: Option<u64>,
        player_gear: bool,
    ) -> Result<KitPreviewDto, ApiError> {
        let roots = resolve_roots(args);
        let overlay = self.preview_overlay_root(&args.install_dir);
        let rendering = PathBuf::from(&roots.rendering);
        self.with_workspace(id, |ws| {
            // Representative player: the caller's pick, else the team's top-overall
            // active skater (a stable, recognisable stand-in for the head/pose).
            // Membership-only teams (Winter Classic / all-star / national) have no
            // main-team players — fall back to their `eGlu` roster block.
            let summaries = ws.display().player_summaries()?;
            let mut roster: Vec<studio_domain::PlayerSummary> = summaries
                .iter()
                .filter(|p| p.team_id == Some(team_id))
                .cloned()
                .collect();
            if roster.is_empty() {
                let ids: std::collections::HashSet<u64> =
                    ws.display().team_roster_ids(team_id)?.into_iter().collect();
                roster = summaries
                    .into_iter()
                    .filter(|p| ids.contains(&p.player_id))
                    .collect();
            }
            let chosen = player_id
                .and_then(|pid| roster.iter().find(|p| p.player_id == pid))
                .or_else(|| {
                    roster
                        .iter()
                        .filter(|p| p.is_active != Some(false))
                        .max_by_key(|p| p.overall.unwrap_or(-1))
                })
                .or_else(|| roster.first());
            // Goalies wear an entirely different gear set (mask, pads, blocker,
            // trapper, goalie skate/sock) — drives the goalie branch below.
            let is_goalie = chosen.map(|p| p.is_goalie).unwrap_or(false);

            // Load the chosen player's full view once — it carries the bio fields
            // (head/eye/hand) and the equipment companion fields (`xmja`/`fMVl`)
            // used to resolve the player's own selected gear when `player_gear`.
            let chosen_view =
                chosen.and_then(|p| ws.display().player_view(p.player_id).ok().flatten());

            // Catching hand for the goalie's glove placement (`ubPc.GRkp`: 0 = Left,
            // 1 = Right; for goalies this is the catch hand). Defaults to a regular
            // catch-left goalie when unknown.
            let catch_left = if is_goalie {
                chosen_view
                    .as_ref()
                    .and_then(|v| {
                        v.all_fields
                            .iter()
                            .find(|f| f.studio_field == "GRkp")
                            .and_then(|f| f.decoded)
                    })
                    .map(|h| h == 0)
                    .unwrap_or(true)
            } else {
                true
            };

            // The chosen player's head model id (`ubPc.pPaD`) and eye colour
            // (`ubPc.Zufj`), read off the full player view. Absent → no head piece
            // (the kit still assembles). The eye texture ships separately from the
            // player texlib, so it's appended explicitly (else the eyes are bare).
            let head_mesh = chosen_view.as_ref().and_then(|view| {
                let field = |id: &str| {
                    view.all_fields
                        .iter()
                        .find(|f| f.studio_field == id)
                        .and_then(|f| f.decoded)
                };
                let head_id = field("pPaD")? as u64;
                let eye_color = field("Zufj").map(|v| v as u32);
                let mesh = roots.mesh_path(head_id, None)?;
                let texs = roots.texture_paths(head_id, None);
                Some((mesh, texs, eye_color, roots.extra_head_mesh_paths(head_id, None)))
            });

            // Player-selected gear resolver: read the equipment id off the player's
            // `xmja`/`fMVl` companion field, then resolve it to its render files
            // (mesh + colour map) via the equipment-slot matchers. `None` when
            // `player_gear` is off, no player is chosen, the field is absent/zero,
            // or the id ships no files — every caller falls back to the team default.
            let player_piece = |slot: &str, field: &str| -> Option<studio_domain::EquipmentPiece> {
                if !player_gear {
                    return None;
                }
                let v = chosen_view.as_ref()?;
                let raw = v
                    .equipment
                    .iter()
                    .find(|f| f.studio_field == field)
                    .and_then(|f| f.decoded)?;
                if raw < 0 {
                    return None;
                }
                studio_domain::resolve_equipment_piece(&rendering, slot, raw as u32)
            };

            // The team's outfit textures, resolved as the Uniforms tab sees them,
            // narrowed to the selected kit (by the file-name style/variant tokens).
            let art_ids_map = ws.display().jersey_art_ids()?;
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
                        // Jersey art ids (`xjgw.GDQL`) — the outfit-filename token space; raw
                        // team-id matching mis-assigned e.g. Team Poland's files to the
                        // Portland Pirates (colliding id spaces).
                        art_ids: art_ids_map.get(&t.team_id).cloned().unwrap_or_default(),
                    })
                })
                .collect();
            let outfits = studio_domain::list_team_outfits(&rendering, &teams, team_id);
            let kit_texture = |piece: &str| -> Vec<String> {
                // Pick the piece's colour-map *diffuse* for this kit's (style,
                // variant). The `sock` folder mixes real colorways
                // (`sock_<set>_<team>_<variant>_cm`) with the sock **mesh's** own
                // maps (`playersock_*_am` / `goaliesock_*_am`) — those sort ahead
                // alphabetically and their `set_variant` tokens can collide with a
                // (style, variant) match, so a naive first-match binds a grey alpha
                // map and the sock renders white. Restrict to genuine diffuses
                // (a `texlib_` bundle carries its `_cm` raster internally; loose
                // rasters must be a `_cm` colour map) and prefer the kit-matched one.
                // Nameplate sheets are excluded — they're not colorways.
                let of_piece: Vec<&studio_domain::OutfitTexture> = outfits
                    .iter()
                    .filter(|t| t.piece == piece && !t.is_nameplate)
                    .collect();
                let diffuses: Vec<&&studio_domain::OutfitTexture> = of_piece
                    .iter()
                    .filter(|t| is_colorway_diffuse(&t.file_name))
                    .collect();
                // 1) kit-matched diffuse, 2) any diffuse, 3) kit-matched of any
                // kind, 4) first entry — so a title that names things differently
                // (e.g. PS4 GNFs) still dresses the body rather than going bare.
                let matched = diffuses
                    .iter()
                    .find(|t| outfit_matches_kit(&t.file_name, style, variant))
                    .or_else(|| diffuses.first())
                    .map(|t| **t)
                    .or_else(|| {
                        of_piece
                            .iter()
                            .find(|t| outfit_matches_kit(&t.file_name, style, variant))
                            .or_else(|| of_piece.first())
                            .copied()
                    });
                matched
                    .map(|t| vec![t.abs_path.to_string_lossy().into_owned()])
                    .unwrap_or_default()
            };

            let mut meshes = studio_domain::kit_mesh_defaults(&rendering);
            meshes.jersey = studio_domain::kit_preview::jersey_mesh_for_style(&rendering, style as u64);

            // Glove/helmet diffuses are *composited* from their zone masks painted
            // with the team's `hHbv` zone colours (there are no flat glove/helmet
            // textures on disk). Skates carry no zone type, so they take a plain
            // texlib diffuse from the skate folder.
            let equipment = ws.display().team_equipment(team_id)?;
            // Link equipment to the kit: prefer the glove/helmet record whose
            // variant matches this kit's variant, falling back to the first of its
            // type. Keeps the previewed glove/helmet colours in step with the
            // kit-linked equipment cards shown in the Uniforms tab.
            let zones_of = |type_raw: u32| -> Option<(u32, Vec<[u8; 3]>)> {
                let of_type: Vec<_> = equipment
                    .iter()
                    .filter(|e| e.equipment_type_raw == type_raw)
                    .collect();
                let rec = of_type
                    .iter()
                    .find(|e| e.variant_raw == variant)
                    .or_else(|| of_type.first())?;
                Some((
                    rec.brand_raw,
                    rec.zones.iter().map(|z| [z.red, z.green, z.blue]).collect(),
                ))
            };

            // Skate diffuse: the first texlib in the skate folder (per-player skate
            // texturing needs a skate-id map not yet built — a later pass). Shared by
            // the skater and goalie skate meshes.
            let skate_tex: Vec<String> = first_skate_texlib(&rendering.join("skate"))
                .map(|p| vec![p.to_string_lossy().into_owned()])
                .unwrap_or_default();

            let mut pieces: Vec<studio_domain::KitPieceInput> = Vec::new();
            let mut push =
                |kind: &str,
                 mesh: Option<PathBuf>,
                 textures: Vec<String>,
                 prebuilt: Vec<studio_domain::PreviewTexture>| {
                    if let Some(mesh_path) = mesh {
                        pieces.push(studio_domain::KitPieceInput {
                            kind: kind.to_string(),
                            mesh_path,
                            texture_paths: textures,
                            prebuilt_textures: prebuilt,
                            mirror_x: false,
                            raise_frac: 0.0,
                        });
                    }
                };

            // Colorway pieces every kit shares: pants + jersey (reused as-is for
            // goalies — there is no goalie-specific jersey/pant cut) and the head.
            push(
                "pants",
                meshes.pant.clone(),
                kit_texture("pant"),
                Vec::new(),
            );
            push(
                "jersey",
                meshes.jersey.clone(),
                kit_texture("jersey"),
                Vec::new(),
            );
            if let Some((mesh, texs, eye_color, companions)) = head_mesh {
                // The player's eye texture (eyes_{Zufj}_cm from the common library),
                // added as a prebuilt texture so the eye submeshes bind it.
                let eye = studio_domain::decode_eye_texture(&rendering, eye_color);
                let textures: Vec<String> = texs.into_iter().map(|p| p.to_string_lossy().into_owned()).collect();
                let eyes: Vec<_> = eye.into_iter().collect();
                // Early titles keep hair in a companion file rather than the head container.
                for path in std::iter::once(mesh).chain(companions) {
                    push("head", Some(path), textures.clone(), eyes.clone());
                }
            }

            if is_goalie {
                // ── Goalie gear ─────────────────────────────────────────────────
                // Zone-painted pieces (blocker / trapper / pads) composite exactly
                // like skater gloves — brand masks painted by the team's `hHbv`
                // zone colours. The `hHbv` equipment-type codes for goalie gear are
                // a FLAGGED BEST-GUESS (see `goalie_zone_type`); the mask takes a
                // team-keyed `goaliehelmet_<style>_<team>_cm` paint directly.
                let g = studio_domain::goalie_kit_mesh_defaults(&rendering);
                let composite = |dir: &str, fe_prefix: &str, type_raw: u32, name: &str| {
                    zones_of(type_raw).and_then(|(brand, zones)| {
                        let (masks, detail) =
                            resolve_goalie_masks(&rendering.join(dir), fe_prefix, brand);
                        studio_domain::composite_equipment_texture(
                            &masks,
                            detail.as_deref(),
                            &zones,
                            name,
                            zones.first().copied(),
                        )
                    })
                };
                let blocker_tex =
                    composite("blocker", "feblocker", GOALIE_TYPE_BLOCKER, "blocker_cm");
                let trapper_tex =
                    composite("trapper", "fetrapper", GOALIE_TYPE_TRAPPER, "trapper_cm");
                let pad_tex =
                    composite("goaliepad", "fegoaliepad", GOALIE_TYPE_PADS, "goaliepad_cm");
                // Early masks use a separate paint selector and a longer filename
                // grammar. Missing paints stay untextured instead of borrowing a team.
                let team_art = ws.display().team_view(team_id)?.and_then(|t| t.logo_artid).unwrap_or(team_id);
                let early_mask = matches!(ws.game_id(), Some("nhl07" | "nhl08" | "nhl09"));
                let paint = if early_mask && player_gear {
                    chosen_view.as_ref().and_then(|v| v.equipment.iter().chain(&v.all_fields).find(|f| f.name == "maskcolor"))
                        .and_then(|f| f.decoded).filter(|v| *v >= 0).map(|v| v as u64).unwrap_or(team_art)
                } else { team_art };
                let mask_tex = studio_domain::kit_preview::goalie_mask_paint(&rendering, ws.game_id(), 0, paint);

                push("skates", g.skate, skate_tex, Vec::new());
                push("socks", g.sock, kit_texture("sock"), Vec::new());
                // Pads / blocker / trapper: the player's selected model (`fMVl.mIUA`
                // / `fMVl.Kmxz` / `fMVl.OTJR`) — its own mesh + colour map — else the
                // pinned default mesh painted by the team's hHbv zones.
                match player_piece("pads", "mIUA") {
                    Some(p) if p.mesh_path.is_some() => {
                        push("pads", p.mesh_path, p.texture_paths, Vec::new())
                    }
                    _ => push("pads", g.pad, Vec::new(), pad_tex.into_iter().collect()),
                }
                match player_piece("blocker", "Kmxz") {
                    Some(p) if p.mesh_path.is_some() => {
                        push("blocker", p.mesh_path, p.texture_paths, Vec::new())
                    }
                    _ => push(
                        "blocker",
                        g.blocker,
                        Vec::new(),
                        blocker_tex.into_iter().collect(),
                    ),
                }
                match player_piece("trapper", "OTJR") {
                    Some(p) if p.mesh_path.is_some() => {
                        push("trapper", p.mesh_path, p.texture_paths, Vec::new())
                    }
                    _ => push(
                        "trapper",
                        g.trapper,
                        Vec::new(),
                        trapper_tex.into_iter().collect(),
                    ),
                }
                // JOlt identifies paint only in later games; NHL08/09 use maskcolor.
                let mask_tex_final = if early_mask { mask_tex } else { player_piece("goalie-helmet", "JOlt")
                    .filter(|p| !p.texture_paths.is_empty())
                    .map(|p| p.texture_paths)
                    .unwrap_or(mask_tex) };
                push("mask", g.mask, mask_tex_final, Vec::new());
                // The cage is bare metal — no paint; it renders with the flat piece colour.
                push("cage", g.cage, Vec::new(), Vec::new());
            } else {
                // ── Skater gear ─────────────────────────────────────────────────
                // Glove/helmet diffuses are *composited* from their zone masks
                // painted with the team's `hHbv` zone colours (there are no flat
                // glove/helmet textures on disk).
                let glove_tex = zones_of(5).and_then(|(brand, zones)| {
                    let (masks, detail) = resolve_glove_masks(&rendering.join("glove"), brand);
                    studio_domain::composite_equipment_texture(
                        &masks,
                        detail.as_deref(),
                        &zones,
                        "glove_cm",
                        None, // gloves: unpainted pixels stay transparent (ref app)
                    )
                });
                let helmet_tex = zones_of(8).and_then(|(_brand, zones)| {
                    let (style, variant) =
                        meshes.helmet.as_deref().and_then(helmet_style_variant)?;
                    let hdir = rendering.join("helmet");
                    let mask = render_platform_file(hdir.join(format!("helmet_0_{style}_tm.rx2")));
                    let masks: Vec<PathBuf> = mask.is_file().then_some(mask).into_iter().collect();
                    let detail = render_platform_file(hdir.join(format!("helmet_0_{style}_{variant}_dm.rx2")));
                    let detail = detail.is_file().then_some(detail);
                    studio_domain::composite_equipment_texture(
                        &masks,
                        detail.as_deref(),
                        &zones,
                        "helmet_cm",
                        zones.first().copied(), // helmet: main colour as the background
                    )
                });
                // Skates share one mesh across ids; swap in the player's own skate
                // texlib (`xmja.AOOe`) when set, else the team's first skate texlib.
                let skate_tex_final = player_piece("skates", "AOOe")
                    .filter(|p| !p.texture_paths.is_empty())
                    .map(|p| p.texture_paths)
                    .unwrap_or(skate_tex);
                push("skates", meshes.skate.clone(), skate_tex_final, Vec::new());
                push(
                    "socks",
                    meshes.sock.clone(),
                    kit_texture("sock"),
                    Vec::new(),
                );
                // Gloves/helmet: the player's selected model (`xmja.NmUR`/`xmja.YUZp`)
                // — its own mesh + colour map — else the pinned default mesh painted
                // by the team's hHbv zones (the Uniforms-tab behaviour).
                match player_piece("gloves", "NmUR") {
                    Some(p) if p.mesh_path.is_some() => {
                        push("gloves", p.mesh_path, p.texture_paths, Vec::new())
                    }
                    _ => push(
                        "gloves",
                        meshes.glove.clone(),
                        Vec::new(),
                        glove_tex.into_iter().collect(),
                    ),
                }
                // Helmet: the player's selected model (`xmja.YUZp`) is a BUNDLE — the shell mesh
                // (`helmet_<ID>_<v>`) plus a separate chinstrap mesh (`chinstrap_<ID>_…`). Push
                // BOTH (the generic first-mesh pick used to land on the alphabetically-first
                // chinstrap, rendering a floating strap with no helmet), and add the visor when
                // the player wears one (`xmja.BFnc`, 0 = none). Falls back to the pinned default
                // shell painted by the team's hHbv zones.
                let equip_field = |field: &str| -> Option<i64> {
                    chosen_view
                        .as_ref()?
                        .equipment
                        .iter()
                        .find(|f| f.studio_field == field)
                        .and_then(|f| f.decoded)
                };
                let mut shell_pushed = false;
                if player_gear {
                    if let Some(raw) = equip_field("YUZp").filter(|&v| v >= 0) {
                        if let Some(h) =
                            studio_domain::resolve_helmet_pieces(&rendering, raw as u32)
                        {
                            if let Some(shell) = h.shell {
                                push("helmet", Some(shell), h.texture_paths.clone(), Vec::new());
                                shell_pushed = true;
                            }
                            if let Some(strap) = h.chinstrap {
                                push("chinstrap", Some(strap), h.texture_paths, Vec::new());
                            }
                        }
                    }
                    // Visor (`xmja.BFnc` visortype, 0 = none): best-effort mesh from the helmet
                    // folder; renders with the flat piece colour (no visor paint maps).
                    if let Some(t) = equip_field("BFnc").filter(|&v| v > 0) {
                        if let Some(v) = studio_domain::resolve_visor_mesh(&rendering, t as u32) {
                            push("visor", Some(v), Vec::new(), Vec::new());
                        }
                    }
                }
                if !shell_pushed {
                    push(
                        "helmet",
                        meshes.helmet.clone(),
                        Vec::new(),
                        helmet_tex.into_iter().collect(),
                    );
                }
            }

            if is_goalie {
                // Place the two hand pieces on opposite hands (both blocker and
                // trapper meshes are authored on the +X hand) and lift the mask off
                // the forehead. Convention: **+X is the character's LEFT hand**, so
                // the catch glove (trapper) sits at +X for a catch-LEFT goalie and
                // the blocker mirrors to the right; a catch-RIGHT goalie is the
                // mirror image. If the hands read swapped in-game, flip this pair.
                for pc in pieces.iter_mut() {
                    match pc.kind.as_str() {
                        "mask" | "cage" => pc.raise_frac = MASK_RAISE_FRAC,
                        "trapper" => pc.mirror_x = !catch_left,
                        "blocker" => pc.mirror_x = catch_left,
                        _ => {}
                    }
                }
            }

            if ws.game_id() == Some("nhl07") {
                let art_id=ws.display().team_view(team_id)?.and_then(|t|t.logo_artid).unwrap_or(team_id);
                studio_domain::kit_preview::complete_nhl07_equipment(&mut pieces,&rendering,art_id,variant);
            }
            if is_goalie && matches!(ws.game_id(), Some("nhl08" | "nhl09")) {
                studio_domain::kit_preview::pair_early_equipment(&mut pieces);
            }
            if ws.game_id() == Some("nhl08") && !is_goalie {
                // NHL08's equipment type codes precede the later 5/8 glove/helmet codes.
                studio_domain::kit_preview::complete_nhl08_equipment(
                    &mut pieces, &rendering, zones_of(4), zones_of(7).map(|(_, zones)| zones),
                );
            }
            if ws.game_id() == Some("nhl09") && !is_goalie {
                studio_domain::kit_preview::complete_nhl09_equipment(&mut pieces, &rendering, zones_of(4), zones_of(7).map(|(_, zones)| zones));
            }
            if matches!(ws.game_id(), Some("nhl10" | "nhl11" | "nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy")) && !is_goalie {
                let player_glove = if player_gear { chosen_view.as_ref().and_then(|v| v.equipment.iter().find(|f| f.studio_field == "NmUR")).and_then(|f| f.decoded).filter(|v| *v >= 0).map(|v| v as u32) } else { None };
                studio_domain::kit_preview::complete_modern_equipment(&mut pieces, &rendering, zones_of(5), zones_of(8).map(|(_, zones)| zones), player_glove);
            }
            if pieces.is_empty() {
                return Err(ApiError::Workspace(
                    "no kit pieces could be resolved for this team in the install's rendering tree"
                        .into(),
                ));
            }
            for piece in &mut pieces {
                piece.mesh_path = preview_asset_path(&piece.mesh_path, &args.install_dir, overlay.as_deref());
                for texture in &mut piece.texture_paths {
                    *texture = preview_asset_path(Path::new(texture), &args.install_dir, overlay.as_deref()).to_string_lossy().into_owned();
                }
            }
            let preview = studio_domain::decode_kit_preview(&pieces).map_err(ApiError::Domain)?;
            Ok(convert::kit_preview_dto(&preview))
        })
    }

    // ── Hockey Shop ─────────────────────────────────────────────────────────────

    /// The Hockey Shop slot catalog (static — resolved names, no workspace needed).
    pub fn equipment_slots(&self) -> Vec<EquipmentSlotDto> {
        studio_domain::SLOT_SPECS
            .iter()
            .map(|s| EquipmentSlotDto {
                key: s.key.to_string(),
                label: s.label.to_string(),
                category: if is_goalie_slot(s.key) {
                    "goalie"
                } else {
                    "skater"
                }
                .to_string(),
                field_ref: format!("{}.{}", s.table, s.field),
                max_id: s.max_id,
            })
            .collect()
    }

    /// The confirmed equipment-model enum vocabularies, loaded once from the bundled `defs/` tree and
    /// cached. Degrades to an empty registry (all ids show as raw numbers) if the dir is unreadable.
    fn shop_enums(&self, defs_root: &Path) -> Arc<studio_domain::EnumRegistry> {
        let mut guard = self.shop_enums.lock().unwrap();
        if let Some(reg) = guard.as_ref() {
            return reg.clone();
        }
        let reg = Arc::new(
            studio_domain::EnumRegistry::from_dir(defs_root.join("enums")).unwrap_or_default(),
        );
        *guard = Some(reg.clone());
        reg
    }

    /// Scan the install's rendering tree for one slot's items (id → render files), resolving each
    /// id's model name from the slot's enum vocabulary. Needs the asset roots plus the bundled `defs`
    /// dir (for the vocabularies) — both independent of the open DB.
    pub fn list_equipment_slot_assets(
        &self,
        args: &AssetRootsArgsDto,
        slot: &str,
        game_id: Option<&str>,
        defs_root: &Path,
    ) -> Result<Vec<EquipmentSlotAssetDto>, ApiError> {
        let spec = &studio_domain::slot_spec(slot)
            .ok_or_else(|| ApiError::Workspace(format!("unknown equipment slot `{slot}`")))?
            .for_title(game_id);
        let roots = resolve_roots(args);
        let rendering = PathBuf::from(&roots.rendering);
        // Title-scoped catalogues: labels only for the title the caller names (none for an
        // unknown title — the ids stay numbers).
        let enums = self
            .shop_enums(defs_root)
            .scoped(game_id, args.platform.as_deref().unwrap_or("xbox"));
        Ok(studio_domain::list_equipment_slot_assets(&rendering, spec)
            .iter()
            .map(|a| {
                let label = studio_domain::slot_enum_label(spec, &enums, a.id);
                convert::equipment_slot_asset_dto(a, label)
            })
            .collect())
    }

    /// Equipment 4.5 G2 (receipt 42): the team's goalie mask paints with the gen7 file check.
    /// The PS4 goalie-mask catalogue of the attached install for `title` (cached; identity-checked by the domain).
    fn ps4_mask_catalogue(&self, install_dir: &Path, defs_root: &Path, title: &str) -> Result<Arc<studio_domain::ps4_goalie_masks::MaskCatalogue>, String> {
        let boot = install_dir.join("databigs").join("boot.big");
        let meta = std::fs::metadata(&boot).map_err(|e| format!("{}: {e}", boot.display()))?;
        // Receipt 67: display cache only (no write consumes it). Title, install and the identity definitions (by content) are in the
        // key; `boot.big` is keyed by length / time, a recorded display limitation (review F2 fixed the write paths separately).
        let key = format!("{title}|{}|{}|{:?}|{}", install_dir.display(), meta.len(), meta.modified().ok(), defs_key(defs_root));
        if let Some((k, c)) = self.ps4_masks.lock().unwrap().as_ref() {
            if *k == key {
                return Ok(c.clone());
            }
        }
        let cat = Arc::new(studio_domain::ps4_goalie_masks::MaskCatalogue::load(install_dir, defs_root, title).map_err(|e| e.to_string())?);
        *self.ps4_masks.lock().unwrap() = Some((key, cat.clone()));
        Ok(cat)
    }

    /// Receipt 66 (review F2): a vault-derived catalogue for a **write** path. The attached install's four vault members and
    /// the identity definitions are read and hashed on every call and must match the open title; the cached parse is reused
    /// only when that exact identity (not file length / time) equals the one it was built from.
    fn ps4_vault_catalogue<T>(
        slot: &Mutex<Option<(String, Arc<T>)>>,
        install_dir: &Path,
        defs_root: &Path,
        title: &str,
        build: impl FnOnce(studio_domain::ps4_vault::VaultSource, [Vec<u8>; 4]) -> studio_domain::Result<T>,
    ) -> Result<Arc<T>, String> {
        let (source, bytes) = studio_domain::ps4_vault::read_install_vault(install_dir, defs_root).map_err(|e| e.to_string())?;
        source.require_title(title).map_err(|e| e.to_string())?;
        let key = format!("{title}|{}", source.identity_key());
        if let Some((k, c)) = slot.lock().unwrap().as_ref() {
            if *k == key {
                return Ok(c.clone());
            }
        }
        let cat = Arc::new(build(source, bytes).map_err(|e| e.to_string())?);
        *slot.lock().unwrap() = Some((key, cat.clone()));
        Ok(cat)
    }

    fn ps4_head_catalogue(&self, install_dir: &Path, defs_root: &Path, title: &str) -> Result<Arc<studio_domain::ps4_heads::HeadCatalogue>, String> {
        Self::ps4_vault_catalogue(&self.ps4_heads, install_dir, defs_root, title, |s, b| studio_domain::ps4_heads::HeadCatalogue::from_members(title, s, b))
    }

    fn ps4_hair_catalogue(&self, install_dir: &Path, defs_root: &Path, title: &str) -> Result<Arc<studio_domain::ps4_hair::HairCatalogue>, String> {
        Self::ps4_vault_catalogue(&self.ps4_hair, install_dir, defs_root, title, |s, b| studio_domain::ps4_hair::HairCatalogue::from_members(title, s, b))
    }

    fn ps4_gear_catalogue(&self, install_dir: &Path, defs_root: &Path, title: &str) -> Result<Arc<studio_domain::ps4_skater_gear::SkaterGearCatalogue>, String> {
        let boot = install_dir.join("databigs").join("boot.big");
        let meta = std::fs::metadata(&boot).map_err(|e| format!("{}: {e}", boot.display()))?;
        // Receipt 67: display cache only (no write consumes it). Title, install and the identity definitions (by content) are in the
        // key; `boot.big` is keyed by length / time, a recorded display limitation (review F2 fixed the write paths separately).
        let key = format!("{title}|{}|{}|{:?}|{}", install_dir.display(), meta.len(), meta.modified().ok(), defs_key(defs_root));
        if let Some((k, c)) = self.ps4_gear.lock().unwrap().as_ref() {
            if *k == key {
                return Ok(c.clone());
            }
        }
        let cat = Arc::new(studio_domain::ps4_skater_gear::SkaterGearCatalogue::load(install_dir, defs_root, title).map_err(|e| e.to_string())?);
        *self.ps4_gear.lock().unwrap() = Some((key, cat.clone()));
        Ok(cat)
    }

    /// Receipt 52: a PS4 skater's gear numbers against the title's own Creation Zone lists and vault enums.
    pub fn ps4_skater_gear(&self, id: u32, args: &AssetRootsArgsDto, player_id: u64, defs_root: &Path) -> Result<crate::dto::Ps4SkaterGearDto, ApiError> {
        let (game_id, platform, values, is_skater) = self.with_workspace(id, |ws| {
            let Some(view) = ws.display().player_view(player_id)? else {
                return Err(ApiError::Workspace(format!("no player with id {player_id}")));
            };
            let values: Vec<(&'static str, i64)> = studio_domain::ps4_skater_gear::FAMILIES
                .iter()
                .filter_map(|(fam, field, ..)| view.equipment.iter().find(|f| f.name == *field).and_then(|f| f.decoded).map(|v| (*fam, v)))
                .collect();
            Ok((ws.game_id().map(str::to_string), ws.display().project().platform, values, view.kind != "goalie"))
        })?;
        let label = |f: &str| match f {
            "helmet" => "Helmet",
            "gloves" => "Gloves",
            "skates" => "Skates",
            "stick" => "Stick",
            _ => "Visor type",
        };
        let mut dto = crate::dto::Ps4SkaterGearDto {
            applies: platform == Platform::Ps4 && is_skater && !values.is_empty(),
            items: values.iter().map(|(f, v)| crate::dto::Ps4SkaterGearItemDto { family: f.to_string(), label: label(f).into(), value: *v, class: None, text: format!("{} {v}", label(f)), key: None, name: None, thumbnail_archive: None, thumbnail_member: None }).collect(),
            catalogue: None,
            problem: None,
            notes: vec!["Names are the title's own text for its list keys (its English dictionary); which item the game draws is not observed. Stick handedness is not derived.".into()],
        };
        if !dto.applies {
            return Ok(dto);
        }
        let Some(title) = game_id else {
            dto.problem = Some("the open project has no recorded title".into());
            return Ok(dto);
        };
        if args.install_dir.trim().is_empty() {
            dto.problem = Some("attach the title's install to check the gear against its own lists".into());
            return Ok(dto);
        }
        match self.ps4_gear_catalogue(Path::new(&args.install_dir), defs_root, &title) {
            Err(e) => dto.problem = Some(e),
            Ok(cat) => {
                for it in dto.items.iter_mut() {
                    if let Some(c) = cat.check(&it.family, it.value) {
                        it.class = Some(c.class);
                        it.text = c.text;
                        it.key = c.key;
                        it.name = c.name;
                        if let Some(t) = c.thumbnail {
                            it.thumbnail_archive = Some(t.archive);
                            it.thumbnail_member = Some(t.member);
                        }
                    }
                }
                dto.catalogue = Some(format!(
                    "{} lists: {}",
                    cat.title,
                    cat.families.iter().filter_map(|f| f.rows_source.as_ref().map(|(m, s)| format!("{} {}…", m.rsplit('/').next().unwrap_or(m), &s[..12]))).collect::<Vec<_>>().join(", ")
                ));
                if let Some(n) = &cat.names {
                    dto.notes.push(format!("Names from {}'s English dictionary in {} ({}…).", n.title, n.archive, &n.sha256[..12]));
                }
                if let Some(p) = &cat.names_problem {
                    dto.notes.push(format!("Names are not shown: {p}"));
                }
            }
        }
        Ok(dto)
    }

    /// A Creation Zone equipment thumbnail of the attached install as a PNG data URL (receipt 52).
    pub fn ps4_gear_thumbnail(&self, args: &AssetRootsArgsDto, archive: &str, member: &str) -> Result<crate::dto::VenueRasterPreviewDto, ApiError> {
        if args.install_dir.trim().is_empty() {
            return Err(ApiError::Workspace("attach the title's install".into()));
        }
        let thumb = studio_domain::ps4_skater_gear::Thumb { archive: archive.to_string(), member: member.to_string() };
        let (width, height, png) = studio_domain::ps4_skater_gear::thumbnail_png(Path::new(&args.install_dir), &thumb, 256).map_err(ApiError::Domain)?;
        use base64::Engine as _;
        Ok(crate::dto::VenueRasterPreviewDto { width, height, png_data_url: format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)) })
    }

    /// G3 (receipt 51): a PS4 goalie's (maskid, masktype) pair against the title's own catalogue.
    pub fn ps4_goalie_mask_pair(&self, id: u32, args: &AssetRootsArgsDto, player_id: u64, defs_root: &Path) -> Result<crate::dto::Ps4GoalieMaskPairDto, ApiError> {
        let (game_id, platform, maskid, helmet_type, is_goalie) = self.with_workspace(id, |ws| {
            let Some(view) = ws.display().player_view(player_id)? else {
                return Err(ApiError::Workspace(format!("no player with id {player_id}")));
            };
            let find = |n: &str| view.equipment.iter().find(|f| f.name == n).and_then(|f| f.decoded);
            Ok((ws.game_id().map(str::to_string), ws.display().project().platform, find("maskid"), find("masktype"), view.kind == "goalie"))
        })?;
        let mut dto = crate::dto::Ps4GoalieMaskPairDto {
            applies: platform == Platform::Ps4 && is_goalie,
            maskid,
            helmet_type,
            class: None,
            text: String::new(),
            pack_debug_name: None,
            colour_map: None,
            catalogue: None,
            catalogue_problem: None,
            mask_name: None,
            notes: vec!["Which mask the game draws, and whether it rejects a pair without a catalogue record, is not observed (card G-MASK-1).".into()],
            thumbnail_archive: None,
            thumbnail_member: None,
            thumbnail_basis: None,
        };
        if !dto.applies {
            return Ok(dto);
        }
        let (Some(m), Some(h)) = (maskid, helmet_type) else {
            dto.text = "this goalie row carries no mask / helmet-type fields".into();
            return Ok(dto);
        };
        let Some(title) = game_id else {
            dto.catalogue_problem = Some("the open project has no recorded title".into());
            dto.text = format!("Mask {m} · helmet type {h}");
            return Ok(dto);
        };
        if args.install_dir.trim().is_empty() {
            dto.catalogue_problem = Some("attach the title's install to check the pair against its own catalogue".into());
            dto.text = format!("Mask {m} · helmet type {h}");
            return Ok(dto);
        }
        match self.ps4_mask_catalogue(Path::new(&args.install_dir), defs_root, &title) {
            Err(e) => {
                dto.catalogue_problem = Some(e);
                dto.text = format!("Mask {m} · helmet type {h}");
            }
            Ok(cat) => {
                let c = cat.classify(m, h);
                dto.class = Some(c.code().to_string());
                dto.text = c.describe(m, h);
                if let studio_domain::ps4_goalie_masks::PairClass::Unique(p) = &c {
                    dto.pack_debug_name = p.debug_name.clone();
                    dto.colour_map = p.colour_map.clone();
                    if let Some((t, basis)) = cat.thumbnail(m, h) {
                        dto.thumbnail_archive = Some(t.archive.clone());
                        dto.thumbnail_member = Some(t.member.clone());
                        dto.thumbnail_basis = Some(basis.as_str().to_string());
                    }
                    dto.mask_name = cat.name(m, h).map(str::to_string);
                }
                if let Some(p) = &cat.names_problem {
                    dto.notes.push(format!("Mask names are not shown: {p}"));
                }
                dto.catalogue = Some(format!(
                    "{} vault in {} ({})",
                    cat.title,
                    cat.source.archive,
                    cat.source.members.iter().map(|(n, s)| format!("{n} {}…", &s[..12])).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        Ok(dto)
    }

    pub fn team_goalie_masks(&self, id: u32, team_id: u64, args: &AssetRootsArgsDto) -> Result<crate::dto::TeamGoalieMasksDto, ApiError> {
        self.team_goalie_masks_with_defs(id, team_id, args, None)
    }

    /// [`Registry::team_goalie_masks`] with `defs_root`, so PS4 paints list the helmet types their design exists for
    /// (G3, receipt 51) when the install's vault is the title's own.
    pub fn team_goalie_masks_with_defs(&self, id: u32, team_id: u64, args: &AssetRootsArgsDto, defs_root: Option<&Path>) -> Result<crate::dto::TeamGoalieMasksDto, ApiError> {
        let (title, platform) = self.with_workspace(id, |ws| Ok((ws.game_id().map(str::to_string), ws.display().project().platform)))?;
        let (cat, cat_note) = match (platform, defs_root, title.as_deref(), args.install_dir.trim().is_empty()) {
            (Platform::Ps4, Some(d), Some(t), false) => match self.ps4_mask_catalogue(Path::new(&args.install_dir), d, t) {
                Ok(c) => (Some(c), Some(format!("Helmet types per paint from the {t} vault catalogue; a slot has no helmet type of its own, and how slots combine with a goalie's own pair is not observed (G-MASK-1)."))),
                Err(e) => (None, Some(format!("The title's mask catalogue was not read: {e}"))),
            },
            _ => (None, None),
        };
        let roots = resolve_roots(args);
        let rendering = PathBuf::from(&roots.rendering);
        self.with_workspace(id, |ws| {
            let Some(view) = ws.display().team_view(team_id)? else {
                return Err(ApiError::Workspace(format!("no team with id {team_id}")));
            };
            let game_id = ws.game_id().map(str::to_string);
            let platform = ws.display().project().platform;
            let root = (!roots.rendering.is_empty()).then_some(rendering.as_path());
            let m = studio_domain::team_goalie_masks::team_goalie_masks(&view.all_fields, root, game_id.as_deref(), platform);
            Ok(crate::dto::TeamGoalieMasksDto {
                applies: m.applies,
                catalogue_ids: m.catalogue_ids,
                notes: m.notes,
                paints: m
                    .paints
                    .into_iter()
                    .map(|p| crate::dto::TeamMaskPaintDto {
                        slot: p.slot,
                        role: p.role.to_string(),
                        field_name: p.field_name,
                        field_ref: format!("{}.{}", p.studio_table, p.studio_field),
                        value: p.value,
                        file_count: p.files.len(),
                        files: p.files.into_iter().take(12).collect(),
                        status: p.status.as_str().to_string(),
                        note: p.note,
                        helmet_types: match (&cat, p.value) {
                            (Some(c), Some(v)) if v > 0 => Some(c.paint_helmet_types(v as i64)),
                            _ => None,
                        },
                    })
                    .collect(),
            })
            .map(|mut d| {
                d.notes.extend(cat_note.clone());
                d
            })
        })
    }

    /// Phase 4 (21 Sep 2026): a player's equipment ids checked against the install's rendering
    /// tree (research 2026-09-21 handoff 12 — the catalogue is the file names).
    pub fn player_equipment_files(&self, id: u32, player_id: u64, args: &AssetRootsArgsDto, defs_root: &Path) -> Result<Vec<crate::dto::EquipmentFileCheckDto>, ApiError> {
        let shop_enums = self.shop_enums(defs_root);
        let roots = resolve_roots(args);
        let rendering = PathBuf::from(&roots.rendering);
        self.with_workspace(id, |ws| {
            let Some(view) = ws.display().player_view(player_id)? else {
                return Err(ApiError::Workspace(format!("no player with id {player_id}")));
            };
            let game_id = ws.game_id().map(str::to_string);
            let platform = ws.display().project().platform;
            // The project's own title scopes the catalogue labels.
            let enums = shop_enums.scoped(game_id.as_deref(), platform.as_str());
            Ok(studio_domain::check_player_equipment_files(&rendering, view.kind == "goalie", &view.equipment, &enums, game_id.as_deref(), platform)
                .into_iter()
                .map(|c| crate::dto::EquipmentFileCheckDto {
                    slot: c.slot.to_string(),
                    label: c.label.to_string(),
                    field_ref: format!("{}.{}", c.studio_table, c.studio_field),
                    folder: c.folder,
                    id: c.id,
                    model_label: c.model_label,
                    file_count: c.files.len(),
                    files: c.files.into_iter().take(24).collect(),
                    catalogue_ids: c.catalogue_ids,
                    catalogue_min: c.catalogue_min,
                    catalogue_max: c.catalogue_max,
                    note: c.note,
                    set_contract: match &c.set_contract {
                        studio_domain::equipment_components::SetContract::Measured { .. } => "measured",
                        studio_domain::equipment_components::SetContract::SingleContainer { .. } => "single_container",
                        studio_domain::equipment_components::SetContract::NotMeasured { .. } => "not_measured",
                    }
                    .to_string(),
                    set_rule: c.set_contract.describe(),
                    component_groups: c
                        .component_groups
                        .into_iter()
                        .map(|g| crate::dto::ComponentGroupDto { group: g.group, status: g.status, present: g.present, missing: g.missing })
                        .collect(),
                })
                .collect())
        })
    }

    /// Assemble the 3D preview for one equipment item (its mesh + colour map, plus
    /// tape/cage). Errors when the item is texture-only (no decodable mesh) — the
    /// frontend then shows the flat texture tiles instead.
    pub fn equipment_item_preview(
        &self,
        args: &AssetRootsArgsDto,
        slot: &str,
        id: u32,
    ) -> Result<KitPreviewDto, ApiError> {
        if studio_domain::slot_spec(slot).is_none() {
            return Err(ApiError::Workspace(format!(
                "unknown equipment slot `{slot}`"
            )));
        }
        let roots = resolve_roots(args);
        let rendering = PathBuf::from(&roots.rendering);
        let mut pieces = studio_domain::slot_preview_pieces(&rendering, slot, id);
        let overlay = self.preview_overlay_root(&args.install_dir);
        for piece in &mut pieces {
            piece.mesh_path = preview_asset_path(&piece.mesh_path, &args.install_dir, overlay.as_deref());
            for texture in &mut piece.texture_paths {
                *texture = preview_asset_path(Path::new(texture), &args.install_dir, overlay.as_deref()).to_string_lossy().into_owned();
            }
        }
        if pieces.is_empty() {
            return Err(ApiError::Workspace(format!(
                "no decodable 3D mesh for {slot} id {id} in the install's rendering tree"
            )));
        }
        let preview = studio_domain::decode_kit_preview(&pieces).map_err(ApiError::Domain)?;
        Ok(convert::kit_preview_dto(&preview))
    }

    /// List a slot's Hockey Shop store items (disk-backed PNG library).
    pub fn hockey_shop_list(
        &self,
        store_root: &str,
        slot: &str,
    ) -> Result<Vec<HockeyShopItemDto>, ApiError> {
        crate::hockey_shop::list(store_root, slot)
    }

    /// Extract a representative texture per game equipment id into the store, naming each entry
    /// from the slot's enum vocabulary (loaded from the bundled `defs/`) and crediting the game the
    /// texture came from.
    pub fn hockey_shop_sync(
        &self,
        project_id: Option<u32>,
        args: &AssetRootsArgsDto,
        store_root: &str,
        slot: &str,
        defs_root: &Path,
        game_label: Option<&str>,
        game_id: Option<&str>,
    ) -> Result<HockeyShopSyncSummaryDto, ApiError> {
        let roots = resolve_roots(args);
        let rendering = PathBuf::from(&roots.rendering);
        // Receipt 47: the open project's title is authoritative; a different requested title is refused.
        let project = self.project_title(project_id)?;
        let resolved = crate::hockey_shop::resolve_title(slot, game_id, project.as_deref())?;
        crate::hockey_shop::refuse_recompiled(resolved.as_deref(), self.recompiled_context(project_id, args)?)?;
        let game_id = resolved.as_deref();
        // Item names come from the title's own catalogue only (title-scoped vocabularies).
        let enums = self
            .shop_enums(defs_root)
            .scoped(game_id, args.platform.as_deref().unwrap_or("xbox"));
        crate::hockey_shop::sync(&rendering, store_root, slot, &enums, game_label, game_id)
    }

    /// Ingest an operator PNG as a new store item.
    pub fn hockey_shop_import(
        &self,
        store_root: &str,
        slot: &str,
        name: &str,
        author: &str,
        image_base64: &str,
    ) -> Result<HockeyShopItemDto, ApiError> {
        crate::hockey_shop::import(store_root, slot, name, author, image_base64)
    }

    /// Remove a store item (manifest entry + its PNG).
    pub fn hockey_shop_delete(
        &self,
        store_root: &str,
        slot: &str,
        entry_id: &str,
    ) -> Result<(), ApiError> {
        crate::hockey_shop::delete(store_root, slot, entry_id)
    }

    /// One store item's PNG as a data URL for the frontend preview tile.
    pub fn hockey_shop_load_texture(
        &self,
        store_root: &str,
        slot: &str,
        entry_id: &str,
    ) -> Result<String, ApiError> {
        crate::hockey_shop::load_texture_data_url(store_root, slot, entry_id)
    }

    /// Read-only plan for installing a store item into the game at a target id.
    pub fn hockey_shop_install_plan(
        &self,
        project_id: Option<u32>,
        args: &AssetRootsArgsDto,
        store_root: &str,
        req: &InstallStoreItemRequestDto,
    ) -> Result<InstallStoreItemPlanDto, ApiError> {
        let roots = resolve_roots(args);
        let rendering = PathBuf::from(&roots.rendering);
        let title = self.project_title(project_id)?;
        crate::hockey_shop::refuse_recompiled(title.as_deref().or(req.game_id.as_deref()), self.recompiled_context(project_id, args)?)?;
        crate::hockey_shop::plan_install(&rendering, store_root, req, title.as_deref())
    }

    /// Receipt 67: whether a Hockey Shop game route targets Legacy Recompiled — the open project's platform, or the attached
    /// roots' platform. (The title is checked separately.)
    fn recompiled_context(&self, project_id: Option<u32>, args: &AssetRootsArgsDto) -> Result<bool, ApiError> {
        let by_roots = args.platform.as_deref().is_some_and(|p| parse_platform(p).ok() == Some(Platform::Recomp));
        let by_project = match project_id {
            None => false,
            Some(id) => self.with_workspace(id, |ws| Ok(ws.display().project().platform == Platform::Recomp))?,
        };
        Ok(by_roots || by_project)
    }

    /// The open project's recorded title (receipt 47): the Hockey Shop names files by it, never by a guess.
    fn project_title(&self, project_id: Option<u32>) -> Result<Option<String>, ApiError> {
        match project_id {
            None => Ok(None),
            Some(id) => self.with_workspace(id, |ws| Ok(ws.game_id().map(str::to_string))),
        }
    }

    /// Apply an install: clone the donor (create mode) + splice the store texture
    /// into the target colour map (writes `.rx2` files into the install, `.bak` first).
    pub fn hockey_shop_install_apply(
        &self,
        project_id: Option<u32>,
        args: &AssetRootsArgsDto,
        store_root: &str,
        req: &InstallStoreItemRequestDto,
    ) -> Result<InstallStoreItemApplyResponseDto, ApiError> {
        let roots = resolve_roots(args);
        let rendering = PathBuf::from(&roots.rendering);
        let title = self.project_title(project_id)?;
        crate::hockey_shop::refuse_recompiled(title.as_deref().or(req.game_id.as_deref()), self.recompiled_context(project_id, args)?)?;
        // ONB-002: the Hockey Shop installer writes donor clones, the colour map and its ledger
        // into the installation's rendering tree. Under a project every edit output must stay in
        // the project, and this route is not yet redirected into the overlay; it is refused here
        // rather than left as a hidden direct writer (recorded gap, ONB-002 receipt).
        if let Some((id, dir)) = self.project_for_install_dir(&args.install_dir) {
            return Err(ApiError::Workspace(format!(
                "Hockey Shop install writes into the game installation's rendering tree, which project {id} ({}) keeps untouched until an explicit deployment. This route is not yet routed into the project folder; it stays unavailable while the project is open",
                dir.display()
            )));
        }
        crate::hockey_shop::apply_install(&rendering, store_root, req, title.as_deref())
    }

    /// Drop a workspace, freeing its working bytes. Returns `false` if it was not registered — or
    /// if a copied-target operation currently holds it (its session state is still needed for the
    /// rebind/refresh at the end of that operation; close it afterwards).
    ///
    /// Closing a project also discards its prepared copied-target handles (S2-C-R2): none of them
    /// could be applied without the session, so their engine sets are aborted and their staging
    /// removed now rather than retained indefinitely. A handle that cannot be discarded leaves the
    /// project open (`false`); its staging is a tracked leftover in the copied-target status.
    pub fn close_project(&self, id: u32) -> bool {
        self.close_project_outcome(id).outcome == "closed"
    }

    /// [`Registry::close_project`] with the reason spelled out (LEAGUE-SMOKE-R1): the boolean
    /// conflated "nothing to close" with "refused", and the shell needs to tell them apart —
    /// `already_closed` completes the UI's close transition, `busy` / `failed` keep the owner.
    pub fn close_project_outcome(&self, id: u32) -> crate::dto::CloseProjectDto {
        let dto = |outcome: &str, reason: Option<String>| crate::dto::CloseProjectDto {
            project_id: id,
            outcome: outcome.to_owned(),
            reason,
        };
        let busy = |holder: &str| {
            dto(
                "busy",
                Some(format!(
                    "{holder} still needs this session; wait for it to finish or cancel it, then close again"
                )),
            )
        };
        {
            let guard = self.inner.lock().unwrap();
            if !guard.sessions.contains_key(&id) {
                drop(guard);
                // ONB-003: an assets-only project has a folder binding (and the project lock) but
                // no session; closing releases the binding.
                if self.is_project_binding(id) {
                    self.forget_project_binding(id);
                    return dto("closed", None);
                }
                return dto("already_closed", None);
            }
            if let Some(r) = guard.reservation_for_project(id) {
                return busy(r.holder());
            }
        }
        if let Err(e) = self.discard_prepared_for_project(id) {
            return dto(
                "failed",
                Some(format!(
                    "a prepared copied-target review could not be discarded ({e}); the project stays open and the review's staging is listed under the copied root's status"
                )),
            );
        }
        let mut guard = self.inner.lock().unwrap();
        if let Some(r) = guard.reservation_for_project(id) {
            return busy(r.holder());
        }
        match guard.sessions.remove(&id) {
            Some(_) => {
                drop(guard);
                self.iso.lock().unwrap().forget_project(id);
                self.forget_project_binding(id);
                self.forget_save_editor(id);
                dto("closed", None)
            }
            None => dto("already_closed", None),
        }
    }

    /// A fresh summary of the project (reflects pending edits / growth).
    pub fn summary(&self, id: u32) -> Result<ProjectSummaryDto, ApiError> {
        if !self.has_session(id) {
            // ONB-003 / R71-R4: an assets-only project folder has no edit session; its summary
            // is the synthetic assets-only one, never a database-workspace error.
            if let Some(dto) = self.asset_only_summary(id) {
                return Ok(dto);
            }
        }
        self.with_workspace(id, |ws| {
            convert::project_summary_dto(id, ws).map_err(ApiError::Domain)
        })
    }

    /// Run the Layer 4 audit over the current working state, resolved to human-readable findings.
    pub fn validate(&self, id: u32) -> Result<IntegrityReportDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::report_dto(&ws.display().audit_report()?))
        })
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
            let (_, corrections) = ws
                .autocorrect_duplicate_jerseys()
                .map_err(ApiError::Domain)?;
            if corrections.is_empty() {
                return Ok(AutocorrectSummaryDto {
                    corrections: Vec::new(),
                });
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
                .map(|p| {
                    (
                        p.player_id,
                        format!("{} {}", p.first_name, p.last_name)
                            .trim()
                            .to_string(),
                    )
                })
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
                return Ok(RepairBlocksSummaryDto {
                    moved_slots: 0,
                    teams: Vec::new(),
                });
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
            Ok(RepairBlocksSummaryDto {
                moved_slots: summary.moved_slots,
                teams: names,
            })
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
            let explicit = studio_domain::explicit_free_agents(ws.display().bytes())
                .map_err(|e| ApiError::Workspace(e.to_string()))?;
            Ok(ws
                .display()
                .player_summaries()?
                .iter()
                .filter(|p| explicit.as_ref().map_or(p.team_id.is_none(), |pool| pool.contains(&p.player_id)))
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

    /// The pending review: baseline ↔ working for the primary database and the attached roster.
    pub fn pending_changes(
        &self,
        id: u32,
        limit: usize,
    ) -> Result<crate::dto::PendingReviewDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (primary, roster) = ws.pending_changes(limit);
            let primary_path = ws.primary_path().to_string();
            let primary_label = format!(
                "Main database ({})",
                Path::new(&primary_path)
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default()
            );
            let roster_dto = match (roster, ws.roster_path()) {
                (Some(p), Some(path)) => Some(convert::pending_file_dto(
                    "roster",
                    &format!(
                        "Roster save ({})",
                        Path::new(path)
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    ),
                    path,
                    &p,
                )),
                _ => None,
            };
            Ok(crate::dto::PendingReviewDto {
                dirty: ws.is_dirty(),
                primary: convert::pending_file_dto("primary", &primary_label, &primary_path, &primary),
                roster: roster_dto,
                dictionary: ws
                    .pending_dictionary()
                    .as_ref()
                    .map(convert::pending_dictionary_dto),
                settings_csv: (!ws.settings_csv_edits().is_empty()).then(|| ws.settings_csv_edits().iter().map(convert::pending_settings_csv_dto).collect()),
                settings_csv_archive: ws.settings_csv_working_set().filter(|w| !w.edits.is_empty()).map(|_| "databigs/inxp_00.big".to_string()),
                asset_members: (!ws.asset_member_edits().is_empty()).then(|| ws.asset_member_edits().iter().map(|p| convert::asset_member_plan_dto("", p)).collect()),
            })
        })
    }

    /// Attach the target's localization dictionary (the file discovery resolved for this install,
    /// or an explicit path) to the project for league label resolution.
    pub fn attach_dictionary(
        &self,
        id: u32,
        path: &str,
    ) -> Result<crate::dto::DictionaryRefDto, ApiError> {
        self.refuse_in_save_editor(id, "Attaching a localization dictionary")?;
        self.with_workspace(id, |ws| {
            Ok(convert::dictionary_ref_dto(
                &ws.attach_dictionary(path).map_err(ApiError::Domain)?,
            ))
        })
    }

    /// Read current dictionary text, including the pending edit, without creating a preview.
    pub fn dictionary_texts(&self, id: u32, keys: &[String]) -> Result<Vec<Option<String>>, ApiError> {
        self.with_workspace(id, |ws| {
            let dictionary = ws.display().dictionary().ok_or_else(|| ApiError::Domain(studio_domain::DomainError::Validation("No language dictionary is attached".into())))?;
            Ok(keys.iter().map(|key| {
                dictionary.dictionary.text(key).or_else(|| {
                    let hash = tdb_loc::loc_hash(key);
                    let mut matches = dictionary.dictionary.rows.iter().filter(|row| row.hash == hash);
                    let row = matches.next()?;
                    if matches.next().is_some() { return None; }
                    row.text.as_deref()
                }).map(str::to_owned)
            }).collect())
        })
    }

    /// Preview (read-only) a dictionary rename: key `key` would show `donor`'s text (LOC-W1).
    pub fn dictionary_rename_preview(
        &self,
        id: u32,
        key: &str,
        donor: &str,
    ) -> Result<crate::dto::DictionaryRepointPreviewDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::dictionary_repoint_preview_dto(
                &ws.dictionary_repoint_preview(key, donor)
                    .map_err(ApiError::Domain)?,
            ))
        })
    }

    /// Hold the rename as the project's pending dictionary change (replacing any previous one).
    /// Labels resolve through the edited dictionary; nothing is written until a save.
    pub fn dictionary_rename_set(
        &self,
        id: u32,
        key: &str,
        donor: &str,
    ) -> Result<crate::dto::DictionaryRepointPreviewDto, ApiError> {
        self.refuse_in_save_editor(id, "Renaming through the localization dictionary")?;
        self.with_workspace(id, |ws| {
            Ok(convert::dictionary_repoint_preview_dto(
                &ws.set_dictionary_repoint(key, donor)
                    .map_err(ApiError::Domain)?,
            ))
        })
    }

    /// LOC-W2: preview (read-only) an in-place free-text re-encode of `key`'s text.
    pub fn dictionary_retext_preview(
        &self,
        id: u32,
        key: &str,
        text: &str,
    ) -> Result<crate::dto::DictionaryRetextPreviewDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::dictionary_retext_preview_dto(
                &ws.dictionary_retext_preview(key, text).map_err(ApiError::Domain)?,
            ))
        })
    }

    /// LOC-W2: hold an in-place free-text re-encode as the pending dictionary change.
    pub fn dictionary_retext_set(
        &self,
        id: u32,
        key: &str,
        text: &str,
    ) -> Result<crate::dto::DictionaryRetextPreviewDto, ApiError> {
        self.refuse_in_save_editor(id, "Editing localization text")?;
        self.with_workspace(id, |ws| {
            Ok(convert::dictionary_retext_preview_dto(
                &ws.set_dictionary_retext(key, text).map_err(ApiError::Domain)?,
            ))
        })
    }

    /// LOC-W3: preview (read-only) a growth of the attached dictionary.
    pub fn dictionary_growth_preview(&self, id: u32, ops: &[crate::dto::DictionaryGrowthOpDto]) -> Result<crate::dto::DictionaryGrowthPreviewDto, ApiError> {
        self.dictionary_growth_preview_with(id, ops, None)
    }

    /// [`Registry::dictionary_growth_preview`] with the serializer option (`preserving` /
    /// `rebuild` / `rebuild-tree`).
    pub fn dictionary_growth_preview_with(&self, id: u32, ops: &[crate::dto::DictionaryGrowthOpDto], options: Option<&crate::dto::DictionaryGrowthOptionsDto>) -> Result<crate::dto::DictionaryGrowthPreviewDto, ApiError> {
        let ops = convert::growth_ops(ops)?;
        let how = convert::growth_serializer(options)?;
        self.with_workspace(id, |ws| Ok(convert::dictionary_growth_preview_dto(&ws.dictionary_growth_preview_with(&ops, how).map_err(ApiError::Domain)?)))
    }

    /// LOC-W3: hold a growth as the pending dictionary change.
    pub fn dictionary_growth_set(&self, id: u32, ops: &[crate::dto::DictionaryGrowthOpDto]) -> Result<crate::dto::DictionaryGrowthPreviewDto, ApiError> {
        self.dictionary_growth_set_with(id, ops, None)
    }

    /// [`Registry::dictionary_growth_set`] with the serializer option.
    pub fn dictionary_growth_set_with(&self, id: u32, ops: &[crate::dto::DictionaryGrowthOpDto], options: Option<&crate::dto::DictionaryGrowthOptionsDto>) -> Result<crate::dto::DictionaryGrowthPreviewDto, ApiError> {
        self.refuse_in_save_editor(id, "Adding localization text")?;
        let ops = convert::growth_ops(ops)?;
        let how = convert::growth_serializer(options)?;
        self.with_workspace(id, |ws| Ok(convert::dictionary_growth_preview_dto(&ws.set_dictionary_growth_with(&ops, how).map_err(ApiError::Domain)?)))
    }

    /// Drop the pending dictionary rename. `false` when none was held.
    pub fn dictionary_rename_clear(&self, id: u32) -> Result<bool, ApiError> {
        self.with_workspace(id, |ws| Ok(ws.clear_dictionary_repoint()))
    }

    /// Deliberately detach the dictionary (and any pending rename) — the intentional action a
    /// failed reload never performs on its own (R1 F2). `false` when none was attached.
    pub fn detach_dictionary(&self, id: u32) -> Result<bool, ApiError> {
        self.with_workspace(id, |ws| Ok(ws.detach_dictionary()))
    }

    /// The loaded target's league catalog (every league identity, resolved per target).
    pub fn list_leagues(&self, id: u32) -> Result<crate::dto::LeagueCatalogDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::league_catalog_dto(
                &ws.display().league_catalog()?,
            ))
        })
    }

    /// One league's detail by native code, or `None` if no such league exists on this target.
    pub fn league_view(
        &self,
        id: u32,
        code: u64,
    ) -> Result<Option<crate::dto::LeagueViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .league_view(code)?
                .as_ref()
                .map(convert::league_view_dto))
        })
    }

    /// Phase 1.2: the membership choices for a team (leagues; conferences/divisions where the
    /// title has a competition table) and how the title models them.
    pub fn team_membership_options(
        &self,
        id: u32,
        team_id: u64,
    ) -> Result<crate::dto::MembershipOptionsDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::membership_options_dto(
                &ws.team_membership_options(team_id).map_err(ApiError::Domain)?,
            ))
        })
    }

    /// Phase 1.2: move a team to a league with validated group ids — one transaction, one undo
    /// entry; refused with a plain reason when the league or the groups do not fit this title.
    pub fn set_team_membership(
        &self,
        id: u32,
        team_id: u64,
        league_code: u64,
        conference: Option<u64>,
        division: Option<u64>,
    ) -> Result<crate::dto::MembershipResultDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (plan, outcome) = ws
                .set_team_membership(team_id, league_code, conference, division)
                .map_err(ApiError::Domain)?;
            Ok(crate::dto::MembershipResultDto {
                team_id: plan.team_id,
                table: plan.table.clone(),
                record: plan.record,
                writes: plan.writes.clone(),
                before: convert::membership_state_dto(&plan.before),
                after: convert::membership_state_dto(&plan.after),
                notes: plan.notes.clone(),
                outcome: convert::outcome_dto(outcome),
            })
        })
    }

    /// Phase 2.3: a team's dictionary key families and texts beside its database strings.
    pub fn team_text(&self, id: u32, team_id: u64) -> Result<Option<crate::dto::TeamTextDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws.display().team_text(team_id)?.map(|t| crate::dto::TeamTextDto {
                team_id: t.team_id,
                artabbr: t.artabbr,
                db_fullname: t.db_fullname,
                db_shortname: t.db_shortname,
                db_abbrname: t.db_abbrname,
                keys: t
                    .keys
                    .into_iter()
                    .map(|k| crate::dto::TeamTextKeyDto {
                        family: k.family,
                        note: k.note,
                        key: k.key,
                        row_key: k.row_key,
                        lookup: k.lookup,
                        text: k.text,
                    })
                    .collect(),
                notes: t.notes,
            }))
        })
    }

    /// Phase 1.4: one league's logo binding and package paths (existence checked under
    /// `install_root` when given).
    pub fn league_logo(
        &self,
        id: u32,
        code: u64,
        install_root: Option<&str>,
    ) -> Result<Option<crate::dto::LeagueLogoDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .league_logo(code, install_root.map(Path::new))?
                .map(|l| crate::dto::LeagueLogoDto {
                    code: l.code,
                    art_index: l.art_index,
                    evidence: l.evidence,
                    witness: l.witness,
                    shared_with: l.shared_with,
                    variants: l
                        .variants
                        .into_iter()
                        .map(|v| crate::dto::LeagueLogoVariantDto {
                            family: v.family,
                            loose_rel_path: install_root.and_then(|r| studio_domain::league_logos::loose_logo_path(Path::new(r), &v.rel_path)),
                            rel_path: v.rel_path,
                            exists: v.exists,
                            archive: v.archive,
                        })
                        .collect(),
                    notes: l.notes,
                }))
        })
    }

    /// Phase 2.4 (C9): a team's logo families on the open title, keyed by its art id; loose-file
    /// existence under `install_root`, the archive the console reads named per variant.
    pub fn team_logo_set(&self, id: u32, team_id: u64, install_root: Option<&str>) -> Result<Option<crate::dto::TeamLogoSetDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws.display().team_logo_set(team_id, install_root.map(Path::new))?.map(|l| crate::dto::TeamLogoSetDto {
                artid: l.artid,
                evidence: l.evidence,
                variants: l
                    .variants
                    .into_iter()
                    .map(|v| crate::dto::LeagueLogoVariantDto {
                        loose_rel_path: install_root.and_then(|r| studio_domain::league_logos::loose_logo_path(Path::new(r), &v.rel_path)),
                        family: v.family, rel_path: v.rel_path, exists: v.exists, archive: v.archive
                    })
                    .collect(),
                notes: l.notes,
            }))
        })
    }

    /// Phase 2.1 (21 Sep 2026): a team's identity dependencies (read-only).
    pub fn team_identity_dependencies(&self, id: u32, team_id: u64) -> Result<crate::dto::TeamIdentityDependenciesDto, ApiError> {
        self.with_workspace(id, |ws| {
            let d = studio_domain::team_reid::team_identity_dependencies(ws.display(), team_id).map_err(ApiError::Domain)?;
            let conv = |v: Vec<studio_domain::team_reid::DependencyCount>| v.into_iter().map(|c| crate::dto::DependencyCountDto { table: c.table, field: c.field, rows: c.rows }).collect();
            Ok(crate::dto::TeamIdentityDependenciesDto {
                team_id: d.team_id,
                team_name: d.team_name,
                artid: d.artid,
                index_dependencies: conv(d.index_dependencies),
                artid_dependencies: conv(d.artid_dependencies),
                teams_sharing_artid: d.teams_sharing_artid,
                index_change: d.index_change,
                artid_change: d.artid_change,
            })
        })
    }

    pub fn team_artid_options(&self, id: u32) -> Result<Vec<(u64, u64, String)>, ApiError> {
        self.with_workspace(id, |ws| studio_domain::team_reid::artid_options(ws.display()).map_err(ApiError::Domain))
    }

    /// Phase 2.1 (21 Sep 2026): swap the art ids of two teams (one transaction, one undo step).
    pub fn team_artid_swap(&self, id: u32, team_a: u64, team_b: u64) -> Result<crate::dto::ArtIdSwapDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (plan, outcome) = ws.apply_artid_swap(team_a, team_b).map_err(ApiError::Domain)?;
            Ok(crate::dto::ArtIdSwapDto {
                team_a: plan.team_a,
                team_b: plan.team_b,
                team_a_name: plan.team_a_name,
                team_b_name: plan.team_b_name,
                artid_a: plan.artid_a,
                artid_b: plan.artid_b,
                tables: plan.tables,
                total_cells: plan.total_cells,
                summary: plan.summary,
                outcome: convert::outcome_dto(outcome),
            })
        })
    }

    /// Phase 2.7 (21 Sep 2026): the name-call listing of one team (read-only) — the team's
    /// `audioid` joined to the `team_ID` graffiti tables of the loose gen7 speech files.
    pub fn team_name_calls(&self, id: u32, team_id: u64, install_root: &str) -> Result<crate::dto::TeamNameCallsDto, ApiError> {
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().ok_or_else(|| ApiError::Workspace("the open project has no recorded title".into()))?.to_string();
            let platform = ws.display().project().platform;
            let (team_name, audio) = studio_domain::speech_catalogue::team_audio_id(ws.display(), team_id).map_err(ApiError::Domain)?;
            let Some(audio) = audio else {
                return Ok(crate::dto::TeamNameCallsDto {
                    team_id,
                    team_name,
                    team_audio_id: None,
                    team_tag: None,
                    tables: Vec::new(),
                    sample_ids_distinct: 0,
                    bank_resolved_distinct: 0,
                    keyword_mentions: None,
                    notes: vec!["this title's team table has no audio id column".into()],
                });
            };
            let c = studio_domain::speech_catalogue::team_name_calls(&game_id, platform, Path::new(install_root), audio).map_err(ApiError::Domain)?;
            Ok(crate::dto::TeamNameCallsDto {
                team_id,
                team_name,
                team_audio_id: Some(c.team_audio_id),
                team_tag: Some(c.team_tag),
                tables: c
                    .tables
                    .into_iter()
                    .map(|t| crate::dto::NameCallTableDto { num_tags: t.num_tags, sample_count: t.sample_ids.len(), bank_resolved: t.bank_resolved, sample_ids: t.sample_ids.into_iter().take(64).collect() })
                    .collect(),
                sample_ids_distinct: c.sample_ids_distinct,
                bank_resolved_distinct: c.bank_resolved_distinct,
                keyword_mentions: c.keyword_mentions,
                notes: c.notes,
            })
        })
    }

    /// Phase 1.5 / 2.7 (21 Sep 2026): the read-only context listing of the open title.
    fn ps4_speech_ctx(&self, id: u32) -> Result<(String, studio_domain::Platform), ApiError> {
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().ok_or_else(|| ApiError::Workspace("the open project has no recorded title".into()))?.to_string();
            Ok((game_id, ws.display().project().platform))
        })
    }

    /// 25 Sep 2026 — the gen7 `.abk` sound banks of the open title: loose `audio/sfx` (R82 / R89) or,
    /// on PS3 NHL 09 / 10 / 12 / 13, the title's own archive members (R98 / R100, 26 Sep). Listing only.
    pub fn gen7_sfx_banks(&self, id: u32, install_root: &str) -> Result<crate::dto::Gen7SfxBankListDto, ApiError> {
        let (game_id, platform) = self.ps4_speech_ctx(id)?;
        let l = studio_domain::gen7_sfx_banks::list_banks(&game_id, platform, Path::new(install_root)).map_err(ApiError::Domain)?;
        Ok(crate::dto::Gen7SfxBankListDto {
            game_id: l.game_id,
            platform: l.platform,
            population: l.population,
            dir: l.dir,
            banks: l
                .banks
                .into_iter()
                .map(|b| crate::dto::Gen7SfxBankSummaryDto {
                    name: b.name,
                    path: b.path,
                    len: b.len,
                    platform: b.platform,
                    records: b.records as u32,
                    stream_records: b.stream_records as u32,
                    ram_records: b.ram_records as u32,
                    lists: b.lists as u32,
                    names: b.names as u32,
                    ast: b.ast,
                    ast_len: b.ast_len,
                    ast_id_matches: b.ast_id_matches,
                    problem: b.problem,
                })
                .collect(),
            notes: l.notes,
        })
    }

    /// 25 Sep 2026 — one bank's named picker tree with every entry framed in its `.ast` (R89 / R91).
    pub fn gen7_sfx_bank_tree(&self, id: u32, install_root: &str, bank: &str) -> Result<crate::dto::Gen7SfxBankTreeDto, ApiError> {
        let (game_id, platform) = self.ps4_speech_ctx(id)?;
        let t = studio_domain::gen7_sfx_banks::bank_tree(&game_id, platform, Path::new(install_root), bank).map_err(ApiError::Domain)?;
        Ok(crate::dto::Gen7SfxBankTreeDto {
            game_id: t.game_id,
            platform: t.platform,
            bank: t.bank,
            bank_path: t.bank_path,
            bank_len: t.bank_len,
            bank_sha256: t.bank_sha256,
            bank_wrapper: t.bank_wrapper,
            bank_stored_sha256: t.bank_stored_sha256,
            bank_copies: t.bank_copies as u32,
            bank_platform: t.bank_platform,
            ast_path: t.ast_path,
            ast_len: t.ast_len,
            ast_id: t.ast_id,
            ast_id_matches: t.ast_id_matches,
            records: t.records as u32,
            stream_records: t.stream_records as u32,
            ram_records: t.ram_records as u32,
            lists: t.lists as u32,
            names: t.names as u32,
            handler_nodes: t.handler_nodes as u32,
            pickers: t
                .pickers
                .into_iter()
                .map(|p| crate::dto::Gen7SfxPickerDto {
                    owners: p.owners,
                    owner_class: p.owner_class,
                    list_offset: p.list_offset,
                    empty_slots: p.empty_slots as u32,
                    entries: p
                        .entries
                        .into_iter()
                        .map(|e| crate::dto::Gen7SfxEntryDto {
                            record: e.record as u32,
                            weight: e.weight as u32,
                            kind: e.kind,
                            codec: e.codec,
                            codec_code: e.codec_code as u32,
                            sample_rate: e.sample_rate,
                            channel_config: e.channel_config as u32,
                            sub_streams: e.sub_streams.map(|n| n as u32),
                            num_samples: e.num_samples,
                            stream_offset: e.stream_offset,
                            blocks: e.blocks.map(|n| n as u32),
                            stream_bytes: e.stream_bytes,
                            framing: e.framing,
                            x: e.x as u32,
                            s: e.s,
                            previewable: e.previewable,
                            preview_refusal: e.preview_refusal,
                        })
                        .collect(),
                })
                .collect(),
            notes: t.notes,
        })
    }

    /// 25 Sep 2026 — the **diagnostic** decode of one `.abk` entry (PS3 EALayer3 v1, one sub-stream;
    /// R92 construction). Never game-verified; no replacement route.
    pub fn gen7_sfx_preview(&self, id: u32, install_root: &str, bank: &str, record: u32, sub_stream: Option<u32>) -> Result<crate::dto::Gen7SfxPreviewDto, ApiError> {
        let (game_id, platform) = self.ps4_speech_ctx(id)?;
        let rec = u16::try_from(record).map_err(|_| ApiError::Workspace(format!("record {record} is outside the 16-bit record index")))?;
        let part = sub_stream.map(|k| u16::try_from(k).map_err(|_| ApiError::Workspace(format!("sub-stream {k} is out of range")))).transpose()?;
        let p = studio_domain::gen7_sfx_banks::preview_part(&game_id, platform, Path::new(install_root), bank, rec, part).map_err(ApiError::Domain)?;
        Ok(crate::dto::Gen7SfxPreviewDto {
            game_id: p.game_id,
            platform: p.platform,
            bank: p.bank,
            bank_sha256: p.bank_sha256,
            record: p.record as u32,
            kind: p.kind,
            sub_stream: p.sub_stream.map(u32::from),
            sub_streams: p.sub_streams as u32,
            loop_start: p.loop_start,
            owners: p.owners,
            source: p.source,
            source_sha256: p.source_sha256,
            codec: p.codec,
            sample_rate: p.sample_rate,
            channels: p.channels,
            num_samples: p.num_samples,
            blocks: p.blocks as u32,
            granules: p.granules as u32,
            rebuilt_frames: p.rebuilt_frames as u32,
            decoded_frames_per_channel: p.decoded_frames_per_channel as u32,
            label: p.label,
            wav_data_url: format!("data:audio/wav;base64,{}", BASE64.encode(&p.wav)),
            notes: p.notes,
        })
    }

    /// P4 (24 Sep 2026) — the PS4 speech banks of the open title (listing only).
    pub fn ps4_speech_banks(&self, id: u32, install_root: &str) -> Result<crate::dto::Ps4SpeechBanksDto, ApiError> {
        let (game_id, platform) = self.ps4_speech_ctx(id)?;
        let l = studio_domain::ps4_speech_bank::list_banks(&game_id, platform, Path::new(install_root)).map_err(ApiError::Domain)?;
        Ok(crate::dto::Ps4SpeechBanksDto { game_id: l.game_id, speech_dir: l.speech_dir, banks: l.banks.iter().map(convert::ps4_speech_bank_dto).collect(), notes: l.notes })
    }

    /// P4 — one bank's samples with their measured SNR headers.
    pub fn ps4_speech_samples(&self, id: u32, install_root: &str, bank_file: &str, query: &str, limit: usize) -> Result<crate::dto::Ps4SpeechSamplesDto, ApiError> {
        let (game_id, platform) = self.ps4_speech_ctx(id)?;
        let l = studio_domain::ps4_speech_bank::list_samples(&game_id, platform, Path::new(install_root), bank_file, query, limit.clamp(1, 5000)).map_err(ApiError::Domain)?;
        Ok(crate::dto::Ps4SpeechSamplesDto {
            bank: convert::ps4_speech_bank_dto(&l.bank),
            total: l.total as u32,
            matching: l.matching as u32,
            truncated: l.truncated,
            codec_census: l.codec_census.into_iter().map(|(codec, count)| crate::dto::Ps4SpeechCodecCountDto { codec, count: count as u32 }).collect(),
            rows: l
                .rows
                .into_iter()
                .map(|r| crate::dto::Ps4SpeechSampleDto {
                    sid: r.sid,
                    offset: r.offset,
                    duration_ms: r.duration_ms,
                    codec: r.codec,
                    codec_code: r.codec_code,
                    sample_rate: r.sample_rate,
                    channels: r.channels,
                    num_samples: r.num_samples,
                    problem: r.problem,
                    previewable: r.previewable,
                })
                .collect(),
            notes: l.notes,
        })
    }

    /// P4 — the diagnostic EA-MP3 preview of one sample. `rendering` is `"mpeg_only"` or
    /// `"candidate_a"`; anything else is refused.
    pub fn ps4_speech_preview(&self, id: u32, install_root: &str, bank_file: &str, sid: u64, rendering: &str) -> Result<crate::dto::Ps4SpeechPreviewDto, ApiError> {
        let (game_id, platform) = self.ps4_speech_ctx(id)?;
        let r = match rendering {
            "mpeg_only" => studio_domain::ps4_speech_bank::PreviewRendering::MpegOnly,
            "candidate_a" => studio_domain::ps4_speech_bank::PreviewRendering::CandidateA,
            other => return Err(ApiError::BadEnum { what: "rendering", value: other.to_string() }),
        };
        let p = studio_domain::ps4_speech_bank::preview_sample(&game_id, platform, Path::new(install_root), bank_file, sid, r).map_err(ApiError::Domain)?;
        Ok(crate::dto::Ps4SpeechPreviewDto {
            bank: convert::ps4_speech_bank_dto(&p.bank),
            sid: p.sid,
            offset: p.offset,
            sns_len: p.sns_len,
            sns_sha256: p.sns_sha256,
            codec: p.codec,
            sample_rate: p.sample_rate,
            channels: p.channels,
            num_samples: p.num_samples,
            blocks: p.blocks as u32,
            frames: p.frames as u32,
            frames_per_block: p.frames_per_block.iter().map(|f| *f as u32).collect(),
            trim: p.trim as u32,
            embedded_pcm_samples: p.embedded_pcm_samples as u32,
            first_sample_over_1_lsb: p.first_sample_over_1_lsb.map(|f| f as u32),
            rendering: rendering.to_string(),
            label: p.label,
            wav_data_url: format!("data:audio/wav;base64,{}", BASE64.encode(&p.wav)),
            notes: p.notes,
        })
    }

    /// P4.6 — list a measured gen7 bank's samples with their SNR headers (listing only).
    pub fn gen7_bank_samples(&self, id: u32, install_root: &str, bank_stem: &str, query: &str, limit: usize) -> Result<crate::dto::Gen7BankListingDto, ApiError> {
        let (game_id, platform) = self.ps4_speech_ctx(id)?;
        let l = studio_domain::gen7_speech_preview::list_bank_samples(&game_id, platform, Path::new(install_root), bank_stem, query, limit.clamp(1, 5000)).map_err(ApiError::Domain)?;
        Ok(crate::dto::Gen7BankListingDto {
            game_id: l.game_id,
            platform: l.platform,
            bank_stem: l.bank_stem,
            tuple_source: l.tuple_source,
            bank: l.bank,
            bank_len: l.bank_len,
            bank_sha256: l.bank_sha256,
            stream: l.stream,
            total: l.total as u32,
            matching: l.matching as u32,
            truncated: l.truncated,
            codec_census: l.codec_census.into_iter().map(|(codec, count)| crate::dto::Ps4SpeechCodecCountDto { codec, count: count as u32 }).collect(),
            rows: l
                .rows
                .into_iter()
                .map(|r| crate::dto::Gen7SampleRowDto { sample_id: r.sample_id, offset: r.offset, codec: r.codec, codec_code: r.codec_code, sample_rate: r.sample_rate, channels: r.channels, num_samples: r.num_samples, problem: r.problem, previewable: r.previewable })
                .collect(),
            preview_unavailable: l.preview_unavailable,
            notes: l.notes,
        })
    }

    /// P4.3 — the diagnostic gen7 EALayer3 v1 preview of one speech sample (measured tuples only).
    pub fn gen7_speech_preview(&self, id: u32, install_root: &str, bank_stem: &str, sample_id: u32) -> Result<crate::dto::Gen7SpeechPreviewDto, ApiError> {
        let (game_id, platform) = self.ps4_speech_ctx(id)?;
        use std::sync::atomic::{AtomicBool, Ordering};
        let flag = Arc::new(AtomicBool::new(false));
        if let Some(previous) = self.gen7_preview_cancel.lock().unwrap_or_else(|e| e.into_inner()).insert(id, flag.clone()) {
            previous.store(true, Ordering::SeqCst);
        }
        let result = studio_domain::gen7_speech_preview::preview_sample_cancellable(&game_id, platform, Path::new(install_root), bank_stem, sample_id, Some(&flag));
        {
            let mut m = self.gen7_preview_cancel.lock().unwrap_or_else(|e| e.into_inner());
            if m.get(&id).is_some_and(|f| Arc::ptr_eq(f, &flag)) {
                m.remove(&id);
            }
        }
        let p = result.map_err(ApiError::Domain)?;
        Ok(crate::dto::Gen7SpeechPreviewDto {
            game_id: p.game_id,
            platform: p.platform,
            bank_stem: p.bank_stem,
            bank_path: p.bank_path,
            bank_len: p.bank_len,
            bank_sha256: p.bank_sha256,
            stream_path: p.stream_path,
            stream_len: p.stream_len,
            sample_id: p.sample_id,
            offset: p.offset,
            sns_len: p.sns_len,
            sns_sha256: p.sns_sha256,
            codec: p.codec,
            sample_rate: p.sample_rate,
            channels: p.channels,
            num_samples: p.num_samples,
            blocks: p.blocks as u32,
            granules: p.granules as u32,
            granules_per_block: p.granules_per_block.iter().map(|g| *g as u32).collect(),
            rebuilt_frames: p.rebuilt_frames as u32,
            max_main_data_begin: p.max_main_data_begin,
            pcm_frame_samples: p.pcm_frame_samples as u32,
            surplus_samples: p.surplus_samples,
            decoded_samples_per_channel: p.decoded_samples_per_channel as u32,
            first_sample_over_1_lsb: p.first_sample_over_1_lsb.map(|f| f as u32),
            label: p.label,
            route_detail: p.route_detail,
            tuple_source: p.tuple_source,
            wav_data_url: format!("data:audio/wav;base64,{}", BASE64.encode(&p.wav)),
            notes: p.notes,
        })
    }

    /// Cancel the project's in-flight gen7 sample preview, if any; `true` when one was running.
    pub fn gen7_speech_preview_cancel(&self, id: u32) -> bool {
        match self.gen7_preview_cancel.lock().unwrap_or_else(|e| e.into_inner()).get(&id) {
            Some(f) => {
                f.store(true, std::sync::atomic::Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    pub fn context_catalogue(&self, id: u32, install_root: &str, query: &str, limit: usize) -> Result<crate::dto::ContextCatalogueDto, ApiError> {
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().ok_or_else(|| ApiError::Workspace("the open project has no recorded title".into()))?.to_string();
            let platform = ws.display().project().platform;
            let v = studio_domain::speech_catalogue::context_catalogue(&game_id, platform, Path::new(install_root), query, limit.clamp(1, 5000)).map_err(ApiError::Domain)?;
            Ok(crate::dto::ContextCatalogueDto {
                game_id: v.game_id,
                platform: v.platform,
                context_dir: v.context_dir,
                records: v.records,
                triggers: v.triggers,
                condition_names: v.condition_names,
                speech_items: v.speech_items,
                context_items: v.context_items,
                query: v.query,
                matching: v.matching,
                truncated: v.truncated,
                contexts: v
                    .contexts
                    .into_iter()
                    .map(|r| crate::dto::ContextRecordDto {
                        index: r.index,
                        name: r.name,
                        trigger: r.trigger,
                        conditions: r.conditions,
                        items: r.items.into_iter().map(|i| crate::dto::ContextItemDto { kind: i.kind, name: i.name, event_id: i.event_id, params: i.params }).collect(),
                    })
                    .collect(),
                notes: v.notes,
            })
        })
    }

    /// Phase 3.5 (21 Sep 2026): one player's recorded names (read-only) from the loose gen7
    /// speech files.
    pub fn player_name_calls(&self, id: u32, player_id: u64, install_root: &str) -> Result<crate::dto::PlayerNameCallsDto, ApiError> {
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().ok_or_else(|| ApiError::Workspace("the open project has no recorded title".into()))?.to_string();
            let platform = ws.display().project().platform;
            let c = studio_domain::speech_catalogue::player_name_calls(ws.display(), &game_id, platform, Path::new(install_root), player_id).map_err(ApiError::Domain)?;
            let conv = |v: Vec<studio_domain::speech_catalogue::NameCallTable>| -> Vec<crate::dto::NameCallTableDto> {
                v.into_iter().map(|t| crate::dto::NameCallTableDto { num_tags: t.num_tags, sample_count: t.sample_ids.len(), bank_resolved: t.bank_resolved, sample_ids: t.sample_ids.into_iter().take(64).collect() }).collect()
            };
            Ok(crate::dto::PlayerNameCallsDto {
                player_id: c.player_id,
                first_name: c.first_name,
                last_name: c.last_name,
                pbp_names: c
                    .pbp_names
                    .into_iter()
                    .map(|n| crate::dto::PbpNameEntryDto { name: n.name, is_first_name: n.is_first_name, bank: n.bank, audio_id: n.audio_id, tables: conv(n.tables), sample_ids_distinct: n.sample_ids_distinct, bank_resolved_distinct: n.bank_resolved_distinct })
                    .collect(),
                pbp_player_tag: c.pbp_player_tag,
                announcer_audio_id: c.announcer_audio_id,
                announcer_tag: c.announcer_tag,
                announcer_tables: conv(c.announcer_tables),
                announcer_sample_ids_distinct: c.announcer_sample_ids_distinct,
                announcer_bank_resolved_distinct: c.announcer_bank_resolved_distinct,
                notes: c.notes,
            })
        })
    }

    /// Phase 1.5 / 2.7 / 3.5 (21 Sep 2026): the read-only gen7 commentary catalogue of the open
    /// title from the loose `audio/speech` files under `install_root`, filtered by `query` /
    /// `league_only` and capped at `limit` events and sentences.
    pub fn speech_catalogue(&self, id: u32, install_root: &str, query: &str, league_only: bool, limit: usize) -> Result<crate::dto::SpeechCatalogueDto, ApiError> {
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().ok_or_else(|| ApiError::Workspace("the open project has no recorded title".into()))?.to_string();
            let platform = ws.display().project().platform;
            let cat = studio_domain::speech_catalogue::speech_catalogue(&game_id, platform, Path::new(install_root)).map_err(ApiError::Domain)?;
            let v = studio_domain::speech_catalogue::view(&cat, query, league_only, limit.clamp(1, 5000));
            Ok(crate::dto::SpeechCatalogueDto {
                game_id: v.game_id,
                platform: v.platform,
                speech_dir: v.speech_dir,
                sources: v.sources.into_iter().map(|s| crate::dto::SpeechSourceFileDto { role: s.role, path: s.path, len: s.len, sha256: s.sha256 }).collect(),
                stats: crate::dto::SpeechStatsDto {
                    events: v.stats.events,
                    events_with_a_sentence: v.stats.events_with_a_sentence,
                    events_without_a_sentence: v.stats.events_without_a_sentence,
                    parameters: v.stats.parameters,
                    sentences: v.stats.sentences,
                    sentences_slot_ok: v.stats.sentences_slot_ok,
                    bodies: v.stats.bodies,
                    phrases: v.stats.phrases,
                    phrases_with_table: v.stats.phrases_with_table,
                    graffiti_tables: v.stats.graffiti_tables,
                    bank_samples: v.stats.bank_samples,
                    keywords: v.stats.keywords,
                    ambiguous_event_ids: v.stats.ambiguous_event_ids,
                },
                notes: v.notes,
                query: v.query,
                league_only: v.league_only,
                events_matching: v.events_matching,
                sentences_matching: v.sentences_matching,
                events: v.events.into_iter().map(|e| crate::dto::SpeechEventDto { id: e.id, name: e.name, parameters: e.parameters, other_names_for_id: e.other_names_for_id, sentences: e.sentences }).collect(),
                sentences: v
                    .sentences
                    .into_iter()
                    .map(|s| crate::dto::SpeechSentenceDto {
                        name: s.name,
                        event_id: s.event_id,
                        slot: s.slot,
                        slot_ok: s.slot_ok,
                        bodies: s
                            .bodies
                            .into_iter()
                            .map(|b| crate::dto::SpeechBodyDto {
                                id: b.id,
                                phrases: b
                                    .phrases
                                    .into_iter()
                                    .map(|p| crate::dto::SpeechPhraseDto {
                                        pool_id: p.pool_id,
                                        group_tag: p.group_tag,
                                        group_value: p.group_value,
                                        group_name: p.group_name,
                                        parameters: p.parameters,
                                        table_found: p.table_found,
                                        candidate_count: p.candidate_sample_ids.len(),
                                        candidate_sample_ids: p.candidate_sample_ids.into_iter().take(64).collect(),
                                        bank_resolved: p.bank_resolved,
                                        unresolved_sample_ids: p.unresolved_sample_ids,
                                    })
                                    .collect(),
                            })
                            .collect(),
                    })
                    .collect(),
                truncated: v.truncated,
            })
        })
    }

    /// P1-G: the league-keyed presentation imagery of the open title (playoff-tree logos,
    /// trophies, PS4 registry types), loose-file existence checked under `install_root`.
    pub fn league_imagery(&self, id: u32, install_root: Option<&str>) -> Result<Option<crate::dto::LeagueImageryDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws.display().league_imagery(install_root.map(Path::new)).map(|li| crate::dto::LeagueImageryDto {
                game_id: li.game_id,
                platform: li.platform,
                registry: li.registry,
                families: li
                    .families
                    .into_iter()
                    .map(|f| crate::dto::LeagueImageryFamilyDto {
                        family: f.family,
                        label: f.label,
                        reading: f.reading,
                        consumers: f.consumers,
                        texture_facts: f.texture_facts,
                        members: f
                            .members
                            .into_iter()
                            .map(|m| crate::dto::LeagueImageryMemberDto {
                                id: m.id,
                                rel_path: m.rel_path,
                                exists: m.exists,
                                fr_rel_path: m.fr_rel_path,
                                fr_exists: m.fr_exists,
                                archive: m.archive,
                            })
                            .collect(),
                    })
                    .collect(),
                notes: li.notes,
            }))
        })
    }

    /// Phase 1.2 (D2): the read-only settings catalogue of the open target, or the blocker.
    /// `install_root` is the install folder (PS4 16–21 read `databigs/inxp_00.big` under it).
    pub fn settings_catalogue(&self, id: u32, install_root: Option<&str>) -> Result<crate::dto::SettingsCatalogueResultDto, ApiError> {
        self.with_workspace(id, |ws| {
            // The workspace overlays its pending NHL 16–21 catalogue edits (D4).
            let r = ws.settings_catalogue(install_root.map(Path::new))?;
            Ok(convert::settings_catalogue_result_dto(&r))
        })
    }

    /// D4: drop the most recent pending NHL 16–21 catalogue edit (the undo of its apply).
    pub fn settings_csv_undo(&self, id: u32) -> Result<bool, ApiError> {
        self.with_workspace(id, |ws| Ok(ws.undo_settings_csv_edit()))
    }

    /// D8: preview the replacement of a texture member inside a packed archive of the install
    /// (`archive_relative` under `install_root`, `member_path` inside it) with a source image
    /// file; the plan is held under a ticket. Writes nothing.
    pub fn asset_member_preview(&self, id: u32, install_root: &str, archive_relative: &str, member_path: &str, source_image_path: &str) -> Result<crate::dto::AssetMemberPreviewDto, ApiError> {
        self.asset_member_preview_scoped(id, install_root, archive_relative, member_path, source_image_path, false)
    }

    /// [`Registry::asset_member_preview`] with the explicit scope for a shared member: `whole_group`
    /// = every path of the deduplicated block follows the new image (C7); `false` = private copy.
    pub fn asset_member_preview_scoped(&self, id: u32, install_root: &str, archive_relative: &str, member_path: &str, source_image_path: &str, whole_group: bool) -> Result<crate::dto::AssetMemberPreviewDto, ApiError> {
        let img = studio_domain::decode_source_image(Path::new(source_image_path)).map_err(ApiError::Domain)?;
        let out = self.with_workspace(id, |ws| {
            Ok(match ws.asset_member_preview(Path::new(install_root), archive_relative, member_path, &img.rgba, img.width, img.height, whole_group) {
                Ok((ticket, plan)) => crate::dto::AssetMemberPreviewDto { plan: Some(convert::asset_member_plan_dto(&ticket, &plan)), refusal_kind: None, refusal: None },
                Err(r) => crate::dto::AssetMemberPreviewDto { plan: None, refusal_kind: Some(r.kind.to_string()), refusal: Some(r.reason) },
            })
        })?;
        if out.plan.is_some() {
            // ONB-003: inside a project the editable source is kept with the project so the member
            // edit the save commits carries its intent, not only the rewritten archive.
            self.capture_member_source(id, archive_relative, member_path, source_image_path)?;
        }
        Ok(out)
    }

    /// Receipt 67 (C37): preview the replacement of PS3 board raster `descriptor` of venue package `package` in the install at
    /// `install_root` (its `USRDIR`) with the image at `source_image_path` (the raster's own size; no resizing). The plan is held
    /// like an asset-member review: apply it with [`Self::asset_member_apply_review`], and the next copied-target save writes the
    /// archive member (and, on NHL 14 / 15 / Legacy, the loose copy) as one transaction. Writes nothing.
    pub fn ps3_board_replacement_preview(&self, id: u32, install_root: &str, package: &str, descriptor: u32, source_image_path: &str) -> Result<crate::dto::BoardReplacementPreviewDto, ApiError> {
        let img = studio_domain::decode_source_image(Path::new(source_image_path)).map_err(ApiError::Domain)?;
        self.with_workspace(id, |ws| {
            Ok(match ws.board_replacement_preview(Path::new(install_root), package, descriptor as usize, &img.rgba, img.width, img.height) {
                Ok((ticket, b)) => crate::dto::BoardReplacementPreviewDto {
                    plan: Some(convert::asset_member_plan_dto(&ticket, &b.plan)),
                    refusal_kind: None,
                    refusal: None,
                    topology: Some(b.topology.to_string()),
                    format: Some(format!("{:?}", b.edge.format).to_uppercase()),
                    levels: Some(b.edge.levels),
                    materials: b.materials,
                },
                Err(r) => crate::dto::BoardReplacementPreviewDto { plan: None, refusal_kind: Some(r.kind.to_string()), refusal: Some(r.reason), topology: None, format: None, levels: None, materials: Vec::new() },
            })
        })
    }

    /// D8: apply a held texture-member review — it becomes a pending replacement written by the
    /// next copied-target save (private copy for shared members).
    pub fn asset_member_apply_review(&self, id: u32, ticket: &str) -> Result<crate::dto::AssetMemberApplyDto, ApiError> {
        self.refuse_in_save_editor(id, "Replacing archive assets")?;
        self.with_workspace(id, |ws| {
            let review = ws.asset_member_apply_review(ticket).map_err(ApiError::Domain)?;
            Ok(crate::dto::AssetMemberApplyDto {
                plan: convert::asset_member_plan_dto(&review.ticket, &review.plan),
                pending: ws.asset_member_edits().len() as u32,
                summary: convert::project_summary_dto(id, ws).map_err(ApiError::Domain)?,
            })
        })
    }

    /// D8: drop the most recent pending texture-member replacement.
    pub fn asset_member_undo(&self, id: u32) -> Result<bool, ApiError> {
        self.with_workspace(id, |ws| Ok(ws.undo_asset_member_edit()))
    }

    /// D3: preview the edit of one stored catalogue default; the plan is held under a ticket
    /// bound to this project instance and its revision. Writes nothing.
    pub fn settings_default_preview(&self, id: u32, legacy_id: i64, context: Option<i64>, code: i64, install_root: Option<&str>) -> Result<crate::dto::SettingsEditPreviewDto, ApiError> {
        self.with_workspace(id, |ws| {
            let target = studio_domain::settings_edit::DefaultTarget { legacy_id, context };
            let csv_title = matches!(ws.game_id(), Some("nhl16" | "nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21"));
            if csv_title {
                let Some(root) = install_root else {
                    return Ok(crate::dto::SettingsEditPreviewDto { plan: None, refusal_kind: Some("no-install-root".into()), refusal: Some("the catalogue is read from the install's databigs/inxp_00.big; open the project from its install folder".into()) });
                };
                return Ok(match ws.settings_csv_default_preview(Path::new(root), target, code).map_err(ApiError::Domain)? {
                    Ok((ticket, plan)) => crate::dto::SettingsEditPreviewDto { plan: Some(convert::settings_csv_plan_dto(&ticket, &plan)), refusal_kind: None, refusal: None },
                    Err(r) => crate::dto::SettingsEditPreviewDto { plan: None, refusal_kind: Some(r.kind.to_string()), refusal: Some(r.reason) },
                });
            }
            Ok(match ws.settings_default_preview(target, code).map_err(ApiError::Domain)? {
                Ok((ticket, plan)) => crate::dto::SettingsEditPreviewDto { plan: Some(convert::settings_edit_plan_dto(&ticket, &plan)), refusal_kind: None, refusal: None },
                Err(r) => crate::dto::SettingsEditPreviewDto { plan: None, refusal_kind: Some(r.kind.to_string()), refusal: Some(r.reason) },
            })
        })
    }

    /// D3: apply a held stored-default review (one cell, one undo entry). Refuses a foreign,
    /// consumed or stale ticket without writing; a write that would change anything beyond the
    /// cell and its covering checksum is rolled back and reported.
    pub fn settings_apply_review(&self, id: u32, ticket: &str) -> Result<crate::dto::SettingsEditApplyDto, ApiError> {
        self.refuse_in_save_editor(id, "Editing game settings")?;
        self.with_workspace(id, |ws| {
            // A CSV review ticket (D4) applies as a pending member edit; a database ticket (D3)
            // writes the cell. Each ticket is held by exactly one of the two lists.
            if ws.settings_csv_review(ticket).is_some() {
                let review = ws.settings_csv_apply_review(ticket).map_err(ApiError::Domain)?;
                return Ok(crate::dto::SettingsEditApplyDto {
                    plan: convert::settings_csv_plan_dto(&review.ticket, &review.plan),
                    outcome: convert::outcome_dto(studio_domain::OpOutcome { edits: if review.plan.no_op { 0 } else { review.plan.members.len() as u32 }, ..Default::default() }),
                    summary: convert::project_summary_dto(id, ws).map_err(ApiError::Domain)?,
                });
            }
            let (review, outcome) = ws.settings_apply_review(ticket).map_err(ApiError::Domain)?;
            Ok(crate::dto::SettingsEditApplyDto {
                plan: convert::settings_edit_plan_dto(&review.ticket, &review.plan),
                outcome: convert::outcome_dto(outcome),
                summary: convert::project_summary_dto(id, ws).map_err(ApiError::Domain)?,
            })
        })
    }

    /// Phase 1.2: every schedule table with its league match.
    pub fn schedule_tables(&self, id: u32) -> Result<Vec<crate::dto::ScheduleTableDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .schedule_tables()?
                .iter()
                .map(convert::schedule_table_dto)
                .collect())
        })
    }

    /// Phase 1.2: one schedule table's games, slot resolution and findings.
    pub fn schedule_view(
        &self,
        id: u32,
        studio_table: &str,
    ) -> Result<Option<crate::dto::ScheduleViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .schedule_view(studio_table)?
                .as_ref()
                .map(convert::schedule_view_dto))
        })
    }

    // --- P1-A: schedule editing / import / export / generation ---

    fn schedule_game_edit(e: &crate::dto::ScheduleGameEditDto) -> studio_domain::schedule_edit::GameEdit {
        studio_domain::schedule_edit::GameEdit {
            record: e.record,
            home_team_id: e.home_team_id,
            away_team_id: e.away_team_id,
            month: e.month,
            day: e.day,
        }
    }

    fn schedule_mapping(m: &[crate::dto::ScheduleTeamMappingDto]) -> Vec<studio_domain::schedule_edit::TeamMapping> {
        m.iter()
            .map(|m| studio_domain::schedule_edit::TeamMapping {
                key: m.key.clone(),
                team_id: m.team_id,
            })
            .collect()
    }

    fn schedule_generation_config(c: &crate::dto::ScheduleGenerationConfigDto) -> studio_domain::schedule_edit::GenerationConfig {
        studio_domain::schedule_edit::GenerationConfig {
            participants: c.participants.clone(),
            rounds: c.rounds,
            start_month: c.start_month,
            start_day: c.start_day,
            days_between_matchdays: c.days_between_matchdays,
            pair_quotas: c.pair_quotas.clone(),
            odd_home: c.odd_home.as_deref().and_then(|h| match h {
                "lower_slot" => Some(studio_domain::schedule_edit::OddHome::LowerSlot),
                "higher_slot" => Some(studio_domain::schedule_edit::OddHome::HigherSlot),
                "balance" => Some(studio_domain::schedule_edit::OddHome::Balance),
                "per_pair" => Some(studio_domain::schedule_edit::OddHome::Explicit),
                _ => None,
            }),
            groups: c.groups.as_ref().map(|g| studio_domain::schedule_edit::GroupQuotas {
                groups: g.groups.clone(),
                within: g.within,
                across: g.across,
                origin: g.origin.clone(),
            }),
            odd_hosts: c.odd_hosts.as_ref().map(|h| h.iter().map(|p| (p.home, p.away)).collect()),
        }
    }

    /// Receipt 48: an odd-host rule Studio does not implement is refused, never read as "no rule".
    fn check_odd_home(c: &crate::dto::ScheduleGenerationConfigDto) -> Result<(), ApiError> {
        match c.odd_home.as_deref() {
            None | Some("lower_slot" | "higher_slot" | "balance" | "per_pair") => Ok(()),
            Some(other) => Err(ApiError::Workspace(format!(
                "`{other}` is not a host rule Studio supports (lower_slot, higher_slot, balance, per_pair); nothing was generated"
            ))),
        }
    }

    /// Preview one game edit (home/away/date) with whole-table validation; writes nothing. The
    /// plan is held under its `ticket` for [`Self::schedule_apply_review`].
    pub fn schedule_edit_preview(
        &self,
        id: u32,
        studio_table: &str,
        edit: &crate::dto::ScheduleGameEditDto,
    ) -> Result<crate::dto::ScheduleChangePlanDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (ticket, plan) = ws
                .schedule_edit_preview(studio_table, &Self::schedule_game_edit(edit))
                .map_err(ApiError::Domain)?;
            Ok(convert::schedule_change_plan_dto(&ticket, &plan))
        })
    }

    /// Apply a held review as one transaction (one undo entry). Refuses — writing nothing — a
    /// ticket this project instance did not issue or no longer holds, a plan with errors, a project whose revision moved since the
    /// preview, and an import whose source file no longer hashes to the previewed bytes (the file
    /// is re-read here for that check; the bytes written are the previewed snapshot).
    pub fn schedule_apply_review(&self, id: u32, ticket: &str) -> Result<crate::dto::ScheduleApplyResultDto, ApiError> {
        self.with_workspace(id, |ws| {
            let source_now = match ws.schedule_review(ticket).map(|r| &r.operation) {
                Some(studio_domain::ScheduleOperation::Import { source_path: Some(p), .. }) => {
                    Some(std::fs::read_to_string(p).map_err(|source| {
                        ApiError::Domain(studio_domain::DomainError::Io {
                            path: p.clone(),
                            source,
                        })
                    })?)
                }
                _ => None,
            };
            let (review, outcome, view) = ws
                .schedule_apply_review(ticket, source_now.as_deref())
                .map_err(ApiError::Domain)?;
            let kind = match review.operation {
                studio_domain::ScheduleOperation::Edit(_) => "edit",
                studio_domain::ScheduleOperation::Import { .. } => "import",
                studio_domain::ScheduleOperation::Generate(_) => "generate",
            };
            Ok(crate::dto::ScheduleApplyResultDto {
                ticket: review.ticket.clone(),
                kind: kind.to_string(),
                plan: convert::schedule_change_plan_dto(&review.ticket, &review.plan),
                import: review.import.as_ref().map(|p| convert::schedule_import_plan_dto(&review.ticket, p)),
                generation: review.generation.as_ref().map(|p| convert::schedule_generation_plan_dto(&review.ticket, p)),
                outcome: convert::outcome_dto(outcome),
                view: convert::schedule_view_dto(&view),
            })
        })
    }

    /// Export a schedule table as `csv` or `json` text; with `out_path` the text is also written
    /// to that file (the desktop's save dialog picks it).
    pub fn schedule_export(
        &self,
        id: u32,
        studio_table: &str,
        format: &str,
        out_path: Option<&str>,
    ) -> Result<crate::dto::ScheduleExportDto, ApiError> {
        let fmt = studio_domain::schedule_edit::ImportFormat::parse(format).map_err(ApiError::Domain)?;
        if let Some(p) = out_path {
            self.refuse_guarded_write(None, "the schedule export", Path::new(p))?;
        }
        self.with_workspace(id, |ws| {
            let text = ws.schedule_export(studio_table, fmt).map_err(ApiError::Domain)?;
            if let Some(p) = out_path {
                std::fs::write(p, &text).map_err(|source| {
                    ApiError::Domain(studio_domain::DomainError::Io {
                        path: p.to_string(),
                        source,
                    })
                })?;
            }
            let name = ws
                .display()
                .schedule_tables()
                .map_err(ApiError::Domain)?
                .into_iter()
                .find(|t| t.studio_table == studio_table)
                .map(|t| t.name)
                .unwrap_or_else(|| "schedule".into());
            let ext = match fmt {
                studio_domain::schedule_edit::ImportFormat::Csv => "csv",
                studio_domain::schedule_edit::ImportFormat::Json => "json",
            };
            Ok(crate::dto::ScheduleExportDto {
                format: ext.to_string(),
                text,
                suggested_filename: format!("{name}.{ext}"),
            })
        })
    }

    /// The import text: given inline, or read from `path` (the desktop's open dialog picks it).
    fn schedule_import_text(text: Option<&str>, path: Option<&str>) -> Result<String, ApiError> {
        match (text, path) {
            (Some(t), _) if !t.trim().is_empty() => Ok(t.to_string()),
            (_, Some(p)) => std::fs::read_to_string(p).map_err(|source| {
                ApiError::Domain(studio_domain::DomainError::Io {
                    path: p.to_string(),
                    source,
                })
            }),
            _ => Err(ApiError::Domain(studio_domain::DomainError::Validation(
                "no schedule file given: paste its text or choose a file".into(),
            ))),
        }
    }

    /// Preview an import (inline text or a file path): parse, resolve keys through the explicit
    /// mapping, validate the whole proposal; the text is snapshotted into the held review.
    /// Malformed input is an error; unresolved keys are refusals inside the plan.
    pub fn schedule_import_preview(
        &self,
        id: u32,
        studio_table: &str,
        format: &str,
        text: Option<&str>,
        path: Option<&str>,
        mapping: &[crate::dto::ScheduleTeamMappingDto],
    ) -> Result<crate::dto::ScheduleImportPlanDto, ApiError> {
        let fmt = studio_domain::schedule_edit::ImportFormat::parse(format).map_err(ApiError::Domain)?;
        let inline = text.filter(|t| !t.trim().is_empty());
        let text = Self::schedule_import_text(text, path)?;
        let source_path = if inline.is_some() { None } else { path };
        self.with_workspace(id, |ws| {
            let (ticket, plan) = ws
                .schedule_import_preview(studio_table, fmt, &text, source_path, &Self::schedule_mapping(mapping))
                .map_err(ApiError::Domain)?;
            Ok(convert::schedule_import_plan_dto(&ticket, &plan))
        })
    }

    /// The topology the table supports for generation, or the precise blocker.
    pub fn schedule_generation_options(&self, id: u32, studio_table: &str) -> Result<crate::dto::ScheduleGenerationOptionsDto, ApiError> {
        self.with_workspace(id, |ws| {
            let o = ws.schedule_generation_options(studio_table).map_err(ApiError::Domain)?;
            Ok(convert::schedule_generation_options_dto(&o))
        })
    }

    /// Receipt 48: the shipped groups of a table, only by exact source identity (else the reason).
    pub fn schedule_stock_groups(&self, id: u32, studio_table: &str) -> Result<crate::dto::ScheduleStockGroupsDto, ApiError> {
        self.with_workspace(id, |ws| {
            let r = ws.schedule_stock_groups(studio_table).map_err(ApiError::Domain)?;
            let model = ws.schedule_session().schedule_model(studio_table).map_err(ApiError::Domain)?;
            let name = |t: &u64| model.slots.by_team(*t).map(|e| e.name.clone()).unwrap_or_else(|| format!("team {t}"));
            Ok(match r {
                Ok(g) => crate::dto::ScheduleStockGroupsDto {
                    available: true,
                    group_names: g.groups.iter().map(|x| x.iter().map(name).collect()).collect(),
                    groups: Some(crate::dto::ScheduleGroupsDto {
                        groups: g.groups.clone(),
                        within: g.within,
                        across: g.across,
                        origin: Some(format!("the shipped {} groups of {}", model.summary.name, g.source)),
                    }),
                    source: Some(g.source),
                    games: Some(g.games),
                    reason: None,
                },
                Err(reason) => crate::dto::ScheduleStockGroupsDto { available: false, groups: None, group_names: Vec::new(), source: None, games: None, reason: Some(reason) },
            })
        })
    }

    /// Preview a deterministic generation for an explicit configuration; writes nothing; held
    /// under its `ticket`.
    pub fn schedule_generate_preview(
        &self,
        id: u32,
        studio_table: &str,
        config: &crate::dto::ScheduleGenerationConfigDto,
    ) -> Result<crate::dto::ScheduleGenerationPlanDto, ApiError> {
        Self::check_odd_home(config)?;
        self.with_workspace(id, |ws| {
            let (ticket, plan) = ws
                .schedule_generate_preview(studio_table, &Self::schedule_generation_config(config))
                .map_err(ApiError::Domain)?;
            Ok(convert::schedule_generation_plan_dto(&ticket, &plan))
        })
    }

    /// The full Team detail for one `kOtt.vfEq`, or `None` if absent.
    pub fn team_view(&self, id: u32, team_id: u64) -> Result<Option<TeamViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .team_view(team_id)?
                .as_ref()
                .map(convert::team_view_dto))
        })
    }

    /// Player ids occupying a team's `eGlu` roster block — the roster source for membership-only
    /// teams (Winter Classic / all-star / national), whose players keep another main team.
    pub fn team_roster_ids(&self, id: u32, team_id: u64) -> Result<Vec<u64>, ApiError> {
        self.with_workspace(id, |ws| Ok(ws.display().team_roster_ids(team_id)?))
    }

    /// A player portrait framed for the **bottom-anchored header tile**: decoded,
    /// then cropped to the content horizontally and down to its lowest *solid*
    /// row (dropping any faded bottom edge — see
    /// [`studio_domain::DecodedImage::cropped_to_solid_bottom`]). Returned as a
    /// PNG data URL; the header pins its bottom to the tile and scales to 95% tall.
    pub fn portrait_header_preview(&self, path: &str) -> Result<String, ApiError> {
        let img = studio_domain::decode_portrait(&self.preview_source_path(Path::new(path))).map_err(ApiError::Domain)?;
        let png = img
            .cropped_to_solid_bottom(8)
            .to_png()
            .map_err(ApiError::Domain)?;
        Ok(format!("data:image/png;base64,{}", BASE64.encode(&png)))
    }

    /// The team's logo as a cropped PNG data URL for the header window, or `None`
    /// when this title ships no logo file / the team has no art id. Resolves the
    /// team's `kOtt.lOnr` art id server-side, decodes the logo texlib, and crops
    /// to the opaque content so the header fills the tile with the mark itself.
    pub fn team_logo(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        team_id: u64,
    ) -> Result<Option<String>, ApiError> {
        let roots = resolve_roots(args);
        let overlay = self.preview_overlay_root(&args.install_dir);
        self.with_workspace(id, |ws| {
            let Some(view) = ws.display().team_view(team_id)? else {
                return Ok(None);
            };
            let Some(artid) = view.logo_artid else {
                return Ok(None);
            };
            let crest = Path::new(&roots.fe).join("ion/artassets/teamlogos").join(format!("t{artid}.big"));
            let preview_path = preview_asset_path(&crest, &args.install_dir, overlay.as_deref());
            let staged = (preview_path != crest).then(|| studio_domain::decode_portrait(&preview_path).ok()).flatten();
            let Some(img) = staged.or_else(|| studio_domain::decode_team_logo(&roots, artid)) else {
                return Ok(None);
            };
            let png = img
                .cropped_to_opaque(0)
                .to_png()
                .map_err(ApiError::Domain)?;
            Ok(Some(format!(
                "data:image/png;base64,{}",
                BASE64.encode(&png)
            )))
        })
    }

    /// One team's outfit textures (jersey / pants / socks) for the Uniforms surface. Resolves the
    /// full roster to derive the title's team-id token position, lists this team's texture files
    /// server-side, and maps each to an install-relative path the preview/replace commands take
    /// (entries outside the install root are dropped — they can't be safely written). `Vec::new()`
    /// when this title ships no team-keyed outfit textures (or the project has no install tree).
    pub fn team_outfits(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        team_id: u64,
    ) -> Result<Vec<crate::dto::OutfitTextureDto>, ApiError> {
        self.team_outfits_with_candidates(id, args, team_id, false)
    }

    pub fn team_outfits_with_candidates(
        &self, id: u32, args: &AssetRootsArgsDto, team_id: u64, include_ambiguous: bool,
    ) -> Result<Vec<crate::dto::OutfitTextureDto>, ApiError> {
        let roots = resolve_roots(args);
        let install_dir = PathBuf::from(&args.install_dir);
        self.with_workspace(id, |ws| {
            // Full resolved roster — the team-id token position is derived across all teams.
            let art_ids_map = ws.display().jersey_art_ids()?;
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
                        // Jersey art ids (`xjgw.GDQL`) — the outfit-filename token space; raw
                        // team-id matching mis-assigned e.g. Team Poland's files to the
                        // Portland Pirates (colliding id spaces).
                        art_ids: art_ids_map.get(&t.team_id).cloned().unwrap_or_default(),
                    })
                })
                .collect();
            let list = if include_ambiguous { studio_domain::list_team_outfit_candidates } else { studio_domain::list_team_outfits };
            let dtos =
                list(Path::new(&roots.rendering), &teams, team_id)
                    .into_iter()
                    .filter_map(|t| {
                        let rel = t.abs_path.strip_prefix(&install_dir).ok()?;
                        Some(crate::dto::OutfitTextureDto {
                            rel_path: rel.to_string_lossy().replace('\\', "/"),
                            texture_index: t.texture_index,
                            piece: t.piece,
                            label: t.label,
                            file_name: t.file_name,
                            is_nameplate: t.is_nameplate,
                        })
                    })
                    .collect();
            Ok(dtos)
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
            Ok(ws
                .display()
                .arena_view(arena_id)?
                .as_ref()
                .map(convert::arena_view_dto))
        })
    }

    /// Teams 2.2 — the disclosure shown before a team's home arena is reassigned: the exact cell,
    /// who stays at the arena the team leaves (or that it becomes vacant) and who already plays at
    /// the arena it joins. Read-only; refusals carry their reason. The write is the ordinary
    /// validated `set_field` on the returned cell with `to_raw`.
    pub fn arena_assignment_preview(
        &self,
        id: u32,
        team_id: u64,
        to_arena: u64,
    ) -> Result<crate::dto::ArenaAssignmentPreviewDto, ApiError> {
        use crate::dto::ArenaAssignmentPreviewDto;
        let view = self
            .team_view(id, team_id)?
            .ok_or_else(|| ApiError::Workspace(format!("team {team_id} is not in this database")))?;
        let team_name = self
            .list_teams(id)?
            .into_iter()
            .find(|t| t.team_id == team_id)
            .map(|t| t.name)
            .unwrap_or_else(|| format!("team {team_id}"));
        let cell = view
            .profile
            .sections
            .iter()
            .flat_map(|s| s.cells.iter())
            .find(|c| c.control == "arena" && c.field.is_some());
        let refused = |mut p: ArenaAssignmentPreviewDto, why: String| {
            p.allowed = false;
            p.refusal = Some(why);
            p
        };
        let mut preview = ArenaAssignmentPreviewDto {
            team_id,
            team_name: team_name.clone(),
            table: cell.and_then(|c| c.field.as_ref()).map(|f| f.studio_table.clone()),
            record: cell.map(|c| c.record),
            field: cell.and_then(|c| c.field.as_ref()).map(|f| f.studio_field.clone()),
            from: view.arena.clone(),
            from_raw: cell.and_then(|c| c.field.as_ref()).and_then(|f| f.decoded).map(|v| v as u64),
            from_other_tenants: Vec::new(),
            from_becomes_vacant: false,
            to: None,
            to_raw: to_arena,
            to_tenants: Vec::new(),
            allowed: true,
            refusal: None,
        };
        let Some(_) = cell else {
            if view.arena.is_some() {
                return Ok(refused(preview, format!("Studio can resolve {}'s home arena, but does not yet expose reassignment for this title's separate arena selector", team_name)));
            }
            return Ok(refused(preview, format!("{}'s team table carries no home-arena link on this title; there is nothing to assign", team_name)));
        };
        // The arena the team leaves: everyone else who stays there.
        if let Some(from) = &view.arena {
            if let Some(av) = self.arena_view(id, from.arena_id)? {
                preview.from_other_tenants = av.teams.iter().filter(|t| t.team_id != team_id).map(|t| t.name.clone()).collect();
                preview.from_becomes_vacant = preview.from_other_tenants.is_empty();
            }
        }
        // The arena it joins: catalogued, and not the one it already has.
        let Some(to) = self.arena_view(id, to_arena)? else {
            return Ok(refused(preview, format!("arena {to_arena} is not in this database's arena table; only a catalogued arena can be assigned")));
        };
        preview.to_tenants = to.teams.iter().filter(|t| t.team_id != team_id).map(|t| t.name.clone()).collect();
        let to_name = to
            .identity
            .iter()
            .find(|f| f.name == "arenaname")
            .map(|f| f.display.clone())
            .unwrap_or_default();
        let to_city = to
            .identity
            .iter()
            .find(|f| f.name == "cityname")
            .map(|f| f.display.clone())
            .unwrap_or_default();
        preview.to = Some(crate::dto::ArenaRefDto { arena_id: to_arena, name: to_name.clone(), city: to_city });
        if preview.from_raw == Some(to_arena) {
            return Ok(refused(preview, format!("{} already plays at {}; nothing would change", team_name, if to_name.is_empty() { format!("arena {to_arena}") } else { to_name })));
        }
        Ok(preview)
    }

    /// Phase 2.5 — the arena's venue-parameter inventory (classes, constants, colours, edit
    /// dispositions). `defs_root` supplies the developer-owned class map.
    pub fn arena_venue_view(&self, id: u32, arena_id: u64, defs_root: &Path, install_root: Option<&Path>) -> Result<Option<crate::dto::ArenaVenueDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .arena_venue_view(arena_id, defs_root, install_root)?
                .as_ref()
                .map(convert::arena_venue_dto))
        })
    }

    /// A player's read-only stock career stats.
    pub fn career_stats(&self, id: u32, player_id: u64) -> Result<CareerStatsDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::career_stats_dto(
                &ws.display().career_stats(player_id)?,
            ))
        })
    }

    /// The depth chart (lines) for one team, or `None` if the team is absent.
    pub fn lines_view(&self, id: u32, team_id: u64) -> Result<Option<LinesViewDto>, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(ws
                .display()
                .lines_view(team_id)?
                .as_ref()
                .map(convert::lines_view_dto))
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

    /// Phase 2.6 (22 Sep 2026): a team's kit rows joined to their jersey / pant / sock textures
    /// under the open title's **measured contract**, with per-component evidence and the shipped
    /// spare candidates. Read-only. The contract is chosen from the project's title and platform
    /// (never from the tree); the tree supplies evidence only. An empty install dir means no tree
    /// is attached and nothing is called complete.
    pub fn team_kits(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        team_id: u64,
    ) -> Result<crate::dto::TeamKitsDto, ApiError> {
        let attached = !args.install_dir.trim().is_empty();
        let roots = resolve_roots(args);
        let install_dir = attached.then(|| PathBuf::from(&args.install_dir));
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().map(str::to_string);
            let platform = ws.display().project().platform;
            let contract = studio_domain::kit_files::kit_contract(game_id.as_deref(), platform);
            let (team_art_id, rows) = ws.display().team_kit_rows(team_id)?;
            let rendering = attached.then(|| PathBuf::from(&roots.rendering));
            let nameplates = studio_domain::kit_files::nameplate_measured(game_id.as_deref(), platform);
            let mut view = studio_domain::kit_files::team_kits(team_id, team_art_id, &rows, contract, rendering.as_deref(), nameplates);
            if attached && roots.platform() != platform {
                view.notes.push(format!(
                    "the attached install is read as `{}` while the open database is `{}`; the kit contract follows the database's title and platform",
                    roots.platform().as_str(),
                    platform.as_str()
                ));
            }
            Ok(convert::team_kits_dto(&view, install_dir.as_deref()))
        })
    }

    /// Phase 2.5 (22 Sep 2026): an arena's shipped assets under the open title's measured
    /// contract — venue model / jumbotron by `arenaartid` (gen7), the team-keyed families through
    /// the home teams, the candidate and unbound ice layers, and on PS4 the venue directories
    /// with no binding claimed. Read-only. `None` when the arena id is not in the table.
    pub fn arena_assets(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        arena_id: u64,
    ) -> Result<Option<crate::dto::ArenaAssetsDto>, ApiError> {
        self.arena_assets_with_defs(id, args, arena_id, None)
    }

    /// Presentation 5.2 (receipt 49): the arena's venue art, read-only (measured gen7 families only).
    pub fn arena_venue_art(&self, id: u32, args: &AssetRootsArgsDto, arena_id: u64) -> Result<Option<crate::dto::VenueArtDto>, ApiError> {
        let attached = !args.install_dir.trim().is_empty();
        let roots = resolve_roots(args);
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().map(str::to_string);
            let platform = ws.display().project().platform;
            let Some(inputs) = ws.display().arena_asset_inputs(arena_id)? else { return Ok(None) };
            let rendering = attached.then(|| PathBuf::from(&roots.rendering));
            let v = studio_domain::venue_art::venue_art(game_id.as_deref(), platform, inputs.arena_art_id, rendering.as_deref());
            Ok(Some(convert::venue_art_dto(&v)))
        })
    }

    /// One raster of this arena's venue package as a PNG — only a `stadium/stadium_<its arenaartid>_container_<n>`
    /// file of the attached tree, and only while it has the listed SHA-256 (receipt 49).
    pub fn arena_venue_raster(&self, id: u32, args: &AssetRootsArgsDto, arena_id: u64, file: &str, sha256: &str, ordinal: u32, max_edge: u32) -> Result<crate::dto::VenueRasterPreviewDto, ApiError> {
        if args.install_dir.trim().is_empty() {
            return Err(ApiError::Workspace("attach the title's install to preview venue art".into()));
        }
        let roots = resolve_roots(args);
        let art = self.with_workspace(id, |ws| {
            let game_id = ws.game_id().map(str::to_string);
            let platform = ws.display().project().platform;
            if !game_id.as_deref().is_some_and(|g| studio_domain::venue_art::MEASURED.contains(&(g, platform))) {
                return Err(ApiError::Workspace("venue art of this title is not measured; nothing is previewed".into()));
            }
            let inputs = ws.display().arena_asset_inputs(arena_id)?.ok_or_else(|| ApiError::Workspace(format!("no arena {arena_id}")))?;
            inputs.arena_art_id.ok_or_else(|| ApiError::Workspace("this arena carries no arenaartid".into()))
        })?;
        let name = file.strip_prefix("stadium/").unwrap_or("");
        let ok = name
            .strip_prefix(&format!("stadium_{art}_container_"))
            .and_then(|r| r.strip_suffix(".rx2").or_else(|| r.strip_suffix(".rpsgl")))
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()));
        if !ok {
            return Err(ApiError::Workspace(format!("{file} is not one of this arena's venue packages")));
        }
        let path = PathBuf::from(&roots.rendering).join("stadium").join(name);
        let (width, height, png) = studio_domain::venue_art::venue_raster_png(&path, sha256, ordinal as usize, max_edge.clamp(64, 2048)).map_err(ApiError::Domain)?;
        use base64::Engine as _;
        Ok(crate::dto::VenueRasterPreviewDto { width, height, png_data_url: format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)) })
    }

    /// [`Registry::arena_assets`] with `defs_root`, so a PS4 arena lists its vault-level candidates
    /// from `defs/arena/ps4-venue-levels.json` (research R73; read-only).
    pub fn arena_assets_with_defs(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        arena_id: u64,
        defs_root: Option<&Path>,
    ) -> Result<Option<crate::dto::ArenaAssetsDto>, ApiError> {
        let attached = !args.install_dir.trim().is_empty();
        let roots = resolve_roots(args);
        let install_dir = attached.then(|| PathBuf::from(&args.install_dir));
        let levels = match defs_root {
            Some(d) => Some(studio_domain::arena_assets::Ps4VenueLevels::load(d).map_err(ApiError::Domain)?),
            None => None,
        };
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().map(str::to_string);
            let platform = ws.display().project().platform;
            let contract = studio_domain::arena_assets::arena_asset_contract(game_id.as_deref(), platform);
            let Some(inputs) = ws.display().arena_asset_inputs(arena_id)? else { return Ok(None) };
            let rendering = attached.then(|| PathBuf::from(&roots.rendering));
            let mut view = studio_domain::arena_assets::arena_assets_with_levels(&inputs, contract, rendering.as_deref(), game_id.as_deref(), levels.as_ref());
            if attached && roots.platform() != platform {
                view.notes.push(format!(
                    "the attached install is read as `{}` while the open database is `{}`; the arena-asset contract follows the database's title and platform",
                    roots.platform().as_str(),
                    platform.as_str()
                ));
            }
            Ok(Some(convert::arena_assets_dto(&view, install_dir.as_deref())))
        })
    }

    /// Phase 3 (22 Sep 2026): every player's portrait / head containment under the open title's
    /// measured contract — the ids, `hasportrait` as stored, what the attached install ships, the
    /// shipped-but-unreferenced pools and the "shipped without asset" names. Read-only.
    pub fn player_containment(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
    ) -> Result<crate::dto::PlayerContainmentViewDto, ApiError> {
        let attached = !args.install_dir.trim().is_empty();
        let roots = resolve_roots(args);
        // Receipt 67: archive-held titles (PS3 NHL 09 → 14, Legacy Recompiled — the reassignment's set) are checked against the
        // install's own archive members where the loose trees are absent; the directories are read outside the workspace lock.
        let (game0, platform0) = self.with_workspace(id, |ws| Ok((ws.game_id().map(str::to_string), ws.display().project().platform)))?;
        let archive_held = (platform0 == Platform::Ps3 && matches!(game0.as_deref(), Some("nhl09" | "nhl10" | "nhl11" | "nhl12" | "nhl13" | "nhl14"))) || platform0 == Platform::Recomp;
        let index = if attached && archive_held { Some(studio_domain::archive_index::ArchiveIndex::build(Path::new(&args.install_dir)).map_err(ApiError::Domain)?) } else { None };
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().map(str::to_string);
            let platform = ws.display().project().platform;
            let contract = studio_domain::player_containment::containment_contract(game_id.as_deref(), platform);
            let (rows, has_flag) = ws
                .display()
                .player_bio_rows()?
                .ok_or_else(|| ApiError::Workspace("the open database has no player bio table".into()))?;
            let fe = attached.then(|| PathBuf::from(&roots.fe));
            let rendering = attached.then(|| PathBuf::from(&roots.rendering));
            let mut view = studio_domain::player_containment::player_containment_with_archives(&rows, has_flag, contract, fe.as_deref(), rendering.as_deref(), index.as_ref());
            if attached && roots.platform() != platform {
                view.summary.notes.push(format!(
                    "the attached install is read as `{}` while the open database is `{}`; the containment contract follows the database's title and platform",
                    roots.platform().as_str(),
                    platform.as_str()
                ));
            }
            Ok(convert::player_containment_dto(&view))
        })
    }

    /// Players 3.4 (receipt 38): plan — and with `apply`, perform — the reassignment of a player's
    /// portrait (`artid`) or head (`headid`) to another asset the attached tree ships. The plan is
    /// re-derived from the current bytes and files on every call; an apply of a refused plan writes
    /// nothing. The write is one ordinary field edit (pending review, undo / redo, save).
    ///
    /// Receipt 63: an apply names the plan fingerprint it reviewed (`reviewed`) and is refused when the plan re-derived now
    /// differs; PS4 heads are checked against the open title's vault (`defs_root` names the identity records), and a paired
    /// NHL 17–21 write (`headid` + `headtexture`) is one operation.
    #[allow(clippy::too_many_arguments)]
    ///
    /// Receipt 66: `hair` (PS4 NHL 17–21, `hairid` against the vault's `GenericHairStyle` values). The vault members and
    /// identity definitions are re-read and hashed on every call — Check and Apply alike — so an apply is always checked
    /// against the source as it is now; the plan carries those hashes and the fingerprint binds them. `target` is signed
    /// so a negative id reaches the plan and is refused there rather than failing to deserialize.
    pub fn player_reassign(&self, id: u32, args: &AssetRootsArgsDto, bio_record: u32, kind: &str, target: i64, apply: bool, reviewed: Option<&str>, defs_root: &Path) -> Result<crate::dto::PlayerReassignPlanDto, ApiError> {
        use studio_domain::player_reassign::{plan_hair_reassign, plan_reassign, AssetKind, Ps4Hair, Ps4Heads};
        let kind = match kind {
            "portrait" => AssetKind::Portrait,
            "head" => AssetKind::Head,
            "hair" => AssetKind::Hair,
            other => return Err(ApiError::Workspace(format!("unknown asset kind `{other}` (portrait / head / hair)"))),
        };
        if target < 0 && kind != AssetKind::Hair {
            return Err(ApiError::Workspace(format!("{} id {target} is negative; nothing was written", if kind == AssetKind::Head { "head" } else { "portrait" })));
        }
        let attached = !args.install_dir.trim().is_empty();
        let roots = resolve_roots(args);
        // Receipt 55: archive-held titles (PS3 NHL 09 → 14, Legacy Recompiled) are checked against the install's own archive
        // members too; the directories are read outside the workspace lock.
        let (game0, platform0) = self.with_workspace(id, |ws| Ok((ws.game_id().map(str::to_string), ws.display().project().platform)))?;
        let archive_held = (platform0 == Platform::Ps3 && matches!(game0.as_deref(), Some("nhl09" | "nhl10" | "nhl11" | "nhl12" | "nhl13" | "nhl14"))) || platform0 == Platform::Recomp;
        let index = if attached && archive_held { Some(studio_domain::archive_index::ArchiveIndex::build(Path::new(&args.install_dir)).map_err(ApiError::Domain)?) } else { None };
        // PS4 heads / hair: the open title's vault, read and identified outside the workspace lock on every call; only the
        // parse is cached, keyed by the exact identity just read.
        let needs_vault = platform0 == Platform::Ps4 && matches!(kind, AssetKind::Head | AssetKind::Hair) && attached;
        const NO_TITLE: &str = "the open project has no recorded title, so no vault catalogue can be chosen";
        let install = Path::new(&args.install_dir);
        let heads = match (needs_vault && kind == AssetKind::Head, game0.as_deref()) {
            (false, _) => None,
            (true, None) => Some(Err(NO_TITLE.to_string())),
            (true, Some(t)) => Some(self.ps4_head_catalogue(install, defs_root, t)),
        };
        // Hair is admitted on NHL 17–21 only; other titles are refused by the plan without reading the vault.
        let hair_title = game0.as_deref().filter(|g| studio_domain::ps4_hair::HAIR_TITLES.contains(g));
        let hair = match (needs_vault && kind == AssetKind::Hair, game0.as_deref(), hair_title) {
            (false, _, _) => None,
            (true, None, _) => Some(Err(NO_TITLE.to_string())),
            (true, Some(_), None) => None,
            (true, Some(_), Some(t)) => Some(self.ps4_hair_catalogue(install, defs_root, t)),
        };
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().map(str::to_string);
            let platform = ws.display().project().platform;
            let db = tdb_core::TdbFile::parse(ws.display().bytes()).map_err(|e| ApiError::Workspace(format!("database parse: {e}")))?;
            let fe = attached.then(|| PathBuf::from(&roots.fe));
            let rendering = attached.then(|| PathBuf::from(&roots.rendering));
            let meta = ws.display().project().meta();
            let plan = if kind == AssetKind::Hair {
                let source = match &hair {
                    None => Ps4Hair::NotAttached,
                    Some(Err(e)) => Ps4Hair::Problem(e.clone()),
                    Some(Ok(c)) => Ps4Hair::Catalogue(c),
                };
                plan_hair_reassign(game_id.as_deref(), platform, &db, meta, source, bio_record, target)
            } else {
                let source = match &heads {
                    None => Ps4Heads::NotAttached,
                    Some(Err(e)) => Ps4Heads::Problem(e.clone()),
                    Some(Ok(c)) => Ps4Heads::Catalogue(c),
                };
                plan_reassign(game_id.as_deref(), platform, &db, meta, fe.as_deref(), rendering.as_deref(), index.as_ref(), source, bio_record, kind, target as u64)
            }
            .map_err(ApiError::Domain)?;
            let what = match kind {
                AssetKind::Portrait => "portrait",
                AssetKind::Head => "head",
                AssetKind::Hair => "hair style",
            };
            let applied = if apply {
                if !plan.allowed {
                    return Err(ApiError::Workspace(format!("{} {what} → {target}: not applied — {}", plan.player, plan.refusals.join("; "))));
                }
                match reviewed {
                    None => return Err(ApiError::Workspace(format!("{} {what} → {target}: not applied — check the reassignment first, then apply the plan you reviewed", plan.player))),
                    Some(f) if f != plan.fingerprint => {
                        return Err(ApiError::Workspace(format!(
                            "{} {what} → {target}: not applied — the plan checked now differs from the one reviewed (the cells it writes, its evidence, the attached source's identity or what it discloses changed); check it again. Nothing was written",
                            plan.player
                        )))
                    }
                    Some(_) => {}
                }
                Some(convert::outcome_dto(if plan.writes.len() == 1 {
                    ws.set_field("ubPc", bio_record, &plan.field, FieldValue::Int(target))?
                } else {
                    let writes: Vec<(String, i64)> = plan.writes.iter().map(|(f, v)| (f.clone(), *v as i64)).collect();
                    ws.set_cells_int("ubPc", bio_record, &writes)?
                }))
            } else {
                None
            };
            Ok(crate::dto::PlayerReassignPlanDto {
                kind: match plan.kind {
                    AssetKind::Portrait => "portrait".into(),
                    AssetKind::Head => "head".into(),
                    AssetKind::Hair => "hair".into(),
                },
                bio_record: plan.bio_record,
                player: plan.player,
                current_id: plan.current_id,
                target_id: plan.target_id,
                allowed: plan.allowed,
                refusals: plan.refusals,
                evidence: plan.evidence,
                notes: plan.notes,
                source_identity: plan.source,
                written_fields: plan.writes.iter().map(|(f, _)| f.clone()).collect(),
                fingerprint: plan.fingerprint,
                applied,
            })
        })
    }

    /// Misc 5.4 (22 Sep 2026): the PS4 song catalogue listing — what a team's goal / power-play /
    /// win song can be chosen from (research R54), never which song the team has. Read-only.
    pub fn song_catalogue(&self, id: u32, install_root: &str, query: &str, limit: usize) -> Result<crate::dto::SongCatalogueDto, ApiError> {
        self.with_workspace(id, |ws| {
            let game_id = ws.game_id().map(str::to_string);
            let platform = ws.display().project().platform;
            // Keys resolve through this project's own attached dictionary — never a sibling's.
            let dictionary = ws.display().dictionary();
            let l = studio_domain::song_catalogue::song_listing(game_id.as_deref(), platform, Path::new(install_root), query, limit.clamp(1, 5000), dictionary).map_err(ApiError::Domain)?;
            Ok(crate::dto::SongCatalogueDto {
                available: l.available,
                reason: l.reason,
                source: l.source,
                rows_total: l.rows_total,
                composers: l.composers,
                genre_counts: l
                    .genre_counts
                    .into_iter()
                    .map(|g| crate::dto::SongGenreCountDto { genre: g.genre, key: g.key, label: g.label, lookup: g.lookup, rows: g.rows })
                    .collect(),
                dictionary: l.dictionary.map(|d| crate::dto::DictionaryStampDto { locale: d.locale, sha256: d.sha256, rows: d.rows }),
                query: l.query,
                matching: l.matching,
                truncated: l.truncated,
                rows: l
                    .rows
                    .into_iter()
                    .map(|r| crate::dto::SongRowDto {
                        index: r.index,
                        identifier: r.identifier,
                        artid: r.artid,
                        audioid: r.audioid,
                        composer: r.composer,
                        genre: r.genre,
                        genre_key: r.genre_key,
                        is_goal: r.is_goal,
                        is_intro: r.is_intro,
                        is_powerplay: r.is_powerplay,
                        is_win: r.is_win,
                        name: r.name,
                        title: r.title,
                        title_lookup: r.title_lookup,
                        genre_label: r.genre_label,
                        genre_lookup: r.genre_lookup,
                    })
                    .collect(),
                notes: l.notes,
            })
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
            Ok(convert::outcome_dto(
                ws.set_field(table, record, field, value)?,
            ))
        })
    }

    /// C8: several integer cells of one record written as **one** operation (one history entry,
    /// one undo) — the team colour triples (`_r`, `_g`, `_b`) and any other multi-field value that
    /// must not be left half-written. Mirrored into the roster save like `set_field`.
    pub fn set_cells_int(&self, id: u32, table: &str, record: u32, writes: &[(String, i64)]) -> Result<OpOutcomeDto, ApiError> {
        if writes.is_empty() {
            return Err(ApiError::Workspace("set_cells_int needs at least one cell".into()));
        }
        self.with_workspace(id, |ws| Ok(convert::outcome_dto(ws.set_cells_int(table, record, writes)?)))
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
            Ok(convert::outcome_dto(
                ws.move_player(player, from_team, to_team)?,
            ))
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

    pub fn release_to_fa(
        &self,
        id: u32,
        from_team: u64,
        player: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.release_to_fa(from_team, player)?))
        })
    }

    pub fn sign_from_fa(
        &self,
        id: u32,
        player: u64,
        to_team: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.sign_from_fa(player, to_team)?))
        })
    }

    pub fn add_membership(
        &self,
        id: u32,
        player: u64,
        to_team: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
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
            Ok(convert::outcome_dto(
                ws.remove_membership(from_team, player)?,
            ))
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
            Ok(convert::outcome_dto(
                ws.set_jersey_number(team, player, number)?,
            ))
        })
    }

    pub fn retire_jersey(&self, id: u32, team: u64, number: u64) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.retire_jersey(team, number)?))
        })
    }

    /// Clone an `xjgw` jersey row into a new kit on `team` — the DB half of "Add New Jersey"
    /// (append + copy + identity stamp; staged as a pending, undoable edit like any other op).
    #[allow(clippy::too_many_arguments)]
    pub fn clone_jersey(
        &self,
        id: u32,
        source_record: u32,
        team: u64,
        style: u64,
        art_id: u64,
        variant: u64,
        jersey_index: Option<u64>,
    ) -> Result<crate::dto::CloneJerseyResultDto, ApiError> {
        self.with_workspace(id, |ws| {
            let (record, outcome) =
                ws.clone_jersey(source_record, team, style, art_id, variant, jersey_index)?;
            Ok(crate::dto::CloneJerseyResultDto {
                record,
                outcome: convert::outcome_dto(outcome),
            })
        })
    }

    /// The in-game jersey-select thumbnail for a kit's `{style}_{art}_{variant}` asset id,
    /// resolved through the game's own fallback chain. Soft-misses with `found: false`.
    pub fn team_jersey_thumbnail(
        &self,
        args: &AssetRootsArgsDto,
        style: u32,
        art_id: u32,
        variant: u32,
    ) -> Result<crate::dto::JerseyThumbnailDto, ApiError> {
        let roots = resolve_roots(args);
        let Some(t) = studio_domain::jersey_thumbnail(&roots, style, art_id, variant) else {
            return Ok(crate::dto::JerseyThumbnailDto {
                found: false,
                data_url: None,
                width: 0,
                height: 0,
                rel_path: None,
                stock: false,
            });
        };
        let (width, height) = (t.image.width, t.image.height);
        let png = t.image.to_png().map_err(ApiError::Domain)?;
        // Only the kit's own stock archive under the install root is a legal replace target —
        // the shared custom fallbacks back every custom kit and must not be clobbered.
        let rel_path = if t.stock {
            t.path
                .strip_prefix(Path::new(&args.install_dir))
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
        } else {
            None
        };
        Ok(crate::dto::JerseyThumbnailDto {
            found: true,
            data_url: Some(format!("data:image/png;base64,{}", BASE64.encode(&png))),
            width,
            height,
            rel_path,
            stock: t.stock,
        })
    }

    /// Clone a kit's texture files (outfit folders + FE thumbnail) onto a new asset id — the file
    /// half of "Add New Jersey". Pure filesystem; never clobbers existing destination files.
    #[allow(clippy::too_many_arguments)]
    pub fn clone_kit_textures(
        &self,
        args: &AssetRootsArgsDto,
        src_style: u32,
        src_art_id: u32,
        src_variant: u32,
        dst_style: u32,
        dst_art_id: u32,
        dst_variant: u32,
    ) -> Result<crate::dto::CloneKitTexturesSummaryDto, ApiError> {
        let roots = resolve_roots(args);
        // ONB-002: under a project the clones are written into the project's overlay (the install
        // is read, never written); each output is recorded as a staged file.
        let overlay = self.project_for_install_dir(&args.install_dir).map(|(id, dir)| (id, dir.join(studio_domain::project_store::WORKING_DIR).join(studio_domain::project_store::OVERLAY_DIR)));
        let summary = studio_domain::clone_kit_textures_to(
            Path::new(&args.install_dir),
            &roots,
            (src_style, src_art_id, src_variant),
            (dst_style, dst_art_id, dst_variant),
            overlay.as_ref().map(|(_, o)| o.as_path()),
        )
        .map_err(ApiError::Domain)?;
        if let Some((id, _)) = &overlay {
            for f in &summary.files {
                self.record_staged_file(*id, &args.install_dir, &f.dst, "kit-clone")?;
            }
        }
        Ok(crate::dto::CloneKitTexturesSummaryDto {
            files: summary
                .files
                .into_iter()
                .map(|f| crate::dto::ClonedKitFileDto {
                    src: f.src,
                    dst: f.dst,
                    rewrites: f.rewrites,
                })
                .collect(),
            skipped_existing: summary.skipped_existing,
        })
    }

    pub fn unretire_jersey(
        &self,
        id: u32,
        team: u64,
        number: u64,
    ) -> Result<OpOutcomeDto, ApiError> {
        self.with_workspace(id, |ws| {
            Ok(convert::outcome_dto(ws.unretire_jersey(team, number)?))
        })
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
        // A Save Editor session always saves through its backup-first path.
        if self.is_save_editor(id) {
            return self.save_editor_save(id).map(|r| u64::from(r.files_written));
        }
        self.save_workspace_unchecked(id)
    }

    /// [`Registry::save_workspace`] without the Save Editor routing.
    pub(crate) fn save_workspace_unchecked(&self, id: u32) -> Result<u64, ApiError> {
        self.with_workspace(id, |ws| ws.save_in_place().map_err(ApiError::Domain))
    }

    /// Write the canonical db to an explicit path ("Save As" / legacy single-file). Returns bytes.
    /// Refused when the project or the target path is held by a copied-target operation. The
    /// target is checked under its reservation identity (canonical directory, resolved final
    /// component, case folded where the file system folds — S2-C-R2), so an equivalent spelling
    /// of a held file is refused too; a target whose identity cannot be established is refused
    /// rather than written.
    pub fn save(&self, id: u32, out_path: &str) -> Result<u64, ApiError> {
        // ONB-002: a Save As never lands inside the game installation, the save location or an
        // imported source of any open project.
        self.refuse_guarded_write(None, "Save As", Path::new(out_path))?;
        let target = copied_target::reservation_key(Path::new(out_path))
            .map_err(|e| ApiError::CopiedTarget(format!("cannot save to {out_path}: {e}")))?;
        let mut guard = self.inner.lock().unwrap();
        if let Some(r) = guard.reservation_for_project(id) {
            return Err(ApiError::CopiedTarget(format!(
                "project {id} is locked by a copied-target operation in progress ({}); wait for it",
                r.holder()
            )));
        }
        if let Some(r) = guard.reservation_covering(std::slice::from_ref(&target)) {
            return Err(ApiError::CopiedTarget(format!(
                "{} is held by a copied-target operation in progress ({}); it cannot be written until that finishes",
                copied_target::display_path(&target),
                r.holder()
            )));
        }
        let ws = guard
            .sessions
            .get_mut(&id)
            .ok_or(ApiError::UnknownProject(id))?;
        ws.save_primary_to(out_path).map_err(ApiError::Domain)
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
        *self.ledger.lock().unwrap() = dir
            .as_ref()
            .map(|d| InstanceRegistry::open(d.join("ea_import_ledger.db")));
        *self.corpus_dir.lock().unwrap() = dir;
        *self.corpus_db_path.lock().unwrap() = Some(PathBuf::from(path));
        // Seed the starter profile + state file so the Formulas dropdown is never empty and the
        // Legends banner has a profile to report on. Best-effort: a read-only corpus dir must not
        // stop the corpus from opening.
        let _ = self.ensure_default_profile();
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
                        let already = it.lowered()
                            && self.was_imported(&ledger, it.person_id, it.season_year, &target);
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
    fn maxima_for(
        &self,
        profile: &studio_domain::FormulaProfile,
    ) -> Result<Arc<AttributeMaxima>, ApiError> {
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
            Ok(convert::historical_filters_dto(
                &c.seasons()?,
                &c.leagues()?,
            ))
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
    pub fn historical_player(
        &self,
        person_id: i64,
    ) -> Result<Option<HistoricalPersonDto>, ApiError> {
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
            Ok(
                compute_person_career_ratings(c, person_id, &profile, &maxima)?
                    .as_ref()
                    .map(convert::historical_career_ratings_dto),
            )
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

    /// Export one CSV row per player installed in the loaded database: identity, export slug, team
    /// and league, and whether a portrait / head mesh actually resolves on disk for them.
    ///
    /// The art columns are file truth, so this needs the install tree — a project opened without one
    /// would report every player as art-less, which is worse than refusing.
    pub fn export_player_manifest(
        &self,
        id: u32,
        args: &AssetRootsArgsDto,
        out_path: &str,
    ) -> Result<PlayerManifestSummaryDto, ApiError> {
        if args.install_dir.trim().is_empty() {
            return Err(ApiError::Workspace(
                "no install tree is attached to this project, so portrait/head-mesh presence can't be checked".into(),
            ));
        }
        let roots = resolve_roots(args);
        let out_path = out_path.to_string();
        self.with_workspace(id, |ws| {
            let players = ws.display().player_summaries()?;
            let teams = ws.display().team_summaries()?;
            let mut assets = ws.display().player_assets(&roots)?;
            // Placeholder portraits can only be spotted with every player in hand, so flag them
            // before the join — otherwise EA's shared blank counts as a real portrait.
            studio_domain::mark_generic_portraits(&mut assets);

            let rows = studio_domain::build_player_manifest(&players, &teams, &assets);
            let summary = studio_domain::summarize_manifest(&rows);
            std::fs::write(&out_path, studio_domain::manifest_to_csv(&rows)).map_err(|e| {
                ApiError::Io {
                    path: out_path.clone(),
                    source: e,
                }
            })?;
            Ok(PlayerManifestSummaryDto {
                path: out_path.clone(),
                players: summary.players as i64,
                with_portrait: summary.with_portrait as i64,
                with_head_mesh: summary.with_head_mesh as i64,
                with_slug: summary.with_slug as i64,
            })
        })
    }

    /// Compute one player's **Legends composite** live, under an arbitrary (possibly unsaved)
    /// profile — the Formulas live preview's "Legends" season option.
    ///
    /// This deliberately bypasses the `legends_cache.db` sidecar: the preview exists to show what
    /// the weights you are editing right now would produce, and the cache only ever holds *saved*
    /// profiles. It is one player's seasons, so it is cheap enough to run per keystroke-debounce.
    pub fn compute_legends_preview(
        &self,
        person_id: i64,
        profile: &FormulaProfileDto,
    ) -> Result<Option<EaComputedRatingsDto>, ApiError> {
        let profile = convert::formula_profile_from_dto(profile);
        let maxima = match profile.normalisation_mode {
            NormalizationMode::PositionMax => self.attribute_maxima()?,
            NormalizationMode::Nominal => Arc::new(AttributeMaxima::default()),
        };
        self.with_corpus(|c| {
            let Some(r) = compute_person_legends_ratings(c, person_id, &profile, &maxima)? else {
                return Ok(None);
            };
            let computed = studio_domain::ComputedRatings {
                set: r.set,
                fhm_values: r.fhm_values,
            };
            Ok(Some(convert::ea_computed_ratings_dto(&computed)))
        })
    }

    // ---- Legends builder -------------------------------------------------

    /// The active profile + Legends pending flags, persisted as `ea_formula_state.json` next to the
    /// corpus. Missing/corrupt reads fall back to the seeded default rather than erroring — losing
    /// this file must never make the Formulas tab unopenable.
    pub fn formula_state(&self) -> Result<FormulaState, ApiError> {
        let Ok(path) = self.formula_state_path() else {
            return Ok(FormulaState::default());
        };
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Ok(FormulaState::default());
        };
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    fn write_formula_state(&self, state: &FormulaState) -> Result<(), ApiError> {
        let path = self.formula_state_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ApiError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| ApiError::Workspace(format!("serialise formula state: {e}")))?;
        std::fs::write(&path, json).map_err(|e| ApiError::Io {
            path: path.display().to_string(),
            source: e,
        })
    }

    /// Switch the active profile. Loading a profile whose Legends run is missing or stale leaves the
    /// pending flag set, so the banner follows the profile rather than the save that created it.
    pub fn set_active_profile(&self, name: &str) -> Result<(), ApiError> {
        let mut state = self.formula_state()?;
        state.active_profile = name.trim().to_string();
        state.legends_dismissed = false;
        state.legends_pending = !self.legends_run_is_current(&state.active_profile)?;
        self.write_formula_state(&state)
    }

    /// Record that a Legends calculation is owed (the user saved a profile and declined to run it).
    pub fn set_legends_pending(&self, pending: bool, dismissed: bool) -> Result<(), ApiError> {
        let mut state = self.formula_state()?;
        state.legends_pending = pending;
        state.legends_dismissed = dismissed;
        self.write_formula_state(&state)
    }

    /// Seed the profile store with the packaged preset as **Default** the first time a corpus opens,
    /// so the profile dropdown is never empty and there is always a named profile to key a Legends
    /// run on. Existing files are never overwritten.
    pub fn ensure_default_profile(&self) -> Result<(), ApiError> {
        let path = self.profile_path(DEFAULT_PROFILE)?;
        if !path.exists() {
            self.save_formula_profile(DEFAULT_PROFILE, &self.formula_default())?;
        }
        if self.formula_state_path().is_ok_and(|p| !p.exists()) {
            self.write_formula_state(&FormulaState::default())?;
        }
        Ok(())
    }

    /// Where the Legends precompute stands for the active profile.
    pub fn legends_status(&self) -> Result<LegendsStatusDto, ApiError> {
        let state = self.formula_state()?;
        let slug = Self::profile_slug(&state.active_profile)?;
        let cache = self.open_legends_cache()?;
        let run = cache.run(&slug).map_err(ApiError::from)?;
        let fingerprint = self.with_corpus(|c| c.legends_fingerprint().map_err(ApiError::from))?;
        let stale = run.as_ref().is_some_and(|r| r.fingerprint != fingerprint);
        let computed = run.as_ref().is_some_and(|r| r.fingerprint == fingerprint);
        Ok(LegendsStatusDto {
            active_profile: state.active_profile,
            computed,
            stale,
            computed_at: run.as_ref().map(|r| r.computed_at.clone()),
            player_count: run.as_ref().map(|r| r.player_count).unwrap_or(0),
            // A stale run is as good as no run — keep asking until it's rebuilt.
            pending: state.legends_pending || stale || run.is_none(),
            dismissed: state.legends_dismissed,
        })
    }

    /// Run the Legends precompute for the active profile and store it in the sidecar cache.
    ///
    /// This is a full-corpus pass (every person, every season), so it fans out across the machine's
    /// cores. The corpus handle is read-only and not `Sync`, so each worker opens its **own**
    /// read-only connection to the same file — the same pattern `compute_maxima_at` uses.
    pub fn compute_legends_ratings(&self) -> Result<LegendsComputeSummaryDto, ApiError> {
        let started = std::time::Instant::now();
        let state = self.formula_state()?;
        let slug = Self::profile_slug(&state.active_profile)?;
        let profile_dto = self
            .load_formula_profile(&state.active_profile)
            .unwrap_or_else(|_| {
                self.get_active_formula()
                    .unwrap_or_else(|_| self.formula_default())
            });
        let profile = convert::formula_profile_from_dto(&profile_dto);
        let maxima = match profile.normalisation_mode {
            NormalizationMode::PositionMax => self.attribute_maxima()?,
            NormalizationMode::Nominal => Arc::new(AttributeMaxima::default()),
        };

        let db_path = self
            .corpus_db_path
            .lock()
            .unwrap()
            .clone()
            .ok_or(ApiError::CorpusNotOpen)?;
        let (candidates, fingerprint) = self.with_corpus(|c| {
            Ok((
                c.legends_candidates().map_err(ApiError::from)?,
                c.legends_fingerprint().map_err(ApiError::from)?,
            ))
        })?;
        let examined = candidates.len() as i64;

        let workers = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(1)
            .min(8);
        let chunk = candidates.len().div_ceil(workers.max(1)).max(1);

        let rated: Vec<CachedRating> = std::thread::scope(|scope| {
            let handles: Vec<_> = candidates
                .chunks(chunk)
                .map(|slice| {
                    let profile = &profile;
                    let maxima = &maxima;
                    let db_path = db_path.as_path();
                    scope.spawn(move || -> Vec<CachedRating> {
                        // A worker that can't open the corpus contributes nothing rather than
                        // poisoning the whole pass; the shortfall shows up in `rated`.
                        let Ok(corpus) = FhmCorpus::open(db_path) else {
                            return Vec::new();
                        };
                        let mut out = Vec::with_capacity(slice.len());
                        for c in slice {
                            let Ok(Some(r)) = compute_person_legends_ratings(
                                &corpus,
                                c.person_id,
                                profile,
                                maxima,
                            ) else {
                                continue;
                            };
                            let Some(overall) = r.set.overall else {
                                continue;
                            };
                            let Ok(attrs_json) = serde_json::to_string(&r.set.ratings) else {
                                continue;
                            };
                            out.push(CachedRating {
                                person_id: c.person_id,
                                position: r.position,
                                overall: overall as i64,
                                seasons_used: r.seasons_used,
                                archetype: r.set.archetype.map(|a| format!("{a:?}")),
                                attrs_json,
                            });
                        }
                        out
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_default())
                .collect()
        });

        let run = LegendsRun {
            profile_slug: slug,
            profile_name: state.active_profile.clone(),
            fingerprint,
            computed_at: timestamp(),
            player_count: rated.len() as i64,
        };
        let mut cache = self.open_legends_cache()?;
        cache.replace_run(&run, &rated).map_err(ApiError::from)?;

        // Build the rosters straight away and store them beside the ratings. Assignment is a
        // global pass over the whole field, so doing it here — once, while the user is already
        // waiting on a long job — is what keeps Puck's Picks instant afterwards.
        self.materialize_legends_assignment(&run.profile_slug)?;

        // The debt is settled — clear both flags so the banner disappears.
        self.set_legends_pending(false, false)?;

        Ok(LegendsComputeSummaryDto {
            profile: state.active_profile,
            examined,
            rated: rated.len() as i64,
            elapsed_ms: started.elapsed().as_millis() as i64,
        })
    }

    /// Every built Legends team for the active profile — a read of the materialised roster tables,
    /// not a fresh assignment pass.
    pub fn legends_teams(&self) -> Result<Vec<LegendsTeamSummaryDto>, ApiError> {
        let slug = self.active_profile_slug()?;
        self.ensure_legends_assignment(&slug)?;
        let cache = self.open_legends_cache()?;
        Ok(cache
            .teams(&slug)
            .map_err(ApiError::from)?
            .into_iter()
            .map(|t| LegendsTeamSummaryDto {
                key: t.team_key,
                name: t.name,
                abbr: t.abbr,
                is_special: t.franchise_id.is_none(),
                forwards: t.forwards,
                defense: t.defense,
                goalies: t.goalies,
            })
            .collect())
    }

    /// One Legends team's roster — one indexed read of ~26 denormalised rows.
    pub fn legends_roster(&self, team_key: &str) -> Result<Vec<LegendsMemberDto>, ApiError> {
        let slug = self.active_profile_slug()?;
        self.ensure_legends_assignment(&slug)?;
        let cache = self.open_legends_cache()?;
        Ok(cache
            .roster(&slug, team_key)
            .map_err(ApiError::from)?
            .into_iter()
            .map(|m| LegendsMemberDto {
                person_id: m.person_id,
                first_name: m.first_name,
                last_name: m.last_name,
                position: m.position,
                is_goalie: m.is_goalie,
                dob: m.dob,
                nationality: m.nationality,
                overall: m.overall,
                team_gp: m.team_gp,
                second_choice: m.second_choice,
            })
            .collect())
    }

    fn active_profile_slug(&self) -> Result<String, ApiError> {
        Self::profile_slug(&self.formula_state()?.active_profile)
    }

    /// Materialise this profile's rosters if they aren't stored yet.
    ///
    /// Normally [`Self::compute_legends_ratings`] stores them, so this is a no-op. It matters for a
    /// cache written before the roster tables existed, and for a profile whose rosters were dropped
    /// — either way the rebuild happens once, not on every page view.
    fn ensure_legends_assignment(&self, slug: &str) -> Result<(), ApiError> {
        if self
            .open_legends_cache()?
            .has_assignment(slug)
            .map_err(ApiError::from)?
        {
            return Ok(());
        }
        self.materialize_legends_assignment(slug)
    }

    /// Run the assignment for `slug` and store the resulting teams + rosters in the sidecar.
    ///
    /// Everything the read path needs is denormalised here — the franchise's display name and each
    /// player's bio — so that browsing Puck's Picks never touches the corpus. That is the whole
    /// point: the corpus scans and the global sort happen once per calculation, not once per click.
    fn materialize_legends_assignment(&self, slug: &str) -> Result<(), ApiError> {
        let ratings: HashMap<i64, i64> = self
            .open_legends_cache()?
            .ratings(slug)
            .map_err(ApiError::from)?
            .into_iter()
            .map(|r| (r.person_id, r.overall))
            .collect();
        if ratings.is_empty() {
            return Err(ApiError::Workspace(format!(
                "no Legends ratings computed for profile {slug:?} — run the Legends calculation first"
            )));
        }

        let (candidates, tenure) = self.with_corpus(|c| {
            Ok((
                c.legends_candidates().map_err(ApiError::from)?,
                c.nhl_franchise_tenure().map_err(ApiError::from)?,
            ))
        })?;
        let assignment = assign_rosters(&candidates, &ratings, &tenure);
        let names = self.franchise_names()?;
        let by_id: HashMap<i64, &LegendsCandidate> =
            candidates.iter().map(|c| (c.person_id, c)).collect();

        let mut teams: Vec<CachedTeam> = Vec::with_capacity(assignment.teams.len());
        let mut members: Vec<CachedMember> = Vec::new();
        for t in assignment.teams.values() {
            let (name, abbr) = match t.franchise_id {
                Some(id) => names
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| (format!("Franchise {id}"), None)),
                None if t.key == EUROPEAN_ELITE_KEY => {
                    (EUROPEAN_ELITE_NAME.to_string(), Some("EUR".to_string()))
                }
                None => (WINDSOR_LEGENDS_NAME.to_string(), Some("WDS".to_string())),
            };
            teams.push(CachedTeam {
                team_key: t.key.clone(),
                name,
                abbr,
                franchise_id: t.franchise_id,
                forwards: t.forwards.len() as i64,
                defense: t.defense.len() as i64,
                goalies: t.goalies.len() as i64,
            });
            for m in t.members() {
                // A placed player is always in `candidates` (that's where they came from), so a
                // miss here would be a bug rather than missing data — skip rather than panic.
                let Some(c) = by_id.get(&m.person_id) else {
                    continue;
                };
                members.push(CachedMember {
                    team_key: t.key.clone(),
                    person_id: m.person_id,
                    slot: slot_code(m.slot).to_string(),
                    first_name: c.first_name.clone(),
                    last_name: c.last_name.clone(),
                    position: c.position.clone(),
                    is_goalie: c.is_goalie,
                    dob: c.dob.clone(),
                    nationality: c.nationality.clone(),
                    overall: m.overall,
                    team_gp: m.team_gp,
                    second_choice: m.second_choice,
                });
            }
        }

        let mut cache = self.open_legends_cache()?;
        cache
            .replace_assignment(slug, &teams, &members)
            .map_err(ApiError::from)
    }

    /// `franchise_id → (display name, abbr)` for the NHL lineage.
    fn franchise_names(&self) -> Result<HashMap<i64, (String, Option<String>)>, ApiError> {
        self.with_corpus(|c| {
            let index = c
                .team_index(Some("NHL"), "", 2000)
                .map_err(ApiError::from)?;
            Ok(index
                .into_iter()
                .filter(|e| e.kind == "franchise")
                .filter_map(|e| e.key.parse::<i64>().ok().map(|id| (id, (e.name, e.abbr))))
                .collect())
        })
    }

    fn open_legends_cache(&self) -> Result<LegendsCache, ApiError> {
        let dir = self
            .corpus_dir
            .lock()
            .unwrap()
            .clone()
            .ok_or(ApiError::CorpusNotOpen)?;
        LegendsCache::open(&dir.join("legends_cache.db")).map_err(ApiError::from)
    }

    /// Path of the active-profile/pending state: `<corpus>/ea_formula_state.json`.
    fn formula_state_path(&self) -> Result<std::path::PathBuf, ApiError> {
        let dir = self
            .corpus_dir
            .lock()
            .unwrap()
            .clone()
            .ok_or(ApiError::CorpusNotOpen)?;
        Ok(dir.join("ea_formula_state.json"))
    }

    /// True when `name`'s cached run exists and matches the open corpus.
    fn legends_run_is_current(&self, name: &str) -> Result<bool, ApiError> {
        let slug = Self::profile_slug(name)?;
        let cache = self.open_legends_cache()?;
        let Some(run) = cache.run(&slug).map_err(ApiError::from)? else {
            return Ok(false);
        };
        let fingerprint = self.with_corpus(|c| c.legends_fingerprint().map_err(ApiError::from))?;
        Ok(run.fingerprint == fingerprint)
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
        let slug = Self::profile_slug(name)?;
        Ok(self.profiles_dir()?.join(format!("{slug}.json")))
    }

    /// The filesystem/cache-safe key for a profile name. Also keys the Legends rating cache, so a
    /// profile's ratings follow it across renames of the *display* name only if the slug matches.
    fn profile_slug(name: &str) -> Result<String, ApiError> {
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
        Ok(slug)
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

    /// Insert a workspace under a fresh id and return its summary. Refused while a copied-target
    /// operation holds any of the workspace's files: a workspace opened on reserved files would be
    /// an alias that bypassed the reservation.
    fn register(&self, ws: Workspace) -> Result<ProjectSummaryDto, ApiError> {
        let mut guard = self.inner.lock().unwrap();
        let files = copied_target::workspace_files(&ws);
        if let Some(r) = guard.reservation_covering(&files) {
            return Err(ApiError::CopiedTarget(format!(
                "these files are held by a copied-target operation in progress ({}); wait for it to finish before opening them",
                r.holder()
            )));
        }
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
        if let Some(r) = guard.reservation_for_project(id) {
            return Err(ApiError::CopiedTarget(format!(
                "project {id} is locked by a copied-target operation in progress ({}); wait for it to finish (or cancel it) before editing, saving, reloading or closing it",
                r.holder()
            )));
        }
        if !guard.sessions.contains_key(&id) {
            drop(guard);
            // ONB-003: an assets-only project folder has no edit session; database routes say so
            // instead of "unknown project".
            if self.is_project_binding(id) {
                return Err(ApiError::Workspace(
                    "this project has no main database or roster save, so database editing is unavailable; add one through Settings → Project (link inputs) or create a project with one".into(),
                ));
            }
            return Err(ApiError::UnknownProject(id));
        }
        let ws = guard
            .sessions
            .get_mut(&id)
            .ok_or(ApiError::UnknownProject(id))?;
        // Save Editor: tables supplied from the game's database read as non-editable in every
        // view built during this call.
        let frozen = ws.frozen_tables().cloned();
        studio_domain::table_overlay::with_frozen_scope(frozen.as_ref(), || f(ws))
    }

    /// `true` when `id` has an edit session (false for an assets-only project).
    /// Hand the registry the verified baseline store (the shipped one, outside any checkout).
    pub fn set_baselines(&self, store: studio_domain::BaselineStore) {
        *self.baselines.lock().unwrap() = Some(std::sync::Arc::new(store));
    }

    /// The baseline store: the one set at startup, else the repository checkout's (development),
    /// else `None` (every consumer then refuses with "no bundled schema metadata").
    pub(crate) fn baselines(&self) -> Option<std::sync::Arc<studio_domain::BaselineStore>> {
        let mut guard = self.baselines.lock().unwrap();
        if guard.is_none() {
            let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            if let Ok(st) = studio_domain::BaselineStore::open(&repo.join("baselines").join(studio_domain::baselines::INVENTORY_FILE), &repo.join("anchors")) {
                *guard = Some(std::sync::Arc::new(st));
            }
        }
        guard.clone()
    }

    pub(crate) fn has_session(&self, id: u32) -> bool {
        self.inner.lock().unwrap().sessions.contains_key(&id)
    }

    /// ONB-003: allocate a project id with no edit session (an assets-only project folder).
    pub(crate) fn register_asset_only(&self) -> u32 {
        let mut guard = self.inner.lock().unwrap();
        let id = guard.next;
        guard.next += 1;
        id
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
/// Does an outfit texture file name belong to the kit `(style, variant)`? The
/// outfit files are `PREFIX_{style}_{team}_…_{variant}[_cm].rx2`; drop the
/// prefix word and any `_cm`, then style is the first number and variant the
/// last — the same parse the Uniforms tab uses to group textures into kits.
/// True when `file_name` is a genuine outfit **colour-map diffuse** — a
/// `texlib_*` bundle (jersey/pant, whose `_cm` raster is chosen internally) or a
/// loose `_cm` colour map (socks) — and not a mesh shape/alpha companion
/// (`playersock_*` / `goaliesock_*`, or a `_am`/`_nm`/`_sm`/`_rm`/`_dm` map).
/// Used to keep the kit preview from binding a sock mesh's grey alpha map instead
/// of the team colorway (which renders the sock white).
fn is_colorway_diffuse(file_name: &str) -> bool {
    let n = file_name.to_ascii_lowercase();
    let stem = n
        .strip_suffix(".rx2")
        .or_else(|| n.strip_suffix(".rpsgl"))
        .or_else(|| n.strip_suffix(".gnf"))
        .unwrap_or(n.as_str());
    // Sock/goalie-sock *mesh* files carry the shape's own maps, never a colorway.
    if stem.starts_with("playersock") || stem.starts_with("goaliesock") {
        return false;
    }
    stem.starts_with("texlib_") || stem.ends_with("_cm")
}

fn outfit_matches_kit(file_name: &str, style: u32, variant: u32) -> bool {
    let stem = file_name
        .strip_suffix(".rx2")
        .or_else(|| file_name.strip_suffix(".RX2"))
        .or_else(|| file_name.strip_suffix(".rpsgl"))
        .unwrap_or(file_name);
    let stem = stem.strip_suffix("_cm").unwrap_or(stem);
    let nums: Vec<u32> = stem
        .split('_')
        .skip(1) // prefix word (texlib / jersey / pant / sock / …)
        .filter_map(|t| t.parse::<u32>().ok())
        .collect();
    match (nums.first(), nums.last()) {
        (Some(&s), Some(&v)) if nums.len() >= 2 => s == style && v == variant,
        _ => false,
    }
}

/// Parse a helmet mesh file name `helmet_<style>_<variant>.rx2` → `(style,
/// variant)`, so its zone mask (`helmet_0_<style>_tm`) and detail overlay
/// (`helmet_0_<style>_<variant>_dm`) — both UV-mapped to that style — match.
fn helmet_style_variant(mesh: &Path) -> Option<(u32, u32)> {
    let stem = mesh.file_stem()?.to_str()?;
    let rest = stem.strip_prefix("helmet_")?;
    let (s, v) = rest.split_once('_')?;
    Some((s.parse().ok()?, v.parse().ok()?))
}

/// The three zone-mask files (+ optional detail) for a glove brand:
/// `feglove_<brand>_{0,1,2}_tm.rx2` and `feglove_<brand>_0_dm.rx2`. Falls back
/// to the first complete mask set present when the team's brand ships none, so
/// the glove is still painted with the team's zone colours rather than left bare.
fn resolve_glove_masks(glove_dir: &Path, brand: u32) -> (Vec<PathBuf>, Option<PathBuf>) {
    let set_for = |b: u32| -> Vec<PathBuf> {
        let masks: Vec<PathBuf> = (0..3)
            .map(|i| render_platform_file(glove_dir.join(format!("feglove_{b}_{i}_tm.rx2"))))
            .collect();
        if masks.iter().all(|p| p.is_file()) {
            masks
        } else {
            Vec::new()
        }
    };
    let mut masks = set_for(brand);
    let mut brand = brand;
    if masks.is_empty() {
        // Scan for any brand with a complete `_0/_1/_2_tm` set.
        if let Ok(rd) = std::fs::read_dir(glove_dir) {
            let mut brands: Vec<u32> = rd
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    n.strip_prefix("feglove_")
                        .and_then(|r| r.split_once('_'))
                        .filter(|(_, tail)| *tail == "0_tm.rx2" || *tail == "0_tm.rpsgl")
                        .and_then(|(b, _)| b.parse::<u32>().ok())
                })
                .collect();
            brands.sort_unstable();
            brands.dedup();
            for b in brands {
                let s = set_for(b);
                if !s.is_empty() {
                    masks = s;
                    brand = b;
                    break;
                }
            }
        }
    }
    let detail = render_platform_file(glove_dir.join(format!("feglove_{brand}_0_dm.rx2")));
    let detail = detail.is_file().then_some(detail);
    (masks, detail)
}

// ── Goalie equipment (Phase 1) ──────────────────────────────────────────────
//
// UNVERIFIED best-guess `hHbv.jJpM` equipment-type codes for the three nine-zone
// goalie pieces. The Legacy anchor carries types 1/2/3 (goalie brands 10–20) plus
// type 4 (mask, brand 0); which of 1/2/3 is blocker vs trapper vs pads is NOT
// derivable from the data alone (the prior tool only ever exposed skater 5/8 and
// handled goalie gear per-player via `fMVl`). Confirm in-game and correct here —
// this is the single source of truth for the mapping.
const GOALIE_TYPE_BLOCKER: u32 = 1;
const GOALIE_TYPE_TRAPPER: u32 = 2;
const GOALIE_TYPE_PADS: u32 = 3;

/// Fraction of its own height to lift the goalie mask so it sits on the head
/// instead of clipping through the forehead.
const MASK_RAISE_FRAC: f32 = 0.2;

/// Zone masks (+ optional `_dm` detail) for a goalie piece of a given brand, from
/// its `fe*` mask files (`feblocker_<model>_<brand>_<i>_tm`,
/// `fetrapper_<model>_<brand>_<i>_tm`, `fegoaliepad_<brand>_<i>_tm`). The brand is
/// the numeric token before the trailing mask index, so one rule covers all three
/// naming shapes. Falls back to the lowest brand present when the team's brand
/// ships no masks, so the piece is still painted rather than left bare.
fn resolve_goalie_masks(
    dir: &Path,
    fe_prefix: &str,
    brand: u32,
) -> (Vec<PathBuf>, Option<PathBuf>) {
    // (brand, mask index) from a stem whose last two numeric tokens are brand, index.
    let brand_idx = |core: &str| -> Option<(u32, u32)> {
        let nums: Vec<u32> = core
            .split('_')
            .filter_map(|t| t.parse::<u32>().ok())
            .collect();
        (nums.len() >= 2).then(|| (nums[nums.len() - 2], nums[nums.len() - 1]))
    };
    let Ok(rd) = std::fs::read_dir(dir) else {
        return (Vec::new(), None);
    };
    let files: Vec<(String, PathBuf)> = rd
        .filter_map(|e| e.ok())
        .map(|e| {
            (
                e.file_name().to_string_lossy().to_ascii_lowercase(),
                e.path(),
            )
        })
        .filter(|(n, _)| n.starts_with(fe_prefix) && (n.ends_with(".rx2") || n.ends_with(".rpsgl")))
        .collect();
    let masks_for = |b: u32| -> (Vec<PathBuf>, Option<PathBuf>) {
        let mut masks: Vec<(u32, PathBuf)> = Vec::new();
        let mut detail = None;
        for (n, p) in &files {
            let stem = n.strip_suffix(".rx2").or_else(||n.strip_suffix(".rpsgl")).unwrap_or(n);
            if let Some(core) = stem.strip_suffix("_tm") {
                if brand_idx(core).is_some_and(|(bb, _)| bb == b) {
                    masks.push((brand_idx(core).unwrap().1, p.clone()));
                }
            } else if let Some(core) = stem.strip_suffix("_dm") {
                if brand_idx(core).is_some_and(|(bb, _)| bb == b) {
                    detail = Some(p.clone());
                }
            }
        }
        masks.sort_by_key(|(i, _)| *i);
        (masks.into_iter().map(|(_, p)| p).collect(), detail)
    };
    let (masks, detail) = masks_for(brand);
    if !masks.is_empty() {
        return (masks, detail);
    }
    // Fall back to the lowest brand that ships a mask set.
    let mut brands: Vec<u32> = files
        .iter()
        .filter_map(|(n, _)| {
            let stem = n.strip_suffix(".rx2").or_else(||n.strip_suffix(".rpsgl")).unwrap_or(n);
            stem.strip_suffix("_tm").and_then(brand_idx).map(|(b, _)| b)
        })
        .collect();
    brands.sort_unstable();
    brands.dedup();
    for b in brands {
        let set = masks_for(b);
        if !set.0.is_empty() {
            return set;
        }
    }
    (Vec::new(), None)
}

/// The first `texlib_*.rx2` (sorted) in the skate folder — a stand-in skate
/// diffuse until per-player skate texturing is mapped.
fn first_skate_texlib(skate_dir: &Path) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(skate_dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name().and_then(|s| s.to_str()).is_some_and(|n| {
                n.starts_with("texlib_") && (n.to_ascii_lowercase().ends_with(".rx2") || n.to_ascii_lowercase().ends_with(".rpsgl"))
            })
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Which Hockey Shop slots belong to the goalie category strip.
fn is_goalie_slot(key: &str) -> bool {
    matches!(
        key,
        "goalie-stick" | "blocker" | "trapper" | "pads" | "goalie-helmet"
    )
}

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

/// N21-001: the PS4 roster save to open. A `chosen` path must be one of the inventory's recognised
/// roster saves; with no choice, a single recognised roster save opens and several are refused so
/// the user picks one (never the first by path order). `None` when there is no recognised roster
/// save — the template then opens browse-only, as before.
fn ps4_chosen_roster<'a>(
    report: &'a studio_domain::InstallReport,
    chosen: Option<&str>,
) -> Result<Option<&'a studio_domain::SaveSlot>, ApiError> {
    let rosters: Vec<&studio_domain::SaveSlot> =
        report.saves.iter().filter(|s| s.kind == "roster" && s.writable).collect();
    match chosen.filter(|c| !c.trim().is_empty()) {
        Some(c) => rosters
            .iter()
            .copied()
            .find(|s| Path::new(&s.path) == Path::new(c))
            .map(Some)
            .ok_or_else(|| {
                ApiError::Workspace(format!(
                    "the chosen roster save {c} isn't one of the roster saves Studio can open in this \
                     save-data location now; detect again and choose a save."
                ))
            }),
        None => match rosters.as_slice() {
            [] => Ok(None),
            [one] => Ok(Some(*one)),
            many => Err(ApiError::Workspace(format!(
                "{} roster saves were found ({}); choose which one to open.",
                many.len(),
                many.iter()
                    .map(|s| {
                        let save = Path::new(&s.path)
                            .parent()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        format!(
                            "{save}, profile {}, {}",
                            s.profile.as_deref().unwrap_or("?"),
                            s.title_id.as_deref().unwrap_or("?")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            ))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{is_colorway_diffuse, outfit_matches_kit};

    #[test]
    fn colorway_diffuse_excludes_sock_mesh_maps() {
        // Real colorways: sock `_cm` colour map + jersey/pant texlib bundle.
        assert!(is_colorway_diffuse("sock_0_0_0_cm.rx2"));
        assert!(is_colorway_diffuse("texlib_0_24_0.rx2"));
        // The sock mesh's own maps must never be taken as a colorway — this is
        // what made the Ducks (team 0) socks render white: `playersock_0_0_am`
        // matched (style 0, variant 0) and sorted ahead of `sock_0_0_0_cm`.
        assert!(!is_colorway_diffuse("playersock_0_0_am.rx2"));
        assert!(!is_colorway_diffuse("goaliesock_1_0_am.rx2"));
        assert!(!is_colorway_diffuse("playersock_0_0.rx2"));
        // Non-colour maps for a loose raster are excluded too.
        assert!(!is_colorway_diffuse("sock_0_0_0_nm.rx2"));
        assert!(!is_colorway_diffuse("sock_0_0_0_sm.rx2"));
    }

    #[test]
    fn outfit_matches_kit_reads_style_and_variant_tokens() {
        // jersey_{style}_{team}_..._{variant}_cm.rx2 → style=first, variant=last.
        assert!(outfit_matches_kit("jersey_0_10_2_0_0_0_cm.rx2", 0, 0));
        assert!(outfit_matches_kit("pant_1_10_0_3.rx2", 1, 3));
        assert!(outfit_matches_kit("playersock_2_4.rx2", 2, 4));
        // Wrong style or variant → no match.
        assert!(!outfit_matches_kit("jersey_1_10_2_0_0_0_cm.rx2", 0, 0));
        assert!(!outfit_matches_kit("pant_1_10_0_3.rx2", 1, 0));
        // Too few numeric tokens (only the team token) → no match.
        assert!(!outfit_matches_kit("jersey_10.rx2", 10, 10));
    }
}

// Resolve a companion on the same extracted platform tree.
fn preview_asset_path(path: &Path, install_dir: &str, overlay: Option<&Path>) -> PathBuf {
    if let (Some(overlay), Ok(relative)) = (overlay, path.strip_prefix(Path::new(install_dir))) {
        let staged = overlay.join(relative);
        if staged.is_file() { return staged; }
    }
    path.to_path_buf()
}

fn render_platform_file(path: PathBuf) -> PathBuf { if !path.is_file() && path.with_extension("rpsgl").is_file() {path.with_extension("rpsgl")} else {path} }
