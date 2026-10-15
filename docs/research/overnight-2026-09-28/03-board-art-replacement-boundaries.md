# Handoff 03 (28 Sep) — gen7 board-art replacement boundaries: a same-size, same-format operation contract for venue-package rasters, and what stays open

Research lane, 28 September 2026, published 06:02 MDT (system clock). Prompt package 4. Claim `research-q4-board-art-writer-boundaries`.
The developer owns the read-only board-art gallery (dispatch after receipt 45 §4); research owns the writer inputs. This extends 27 Sep
handoff 20 §6–§9 (R127c / f / g), as corrected in [00](00-correction-checkpoint.md) K3 / K4.

**Read-only** on the installed trees, with two exceptions:
- one mutation round trip each on disposable scratch copies of X360 and PS3 Legacy `stadium_1_container_0` (copies deleted after
  containment and reparse checks);
- nothing written to any installed file, and nothing launched.

Machine-readable: [`data/r137-venue-writer-boundaries.json`](data/r137-venue-writer-boundaries.json). This is merged from the full X360 run
and a corrected PS3 rerun (PS3 data entries are `0x10030`, not `0x10031`; the first PS3 pass used the wrong type and is not published).
Probes: `r137_venue_package_writer_boundaries.py` (`9c6f6381…`), `r137b_venue_roundtrip_copy.py` (`b340ff97…`).

## Plain-language summary

Each arena's board ads are pictures inside that arena's venue package. Research can now say exactly which bytes a same-size, same-format
replacement of one board-ad picture touches, on both consoles:
- The packages have no overlapping or shared picture data.
- No checksum was found where one was looked for.
- The Xbox and PS3 copies of the same picture hold the same image bytes, one tiled and one plain.
- A test replacement on a throw-away copy changed only the planned bytes and restored exactly.

Every venue package also exists inside the game's render archive as an identical copy, so a writer has two copies to keep consistent *(qualified 28 Sep 13:21, [09](09-after-receipt5-corrections.md) K10: for the measured loose / archive pairs; PS3 NHL 11-13 and Recompiled are archive-only, X360 NHL 10 is unmeasured)*. It
is still unknown which copy the console loads and how the game shows the result. What is left open:
- smaller picture sizes (mip levels) on Xbox, which Studio already handles conservatively for other pictures;
- the archive copy;
- the in-game result.

## 1. Measured families (every venue package `stadium_<artid>_container_<n>`)

| Tree | Packages | Descriptors | DW8 / PS3 index → a data entry | Shared data entries | Overlapping spans | `hdr_size + data_size` = file length | Board-ad rasters (descriptors) · with several materials |
|---|---|---|---|---|---|---|---|
| X360 NHL 10 | 44 | 1 704 | 1 704 | 0 | 0 | 44 / 44 | 105 · 16 |
| X360 NHL 13 | 61 | 2 912 | 2 912 | 0 | 0 | 61 / 61 | 132 · 84 |
| X360 NHL 15 | 64 | 3 033 | 3 033 | 0 | 0 | 64 / 64 | 139 · 88 |
| X360 Legacy | 64 | 3 035 | 3 035 | 0 | 0 | 64 / 64 | 139 · 88 |
| PS3 NHL 15 | 64 | 3 033 | 3 033 (descriptor u32 @ +0) | 0 | 0 | 64 / 64 (`data_size` @ +0x4C) | 139 · 88 |
| PS3 Legacy | 64 | 3 035 | 3 035 (descriptor u32 @ +0) | 0 | 0 | 64 / 64 | 139 · 88 |

"Several materials" means one raster serves two or more board-ad materials (e.g. `BoardAd_plastic_mat1_Clone_237` and `…_96` share
one). Replacing that raster changes every material that points at it, and the operation must list them.

**X360 descriptor facts:**
- On every board-ad raster, `DW0` = `0x00400003` (the NHL 09–Legacy form Studio's texlib measures).
- Format DXT1 (0x52), except one DXT3 (512 × 128) per title from NHL 13.
- Sizes 512 × 512 (9–10 levels), 512 × 1024 and 1 024 × 256 (11), and a few 256 × 256 / 256 × 512 / 512 × 256 / 1 024 × 1 024.
- The data entry is at least the Studio-model tiled base on **every** board-ad raster.
- **The DW12 mip page = the tiled base rounded up to 4 KB on every board-ad raster** (and 97–100 % of all descriptors that declare a mip page). The mip chain
  starts after the base inside the same entry.

**PS3 descriptor facts:**
- Signatures `83f1` / `83f2` / `83f3`: 2 122 / 419 / 492 (NHL 15).
- Every descriptor's first u32 names its data entry, and every entry's size equals the **plain** chain size for DXT1 / DXT3 / DXT5
  (8 / 16 / 16 bytes per block, one-block floor per level), all levels concatenated base-first.
- `83f1` is **DXT1**: the X360 base level, untiled (XGAddress2DTiledOffset) and 16-bit byte-swapped, **equals the PS3 entry's base bytes**
  for every board-ad material tested:
  - `stadium_1`: `adbd_WPG`, `adbd_22`, `BoardAd_plastic_mat1_Clone_237` / `_96`;
  - `stadium_20`: `adbd_01`, `adbd_13`, `adbd_15_Clone_244`, `BoardAd_plastic_mat1_Clone_100` / `_240`.
- The PS3 data entry index equals the X360 DW8 index in every compared case.
- Studio's `texlib_ps3` knows only `6007` (ARGB8) and `83f3` (DXT5) and pairs slots with descriptors by order. It must use the
  descriptor index here: the venue packages carry 300+ non-raster data entries after the rasters.

## 2. Loose / archive copies (full, not sampled)

| Tree | Archive | Members | Wrapping | Unwrapped = loose file |
|---|---|---|---|---|
| X360 NHL 10 | `stadium.big` (BIG4) | 49 | RefPack | 49 / 49 |
| X360 NHL 13 | `cacherender.big` | 64 | chunklzx | 64 / 64 |
| X360 NHL 15 / Legacy | `cacherender.big` | 67 / 67 | chunklzx | 67 / 67 each |
| PS3 NHL 15 / Legacy | `cacherender.big` | 67 / 67 | chunkzip | 67 / 67 each |

**Copies.**
- Every loose venue package has exactly one archive twin with the same bytes. *(Corrected 28 Sep 13:21, [09](09-after-receipt5-corrections.md) K10: true for the measured pairs only - X360 NHL 11-15 / Legacy and PS3 NHL 14 / 15 / Legacy. PS3 NHL 11-13 are archive-only (§9). X360 NHL 10 archives are BIG4 and were **not compared**, so its topology is unmeasured.)*
- Legacy Recompiled ships the identical X360 Legacy `cacherender.big` and **no** loose copy (handoff 04).

**Precedence.** Byte identity does not establish which copy loads. A writer that changes only the loose file leaves the twin stale, and
vice versa. Re-wrapping is known only for chunkzip (Studio already writes it on PS3). X360 chunklzx re-wrapping is card R30-LZX-1, and
NHL 10 RefPack re-wrapping is not measured.

## 3. Integrity (bounded negative, K4 widened)

Searched:
- CRC-32 and Adler-32 of the whole file, of the data region, and of **every data entry**;
- FNV-1a-32 of the first 64 KB of data;
- against every u32 in 0 .. `hdr_size` (header, file table, descriptors, dictionaries, materials);
- 361 packages across six trees.

**Result:** no match, except one coincidence per X360 NHL 13 / 15 / Legacy package: an entry whose CRC-32 is `0xFFFFFFFF` equals a
`0xFFFFFFFF` sentinel word.

**Untested:**
- other algorithms;
- checksums stored outside 0 .. `hdr_size`;
- runtime validation.

Unknown header words are kept byte-for-byte.

## 4. Round trip on disposable copies (research scratch; not a Studio writer)

X360 Legacy `stadium_1_container_0.rx2` (SHA-256 `1a986bea…`) and PS3 Legacy `.rpsgl` (`171848df…`): `adbd_WPG` (512 × 1 024 DXT1, data
entry 43). Only its base level was replaced with valid solid DXT1 blocks.

| Check | X360 (mask `hdr + entry` … +262 144, tiled + 16-bit swap) | PS3 (mask … +262 144, plain LE) |
|---|---|---|
| bytes outside the mask unchanged | yes | yes |
| header / file table / descriptors / dictionaries / materials unchanged | yes | yes |
| every board-ad material → raster edge unchanged | yes | yes |
| read-back (untile + swap / plain) = the written blocks | yes | yes |
| restore by byte copy → SHA-256 = installed file | yes | yes |

This validates the container invariants of an in-place base-level write. It says nothing about how the game shows the result.

## 5. Operation contract (developer-reviewable; same size and same format only)

**Inputs, bound at plan time:**
1. **Source identity:** the package path and its SHA-256, plus the title / platform tree.
2. **Target raster:** the descriptor chunk index, reached from the material by the R127 slot rule (u32 @ +256 of the `0xEF0001` chunk
   before the `0xEF0005` material). The word must lie **inside** that block, which needs at least 260 bytes (corrected 06:03 after developer observation 49; 288- and 240-byte blocks exist on both platforms, handoff 20 §10). It must be a `0x20003` chunk, and must be re-resolved and equal at apply.
3. **Data entry:** X360 DW8 (`u32 @ +32 >> 12`); PS3 descriptor u32 @ +0. It must be a data entry (`0x10031` X360 / `0x10030` PS3) whose
   span lies inside the data region and overlaps no other entry.
4. **Format and size:** from the descriptor. The replacement image must have identical dimensions and format:
   - X360 DXT1 / DXT3 / DXT5 (0x52 / 0x53 / 0x54);
   - PS3 `83f1` / `83f2` / `83f3` (= DXT1 / DXT3 / DXT5 plain);
   - anything else is refused.

**Permitted bytes** (the planned byte mask):
- **X360:**
  - the tiled base level at `hdr_size` + entry offset, length = align(w/4, 32) × align(h/4, 32) × bytes-per-block, 16-bit-swapped as
    Studio's texlib writes it;
  - the **large** mip levels (above the 16 × 16-block packed tail) at the sequential offsets from the DW12 mip page, **only** for rasters
    inside Studio's audited `LargeLevels` gate (NHL 10: 105 / 105; NHL 13: 131 / 132; NHL 15 / Legacy: 138 / 139 board-ad rasters; the one
    512 × 128 DXT3 falls outside and is base-only);
  - the packed tail is **kept**, so the operation must say its smallest levels still show the old art.
- **PS3:** base level plus every mip level, plain and concatenated, lengths as §1. Levels are regenerated from the replacement; the PS3
  plain layout has no packed-tail question.

Nothing else changes: no header, file-table, descriptor, dictionary, material or size word.

**Sharing.** List every material whose slot names the raster. Refuse when the data entry is referenced by another descriptor (none
measured, but keep the check).

**Copies:**
- Plan both the loose file and its archive twin, or refuse.
- The archive twin may be written only through the existing archive-member stager: chunkzip (PS3) now; chunklzx after R30-LZX-1; NHL 10
  RefPack not designed.
- ~~The loose-only write is allowed only if the operation states that the twin stays stale and precedence is unknown.~~ *(Withdrawn 28 Sep 13:21, [09](09-after-receipt5-corrections.md) K11: it contradicted the line above. A measured pair shares one review / recovery scope or refuses; archive-only sources use the member route; see [handoff 10](10-ps3-board-raster-writer-contract.md) §2.)*

**Stale-source and restore:**
- At apply, re-hash the package and compare it with the plan; any difference refuses.
- Back up the exact original bytes. Restore = byte copy, verified by SHA-256 against the recorded hash.

**Refusals:**
- size or format change;
- non-board raster classes not in scope;
- descriptors outside the measured forms;
- a data entry shared or out of range;
- Recompiled (no loose tree; archive route only);
- any title outside the measured trees (see §8: X360 NHL 10–15 / Legacy, PS3 NHL 14 / 15 / Legacy).

## 6. Still open (smallest next action)

| Open item | Next action |
|---|---|
| Which copy the console loads | Runtime card: change a board ad in the loose file only, then in the archive only, on copies |
| chunklzx re-wrap acceptance | Card R30-LZX-1 |
| X360 packed-tail layout | Studio's existing keep-tail rule; an independent contract is still missing |
| In-game appearance, and whether the game rejects altered art | Card |
| PS3 NHL 11–13 and NHL 09 and earlier venue families | Archive-member variant of the probe (X360 11 / 12 / 14 and PS3 14 done, §8) |

## 7. Reproduce

```
python -B local/runs/overnight-research-2026-09-28/probes/r137_venue_package_writer_boundaries.py <out.json> [--trees …]   (≈ 45 min, LZX-bound)
python -B local/runs/overnight-research-2026-09-28/probes/r137b_venue_roundtrip_copy.py <out.json> <scratch dir> x360|ps3 <package> <material> [<ps3 entry>]
```

## 8. Addendum 06:34 — X360 NHL 11 / 12 / 14 and PS3 NHL 14 ([`data/r137-ext-venue-writer-boundaries.json`](data/r137-ext-venue-writer-boundaries.json))

The same probe (`--trees x360-nhl11,x360-nhl12,x360-nhl14,ps3-nhl14`) gives the same results on every measure:

| Tree | Packages | Descriptors → one data entry | Shared / overlapping | `hdr + data = len` | Board-ad rasters · several materials | Studio `LargeLevels` gate | Loose ↔ archive twins |
|---|---|---|---|---|---|---|---|
| X360 NHL 11 | 51 | 2 060 / 2 060 | 0 / 0 | 51 / 51 | 112 · 21 | 109 / 112 (3 × 128 × 128 base only) | `stadium.big` chunklzx 53 / 53 |
| X360 NHL 12 | 57 | 2 567 / 2 567 | 0 / 0 | 57 / 57 | 127 · 83 | 126 / 127 | `cacherender.big` chunklzx 59 / 59 |
| X360 NHL 14 | 64 | 3 033 / 3 033 | 0 / 0 | 64 / 64 | 139 · 88 | 138 / 139 | `cacherender.big` chunklzx 67 / 67 |
| PS3 NHL 14 | 64 | 3 033 / 3 033 (descriptor u32 @ +0) | 0 / 0 | 64 / 64 | 139 · 88 | (plain chain) | `cacherender.big` chunkzip 67 / 67 |

Further results:
- Every X360 board-ad raster has `DW0 0x00400003`, an entry at least as large as the tiled base, and DW12 mip page = the 4 KB-aligned tiled
  base.
- The NHL 11 archive is `stadium.big` (an EB archive, chunklzx), not `cacherender.big`.
- Integrity hits are the same `0xFFFFFFFF` coincidence (46 on NHL 12, 57 on NHL 14; none on NHL 11 / PS3 14).

**Coverage:** the §5 contract now covers X360 NHL 10–15 / Legacy and PS3 NHL 14 / 15 / Legacy. Still not measured: PS3 NHL 11–13
(PS3 NHL 11 has no loose `rendering/stadium`; 12 / 13 are archive-held) and NHL 09 and earlier.

## 9. Addendum 06:35 — PS3 NHL 11 / 12 / 13: archive-only venue packages ([`data/r137c-ps3-archived-venue-packages.json`](data/r137c-ps3-archived-venue-packages.json))

These titles ship their venue packages **only** as archive members:
- NHL 11: `stadium.big` (51);
- NHL 12: `cacherender.big` (57);
- NHL 13: `cacherender.big` (61);
- all chunkzip; there is no loose `rendering/stadium`.

Probe `r137c_ps3_archived_venue_packages.py` unwrapped each member into package scratch, ran the same analysis, and deleted the copy.

| Tree | Packages | Descriptors: u32 @ +0 names an entry of the plain-chain size | Shared / overlapping | `hdr + data = len` | Board-ad rasters · several materials | Integrity words found |
|---|---|---|---|---|---|---|
| PS3 NHL 11 | 51 | 2 049 / 2 060 | 0 / 0 | 51 / 51 | 112 · 21 | 0 |
| PS3 NHL 12 | 57 | 2 567 / 2 567 | 0 / 0 | 57 / 57 | 127 · 83 | 0 |
| PS3 NHL 13 | 61 | 2 912 / 2 912 | 0 / 0 | 61 / 61 | 132 · 84 | 0 |

**The 11 NHL 11 exceptions** (one per package in 11 packages):
- single-level 256 × 256 `83f1` rasters whose entry holds **6 ×** the face size (196 608 bytes), i.e. **cube maps**;
- the index rule still names them;
- one example serves `glass_arch_texturepage5` (`stadium_47`);
- none is a board ad;
- the §5 refusal of cubes and non-board classes covers them.

**Contract consequence:** PS3 NHL 11–13 board ads follow the same §5 rules, with **archive-member-only** source and write routes. The
archive twin is the only copy. A write there means the stager and chunkzip re-wrap Studio already has for PS3, plus the usual runtime
card. Not measured: NHL 09 and earlier.
