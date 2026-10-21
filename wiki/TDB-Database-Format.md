# TDB Database Format

**Evidence:** measured. The layout below re-parses every table of every reference database (ids and widths match the metadata XML 100%, records tile cleanly), the CRC routines reproduce every stored checksum, and a no-edit read-then-write round trip is byte-identical on every anchor in both byte orders. Two items are marked open below.

EA's NHL games store their editable data in **TDB version 6** files: `nhlng.db`, `NHL07NG.db`, `nhl_game.db`, the localization dictionaries, `draft_roster.db` and others. A TDB is a flat set of tables; each table has a fixed record size, a list of fields packed at bit offsets inside the record, optional variable-length string storage, and CRC-32 checksums that must be recomputed after any change.

## File layout

```
0x00  File header, 24 bytes
0x18  Table directory: one 8-byte entry per table { 4-char id, u32 data offset }
...   Table data section, one block per table:
        40-byte table header
        16 bytes per field descriptor
        record block (record_length_bytes × rows)
        [tables of allocation type 34 or 66 only] extra-data pool for varchars
EOF-4 trailing CRC over the last table's body
```

**File header.** `"DB"` magic at offset 0; the **endianness byte at offset 4** (0 little-endian, 1 big-endian) governs every multi-byte integer and the bit-packing direction; `db_size` at offset 8; `table_count` at offset 16; a CRC at offset 20 covering bytes 0 to 19.

**Table directory.** `table_count` entries of `{ id (4 bytes), data_offset (u32) }`. Data offsets are relative to the end of the directory.

**Table header (40 bytes).**

| Offset | Field |
|---|---|
| +0 | `prior_crc`: CRC of the *previous* table's body (for the first table, of the directory) |
| +4 | `data_allocation_type` |
| +8 | `record_length_bytes` |
| +12 | `record_length_bits` |
| +20 | `max_records` (u16): allocated rows |
| +22 | `current_records` (u16): populated rows |
| +28 | `num_fields` (u8) |
| +29 | `index_count` (u8) |
| +36 | `header_crc`: CRC of header bytes +4 to +35 |

**Field descriptor (16 bytes).** Four whole 32-bit words: `kind_code`, `record_bit_offset`, `id` (4 raw bytes), `bit_width`. "Depth" in the metadata vocabulary is `bit_width`.

| `kind_code` | Meaning |
|---|---|
| 0 | fixed inline string (byte length = `bit_width / 8`) |
| 1 | fixed inline binary |
| 3 | fixed-width unsigned integer: the workhorse for ratings, ids and flags |
| 13 | varchar with a 1-byte length prefix, stored in the extra-data pool |
| 14 | varchar with a 2-byte length prefix |
| 2, 4 | signed and float: declared but not used by these games' files |

Signedness is **not** in the kind code. A field is signed if its metadata declares `rangelow < 0`; see [Field Encodings](Field-Encodings) for how negative values are actually stored.

## Records

Records are bit-packed with no alignment padding beyond the byte-rounded record length. Record *i* starts at `i × record_length_bytes`. A value is read from `record_bit_offset` for `bit_width` bits:

- **big-endian files (X360, PS3):** bit 0 is the most significant bit of byte 0; read MSB-first;
- **little-endian files (PS4):** bit 0 is the least significant bit of byte 0; read LSB-first.

How many rows to read depends on `data_allocation_type`: types 2 and 6 hold `max_records` rows all treated as populated; types 10, 14, 34 and 66 hold `max_records` rows of which the first `current_records` are populated; other types iterate `current_records`.

The main databases on Xbox 360 and PlayStation 3 are **packed**: `max_records` equals `current_records` for the roster tables, so adding a row means growing the table. The PlayStation 4 main database and every platform's roster save carry slack above the population; see [Roster Model](Roster-Model).

## Strings

Fixed inline strings (kind 0) sit in the record, byte-aligned, UTF-8 with NUL padding stripped.

Varchars (kinds 13 and 14) store a **4-byte big-endian byte offset** in the record, regardless of file endianness, pointing into the table's extra-data pool. `0xFFFFFFFF` means missing. At the target sits a big-endian length prefix (1 or 2 bytes) and then the data. For allocation type 34 the prefix is a byte count and the data is plain; for type 66 the prefix is a decoded character count and the data is a Huffman bitstream. See [TDB String Pools and Localization Database](TDB-String-Pools-and-Localization-Database).

Text in the dictionaries uses Windows-1252, not UTF-8 (the curly apostrophe is byte 0x92, the non-breaking space 0xA0).

## Checksums

All inner CRCs use **CRC-32/MPEG-2**: polynomial 0x04C11DB7, initial value 0xFFFFFFFF, no input or output reflection, no final XOR. Stored in the file's byte order.

| Scope | Where stored | Covers |
|---|---|---|
| File header CRC | offset 20 | bytes 0 to 19 |
| Table `prior_crc` | table header +0 | the previous table's body (its bytes from header end to the next table's block offset, including any padding); for the first table, the directory |
| Table `header_crc` | table header +36 | header bytes +4 to +35 |
| EOF CRC | last 4 bytes of the file | the last table's body, up to `file length − 4` |

Recompute order after an edit: write every non-CRC byte; then each table's `prior_crc` in data-offset order; each `header_crc`; the EOF CRC; and the file-header CRC last, so it reflects the final table count and size.

Known answers for checking an implementation: CRC-32/MPEG-2 of the ASCII string `123456789` is `0x0376E6E7`.

## What changes with the edit model

Changing a fixed-width value is a bit write in place plus the four CRC recomputations. Changing a varchar in a type-66 table rebuilds that table's whole pool. Adding rows to a packed table grows the record block and shifts every later table's data offset. Studio refuses to grow a table past its allocation unless capacity normalization has been applied; see [Roster Model](Roster-Model).

## Open points

- Whether every field with `rangelow < 0` is stored as offset-binary rather than two's complement (the two measured sites are offset-binary; see [Field Encodings](Field-Encodings)).
- The meaning of the unnamed words at table header +16, +24 and +30, preserved verbatim.

Last reviewed: 2026-10-20
