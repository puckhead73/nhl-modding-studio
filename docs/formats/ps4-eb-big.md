# PS4 EA "EB\0\3" archive (`*.big`)

The PS4 install container for the "Kettle" asset stack (NHL 15 → 21). Every PS4
`*.big` — `game_*.big`, `inxp_*.big`, `boot.big`, `overlays_*.big` — is an
`EB\0\3` archive. This is the PS4 counterpart of the Xbox/PS3 `BIGF`/`BIG4`
container (see [`crates/tdb-archive-big`](../../crates/tdb-archive-big)); the
payload codecs (`chunkzip` / `chunklzx`) are shared, only the directory format
differs.

Status: **decode verified** against a real NHL install (CUSA18182), across the
full archive spread — `overlays_20`..`overlays_24` (6–257 files), `boot.big`
(7 365 files / 1 533 dirs), `inxp_00.big` (2 950 files / 152 dirs),
`game_60.big` (843 MB, player portraits), `game_70.big` (3 GB movies) and
`game_90.big` (**17 GB**, 92 322 stadium files). For every archive the declared
size equals the file length, every reconstructed path resolves to the real
extracted file, every raw payload byte-matches, and every `chunkzip` payload
decompresses byte-identically. All multi-byte integers are **big-endian**
(unlike the little-endian on-disk *data*), except where noted.

## Header (32 bytes)

| Off  | Width | Field                | Notes |
| ---- | ----- | -------------------- | ----- |
| 0x00 | 4     | magic `EB\0\3`       | `45 42 00 03` |
| 0x04 | 4 BE  | `file_count`         | number of file-table entries |
| 0x08 | 4 BE  | version tag          | constant `0x00110400` on every observed archive — used as a sanity gate |
| 0x0C | 4 BE  | `name_table_offset`  | byte offset of the fixed-width name table |
| 0x10 | 4 BE  | (reserved)           | points inside the name region; **not needed** to extract, meaning unresolved |
| 0x14 | 1     | `name_slot`          | width of one name-table record, bytes |
| 0x15 | 1     | `dir_slot`           | width of one dir-path record, bytes |
| 0x16 | 2 BE  | `dir_count`          | number of directory-path records |
| 0x18 | 8 BE  | `archive_size`       | total file length — **u64** (a 17 GB archive overflows u32; the high word at 0x18 is non-zero there) |

The file table begins at a fixed **0x30**.

## File table — `file_count` × 20 bytes, from 0x30

Each entry, all fields big-endian:

| Off  | Width | Field        | Notes |
| ---- | ----- | ------------ | ----- |
| +0x00 | 4 | `block`      | data offset in **16-byte units** — real offset = `(block as u64) << 4` |
| +0x04 | 4 | (zero)       | always 0 in samples |
| +0x08 | 4 | `stored_size`| bytes on disk (compressed size for `chunkzip` payloads, exact size for raw) |
| +0x0C | 4 | hash A       | content/name hash — not needed for extraction |
| +0x10 | 4 | hash B       | content/name hash — not needed for extraction |

Entry `i` corresponds 1:1 (same order) to name record `i`. Payload bytes are
`[offset, offset + stored_size)`; data blocks are 16-byte aligned but
`stored_size` is exact (no trailing pad included).

**Deduplication.** Identical payloads are stored once and referenced by multiple
entries — e.g. two widgets' byte-identical `_i1.gnf` progressbar textures share
one `block`. Extraction writes each logical entry to its own path regardless.

## Name table — `file_count` × `name_slot` bytes, from `name_table_offset`

Fixed-width records, each:

- `+0x00` **u16 BE** `dir_index` — index into the directory-path table
- `+0x02` NUL-terminated ASCII **basename**, right-padded with zeros to fill the slot

`name_slot` is sized to the longest basename in the archive
(`max(len(basename)) + 1 NUL + 2 prefix`), so it varies per archive (31, 36, 61,
70, 86 … observed) and is read from header `0x14`, never assumed.

## Directory-path table — `dir_count` × `dir_slot` bytes

Located at `align_up(name_table_offset + file_count * name_slot, 16)` (the name
table is followed by ≤15 bytes of zero padding to the next 16-byte boundary).
Each record is a NUL-terminated ASCII path (no `dir_index` prefix), zero-padded
to `dir_slot` (= longest dir path `+ 1 NUL`, header `0x15`). Paths use forward
slashes, e.g. `nhl/data/common/gamemodes/database/t3db`.

**Full path** of entry `i` = `dirs[names[i].dir_index] + "/" + names[i].basename`.

## Payloads — shared with BIGF

A stored payload is one of:

- **`chunkzip`** (magic `chunkzip` at +0) — chunked zlib, decoded by
  `tdb_archive_codec::decompress_chunkzip` (both header variants). PS4 `.gnf`
  textures use the "padded" variant and decode byte-identically.
- **`chunklzx`** (magic `chunklzx`) — chunked LZX, **not implemented**; skipped.
- **raw** — anything else (e.g. `.gfx` Scaleform, which itself begins with
  `CFX%`; `.mkv` movies). Written verbatim.

This is exactly the dispatch `Archive::read_entry_at` already performs, so the
codec layer is unchanged — only the directory parse is PS4-specific.

## Memory note

`game_90.big` is 17 GB. The extractor must **stream** (parse the front tables,
then seek+read each entry) rather than loading the whole archive into memory the
way the small BIGF path can. See `studio-domain`'s `extract` module.

## Where the modding minimum lives

The databases onboarding needs are **not** in the huge `game_*` archives — on
CUSA18182 `nhl_game.db`, `initial_experience.db`, the `entry_draft`/`expansion_draft`
rosters and the `*_us.db` localization set all live under
`nhl/data/common/gamemodes/…` and `nhl/data/kettle/loc/…` inside `inxp_00.big`
(777 MB). A future optimization could extract only the DB-bearing archives for a
fast onboard; today extraction mirrors the Xbox behaviour (extract every `.big`).

## Related

- Reader: `crates/tdb-archive-big/src/eb.rs`
- Shared codecs: `crates/tdb-archive-codec` (`chunkzip` / `chunklzx`)
- Extraction: `crates/studio-domain/src/extract.rs`
- Xbox/PS3 sibling container: `crates/tdb-archive-big/src/lib.rs` (`BIGF`/`BIG4`)
