//! EA BIGF / BIG4 archive reader + builder.
//!
//! Reads the EA "BIG" container that packs an NHL install's game files
//! (databases, textures, meshes, …). The extractor points at an install root,
//! parses each `.big`, and writes every entry out to its internal path — see
//! `studio-domain`'s extraction module.
//!
//! ## Format summary (BIGF / BIG4 — same parser, different magic)
//!
//! Header (16 bytes):
//!
//! | Off | Width | Endian | Field          |
//! | --- | ----- | ------ | -------------- |
//! | 0   | 4     | bytes  | magic ("BIGF" / "BIG4" / "BIG5".."BIG9") |
//! | 4   | 4     | **LE** | archive_size (quirky — only LE field)   |
//! | 8   | 4     | BE     | file_count                              |
//! | 12  | 4     | BE     | data_offset (first byte after the file table) |
//!
//! Then `file_count` entries. Each entry:
//!
//! - BIG5..BIG9 variants only: 1 leading zero byte
//! - 4 bytes BE: file offset (within the archive)
//! - 4 bytes BE: file size (or zsize for compressed entries)
//! - N bytes ASCII: null-terminated file name
//!
//! Then file data lives at `[offset, offset + size)`.
//!
//! ## Auto-decompression
//!
//! Individual file payloads may be wrapped in `chunkzip` (zlib chunked
//! container). [`Archive::read_entry`] transparently decompresses these via
//! `tdb-archive-codec`; [`Archive::read_entry_raw`] returns the bytes as stored
//! on disk. `chunklzx` (LZX) entries are not implemented — `read_entry` returns
//! [`BigError::ChunkLzxUnsupported`] for them.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod eb;

use std::collections::HashMap;

use thiserror::Error;

/// All 4-byte magic markers we recognise. All resolve to the same parser. Reject
/// anything outside this set so the reader fails loudly on unrelated files (e.g.
/// EB-format archives or stray PNGs).
pub const KNOWN_MAGICS: &[&[u8; 4]] = &[
    b"BIGF", b"BIG4", b"BIG5", b"BIG6", b"BIG7", b"BIG8", b"BIG9",
];

/// Magics that ship with one leading zero byte per file-table entry (vs. plain
/// BIGF/BIG4 which have none). Per the QuickBMS reference script
/// `skate1_skate2_ea_big4.bms`.
const MAGICS_WITH_ENTRY_PAD: &[&[u8; 4]] = &[b"BIG5", b"BIG6", b"BIG7", b"BIG8", b"BIG9"];

#[derive(Debug, Error)]
pub enum BigError {
    #[error("input is shorter than the 16-byte BIG header ({0} bytes)")]
    TruncatedHeader(usize),
    #[error("unrecognised archive magic {0:?} — expected one of BIGF/BIG4/BIG5..BIG9")]
    BadMagic([u8; 4]),
    #[error("file table is truncated at offset {offset} (entry {entry_index} needs {needed} more bytes)")]
    TruncatedFileTable {
        entry_index: usize,
        offset: usize,
        needed: usize,
    },
    #[error("file table entry name at offset {offset} is not null-terminated within the archive")]
    UnterminatedName { offset: usize },
    #[error("file table entry name at offset {offset} contains a non-ASCII byte (0x{byte:02X})")]
    NonAsciiName { offset: usize, byte: u8 },
    #[error("entry `{name}` data range {start}..{end} runs past the end of the archive ({archive_size} bytes)")]
    TruncatedEntry {
        name: String,
        start: usize,
        end: usize,
        archive_size: usize,
    },
    #[error("entry `{0}` was not found in the archive")]
    EntryNotFound(String),
    #[error("entry `{name}` is chunklzx-wrapped; LZX decompression is not yet implemented")]
    ChunkLzxUnsupported { name: String },
    #[error("chunkzip decompression of entry `{name}` failed: {source}")]
    ChunkZipDecodeFailed {
        name: String,
        #[source]
        source: tdb_archive_codec::CodecError,
    },
    #[error(
        "entry name `{0}` is already in use by another entry — refusing to create a duplicate"
    )]
    DuplicateName(String),
}

/// One file-table entry. Carries metadata only — call [`Archive::read_entry`]
/// (or [`Archive::read_entry_at`]) to pull the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// EA-internal name; usually a forward-slash path like
    /// `"fe/ion/.../foo.rpsgl"` or a short token like `"sg2"`.
    pub name: String,
    /// Absolute byte offset within the archive where the file data starts.
    pub offset: u32,
    /// Stored size in bytes. May be smaller than the decompressed size when the
    /// payload is `chunkzip`-wrapped.
    pub size: u32,
}

/// Parsed BIGF / BIG4 archive header + file table. The full archive bytes are
/// kept alongside so [`Archive::read_entry`] can slice them on demand without
/// re-parsing.
#[derive(Debug, Clone)]
pub struct Archive {
    /// Magic as it appeared in the source file — preserve on round trip when we
    /// get to the write path.
    pub magic: [u8; 4],
    /// `archive_size` field from the header, in bytes. This is the quirky
    /// little-endian field; often slightly less than the actual byte length
    /// (16-byte EOF padding).
    pub declared_archive_size: u32,
    /// Offset of the first byte of file data.
    pub data_offset: u32,
    /// One entry per file in declaration order.
    pub entries: Vec<Entry>,
    bytes: Vec<u8>,
    /// `name → entries[i]` for `read_entry` lookups. First entry wins on
    /// duplicate names (consistent with EA tools).
    name_index: HashMap<String, usize>,
}

impl Archive {
    /// Parse the BIGF / BIG4 header + file table from a byte buffer (typically
    /// the contents of a `.big` file). Does NOT touch the per-entry file data;
    /// that's lazy via [`Self::read_entry`].
    pub fn parse(bytes: impl Into<Vec<u8>>) -> Result<Self, BigError> {
        let bytes = bytes.into();
        if bytes.len() < 16 {
            return Err(BigError::TruncatedHeader(bytes.len()));
        }
        let magic: [u8; 4] = bytes[0..4].try_into().expect("len checked");
        if !KNOWN_MAGICS.iter().any(|m| **m == magic) {
            return Err(BigError::BadMagic(magic));
        }

        let declared_archive_size = read_u32_le(&bytes, 4);
        let file_count = read_u32_be(&bytes, 8) as usize;
        let data_offset = read_u32_be(&bytes, 12);

        // Variant-dependent entry size knob.
        let entry_pad = if MAGICS_WITH_ENTRY_PAD.iter().any(|m| **m == magic) {
            1
        } else {
            0
        };

        let mut entries = Vec::with_capacity(file_count);
        let mut name_index = HashMap::with_capacity(file_count);
        let mut cursor = 16usize;
        for entry_index in 0..file_count {
            // Pad byte for BIG5..BIG9.
            if entry_pad == 1 {
                if cursor >= bytes.len() {
                    return Err(BigError::TruncatedFileTable {
                        entry_index,
                        offset: cursor,
                        needed: 1,
                    });
                }
                cursor += 1;
            }
            // offset + size (4 bytes each, BE).
            if cursor + 8 > bytes.len() {
                return Err(BigError::TruncatedFileTable {
                    entry_index,
                    offset: cursor,
                    needed: 8,
                });
            }
            let offset = read_u32_be(&bytes, cursor);
            let size = read_u32_be(&bytes, cursor + 4);
            cursor += 8;
            // Null-terminated ASCII name.
            let name_start = cursor;
            let mut name_end = None;
            while cursor < bytes.len() {
                let b = bytes[cursor];
                if b == 0 {
                    name_end = Some(cursor);
                    cursor += 1; // consume the terminator
                    break;
                }
                if !b.is_ascii() {
                    return Err(BigError::NonAsciiName {
                        offset: cursor,
                        byte: b,
                    });
                }
                cursor += 1;
            }
            let name_end = name_end.ok_or(BigError::UnterminatedName { offset: name_start })?;
            let name = std::str::from_utf8(&bytes[name_start..name_end])
                .expect("ASCII bytes are valid UTF-8")
                .to_owned();
            // First-wins on duplicate names — silent duplicates beyond the first
            // are accessible via [`Self::read_entry_at`].
            name_index.entry(name.clone()).or_insert(entry_index);
            entries.push(Entry { name, offset, size });
        }

        Ok(Self {
            magic,
            declared_archive_size,
            data_offset,
            entries,
            bytes,
            name_index,
        })
    }

    /// Variant magic as a borrowed slice — convenient for debug logging without
    /// needing to re-derive ASCII conversion.
    pub fn magic_str(&self) -> &str {
        std::str::from_utf8(&self.magic).unwrap_or("????")
    }

    /// Iterator over every entry in the file table.
    pub fn iter_entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    /// Return the stored bytes for an entry, decompressing `chunkzip`-wrapped
    /// payloads transparently. Returns [`BigError::ChunkLzxUnsupported`] for
    /// chunklzx entries.
    pub fn read_entry(&self, name: &str) -> Result<Vec<u8>, BigError> {
        let idx = *self
            .name_index
            .get(name)
            .ok_or_else(|| BigError::EntryNotFound(name.to_owned()))?;
        self.read_entry_at(idx)
    }

    /// Same as [`Self::read_entry`] but indexed by file-table position. Useful
    /// when the archive contains duplicate names or when the caller is iterating
    /// with [`Self::iter_entries`].
    pub fn read_entry_at(&self, idx: usize) -> Result<Vec<u8>, BigError> {
        let raw = self.read_entry_raw_at(idx)?;
        // Auto-decompress chunkzip; pass through anything else; bail loudly on
        // chunklzx.
        if raw.len() >= 8 && &raw[..8] == tdb_archive_codec::MAGIC_CHUNKLZX {
            return Err(BigError::ChunkLzxUnsupported {
                name: self.entries[idx].name.clone(),
            });
        }
        if raw.len() >= 8 && &raw[..8] == tdb_archive_codec::MAGIC_CHUNKZIP {
            tdb_archive_codec::decompress_chunkzip(&raw).map_err(|source| {
                BigError::ChunkZipDecodeFailed {
                    name: self.entries[idx].name.clone(),
                    source,
                }
            })
        } else {
            Ok(raw)
        }
    }

    /// Return the stored bytes exactly as they appear in the archive, without
    /// any decompression.
    pub fn read_entry_raw(&self, name: &str) -> Result<Vec<u8>, BigError> {
        let idx = *self
            .name_index
            .get(name)
            .ok_or_else(|| BigError::EntryNotFound(name.to_owned()))?;
        self.read_entry_raw_at(idx)
    }

    /// Indexed counterpart to [`Self::read_entry_raw`].
    pub fn read_entry_raw_at(&self, idx: usize) -> Result<Vec<u8>, BigError> {
        let entry = &self.entries[idx];
        let start = entry.offset as usize;
        let end = start.saturating_add(entry.size as usize);
        if end > self.bytes.len() {
            return Err(BigError::TruncatedEntry {
                name: entry.name.clone(),
                start,
                end,
                archive_size: self.bytes.len(),
            });
        }
        Ok(self.bytes[start..end].to_vec())
    }
}

/// An entry slated for inclusion in a new archive. Produced by
/// [`Archive::to_builder`] (start from an existing archive) or built from
/// scratch via [`ArchiveBuilder::new`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuilderEntry {
    pub name: String,
    /// Bytes that go on disk verbatim — caller is responsible for any
    /// compression / encoding work. The writer does not auto-compress.
    pub data: Vec<u8>,
}

/// Build a new `.big` byte stream. Use [`Archive::to_builder`] to start from an
/// existing archive's entries, then mutate before calling [`Self::build`].
#[derive(Debug, Clone)]
pub struct ArchiveBuilder {
    /// Magic to emit. Round-trip preservation: defaults to the source archive's
    /// magic when constructed via `to_builder`.
    pub magic: [u8; 4],
    pub entries: Vec<BuilderEntry>,
    /// Bytes of trailing EOF padding to append. Sampled NHL13 archives have a
    /// 16-byte padding tail; preserving it on round-trip is the safest default.
    pub eof_padding: u32,
}

impl ArchiveBuilder {
    /// New empty BIGF builder.
    pub fn new() -> Self {
        Self {
            magic: *b"BIGF",
            entries: Vec::new(),
            eof_padding: 0,
        }
    }

    /// Replace the body of an existing entry by name. Returns `false` when no
    /// matching entry was found. First-match wins, consistent with
    /// [`Archive::read_entry`].
    pub fn replace_entry(&mut self, name: &str, data: Vec<u8>) -> bool {
        for e in &mut self.entries {
            if e.name == name {
                e.data = data;
                return true;
            }
        }
        false
    }

    /// Append a new entry. The writer doesn't deduplicate names — caller is
    /// responsible if that matters.
    pub fn add_entry(&mut self, name: impl Into<String>, data: Vec<u8>) {
        self.entries.push(BuilderEntry {
            name: name.into(),
            data,
        });
    }

    /// Remove an existing entry by name. Returns `true` when an entry was found
    /// and removed, `false` when no match was present. First-match wins.
    pub fn remove_entry(&mut self, name: &str) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.name == name) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    /// Rename an existing entry AND swap its body in one shot. Returns
    /// `Ok(true)` on success, `Ok(false)` when `old_name` doesn't exist, or
    /// `Err(BigError::DuplicateName)` when `new_name` is already in use by a
    /// different entry. First-match wins on the `old_name` lookup.
    pub fn rename_entry(
        &mut self,
        old_name: &str,
        new_name: impl Into<String>,
        new_data: Vec<u8>,
    ) -> Result<bool, BigError> {
        let new_name: String = new_name.into();
        if old_name != new_name && self.entries.iter().any(|e| e.name == new_name) {
            return Err(BigError::DuplicateName(new_name));
        }
        for e in &mut self.entries {
            if e.name == old_name {
                e.name = new_name;
                e.data = new_data;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Encode the builder into a `.big` byte stream. Recomputes the archive size
    /// (LE quirky field), file count, data offset, and every per-entry offset.
    pub fn build(&self) -> Result<Vec<u8>, BigError> {
        if !KNOWN_MAGICS.iter().any(|m| **m == self.magic) {
            return Err(BigError::BadMagic(self.magic));
        }
        for e in &self.entries {
            if !e.name.is_ascii() {
                return Err(BigError::NonAsciiName {
                    offset: 0,
                    byte: e.name.bytes().find(|b| !b.is_ascii()).unwrap(),
                });
            }
        }
        let entry_pad = if MAGICS_WITH_ENTRY_PAD.iter().any(|m| **m == self.magic) {
            1
        } else {
            0
        };

        // Compute the file-table size so we know where data starts.
        let mut table_size: usize = 0;
        for e in &self.entries {
            // pad + offset(4) + size(4) + name + null
            table_size += entry_pad + 8 + e.name.len() + 1;
        }
        let data_offset = 16 + table_size;
        // First file goes immediately at data_offset; subsequent files are
        // packed tightly with no per-entry alignment.
        let mut file_offsets: Vec<u32> = Vec::with_capacity(self.entries.len());
        let mut cursor = data_offset;
        for e in &self.entries {
            let off = u32::try_from(cursor).map_err(|_| BigError::TruncatedHeader(cursor))?;
            file_offsets.push(off);
            cursor += e.data.len();
        }
        let payload_end = cursor;
        let archive_size = payload_end + self.eof_padding as usize;
        let archive_size_u32 =
            u32::try_from(archive_size).map_err(|_| BigError::TruncatedHeader(archive_size))?;
        let data_offset_u32 =
            u32::try_from(data_offset).map_err(|_| BigError::TruncatedHeader(data_offset))?;

        let mut out = Vec::with_capacity(archive_size);
        // Header.
        out.extend_from_slice(&self.magic);
        out.extend_from_slice(&archive_size_u32.to_le_bytes()); // LE quirk
        out.extend_from_slice(&(self.entries.len() as u32).to_be_bytes());
        out.extend_from_slice(&data_offset_u32.to_be_bytes());
        // File table.
        for (e, off) in self.entries.iter().zip(file_offsets.iter()) {
            if entry_pad == 1 {
                out.push(0);
            }
            out.extend_from_slice(&off.to_be_bytes());
            out.extend_from_slice(&(e.data.len() as u32).to_be_bytes());
            out.extend_from_slice(e.name.as_bytes());
            out.push(0);
        }
        debug_assert_eq!(out.len(), data_offset, "data offset should match cursor");
        // File data.
        for e in &self.entries {
            out.extend_from_slice(&e.data);
        }
        // EOF padding.
        out.resize(out.len() + self.eof_padding as usize, 0);
        Ok(out)
    }
}

impl Default for ArchiveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Archive {
    /// Snapshot the archive's entries (as raw stored bytes) into a builder. Use
    /// this when you want to read an archive, swap one entry, and write it back.
    /// Mutating the builder doesn't touch the original `Archive`.
    pub fn to_builder(&self) -> Result<ArchiveBuilder, BigError> {
        let mut entries = Vec::with_capacity(self.entries.len());
        for i in 0..self.entries.len() {
            entries.push(BuilderEntry {
                name: self.entries[i].name.clone(),
                data: self.read_entry_raw_at(i)?,
            });
        }
        // Preserve the trailing padding (header's archive_size field is often
        // smaller than the actual byte length — that delta is the padding tail).
        let payload_end = self
            .entries
            .iter()
            .map(|e| e.offset as usize + e.size as usize)
            .max()
            .unwrap_or(self.data_offset as usize);
        let eof_padding = self.bytes.len().saturating_sub(payload_end) as u32;
        Ok(ArchiveBuilder {
            magic: self.magic,
            entries,
            eof_padding,
        })
    }
}

fn read_u32_le(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

fn read_u32_be(data: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic BIGF archive in memory. Used for tests that need
    /// precise control over the header / file table contents.
    fn build_synthetic_bigf(entries: &[(&str, &[u8])]) -> Vec<u8> {
        // First pass: compute file-table size so we can pick a sane data_offset.
        let mut table_size = 0usize;
        for (name, _) in entries {
            table_size += 8 + name.len() + 1; // offset + size + null-term name
        }
        let header_size = 16;
        let data_offset = header_size + table_size;

        // Pre-compute each file's offset within the archive.
        let mut data_layout = Vec::with_capacity(entries.len());
        let mut current_data_offset = data_offset;
        for (_, body) in entries {
            data_layout.push(current_data_offset);
            current_data_offset += body.len();
        }
        let total_size = current_data_offset;

        let mut out = Vec::with_capacity(total_size);
        // Header.
        out.extend_from_slice(b"BIGF");
        out.extend_from_slice(&(total_size as u32).to_le_bytes()); // LE quirk
        out.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        out.extend_from_slice(&(data_offset as u32).to_be_bytes());
        // File table.
        for (i, (name, body)) in entries.iter().enumerate() {
            out.extend_from_slice(&(data_layout[i] as u32).to_be_bytes());
            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            out.extend_from_slice(name.as_bytes());
            out.push(0);
        }
        // File data.
        for (_, body) in entries {
            out.extend_from_slice(body);
        }
        out
    }

    #[test]
    fn parses_synthetic_bigf_round_trip() {
        let entries: &[(&str, &[u8])] = &[
            ("0", b"hello, file zero"),
            ("foo/bar.txt", b"line one\nline two\n"),
            ("empty", b""),
            ("z", b"trailing entry"),
        ];
        let bytes = build_synthetic_bigf(entries);
        let archive = Archive::parse(bytes).expect("parse should succeed");
        assert_eq!(&archive.magic, b"BIGF");
        assert_eq!(archive.entries.len(), 4);
        for (i, (name, body)) in entries.iter().enumerate() {
            assert_eq!(archive.entries[i].name, *name);
            assert_eq!(archive.entries[i].size as usize, body.len());
            let extracted = archive.read_entry(name).unwrap();
            assert_eq!(extracted, *body);
        }
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = vec![b'X', b'X', b'X', b'X'];
        bytes.extend_from_slice(&[0u8; 12]);
        let err = Archive::parse(bytes).unwrap_err();
        assert!(matches!(err, BigError::BadMagic([b'X', b'X', b'X', b'X'])));
    }

    #[test]
    fn truncated_header_is_an_error() {
        assert!(matches!(
            Archive::parse(b"BIGFshort".to_vec()),
            Err(BigError::TruncatedHeader(9))
        ));
    }

    #[test]
    fn missing_entry_lookup_returns_not_found() {
        let bytes = build_synthetic_bigf(&[("only", b"x")]);
        let archive = Archive::parse(bytes).unwrap();
        assert!(matches!(
            archive.read_entry("missing"),
            Err(BigError::EntryNotFound(s)) if s == "missing"
        ));
    }

    #[test]
    fn duplicate_names_first_wins() {
        let bytes = build_synthetic_bigf(&[("dup", b"first"), ("dup", b"second")]);
        let archive = Archive::parse(bytes).unwrap();
        // Name lookup returns the first occurrence.
        assert_eq!(archive.read_entry("dup").unwrap(), b"first".to_vec());
        // Indexed access reaches the duplicate.
        assert_eq!(archive.read_entry_at(1).unwrap(), b"second".to_vec());
    }

    // -------- write-path tests --------

    #[test]
    fn builder_round_trips_synthetic_archive() {
        let entries: &[(&str, &[u8])] = &[
            ("0", b"hello, file zero"),
            ("foo/bar.txt", b"line one\nline two\n"),
            ("empty", b""),
            ("z", b"trailing entry"),
        ];
        let original = build_synthetic_bigf(entries);
        let archive = Archive::parse(original.clone()).unwrap();
        let builder = archive.to_builder().unwrap();
        let rebuilt = builder.build().unwrap();
        let archive_b = Archive::parse(rebuilt).unwrap();
        assert_eq!(archive_b.entries.len(), entries.len());
        for (i, (name, body)) in entries.iter().enumerate() {
            assert_eq!(archive_b.entries[i].name, *name);
            assert_eq!(archive_b.read_entry(name).unwrap(), *body);
        }
    }

    #[test]
    fn builder_replace_entry_changes_size_and_offsets() {
        let original = build_synthetic_bigf(&[("0", b"AAAA"), ("1", b"BBBB"), ("2", b"CCCC")]);
        let archive = Archive::parse(original).unwrap();
        let mut builder = archive.to_builder().unwrap();
        let new_body = vec![b'X'; 64];
        let replaced = builder.replace_entry("1", new_body.clone());
        assert!(replaced, "replace_entry should report success");
        let rebuilt = builder.build().unwrap();
        let archive_b = Archive::parse(rebuilt).unwrap();
        assert_eq!(archive_b.read_entry("0").unwrap(), b"AAAA");
        assert_eq!(archive_b.read_entry("1").unwrap(), new_body);
        assert_eq!(archive_b.read_entry("2").unwrap(), b"CCCC");
        let file2_offset = archive_b
            .entries
            .iter()
            .find(|e| e.name == "2")
            .unwrap()
            .offset;
        assert!(
            file2_offset > 16 + 64,
            "file 2 should be pushed past the larger file 1"
        );
    }

    #[test]
    fn builder_replace_missing_entry_returns_false() {
        let original = build_synthetic_bigf(&[("a", b"x")]);
        let archive = Archive::parse(original).unwrap();
        let mut builder = archive.to_builder().unwrap();
        assert!(!builder.replace_entry("nope", vec![1, 2, 3]));
        assert_eq!(builder.entries.len(), 1);
        assert_eq!(builder.entries[0].name, "a");
    }

    #[test]
    fn builder_preserves_magic_variant() {
        let mut builder = ArchiveBuilder::new();
        builder.magic = *b"BIG4";
        builder.add_entry("test", b"hello".to_vec());
        let bytes = builder.build().unwrap();
        let archive = Archive::parse(bytes).unwrap();
        assert_eq!(&archive.magic, b"BIG4");
        assert_eq!(archive.read_entry("test").unwrap(), b"hello");
    }

    // ----- rename_entry / remove_entry --------------------------------------

    #[test]
    fn rename_entry_swaps_name_and_data_on_match() {
        let mut builder = ArchiveBuilder::new();
        builder.add_entry("name_0_19_3_cm.rpsgl", b"OLD_BYTES".to_vec());
        builder.add_entry("untouched.rpsgl", b"UNTOUCHED".to_vec());
        let renamed = builder
            .rename_entry(
                "name_0_19_3_cm.rpsgl",
                "name_1_22_7_cm.rpsgl",
                b"NEW_BYTES".to_vec(),
            )
            .expect("rename ok");
        assert!(renamed, "expected rename to find the source entry");
        let names: Vec<&str> = builder.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["name_1_22_7_cm.rpsgl", "untouched.rpsgl"]);
        assert_eq!(builder.entries[0].data, b"NEW_BYTES");
        assert_eq!(builder.entries[1].data, b"UNTOUCHED");
    }

    #[test]
    fn rename_entry_returns_false_when_source_missing() {
        let mut builder = ArchiveBuilder::new();
        builder.add_entry("a.rpsgl", b"A".to_vec());
        let renamed = builder
            .rename_entry("missing.rpsgl", "b.rpsgl", b"B".to_vec())
            .expect("no error path here");
        assert!(!renamed, "rename on missing source must report false");
        let names: Vec<&str> = builder.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["a.rpsgl"]);
    }

    #[test]
    fn rename_entry_rejects_collision_with_another_entry() {
        let mut builder = ArchiveBuilder::new();
        builder.add_entry("a.rpsgl", b"A".to_vec());
        builder.add_entry("b.rpsgl", b"B".to_vec());
        let err = builder
            .rename_entry("a.rpsgl", "b.rpsgl", b"NEW".to_vec())
            .expect_err("expected DuplicateName error");
        match err {
            BigError::DuplicateName(name) => assert_eq!(name, "b.rpsgl"),
            other => panic!("expected DuplicateName, got {other:?}"),
        }
        let names: Vec<&str> = builder.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["a.rpsgl", "b.rpsgl"]);
        assert_eq!(builder.entries[0].data, b"A");
    }

    #[test]
    fn remove_entry_drops_named_entry() {
        let mut builder = ArchiveBuilder::new();
        builder.add_entry("first.rpsgl", b"A".to_vec());
        builder.add_entry("middle.rpsgl", b"B".to_vec());
        builder.add_entry("last.rpsgl", b"C".to_vec());
        assert!(builder.remove_entry("middle.rpsgl"));
        let names: Vec<&str> = builder.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["first.rpsgl", "last.rpsgl"]);
    }

    #[test]
    fn remove_entry_returns_false_when_missing() {
        let mut builder = ArchiveBuilder::new();
        builder.add_entry("only.rpsgl", b"A".to_vec());
        assert!(!builder.remove_entry("nope.rpsgl"));
        assert_eq!(builder.entries.len(), 1);
    }
}
