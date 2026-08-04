//! EA PS4 `EB\0\3` archive reader (NHL 15 → 21 `game_*.big` / `inxp_*.big` /
//! `boot.big` / `overlays_*.big`).
//!
//! The PS4 counterpart of the Xbox/PS3 [`BIGF`/`BIG4`](crate) container: the
//! payload codecs (`chunkzip` / `chunklzx`) are identical, only the directory
//! format differs. See `docs/formats/ps4-eb-big.md` for the verified byte
//! layout.
//!
//! ## Why a separate reader (not another [`Archive`] magic)
//!
//! Two structural facts rule out folding EB into [`Archive::parse`]:
//!
//! * **`u64` offsets.** `game_90.big` is 17 GB, so an entry's real offset
//!   (`block << 4`) and the archive size both exceed `u32`. [`Entry::offset`]
//!   is `u32`, sized for sub-4 GB BIGF archives.
//! * **Streaming.** A 17 GB archive can't be `read`-into-`Vec<u8>` the way the
//!   BIGF path slices its `bytes`. This reader parses only the front tables;
//!   the caller seeks + reads each entry's payload on demand.
//!
//! The name/path table is parsed up front (a few MB even for the largest
//! archive); payload bytes are never held here.

use tdb_archive_codec::{MAGIC_CHUNKLZX, MAGIC_CHUNKZIP};
use thiserror::Error;

/// 4-byte magic at offset 0 of every PS4 `EB\0\3` archive.
pub const MAGIC_EB: &[u8; 4] = b"EB\x00\x03";

/// Version/flags word at 0x08 — a **variant marker**. The magic, header layout, name/dir tables, and
/// `offset = block << 4` encoding are identical across variants; only the **file-table entry stride**
/// differs (see [`entry_size_for`]). Two are known on real NHL installs:
/// * `0x0011_0400` — PS4 `game_*.big` / `inxp_*.big` (NHL 15 → 21); 20-byte entries.
/// * `0x0000_0400` — Xbox 360 NHL Legacy `.big` (render + `fe/ion` cache); **16-byte entries**.
///   Reversed + validated (Phase 5.0): every payload lands 16-byte-aligned at `block<<4` with a valid
///   magic (`GD.STRM`, `chunkzip`/`chunklzx`, `EB\0\3`), and the last entry ends at `archive_size`.
pub const VERSION_TAG: u32 = 0x0011_0400;
/// Every EB version/flags word this reader handles (see [`VERSION_TAG`]).
pub const KNOWN_VERSION_TAGS: &[u32] = &[0x0011_0400, 0x0000_0400];

/// Bytes per file-table entry for a given version word. The two observed variants share the same
/// `[block:u32][_:u32][size:u32][crc:u32]` field order; the PS4 variant carries one extra trailing
/// `u32` (20 vs 16 bytes). Unknown versions never reach here (gated in [`EbHeader::parse`]).
pub fn entry_size_for(version: u32) -> usize {
    match version {
        0x0000_0400 => 16,
        _ => 20,
    }
}

/// File table starts here on every EB archive.
const FILE_TABLE_OFFSET: usize = 0x30;
/// Fixed header length we must read before we can size the rest of the tables.
pub const HEADER_LEN: usize = 0x20;

#[derive(Debug, Error)]
pub enum EbError {
    #[error("input is shorter than the 32-byte EB header ({0} bytes)")]
    TruncatedHeader(usize),
    #[error("not an EB\\0\\3 archive — magic was {0:?}")]
    BadMagic([u8; 4]),
    #[error("unexpected EB version tag 0x{0:08X} (known: {KNOWN_VERSION_TAGS:08X?})")]
    BadVersion(u32),
    #[error("name slot width is zero — cannot hold a `dir_index` prefix")]
    ZeroNameSlot,
    #[error(
        "prefix buffer is too short to parse the tables: need {needed} bytes, have {have}. \
         Read `header.table_end()` bytes from the archive front and retry."
    )]
    TruncatedTables { needed: usize, have: usize },
    #[error("name record {index} is not NUL-terminated within its {slot}-byte slot")]
    UnterminatedName { index: usize, slot: usize },
    #[error("dir record {index} is not NUL-terminated within its {slot}-byte slot")]
    UnterminatedDir { index: usize, slot: usize },
    #[error("name record {index} contains a non-ASCII byte (0x{byte:02X})")]
    NonAsciiName { index: usize, byte: u8 },
    #[error("dir record {index} contains a non-ASCII byte (0x{byte:02X})")]
    NonAsciiDir { index: usize, byte: u8 },
    #[error("entry {index} references dir_index {dir_index}, but only {dir_count} dirs exist")]
    DirIndexOutOfRange {
        index: usize,
        dir_index: usize,
        dir_count: usize,
    },
    #[error("in-place replace of entry {index}: {new} bytes exceeds the {slot}-byte slot")]
    PayloadTooLargeForSlot {
        index: usize,
        new: usize,
        slot: usize,
    },
}

/// The fixed-size front of an EB archive. Parse this from the first
/// [`HEADER_LEN`] bytes to learn how much of the archive to read for the full
/// table parse ([`EbHeader::table_end`]).
#[derive(Debug, Clone, Copy)]
pub struct EbHeader {
    /// Version/flags word at 0x08 — selects the file-table entry stride (see [`entry_size_for`]).
    pub version: u32,
    pub file_count: usize,
    pub name_table_offset: usize,
    pub name_slot: usize,
    pub dir_slot: usize,
    pub dir_count: usize,
    pub archive_size: u64,
}

impl EbHeader {
    /// Parse the 32-byte header. `bytes` must be at least [`HEADER_LEN`] long
    /// (reading more is fine — trailing bytes are ignored).
    pub fn parse(bytes: &[u8]) -> Result<Self, EbError> {
        if bytes.len() < HEADER_LEN {
            return Err(EbError::TruncatedHeader(bytes.len()));
        }
        let magic: [u8; 4] = bytes[0..4].try_into().expect("len checked");
        if &magic != MAGIC_EB {
            return Err(EbError::BadMagic(magic));
        }
        let version = read_u32_be(bytes, 0x08);
        if !KNOWN_VERSION_TAGS.contains(&version) {
            return Err(EbError::BadVersion(version));
        }
        let name_slot = bytes[0x14] as usize;
        if name_slot < 3 {
            // Needs 2 bytes of `dir_index` prefix + at least a NUL.
            return Err(EbError::ZeroNameSlot);
        }
        Ok(Self {
            version,
            file_count: read_u32_be(bytes, 0x04) as usize,
            name_table_offset: read_u32_be(bytes, 0x0C) as usize,
            name_slot,
            dir_slot: bytes[0x15] as usize,
            dir_count: read_u16_be(bytes, 0x16) as usize,
            archive_size: read_u64_be(bytes, 0x18),
        })
    }

    /// Bytes per file-table entry for this archive's version (see [`entry_size_for`]).
    pub fn entry_size(&self) -> usize {
        entry_size_for(self.version)
    }

    /// Byte length of the archive prefix that [`EbArchive::parse`] needs: the
    /// name table, its 16-byte alignment pad, and the directory-path table.
    /// Read exactly this many bytes from the archive front and hand them to
    /// [`EbArchive::parse`].
    pub fn table_end(&self) -> usize {
        let name_end = self.name_table_offset + self.file_count * self.name_slot;
        let dir_start = align_up_16(name_end);
        dir_start + self.dir_count * self.dir_slot
    }
}

/// One logical file: a full internal path plus the location and length of its
/// stored bytes. `offset`/`size` describe the payload **as stored** (which may
/// be `chunkzip`-wrapped — see [`decode_payload`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbEntry {
    /// Full forward-slash path, e.g.
    /// `nhl/data/common/gamemodes/database/t3db/nhl_game.db`.
    pub path: String,
    /// Absolute byte offset of the payload within the archive.
    pub offset: u64,
    /// Stored byte count (compressed for `chunkzip`, exact for raw).
    pub size: u32,
}

/// A parsed EB archive **directory** — entries only, no payload bytes. Payloads
/// are streamed by the caller (seek to [`EbEntry::offset`], read
/// [`EbEntry::size`] bytes, then [`decode_payload`]).
#[derive(Debug, Clone)]
pub struct EbArchive {
    pub archive_size: u64,
    pub entries: Vec<EbEntry>,
}

impl EbArchive {
    /// Parse the header + file/name/dir tables from an archive prefix.
    ///
    /// `prefix` must cover at least [`EbHeader::table_end`] bytes from the
    /// archive's front. The usual flow is: read [`HEADER_LEN`], call
    /// [`EbHeader::parse`], then read `header.table_end()` bytes and call this.
    /// Passing the whole archive works too (only the front is touched) but
    /// defeats the streaming design for large archives.
    pub fn parse(prefix: &[u8]) -> Result<Self, EbError> {
        let header = EbHeader::parse(prefix)?;
        let needed = header.table_end();
        if prefix.len() < needed {
            return Err(EbError::TruncatedTables {
                needed,
                have: prefix.len(),
            });
        }

        // Directory-path table: NUL-terminated paths, no prefix, one per slot.
        let name_end = header.name_table_offset + header.file_count * header.name_slot;
        let dir_start = align_up_16(name_end);
        let mut dirs = Vec::with_capacity(header.dir_count);
        for i in 0..header.dir_count {
            let base = dir_start + i * header.dir_slot;
            let slot = &prefix[base..base + header.dir_slot];
            dirs.push(read_slot_str(slot).ok_or_else(|| {
                first_non_ascii(slot).map_or(
                    EbError::UnterminatedDir {
                        index: i,
                        slot: header.dir_slot,
                    },
                    |byte| EbError::NonAsciiDir { index: i, byte },
                )
            })?);
        }

        // Name table + file table are 1:1 by index; join them into entries.
        let mut entries = Vec::with_capacity(header.file_count);
        for i in 0..header.file_count {
            let name_base = header.name_table_offset + i * header.name_slot;
            let dir_index = read_u16_be(prefix, name_base) as usize;
            let name_slot = &prefix[name_base + 2..name_base + header.name_slot];
            let basename = read_slot_str(name_slot).ok_or_else(|| {
                first_non_ascii(name_slot).map_or(
                    EbError::UnterminatedName {
                        index: i,
                        slot: header.name_slot,
                    },
                    |byte| EbError::NonAsciiName { index: i, byte },
                )
            })?;
            let dir = dirs.get(dir_index).ok_or(EbError::DirIndexOutOfRange {
                index: i,
                dir_index,
                dir_count: dirs.len(),
            })?;

            let entry_base = FILE_TABLE_OFFSET + i * header.entry_size();
            let block = read_u32_be(prefix, entry_base) as u64;
            let size = read_u32_be(prefix, entry_base + 0x08);

            entries.push(EbEntry {
                path: join_path(dir, &basename),
                offset: block << 4,
                size,
            });
        }

        Ok(Self {
            archive_size: header.archive_size,
            entries,
        })
    }
}

/// Outcome of decoding one stored EB payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    /// Decoded bytes ready to write (raw pass-through or inflated `chunkzip`).
    Data(Vec<u8>),
    /// A `chunklzx` (LZX) payload — the codec is not implemented; skip it.
    UnsupportedLzx,
}

/// Decode a stored EB payload the same way [`Archive::read_entry_at`](crate::Archive::read_entry_at)
/// decodes a BIGF entry: transparently inflate `chunkzip`, report `chunklzx` as
/// unsupported, pass everything else (raw `.gfx`/`.mkv`/…) through verbatim.
pub fn decode_payload(raw: Vec<u8>) -> Result<Payload, tdb_archive_codec::CodecError> {
    if raw.len() >= 8 && &raw[..8] == MAGIC_CHUNKLZX {
        return Ok(Payload::UnsupportedLzx);
    }
    if raw.len() >= 8 && &raw[..8] == MAGIC_CHUNKZIP {
        Ok(Payload::Data(tdb_archive_codec::decompress_chunkzip(&raw)?))
    } else {
        Ok(Payload::Data(raw))
    }
}

fn align_up_16(n: usize) -> usize {
    (n + 15) & !15
}

fn align_up_16_u64(n: u64) -> u64 {
    (n + 15) & !15
}

// ======================================================================= writer
//
// `EbFile` is the **rewrite** counterpart of the streaming [`EbArchive`] reader: it keeps the whole
// header+table prefix verbatim and every payload's bytes, so an unmodified `encode()` reproduces the
// source byte-for-byte (the correctness bar), and `replace_payload` swaps one entry's bytes, reflowing
// later blocks when the size changes. Because it holds all payloads in memory it's for the **small**
// front-end / render sub-archives being edited (e.g. `data0.big`), not the multi-GB PS4 `game_*.big`.
//
// Layout reproduced (verified against `data0.big`, Phase 5.4): `data_start = align_up_16(table_end)`;
// payloads laid contiguously in file-table order, each padded up to the next 16-byte boundary with
// zeros; `archive_size = align_up_16(last_payload_end)`. The file-table `+12` word is a per-entry
// checksum whose algorithm is **not yet identified** (not IEEE-CRC32 of the stored bytes, not a path
// hash) — it is preserved verbatim on round-trip and left unchanged on replace; resolving it (and
// whether the game validates it, or accepts a substituted payload codec) is an install-time /
// game-verified concern, tracked in the Legacy Xbox 360 `PLATFORM_LIMITATIONS.md`.

/// One payload slot in an [`EbFile`]: its full path, current archive offset, and stored bytes
/// (exactly as they sit in the archive — `chunkzip`/`chunklzx`-wrapped or raw; use
/// [`decode_payload`] to inflate).
#[derive(Debug, Clone)]
pub struct EbSlot {
    pub path: String,
    /// Absolute archive offset of the payload (`block << 4`). Updated by a reflow.
    pub offset: u64,
    /// Stored bytes, verbatim.
    pub bytes: Vec<u8>,
}

/// A fully-materialised EB archive for editing + re-emission. See the module note above.
#[derive(Debug, Clone)]
pub struct EbFile {
    /// Bytes `0..data_start` (header + file/name/dir tables), preserved verbatim. Only the per-entry
    /// `block`/`size` words and the header `archive_size` are patched here on a reflow.
    prefix: Vec<u8>,
    data_start: usize,
    entry_size: usize,
    slots: Vec<EbSlot>,
    archive_size: u64,
}

impl EbFile {
    /// Number of entries.
    pub fn len(&self) -> usize {
        self.slots.len()
    }
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
    pub fn slots(&self) -> &[EbSlot] {
        &self.slots
    }

    /// Parse a **whole** archive (all bytes) into an editable form.
    pub fn parse(full: &[u8]) -> Result<Self, EbError> {
        let header = EbHeader::parse(full)?;
        let table_end = header.table_end();
        let data_start = align_up_16(table_end);
        if full.len() < data_start {
            return Err(EbError::TruncatedTables {
                needed: data_start,
                have: full.len(),
            });
        }
        // Reuse the reader to resolve paths/offsets/sizes.
        let arc = EbArchive::parse(&full[..table_end.min(full.len())])?;
        let mut slots = Vec::with_capacity(arc.entries.len());
        for e in &arc.entries {
            let start = e.offset as usize;
            let end = start
                .checked_add(e.size as usize)
                .filter(|&end| end <= full.len())
                .ok_or(EbError::TruncatedTables {
                    needed: start + e.size as usize,
                    have: full.len(),
                })?;
            slots.push(EbSlot {
                path: e.path.clone(),
                offset: e.offset,
                bytes: full[start..end].to_vec(),
            });
        }
        Ok(Self {
            prefix: full[..data_start].to_vec(),
            data_start,
            entry_size: header.entry_size(),
            slots,
            archive_size: header.archive_size,
        })
    }

    /// Re-emit the archive. With no edits this is **byte-identical** to the source
    /// (padding between blocks and the tail is zero, matching EA's layout).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.archive_size as usize];
        out[..self.data_start].copy_from_slice(&self.prefix);
        for slot in &self.slots {
            let start = slot.offset as usize;
            let end = start + slot.bytes.len();
            // The archive_size envelope always covers every block (kept in sync by `reflow`).
            out[start..end].copy_from_slice(&slot.bytes);
        }
        out
    }

    /// Replace entry `index`'s payload **in place** — `new_bytes` must be **no larger** than the
    /// current stored size, so nothing moves: the offset, every other entry, and the archive length
    /// are untouched; only this entry's bytes and its file-table `size` change (the `+12` checksum is
    /// left as-is). Any slack between the new (shorter) payload and the next block becomes zero gap.
    /// This is the minimal-change edit — use it (padding the payload to the exact old size) to test a
    /// codec/content swap without introducing a reflow. Errors if `new_bytes` exceeds the old size.
    pub fn replace_payload_in_place(&mut self, index: usize, new_bytes: Vec<u8>) -> Result<(), EbError> {
        let count = self.slots.len();
        let slot = self.slots.get_mut(index).ok_or(EbError::DirIndexOutOfRange {
            index,
            dir_index: index,
            dir_count: count,
        })?;
        if new_bytes.len() > slot.bytes.len() {
            return Err(EbError::PayloadTooLargeForSlot {
                index,
                new: new_bytes.len(),
                slot: slot.bytes.len(),
            });
        }
        slot.bytes = new_bytes;
        let e = FILE_TABLE_OFFSET + index * self.entry_size;
        self.prefix[e + 8..e + 12].copy_from_slice(&(self.slots[index].bytes.len() as u32).to_be_bytes());
        Ok(())
    }

    /// Replace entry `index`'s stored payload with `new_bytes` (already in the on-disk form the game
    /// expects — raw, `chunkzip`, …). Blocks are reflowed so every payload stays 16-byte-block aligned
    /// and contiguous; the file table (`block`/`size`) and header `archive_size` are patched. The
    /// `+12` checksum is left unchanged (see the module note). Returns the entry's new offset.
    pub fn replace_payload(&mut self, index: usize, new_bytes: Vec<u8>) -> Result<u64, EbError> {
        let count = self.slots.len();
        let slot = self.slots.get_mut(index).ok_or(EbError::DirIndexOutOfRange {
            index,
            dir_index: index,
            dir_count: count,
        })?;
        slot.bytes = new_bytes;
        self.reflow();
        Ok(self.slots[index].offset)
    }

    /// Re-lay every payload contiguously from `data_start` in file-table order, each padded up to a
    /// 16-byte boundary, and patch the file table + `archive_size`. Independent (no dedup) — if the
    /// source shared a block between two entries, they are split into separate blocks here (the file
    /// grows slightly but stays valid + re-parseable). Deterministic: on an unedited, non-dedup
    /// archive it reproduces the original offsets exactly.
    fn reflow(&mut self) {
        let mut cursor = self.data_start as u64;
        for slot in &mut self.slots {
            slot.offset = cursor;
            cursor = align_up_16_u64(cursor + slot.bytes.len() as u64);
        }
        self.archive_size = cursor;

        // Patch the file-table `block` (@0) and `size` (@8) words for each entry.
        for (i, slot) in self.slots.iter().enumerate() {
            let e = FILE_TABLE_OFFSET + i * self.entry_size;
            let block = (slot.offset >> 4) as u32;
            self.prefix[e..e + 4].copy_from_slice(&block.to_be_bytes());
            self.prefix[e + 8..e + 12].copy_from_slice(&(slot.bytes.len() as u32).to_be_bytes());
        }
        // Patch header `archive_size` (u64 BE @0x18).
        self.prefix[0x18..0x20].copy_from_slice(&self.archive_size.to_be_bytes());
    }
}

fn join_path(dir: &str, base: &str) -> String {
    let mut s = String::with_capacity(dir.len() + 1 + base.len());
    s.push_str(dir);
    s.push('/');
    s.push_str(base);
    s
}

/// Read a NUL-terminated ASCII string from a fixed-width, zero-padded slot.
/// Returns `None` if there's no NUL in the slot or a byte is non-ASCII.
fn read_slot_str(slot: &[u8]) -> Option<String> {
    let end = slot.iter().position(|&b| b == 0)?;
    let s = &slot[..end];
    if !s.is_ascii() {
        return None;
    }
    Some(std::str::from_utf8(s).expect("ASCII is valid UTF-8").to_owned())
}

/// First non-ASCII byte before the terminator (for a precise error), if any.
fn first_non_ascii(slot: &[u8]) -> Option<u8> {
    slot.iter()
        .take_while(|&&b| b != 0)
        .copied()
        .find(|b| !b.is_ascii())
}

fn read_u16_be(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}

fn read_u32_be(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn read_u64_be(d: &[u8], o: usize) -> u64 {
    u64::from_be_bytes([
        d[o], d[o + 1], d[o + 2], d[o + 3], d[o + 4], d[o + 5], d[o + 6], d[o + 7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic EB archive from `(dir_index, basename, payload)`
    /// entries plus the directory list. Mirrors the real layout precisely
    /// (fixed 20-byte file table, `dir_index`-prefixed fixed-width name slots,
    /// 16-byte-aligned dir table, 16-byte-aligned data blocks) so the parser is
    /// exercised the same way it is on a real archive.
    fn build_synthetic_eb(dirs: &[&str], files: &[(u16, &str, &[u8])]) -> Vec<u8> {
        build_synthetic_eb_ver(VERSION_TAG, dirs, files)
    }

    fn build_synthetic_eb_ver(version: u32, dirs: &[&str], files: &[(u16, &str, &[u8])]) -> Vec<u8> {
        let entry_size = entry_size_for(version);
        let file_count = files.len();
        let name_slot = files
            .iter()
            .map(|(_, n, _)| n.len() + 1 + 2)
            .max()
            .unwrap_or(3);
        let dir_slot = dirs.iter().map(|d| d.len() + 1).max().unwrap_or(1);

        let name_table_offset = FILE_TABLE_OFFSET + file_count * entry_size;
        let name_end = name_table_offset + file_count * name_slot;
        let dir_start = align_up_16(name_end);
        let table_end = dir_start + dirs.len() * dir_slot;
        let data_start = align_up_16(table_end);

        // Lay data out 16-byte aligned; record each file's block + size.
        let mut blocks = Vec::with_capacity(file_count);
        let mut cursor = data_start;
        for (_, _, body) in files {
            blocks.push((cursor >> 4, body.len()));
            cursor = align_up_16(cursor + body.len());
        }
        let archive_size = cursor as u64;

        let mut out = vec![0u8; data_start];
        out[0..4].copy_from_slice(MAGIC_EB);
        out[0x04..0x08].copy_from_slice(&(file_count as u32).to_be_bytes());
        out[0x08..0x0C].copy_from_slice(&version.to_be_bytes());
        out[0x0C..0x10].copy_from_slice(&(name_table_offset as u32).to_be_bytes());
        out[0x14] = name_slot as u8;
        out[0x15] = dir_slot as u8;
        out[0x16..0x18].copy_from_slice(&(dirs.len() as u16).to_be_bytes());
        out[0x18..0x20].copy_from_slice(&archive_size.to_be_bytes());

        for (i, ((_, _, _), (block, size))) in files.iter().zip(blocks.iter()).enumerate() {
            let e = FILE_TABLE_OFFSET + i * entry_size;
            out[e..e + 4].copy_from_slice(&(*block as u32).to_be_bytes());
            out[e + 0x08..e + 0x0C].copy_from_slice(&(*size as u32).to_be_bytes());
        }
        for (i, (dir_index, name, _)) in files.iter().enumerate() {
            let n = name_table_offset + i * name_slot;
            out[n..n + 2].copy_from_slice(&dir_index.to_be_bytes());
            out[n + 2..n + 2 + name.len()].copy_from_slice(name.as_bytes());
        }
        for (i, dir) in dirs.iter().enumerate() {
            let d = dir_start + i * dir_slot;
            out[d..d + dir.len()].copy_from_slice(dir.as_bytes());
        }
        // File data at each block offset.
        for ((_, _, body), (block, _)) in files.iter().zip(blocks.iter()) {
            let off = block << 4;
            out.resize(out.len().max(off + body.len()), 0);
            out[off..off + body.len()].copy_from_slice(body);
        }
        out.resize(archive_size as usize, 0);
        out
    }

    #[test]
    fn parses_synthetic_archive_paths_and_payloads() {
        let dirs = &["nhl/data/db", "nhl/ui/overlays"];
        let files: &[(u16, &str, &[u8])] = &[
            (0, "nhl_game.db", b"MAIN-DB-BYTES"),
            (0, "initial_experience.db", b"IXP"),
            (1, "scoreclock.gfx", b"CFX%scaleform-ish"),
        ];
        let bytes = build_synthetic_eb(dirs, files);

        let header = EbHeader::parse(&bytes).unwrap();
        assert_eq!(header.file_count, 3);
        assert_eq!(header.dir_count, 2);
        assert_eq!(header.archive_size, bytes.len() as u64);

        let arch = EbArchive::parse(&bytes).unwrap();
        assert_eq!(arch.entries.len(), 3);
        assert_eq!(arch.entries[0].path, "nhl/data/db/nhl_game.db");
        assert_eq!(arch.entries[1].path, "nhl/data/db/initial_experience.db");
        assert_eq!(arch.entries[2].path, "nhl/ui/overlays/scoreclock.gfx");

        for (entry, (_, _, body)) in arch.entries.iter().zip(files.iter()) {
            let raw = bytes[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec();
            assert_eq!(decode_payload(raw).unwrap(), Payload::Data(body.to_vec()));
        }
    }

    #[test]
    fn deduplicated_payload_extracts_to_both_paths() {
        // Two entries pointing at the same block (the real archives share the
        // byte-identical `_i1.gnf` progressbar textures across widgets).
        let dirs = &["a", "b"];
        let files: &[(u16, &str, &[u8])] = &[(0, "x.gnf", b"SAME"), (1, "y.gnf", b"SAME")];
        let mut bytes = build_synthetic_eb(dirs, files);
        // Force entry 1 to reuse entry 0's block.
        let e0_block = read_u32_be(&bytes, FILE_TABLE_OFFSET);
        let e1 = FILE_TABLE_OFFSET + entry_size_for(VERSION_TAG);
        bytes[e1..e1 + 4].copy_from_slice(&e0_block.to_be_bytes());

        let arch = EbArchive::parse(&bytes).unwrap();
        assert_eq!(arch.entries[0].offset, arch.entries[1].offset);
        assert_eq!(arch.entries[0].path, "a/x.gnf");
        assert_eq!(arch.entries[1].path, "b/y.gnf");
    }

    #[test]
    fn table_end_bounds_the_prefix() {
        let dirs = &["dir"];
        let files: &[(u16, &str, &[u8])] = &[(0, "f", b"data")];
        let bytes = build_synthetic_eb(dirs, files);
        let header = EbHeader::parse(&bytes).unwrap();
        // A prefix one byte short of the tables must fail cleanly (not panic).
        let short = &bytes[..header.table_end() - 1];
        assert!(matches!(
            EbArchive::parse(short),
            Err(EbError::TruncatedTables { .. })
        ));
        // Exactly table_end bytes is enough to parse the directory.
        assert!(EbArchive::parse(&bytes[..header.table_end()]).is_ok());
    }

    #[test]
    fn rejects_bad_magic_and_version() {
        let mut bytes = vec![0u8; HEADER_LEN];
        bytes[0..4].copy_from_slice(b"BIGF");
        assert!(matches!(EbHeader::parse(&bytes), Err(EbError::BadMagic(_))));

        bytes[0..4].copy_from_slice(MAGIC_EB);
        bytes[0x08..0x0C].copy_from_slice(&0xDEAD_BEEFu32.to_be_bytes());
        bytes[0x14] = 8;
        assert!(matches!(EbHeader::parse(&bytes), Err(EbError::BadVersion(_))));
    }

    /// The Xbox 360 NHL Legacy variant (version 0x0000_0400) uses a **16-byte** file-table entry.
    /// Reversed + validated against `data0.big` (Phase 5.0): with the 16-byte stride, every entry
    /// offset is `block<<4` and `size@+8` is exact, so paths *and* payloads round-trip.
    #[test]
    fn parses_x360_legacy_16byte_entry_variant() {
        const X360: u32 = 0x0000_0400;
        assert_eq!(entry_size_for(X360), 16);
        assert_eq!(entry_size_for(VERSION_TAG), 20);

        let dirs = &["rendering/logo", "gamedata/anim"];
        let files: &[(u16, &str, &[u8])] = &[
            (0, "texlib_0_0_0.rx2", b"RX2-LOGO-PAYLOAD"),
            (1, "anim.cba", b"GD.STRM-anim-bytes"),
            (0, "texlib_0_0_1.rx2", b"another-rx2"),
        ];
        let bytes = build_synthetic_eb_ver(X360, dirs, files);

        let header = EbHeader::parse(&bytes).unwrap();
        assert_eq!(header.version, X360);
        assert_eq!(header.entry_size(), 16);

        let arch = EbArchive::parse(&bytes).unwrap();
        assert_eq!(arch.entries.len(), 3);
        assert_eq!(arch.entries[0].path, "rendering/logo/texlib_0_0_0.rx2");
        assert_eq!(arch.entries[1].path, "gamedata/anim/anim.cba");
        // Offsets are 16-byte aligned and payloads extract byte-for-byte.
        for (entry, (_, _, body)) in arch.entries.iter().zip(files.iter()) {
            assert_eq!(entry.offset % 16, 0, "offset is block<<4 aligned");
            let raw = bytes[entry.offset as usize..entry.offset as usize + entry.size as usize].to_vec();
            assert_eq!(decode_payload(raw).unwrap(), Payload::Data(body.to_vec()));
        }
    }

    #[test]
    fn chunklzx_payload_is_reported_unsupported() {
        let mut raw = MAGIC_CHUNKLZX.to_vec();
        raw.extend_from_slice(&[0u8; 16]);
        assert_eq!(decode_payload(raw).unwrap(), Payload::UnsupportedLzx);
    }

    // ----------------------------------------------------------------- writer

    #[test]
    fn writer_round_trips_synthetic_byte_identical() {
        for &version in KNOWN_VERSION_TAGS {
            let dirs = &["rendering/logo", "gamedata/physics"];
            let files: &[(u16, &str, &[u8])] = &[
                (0, "texlib_10_0_0.rx2", b"\x89RW4xb2-detroit-logo-payload-bytes"),
                (1, "refcoll.xml", b"chunkzip\x00\x00\x00\x02fake"),
                (0, "texlib_10_1_0.rx2", b"another-logo"),
            ];
            let src = build_synthetic_eb_ver(version, dirs, files);
            let eb = EbFile::parse(&src).unwrap();
            assert_eq!(eb.len(), 3);
            let out = eb.encode();
            assert_eq!(out, src, "unmodified encode must be byte-identical (version 0x{version:08X})");
        }
    }

    #[test]
    fn writer_replace_payload_reparses_and_reflows() {
        let dirs = &["rendering/logo"];
        let files: &[(u16, &str, &[u8])] = &[
            (0, "a.rx2", b"AAAA"),
            (0, "b.rx2", b"BBBBBBBB"),
            (0, "c.rx2", b"CC"),
        ];
        let src = build_synthetic_eb_ver(0x0000_0400, dirs, files);
        let mut eb = EbFile::parse(&src).unwrap();

        // Grow the middle payload past its 16-byte slot so the tail entry must shift.
        let bigger = vec![0x42u8; 40];
        eb.replace_payload(1, bigger.clone()).unwrap();

        let out = eb.encode();
        // Re-parse the emitted archive with the independent reader — structure + payloads must hold.
        let re = EbArchive::parse(&out).unwrap();
        assert_eq!(re.entries.len(), 3);
        assert_eq!(re.entries[0].path, "rendering/logo/a.rx2");
        assert_eq!(re.entries[1].path, "rendering/logo/b.rx2");
        assert_eq!(re.entries[2].path, "rendering/logo/c.rx2");
        let payload = |e: &EbEntry| out[e.offset as usize..e.offset as usize + e.size as usize].to_vec();
        assert_eq!(payload(&re.entries[0]), b"AAAA");
        assert_eq!(payload(&re.entries[1]), bigger);
        assert_eq!(payload(&re.entries[2]), b"CC");
        // Every block stays 16-byte aligned and inside the archive.
        for e in &re.entries {
            assert_eq!(e.offset % 16, 0);
            assert!(e.offset as usize + e.size as usize <= out.len());
        }
        assert_eq!(re.archive_size, out.len() as u64);
    }

    /// LZX (`chunklzx`) decode verified byte-for-byte against the extracted tree: pull a compressed
    /// entry out of a real archive, inflate it, and compare to the game's own decompressed file.
    /// Covers a single-chunk payload (`refcoll.xml`) and a multi-chunk one (a 495 KB boot texlib >
    /// the 256 KB window). Skips when the mount is absent (CI / clean-room).
    ///
    /// **WIP / ignored:** the LZX decoder ([`tdb_archive_codec::lzx`]) is scaffolded to textbook
    /// MS-LZX, but the real EA `chunklzx` bitstream framing differs (no bit/word/header combination
    /// yields a valid Huffman pretree) — the exact variant is still being pinned down. This test is
    /// the byte-exact oracle it must pass once the framing is correct.
    #[test]
    #[ignore = "LZX bitstream framing not yet pinned — see lzx.rs"]
    fn chunklzx_decodes_against_extracted_oracle() {
        const ROOT: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";
        // (archive, entry path, extracted oracle relative path)
        let cases: &[(&str, &str, &str)] = &[
            ("data0.big", "gamedata/physics/refcoll.xml", "gamedata/physics/refcoll.xml"),
            ("data0.big", "gamedata/physics/golycoll.xml", "gamedata/physics/golycoll.xml"),
            ("boot.big", "rendering/boot/texlib_0.rx2", "rendering/boot/texlib_0.rx2"),
        ];
        let mut ran = 0;
        for (archive, entry_path, oracle_rel) in cases {
            let apath = std::path::Path::new(ROOT).join(archive);
            let opath = std::path::Path::new(ROOT).join(oracle_rel.replace('/', "\\"));
            let (Ok(abytes), Ok(oracle)) = (std::fs::read(&apath), std::fs::read(&opath)) else {
                eprintln!("skipping {archive}/{entry_path}: mount not present");
                continue;
            };
            let arc = EbArchive::parse(&abytes).expect("parse archive");
            let e = arc
                .entries
                .iter()
                .find(|e| e.path == *entry_path)
                .unwrap_or_else(|| panic!("{entry_path} not in {archive}"));
            let raw = abytes[e.offset as usize..e.offset as usize + e.size as usize].to_vec();
            assert_eq!(&raw[..8], MAGIC_CHUNKLZX, "{entry_path} is chunklzx");
            let dec = tdb_archive_codec::decompress_chunklzx(&raw)
                .unwrap_or_else(|err| panic!("{entry_path}: lzx decode — {err}"));
            assert_eq!(
                dec.len(),
                oracle.len(),
                "{entry_path}: decoded {} bytes, oracle {} bytes",
                dec.len(),
                oracle.len()
            );
            assert!(dec == oracle, "{entry_path}: decoded bytes differ from the extracted oracle");
            eprintln!("  {archive}/{entry_path}: LZX -> {} bytes, byte-exact", dec.len());
            ran += 1;
        }
        if ran == 0 {
            eprintln!("chunklzx oracle: no cases ran (mount absent)");
        }
    }

    /// Byte-identical round-trip on a **real** NHL Legacy Xbox 360 archive, then a payload replace that
    /// re-parses cleanly. Skips when the extracted mount is absent (CI / clean-room).
    #[test]
    fn writer_round_trips_real_data0_big() {
        let path = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\data0.big";
        let Ok(src) = std::fs::read(path) else {
            eprintln!("skipping: {path} not present");
            return;
        };
        let eb = EbFile::parse(&src).unwrap();
        let out = eb.encode();
        assert_eq!(out.len(), src.len(), "size preserved");
        assert!(out == src, "unmodified encode of a real archive is byte-identical");

        // Replace the smallest entry (refcoll.xml) with a shorter raw payload; re-parse the result.
        let smallest = eb
            .slots()
            .iter()
            .enumerate()
            .min_by_key(|(_, s)| s.bytes.len())
            .map(|(i, _)| i)
            .unwrap();
        let mut edited = eb.clone();
        let new_payload = b"\x89RW4xb2-replacement".to_vec();
        edited.replace_payload(smallest, new_payload.clone()).unwrap();
        let out2 = edited.encode();
        let re = EbArchive::parse(&out2).unwrap();
        assert_eq!(re.entries.len(), eb.len());
        let e = &re.entries[smallest];
        let got = out2[e.offset as usize..e.offset as usize + e.size as usize].to_vec();
        assert_eq!(got, new_payload, "replaced payload reads back");
        assert_eq!(re.archive_size, out2.len() as u64);
        for e in &re.entries {
            assert_eq!(e.offset % 16, 0);
        }
    }
}
