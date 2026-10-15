//! Same-length byte-range patches on an Xbox 360 disc image (`p1.1-xbox360-iso-member-route`).
//!
//! A title Xenia launches from its `.iso` reads its database and dictionary from inside the
//! image — loose members or raw entries of an EB archive member (measured 2026-09-18 on all
//! seven X: images: every database/dictionary entry is stored uncompressed, stored length =
//! decoded length). An edit that keeps the file length (every field edit, the dictionary
//! re-point) is therefore a write of exactly the bytes at one absolute range of the image.
//! Nothing else moves; the disc file system, the archive tables and every other member stay
//! byte-identical.
//!
//! The protocol is the small hash-bound one the deploy helpers use, at range granularity:
//!
//! 1. **Plan** ([`plan`]): resolve the located file to `[start, end)` in the image, read the range,
//!    require it to hash to the caller's expected prior identity, and require the replacement to be
//!    exactly `end - start` bytes. Nothing is written.
//! 2. **Apply** ([`apply`] / [`apply_with`]): write the record directory *first* — `prior.bin`
//!    (the range as it is), `replacement.bin` (what will be written) and `record.json` with
//!    status `writing`, all fsynced — then write the replacement over the range, fsync, read it
//!    back and require the replacement identity, then record status `applied`. The record with
//!    status `writing` is the durable statement of mutation intent: it exists before the first
//!    image byte changes, so an interrupted write is found by its record, not by memory.
//! 3. **Restore** ([`restore`]): the record is **re-read from disk first** (the durable state
//!    decides, never a stale in-memory copy); the range is read; if it already hashes to the prior
//!    identity the restore is a verified no-op; if it hashes to the replacement, the record is
//!    marked `restoring` (durable restore intent), the backup is written back and verified, and
//!    the record is marked `restored`; if it is **in transit** — every byte is either the prior or
//!    the replacement byte at that position **and the durable record says a mutation of this range
//!    is unfinished** (`writing` or `restoring`) — the same recovery runs (bounded: this image,
//!    this range, the two recorded byte strings, an open transaction). A mixture of old/new bytes
//!    on a **completed** record (`applied` / `restored`) is *not* evidence of an interrupted write
//!    (OVERNIGHT-DEV-R3 F1): it is classified `mixed` and refused like any other external change;
//!    arbitrary foreign bytes are refused with both identities named. The record is kept for
//!    inspection.
//!
//! **Failure after the mutation boundary is an outcome, not an error** (OVERNIGHT-DEV-R2 F1):
//! [`apply_with`] returns [`ApplyResult`] — `Refused` when nothing was created, otherwise the
//! record plus what happened, and on failure it has already attempted the bounded restore of the
//! range and reports the range's classified state (`prior`, `replacement`, `partial`, `unknown`).
//! A caller therefore always holds the record of an attempted write.
//!
//! What is **not** claimed: atomicity of the range write against a crash (the `writing` record
//! and the bounded restore are the recovery path, not a guarantee the write cannot be interrupted)
//! and any statement about what the game does with the bytes (runtime acceptance). Live images
//! are never touched by tests; a live apply is a user-gated runtime-session action.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tdb_txn::FileIdentity;

use crate::error::DomainError;
use crate::xiso_source::Located;
use crate::Result;

pub const RECORD_SCHEMA: &str = "studio-domain/iso-range-patch/2";
/// The previous schema: records without `replacement.bin` and without the `writing` status.
pub const RECORD_SCHEMA_V1: &str = "studio-domain/iso-range-patch/1";
pub const PRIOR_FILE: &str = "prior.bin";
pub const REPLACEMENT_FILE: &str = "replacement.bin";

/// A prepared patch: exactly what would change, verified against the image as it is now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangePatch {
    pub image: PathBuf,
    /// The disc member that carries the bytes (the archive for a packed entry).
    pub member_path: String,
    /// The file inside that member (`db/nhlng.db`), or the member itself when loose.
    pub entry_path: String,
    pub start: u64,
    pub end: u64,
    pub prior: FileIdentity,
    pub replacement: FileIdentity,
    #[doc(hidden)]
    pub replacement_bytes: Vec<u8>,
}

/// Where a recorded patch stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordStatus {
    /// Backup written, replacement not yet written (v1 records only).
    BackedUp,
    /// Backup and replacement recorded; the range write was started and has not been verified
    /// (durable mutation intent — an interrupted apply leaves this).
    Writing,
    /// Replacement written and verified.
    Applied,
    /// A restore has started writing the prior bytes back and has not verified yet (durable
    /// restore intent — an interrupted restore leaves this).
    Restoring,
    /// Prior bytes written back and verified.
    Restored,
}

impl RecordStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RecordStatus::BackedUp => "backed-up",
            RecordStatus::Writing => "writing",
            RecordStatus::Applied => "applied",
            RecordStatus::Restoring => "restoring",
            RecordStatus::Restored => "restored",
        }
    }

    /// `true` while a mutation of the range is in flight per the durable record: only then may a
    /// prior/replacement byte mixture be treated as this transaction's interrupted write.
    pub fn is_unfinished(self) -> bool {
        matches!(self, RecordStatus::Writing | RecordStatus::Restoring)
    }
}

/// The durable record of one attempted patch (its directory holds `record.json`, `prior.bin` and
/// `replacement.bin`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchRecord {
    #[serde(skip)]
    pub dir: PathBuf,
    #[serde(default = "schema_tag")]
    pub schema: String,
    pub image: PathBuf,
    pub member_path: String,
    pub entry_path: String,
    pub start: u64,
    pub end: u64,
    pub prior: FileIdentity,
    pub replacement: FileIdentity,
    pub status: RecordStatus,
}

fn schema_tag() -> String {
    RECORD_SCHEMA.to_string()
}

fn io_err(path: &Path, source: std::io::Error) -> DomainError {
    DomainError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Points at which an apply can be interrupted (test fault injection and progress).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The record directory (backup, replacement copy, `writing` record) is complete; the range
    /// write starts next.
    BeforeRangeWrite(String),
    /// The range bytes were written; the flush (`sync_all`) runs next.
    BeforeFlush(String),
    /// The range was written and flushed; the read-back runs next.
    BeforeReadBack(String),
    /// The read-back verified; the final record update (`applied`) runs next.
    BeforeFinalRecord(String),
}

/// Fault-injection / progress hooks; every method is a no-op by default.
pub trait Hooks {
    /// Return `Err` to fail the apply at this point exactly as an I/O error there would.
    fn event(&mut self, _event: &Event) -> std::io::Result<()> {
        Ok(())
    }
    /// Return `Some(n)` to make the range write of `entry` stop after `n` bytes with an error —
    /// an interrupted (partial) range write.
    fn truncate_range_write(&mut self, _entry: &str) -> Option<usize> {
        None
    }
}

/// No progress, no faults.
pub struct NoHooks;
impl Hooks for NoHooks {}

fn read_range(image: &Path, start: u64, end: u64) -> Result<Vec<u8>> {
    let mut f = File::open(image).map_err(|e| io_err(image, e))?;
    let len = f.metadata().map_err(|e| io_err(image, e))?.len();
    if end > len || start > end {
        return Err(DomainError::Validation(format!(
            "range [{start}, {end}) lies outside {} ({len} bytes)",
            image.display()
        )));
    }
    f.seek(SeekFrom::Start(start)).map_err(|e| io_err(image, e))?;
    let mut buf = vec![0u8; (end - start) as usize];
    f.read_exact(&mut buf).map_err(|e| io_err(image, e))?;
    Ok(buf)
}

/// Write `bytes` at `start` and flush. `truncate` stops the write after that many bytes with an
/// error (fault injection of an interrupted write); `flush_hook` runs between the write and the
/// flush.
fn write_range_with(
    image: &Path,
    start: u64,
    bytes: &[u8],
    truncate: Option<usize>,
    flush_hook: &mut dyn FnMut() -> std::io::Result<()>,
) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .open(image)
        .map_err(|e| io_err(image, e))?;
    f.seek(SeekFrom::Start(start)).map_err(|e| io_err(image, e))?;
    match truncate {
        Some(n) if n < bytes.len() => {
            f.write_all(&bytes[..n]).map_err(|e| io_err(image, e))?;
            let _ = f.sync_all();
            return Err(io_err(
                image,
                std::io::Error::other(format!("range write interrupted after {n} bytes")),
            ));
        }
        _ => f.write_all(bytes).map_err(|e| io_err(image, e))?,
    }
    flush_hook().map_err(|e| io_err(image, e))?;
    f.sync_all().map_err(|e| io_err(image, e))
}

fn write_range(image: &Path, start: u64, bytes: &[u8]) -> Result<()> {
    write_range_with(image, start, bytes, None, &mut || Ok(()))
}

/// Resolve `located` to its absolute range in the image and verify it against `expected_prior`
/// and the replacement's length. `expected_prior` is the identity of the bytes the workspace was
/// opened from (the decoded file); for a packed entry that is the entry's stored bytes, which are
/// raw on every measured title — a compressed entry is refused here because its stored bytes
/// would not hash to the decoded identity.
pub fn plan(
    image: &Path,
    located: &Located,
    expected_prior: &FileIdentity,
    replacement: Vec<u8>,
) -> Result<RangePatch> {
    let x = crate::xiso_source::Container::open(image)?;
    let (member_path, entry_path, start, end) = match located {
        Located::Loose { member } => {
            let (s, e) = x.member_range(member)?;
            (member.path.clone(), member.path.clone(), s, e)
        }
        Located::InArchive {
            archive,
            entry_path,
            entry_offset,
            stored_size,
            ..
        } => {
            let (s, e) = x.member_range(archive)?;
            let start = s + entry_offset;
            let end = start + *stored_size as u64;
            if end > e {
                return Err(DomainError::Validation(format!(
                    "entry {entry_path} ([{start}, {end})) extends past its archive member {} ([{s}, {e}))",
                    archive.path
                )));
            }
            (archive.path.clone(), entry_path.clone(), start, end)
        }
    };
    let len = end - start;
    if replacement.len() as u64 != len {
        return Err(DomainError::Validation(format!(
            "the replacement for {entry_path} is {} bytes but the image holds exactly {len} bytes for it; a disc-image entry cannot change length",
            replacement.len()
        )));
    }
    let current = FileIdentity::of_bytes(&read_range(image, start, end)?);
    if current != *expected_prior {
        return Err(DomainError::Validation(format!(
            "{entry_path} in {} is {current} on the image but the edit was planned on {expected_prior}; the image changed (or the entry is not stored raw) — nothing was written",
            image.display()
        )));
    }
    Ok(RangePatch {
        image: image.to_path_buf(),
        member_path,
        entry_path,
        start,
        end,
        prior: current,
        replacement: FileIdentity::of_bytes(&replacement),
        replacement_bytes: replacement,
    })
}

/// Create-or-truncate `path` with `bytes` and flush it to the device (the handle that wrote it
/// is the one flushed: on Windows a read-only handle cannot flush).
fn write_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = File::create(path).map_err(|e| io_err(path, e))?;
    f.write_all(bytes).map_err(|e| io_err(path, e))?;
    f.sync_all().map_err(|e| io_err(path, e))
}

fn write_record(r: &PatchRecord) -> Result<()> {
    let p = r.dir.join("record.json");
    let tmp = r.dir.join("record.json.tmp");
    let text = serde_json::to_string_pretty(r)
        .map_err(|e| DomainError::Validation(format!("record serialization: {e}")))?;
    write_synced(&tmp, text.as_bytes())?;
    fs::rename(&tmp, &p).map_err(|e| io_err(&p, e))
}

/// Read a record directory back (v1 and v2 records).
pub fn read_record(dir: &Path) -> Result<PatchRecord> {
    let p = dir.join("record.json");
    let text = fs::read_to_string(&p).map_err(|e| io_err(&p, e))?;
    let mut r: PatchRecord = serde_json::from_str(&text)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", p.display())))?;
    if r.schema != RECORD_SCHEMA && r.schema != RECORD_SCHEMA_V1 {
        return Err(DomainError::Validation(format!(
            "{} is a {:?} record, not {RECORD_SCHEMA}",
            p.display(),
            r.schema
        )));
    }
    r.dir = dir.to_path_buf();
    Ok(r)
}

/// What the image holds at a recorded range right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeState {
    /// The recorded prior identity.
    Prior,
    /// The recorded replacement identity.
    Replacement,
    /// Neither, but every byte is the prior or the replacement byte at its position **and** the
    /// durable record says the range's mutation is unfinished (`writing` / `restoring`): an
    /// interrupted apply or restore. Recoverable from the backup.
    Partial,
    /// A prior/replacement byte mixture on a **completed** record (`applied` / `restored`): not
    /// this transaction's interrupted write but a later external change. Refused.
    Mixed,
    /// Bytes this record does not account for (or the record lacks the data to tell).
    Unknown,
}

impl RangeState {
    pub fn as_str(self) -> &'static str {
        match self {
            RangeState::Prior => "prior",
            RangeState::Replacement => "replacement",
            RangeState::Partial => "partial",
            RangeState::Mixed => "mixed",
            RangeState::Unknown => "unknown",
        }
    }
}

/// The record's backup, verified against the recorded prior identity.
fn verified_prior_bytes(record: &PatchRecord) -> Result<Vec<u8>> {
    let backup = record.dir.join(PRIOR_FILE);
    let bytes = fs::read(&backup).map_err(|e| io_err(&backup, e))?;
    let id = FileIdentity::of_bytes(&bytes);
    if id != record.prior {
        return Err(DomainError::Sync(format!(
            "the backup {} is {id}, not the recorded prior {}; nothing written",
            backup.display(),
            record.prior
        )));
    }
    Ok(bytes)
}

/// The record's replacement copy, verified against the recorded replacement identity; `None`
/// for a v1 record that has none.
fn verified_replacement_bytes(record: &PatchRecord) -> Result<Option<Vec<u8>>> {
    let p = record.dir.join(REPLACEMENT_FILE);
    if !p.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&p).map_err(|e| io_err(&p, e))?;
    let id = FileIdentity::of_bytes(&bytes);
    if id != record.replacement {
        return Err(DomainError::Sync(format!(
            "the replacement copy {} is {id}, not the recorded replacement {}; nothing written",
            p.display(),
            record.replacement
        )));
    }
    Ok(Some(bytes))
}

/// Classify `current` against the record: prior, replacement, in transit between the two (only
/// while the record's durable status says the mutation is unfinished), a mixture on a completed
/// record, or unknown. The transit check needs both recorded byte strings. `record` must be the
/// durable record (callers re-read it; see [`durable`]).
fn classify(record: &PatchRecord, current: &[u8]) -> Result<RangeState> {
    let id = FileIdentity::of_bytes(current);
    if id == record.prior {
        return Ok(RangeState::Prior);
    }
    if id == record.replacement {
        return Ok(RangeState::Replacement);
    }
    let prior = verified_prior_bytes(record)?;
    let Some(replacement) = verified_replacement_bytes(record)? else {
        return Ok(RangeState::Unknown);
    };
    if prior.len() != current.len() || replacement.len() != current.len() {
        return Ok(RangeState::Unknown);
    }
    let in_transit = current
        .iter()
        .zip(prior.iter().zip(replacement.iter()))
        .all(|(c, (p, r))| c == p || c == r);
    Ok(if !in_transit {
        RangeState::Unknown
    } else if record.status.is_unfinished() {
        RangeState::Partial
    } else {
        RangeState::Mixed
    })
}

/// The record as it is on disk right now. Every classification and every restore decision is
/// made on this, never on the caller's copy (which may predate a completed apply or restore).
fn durable(record: &PatchRecord) -> Result<PatchRecord> {
    read_record(&record.dir)
}

/// Current state of a recorded range, without writing (the durable record decides).
pub fn state(record: &PatchRecord) -> Result<RangeState> {
    let record = durable(record)?;
    let current = read_range(&record.image, record.start, record.end)?;
    classify(&record, &current)
}

/// Current state of a recorded range as a label (`prior` / `replacement` / `partial` /
/// `unknown`), without writing.
pub fn status(record: &PatchRecord) -> Result<&'static str> {
    Ok(state(record)?.as_str())
}

/// The result of one attempted apply.
#[derive(Debug)]
pub enum ApplyResult {
    /// Nothing was created and nothing was written (the boundary re-check, the record directory
    /// or the backup refused).
    Refused(DomainError),
    /// The replacement was written and verified; the record says `applied`.
    Applied(PatchRecord),
    /// A step at or after the range write failed. The bounded restore of the range was
    /// attempted immediately; `state` is what the image holds at the range now (`Prior` when
    /// the restore verified) and `restore_error` is why it did not, when it did not.
    Failed {
        record: PatchRecord,
        cause: String,
        state: RangeState,
        restore_error: Option<String>,
    },
}

impl ApplyResult {
    /// The record, whenever one exists on disk.
    pub fn record(&self) -> Option<&PatchRecord> {
        match self {
            ApplyResult::Refused(_) => None,
            ApplyResult::Applied(r) | ApplyResult::Failed { record: r, .. } => Some(r),
        }
    }
}

/// Apply a planned patch (see the module docs): record directory first (refused when it exists —
/// a record is never overwritten), then the range write, flush, read-back verification and the
/// final record update. Every failure after the range write started is returned with the record
/// and the range's state after the immediate bounded restore.
pub fn apply_with(patch: &RangePatch, record_dir: &Path, hooks: &mut dyn Hooks) -> ApplyResult {
    match prepare(patch, record_dir) {
        Ok(record) => mutate(patch, record, hooks),
        Err(e) => ApplyResult::Refused(e),
    }
}

/// [`apply_with`] without hooks, as a `Result`: a failure after the mutation boundary is an
/// error naming the record (the caller that needs the record itself uses [`apply_with`]).
pub fn apply(patch: &RangePatch, record_dir: &Path) -> Result<PatchRecord> {
    match apply_with(patch, record_dir, &mut NoHooks) {
        ApplyResult::Refused(e) => Err(e),
        ApplyResult::Applied(r) => Ok(r),
        ApplyResult::Failed {
            record,
            cause,
            state,
            restore_error,
        } => Err(DomainError::Sync(format!(
            "writing {} to {} failed: {cause}; the range now holds {} ({}); the record is {}",
            record.entry_path,
            record.image.display(),
            state.as_str(),
            restore_error
                .map(|e| format!("the immediate restore failed: {e}"))
                .unwrap_or_else(|| "put back and verified".to_string()),
            record.dir.display()
        ))),
    }
}

/// Everything before the first image byte changes: the boundary re-check, the record directory,
/// the verified backup, the replacement copy and the `writing` record.
fn prepare(patch: &RangePatch, record_dir: &Path) -> Result<PatchRecord> {
    // Re-check the range at the mutation boundary: the plan may be stale.
    let prior_bytes = read_range(&patch.image, patch.start, patch.end)?;
    let current = FileIdentity::of_bytes(&prior_bytes);
    if current != patch.prior {
        return Err(DomainError::Validation(format!(
            "{} in {} is {current} now, not the planned prior {}; nothing was written",
            patch.entry_path,
            patch.image.display(),
            patch.prior
        )));
    }
    if patch.replacement_bytes.len() != prior_bytes.len()
        || FileIdentity::of_bytes(&patch.replacement_bytes) != patch.replacement
    {
        return Err(DomainError::Validation(format!(
            "the replacement bytes for {} no longer match the plan; nothing was written",
            patch.entry_path
        )));
    }
    fs::create_dir(record_dir).map_err(|e| DomainError::Io {
        path: record_dir.display().to_string(),
        source: std::io::Error::new(
            e.kind(),
            format!("record directory must not exist yet ({e})"),
        ),
    })?;
    let backup = record_dir.join(PRIOR_FILE);
    write_synced(&backup, &prior_bytes)?;
    let on_disk = FileIdentity::of_file(&backup).map_err(|e| io_err(&backup, e))?;
    if on_disk != patch.prior {
        return Err(DomainError::Sync(format!(
            "the backup {} is {on_disk}, not the prior {}; nothing was written to the image",
            backup.display(),
            patch.prior
        )));
    }
    let copy = record_dir.join(REPLACEMENT_FILE);
    write_synced(&copy, &patch.replacement_bytes)?;
    let on_disk = FileIdentity::of_file(&copy).map_err(|e| io_err(&copy, e))?;
    if on_disk != patch.replacement {
        return Err(DomainError::Sync(format!(
            "the replacement copy {} is {on_disk}, not {}; nothing was written to the image",
            copy.display(),
            patch.replacement
        )));
    }
    let record = PatchRecord {
        dir: record_dir.to_path_buf(),
        schema: RECORD_SCHEMA.to_string(),
        image: patch.image.clone(),
        member_path: patch.member_path.clone(),
        entry_path: patch.entry_path.clone(),
        start: patch.start,
        end: patch.end,
        prior: patch.prior.clone(),
        replacement: patch.replacement.clone(),
        status: RecordStatus::Writing,
    };
    write_record(&record)?;
    Ok(record)
}

/// The range write and everything after it. The record already says `writing`.
fn mutate(patch: &RangePatch, mut record: PatchRecord, hooks: &mut dyn Hooks) -> ApplyResult {
    let entry = patch.entry_path.clone();
    let hook = |hooks: &mut dyn Hooks, e: Event| hooks.event(&e).map_err(|e| io_err(&patch.image, e));
    let truncate = hooks.truncate_range_write(&entry);
    let step = hook(hooks, Event::BeforeRangeWrite(entry.clone()))
        .and_then(|()| {
            write_range_with(
                &patch.image,
                patch.start,
                &patch.replacement_bytes,
                truncate,
                &mut || hooks.event(&Event::BeforeFlush(entry.clone())),
            )
        })
        .and_then(|()| hook(hooks, Event::BeforeReadBack(entry.clone())))
        .and_then(|()| {
            let after = FileIdentity::of_bytes(&read_range(&patch.image, patch.start, patch.end)?);
            if after != patch.replacement {
                return Err(DomainError::Sync(format!(
                    "{} reads back as {after} after the write, not {}",
                    entry, patch.replacement
                )));
            }
            Ok(())
        })
        .and_then(|()| hook(hooks, Event::BeforeFinalRecord(entry.clone())))
        .and_then(|()| {
            record.status = RecordStatus::Applied;
            write_record(&record)
        });
    match step {
        Ok(()) => ApplyResult::Applied(record),
        Err(e) => {
            // The write was attempted: put the range back now, from the verified backup, and say
            // exactly what the range holds afterwards.
            record.status = RecordStatus::Writing;
            let (state, restore_error) = match restore(&record) {
                Ok(_) => (RangeState::Prior, None),
                Err(re) => (
                    state(&record).unwrap_or(RangeState::Unknown),
                    Some(re.to_string()),
                ),
            };
            let record = read_record(&record.dir).unwrap_or(record);
            ApplyResult::Failed {
                record,
                cause: e.to_string(),
                state,
                restore_error,
            }
        }
    }
}

/// What a restore found and did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreOutcome {
    /// The range already held the prior bytes; verified, nothing written.
    AlreadyPrior,
    /// The range held the replacement; the backup was written back and verified.
    Restored,
    /// The range held an interrupted write (in transit between prior and replacement); the
    /// backup was written back and verified.
    RestoredPartial,
}

/// Put the prior bytes back from the record's backup, idempotently. Decides on the **durable**
/// record: handles the replacement state and an interrupted apply/restore (`partial`); refuses a
/// mixture on a completed record (`mixed`) and unknown bytes. Persists `restoring` before the
/// first byte is written back so an interrupted restore is itself recoverable.
pub fn restore(record: &PatchRecord) -> Result<RestoreOutcome> {
    restore_with(record, &mut NoHooks)
}

/// [`restore`] with hooks (`truncate_range_write` cuts the write-back short: an interrupted
/// restore).
pub fn restore_with(record: &PatchRecord, hooks: &mut dyn Hooks) -> Result<RestoreOutcome> {
    let record = &durable(record)?;
    let current = read_range(&record.image, record.start, record.end)?;
    let state = classify(record, &current)?;
    let outcome = match state {
        RangeState::Prior => {
            let mut r = record.clone();
            r.status = RecordStatus::Restored;
            write_record(&r)?;
            return Ok(RestoreOutcome::AlreadyPrior);
        }
        RangeState::Replacement => RestoreOutcome::Restored,
        RangeState::Partial => RestoreOutcome::RestoredPartial,
        RangeState::Mixed => {
            return Err(DomainError::Validation(format!(
                "{} in {} is {}: a mixture of the prior and replacement bytes, but this record is {} — its write completed, so the mixture is a later change and not an interrupted write; refusing to overwrite it (the backup is intact at {}; restore by hand only after inspecting the range)",
                record.entry_path,
                record.image.display(),
                FileIdentity::of_bytes(&current),
                record.status.as_str(),
                record.dir.join(PRIOR_FILE).display()
            )));
        }
        RangeState::Unknown => {
            return Err(DomainError::Validation(format!(
                "{} in {} is {}: neither the prior {} nor the replacement {} nor an interrupted write between them; refusing to overwrite unknown bytes (the backup is intact at {})",
                record.entry_path,
                record.image.display(),
                FileIdentity::of_bytes(&current),
                record.prior,
                record.replacement,
                record.dir.join(PRIOR_FILE).display()
            )));
        }
    };
    let bytes = verified_prior_bytes(record)?;
    // Durable restore intent before the first byte goes back: an interruption from here on
    // leaves `restoring`, which authorizes the partial recovery on the next restore.
    let mut r = record.clone();
    r.status = RecordStatus::Restoring;
    write_record(&r)?;
    let truncate = hooks.truncate_range_write(&record.entry_path);
    write_range_with(&record.image, record.start, &bytes, truncate, &mut || Ok(()))?;
    let after = FileIdentity::of_bytes(&read_range(&record.image, record.start, record.end)?);
    if after != record.prior {
        return Err(DomainError::Sync(format!(
            "{} reads back as {after} after the restore, not {}",
            record.entry_path, record.prior
        )));
    }
    let mut r = record.clone();
    r.status = RecordStatus::Restored;
    write_record(&r)?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xiso_source::{locate_databases, read_located};
    use tdb_xiso::synth::{image, SynthFile};
    use tdb_xiso::Layout;

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("nhlms-isopatch-{tag}-{}", std::process::id()));
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

    fn synth_image(dir: &Path) -> (PathBuf, Vec<u8>, Vec<u8>) {
        let db = (0..4000u32).map(|i| (i % 251) as u8).collect::<Vec<u8>>();
        let dict = b"loose dictionary bytes".to_vec();
        let files = [
            SynthFile {
                path: "db/nhlng.db",
                bytes: &db,
            },
            SynthFile {
                path: "fe/loc/nhl_eng_us.db",
                bytes: &dict,
            },
            SynthFile {
                path: "default.xex",
                bytes: b"XEX2",
            },
        ];
        let p = dir.join("game.iso");
        fs::write(&p, image(Layout::Xgd2, &files, None)).unwrap();
        (p, db, dict)
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

    struct Truncate(usize);
    impl Hooks for Truncate {
        fn truncate_range_write(&mut self, _entry: &str) -> Option<usize> {
            Some(self.0)
        }
    }

    #[test]
    fn plan_apply_restore_round_trip_is_byte_exact_and_idempotent() {
        let t = Tmp::new("roundtrip");
        let dir = &t.0;
        let (iso, db, _) = synth_image(dir);
        let whole_before = fs::read(&iso).unwrap();
        let found = locate_databases(&iso).unwrap();
        let located = found.nhlng.clone().unwrap();
        let prior = FileIdentity::of_bytes(&db);
        let mut edited = db.clone();
        edited[100] ^= 0xFF;
        edited[3999] = 0x42;
        let patch = plan(&iso, &located, &prior, edited.clone()).unwrap();
        assert_eq!(patch.end - patch.start, 4000);

        let rec_dir = dir.join("rec1");
        let rec = apply(&patch, &rec_dir).unwrap();
        assert_eq!(rec.status, RecordStatus::Applied);
        assert_eq!(read_located(&iso, &located).unwrap(), edited);
        assert_eq!(status(&rec).unwrap(), "replacement");
        // Everything outside the range is untouched.
        let whole_after = fs::read(&iso).unwrap();
        assert_eq!(whole_after.len(), whole_before.len());
        assert_eq!(&whole_after[..patch.start as usize], &whole_before[..patch.start as usize]);
        assert_eq!(&whole_after[patch.end as usize..], &whole_before[patch.end as usize..]);
        // The record round-trips; the backup is the prior bytes, the copy the replacement.
        let back = read_record(&rec_dir).unwrap();
        assert_eq!(back, rec);
        assert_eq!(fs::read(rec_dir.join(PRIOR_FILE)).unwrap(), db);
        assert_eq!(fs::read(rec_dir.join(REPLACEMENT_FILE)).unwrap(), edited);

        assert_eq!(restore(&back).unwrap(), RestoreOutcome::Restored);
        assert_eq!(fs::read(&iso).unwrap(), whole_before, "image byte-identical after restore");
        assert_eq!(restore(&back).unwrap(), RestoreOutcome::AlreadyPrior);
        assert_eq!(read_record(&rec_dir).unwrap().status, RecordStatus::Restored);
        // A second apply refuses to reuse the record directory.
        let err = apply(&patch, &rec_dir).unwrap_err().to_string();
        assert!(err.contains("must not exist"), "{err}");
    }

    #[test]
    fn plan_refuses_a_changed_range_a_wrong_length_and_apply_rechecks_at_the_boundary() {
        let t = Tmp::new("refuse");
        let dir = &t.0;
        let (iso, db, _) = synth_image(dir);
        let found = locate_databases(&iso).unwrap();
        let located = found.nhlng.clone().unwrap();
        let prior = FileIdentity::of_bytes(&db);
        let err = plan(&iso, &located, &prior, vec![0u8; 3999]).unwrap_err().to_string();
        assert!(err.contains("cannot change length"), "{err}");
        let wrong = FileIdentity::of_bytes(b"other");
        let err = plan(&iso, &located, &wrong, db.clone()).unwrap_err().to_string();
        assert!(err.contains("nothing was written"), "{err}");
        // A valid plan, then the image changes before apply: refused at the boundary.
        let patch = plan(&iso, &located, &prior, db.iter().map(|b| b ^ 1).collect()).unwrap();
        write_range(&iso, patch.start, &vec![9u8; 10]).unwrap();
        let rec_dir = dir.join("rec");
        match apply_with(&patch, &rec_dir, &mut NoHooks) {
            ApplyResult::Refused(e) => assert!(e.to_string().contains("nothing was written"), "{e}"),
            other => panic!("{other:?}"),
        }
        assert!(!rec_dir.exists(), "no record for a refused apply");
    }

    #[test]
    fn restore_refuses_unknown_bytes_and_keeps_the_backup() {
        let t = Tmp::new("unknown");
        let dir = &t.0;
        let (iso, db, _) = synth_image(dir);
        let found = locate_databases(&iso).unwrap();
        let located = found.nhlng.clone().unwrap();
        let patch = plan(&iso, &located, &FileIdentity::of_bytes(&db), db.iter().map(|b| b ^ 1).collect()).unwrap();
        let rec = apply(&patch, &dir.join("rec")).unwrap();
        // Someone else writes bytes that are neither prior nor replacement into the range.
        write_range(&iso, rec.start + 5, &[0xEE; 3]).unwrap();
        assert_eq!(status(&rec).unwrap(), "unknown");
        let err = restore(&rec).unwrap_err().to_string();
        assert!(err.contains("neither the prior"), "{err}");
        assert!(err.contains("backup is intact"), "{err}");
        assert_eq!(fs::read(rec.dir.join(PRIOR_FILE)).unwrap(), db);
        assert_eq!(read_record(&rec.dir).unwrap().status, RecordStatus::Applied);
    }

    /// R2 F1: an interrupted range write (a prefix of the replacement over the prior) is found
    /// by the record written before the first byte changed, classified `partial`, restored from
    /// the verified backup by the apply itself, and the image ends byte-identical.
    #[test]
    fn r2_f1_an_interrupted_range_write_is_recorded_classified_and_restored() {
        let t = Tmp::new("partial");
        let dir = &t.0;
        let (iso, db, _) = synth_image(dir);
        let before = fs::read(&iso).unwrap();
        let found = locate_databases(&iso).unwrap();
        let located = found.nhlng.clone().unwrap();
        let patch = plan(&iso, &located, &FileIdentity::of_bytes(&db), db.iter().map(|b| b ^ 1).collect()).unwrap();
        let rec_dir = dir.join("rec");
        match apply_with(&patch, &rec_dir, &mut Truncate(1234)) {
            ApplyResult::Failed {
                record,
                cause,
                state,
                restore_error,
            } => {
                assert!(cause.contains("interrupted after 1234 bytes"), "{cause}");
                assert_eq!(state, RangeState::Prior, "the immediate restore verified");
                assert!(restore_error.is_none(), "{restore_error:?}");
                assert_eq!(record.status, RecordStatus::Restored);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(fs::read(&iso).unwrap(), before, "byte-identical after the bounded restore");

        // The same interruption with the immediate restore *not* run (a crash): the durable
        // `writing` record classifies the range as partial and restore recovers it.
        let rec2 = dir.join("rec2");
        let record = prepare(&patch, &rec2).unwrap();
        assert_eq!(read_record(&rec2).unwrap().status, RecordStatus::Writing, "intent recorded before the write");
        let _ = write_range_with(&iso, patch.start, &patch.replacement_bytes, Some(777), &mut || Ok(()));
        assert_eq!(state(&record).unwrap(), RangeState::Partial);
        assert_eq!(status(&record).unwrap(), "partial");
        assert_eq!(restore(&record).unwrap(), RestoreOutcome::RestoredPartial);
        assert_eq!(fs::read(&iso).unwrap(), before);
        assert_eq!(read_record(&rec2).unwrap().status, RecordStatus::Restored);
        // The same mixture again, now that the record is complete: not this transaction's write.
        let _ = write_range_with(&iso, patch.start, &patch.replacement_bytes, Some(777), &mut || Ok(()));
        assert_eq!(state(&record).unwrap(), RangeState::Mixed, "completed record: a mixture is external");
        assert!(restore(&record).unwrap_err().to_string().contains("its write completed"));
        write_range(&iso, patch.start, &db).unwrap();
        assert_eq!(fs::read(&iso).unwrap(), before);

        // Bytes that are not from either string at their position are still refused: the same
        // partial write plus one foreign byte.
        let rec3 = dir.join("rec3");
        let record = prepare(&patch, &rec3).unwrap();
        let _ = write_range_with(&iso, patch.start, &patch.replacement_bytes, Some(777), &mut || Ok(()));
        let foreign = [db[2000] ^ 0x55];
        write_range(&iso, patch.start + 2000, &foreign).unwrap();
        assert_eq!(state(&record).unwrap(), RangeState::Unknown);
        assert!(restore(&record).unwrap_err().to_string().contains("refusing to overwrite unknown bytes"));
        // Put it right by hand for the temp dir's sake; the refusal is the assertion.
        write_range(&iso, patch.start, &db).unwrap();
        assert_eq!(fs::read(&iso).unwrap(), before);
    }

    /// R3 F1 (Codex's reproduction): after a successful apply *and* restore, a later independent
    /// edit that happens to mix prior and replacement bytes must not be "recovered" — the record
    /// is complete, so the mixture is external. The same after a successful apply alone.
    #[test]
    fn r3_f1_a_completed_record_never_authorizes_partial_recovery() {
        let t = Tmp::new("completed");
        let dir = &t.0;
        let (iso, db, _) = synth_image(dir);
        let found = locate_databases(&iso).unwrap();
        let located = found.nhlng.clone().unwrap();
        let replacement: Vec<u8> = db.iter().map(|b| b ^ 1).collect();
        let patch = plan(&iso, &located, &FileIdentity::of_bytes(&db), replacement.clone()).unwrap();
        let rec = apply(&patch, &dir.join("rec")).unwrap();
        assert_eq!(restore(&rec).unwrap(), RestoreOutcome::Restored);
        assert_eq!(read_record(&rec.dir).unwrap().status, RecordStatus::Restored);
        // Later independent edit: the first byte becomes the replacement byte (`BAAA`).
        let later = [replacement[0]];
        write_range(&iso, patch.start, &later).unwrap();
        let mixed = fs::read(&iso).unwrap();
        assert_eq!(state(&rec).unwrap(), RangeState::Mixed);
        assert_eq!(status(&rec).unwrap(), "mixed");
        let err = restore(&rec).unwrap_err().to_string();
        assert!(err.contains("its write completed"), "{err}");
        assert_eq!(fs::read(&iso).unwrap(), mixed, "the later edit is untouched");
        assert_eq!(read_record(&rec.dir).unwrap().status, RecordStatus::Restored, "the record stays complete");

        // After a successful apply (status `applied`) an external mixture is refused the same way.
        write_range(&iso, patch.start, &db).unwrap();
        let rec2 = apply(&patch, &dir.join("rec2")).unwrap();
        assert_eq!(rec2.status, RecordStatus::Applied);
        write_range(&iso, patch.start + 10, &db[10..20]).unwrap();
        let mixed = fs::read(&iso).unwrap();
        assert_eq!(state(&rec2).unwrap(), RangeState::Mixed);
        assert!(restore(&rec2).unwrap_err().to_string().contains("its write completed"));
        assert_eq!(fs::read(&iso).unwrap(), mixed);
        assert_eq!(read_record(&rec2.dir).unwrap().status, RecordStatus::Applied);
        // Put the range back to prior by hand for the temp image (not a recovery claim).
        write_range(&iso, patch.start, &db).unwrap();
    }

    /// R3 F1: a caller holding a stale `Writing` record object after the durable record completed
    /// gets the durable decision — the mixture is refused, nothing written.
    #[test]
    fn r3_f1_a_stale_in_memory_writing_record_does_not_authorize_recovery() {
        let t = Tmp::new("stale");
        let dir = &t.0;
        let (iso, db, _) = synth_image(dir);
        let found = locate_databases(&iso).unwrap();
        let located = found.nhlng.clone().unwrap();
        let replacement: Vec<u8> = db.iter().map(|b| b ^ 1).collect();
        let patch = plan(&iso, &located, &FileIdentity::of_bytes(&db), replacement.clone()).unwrap();
        let rec_dir = dir.join("rec");
        // `prepare` hands out a `Writing` record; the apply then completes and is restored.
        let stale = prepare(&patch, &rec_dir).unwrap();
        assert_eq!(stale.status, RecordStatus::Writing);
        let done = mutate(&patch, stale.clone(), &mut NoHooks);
        assert!(matches!(done, ApplyResult::Applied(_)));
        assert_eq!(restore(&stale).unwrap(), RestoreOutcome::Restored, "durable `applied` → restored");
        assert_eq!(read_record(&rec_dir).unwrap().status, RecordStatus::Restored);
        write_range(&iso, patch.start, &replacement[..3]).unwrap();
        let mixed = fs::read(&iso).unwrap();
        assert_eq!(state(&stale).unwrap(), RangeState::Mixed, "the stale object still says writing; disk says restored");
        assert!(restore(&stale).unwrap_err().to_string().contains("its write completed"));
        assert_eq!(fs::read(&iso).unwrap(), mixed);
        write_range(&iso, patch.start, &db).unwrap();
    }

    /// R3 F1: a restore that is interrupted while writing the prior bytes back leaves a durable
    /// `restoring` record and a mixture; the next restore recovers it (and only then).
    #[test]
    fn r3_f1_an_interrupted_restore_is_recorded_and_recoverable() {
        let t = Tmp::new("restoring");
        let dir = &t.0;
        let (iso, db, _) = synth_image(dir);
        let before = fs::read(&iso).unwrap();
        let found = locate_databases(&iso).unwrap();
        let located = found.nhlng.clone().unwrap();
        let patch = plan(&iso, &located, &FileIdentity::of_bytes(&db), db.iter().map(|b| b ^ 1).collect()).unwrap();
        let rec = apply(&patch, &dir.join("rec")).unwrap();
        let err = restore_with(&rec, &mut Truncate(900)).unwrap_err().to_string();
        assert!(err.contains("interrupted after 900 bytes"), "{err}");
        assert_eq!(read_record(&rec.dir).unwrap().status, RecordStatus::Restoring, "durable restore intent");
        assert_eq!(state(&rec).unwrap(), RangeState::Partial);
        assert_eq!(restore(&rec).unwrap(), RestoreOutcome::RestoredPartial);
        assert_eq!(fs::read(&iso).unwrap(), before);
        assert_eq!(read_record(&rec.dir).unwrap().status, RecordStatus::Restored);
        assert_eq!(restore(&rec).unwrap(), RestoreOutcome::AlreadyPrior);
    }

    /// R2 F1: faults at the flush, the read-back and the final record update all leave the range
    /// attempted; each is returned with its record and the range verified back to prior.
    #[test]
    fn r2_f1_faults_at_flush_read_back_and_final_record_return_the_record_and_restore() {
        let t = Tmp::new("faults");
        let dir = &t.0;
        let (iso, db, _) = synth_image(dir);
        let before = fs::read(&iso).unwrap();
        let found = locate_databases(&iso).unwrap();
        let located = found.nhlng.clone().unwrap();
        let entry = "db/nhlng.db".to_string();
        for (i, event) in [
            Event::BeforeFlush(entry.clone()),
            Event::BeforeReadBack(entry.clone()),
            Event::BeforeFinalRecord(entry.clone()),
        ]
        .into_iter()
        .enumerate()
        {
            let patch = plan(&iso, &located, &FileIdentity::of_bytes(&db), db.iter().map(|b| b ^ 1).collect()).unwrap();
            let rec_dir = dir.join(format!("rec{i}"));
            match apply_with(&patch, &rec_dir, &mut FailAt(event.clone())) {
                ApplyResult::Failed {
                    record,
                    cause,
                    state,
                    restore_error,
                } => {
                    assert!(cause.contains("injected"), "{event:?}: {cause}");
                    assert_eq!(state, RangeState::Prior, "{event:?}");
                    assert!(restore_error.is_none(), "{event:?}: {restore_error:?}");
                    assert_eq!(record.status, RecordStatus::Restored, "{event:?}");
                    assert_eq!(record.dir, rec_dir);
                }
                other => panic!("{event:?}: {other:?}"),
            }
            assert_eq!(fs::read(&iso).unwrap(), before, "{event:?}");
        }
        // A fault before the range write: the record exists (`writing`), the range never changed,
        // and the outcome still carries the record.
        let patch = plan(&iso, &located, &FileIdentity::of_bytes(&db), db.iter().map(|b| b ^ 1).collect()).unwrap();
        match apply_with(&patch, &dir.join("rec-pre"), &mut FailAt(Event::BeforeRangeWrite(entry))) {
            ApplyResult::Failed { state, record, .. } => {
                assert_eq!(state, RangeState::Prior);
                assert_eq!(record.status, RecordStatus::Restored);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(fs::read(&iso).unwrap(), before);
    }

    #[test]
    fn a_packed_entry_patches_only_its_own_bytes_inside_the_archive_member() {
        let t = Tmp::new("packed");
        let dir = &t.0;
        // A BIGF archive member carrying the database; the range is the entry inside it.
        let db = vec![5u8; 2048];
        let mut b = tdb_archive_big::ArchiveBuilder::new();
        b.add_entry("fe/loc/nhl_eng_us.db", b"DICT".to_vec());
        b.add_entry("db/nhlng.db", db.clone());
        b.add_entry("zz/tail.bin", vec![1u8; 100]);
        let big = b.build().unwrap();
        let files = [SynthFile {
            path: "cacheboot.big",
            bytes: &big,
        }];
        let iso = dir.join("packed.iso");
        fs::write(&iso, image(Layout::Xgd3, &files, None)).unwrap();
        let before = fs::read(&iso).unwrap();
        let found = locate_databases(&iso).unwrap();
        let located = found.nhlng.clone().unwrap();
        let patch = plan(&iso, &located, &FileIdentity::of_bytes(&db), vec![6u8; 2048]).unwrap();
        assert_eq!(patch.member_path, "cacheboot.big");
        assert_eq!(patch.entry_path, "db/nhlng.db");
        let rec = apply(&patch, &dir.join("rec")).unwrap();
        assert_eq!(read_located(&iso, &located).unwrap(), vec![6u8; 2048]);
        // The archive still parses and its other entries are untouched.
        let arc_member = tdb_xiso::Xiso::open(&iso).unwrap();
        let m = arc_member.member("cacheboot.big").unwrap().clone();
        let arc = tdb_archive_big::Archive::parse(arc_member.read_member(&m).unwrap()).unwrap();
        assert_eq!(arc.read_entry("fe/loc/nhl_eng_us.db").unwrap(), b"DICT");
        assert_eq!(arc.read_entry("zz/tail.bin").unwrap(), vec![1u8; 100]);
        let after = fs::read(&iso).unwrap();
        assert_eq!(after.len(), before.len());
        let differing = after.iter().zip(&before).filter(|(a, b)| a != b).count();
        assert_eq!(differing, 2048, "exactly the entry's bytes differ");
        restore(&rec).unwrap();
        assert_eq!(fs::read(&iso).unwrap(), before);
    }
}
