//! `tdb-core` — byte-exact reader for EA NHL **TDB v6** databases.
//!
//! Implements the verified format spec (`docs/formats/tdb-v6-format.md` /
//! `tdb-v6-format-spec.md`): the 24-byte file header, the table directory, the 40-byte table
//! headers, the 16-byte field descriptors (`kind_code, bit_offset, id, bit_width`), endianness
//! (PS3/Xbox360 big-endian, PS4 little-endian), the id byte-reversal rule, and bit-packed record
//! decoding. Verified end-to-end against the committed anchors (see the golden tests).
//!
//! This crate is the engine read foundation; editing/writing live in higher crates.

pub mod crc;

use std::fmt;

/// File byte order. Selected by file-header byte `[4]` (`1` = big, `0` = little).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    Big,
    Little,
}

/// A 4-character TDB id, stored in on-disk byte order.
///
/// On big-endian files the on-disk bytes are reversed relative to the community/"real" id; on
/// little-endian files they match. Use [`RawId::canonical`] for the community id and
/// [`RawId::raw`] for the on-disk form (which is what the meta XML keys on).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawId(pub [u8; 4]);

impl RawId {
    /// The on-disk 4-char id (matches the `nhlng-meta.xml` shortname convention).
    pub fn raw(&self) -> String {
        self.0.iter().map(|&b| b as char).collect()
    }

    /// The community/"real" id: characters reversed on big-endian, as-is on little-endian.
    pub fn canonical(&self, endian: Endian) -> String {
        match endian {
            Endian::Big => self.0.iter().rev().map(|&b| b as char).collect(),
            Endian::Little => self.raw(),
        }
    }
}

impl fmt::Debug for RawId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawId({:?})", self.raw())
    }
}

/// Errors from parsing a TDB file.
#[derive(Debug, PartialEq, Eq)]
pub enum TdbError {
    TooShort,
    BadMagic,
    Truncated { what: &'static str },
}

impl fmt::Display for TdbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TdbError::TooShort => write!(f, "buffer too short for a TDB header"),
            TdbError::BadMagic => write!(f, "missing 'DB' magic"),
            TdbError::Truncated { what } => write!(f, "truncated while reading {what}"),
        }
    }
}
impl std::error::Error for TdbError {}

/// A 16-byte field descriptor.
#[derive(Clone, Debug)]
pub struct FieldDescriptor {
    /// Value class: `0` inline string, `1` inline binary, `3` fixed-bit uint, `13`/`14` varchar,
    /// `2` signed, `4` float. (Signedness is actually resolved at the meta layer from `range_low`.)
    pub kind: u32,
    /// Bit position of this field within a record.
    pub bit_offset: u32,
    pub id: RawId,
    /// Storage width in bits (the meta `depth`).
    pub bit_width: u32,
}

/// A parsed table: its block header values plus field descriptors.
#[derive(Clone, Debug)]
pub struct Table {
    pub id: RawId,
    /// Absolute byte offset of the table block.
    pub block_offset: usize,
    pub data_alloc_type: u32,
    pub record_len_bytes: u32,
    pub record_len_bits: u32,
    pub max_records: u16,
    pub current_records: u16,
    pub fields: Vec<FieldDescriptor>,
    /// Absolute offset where the packed record block begins.
    pub record_block: usize,
}

impl Table {
    pub fn field(&self, raw_id: &str) -> Option<&FieldDescriptor> {
        let b = raw_id.as_bytes();
        self.fields.iter().find(|f| f.id.0 == b)
    }
}

/// A parsed TDB v6 file (borrows the source bytes).
pub struct TdbFile<'a> {
    bytes: &'a [u8],
    pub endian: Endian,
    pub db_size: u32,
    pub header_crc: u32,
    pub tables: Vec<Table>,
}

const FILE_HEADER: usize = 0x18;
const TABLE_HEADER: usize = 0x28;
const DESCRIPTOR: usize = 0x10;

impl<'a> TdbFile<'a> {
    /// Parse a TDB v6 file from `bytes`.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, TdbError> {
        if bytes.len() < FILE_HEADER {
            return Err(TdbError::TooShort);
        }
        if &bytes[0..2] != b"DB" {
            return Err(TdbError::BadMagic);
        }
        let endian = if bytes[4] == 1 {
            Endian::Big
        } else {
            Endian::Little
        };
        let db_size = u32_at(bytes, 0x08, endian);
        let table_count = u32_at(bytes, 0x10, endian) as usize;
        let header_crc = u32_at(bytes, 0x14, endian);

        let dir_end = FILE_HEADER + 8 * table_count;
        if bytes.len() < dir_end {
            return Err(TdbError::Truncated {
                what: "table directory",
            });
        }

        let mut entries: Vec<(RawId, usize)> = Vec::with_capacity(table_count);
        for i in 0..table_count {
            let o = FILE_HEADER + i * 8;
            let id = RawId([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
            let off = u32_at(bytes, o + 4, endian) as usize;
            entries.push((id, off));
        }

        let mut tables = Vec::with_capacity(table_count);
        for (id, off) in entries {
            let abs = dir_end + off;
            if abs + TABLE_HEADER > bytes.len() {
                return Err(TdbError::Truncated {
                    what: "table header",
                });
            }
            let data_alloc_type = u32_at(bytes, abs + 0x04, endian);
            let record_len_bytes = u32_at(bytes, abs + 0x08, endian);
            let record_len_bits = u32_at(bytes, abs + 0x0C, endian);
            let max_records = u16_at(bytes, abs + 0x14, endian);
            let current_records = u16_at(bytes, abs + 0x16, endian);
            let num_fields = bytes[abs + 0x1C] as usize;

            let desc_start = abs + TABLE_HEADER;
            if desc_start + num_fields * DESCRIPTOR > bytes.len() {
                return Err(TdbError::Truncated {
                    what: "field descriptors",
                });
            }
            let mut fields = Vec::with_capacity(num_fields);
            let mut o = desc_start;
            for _ in 0..num_fields {
                fields.push(FieldDescriptor {
                    kind: u32_at(bytes, o, endian),
                    bit_offset: u32_at(bytes, o + 4, endian),
                    id: RawId([bytes[o + 8], bytes[o + 9], bytes[o + 10], bytes[o + 11]]),
                    bit_width: u32_at(bytes, o + 12, endian),
                });
                o += DESCRIPTOR;
            }
            tables.push(Table {
                id,
                block_offset: abs,
                data_alloc_type,
                record_len_bytes,
                record_len_bits,
                max_records,
                current_records,
                fields,
                record_block: o,
            });
        }

        Ok(TdbFile {
            bytes,
            endian,
            db_size,
            header_crc,
            tables,
        })
    }

    /// `true` if the on-disk header hash matches `crc32_tdb(file[0x00..0x14])`.
    pub fn header_crc_ok(&self) -> bool {
        crc::crc32_mpeg2(&self.bytes[0..0x14]) == self.header_crc
    }

    /// `true` if the stored `db_size` equals the actual file length.
    pub fn size_ok(&self) -> bool {
        self.db_size as usize == self.bytes.len()
    }

    /// Find a table by its on-disk (raw) id, e.g. `"cPbu"` on PS3 or `"KdqM"` on PS4.
    pub fn table(&self, raw_id: &str) -> Option<&Table> {
        let b = raw_id.as_bytes();
        self.tables.iter().find(|t| t.id.0 == b)
    }

    /// Find a table by its canonical/community id (handles the BE reversal), e.g. `"ubPc"`.
    pub fn table_canonical(&self, canonical: &str) -> Option<&Table> {
        self.tables
            .iter()
            .find(|t| t.id.canonical(self.endian) == canonical)
    }

    /// Read a fixed-bit field's raw bits for record `index` as a `u64` (no sign handling).
    /// Returns `None` if the record/field is out of range.
    pub fn read_u64(&self, table: &Table, index: u32, field: &FieldDescriptor) -> Option<u64> {
        if index >= table.current_records as u32 || field.bit_width > 64 {
            return None;
        }
        let start = table.record_block + (index as usize) * table.record_len_bytes as usize;
        let end = start + table.record_len_bytes as usize;
        if end > self.bytes.len() {
            return None;
        }
        Some(read_bits(
            &self.bytes[start..end],
            field.bit_offset,
            field.bit_width,
            self.endian,
        ))
    }

    /// Read an inline-string (kind 0) field for record `index` as bytes (NUL-trimmed).
    /// Both `bit_offset` and `bit_width` must be byte-aligned.
    pub fn read_inline_str<'b>(
        &'b self,
        table: &Table,
        index: u32,
        field: &FieldDescriptor,
    ) -> Option<&'b [u8]> {
        if !field.bit_offset.is_multiple_of(8) || !field.bit_width.is_multiple_of(8) {
            return None;
        }
        let start = table.record_block + (index as usize) * table.record_len_bytes as usize;
        let fs = start + (field.bit_offset / 8) as usize;
        let fe = fs + (field.bit_width / 8) as usize;
        if fe > self.bytes.len() {
            return None;
        }
        let raw = &self.bytes[fs..fe];
        let n = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        Some(&raw[..n])
    }
}

/// Read `width` bits at `bit_offset` from `data`, honoring `endian` (BE = MSB-first, LE = LSB-first).
pub fn read_bits(data: &[u8], bit_offset: u32, width: u32, endian: Endian) -> u64 {
    if width == 0 {
        return 0;
    }
    debug_assert!(width <= 64);
    let bs = (bit_offset / 8) as usize;
    let be = (bit_offset + width).div_ceil(8) as usize;
    let span = &data[bs..be];
    let from_top = bit_offset - (bs as u32) * 8;
    let mask: u128 = if width >= 128 {
        u128::MAX
    } else {
        (1u128 << width) - 1
    };
    match endian {
        Endian::Little => {
            let mut acc: u128 = 0;
            for (i, &b) in span.iter().enumerate() {
                acc |= (b as u128) << (8 * i);
            }
            ((acc >> from_top) & mask) as u64
        }
        Endian::Big => {
            let mut acc: u128 = 0;
            for &b in span {
                acc = (acc << 8) | b as u128;
            }
            let span_bits = (span.len() as u32) * 8;
            let shift = span_bits - from_top - width;
            ((acc >> shift) & mask) as u64
        }
    }
}

fn u32_at(b: &[u8], o: usize, e: Endian) -> u32 {
    let a = [b[o], b[o + 1], b[o + 2], b[o + 3]];
    match e {
        Endian::Big => u32::from_be_bytes(a),
        Endian::Little => u32::from_le_bytes(a),
    }
}

fn u16_at(b: &[u8], o: usize, e: Endian) -> u16 {
    let a = [b[o], b[o + 1]];
    match e {
        Endian::Big => u16::from_be_bytes(a),
        Endian::Little => u16::from_le_bytes(a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_spec_vector_le() {
        // Spec §1.3 worked vector: record FA CE 00 80 10 00 00 00, read(31,5) == 1 (little-endian).
        let rec = [0xFA, 0xCE, 0x00, 0x80, 0x10, 0x00, 0x00, 0x00];
        assert_eq!(read_bits(&rec, 31, 5, Endian::Little), 1);
    }

    #[test]
    fn id_reversal() {
        let id = RawId(*b"cPbu");
        assert_eq!(id.raw(), "cPbu");
        assert_eq!(id.canonical(Endian::Big), "ubPc");
        assert_eq!(id.canonical(Endian::Little), "cPbu");
    }
}
