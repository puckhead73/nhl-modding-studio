//! The transaction engine. See the crate docs for the protocol.
//!
//! ## Containment (S2-B-R1)
//!
//! Every path the engine reads or writes is resolved through
//! [`contained`]: the root is canonicalized and must carry the copied-target
//! marker; every component from the root down to the file's parent is
//! checked with `symlink_metadata` and refused if it is a symlink or
//! junction (reparse point); the canonical parent must lie inside the
//! canonical root; the leaf itself, when present, must be a regular file
//! (never a reparse point or directory). That applies to destinations, the
//! `.tdb-txn` directory, every recovery directory and every backup, staged
//! and temporary file — and it is done again at the mutation boundary (each
//! rename, each revert), not only at prepare. Volume identity is compared on
//! the **canonical** paths (junctions to other volumes resolve to a different
//! prefix and are refused as escapes before the volume check is reached).
//!
//! ## History (S2-B-R1)
//!
//! Every transaction has a per-root `sequence`; transaction ids sort in
//! sequence order. A destination is **owned** by the highest-sequence
//! transaction that took it (`Applying`, `Committed`, `Restoring`, `Blocked`)
//! and has not given it back (`Restored`, `RolledBack`, `Aborted`). Recovery
//! verifies a committed transaction's destination only when that transaction
//! still owns it; an older committed transaction whose destination a later
//! one took is *superseded* there, not blamed. Restore is refused while a
//! later owner exists (`RestoreOutOfOrder`, no state change); restoring the
//! later transaction first puts the destination back to the older one's
//! replacement, after which the older one restores normally.

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::changeset::{self, valid_txn_id, ChangeSet, ChangeSetError, Expected, Mode};
use crate::identity::{copy_new_verified, fsync_path, FileIdentity};
use crate::journal::{
    ArtifactPhase, Journal, JournalArtifact, JournalError, State, ACTIVE_MARKER, BACKUPS_DIR,
    CHANGESET_FILE, JOURNAL_SCHEMA, LOCK_FILE, RESTORE_DIR, SEQUENCE_FILE, STAGED_DIR, TXN_DIR,
};

/// Marker file a copied-target root must carry. Written only by
/// [`create_copied_target_root`], which refuses an existing directory, so a
/// root can never be a live game tree that happened to be pointed at.
pub const COPIED_TARGET_MARKER: &str = "COPIED-TARGET.tdb-txn";

/// A point in the protocol at which [`Hooks::event`] is called. `Before*`
/// points fire before the durable action, `After*` right after it. A hook
/// that returns an error at a point simulates a failure there; a hook that
/// exits the process simulates a crash there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Prepared,
    BeforeStageReplacement(String),
    AfterStageReplacement(String),
    BeforeBackup(String),
    AfterBackup(String),
    BeforeJournal(State),
    AfterJournal(State),
    /// Before the destination's identity is rechecked and the rename happens.
    BeforeReplace(String),
    /// At the mutation boundary: after the `Replacing` journal write and the
    /// first identity check, immediately before containment and identity are
    /// re-established and the rename runs. Firing an error or a filesystem
    /// change here exercises the boundary re-check.
    BeforeRename(String),
    /// After the rename, before the destination is fsynced/verified and the
    /// `Replaced` phase is journaled.
    AfterReplace(String),
    /// After the destination was verified and journaled `Replaced`.
    AfterReplaced(String),
    BeforeCommitRecord,
    AfterCommitRecord,
    /// Rollback / restore: before and after the backup is renamed over the
    /// destination (or the destination removed).
    BeforeRevert(String),
    AfterRevert(String),
    AfterReverted(String),
    RecoveryDecision(String),
}

/// Progress and fault-injection hooks. All methods have no-op defaults.
pub trait Hooks {
    /// Called at every protocol point. Returning `Err` aborts the operation
    /// at that point as if the underlying I/O had failed.
    fn event(&mut self, _event: &Event) -> io::Result<()> {
        Ok(())
    }
    /// Polled between artifacts and before every mutation.
    fn cancelled(&self) -> bool {
        false
    }
}

/// No progress reporting, no faults.
pub struct NoHooks;
impl Hooks for NoHooks {}

/// A progress line as reported to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub event: Event,
}

/// What an apply / recover / restore ended as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub txn_id: String,
    pub state: State,
    pub recovery_dir: PathBuf,
    /// Every artifact's final destination identity, in apply order.
    pub destinations: Vec<(String, Option<FileIdentity>)>,
}

#[derive(Debug, thiserror::Error)]
pub enum TxnError {
    #[error("change set: {0}")]
    ChangeSet(#[from] ChangeSetError),
    #[error("journal: {0}")]
    Journal(#[from] JournalError),
    #[error("target root {0} is not a copied-target root created by this engine (missing {COPIED_TARGET_MARKER})")]
    NotCopiedTarget(PathBuf),
    #[error(
        "target root {root} is busy: transaction {txn_id} is {state}; recover or restore it first"
    )]
    RootBusy {
        root: PathBuf,
        txn_id: String,
        state: String,
    },
    #[error("target root {0} is locked by another operation")]
    Locked(PathBuf),
    #[error("path `{path}` is not contained in the target root ({why})")]
    Escaping { path: String, why: String },
    #[error("{what} {path} is on a different volume than the target root {root}; rename would not be a single operation")]
    CrossVolume {
        what: &'static str,
        path: PathBuf,
        root: PathBuf,
    },
    #[error("artifact `{key}`: destination {path} is {actual}, expected {expected}")]
    PriorMismatch {
        key: String,
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("artifact `{key}`: replacement {path} is {actual}, expected {expected}")]
    ReplacementMismatch {
        key: String,
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("artifact `{key}`: destination parent {0} does not exist", .parent.display())]
    MissingParent { key: String, parent: PathBuf },
    #[error("artifact `{key}` at {path}: {source}")]
    ArtifactIo {
        key: String,
        path: PathBuf,
        source: io::Error,
    },
    #[error("{what}: {source}")]
    Io { what: String, source: io::Error },
    #[error("cancelled at {0:?}")]
    Cancelled(Event),
    #[error("transaction {txn_id} is {state:?}; {what} requires {required}")]
    WrongState {
        txn_id: String,
        state: State,
        what: &'static str,
        required: &'static str,
    },
    #[error(
        "transaction {txn_id}: the handle does not match the persisted journal ({why}); reload it"
    )]
    StaleHandle { txn_id: String, why: String },
    #[error("transaction {txn_id}: the supplied change set ({supplied}) is not the one recorded ({recorded})")]
    ChangeSetMismatch {
        txn_id: String,
        supplied: String,
        recorded: String,
    },
    #[error("transaction {txn_id} is not the root's active transaction ({active:?})")]
    NotActive {
        txn_id: String,
        active: Option<String>,
    },
    #[error("transaction {txn_id} is blocked: {why}; nothing was overwritten, journal and backups retained in {recovery}")]
    Blocked {
        txn_id: String,
        why: String,
        recovery: PathBuf,
    },
    #[error("transaction {txn_id}: {op} failed at `{key}` ({source}); {outcome}")]
    Failed {
        txn_id: String,
        op: &'static str,
        key: String,
        source: Box<TxnError>,
        outcome: String,
    },
    #[error("no transaction {0} under this root")]
    NoSuchTransaction(String),
    #[error("transaction {txn_id} cannot be restored yet: destination `{destination}` is owned by later transaction {later}; restore that first")]
    RestoreOutOfOrder {
        txn_id: String,
        destination: String,
        later: String,
    },
}

fn io_err(what: impl Into<String>) -> impl FnOnce(io::Error) -> TxnError {
    let what = what.into();
    move |source| TxnError::Io { what, source }
}

// ---------------------------------------------------------------- containment

/// Resolve `rel` (already-normalized, root-relative, forward slashes)
/// under the canonical `root` for use as a file path. Refuses any symlink or
/// junction on the way, a parent outside the root, a leaf that is a
/// directory or reparse point, and a parent on another volume. A missing
/// parent is `MissingParent` — every parent directory must already exist;
/// the engine never creates directories outside its own.
fn contained(root: &Path, rel: &str, key: &str) -> Result<PathBuf, TxnError> {
    let escaping = |why: String| TxnError::Escaping {
        path: rel.to_owned(),
        why,
    };
    let comps: Vec<&str> = rel.split('/').filter(|c| !c.is_empty()).collect();
    if comps.is_empty() {
        return Err(escaping("empty".into()));
    }
    let mut cur = root.to_path_buf();
    for c in &comps[..comps.len() - 1] {
        cur.push(c);
        match std::fs::symlink_metadata(&cur) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(TxnError::MissingParent {
                    key: key.to_owned(),
                    parent: cur,
                })
            }
            Err(e) => return Err(io_err(format!("inspecting {}", cur.display()))(e)),
            Ok(m) => {
                if m.file_type().is_symlink() {
                    return Err(escaping(format!(
                        "{} is a symlink or junction",
                        cur.display()
                    )));
                }
                if !m.is_dir() {
                    return Err(escaping(format!("{} is not a directory", cur.display())));
                }
            }
        }
    }
    // The resolved parent must still be inside the root (catches reparse
    // tags `is_symlink` does not classify) and on the root's volume.
    let parent_canon =
        std::fs::canonicalize(&cur).map_err(io_err(format!("resolving {}", cur.display())))?;
    if !parent_canon.starts_with(root) {
        return Err(escaping(format!(
            "parent resolves to {} outside {}",
            parent_canon.display(),
            root.display()
        )));
    }
    if !same_volume(root, &parent_canon) {
        return Err(TxnError::CrossVolume {
            what: "destination parent",
            path: parent_canon,
            root: root.to_path_buf(),
        });
    }
    let leaf = cur.join(comps[comps.len() - 1]);
    match std::fs::symlink_metadata(&leaf) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_err(format!("inspecting {}", leaf.display()))(e)),
        Ok(m) => {
            if m.file_type().is_symlink() {
                return Err(escaping(format!(
                    "{} is a symlink or junction",
                    leaf.display()
                )));
            }
            if !m.is_file() {
                return Err(escaping(format!(
                    "{} is not a regular file",
                    leaf.display()
                )));
            }
        }
    }
    Ok(leaf)
}

/// Resolve a directory the engine owns (`.tdb-txn`, a recovery directory or
/// one of its subdirectories) under the canonical `root`: no reparse point
/// anywhere on the way, canonical location inside the root, same volume.
/// `create` makes the leaf directory (the intermediate ones must exist).
fn contained_dir(root: &Path, rel: &str, create: bool) -> Result<PathBuf, TxnError> {
    let escaping = |why: String| TxnError::Escaping {
        path: rel.to_owned(),
        why,
    };
    let comps: Vec<&str> = rel.split('/').filter(|c| !c.is_empty()).collect();
    let mut cur = root.to_path_buf();
    for (i, c) in comps.iter().enumerate() {
        cur.push(c);
        let last = i + 1 == comps.len();
        match std::fs::symlink_metadata(&cur) {
            Err(e) if e.kind() == io::ErrorKind::NotFound && last && create => {
                std::fs::create_dir(&cur).map_err(io_err(format!("creating {}", cur.display())))?;
            }
            Err(e) => return Err(io_err(format!("inspecting {}", cur.display()))(e)),
            Ok(m) => {
                if m.file_type().is_symlink() {
                    return Err(escaping(format!(
                        "{} is a symlink or junction",
                        cur.display()
                    )));
                }
                if !m.is_dir() {
                    return Err(escaping(format!("{} is not a directory", cur.display())));
                }
            }
        }
    }
    let canon =
        std::fs::canonicalize(&cur).map_err(io_err(format!("resolving {}", cur.display())))?;
    if !canon.starts_with(root) {
        return Err(escaping(format!(
            "resolves to {} outside {}",
            canon.display(),
            root.display()
        )));
    }
    if !same_volume(root, &canon) {
        return Err(TxnError::CrossVolume {
            what: "engine directory",
            path: canon,
            root: root.to_path_buf(),
        });
    }
    Ok(cur)
}

fn same_volume(a: &Path, b: &Path) -> bool {
    let pa = a.components().next();
    let pb = b.components().next();
    match (pa, pb) {
        (Some(Component::Prefix(x)), Some(Component::Prefix(y))) => x == y,
        (Some(Component::RootDir), Some(Component::RootDir)) => true,
        _ => false,
    }
}

fn canonical_root(root: &Path) -> Result<PathBuf, TxnError> {
    let canon =
        std::fs::canonicalize(root).map_err(io_err(format!("resolving {}", root.display())))?;
    let marker = canon.join(COPIED_TARGET_MARKER);
    match std::fs::symlink_metadata(&marker) {
        Ok(m) if m.is_file() && !m.file_type().is_symlink() => {}
        _ => return Err(TxnError::NotCopiedTarget(root.to_path_buf())),
    }
    Ok(canon)
}

/// The `.tdb-txn` directory of a canonical root, created if absent.
fn txn_dir(root: &Path) -> Result<PathBuf, TxnError> {
    contained_dir(root, TXN_DIR, true)
}

fn recovery_dir_of(root: &Path, txn_id: &str) -> Result<PathBuf, TxnError> {
    if !valid_txn_id(txn_id) {
        return Err(TxnError::Escaping {
            path: txn_id.to_owned(),
            why: "not a transaction id".into(),
        });
    }
    let dir = txn_dir(root)?;
    contained_dir(&dir, txn_id, false).map_err(|e| match e {
        TxnError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound => {
            TxnError::NoSuchTransaction(txn_id.to_owned())
        }
        other => other,
    })
}

// ---------------------------------------------------------------- roots and locks

/// The canonical path of an existing copied-target root — the same check
/// every engine entry point performs (`canonicalize` + the marker file). A
/// consumer that must place files relative to the root (S2-C Workspace
/// staging) resolves the root through this so its containment decisions and
/// the engine's agree; anything else is `NotCopiedTarget`.
pub fn copied_target_root(root: &Path) -> Result<PathBuf, TxnError> {
    canonical_root(root)
}

/// Create a **new** copied-target root at `path` (refused if it exists) and
/// mark it. Only roots created this way accept change sets.
pub fn create_copied_target_root(path: &Path) -> Result<(), TxnError> {
    std::fs::create_dir(path).map_err(io_err(format!("creating {}", path.display())))?;
    std::fs::write(
        path.join(COPIED_TARGET_MARKER),
        b"copied target root created by tdb-txn; not a live game tree\n",
    )
    .map_err(io_err("writing the copied-target marker"))?;
    fsync_path(&path.join(COPIED_TARGET_MARKER)).map_err(io_err("fsync marker"))?;
    Ok(())
}

struct Lock {
    _file: File,
}

fn lock_root(root: &Path) -> Result<Lock, TxnError> {
    let dir = txn_dir(root)?;
    let path = contained(&dir, LOCK_FILE, "lock")?;
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(0);
    }
    match opts.open(&path) {
        Ok(f) => Ok(Lock { _file: f }),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32) => {
            Err(TxnError::Locked(root.to_path_buf()))
        }
        Err(e) => Err(io_err(format!("opening {}", path.display()))(e)),
    }
}

fn active_marker(root: &Path) -> Result<Option<String>, TxnError> {
    let p = contained(&txn_dir(root)?, ACTIVE_MARKER, "marker")?;
    match std::fs::read_to_string(&p) {
        Ok(s) => Ok(Some(s.trim().to_owned())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err(format!("reading {}", p.display()))(e)),
    }
}

fn write_small(dir: &Path, name: &str, tmp_name: &str, content: &[u8]) -> Result<(), TxnError> {
    let p = contained(dir, name, name)?;
    let tmp = contained(dir, tmp_name, tmp_name)?;
    {
        let mut f = File::create(&tmp).map_err(io_err(format!("writing {name}")))?;
        use std::io::Write;
        f.write_all(content)
            .and_then(|_| f.sync_all())
            .map_err(io_err(format!("writing {name}")))?;
    }
    std::fs::rename(&tmp, &p).map_err(io_err(format!("placing {name}")))
}

/// Set the `ACTIVE` marker to `txn_id` — only when it is absent or already
/// names `txn_id`. Another transaction's marker is never overwritten.
fn claim_active_marker(root: &Path, txn_id: &str) -> Result<(), TxnError> {
    match active_marker(root)? {
        Some(other) if other != txn_id => Err(TxnError::NotActive {
            txn_id: txn_id.to_owned(),
            active: Some(other),
        }),
        _ => write_small(
            &txn_dir(root)?,
            ACTIVE_MARKER,
            "ACTIVE.tmp",
            txn_id.as_bytes(),
        ),
    }
}

/// Clear the marker only if it names `txn_id`.
fn release_active_marker(root: &Path, txn_id: &str) -> Result<(), TxnError> {
    match active_marker(root)? {
        Some(mine) if mine == txn_id => {
            let p = contained(&txn_dir(root)?, ACTIVE_MARKER, "marker")?;
            std::fs::remove_file(&p).map_err(io_err("removing ACTIVE marker"))
        }
        _ => Ok(()),
    }
}

/// Recompute the marker from every journal: it names the lowest-sequence
/// transaction that blocks the root, or is absent.
fn settle_active_marker(root: &Path, blocking: &[(u64, String)]) -> Result<(), TxnError> {
    let dir = txn_dir(root)?;
    match blocking.iter().min() {
        Some((_, id)) => write_small(&dir, ACTIVE_MARKER, "ACTIVE.tmp", id.as_bytes()),
        None => {
            let p = contained(&dir, ACTIVE_MARKER, "marker")?;
            let _ = std::fs::remove_file(&p);
            Ok(())
        }
    }
}

/// Allocate the next per-root sequence number durably.
fn next_sequence(root: &Path) -> Result<u64, TxnError> {
    let dir = txn_dir(root)?;
    let p = contained(&dir, SEQUENCE_FILE, "sequence")?;
    let current: u64 = match std::fs::read_to_string(&p) {
        Ok(s) => s
            .trim()
            .parse()
            .map_err(|_| io_err("SEQUENCE is not a number")(io::Error::other("parse")))?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
        Err(e) => return Err(io_err("reading SEQUENCE")(e)),
    };
    let next = current
        .checked_add(1)
        .ok_or_else(|| io_err("SEQUENCE overflow")(io::Error::other("overflow")))?;
    write_small(
        &dir,
        SEQUENCE_FILE,
        "SEQUENCE.tmp",
        next.to_string().as_bytes(),
    )?;
    fsync_path(&p).map_err(io_err("fsync SEQUENCE"))?;
    Ok(next)
}

// ---------------------------------------------------------------- transaction handle

/// A prepared (or loaded) transaction: the root, its recovery directory and
/// its journal. `apply` reloads and revalidates the persisted journal under
/// the lock; a handle that no longer matches it is refused.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub txn_id: String,
    pub root: PathBuf,
    pub recovery_dir: PathBuf,
    pub journal: Journal,
}

impl Transaction {
    fn load(root: &Path, txn_id: &str) -> Result<Self, TxnError> {
        let recovery_dir = recovery_dir_of(root, txn_id)?;
        let journal = Journal::read(&recovery_dir)?;
        if journal.txn_id != txn_id {
            return Err(JournalError::Corrupt {
                path: recovery_dir.join(crate::journal::JOURNAL_FILE),
                why: format!("txn_id `{}` does not match its directory", journal.txn_id),
            }
            .into());
        }
        // Bind the journal to the persisted change set it was prepared from.
        let cs_path = contained(&recovery_dir, CHANGESET_FILE, "changeset")?;
        let cs_bytes = std::fs::read(&cs_path).map_err(|source| JournalError::Unreadable {
            path: cs_path.clone(),
            source,
        })?;
        let cs: ChangeSet =
            serde_json::from_slice(&cs_bytes).map_err(|e| JournalError::Corrupt {
                path: cs_path.clone(),
                why: e.to_string(),
            })?;
        let corrupt = |why: String| JournalError::Corrupt {
            path: cs_path.clone(),
            why,
        };
        if cs.id != cs.compute_id() || cs.id != journal.changeset_id {
            return Err(
                corrupt("change set id does not match its content or the journal".into()).into(),
            );
        }
        if cs.artifacts.len() != journal.artifacts.len() {
            return Err(corrupt("artifact count differs from the journal".into()).into());
        }
        for ja in &journal.artifacts {
            let Some(ca) = cs.artifacts.iter().find(|c| c.key == ja.key) else {
                return Err(corrupt(format!(
                    "journal artifact `{}` is not in the change set",
                    ja.key
                ))
                .into());
            };
            if ca.destination != ja.destination
                || ca.expected_prior != ja.prior
                || ca.replacement_identity != ja.replacement
            {
                return Err(corrupt(format!(
                    "journal artifact `{}` disagrees with the change set",
                    ja.key
                ))
                .into());
            }
        }
        Ok(Self {
            txn_id: txn_id.to_owned(),
            root: root.to_path_buf(),
            recovery_dir,
            journal,
        })
    }

    /// Durably record `state`. The in-memory journal advances only once the
    /// on-disk write succeeded, so a failed write leaves both in agreement.
    fn write_journal(
        &mut self,
        hooks: &mut dyn Hooks,
        state: State,
        note: String,
    ) -> Result<(), TxnError> {
        hooks
            .event(&Event::BeforeJournal(state))
            .map_err(io_err("hook"))?;
        let mut next = self.journal.clone();
        next.state = state;
        next.log.push(format!("{state:?}: {note}"));
        next.write(&self.recovery_dir).map_err(io_err(format!(
            "writing journal in {}",
            self.recovery_dir.display()
        )))?;
        self.journal = next;
        hooks
            .event(&Event::AfterJournal(state))
            .map_err(io_err("hook"))?;
        Ok(())
    }

    fn set_phase(&mut self, key: &str, phase: ArtifactPhase) -> Result<(), TxnError> {
        let state = self.journal.state;
        let Some(a) = self.journal.artifact_mut(key) else {
            return Err(TxnError::StaleHandle {
                txn_id: self.txn_id.clone(),
                why: format!("no artifact `{key}`"),
            });
        };
        a.phase = phase;
        self.journal
            .log
            .push(format!("{state:?}: `{key}` {phase:?}"));
        self.journal
            .write(&self.recovery_dir)
            .map_err(io_err("writing journal"))
    }

    /// The destination path, resolved and contained **now**.
    fn dest(&self, a: &JournalArtifact) -> Result<PathBuf, TxnError> {
        contained(&self.root, &a.destination, &a.key)
    }

    /// A recovery-directory file (`backups/…`, `staged/…`, `restore/…`),
    /// resolved and contained now.
    fn recovery_file(&self, rel: &str) -> Result<PathBuf, TxnError> {
        contained(&self.recovery_dir, rel, "recovery")
    }

    fn outcome(&self) -> Result<Outcome, TxnError> {
        let mut destinations = Vec::new();
        for a in &self.journal.artifacts {
            let p = self.dest(a)?;
            let id =
                FileIdentity::of_path_or_absent(&p).map_err(|source| TxnError::ArtifactIo {
                    key: a.key.clone(),
                    path: p.clone(),
                    source,
                })?;
            destinations.push((a.key.clone(), id));
        }
        Ok(Outcome {
            txn_id: self.txn_id.clone(),
            state: self.journal.state,
            recovery_dir: self.recovery_dir.clone(),
            destinations,
        })
    }
}

/// What is on disk at a destination, relative to the journal's record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Found {
    Prior,
    Replacement,
    /// Neither: third-party bytes, or the file vanished when it should not.
    Foreign,
}

fn inspect(dest: &Path, a: &JournalArtifact) -> Result<(Found, Option<FileIdentity>), TxnError> {
    let actual = FileIdentity::of_path_or_absent(dest).map_err(|source| TxnError::ArtifactIo {
        key: a.key.clone(),
        path: dest.to_path_buf(),
        source,
    })?;
    let found = match (&a.prior, &actual) {
        (Expected::Absent, None) => Found::Prior,
        (Expected::Present(p), Some(x)) if p == x => Found::Prior,
        (_, Some(x)) if *x == a.replacement => Found::Replacement,
        _ => Found::Foreign,
    };
    Ok((found, actual))
}

/// True when the destination holds the transaction's replacement — or the
/// prior, for an artifact whose replacement is byte-identical to its prior
/// (a re-serialization that changed nothing): such a destination is both.
fn holds_replacement(found: Found, a: &JournalArtifact) -> bool {
    match found {
        Found::Replacement => true,
        Found::Prior => matches!(&a.prior, Expected::Present(p) if *p == a.replacement),
        Found::Foreign => false,
    }
}

fn describe(id: &Option<FileIdentity>) -> String {
    match id {
        None => "absent".into(),
        Some(i) => i.to_string(),
    }
}

// ---------------------------------------------------------------- history

/// Every readable journal under the root (sequence order), plus the ids of
/// unreadable ones with the reason.
/// Readable journals in sequence order, and `(id, reason)` for unreadable ones.
type History = (Vec<Journal>, Vec<(String, String)>);

fn load_history(root: &Path) -> Result<History, TxnError> {
    let dir = txn_dir(root)?;
    let mut journals = Vec::new();
    let mut unreadable = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Ok((journals, unreadable));
    };
    let mut names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for n in names {
        match Transaction::load(root, &n) {
            Ok(t) => journals.push(t.journal),
            Err(e) => unreadable.push((n, e.to_string())),
        }
    }
    journals.sort_by_key(|j| j.sequence);
    Ok((journals, unreadable))
}

/// Does `state` currently hold ownership of the destinations it recorded?
fn holds_destinations(state: State) -> bool {
    matches!(
        state,
        State::Applying | State::Committed | State::Restoring | State::Blocked
    )
}

/// The later transaction (highest sequence above `sequence`) that currently
/// owns `destination`, if any.
fn later_owner<'a>(
    history: &'a [Journal],
    sequence: u64,
    destination: &str,
) -> Option<&'a Journal> {
    let norm = changeset::normalized_destination(destination).ok()?;
    history
        .iter()
        .filter(|j| j.sequence > sequence && holds_destinations(j.state))
        .filter(|j| j.artifact_for_destination(&norm).is_some())
        .max_by_key(|j| j.sequence)
}

// ---------------------------------------------------------------- prepare

/// Validate `cs` against the root and the disk, and record it. Touches
/// nothing outside `<root>/.tdb-txn/` (the recovery directory, the sequence
/// file and the `ACTIVE` marker).
pub fn prepare(
    root: &Path,
    cs: &ChangeSet,
    hooks: &mut dyn Hooks,
) -> Result<Transaction, TxnError> {
    let order = changeset::validate(cs)?;
    let root = canonical_root(root)?;
    let _lock = lock_root(&root)?;
    if let Some(active) = active_marker(&root)? {
        let state = Transaction::load(&root, &active)
            .map(|t| format!("{:?}", t.journal.state))
            .unwrap_or_else(|e| format!("unreadable ({e})"));
        return Err(TxnError::RootBusy {
            root,
            txn_id: active,
            state,
        });
    }
    let (history, unreadable) = load_history(&root)?;
    if let Some((id, why)) = unreadable.first() {
        return Err(TxnError::RootBusy {
            root,
            txn_id: id.clone(),
            state: format!("unreadable ({why})"),
        });
    }
    match cs.mode {
        Mode::CopiedTarget => {}
    }

    // Recheck every identity on disk before anything is recorded.
    let mut artifacts = Vec::with_capacity(order.len());
    for &i in &order {
        let a = &cs.artifacts[i];
        let dest = contained(&root, &a.destination, &a.key)?;
        let actual =
            FileIdentity::of_path_or_absent(&dest).map_err(|source| TxnError::ArtifactIo {
                key: a.key.clone(),
                path: dest.clone(),
                source,
            })?;
        let ok = match (&a.expected_prior, &actual) {
            (Expected::Absent, None) => true,
            (Expected::Present(p), Some(x)) => p == x,
            _ => false,
        };
        if !ok {
            return Err(TxnError::PriorMismatch {
                key: a.key.clone(),
                path: dest,
                expected: match &a.expected_prior {
                    Expected::Absent => "absent".into(),
                    Expected::Present(p) => p.to_string(),
                },
                actual: describe(&actual),
            });
        }
        let rep = FileIdentity::of_file(&a.replacement).map_err(|source| TxnError::ArtifactIo {
            key: a.key.clone(),
            path: a.replacement.clone(),
            source,
        })?;
        if rep != a.replacement_identity {
            return Err(TxnError::ReplacementMismatch {
                key: a.key.clone(),
                path: a.replacement.clone(),
                expected: a.replacement_identity.to_string(),
                actual: rep.to_string(),
            });
        }
        // Lineage: the latest owner whose replacement is this prior.
        let lineage_prior = match &a.expected_prior {
            Expected::Present(p) => history
                .iter()
                .rev()
                .find(|j| {
                    holds_destinations(j.state)
                        && changeset::normalized_destination(&a.destination)
                            .ok()
                            .and_then(|n| j.artifact_for_destination(&n))
                            .is_some_and(|ja| ja.replacement == *p)
                })
                .map(|j| j.txn_id.clone()),
            Expected::Absent => None,
        };
        let n = artifacts.len();
        artifacts.push(JournalArtifact {
            key: a.key.clone(),
            destination: a.destination.clone(),
            prior: a.expected_prior.clone(),
            lineage_prior,
            backup: match a.expected_prior {
                Expected::Absent => None,
                Expected::Present(_) => Some(format!("{BACKUPS_DIR}/{n:03}.bak")),
            },
            staged: format!("{STAGED_DIR}/{n:03}.new"),
            replacement: a.replacement_identity.clone(),
            phase: ArtifactPhase::Pending,
        });
    }

    // Recovery directory: sequence-ordered id, unique, never reused.
    let sequence = next_sequence(&root)?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let txn_id = format!("{sequence:06}-{millis}-{}", &cs.id[..12]);
    let txn_root = txn_dir(&root)?;
    if std::fs::symlink_metadata(txn_root.join(&txn_id)).is_ok() {
        return Err(io_err(format!(
            "recovery directory {txn_id} already exists"
        ))(io::Error::from(io::ErrorKind::AlreadyExists)));
    }
    let recovery_dir = contained_dir(&txn_root, &txn_id, true)?;
    for d in [BACKUPS_DIR, STAGED_DIR, RESTORE_DIR] {
        contained_dir(&recovery_dir, d, true)?;
    }
    let cs_path = contained(&recovery_dir, CHANGESET_FILE, "changeset")?;
    std::fs::write(
        &cs_path,
        serde_json::to_vec_pretty(cs).expect("serializable"),
    )
    .map_err(io_err("writing changeset.json"))?;
    fsync_path(&cs_path).map_err(io_err("fsync changeset.json"))?;

    let applied_unqualified = cs
        .artifacts
        .iter()
        .any(|a| !a.validation.qualification.is_qualified());
    let mut txn = Transaction {
        txn_id: txn_id.clone(),
        root: root.clone(),
        recovery_dir,
        journal: Journal {
            schema: JOURNAL_SCHEMA.to_owned(),
            txn_id,
            sequence,
            changeset_id: cs.id.clone(),
            target_root: root.display().to_string(),
            state: State::Prepared,
            applied_unqualified,
            artifacts,
            log: Vec::new(),
        },
    };
    txn.write_journal(
        hooks,
        State::Prepared,
        format!(
            "change set {} `{}` ({} artifacts; unqualified: {applied_unqualified}; sequence {sequence})",
            cs.id,
            cs.title,
            cs.artifacts.len()
        ),
    )?;
    claim_active_marker(&root, &txn.txn_id)?;
    hooks.event(&Event::Prepared).map_err(io_err("hook"))?;
    Ok(txn)
}

// ---------------------------------------------------------------- apply

/// Stage, apply and commit a prepared transaction. The persisted journal and
/// change set are reloaded under the lock and the handle must match them
/// exactly; a stale handle, a different change set, or a root whose active
/// transaction is another one is refused before anything is staged. On any
/// failure after mutation began, every destination is rolled back to its
/// prior through the journal policy before the error is returned; the
/// recovery directory is always retained.
pub fn apply(
    txn: &mut Transaction,
    cs: &ChangeSet,
    hooks: &mut dyn Hooks,
) -> Result<Outcome, TxnError> {
    let root = canonical_root(&txn.root)?;
    let _lock = lock_root(&root)?;
    // Authoritative state: the persisted journal, not the handle.
    let disk = Transaction::load(&root, &txn.txn_id)?;
    if disk.journal != txn.journal {
        return Err(TxnError::StaleHandle {
            txn_id: txn.txn_id.clone(),
            why: format!(
                "persisted state is {:?}, handle says {:?}",
                disk.journal.state, txn.journal.state
            ),
        });
    }
    if disk.journal.state != State::Prepared {
        return Err(TxnError::WrongState {
            txn_id: txn.txn_id.clone(),
            state: disk.journal.state,
            what: "apply",
            required: "Prepared",
        });
    }
    if cs.id != cs.compute_id() || cs.id != disk.journal.changeset_id {
        return Err(TxnError::ChangeSetMismatch {
            txn_id: txn.txn_id.clone(),
            supplied: cs.id.clone(),
            recorded: disk.journal.changeset_id.clone(),
        });
    }
    let recorded: ChangeSet = serde_json::from_slice(
        &std::fs::read(disk.recovery_dir.join(CHANGESET_FILE)).map_err(io_err("changeset.json"))?,
    )
    .map_err(|e| io_err("changeset.json")(io::Error::other(e.to_string())))?;
    if recorded != *cs {
        return Err(TxnError::ChangeSetMismatch {
            txn_id: txn.txn_id.clone(),
            supplied: cs.id.clone(),
            recorded: format!("{} (content differs)", recorded.id),
        });
    }
    let active = active_marker(&root)?;
    if active.as_deref() != Some(txn.txn_id.as_str()) {
        return Err(TxnError::NotActive {
            txn_id: txn.txn_id.clone(),
            active,
        });
    }
    *txn = disk;
    txn.root = root;

    // ---- stage: nothing outside the recovery directory is touched.
    if let Err(e) = stage_all(txn, cs, hooks) {
        let _ = txn.write_journal(hooks, State::Aborted, format!("staging failed: {e}"));
        let _ = release_active_marker(&txn.root, &txn.txn_id);
        return Err(TxnError::Failed {
            txn_id: txn.txn_id.clone(),
            op: "stage",
            key: failing_key(&e),
            source: Box::new(e),
            outcome: format!(
                "Aborted; nothing was changed; recovery dir {} retained",
                txn.recovery_dir.display()
            ),
        });
    }
    if hooks.cancelled() {
        txn.write_journal(hooks, State::Aborted, "cancelled before mutation".into())?;
        release_active_marker(&txn.root, &txn.txn_id)?;
        return Err(TxnError::Cancelled(Event::BeforeJournal(State::Applying)));
    }
    // ---- apply: the durable "mutation has begun" record, then each destination,
    // then the durable commit record. Any failure before the commit record is
    // on disk ends in Aborted (nothing mutated) or RolledBack (pre-images put
    // back); a failure after it leaves the committed set.
    let result = (|| -> Result<(), TxnError> {
        txn.write_journal(hooks, State::Applying, "mutation begins".into())?;
        replace_all(txn, hooks)?;
        hooks
            .event(&Event::BeforeCommitRecord)
            .map_err(io_err("hook"))?;
        txn.write_journal(
            hooks,
            State::Committed,
            "all destinations replaced and verified".into(),
        )?;
        Ok(())
    })();
    if let Err(e) = result {
        let key = failing_key(&e);
        let outcome = match txn.journal.state {
            State::Committed => {
                let _ = release_active_marker(&txn.root, &txn.txn_id);
                "Committed (the commit record is on disk); backups retained for restore".to_owned()
            }
            State::Applying => match rollback(txn, hooks) {
                Ok(state) => format!(
                    "{state:?}; recovery dir {} retained",
                    txn.recovery_dir.display()
                ),
                Err(rbe) => format!(
                    "rollback failed: {rbe}; recovery dir {} retained",
                    txn.recovery_dir.display()
                ),
            },
            _ => {
                let _ = txn.write_journal(
                    hooks,
                    State::Aborted,
                    format!("failed before mutation: {e}"),
                );
                let _ = release_active_marker(&txn.root, &txn.txn_id);
                format!(
                    "Aborted; nothing was changed; recovery dir {} retained",
                    txn.recovery_dir.display()
                )
            }
        };
        return Err(TxnError::Failed {
            txn_id: txn.txn_id.clone(),
            op: "apply",
            key,
            source: Box::new(e),
            outcome,
        });
    }
    let after = hooks
        .event(&Event::AfterCommitRecord)
        .map_err(io_err("hook"));
    release_active_marker(&txn.root, &txn.txn_id)?;
    after?;
    txn.outcome()
}

/// Abort a transaction that has not begun mutating: only from `Prepared` or
/// `Staged`, under the root lock, with the handle checked against the
/// persisted journal exactly as `apply` checks it. Journals `Aborted`,
/// releases the `ACTIVE` marker and retains the recovery directory. This is
/// the explicit discard of a prepared-but-never-applied set (a consumer whose
/// input changed after preview re-prepares instead of applying); recovery
/// would reach the same state at the next startup, but a live consumer must
/// not have to run root-wide recovery to free the root from its own
/// abandoned preparation.
pub fn abort(txn: &Transaction, hooks: &mut dyn Hooks) -> Result<Outcome, TxnError> {
    let root = canonical_root(&txn.root)?;
    let _lock = lock_root(&root)?;
    let mut disk = Transaction::load(&root, &txn.txn_id)?;
    if disk.journal != txn.journal {
        return Err(TxnError::StaleHandle {
            txn_id: txn.txn_id.clone(),
            why: format!(
                "persisted state is {:?}, handle says {:?}",
                disk.journal.state, txn.journal.state
            ),
        });
    }
    match disk.journal.state {
        State::Prepared | State::Staged => {}
        other => {
            return Err(TxnError::WrongState {
                txn_id: txn.txn_id.clone(),
                state: other,
                what: "abort",
                required: "Prepared or Staged",
            })
        }
    }
    disk.root = root;
    disk.write_journal(
        hooks,
        State::Aborted,
        "aborted by the consumer before mutation".into(),
    )?;
    release_active_marker(&disk.root, &disk.txn_id)?;
    disk.outcome()
}

fn failing_key(e: &TxnError) -> String {
    match e {
        TxnError::PriorMismatch { key, .. }
        | TxnError::ReplacementMismatch { key, .. }
        | TxnError::ArtifactIo { key, .. }
        | TxnError::MissingParent { key, .. } => key.clone(),
        TxnError::Failed { key, .. } => key.clone(),
        _ => String::new(),
    }
}

fn stage_all(txn: &mut Transaction, cs: &ChangeSet, hooks: &mut dyn Hooks) -> Result<(), TxnError> {
    for i in 0..txn.journal.artifacts.len() {
        if hooks.cancelled() {
            return Err(TxnError::Cancelled(Event::BeforeStageReplacement(
                txn.journal.artifacts[i].key.clone(),
            )));
        }
        let a = txn.journal.artifacts[i].clone();
        let Some(src) = cs.artifacts.iter().find(|c| c.key == a.key) else {
            return Err(TxnError::ChangeSetMismatch {
                txn_id: txn.txn_id.clone(),
                supplied: cs.id.clone(),
                recorded: format!("journal artifact `{}` is not in the change set", a.key),
            });
        };
        let key = a.key.clone();
        let wrap_src = |source: io::Error| TxnError::ArtifactIo {
            key: key.clone(),
            path: src.replacement.clone(),
            source,
        };
        hooks
            .event(&Event::BeforeStageReplacement(key.clone()))
            .map_err(wrap_src)?;
        let staged = txn.recovery_file(&a.staged)?;
        copy_new_verified(&src.replacement, &staged, &a.replacement).map_err(|source| {
            TxnError::ArtifactIo {
                key: key.clone(),
                path: staged.clone(),
                source,
            }
        })?;
        hooks
            .event(&Event::AfterStageReplacement(key.clone()))
            .map_err(|source| TxnError::ArtifactIo {
                key: key.clone(),
                path: staged.clone(),
                source,
            })?;
        if let (Expected::Present(prior), Some(b)) = (&a.prior, &a.backup) {
            let dest = txn.dest(&a)?;
            hooks
                .event(&Event::BeforeBackup(key.clone()))
                .map_err(|source| TxnError::ArtifactIo {
                    key: key.clone(),
                    path: dest.clone(),
                    source,
                })?;
            let backup = txn.recovery_file(b)?;
            copy_new_verified(&dest, &backup, prior).map_err(|source| TxnError::ArtifactIo {
                key: key.clone(),
                path: backup.clone(),
                source,
            })?;
            hooks
                .event(&Event::AfterBackup(key.clone()))
                .map_err(|source| TxnError::ArtifactIo {
                    key: key.clone(),
                    path: backup,
                    source,
                })?;
        }
        txn.journal.artifacts[i].phase = ArtifactPhase::Staged;
    }
    txn.write_journal(
        hooks,
        State::Staged,
        "all backups and replacements staged and fsynced".into(),
    )
}

fn replace_all(txn: &mut Transaction, hooks: &mut dyn Hooks) -> Result<(), TxnError> {
    for i in 0..txn.journal.artifacts.len() {
        let a = txn.journal.artifacts[i].clone();
        let key = a.key.clone();
        if hooks.cancelled() {
            return Err(TxnError::Cancelled(Event::BeforeReplace(key)));
        }
        // Containment is re-established at the mutation boundary.
        let dest = txn.dest(&a)?;
        let wrap = |source: io::Error| TxnError::ArtifactIo {
            key: key.clone(),
            path: dest.clone(),
            source,
        };
        hooks
            .event(&Event::BeforeReplace(key.clone()))
            .map_err(wrap)?;
        // The destination must still be exactly the prior.
        let (found, actual) = inspect(&dest, &a)?;
        if found != Found::Prior {
            return Err(TxnError::PriorMismatch {
                key: key.clone(),
                path: dest,
                expected: match &a.prior {
                    Expected::Absent => "absent".into(),
                    Expected::Present(p) => p.to_string(),
                },
                actual: describe(&actual),
            });
        }
        txn.set_phase(&key, ArtifactPhase::Replacing)?;
        hooks
            .event(&Event::BeforeRename(key.clone()))
            .map_err(wrap)?;
        // Mutation boundary. Re-establish containment (a parent swapped for a
        // junction since the check above is refused here) and re-read the
        // destination immediately before the rename, so nothing that changed
        // after the `Replacing` journal write can redirect the mutation or be
        // clobbered undetected. See the module note on the residual external-
        // concurrency window.
        let dest = txn.dest(&a)?;
        let wrap = |source: io::Error| TxnError::ArtifactIo {
            key: key.clone(),
            path: dest.clone(),
            source,
        };
        let (found, actual) = inspect(&dest, &a)?;
        if found != Found::Prior {
            return Err(TxnError::PriorMismatch {
                key: key.clone(),
                path: dest,
                expected: match &a.prior {
                    Expected::Absent => "absent".into(),
                    Expected::Present(p) => p.to_string(),
                },
                actual: describe(&actual),
            });
        }
        let staged = txn.recovery_file(&a.staged)?;
        std::fs::rename(&staged, &dest).map_err(wrap)?;
        hooks
            .event(&Event::AfterReplace(key.clone()))
            .map_err(wrap)?;
        fsync_path(&dest).map_err(wrap)?;
        let got = FileIdentity::of_file(&dest).map_err(wrap)?;
        if got != a.replacement {
            return Err(TxnError::ReplacementMismatch {
                key: key.clone(),
                path: dest,
                expected: a.replacement.to_string(),
                actual: got.to_string(),
            });
        }
        txn.set_phase(&key, ArtifactPhase::Replaced)?;
        hooks
            .event(&Event::AfterReplaced(key.clone()))
            .map_err(wrap)?;
    }
    Ok(())
}

/// Put every destination back to its prior from the backups (reverse
/// order), verifying each. Ends `RolledBack`, or `Blocked` if a destination
/// holds foreign bytes or a backup is not what the journal recorded.
fn rollback(txn: &mut Transaction, hooks: &mut dyn Hooks) -> Result<State, TxnError> {
    revert_all(txn, hooks, State::RolledBack, "rolled back to pre-images")
}

fn revert_all(
    txn: &mut Transaction,
    hooks: &mut dyn Hooks,
    done: State,
    note: &str,
) -> Result<State, TxnError> {
    // A destination holding bytes that are neither the prior nor the
    // replacement is never touched; every destination that is ours is still
    // put back, and the set ends Blocked.
    let mut conflict: Option<String> = None;
    for i in (0..txn.journal.artifacts.len()).rev() {
        let a = txn.journal.artifacts[i].clone();
        let key = a.key.clone();
        let dest = txn.dest(&a)?;
        let wrap = |source: io::Error| TxnError::ArtifactIo {
            key: key.clone(),
            path: dest.clone(),
            source,
        };
        let (found, actual) = inspect(&dest, &a)?;
        match found {
            Found::Prior => {
                if !matches!(
                    a.phase,
                    ArtifactPhase::Reverted | ArtifactPhase::Pending | ArtifactPhase::Staged
                ) {
                    txn.set_phase(&key, ArtifactPhase::Reverted)?;
                }
                continue;
            }
            Found::Foreign => {
                conflict.get_or_insert_with(|| {
                    format!(
                        "destination `{}` is {} — neither the prior nor the replacement",
                        a.destination,
                        describe(&actual)
                    )
                });
                continue;
            }
            Found::Replacement => {}
        }
        hooks
            .event(&Event::BeforeRevert(key.clone()))
            .map_err(wrap)?;
        txn.set_phase(&key, ArtifactPhase::Reverting)?;
        // Prepare the pre-image for the present case *before* the mutation
        // boundary; the boundary check (below) is the last thing before the
        // syscall that touches the destination.
        let prepared_tmp = match (&a.prior, &a.backup) {
            (Expected::Present(prior), Some(b)) => {
                let backup = txn.recovery_file(b)?;
                let tmp = txn.recovery_file(&format!("{RESTORE_DIR}/{i:03}.tmp"))?;
                let _ = std::fs::remove_file(&tmp);
                if let Err(e) = copy_new_verified(&backup, &tmp, prior) {
                    return block(
                        txn,
                        hooks,
                        format!("backup for `{}` is missing or corrupt: {e}", a.destination),
                    );
                }
                Some(tmp)
            }
            (Expected::Present(_), None) => {
                return block(
                    txn,
                    hooks,
                    format!("journal records no backup for `{}`", a.destination),
                );
            }
            (Expected::Absent, _) => None,
        };
        // Mutation boundary. Re-establish containment and re-read the
        // destination immediately before the rename/remove, after the hook,
        // the `Reverting` journal write and the backup copy. If foreign bytes
        // appeared in that interval (e.g. an external edit at `BeforeRevert`),
        // they are left in place and the set is blocked — never overwritten.
        let dest = txn.dest(&a)?;
        let wrap = |source: io::Error| TxnError::ArtifactIo {
            key: key.clone(),
            path: dest.clone(),
            source,
        };
        let (found, actual) = inspect(&dest, &a)?;
        if found != Found::Replacement {
            return block(
                txn,
                hooks,
                format!(
                    "destination `{}` is {} at the revert boundary — no longer the replacement this transaction installed; leaving it untouched",
                    a.destination,
                    describe(&actual)
                ),
            );
        }
        match (&a.prior, prepared_tmp) {
            (Expected::Absent, _) => {
                std::fs::remove_file(&dest).map_err(wrap)?;
            }
            (Expected::Present(_), Some(tmp)) => {
                std::fs::rename(&tmp, &dest).map_err(wrap)?;
                fsync_path(&dest).map_err(wrap)?;
            }
            (Expected::Present(_), None) => unreachable!("present prior prepared a tmp above"),
        }
        hooks
            .event(&Event::AfterRevert(key.clone()))
            .map_err(wrap)?;
        let (found, actual) = inspect(&dest, &a)?;
        if found != Found::Prior {
            return block(
                txn,
                hooks,
                format!(
                    "destination `{}` is {} after revert; expected the prior",
                    a.destination,
                    describe(&actual)
                ),
            );
        }
        txn.set_phase(&key, ArtifactPhase::Reverted)?;
        hooks
            .event(&Event::AfterReverted(key.clone()))
            .map_err(wrap)?;
    }
    if let Some(why) = conflict {
        return block(txn, hooks, why);
    }
    txn.write_journal(hooks, done, note.into())?;
    release_active_marker(&txn.root, &txn.txn_id)?;
    Ok(done)
}

fn block(txn: &mut Transaction, hooks: &mut dyn Hooks, why: String) -> Result<State, TxnError> {
    let _ = txn.write_journal(hooks, State::Blocked, why.clone());
    // The marker stays (or is taken if free): the root refuses new transactions.
    let _ = claim_active_marker(&txn.root, &txn.txn_id);
    Err(TxnError::Blocked {
        txn_id: txn.txn_id.clone(),
        why,
        recovery: txn.recovery_dir.clone(),
    })
}

// ---------------------------------------------------------------- recover

/// One transaction's state as seen from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub txn_id: String,
    pub sequence: u64,
    pub state: Option<State>,
    /// Set when the journal could not be read; the root is blocked.
    pub problem: Option<String>,
    /// Destinations this transaction no longer owns, with the later
    /// transaction that took them.
    pub superseded: Vec<(String, String)>,
}

/// List every transaction under the root without changing anything.
pub fn status(root: &Path) -> Result<Vec<Summary>, TxnError> {
    let root = canonical_root(root)?;
    let (history, unreadable) = load_history(&root)?;
    let mut out: Vec<Summary> = history
        .iter()
        .map(|j| Summary {
            txn_id: j.txn_id.clone(),
            sequence: j.sequence,
            state: Some(j.state),
            problem: None,
            superseded: j
                .artifacts
                .iter()
                .filter_map(|a| {
                    later_owner(&history, j.sequence, &a.destination)
                        .map(|u| (a.destination.clone(), u.txn_id.clone()))
                })
                .collect(),
        })
        .collect();
    for (id, why) in unreadable {
        out.push(Summary {
            txn_id: id,
            sequence: 0,
            state: None,
            problem: Some(why),
            superseded: vec![],
        });
    }
    Ok(out)
}

/// Resolve every non-terminal transaction under the root from its journal
/// and the destinations' on-disk identities alone, newest first.
/// Deterministic and idempotent; safe to run at every startup. A
/// transaction whose journal cannot be read blocks the root and is reported,
/// never deleted. Afterwards the `ACTIVE` marker names the oldest
/// transaction that still blocks the root, or is absent.
pub fn recover(root: &Path, hooks: &mut dyn Hooks) -> Result<Vec<Summary>, TxnError> {
    let root = canonical_root(root)?;
    let _lock = lock_root(&root)?;
    let (history, unreadable) = load_history(&root)?;
    let mut out = Vec::new();
    let mut blocking: Vec<(u64, String)> = Vec::new();
    for (id, why) in &unreadable {
        blocking.push((0, id.clone()));
        out.push(Summary {
            txn_id: id.clone(),
            sequence: 0,
            state: None,
            problem: Some(format!("unreadable journal: {why}")),
            superseded: vec![],
        });
    }
    // Newest first, so an in-progress later transaction is resolved before
    // an older one is judged against the destinations it touched.
    let mut ordered: Vec<&Journal> = history.iter().collect();
    ordered.sort_by_key(|j| std::cmp::Reverse(j.sequence));
    // Ownership reflects the resolutions made earlier in this pass.
    let mut current = history.clone();
    for j0 in ordered {
        let mut txn = Transaction::load(&root, &j0.txn_id)?;
        let mut superseded = Vec::new();
        // A transaction whose destinations cannot be resolved right now (a
        // junction appeared under one) is reported as a problem and keeps the
        // root blocked; the others are still resolved.
        let decided: Result<&str, TxnError> = (|| {
            let decision = match txn.journal.state {
                State::Prepared | State::Staged => {
                    txn.write_journal(
                        hooks,
                        State::Aborted,
                        "recovered: no committed record and mutation had not begun".into(),
                    )?;
                    release_active_marker(&root, &txn.txn_id)?;
                    "aborted (no mutation had begun)"
                }
                State::Applying => match revert_all(
                    &mut txn,
                    hooks,
                    State::RolledBack,
                    "recovered: no committed record; every destination back to its pre-image",
                ) {
                    Ok(_) => "rolled back to pre-images",
                    Err(TxnError::Blocked { .. }) => "blocked",
                    Err(e) => return Err(e),
                },
                State::Committed => {
                    let mut foreign = None;
                    for a in &txn.journal.artifacts {
                        if let Some(u) = later_owner(&current, txn.journal.sequence, &a.destination)
                        {
                            superseded.push((a.destination.clone(), u.txn_id.clone()));
                            continue;
                        }
                        let dest = txn.dest(a)?;
                        let (found, actual) = inspect(&dest, a)?;
                        if !holds_replacement(found, a) {
                            foreign = Some(format!(
                                "committed destination `{}` is {}, not the replacement",
                                a.destination,
                                describe(&actual)
                            ));
                            break;
                        }
                    }
                    match foreign {
                        None => "committed set verified intact",
                        Some(why) => match block(&mut txn, hooks, why) {
                            Err(TxnError::Blocked { .. }) | Ok(_) => "blocked",
                            Err(e) => return Err(e),
                        },
                    }
                }
                State::Restoring => match revert_all(
                    &mut txn,
                    hooks,
                    State::Restored,
                    "recovered: restore completed",
                ) {
                    Ok(_) => "restore completed",
                    Err(TxnError::Blocked { .. }) => "blocked",
                    Err(e) => return Err(e),
                },
                State::Aborted | State::RolledBack | State::Restored => "terminal; unchanged",
                State::Blocked => "blocked; unchanged",
            };
            Ok(decision)
        })();
        let decision = match decided {
            Ok(d) => d,
            Err(e) => {
                hooks
                    .event(&Event::RecoveryDecision(format!(
                        "{}: unresolved ({e})",
                        txn.txn_id
                    )))
                    .map_err(io_err("hook"))?;
                blocking.push((txn.journal.sequence, txn.txn_id.clone()));
                out.push(Summary {
                    txn_id: txn.txn_id.clone(),
                    sequence: txn.journal.sequence,
                    state: Some(txn.journal.state),
                    problem: Some(e.to_string()),
                    superseded: vec![],
                });
                continue;
            }
        };
        hooks
            .event(&Event::RecoveryDecision(format!(
                "{}: {decision}",
                txn.txn_id
            )))
            .map_err(io_err("hook"))?;
        if let Some(c) = current.iter_mut().find(|c| c.txn_id == txn.txn_id) {
            *c = txn.journal.clone();
        }
        if txn.journal.state.blocks_root() {
            blocking.push((txn.journal.sequence, txn.txn_id.clone()));
        }
        out.push(Summary {
            txn_id: txn.txn_id.clone(),
            sequence: txn.journal.sequence,
            state: Some(txn.journal.state),
            problem: None,
            superseded,
        });
    }
    settle_active_marker(&root, &blocking)?;
    out.sort_by_key(|s| (s.sequence, s.txn_id.clone()));
    Ok(out)
}

// ---------------------------------------------------------------- restore

/// Put the originals back for a committed transaction, verifying exact
/// prior identities and lengths. Refuses if any destination is owned by a
/// later transaction (`RestoreOutOfOrder`, no state change), if the root's
/// active transaction is another one, or if any destination no longer holds
/// the transaction's replacement (a later external edit → `Blocked`).
/// Idempotent: a `Restored` transaction returns its outcome; an interrupted
/// restore is completed by [`recover`] or by calling this again.
pub fn restore(root: &Path, txn_id: &str, hooks: &mut dyn Hooks) -> Result<Outcome, TxnError> {
    let root = canonical_root(root)?;
    let _lock = lock_root(&root)?;
    let mut txn = Transaction::load(&root, txn_id)?;
    match txn.journal.state {
        State::Restored => return txn.outcome(),
        State::Committed | State::Restoring => {}
        other => {
            return Err(TxnError::WrongState {
                txn_id: txn_id.to_owned(),
                state: other,
                what: "restore",
                required: "Committed",
            })
        }
    }
    if let Some(active) = active_marker(&root)? {
        if active != txn_id {
            let state = Transaction::load(&root, &active)
                .map(|t| format!("{:?}", t.journal.state))
                .unwrap_or_else(|e| format!("unreadable ({e})"));
            return Err(TxnError::RootBusy {
                root,
                txn_id: active,
                state,
            });
        }
    }
    if txn.journal.state == State::Committed {
        let (history, _) = load_history(&root)?;
        for a in &txn.journal.artifacts {
            if let Some(u) = later_owner(&history, txn.journal.sequence, &a.destination) {
                return Err(TxnError::RestoreOutOfOrder {
                    txn_id: txn_id.to_owned(),
                    destination: a.destination.clone(),
                    later: u.txn_id.clone(),
                });
            }
        }
        // Every destination must still be exactly the replacement.
        for a in &txn.journal.artifacts {
            let dest = txn.dest(a)?;
            let (found, actual) = inspect(&dest, a)?;
            if !holds_replacement(found, a) {
                let why = format!(
                    "destination `{}` is {}, not the replacement this transaction installed; refusing to clobber it",
                    a.destination,
                    describe(&actual)
                );
                return match block(&mut txn, hooks, why) {
                    Err(e) => Err(e),
                    Ok(_) => Err(TxnError::Io {
                        what: "block".into(),
                        source: io::Error::other("block returned Ok"),
                    }),
                };
            }
        }
        claim_active_marker(&root, txn_id)?;
        txn.write_journal(hooks, State::Restoring, "restore begins".into())?;
    }
    revert_all(
        &mut txn,
        hooks,
        State::Restored,
        "every destination verified back to its prior",
    )?;
    txn.outcome()
}

/// Every readable journal under the root, keyed by transaction id.
pub fn history(root: &Path) -> Result<BTreeMap<String, Journal>, TxnError> {
    let root = canonical_root(root)?;
    let (h, _) = load_history(&root)?;
    Ok(h.into_iter().map(|j| (j.txn_id.clone(), j)).collect())
}
