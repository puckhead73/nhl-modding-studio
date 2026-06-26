# TDB v6 Binary Format (verified)

> **Verification status: VERIFIED** against the anchor `nhlng.db` files (nhl10–nhl15 + legacy, PS3).
> The table list and field list reproduce the `nhlng-meta.xml` schema with **100% agreement**, and
> per-field bit-depths match with **zero discrepancies** across ~17,000 fields (nhl11–legacy). nhl10
> has no meta and is validated structurally (filesize + directory consistency). Reverse-engineered
> from the anchors with read-only tooling; cross-referenced to the NHLViewNG EXE
> (`TtdbDatabaseHeader/MetaTable/MetaField`).

Applies to the EA NHL **`*.db`** databases (`nhlng.db`, `nhl_eng_us.db`, `draft_roster.db`,
`dynamic_ratings.db`). **Big-endian** (PS3/PowerPC). Internal `version = 6`.

## The id-reversal rule (read first)

Two conventions exist for the 4-character table/field ids:

- **Raw** order — used by **the binary itself** and by **`nhlng-meta.xml`** (e.g. table `cPbu`,
  field `BSXd`).
- **"Real" / reversed** order — the same 4 chars reversed — used by the **EA DB Editor XML**, the
  **staging TSV audits**, the **functional report**, and **EDIT-MODEL.md** (e.g. `ubPc`, `dXSB`).

Convert by reversing the 4 characters: `cPbu` ⇄ `ubPc`. The binary and the per-game meta XML match
**without** reversal; reverse only to reach the human/"real id" convention.

## File header (24 bytes)

| Offset | Size | Endian | Field | Notes |
|---:|---:|---|---|---|
| `0x00` | 2 | – | magic | ASCII `"DB"` (`44 42`) |
| `0x02` | 6 | – | format bytes | constant `00 08 01 00 00 00` (version/format marker) |
| `0x08` | 4 | BE | **file length** | equals the file size (verified every DB) |
| `0x0C` | 4 | BE | reserved | `0` |
| `0x10` | 4 | BE | **table count** | matches meta XML table count exactly |
| `0x14` | 4 | – | head hash | integrity value (CRC-family; algorithm not yet reversed) |
| `0x18` | — | — | table directory | begins here |

## Table directory

`table_count` entries × **8 bytes**: `tag(4 raw chars) + u32 BE block_offset`. Offsets are relative to
the end of the directory:

```
data_base   = 0x18 + 8 * table_count
block_abs   = data_base + block_offset      # absolute file offset of the table's block
block_size  = next_block_offset - block_offset   (last table runs to EOF)
```

Offsets are monotonically increasing.

## Per-table block

A **0x30-byte block header** followed by the field-descriptor array, then the packed record region.

| Offset (in block) | Size | Endian | Field | Notes |
|---:|---:|---|---|---|
| `0x00` | 4 | – | table hash | CRC-family |
| `0x04` | 4 | BE | const | `2` |
| `0x08` | 4 | BE | (record/layout) | varies; not the field count |
| `0x1C` | 4 | **LE** | **field count** | the reliable count — matches meta XML 100% |
| `0x30` | — | — | field descriptors | `field_count` × 16 bytes |

> The field count at `+0x1C` is stored **little-endian** (`count, 00, 00, 00`) even though the rest of
> the format is big-endian. This is the authoritative count.

### Field descriptor (16 bytes)

| Offset | Size | Endian | Field | Notes |
|---:|---:|---|---|---|
| `+0x00` | 4 | – | field id | raw 4 chars (reverse for "real id") |
| `+0x04` | 4 | BE | **bit-depth** | storage width; matches meta `depth` exactly |
| `+0x08` | 8 | – | record-layout | type/bit-offset packing — **not** a clean type code; **to refine** |

Descriptors are stored sorted ascending by raw id. The **field type, legal range, key, update flag,
FK parent, and human name come from the meta XML**, not the descriptor (the descriptor's last 8 bytes
do not decode to a stable type; the last descriptor's trailing bytes overlap the record region).

## Integrity

- File length (`0x08`) is verified for every database.
- The head hash (`0x14`) and per-table hash (`+0x00`) are CRC-family values; the exact algorithm is
  **not yet reversed** (read verification only this pass — write/recompute is deferred to the engine).

## Validation

Independently validated against `nhlng-meta.xml` for nhl11/12/13/14/15/legacy:

- **table set**: identical, 100% (131/131 … 134/134).
- **per-table field set**: identical, 100% every table.
- **per-field bit-depth**: 0 mismatches across all ~17,000 fields.

The verified per-game schema corpus is `../audits/data/schema/<game>-schema.json`; the cross-game
drift is `../audits/data/drift-matrix.csv` (see [../audits/nhlng-schema.md](../audits/nhlng-schema.md)).

## Open items

- Decode the descriptor's trailing 8 bytes (record bit-offset / type encoding).
- Reverse the head/table CRC algorithm (needed for the write path, not for reads).
- Localization `nhl_eng_us.db` uses large varchar/string fields (depth in bytes, up to 32000) and a
  Huffman string pool — structure noted; full pool handling deferred.
