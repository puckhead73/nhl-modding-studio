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

/// Version/flags word at 0x08 — constant across every archive observed on a
/// real NHL install. Used as a sanity gate, not a variant selector.
pub const VERSION_TAG: u32 = 0x0011_0400;

/// File table starts here on every EB archive.
const FILE_TABLE_OFFSET: usize = 0x30;
/// Bytes per file-table entry.
const ENTRY_SIZE: usize = 20;
/// Fixed header length we must read before we can size the rest of the tables.
pub const HEADER_LEN: usize = 0x20;

#[derive(Debug, Error)]
pub enum EbError {
    #[error("input is shorter than the 32-byte EB header ({0} bytes)")]
    TruncatedHeader(usize),
    #[error("not an EB\\0\\3 archive — magic was {0:?}")]
    BadMagic([u8; 4]),
    #[error("unexpected EB version tag 0x{0:08X} (expected 0x{VERSION_TAG:08X})")]
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
}

/// The fixed-size front of an EB archive. Parse this from the first
/// [`HEADER_LEN`] bytes to learn how much of the archive to read for the full
/// table parse ([`EbHeader::table_end`]).
#[derive(Debug, Clone, Copy)]
pub struct EbHeader {
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
        if version != VERSION_TAG {
            return Err(EbError::BadVersion(version));
        }
        let name_slot = bytes[0x14] as usize;
        if name_slot < 3 {
            // Needs 2 bytes of `dir_index` prefix + at least a NUL.
            return Err(EbError::ZeroNameSlot);
        }
        Ok(Self {
            file_count: read_u32_be(bytes, 0x04) as usize,
            name_table_offset: read_u32_be(bytes, 0x0C) as usize,
            name_slot,
            dir_slot: bytes[0x15] as usize,
            dir_count: read_u16_be(bytes, 0x16) as usize,
            archive_size: read_u64_be(bytes, 0x18),
        })
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

            let entry_base = FILE_TABLE_OFFSET + i * ENTRY_SIZE;
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
        let file_count = files.len();
        let name_slot = files
            .iter()
            .map(|(_, n, _)| n.len() + 1 + 2)
            .max()
            .unwrap_or(3);
        let dir_slot = dirs.iter().map(|d| d.len() + 1).max().unwrap_or(1);

        let name_table_offset = FILE_TABLE_OFFSET + file_count * ENTRY_SIZE;
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
        out[0x08..0x0C].copy_from_slice(&VERSION_TAG.to_be_bytes());
        out[0x0C..0x10].copy_from_slice(&(name_table_offset as u32).to_be_bytes());
        out[0x14] = name_slot as u8;
        out[0x15] = dir_slot as u8;
        out[0x16..0x18].copy_from_slice(&(dirs.len() as u16).to_be_bytes());
        out[0x18..0x20].copy_from_slice(&archive_size.to_be_bytes());

        for (i, ((_, _, _), (block, size))) in files.iter().zip(blocks.iter()).enumerate() {
            let e = FILE_TABLE_OFFSET + i * ENTRY_SIZE;
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
        let e1 = FILE_TABLE_OFFSET + ENTRY_SIZE;
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

    #[test]
    fn chunklzx_payload_is_reported_unsupported() {
        let mut raw = MAGIC_CHUNKLZX.to_vec();
        raw.extend_from_slice(&[0u8; 16]);
        assert_eq!(decode_payload(raw).unwrap(), Payload::UnsupportedLzx);
    }
}
