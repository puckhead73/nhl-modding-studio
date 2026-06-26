# EA NHL "TDB v6" + RosterFile Save — Format Specification

> **VERIFICATION STATUS (2026-06-26): VERIFIED against our PS3 anchors.** User-provided spec from the
> prior app; per the clean-room rule it was treated as a claim and independently checked against
> `anchors/ps3/**`. Confirmed end-to-end:
> - **Descriptor layout** (40-byte table header `0x28`; 16-byte descriptor `[kind_code,
>   record_bit_offset, id, bit_width]`): all 2969/2965 fields re-parse with ids+widths matching the
>   meta XML, kind_codes ∈ {0,3}, and records **tile cleanly 134/134 tables**. *(This corrected our
>   earlier reverse-engineering, which had read id+width from a +8-shifted window.)*
> - **Inner-DB CRC** (Routine A, CRC-32/MPEG-2): `crc32_tdb(file[0x00..0x14])` == the stored header
>   hash on **all 7 games**; known-answers pass.
> - **Container checksums** (A=CRC-32/IEEE over `[0x1C..]`, B=CRC-32/BZIP2 over `[0x2C..]`): match on
>   every v2/v4 save tested (nhl12/13/14/15/legacy).
> - **Record decoding** (BE MSB-first): decodes known fields to sane values — position 0–4, captaincy
>   `{0:4872,1:220,2:440}`, jersey 0–99, skin 1–10.
> - **Enums** §6 (position/captaincy/skin) confirmed by value distribution.
> - **Not yet verified:** Huffman varchar codec §3.1 (verify when decoding type-66 pools).
>   **Out of scope / net-new:** PFD/AES §5 (the spec itself states none exists — matches our finding).
>
> The `file:line` citations point at the prior `nhl-database-studio` repo (provenance only — that code
> was **not** read; verification used the anchor binaries).

---

Extracted read-only from the engine crates at `e:\Repositories\nhl-database-studio`.
Big-endian PS3/Xbox360 (NHL10–15 + Legacy); the same code paths also read little-endian PC files.
Every structural claim is cited `file:line`. Constants are the literal values in the code.

This document answers a greenfield rebuild's seven questions:
1. [TDB v6 record decoding](#1-tdb-v6-record-decoding)
2. [Integrity algorithms](#2-integrity-algorithms)
3. [Compression codecs](#3-compression-codecs)
4. [Save container layout](#4-save-container-layout)
5. [PS3 PFD / AES](#5-ps3-pfd--aes)
6. [Confirmed value→label mappings](#6-confirmed-valuelabel-mappings)
7. [Worked examples](#worked-examples) (inline per section)

---

## 0. File layout overview

```
[0x00] File header (0x18 = 24 bytes)                 tdb-core/header.rs:7  HEADER_SIZE=0x18
[0x18] Table directory: num_tables x 8 bytes          tdb-core/file.rs:9   DIRECTORY_ENTRY_SIZE=8
       per entry: {4B disk id}{4B u32 data_offset}     tdb-core/file.rs:50-58
[ ...] Table-data section (right after the directory)  tdb-core/table.rs:115-116
       per table:
         Table header (0x28 = 40 bytes)               tdb-core/table.rs:10  TABLE_HEADER_SIZE=0x28
         num_fields x FieldDescriptor (0x10 = 16 B)    tdb-core/table.rs:13  FIELD_DESCRIPTOR_SIZE=0x10
         Record block (record_length_bytes x rows)     tdb-core/record.rs:120-131
         [optional] extra-data section (varchar pool)  tdb-core/record.rs:137-141
[EOF-4] trailing 4-byte EOF CRC                        tdb-core/table.rs:199-200
```

Endianness is selected by **file-header byte `[4]`**: `0`=Little, `1`=Big (`tdb-core/endian.rs:12-18`, `header.rs:46`). It governs every multi-byte integer read AND the record bit-packing direction.

Magic: `"DB"` = `0x44 0x42` at offset `0x00` (`header.rs:10`, `MAGIC_DB=0x4442`). The 32-bit `db_size` lives at body offset `0x08`.

4-char table/field ids are **byte-reversed on big-endian files** so the canonical id matches community names: on-disk `[a,b,c,d]` → canonical `[d,c,b,a]` (`tdb-core/raw_id.rs:24-30`). This is the same reversal that bridges live ids to the `nhlng-meta` namespace (e.g. `dSvy.AdEb` ⇄ meta `yvSd.bEdA`).

---

## 1. TDB v6 record decoding

### 1.1 Field descriptor — 16 bytes

Parsed at `tdb-core/table.rs:85-94`. Exactly `0x10` bytes:

| Bytes | Field | Type | Meaning |
|---|---|---|---|
| `[0..4]`  | `kind_code`         | u32 (endian) | **type tag** (string/binary/uint/varchar) |
| `[4..8]`  | `record_bit_offset` | u32 (endian) | **bit offset** of this field within one record |
| `[8..12]` | `id`                | 4 raw bytes → `RawId` | field 4-char id (byte-reversed on BE) |
| `[12..16]`| `bit_width`         | u32 (endian) | **bit width / "depth"** of the field |

So the descriptor's **last 8 bytes = `id` (8–11) + `bit_width` (12–15)**; the **type tag + bit offset are the first 8 bytes**. Each 4-byte slot is a plain whole 32-bit integer — there is **no sub-packing inside any slot** (`table.rs:89-92`). "Depth" in the `nhlng-meta` vocabulary is this `bit_width`; the external defs carry a redundant `bit_depth` used only as a cross-check (`tdb-defs/field_meta.rs:55-57`).

### 1.2 Type tag (`kind_code`) → value class

Dispatch at `tdb-core/record.rs:188-200`; semantics documented `record.rs:10-50`:

| `kind_code` | Class | Decoder | Notes |
|---|---|---|---|
| `0`  | STRING, fixed inline UTF-8 | `decode_inline_string` (`record.rs:189,215`) | byte_len = `bit_width/8` |
| `1`  | BINARY, fixed inline blob  | `decode_inline_bytes`  (`record.rs:190,227`) | raw bytes preserved |
| `3`  | UINT, fixed-bit numeric    | `decode_fixed_bit_uint`(`record.rs:191,202`) | the workhorse: sliders/ids/flags |
| `13` | VARCHAR, 1-byte length prefix | `decode_var_string` (`record.rs:192,236`) | pointer into extra-data |
| `14` | VARCHAR, 2-byte length prefix | `decode_var_string` (`record.rs:192,236`) | pointer into extra-data |
| `2`  | signed int  | NotDecoded here (`record.rs:48,198`) | signedness handled at meta layer (§1.6) |
| `4`  | float       | NotDecoded (`record.rs:48,198`) | |
| other| unknown     | NotDecoded (`record.rs:198`) | |

**Signedness is NOT in `kind_code`.** Numeric fields decode as raw `kind_code==3` bits regardless of sign; the signed interpretation is applied later from range metadata (§1.6).

### 1.3 Bit-packing order & extraction

Endian-dependent, mirroring the JS `bit-buffer` reference (`tdb-core/bitview.rs:74-85`):

- **Little-endian → LSB-first.** Bit 0 = LSB of byte 0. `value = (u128_le >> from_top) & mask` (`bitview.rs:121-127`).
- **Big-endian → MSB-first** (the PS3 case). Bit 0 = MSB of byte 0; field read from the top of the loaded span (`bitview.rs:128-135`).

`BitView::read(bit_offset, width)` (`bitview.rs:97-137`):
```
byte_start = bit_offset / 8
byte_end   = (bit_offset + width + 7) / 8        // ceil
span       = bytes[byte_start..byte_end]
from_top   = bit_offset - byte_start*8
mask       = width==64 ? u64::MAX : (1<<width)-1
LE: acc = sum(span[i] << (i*8));        value = (acc >> from_top) & mask
BE: acc = fold((acc<<8)|b);  shift = span_bits - from_top - width;  value = (acc >> shift) & mask
```
Guards: width 0 → `Some(0)`; width > 64 → `None`; out-of-range → `None`. The numeric decoder also rejects width `0` or `> 64` (`record.rs:203-205`). Result is raw `u64` bits in `FixedBitNumeric { raw_bits, bit_width }` — no sign handling here. `write_bits` (`bitview.rs:12-72`) is the exact inverse (mirror tests `bitview.rs:252-358`).

### 1.4 Record stride / alignment / placement

- **Stride = `record_length_bytes`** (table header offset 8, `table.rs:51`; used as `record_bytes` `record.rs:110`). The header also has `record_length_bits` (offset 12) but decoding uses the byte length. **No per-record alignment padding beyond the byte-rounded stride** — the last field may leave unused bits in the final byte.
- **Record `i` starts at** `i * record_bytes` within the block (`record.rs:158`); the block starts immediately after the last 16-byte descriptor (`table.rs:138,160-169`).
- **Row count** depends on `data_allocation_type` (`record.rs:114-118`): types `2,6` iterate `max_records`; types `10,14,34,66` hold `max_records` rows but iterate `current_records`; others iterate `current_records`. Each row's `populated` flag = `i < current_records` (`record.rs:174`). Truncation past EOF is a hard `RecordsTruncated` error (`record.rs:120-130`).

40-byte table header layout (`table.rs:49-63`): `prior_crc`(0), `data_allocation_type`(4), `record_length_bytes`(8), `record_length_bits`(12), `reserved_zero1`(16), `max_records`(u16@20), `current_records`(u16@22), `unknown2`(24), `num_fields`(u8@28), `index_count`(u8@29), `reserved_zero2`(u16@30), `reserved_zero3`(32), `header_crc`(36).

### 1.5 Strings / varchars

**(a) Fixed inline** (`kind_code` 0 string / 1 binary): stored in the record. Must be byte-aligned — both `record_bit_offset % 8 == 0` and `bit_width % 8 == 0`, else `NotDecoded` (`record.rs:322-332`). Bytes = `[offset/8 .. +bit_width/8)`. Decoded **UTF-8 lossy**, NULs stripped (`record.rs:342-367`; EA stores UTF-8 — e.g. "Éric" = `C3 89 72 69 63`).

**(b) Variable-length varchar** (`kind_code` 13/14): stored in the **table extra-data section**, which exists only for `data_allocation_type` 34 (uncompressed) or 66 (Huffman) (`record.rs:137-141`). The in-record cell is **always a 32-bit BIG-ENDIAN byte offset** into that section, regardless of file endianness (`record.rs:242-259`), and must be byte-aligned. Sentinel `0xFFFFFFFF` (or any offset past the section) → `VarStringMissing` (`record.rs:271-286`). At the target: a length prefix — 1 byte (kind 13) or 2 bytes (kind 14), **big-endian** — then the data:
- **Type 34:** prefix = byte count; string = next `length` bytes (`record.rs:303-312`).
- **Type 66:** prefix = decoded **character count**; bits follow and feed the Huffman decoder (`record.rs:290-301`). See §3.

### 1.6 Signed values (two's complement)

The reader treats every numeric as unsigned raw bits; sign interpretation lives in `tdb-defs`, driven by the EA range metadata.

A field is **signed iff `range_low < 0`** (`tdb-defs/field_meta.rs:95-97`). Sign extension — `FieldMeta::decode_raw` (`field_meta.rs:105-117`):
```
if is_signed && (1..64).contains(bit_width):
    sign_bit = 1 << (bit_width - 1)
    if raw_bits & sign_bit: return (raw_bits as i64) - (1 << bit_width)   // two's complement
return raw_bits as i64
```
Width 64 falls through to a plain reinterpret (`decode_raw(u64::MAX,64) == -1`, `field_meta.rs:524`). Range validation runs on the **decoded** value (`field_meta.rs:121-133`).

**Worked vector** (8-bit slider, range -50..=50, `field_meta.rs:506-516`): raw `246` → `246-256 = -10`; raw `206` → `-50`; raw `50` → `50`.

### Worked example — decode `ddep` (DCHT depth field)

Real fixture, little-endian, 8-byte record `FA CE 00 80 10 00 00 00` (`bitview.rs:181-194`). `ddep` is at bit offset 31, width 5:
1. `byte_start = 31/8 = 3`; `byte_end = (31+5+7)/8 = 5` → span = `80 10`.
2. `from_top = 31 - 24 = 7`.
3. LE assemble: `acc = 0x80 | (0x10 << 8) = 0x1080`.
4. `mask = (1<<5)-1 = 0x1F`.
5. `(0x1080 >> 7) & 0x1F = 0x21 & 0x1F = 1`.

→ `raw_bits = 1`; range is non-negative so `decode_raw = 1`. Confirmed `assert_eq!(v.read(31,5), Some(1))` (`bitview.rs:193`). The BE analogue uses `shift = span_bits - from_top - width` (cross-byte BE example `read(5,16)==0xAB58` traced at `bitview.rs:205-216`).

---

## 2. Integrity algorithms

> Offset framing correction vs the common shorthand:
> - The **file-header hash is stored AT offset `0x14`** (parsed as `unknown3`); it covers `file[0x00..0x14]`.
> - There is no "per-table hash of self at offset 0". The table's `+0x00` slot is `prior_crc` (covers the *previous* table's body). Each table also has `header_crc` at `+0x24`.
> - The container checksum at `0x10` (`checksum_a`) is exactly as described; there is also `checksum_b` at `0x28`.

### Routine A — `crc32_tdb` (all four inner-DB scopes) = **CRC-32/MPEG-2**

`tdb-core/crc.rs`. Built as `!crc32_bzip2(bytes)` (`crc.rs:55-57`), which cancels BZIP2's XorOut → effective XorOut 0.

| Param | Value | Cite |
|---|---|---|
| Polynomial | `0x04C11DB7` | `crc.rs:10` |
| Init | `0xFFFFFFFF` | `crc.rs:39` |
| Input reflection | none (MSB-first table) | `crc.rs:21-28` |
| Output reflection | none | |
| XorOut (effective) | `0x00000000` | `crc.rs:44,56` |
| Update | `crc = (crc<<8) ^ TABLE[(crc>>24) ^ b]` | `crc.rs:41-42` |
| Stored | in **file endianness** via `write_u32` | `serialize.rs:180-186` |

Known answers: `crc32_bzip2("123456789") = 0xFC891918` (`crc.rs:75`); `crc32_tdb("123456789") = 0x0376E6E7` (`crc.rs:89`).

The four scopes that use Routine A:

| Scope | Stored at | Covers |
|---|---|---|
| **File-header hash** | file `0x14..0x18` (`header.rs:52`, write `serialize.rs:123-124`) | `file[0x00..0x14]` (first 20 bytes; excludes the hash slot) |
| **Table `prior_crc`** | table `+0x00` (`table.rs:50`) | **previous** table's body, or the directory `file[0x18 .. 0x18+8*num_tables]` for the first table (`serialize.rs:93-103`, `validate.rs:182-193`) |
| **Table `header_crc`** | table `+0x24` (`table.rs:62`) | `table[+4 .. +36]` (32 header bytes between the two CRC slots). **Write-only**: the reader returns `Unsupported` and does not verify it (`serialize.rs:108`, `validate.rs:199-201`) |
| **EOF CRC** | last 4 bytes of file | the **last** table's body (`serialize.rs:114-118`, `validate.rs:205-223`) |

`data_buffer_end` per table (`validate.rs:228-242`, `serialize.rs:199-213`): types `2|6|10|14` → `data_start + record_length_bytes*max_records`; types `34|66` → start of next table (includes extra-data); default → `data_start + record_length_bytes*current_records`.

### Routine B — container `checksum_a` @ `0x10` = **CRC-32/IEEE (reflected)**

`tdb-savedata/checksum.rs` (deliberately re-implemented, no `tdb-core` dependency).

| Param | Value | Cite |
|---|---|---|
| Polynomial (reflected) | `0xEDB88320` | `checksum.rs:51` |
| Init | `0xFFFFFFFF` | `checksum.rs:86` |
| In/out reflection | yes / yes | `checksum.rs:43-58` |
| XorOut | `0xFFFFFFFF` | `checksum.rs:90` |
| Covers | `file[0x1C..EOF]` (incl. padding and the stored `checksum_b`) | `CHECKSUM_A_REGION_START=0x1C` `checksum.rs:35,113` |
| Stored | **big-endian** at `0x10..0x14` | `pack.rs:184,226` |

Known answer: `crc32_ieee("123456789") = 0xCBF43926` (`checksum.rs:135`).

### Routine C — container `checksum_b` @ `0x28` = **CRC-32/BZIP2 (non-reflected, XorOut applied)**

Same family as Routine A but keeps the final complement.

| Param | Value | Cite |
|---|---|---|
| Polynomial | `0x04C11DB7` | `checksum.rs:74` |
| Init | `0xFFFFFFFF` | `checksum.rs:96` |
| In/out reflection | none / none | `checksum.rs:63-81` |
| XorOut | `0xFFFFFFFF` | `checksum.rs:100` |
| Covers | `file[0x2C..EOF]` (incl. padding) | `CHECKSUM_B_REGION_START=0x2C` `checksum.rs:37,124` |
| Stored | **big-endian** at `0x28..0x2C` | `pack.rs:182,224` |

`crc32_tdb` (Routine A) and `checksum_b` (Routine C) differ **only by the final complement**. `checksum_a` is a wholly different reflected/IEEE CRC.

The v1 (NHL10/11) container has a third tag at `0x1C` (`tag_1c`) whose algorithm is **un-reversed** — this is why v1 is read-only (`pack.rs:117`).

### Recompute order after editing

**Inner `.db`** (`serialize.rs:46-127`):
1. Write all non-CRC bytes (header fields, directory, table headers, descriptors, bodies).
2. `prior_crc` per table, in **data_offset-sorted** order (each hashes the previous table's now-final body; first hashes the directory).
3. `header_crc` per table over `[+4..+36]`.
4. EOF CRC over the last table's body → last 4 bytes.
5. **File-header CRC last**, over `file[0..20]`, written at `0x14` (must reflect final `num_tables`/`db_size`). All five use `crc32_tdb`, stored in file endianness.

**Save container** (`pack.rs`):
1. Assemble header (checksum slots zeroed) + body (zlib for v4 / raw `.db` for v2) + zero padding (v4).
2. `checksum_b` first (region `0x2C..` starts after both slots) → BE at `0x28`.
3. `checksum_a` second (region `0x1C..` covers stored `checksum_b`) → BE at `0x10`.

---

## 3. Compression codecs

Two **independent** systems — do not conflate:

| | DB-internal Huffman varchar | Save-container zlib |
|---|---|---|
| Scope | per-table varchar pool inside a `.db` | wraps a whole `.db` in a console save |
| Algorithm | custom EA static Huffman bitstream | standard zlib/DEFLATE (`flate2`) |
| Trigger | table `data_allocation_type == 66` | container `ZlibV4` (NHL14+) |

### 3.1 DB-internal Huffman varchar codec

`tdb-core/huffman.rs`. The pool lives in the table extra-data section: **Huffman tree first, then packed per-string bitstreams** (`huffman.rs:11-13`, `record.rs:135`).

**Tree on disk** (`huffman.rs:104-162`): a level-order (BFS) sequence of 2-byte child descriptors `[index, value]`. Root seeded as a placeholder internal node at arena index 0. For each parent: read 4 bytes = LEFT descriptor then RIGHT descriptor. `index == 0` → **leaf** carrying byte `value`; nonzero `index` → **internal** (its `value` byte ignored). **Critical quirk:** when enqueuing newly-discovered internal children, push the **RIGHT child before the LEFT** (`huffman.rs:24-29,145-151`; test `huffman.rs:497-528`). Truncation → `None`.

**Code table** (`huffman.rs:172-222`): DFS from root, **left appends bit `0`, right appends bit `1`**, MSB-first in the low `bit_count` bits. Depth capped at 64; EA trees top out ~16 bits.

**Per-string format**: `[length prefix BE][bitstream]`. Prefix = 1 byte (kind 13) / 2 bytes (kind 14), **big-endian**, holding the decoded **character (byte) count** — NOT a bitstream byte length. **Bits read MSB-first** within each byte (`huffman.rs:291-295`). Decode: bit `0` = left, `1` = right; at a leaf emit the byte and reset to root; **stop after exactly `char_count` bytes** (`huffman.rs:315-318`). **No in-band EOF symbol** — the length prefix is the sole terminator; the pool is densely packed so the decoder is handed the whole tail and self-terminates by count.

**Recompress reuses the on-disk tree** (`encode_bytes`, `huffman.rs:233-267`): look up each byte's code, pack **MSB-first** (`current |= 1 << (7 - bit_in_byte)`), zero-pad the final partial byte. A byte absent from the tree → `UnknownGlyph` error. **The writer never builds a new tree** (new-glyph support deferred).

**Worked vectors** (`huffman.rs:330-528`): tree `[0,'A', 1,0, 0,'B', 0,'C']` → codes A=`0`, B=`10`, C=`11`; `"ABCAB"` = bits `0 10 11 0 10` → MSB-packed `0x5A`. 4-leaf BFS tree `[1,0, 1,0, 0,'C', 0,'D', 0,'A', 0,'B']` → A=00,B=01,C=10,D=11; `"ABCD"` = `0x1B`.

### 3.2 type-66 string-pool rebuild

`tdb-edit/pool_rebuild.rs` — `rebuild_type66_pool` (`pool_rebuild.rs:70-248`). A single varchar edit ripples through every downstream offset, so the whole pool is rebuilt on save:

1. Locate extra-data `[record_block_end, next_table_offset)`; the last table ends at `source_size - 4` (the EOF CRC) (`pool_rebuild.rs:80-90,314-342`).
2. Parse the tree and **preserve its bytes verbatim**; `available_pool_size = section.len() - tree_byte_size` (`pool_rebuild.rs:92-100`).
3. Require `TableRecords::Decoded` (else `RecordsSkippedHuffman`).
4. Walk only the first `current_records` rows. Per varchar field: pick effective text (staged edit else original); `VarStringMissing` → write `0xFFFFFFFF`. **Deduplicate** by `(text_bytes, prefix_width)` via a BTreeMap. Length must fit the prefix width (`VarStringLengthExceedsPrefixWidth`). Encode via the **preserved tree's** `encode_bytes` (`UnknownGlyph` → `UnencodableGlyph`). Append `[len BE][bitstream]`, queue a cell write.
5. **Fit check:** `pool_bytes > available_pool_size` → `PoolDoesNotFit`. The pool may **shrink** (tail zero-padded) but must not grow past the original section (growing would shift every later table's `data_offset` — deferred).
6. Patch the new pool over `[tree_end .. tree_end+pool_len]`, zero-pad the tail, and patch every record cell with the new offset as **4-byte BE**.
7. Caller re-runs `write_tdb` to refresh CRCs.

**Enforced constraints:** *no-new-glyph* (any byte absent from the original tree fails loud) + *pool-fit / same-length-or-shrink*. Localization (`vCJG`) edits flow through this same path.

### 3.3 Save-container zlib (NHL14+)

Standard **RFC-1950 zlib / DEFLATE via `flate2`** — only in the `ZlibV4` container. **Only `file[0x30..]` is the zlib stream** (`78 01/9C/DA/5E` + DEFLATE + Adler-32), followed by zero padding (`tdb-savedata/header.rs:17,95`, `extract.rs:78`).

- **Inflate** (`extract.rs:47-105`): reject declared size > 128 MiB; `ZlibDecoder` over `data[0x30..]`; cross-check inflated length vs `uncompressed_size@0x24` within platform slack (PS3 = 0, Xbox360 = 4, the latter counting the Adler-32 trailer). Container tags are NOT verified on read — zlib's Adler-32 validates the stream.
- **Deflate** (`pack.rs:124-192`): `ZlibEncoder` at default level 6 over the new `.db`; error `DeflatedTooLarge` if `0x30 + compressed > original_len`; rebuild header; **pad with zeros back to the original file allocation** (PS3 over-allocates for in-place inflate); then recompute `checksum_b` then `checksum_a`.
- RawV2 (NHL12/13) writes the body uncompressed and tight (no padding) — proven byte-for-byte against real saves (`pack.rs:194-234`).

---

## 4. Save container layout

`tdb-savedata`. All header fields are **big-endian except `compressed_size` (LE)**. Magic (16 bytes, NUL-padded): PS3 `"PS3RosterFile"`, Xbox360/PC `"RosterFile"` (`header.rs:110,114`). Body start = `header_len()` (`header.rs:438-444`).

### Format discriminator — by BODY MARKER, not the version byte

The `version` byte @ `0x14` is unreliable (NHL10/11=1, NHL12=2, NHL13=0, NHL14/15=2, Legacy=4 — version 2 is shared by raw NHL12 *and* zlib NHL14). Resolution (`header.rs:325-347`):
- `version == 1` → **RawV1** (32B header).
- `version == 0` → **RawV2** (NHL13).
- `version == 2` → ambiguous: zlib magic `78 xx` at **0x30**? → **ZlibV4** (NHL14/15). Else `"DB"` at **0x2C**? → **RawV2** (NHL12). Else `NotTdb`.
- `version == 4` → **ZlibV4**.
- else → `UnsupportedVersion`.

### v1 — RawV1, 32-byte header (`V1_HEADER_LEN=0x20`, NHL10/11)

Uncompressed; body = literal `.db` at `0x20`.

| Offset | Size | Endian | Field | Meaning |
|---|---|---|---|---|
| 0x00 | 16 | ASCII | magic+pad | |
| 0x10 | 4 | BE | `checksum_a` | CRC-32/IEEE of `file[0x1C..]`; surfaced, **not validated** |
| 0x14 | 4 | BE | `version` | = 1 |
| 0x18 | 4 | BE | `flags` | = 0 |
| 0x1C | 4 | BE | `tag_1c` | second tag, **algorithm un-reversed** → v1 read-only |
| 0x20 | — | — | body | raw TDB to EOF (trimmed via inner `db_size@0x08`) |

### v2 — RawV2, 44-byte header (`V2_HEADER_LEN=0x2C`, NHL12/13)

| Offset | Size | Endian | Field |
|---|---|---|---|
| 0x00 | 16 | ASCII | magic+pad |
| 0x10 | 4 | BE | `checksum_a` (CRC-32/IEEE of `file[0x1C..]`) |
| 0x14 | 4 | BE | `version` (2=NHL12, 0=NHL13) |
| 0x18 | 4 | BE | `flags` (=0) |
| 0x1C | 4 | BE | `field_1c` (=1) |
| 0x20 | 4 | BE | `field_20` (=1) |
| 0x24 | 4 | BE | `uncompressed_size` |
| 0x28 | 4 | BE | `checksum_b` (CRC-32/BZIP2 of `file[0x2C..]`) |
| 0x2C | — | — | body: raw TDB, written tight (no padding) |

Both tags use the same formulas as v4 → fully writeable.

### v4 — ZlibV4, 48-byte header (`HEADER_LEN=0x30`, NHL14/15/Legacy)

| Offset | Size | Endian | Field | Meaning |
|---|---|---|---|---|
| 0x00 | 16 | ASCII | magic+pad | PS3 or Xbox |
| 0x10 | 4 | BE | `checksum_a` | CRC-32/IEEE of `file[0x1C..]` |
| 0x14 | 4 | BE | `version` | 4 (Legacy/PS3) or 2 (NHL14/15 Xbox) |
| 0x18 | 4 | BE | `flags` | = 0 |
| 0x1C | 4 | BE | `field_1c` | = 1 |
| 0x20 | 4 | BE | `field_20` | 2=PS3, 1=Xbox360 (platform marker) |
| 0x24 | 4 | BE | `uncompressed_size` | inflated `.db` length |
| 0x28 | 4 | BE | `checksum_b` | CRC-32/BZIP2 of `file[0x2C..]` |
| 0x2C | 4 | **LE** | `compressed_size` | length of the zlib stream |
| 0x30 | — | — | body | zlib stream + trailing zeros |

### Trailing sections, padding, multi-part

- **No trailing non-DB sections** inside a single container — only zero padding (v1/v4) or nothing (v2). No footer/index/per-part table.
- **Padding:** v4 (PS3) is over-allocated; the writer zero-pads back to the **original file allocation** (`pack.rs:171-175`). v2 is tight. v1/v2 reads trim trailing padding using the inner TDB `db_size@0x08` (endian from body byte `0x04`).
- Both checksum regions **include** the trailing padding.
- **`DATA000N` multi-part reassembly: NOT PRESENT.** A repo-wide grep for `DATA000*` / `part_count` / `num_parts` / `reassembl*` returns zero matches in save code. These saves are single-file: PS3 = one `SYS-DATA` file; Xbox360 (Xenia) = the single inner file named after its folder (`<folder>/<basename>`, else "the only regular file"). There is no concatenation logic because no save is split.

### Title IDs and container version per game

From `app/src-tauri/resources/games/*.toml` + `header.rs:148-178` + `pack.rs:71-118`. PS3 ids mostly TODO (only Xbox dumps provided) except Legacy/NHL15.

| Game | Xbox title ID | PS3 id | Container | version@0x14 | Writeable |
|---|---|---|---|---|---|
| NHL 10 | `454108BA` | — | RawV1 (32B) | 1 | **No** |
| NHL 11 | `454108FD` | — | RawV1 (32B) | 1 | **No** |
| NHL 12 | `45410964` | — | RawV2 (44B) | 2 | Yes |
| NHL 13 | `4541099B` | — | RawV2 (44B) | 0 | Yes |
| NHL 14 | `454109BE` | — | ZlibV4 (48B) | 2 (body-disambiguated) | Yes |
| NHL 15 | `454109DC` | `BLUS31436` | ZlibV4 (48B) | 2 | Yes |
| NHL Legacy | `454109EC` | `BLUS31540` | ZlibV4 (48B) | 4 | Yes |

Writeability rule (`header.rs:175-177`): writeable iff `ZlibV4 | RawV2`. RawV1 is refused (`PackError::UnsupportedV1Container`) because `tag_1c@0x1C` is un-reversed. RawV2 and ZlibV4 writers reproduce real saves byte-for-byte (gold tests `pack.rs:459-471`).

### Worked example (NHL14 disambiguation, `header.rs:769-785`)

version=2 + zlib magic at `0x30` + `uncompressed_size@0x24 = 2,215,112` + `compressed_size@0x2C = 677,745` (LE) → parses as ZlibV4, not RawV2.

---

## 5. PS3 PFD / AES

**Not implemented — explicitly out of scope.** The crate operates only on **already-decrypted** SYS-DATA bytes (RPCS3-extracted / plain):
- `tdb-savedata/lib.rs:32-37`: *"No PFD decryption. Real-PS3 saves are sealed by `PARAM.PFD`; this crate assumes RPCS3-extracted (already plain) SYS-DATA bytes. Re-encryption for hardware playback is out of scope."*
- `lib.rs:38-40`: *"No PARAM.SFO awareness."*
- Error path: `BadMagic` says *"…or it has been re-encrypted by PARAM.PFD — extract via RPCS3 first"* (`error.rs:12-17`).

There is **no PFD layout, no key handling, and no AES** anywhere in the save pipeline. (The only AES in the repo is `tdb-xex` — Xbox360 **executable** XEX2 decryption — an unrelated concern.) The exported-vs-raw distinction is **implicit via magic at offset 0**: if `PS3RosterFile`/`RosterFile` parses, it's plaintext; if not, `Platform::detect → None → BadMagic` tells the user to extract via RPCS3 first (`header.rs:215-229,288-306`).

> Rebuild note: if you want to read raw (PFD-sealed) PS3 saves directly, that is **net-new** work — PFD table parse + per-file AES-CBC + the console's secure-file-id keying — none of which exists here. The current pipeline assumes the emulator already did it.

---

## 6. Confirmed value→label mappings

### 6.1 Captaincy — `eGlu.mCcl` (`defs/enums/roster-captaincy.toml:49-59`)

2-bit; from EXE `TNHLRosterPlayerCaptaincy`. (Raw 3 reachable, unlabelled.)

| raw | label |
|---|---|
| 0 | None |
| 1 | Captain |
| 2 | Alternate |

### 6.2 Player status — `ZBac.RREB` — **no confirmed table**

Deliberately held raw-only (`tdb-view-models/roster_model.rs:26-28,102-106`). Live data has **4 distinct raw values (0,1,2,3)** but the EXE `TNHLRosterPlayerStatus` enum only has 3 labels, so the mapping was **not promoted** — the field renders as a raw integer. Pro/Minors-style labels are **not present in the repo**; they exist only in the NHLViewNG EXE.

### 6.3 Player style / archetype — `QgFs`

`eGlu.QgFs` is a per-slot copy of the skater (`dSvy.QgFs`) or goalie (`mHuy.QgFs`) style, written so the line editor can read it.

**Skater** (`defs/enums/player-style.toml:29-67`): D=1..4, F=5..10.

| raw | label | raw | label |
|---|---|---|---|
| 1 | Defensive Defenseman | 6 | Playmaker |
| 2 | Offensive Defenseman | 7 | Sniper |
| 3 | Enforcer Defenseman | 8 | Power Forward |
| 4 | Two-Way Defenseman | 9 | Two-Way Forward |
| 5 | Grinder | 10 | Enforcer Forward |

**Goalie** (`defs/enums/goalie-style.toml:36-45`, 2-bit; raw 3 unlabelled): `0=Standup, 1=Hybrid, 2=Butterfly`.

### 6.4 Nationality — `ubPc.Lelh` (`defs/enums/nationality.toml:44-175`, 7-bit; 127 = "-- N/A --")

| raw | label | raw | label | raw | label |
|---|---|---|---|---|---|
| 0 | Canada | 76 | Russia | 89 | Netherlands |
| 14 | United States | 77 | Ukraine | 91 | Poland |
| 66 | Denmark | 78 | Austria | 92 | Romania |
| 67 | Finland | 81 | Croatia | 93 | Scotland |
| 68 | Norway | 82 | Czech Republic | 95 | Slovakia |
| 69 | Sweden | 83 | England | 96 | Slovenia |
| 70 | Belarus | 84 | France | 97 | Switzerland |
| 71 | Estonia | 85 | Germany | 98 | Australia |
| 72 | Kazakhstan | 86 | Hungary | 106 | Fiji |
| 74 | Latvia | 87 | Italy | 109 | Japan |
| 75 | Lithuania | | | 110 | Liberia |
| | | | | 127 | -- N/A -- |

### 6.5 Birth state/province — `ubPc.QvxI` (`defs/enums/birth-state.toml:79-486`, 7-bit; 127 = "-- N/A --")

Canada 1–13 (alphabetical), USA+DC 15–65 (alphabetical, DC between Delaware and Florida), 0=Canada / 14=USA bare, 66+ non-NA fallbacks (same codes as nationality where they overlap).

**Canada (1–13):** 1 Alberta, 2 British Columbia, 3 Manitoba, 4 New Brunswick, 5 Newfoundland and Labrador, 6 Northwest Territories, 7 Nova Scotia, 8 Nunavut, 9 Ontario, 10 Prince Edward Island, 11 Quebec, 12 Saskatchewan, 13 Yukon (all prefixed "Canada, ").

**USA (15–65):** 15 Alabama, 16 Alaska, 17 Arizona, 18 Arkansas, 19 California, 20 Colorado, 21 Connecticut, 22 Delaware, 23 District of Columbia, 24 Florida, 25 Georgia, 26 Hawaii, 27 Idaho, 28 Illinois, 29 Indiana, 30 Iowa, 31 Kansas, 32 Kentucky, 33 Louisiana, 34 Maine, 35 Maryland, 36 Massachusetts, 37 Michigan, 38 Minnesota, 39 Mississippi, 40 Missouri, 41 Montana, 42 Nebraska, 43 Nevada, 44 New Hampshire, 45 New Jersey, 46 New Mexico, 47 New York, 48 North Carolina, 49 North Dakota, 50 Ohio, 51 Oklahoma, 52 Oregon, 53 Pennsylvania, 54 Rhode Island, 55 South Carolina, 56 South Dakota, 57 Tennessee, 58 Texas, 59 Utah, 60 Vermont, 61 Virginia, 62 Washington, 63 West Virginia, 64 Wisconsin, 65 Wyoming (all prefixed "USA, ").

### 6.6 Skin color — `ubPc.KTDQ` (`defs/enums/skin-color.toml:37-78`, 4-bit)

| raw | label | raw | label |
|---|---|---|---|
| 1 | White 1 | 6 | Light Brown |
| 2 | White 2 | 9 | Dark Brown |
| 3 | Tanned White | 10 | Black |
| 4 | Pale White | 5/7/8 | Unused (#N) |

Raws 0, 11..15 unlabelled (render raw).

### 6.7 Related: position & eye color

- **Position** `ubPc.vjla` (`defs/enums/position.toml:30-48`): `0=C, 1=LW, 2=RW, 3=D, 4=G`.
- **Eye color** `ubPc.Zufj` (`defs/enums/eye-color.toml`, `docs/confirmed-enums.md:43`): `0=Blue, 1=Light Blue, 2=Brown, 3=Light Brown, 4=Hazel, 5=Green, 6=Yellow`.

### 6.8 The 71 eGlu line-slot fields → hockey position

Authoritative source: `crates/tdb-relations/src/roster_audit.rs:95-201`, `const EGLU_SLOTS: [RosterSlotSpec; 71]` (golden-ordering test pins this exact order). TS `linePresentation.ts:242-263 GROUP_ORDER` only orders the groups. Each row = `(group, group_index, field_id, position, category)`.

| # | group | idx | field | position | category |
|---|---|---|---|---|---|
| 0 | C | 0 | PKtc | C | RegularLine |
| 1 | C | 1 | EfuP | C | RegularLine |
| 2 | C | 2 | NELC | C | RegularLine |
| 3 | C | 3 | CRxl | C | RegularLine |
| 4 | LW | 0 | XvAa | LW | RegularLine |
| 5 | LW | 1 | ZhGP | LW | RegularLine |
| 6 | LW | 2 | YTkI | LW | RegularLine |
| 7 | LW | 3 | VUOp | LW | RegularLine |
| 8 | RW | 0 | vifp | RW | RegularLine |
| 9 | RW | 1 | xsdA | RW | RegularLine |
| 10 | RW | 2 | wGbT | RW | RegularLine |
| 11 | RW | 3 | Bhxm | RW | RegularLine |
| 12 | LD | 0 | rNpo | LD | RegularLine |
| 13 | LD | 1 | tDvB | LD | RegularLine |
| 14 | LD | 2 | spxS | LD | RegularLine |
| 15 | RD | 0 | bASb | RD | RegularLine |
| 16 | RD | 1 | dOQO | RD | RegularLine |
| 17 | RD | 2 | ccmJ | RD | RegularLine |
| 18 | G | 0 | fGPW | G | RegularLine |
| 19 | G | 1 | yzmI | G | RegularLine |
| 20 | PP5-1 | 0 | BHsO | PP 5-on-4 unit 1 | SpecialTeams |
| 21 | PP5-1 | 1 | tfXi | PP5-1 | SpecialTeams |
| 22 | PP5-1 | 2 | RqIt | PP5-1 | SpecialTeams |
| 23 | PP5-1 | 3 | NFKs | PP5-1 | SpecialTeams |
| 24 | PP5-1 | 4 | xQxj | PP5-1 | SpecialTeams |
| 25 | PP5-2 | 0 | WOnB | PP5-2 | SpecialTeams |
| 26 | PP5-2 | 1 | rxlP | PP5-2 | SpecialTeams |
| 27 | PP5-2 | 2 | PkyM | PP5-2 | SpecialTeams |
| 28 | PP5-2 | 3 | LLYN | PP5-2 | SpecialTeams |
| 29 | PP5-2 | 4 | vynO | PP5-2 | SpecialTeams |
| 30 | PP4-1 | 0 | WkPp | C (4-on-3 PP) | SpecialTeams |
| 31 | PP4-1 | 1 | bkMF | LW | SpecialTeams |
| 32 | PP4-1 | 2 | zWBX | LD | SpecialTeams |
| 33 | PP4-1 | 3 | XLGE | RD | SpecialTeams |
| 34 | PP4-2 | 0 | BNoy | C (4-on-3 PP) | SpecialTeams |
| 35 | PP4-2 | 1 | dauA | LW | SpecialTeams |
| 36 | PP4-2 | 2 | BIja | LD | SpecialTeams |
| 37 | PP4-2 | 3 | ZVcB | RD | SpecialTeams |
| 38 | PK4-1 | 0 | NgRa | C (4-on-5 PK) | SpecialTeams |
| 39 | PK4-1 | 1 | KVWB | LW | SpecialTeams |
| 40 | PK4-1 | 2 | injb | LD | SpecialTeams |
| 41 | PK4-1 | 3 | iaAA | RD | SpecialTeams |
| 42 | PK4-2 | 0 | mxmt | C (4-on-5 PK) | SpecialTeams |
| 43 | PK4-2 | 1 | MLME | LW | SpecialTeams |
| 44 | PK4-2 | 2 | kZZW | LD | SpecialTeams |
| 45 | PK4-2 | 3 | kkGF | RD | SpecialTeams |
| 46 | pk3-1 | 0 | EDNb | PK 3-skater unit 1 | SpecialTeams |
| 47 | pk3-1 | 1 | MTrr | pk3-1 | SpecialTeams |
| 48 | pk3-1 | 2 | wGQk | pk3-1 | SpecialTeams |
| 49 | PK3-2 | 0 | PKOo | PK3-2 | SpecialTeams |
| 50 | PK3-2 | 1 | KFtG | PK3-2 | SpecialTeams |
| 51 | PK3-2 | 2 | uQSV | PK3-2 | SpecialTeams |
| 52 | SO | 0 | RMUX | Shootout | SpecialTeams |
| 53 | SO | 1 | UVxp | Shootout | SpecialTeams |
| 54 | SO | 2 | vOBv | Shootout | SpecialTeams |
| 55 | SO | 3 | JykA | Shootout | SpecialTeams |
| 56 | SO | 4 | SxAu | Shootout | SpecialTeams |
| 57 | X | 0 | ycNs | Extra Attacker (6-on-5) | SpecialTeams |
| 58 | X | 1 | zlku | Extra Attacker | SpecialTeams |
| 59 | OT1 | 0 | rGnk | C (4-on-4 OT) | SpecialTeams |
| 60 | OT1 | 1 | ueNR | LW | SpecialTeams |
| 61 | OT1 | 2 | OcAz | LD | SpecialTeams |
| 62 | OT1 | 3 | qRHQ | RD | SpecialTeams |
| 63 | OT2 | 0 | gnQz | C | SpecialTeams |
| 64 | OT2 | 1 | wQvg | LW | SpecialTeams |
| 65 | OT2 | 2 | QSii | LD | SpecialTeams |
| 66 | OT2 | 3 | sfdh | RD | SpecialTeams |
| 67 | OT3 | 0 | pwhK | C | SpecialTeams |
| 68 | OT3 | 1 | xCxX | LW | SpecialTeams |
| 69 | OT3 | 2 | REIF | LD | SpecialTeams |
| 70 | OT3 | 3 | ttbW | RD | SpecialTeams |

Source notes: PP4/PK4 4-skater units are C/LW/LD/RD; `pk3-1` keeps its lowercase prefix verbatim from the NHLViewNG XML; OT1/OT2/OT3 are pre-2015 4-on-4 overtime units. Related per-slot fields the line editor also reads: `eGlu.toWX` (unique id = `team_vfeq*40 + slot_index`), `eGlu.QgFs` (archetype, §6.3), `eGlu.sVRt` (jersey).

---

## Quick constant reference

| Constant | Value | Cite |
|---|---|---|
| `HEADER_SIZE` | `0x18` | tdb-core/header.rs:7 |
| `MAGIC_DB` | `0x4442` ("DB") | tdb-core/header.rs:10 |
| `DIRECTORY_ENTRY_SIZE` | `8` | tdb-core/file.rs:9 |
| `TABLE_HEADER_SIZE` | `0x28` | tdb-core/table.rs:10 |
| `FIELD_DESCRIPTOR_SIZE` | `0x10` | tdb-core/table.rs:13 |
| endian byte | 0=LE / 1=BE | tdb-core/endian.rs:12-18 |
| inner-DB CRC | CRC-32/MPEG-2, poly `0x04C11DB7`, init `0xFFFFFFFF`, xorout 0 | tdb-core/crc.rs |
| container `checksum_a` | CRC-32/IEEE, poly `0xEDB88320`, refl, xorout `0xFFFFFFFF`, covers `0x1C..` | tdb-savedata/checksum.rs |
| container `checksum_b` | CRC-32/BZIP2, poly `0x04C11DB7`, xorout `0xFFFFFFFF`, covers `0x2C..` | tdb-savedata/checksum.rs |
| v1/v2/v4 header len | `0x20` / `0x2C` / `0x30` | tdb-savedata/header.rs |
| save zlib stream | `file[0x30..]`, RFC-1950 | tdb-savedata/extract.rs:78 |
