//! BIG4 + Viv4 `.bh` member placement — the outer archives of NHL 07–10 (Xbox 360) and NHL
//! 08–10 (PS3): research handoff 02 (2026-09-19) §1.1 and §4.3.
//!
//! Every BIG4 archive of that era ships with a **sidecar index** `<name>.bh` (`Viv4`) that repeats
//! each entry's offset, stored size, decoded length (RefPack members) or 0 (raw) and a djb2 hash of
//! the path, plus a trailer carrying the `.big` length. A member whose stored length changes must
//! therefore be patched in **both** files, as one transaction. Measured facts this module binds to
//! (9/9 archives): header `size_le` == file length; every entry offset 64-byte aligned; index order
//! == offset order; inter-entry gaps 0–63 bytes; `.bh` count/offset/size equal to the directory on
//! every record; `.bh` word 8 == RefPack declared length on RefPack members and 0 on raw ones;
//! `.bh` word 12 == djb2(path); trailer word 0 == `.big` length.
//!
//! Placement of a replacement of stored length `L` in a slot of `S` bytes:
//! * **fit-in-slot** when `L ≤ S + slack`, where `slack` counts only the gap bytes to the next
//!   entry that were **read and verified zero** (research 02 F2 — a non-zero gap byte is
//!   preserved, so the usable slack ends before it): the payload is written at the entry's offset,
//!   the rest of the old slot zero-filled, the directory `size` and the `.bh` record words 4/8
//!   patched. Nothing else moves.
//! * **append-and-retarget** otherwise: the payload at `align64(EOF)`, the directory `offset`+`size`,
//!   the header `size_le`, the `.bh` record words 0/4/8 and the trailer's length word patched; the
//!   old bytes stay in place (dead). Structurally consistent with everything the loader can read
//!   from either index, but an entry whose offset order breaks the index order is a shape **no
//!   stock archive exhibits** (research 02 §4.3, card R2-BIG4-1) — the plan says so and the caller
//!   decides (`Growth::Append` / `Growth::Refuse`).
//! * The **ordered reflow** (every later entry shifted, the stock shape) is not implemented: it
//!   rewrites hundreds of MB and its `.bh` consistency proof is the same as the append's.
//!
//! Refused: a `.bh` that is missing or disagrees with the directory (count, any offset/size, any
//! hash, trailer length), a header `size_le` that is not the file length, an offset that is not
//! 64-aligned, overlapping entries, a replacement whose stored bytes do not match the bound
//! identity, an RefPack decoded length above the 3-byte field, offsets beyond `u32`, and — through
//! `Growth::Refuse` — any placement the caller has not authorised (NHL 07's `dataN.big`, whose `.bh`
//! is mirrored inside `boot.big`: a `.bh` change there is a cascade this module does not plan).

use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Range;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::stage::{hex, path_hash, Sha256Bytes};

const CHUNK: usize = 1 << 20;

#[derive(Debug, Error)]
pub enum Big4Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a BIG4 archive (magic {0:?})")]
    BadMagic([u8; 4]),
    #[error("archive header size_le {size_le} is not the file length {len}; the header word's meaning on this archive is not established")]
    SizeWordUnexplained { size_le: u32, len: u64 },
    #[error("directory truncated at entry {0}")]
    TruncatedDirectory(usize),
    #[error("entry {index} `{path}` offset {offset} is not 64-byte aligned")]
    Unaligned { index: usize, path: String, offset: u32 },
    #[error("entry {index} `{path}` span {offset}+{size} lies outside the payload region {data_start}..{len}")]
    OutOfRange { index: usize, path: String, offset: u32, size: u32, data_start: u64, len: u64 },
    #[error("entries `{a}` and `{b}` overlap")]
    Overlap { a: String, b: String },
    #[error("sidecar: not a Viv4 index (magic {0:?})")]
    BadSidecarMagic([u8; 4]),
    #[error("sidecar: {0}")]
    SidecarShape(String),
    #[error("sidecar record {index} `{path}` disagrees with the directory: {what}")]
    SidecarMismatch { index: usize, path: String, what: String },
    #[error("sidecar trailer length {trailer} is not the archive length {len}")]
    SidecarTrailer { trailer: u32, len: u64 },
    #[error("no such entry {index} (count {count})")]
    NoSuchEntry { index: usize, count: usize },
    #[error("entry {index}: expected path `{expected}`, found `{actual}`")]
    PathMismatch { index: usize, expected: String, actual: String },
    #[error("entry {index} `{path}`: stored bytes {actual} are not the bound {expected}; the archive changed since the plan was made")]
    StoredHashMismatch { index: usize, path: String, expected: String, actual: String },
    #[error("entry {index} `{path}` is named twice")]
    Duplicate { index: usize, path: String },
    #[error("entry {index} `{path}`: the {len}-byte replacement does not fit its slot ({slot} bytes incl. {slack} verified-zero slack) and growth is refused: {reason}")]
    GrowthRefused { index: usize, path: String, len: usize, slot: u64, slack: u64, reason: String },
    #[error("entry {index} `{path}`: RefPack decoded length {0} exceeds the 3-byte field", .decoded)]
    DecodedTooLarge { index: usize, path: String, decoded: u64 },
    #[error("entry {index} `{path}`: the `.bh` decoded-length word {given} does not describe the replacement ({expected}); the measured contract is the RefPack declared length on a RefPack member and 0 on a raw one")]
    DecodedWordInconsistent { index: usize, path: String, given: u32, expected: String },
    #[error("header: {0}")]
    HeaderShape(String),
    #[error("the staged archive would be {0} bytes, beyond the 32-bit offsets of the format")]
    TooLarge(u64),
    #[error("verification: {0}")]
    Verification(String),
}

/// One directory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Big4Entry {
    pub index: usize,
    pub path: String,
    pub offset: u32,
    pub size: u32,
    /// Position of the row's offset word in the archive (size follows at +4).
    pub row_pos: usize,
}

/// A parsed `.bh` sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Viv4 {
    pub count: usize,
    /// `[offset, stored size, decoded length or 0, djb2(path)]` per entry.
    pub records: Vec<[u32; 4]>,
    /// The 16-byte trailer (`[u32 BE archive length][12 zero bytes]` on every measured file).
    pub trailer: [u8; 16],
    pub bytes: Vec<u8>,
}

impl Viv4 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Big4Error> {
        if bytes.len() < 16 {
            return Err(Big4Error::SidecarShape(format!("{} bytes is shorter than the 16-byte header", bytes.len())));
        }
        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        if &magic != b"Viv4" {
            return Err(Big4Error::BadSidecarMagic(magic));
        }
        let size_le = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let count = u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let index_size = u32::from_be_bytes(bytes[12..16].try_into().unwrap());
        if size_le as usize != bytes.len() || index_size as usize != bytes.len() {
            return Err(Big4Error::SidecarShape(format!("size words {size_le}/{index_size} are not the file length {}", bytes.len())));
        }
        if 16 + count * 16 + 16 != bytes.len() {
            return Err(Big4Error::SidecarShape(format!("{count} records + trailer do not account for {} bytes", bytes.len())));
        }
        let mut records = Vec::with_capacity(count);
        for i in 0..count {
            let r = &bytes[16 + i * 16..32 + i * 16];
            records.push([
                u32::from_be_bytes(r[0..4].try_into().unwrap()),
                u32::from_be_bytes(r[4..8].try_into().unwrap()),
                u32::from_be_bytes(r[8..12].try_into().unwrap()),
                u32::from_be_bytes(r[12..16].try_into().unwrap()),
            ]);
        }
        let trailer: [u8; 16] = bytes[bytes.len() - 16..].try_into().unwrap();
        Ok(Self { count, records, trailer, bytes: bytes.to_vec() })
    }

    fn record_pos(i: usize) -> usize {
        16 + i * 16
    }

    pub fn trailer_len(&self) -> u32 {
        u32::from_be_bytes(self.trailer[0..4].try_into().unwrap())
    }
}

/// Identity of a bound archive + sidecar pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Big4Identity {
    pub len: u64,
    pub sha256: Sha256Bytes,
    pub sidecar_len: u64,
    pub sidecar_sha256: Sha256Bytes,
}

/// A BIG4 archive and its `.bh`, parsed and cross-checked; the plan and stager bind to it.
#[derive(Debug, Clone)]
pub struct BoundBig4 {
    pub identity: Big4Identity,
    pub size_le: u32,
    pub index_size: u32,
    /// Bytes `0..min(16 + index_size, data_start)` (header, directory, the `L2xx` trailer as far
    /// as it lies before the first payload line) verbatim — the region the stager rewrites.
    pub index_bytes: Vec<u8>,
    /// First 64-byte line at or after the end of the parsed directory (the last entry's name
    /// terminator). Measured 20 Sep on all seven stock BIG4 archives: equals the lowest entry
    /// offset everywhere. It is **not** `align64(16 + index_size)`: PS3 NHL 10 `fe.big` declares
    /// an index size whose 24-byte `L286` trailer runs 10 bytes past the first payload line
    /// (`16 + index_size = 624 842`, first entry at `624 832`), so the declared word overstates.
    pub data_start: u64,
    /// `16 + index_size` as declared; greater than `data_start` on PS3 NHL 10 (see `data_start`).
    pub declared_index_end: u64,
    pub entries: Vec<Big4Entry>,
    /// Entry indices sorted by offset (== directory order on every stock archive; an entry a
    /// previous append re-pointed past the EOF breaks it — `index_order_is_offset_order`).
    pub offset_order: Vec<usize>,
    pub index_order_is_offset_order: bool,
    pub sidecar: Viv4,
}

fn align64(x: u64) -> u64 {
    x.div_ceil(64) * 64
}

fn sha_file<R: Read + Seek>(r: &mut R) -> Result<(u64, Sha256Bytes), Big4Error> {
    r.seek(SeekFrom::Start(0))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    let mut n = 0u64;
    loop {
        let k = r.read(&mut buf)?;
        if k == 0 {
            break;
        }
        h.update(&buf[..k]);
        n += k as u64;
    }
    Ok((n, h.finalize().into()))
}

impl BoundBig4 {
    /// Parse the archive's header + directory from `big` and the sidecar bytes; cross-check every
    /// record; hash both files.
    pub fn bind<R: Read + Seek>(big: &mut R, sidecar: &[u8]) -> Result<Self, Big4Error> {
        let (len, sha256) = sha_file(big)?;
        Self::parse_with(big, sidecar, len, sha256)
    }

    /// The directory only (26 Sep 2026): the same header / directory / range / overlap / sidecar
    /// cross-checks as [`BoundBig4::bind`] **without hashing the archive**, for read-only member
    /// lookups that run per request. It carries no identity, so nothing may plan or stage from it.
    pub fn read_directory<R: Read + Seek>(big: &mut R, sidecar: &[u8]) -> Result<Vec<Big4Entry>, Big4Error> {
        let len = big.seek(SeekFrom::End(0))?;
        Ok(Self::parse_with(big, sidecar, len, [0u8; 32])?.entries)
    }

    fn parse_with<R: Read + Seek>(big: &mut R, sidecar: &[u8], len: u64, sha256: Sha256Bytes) -> Result<Self, Big4Error> {
        big.seek(SeekFrom::Start(0))?;
        let mut hdr = [0u8; 16];
        big.read_exact(&mut hdr)?;
        let magic: [u8; 4] = hdr[0..4].try_into().unwrap();
        if &magic != b"BIG4" {
            return Err(Big4Error::BadMagic(magic));
        }
        let size_le = u32::from_le_bytes(hdr[4..8].try_into().unwrap());
        let count = u32::from_be_bytes(hdr[8..12].try_into().unwrap()) as usize;
        let index_size = u32::from_be_bytes(hdr[12..16].try_into().unwrap());
        if size_le as u64 != len {
            return Err(Big4Error::SizeWordUnexplained { size_le, len });
        }
        // Bounded before any allocation: the directory lies inside the file and every entry needs
        // at least its 8-byte row plus a name terminator.
        if 16 + index_size as u64 > len {
            return Err(Big4Error::HeaderShape(format!("declared index size {index_size} runs past the {len}-byte file")));
        }
        if count as u64 * 9 > index_size as u64 {
            return Err(Big4Error::HeaderShape(format!("{count} entries cannot fit a {index_size}-byte directory")));
        }
        let mut index_bytes = vec![0u8; 16 + index_size as usize];
        index_bytes[..16].copy_from_slice(&hdr);
        big.read_exact(&mut index_bytes[16..])?;
        let mut cursor = 16usize;
        let mut entries = Vec::with_capacity(count);
        for index in 0..count {
            if cursor + 8 > index_bytes.len() {
                return Err(Big4Error::TruncatedDirectory(index));
            }
            let row_pos = cursor;
            let offset = u32::from_be_bytes(index_bytes[cursor..cursor + 4].try_into().unwrap());
            let size = u32::from_be_bytes(index_bytes[cursor + 4..cursor + 8].try_into().unwrap());
            cursor += 8;
            let Some(rel) = index_bytes[cursor..].iter().position(|b| *b == 0) else {
                return Err(Big4Error::TruncatedDirectory(index));
            };
            let path = String::from_utf8_lossy(&index_bytes[cursor..cursor + rel]).replace('\\', "/");
            cursor += rel + 1;
            entries.push(Big4Entry { index, path, offset, size, row_pos });
        }
        let declared_index_end = 16 + index_size as u64;
        let data_start = align64(cursor as u64);
        if declared_index_end < cursor as u64 {
            return Err(Big4Error::TruncatedDirectory(count));
        }
        if declared_index_end > data_start {
            // The declared trailer overlaps the first payload line: keep only what lies before it
            // so the rewritten prefix never touches payload bytes.
            index_bytes.truncate(data_start as usize);
        }
        for e in &entries {
            if e.offset % 64 != 0 {
                return Err(Big4Error::Unaligned { index: e.index, path: e.path.clone(), offset: e.offset });
            }
            let end = e.offset as u64 + e.size as u64;
            if (e.offset as u64) < data_start || end > len {
                return Err(Big4Error::OutOfRange { index: e.index, path: e.path.clone(), offset: e.offset, size: e.size, data_start, len });
            }
        }
        let mut offset_order: Vec<usize> = (0..entries.len()).collect();
        offset_order.sort_by_key(|i| (entries[*i].offset, *i));
        let index_order_is_offset_order = offset_order.iter().enumerate().all(|(k, i)| k == *i);
        for w in offset_order.windows(2) {
            let (a, b) = (&entries[w[0]], &entries[w[1]]);
            if a.size > 0 && b.offset < a.offset + a.size {
                return Err(Big4Error::Overlap { a: a.path.clone(), b: b.path.clone() });
            }
        }
        let viv = Viv4::parse(sidecar)?;
        if viv.count != count {
            return Err(Big4Error::SidecarShape(format!("{} records for {count} directory entries", viv.count)));
        }
        for (e, r) in entries.iter().zip(&viv.records) {
            if r[0] != e.offset || r[1] != e.size {
                return Err(Big4Error::SidecarMismatch { index: e.index, path: e.path.clone(), what: format!("record offset/size {}+{} vs directory {}+{}", r[0], r[1], e.offset, e.size) });
            }
            let h = path_hash(&e.path);
            if r[3] != h {
                return Err(Big4Error::SidecarMismatch { index: e.index, path: e.path.clone(), what: format!("record hash {:#010x} is not djb2 of the path ({h:#010x})", r[3]) });
            }
        }
        if viv.trailer_len() as u64 != len {
            return Err(Big4Error::SidecarTrailer { trailer: viv.trailer_len(), len });
        }
        if viv.trailer[4..].iter().any(|b| *b != 0) {
            return Err(Big4Error::SidecarShape("trailer bytes 4..16 are not zero".into()));
        }
        let sidecar_sha256: Sha256Bytes = Sha256::digest(sidecar).into();
        Ok(Self {
            identity: Big4Identity { len, sha256, sidecar_len: sidecar.len() as u64, sidecar_sha256 },
            size_le,
            index_size,
            index_bytes,
            data_start,
            declared_index_end,
            entries,
            offset_order,
            index_order_is_offset_order,
            sidecar: viv,
        })
    }

    /// The slot end of entry `i`: the next entry's offset in offset order, or the archive
    /// length for the last one.
    pub fn slot_end(&self, i: usize) -> u64 {
        let k = self.offset_order.iter().position(|x| *x == i).expect("entry is in the order");
        self.offset_order.get(k + 1).map(|n| self.entries[*n].offset as u64).unwrap_or(self.identity.len)
    }
}

/// What the caller allows when a replacement does not fit its slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Growth {
    /// Append past the EOF and re-point the entry (index order ≠ offset order afterwards — a
    /// shape no stock archive exhibits; card R2-BIG4-1).
    Append,
    /// **Ordered reflow** — the stock shape (research 2026-09-20 handoff 03 §2): the entry keeps
    /// its offset, its slot grows to `align64(offset + L)`, and everything from the old slot end
    /// to the EOF moves up by Δ unchanged (later entries' offsets, their `.bh` words 0, the
    /// header `size_le` and the `.bh` trailer += Δ; gap bytes travel with their entries). The
    /// gap right after the grown entry must be all zero (a non-zero gap byte would be lost — it
    /// is never erased, research 02 F2), else the growth is refused.
    Reflow,
    /// Refuse with this reason (e.g. NHL 07 `dataN.big`: the `.bh` is mirrored in `boot.big`).
    Refuse(String),
}

/// One requested replacement.
#[derive(Debug, Clone)]
pub struct Big4Replacement {
    pub entry_index: usize,
    pub expected_path: String,
    pub expected_stored_sha256: Sha256Bytes,
    pub new_stored: Vec<u8>,
    /// The `.bh` word 8: the RefPack declared decoded length, or 0 for a raw member.
    pub decoded_len: u32,
    /// This replacement's own growth policy; `None` takes the plan-wide one. Lets a review carry
    /// the placement it predicted per member (fit → `Refuse`, reflow, append) so the apply does
    /// exactly what was previewed.
    pub growth: Option<Growth>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Big4Placement {
    NoOp,
    FitInSlot { offset: u64, slot_end: u64, zero_slack: u64 },
    AppendAndRetarget { new_offset: u64 },
    /// The entry keeps `offset` (as shifted by earlier reflows, `new_offset`); its slot end moves
    /// from `old_slot_end` to `old_slot_end + delta`; every source byte from `old_slot_end` on
    /// moves up by `delta` (cumulative with earlier reflows).
    Reflow { new_offset: u64, old_slot_end: u64, delta: u64 },
}

#[derive(Debug, Clone)]
pub struct Big4Edit {
    pub entry_index: usize,
    pub path: String,
    pub old_offset: u32,
    pub old_size: u32,
    pub new_size: u32,
    pub old_decoded_len: u32,
    pub new_decoded_len: u32,
    pub new_stored_sha256: Sha256Bytes,
    pub placement: Big4Placement,
}

/// One piece of the output archive, in output order: a verbatim copy of a source span, a
/// planned payload (by edit index) or zero fill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Copy { src: Range<u64>, dst: u64 },
    Payload { edit: usize, dst: u64 },
    Zero { dst: u64, len: u64 },
}

/// A planned BIG4 + `.bh` rewrite.
#[derive(Debug, Clone)]
pub struct Big4Plan {
    pub source: Big4Identity,
    pub original_len: u64,
    pub new_len: u64,
    pub edits: Vec<Big4Edit>,
    /// Payloads per edit (empty for a no-op).
    payloads: Vec<Vec<u8>>,
    /// The patched `0..16 + index_size` region.
    pub new_index_bytes: Vec<u8>,
    /// The complete new sidecar.
    pub new_sidecar: Vec<u8>,
    /// The output, piece by piece (contiguous, covering `0..new_len` exactly). Every `Copy` is
    /// verified byte-equal to its source span; the verifier proves coverage.
    pub segments: Vec<Segment>,
    /// Ranges of the output archive that are not verbatim copies of the source at the same
    /// position: the index region, payloads, zero fill and (after a reflow) every moved byte.
    pub changed_ranges: Vec<Range<u64>>,
    /// Plain-language facts the review must show.
    pub notes: Vec<String>,
}

impl Big4Plan {
    pub fn is_noop(&self) -> bool {
        self.edits.iter().all(|e| e.placement == Big4Placement::NoOp)
    }
    /// The total shift a reflow applied to the tail (0 without reflow).
    pub fn reflow_shift(&self) -> u64 {
        self.edits.iter().filter_map(|e| if let Big4Placement::Reflow { delta, .. } = e.placement { Some(delta) } else { None }).sum()
    }
}

fn normalize(mut r: Vec<Range<u64>>) -> Vec<Range<u64>> {
    r.retain(|x| x.start < x.end);
    r.sort_by_key(|x| x.start);
    let mut out: Vec<Range<u64>> = Vec::new();
    for x in r {
        if let Some(last) = out.last_mut() {
            if x.start <= last.end {
                last.end = last.end.max(x.end);
                continue;
            }
        }
        out.push(x);
    }
    out
}

/// Plan the replacements. `src` is read for the target entries' stored bytes (identity check)
/// and the gap bytes after each slot (verified-zero slack).
pub fn plan<R: Read + Seek>(bound: &BoundBig4, src: &mut R, replacements: Vec<Big4Replacement>, growth: Growth) -> Result<Big4Plan, Big4Error> {
    let count = bound.entries.len();
    let mut reqs = replacements;
    reqs.sort_by_key(|r| r.entry_index);
    for w in reqs.windows(2) {
        if w[0].entry_index == w[1].entry_index {
            return Err(Big4Error::Duplicate { index: w[0].entry_index, path: w[0].expected_path.clone() });
        }
    }
    // Validate every request and classify it; placements are laid out afterwards in offset order.
    enum Kind {
        NoOp,
        Fit { zero_slack: u64, slot_end: u64 },
        Grow { slot_end: u64, reflow: bool },
    }
    struct Req {
        r: Big4Replacement,
        new_sha: Sha256Bytes,
        old_decoded: u32,
        kind: Kind,
    }
    let mut checked: Vec<Req> = Vec::new();
    let mut notes = Vec::new();
    for r in reqs {
        let e = bound.entries.get(r.entry_index).ok_or(Big4Error::NoSuchEntry { index: r.entry_index, count })?;
        if e.path != r.expected_path {
            return Err(Big4Error::PathMismatch { index: e.index, expected: r.expected_path.clone(), actual: e.path.clone() });
        }
        // Identity of the stored bytes now.
        src.seek(SeekFrom::Start(e.offset as u64))?;
        let mut cur = vec![0u8; e.size as usize];
        src.read_exact(&mut cur)?;
        let cur_sha: Sha256Bytes = Sha256::digest(&cur).into();
        if cur_sha != r.expected_stored_sha256 {
            return Err(Big4Error::StoredHashMismatch { index: e.index, path: e.path.clone(), expected: hex(&r.expected_stored_sha256), actual: hex(&cur_sha) });
        }
        if r.decoded_len as u64 > 0xFF_FFFF {
            return Err(Big4Error::DecodedTooLarge { index: e.index, path: e.path.clone(), decoded: r.decoded_len as u64 });
        }
        let new_sha: Sha256Bytes = Sha256::digest(&r.new_stored).into();
        let new_size = r.new_stored.len();
        let old_decoded = bound.sidecar.records[e.index][2];
        if new_sha == cur_sha && r.decoded_len == old_decoded {
            checked.push(Req { r, new_sha, old_decoded, kind: Kind::NoOp });
            continue;
        }
        // The `.bh` word 8 must describe the bytes actually placed (measured contract): the
        // RefPack header's declared length on a wrapped member, 0 on a raw one.
        let expected_word = if tdb_archive_codec::is_refpack(&r.new_stored) {
            match tdb_archive_codec::parse_refpack_header(&r.new_stored) {
                Some(h) => Some(h.uncompressed_len as u64),
                None => return Err(Big4Error::DecodedWordInconsistent { index: e.index, path: e.path.clone(), given: r.decoded_len, expected: "a RefPack member whose header is truncated".into() }),
            }
        } else {
            Some(0)
        };
        if expected_word != Some(r.decoded_len as u64) {
            let expected = if expected_word == Some(0) { "0 for a raw member".to_string() } else { format!("the RefPack declared length {}", expected_word.unwrap_or(0)) };
            return Err(Big4Error::DecodedWordInconsistent { index: e.index, path: e.path.clone(), given: r.decoded_len, expected });
        }
        if new_size > u32::MAX as usize {
            return Err(Big4Error::TooLarge(new_size as u64));
        }
        // Verified-zero slack: read the gap up to the next entry.
        let old_end = e.offset as u64 + e.size as u64;
        let slot_end = bound.slot_end(e.index);
        let gap_len = slot_end - old_end;
        let mut gap = vec![0u8; gap_len as usize];
        src.seek(SeekFrom::Start(old_end))?;
        src.read_exact(&mut gap)?;
        let zero_slack = gap.iter().position(|b| *b != 0).unwrap_or(gap_len as usize) as u64;
        if zero_slack != gap_len {
            notes.push(format!("entry {} `{}`: {} of the {gap_len} gap bytes after the slot are not zero and are preserved; usable slack is {zero_slack} bytes", e.index, e.path, gap_len - zero_slack));
        }
        let fits = e.offset as u64 + new_size as u64 <= old_end + zero_slack;
        let policy = r.growth.clone().unwrap_or_else(|| growth.clone());
        let kind = if fits {
            Kind::Fit { zero_slack, slot_end }
        } else {
            match &policy {
                Growth::Refuse(reason) => {
                    return Err(Big4Error::GrowthRefused { index: e.index, path: e.path.clone(), len: new_size, slot: slot_end - e.offset as u64, slack: zero_slack, reason: reason.clone() });
                }
                Growth::Reflow if zero_slack != gap_len => {
                    return Err(Big4Error::GrowthRefused { index: e.index, path: e.path.clone(), len: new_size, slot: slot_end - e.offset as u64, slack: zero_slack, reason: format!("the ordered reflow would overwrite the {} non-zero gap byte(s) after the slot, which are never erased; append is the alternative", gap_len - zero_slack) });
                }
                Growth::Append | Growth::Reflow => {}
            }
            Kind::Grow { slot_end, reflow: policy == Growth::Reflow }
        };
        checked.push(Req { r, new_sha, old_decoded, kind });
    }

    // ---- lay out, in source offset order
    let mut order: Vec<usize> = (0..checked.len()).collect();
    order.sort_by_key(|&k| bound.entries[checked[k].r.entry_index].offset);
    let mut edits: Vec<Option<Big4Edit>> = (0..checked.len()).map(|_| None).collect();
    let mut segments: Vec<Segment> = Vec::new();
    let mut changed: Vec<Range<u64>> = Vec::new();
    let mut new_index = bound.index_bytes.clone();
    let mut new_bh = bound.sidecar.bytes.clone();
    // The index region is emitted first, then the source is walked from its end.
    let index_len = bound.index_bytes.len() as u64;
    changed.push(0..index_len);
    let mut src_pos = index_len; // next source byte to copy
    let mut out_pos = index_len; // next output byte
    let mut shift: u64 = 0; // cumulative reflow shift for source bytes at/after `src_pos`
    let mut reflow_shifts: Vec<(u64, u64)> = Vec::new(); // (old_slot_end, delta), in order
    let mut appends: Vec<usize> = Vec::new();
    for &k in &order {
        let req = &checked[k];
        let e = &bound.entries[req.r.entry_index];
        let new_size = req.r.new_stored.len() as u64;
        let rec = Viv4::record_pos(e.index);
        match &req.kind {
            Kind::NoOp => {
                edits[k] = Some(Big4Edit { entry_index: e.index, path: e.path.clone(), old_offset: e.offset, old_size: e.size, new_size: e.size, old_decoded_len: req.old_decoded, new_decoded_len: req.old_decoded, new_stored_sha256: req.new_sha, placement: Big4Placement::NoOp });
            }
            Kind::Fit { zero_slack, slot_end } => {
                let off = e.offset as u64;
                debug_assert!(off >= src_pos);
                segments.push(Segment::Copy { src: src_pos..off, dst: out_pos });
                out_pos += off - src_pos;
                let new_off = out_pos;
                segments.push(Segment::Payload { edit: k, dst: out_pos });
                out_pos += new_size;
                let old_end = off + e.size as u64;
                if old_end > off + new_size {
                    // Shrink: zero fill to the old end (the gap after it is carried verbatim).
                    segments.push(Segment::Zero { dst: out_pos, len: old_end - (off + new_size) });
                    out_pos += old_end - (off + new_size);
                }
                let resume = old_end.max(off + new_size);
                changed.push(new_off..new_off + (resume - off));
                src_pos = resume;
                new_index[e.row_pos + 4..e.row_pos + 8].copy_from_slice(&(new_size as u32).to_be_bytes());
                new_bh[rec + 4..rec + 8].copy_from_slice(&(new_size as u32).to_be_bytes());
                new_bh[rec + 8..rec + 12].copy_from_slice(&req.r.decoded_len.to_be_bytes());
                if shift != 0 {
                    new_index[e.row_pos..e.row_pos + 4].copy_from_slice(&(new_off as u32).to_be_bytes());
                    new_bh[rec..rec + 4].copy_from_slice(&(new_off as u32).to_be_bytes());
                }
                edits[k] = Some(Big4Edit { entry_index: e.index, path: e.path.clone(), old_offset: e.offset, old_size: e.size, new_size: new_size as u32, old_decoded_len: req.old_decoded, new_decoded_len: req.r.decoded_len, new_stored_sha256: req.new_sha, placement: Big4Placement::FitInSlot { offset: new_off, slot_end: *slot_end + shift, zero_slack: *zero_slack } });
            }
            Kind::Grow { slot_end, reflow: true } => {
                let off = e.offset as u64;
                debug_assert!(off >= src_pos);
                segments.push(Segment::Copy { src: src_pos..off, dst: out_pos });
                out_pos += off - src_pos;
                let new_off = out_pos;
                let new_slot_end_out = align64(new_off + new_size);
                let delta = new_slot_end_out - (*slot_end + shift);
                debug_assert!(delta > 0, "a growth that needs no shift would have fitted");
                if new_slot_end_out > u32::MAX as u64 {
                    return Err(Big4Error::TooLarge(new_slot_end_out));
                }
                segments.push(Segment::Payload { edit: k, dst: out_pos });
                out_pos += new_size;
                segments.push(Segment::Zero { dst: out_pos, len: new_slot_end_out - out_pos });
                out_pos = new_slot_end_out;
                changed.push(new_off..new_slot_end_out);
                src_pos = *slot_end;
                shift += delta;
                reflow_shifts.push((*slot_end, delta));
                new_index[e.row_pos + 4..e.row_pos + 8].copy_from_slice(&(new_size as u32).to_be_bytes());
                new_bh[rec + 4..rec + 8].copy_from_slice(&(new_size as u32).to_be_bytes());
                new_bh[rec + 8..rec + 12].copy_from_slice(&req.r.decoded_len.to_be_bytes());
                new_index[e.row_pos..e.row_pos + 4].copy_from_slice(&(new_off as u32).to_be_bytes());
                new_bh[rec..rec + 4].copy_from_slice(&(new_off as u32).to_be_bytes());
                notes.push(format!("entry {} `{}`: {new_size} bytes exceed the slot; ordered reflow — the slot grows by {delta} bytes and every later entry moves up by {delta} (the stock shape; research 2026-09-20 handoff 03 §2)", e.index, e.path));
                edits[k] = Some(Big4Edit { entry_index: e.index, path: e.path.clone(), old_offset: e.offset, old_size: e.size, new_size: new_size as u32, old_decoded_len: req.old_decoded, new_decoded_len: req.r.decoded_len, new_stored_sha256: req.new_sha, placement: Big4Placement::Reflow { new_offset: new_off, old_slot_end: *slot_end, delta } });
            }
            Kind::Grow { .. } => {
                // Append: laid out after the tail; the source slot is copied verbatim (dead bytes).
                appends.push(k);
            }
        }
    }
    // The rest of the source (verbatim, shifted by the cumulative reflow).
    if src_pos < bound.identity.len {
        segments.push(Segment::Copy { src: src_pos..bound.identity.len, dst: out_pos });
        out_pos += bound.identity.len - src_pos;
    }
    if shift != 0 {
        // Every byte after the first reflow point moved: mark the moved region as changed and
        // re-point every entry / `.bh` record that lies at or after each reflow's old slot end.
        let first = reflow_shifts[0].0;
        changed.push(first..out_pos);
        for (i, other) in bound.entries.iter().enumerate() {
            let old = other.offset as u64;
            let s: u64 = reflow_shifts.iter().filter(|(end, _)| old >= *end).map(|(_, d)| *d).sum();
            if s == 0 {
                continue;
            }
            // A reflowed entry itself keeps `old + (shift before it)` — written above; an entry
            // that a fit-in-slot edit re-pointed likewise. Others take the cumulative shift.
            let already = checked.iter().any(|req| req.r.entry_index == i && !matches!(req.kind, Kind::NoOp));
            if already {
                continue;
            }
            let new_off = old + s;
            if new_off > u32::MAX as u64 {
                return Err(Big4Error::TooLarge(new_off));
            }
            new_index[other.row_pos..other.row_pos + 4].copy_from_slice(&(new_off as u32).to_be_bytes());
            let r = Viv4::record_pos(i);
            new_bh[r..r + 4].copy_from_slice(&(new_off as u32).to_be_bytes());
        }
    }
    // Appends, each at align64 of the running end.
    for k in appends {
        let req = &checked[k];
        let e = &bound.entries[req.r.entry_index];
        let new_size = req.r.new_stored.len() as u64;
        let new_offset = align64(out_pos);
        let end = new_offset + new_size;
        if end > u32::MAX as u64 {
            return Err(Big4Error::TooLarge(end));
        }
        if new_offset > out_pos {
            segments.push(Segment::Zero { dst: out_pos, len: new_offset - out_pos });
        }
        segments.push(Segment::Payload { edit: k, dst: new_offset });
        changed.push(out_pos..end);
        out_pos = end;
        let rec = Viv4::record_pos(e.index);
        new_index[e.row_pos..e.row_pos + 4].copy_from_slice(&(new_offset as u32).to_be_bytes());
        new_index[e.row_pos + 4..e.row_pos + 8].copy_from_slice(&(new_size as u32).to_be_bytes());
        new_bh[rec..rec + 4].copy_from_slice(&(new_offset as u32).to_be_bytes());
        new_bh[rec + 4..rec + 8].copy_from_slice(&(new_size as u32).to_be_bytes());
        new_bh[rec + 8..rec + 12].copy_from_slice(&req.r.decoded_len.to_be_bytes());
        notes.push(format!("entry {} `{}`: appended at {new_offset} and re-pointed — the directory's offset order is broken by this entry, a shape no stock archive exhibits (research 02 §4.3; card R2-BIG4-1)", e.index, e.path));
        edits[k] = Some(Big4Edit { entry_index: e.index, path: e.path.clone(), old_offset: e.offset, old_size: e.size, new_size: new_size as u32, old_decoded_len: req.old_decoded, new_decoded_len: req.r.decoded_len, new_stored_sha256: req.new_sha, placement: Big4Placement::AppendAndRetarget { new_offset } });
    }
    let new_len = out_pos;
    if new_len != bound.identity.len {
        new_index[4..8].copy_from_slice(&(new_len as u32).to_le_bytes());
        let t = new_bh.len() - 16;
        new_bh[t..t + 4].copy_from_slice(&(new_len as u32).to_be_bytes());
    }
    // The index region is written from `new_index_bytes` by the stager before the segments.
    let edits: Vec<Big4Edit> = edits.into_iter().map(|e| e.expect("every request was laid out")).collect();
    let payloads: Vec<Vec<u8>> = checked.into_iter().map(|c| if matches!(c.kind, Kind::NoOp) { Vec::new() } else { c.r.new_stored }).collect();
    Ok(Big4Plan {
        source: bound.identity.clone(),
        original_len: bound.identity.len,
        new_len,
        edits,
        payloads,
        new_index_bytes: new_index,
        new_sidecar: new_bh,
        segments,
        changed_ranges: normalize(changed),
        notes,
    })
}

/// The staged output: identities of both new files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Big4StageReport {
    pub source: Big4Identity,
    pub output_len: u64,
    pub output_sha256: Sha256Bytes,
    pub output_sidecar_sha256: Sha256Bytes,
    pub changed_ranges: Vec<Range<u64>>,
}

/// Write the planned archive to `out` (the patched index region, then the plan's segments in
/// order: verbatim source spans, payloads and zero fill) and return the new sidecar bytes. `src`
/// must still be the bound source.
pub fn stage<R: Read + Seek, W: Write>(bound: &BoundBig4, plan: &Big4Plan, src: &mut R, out: &mut W) -> Result<(Vec<u8>, u64), Big4Error> {
    let (len, sha) = sha_file(src)?;
    if len != bound.identity.len || sha != bound.identity.sha256 {
        return Err(Big4Error::Verification("the source archive is not the bound one".into()));
    }
    out.write_all(&plan.new_index_bytes)?;
    let mut written = plan.new_index_bytes.len() as u64;
    let mut buf = vec![0u8; CHUNK];
    for seg in &plan.segments {
        match seg {
            Segment::Copy { src: span, dst } => {
                if *dst != written {
                    return Err(Big4Error::Verification(format!("segment order broken at {written} (copy planned at {dst})")));
                }
                src.seek(SeekFrom::Start(span.start))?;
                let mut remaining = span.end - span.start;
                while remaining > 0 {
                    let k = src.read(&mut buf[..remaining.min(CHUNK as u64) as usize])?;
                    if k == 0 {
                        return Err(Big4Error::Verification("source shorter than bound".into()));
                    }
                    out.write_all(&buf[..k])?;
                    remaining -= k as u64;
                    written += k as u64;
                }
            }
            Segment::Payload { edit, dst } => {
                if *dst != written {
                    return Err(Big4Error::Verification(format!("segment order broken at {written} (payload planned at {dst})")));
                }
                out.write_all(&plan.payloads[*edit])?;
                written += plan.payloads[*edit].len() as u64;
            }
            Segment::Zero { dst, len } => {
                if *dst != written {
                    return Err(Big4Error::Verification(format!("segment order broken at {written} (zero fill planned at {dst})")));
                }
                let mut remaining = *len;
                let zeros = vec![0u8; CHUNK];
                while remaining > 0 {
                    let n = remaining.min(CHUNK as u64) as usize;
                    out.write_all(&zeros[..n])?;
                    remaining -= n as u64;
                }
                written += len;
            }
        }
    }
    if written != plan.new_len {
        return Err(Big4Error::Verification(format!("wrote {written} bytes, planned {}", plan.new_len)));
    }
    Ok((plan.new_sidecar.clone(), written))
}

/// Verify a staged output against its plan: both files re-bind and cross-check, every entry is
/// where the plan put it (unchanged entries at their offset plus the reflow shift, with their
/// `.bh` records likewise), the edited entries hold the planned bytes, every `Copy` segment of the
/// output equals its source span, and the segments cover the output exactly.
pub fn verify<R: Read + Seek, S: Read + Seek>(bound: &BoundBig4, plan: &Big4Plan, out: &mut R, out_sidecar: &[u8], src: &mut S) -> Result<Big4StageReport, Big4Error> {
    let rebound = BoundBig4::bind(out, out_sidecar)?;
    if rebound.identity.len != plan.new_len {
        return Err(Big4Error::Verification(format!("output length {} vs planned {}", rebound.identity.len, plan.new_len)));
    }
    if rebound.entries.len() != bound.entries.len() {
        return Err(Big4Error::Verification("entry count changed".into()));
    }
    // The reflow shift that applies to a source offset: the sum of the deltas of every reflow
    // whose old slot end is at or before it.
    let reflows: Vec<(u64, u64)> = plan.edits.iter().filter_map(|e| if let Big4Placement::Reflow { old_slot_end, delta, .. } = e.placement { Some((old_slot_end, delta)) } else { None }).collect();
    let shift_of = |old: u64| -> u64 { reflows.iter().filter(|(end, _)| old >= *end).map(|(_, d)| *d).sum() };
    for (a, b) in bound.entries.iter().zip(&rebound.entries) {
        let edit = plan.edits.iter().find(|e| e.entry_index == a.index && e.placement != Big4Placement::NoOp);
        match edit {
            None => {
                let expect_off = a.offset as u64 + shift_of(a.offset as u64);
                if b.offset as u64 != expect_off || b.size != a.size || b.path != a.path || b.index != a.index {
                    return Err(Big4Error::Verification(format!("entry {} `{}` changed", a.index, a.path)));
                }
                let (ra, rb) = (bound.sidecar.records[a.index], rebound.sidecar.records[a.index]);
                if rb[0] as u64 != expect_off || rb[1..] != ra[1..] {
                    return Err(Big4Error::Verification(format!("sidecar record {} changed", a.index)));
                }
                // Its bytes moved verbatim (covered by the Copy segments below).
            }
            Some(e) => {
                let (off, size) = match e.placement {
                    Big4Placement::FitInSlot { offset, .. } => (offset, e.new_size),
                    Big4Placement::AppendAndRetarget { new_offset } => (new_offset, e.new_size),
                    Big4Placement::Reflow { new_offset, .. } => (new_offset, e.new_size),
                    Big4Placement::NoOp => unreachable!(),
                };
                if b.offset as u64 != off || b.size != size || b.path != a.path {
                    return Err(Big4Error::Verification(format!("entry {} `{}` is not as planned", a.index, a.path)));
                }
                let r = rebound.sidecar.records[a.index];
                if r[0] as u64 != off || r[1] != size || r[2] != e.new_decoded_len || r[3] != bound.sidecar.records[a.index][3] {
                    return Err(Big4Error::Verification(format!("sidecar record {} is not as planned", a.index)));
                }
                out.seek(SeekFrom::Start(off))?;
                let mut got = vec![0u8; size as usize];
                out.read_exact(&mut got)?;
                let sha: Sha256Bytes = Sha256::digest(&got).into();
                if sha != e.new_stored_sha256 {
                    return Err(Big4Error::Verification(format!("entry {} `{}` bytes are not the planned replacement", a.index, a.path)));
                }
            }
        }
    }
    // Header words other than `size_le` and the directory rows: the index region equals the
    // plan's (which the stager wrote); check it against the output.
    let mut idx = vec![0u8; plan.new_index_bytes.len()];
    out.seek(SeekFrom::Start(0))?;
    out.read_exact(&mut idx)?;
    if idx != plan.new_index_bytes {
        return Err(Big4Error::Verification("the index region is not the planned one".into()));
    }
    // Coverage and verbatim copies.
    let mut cursor = plan.new_index_bytes.len() as u64;
    for seg in &plan.segments {
        match seg {
            Segment::Copy { src: span, dst } => {
                if *dst != cursor {
                    return Err(Big4Error::Verification(format!("segments leave a hole at {cursor}")));
                }
                compare_span_shifted(out, src, *dst, span.start, span.end - span.start)?;
                cursor += span.end - span.start;
            }
            Segment::Payload { edit, dst } => {
                if *dst != cursor {
                    return Err(Big4Error::Verification(format!("segments leave a hole at {cursor}")));
                }
                cursor += plan.payloads[*edit].len() as u64;
            }
            Segment::Zero { dst, len } => {
                if *dst != cursor {
                    return Err(Big4Error::Verification(format!("segments leave a hole at {cursor}")));
                }
                out.seek(SeekFrom::Start(*dst))?;
                let mut remaining = *len;
                let mut buf = vec![0u8; CHUNK];
                while remaining > 0 {
                    let n = remaining.min(CHUNK as u64) as usize;
                    out.read_exact(&mut buf[..n])?;
                    if buf[..n].iter().any(|b| *b != 0) {
                        return Err(Big4Error::Verification(format!("planned zero fill at {dst} is not zero")));
                    }
                    remaining -= n as u64;
                }
                cursor += len;
            }
        }
    }
    if cursor != plan.new_len {
        return Err(Big4Error::Verification(format!("segments cover {cursor} of {} bytes", plan.new_len)));
    }
    let sidecar_sha256: Sha256Bytes = Sha256::digest(out_sidecar).into();
    Ok(Big4StageReport { source: bound.identity.clone(), output_len: rebound.identity.len, output_sha256: rebound.identity.sha256, output_sidecar_sha256: sidecar_sha256, changed_ranges: plan.changed_ranges.clone() })
}

/// `a[dst..dst+len] == b[src..src+len]`, streamed.
fn compare_span_shifted<R: Read + Seek, S: Read + Seek>(a: &mut R, b: &mut S, dst: u64, src: u64, len: u64) -> Result<(), Big4Error> {
    if len == 0 {
        return Ok(());
    }
    a.seek(SeekFrom::Start(dst))?;
    b.seek(SeekFrom::Start(src))?;
    let mut remaining = len;
    let mut ba = vec![0u8; CHUNK];
    let mut bb = vec![0u8; CHUNK];
    while remaining > 0 {
        let n = remaining.min(CHUNK as u64) as usize;
        a.read_exact(&mut ba[..n])?;
        b.read_exact(&mut bb[..n])?;
        if ba[..n] != bb[..n] {
            let at = (len - remaining) + ba[..n].iter().zip(&bb[..n]).position(|(x, y)| x != y).unwrap() as u64;
            return Err(Big4Error::Verification(format!("copied byte {} (source {}) differs from the source", dst + at, src + at)));
        }
        remaining -= n as u64;
    }
    Ok(())
}

/// Stage to new files (refused if they exist) and verify.
pub fn stage_files(bound: &BoundBig4, plan: &Big4Plan, src_path: &std::path::Path, out_big: &std::path::Path, out_bh: &std::path::Path) -> Result<Big4StageReport, Big4Error> {
    let mut src = std::fs::File::open(src_path)?;
    let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(out_big)?;
    let result = (|| {
        let (sidecar, _) = {
            let mut w = std::io::BufWriter::new(&mut out);
            stage(bound, plan, &mut src, &mut w)?
        };
        out.sync_all()?;
        std::fs::OpenOptions::new().write(true).create_new(true).open(out_bh)?.write_all(&sidecar)?;
        let mut check = std::fs::File::open(out_big)?;
        verify(bound, plan, &mut check, &sidecar, &mut src)
    })();
    if result.is_err() {
        drop(out);
        let _ = std::fs::remove_file(out_big);
        let _ = std::fs::remove_file(out_bh);
    }
    result
}

/// Synthetic BIG4 + `.bh` pairs in the measured shape, for tests.
#[doc(hidden)]
pub mod synth {
    use super::*;

    /// `members`: `(path, stored bytes, .bh decoded length)`; `gaps`: extra zero bytes after each
    /// member before the next 64-byte line is taken (every offset stays 64-aligned, as measured).
    pub fn build(members: &[(&str, &[u8], u32)], gaps: &[u64]) -> (Vec<u8>, Vec<u8>) {
        let mut dir = Vec::new();
        for (name, _, _) in members {
            dir.extend_from_slice(&[0u8; 8]);
            dir.extend_from_slice(name.as_bytes());
            dir.push(0);
        }
        dir.extend_from_slice(b"L286\x15\x05\x00\x00");
        let index_size = dir.len() as u32;
        let mut big = vec![0u8; 16];
        big[..4].copy_from_slice(b"BIG4");
        big[8..12].copy_from_slice(&(members.len() as u32).to_be_bytes());
        big[12..16].copy_from_slice(&index_size.to_be_bytes());
        big.extend_from_slice(&dir);
        let mut offsets = Vec::new();
        let mut pos = align64(big.len() as u64);
        for (k, (_, data, _)) in members.iter().enumerate() {
            big.resize(pos as usize, 0);
            offsets.push(pos as u32);
            big.extend_from_slice(data);
            pos = align64(pos + data.len() as u64 + gaps.get(k).copied().unwrap_or(0));
        }
        let len = big.len() as u32;
        big[4..8].copy_from_slice(&len.to_le_bytes());
        let mut c = 16;
        for (k, (name, data, _)) in members.iter().enumerate() {
            big[c..c + 4].copy_from_slice(&offsets[k].to_be_bytes());
            big[c + 4..c + 8].copy_from_slice(&(data.len() as u32).to_be_bytes());
            c += 8 + name.len() + 1;
        }
        let mut bh = vec![0u8; 16];
        bh[..4].copy_from_slice(b"Viv4");
        bh[8..12].copy_from_slice(&(members.len() as u32).to_be_bytes());
        for (k, (name, data, dec)) in members.iter().enumerate() {
            bh.extend_from_slice(&offsets[k].to_be_bytes());
            bh.extend_from_slice(&(data.len() as u32).to_be_bytes());
            bh.extend_from_slice(&dec.to_be_bytes());
            bh.extend_from_slice(&path_hash(name).to_be_bytes());
        }
        bh.extend_from_slice(&len.to_be_bytes());
        bh.extend_from_slice(&[0u8; 12]);
        let bl = bh.len() as u32;
        bh[4..8].copy_from_slice(&bl.to_le_bytes());
        bh[12..16].copy_from_slice(&bl.to_be_bytes());
        (big, bh)
    }
}

#[cfg(test)]
mod tests {
    use super::synth::build as synth;
    use super::*;

    fn bound(big: &[u8], bh: &[u8]) -> BoundBig4 {
        BoundBig4::bind(&mut std::io::Cursor::new(big), bh).unwrap()
    }

    fn req(b: &BoundBig4, big: &[u8], i: usize, new: &[u8], dec: u32) -> Big4Replacement {
        let e = &b.entries[i];
        Big4Replacement { entry_index: i, expected_path: e.path.clone(), expected_stored_sha256: Sha256::digest(&big[e.offset as usize..(e.offset + e.size) as usize]).into(), new_stored: new.to_vec(), decoded_len: dec, growth: None }
    }

    /// A per-replacement policy overrides the plan-wide one: in one plan, entry 0 reflows while
    /// entry 2 appends; a member whose review predicted "fit" carries `Refuse` and is refused
    /// (rather than silently grown) when the copy no longer fits it.
    #[test]
    fn a_replacement_carries_its_own_growth_policy() {
        let (big, bh) = synth(&[("fe/a.big", &[1u8; 100], 0), ("fe/b.big", &[2u8; 70], 0), ("fe/c.big", &[3u8; 10], 0)], &[0, 0, 0]);
        let b = bound(&big, &bh);
        let mut r0 = req(&b, &big, 0, &[9u8; 200], 0);
        r0.growth = Some(Growth::Reflow);
        let mut r2 = req(&b, &big, 2, &[7u8; 130], 0);
        r2.growth = Some(Growth::Append);
        let p = plan(&b, &mut std::io::Cursor::new(&big), vec![r0, r2], Growth::Refuse("plan-wide".into())).unwrap();
        assert!(matches!(p.edits[0].placement, Big4Placement::Reflow { .. }));
        assert!(matches!(p.edits[1].placement, Big4Placement::AppendAndRetarget { .. }));
        let (out, sc) = run(&big, &bh, &p);
        let rb = bound(&out, &sc);
        assert_eq!(rb.entries[0].size, 200);
        assert_eq!(rb.entries[2].size, 130);
        assert!(rb.entries[2].offset as u64 >= big.len() as u64 + p.reflow_shift() - 64);
        let mut r1 = req(&b, &big, 1, &[8u8; 300], 0);
        r1.growth = Some(Growth::Refuse("previewed as fit-in-slot".into()));
        let err = plan(&b, &mut std::io::Cursor::new(&big), vec![r1], Growth::Reflow).unwrap_err();
        assert!(matches!(&err, Big4Error::GrowthRefused { reason, .. } if reason.contains("previewed as fit")), "{err}");
    }

    fn run(big: &[u8], bh: &[u8], plan: &Big4Plan) -> (Vec<u8>, Vec<u8>) {
        let b = bound(big, bh);
        let mut out = Vec::new();
        let (sidecar, _) = stage(&b, plan, &mut std::io::Cursor::new(big), &mut out).unwrap();
        verify(&b, plan, &mut std::io::Cursor::new(&out), &sidecar, &mut std::io::Cursor::new(big)).unwrap();
        (out, sidecar)
    }

    #[test]
    fn binds_and_cross_checks_a_measured_shape() {
        let (big, bh) = synth(&[("fe/a.big", &[1u8; 100], 500), ("fe/b.big", &[2u8; 70], 0), ("fe/c.big", &[3u8; 10], 0)], &[0, 64, 0]);
        let b = bound(&big, &bh);
        assert_eq!(b.entries.len(), 3);
        assert_eq!(b.slot_end(0), b.entries[1].offset as u64);
        assert_eq!(b.slot_end(2), big.len() as u64);
        // A disagreeing sidecar refuses.
        let mut bad = bh.clone();
        bad[16 + 4] ^= 1;
        assert!(matches!(BoundBig4::bind(&mut std::io::Cursor::new(&big), &bad).unwrap_err(), Big4Error::SidecarMismatch { .. }));
        let mut bad = bh.clone();
        let t = bad.len() - 16;
        bad[t + 3] ^= 1;
        assert!(matches!(BoundBig4::bind(&mut std::io::Cursor::new(&big), &bad).unwrap_err(), Big4Error::SidecarTrailer { .. }));
        let mut badbig = big.clone();
        badbig[4] ^= 1;
        assert!(matches!(BoundBig4::bind(&mut std::io::Cursor::new(&badbig), &bh).unwrap_err(), Big4Error::SizeWordUnexplained { .. }));
    }

    /// The PS3 NHL 10 `fe.big` shape (measured 20 Sep): the declared index size counts a trailer
    /// that runs past the first payload line. Binding derives the payload start from the parsed
    /// directory, keeps only the pre-payload prefix rewritable, and a fit-in-slot edit of the
    /// very first entry neither refuses nor lets the prefix rewrite clobber the new bytes.
    #[test]
    fn declared_index_size_overrunning_the_first_payload_line_binds_and_stages() {
        let (mut big, bh) = synth(&[("fe/a.big", &[1u8; 100], 500), ("fe/b.big", &[2u8; 70], 0)], &[0, 0]);
        let index_size = u32::from_be_bytes(big[12..16].try_into().unwrap());
        let first = u32::from_be_bytes(big[16..20].try_into().unwrap()) as u64;
        // Overstate the index size so `16 + index_size` lands 10 bytes into entry 0.
        let overstated = (first + 10 - 16) as u32;
        assert!(overstated > index_size);
        big[12..16].copy_from_slice(&overstated.to_be_bytes());
        let b = bound(&big, &bh);
        assert_eq!(b.data_start, first);
        assert_eq!(b.declared_index_end, first + 10);
        assert_eq!(b.index_bytes.len() as u64, first);
        let plan = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 120], 0)], Growth::Refuse("no".into())).unwrap();
        let (out, sc) = run(&big, &bh, &plan);
        let e = &b.entries[0];
        assert_eq!(&out[e.offset as usize..e.offset as usize + 120], &[9u8; 120], "the prefix rewrite did not clobber the new member");
        assert_eq!(&out[12..16], &big[12..16], "the declared index size word is carried verbatim");
        let rb = bound(&out, &sc);
        assert_eq!(rb.entries[0].size, 120);
    }

    #[test]
    fn fit_in_slot_patches_size_words_and_zero_fills_the_old_slot() {
        let (big, bh) = synth(&[("fe/a.big", &[1u8; 100], 500), ("fe/b.big", &[2u8; 70], 0), ("fe/c.big", &[3u8; 10], 0)], &[0, 64, 0]);
        let b = bound(&big, &bh);
        // Entry 0: slot 100 + 28 zero slack (to 128); 120 fits.
        let plan = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 120], 0)], Growth::Refuse("no".into())).unwrap();
        assert!(matches!(plan.edits[0].placement, Big4Placement::FitInSlot { zero_slack: 28, .. }));
        assert_eq!(plan.new_len, big.len() as u64);
        let (out, sc) = run(&big, &bh, &plan);
        assert_eq!(out.len(), big.len());
        let e = &b.entries[0];
        assert_eq!(&out[e.offset as usize..e.offset as usize + 120], &[9u8; 120]);
        assert_eq!(&out[e.offset as usize + 120..e.offset as usize + 128], &[0u8; 8]);
        let rb = bound(&out, &sc);
        assert_eq!(rb.entries[0].size, 120);
        assert_eq!(rb.sidecar.records[0], [e.offset, 120, 0, path_hash("fe/a.big")]);
        assert_eq!(rb.sidecar.records[1], b.sidecar.records[1]);
        assert_eq!(&out[4..8], &big[4..8]);
        // Shrinking: zero fill to the old end.
        let plan = super::plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[7u8; 40], 0)], Growth::Refuse("no".into())).unwrap();
        let (out, _) = run(&big, &bh, &plan);
        assert_eq!(&out[e.offset as usize + 40..e.offset as usize + 100], &[0u8; 60]);
    }

    #[test]
    fn growth_appends_and_repoints_or_refuses_by_policy() {
        let (big, bh) = synth(&[("fe/a.big", &[1u8; 100], 500), ("fe/b.big", &[2u8; 70], 0), ("fe/c.big", &[3u8; 10], 0)], &[0, 64, 0]);
        let b = bound(&big, &bh);
        let e = super::Big4Error::GrowthRefused { index: 0, path: String::new(), len: 0, slot: 0, slack: 0, reason: String::new() };
        let _ = e.to_string();
        let err = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 129], 0)], Growth::Refuse("NHL 07 boot.big cascade".into())).unwrap_err();
        assert!(matches!(err, Big4Error::GrowthRefused { slack: 28, .. }), "{err}");
        let plan = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 129], 0), req(&b, &big, 2, &[5u8; 200], 0)], Growth::Append).unwrap();
        assert_eq!(plan.edits.len(), 2);
        let a0 = align64(big.len() as u64);
        assert_eq!(plan.edits[0].placement, Big4Placement::AppendAndRetarget { new_offset: a0 });
        let a2 = align64(a0 + 129);
        assert_eq!(plan.edits[1].placement, Big4Placement::AppendAndRetarget { new_offset: a2 });
        assert_eq!(plan.new_len, a2 + 200);
        assert!(plan.notes.iter().any(|n| n.contains("no stock archive exhibits")));
        let (out, sc) = run(&big, &bh, &plan);
        assert_eq!(out.len() as u64, plan.new_len);
        assert_eq!(&out[..big.len()][b.entries[0].offset as usize..b.entries[0].offset as usize + 100], &[1u8; 100], "old bytes left in place");
        let rb = bound(&out, &sc);
        assert_eq!(rb.entries[0].offset as u64, a0);
        assert_eq!(rb.entries[2].offset as u64, a2);
        assert_eq!(rb.size_le as u64, plan.new_len);
        assert_eq!(rb.sidecar.trailer_len() as u64, plan.new_len);
        assert_eq!(rb.sidecar.records[0], [a0 as u32, 129, 0, path_hash("fe/a.big")]);
        assert_eq!(rb.sidecar.records[1], b.sidecar.records[1]);
    }

    #[test]
    fn non_zero_gap_bytes_limit_the_slack_and_are_preserved() {
        let (mut big, bh) = synth(&[("fe/a.big", &[1u8; 100], 0), ("fe/b.big", &[2u8; 70], 0)], &[0, 0]);
        let b = bound(&big, &bh);
        let e = &b.entries[0];
        big[e.offset as usize + 110] = 0x5A; // a non-zero gap byte 10 bytes into the 28-byte gap
        let b = bound(&big, &bh);
        let plan = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 108], 0)], Growth::Refuse("n".into())).unwrap();
        assert!(matches!(plan.edits[0].placement, Big4Placement::FitInSlot { zero_slack: 10, .. }));
        assert!(plan.notes.iter().any(|n| n.contains("not zero")));
        let (out, _) = run(&big, &bh, &plan);
        assert_eq!(out[e.offset as usize + 110], 0x5A);
        let err = super::plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 111], 0)], Growth::Refuse("n".into())).unwrap_err();
        assert!(matches!(err, Big4Error::GrowthRefused { slack: 10, .. }));
    }

    #[test]
    fn identity_and_stale_source_are_handled() {
        let (big, bh) = synth(&[("fe/a.big", &[1u8; 100], 500), ("fe/b.big", &[2u8; 70], 0)], &[0, 0]);
        let b = bound(&big, &bh);
        let plan = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[1u8; 100], 500)], Growth::Refuse("n".into())).unwrap();
        assert!(plan.is_noop());
        let (out, sc) = run(&big, &bh, &plan);
        assert_eq!((out, sc), (big.clone(), bh.clone()));
        let mut stale = req(&b, &big, 0, &[3u8; 10], 0);
        stale.expected_stored_sha256[0] ^= 1;
        assert!(matches!(super::plan(&b, &mut std::io::Cursor::new(&big), vec![stale], Growth::Append).unwrap_err(), Big4Error::StoredHashMismatch { .. }));
    }

    // ---------------------------------------------------------------- P1 audit (21 Sep 2026)

    #[test]
    fn a_malformed_header_is_refused_before_any_allocation() {
        let (big, bh) = synth(&[("fe/a.big", &[1u8; 100], 0), ("fe/b.big", &[2u8; 70], 0)], &[0, 0]);
        // Entry count far beyond what the directory can hold (would be a capacity overflow).
        let mut bad = big.clone();
        bad[8..12].copy_from_slice(&0xFFFF_FFF0u32.to_be_bytes());
        assert!(matches!(BoundBig4::bind(&mut std::io::Cursor::new(&bad), &bh).unwrap_err(), Big4Error::HeaderShape(_)));
        // Index size beyond the file (would be a multi-GB allocation and an EOF).
        let mut bad = big.clone();
        bad[12..16].copy_from_slice(&0xFFFF_FF00u32.to_be_bytes());
        assert!(matches!(BoundBig4::bind(&mut std::io::Cursor::new(&bad), &bh).unwrap_err(), Big4Error::HeaderShape(_)));
        // A sidecar whose count word claims more records than its bytes hold.
        let mut badbh = bh.clone();
        badbh[8..12].copy_from_slice(&0xFFFF_FFF0u32.to_be_bytes());
        assert!(matches!(BoundBig4::bind(&mut std::io::Cursor::new(&big), &badbh).unwrap_err(), Big4Error::SidecarShape(_)));
        // A directory whose last name is unterminated.
        let mut bad = big.clone();
        let idx = 16 + u32::from_be_bytes(big[12..16].try_into().unwrap()) as usize;
        for b in &mut bad[16..idx] {
            if *b == 0 {
                *b = b'x';
            }
        }
        assert!(matches!(BoundBig4::bind(&mut std::io::Cursor::new(&bad), &bh).unwrap_err(), Big4Error::TruncatedDirectory(_)));
    }

    /// The `.bh` decoded-length word must describe the placed bytes: a raw replacement with a
    /// non-zero word, a RefPack replacement with the wrong (or zero) word, are refused at
    /// planning; the right word plans and the record reads back as planned.
    #[test]
    fn the_sidecar_decoded_word_must_match_the_replacement() {
        let plain: Vec<u8> = (0..300u32).map(|i| (i * 7 % 13) as u8).collect();
        let stock = tdb_archive_codec::refpack_compress(&plain);
        let (big, bh) = synth(&[("fe/a.big", &stock, plain.len() as u32), ("fe/b.big", &[2u8; 70], 0)], &[64, 0]);
        let b = bound(&big, &bh);
        let plain2: Vec<u8> = (0..300u32).map(|i| (i * 5 % 11) as u8).collect();
        let wrapped = tdb_archive_codec::refpack_compress(&plain2);
        let declared = tdb_archive_codec::parse_refpack_header(&wrapped).unwrap().uncompressed_len as u32;
        assert_eq!(declared, 300);
        for wrong in [0u32, 299, 301] {
            let err = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &wrapped, wrong)], Growth::Append).unwrap_err();
            assert!(matches!(err, Big4Error::DecodedWordInconsistent { given, .. } if given == wrong), "{err}");
        }
        let err = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 1, &[3u8; 70], 70)], Growth::Append).unwrap_err();
        assert!(matches!(err, Big4Error::DecodedWordInconsistent { given: 70, .. }), "{err}");
        let plan = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &wrapped, declared)], Growth::Append).unwrap();
        let (out, sc) = run(&big, &bh, &plan);
        let rb = bound(&out, &sc);
        assert_eq!(rb.sidecar.records[0][2], 300);
        assert_eq!(rb.sidecar.records[0][1], wrapped.len() as u32);
        let e = &rb.entries[0];
        assert_eq!(tdb_archive_codec::refpack_decompress(&out[e.offset as usize..(e.offset + e.size) as usize]).unwrap(), plain2);
        // An identical replacement with the stock word is a no-op even on a synthetic member whose
        // stock word is unusual — the identity check precedes the consistency check.
        let (big2, bh2) = synth(&[("fe/a.big", &[1u8; 100], 500)], &[0]);
        let b2 = bound(&big2, &bh2);
        assert!(super::plan(&b2, &mut std::io::Cursor::new(&big2), vec![req(&b2, &big2, 0, &[1u8; 100], 500)], Growth::Refuse("n".into())).unwrap().is_noop());
    }

    /// Two adjacent fit-in-slot edits in one plan: entry 0 grows into its own verified-zero slack
    /// (up to entry 1's line), entry 1 shrinks; both directory rows and `.bh` records are patched,
    /// nothing moves, the third entry and the gap bytes are verbatim.
    #[test]
    fn adjacent_fit_in_slot_edits_stay_within_their_own_slots() {
        let (big, bh) = synth(&[("fe/a.big", &[1u8; 100], 0), ("fe/b.big", &[2u8; 70], 0), ("fe/c.big", &[3u8; 10], 0)], &[0, 0, 0]);
        let b = bound(&big, &bh);
        let plan = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 128], 0), req(&b, &big, 1, &[8u8; 60], 0)], Growth::Refuse("n".into())).unwrap();
        assert!(matches!(plan.edits[0].placement, Big4Placement::FitInSlot { zero_slack: 28, .. }));
        assert!(matches!(plan.edits[1].placement, Big4Placement::FitInSlot { .. }));
        assert_eq!(plan.new_len, big.len() as u64);
        let (out, sc) = run(&big, &bh, &plan);
        let rb = bound(&out, &sc);
        assert_eq!(rb.entries[0].offset, b.entries[0].offset);
        assert_eq!(rb.entries[0].size, 128);
        assert_eq!(rb.entries[1].offset, b.entries[1].offset);
        assert_eq!(rb.entries[1].size, 60);
        assert_eq!(rb.entries[2], b.entries[2]);
        let e1 = &b.entries[1];
        assert_eq!(&out[e1.offset as usize..e1.offset as usize + 60], &[8u8; 60]);
        assert_eq!(&out[e1.offset as usize + 60..e1.offset as usize + 70], &[0u8; 10], "shrink zero-fills to the old end");
        // 129 would cross into entry 1's line: refused with the slot named.
        let err = super::plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 129], 0), req(&b, &big, 1, &[8u8; 60], 0)], Growth::Refuse("n".into())).unwrap_err();
        assert!(matches!(err, Big4Error::GrowthRefused { index: 0, slack: 28, .. }));
    }

    /// A second plan on an archive a previous append already re-pointed: the re-pointed entry is
    /// last in offset order (its slot runs to the EOF), a further growth appends again past the new
    /// EOF, the dead old bytes stay, and both indexes keep cross-checking.
    #[test]
    fn a_previously_appended_archive_binds_and_takes_a_second_append() {
        let (big, bh) = synth(&[("fe/a.big", &[1u8; 100], 0), ("fe/b.big", &[2u8; 70], 0)], &[0, 0]);
        let b = bound(&big, &bh);
        let p1 = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 200], 0)], Growth::Append).unwrap();
        let (out1, sc1) = run(&big, &bh, &p1);
        let b1 = bound(&out1, &sc1);
        assert!(!b1.index_order_is_offset_order);
        assert_eq!(b1.slot_end(0), out1.len() as u64);
        let p2 = plan(&b1, &mut std::io::Cursor::new(&out1), vec![req(&b1, &out1, 0, &[7u8; 300], 0)], Growth::Append).unwrap();
        assert!(matches!(p2.edits[0].placement, Big4Placement::AppendAndRetarget { .. }));
        let (out2, sc2) = run(&out1, &sc1, &p2);
        let b2 = bound(&out2, &sc2);
        assert_eq!(b2.entries[0].size, 300);
        assert_eq!(b2.entries[1], b.entries[1]);
        assert_eq!(&out2[..out1.len()][b.entries[0].offset as usize..b.entries[0].offset as usize + 100], &[1u8; 100]);
        assert_eq!(b2.size_le as usize, out2.len());
        assert_eq!(b2.sidecar.trailer_len() as usize, out2.len());
        // A source that is no longer the bound one is refused at staging.
        let mut drift = out1.clone();
        drift[b.entries[1].offset as usize] ^= 1;
        let mut sink = Vec::new();
        assert!(matches!(stage(&b1, &p2, &mut std::io::Cursor::new(&drift), &mut sink).unwrap_err(), Big4Error::Verification(_)));
    }

    // ---------------------------------------------------------------- P2-A reflow (21 Sep 2026)

    /// Ordered reflow on a synthetic pair: entry 0 outgrows its slot; entries 1 and 2 (and a
    /// non-zero gap byte that lives between them) move up by Δ unchanged; the directory rows, the
    /// `.bh` records, `size_le` and the trailer follow; the output re-binds in the stock order and
    /// the moved bytes equal the source. Δ is the oracle's `align64(offset + L) − old slot end`.
    #[test]
    fn ordered_reflow_moves_every_later_entry_and_its_gap_bytes() {
        let (mut big, bh) = synth(&[("fe/a.big", &[1u8; 100], 0), ("fe/b.big", &[2u8; 70], 0), ("fe/c.big", &[3u8; 10], 0)], &[0, 64, 0]);
        let b0 = bound(&big, &bh);
        // A non-zero byte in the gap after entry 1 (inside its 64-byte extra gap) must travel.
        let e1 = b0.entries[1].clone();
        big[e1.offset as usize + 70 + 5] = 0x5A;
        let b = bound(&big, &bh);
        let (o0, o1, o2) = (b.entries[0].offset as u64, b.entries[1].offset as u64, b.entries[2].offset as u64);
        let plan = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 200], 0)], Growth::Reflow).unwrap();
        let delta = align64(o0 + 200) - o1;
        assert_eq!(plan.edits[0].placement, Big4Placement::Reflow { new_offset: o0, old_slot_end: o1, delta });
        assert_eq!(plan.new_len, big.len() as u64 + delta);
        assert_eq!(plan.reflow_shift(), delta);
        assert!(plan.notes.iter().any(|n| n.contains("ordered reflow")));
        let (out, sc) = run(&big, &bh, &plan);
        let rb = bound(&out, &sc);
        assert!(rb.index_order_is_offset_order);
        assert_eq!(rb.entries[0].offset as u64, o0);
        assert_eq!(rb.entries[0].size, 200);
        assert_eq!(rb.entries[1].offset as u64, o1 + delta);
        assert_eq!(rb.entries[2].offset as u64, o2 + delta);
        assert_eq!(rb.entries[1].size, 70);
        assert_eq!(rb.entries[2].size, 10);
        assert_eq!(rb.size_le as u64, plan.new_len);
        assert_eq!(rb.sidecar.trailer_len() as u64, plan.new_len);
        assert_eq!(rb.sidecar.records[1], [(o1 + delta) as u32, 70, 0, path_hash("fe/b.big")]);
        assert_eq!(rb.sidecar.records[2], [(o2 + delta) as u32, 10, 0, path_hash("fe/c.big")]);
        assert_eq!(&out[o0 as usize..o0 as usize + 200], &[9u8; 200]);
        assert!(out[o0 as usize + 200..(o1 + delta) as usize].iter().all(|x| *x == 0), "fresh zero fill to the new slot end");
        assert_eq!(&out[(o1 + delta) as usize..], &big[o1 as usize..], "everything from the old slot end moved up verbatim, gap byte included");
        assert_eq!(out[(e1.offset as u64 + delta) as usize + 75], 0x5A);
        assert_eq!(&out[16..b.entries[0].row_pos], &big[16..b.entries[0].row_pos]);
        // The reflowed pair takes a further fit-in-slot edit and a further reflow of a later entry.
        let p2 = super::plan(&rb, &mut std::io::Cursor::new(&out), vec![req(&rb, &out, 1, &[8u8; 60], 0)], Growth::Refuse("n".into())).unwrap();
        assert!(matches!(p2.edits[0].placement, Big4Placement::FitInSlot { .. }));
        run(&out, &sc, &p2);
        // Entry 1's own gap carries the non-zero byte: its reflow is refused (never erased);
        // the last entry reflows (its slot runs to the EOF).
        let e3 = super::plan(&rb, &mut std::io::Cursor::new(&out), vec![req(&rb, &out, 1, &[8u8; 300], 0)], Growth::Reflow).unwrap_err();
        assert!(matches!(e3, Big4Error::GrowthRefused { index: 1, .. }), "{e3}");
        let p3 = super::plan(&rb, &mut std::io::Cursor::new(&out), vec![req(&rb, &out, 2, &[6u8; 300], 0)], Growth::Reflow).unwrap();
        let (out3, sc3) = run(&out, &sc, &p3);
        let rb3 = bound(&out3, &sc3);
        assert!(rb3.index_order_is_offset_order);
        assert_eq!(rb3.entries[2].offset, rb.entries[2].offset);
        assert_eq!(rb3.entries[2].size, 300);
        assert_eq!(out3.len() as u64, align64(rb.entries[2].offset as u64 + 300));
    }

    /// Two reflows in one plan accumulate; a fit-in-slot edit between them lands at its shifted
    /// offset; an append policy on the same growth set moves nothing.
    #[test]
    fn several_reflows_in_one_plan_accumulate_and_mix_with_fits_and_appends() {
        let (big, bh) = synth(&[("fe/a.big", &[1u8; 100], 0), ("fe/b.big", &[2u8; 70], 0), ("fe/c.big", &[3u8; 10], 0), ("fe/d.big", &[4u8; 30], 0)], &[0, 0, 0, 0]);
        let b = bound(&big, &bh);
        let offs: Vec<u64> = b.entries.iter().map(|e| e.offset as u64).collect();
        let plan = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 200], 0), req(&b, &big, 1, &[8u8; 60], 0), req(&b, &big, 2, &[7u8; 130], 0)], Growth::Reflow).unwrap();
        let d0 = align64(offs[0] + 200) - offs[1];
        let d2 = align64(offs[2] + 130) - offs[3];
        assert_eq!(plan.edits[0].placement, Big4Placement::Reflow { new_offset: offs[0], old_slot_end: offs[1], delta: d0 });
        assert_eq!(plan.edits[1].placement, Big4Placement::FitInSlot { offset: offs[1] + d0, slot_end: offs[2] + d0, zero_slack: 58 });
        assert_eq!(plan.edits[2].placement, Big4Placement::Reflow { new_offset: offs[2] + d0, old_slot_end: offs[3], delta: d2 });
        assert_eq!(plan.reflow_shift(), d0 + d2);
        assert_eq!(plan.new_len, big.len() as u64 + d0 + d2);
        let (out, sc) = run(&big, &bh, &plan);
        let rb = bound(&out, &sc);
        assert!(rb.index_order_is_offset_order);
        assert_eq!(rb.entries.iter().map(|e| e.offset as u64).collect::<Vec<_>>(), vec![offs[0], offs[1] + d0, offs[2] + d0, offs[3] + d0 + d2]);
        assert_eq!(rb.entries.iter().map(|e| e.size).collect::<Vec<_>>(), vec![200, 60, 130, 30]);
        assert_eq!(&out[rb.entries[3].offset as usize..rb.entries[3].offset as usize + 30], &[4u8; 30]);
        assert_eq!(&out[rb.entries[1].offset as usize + 60..rb.entries[1].offset as usize + 70], &[0u8; 10], "shrunk entry zero-filled to its old end");
        // Append policy on the same growth set: nothing moves, both grown entries go past the EOF.
        let pa = super::plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 200], 0), req(&b, &big, 2, &[7u8; 130], 0)], Growth::Append).unwrap();
        assert_eq!(pa.reflow_shift(), 0);
        let (oa, sa) = run(&big, &bh, &pa);
        let ra = bound(&oa, &sa);
        assert!(!ra.index_order_is_offset_order);
        assert_eq!(ra.entries[1].offset as u64, offs[1]);
        assert_eq!(ra.entries[3].offset as u64, offs[3]);
    }

    /// Reflow refuses when the grown entry's own gap carries a non-zero byte (never erased), and
    /// names append as the alternative; append still works on that archive; a reflow of the last
    /// entry extends the EOF with no tail.
    #[test]
    fn reflow_refuses_a_non_zero_gap_and_handles_the_last_entry() {
        let (mut big, bh) = synth(&[("fe/a.big", &[1u8; 100], 0), ("fe/b.big", &[2u8; 70], 0)], &[0, 0]);
        let b = bound(&big, &bh);
        big[b.entries[0].offset as usize + 110] = 0x5A;
        let b = bound(&big, &bh);
        let err = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 200], 0)], Growth::Reflow).unwrap_err();
        assert!(matches!(&err, Big4Error::GrowthRefused { slack: 10, reason, .. } if reason.contains("append is the alternative")), "{err}");
        let pa = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 0, &[9u8; 200], 0)], Growth::Append).unwrap();
        let (oa, _) = run(&big, &bh, &pa);
        assert_eq!(oa[b.entries[0].offset as usize + 110], 0x5A);
        // Last entry: slot end == EOF; the reflow grows the file to align64(offset + L).
        let p = plan(&b, &mut std::io::Cursor::new(&big), vec![req(&b, &big, 1, &[8u8; 500], 0)], Growth::Reflow).unwrap();
        let o1 = b.entries[1].offset as u64;
        assert_eq!(p.new_len, align64(o1 + 500));
        let (out, sc) = run(&big, &bh, &p);
        let rb = bound(&out, &sc);
        assert_eq!(rb.entries[1].size, 500);
        assert_eq!(rb.size_le as usize, out.len());
        assert_eq!(&out[b.data_start as usize..o1 as usize], &big[b.data_start as usize..o1 as usize], "everything before the grown last entry verbatim");
    }
}
