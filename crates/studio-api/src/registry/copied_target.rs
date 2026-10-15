//! S2-C — the copied-target transaction surface of the [`Registry`]: review/prepare, apply,
//! status, recover, restore, discard, cancel and reload, over
//! [`studio_domain::copied_target::stage_workspace`] and the shared `tdb-txn` engine.
//!
//! ## Authority
//!
//! Every input that decides what is written is **server-derived**: the frontend names a project,
//! a copied-target root and later hands back the `handle` this module minted. Destinations,
//! identities, validation, qualification, dependencies and bound inputs come from the workspace's
//! sessions and the content-header planner; the engine re-validates the persisted change set and
//! journal under its own lock at apply. There is no live mode and no force option:
//! `Mode::CopiedTarget` only, against a root that carries the engine's marker.
//!
//! ## Binding and lifecycle
//!
//! A prepared handle is bound to `(project_id, canonical root, workspace content revision)`. Apply
//! refuses a handle for another project, a handle already used or discarded, a workspace whose
//! revision changed after review (any edit, undo or redo), a changed destination, bound input
//! (an existing or template header) or staged output, and a missing parent directory — every
//! refusal aborts the engine's prepared set so the root is free again and keeps the session's
//! edits. On commit the initiating sessions **rebind** to the installed files (re-read and
//! verified); every other open workspace on the same files (an *alias*) is **refreshed** — clean
//! sessions reload, dirty ones keep their edits and are marked stale. On any other outcome sessions
//! are untouched. Restore and recovery refresh every open workspace under the root the same way. A
//! refresh that fails (file missing, unreadable, malformed, alignment not derivable) is reported
//! as `refresh-failed`, changes nothing in that workspace, keeps its edits, and marks it unusable
//! for copied-target saves until a reload succeeds.
//!
//! ## Reservations (S2-C-R1)
//!
//! An apply, recovery or restore holds a **reservation** in the registry's `inner` state for its
//! duration: the canonical files it will touch (apply: every destination and bound input;
//! recovery/restore: the whole root), and every open workspace whose files those are — the
//! initiating project *and* every alias. The binding checks (project, revision, baseline) and the
//! reservation happen in one `inner` critical section, so no edit can slip between the revision
//! check and the lock. While a reservation exists: every registry access to a reserved project is
//! refused (edits, legacy `save_workspace`/`save_project`, reload, close), opening a workspace on
//! reserved files is refused, and a "Save As" into a reserved file is refused. Reservations may
//! not overlap (files, projects or nested roots); unrelated projects are unaffected. A guard
//! releases the reservation on every exit path — success, refusal, cancellation, error or panic.
//!
//! ## Lock scope (LEAGUE-RESP-1)
//!
//! The registry's `inner` mutex is held only for work that needs the live workspace: packaging
//! the sessions at review, the binding checks and the reservation at apply, and the rebind /
//! refresh after the engine. Everything that hashes, copies or stages whole files — the apply's
//! identity verification of every destination, staged output and bound input (~1.5 GB on a real
//! PS4 archive), the archive staging and the engine's `prepare` at review — runs with **only a
//! reservation** held, never `inner`. A **review** therefore also holds a reservation ("review of
//! project N (handle H)") on the project, its aliases and its files from the moment the sessions
//! are packaged until the handle is registered: the packaged bytes stay the workspace's current
//! revision by exclusion, not by luck, and a competing edit, save, close or open is refused at
//! once with the holder named instead of waiting on the mutex. The revision is re-checked under
//! `inner` once more before a handle is registered and before the engine runs — a defensive
//! check, since the reservation already excludes every registry writer.
//!
//! The registry's `inner` mutex itself is never held across engine work or whole-file hashing.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use studio_domain::copied_target::{
    stage_workspace, verify_plan_inputs_unless, verify_plan_layout, ArtifactRole, CopiedTargetPlan,
    PlanVerification,
};
use studio_domain::tdb_txn::{self, Event, Expected, FileIdentity, Hooks, State, TxnError};
use studio_domain::{BaselineAction, BaselineRefresh, ProjectRole, Workspace};

use super::{Inner, Registry};
use crate::dto::{
    BoundInputDto, CopiedTargetApplyDto, CopiedTargetArtifactDto, CopiedTargetRecoveryDto,
    CopiedTargetRestoreDto, CopiedTargetReviewDto, CopiedTargetStatusDto,
    CopiedTargetTransactionDto, DestinationOutcomeDto, FileIdentityDto, PendingHandleDto,
    ProjectBaselineDto, QualificationItemDto, ReservationDto, StagingLeftoverDto,
    SupersededDestinationDto,
};
use crate::error::ApiError;

// ---------------------------------------------------------------- reservation identity (S2-C-R2)

/// The identity a file is reserved and checked under: the **canonical parent directory** (every
/// directory symlink/junction resolved, Windows on-disk spelling) joined with the **leaf**, folded
/// to lower case when that directory's file system matches names case-insensitively (probed, not
/// assumed from the platform). An existing final component is resolved first, so a symlink or
/// junction in the last position reserves and checks its *target*, and `NHLNG.DB` and `nhlng.db`
/// yield one identity on NTFS/APFS while staying distinct on a case-sensitive directory.
///
/// Every producer of reservation keys — an apply's destinations and bound inputs, a workspace's
/// files at open/refresh time, a "Save As" target — goes through this one function, so a guard is
/// never bypassed by an equivalent spelling. A path whose identity cannot be established (no
/// parent, a parent that does not exist, a `.`/`..`/empty leaf) is an error: callers that would
/// write refuse rather than guess.
pub(super) fn reservation_key(path: &Path) -> Result<PathBuf, String> {
    let leaf = match path.file_name() {
        Some(n) if n != "." && n != ".." => n.to_os_string(),
        _ => {
            return Err(format!(
                "{} does not name a file (no final path component)",
                path.display()
            ))
        }
    };
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let parent = std::fs::canonicalize(&parent).map_err(|e| {
        format!(
            "the directory of {} could not be resolved ({e}); it must exist",
            path.display()
        )
    })?;
    let full = parent.join(&leaf);
    // An existing final component (a regular file, or a symlink/junction to one) is resolved to
    // the file it actually is; the key is built from *that* parent and leaf.
    let (parent, leaf) = match std::fs::symlink_metadata(&full) {
        Ok(_) => {
            let target = std::fs::canonicalize(&full).map_err(|e| {
                format!(
                    "{} exists but could not be resolved ({e}); refusing to treat it as a plain file",
                    full.display()
                )
            })?;
            match (target.parent(), target.file_name()) {
                (Some(p), Some(n)) => (p.to_path_buf(), n.to_os_string()),
                _ => {
                    return Err(format!(
                        "{} resolves to {}, which is not a file inside a directory",
                        full.display(),
                        target.display()
                    ))
                }
            }
        }
        Err(_) => (parent, leaf),
    };
    let leaf = if directory_folds_case(&parent) {
        std::ffi::OsString::from(leaf.to_string_lossy().to_lowercase())
    } else {
        leaf
    };
    Ok(parent.join(leaf))
}

/// Whether name lookups inside `dir` ignore ASCII letter case, decided by the file system itself:
/// an existing entry of `dir` is looked up under a case-swapped spelling and must resolve to the
/// same file. Falls back to the platform default (Windows and macOS fold; others do not) only when
/// `dir` has no entry with a letter to probe with.
fn directory_folds_case(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let spelled = name.to_string_lossy();
            let swapped: String = spelled
                .chars()
                .map(|c| {
                    if c.is_ascii_lowercase() {
                        c.to_ascii_uppercase()
                    } else if c.is_ascii_uppercase() {
                        c.to_ascii_lowercase()
                    } else {
                        c
                    }
                })
                .collect();
            if swapped == *spelled {
                continue;
            }
            let Ok(as_spelled) = std::fs::canonicalize(dir.join(&name)) else {
                continue; // a broken link says nothing about name matching
            };
            return std::fs::canonicalize(dir.join(&swapped)).is_ok_and(|b| b == as_spelled);
        }
    }
    cfg!(any(windows, target_os = "macos"))
}

// ---------------------------------------------------------------- reservations

/// Files and projects held by one apply / recovery / restore (see the module docs).
pub(super) struct Reservation {
    id: u64,
    /// Human-readable owner: "apply of review handle 3 (project 0)", "recovery of <root>", …
    holder: String,
    /// Canonical copied-target root.
    root: PathBuf,
    /// Held files: [`reservation_key`] → the path as planned (for display).
    files: BTreeMap<PathBuf, PathBuf>,
    /// The whole root is claimed (recovery / restore).
    whole_root: bool,
    /// Every open workspace whose files are covered (initiator and aliases).
    projects: BTreeSet<u32>,
}

impl Reservation {
    /// `key` is a [`reservation_key`].
    fn covers(&self, key: &Path) -> bool {
        (self.whole_root && key.starts_with(&self.root)) || self.files.contains_key(key)
    }

    fn overlaps(&self, other: &Reservation) -> bool {
        self.projects.intersection(&other.projects).next().is_some()
            || other.files.keys().any(|f| self.covers(f))
            || self.files.keys().any(|f| other.covers(f))
            || (self.whole_root
                && other.whole_root
                && (self.root.starts_with(&other.root) || other.root.starts_with(&self.root)))
    }

    pub(super) fn holder(&self) -> &str {
        &self.holder
    }
}

/// A workspace's files as reservation keys. A file whose identity cannot be established right now
/// (it vanished together with its directory) keeps its path as given, which can only make the
/// workspace *more* likely to count as held.
pub(super) fn workspace_files(ws: &Workspace) -> Vec<PathBuf> {
    let mut v = vec![key_or_self(ws.primary_path())];
    if let Some(r) = ws.roster_path() {
        v.push(key_or_self(r));
    }
    v
}

fn key_or_self(p: &str) -> PathBuf {
    reservation_key(Path::new(p)).unwrap_or_else(|_| PathBuf::from(p))
}

impl Inner {
    /// The reservation holding `project_id`, if any.
    pub(super) fn reservation_for_project(&self, project_id: u32) -> Option<&Reservation> {
        self.reservations
            .iter()
            .find(|r| r.projects.contains(&project_id))
    }

    /// The reservation covering any of `paths`, if any.
    pub(super) fn reservation_covering(&self, paths: &[PathBuf]) -> Option<&Reservation> {
        self.reservations
            .iter()
            .find(|r| paths.iter().any(|p| r.covers(p)))
    }

    /// Every open workspace with a file among `paths`, or (when `whole_root`) under `root`.
    fn projects_on(
        &self,
        root: &Path,
        paths: &BTreeMap<PathBuf, PathBuf>,
        whole_root: bool,
    ) -> BTreeSet<u32> {
        self.sessions
            .iter()
            .filter(|(_, ws)| {
                workspace_files(ws)
                    .iter()
                    .any(|f| paths.contains_key(f) || (whole_root && f.starts_with(root)))
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Reserve `files` (or the whole `root`) and every open workspace on them. Refuses when an
    /// existing reservation overlaps. Must be called with the caller already holding `inner`.
    pub(super) fn reserve(
        &mut self,
        holder: String,
        root: &Path,
        files: BTreeMap<PathBuf, PathBuf>,
        whole_root: bool,
        mut projects: BTreeSet<u32>,
    ) -> Result<(u64, BTreeSet<u32>), ApiError> {
        projects.extend(self.projects_on(root, &files, whole_root));
        let candidate = Reservation {
            id: self.next_reservation,
            holder,
            root: root.to_path_buf(),
            files,
            whole_root,
            projects,
        };
        if let Some(existing) = self.reservations.iter().find(|r| r.overlaps(&candidate)) {
            return Err(ApiError::CopiedTarget(format!(
                "{} cannot start: it overlaps {} (projects {:?}); wait for it to finish or cancel it",
                candidate.holder,
                existing.holder,
                existing.projects.iter().collect::<Vec<_>>()
            )));
        }
        self.next_reservation += 1;
        let id = candidate.id;
        let projects = candidate.projects.clone();
        self.reservations.push(candidate);
        Ok((id, projects))
    }

    /// Add `files` to the reservation `id` (a review whose plan grew an artifact after the
    /// sessions were packaged). Refused — with the reservation left as it was — when another
    /// reservation already covers one of them.
    fn extend_reservation(
        &mut self,
        id: u64,
        files: BTreeMap<PathBuf, PathBuf>,
    ) -> Result<(), ApiError> {
        let Some(at) = self.reservations.iter().position(|r| r.id == id) else {
            return Err(ApiError::CopiedTarget(format!(
                "reservation {id} is gone; it cannot be extended"
            )));
        };
        for (key, shown) in &files {
            if let Some(other) = self
                .reservations
                .iter()
                .find(|r| r.id != id && r.covers(key))
            {
                return Err(ApiError::CopiedTarget(format!(
                    "{} is held by {} (projects {:?}); wait for it to finish or cancel it",
                    display_path(shown),
                    other.holder,
                    other.projects.iter().collect::<Vec<_>>()
                )));
            }
        }
        let root = self.reservations[at].root.clone();
        let extra_projects = self.projects_on(&root, &files, false);
        let r = &mut self.reservations[at];
        r.files.extend(files);
        r.projects.extend(extra_projects);
        Ok(())
    }

    fn release(&mut self, id: u64) {
        self.reservations.retain(|r| r.id != id);
    }
}

/// Releases its reservation when dropped — on every exit path of the operation that took it.
/// Never drop it while holding `inner` (it locks `inner` itself).
pub(super) struct ReservationGuard<'a> {
    pub(super) registry: &'a Registry,
    pub(super) id: u64,
}

impl Drop for ReservationGuard<'_> {
    fn drop(&mut self) {
        self.registry.inner.lock().unwrap().release(self.id);
    }
}

/// The registry-level phases of an apply outside the engine, as reported to the observer seam of
/// [`Registry::copied_target_apply_observed`].
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyPhase {
    /// The binding checks passed and the reservation is held; `inner` is free. The identity
    /// verification (whole-file hashing) runs next.
    Reserved,
    /// The identity verification finished (or was skipped on cancellation); the engine runs next.
    Verified,
}

// ---------------------------------------------------------------- handles

/// A reviewed, prepared change set the registry owns until it is applied or discarded.
struct Prepared {
    project_id: u32,
    root: PathBuf,
    plan: CopiedTargetPlan,
    txn: tdb_txn::Transaction,
    staging_dir: PathBuf,
    cancel: Arc<AtomicBool>,
}

/// An apply the engine is running right now.
struct InFlight {
    project_id: u32,
    root: PathBuf,
    txn_id: String,
    cancel: Arc<AtomicBool>,
}

/// Why a staging directory is still on disk (S2-C-R3). The two kinds are never confused: only
/// the first may be deleted by a plain retry; the second is deleted only once the engine's
/// persisted journal shows the set no longer references the staged sources.
enum Obligation {
    /// The set is resolved; only the directory removal failed (a pinned file, say).
    RemovalFailed { error: String },
    /// The prepared set could not be aborted (or its journal cannot be read): the persisted set
    /// still names the staged files as its replacement sources, so the staging is **retained**
    /// together with the engine handle needed to retry the abort.
    Unresolved {
        txn: Box<tdb_txn::Transaction>,
        reason: String,
    },
}

impl Obligation {
    fn kind(&self) -> &'static str {
        match self {
            Obligation::RemovalFailed { .. } => "removal-failed",
            Obligation::Unresolved { .. } => "unresolved-transaction",
        }
    }
    fn reason(&self) -> &str {
        match self {
            Obligation::RemovalFailed { error } => error,
            Obligation::Unresolved { reason, .. } => reason,
        }
    }
    fn txn_id(&self) -> Option<&str> {
        match self {
            Obligation::RemovalFailed { .. } => None,
            Obligation::Unresolved { txn, .. } => Some(&txn.txn_id),
        }
    }
}

/// What the engine's persisted journal says about a set, read through the public status API:
/// `Ok(None)` — no journal for it any more; `Ok(Some(state))`; `Err` — the journal exists but
/// cannot be read (the root is blocked).
fn persisted_state(txn: &tdb_txn::Transaction) -> Result<Option<State>, String> {
    let summaries = tdb_txn::status(&txn.root).map_err(|e| e.to_string())?;
    match summaries.iter().find(|s| s.txn_id == txn.txn_id) {
        None => Ok(None),
        Some(s) => match (s.state, &s.problem) {
            (Some(state), _) => Ok(Some(state)),
            (None, problem) => Err(problem
                .clone()
                .unwrap_or_else(|| "the journal could not be read".into())),
        },
    }
}

#[derive(Default)]
pub(super) struct CopiedState {
    next_handle: u32,
    prepared: HashMap<u32, Prepared>,
    in_flight: HashMap<u32, InFlight>,
    /// Staging directories this registry created but has not removed yet, by path: tracked
    /// cleanup obligations, settled again at every later teardown/recovery and reported in
    /// status; never silently forgotten, and never deleted while the engine still references
    /// them ([`Obligation`]).
    leftovers: BTreeMap<PathBuf, Obligation>,
}

impl CopiedState {
    /// Remove `dir` (a staging directory this registry created **whose set is resolved**). A
    /// failure that leaves the directory behind becomes a `RemovalFailed` obligation; success
    /// clears any earlier obligation for it.
    fn drop_staging(&mut self, dir: &Path) {
        match std::fs::remove_dir_all(dir) {
            Ok(()) => {
                self.leftovers.remove(dir);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.leftovers.remove(dir);
            }
            Err(e) => {
                self.leftovers.insert(
                    dir.to_path_buf(),
                    Obligation::RemovalFailed {
                        error: e.to_string(),
                    },
                );
            }
        }
    }

    /// Settle one prepared set's staging against the engine's contract: a set the journal shows
    /// as `Prepared`/`Staged` is aborted (the only states `abort` accepts); a set whose journal is
    /// gone or resolved (`Aborted`, `Committed`, `RolledBack`, `Restored` — the engine holds its
    /// own copies of everything it still needs) releases the staging; any other state (an apply
    /// or restore in progress, `Blocked`), an unreadable journal or a failed abort **retains** the
    /// staging with the handle, and the reason is returned. `StaleHandle`/`WrongState` from
    /// `abort` are never taken as proof of anything: the persisted state decides.
    fn settle(&mut self, staging: PathBuf, txn: tdb_txn::Transaction) -> Result<(), String> {
        let outcome: Result<(), String> = match persisted_state(&txn) {
            Ok(None) => Ok(()),
            Ok(Some(State::Prepared)) | Ok(Some(State::Staged)) => {
                match tdb_txn::abort(&txn, &mut tdb_txn::NoHooks) {
                    Ok(_) => Ok(()),
                    // The set changed under us between the status read and the abort: re-read.
                    Err(TxnError::StaleHandle { .. }) | Err(TxnError::WrongState { .. }) => {
                        match persisted_state(&txn) {
                            Ok(None)
                            | Ok(Some(State::Aborted))
                            | Ok(Some(State::Committed))
                            | Ok(Some(State::RolledBack))
                            | Ok(Some(State::Restored)) => Ok(()),
                            Ok(Some(other)) => Err(format!(
                                "its persisted state is {other:?}; run recovery on the root"
                            )),
                            Err(e) => Err(format!("its journal cannot be read ({e})")),
                        }
                    }
                    Err(e) => Err(format!(
                        "it could not be aborted ({e}); free the journal and discard or tear down again, or run recovery on the root"
                    )),
                }
            }
            Ok(Some(State::Aborted))
            | Ok(Some(State::Committed))
            | Ok(Some(State::RolledBack))
            | Ok(Some(State::Restored)) => Ok(()),
            Ok(Some(other)) => Err(format!(
                "its persisted state is {other:?}; run recovery on the root"
            )),
            Err(e) => Err(format!("its journal cannot be read ({e})")),
        };
        match outcome {
            Ok(()) => {
                self.drop_staging(&staging);
                Ok(())
            }
            Err(why) => {
                let reason = format!(
                    "retained: prepared set {} in {} still references this staging — {why}",
                    txn.txn_id,
                    display_path(&txn.root)
                );
                self.leftovers.insert(
                    staging,
                    Obligation::Unresolved {
                        txn: Box::new(txn),
                        reason: reason.clone(),
                    },
                );
                Err(reason)
            }
        }
    }

    /// Settle every tracked obligation once more: removal failures are retried; unresolved sets
    /// are re-checked against their journal (and aborted if that is now possible).
    fn retry_leftovers(&mut self) {
        let pending: Vec<(PathBuf, Obligation)> =
            std::mem::take(&mut self.leftovers).into_iter().collect();
        for (dir, obligation) in pending {
            match obligation {
                Obligation::RemovalFailed { .. } => self.drop_staging(&dir),
                Obligation::Unresolved { txn, .. } => {
                    let _ = self.settle(dir, *txn);
                }
            }
        }
    }

    /// The obligations still open, for status and teardown reports.
    fn leftover_dtos(&self) -> Vec<StagingLeftoverDto> {
        self.leftovers
            .iter()
            .map(|(p, o)| StagingLeftoverDto {
                path: display_path(p),
                kind: o.kind().into(),
                txn_id: o.txn_id().map(str::to_owned),
                error: o.reason().to_owned(),
            })
            .collect()
    }
}

/// Backstop when a registry is dropped normally with prepared handles still open (a test's
/// registry going out of scope, a shell that never called [`Registry::teardown_copied_target`]):
/// each prepared set is settled exactly as by teardown — aborted where the journal allows and its
/// staging removed; retained on disk, with its journal untouched, where it does not — and what is
/// retained is printed to stderr, the only reporting available here. Nothing is promised for a
/// forced termination: a killed process leaves its prepared set on disk for the engine's
/// recovery, which aborts it, and its staging directory under the OS temp directory, which is
/// then unreferenced scratch (the engine copies what it needs into its own recovery directory
/// before any destination changes).
impl Drop for CopiedState {
    fn drop(&mut self) {
        let prepared: Vec<Prepared> = self.prepared.drain().map(|(_, p)| p).collect();
        for p in prepared {
            let _ = self.settle(p.staging_dir, p.txn);
        }
        self.retry_leftovers();
        for l in self.leftover_dtos() {
            eprintln!(
                "copied-target staging left behind: {} ({}: {})",
                l.path, l.kind, l.error
            );
        }
    }
}

/// Progress, cancellation and (for tests) fault injection, composed for one apply.
struct ApplyHooks<'a> {
    cancel: Arc<AtomicBool>,
    log: Vec<String>,
    extra: Option<&'a mut dyn Hooks>,
}

impl Hooks for ApplyHooks<'_> {
    fn event(&mut self, e: &Event) -> std::io::Result<()> {
        self.log.push(format!("{e:?}"));
        if let Some(x) = self.extra.as_mut() {
            x.event(e)?;
        }
        Ok(())
    }
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst) || self.extra.as_ref().is_some_and(|x| x.cancelled())
    }
}

// ---------------------------------------------------------------- small helpers

fn identity_dto(id: &FileIdentity) -> FileIdentityDto {
    FileIdentityDto {
        len: id.len,
        sha256: id.sha256.clone(),
    }
}

/// A path for the user: the verbatim prefix (`\\?\`) Windows' `canonicalize` adds is stripped.
pub(super) fn display_path(p: &Path) -> String {
    let s = p.display().to_string();
    match s.strip_prefix(r"\\?\UNC\") {
        Some(rest) => format!(r"\\{rest}"),
        None => s.strip_prefix(r"\\?\").map(str::to_owned).unwrap_or(s),
    }
}

fn state_name(state: State) -> String {
    serde_json::to_value(state)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{state:?}").to_lowercase())
}

fn canonical_root(root: &str) -> Result<PathBuf, ApiError> {
    Ok(tdb_txn::copied_target_root(Path::new(root))?)
}

/// A fresh staging directory name: unique per process **and** per registry instance (tests run
/// several registries in one process; handles alone would collide).
fn staging_dir_for(handle: u32) -> PathBuf {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir()
        .join("nhl-modding-studio")
        .join("copied-target-staging")
        .join(format!("{}-{seq}-{handle}", std::process::id()))
}

fn action_name(a: BaselineAction) -> String {
    match a {
        BaselineAction::Unchanged => "unchanged",
        BaselineAction::Reloaded => "reloaded",
        BaselineAction::StaleKeptEdits => "stale-kept-edits",
        BaselineAction::NoBaseline => "no-baseline",
    }
    .into()
}

fn baseline_dto(project_id: u32, ws: &Workspace, refresh: &BaselineRefresh) -> ProjectBaselineDto {
    ProjectBaselineDto {
        project_id,
        primary: action_name(refresh.primary),
        roster: refresh.roster.map(action_name),
        dirty: ws.is_dirty(),
        stale_baseline: ws.stale_baseline().map(str::to_owned),
        refresh_error: None,
    }
}

/// Refresh one workspace after its files may have changed. A failure is **reported**, never
/// mapped to "unchanged": the workspace keeps its edits, the domain marked it stale with the
/// reason, and `refresh_error` carries the error.
fn refresh_workspace(project_id: u32, ws: &mut Workspace) -> ProjectBaselineDto {
    match ws.refresh_baselines() {
        Ok(r) => baseline_dto(project_id, ws, &r),
        Err(e) => ProjectBaselineDto {
            project_id,
            primary: "refresh-failed".into(),
            roster: ws.roster_attached().then(|| "refresh-failed".to_owned()),
            dirty: ws.is_dirty(),
            stale_baseline: ws.stale_baseline().map(str::to_owned),
            refresh_error: Some(e.to_string()),
        },
    }
}

fn artifact_dto(
    primary_role: ProjectRole,
    plan: &CopiedTargetPlan,
    key: &str,
) -> CopiedTargetArtifactDto {
    let a = plan.artifact(key).expect("key from the plan");
    let file_name = |p: &Path| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let (label, kind) = match &a.role {
        ArtifactRole::Primary => match primary_role {
            ProjectRole::Nhlng => (
                format!("Main database ({})", file_name(&a.destination_path)),
                "database",
            ),
            ProjectRole::Roster => (
                format!("Roster save ({})", file_name(&a.destination_path)),
                "roster-save",
            ),
            ProjectRole::Localization => (
                format!("Localization database ({})", file_name(&a.destination_path)),
                "database",
            ),
        },
        ArtifactRole::Roster => (
            format!("Roster save ({})", file_name(&a.destination_path)),
            "roster-save",
        ),
        ArtifactRole::Archive => {
            let name = file_name(&a.destination_path);
            if key.ends_with(".bh") {
                (format!("Archive index sidecar ({name}) - patched together with its archive"), "archive")
            } else if key.starts_with("archive-asset-") {
                (format!("Archive with replaced texture member(s) ({name})"), "archive")
            } else if key == "archive-csv" {
                (format!("Archive with the edited settings catalogue ({name})"), "archive")
            } else {
                (format!("Archive ({name})"), "archive")
            }
        }
        ArtifactRole::Dictionary => (
            format!("Localization dictionary ({})", file_name(&a.destination_path)),
            "dictionary",
        ),
        ArtifactRole::LooseAsset => (
            format!("Loose venue package ({}) - replaced together with its archive member", file_name(&a.destination_path)),
            "loose-asset",
        ),
        ArtifactRole::ContentHeader { for_key } => {
            let indexed = plan
                .artifact(for_key)
                .map(|x| file_name(&x.destination_path))
                .unwrap_or_default();
            (
                format!(
                    "Save index header for {indexed} ({})",
                    a.artifact.destination
                ),
                "content-header",
            )
        }
    };
    let q = &a.artifact.validation.qualification;
    CopiedTargetArtifactDto {
        key: key.to_owned(),
        label,
        kind: kind.into(),
        destination: a.artifact.destination.clone(),
        destination_path: display_path(&a.destination_path),
        prior: match &a.artifact.expected_prior {
            Expected::Absent => None,
            Expected::Present(p) => Some(identity_dto(p)),
        },
        output: identity_dto(&a.artifact.replacement_identity),
        dependencies: a.artifact.dependencies.clone(),
        producer: a.artifact.validation.producer.clone(),
        qualification_state: q.state.clone(),
        qualification_items: q
            .items
            .iter()
            .map(|i| QualificationItemDto {
                subject: i.subject.clone(),
                status: i.status.clone(),
                note: i.note.clone(),
            })
            .collect(),
    }
}

fn destinations_dto(
    out: &tdb_txn::Outcome,
    txn: &tdb_txn::Transaction,
) -> Vec<DestinationOutcomeDto> {
    out.destinations
        .iter()
        .map(|(key, id)| DestinationOutcomeDto {
            key: key.clone(),
            destination: txn
                .journal
                .artifact(key)
                .map(|a| a.destination.clone())
                .unwrap_or_default(),
            identity: id.as_ref().map(identity_dto),
        })
        .collect()
}

/// The files a plan holds, under their reservation identity ([`reservation_key`]): every
/// destination and bound input, plus `also` (files an operation reads outside the plan).
fn reservation_files(
    plan: &CopiedTargetPlan,
    also: &[PathBuf],
) -> Result<BTreeMap<PathBuf, PathBuf>, ApiError> {
    // D4/D8: the copied archives the pending text-/asset-member rewrites will be staged into.
    let text_archive: Vec<PathBuf> = plan
        .text_members
        .iter()
        .chain(plan.asset_members.iter())
        .flat_map(|t| {
            let mut v = vec![plan.root.join(&t.archive_relative)];
            if let Some(b) = &t.big4 {
                v.push(plan.root.join(&b.sidecar_relative));
            }
            v
        })
        .collect();
    plan.artifacts
        .iter()
        .map(|a| &a.destination_path)
        .chain(plan.bound_inputs.iter().map(|b| &b.path))
        .chain(also.iter())
        .chain(text_archive.iter())
        .map(|p| {
            reservation_key(p)
                .map(|k| (k, p.clone()))
                .map_err(|e| ApiError::CopiedTarget(format!("cannot reserve {e}")))
        })
        .collect()
}

impl Registry {
    // ---- status ---------------------------------------------------------------------------

    /// The copied-target root the project's primary file lies under: the nearest ancestor that
    /// carries the engine's marker (checked through the engine's own root check). A project whose
    /// files are not inside a marked root gets `None` with the reason — it is never treated as a
    /// disposable tree by assumption.
    pub fn copied_target_root_for(
        &self,
        id: u32,
    ) -> Result<crate::dto::CopiedTargetRootDto, ApiError> {
        let primary = self.with_workspace(id, |ws| Ok(ws.primary_path().to_string()))?;
        let mut cursor = Path::new(&primary).parent();
        while let Some(dir) = cursor {
            if let Ok(root) = tdb_txn::engine::copied_target_root(dir) {
                return Ok(crate::dto::CopiedTargetRootDto {
                    project_id: id,
                    root: Some(display_path(&root)),
                    note: None,
                });
            }
            cursor = dir.parent();
        }
        Ok(crate::dto::CopiedTargetRootDto {
            project_id: id,
            root: None,
            note: Some(format!(
                "{primary} is not inside a copied-target root (no ancestor carries {}); create a copied root, copy the game files into it and open them from there",
                tdb_txn::engine::COPIED_TARGET_MARKER
            )),
        })
    }

    /// Create a **new** copied-target root at `path` (refused if it already exists). Only roots
    /// created this way accept change sets; an installed game directory is never marked in place.
    pub fn copied_target_create_root(&self, path: &str) -> Result<String, ApiError> {
        let p = Path::new(path);
        if p.exists() {
            return Err(ApiError::CopiedTarget(format!(
                "{path} already exists; a copied-target root is only ever created as a new, empty directory"
            )));
        }
        tdb_txn::engine::create_copied_target_root(p).map_err(ApiError::Transaction)?;
        let root = canonical_root(path)?;
        Ok(display_path(&root))
    }

    /// Everything the engine knows about `root` (from its journals, never from handles), plus the
    /// registry's own pending handles, reservations and busy projects for it, and what to do next.
    pub fn copied_target_status(&self, root: &str) -> Result<CopiedTargetStatusDto, ApiError> {
        let root = canonical_root(root)?;
        self.status_of(&root)
    }

    fn status_of(&self, root: &Path) -> Result<CopiedTargetStatusDto, ApiError> {
        let summaries = tdb_txn::status(root)?;
        let journals = tdb_txn::engine::history(root)?;
        let mut transactions = Vec::new();
        let mut blocking: Vec<(u64, String)> = Vec::new();
        for s in &summaries {
            let j = journals.get(&s.txn_id);
            let state = match s.state {
                Some(st) => state_name(st),
                None => "unreadable".into(),
            };
            let blocks = s.problem.is_some() || s.state.is_some_and(|st| st.blocks_root());
            if blocks {
                blocking.push((s.sequence, s.txn_id.clone()));
            }
            transactions.push(CopiedTargetTransactionDto {
                txn_id: s.txn_id.clone(),
                sequence: s.sequence,
                state,
                problem: s.problem.clone(),
                superseded: s
                    .superseded
                    .iter()
                    .map(|(d, by)| SupersededDestinationDto {
                        destination: d.clone(),
                        by_txn_id: by.clone(),
                    })
                    .collect(),
                recovery_dir: display_path(&root.join(tdb_txn::journal::TXN_DIR).join(&s.txn_id)),
                applied_unqualified: j.map(|j| j.applied_unqualified).unwrap_or(false),
                destinations: j
                    .map(|j| j.artifacts.iter().map(|a| a.destination.clone()).collect())
                    .unwrap_or_default(),
                restorable: s.state == Some(State::Committed) && s.superseded.is_empty(),
            });
        }
        let copied = self.copied.lock().unwrap();
        let pending_handles: Vec<PendingHandleDto> = copied
            .prepared
            .iter()
            .filter(|(_, p)| p.root == root)
            .map(|(h, p)| PendingHandleDto {
                handle: *h,
                project_id: p.project_id,
                txn_id: p.txn.txn_id.clone(),
                in_progress: false,
            })
            .chain(
                copied
                    .in_flight
                    .iter()
                    .filter(|(_, p)| p.root == root)
                    .map(|(h, p)| PendingHandleDto {
                        handle: *h,
                        project_id: p.project_id,
                        txn_id: p.txn_id.clone(),
                        in_progress: true,
                    }),
            )
            .collect();
        let in_flight_here = copied.in_flight.values().any(|p| p.root == root);
        let staging_leftovers = copied.leftover_dtos();
        drop(copied);
        let (busy_projects, reservations) = {
            let guard = self.inner.lock().unwrap();
            let mut busy: Vec<u32> = guard
                .reservations
                .iter()
                .flat_map(|r| r.projects.iter().copied())
                .collect();
            busy.sort_unstable();
            busy.dedup();
            let reservations = guard
                .reservations
                .iter()
                .filter(|r| r.root == root || r.root.starts_with(root) || root.starts_with(&r.root))
                .map(|r| ReservationDto {
                    holder: r.holder.clone(),
                    root: display_path(&r.root),
                    whole_root: r.whole_root,
                    projects: r.projects.iter().copied().collect(),
                    files: r.files.values().map(|f| display_path(f)).collect(),
                })
                .collect();
            (busy, reservations)
        };
        blocking.sort();
        let active_txn_id = blocking.first().map(|(_, id)| id.clone());
        let blocked = !blocking.is_empty();
        let apply_allowed = !blocked && !in_flight_here;
        let guidance = if in_flight_here {
            "an apply is running on this root; wait for it or cancel it".to_owned()
        } else if let Some(id) = &active_txn_id {
            let prepared_here = pending_handles.iter().find(|h| &h.txn_id == id);
            match prepared_here {
                Some(h) => format!(
                    "review handle {} is prepared for transaction {id}; apply it or discard it before starting another",
                    h.handle
                ),
                None => {
                    let st = transactions
                        .iter()
                        .find(|t| &t.txn_id == id)
                        .map(|t| t.state.clone())
                        .unwrap_or_default();
                    if st == "blocked" {
                        format!(
                            "transaction {id} is blocked: a destination holds bytes the transaction does not own. Nothing was overwritten; inspect the retained recovery material and resolve it by hand, then run recovery again. No new copied-target save can start until then"
                        )
                    } else {
                        format!(
                            "transaction {id} is {st}; run recovery inspection on this root before another copied-target save"
                        )
                    }
                }
            }
        } else {
            "the root accepts a new copied-target review".to_owned()
        };
        Ok(CopiedTargetStatusDto {
            root: display_path(root),
            transactions,
            blocked,
            active_txn_id,
            pending_handles,
            busy_projects,
            reservations,
            staging_leftovers,
            apply_allowed,
            guidance,
        })
    }

    // ---- review / prepare -----------------------------------------------------------------

    /// As [`Registry::copied_target_review`], and additionally replace one **existing** raw member
    /// of a copied `EB\0\3` archive in the same change set: `archive_relative` names the archive
    /// inside `root` (e.g. `databigs/inxp_00.big`), `member_path` the exact member path (e.g.
    /// `nhl/data/common/gamemodes/database/t3db/nhl_game.db`); the member's new bytes are the
    /// project's packaged primary database. Refused when the member is missing, deduplicated
    /// (shared span) or not stored raw — no codec substitution, no entry addition. The staged
    /// archive is verified by re-parse and joins the plan as an `archive` artifact (unqualified:
    /// the PS4 `+12` row word is preserved but not understood, and native loading is unverified).
    pub fn copied_target_review_archive_member(
        &self,
        project_id: u32,
        root: &str,
        archive_relative: &str,
        member_path: &str,
    ) -> Result<CopiedTargetReviewDto, ApiError> {
        self.copied_target_review_archive_members(
            project_id,
            root,
            vec![crate::dto::ArchiveMemberSpecDto {
                archive_relative: archive_relative.to_owned(),
                member_path: member_path.to_owned(),
                artifact_key: studio_domain::copied_target::KEY_PRIMARY.to_owned(),
            }],
        )
    }

    /// As [`Registry::copied_target_review`], and additionally replace one or more **existing raw
    /// members** of copied `EB\0\3` archives in the same change set. Each spec names the archive
    /// inside `root`, the exact member path, and which plan artifact supplies the bytes: `primary`
    /// (the packaged main database — refused unless it is a bare database) or `dictionary` (the
    /// pending LOC-W1 rename's output — refused when no rename is held). Members of one archive
    /// are planned together as one staged archive; each staged archive joins the plan as an
    /// `archive` artifact (`archive`, `archive-2`, … in spec order), unqualified as before.
    pub fn copied_target_review_archive_members(
        &self,
        project_id: u32,
        root: &str,
        specs: Vec<crate::dto::ArchiveMemberSpecDto>,
    ) -> Result<CopiedTargetReviewDto, ApiError> {
        use studio_domain::tdb_archive_big::stage::{
            plan as plan_archive, BoundSource, ExpectedDecoded, Replacement, StoredForm,
        };
        if specs.is_empty() {
            return Err(ApiError::CopiedTarget(
                "no archive members were named".into(),
            ));
        }
        let canon_root = canonical_root(root)?;
        // Resolve and validate every archive path first; group specs per archive in first-seen order.
        let mut groups: Vec<(PathBuf, Vec<(String, String)>)> = Vec::new();
        for spec in &specs {
            let rel = Path::new(&spec.archive_relative);
            if rel.is_absolute()
                || rel
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err(ApiError::CopiedTarget(format!(
                    "archive path {:?} must be a plain relative path inside the copied root",
                    spec.archive_relative
                )));
            }
            let archive_path = canon_root.join(rel);
            if !archive_path.is_file() {
                return Err(ApiError::CopiedTarget(format!(
                    "{} is not a file in the copied root",
                    archive_path.display()
                )));
            }
            match spec.artifact_key.as_str() {
                studio_domain::copied_target::KEY_PRIMARY
                | studio_domain::copied_target::KEY_DICTIONARY => {}
                other => {
                    return Err(ApiError::CopiedTarget(format!(
                        "artifact key {other:?} cannot supply an archive member (only `primary` or `dictionary`)"
                    )))
                }
            }
            let entry = (spec.member_path.clone(), spec.artifact_key.clone());
            match groups.iter_mut().find(|(p, _)| *p == archive_path) {
                Some((_, members)) => {
                    if members.iter().any(|(m, _)| *m == spec.member_path) {
                        return Err(ApiError::CopiedTarget(format!(
                            "member {:?} of {} is named twice",
                            spec.member_path,
                            archive_path.display()
                        )));
                    }
                    members.push(entry);
                }
                None => groups.push((archive_path, vec![entry])),
            }
        }
        // The archives are reserved together with the workspace's files before `inner` is
        // released: the staging below reads them for minutes with no registry lock held.
        let reserve_also: Vec<PathBuf> = groups.iter().map(|(p, _)| p.clone()).collect();
        self.review_reserving(project_id, root, reserve_also, move |plan, staging_dir| {
            for (gi, (archive_for_closure, members)) in groups.iter().enumerate() {
                let mut f = std::io::BufReader::new(
                    std::fs::File::open(archive_for_closure).map_err(|source| ApiError::Io {
                        path: archive_for_closure.display().to_string(),
                        source,
                    })?,
                );
                let bound = BoundSource::bind(&mut f).map_err(|e| {
                    ApiError::CopiedTarget(format!("{}: {e}", archive_for_closure.display()))
                })?;
                let mut replacements = Vec::new();
                for (member, artifact_key) in members {
                    // LEAGUE-R1 F1: the member's bytes are the plan's own staged artifact — the
                    // very file staged under the workspace lock for this revision — never a second
                    // session read. Its identity is re-checked against the artifact's recorded
                    // identity so a mismatch is a refusal, not a silently different payload.
                    let art = plan.artifact(artifact_key).ok_or_else(|| {
                        ApiError::CopiedTarget(format!(
                            "the plan has no `{artifact_key}` artifact{}",
                            if artifact_key == studio_domain::copied_target::KEY_DICTIONARY {
                                " (no dictionary rename is pending)"
                            } else {
                                ""
                            }
                        ))
                    })?;
                    let expected_role = if artifact_key == studio_domain::copied_target::KEY_PRIMARY {
                        studio_domain::copied_target::ArtifactRole::Primary
                    } else {
                        studio_domain::copied_target::ArtifactRole::Dictionary
                    };
                    if art.role != expected_role {
                        return Err(ApiError::CopiedTarget(format!(
                            "the `{artifact_key}` artifact is {:?}; it cannot supply an archive member",
                            art.role
                        )));
                    }
                    let packaged = std::fs::read(&art.artifact.replacement).map_err(|source| {
                        ApiError::Io {
                            path: art.artifact.replacement.display().to_string(),
                            source,
                        }
                    })?;
                    // Only a bare database (or the verified dictionary, itself a bare single-table
                    // database) is a valid archive member: a packaged save container or a PS4
                    // roster stream is not the file the archive stores.
                    match tdb_core::TdbFile::parse(&packaged) {
                        Ok(db) if db.size_ok() => {}
                        _ => {
                            return Err(ApiError::CopiedTarget(format!(
                                "the staged `{artifact_key}` file is not a bare database; it cannot replace the archive member"
                            )))
                        }
                    }
                    let actual = tdb_txn::FileIdentity::of_bytes(&packaged);
                    if actual != art.artifact.replacement_identity {
                        return Err(ApiError::CopiedTarget(format!(
                            "the staged `{artifact_key}` file {} is {actual}, not the planned {}; refusing to derive an archive member from it",
                            art.artifact.replacement.display(),
                            art.artifact.replacement_identity
                        )));
                    }
                    let Some(entry) = bound.entries.iter().find(|e| e.path == *member) else {
                        return Err(ApiError::CopiedTarget(format!(
                            "{} has no member {member:?}",
                            archive_for_closure.display()
                        )));
                    };
                    if entry.shared_span {
                        return Err(ApiError::CopiedTarget(format!(
                            "member {member:?} shares its stored bytes with another entry (deduplicated); it cannot be replaced unambiguously"
                        )));
                    }
                    // D5: a `chunkzip` member (the PS4 `eng_us.db` dictionaries, and any other
                    // compressed database member) takes the artifact re-encoded with the
                    // native-framed encoder; the stager decodes it back and compares with the
                    // artifact bytes. Raw members take the bytes as they are. A `chunklzx` member
                    // takes the store route (research 2026-09-21 handoff 06: same container, LZX
                    // uncompressed blocks; loader acceptance unqualified — card R30-LZX-1). No
                    // codec substitution.
                    let new_stored = match entry.stored_form {
                        StoredForm::Raw => packaged.clone(),
                        StoredForm::ChunkZip => {
                            let enc = studio_domain::tdb_archive_codec::compress_chunkzip_native(&packaged);
                            studio_domain::tdb_archive_codec::is_native_chunkzip_framing(&enc).map_err(|e| ApiError::CopiedTarget(format!("member {member:?}: re-encoded bytes are not native-framed: {e}")))?;
                            match studio_domain::tdb_archive_codec::decompress_chunkzip(&enc) {
                                Ok(d) if d == packaged => enc,
                                Ok(_) => return Err(ApiError::CopiedTarget(format!("member {member:?}: the re-encoded bytes do not decode to the staged file"))),
                                Err(e) => return Err(ApiError::CopiedTarget(format!("member {member:?}: the re-encoded bytes do not decode: {e}"))),
                            }
                        }
                        StoredForm::ChunkLzx => {
                            let enc = studio_domain::tdb_archive_codec::compress_chunklzx_store(&packaged).map_err(|e| ApiError::CopiedTarget(format!("member {member:?}: {e}")))?;
                            match studio_domain::tdb_archive_codec::decompress_chunklzx(&enc) {
                                Ok(d) if d == packaged => enc,
                                Ok(_) => return Err(ApiError::CopiedTarget(format!("member {member:?}: the store-encoded bytes do not decode to the staged file"))),
                                Err(e) => return Err(ApiError::CopiedTarget(format!("member {member:?}: the store-encoded bytes do not decode: {e}"))),
                            }
                        }
                    };
                    replacements.push(Replacement {
                        entry_index: entry.index,
                        expected_path: entry.path.clone(),
                        expected_stored_sha256: entry.stored_sha256,
                        new_stored,
                        expected_decoded: ExpectedDecoded::of(&packaged),
                    });
                }
                let archive_plan = plan_archive(&bound, replacements).map_err(|e| {
                    ApiError::CopiedTarget(format!("planning the archive replacement: {e}"))
                })?;
                let file_name = archive_for_closure
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "archive.big".into());
                let output = staging_dir.join(format!("{file_name}.staged"));
                let staged = studio_domain::txn::stage_archive(
                    bound,
                    archive_plan,
                    archive_for_closure,
                    &output,
                )
                .map_err(ApiError::Domain)?;
                let key = if gi == 0 {
                    "archive".to_owned()
                } else {
                    format!("archive-{}", gi + 1)
                };
                plan.add_archive(&key, archive_for_closure, &staged)
                    .map_err(ApiError::Domain)?;
            }
            Ok(())
        })
    }

    /// D4: stage the pending NHL 16–21 catalogue member rewrites into the copied root's archive
    /// through the S2-A stager (fit-in-slot or append-and-retarget, decided there) and add it to
    /// the plan as the `archive-csv` artifact. Nothing to do when no CSV edit is pending.
    fn stage_text_members(plan: &mut CopiedTargetPlan, staging_dir: &Path) -> Result<(), ApiError> {
        use studio_domain::tdb_archive_big::stage::{plan as plan_archive, BoundSource};
        let Some(text) = plan.text_members.clone() else { return Ok(()) };
        let archive_path = plan.root.join(&text.archive_relative);
        if !archive_path.is_file() {
            return Err(ApiError::CopiedTarget(format!(
                "the copied root has no {} to take the pending catalogue edit(s) ({})",
                text.archive_relative,
                text.descriptions.join(" | ")
            )));
        }
        let mut f = std::io::BufReader::new(std::fs::File::open(&archive_path).map_err(|source| ApiError::Io { path: archive_path.display().to_string(), source })?);
        let bound = BoundSource::bind(&mut f).map_err(|e| ApiError::CopiedTarget(format!("{}: {e}", archive_path.display())))?;
        let replacements = studio_domain::copied_target::text_member_replacements(&bound, &text).map_err(ApiError::Domain)?;
        let archive_plan = plan_archive(&bound, replacements).map_err(|e| ApiError::CopiedTarget(format!("planning the catalogue member replacement: {e}")))?;
        let output = staging_dir.join("inxp_00.big.csv.staged");
        let staged = studio_domain::txn::stage_archive(bound, archive_plan, &archive_path, &output).map_err(ApiError::Domain)?;
        plan.add_archive("archive-csv", &archive_path, &staged).map_err(ApiError::Domain)
    }

    /// D8: bind each copied archive a pending texture-member replacement targets, plan the
    /// replacements under the **private-copy** rule (a deduplicated member is appended and only
    /// its row re-pointed; the stager names the siblings), stage the archive and add it to the
    /// plan as an `archive-asset-<n>` artifact. Nothing to do when none is pending.
    fn stage_asset_members(plan: &mut CopiedTargetPlan, staging_dir: &Path) -> Result<(), ApiError> {
        use studio_domain::tdb_archive_big::stage::{plan_with as plan_archive, BoundSource, SharedSpanRule};
        let groups = plan.asset_members.clone();
        // NHL 07: the staged `.bh` of every cascading group is mirrored into boot.big once, after
        // the loop (one boot plan with one replacement per group).
        let mut cascades: Vec<(studio_domain::asset_member_edit::Big4Cascade, PathBuf, Vec<u8>, String)> = Vec::new();
        for (n, group) in groups.iter().enumerate() {
            let archive_path = plan.root.join(&group.archive_relative);
            if !archive_path.is_file() {
                return Err(ApiError::CopiedTarget(format!(
                    "the copied root has no {} to take the pending texture replacement(s) ({})",
                    group.archive_relative,
                    group.descriptions.join(" | ")
                )));
            }
            if group.big4.is_some() && (!group.loose.is_empty() || !group.absent_loose.is_empty()) {
                return Err(ApiError::CopiedTarget(format!("{}: a PS3 board replacement cannot target a BIG4 archive", group.archive_relative)));
            }
            if let Some(info) = &group.big4 {
                // C4: a BIG4 + `.bh` pair (NHL 07–10). Bound and cross-checked together, planned
                // with the group's growth policy, staged as two files, added as two artifacts.
                use studio_domain::tdb_archive_big::big4::{plan as plan_big4, BoundBig4, Growth};
                let sidecar_path = plan.root.join(&info.sidecar_relative);
                if !sidecar_path.is_file() {
                    return Err(ApiError::CopiedTarget(format!("the copied root has no {} beside {} (the BIG4 `.bh` index is required)", info.sidecar_relative, group.archive_relative)));
                }
                let sidecar = std::fs::read(&sidecar_path).map_err(|source| ApiError::Io { path: sidecar_path.display().to_string(), source })?;
                let mut f = std::io::BufReader::new(std::fs::File::open(&archive_path).map_err(|source| ApiError::Io { path: archive_path.display().to_string(), source })?);
                let bound = BoundBig4::bind(&mut f, &sidecar).map_err(|e| ApiError::CopiedTarget(format!("{}: {e}", archive_path.display())))?;
                let replacements = studio_domain::copied_target::big4_member_replacements(&bound, group).map_err(ApiError::Domain)?;
                // Every replacement carries the placement its review predicted (fit → refuse
                // if it no longer fits, reflow, append); the plan-wide policy only backs a
                // title-level refusal (NHL 07's `boot.big` cascade).
                let growth = match &info.growth_refusal {
                    Some(reason) => Growth::Refuse(reason.clone()),
                    None => Growth::Refuse("no placement was previewed for this member".into()),
                };
                let big4_plan = plan_big4(&bound, &mut f, replacements, growth).map_err(|e| ApiError::CopiedTarget(format!("planning the texture member replacement in {}: {e}", group.archive_relative)))?;
                drop(f);
                let file_name = archive_path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "archive".into());
                let output = staging_dir.join(format!("{file_name}.asset{n}.staged"));
                let output_bh = staging_dir.join(format!("{file_name}.asset{n}.bh.staged"));
                let staged = studio_domain::txn::stage_big4(bound, big4_plan, &archive_path, &sidecar_path, &output, &output_bh).map_err(ApiError::Domain)?;
                if let Some(c) = &info.cascade {
                    // The staged sidecar differs from the source one iff a record changed; only
                    // then does the boot.big copy need to follow.
                    let new_bh = std::fs::read(staged.sidecar_output_path()).map_err(|source| ApiError::Io { path: staged.sidecar_output_path().display().to_string(), source })?;
                    if new_bh != sidecar {
                        cascades.push((c.clone(), sidecar_path.clone(), new_bh, group.archive_relative.clone()));
                    }
                }
                plan.add_big4(&format!("archive-asset-{n}"), &archive_path, &sidecar_path, &staged).map_err(ApiError::Domain)?;
                continue;
            }
            let mut f = std::io::BufReader::new(std::fs::File::open(&archive_path).map_err(|source| ApiError::Io { path: archive_path.display().to_string(), source })?);
            let bound = BoundSource::bind(&mut f).map_err(|e| ApiError::CopiedTarget(format!("{}: {e}", archive_path.display())))?;
            let replacements = studio_domain::copied_target::member_replacements(&bound, group, true).map_err(ApiError::Domain)?;
            // C7: the members whose review chose the whole group; everything else shared takes a
            // private copy.
            let whole: Vec<usize> = bound.entries.iter().filter(|e| group.whole_group.iter().any(|p| *p == e.path)).map(|e| e.index).collect();
            let archive_plan = studio_domain::tdb_archive_big::stage::plan_scoped(&bound, replacements, SharedSpanRule::PrivateCopy, &whole).map_err(|e| ApiError::CopiedTarget(format!("planning the texture member replacement in {}: {e}", group.archive_relative)))?;
            let _ = plan_archive;
            let file_name = archive_path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "archive".into());
            let output = staging_dir.join(format!("{file_name}.asset{n}.staged"));
            let staged = studio_domain::txn::stage_archive(bound, archive_plan, &archive_path, &output).map_err(ApiError::Domain)?;
            let archive_key = format!("archive-asset-{n}");
            plan.add_archive(&archive_key, &archive_path, &staged).map_err(ApiError::Domain)?;
            // Receipt 67 (C37): an archive-only PS3 board package must not have a loose copy in the copied tree — checked now and
            // again immediately before apply (`verify_plan_inputs`).
            for rel in &group.absent_loose {
                let p = plan.root.join(rel);
                if std::fs::symlink_metadata(&p).is_ok() {
                    return Err(ApiError::CopiedTarget(format!(
                        "{} exists in the copied tree, but this title ships the package only inside {}; a loose copy is another topology — nothing was staged, plan again",
                        p.display(),
                        group.archive_relative
                    )));
                }
                plan.absent_paths.push(p);
            }
            // …and a PS3 board pair's loose half joins the same change set, expected to be exactly the reviewed loose file.
            for (k, l) in group.loose.iter().enumerate() {
                let dest = plan.root.join(&l.relative);
                if !dest.is_file() {
                    return Err(ApiError::CopiedTarget(format!(
                        "the copied tree has no {} — the loose half of the reviewed PS3 board pair; updating only the archive member is not offered",
                        l.relative
                    )));
                }
                let prior = studio_domain::tdb_txn::FileIdentity { len: l.prior_len, sha256: l.prior_sha256.clone() };
                plan.add_loose(&format!("loose-asset-{n}-{k}"), &dest, prior, &l.new_bytes, staging_dir, &archive_key).map_err(ApiError::Domain)?;
            }
        }
        if !cascades.is_empty() {
            Self::stage_boot_cascade(plan, staging_dir, cascades)?;
        }
        Ok(())
    }

    /// NHL 07 (research 2026-09-20 handoff 03 §4): place each cascading group's re-encoded `.bh`
    /// into `boot.big` (the member that mirrors it: fit-in-slot against its verified-zero slack,
    /// else the ordered reflow of boot.big) and patch `boot.bh`, as two more artifacts of the same
    /// transaction. The boot pair must still be the one the reviews bound; the members are
    /// RefPack-encoded here (Studio's encoder — its stream length decides fit vs reflow).
    fn stage_boot_cascade(plan: &mut CopiedTargetPlan, staging_dir: &Path, cascades: Vec<(studio_domain::asset_member_edit::Big4Cascade, PathBuf, Vec<u8>, String)>) -> Result<(), ApiError> {
        use studio_domain::tdb_archive_big::big4::{plan as plan_big4, Big4Replacement, BoundBig4, Growth};
        let first = &cascades[0].0;
        for (c, _, _, rel) in &cascades {
            if c.boot_relative != first.boot_relative || c.boot_sha256 != first.boot_sha256 || c.boot_sidecar_sha256 != first.boot_sidecar_sha256 {
                return Err(ApiError::CopiedTarget(format!("{rel}: its review bound a different boot.big / boot.bh than another pending replacement's; preview again")));
            }
        }
        let boot_path = plan.root.join(&first.boot_relative);
        let boot_bh_path = plan.root.join(&first.boot_sidecar_relative);
        let boot_bh = std::fs::read(&boot_bh_path).map_err(|source| ApiError::Io { path: boot_bh_path.display().to_string(), source })?;
        let mut f = std::io::BufReader::new(std::fs::File::open(&boot_path).map_err(|source| ApiError::Io { path: boot_path.display().to_string(), source })?);
        let bound = BoundBig4::bind(&mut f, &boot_bh).map_err(|e| ApiError::CopiedTarget(format!("{}: {e}", boot_path.display())))?;
        let hex = |b: &[u8; 32]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        if hex(&bound.identity.sha256) != first.boot_sha256 || hex(&bound.identity.sidecar_sha256) != first.boot_sidecar_sha256 {
            return Err(ApiError::CopiedTarget(format!("{}: the copied boot.big / boot.bh are not the pair the texture reviews were made on; preview again", boot_path.display())));
        }
        let mut replacements = Vec::new();
        for (c, _, new_bh, rel) in &cascades {
            let entry = bound.entries.get(c.boot_member_index).ok_or_else(|| ApiError::CopiedTarget(format!("{rel}: boot.big has no entry {}", c.boot_member_index)))?;
            if entry.path != c.boot_member_path {
                return Err(ApiError::CopiedTarget(format!("{rel}: boot.big entry {} is {:?}, the review bound {:?}", c.boot_member_index, entry.path, c.boot_member_path)));
            }
            let mut expected = [0u8; 32];
            for (i, b) in expected.iter_mut().enumerate() {
                *b = u8::from_str_radix(&c.boot_member_stored_sha256[i * 2..i * 2 + 2], 16).map_err(|_| ApiError::CopiedTarget("malformed cascade identity".into()))?;
            }
            if new_bh.len() > 0xFF_FFFF {
                return Err(ApiError::CopiedTarget(format!("{rel}: the re-encoded index ({} bytes) exceeds the RefPack 3-byte length field", new_bh.len())));
            }
            let stored = studio_domain::tdb_archive_codec::refpack_compress(new_bh);
            let back = studio_domain::tdb_archive_codec::refpack_decompress(&stored).map_err(|e| ApiError::CopiedTarget(format!("{rel}: the re-wrapped index does not decode: {e}")))?;
            if &back != new_bh {
                return Err(ApiError::CopiedTarget(format!("{rel}: the re-wrapped index decodes to different bytes")));
            }
            replacements.push(Big4Replacement { entry_index: entry.index, expected_path: entry.path.clone(), expected_stored_sha256: expected, new_stored: stored, decoded_len: new_bh.len() as u32, growth: Some(Growth::Reflow) });
        }
        let boot_plan = plan_big4(&bound, &mut f, replacements, Growth::Refuse("boot.big cascade members carry their own policy".into())).map_err(|e| ApiError::CopiedTarget(format!("planning the boot.big index cascade: {e}")))?;
        drop(f);
        let output = staging_dir.join("boot.big.cascade.staged");
        let output_bh = staging_dir.join("boot.big.cascade.bh.staged");
        let staged = studio_domain::txn::stage_big4(bound, boot_plan, &boot_path, &boot_bh_path, &output, &output_bh).map_err(ApiError::Domain)?;
        plan.add_big4("archive-asset-boot", &boot_path, &boot_bh_path, &staged).map_err(ApiError::Domain)
    }

    /// Package the project's sessions for the copied-target root `root`, plan the content headers
    /// the copied tree newly requires (binding the existing ones), and **prepare** the change set
    /// with the engine. Returns a server-owned handle bound to the project, the root and the
    /// workspace's content revision. A previous prepared handle for the same project is discarded
    /// first (its engine set is aborted); a root that is busy, blocked or interrupted is refused
    /// with guidance and nothing is staged.
    pub fn copied_target_review(
        &self,
        project_id: u32,
        root: &str,
    ) -> Result<CopiedTargetReviewDto, ApiError> {
        self.copied_target_review_with(project_id, root, |_, _| Ok(()))
    }

    /// [`Registry::copied_target_review`] with a hook that may extend the staged plan before it is
    /// prepared — the seam a Studio-internal producer (an archive staged through the controlled
    /// S2-A adapter, `CopiedTargetPlan::add_archive`) joins the same prepared-artifact service
    /// through. Not a command: nothing frontend-supplied reaches the plan.
    #[doc(hidden)]
    pub fn copied_target_review_with(
        &self,
        project_id: u32,
        root: &str,
        extend: impl FnOnce(&mut CopiedTargetPlan, &Path) -> Result<(), ApiError>,
    ) -> Result<CopiedTargetReviewDto, ApiError> {
        self.review_reserving(project_id, root, Vec::new(), extend)
    }

    /// The review, with `reserve_also` — files the `extend` hook will read or stage (an archive)
    /// — reserved alongside the workspace's own files before `inner` is released. See the
    /// module's *Lock scope* section: the sessions are packaged under `inner`; `extend` and the
    /// engine's `prepare` run with only the review's reservation held.
    fn review_reserving(
        &self,
        project_id: u32,
        root: &str,
        reserve_also: Vec<PathBuf>,
        extend: impl FnOnce(&mut CopiedTargetPlan, &Path) -> Result<(), ApiError>,
    ) -> Result<CopiedTargetReviewDto, ApiError> {
        let root = canonical_root(root)?;
        // One prepared set per project: re-review discards the previous one.
        let previous: Vec<u32> = {
            let copied = self.copied.lock().unwrap();
            if copied
                .in_flight
                .values()
                .any(|p| p.project_id == project_id)
            {
                return Err(ApiError::CopiedTarget(format!(
                    "project {project_id} has a copied-target apply in progress; wait for it or cancel it before reviewing again"
                )));
            }
            copied
                .prepared
                .iter()
                .filter(|(_, p)| p.project_id == project_id)
                .map(|(h, _)| *h)
                .collect()
        };
        for h in previous {
            let _ = self.copied_target_discard(h);
        }
        let status = self.status_of(&root)?;
        if !status.apply_allowed {
            return Err(ApiError::CopiedTarget(format!(
                "the copied-target root {} cannot take a new save: {}",
                root.display(),
                status.guidance
            )));
        }
        let handle = {
            let mut copied = self.copied.lock().unwrap();
            let h = copied.next_handle;
            copied.next_handle += 1;
            h
        };
        // A fresh, process-unique name (pid, sequence, handle): never an existing directory,
        // so nothing is removed here — every staging deletion goes through `settle`/`drop_staging`.
        let staging_dir = staging_dir_for(handle);
        std::fs::create_dir_all(&staging_dir).map_err(|source| ApiError::Io {
            path: staging_dir.display().to_string(),
            source,
        })?;
        // Package the sessions and take the review's reservation in one `inner` critical
        // section: the plan is bound to the workspace's revision, and from here until the handle
        // is registered no edit, save, reload, close, alias open or competing copied-target
        // operation can reach the project or its files (they are refused with the holder named).
        let holder = format!("review of project {project_id} (handle {handle})");
        let packaged = {
            let mut guard = self.inner.lock().unwrap();
            if let Some(r) = guard.reservation_for_project(project_id) {
                Err(ApiError::CopiedTarget(format!(
                    "project {project_id} is locked by a copied-target operation in progress ({}); wait for it to finish (or cancel it) before reviewing",
                    r.holder()
                )))
            } else {
                match guard.sessions.get(&project_id) {
                    None => Err(ApiError::UnknownProject(project_id)),
                    Some(ws) => {
                        let title = format!(
                            "Studio copied-target save of project {project_id} ({})",
                            Path::new(ws.primary_path())
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default()
                        );
                        let primary_role = ws.primary().project().role;
                        stage_workspace(ws, &root, &staging_dir, &title)
                            .map_err(ApiError::Domain)
                            .and_then(|plan| {
                                let files = reservation_files(&plan, &reserve_also)?;
                                Ok((plan, primary_role, files))
                            })
                    }
                }
                .and_then(|(plan, primary_role, files)| {
                    let (reservation_id, _) = guard.reserve(
                        holder.clone(),
                        &root,
                        files,
                        false,
                        BTreeSet::from([project_id]),
                    )?;
                    Ok((plan, primary_role, reservation_id))
                })
            }
        };
        let (mut plan, primary_role, reservation_id) = match packaged {
            Ok(x) => x,
            Err(e) => {
                // No handle exists yet: `prepare` did not run, so no engine journal names these
                // files. Plain removal; a failure to remove is still tracked.
                self.copied.lock().unwrap().drop_staging(&staging_dir);
                return Err(e);
            }
        };
        let reservation = ReservationGuard {
            registry: self,
            id: reservation_id,
        };

        // The long work — an archive staged and re-verified by `extend`, the engine's `prepare`
        // hashing every destination and staged output — with only the reservation held.
        let prepared = (|| -> Result<tdb_txn::Transaction, ApiError> {
            Self::stage_text_members(&mut plan, &staging_dir)?;
            Self::stage_asset_members(&mut plan, &staging_dir)?;
            extend(&mut plan, &staging_dir)?;
            // An artifact `extend` added whose file is not reserved yet (the generic seam;
            // the archive-member command reserves its archive up front) joins the reservation
            // before anything is prepared against it.
            let grown = reservation_files(&plan, &[])?;
            self.inner
                .lock()
                .unwrap()
                .extend_reservation(reservation_id, grown)?;
            Ok(tdb_txn::prepare(
                &plan.root,
                &plan.change_set,
                &mut tdb_txn::NoHooks,
            )?)
        })();
        let txn = match prepared {
            Ok(txn) => txn,
            Err(e) => {
                drop(reservation);
                self.copied.lock().unwrap().drop_staging(&staging_dir);
                return Err(e);
            }
        };

        // Final binding validation under `inner` (defensive: the reservation already excluded
        // every registry writer), then the handle is registered **before** the reservation is
        // released — there is never a moment where the root's active set has no owner.
        let still_bound = {
            let guard = self.inner.lock().unwrap();
            match guard.sessions.get(&project_id) {
                None => Err(format!("project {project_id} was closed during the review")),
                Some(ws) if ws.revision() != plan.revision => Err(format!(
                    "project {project_id} changed during the review (revision {} is now {})",
                    plan.revision,
                    ws.revision()
                )),
                Some(_) => Ok(()),
            }
        };
        if let Err(why) = still_bound {
            drop(reservation);
            let tail = match self.copied.lock().unwrap().settle(staging_dir, txn) {
                Ok(()) => "the prepared set was aborted and the root is free".to_owned(),
                Err(reason) => format!(
                    "the prepared set could not be aborted cleanly and its staging is retained ({reason})"
                ),
            };
            return Err(ApiError::CopiedTarget(format!(
                "{why}; {tail} — review again"
            )));
        }
        let artifacts: Vec<CopiedTargetArtifactDto> = plan
            .artifacts
            .iter()
            .map(|a| artifact_dto(primary_role, &plan, &a.artifact.key))
            .collect();
        let qualification_state = if plan
            .change_set
            .artifacts
            .iter()
            .all(|a| a.validation.qualification.is_qualified())
        {
            "qualified"
        } else {
            "unqualified"
        };
        let dto = CopiedTargetReviewDto {
            handle,
            project_id,
            txn_id: txn.txn_id.clone(),
            root: display_path(&plan.root),
            workspace_revision: plan.revision.to_string(),
            mode: "copied-target".into(),
            qualification_state: qualification_state.into(),
            live_install_allowed: false,
            artifacts,
            bound_inputs: plan
                .bound_inputs
                .iter()
                .map(|b| BoundInputDto {
                    kind: b.kind.as_str().into(),
                    purpose: b.purpose.clone(),
                    path: display_path(&b.path),
                    identity: identity_dto(&b.identity),
                })
                .collect(),
            status: CopiedTargetStatusDto {
                // Filled below, once the handle is registered.
                root: String::new(),
                transactions: vec![],
                blocked: false,
                active_txn_id: None,
                pending_handles: vec![],
                busy_projects: vec![],
                reservations: vec![],
                staging_leftovers: vec![],
                apply_allowed: false,
                guidance: String::new(),
            },
        };
        self.copied.lock().unwrap().prepared.insert(
            handle,
            Prepared {
                project_id,
                root: plan.root.clone(),
                plan,
                txn,
                staging_dir,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );
        drop(reservation);
        let status = self.status_of(&root)?;
        Ok(CopiedTargetReviewDto { status, ..dto })
    }

    // ---- apply ----------------------------------------------------------------------------

    /// Apply a prepared handle: re-verify the binding (project, revision, destinations, bound
    /// inputs, parents, staged outputs) and **reserve** the affected files and every workspace on
    /// them in the same critical section, run the engine, and on commit rebind the initiating
    /// sessions and refresh the aliases. Any refusal or failure keeps every session's edits; the
    /// handle is consumed either way (re-review to try again).
    pub fn copied_target_apply(
        &self,
        project_id: u32,
        handle: u32,
    ) -> Result<CopiedTargetApplyDto, ApiError> {
        self.copied_target_apply_with_hooks(project_id, handle, None)
    }

    /// [`Registry::copied_target_apply`] with an additional engine hook — the fault-injection and
    /// interruption seam the tests use (the engine's own `Hooks`). Not a command.
    #[doc(hidden)]
    pub fn copied_target_apply_with_hooks(
        &self,
        project_id: u32,
        handle: u32,
        extra: Option<&mut dyn Hooks>,
    ) -> Result<CopiedTargetApplyDto, ApiError> {
        self.copied_target_apply_observed(project_id, handle, extra, &mut |_| {})
    }

    /// [`Registry::copied_target_apply_with_hooks`] with a registry-level phase observer — the
    /// contention seam (LEAGUE-RESP-1): `observe` is called with each [`ApplyPhase`] as the apply
    /// passes it, outside every registry lock, so a test can hold the apply at the exact point
    /// where its reservation exists but `inner` is free and exercise close/status/cancel/edits
    /// against it. Not a command.
    #[doc(hidden)]
    pub fn copied_target_apply_observed(
        &self,
        project_id: u32,
        handle: u32,
        extra: Option<&mut dyn Hooks>,
        observe: &mut dyn FnMut(ApplyPhase),
    ) -> Result<CopiedTargetApplyDto, ApiError> {
        // Take the handle: a second apply (concurrent or later) finds nothing.
        let Prepared {
            project_id: bound_project,
            root,
            plan,
            mut txn,
            staging_dir,
            cancel,
        } = {
            let mut copied = self.copied.lock().unwrap();
            let Some(p) = copied.prepared.remove(&handle) else {
                return Err(ApiError::CopiedTarget(format!(
                    "review handle {handle} is unknown, already applied, discarded or superseded by a newer review; review again"
                )));
            };
            if p.project_id != project_id {
                let bound = p.project_id;
                copied.prepared.insert(handle, p);
                return Err(ApiError::CopiedTarget(format!(
                    "review handle {handle} belongs to project {bound}, not project {project_id}"
                )));
            }
            let txn_id = p.txn.txn_id.clone();
            copied.in_flight.insert(
                handle,
                InFlight {
                    project_id,
                    root: p.root.clone(),
                    txn_id,
                    cancel: p.cancel.clone(),
                },
            );
            p
        };
        debug_assert_eq!(bound_project, project_id);
        // Owns the handle's exit on every path out of this function — return, `?`, or a panic
        // unwinding out of the engine (a hook's assertion, say): the handle leaves `in_flight`
        // and its staging is **settled** ([`CopiedState::settle`]) with a snapshot of the
        // prepared handle — aborted where the journal still says `Prepared`/`Staged`, released
        // where the journal shows the set resolved (the engine holds its own copies), retained
        // as a reported obligation otherwise. Nothing is deleted on the assumption that the abort
        // or the engine's staging succeeded.
        struct Finish<'a> {
            registry: &'a Registry,
            handle: u32,
            /// `None` once settled explicitly (the Drop is then a no-op).
            staging: Option<(PathBuf, tdb_txn::Transaction)>,
        }
        impl Finish<'_> {
            fn settle(&mut self) -> Result<(), String> {
                let mut copied = self.registry.copied.lock().unwrap();
                copied.in_flight.remove(&self.handle);
                match self.staging.take() {
                    Some((dir, snapshot)) => copied.settle(dir, snapshot),
                    None => Ok(()),
                }
            }
        }
        impl Drop for Finish<'_> {
            fn drop(&mut self) {
                let _ = self.settle();
            }
        }
        let mut finish_guard = Finish {
            registry: self,
            handle,
            staging: Some((staging_dir, txn.clone())),
        };
        // A refusal before the engine ran: settle now so the message can say what happened to
        // the prepared set (aborted and the root is free, or retained as an obligation).
        let mut abort_and_fail = |msg: String| {
            let tail = match finish_guard.settle() {
                Ok(()) => "the prepared set was aborted and the root is free; the edits are kept — review again".to_owned(),
                Err(reason) => format!("the prepared set could not be aborted cleanly and its staging is retained ({reason}); the edits are kept"),
            };
            ApiError::CopiedTarget(format!("{msg}. {tail}"))
        };

        // Binding checks **and** reservation in one `inner` critical section: the project, its
        // revision and its baseline are verified and the affected files + every workspace on
        // them are reserved before the lock is released, so no edit, save, open or close can
        // slip in between. The identity verification of the files themselves (hashing every
        // destination, staged output and bound input) follows *after* the lock is released,
        // with only the reservation held (LEAGUE-RESP-1).
        let reserved = {
            let mut guard = self.inner.lock().unwrap();
            let checked = (|| -> Result<BTreeMap<PathBuf, PathBuf>, ApiError> {
                if let Some(r) = guard.reservation_for_project(project_id) {
                    return Err(ApiError::CopiedTarget(format!(
                        "project {project_id} is locked by {}; wait for it",
                        r.holder
                    )));
                }
                let ws = guard
                    .sessions
                    .get(&project_id)
                    .ok_or(ApiError::UnknownProject(project_id))?;
                if ws.revision() != plan.revision {
                    return Err(ApiError::CopiedTarget(format!(
                        "project {project_id} was edited after the review (revision {} is now {})",
                        plan.revision,
                        ws.revision()
                    )));
                }
                if let Some(reason) = ws.stale_baseline() {
                    return Err(ApiError::CopiedTarget(format!(
                        "project {project_id} cannot be saved: {reason}"
                    )));
                }
                // The tree's layout (no file opened or hashed), then the held files under their
                // reservation identity (S2-C-R2); a path whose identity cannot be established
                // refuses here, before anything is held.
                verify_plan_layout(&plan).map_err(|e| ApiError::CopiedTarget(e.to_string()))?;
                reservation_files(&plan, &[])
            })();
            checked.and_then(|files| {
                guard.reserve(
                    format!("apply of review handle {handle} (project {project_id})"),
                    &root,
                    files,
                    false,
                    BTreeSet::from([project_id]),
                )
            })
        };
        let (reservation_id, projects) = match reserved {
            Ok(x) => x,
            Err(e) => return Err(abort_and_fail(e.to_string())),
        };
        let guard = ReservationGuard {
            registry: self,
            id: reservation_id,
        };
        observe(ApplyPhase::Reserved);

        let mut hooks = ApplyHooks {
            cancel,
            log: Vec::new(),
            extra,
        };
        // Identity verification with the reservation held and no registry lock: every
        // destination still holds its expected prior, every staged output is exactly the planned
        // bytes, every bound input is unchanged. Minutes on a real archive — so a cancellation
        // requested meanwhile is honoured between files (the engine then aborts the set at its
        // first checkpoint, exactly as a cancel before the apply would). A skipped verification
        // is never treated as passed: the engine only ever runs after `Verified`, or to abort.
        let verification = verify_plan_inputs_unless(&plan, &|| hooks.cancelled());
        match verification {
            Ok(PlanVerification::Verified) => {}
            Ok(PlanVerification::Cancelled) => {
                hooks
                    .log
                    .push("cancellation requested during the apply's preflight".into());
            }
            Err(e) => {
                drop(guard);
                return Err(abort_and_fail(e.to_string()));
            }
        }
        observe(ApplyPhase::Verified);
        // Final binding validation under `inner` — defensive; the reservation excluded every
        // registry writer since the revision check above, so this can only fail on a bypass.
        let rebound = {
            let inner = self.inner.lock().unwrap();
            match inner.sessions.get(&project_id) {
                None => Err(format!("project {project_id} is no longer open")),
                Some(ws) if ws.revision() != plan.revision => Err(format!(
                    "project {project_id} changed after the review (revision {} is now {})",
                    plan.revision,
                    ws.revision()
                )),
                Some(_) => Ok(()),
            }
        };
        if let Err(why) = rebound {
            drop(guard);
            return Err(abort_and_fail(why));
        }

        // The engine, without any registry lock held.
        let result = tdb_txn::apply(&mut txn, &plan.change_set, &mut hooks);
        let log = std::mem::take(&mut hooks.log);

        // Back under the session lock (reservation still held): on commit, rebind the initiator
        // and refresh every alias; on any other outcome nothing in any session changes.
        let outcome = {
            let mut inner = self.inner.lock().unwrap();
            match result {
                Ok(outcome) => {
                    let Some(ws) = inner.sessions.get_mut(&project_id) else {
                        return Err(ApiError::CopiedTarget(format!(
                            "transaction {} committed but project {project_id} is no longer open; the files are installed and restorable through the root's status",
                            outcome.txn_id
                        )));
                    };
                    let id_of = |key: &str| {
                        outcome
                            .destinations
                            .iter()
                            .find(|(k, _)| k == key)
                            .and_then(|(_, id)| id.clone())
                    };
                    let primary = id_of(studio_domain::copied_target::KEY_PRIMARY);
                    let roster = id_of(studio_domain::copied_target::KEY_ROSTER);
                    let dictionary = id_of(studio_domain::copied_target::KEY_DICTIONARY);
                    if outcome.destinations.iter().any(|(k, _)| k == "archive-csv") {
                        ws.rebind_csv_after_copied_save();
                    }
                    if outcome.destinations.iter().any(|(k, _)| k.starts_with("archive-asset-")) {
                        ws.rebind_assets_after_copied_save();
                    }
                    if let Err(e) = ws.rebind_after_copied_save(
                        primary.as_ref(),
                        roster.as_ref(),
                        dictionary.as_ref(),
                    ) {
                        return Err(ApiError::CopiedTarget(format!(
                            "transaction {} committed and is restorable, but the session could not adopt the installed file as its baseline: {e}",
                            outcome.txn_id
                        )));
                    }
                    let project = ProjectBaselineDto {
                        project_id,
                        primary: "rebound".into(),
                        roster: ws.roster_attached().then(|| "rebound".to_owned()),
                        dirty: ws.is_dirty(),
                        stale_baseline: ws.stale_baseline().map(str::to_owned),
                        refresh_error: None,
                    };
                    let mut aliases = Vec::new();
                    for id in projects.iter().filter(|id| **id != project_id) {
                        if let Some(alias) = inner.sessions.get_mut(id) {
                            aliases.push(refresh_workspace(*id, alias));
                        }
                    }
                    Ok((outcome, project, aliases))
                }
                Err(e) => Err(e),
            }
        };
        drop(guard);
        // Settle explicitly (the journal now says committed / aborted / rolled back, or the
        // engine failed part-way and recovery owns it) so the status below reflects it.
        let _ = finish_guard.settle();
        let (outcome, project, aliases) = outcome?;
        let status = self.status_of(&root)?;
        Ok(CopiedTargetApplyDto {
            handle,
            txn_id: outcome.txn_id.clone(),
            state: state_name(outcome.state),
            recovery_dir: display_path(&outcome.recovery_dir),
            destinations: destinations_dto(&outcome, &txn),
            applied_unqualified: txn.journal.applied_unqualified,
            project,
            aliases,
            log,
            status,
        })
    }

    /// The staging directory of a prepared handle — the tests' seam for asserting the staging
    /// lifecycle. Not a command.
    #[doc(hidden)]
    pub fn copied_target_staging_dir(&self, handle: u32) -> Option<PathBuf> {
        self.copied
            .lock()
            .unwrap()
            .prepared
            .get(&handle)
            .map(|p| p.staging_dir.clone())
    }

    // ---- cancel / discard -----------------------------------------------------------------

    /// Request cancellation of a prepared or in-flight handle. The engine polls the flag between
    /// artifacts and before every mutation: before mutation began the set ends `Aborted`, after it
    /// began every destination is put back (`RolledBack`). Returns whether the handle was found.
    pub fn copied_target_cancel(&self, handle: u32) -> Result<bool, ApiError> {
        let copied = self.copied.lock().unwrap();
        if let Some(p) = copied.prepared.get(&handle) {
            p.cancel.store(true, Ordering::SeqCst);
            return Ok(true);
        }
        if let Some(p) = copied.in_flight.get(&handle) {
            p.cancel.store(true, Ordering::SeqCst);
            return Ok(true);
        }
        Ok(false)
    }

    /// Discard a prepared handle: abort its engine set (freeing the root), remove the staged
    /// outputs. Nothing about the sessions changes. Returns the root's status afterwards.
    pub fn copied_target_discard(&self, handle: u32) -> Result<CopiedTargetStatusDto, ApiError> {
        let p = {
            let mut copied = self.copied.lock().unwrap();
            if copied.in_flight.contains_key(&handle) {
                return Err(ApiError::CopiedTarget(format!(
                    "review handle {handle} is being applied; cancel it instead"
                )));
            }
            copied.prepared.remove(&handle).ok_or_else(|| {
                ApiError::CopiedTarget(format!(
                    "review handle {handle} is unknown, already applied or already discarded"
                ))
            })?
        };
        let root = p.root.clone();
        self.discard_prepared(p)?;
        self.status_of(&root)
    }

    /// Settle a prepared handle taken out of the state ([`CopiedState::settle`]): abort where
    /// the journal allows, then remove the staging; otherwise retain both and return the reason.
    /// The handle is consumed either way (a retained set lives on as an obligation, with its
    /// engine handle, until a later settle resolves it).
    fn discard_prepared(&self, p: Prepared) -> Result<(), ApiError> {
        self.copied
            .lock()
            .unwrap()
            .settle(p.staging_dir, p.txn)
            .map_err(ApiError::CopiedTarget)
    }

    /// Every prepared handle bound to `project_id`, taken out of the state; refused while an
    /// apply for the project is in flight.
    fn take_prepared_for_project(&self, project_id: u32) -> Result<Vec<Prepared>, ApiError> {
        let mut copied = self.copied.lock().unwrap();
        if copied
            .in_flight
            .values()
            .any(|p| p.project_id == project_id)
        {
            return Err(ApiError::CopiedTarget(format!(
                "project {project_id} has a copied-target apply in progress; wait for it or cancel it"
            )));
        }
        let keys: Vec<u32> = copied
            .prepared
            .iter()
            .filter(|(_, p)| p.project_id == project_id)
            .map(|(h, _)| *h)
            .collect();
        Ok(keys
            .into_iter()
            .filter_map(|h| copied.prepared.remove(&h))
            .collect())
    }

    /// Discard every prepared handle of `project_id` (its session is being closed, so none of them
    /// could ever be applied). Called by `close_project`. Every handle is settled even when an
    /// earlier one fails; the first failure is returned afterwards.
    pub(super) fn discard_prepared_for_project(&self, project_id: u32) -> Result<(), ApiError> {
        let mut first_error = None;
        for p in self.take_prepared_for_project(project_id)? {
            if let Err(e) = self.discard_prepared(p) {
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    /// Normal teardown of the copied-target state (the shell's exit, an explicit session end):
    /// every prepared handle is settled — aborted with the engine and its staging removed where
    /// the journal allows, retained as an obligation otherwise — and every earlier obligation is
    /// settled again. Every handle is processed even when one fails. In-flight applies are not
    /// interrupted (the engine's recovery covers them if the process ends). Returns what is still
    /// on disk afterwards, with its kind and reason; an empty list means nothing is left behind.
    /// Calling it again is safe: a retained set is re-checked, never deleted while unresolved.
    pub fn teardown_copied_target(&self) -> Vec<StagingLeftoverDto> {
        let mut copied = self.copied.lock().unwrap();
        let prepared: Vec<Prepared> = copied.prepared.drain().map(|(_, p)| p).collect();
        for p in prepared {
            let _ = copied.settle(p.staging_dir, p.txn);
        }
        copied.retry_leftovers();
        copied.leftover_dtos()
    }

    // ---- recover / restore ----------------------------------------------------------------

    /// Reserve the whole `root` and every open workspace under it for `holder`. Refuses when any
    /// reservation overlaps (a nested or enclosing root, a file under it, a shared project).
    fn reserve_root(&self, root: &Path, holder: String) -> Result<(u64, BTreeSet<u32>), ApiError> {
        self.inner
            .lock()
            .unwrap()
            .reserve(holder, root, BTreeMap::new(), true, BTreeSet::new())
    }

    /// Refresh the baselines of every reserved project (the reservation is still held, so nothing
    /// else can touch them meanwhile). A failed refresh is reported per project, never hidden.
    fn refresh_projects(&self, projects: &BTreeSet<u32>) -> Vec<ProjectBaselineDto> {
        let mut inner = self.inner.lock().unwrap();
        let mut out = Vec::new();
        for id in projects {
            if let Some(ws) = inner.sessions.get_mut(id) {
                out.push(refresh_workspace(*id, ws));
            }
        }
        out
    }

    /// Run the engine's deterministic recovery on `root` (from its journals and the files alone —
    /// no handle is needed, so a restarted registry finds an interrupted transaction here), then
    /// refresh every open workspace under the root. Prepared handles on the root are dropped: the
    /// engine aborts prepared-but-unapplied sets. Refuses while an apply is in flight on the root
    /// or any reservation overlaps the root.
    pub fn copied_target_recover(&self, root: &str) -> Result<CopiedTargetRecoveryDto, ApiError> {
        let root = canonical_root(root)?;
        if self
            .copied
            .lock()
            .unwrap()
            .in_flight
            .values()
            .any(|p| p.root == root)
        {
            return Err(ApiError::CopiedTarget(
                "an apply is running on this root; wait for it or cancel it before recovery".into(),
            ));
        }
        // Reserve **before** touching any handle: a refused reservation (an overlapping apply,
        // restore or recovery) leaves every prepared handle exactly where it was, still usable.
        let (reservation_id, projects) =
            self.reserve_root(&root, format!("recovery of {}", display_path(&root)))?;
        let guard = ReservationGuard {
            registry: self,
            id: reservation_id,
        };
        // Now the root is ours: no apply can take one of these handles into flight (its
        // reservation would overlap this one), so taking them here is an owned transition.
        let stale_handles: Vec<Prepared> = {
            let mut copied = self.copied.lock().unwrap();
            let keys: Vec<u32> = copied
                .prepared
                .iter()
                .filter(|(_, p)| p.root == root)
                .map(|(h, _)| *h)
                .collect();
            keys.into_iter()
                .filter_map(|h| copied.prepared.remove(&h))
                .collect()
        };
        let recovered = tdb_txn::recover(&root, &mut tdb_txn::NoHooks);
        let affected = self.refresh_projects(&projects);
        drop(guard);
        {
            // Recovery aborts every prepared set on the root; each handle's staging is settled
            // against what the journal now says (never removed on the assumption alone).
            let mut copied = self.copied.lock().unwrap();
            for p in stale_handles {
                let _ = copied.settle(p.staging_dir, p.txn);
            }
            copied.retry_leftovers();
        }
        recovered?;
        Ok(CopiedTargetRecoveryDto {
            affected,
            status: self.status_of(&root)?,
        })
    }

    /// Restore the exact originals of a committed transaction, then refresh every open workspace
    /// under the root. A prepared handle on the root must be discarded first (the engine refuses a
    /// root whose active transaction is another one); out-of-order restores are refused by the
    /// engine with the transaction to restore first.
    pub fn copied_target_restore(
        &self,
        root: &str,
        txn_id: &str,
    ) -> Result<CopiedTargetRestoreDto, ApiError> {
        self.copied_target_restore_with_hooks(root, txn_id, &mut tdb_txn::NoHooks)
    }

    /// [`Registry::copied_target_restore`] with an engine hook — the seam the tests use to act
    /// while the whole-root reservation is held. Not a command.
    #[doc(hidden)]
    pub fn copied_target_restore_with_hooks(
        &self,
        root: &str,
        txn_id: &str,
        hooks: &mut dyn Hooks,
    ) -> Result<CopiedTargetRestoreDto, ApiError> {
        let root = canonical_root(root)?;
        {
            let copied = self.copied.lock().unwrap();
            if copied.in_flight.values().any(|p| p.root == root) {
                return Err(ApiError::CopiedTarget(
                    "an apply is running on this root; wait for it or cancel it before restoring"
                        .into(),
                ));
            }
            if let Some((h, _)) = copied.prepared.iter().find(|(_, p)| p.root == root) {
                return Err(ApiError::CopiedTarget(format!(
                    "review handle {h} is prepared on this root; discard it before restoring"
                )));
            }
        }
        let (reservation_id, projects) = self.reserve_root(
            &root,
            format!("restore of transaction {txn_id} in {}", display_path(&root)),
        )?;
        let guard = ReservationGuard {
            registry: self,
            id: reservation_id,
        };
        let restored = tdb_txn::restore(&root, txn_id, hooks);
        let affected = self.refresh_projects(&projects);
        drop(guard);
        let outcome = restored?;
        let journals = tdb_txn::engine::history(&root)?;
        let destinations = outcome
            .destinations
            .iter()
            .map(|(key, id)| DestinationOutcomeDto {
                key: key.clone(),
                destination: journals
                    .get(txn_id)
                    .and_then(|j| j.artifact(key))
                    .map(|a| a.destination.clone())
                    .unwrap_or_default(),
                identity: id.as_ref().map(identity_dto),
            })
            .collect();
        Ok(CopiedTargetRestoreDto {
            txn_id: outcome.txn_id.clone(),
            state: state_name(outcome.state),
            destinations,
            affected,
            status: self.status_of(&root)?,
        })
    }

    // ---- reload ---------------------------------------------------------------------------

    /// Explicitly reload a project's sessions from disk (after a restore, recovery or failed
    /// refresh left its baseline stale). A dirty workspace is first **re-checked**: if its files
    /// carry the loaded identities again (the file was put back), the stale mark clears and the
    /// edits are kept. Otherwise reloading would discard the edits, which is refused unless
    /// `discard_edits` is set — the user's decision, never implicit. A reload that fails (the file
    /// is still missing or unreadable) changes nothing and keeps the workspace marked stale.
    pub fn copied_target_reload(
        &self,
        project_id: u32,
        discard_edits: bool,
    ) -> Result<ProjectBaselineDto, ApiError> {
        self.with_workspace(project_id, |ws| {
            if ws.is_dirty() && !discard_edits {
                let refresh = refresh_workspace(project_id, ws);
                let resolved = refresh.refresh_error.is_none()
                    && refresh.primary != "stale-kept-edits"
                    && refresh.roster.as_deref() != Some("stale-kept-edits");
                if resolved {
                    return Ok(refresh);
                }
                return Err(ApiError::CopiedTarget(format!(
                    "project {project_id} has unsaved edits and its files still differ from what it was opened on ({}); reloading would discard the edits. Save them to a copied target or export them first, or reload with discard_edits set",
                    refresh
                        .refresh_error
                        .or(refresh.stale_baseline)
                        .unwrap_or_default()
                )));
            }
            ws.reload_from_disk().map_err(ApiError::Domain)?;
            Ok(ProjectBaselineDto {
                project_id,
                primary: "reloaded".into(),
                roster: ws.roster_attached().then(|| "reloaded".to_owned()),
                dirty: ws.is_dirty(),
                stale_baseline: None,
                refresh_error: None,
            })
        })
    }
}
