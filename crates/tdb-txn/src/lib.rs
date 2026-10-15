//! `tdb-txn` — the shared **copied-target** file transaction engine (S2-B).
//!
//! One change set replaces a set of destination files inside an explicitly
//! selected **target root** with validated replacement files, keeping a full
//! intact backup of every prior destination, journaling every durable
//! transition, recovering deterministically after an interruption in a fresh
//! process, and restoring the exact originals on request.
//!
//! The engine knows nothing about archives or databases: an artifact is a
//! destination path, an expected prior identity (a length + SHA-256, or
//! *absent*), a replacement file with its identity, a validation record and a
//! qualification record it carries but never resolves. Callers (the Studio
//! domain adapters) fill those in from the S2-A stager, the edit session and
//! companion files.
//!
//! ## Protocol (durable ordering)
//!
//! ```text
//! prepare   validate the change set (duplicates, aliases, dependencies,
//!           structural blockers, mode), recheck every prior and replacement
//!           identity on disk, create <root>/.tdb-txn/<txn>/ with the change
//!           set and journal (state Prepared). Nothing outside that
//!           directory is touched.
//! stage     for every artifact: preallocate + copy + fsync the replacement
//!           into the recovery dir; for every present prior: preallocate +
//!           copy + fsync a full backup; verify each copy's identity.
//!           Journal -> Staged. Still nothing outside the recovery dir.
//! apply     Journal -> Applying (the durable "mutation has begun" record).
//!           For each artifact in order: recheck the destination's identity
//!           (prior, or absent), journal phase Replacing, rename the staged
//!           replacement over the destination, fsync the destination,
//!           verify its identity, journal phase Replaced.
//!           After all: Journal -> Committed (the durable commit record).
//! recover   from the journal and the on-disk identities only:
//!             Prepared/Staged/Aborted-in-progress -> nothing was mutated;
//!               state Aborted, recovery dir retained.
//!             Applying -> every destination is inspected: prior -> leave;
//!               replacement -> put the backup back; anything else ->
//!               Blocked (third-party bytes, nothing overwritten). State
//!               RolledBack when all are prior again.
//!             Committed -> every destination must be the replacement;
//!               otherwise Blocked. Backups retained.
//!             Restoring -> continue the restore (idempotent).
//!             Restored/RolledBack/Blocked -> unchanged.
//! restore   only from Committed: every destination must still be the
//!           replacement (else Blocked); Journal -> Restoring; each backup
//!           is copied to a fresh file, fsynced and renamed over the
//!           destination (an originally absent destination is removed);
//!           verified against the prior identity; Journal -> Restored.
//! ```
//!
//! Every journal write is `write temp → fsync → rename over journal.json`.
//! Every file the engine produces is fsynced before it is renamed into place.
//!
//! ## Windows (the primary host) — what is and is not claimed
//!
//! * Replacement is `std::fs::rename` with an existing destination, which on
//!   Windows is `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`. On NTFS, same
//!   volume, this replaces the directory entry in one operation; the engine
//!   **refuses** a recovery directory on a different volume than the target
//!   (renames would degrade to copy + delete). It is **not** a claim of
//!   atomicity on every filesystem; the journal + identity check is what
//!   makes an interruption around it recoverable.
//! * `File::sync_all` (`FlushFileBuffers`) is called on every produced file
//!   before rename and on every destination after rename. Directory
//!   handles cannot be flushed on Windows; NTFS journals its own metadata.
//!   This is process-interruption safety as tested here, **not** a power-loss
//!   or hardware write-cache durability qualification.
//! * A destination held open by another process without share-delete makes
//!   the rename fail with a sharing violation; the engine reports the
//!   failing target and rolls back through the journal policy.
//! * Space is accounted for by preallocating (`set_len`) every backup and
//!   staged copy to its final length before writing; a failing preallocation
//!   is reported as the failing target before any destination is touched.
//!
//! Competing Studio operations on the same target root are blocked by an
//! exclusively opened lock file for the duration of apply/recover/restore, and
//! by an `ACTIVE` marker that names the transaction whose set is not yet in a
//! terminal state; `prepare` refuses a root with an active or blocked set.
//!
//! Containment (every path resolved and reparse points refused, at every
//! use) and transaction history (per-root sequence, destination ownership,
//! restore ordering) are described at the top of [`engine`].
//!
//! ## Three failure classes, kept distinct
//!
//! * **Process interruption** (a crash, a kill, a returned error at any
//!   point): fully handled. Recovery reconstructs a journal-named state from
//!   on-disk material and never overwrites bytes it does not own.
//! * **External concurrency** (another process editing a destination while a
//!   transaction runs): every mutation re-establishes containment and
//!   re-reads the destination's identity immediately before the rename or
//!   remove — after the hooks, the journal write and the backup copy — so an
//!   edit that appears up to that point is preserved and the set is blocked,
//!   not clobbered. A *residual* window remains between that final check and
//!   the single rename/remove syscall: a writer active in exactly that
//!   sub-syscall interval is not detected, because `std::fs::rename` cannot
//!   both exclude a concurrent writer and replace the file. This is a
//!   narrowed TOCTOU, not a race-free guarantee. On Windows the residual is
//!   itself bounded: a destination another process holds open without
//!   `FILE_SHARE_DELETE` makes our rename fail rather than clobber, and the
//!   set rolls back.
//! * **Power loss / hardware write-cache durability**: unqualified. Directory
//!   entries cannot be flushed on Windows; NTFS metadata journaling is
//!   assumed, unmeasured, and no power-loss experiment is run.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod changeset;
pub mod engine;
pub mod identity;
pub mod journal;

pub use changeset::{
    Artifact, ChangeSet, Expected, Mode, Qualification, QualificationItem, Validation,
    CHANGESET_SCHEMA,
};
pub use engine::{
    abort, apply, copied_target_root, prepare, recover, restore, status, Event, Hooks, NoHooks,
    Outcome, Progress, Transaction, TxnError,
};
pub use identity::FileIdentity;
pub use journal::{ArtifactPhase, Journal, JournalArtifact, State, JOURNAL_SCHEMA};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_r1;
#[cfg(test)]
mod tests_r2;
