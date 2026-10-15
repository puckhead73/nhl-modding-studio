//! S2-A — replacement planning and streamed, validated **staged** output for
//! `EB\0\3` archives with 16-byte file-table rows (the PS3 NHL Legacy
//! `patch.big` layout; the same row variant is on the Xbox 360 Legacy render
//! archives).
//!
//! This is one building block of the C3a archive transaction: it turns a set
//! of explicit entry replacements into an auditable plan, streams a **new**
//! file from the source without ever writing to the source, and verifies the
//! result against the plan. It does **not** install anything; durable apply
//! and restore belong to the next package.
//!
//! ## The three steps
//!
//! 1. [`BoundSource::bind`] reads the source once: whole-file SHA-256, header
//!    and tables, every entry's span and stored-bytes SHA-256, and the layout
//!    facts the plan depends on. Anything the layout does not establish is a
//!    [`LayoutReport`] refusal (the plan cannot be made) or a deployment
//!    blocker (the plan can be staged, but the output is not claimed
//!    deployable).
//! 2. [`plan`] resolves each [`Replacement`] against the bound source and the
//!    staged state built up by earlier replacements: **no-op** (identical
//!    bytes), **fit-in-slot** (the new payload fits the entry's own slot —
//!    its span plus the zero padding up to the next 16-byte line, never into
//!    a neighbour), or **append-and-retarget** (the new payload is appended
//!    at the next 16-byte line after the staged end of file and the row's
//!    `block`/`size` words point at it; the old bytes stay where they were).
//!    Nothing else moves. Every offset is checked `u64` arithmetic and every
//!    row word is checked for on-disk representability before any output
//!    exists.
//! 3. [`stage`] streams the output from the source with the planned
//!    substitutions, hashing both as it goes and refusing to finish if the
//!    source no longer matches the bound identity. [`verify`] then re-parses
//!    the output, decodes every changed entry and compares it with the
//!    expected decoded content, and compares every untouched byte of the
//!    original span with the source. A successful decode alone is never
//!    treated as correctness.
//!
//! ## Layout facts this module relies on (measured; see the S2-A receipt)
//!
//! * `EB\0\3` version `0x0000_0400`, 16-byte rows `[block][word_4][size][path_hash]`.
//! * `word_4` is `0` on every observed row; a nonzero value is unexplained.
//! * The `+12` word is the **djb2 hash of the entry's full path** on every
//!   observed row (16 541 rows across three archives). A replacement keeps
//!   the path, so the word stays correct; a row whose word disagrees makes
//!   that entry's row **unexplained** and is a deployment blocker.
//! * Payloads sit at `block << 4`, in file-table order, each followed by zero
//!   padding to the next 16-byte line; `archive_size` equals the file length
//!   and the end of the last payload, and is 16-aligned.
//! * The header words at `0x10` and `0x20..0x30`, and the zero bytes between
//!   the file table and the name table, are **opaque**: copied verbatim.
//!
//! ## The 20-byte-row PS4 variant (version `0x0011_0400`)
//!
//! Measured on NHL 21 CUSA18182 `inxp_00.big` (2 950 rows, overnight run
//! 2026-09-17): rows are `[block][word_4][size][word_12][word_16]`; `word_4`
//! is zero on every row; **`word_16` is the djb2 path hash on every row**
//! (the same rule as the 16-byte `+12` word); `word_12` is **path-derived
//! but its formula is unknown** — content-different siblings differing only
//! in a trailing character share it, while content-identical deduplicated
//! payloads at different paths do not, and the table is sorted by it. A
//! same-path replacement therefore leaves both hash words correct; the
//! `+12` word stays an *unresolved* qualification item (preserved verbatim,
//! not understood), so a 20-byte-row output is never deployable-qualified
//! by this module alone.

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Range;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::eb::{decode_payload, EbArchive, EbHeader, Payload, HEADER_LEN};
use tdb_archive_codec::{MAGIC_CHUNKLZX, MAGIC_CHUNKZIP};

/// Row stride of the PS3 / Xbox 360 Legacy layout.
pub const ROW_LEN: usize = 16;
/// Row stride of the PS4 layout.
pub const ROW_LEN_PS4: usize = 20;
/// The largest row stride this module plans for (rows are held in fixed
/// buffers of this size, `row_len` bytes of which are meaningful).
pub const MAX_ROW_LEN: usize = 20;
/// Version word of the 16-byte-row layout (see [`crate::eb::KNOWN_VERSION_TAGS`]).
pub const QUALIFIED_VERSION: u32 = 0x0000_0400;
/// Version word of the 20-byte-row PS4 layout.
pub const QUALIFIED_VERSION_PS4: u32 = 0x0011_0400;
/// Version word of the PS4 NHL 15 16-byte-row layout (EB-V1; see
/// [`crate::eb::VERSION_TAG_PS4_16`] for the characterisation). Same row shape as
/// [`QUALIFIED_VERSION`]; `archive_size` is a declared value that may be smaller
/// than the file length, so plans on such an archive never change its length.
pub const QUALIFIED_VERSION_PS4_16: u32 = crate::eb::VERSION_TAG_PS4_16;

/// The row stride this module plans for a version word, if it plans for it.
pub fn planned_row_len(version: u32) -> Option<usize> {
    match version {
        QUALIFIED_VERSION | QUALIFIED_VERSION_PS4_16 => Some(ROW_LEN),
        QUALIFIED_VERSION_PS4 => Some(ROW_LEN_PS4),
        _ => None,
    }
}

/// A file-table row: `row_len` meaningful bytes in a fixed buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub len: usize,
    pub bytes: [u8; MAX_ROW_LEN],
}

impl Row {
    fn from_slice(b: &[u8]) -> Self {
        let mut bytes = [0u8; MAX_ROW_LEN];
        bytes[..b.len()].copy_from_slice(b);
        Row { len: b.len(), bytes }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl std::ops::Deref for Row {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}
/// The file table starts here on every EB archive.
const FILE_TABLE_OFFSET: u64 = 0x30;
/// Offset of the `u64` `archive_size` header word.
const ARCHIVE_SIZE_OFFSET: usize = 0x18;
/// Payload blocks are addressed in 16-byte units, so a block index must fit
/// `u32`: the highest representable payload offset is `(2^32 - 1) << 4`.
const MAX_BLOCK_OFFSET: u64 = (u32::MAX as u64) << 4;
/// Streaming buffer for copying and hashing.
const CHUNK: usize = 1 << 20;

/// djb2 (`h = h * 33 + byte`, seed 5381) over the full forward-slash path —
/// the algorithm behind the `+12` row word on every observed 16-byte row.
pub fn path_hash(path: &str) -> u32 {
    path.bytes()
        .fold(5381u32, |h, b| h.wrapping_mul(33).wrapping_add(b as u32))
}

/// A 32-byte SHA-256 digest.
pub type Sha256Bytes = [u8; 32];

/// Hex rendering for reports and messages.
pub fn hex(d: &Sha256Bytes) -> String {
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// What a source archive **is**: its byte length and whole-file SHA-256.
/// Every plan is bound to one of these; staging refuses a source that no
/// longer matches it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceIdentity {
    pub len: u64,
    pub sha256: Sha256Bytes,
}

/// One entry of the bound source: its file-table row, span and stored digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundEntry {
    pub index: usize,
    pub path: String,
    /// Absolute offset of the stored bytes (`block << 4`).
    pub offset: u64,
    /// Stored length in bytes.
    pub size: u32,
    /// The row's `+4` word (zero on every observed row).
    pub word_4: u32,
    /// The row's `+12` word: the path hash on the 16-byte layout; on the
    /// 20-byte layout a path-derived word whose formula is unknown.
    pub word_12: u32,
    /// The row's `+16` word on the 20-byte layout (the path hash on every
    /// observed row); `None` on 16-byte rows.
    pub word_16: Option<u32>,
    /// SHA-256 of the stored bytes.
    pub stored_sha256: Sha256Bytes,
    /// Leading magic of the stored bytes, if it is one the codec layer knows.
    pub stored_form: StoredForm,
    /// End of the entry's own slot: the next 16-byte line after the span, or
    /// the next entry's offset if that comes first. Padding between the span
    /// and the slot end was verified zero at bind time (otherwise the slot
    /// end is the span end).
    pub slot_end: u64,
    /// True when the span intersects another entry's span (deduplicated or
    /// corrupt archives). Such an entry cannot be replaced unambiguously.
    pub shared_span: bool,
}

/// The stored form of a payload, by leading magic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredForm {
    Raw,
    ChunkZip,
    ChunkLzx,
}

impl StoredForm {
    fn of(bytes: &[u8]) -> Self {
        if bytes.len() >= 8 && &bytes[..8] == MAGIC_CHUNKZIP {
            Self::ChunkZip
        } else if bytes.len() >= 8 && &bytes[..8] == MAGIC_CHUNKLZX {
            Self::ChunkLzx
        } else {
            Self::Raw
        }
    }
}

/// Status of one deployment-qualification item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualificationStatus {
    /// Its semantics are established and the staged output keeps it correct.
    Resolved,
    /// Its semantics, or the game's acceptance of it, are not established.
    /// The bytes are preserved or produced structurally; nothing says they
    /// are right for the game.
    Unresolved,
    /// The item contradicts an established layout fact; also a
    /// `deploy_blockers` entry.
    Blocked,
}

/// One thing that must be affirmatively resolved before a staged output may
/// be called deployable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualificationItem {
    /// What is in question (a header word, a row word, an entry's
    /// replacement, native loading).
    pub subject: String,
    pub status: QualificationStatus,
    /// What is known, what was done with the bytes, and what is missing.
    pub note: String,
}

impl QualificationItem {
    fn new(
        subject: impl Into<String>,
        status: QualificationStatus,
        note: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            status,
            note: note.into(),
        }
    }
}

/// The affirmative deployment-qualification state. `Qualified` is reached
/// only when **every** item is [`QualificationStatus::Resolved`]; an unknown
/// never reads as qualified. Structural verification (the output re-parses,
/// decodes and matches the plan) is a separate fact carried by
/// [`VerificationReport`]; both are required to apply to a live target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualificationState {
    Unqualified,
    Qualified,
}

/// Deployment qualification of a plan or staged output: the items and the
/// state they imply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentQualification {
    pub state: QualificationState,
    pub items: Vec<QualificationItem>,
}

impl DeploymentQualification {
    /// Build from items; the state follows from them.
    pub fn from_items(items: Vec<QualificationItem>) -> Self {
        let state = if items
            .iter()
            .all(|i| i.status == QualificationStatus::Resolved)
        {
            QualificationState::Qualified
        } else {
            QualificationState::Unqualified
        };
        Self { state, items }
    }

    pub fn is_qualified(&self) -> bool {
        self.state == QualificationState::Qualified
    }

    /// Every item that is not resolved.
    pub fn open_items(&self) -> impl Iterator<Item = &QualificationItem> {
        self.items
            .iter()
            .filter(|i| i.status != QualificationStatus::Resolved)
    }
}

/// Observed native `chunkzip` framing on every measured member (PS3 Legacy
/// `patch.big`, PS4 NHL 21 `boot.big` / `overlays_2x.big`): 128 KiB chunks.
pub const NATIVE_CHUNK_SIZE: u32 = 131_072;

/// What binding established about the layout, and what it could not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutReport {
    pub version: u32,
    pub row_len: usize,
    pub file_count: usize,
    /// First byte of the payload region (`align_up_16(table_end)`).
    pub data_start: u64,
    /// Byte ranges before `data_start` this module does not interpret and
    /// copies verbatim: header `0x10..0x14`, `0x20..0x30`, the gap between
    /// the file table and the name table, and the alignment pads.
    pub opaque_regions: Vec<Range<u64>>,
    /// Every row's path-hash word (`+12` on 16-byte rows, `+16` on 20-byte
    /// rows) equals `path_hash(path)`.
    pub path_hash_verified: bool,
    /// Every row's `+4` word is zero.
    pub word_4_all_zero: bool,
    /// Every inter-entry gap and the tail after the last payload is zero.
    pub padding_all_zero: bool,
    /// Gap regions (bytes no entry covers) that hold nonzero bytes. Copied
    /// verbatim; an entry whose trailing pad is one of these gets no slack.
    /// A staged output that appended a payload has one at the old location.
    pub nonzero_gap_regions: Vec<Range<u64>>,
    /// Entries in file-table order have non-decreasing offsets.
    pub table_order_is_offset_order: bool,
    /// `archive_size` equals the file length.
    pub archive_size_is_file_len: bool,
    /// Facts that contradict an established layout rule (a row word that is
    /// not what every observed row carries). Staging still proceeds; the plan
    /// and report carry these forward and the output is never deployable
    /// while any exist.
    pub deploy_blockers: Vec<String>,
    /// Structural qualification items of this layout: every header word and
    /// region this module preserves without understanding, and every row
    /// word it does understand. A plan adds the per-replacement and native
    /// loading items on top of these.
    pub qualification_items: Vec<QualificationItem>,
}

/// A source archive read and measured once.
#[derive(Debug, Clone)]
pub struct BoundSource {
    pub identity: SourceIdentity,
    pub header: EbHeader,
    /// Bytes `0..data_start`, verbatim.
    pub prefix: Vec<u8>,
    pub layout: LayoutReport,
    pub entries: Vec<BoundEntry>,
}

/// A requested replacement of one entry's stored bytes, bound to what the
/// caller believes the source entry is. Every binding is checked against the
/// bound source before a plan exists.
#[derive(Debug, Clone)]
pub struct Replacement {
    pub entry_index: usize,
    /// The full path the caller expects at `entry_index`.
    pub expected_path: String,
    /// The stored-bytes digest the caller expects at `entry_index`.
    pub expected_stored_sha256: Sha256Bytes,
    /// The new stored bytes, already in on-disk form (raw or `chunkzip`).
    pub new_stored: Vec<u8>,
    /// What the new stored bytes must decode to. Verification compares the
    /// output entry against this, not merely against `new_stored`.
    pub expected_decoded: ExpectedDecoded,
}

/// Expected decoded content of a replaced entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedDecoded {
    pub len: u64,
    pub sha256: Sha256Bytes,
}

impl ExpectedDecoded {
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            len: bytes.len() as u64,
            sha256: Sha256::digest(bytes).into(),
        }
    }
}

/// Where a replacement's bytes go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// New bytes equal the stored bytes: nothing changes.
    NoOp,
    /// The new bytes overwrite the entry's own slot; slack up to the old end
    /// is zero-filled; nothing else moves.
    FitInSlot { offset: u64, slot_end: u64 },
    /// The new bytes are appended at `new_offset` (16-aligned, after the
    /// staged end of file); the row is retargeted; the old bytes stay.
    AppendAndRetarget { new_offset: u64 },
}

/// The exact file-table row edit a placement needs, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowEdit {
    pub row_offset: u64,
    pub old_row: Row,
    pub new_row: Row,
}

/// One resolved replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedEdit {
    pub entry_index: usize,
    pub path: String,
    pub old_offset: u64,
    pub old_size: u32,
    pub old_stored_sha256: Sha256Bytes,
    pub new_size: u32,
    pub new_stored_sha256: Sha256Bytes,
    pub expected_decoded: ExpectedDecoded,
    pub placement: Placement,
    pub row_edit: Option<RowEdit>,
    /// Byte ranges of the **output** this edit writes.
    pub written_ranges: Vec<Range<u64>>,
    /// The paths of the other entries that share this entry's stored span (deduplicated
    /// archives). Under [`SharedSpanRule::PrivateCopy`] those rows keep pointing at the shared
    /// bytes, which stay in place, and this row alone is retargeted at a private copy; under
    /// [`SharedSpanRule::WholeGroup`] they follow the new bytes (`sibling_row_edits`).
    pub shared_with: Vec<String>,
    /// Whole-group replacement: the row edits of every sibling (size word on a fit, block + size
    /// on an append); empty otherwise.
    pub sibling_row_edits: Vec<RowEdit>,
}

/// How [`plan_with`] treats a replacement whose entry shares its stored span with other rows
/// (the deduplicated PS4 `0x00110400` archives: a league logo, its French twin and the scouting
/// region logo can all be one block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SharedSpanRule {
    /// Refuse with [`StageError::SharedSpan`] (the default of [`plan`]).
    #[default]
    Refuse,
    /// Give the chosen row a private copy: the new bytes are always appended and only that row
    /// is retargeted; the shared bytes and every sibling row stay exactly as they are (research
    /// 02 §1.4 / §2 "private copy"). The siblings are named on the edit for the review.
    PrivateCopy,
    /// Replace the whole deduplicated group: every row of the block takes the new bytes — in place
    /// when they fit the shared slot (each row's size word patched), else appended once with every
    /// row re-pointed (research 02 §4.1 "whole group replace"). Requires every sibling to share the
    /// entry's exact `(offset, size)`; the siblings are named on the edit.
    WholeGroup,
}

/// The header edit an append needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderEdit {
    pub old_archive_size: u64,
    pub new_archive_size: u64,
}

/// A complete, checked plan. Holds the new bytes it will write.
#[derive(Debug, Clone)]
pub struct Plan {
    pub source: SourceIdentity,
    pub layout: LayoutReport,
    pub original_eof: u64,
    pub new_eof: u64,
    /// The `archive_size` word the output must carry: the header edit's new
    /// value when the length changes, else the source's declared word verbatim
    /// (which is not necessarily the length on the NHL 15 PS4 layout).
    pub expected_archive_size: u64,
    pub header_edit: Option<HeaderEdit>,
    pub edits: Vec<PlannedEdit>,
    /// Every output byte range that differs from, or lies beyond, the source:
    /// row edits, the header edit, fit-in-slot spans, appended payloads and
    /// their padding. Sorted, non-overlapping.
    pub changed_ranges: Vec<Range<u64>>,
    /// Carried from the layout: the output is not deployable while non-empty.
    pub deploy_blockers: Vec<String>,
    /// The layout's items plus one per non-no-op replacement and the native
    /// loading item. `Unqualified` until every one is resolved.
    pub qualification: DeploymentQualification,
    /// Payload bytes per edit, in `edits` order (empty for a no-op).
    new_payloads: Vec<Vec<u8>>,
}

impl Plan {
    /// True when no byte of the output would differ from the source.
    pub fn is_noop(&self) -> bool {
        self.changed_ranges.is_empty()
    }
    /// The new stored bytes for edit `i`.
    pub fn new_payload(&self, i: usize) -> &[u8] {
        &self.new_payloads[i]
    }
}

/// What staging produced: identities and the ranges the apply step needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageReport {
    pub source: SourceIdentity,
    pub output: SourceIdentity,
    pub original_eof: u64,
    pub new_eof: u64,
    pub changed_ranges: Vec<Range<u64>>,
    /// Structural contradictions (see [`LayoutReport::deploy_blockers`]).
    pub deploy_blockers: Vec<String>,
    /// The plan's qualification, carried unchanged: what remains unresolved
    /// before this output may be applied to a live target.
    pub qualification: DeploymentQualification,
}

impl StageReport {
    /// True only when nothing contradicts the layout **and** every
    /// qualification item is affirmatively resolved. Structural verification
    /// is a further, separate requirement (see [`VerificationReport`]).
    pub fn deployable(&self) -> bool {
        self.deploy_blockers.is_empty() && self.qualification.is_qualified()
    }
}

/// What verification checked. Every field is a count of checks that ran.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VerificationReport {
    pub entries_reparsed: usize,
    pub changed_entries_decoded: usize,
    pub unchanged_rows_compared: usize,
    pub untouched_bytes_compared: u64,
    pub output: Option<SourceIdentity>,
}

#[derive(Debug, Error)]
pub enum StageError {
    #[error("I/O: {0}")]
    Io(#[from] io::Error),
    #[error("source is not a parseable EB archive: {0}")]
    Parse(#[from] crate::eb::EbError),
    #[error(
        "layout is not qualified for planning: version 0x{version:08X} has {row_len}-byte rows; only versions 0x{QUALIFIED_VERSION:08X} and 0x{QUALIFIED_VERSION_PS4_16:08X} with {ROW_LEN}-byte rows and version 0x{QUALIFIED_VERSION_PS4:08X} with {ROW_LEN_PS4}-byte rows are"
    )]
    LayoutUnqualified { version: u32, row_len: usize },
    #[error("header archive_size {declared} does not equal the file length {actual}")]
    ArchiveSizeMismatch { declared: u64, actual: u64 },
    #[error("header archive_size {declared} is smaller than the file length {actual} but the entries end at {entries_end} (align16 {entries_end_aligned}), not at the file length; the layout is inconsistent")]
    ArchiveSizeInconsistent {
        declared: u64,
        actual: u64,
        entries_end: u64,
        entries_end_aligned: u64,
    },
    #[error("the archive's declared archive_size ({declared}) differs from its length ({actual}); a plan that changes the length (staged end {staged_eof}) cannot be made because the header word's meaning on this layout is not established")]
    LengthChangeOnUnresolvedArchiveSize {
        declared: u64,
        actual: u64,
        staged_eof: u64,
    },
    #[error("entry {index} `{path}` span {offset}+{size} lies outside the payload region {data_start}..{eof}")]
    SpanOutOfRange {
        index: usize,
        path: String,
        offset: u64,
        size: u32,
        data_start: u64,
        eof: u64,
    },
    #[error("replacement names entry {index} but the archive has {count} entries")]
    NoSuchEntry { index: usize, count: usize },
    #[error(
        "replacement for entry {index} expects path `{expected}` but the archive has `{actual}`"
    )]
    PathMismatch {
        index: usize,
        expected: String,
        actual: String,
    },
    #[error("replacement for entry {index} `{path}` expects stored SHA-256 {expected} but the archive has {actual}")]
    StoredHashMismatch {
        index: usize,
        path: String,
        expected: String,
        actual: String,
    },
    #[error("entry {index} `{path}` is named by more than one replacement")]
    DuplicateReplacement { index: usize, path: String },
    #[error("entry {index} `{path}` shares its stored span with another entry; replacing it is ambiguous")]
    SharedSpan { index: usize, path: String },
    #[error("entry {index} `{path}`: whole-group replacement needs every sibling to share the exact span; `{sibling}` overlaps it with a different offset or size")]
    SharedGroupIrregular { index: usize, path: String, sibling: String },
    /// R1 F1 (20 September 2026): two replacements claim the same file-table row — a whole-group
    /// replacement owns every sibling row of its block, so a second whole-group request on the
    /// same block, or a private-copy request on one of its siblings, contradicts it. Refused
    /// before any placement; the caller resolves the scopes (no winner is chosen).
    #[error("entry {index} `{path}` and entry {other_index} `{other_path}` both replace the row of `{contested}` (a whole-group replacement owns every row of its block); choose one scope for the block")]
    ConflictingOwnership {
        index: usize,
        path: String,
        other_index: usize,
        other_path: String,
        contested: String,
    },
    #[error("entry {index} `{path}`: new stored bytes are in a form that cannot be verified")]
    UnsupportedNewPayload { index: usize, path: String },
    #[error("entry {index} `{path}`: new stored length {len} does not fit the 32-bit size word")]
    SizeNotRepresentable {
        index: usize,
        path: String,
        len: u64,
    },
    #[error("entry {index} `{path}`: appended offset {offset} does not fit the 32-bit block word (max {MAX_BLOCK_OFFSET})")]
    OffsetNotRepresentable {
        index: usize,
        path: String,
        offset: u64,
    },
    #[error("arithmetic overflow computing {what}")]
    Overflow { what: &'static str },
    #[error("source changed since it was bound: expected {expected_len} bytes / {expected_sha}, saw {actual_len} bytes / {actual_sha}")]
    SourceChanged {
        expected_len: u64,
        expected_sha: String,
        actual_len: u64,
        actual_sha: String,
    },
    #[error("plan was made for a different source ({plan_sha}) than the one bound ({source_sha})")]
    PlanSourceMismatch {
        plan_sha: String,
        source_sha: String,
    },
    #[error("output length {actual} differs from the planned {expected}")]
    OutputLengthMismatch { expected: u64, actual: u64 },
    #[error("output SHA-256 {actual} differs from the staged {expected}")]
    OutputHashMismatch { expected: String, actual: String },
    #[error("output header archive_size {actual} differs from the planned {expected}")]
    OutputArchiveSizeMismatch { expected: u64, actual: u64 },
    #[error("output entry count {actual} differs from the source's {expected}")]
    OutputEntryCountMismatch { expected: usize, actual: usize },
    #[error("output entry {index} `{path}` row is {actual:02x?}, planned {expected:02x?}")]
    RowMismatch {
        index: usize,
        path: String,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },
    #[error("output entry {index} `{path}` stored bytes hash to {actual}, planned {expected}")]
    StoredMismatch {
        index: usize,
        path: String,
        expected: String,
        actual: String,
    },
    #[error("output entry {index} `{path}` does not decode: {source}")]
    DecodeFailed {
        index: usize,
        path: String,
        source: tdb_archive_codec::CodecError,
    },
    #[error("output entry {index} `{path}` decodes to {actual_len} bytes / {actual_sha}, expected {expected_len} bytes / {expected_sha}")]
    DecodedMismatch {
        index: usize,
        path: String,
        expected_len: u64,
        expected_sha: String,
        actual_len: u64,
        actual_sha: String,
    },
    #[error(
        "untouched byte range {start}..{end} of the output differs from the source at offset {at}"
    )]
    UntouchedBytesDiffer { start: u64, end: u64, at: u64 },
}

fn checked_add(a: u64, b: u64, what: &'static str) -> Result<u64, StageError> {
    a.checked_add(b).ok_or(StageError::Overflow { what })
}

fn align_up_16(n: u64, what: &'static str) -> Result<u64, StageError> {
    Ok(checked_add(n, 15, what)? & !15)
}

// ================================================================= binding

impl BoundSource {
    /// Read the source once: identity, tables, spans, per-entry digests and
    /// the layout facts. The reader is never written to.
    pub fn bind<R: Read + Seek>(src: &mut R) -> Result<Self, StageError> {
        let len = src.seek(SeekFrom::End(0))?;
        src.seek(SeekFrom::Start(0))?;

        // Header + tables.
        let mut head = vec![0u8; HEADER_LEN];
        src.read_exact(&mut head)?;
        let header = EbHeader::parse(&head)?;
        let row_len = match planned_row_len(header.version) {
            Some(n) if n == header.entry_size() => n,
            _ => {
                return Err(StageError::LayoutUnqualified {
                    version: header.version,
                    row_len: header.entry_size(),
                })
            }
        };
        // Every table claim is checked against the file length (ordering,
        // overlap, overflow, bounds) before any allocation or slice.
        let tables = header.table_layout(len)?;
        let table_end = tables.table_end;
        let data_start = tables.data_start;
        if data_start > len {
            return Err(crate::eb::EbError::TruncatedTables {
                needed: usize::try_from(data_start).unwrap_or(usize::MAX),
                have: usize::try_from(len).unwrap_or(usize::MAX),
            }
            .into());
        }
        let mut prefix = vec![0u8; data_start as usize];
        src.seek(SeekFrom::Start(0))?;
        src.read_exact(&mut prefix)?;
        let arc = EbArchive::parse(&prefix[..table_end as usize])?;
        // `archive_size`: on the 0x0000_0400 and 0x0011_0400 layouts it equals the file length on
        // every observed archive and is required to. On the PS4 NHL 15 layout (0x0010_0400) it is
        // a declared value that three of the 17 installed archives set *below* the file length
        // while their entries still end exactly at the file length; that is accepted and recorded
        // (never "fixed"), anything else is refused.
        let archive_size_is_file_len = header.archive_size == len;
        if !archive_size_is_file_len {
            if header.version != QUALIFIED_VERSION_PS4_16 || header.archive_size > len {
                return Err(StageError::ArchiveSizeMismatch {
                    declared: header.archive_size,
                    actual: len,
                });
            }
            let entries_end = arc
                .entries
                .iter()
                .map(|e| checked_add(e.offset, e.size as u64, "entry span end"))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .max()
                .unwrap_or(data_start);
            let entries_end_aligned = align_up_16(entries_end, "entries end")?;
            if entries_end_aligned != len {
                return Err(StageError::ArchiveSizeInconsistent {
                    declared: header.archive_size,
                    actual: len,
                    entries_end,
                    entries_end_aligned,
                });
            }
        }

        // Rows and spans.
        let mut entries = Vec::with_capacity(arc.entries.len());
        for (i, e) in arc.entries.iter().enumerate() {
            let row = FILE_TABLE_OFFSET as usize + i * row_len;
            let word_4 = u32::from_be_bytes(prefix[row + 4..row + 8].try_into().unwrap());
            let word_12 = u32::from_be_bytes(prefix[row + 12..row + 16].try_into().unwrap());
            let word_16 = (row_len == ROW_LEN_PS4)
                .then(|| u32::from_be_bytes(prefix[row + 16..row + 20].try_into().unwrap()));
            let end = checked_add(e.offset, e.size as u64, "entry span end")?;
            if e.offset < data_start || end > len {
                return Err(StageError::SpanOutOfRange {
                    index: i,
                    path: e.path.clone(),
                    offset: e.offset,
                    size: e.size,
                    data_start,
                    eof: len,
                });
            }
            entries.push(BoundEntry {
                index: i,
                path: e.path.clone(),
                offset: e.offset,
                size: e.size,
                word_4,
                word_12,
                word_16,
                stored_sha256: [0; 32],
                stored_form: StoredForm::Raw,
                slot_end: end,
                shared_span: false,
            });
        }

        // Shared spans and slot ends, by offset order.
        let mut order: Vec<usize> = (0..entries.len()).collect();
        order.sort_by_key(|&i| (entries[i].offset, entries[i].index));
        let table_order_is_offset_order = order.iter().enumerate().all(|(k, &i)| k == i);
        for k in 0..order.len() {
            let i = order[k];
            let (off, end) = (
                entries[i].offset,
                entries[i].offset + entries[i].size as u64,
            );
            // Any later-starting entry that begins before this one ends shares bytes.
            for &j in &order[k + 1..] {
                if entries[j].offset < end {
                    entries[i].shared_span = true;
                    entries[j].shared_span = true;
                } else {
                    break;
                }
            }
            // Zero-size entries at the same offset also collide.
            if k + 1 < order.len() && entries[order[k + 1]].offset == off {
                entries[i].shared_span = true;
                entries[order[k + 1]].shared_span = true;
            }
            let line = align_up_16(end, "slot end")?;
            let next = order[k + 1..]
                .iter()
                .map(|&j| entries[j].offset)
                .find(|&o| o >= end)
                .unwrap_or(len);
            entries[i].slot_end = line.min(next).min(len);
        }

        // Pass 2, sequential: whole-file hash, and every byte outside the union of
        // the spans (the gaps and the tail) must be zero.
        let mut covered: Vec<Range<u64>> = entries
            .iter()
            .map(|e| e.offset..e.offset + e.size as u64)
            .collect();
        covered = normalize(covered);
        let gaps = complement(&covered, len)
            .into_iter()
            .map(|g| g.start.max(data_start)..g.end)
            .filter(|g| g.end > g.start)
            .collect::<Vec<_>>();
        let mut file_hash = Sha256::new();
        file_hash.update(&prefix);
        let mut nonzero_gaps: Vec<Range<u64>> = Vec::new();
        let mut buf = vec![0u8; CHUNK];
        src.seek(SeekFrom::Start(data_start))?;
        let mut pos = data_start;
        while pos < len {
            let n = src.read(&mut buf[..(len - pos).min(CHUNK as u64) as usize])?;
            if n == 0 {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
            }
            let chunk = &buf[..n];
            file_hash.update(chunk);
            let (cs, ce) = (pos, pos + n as u64);
            for g in &gaps {
                let (gs, ge) = (g.start.max(cs), g.end.min(ce));
                if gs < ge {
                    // Record each run of nonzero bytes exactly.
                    let slice = &chunk[(gs - cs) as usize..(ge - cs) as usize];
                    let mut run: Option<u64> = None;
                    for (k, &b) in slice.iter().enumerate() {
                        match (b != 0, run) {
                            (true, None) => run = Some(gs + k as u64),
                            (false, Some(start)) => {
                                nonzero_gaps.push(start..gs + k as u64);
                                run = None;
                            }
                            _ => {}
                        }
                    }
                    if let Some(start) = run {
                        nonzero_gaps.push(start..ge);
                    }
                }
            }
            pos = ce;
        }
        let sha256: Sha256Bytes = file_hash.finalize().into();

        // Pass 3, per entry: stored digest and leading magic.
        for e in &mut entries {
            src.seek(SeekFrom::Start(e.offset))?;
            let mut h = Sha256::new();
            let mut first = [0u8; 8];
            let mut first_len = 0usize;
            let mut left = e.size as u64;
            while left > 0 {
                let n = src.read(&mut buf[..left.min(CHUNK as u64) as usize])?;
                if n == 0 {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
                }
                if first_len < 8 {
                    let take = (8 - first_len).min(n);
                    first[first_len..first_len + take].copy_from_slice(&buf[..take]);
                    first_len += take;
                }
                h.update(&buf[..n]);
                left -= n as u64;
            }
            e.stored_sha256 = h.finalize().into();
            e.stored_form = StoredForm::of(&first[..first_len]);
        }

        // Slack is only usable when the entry's own trailing pad is zero.
        let nonzero_gaps = normalize(nonzero_gaps);
        let padding_all_zero = nonzero_gaps.is_empty();
        for e in &mut entries {
            let end = e.offset + e.size as u64;
            if nonzero_gaps
                .iter()
                .any(|g| g.start < e.slot_end && g.end > end)
            {
                e.slot_end = end;
            }
        }

        // Layout facts.
        // The path-hash word sits at +12 on 16-byte rows and at +16 on 20-byte rows.
        let hash_word = |e: &BoundEntry| e.word_16.unwrap_or(e.word_12);
        let hash_word_name = if row_len == ROW_LEN_PS4 { "+16" } else { "+12" };
        let path_hash_verified = entries.iter().all(|e| hash_word(e) == path_hash(&e.path));
        let word_4_all_zero = entries.iter().all(|e| e.word_4 == 0);
        let mut deploy_blockers = Vec::new();
        for e in &entries {
            if hash_word(e) != path_hash(&e.path) {
                deploy_blockers.push(format!(
                    "entry {} `{}`: row {hash_word_name} word 0x{:08x} is not the path hash 0x{:08x}; its meaning is unknown",
                    e.index,
                    e.path,
                    hash_word(e),
                    path_hash(&e.path)
                ));
            }
            if e.word_4 != 0 {
                deploy_blockers.push(format!(
                    "entry {} `{}`: row +4 word 0x{:08x} is nonzero; its meaning is unknown",
                    e.index, e.path, e.word_4
                ));
            }
        }
        let mut opaque_regions = vec![0x10..0x14, 0x20..0x30];
        if tables.file_table_end < tables.name_table_offset {
            opaque_regions.push(tables.file_table_end..tables.name_table_offset);
        }
        if tables.name_end < tables.dir_start {
            opaque_regions.push(tables.name_end..tables.dir_start);
        }
        if table_end < data_start {
            opaque_regions.push(table_end..data_start);
        }

        // Qualification items: what is preserved without being understood.
        use QualificationStatus::{Blocked, Resolved, Unresolved};
        let mut items = Vec::new();
        let hash_subject = format!("row {hash_word_name} word (path hash)");
        items.push(if path_hash_verified {
            QualificationItem::new(
                hash_subject,
                Resolved,
                "djb2 of the full path on every row; a same-path replacement keeps it correct",
            )
        } else {
            QualificationItem::new(
                hash_subject,
                Blocked,
                "at least one row's word is not the path hash; its meaning is unknown (see deploy_blockers)",
            )
        });
        if row_len == ROW_LEN_PS4 {
            items.push(QualificationItem::new(
                "row +12 word (PS4 sort key)",
                Unresolved,
                "path-derived on every measured archive (shared by siblings that differ only in a trailing character, distinct across deduplicated payloads) but its formula is unknown; preserved verbatim on a same-path replacement, not recomputed",
            ));
        }
        items.push(if word_4_all_zero {
            QualificationItem::new(
                "row +4 word",
                Resolved,
                "zero on every row, as on every observed archive; preserved",
            )
        } else {
            QualificationItem::new(
                "row +4 word",
                Blocked,
                "nonzero on at least one row; its meaning is unknown (see deploy_blockers)",
            )
        });
        let word_0x10 = u32::from_be_bytes(prefix[0x10..0x14].try_into().unwrap());
        items.push(if word_0x10 == 0 {
            QualificationItem::new("header word 0x10", Resolved, "zero; preserved")
        } else {
            QualificationItem::new(
                "header word 0x10",
                Unresolved,
                format!(
                    "nonzero ({word_0x10}) and its meaning is unknown; preserved verbatim, but whether it stays correct after payload, table or length changes is not established"
                ),
            )
        });
        let opaque_head = &prefix[0x20..0x30];
        items.push(if opaque_head.iter().all(|&b| b == 0) {
            QualificationItem::new("header bytes 0x20..0x30", Resolved, "zero; preserved")
        } else {
            QualificationItem::new(
                "header bytes 0x20..0x30",
                Unresolved,
                "nonzero and uninterpreted; preserved verbatim",
            )
        });
        let table_gaps_zero = opaque_regions.iter().skip(2).all(|r| {
            prefix[r.start as usize..r.end as usize]
                .iter()
                .all(|&b| b == 0)
        });
        items.push(if table_gaps_zero {
            QualificationItem::new(
                "table gaps and alignment pads",
                Resolved,
                "zero, as on every observed archive; preserved",
            )
        } else {
            QualificationItem::new(
                "table gaps and alignment pads",
                Unresolved,
                "nonzero bytes between the metadata tables; uninterpreted; preserved verbatim",
            )
        });
        items.push(if padding_all_zero {
            QualificationItem::new(
                "payload-region gaps",
                Resolved,
                "every byte no entry covers is zero; preserved",
            )
        } else {
            QualificationItem::new(
                "payload-region gaps",
                Unresolved,
                format!(
                    "{} nonzero region(s) no entry covers (an earlier append leaves the old payload behind); preserved verbatim; whether the game tolerates them is not established",
                    nonzero_gaps.len()
                ),
            )
        });
        items.push(if archive_size_is_file_len {
            QualificationItem::new(
                "header archive_size",
                Resolved,
                "equals the file length and is kept equal to the staged length; 16-aligned as on every observed archive",
            )
        } else {
            QualificationItem::new(
                "header archive_size",
                Unresolved,
                format!(
                    "declared {} is smaller than the file length {} while the entries end at the file length (as on 3 of the 17 NHL 15 PS4 archives); preserved verbatim, the file length is never changed on this archive, and whether the game reads the word at all is not established",
                    header.archive_size, len
                ),
            )
        });
        let layout = LayoutReport {
            version: header.version,
            row_len,
            file_count: header.file_count,
            data_start,
            opaque_regions,
            path_hash_verified,
            word_4_all_zero,
            padding_all_zero,
            nonzero_gap_regions: nonzero_gaps,
            table_order_is_offset_order,
            archive_size_is_file_len,
            deploy_blockers,
            qualification_items: items,
        };

        Ok(Self {
            identity: SourceIdentity { len, sha256 },
            header,
            prefix,
            layout,
            entries,
        })
    }

    pub fn data_start(&self) -> u64 {
        self.layout.data_start
    }
}

// ================================================================= planning

/// Decode `stored` and compare it with `expected`.
fn check_decoded(
    index: usize,
    path: &str,
    stored: Vec<u8>,
    expected: &ExpectedDecoded,
) -> Result<(), StageError> {
    let decoded = match decode_payload(stored) {
        Ok(Payload::Data(d)) => d,
        Ok(Payload::UnsupportedLzx) => {
            // Not produced by `decode_payload` any more; kept so a future variant is not skipped.
            return Err(StageError::UnsupportedNewPayload {
                index,
                path: path.to_owned(),
            })
        }
        Err(source) => {
            return Err(StageError::DecodeFailed {
                index,
                path: path.to_owned(),
                source,
            })
        }
    };
    let dsha: Sha256Bytes = Sha256::digest(&decoded).into();
    if decoded.len() as u64 != expected.len || dsha != expected.sha256 {
        return Err(StageError::DecodedMismatch {
            index,
            path: path.to_owned(),
            expected_len: expected.len,
            expected_sha: hex(&expected.sha256),
            actual_len: decoded.len() as u64,
            actual_sha: hex(&dsha),
        });
    }
    Ok(())
}

/// Resolve `replacements` against `source` into a [`Plan`]. Nothing is
/// written. Replacements are applied in ascending entry order; appends are
/// laid out in that order after the source's end of file.
pub fn plan(source: &BoundSource, replacements: Vec<Replacement>) -> Result<Plan, StageError> {
    plan_with(source, replacements, SharedSpanRule::Refuse)
}

/// [`plan`] with an explicit rule for shared-span entries.
pub fn plan_with(
    source: &BoundSource,
    replacements: Vec<Replacement>,
    shared_rule: SharedSpanRule,
) -> Result<Plan, StageError> {
    plan_scoped(source, replacements, shared_rule, &[])
}

/// [`plan_with`] where the entries listed in `whole_group` (by index) take the
/// [`SharedSpanRule::WholeGroup`] treatment regardless of `shared_rule` — the per-replacement
/// choice an explicit review makes (C7, overnight 2026-09-20); every other shared entry follows
/// `shared_rule`.
pub fn plan_scoped(
    source: &BoundSource,
    replacements: Vec<Replacement>,
    shared_rule: SharedSpanRule,
    whole_group: &[usize],
) -> Result<Plan, StageError> {
    let count = source.entries.len();
    let original_eof = source.identity.len;

    // Bind and order.
    let mut reqs = replacements;
    reqs.sort_by_key(|r| r.entry_index);
    for w in reqs.windows(2) {
        if w[0].entry_index == w[1].entry_index {
            let e = source.entries.get(w[0].entry_index);
            return Err(StageError::DuplicateReplacement {
                index: w[0].entry_index,
                path: e.map(|e| e.path.clone()).unwrap_or_default(),
            });
        }
    }
    for r in &reqs {
        let e = source
            .entries
            .get(r.entry_index)
            .ok_or(StageError::NoSuchEntry {
                index: r.entry_index,
                count,
            })?;
        if e.path != r.expected_path {
            return Err(StageError::PathMismatch {
                index: e.index,
                expected: r.expected_path.clone(),
                actual: e.path.clone(),
            });
        }
        if e.stored_sha256 != r.expected_stored_sha256 {
            return Err(StageError::StoredHashMismatch {
                index: e.index,
                path: e.path.clone(),
                expected: hex(&r.expected_stored_sha256),
                actual: hex(&e.stored_sha256),
            });
        }
        if e.shared_span && shared_rule == SharedSpanRule::Refuse {
            return Err(StageError::SharedSpan {
                index: e.index,
                path: e.path.clone(),
            });
        }
        // A new `chunklzx` payload is no longer refused by form (21 Sep 2026): the decoder is
        // promoted, so `check_decoded` below verifies it like any other stored form — the store
        // route (`compress_chunklzx_store`) produces such payloads. Loader acceptance of a
        // store-encoded member is a separate, runtime question the caller must qualify.
        if r.new_stored.len() as u64 > u32::MAX as u64 {
            return Err(StageError::SizeNotRepresentable {
                index: e.index,
                path: e.path.clone(),
                len: r.new_stored.len() as u64,
            });
        }
        // The new bytes must already decode to what the caller expects; a plan
        // that could only fail at verification is refused here instead.
        check_decoded(e.index, &e.path, r.new_stored.clone(), &r.expected_decoded)?;
    }

    // Ownership (R1 F1): the rows each replacement writes. A whole-group replacement owns its own
    // row and every sibling row of the block; a private copy or an unshared entry owns its own row
    // only. Two requests owning one row contradict each other and are refused here, before any
    // placement — never resolved by order, and never deferred to staging.
    {
        let mut owner: Vec<Option<usize>> = vec![None; count];
        for r in &reqs {
            let e = &source.entries[r.entry_index];
            let group = e.shared_span && (whole_group.contains(&e.index) || shared_rule == SharedSpanRule::WholeGroup);
            let mut owned = vec![e.index];
            if group {
                owned.extend(shared_siblings(source, e).into_iter().map(|o| o.index));
            }
            for i in owned {
                if let Some(first) = owner[i] {
                    return Err(StageError::ConflictingOwnership {
                        index: first,
                        path: source.entries[first].path.clone(),
                        other_index: e.index,
                        other_path: e.path.clone(),
                        contested: source.entries[i].path.clone(),
                    });
                }
                owner[i] = Some(e.index);
            }
        }
    }

    // Place.
    let mut staged_eof = original_eof;
    let mut edits = Vec::with_capacity(reqs.len());
    let mut payloads = Vec::with_capacity(reqs.len());
    let mut changed: Vec<Range<u64>> = Vec::new();
    for r in reqs {
        let e = &source.entries[r.entry_index];
        let new_size = r.new_stored.len() as u32;
        let new_sha: Sha256Bytes = Sha256::digest(&r.new_stored).into();
        let row_len = source.layout.row_len;
        let row_offset = FILE_TABLE_OFFSET + (e.index * row_len) as u64;
        let old_row =
            Row::from_slice(&source.prefix[row_offset as usize..row_offset as usize + row_len]);

        // Under the private-copy rule a shared span is never written in place: the siblings
        // keep the block, so the chosen row always takes an appended private copy. Under the
        // whole-group rule (plan-wide or per replacement) every sibling row follows.
        let siblings: Vec<&BoundEntry> = if e.shared_span { shared_siblings(source, e) } else { Vec::new() };
        let shared_with: Vec<String> = siblings.iter().map(|o| o.path.clone()).collect();
        let whole_group = e.shared_span && (whole_group.contains(&e.index) || shared_rule == SharedSpanRule::WholeGroup);
        if whole_group {
            if let Some(o) = siblings.iter().find(|o| o.offset != e.offset || o.size != e.size) {
                return Err(StageError::SharedGroupIrregular { index: e.index, path: e.path.clone(), sibling: o.path.clone() });
            }
        }
        let sibling_row = |o: &BoundEntry| -> (u64, Row) {
            let ro = FILE_TABLE_OFFSET + (o.index * row_len) as u64;
            (ro, Row::from_slice(&source.prefix[ro as usize..ro as usize + row_len]))
        };
        let mut sibling_row_edits: Vec<RowEdit> = Vec::new();
        let (placement, row_edit, written) = if new_sha == e.stored_sha256 {
            (Placement::NoOp, None, Vec::new())
        } else {
            let old_end = e.offset + e.size as u64;
            let fits = (!e.shared_span || whole_group) && checked_add(e.offset, new_size as u64, "fit end")? <= e.slot_end;
            if fits {
                let mut new_row = old_row;
                new_row.bytes[8..12].copy_from_slice(&new_size.to_be_bytes());
                let span_end = old_end.max(e.offset + new_size as u64);
                let mut w = Vec::with_capacity(2);
                w.push(e.offset..span_end);
                let row_edit = if new_row != old_row {
                    w.push(row_offset..row_offset + row_len as u64);
                    Some(RowEdit {
                        row_offset,
                        old_row,
                        new_row,
                    })
                } else {
                    None
                };
                if whole_group {
                    for o in &siblings {
                        let (ro, old) = sibling_row(o);
                        let mut nr = old;
                        nr.bytes[8..12].copy_from_slice(&new_size.to_be_bytes());
                        if nr != old {
                            w.push(ro..ro + row_len as u64);
                            sibling_row_edits.push(RowEdit { row_offset: ro, old_row: old, new_row: nr });
                        }
                    }
                }
                (
                    Placement::FitInSlot {
                        offset: e.offset,
                        slot_end: e.slot_end,
                    },
                    row_edit,
                    w,
                )
            } else {
                let new_offset = align_up_16(staged_eof, "append offset")?;
                if new_offset > MAX_BLOCK_OFFSET {
                    return Err(StageError::OffsetNotRepresentable {
                        index: e.index,
                        path: e.path.clone(),
                        offset: new_offset,
                    });
                }
                let end = checked_add(new_offset, new_size as u64, "append end")?;
                let mut new_row = old_row;
                new_row.bytes[0..4].copy_from_slice(&((new_offset >> 4) as u32).to_be_bytes());
                new_row.bytes[8..12].copy_from_slice(&new_size.to_be_bytes());
                // The pad between the staged EOF and the new offset, the payload, and
                // (for the plan's final padding) up to the next line are all new bytes.
                let mut w = vec![
                    staged_eof..align_up_16(end, "append pad")?,
                    row_offset..row_offset + row_len as u64,
                ];
                if whole_group {
                    for o in &siblings {
                        let (ro, old) = sibling_row(o);
                        let mut nr = old;
                        nr.bytes[0..4].copy_from_slice(&((new_offset >> 4) as u32).to_be_bytes());
                        nr.bytes[8..12].copy_from_slice(&new_size.to_be_bytes());
                        w.push(ro..ro + row_len as u64);
                        sibling_row_edits.push(RowEdit { row_offset: ro, old_row: old, new_row: nr });
                    }
                }
                staged_eof = end;
                (
                    Placement::AppendAndRetarget { new_offset },
                    Some(RowEdit {
                        row_offset,
                        old_row,
                        new_row,
                    }),
                    w,
                )
            }
        };
        changed.extend(written.iter().cloned());
        edits.push(PlannedEdit {
            entry_index: e.index,
            path: e.path.clone(),
            old_offset: e.offset,
            old_size: e.size,
            old_stored_sha256: e.stored_sha256,
            new_size,
            new_stored_sha256: new_sha,
            expected_decoded: r.expected_decoded,
            placement,
            row_edit,
            written_ranges: written,
            shared_with,
            sibling_row_edits,
        });
        payloads.push(if new_sha == e.stored_sha256 {
            Vec::new()
        } else {
            r.new_stored
        });
    }

    let new_eof = if staged_eof == original_eof {
        original_eof
    } else {
        align_up_16(staged_eof, "new eof")?
    };
    if new_eof != original_eof && !source.layout.archive_size_is_file_len {
        return Err(StageError::LengthChangeOnUnresolvedArchiveSize {
            declared: source.header.archive_size,
            actual: original_eof,
            staged_eof,
        });
    }
    let header_edit = if new_eof != original_eof {
        changed.push(ARCHIVE_SIZE_OFFSET as u64..ARCHIVE_SIZE_OFFSET as u64 + 8);
        Some(HeaderEdit {
            old_archive_size: original_eof,
            new_archive_size: new_eof,
        })
    } else {
        None
    };

    // Qualification: the layout's items, one per replacement that changes
    // bytes, and native loading, which no staged output has yet been shown
    // to pass.
    let mut items = source.layout.qualification_items.clone();
    for (e, payload) in edits.iter().zip(&payloads) {
        let how = match e.placement {
            Placement::NoOp => continue,
            Placement::FitInSlot { .. } => "fit-in-slot",
            Placement::AppendAndRetarget { .. } => {
                "append-and-retarget (row re-pointed past the original EOF)"
            }
        };
        let framing = chunkzip_framing_note(payload);
        items.push(QualificationItem::new(
            format!("entry {} `{}`: replacement", e.entry_index, e.path),
            QualificationStatus::Unresolved,
            format!(
                "{how}; acceptance of replaced content in this slot by the game is not established{framing}"
            ),
        ));
    }
    items.push(QualificationItem::new(
        "native loader acceptance",
        QualificationStatus::Unresolved,
        "no game-verified load of a staged output exists for this title/platform; structural verification is not game acceptance",
    ));
    let qualification = DeploymentQualification::from_items(items);

    Ok(Plan {
        source: source.identity.clone(),
        layout: source.layout.clone(),
        original_eof,
        new_eof,
        expected_archive_size: header_edit
            .as_ref()
            .map(|h| h.new_archive_size)
            .unwrap_or(source.header.archive_size),
        header_edit,
        edits,
        changed_ranges: normalize(changed),
        deploy_blockers: source.layout.deploy_blockers.clone(),
        qualification,
        new_payloads: payloads,
    })
}

/// The other entries whose stored span overlaps `e`'s (the siblings of a deduplicated block).
fn shared_siblings<'a>(source: &'a BoundSource, e: &BoundEntry) -> Vec<&'a BoundEntry> {
    let (off, end) = (e.offset, e.offset + e.size as u64);
    source
        .entries
        .iter()
        .filter(|o| o.index != e.index && o.offset < end.max(off + 1) && o.offset + (o.size as u64).max(1) > off)
        .collect()
}

/// A note on how a new `chunkzip` payload is framed relative to the native
/// members: single-chunk output of this crate's encoder is not the framing
/// the game ships, and no loader has been shown to accept it.
fn chunkzip_framing_note(stored: &[u8]) -> String {
    if stored.len() < 24 || &stored[..8] != MAGIC_CHUNKZIP {
        return String::new();
    }
    let dummy = u32::from_be_bytes(stored[8..12].try_into().unwrap());
    if dummy > 2 {
        return "; chunkzip legacy (dummy > 2) header, unobserved on this layout".into();
    }
    let chunk_size = u32::from_be_bytes(stored[16..20].try_into().unwrap());
    let n_chunks = u32::from_be_bytes(stored[20..24].try_into().unwrap());
    if chunk_size != NATIVE_CHUNK_SIZE {
        format!(
            "; chunkzip framing differs from native: chunk_size {chunk_size} ({n_chunks} chunk(s)) where every observed member uses {NATIVE_CHUNK_SIZE}; the single-chunk encoder is not qualified for native loading"
        )
    } else {
        // D4: Studio's native-framed encoder (`tdb-archive-codec::compress_chunkzip_native`, real
        // zlib level 9) reproduces every measured stock member byte-for-byte (65 NHL 16–21 CSV
        // members, the two stored-chunk NHL 21 members; research 240/240 PS3+PS4). That is
        // evidence of consistency with EA's tooling, not a loader rule: the item stays open until
        // a game-verified load of a re-encoded member is recorded.
        "; chunkzip framed like native members (128 KiB chunks; the native-framed zlib-9 encoder reproduces every measured stock member byte-for-byte, which is a reproduction target, not proof of loader acceptance)"
            .into()
    }
}

/// Sort and merge ranges; drop empties.
fn normalize(mut v: Vec<Range<u64>>) -> Vec<Range<u64>> {
    v.retain(|r| r.end > r.start);
    v.sort_by_key(|r| (r.start, r.end));
    let mut out: Vec<Range<u64>> = Vec::with_capacity(v.len());
    for r in v {
        if let Some(last) = out.last_mut() {
            if r.start <= last.end {
                last.end = last.end.max(r.end);
                continue;
            }
        }
        out.push(r);
    }
    out
}

/// The complement of `ranges` within `0..len`.
fn complement(ranges: &[Range<u64>], len: u64) -> Vec<Range<u64>> {
    let mut out = Vec::new();
    let mut at = 0;
    for r in ranges {
        let s = r.start.min(len);
        if s > at {
            out.push(at..s);
        }
        at = at.max(r.end.min(len));
    }
    if at < len {
        out.push(at..len);
    }
    out
}

// ================================================================= staging

/// The output prefix (`0..data_start`) with the plan's row and header edits
/// applied. Every other prefix byte is the source's.
fn patched_prefix(source: &BoundSource, plan: &Plan) -> Vec<u8> {
    let mut p = source.prefix.clone();
    for e in &plan.edits {
        for r in e.row_edit.iter().chain(e.sibling_row_edits.iter()) {
            let o = r.row_offset as usize;
            debug_assert_eq!(&p[o..o + r.old_row.len], r.old_row.as_slice());
            p[o..o + r.new_row.len].copy_from_slice(r.new_row.as_slice());
        }
    }
    if let Some(h) = &plan.header_edit {
        p[ARCHIVE_SIZE_OFFSET..ARCHIVE_SIZE_OFFSET + 8]
            .copy_from_slice(&h.new_archive_size.to_be_bytes());
    }
    p
}

/// Stream the planned output from `src` into `out`. `src` is only read. The
/// source is hashed while it is copied and must still match the bound
/// identity, otherwise the result is an error and the output is not to be
/// used. Returns the identities and the changed ranges.
pub fn stage<R: Read + Seek, W: Write>(
    source: &BoundSource,
    plan: &Plan,
    src: &mut R,
    out: &mut W,
) -> Result<StageReport, StageError> {
    if plan.source != source.identity {
        return Err(StageError::PlanSourceMismatch {
            plan_sha: hex(&plan.source.sha256),
            source_sha: hex(&source.identity.sha256),
        });
    }
    let len = src.seek(SeekFrom::End(0))?;
    if len != source.identity.len {
        return Err(StageError::SourceChanged {
            expected_len: source.identity.len,
            expected_sha: hex(&source.identity.sha256),
            actual_len: len,
            actual_sha: "(not computed)".into(),
        });
    }
    src.seek(SeekFrom::Start(0))?;

    let mut src_hash = Sha256::new();
    let mut out_hash = Sha256::new();
    let mut written: u64 = 0;
    let mut buf = vec![0u8; CHUNK];

    // 1. Prefix: consume the source prefix (for the hash), emit the patched one.
    let data_start = source.layout.data_start;
    src.read_exact(buf_for(&mut buf, data_start as usize))?;
    src_hash.update(&buf[..data_start as usize]);
    let prefix = patched_prefix(source, plan);
    emit(out, &mut out_hash, &mut written, &prefix)?;

    // 2. Data region: copy, substituting fit-in-slot spans.
    let mut fits: Vec<(u64, u64, usize)> = plan
        .edits
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match e.placement {
            Placement::FitInSlot { offset, .. } => Some((offset, offset + e.old_size as u64, i)),
            _ => None,
        })
        .collect();
    fits.sort();
    let mut pos = data_start;
    for (off, old_end, i) in fits {
        copy_through(
            src,
            out,
            &mut src_hash,
            &mut out_hash,
            &mut written,
            &mut buf,
            pos,
            off,
        )?;
        // Consume the old span from the source (hash only), emit the new bytes and
        // zero fill up to the old end.
        consume(src, &mut src_hash, &mut buf, off, old_end)?;
        let payload = plan.new_payload(i);
        emit(out, &mut out_hash, &mut written, payload)?;
        let new_end = off + payload.len() as u64;
        if new_end < old_end {
            emit_zeros(
                out,
                &mut out_hash,
                &mut written,
                old_end - new_end,
                &mut buf,
            )?;
        }
        pos = old_end.max(new_end);
        if new_end > old_end {
            // The new bytes extend into the slot's zero padding: skip those source bytes.
            consume(src, &mut src_hash, &mut buf, old_end, new_end)?;
        }
    }
    copy_through(
        src,
        out,
        &mut src_hash,
        &mut out_hash,
        &mut written,
        &mut buf,
        pos,
        len,
    )?;

    // The source must be exactly what was bound.
    let actual: Sha256Bytes = src_hash.finalize().into();
    if actual != source.identity.sha256 {
        return Err(StageError::SourceChanged {
            expected_len: source.identity.len,
            expected_sha: hex(&source.identity.sha256),
            actual_len: len,
            actual_sha: hex(&actual),
        });
    }

    // 3. Appends, each at its planned 16-aligned offset.
    for (i, e) in plan.edits.iter().enumerate() {
        if let Placement::AppendAndRetarget { new_offset } = e.placement {
            debug_assert!(new_offset >= written);
            let pad = new_offset - written;
            emit_zeros(out, &mut out_hash, &mut written, pad, &mut buf)?;
            emit(out, &mut out_hash, &mut written, plan.new_payload(i))?;
        }
    }
    if written < plan.new_eof {
        let pad = plan.new_eof - written;
        emit_zeros(out, &mut out_hash, &mut written, pad, &mut buf)?;
    }
    out.flush()?;
    if written != plan.new_eof {
        return Err(StageError::OutputLengthMismatch {
            expected: plan.new_eof,
            actual: written,
        });
    }

    Ok(StageReport {
        source: source.identity.clone(),
        output: SourceIdentity {
            len: written,
            sha256: out_hash.finalize().into(),
        },
        original_eof: plan.original_eof,
        new_eof: plan.new_eof,
        changed_ranges: plan.changed_ranges.clone(),
        deploy_blockers: plan.deploy_blockers.clone(),
        qualification: plan.qualification.clone(),
    })
}

fn buf_for(buf: &mut Vec<u8>, n: usize) -> &mut [u8] {
    if buf.len() < n {
        buf.resize(n, 0);
    }
    &mut buf[..n]
}

fn emit<W: Write>(
    out: &mut W,
    h: &mut Sha256,
    written: &mut u64,
    bytes: &[u8],
) -> Result<(), StageError> {
    out.write_all(bytes)?;
    h.update(bytes);
    *written += bytes.len() as u64;
    Ok(())
}

fn emit_zeros<W: Write>(
    out: &mut W,
    h: &mut Sha256,
    written: &mut u64,
    n: u64,
    buf: &mut [u8],
) -> Result<(), StageError> {
    let mut left = n;
    while left > 0 {
        let take = left.min(buf.len() as u64) as usize;
        buf[..take].fill(0);
        emit(out, h, written, &buf[..take])?;
        left -= take as u64;
    }
    Ok(())
}

/// Copy `[from, to)` of the source (positioned at `from`) to the output.
#[allow(clippy::too_many_arguments)]
fn copy_through<R: Read, W: Write>(
    src: &mut R,
    out: &mut W,
    sh: &mut Sha256,
    oh: &mut Sha256,
    written: &mut u64,
    buf: &mut [u8],
    from: u64,
    to: u64,
) -> Result<(), StageError> {
    let mut left = to - from;
    while left > 0 {
        let want = left.min(buf.len() as u64) as usize;
        let n = src.read(&mut buf[..want])?;
        if n == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
        }
        sh.update(&buf[..n]);
        emit(out, oh, written, &buf[..n])?;
        left -= n as u64;
    }
    Ok(())
}

/// Read `[from, to)` of the source (positioned at `from`) into the hash only.
fn consume<R: Read>(
    src: &mut R,
    sh: &mut Sha256,
    buf: &mut [u8],
    from: u64,
    to: u64,
) -> Result<(), StageError> {
    let mut left = to - from;
    while left > 0 {
        let want = left.min(buf.len() as u64) as usize;
        let n = src.read(&mut buf[..want])?;
        if n == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
        }
        sh.update(&buf[..n]);
        left -= n as u64;
    }
    Ok(())
}

// ================================================================= verification

/// Verify a staged output against its plan and source: re-parse, check every
/// planned row, hash and decode every changed entry and compare it with the
/// expected decoded content, compare every unchanged row, and compare every
/// untouched byte of `0..original_eof` with the source.
pub fn verify<R: Read + Seek, S: Read + Seek>(
    source: &BoundSource,
    plan: &Plan,
    report: &StageReport,
    output: &mut R,
    src: &mut S,
) -> Result<VerificationReport, StageError> {
    let mut v = VerificationReport::default();

    // Length and hash of the output.
    let len = output.seek(SeekFrom::End(0))?;
    if len != plan.new_eof {
        return Err(StageError::OutputLengthMismatch {
            expected: plan.new_eof,
            actual: len,
        });
    }
    output.seek(SeekFrom::Start(0))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    let mut left = len;
    while left > 0 {
        let n = output.read(&mut buf[..left.min(CHUNK as u64) as usize])?;
        if n == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
        }
        h.update(&buf[..n]);
        left -= n as u64;
    }
    let out_sha: Sha256Bytes = h.finalize().into();
    if out_sha != report.output.sha256 {
        return Err(StageError::OutputHashMismatch {
            expected: hex(&report.output.sha256),
            actual: hex(&out_sha),
        });
    }

    // Re-parse.
    output.seek(SeekFrom::Start(0))?;
    let mut head = vec![0u8; HEADER_LEN];
    output.read_exact(&mut head)?;
    let header = EbHeader::parse(&head)?;
    if header.archive_size != plan.expected_archive_size {
        return Err(StageError::OutputArchiveSizeMismatch {
            expected: plan.expected_archive_size,
            actual: header.archive_size,
        });
    }
    // Same protection as bind: no allocation from unchecked header claims.
    let tables = header.table_layout(len)?;
    if tables.data_start > len {
        return Err(crate::eb::EbError::TruncatedTables {
            needed: usize::try_from(tables.data_start).unwrap_or(usize::MAX),
            have: usize::try_from(len).unwrap_or(usize::MAX),
        }
        .into());
    }
    let mut prefix = vec![0u8; tables.table_end as usize];
    output.seek(SeekFrom::Start(0))?;
    output.read_exact(&mut prefix)?;
    let arc = EbArchive::parse(&prefix)?;
    if arc.entries.len() != source.entries.len() {
        return Err(StageError::OutputEntryCountMismatch {
            expected: source.entries.len(),
            actual: arc.entries.len(),
        });
    }
    v.entries_reparsed = arc.entries.len();

    // Rows: planned for edited entries, identical for the rest.
    let expected_prefix = patched_prefix(source, plan);
    for (i, e) in source.entries.iter().enumerate() {
        let row_len = source.layout.row_len;
        let o = FILE_TABLE_OFFSET as usize + i * row_len;
        let actual: Vec<u8> = prefix[o..o + row_len].to_vec();
        let expected: Vec<u8> = expected_prefix[o..o + row_len].to_vec();
        if actual != expected {
            return Err(StageError::RowMismatch {
                index: i,
                path: e.path.clone(),
                expected,
                actual,
            });
        }
        if !plan.edits.iter().any(|pe| pe.entry_index == i || pe.sibling_row_edits.iter().any(|s| s.row_offset as usize == o)) {
            v.unchanged_rows_compared += 1;
        }
    }

    // Changed entries: stored digest, decode, decoded digest.
    for e in &plan.edits {
        let entry = &arc.entries[e.entry_index];
        let (want_off, want_size) = match e.placement {
            Placement::NoOp => (e.old_offset, e.old_size),
            Placement::FitInSlot { offset, .. } => (offset, e.new_size),
            Placement::AppendAndRetarget { new_offset } => (new_offset, e.new_size),
        };
        if entry.offset != want_off || entry.size != want_size || entry.path != e.path {
            let row_len = source.layout.row_len;
            let o = FILE_TABLE_OFFSET as usize + e.entry_index * row_len;
            return Err(StageError::RowMismatch {
                index: e.entry_index,
                path: e.path.clone(),
                expected: expected_prefix[o..o + row_len].to_vec(),
                actual: prefix[o..o + row_len].to_vec(),
            });
        }
        let mut stored = vec![0u8; want_size as usize];
        output.seek(SeekFrom::Start(want_off))?;
        output.read_exact(&mut stored)?;
        let sha: Sha256Bytes = Sha256::digest(&stored).into();
        if sha != e.new_stored_sha256 {
            return Err(StageError::StoredMismatch {
                index: e.entry_index,
                path: e.path.clone(),
                expected: hex(&e.new_stored_sha256),
                actual: hex(&sha),
            });
        }
        check_decoded(e.entry_index, &e.path, stored, &e.expected_decoded)?;
        v.changed_entries_decoded += 1;
        // Whole-group siblings point at the same new span.
        for s in &e.sibling_row_edits {
            let si = (s.row_offset as usize - FILE_TABLE_OFFSET as usize) / source.layout.row_len;
            let sib = &arc.entries[si];
            if sib.offset != want_off || sib.size != want_size {
                return Err(StageError::RowMismatch { index: si, path: sib.path.clone(), expected: s.new_row.as_slice().to_vec(), actual: prefix[s.row_offset as usize..s.row_offset as usize + source.layout.row_len].to_vec() });
            }
        }
    }

    // Untouched bytes of the original span: byte-equal to the source.
    let mut sbuf = vec![0u8; CHUNK];
    for r in complement(&plan.changed_ranges, plan.original_eof) {
        output.seek(SeekFrom::Start(r.start))?;
        src.seek(SeekFrom::Start(r.start))?;
        let mut at = r.start;
        while at < r.end {
            let want = (r.end - at).min(CHUNK as u64) as usize;
            output.read_exact(&mut buf[..want])?;
            src.read_exact(&mut sbuf[..want])?;
            if buf[..want] != sbuf[..want] {
                let k = buf[..want]
                    .iter()
                    .zip(&sbuf[..want])
                    .position(|(a, b)| a != b)
                    .unwrap();
                return Err(StageError::UntouchedBytesDiffer {
                    start: r.start,
                    end: r.end,
                    at: at + k as u64,
                });
            }
            at += want as u64;
            v.untouched_bytes_compared += want as u64;
        }
    }

    v.output = Some(SourceIdentity {
        len,
        sha256: out_sha,
    });
    Ok(v)
}

// ================================================================= files

/// Stage `plan` from the file at `source_path` into a **new** file at
/// `output_path` (refused if it exists), then verify it. On any failure the
/// partial output is removed and the source is untouched.
pub fn stage_file(
    source: &BoundSource,
    plan: &Plan,
    source_path: &std::path::Path,
    output_path: &std::path::Path,
) -> Result<(StageReport, VerificationReport), StageError> {
    let mut src = std::fs::File::open(source_path)?;
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)?;
    let result = (|| {
        let report = {
            let mut w = io::BufWriter::new(&mut out);
            stage(source, plan, &mut src, &mut w)?
        };
        out.sync_all()?;
        let mut check = std::fs::File::open(output_path)?;
        let v = verify(source, plan, &report, &mut check, &mut src)?;
        Ok((report, v))
    })();
    if result.is_err() {
        drop(out);
        let _ = std::fs::remove_file(output_path);
    }
    result
}

#[doc(hidden)]
pub mod synth;

#[cfg(test)]
mod tests;
