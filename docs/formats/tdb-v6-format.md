# TDB v6 Binary Format (verified)

> **VERIFIED.** Full byte-exact spec: **[tdb-v6-format-spec.md](tdb-v6-format-spec.md)** (user-provided
> from the prior app, independently verified against our anchors). This page is the short summary +
> what we confirmed. **Big-endian** on PS3 (endianness is file-header byte `[4]`: 0=LE, 1=BE).
> Internal `version = 6`. Applies to `nhlng.db`, `nhl_eng_us.db`, `draft_roster.db`, `dynamic_ratings.db`.

## Id-reversal rule

4-char ids are byte-reversed on big-endian files. The **binary and `nhlng-meta.xml`** use the **raw**
order (table `cPbu`, field `BSXd`); the **EA DB Editor XML / TSV audits / EDIT-MODEL** use the
**reversed "real id"** (`ubPc`, `dXSB`). Reverse the 4 chars to convert.

## Layout (verified)

```
[0x00] File header (0x18 = 24 B): "DB"(0x00); endian byte(0x04); db_size u32(0x08);
       table_count u32(0x10); header CRC(0x14)  [covers file[0x00..0x14]]
[0x18] Table directory: table_count × 8 B = { id(4) , u32 data_offset } ; data_base = 0x18+8*count
[...]  Per table:
         Table header (0x28 = 40 B): prior_crc(+0), data_alloc_type(+4), record_len_bytes(+8),
            record_len_bits(+0x0C), max_records u16(+0x14), current_records u16(+0x16),
            num_fields u8(+0x1C), index_count u8(+0x1D), header_crc(+0x24)
         num_fields × FieldDescriptor (0x10 = 16 B)
         Record block (record_len_bytes × rows)
         [type 34/66 only] varchar extra-data pool (uncompressed / Huffman)
[EOF-4] trailing EOF CRC
```

### Field descriptor (16 bytes) — CORRECTED

| Offset | Field | Meaning |
|---:|---|---|
| `[0..4]` | `kind_code` | type tag: `0`=inline string, `1`=inline binary, `3`=fixed-bit uint (the workhorse), `13`/`14`=varchar (1/2-byte len prefix into the pool), `2`=signed, `4`=float |
| `[4..8]` | `record_bit_offset` | bit position of the field within a record |
| `[8..12]` | `id` | 4-char field id (raw; reverse for real id) |
| `[12..16]` | `bit_width` | storage width ("depth"); matches meta `depth` |

> **Correction:** an earlier version of this page placed the `id` at `[0..4]` and a 0x30 header. That
> was a +8-shifted read that *happened* to capture id+width correctly; the true layout (40-byte header;
> `kind_code, bit_offset, id, bit_width`) is verified above. Signedness is **not** in `kind_code` — a
> field is signed iff its meta `range_low < 0` (two's-complement at the meta layer).

### Record decoding

Records are bit-packed; **big-endian = MSB-first** (PS3). A value =
`read_be_bits(record, bit_offset, bit_width)`. Record `i` starts at `i * record_len_bytes` in the
block; row count from `data_alloc_type` (`current_records`, except types 2/6 use `max_records`).
Varchars (kind 13/14) store a **4-byte BE offset** into the table's extra-data pool. See spec §1, §3.

## Integrity (verified — write path unlocked)

| Hash | Algorithm | Covers / stored |
|---|---|---|
| Inner-DB CRCs (header, prior_crc, header_crc, EOF) | **CRC-32/MPEG-2** (poly `0x04C11DB7`, init `0xFFFFFFFF`, no reflection, xorout 0) | header hash covers `file[0x00..0x14]`, stored at `0x14` (file endianness) |
| Save container `checksum_a` @`0x10` | **CRC-32/IEEE** (reflected) | `file[0x1C..EOF]`, stored BE |
| Save container `checksum_b` @`0x28` | **CRC-32/BZIP2** | `file[0x2C..EOF]`, stored BE |

Recompute order on write: inner-DB CRCs (prior_crc → header_crc → EOF → header CRC last); then save
container `checksum_b` then `checksum_a`. See spec §2.

> **CRC body scope (verified refinement):** each table's "body" — hashed by the *next* table's
> `prior_crc`, and by the EOF CRC for the last table — is `[table_block + 0x28 .. body_end)`, i.e. it
> **starts right after the 40-byte header and includes the field descriptors**, not just the record
> region. `body_end = (block+0x28+num_fields*16) + record_len_bytes * rows` where `rows = max_records`
> for alloc types {2,6,10,14} else `current_records`; for alloc types {34,66} (extra-data pool)
> `body_end` = the next table's block start. Table 0's `prior_crc` hashes the directory
> (`file[0x18 .. 0x18+8*num_tables]`). **Verified: all four CRC scopes match across all 7 games** —
> so a no-op read→serialize is byte-identical, and edited writes are valid.

## What we independently verified (against `anchors/ps3/**`)

- table set + per-field id & `bit_width` match `nhlng-meta.xml` 100% (all games, ~17k fields);
- **records tile cleanly** (offset+width within `record_len_bits`) for 134/134 tables;
- **inner-DB header CRC matches all 7 games**; CRC known-answers pass;
- **container `checksum_a`/`checksum_b` match** on every v2/v4 save tested;
- **record decode → sane values** (position 0–4, captaincy `{0:4872,1:220,2:440}`, jersey 0–99, skin 1–10).

Verified per-game schema corpus: `../audits/data/schema/<game>-schema.json`; drift:
`../audits/data/drift-matrix.csv` ([../audits/nhlng-schema.md](../audits/nhlng-schema.md)).

## Still open

- **Huffman varchar codec** (type-66 pools) — spec §3.1; verify when we decode varchar strings.
- **PFD/AES** for raw (non-exported) PS3 saves — **net-new**, no implementation exists (spec §5);
  the anchors are already-decrypted exports.
- **v1 container write** (NHL10/11) — `tag_1c@0x1C` un-reversed → v1 read-only (spec §4).
