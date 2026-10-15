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
pub mod dev_key;

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

/// Errors from the record-allocation primitives ([`TdbFile::append_record`] /
/// [`TdbFile::remove_record`]).
#[derive(Debug, PartialEq, Eq)]
pub enum AllocError {
    /// No table with that on-disk (raw) id.
    UnknownTable,
    /// Record index past `current_records`.
    IndexOutOfRange,
    /// `max_records` is already `u16::MAX`; the table cannot grow.
    CapacityExhausted,
    /// A computed byte range fell outside the file.
    OutOfBounds,
    /// The grown buffer failed to re-parse (should not happen — a bug guard).
    Reparse(TdbError),
}

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocError::UnknownTable => write!(f, "unknown table"),
            AllocError::IndexOutOfRange => write!(f, "record index out of range"),
            AllocError::CapacityExhausted => write!(f, "table at max_records capacity"),
            AllocError::OutOfBounds => write!(f, "computed byte range outside file"),
            AllocError::Reparse(e) => write!(f, "grown buffer failed to re-parse: {e}"),
        }
    }
}
impl std::error::Error for AllocError {}

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
    /// The bytes this file was parsed from (the whole inner TDB image).
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

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

fn write_u32(b: &mut [u8], o: usize, v: u32, e: Endian) {
    let a = match e {
        Endian::Big => v.to_be_bytes(),
        Endian::Little => v.to_le_bytes(),
    };
    b[o..o + 4].copy_from_slice(&a);
}

fn write_u16(b: &mut [u8], o: usize, v: u16, e: Endian) {
    let a = match e {
        Endian::Big => v.to_be_bytes(),
        Endian::Little => v.to_le_bytes(),
    };
    b[o..o + 2].copy_from_slice(&a);
}

/// Write `width` bits at `bit_offset` in `data` (inverse of [`read_bits`]); leaves other bits intact.
pub fn write_bits(data: &mut [u8], bit_offset: u32, width: u32, value: u64, endian: Endian) {
    if width == 0 {
        return;
    }
    debug_assert!(width <= 64);
    let bs = (bit_offset / 8) as usize;
    let be = (bit_offset + width).div_ceil(8) as usize;
    let span = &mut data[bs..be];
    let from_top = bit_offset - (bs as u32) * 8;
    let mask: u128 = if width >= 128 {
        u128::MAX
    } else {
        (1u128 << width) - 1
    };
    let v = (value as u128) & mask;
    match endian {
        Endian::Little => {
            let mut acc: u128 = 0;
            for (i, &b) in span.iter().enumerate() {
                acc |= (b as u128) << (8 * i);
            }
            acc &= !(mask << from_top);
            acc |= v << from_top;
            for (i, b) in span.iter_mut().enumerate() {
                *b = (acc >> (8 * i)) as u8;
            }
        }
        Endian::Big => {
            let mut acc: u128 = 0;
            for &b in span.iter() {
                acc = (acc << 8) | b as u128;
            }
            let span_bits = (span.len() as u32) * 8;
            let shift = span_bits - from_top - width;
            acc &= !(mask << shift);
            acc |= v << shift;
            let len = span.len();
            for (i, b) in span.iter_mut().enumerate() {
                *b = (acc >> ((len - 1 - i) * 8)) as u8;
            }
        }
    }
}

impl TdbFile<'_> {
    /// `[start, end)` byte range of a table's CRC "body": the descriptors + record region. It begins
    /// right after the 40-byte header and runs to the **next table's block offset** (`next_block`),
    /// i.e. it includes any inter-table padding; for the last table `next_block` is `file_len - 4`
    /// (just before the EOF CRC). Verified across all anchors, including padded layouts.
    fn body_range(&self, idx: usize, next_block: usize) -> (usize, usize) {
        (self.tables[idx].block_offset + 0x28, next_block)
    }

    /// Recompute all four inner-DB CRC scopes in `buf` (in place), in the spec's order:
    /// per-table `prior_crc`, per-table `header_crc`, the EOF CRC, then the file-header hash.
    /// For an unedited buffer this reproduces the original bytes.
    pub fn recompute_crcs(&self, buf: &mut [u8]) {
        let e = self.endian;
        let n = buf.len();
        let table_count = self.tables.len();
        if table_count == 0 {
            return;
        }
        // Tables in data-offset (file layout) order, and each one's successor block offset.
        let mut order: Vec<usize> = (0..table_count).collect();
        order.sort_by_key(|&i| self.tables[i].block_offset);
        let next_block: Vec<usize> = (0..order.len())
            .map(|k| {
                if k + 1 < order.len() {
                    self.tables[order[k + 1]].block_offset
                } else {
                    n - 4
                }
            })
            .collect();

        // 1. prior_crc @ block+0x00: table 0 hashes the directory; table k hashes table k-1's body.
        let dir = 0x18..0x18 + 8 * table_count;
        for k in 0..order.len() {
            let scope = if k == 0 {
                crc::crc32_mpeg2(&buf[dir.clone()])
            } else {
                let (s, en) = self.body_range(order[k - 1], next_block[k - 1]);
                crc::crc32_mpeg2(&buf[s..en])
            };
            write_u32(buf, self.tables[order[k]].block_offset, scope, e);
        }
        // 2. header_crc @ block+0x24 over table[+4..+36].
        for &idx in &order {
            let b = self.tables[idx].block_offset;
            let v = crc::crc32_mpeg2(&buf[b + 4..b + 36]);
            write_u32(buf, b + 0x24, v, e);
        }
        // 3. EOF CRC (last 4 bytes) over the last table's body.
        let last = order.len() - 1;
        let (s, en) = self.body_range(order[last], next_block[last]);
        let v = crc::crc32_mpeg2(&buf[s..en]);
        write_u32(buf, n - 4, v, e);
        // 4. File-header hash @ 0x14 over file[0x00..0x14].
        let v = crc::crc32_mpeg2(&buf[0..0x14]);
        write_u32(buf, 0x14, v, e);
    }

    /// Clone the source bytes and recompute all CRCs. For an unedited file this returns a
    /// byte-identical copy (the round-trip invariant).
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = self.bytes.to_vec();
        self.recompute_crcs(&mut buf);
        buf
    }

    /// Patch a fixed-bit field's value for record `index` in `buf` (does **not** recompute CRCs —
    /// call [`TdbFile::recompute_crcs`] afterwards). Returns `false` if out of range.
    pub fn set_u64(
        &self,
        buf: &mut [u8],
        table: &Table,
        index: u32,
        field: &FieldDescriptor,
        value: u64,
    ) -> bool {
        if index >= table.current_records as u32 || field.bit_width > 64 {
            return false;
        }
        let start = table.record_block + index as usize * table.record_len_bytes as usize;
        let end = start + table.record_len_bytes as usize;
        if end > buf.len() {
            return false;
        }
        write_bits(
            &mut buf[start..end],
            field.bit_offset,
            field.bit_width,
            value,
            self.endian,
        );
        true
    }

    /// Overwrite an inline-string (kind 0) field for record `index` in `buf` with `text` (does **not**
    /// recompute CRCs — call [`TdbFile::recompute_crcs`] afterwards). The field's full byte-width is
    /// zeroed first, then `text`'s bytes are written and the remainder stays NUL (so
    /// [`TdbFile::read_inline_str`] reads it back exactly). Both `bit_offset` and `bit_width` must be
    /// byte-aligned. Returns `false` if out of range or if `text` is longer than the field width.
    pub fn set_inline_str(
        &self,
        buf: &mut [u8],
        table: &Table,
        index: u32,
        field: &FieldDescriptor,
        text: &str,
    ) -> bool {
        if !field.bit_offset.is_multiple_of(8) || !field.bit_width.is_multiple_of(8) {
            return false;
        }
        if index >= table.current_records as u32 {
            return false;
        }
        let start = table.record_block + index as usize * table.record_len_bytes as usize;
        let fs = start + (field.bit_offset / 8) as usize;
        let width = (field.bit_width / 8) as usize;
        let fe = fs + width;
        if fe > buf.len() {
            return false;
        }
        let bytes = text.as_bytes();
        if bytes.len() > width {
            return false;
        }
        buf[fs..fe].fill(0);
        buf[fs..fs + bytes.len()].copy_from_slice(bytes);
        true
    }

    /// Copy the entire fixed-width record block from record `src` to record `dst` within `table`
    /// (both must be `< current_records`) in `buf`. This clones **every** field at once — fixed-bit,
    /// inline strings, and any other in-record bytes — so callers can deep-copy a record then override
    /// just the key fields. Does **not** recompute CRCs (call [`TdbFile::recompute_crcs`] after).
    /// Returns `false` if either index is out of range or the byte range escapes `buf`.
    pub fn copy_record(&self, buf: &mut [u8], table: &Table, src: u32, dst: u32) -> bool {
        let cur = table.current_records as u32;
        if src >= cur || dst >= cur {
            return false;
        }
        let reclen = table.record_len_bytes as usize;
        let (s, d) = (
            table.record_block + src as usize * reclen,
            table.record_block + dst as usize * reclen,
        );
        if s + reclen > buf.len() || d + reclen > buf.len() {
            return false;
        }
        buf.copy_within(s..s + reclen, d);
        true
    }

    /// Reorder a table's records in place: new record `i` becomes old record `order[i]`. `order`
    /// must be a **permutation** of `0..current_records` (checked; anything else errors with no
    /// bytes touched). Whole fixed-width record blocks move — every field at once, like
    /// [`TdbFile::copy_record`] — so row *content* is conserved exactly; only physical record
    /// position changes. File length, `max_records`, and every other table are untouched. Use for
    /// healing block-structured tables (`eGlu`) whose rows sit outside their team's contiguous
    /// block. Does **not** recompute CRCs (call [`TdbFile::recompute_crcs`] after).
    pub fn permute_records(
        &self,
        buf: &mut [u8],
        table_raw: &str,
        order: &[u32],
    ) -> Result<(), AllocError> {
        let t = self.table(table_raw).ok_or(AllocError::UnknownTable)?;
        let cur = t.current_records as usize;
        if order.len() != cur {
            return Err(AllocError::IndexOutOfRange);
        }
        let mut seen = vec![false; cur];
        for &src in order {
            let s = src as usize;
            if s >= cur || seen[s] {
                return Err(AllocError::IndexOutOfRange);
            }
            seen[s] = true;
        }
        let reclen = t.record_len_bytes as usize;
        let start = t.record_block;
        let end = start + cur * reclen;
        if end > buf.len() {
            return Err(AllocError::OutOfBounds);
        }
        let original = buf[start..end].to_vec();
        for (dst, &src) in order.iter().enumerate() {
            let s = src as usize * reclen;
            buf[start + dst * reclen..start + (dst + 1) * reclen]
                .copy_from_slice(&original[s..s + reclen]);
        }
        Ok(())
    }

    /// Append a zeroed record to a table and return the **new file bytes** (CRCs recomputed) plus the
    /// new record's index.
    ///
    /// Two paths:
    /// * **Slack reuse** — if `current_records < max_records`, the table already has an allocated slot;
    ///   zero it and bump `current_records`. The file length is unchanged.
    /// * **Table growth** — if the table is full (`current == max`, the vanilla state of every anchor
    ///   table), the allocation is grown by one record: `record_len` zero bytes are inserted at the
    ///   end of the table's record region, every later table's directory offset is shifted, `db_size`
    ///   and the table's `max_records`/`current_records` are updated, and all CRCs are recomputed.
    ///
    /// The new record is zero-filled; populate it with [`TdbFile::set_u64`] on the returned bytes.
    pub fn append_record(&self, table_raw: &str) -> Result<(Vec<u8>, u32), AllocError> {
        let t = self.table(table_raw).ok_or(AllocError::UnknownTable)?;
        let reclen = t.record_len_bytes as usize;
        let cur = t.current_records;
        let max = t.max_records;
        let new_index = cur as u32;

        // --- Slack path: a free allocated slot already exists. ---
        if cur < max {
            let slot = t.record_block + cur as usize * reclen;
            if slot + reclen > self.bytes.len() {
                return Err(AllocError::OutOfBounds);
            }
            let mut buf = self.bytes.to_vec();
            buf[slot..slot + reclen].fill(0);
            write_u16(&mut buf, t.block_offset + 0x16, cur + 1, self.endian);
            self.recompute_crcs(&mut buf);
            return Ok((buf, new_index));
        }

        // --- Growth path: table is full; grow its allocation by one record. ---
        if max == u16::MAX {
            return Err(AllocError::CapacityExhausted);
        }
        let insert_at = t.record_block + max as usize * reclen;
        if insert_at > self.bytes.len() {
            return Err(AllocError::OutOfBounds);
        }
        let block_offset = t.block_offset;

        let mut buf = Vec::with_capacity(self.bytes.len() + reclen);
        buf.extend_from_slice(&self.bytes[..insert_at]);
        buf.extend(std::iter::repeat_n(0u8, reclen));
        buf.extend_from_slice(&self.bytes[insert_at..]);

        // Shift the directory offset of every table whose block begins at/after the insertion point.
        let dir_end = FILE_HEADER + 8 * self.tables.len();
        for i in 0..self.tables.len() {
            let entry = FILE_HEADER + i * 8;
            let off = u32_at(&buf, entry + 4, self.endian) as usize;
            if dir_end + off >= insert_at {
                write_u32(&mut buf, entry + 4, (off + reclen) as u32, self.endian);
            }
        }
        // Grow the table's max + current, and the file's db_size.
        write_u16(&mut buf, block_offset + 0x14, max + 1, self.endian);
        write_u16(&mut buf, block_offset + 0x16, cur + 1, self.endian);
        write_u32(&mut buf, 0x08, self.db_size + reclen as u32, self.endian);

        // Recompute all CRCs over the new structure (re-parse a clone to satisfy the borrow checker).
        let view = buf.clone();
        let db = TdbFile::parse(&view).map_err(AllocError::Reparse)?;
        db.recompute_crcs(&mut buf);
        Ok((buf, new_index))
    }

    /// Grow a table's **allocation** to `new_max` records without touching its population:
    /// `(new_max - max_records)` zeroed slack slots are appended to the table's record region,
    /// every later table's directory offset is shifted, `db_size` and `max_records` are updated,
    /// and all CRCs are recomputed — `current_records` is unchanged. This is [`append_record`]'s
    /// growth path generalized to a bulk pre-allocation (the vanilla ROSTER-save shape:
    /// `current < max`), used by roster capacity normalization.
    ///
    /// Idempotent: `new_max <= max_records` returns the bytes unchanged.
    ///
    /// [`append_record`]: TdbFile::append_record
    pub fn grow_table_max(&self, table_raw: &str, new_max: u16) -> Result<Vec<u8>, AllocError> {
        let t = self.table(table_raw).ok_or(AllocError::UnknownTable)?;
        let max = t.max_records;
        if new_max <= max {
            return Ok(self.bytes.to_vec());
        }
        let reclen = t.record_len_bytes as usize;
        let added = (new_max - max) as usize * reclen;
        let insert_at = t.record_block + max as usize * reclen;
        if insert_at > self.bytes.len() {
            return Err(AllocError::OutOfBounds);
        }
        let block_offset = t.block_offset;

        let mut buf = Vec::with_capacity(self.bytes.len() + added);
        buf.extend_from_slice(&self.bytes[..insert_at]);
        buf.extend(std::iter::repeat_n(0u8, added));
        buf.extend_from_slice(&self.bytes[insert_at..]);

        // Shift the directory offset of every table whose block begins at/after the insertion point.
        let dir_end = FILE_HEADER + 8 * self.tables.len();
        for i in 0..self.tables.len() {
            let entry = FILE_HEADER + i * 8;
            let off = u32_at(&buf, entry + 4, self.endian) as usize;
            if dir_end + off >= insert_at {
                write_u32(&mut buf, entry + 4, (off + added) as u32, self.endian);
            }
        }
        // Grow the table's max (current stays) and the file's db_size.
        write_u16(&mut buf, block_offset + 0x14, new_max, self.endian);
        write_u32(&mut buf, 0x08, self.db_size + added as u32, self.endian);

        // Recompute all CRCs over the new structure (re-parse a clone to satisfy the borrow checker).
        let view = buf.clone();
        let db = TdbFile::parse(&view).map_err(AllocError::Reparse)?;
        db.recompute_crcs(&mut buf);
        Ok(buf)
    }

    /// Remove a record by **swap-pop**: the last record is moved into the removed slot and
    /// `current_records` is decremented (the freed slot stays allocated as reusable slack, so the file
    /// length is unchanged). Returns the **new file bytes** (CRCs recomputed) and, when a relocation
    /// happened, `Some(old_tail_index)` — the index the moved record *used to* occupy.
    ///
    /// ⚠️ **Swap-pop tail-row hazard.** When `Some(old)` is returned, any reference elsewhere to record
    /// `old` of this table now dangles (that row lives at `index` now). The caller (a Layer 3
    /// `delete_*` operation) must repoint such references from `old` → `index`.
    pub fn remove_record(
        &self,
        table_raw: &str,
        index: u32,
    ) -> Result<(Vec<u8>, Option<u32>), AllocError> {
        let t = self.table(table_raw).ok_or(AllocError::UnknownTable)?;
        let cur = t.current_records as u32;
        if index >= cur {
            return Err(AllocError::IndexOutOfRange);
        }
        let reclen = t.record_len_bytes as usize;
        let last = cur - 1;
        let tail = t.record_block + last as usize * reclen;
        if tail + reclen > self.bytes.len() {
            return Err(AllocError::OutOfBounds);
        }

        let mut buf = self.bytes.to_vec();
        let moved = if index != last {
            let dst = t.record_block + index as usize * reclen;
            buf.copy_within(tail..tail + reclen, dst);
            Some(last)
        } else {
            None
        };
        buf[tail..tail + reclen].fill(0); // clear the vacated tail slot
        write_u16(
            &mut buf,
            t.block_offset + 0x16,
            (cur - 1) as u16,
            self.endian,
        );
        self.recompute_crcs(&mut buf);
        Ok((buf, moved))
    }

    /// Insert a zeroed record **at `index`**, shifting records `[index, current)` down by one so their
    /// relative order is preserved, and return the new file bytes (CRCs recomputed). `index` may equal
    /// `current_records` (then this is exactly [`append_record`], appending at the end).
    ///
    /// Order-preserving counterpart to [`append_record`]. Required for tables whose record order is
    /// load-bearing — notably `eGlu`, which is partitioned into contiguous per-team blocks
    /// (`docs/audits/towx-blocks.md`): growing a team's roster means inserting a slot *at that team's
    /// block boundary*, not appending at the table end (which would break block coherence).
    ///
    /// Same two paths as [`append_record`]: **slack reuse** when `current < max` (file length
    /// unchanged), or **table growth** by one record when full. Unlike swap-oriented ops this never
    /// relocates a far row into the hole — every record keeps its order; indices at/after `index` shift
    /// up by one, so a caller holding raw indices past `index` must re-resolve.
    pub fn insert_record(&self, table_raw: &str, index: u32) -> Result<Vec<u8>, AllocError> {
        let t = self.table(table_raw).ok_or(AllocError::UnknownTable)?;
        let reclen = t.record_len_bytes as usize;
        let cur = t.current_records;
        let max = t.max_records;
        let block_offset = t.block_offset;
        let record_block = t.record_block;
        if index > cur as u32 {
            return Err(AllocError::IndexOutOfRange);
        }

        // Secure an allocated slack slot to receive the shifted tail; grow the table if it is full.
        // `grew` selects the CRC path: a grown file has new offsets, so it must be re-parsed.
        let mut grew = false;
        let mut buf = if cur < max {
            self.bytes.to_vec()
        } else {
            if max == u16::MAX {
                return Err(AllocError::CapacityExhausted);
            }
            let insert_at = record_block + max as usize * reclen;
            if insert_at > self.bytes.len() {
                return Err(AllocError::OutOfBounds);
            }
            let mut b = Vec::with_capacity(self.bytes.len() + reclen);
            b.extend_from_slice(&self.bytes[..insert_at]);
            b.extend(std::iter::repeat_n(0u8, reclen));
            b.extend_from_slice(&self.bytes[insert_at..]);
            // Shift the directory offset of every table whose block begins at/after the insertion.
            let dir_end = FILE_HEADER + 8 * self.tables.len();
            for i in 0..self.tables.len() {
                let entry = FILE_HEADER + i * 8;
                let off = u32_at(&b, entry + 4, self.endian) as usize;
                if dir_end + off >= insert_at {
                    write_u32(&mut b, entry + 4, (off + reclen) as u32, self.endian);
                }
            }
            // Grow max + db_size now; current is bumped after the shift, below. The new slot is pure
            // slack (appended at the record region's end) until the shift moves the tail into it.
            write_u16(&mut b, block_offset + 0x14, max + 1, self.endian);
            write_u32(&mut b, 0x08, self.db_size + reclen as u32, self.endian);
            grew = true;
            b
        };

        // Open the hole at `index`: shift records [index, cur) up by one slot, then zero the hole.
        let hole = record_block + index as usize * reclen;
        let shift_end = record_block + cur as usize * reclen;
        if shift_end + reclen > buf.len() {
            return Err(AllocError::OutOfBounds);
        }
        buf.copy_within(hole..shift_end, hole + reclen);
        buf[hole..hole + reclen].fill(0);
        write_u16(&mut buf, block_offset + 0x16, cur + 1, self.endian);

        if grew {
            let view = buf.clone();
            let db = TdbFile::parse(&view).map_err(AllocError::Reparse)?;
            db.recompute_crcs(&mut buf);
        } else {
            self.recompute_crcs(&mut buf);
        }
        Ok(buf)
    }

    /// Remove a record by **ordered shift**: records `(index, current)` slide up by one to close the
    /// hole, the vacated tail slot is zeroed, and `current_records` is decremented (the freed slot
    /// stays allocated as reusable slack; file length unchanged). Returns the new file bytes (CRCs
    /// recomputed).
    ///
    /// Order-preserving counterpart to [`remove_record`]'s swap-pop. Use it for tables whose record
    /// order matters (e.g. `eGlu` per-team blocks): closing the hole shifts later records — and later
    /// blocks — down by one index while every block stays contiguous and the table still tiles. No far
    /// tail row is dropped into the hole, so no `dXSB`-style block corruption. Indices past `index`
    /// shift down by one; a caller holding raw indices must re-resolve.
    pub fn remove_record_ordered(
        &self,
        table_raw: &str,
        index: u32,
    ) -> Result<Vec<u8>, AllocError> {
        let t = self.table(table_raw).ok_or(AllocError::UnknownTable)?;
        let cur = t.current_records as u32;
        if index >= cur {
            return Err(AllocError::IndexOutOfRange);
        }
        let reclen = t.record_len_bytes as usize;
        let last = cur - 1;
        let tail = t.record_block + last as usize * reclen;
        if tail + reclen > self.bytes.len() {
            return Err(AllocError::OutOfBounds);
        }

        let mut buf = self.bytes.to_vec();
        // Slide (index, cur) up into [index, cur-1). When index == last this range is empty (a plain
        // tail pop); copy_within handles the overlap like memmove.
        let src_start = t.record_block + (index + 1) as usize * reclen;
        let src_end = t.record_block + cur as usize * reclen;
        let dst = t.record_block + index as usize * reclen;
        buf.copy_within(src_start..src_end, dst);
        buf[tail..tail + reclen].fill(0);
        write_u16(&mut buf, t.block_offset + 0x16, last as u16, self.endian);
        self.recompute_crcs(&mut buf);
        Ok(buf)
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

    #[test]
    fn write_is_inverse_of_read() {
        for endian in [Endian::Big, Endian::Little] {
            for &(off, width, val) in &[
                (0u32, 1u32, 1u64),
                (3, 5, 17),
                (7, 9, 300),
                (1, 16, 0xABCD),
                (5, 31, 0x1234_5678),
            ] {
                let masked = val & ((1u128 << width) - 1) as u64;
                let mut b = [0u8; 8];
                write_bits(&mut b, off, width, val, endian);
                assert_eq!(
                    read_bits(&b, off, width, endian),
                    masked,
                    "{endian:?} {off}/{width}"
                );
            }
        }
        // Adjacent fields don't disturb each other.
        let mut b = [0u8; 8];
        write_bits(&mut b, 0, 3, 0b101, Endian::Big);
        write_bits(&mut b, 3, 5, 0b10110, Endian::Big);
        assert_eq!(read_bits(&b, 0, 3, Endian::Big), 0b101);
        assert_eq!(read_bits(&b, 3, 5, Endian::Big), 0b10110);
    }
}
