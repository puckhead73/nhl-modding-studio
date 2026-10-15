//! Xbox 360 disc images (`p1.1-xbox360-iso-member-route`) — the [`Registry`] surface over
//! [`studio_domain::xiso_source`], [`studio_domain::iso_staging`] and
//! [`studio_domain::iso_patch`]: inspect an image, stage its database and dictionary into a
//! Studio-managed folder, open that folder as an ordinary single-database workspace (edits and
//! in-place saves land in the staging folder), review a plan, apply **that** plan as same-length
//! range patches on the image with hash-bound records, restore, and list the records.
//!
//! **Ownership (OVERNIGHT-DEV-R2 F2).** The route is project-bound: `iso_open` binds the new
//! project to its staging folder and image; every later call names the project, never a path.
//! A plan is a server-held review — the exact patches (replacement bytes included), the staged
//! files' identities, the manifest snapshot and the workspace revision it was computed for — and
//! an apply names the review it applies. At the mutation boundary the apply refuses when the
//! review is not this project's, the workspace is dirty or was edited since the review, the
//! staged bytes or the manifest changed since the review, the platform's emulator is running,
//! or the image / staged files are held by another operation. It then reserves the image and
//! the staged files (under their canonical reservation identity, so path aliases and a second
//! staging root for the same image conflict) together with every open workspace on them for the
//! duration — the same reservation the copied-target engine uses, so saves, reloads, closes,
//! other applies and restores on those files are refused with the holder named while it runs.
//! The reviewed plan is what is written; nothing is re-planned behind the review.
//!
//! **Not claimed.** A range write is not cancellable once started: it completes and verifies or
//! is put back from its record. There is no restart detection beyond the records on disk
//! (`iso_records` lists them, `writing` status included).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use studio_domain::iso_staging::{self, ApplyOutcome, PlannedFile, StagingManifest};
use studio_domain::{iso_patch, xiso_source};

use crate::dto::{
    IsoApplyDto, IsoAttemptedDto, IsoInspectDto, IsoLocatedDto, IsoPlanDto, IsoPlannedFileDto,
    IsoProbedArchiveDto, IsoRecordDto, IsoRestoreDto, IsoRestoreItemDto, IsoStagedDto,
    IsoStagedFileDto, IsoStatusDto, ProjectSummaryDto,
};
use crate::error::ApiError;
use crate::registry::copied_target::{reservation_key, ReservationGuard};
use crate::registry::{parse_platform, Registry};

// ---------------------------------------------------------------- state

/// A project opened through the image route: its staging folder, its image and the manifest as
/// it was when the project was opened.
pub(super) struct IsoBinding {
    root: PathBuf,
    manifest: StagingManifest,
}

/// A reviewed plan the registry holds until it is applied or superseded.
pub(super) struct IsoReview {
    project_id: u32,
    root: PathBuf,
    /// The workspace revision the plan was computed for.
    revision: String,
    /// The workspace had unsaved edits when reviewed (the plan describes the files on disk).
    dirty: bool,
    manifest: StagingManifest,
    planned: Vec<PlannedFile>,
}

/// Names of running processes, lower-cased executable stems (`xenia`, `rpcs3`, …), or why the
/// list could not be read (the gate then fails closed exactly as the live listing does).
type ProcessProbe = std::sync::Arc<dyn Fn() -> Result<Vec<String>, String> + Send + Sync>;

#[derive(Default)]
pub(super) struct IsoState {
    bindings: HashMap<u32, IsoBinding>,
    reviews: HashMap<u64, IsoReview>,
    next_review: u64,
    /// Test seam: replaces the live process listing.
    probe: Option<ProcessProbe>,
    /// Test seam: a fault injected into the next apply (an event to fail at, or an entry whose
    /// range write is cut short after `n` bytes).
    fault: Option<IsoFault>,
}

/// A fault the next apply injects (test seam).
#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum IsoFault {
    At(iso_patch::Event),
    Truncate { entry: String, after: usize },
}

impl iso_patch::Hooks for IsoFault {
    fn event(&mut self, e: &iso_patch::Event) -> std::io::Result<()> {
        match self {
            IsoFault::At(at) if at == e => Err(std::io::Error::other("injected fault")),
            _ => Ok(()),
        }
    }
    fn truncate_range_write(&mut self, entry: &str) -> Option<usize> {
        match self {
            IsoFault::Truncate { entry: e, after } if e == entry => Some(*after),
            _ => None,
        }
    }
}

impl IsoState {
    pub(super) fn forget_project(&mut self, project_id: u32) {
        self.bindings.remove(&project_id);
        self.reviews.retain(|_, r| r.project_id != project_id);
    }
}

// ---------------------------------------------------------------- emulator exclusion

/// The emulator executables that could hold the live source open for a platform (the mapping
/// the batch deploy helper uses: `deploy.ps1` `Test-EmulatorRunning`).
fn emulator_names(platform: &str) -> &'static [&'static str] {
    match platform {
        "ps4" => &["shadps4"],
        "ps3" => &["rpcs3"],
        "xbox" | "xbox360" => &["xenia", "xenia_canary", "xenia_edge"],
        "recomp" => &["nhllegacy"],
        _ => &["shadps4", "rpcs3", "xenia", "xenia_canary", "xenia_edge", "nhllegacy"],
    }
}

/// Lower-cased executable stems of every running process (best effort; an unavailable listing
/// is reported as such by the caller, never treated as "nothing running").
fn live_process_stems() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    let out = std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output();
    #[cfg(not(windows))]
    let out = std::process::Command::new("ps").args(["-eo", "comm="]).output();
    let out = out.map_err(|e| format!("the process list could not be read ({e})"))?;
    if !out.status.success() {
        return Err(format!("the process list could not be read (exit {})", out.status));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .lines()
        .filter_map(|l| {
            let name = if cfg!(windows) {
                l.trim_start_matches('"').split('"').next()?
            } else {
                l.trim()
            };
            let stem = Path::new(name).file_stem()?.to_string_lossy().to_lowercase();
            (!stem.is_empty()).then_some(stem)
        })
        .collect())
}

/// Refuse when an emulator for `platform` is running: it may hold the image open and read the
/// bytes mid-write.
fn refuse_if_emulator_running(probe: Option<ProcessProbe>, platform: &str) -> Result<(), ApiError> {
    let stems = match probe {
        Some(p) => p(),
        None => live_process_stems(),
    }
    .map_err(ApiError::DiscImage)?;
    let wanted = emulator_names(platform);
    let running: Vec<&String> = stems.iter().filter(|s| wanted.contains(&s.as_str())).collect();
    if running.is_empty() {
        Ok(())
    } else {
        Err(ApiError::DiscImage(format!(
            "{} is running; close the emulator before the image is written or restored (it may hold the image open)",
            running
                .iter()
                .map(|s| format!("{s}.exe"))
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

// ---------------------------------------------------------------- DTO helpers

fn located_dto(l: &xiso_source::Located) -> IsoLocatedDto {
    match l {
        xiso_source::Located::Loose { member } => IsoLocatedDto {
            member_path: member.path.clone(),
            member_sector: member.sector,
            member_size: member.size,
            entry_path: None,
            entry_index: None,
            entry_offset: None,
            stored_size: None,
            describe: l.describe(),
        },
        xiso_source::Located::InArchive {
            archive,
            entry_index,
            entry_path,
            entry_offset,
            stored_size,
            ..
        } => IsoLocatedDto {
            member_path: archive.path.clone(),
            member_sector: archive.sector,
            member_size: archive.size,
            entry_path: Some(entry_path.clone()),
            entry_index: Some(*entry_index),
            entry_offset: Some(*entry_offset),
            stored_size: Some(*stored_size),
            describe: l.describe(),
        },
    }
}

fn located_record_dto(r: &iso_staging::LocatedRecord) -> IsoLocatedDto {
    IsoLocatedDto {
        member_path: r.member_path.clone(),
        member_sector: r.member_sector,
        member_size: r.member_size,
        entry_path: r.entry.as_ref().map(|e| e.path.clone()),
        entry_index: r.entry.as_ref().map(|e| e.index),
        entry_offset: r.entry.as_ref().map(|e| e.offset),
        stored_size: r.entry.as_ref().map(|e| e.stored_size),
        describe: r.describe(),
    }
}

fn staged_dto(s: &iso_staging::Staged, project_id: Option<u32>) -> IsoStagedDto {
    IsoStagedDto {
        root: s.root.display().to_string(),
        image: s.manifest.image.display().to_string(),
        layout: s.manifest.layout.clone(),
        game_id: s.manifest.game_id.clone(),
        platform: s.manifest.platform.clone(),
        db_path: s.db_path.display().to_string(),
        meta_xml: s.meta_xml.display().to_string(),
        dictionary_path: s.dictionary_path.as_ref().map(|p| p.display().to_string()),
        files: s
            .manifest
            .files
            .iter()
            .map(|f| IsoStagedFileDto {
                role: f.role.clone(),
                staged: f.staged.clone(),
                prior_sha256: f.prior.sha256.clone(),
                prior_len: f.prior.len,
                located: located_record_dto(&f.located),
            })
            .collect(),
        reused: s.reused,
        staged_at: s.manifest.staged_at.clone(),
        project_id,
    }
}

fn record_dto(records_dir: &Path, r: &iso_patch::PatchRecord, on_image: &str) -> IsoRecordDto {
    IsoRecordDto {
        records_dir: records_dir.display().to_string(),
        record_dir: r.dir.display().to_string(),
        image: r.image.display().to_string(),
        member_path: r.member_path.clone(),
        entry_path: r.entry_path.clone(),
        start: r.start,
        end: r.end,
        prior_sha256: r.prior.sha256.clone(),
        replacement_sha256: r.replacement.sha256.clone(),
        status: r.status.as_str().to_string(),
        on_image: on_image.to_string(),
    }
}

fn attempted_dto(records_dir: &Path, a: &iso_staging::AttemptedRange) -> IsoAttemptedDto {
    IsoAttemptedDto {
        role: a.role.clone(),
        entry_path: a.entry_path.clone(),
        record: record_dto(records_dir, &a.record, a.state.as_str()),
        state: a.state.as_str().to_string(),
        restore_error: a.restore_error.clone(),
    }
}

fn planned_file_dto(p: &PlannedFile) -> IsoPlannedFileDto {
    IsoPlannedFileDto {
        role: p.role.clone(),
        staged: p.staged.clone(),
        staged_sha256: p.staged_identity.sha256.clone(),
        changed: p.patch.is_some(),
        member_path: p.patch.as_ref().map(|x| x.member_path.clone()),
        entry_path: p.patch.as_ref().map(|x| x.entry_path.clone()),
        start: p.patch.as_ref().map(|x| x.start),
        end: p.patch.as_ref().map(|x| x.end),
        prior_sha256: p.patch.as_ref().map(|x| x.prior.sha256.clone()),
        replacement_sha256: p.patch.as_ref().map(|x| x.replacement.sha256.clone()),
    }
}

/// The files an apply or restore holds: the image and every staged file, under their
/// reservation identity (canonical directory + leaf).
fn held_files(m: &StagingManifest, root: &Path) -> Result<BTreeMap<PathBuf, PathBuf>, ApiError> {
    let mut files = BTreeMap::new();
    let key = reservation_key(&m.image).map_err(ApiError::DiscImage)?;
    files.insert(key, m.image.clone());
    for f in &m.files {
        let p = root.join(&f.staged);
        let key = reservation_key(&p).map_err(ApiError::DiscImage)?;
        files.insert(key, p);
    }
    Ok(files)
}

// ---------------------------------------------------------------- registry surface

impl Registry {
    /// Test seam: replace the live process listing used by the running-emulator exclusion.
    #[doc(hidden)]
    pub fn set_process_probe_for_tests(
        &self,
        probe: impl Fn() -> Result<Vec<String>, String> + Send + Sync + 'static,
    ) {
        self.iso.lock().unwrap().probe = Some(std::sync::Arc::new(probe));
    }

    /// The probe, cloned out so the state lock is not held while it runs.
    fn process_probe(&self) -> Option<ProcessProbe> {
        self.iso.lock().unwrap().probe.clone()
    }

    /// Test seam: inject a fault into the next apply (taken once).
    #[doc(hidden)]
    pub fn set_iso_fault_for_tests(&self, fault: IsoFault) {
        self.iso.lock().unwrap().fault = Some(fault);
    }

    /// Read-only: where the database and the dictionary live inside the image, and what they are.
    pub fn iso_inspect(&self, image: &str) -> Result<IsoInspectDto, ApiError> {
        let p = Path::new(image);
        let found = xiso_source::locate_databases(p).map_err(ApiError::Domain)?;
        let mut out = IsoInspectDto {
            image: image.to_string(),
            layout: found.layout.to_string(),
            members: found.members,
            nhlng: found.nhlng.as_ref().map(located_dto),
            nhlng_sha256: None,
            nhlng_len: None,
            nhlng_tables: None,
            dictionary: found.dictionary.as_ref().map(located_dto),
            dictionary_sha256: None,
            dictionary_len: None,
            dictionary_rows: None,
            probed: found
                .probed
                .iter()
                .map(|a| IsoProbedArchiveDto {
                    path: a.path.clone(),
                    size: a.size,
                    verdict: a.verdict.clone(),
                })
                .collect(),
        };
        if let Some(l) = &found.nhlng {
            let bytes = xiso_source::read_located(p, l).map_err(ApiError::Domain)?;
            let id = studio_domain::tdb_txn::FileIdentity::of_bytes(&bytes);
            out.nhlng_len = Some(id.len);
            out.nhlng_sha256 = Some(id.sha256);
            out.nhlng_tables = tdb_core::TdbFile::parse(&bytes).ok().map(|db| db.tables.len());
        }
        if let Some(l) = &found.dictionary {
            let bytes = xiso_source::read_located(p, l).map_err(ApiError::Domain)?;
            let id = studio_domain::tdb_txn::FileIdentity::of_bytes(&bytes);
            out.dictionary_len = Some(id.len);
            out.dictionary_sha256 = Some(id.sha256);
            out.dictionary_rows =
                studio_domain::leagues::AttachedDictionary::from_bytes(image, bytes)
                    .ok()
                    .map(|d| d.dictionary.len());
        }
        Ok(out)
    }

    /// Stage the image's database and dictionary under `staging_root` (or reuse an existing,
    /// validated staging of the same image) and describe the folder. Binds nothing.
    pub fn iso_stage(
        &self,
        image: &str,
        game_id: &str,
        platform: &str,
        meta_xml: &str,
        staging_root: &str,
    ) -> Result<IsoStagedDto, ApiError> {
        let platform_token = parse_platform(platform)?.as_str();
        let s = iso_staging::stage(
            Path::new(image),
            game_id,
            platform_token,
            Path::new(meta_xml),
            Path::new(staging_root),
        )
        .map_err(ApiError::Domain)?;
        Ok(staged_dto(&s, None))
    }

    /// Stage (or reuse) and open the staged database as a single-file workspace with the staged
    /// dictionary attached, and bind the project to the staging folder and its image. The
    /// workspace's in-place save writes the staging folder only.
    pub fn iso_open(
        &self,
        image: &str,
        game_id: &str,
        platform: &str,
        meta_xml: &str,
        staging_root: &str,
        defs_root: &str,
    ) -> Result<(IsoStagedDto, ProjectSummaryDto), ApiError> {
        let platform_parsed = parse_platform(platform)?;
        let s = iso_staging::stage(
            Path::new(image),
            game_id,
            platform_parsed.as_str(),
            Path::new(meta_xml),
            Path::new(staging_root),
        )
        .map_err(ApiError::Domain)?;
        // The image is a held file while an apply/restore runs: opening a staging of it then
        // would be an alias that bypassed the reservation.
        {
            let files = held_files(&s.manifest, &s.root)?;
            let guard = self.inner.lock().unwrap();
            let keys: Vec<PathBuf> = files.keys().cloned().collect();
            if let Some(r) = guard.reservation_covering(&keys) {
                return Err(ApiError::DiscImage(format!(
                    "{} is held by {}; wait for it to finish before opening this image",
                    s.manifest.image.display(),
                    r.holder()
                )));
            }
        }
        let db = s.db_path.display().to_string();
        let meta = s.meta_xml.display().to_string();
        let summary =
            self.open_workspace_from_paths(Some(game_id), platform_parsed, &db, None, &meta, defs_root)?;
        if let Some(d) = &s.dictionary_path {
            let d = d.display().to_string();
            // Best effort like the onboarding path: a damaged staged dictionary leaves labels as
            // keys rather than blocking the open; the staging step already verified it.
            let _ = self.attach_dictionary(summary.project_id, &d);
        }
        let summary = self.summary(summary.project_id)?;
        self.iso.lock().unwrap().bindings.insert(
            summary.project_id,
            IsoBinding {
                root: s.root.clone(),
                manifest: s.manifest.clone(),
            },
        );
        Ok((staged_dto(&s, Some(summary.project_id)), summary))
    }

    fn binding(&self, project_id: u32) -> Result<(PathBuf, StagingManifest), ApiError> {
        let state = self.iso.lock().unwrap();
        state
            .bindings
            .get(&project_id)
            .map(|b| (b.root.clone(), b.manifest.clone()))
            .ok_or_else(|| {
                ApiError::DiscImage(format!(
                    "project {project_id} was not opened from a disc image (or is closed); the image route only serves projects it opened"
                ))
            })
    }

    /// The manifest as it is on disk now, required to still be the one the project was bound to.
    fn current_manifest(&self, root: &Path, bound: &StagingManifest) -> Result<StagingManifest, ApiError> {
        let m = iso_staging::read_manifest(root).map_err(ApiError::Domain)?;
        if m != *bound {
            return Err(ApiError::DiscImage(format!(
                "the staging manifest {} changed since the project was opened; close and reopen the image",
                root.join(iso_staging::MANIFEST).display()
            )));
        }
        Ok(m)
    }

    /// Where the route stands for a project: its binding and whether an operation holds it.
    pub fn iso_status(&self, project_id: u32) -> Result<IsoStatusDto, ApiError> {
        let (root, m) = self.binding(project_id)?;
        let in_flight = {
            let guard = self.inner.lock().unwrap();
            guard
                .reservation_for_project(project_id)
                .map(|r| r.holder().to_string())
        };
        let reviews = {
            let state = self.iso.lock().unwrap();
            state.reviews.values().filter(|r| r.project_id == project_id).count()
        };
        Ok(IsoStatusDto {
            project_id,
            root: root.display().to_string(),
            image: m.image.display().to_string(),
            in_flight,
            pending_reviews: reviews,
            cancellable: false,
            note: "a range write completes and verifies or is put back from its record; it cannot be cancelled once started. While it runs, saves, reloads, closes and other image operations on these files are refused with the holder named".to_string(),
        })
    }

    /// Review: the apply plan for the project's staging folder, verified against the image now
    /// and held by the server under a review id. Nothing written. A newer review for the same
    /// project supersedes the older one.
    pub fn iso_plan(&self, project_id: u32) -> Result<IsoPlanDto, ApiError> {
        let (root, bound) = self.binding(project_id)?;
        let (revision, dirty) = self.with_workspace(project_id, |ws| {
            Ok((ws.revision().to_string(), ws.is_dirty()))
        })?;
        let m = self.current_manifest(&root, &bound)?;
        let (_, planned) = iso_staging::plan_apply(&root).map_err(ApiError::Domain)?;
        let files: Vec<IsoPlannedFileDto> = planned.iter().map(planned_file_dto).collect();
        let writes = files.iter().filter(|f| f.changed).count();
        let review_id = {
            let mut state = self.iso.lock().unwrap();
            state.reviews.retain(|_, r| r.project_id != project_id);
            state.next_review += 1;
            let id = state.next_review;
            state.reviews.insert(
                id,
                IsoReview {
                    project_id,
                    root: root.clone(),
                    revision: revision.clone(),
                    dirty,
                    manifest: m.clone(),
                    planned,
                },
            );
            id
        };
        Ok(IsoPlanDto {
            review_id,
            project_id,
            workspace_revision: revision,
            dirty,
            image: m.image.display().to_string(),
            root: root.display().to_string(),
            files,
            writes,
        })
    }

    /// Reserve the image and the staged files (and every open workspace on them) for `holder`.
    fn reserve_image(
        &self,
        project_id: u32,
        root: &Path,
        m: &StagingManifest,
        holder: String,
    ) -> Result<ReservationGuard<'_>, ApiError> {
        let files = held_files(m, root)?;
        let image_dir = m
            .image
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let mut guard = self.inner.lock().unwrap();
        if !guard.sessions.contains_key(&project_id) {
            return Err(ApiError::UnknownProject(project_id));
        }
        let (id, _) = guard
            .reserve(holder, &image_dir, files, false, BTreeSet::from([project_id]))
            .map_err(|e| ApiError::DiscImage(e.to_string()))?;
        Ok(ReservationGuard { registry: self, id })
    }

    /// Apply the reviewed plan `review_id` to the image. **Writes to that image.** Every refusal
    /// happens before any byte changes; every failure after that is an outcome with records.
    pub fn iso_apply(&self, project_id: u32, review_id: u64) -> Result<IsoApplyDto, ApiError> {
        let (root, bound) = self.binding(project_id)?;
        // The review is consumed whatever happens next: a refused apply needs a fresh review.
        let review = {
            let mut state = self.iso.lock().unwrap();
            state.reviews.remove(&review_id).ok_or_else(|| {
                ApiError::DiscImage(format!(
                    "review {review_id} is not current (it was applied, superseded by a newer plan, or never existed); review the plan again"
                ))
            })?
        };
        if review.project_id != project_id || review.root != root {
            return Err(ApiError::DiscImage(format!(
                "review {review_id} belongs to project {} ({}), not project {project_id}; review this project's plan",
                review.project_id,
                review.root.display()
            )));
        }
        if review.dirty {
            return Err(ApiError::DiscImage(
                "the plan was reviewed while the project had unsaved edits; save, then review again".into(),
            ));
        }
        // The workspace as it is now, checked and then held under the same lock.
        self.with_workspace(project_id, |ws| {
            if ws.is_dirty() {
                return Err(ApiError::DiscImage(
                    "this project has unsaved edits; save first — the image receives the staging files as they are on disk".into(),
                ));
            }
            let now = ws.revision().to_string();
            if now != review.revision {
                return Err(ApiError::DiscImage(format!(
                    "the project was edited after the review (revision {} is now {now}); review the plan again",
                    review.revision
                )));
            }
            Ok(())
        })?;
        let holder = format!("disc-image write of review {review_id} (project {project_id})");
        let guard = self.reserve_image(project_id, &root, &bound, holder)?;
        let out = self.apply_reserved(project_id, &root, &bound, &review);
        drop(guard);
        out
    }

    /// The apply with the reservation held: emulator exclusion, the review's bindings re-checked
    /// against disk, then exactly the reviewed patches.
    fn apply_reserved(
        &self,
        project_id: u32,
        root: &Path,
        bound: &StagingManifest,
        review: &IsoReview,
    ) -> Result<IsoApplyDto, ApiError> {
        refuse_if_emulator_running(self.process_probe(), &bound.platform)?;
        let m = self.current_manifest(root, bound)?;
        if m != review.manifest {
            return Err(ApiError::DiscImage(
                "the staging manifest changed since the review; review the plan again".into(),
            ));
        }
        for p in &review.planned {
            let path = root.join(&p.staged);
            let now = studio_domain::tdb_txn::FileIdentity::of_file(&path).map_err(|e| ApiError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
            if now != p.staged_identity {
                return Err(ApiError::DiscImage(format!(
                    "{} is {now} on disk but the review described {}; the staged bytes changed after the review — review the plan again",
                    p.staged, p.staged_identity
                )));
            }
        }
        // Re-verify the workspace revision under the reservation: nothing else can edit it now,
        // but the check between "reviewed" and "held" was not atomic.
        let rev = {
            let guard = self.inner.lock().unwrap();
            guard
                .sessions
                .get(&project_id)
                .map(|ws| ws.revision().to_string())
                .ok_or(ApiError::UnknownProject(project_id))?
        };
        if rev != review.revision {
            return Err(ApiError::DiscImage(
                "the project was edited after the review; review the plan again".into(),
            ));
        }
        let records_dir = iso_staging::new_records_dir(root).map_err(ApiError::Domain)?;
        let mut hooks: Box<dyn iso_patch::Hooks> = match self.iso.lock().unwrap().fault.take() {
            Some(f) => Box::new(f),
            None => Box::new(iso_patch::NoHooks),
        };
        let outcome = match iso_staging::apply_planned(&review.planned, &records_dir, hooks.as_mut()) {
            Ok(o) => o,
            Err(e) => {
                let _ = std::fs::remove_dir(&records_dir);
                return Err(ApiError::Domain(e));
            }
        };
        self.apply_outcome_dto(outcome)
    }

    fn apply_outcome_dto(&self, outcome: ApplyOutcome) -> Result<IsoApplyDto, ApiError> {
        let message = outcome.describe();
        Ok(match outcome {
            ApplyOutcome::Committed(applied) => {
                let mut records = Vec::new();
                for r in &applied.records {
                    let on_image = iso_patch::status(r).map_err(ApiError::Domain)?;
                    records.push(record_dto(&applied.records_dir, r, on_image));
                }
                IsoApplyDto {
                    outcome: "committed".into(),
                    message,
                    records_dir: applied.records_dir.display().to_string(),
                    records,
                    attempted: Vec::new(),
                    unchanged: applied.unchanged,
                    failed: None,
                    cause: None,
                }
            }
            ApplyOutcome::RolledBack {
                records_dir,
                failed,
                cause,
                attempted,
            } => IsoApplyDto {
                outcome: "rolled-back".into(),
                message,
                records_dir: records_dir.display().to_string(),
                records: attempted.iter().map(|a| record_dto(&records_dir, &a.record, a.state.as_str())).collect(),
                attempted: attempted.iter().map(|a| attempted_dto(&records_dir, a)).collect(),
                unchanged: Vec::new(),
                failed: Some(failed),
                cause: Some(cause),
            },
            ApplyOutcome::Split {
                records_dir,
                failed,
                cause,
                attempted,
            } => IsoApplyDto {
                outcome: "split".into(),
                message,
                records_dir: records_dir.display().to_string(),
                records: attempted.iter().map(|a| record_dto(&records_dir, &a.record, a.state.as_str())).collect(),
                attempted: attempted.iter().map(|a| attempted_dto(&records_dir, a)).collect(),
                unchanged: Vec::new(),
                failed: Some(failed),
                cause: Some(cause),
            },
        })
    }

    /// Restore one apply's records (newest first), verified and idempotent. `records_dir` must
    /// be an apply directory of this project's staging, naming this image and its staged ranges.
    pub fn iso_restore(&self, project_id: u32, records_dir: &str) -> Result<IsoRestoreDto, ApiError> {
        let (root, bound) = self.binding(project_id)?;
        let dir = Path::new(records_dir);
        self.current_manifest(&root, &bound)?;
        iso_staging::validate_records_dir(&root, dir).map_err(ApiError::Domain)?;
        let holder = format!(
            "disc-image restore of {} (project {project_id})",
            dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        );
        let guard = self.reserve_image(project_id, &root, &bound, holder)?;
        let out = (|| {
            refuse_if_emulator_running(self.process_probe(), &bound.platform)?;
            let outcomes = iso_staging::restore(dir).map_err(ApiError::Domain)?;
            let items: Vec<IsoRestoreItemDto> = outcomes
                .iter()
                .map(|(entry, o)| IsoRestoreItemDto {
                    entry_path: entry.clone(),
                    outcome: match o {
                        Ok(iso_patch::RestoreOutcome::Restored) => "restored".to_string(),
                        Ok(iso_patch::RestoreOutcome::RestoredPartial) => "restored-partial".to_string(),
                        Ok(iso_patch::RestoreOutcome::AlreadyPrior) => "already-prior".to_string(),
                        Err(e) => format!("refused: {e}"),
                    },
                })
                .collect();
            let all_prior = outcomes.iter().all(|(_, o)| o.is_ok());
            Ok(IsoRestoreDto {
                records_dir: records_dir.to_string(),
                items,
                all_prior,
            })
        })();
        drop(guard);
        out
    }

    /// Every record under the project's staging folder with what the image holds at each range
    /// right now (`prior` / `replacement` / `partial` / `unknown`), failed applies included.
    pub fn iso_records(&self, project_id: u32) -> Result<Vec<IsoRecordDto>, ApiError> {
        let (root, _) = self.binding(project_id)?;
        let out = iso_staging::records(&root).map_err(ApiError::Domain)?;
        Ok(out
            .iter()
            .map(|(dir, r, s)| record_dto(dir, r, s))
            .collect())
    }
}
