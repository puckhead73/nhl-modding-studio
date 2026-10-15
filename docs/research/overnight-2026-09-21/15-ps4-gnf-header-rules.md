# Handoff R31 — PS4 `GNF` texture headers as functions of the dimensions: the size, pitch and mip rules a resize writer needs, censused on every `.gnf` of the NHL 16 and NHL 21 extracted trees (216 000 files)

Research lane, 21 September 2026, Priority 1.3 ("GNF descriptors"; the PS4 half of the resize question that handoff 11 answers for gen7).
Claim `research-r31-ps4-gnf`. Read-only over the extracted PS4 trees on H:; nothing game-verified; no writer. Studio's `tdb-rx2::gnf`
reader (its T# field split) was read, not edited; the census tests its fields against the file sizes.

Machine-readable: [`data/r31-gnf-header-census.json`](data/r31-gnf-header-census.json) (per title: header checks, shape census, families,
size-rule failures with examples). Probe `r31_gnf_header_census.py` (NHL 16 ≈ 25 min, NHL 21 ≈ 40 min; `--titles`, `--max`).

> **Corrections R1 / R3 (coordinator review of 2026-09-21, applied 2026-09-21 17:00 MDT; campaign
> [overnight-2026-09-21-pm](../overnight-2026-09-21-pm/00-ATLAS.md)).** (1) *Coverage classes, kept separate* — from
> `data/r31-gnf-header-census.json`: NHL 16 — 68 849 `.gnf` walked, **68 083 readable** (766 not `GNF ` / short), of which
> **68 083 predicted-format** (no `0x28`), **68 060 pass** the size rule and **23 fail = cube maps**; NHL 21 — 149 613 walked, **148 066
> readable** (1 547 not `GNF ` / short), **145 225 predicted-format**, **145 136 pass**, **89 fail = cube maps**, **2 841 = format `0x28`
> (BC6H, unmodelled — not evaluated, not "passing")**. "All readable files pass the size rule" is **not** claimed anywhere; the pass counts
> are the predicted-format subset minus the cube maps. (2) *Titles* — this handoff censused **NHL 16 and NHL 21 only**. NHL 15 / 17 / 18 / 19
> **and NHL 20** were not censused here; nothing about NHL 20 is implied by NHL 21 or by the `anchors/ps4/nhl20` files (which are NHL 19
> base members — `docs/validation/league-parity-batch-2026-09-18.md`). The per-title census with checkpointable output is
> `local/runs/overnight-research-2026-09-21-pm/probes/r43_gnf_header_census_per_title.py` (one JSON per title, NHL 20 first); its results
> are published in the pm campaign's handoff 03 as each title completes. (3) *Pitch sample* — the "6 000-file random sample" behind the
> `roundup(w, 32) − 1` rule came from a deleted scratch script; r43 tests that rule on **every** readable file of each title it walks
> (no sample), with the failing files listed. (4) *Cube maps* — the §2 sentence "data = 6 × the prediction" is exact only for single-level
> cubes; the r43 smoke run on NHL 20 shows mipped cube maps (T# `type` 11) at **8 ×** the 2D prediction (e.g. `board_glass_cubemap`
> 512² BC3 10 levels: 2 834 432 B vs 354 304 B). Cubes stay refused; the ratio is recorded, not modelled. The text below is preserved as written.

## Plain-language summary

A PS4 texture is a 256-byte header followed by the GPU-ready pixel data, and the header's dimension-dependent words follow fixed rules on
every one of the 216 000 shipped files: the stream size is the file size, word 7 is the pixel-data byte count, the mip count is in the
"last level" field, the pitch is the width rounded up to 32 texels (minus one) for block-compressed formats, and — the important part
for a writer — the pixel-data size is exactly the sum over mip levels of each level padded to whole 8 × 8-block micro-tiles (tiling mode
13 on every file). Cube maps are 6 × that, and a handful of environment maps (BC6H, format `0x28`) use a block size the probe did not
model. So a PS4 resize is: recompute width / height / last level / pitch, re-emit the micro-tiled level chain at the new size, and set
`d7` and `streamSize` to the new byte counts — with cubes and `0x28` refused until measured. Nothing here is proven at runtime; a card is
listed for when the developer builds the writer.

## 1. Header facts (both titles; counts NHL 16 / NHL 21 of 68 083 / 148 066 readable `GNF ` files; 766 / 1 547 non-GNF or truncated)

| Field | Rule | Count |
|---|---|---|
| `contentsSize` | `0xf8` | all / all |
| `version`, `count`, `alignment` | 2, 1, 8 | all / all |
| `streamSize` | **= file size** | all / all |
| T# `dfmt` / `nfmt` | BC1 `0x23`, BC3 `0x25`, BC5 `0x27` (nfmt 1 = SNORM on BC5), BC7 `0x29`, BC6 `0x28` (unmodelled), `0x0a` 8_8_8_8 …; nfmt 9 = SRGB dominant | shape census in the JSON |
| T# `base_level` | 0 | all / all |
| T# `last_level` | mips − 1 (0 = single level: 15 999 / 60 714 files, mostly UI; 7 = the 128² player portraits 24 480 / 34 326) | all |
| T# `tiling` | **13** | all / all |
| T# `type` | 9 (2D) dominant; cube maps carry the cube type with 6 faces in the data | see §2 |
| T# `pitch` | **`roundup(width, 32) − 1`** for block formats (= `width − 1` when the width is a multiple of 32: 64 267 / 125 311 direct matches; every "failure" of the naive `w − 1` rule is a non-multiple width: 164 → 191, 200 → 223, 332 → 351, 88 → 95, 4 → 31; verified on a 6 000-file random sample: 5 828 / 5 828 block-format files); `width − 1` on the uncompressed / `0x28` files seen | all |
| `depth` | 0 (5 / 10 exceptions: volume textures) | 68 078 / 148 056 |
| `d0` | 0 | all |
| `d5`, `d6` | 0 (23 / 208 exceptions carry values — not decoded) | 68 060 / 147 858 |
| **`d7`** | **= pixel-data bytes (file size − 0x100)** | all / all |
| 16 bytes after the T# | zeros (64 809 / 146 916), `USER` tag (3 257 / 1 150), `04 00 00 00` (17 / 0) | census |

## 2. Data-size rule (tiling 13 = 1D-thin micro-tiled: each level padded to whole 8 × 8-**block** micro-tiles × block bytes)

`data bytes == Σ_levels ceil(⌈w_l / 4⌉ / 8) · ceil(⌈h_l / 4⌉ / 8) · 64 · block bytes` (BC1 / BC4 8 B, BC2 / BC3 / BC5 / BC7 16 B; uncompressed:
8 × 8 texels × bytes per texel): **68 060 / 68 083** on NHL 16 and **145 136 / 145 225** predicted on NHL 21. Every failure is a
**cube map** (data = 6 × the prediction: `board_glass_cubemap`, `character_common_env_cube_1`, `chl_cube_map` …, 23 / 89 files) and the
2 841 NHL 21 files of format `0x28` (BC6H, 16 B block presumed — not modelled, listed as `size_not_predicted_unknown_format`). No file
needs a further alignment pad; there is no 256-byte or page rounding beyond the micro-tile.

## 3. Consequence for the resize writer (proposal)

For a 2D block-format texture: new `w` / `h` (T# word 2), `last_level` = new mip count − 1, `pitch = roundup(w, 32) − 1`, data =
the micro-tiled chain re-emitted per §2 (Studio's `gnf` reader already untiles mode 13 — the inverse tiler is the developer's), `d7` =
new data bytes, `streamSize` = `0x100 + d7`; keep every other word (`d5` / `d6` / tail bytes copied). Refuse cube maps (type ≠ 2D or
data = 6 × prediction), `0x28`, depth ≠ 0, and any file whose shipped header fails §1 / §2 (a "descriptor not reproducible" gate that
also protects against a wrong field split). The PS4 registry's `size64 / 256 / 512` directory rule (2026-09-20 handoff 05) stays the
outer constraint: a new size that does not have a directory is not addressable.

## 4. Not claimed

The BC6H block size / its swizzle, the `d5` / `d6` values, the meaning of `USER`, volume textures, runtime acceptance of a changed size
(card R31-GNF-1), NHL 15 / 17 / 18 / 19 trees (not censused tonight; the same rules are expected and cheap to confirm with `--titles`).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r31_gnf_header_census.py docs/research/overnight-2026-09-21/data/r31-gnf-header-census.json --titles nhl16,nhl21   (≈ 65 min)
```
The pitch-rounding verification was a 6 000-file scratch sample (rule stated in §1; removed).
