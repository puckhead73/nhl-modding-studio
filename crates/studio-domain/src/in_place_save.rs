//! Coordinated in-place publication (LEAGUE-PARITY-COMPLETION-R1 F1).
//!
//! An in-place save touches up to four files — the main database, the roster save, a content
//! header sidecar and the localization dictionary — and they must move together: either every
//! destination holds the bytes the workspace planned, or every destination holds what it held
//! before and the workspace still carries its edits. This module is the small publication engine
//! [`crate::Workspace::save_in_place`] runs on top of. It is the in-place sibling of the
//! copied-target transaction in `tdb-txn`: that engine journals inside a marked copied root and
//! is deliberately refused on a live tree, so a live in-place save cannot borrow it; the same
//! two-phase shape is kept instead.
//!
//! **Preflight** ([`plan`]): every replacement is packaged in memory and every destination is read
//! and compared with the identity the workspace was loaded from (or, for the dictionary, the
//! attached baseline). Nothing is written until all of that succeeded — an externally changed or
//! unreadable destination, or a packaging error, refuses the whole save with no file touched.
//!
//! **Publish** ([`publish`]): each destination is written through a sibling temporary file and
//! renamed over the destination, then read back and verified. If any step fails, the files that
//! were already published are put back from the prior bytes the plan read, in reverse order, and
//! that rollback is verified too. The outcome distinguishes three cases the caller reports as
//! such: nothing changed (refused or fully rolled back), everything changed (committed), and the
//! explicit split state where a rollback itself failed and the outcome names exactly which
//! destination holds which bytes.
//!
//! **The recovery set is every destination whose mutation was attempted**, not only the ones that
//! verified (OVERNIGHT-DEV-R2 F1): a destination enters it the moment its rename is attempted, so a
//! rename that fails, a read-back that fails or a read-back that finds the wrong identity all
//! leave that destination inside the rollback and inside the classification. "Nothing changed" is
//! only ever reported after every attempted destination verified back to its prior bytes.
//!
//! No crash/power-loss atomicity across files is claimed, and no restart detection is claimed:
//! the prior bytes live only in this process, so a crash between two renames leaves files in
//! mixed states that nothing records durably (the `.tdb-save-tmp` sibling is the only trace).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tdb_txn::FileIdentity;

use crate::error::DomainError;
use crate::Result;

/// Suffix of the sibling temporary file a destination is written through.
pub const TEMP_SUFFIX: &str = ".tdb-save-tmp";

/// What a destination held before the save (and what a rollback puts back).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prior {
    /// The destination did not exist (a sidecar being created); rollback removes it.
    Absent,
    /// The destination held exactly these bytes; rollback rewrites them.
    Bytes(Vec<u8>),
}

/// One destination the save will replace, fully prepared: nothing about it is read from disk again
/// except to verify.
#[derive(Debug, Clone)]
pub struct PlannedWrite {
    /// Plain-language role for messages ("the main database", "the roster save", …).
    pub role: &'static str,
    pub path: PathBuf,
    pub prior: Prior,
    /// The bytes to publish.
    pub bytes: Vec<u8>,
    /// Identity of `bytes` (what the destination must verify as after the rename).
    pub identity: FileIdentity,
}

/// One destination the publication removes (R71-R3: a sidecar an installation created is taken
/// away again by its restore **inside** the same coordinated step set). The prior bytes are what a
/// rollback puts back; the destination must hold exactly them when the removal is attempted.
#[derive(Debug, Clone)]
pub struct PlannedRemoval {
    pub role: &'static str,
    pub path: PathBuf,
    /// What the destination holds now (verified before removal; rewritten by a rollback).
    pub prior: Vec<u8>,
}

/// A destination as the plan requires it to be found before mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expectation {
    /// The file must not exist.
    Absent,
    /// The file must exist and carry exactly this identity.
    Identity(FileIdentity),
    /// The file must exist and be readable; no baseline identity is known (a session built from
    /// bytes). Its bytes are still captured for rollback.
    AnyPresent,
}

/// Read one destination as the preflight sees it: the prior bytes for rollback, verified against
/// the expectation. Every refusal names the role and the path.
pub fn read_prior(role: &str, path: &Path, expectation: &Expectation) -> Result<Prior> {
    let display = path.display();
    match expectation {
        Expectation::Absent => match fs::symlink_metadata(path) {
            Ok(_) => Err(DomainError::Sync(format!(
                "{role} sidecar {display} already exists; the save planned to create it and never overwrites a file it did not plan for"
            ))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Prior::Absent),
            Err(source) => Err(DomainError::Io {
                path: display.to_string(),
                source,
            }),
        },
        Expectation::Identity(expected) => {
            let bytes = fs::read(path).map_err(|source| DomainError::Io {
                path: display.to_string(),
                source,
            })?;
            let current = FileIdentity::of_bytes(&bytes);
            if current != *expected {
                return Err(DomainError::Sync(format!(
                    "{role} {display} is {current} on disk but this workspace was opened on {expected}; it changed since it was opened — nothing was written. Reload the workspace (or re-attach the dictionary) and repeat the change"
                )));
            }
            Ok(Prior::Bytes(bytes))
        }
        Expectation::AnyPresent => {
            let bytes = fs::read(path).map_err(|source| DomainError::Io {
                path: display.to_string(),
                source,
            })?;
            Ok(Prior::Bytes(bytes))
        }
    }
}

/// Where a publication stands when [`publish`] returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every planned destination verified as its planned bytes.
    Committed,
    /// A step failed; every destination whose mutation had been attempted (the failing one
    /// included) was verified back to its prior bytes (or removed). `failed` is the destination
    /// that could not be published.
    RolledBack { failed: String, cause: String },
    /// A step failed **and** the rollback could not put every destination back. Nothing is hidden:
    /// each list names paths by their state on disk right now.
    Split {
        failed: String,
        cause: String,
        /// Destinations still holding the newly published bytes (their rollback failed).
        holding_new: Vec<String>,
        /// Destinations verified back to their prior bytes.
        restored: Vec<String>,
        /// Destinations whose state could not be established (rollback failed and the read-back
        /// failed too).
        unknown: Vec<String>,
    },
}

impl Outcome {
    /// `true` when no destination differs from before the save: only after every attempted
    /// destination — the failing one included — verified back to its prior bytes.
    pub fn nothing_changed(&self) -> bool {
        matches!(self, Outcome::RolledBack { .. })
    }
}

/// Points at which a publication can be interrupted (test fault injection and progress).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Before the temporary file for this destination is written.
    BeforeWrite(String),
    /// Immediately after the destination was renamed into place, before it is read back: the
    /// destination already holds the new bytes and is inside the recovery set.
    AfterRename(String),
    /// Before the read-back of the renamed destination (a failure here is a read-back I/O error).
    BeforeReadBack(String),
    /// After the destination was renamed into place and verified.
    AfterPublished(String),
    /// Before a destination is put back during rollback.
    BeforeRevert(String),
    /// Before a planned removal is attempted (the destination still holds its prior bytes).
    BeforeRemove(String),
    /// After a planned removal succeeded and the absence was verified.
    AfterRemoved(String),
}

/// Fault-injection / progress hooks; every method is a no-op by default.
pub trait Hooks {
    /// Return `Err` to fail the publication at this point exactly as an I/O error there would.
    fn event(&mut self, _event: &Event) -> std::io::Result<()> {
        Ok(())
    }
}

/// No progress, no faults.
pub struct NoHooks;
impl Hooks for NoHooks {}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(TEMP_SUFFIX);
    path.with_file_name(name)
}

fn io_err(path: &Path, source: std::io::Error) -> DomainError {
    DomainError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Write `bytes` to the destination's sibling temporary file (the destination itself is not
/// touched). A stale temporary *file* from an interrupted earlier save is replaced; anything else
/// at that name (a directory, a reparse point) refuses.
fn stage_temp(path: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let tmp = temp_path(path);
    match fs::symlink_metadata(&tmp) {
        Ok(m) if m.is_file() => fs::remove_file(&tmp).map_err(|e| io_err(&tmp, e))?,
        Ok(_) => {
            return Err(DomainError::Sync(format!(
                "{} is occupied by something that is not a regular file; move it away before saving",
                tmp.display()
            )))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_err(&tmp, e)),
    }
    {
        let mut f = fs::File::create(&tmp).map_err(|e| io_err(&tmp, e))?;
        f.write_all(bytes).map_err(|e| io_err(&tmp, e))?;
        f.sync_all().map_err(|e| io_err(&tmp, e))?;
    }
    Ok(tmp)
}

/// Rename the staged temporary file over the destination. From this call on the destination is
/// inside the recovery set whatever happens.
fn rename_into_place(tmp: &Path, path: &Path) -> Result<()> {
    if let Err(e) = fs::rename(tmp, path) {
        let _ = fs::remove_file(tmp);
        return Err(io_err(path, e));
    }
    Ok(())
}

/// Read the destination back and require `identity`.
fn verify(path: &Path, identity: &FileIdentity) -> Result<()> {
    let actual = FileIdentity::of_file(path).map_err(|e| io_err(path, e))?;
    if actual != *identity {
        return Err(DomainError::Sync(format!(
            "{} is {actual} after the write, not the planned {identity}",
            path.display()
        )));
    }
    Ok(())
}

/// Stage, rename and verify in one step (used by the rollback, which has no hooks between).
fn place(path: &Path, bytes: &[u8], identity: &FileIdentity) -> Result<()> {
    let tmp = stage_temp(path, bytes)?;
    rename_into_place(&tmp, path)?;
    verify(path, identity)
}

/// Put one destination back to its prior state and verify it.
fn revert(w: &PlannedWrite) -> Result<()> {
    match &w.prior {
        Prior::Absent => match fs::remove_file(&w.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_err(&w.path, e)),
        },
        Prior::Bytes(bytes) => place(&w.path, bytes, &FileIdentity::of_bytes(bytes)),
    }
}

/// Publish every planned write in order; on failure roll back every destination whose mutation
/// was attempted, in reverse. Never returns `Err`: every failure is an [`Outcome`] the caller
/// turns into its own error, so a caller cannot lose the rollback result behind a `?`.
pub fn publish(plan: &[PlannedWrite], hooks: &mut dyn Hooks) -> Outcome {
    publish_all(plan, &[], hooks)
}

/// One attempted step, for the reverse rollback and the classification.
enum Attempted<'a> {
    Write(&'a PlannedWrite),
    Remove(&'a PlannedRemoval),
}

/// Check a planned removal's destination still holds its prior bytes (before it enters the
/// recovery set: a stale plan refuses without touching anything).
fn check_removal(r: &PlannedRemoval) -> Result<()> {
    let current = FileIdentity::of_file(&r.path).map_err(|e| io_err(&r.path, e))?;
    let expected = FileIdentity::of_bytes(&r.prior);
    if current != expected {
        return Err(DomainError::Sync(format!(
            "{} is {current} on disk, not the {expected} the removal was planned on; nothing removed",
            r.path.display()
        )));
    }
    Ok(())
}

/// Remove one destination and verify the absence.
fn remove_planned(r: &PlannedRemoval) -> Result<()> {
    fs::remove_file(&r.path).map_err(|e| io_err(&r.path, e))?;
    match fs::symlink_metadata(&r.path) {
        Ok(_) => Err(DomainError::Sync(format!("{} still exists after its removal", r.path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_err(&r.path, e)),
    }
}

/// Publish every planned write, then perform every planned removal, as one step set: a failure
/// anywhere rolls back every attempted step in reverse (a removed file is rewritten from its prior
/// bytes), and the outcome classifies every attempted destination by its state on disk.
pub fn publish_all(plan: &[PlannedWrite], removals: &[PlannedRemoval], hooks: &mut dyn Hooks) -> Outcome {
    // Every destination whose mutation was attempted — verified or not — is put back and classified.
    let mut attempted: Vec<Attempted<'_>> = Vec::new();
    let mut failure: Option<(String, String)> = None;
    for w in plan {
        let label = w.path.display().to_string();
        let staged = hooks
            .event(&Event::BeforeWrite(label.clone()))
            .map_err(|e| io_err(&w.path, e))
            .and_then(|()| stage_temp(&w.path, &w.bytes));
        let tmp = match staged {
            Ok(t) => t,
            Err(e) => {
                // The destination itself was never touched: it is not in the recovery set.
                failure = Some((label, e.to_string()));
                break;
            }
        };
        attempted.push(Attempted::Write(w));
        let step = rename_into_place(&tmp, &w.path)
            .and_then(|()| {
                hooks
                    .event(&Event::AfterRename(label.clone()))
                    .map_err(|e| io_err(&w.path, e))
            })
            .and_then(|()| {
                hooks
                    .event(&Event::BeforeReadBack(label.clone()))
                    .map_err(|e| io_err(&w.path, e))
            })
            .and_then(|()| verify(&w.path, &w.identity))
            .and_then(|()| {
                hooks
                    .event(&Event::AfterPublished(label.clone()))
                    .map_err(|e| io_err(&w.path, e))
            });
        if let Err(e) = step {
            failure = Some((label, e.to_string()));
            break;
        }
    }
    if failure.is_none() {
        for r in removals {
            let label = r.path.display().to_string();
            let ready = hooks
                .event(&Event::BeforeRemove(label.clone()))
                .map_err(|e| io_err(&r.path, e))
                .and_then(|()| check_removal(r));
            if let Err(e) = ready {
                // The destination was never touched: it is not in the recovery set.
                failure = Some((label, e.to_string()));
                break;
            }
            attempted.push(Attempted::Remove(r));
            let step = remove_planned(r).and_then(|()| {
                hooks
                    .event(&Event::AfterRemoved(label.clone()))
                    .map_err(|e| io_err(&r.path, e))
            });
            if let Err(e) = step {
                failure = Some((label, e.to_string()));
                break;
            }
        }
    }
    let Some((failed, cause)) = failure else {
        return Outcome::Committed;
    };
    let mut holding_new = Vec::new();
    let mut restored = Vec::new();
    let mut unknown = Vec::new();
    for a in attempted.iter().rev() {
        match a {
            Attempted::Write(w) => {
                let label = w.path.display().to_string();
                let ok = hooks
                    .event(&Event::BeforeRevert(label.clone()))
                    .map_err(|e| io_err(&w.path, e))
                    .and_then(|()| revert(w));
                match ok {
                    Ok(()) => restored.push(label),
                    Err(_) => match (&w.prior, FileIdentity::of_path_or_absent(&w.path)) {
                        (Prior::Absent, Ok(None)) => restored.push(label),
                        (Prior::Bytes(b), Ok(Some(id))) if id == FileIdentity::of_bytes(b) => {
                            restored.push(label)
                        }
                        (_, Ok(Some(id))) if id == w.identity => holding_new.push(label),
                        _ => unknown.push(label),
                    },
                }
            }
            Attempted::Remove(r) => {
                let label = r.path.display().to_string();
                let prior_id = FileIdentity::of_bytes(&r.prior);
                let ok = hooks
                    .event(&Event::BeforeRevert(label.clone()))
                    .map_err(|e| io_err(&r.path, e))
                    .and_then(|()| match FileIdentity::of_path_or_absent(&r.path).map_err(|e| io_err(&r.path, e))? {
                        Some(id) if id == prior_id => Ok(()),
                        _ => place(&r.path, &r.prior, &prior_id),
                    });
                match ok {
                    Ok(()) => restored.push(label),
                    Err(_) => match FileIdentity::of_path_or_absent(&r.path) {
                        Ok(Some(id)) if id == prior_id => restored.push(label),
                        // Its "new" state is the absence the removal produced.
                        Ok(None) => holding_new.push(label),
                        _ => unknown.push(label),
                    },
                }
            }
        }
    }
    if holding_new.is_empty() && unknown.is_empty() {
        Outcome::RolledBack { failed, cause }
    } else {
        Outcome::Split {
            failed,
            cause,
            holding_new,
            restored,
            unknown,
        }
    }
}

impl Outcome {
    /// The plain-language account of a failed publication (the error the workspace raises).
    pub fn describe(&self, plan: &[PlannedWrite]) -> String {
        self.describe_all(plan, &[])
    }

    /// [`Outcome::describe`] for a step set with removals.
    pub fn describe_all(&self, plan: &[PlannedWrite], removals: &[PlannedRemoval]) -> String {
        let role_of = |label: &str| {
            plan.iter()
                .find(|w| w.path.display().to_string() == label)
                .map(|w| w.role)
                .or_else(|| removals.iter().find(|r| r.path.display().to_string() == label).map(|r| r.role))
                .unwrap_or("file")
        };
        match self {
            Outcome::Committed => "every file was written and verified".to_string(),
            Outcome::RolledBack { failed, cause } => format!(
                "the save could not write {} ({failed}): {cause}. Every file it had already written was put back and verified, so nothing on disk changed; the edits are still pending in this workspace",
                role_of(failed)
            ),
            Outcome::Split {
                failed,
                cause,
                holding_new,
                restored,
                unknown,
            } => {
                let list = |v: &[String]| {
                    if v.is_empty() {
                        "none".to_string()
                    } else {
                        v.iter()
                            .map(|l| format!("{} ({l})", role_of(l)))
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                };
                format!(
                    "the save could not write {} ({failed}): {cause}, and putting the already written files back also failed. State on disk now — holding the NEW bytes: {}; put back to the ORIGINAL bytes: {}; could not be read: {}. The edits are still pending in this workspace and its baseline is marked stale; reload the workspace once the files are reachable again",
                    role_of(failed),
                    list(holding_new),
                    list(restored),
                    list(unknown)
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "nhlms-inplace-{tag}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Tmp(dir)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn planned(role: &'static str, path: PathBuf, prior: Prior, bytes: &[u8]) -> PlannedWrite {
        PlannedWrite {
            role,
            path,
            prior,
            bytes: bytes.to_vec(),
            identity: FileIdentity::of_bytes(bytes),
        }
    }

    struct FailAt(Event);
    impl Hooks for FailAt {
        fn event(&mut self, e: &Event) -> std::io::Result<()> {
            if *e == self.0 {
                Err(std::io::Error::other("injected"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn read_prior_refuses_a_changed_destination_and_captures_bytes_otherwise() {
        let t = Tmp::new("prior");
        let p = t.0.join("a.db");
        fs::write(&p, b"one").unwrap();
        let id = FileIdentity::of_bytes(b"one");
        assert_eq!(
            read_prior("the main database", &p, &Expectation::Identity(id.clone())).unwrap(),
            Prior::Bytes(b"one".to_vec())
        );
        fs::write(&p, b"two").unwrap();
        let e = read_prior("the main database", &p, &Expectation::Identity(id))
            .unwrap_err()
            .to_string();
        assert!(e.contains("changed since it was opened"), "{e}");
        assert!(e.contains("nothing was written"), "{e}");
        assert_eq!(
            read_prior("the roster save", &p, &Expectation::AnyPresent).unwrap(),
            Prior::Bytes(b"two".to_vec())
        );
        let missing = t.0.join("missing");
        assert_eq!(
            read_prior("x", &missing, &Expectation::Absent).unwrap(),
            Prior::Absent
        );
        assert!(read_prior("x", &p, &Expectation::Absent).is_err());
        assert!(read_prior("x", &missing, &Expectation::AnyPresent).is_err());
    }

    #[test]
    fn publish_commits_all_or_rolls_back_all() {
        let t = Tmp::new("all");
        let a = t.0.join("a.db");
        let b = t.0.join("b.db");
        let c = t.0.join("c.header");
        fs::write(&a, b"A0").unwrap();
        fs::write(&b, b"B0").unwrap();
        let plan = vec![
            planned("the main database", a.clone(), Prior::Bytes(b"A0".into()), b"A1"),
            planned("the roster save", b.clone(), Prior::Bytes(b"B0".into()), b"B1"),
            planned("the roster save's content header", c.clone(), Prior::Absent, b"C1"),
        ];
        // Failure at the third write: the first two go back, the sidecar never appears.
        let out = publish(&plan, &mut FailAt(Event::BeforeWrite(c.display().to_string())));
        assert!(matches!(out, Outcome::RolledBack { .. }), "{out:?}");
        assert_eq!(fs::read(&a).unwrap(), b"A0");
        assert_eq!(fs::read(&b).unwrap(), b"B0");
        assert!(!c.exists());
        assert!(!temp_path(&a).exists() && !temp_path(&b).exists());
        let text = out.describe(&plan);
        assert!(text.contains("nothing on disk changed"), "{text}");
        assert!(text.contains("the roster save's content header"), "{text}");

        // Failure after the sidecar was published: it is removed again.
        let out = publish(&plan, &mut FailAt(Event::AfterPublished(c.display().to_string())));
        assert!(matches!(out, Outcome::RolledBack { .. }), "{out:?}");
        assert!(!c.exists());
        assert_eq!(fs::read(&a).unwrap(), b"A0");

        let out = publish(&plan, &mut NoHooks);
        assert_eq!(out, Outcome::Committed);
        assert_eq!(fs::read(&a).unwrap(), b"A1");
        assert_eq!(fs::read(&b).unwrap(), b"B1");
        assert_eq!(fs::read(&c).unwrap(), b"C1");
    }

    /// R71-R3: a removal is a coordinated step. When it fails, every write before it is put back;
    /// when a later step fails, the removed file is rewritten from its prior bytes; when its own
    /// rollback fails, the absence is classified as "holding new".
    #[test]
    fn removals_roll_back_with_the_writes_and_are_classified() {
        let t = Tmp::new("removals");
        let a = t.0.join("a.db");
        let h = t.0.join("x.header");
        fs::write(&a, b"A1").unwrap();
        fs::write(&h, b"H1").unwrap();
        let plan = vec![planned("the main database", a.clone(), Prior::Bytes(b"A1".into()), b"A0")];
        let removals = vec![PlannedRemoval { role: "the save index header", path: h.clone(), prior: b"H1".to_vec() }];
        // The removal itself fails: the write before it is put back, the header stays.
        let out = publish_all(&plan, &removals, &mut FailAt(Event::BeforeRemove(h.display().to_string())));
        assert!(matches!(out, Outcome::RolledBack { .. }), "{out:?}");
        assert_eq!(fs::read(&a).unwrap(), b"A1");
        assert_eq!(fs::read(&h).unwrap(), b"H1");
        let text = out.describe_all(&plan, &removals);
        assert!(text.contains("the save index header"), "{text}");
        // A failure after the removal: the removed file is rewritten from its prior bytes.
        let out = publish_all(&plan, &removals, &mut FailAt(Event::AfterRemoved(h.display().to_string())));
        assert!(matches!(out, Outcome::RolledBack { .. }), "{out:?}");
        assert_eq!(fs::read(&a).unwrap(), b"A1");
        assert_eq!(fs::read(&h).unwrap(), b"H1");
        // A removal planned on stale prior bytes refuses before removing anything.
        let stale = vec![PlannedRemoval { role: "the save index header", path: h.clone(), prior: b"H9".to_vec() }];
        let out = publish_all(&plan, &stale, &mut NoHooks);
        match &out {
            Outcome::RolledBack { cause, .. } => assert!(cause.contains("nothing removed"), "{cause}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(fs::read(&h).unwrap(), b"H1");
        // Success: the write lands and the header is gone.
        let out = publish_all(&plan, &removals, &mut NoHooks);
        assert_eq!(out, Outcome::Committed);
        assert_eq!(fs::read(&a).unwrap(), b"A0");
        assert!(!h.exists());
    }

    #[test]
    fn a_directory_at_the_temp_name_fails_that_write_and_rolls_back_the_rest() {
        let t = Tmp::new("tmpdir");
        let a = t.0.join("a.db");
        let b = t.0.join("b.db");
        fs::write(&a, b"A0").unwrap();
        fs::write(&b, b"B0").unwrap();
        fs::create_dir(temp_path(&b)).unwrap();
        let plan = vec![
            planned("the main database", a.clone(), Prior::Bytes(b"A0".into()), b"A1"),
            planned("the dictionary", b.clone(), Prior::Bytes(b"B0".into()), b"B1"),
        ];
        let out = publish(&plan, &mut NoHooks);
        match &out {
            Outcome::RolledBack { failed, .. } => assert_eq!(*failed, b.display().to_string()),
            other => panic!("{other:?}"),
        }
        assert_eq!(fs::read(&a).unwrap(), b"A0");
        assert_eq!(fs::read(&b).unwrap(), b"B0");
    }

    /// OVERNIGHT-DEV-R2 F1 (Codex's probe): a wrong expected identity enters the post-rename
    /// verification branch with the destination already holding the new bytes. It must be put
    /// back before "nothing changed" is reported.
    #[test]
    fn r2_f1_a_verification_mismatch_after_the_rename_puts_the_destination_back() {
        let t = Tmp::new("verify-mismatch");
        let a = t.0.join("a.db");
        fs::write(&a, b"OLD").unwrap();
        let mut w = planned("the main database", a.clone(), Prior::Bytes(b"OLD".into()), b"NEW");
        w.identity = FileIdentity::of_bytes(b"BAD");
        let plan = vec![w];
        let out = publish(&plan, &mut NoHooks);
        match &out {
            Outcome::RolledBack { failed, cause } => {
                assert_eq!(*failed, a.display().to_string());
                assert!(cause.contains("not the planned"), "{cause}");
            }
            other => panic!("{other:?}"),
        }
        assert!(out.nothing_changed());
        assert_eq!(fs::read(&a).unwrap(), b"OLD", "the failing destination is inside the rollback");
        assert!(!temp_path(&a).exists());
    }

    /// F1: faults immediately after the rename and at the read-back of the **second** file leave
    /// the first file restored *and* the second file restored; the outcome says nothing changed
    /// only because both verified back.
    #[test]
    fn r2_f1_faults_after_rename_and_at_read_back_of_a_second_file_roll_back_both() {
        let t = Tmp::new("after-rename");
        let a = t.0.join("a.db");
        let b = t.0.join("b.db");
        for event in [
            Event::AfterRename(b.display().to_string()),
            Event::BeforeReadBack(b.display().to_string()),
        ] {
            fs::write(&a, b"A0").unwrap();
            fs::write(&b, b"B0").unwrap();
            let plan = vec![
                planned("the main database", a.clone(), Prior::Bytes(b"A0".into()), b"A1"),
                planned("the dictionary", b.clone(), Prior::Bytes(b"B0".into()), b"B1"),
            ];
            let out = publish(&plan, &mut FailAt(event.clone()));
            match &out {
                Outcome::RolledBack { failed, .. } => assert_eq!(*failed, b.display().to_string(), "{event:?}"),
                other => panic!("{event:?}: {other:?}"),
            }
            assert_eq!(fs::read(&a).unwrap(), b"A0", "{event:?}");
            assert_eq!(fs::read(&b).unwrap(), b"B0", "{event:?}: the failing destination was renamed and must be put back");
            assert!(!temp_path(&a).exists() && !temp_path(&b).exists());
        }
    }

    /// F1: when the failing destination itself cannot be put back, it is named as holding the
    /// new bytes — never omitted from the classification.
    #[test]
    fn r2_f1_the_failing_destination_is_classified_when_its_own_rollback_fails() {
        let t = Tmp::new("attempted-split");
        let a = t.0.join("a.db");
        let b = t.0.join("b.db");
        fs::write(&a, b"A0").unwrap();
        fs::write(&b, b"B0").unwrap();
        let plan = vec![
            planned("the main database", a.clone(), Prior::Bytes(b"A0".into()), b"A1"),
            planned("the dictionary", b.clone(), Prior::Bytes(b"B0".into()), b"B1"),
        ];
        struct H(PathBuf);
        impl Hooks for H {
            fn event(&mut self, e: &Event) -> std::io::Result<()> {
                match e {
                    Event::AfterRename(l) if *l == self.0.display().to_string() => {
                        Err(std::io::Error::other("read-back device gone"))
                    }
                    Event::BeforeRevert(l) if *l == self.0.display().to_string() => {
                        fs::create_dir(temp_path(&self.0)).unwrap();
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
        }
        let out = publish(&plan, &mut H(b.clone()));
        match &out {
            Outcome::Split {
                failed,
                holding_new,
                restored,
                unknown,
                ..
            } => {
                assert_eq!(*failed, b.display().to_string());
                assert_eq!(holding_new, &[b.display().to_string()]);
                assert_eq!(restored, &[a.display().to_string()]);
                assert!(unknown.is_empty());
            }
            other => panic!("{other:?}"),
        }
        assert!(!out.nothing_changed());
        assert_eq!(fs::read(&a).unwrap(), b"A0");
        assert_eq!(fs::read(&b).unwrap(), b"B1");
        let text = out.describe(&plan);
        assert!(text.contains("holding the NEW bytes: the dictionary"), "{text}");
    }

    #[test]
    fn a_failed_rollback_is_reported_as_a_split_state_not_hidden() {
        let t = Tmp::new("split");
        let a = t.0.join("a.db");
        let b = t.0.join("b.db");
        fs::write(&a, b"A0").unwrap();
        fs::write(&b, b"B0").unwrap();
        let plan = vec![
            planned("the main database", a.clone(), Prior::Bytes(b"A0".into()), b"A1"),
            planned("the dictionary", b.clone(), Prior::Bytes(b"B0".into()), b"B1"),
        ];
        struct SplitHooks {
            b: PathBuf,
            a: PathBuf,
        }
        impl Hooks for SplitHooks {
            fn event(&mut self, e: &Event) -> std::io::Result<()> {
                match e {
                    Event::BeforeWrite(l) if *l == self.b.display().to_string() => {
                        Err(std::io::Error::other("dictionary write refused"))
                    }
                    Event::BeforeRevert(l) if *l == self.a.display().to_string() => {
                        // Make the main database's temp name unusable so its revert fails.
                        fs::create_dir(temp_path(&self.a)).unwrap();
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
        }
        let out = publish(
            &plan,
            &mut SplitHooks {
                a: a.clone(),
                b: b.clone(),
            },
        );
        match &out {
            Outcome::Split {
                holding_new,
                restored,
                unknown,
                ..
            } => {
                assert_eq!(holding_new, &[a.display().to_string()]);
                assert!(restored.is_empty() && unknown.is_empty());
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(fs::read(&a).unwrap(), b"A1");
        assert_eq!(fs::read(&b).unwrap(), b"B0");
        let text = out.describe(&plan);
        assert!(text.contains("holding the NEW bytes: the main database"), "{text}");
        assert!(text.contains("marked stale"), "{text}");
    }
}
