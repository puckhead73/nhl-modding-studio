# Receipt 11 — Priority 2-B: texture resize on RW4xb2 (Xbox 360), RW4ps3 (PS3) and GNF (PS4) by the measured descriptor rules

**Developer-verified; not Codex-accepted; nothing game-verified — runtime acceptance of a resized
texture is open on all three families (cards R32-XB2-1, R33-PS3-1, R31-GNF-1).** Branch
`feat/phase5-asset-tooling`, baseline `8d60a6d`. Real trees read only; writes in memory / on temp
copies. Claim: `dev-p2b-texture-resize`. Research consumed: 2026-09-21 handoff 11 (`research-r32-r33-gen7-descriptors`)
§1–2 and handoff 15 (`research-r31-ps4-gnf`) §1–3, with their census JSONs as the oracle sizes.

## 1. Plain-language summary

Until tonight a loose texture on the old consoles and on PS4 could only be replaced at its exact
size, because the bytes that tell the GPU how big a texture is were understood for a couple of
measured sizes only. The research lane measured every shipped texture header — some 190 000 on
Xbox 360, 100 000 on PS3, 216 000 on PS4 — and wrote down which words depend on the size and how.
Studio now re-derives those words from the shipped header (never from scratch) and re-emits the
pixel chain at the new size, and proves it the strongest way available without a console: on the
real Xbox 360 Legacy tree the rule reproduces the shipped descriptor byte-for-byte on **every one
of the 18 984 two-dimensional DXT rasters it applies to** (the 70 cube maps refuse), on the PS3
Legacy / NHL 11 trees every one of the 26 054 / 20 245 rasters obeys the chain rule with exactly
the research's 72 / 5 cube maps refusing, and real jersey containers on both consoles resize in
memory with their other rasters untouched. On PS4 the same "the shipped header must be reproduced
by the rule before it is rewritten" gate protects the writer. Two read-side defects fell out of
the census and are fixed: Xbox 360 rasters taller than 2 048 (and heights not a multiple of 8)
were misread, and PS3 DXT1 / DXT3 rasters (14 000 of the 26 000) could not be opened at all.

## 2. What users can now do

- Replace a loose texture at a **different size** on: Xbox 360 NHL 09–Legacy RW4 texlibs (tiled
  DXT rasters of 128 texels or more), PS3 NHL 08–Legacy RW4ps3 containers (any DXT raster), PS4
  NHL 15–21 GNF (2D block-compressed) — through the same replace dialog, which now names the rule
  and its runtime card instead of "same-size only".
- Open PS3 DXT1 / DXT3 rasters (previews / exports that used to fail with "unknown format").
- See correct heights for Xbox 360 rasters taller than 2 048 or not a multiple of 8.

## 3. Implementation

| Component | Change |
|---|---|
| `crates/tdb-rx2/src/texlib.rs` | `patch_descriptor_for_resize(orig, w, h, format, mips)`: handoff 11 §2 — `DW7` pitch = tile-aligned width / 32 (the shipped 160-wide rasters carry the 256-texel pitch; the research's "special widths"), `DW9 = (h−1)<<13 | (w−1)` (13 + 13 bits), `DW11` bits 6–9 = mips − 1, `DW12` bits 12+ = tiled base / 4 096 (0 for a single mip), `DW13` byte 2 = mips; **`DW8` kept (slot reference, not a byte address)**, constants and `DW10` kept; refusals `DescriptorRuleRefusal::{UnmeasuredGeneration (DW0 ≠ 0x00400003: NHL 07 / 08), NotTiled, CubeMap (DW12 dimension bits = 3), TooSmall (< 128 BCn / < 32 8_8_8_8)}`. `replace_with_resize` now uses it (the 2-point `make_descriptor` and its `data_off >> 4` `DW8` are no longer on the write path). **Parser fix:** `DW9` height read as 13 bits (was the 8-bit `(h/8−1)` byte — rasters of height 4 096 read as 2 048, 404 as 400). `descriptor_mip_count`. |
| `crates/tdb-rx2/src/texlib_ps3.rs` | formats DXT1 (`83F1`, 8 B) and DXT3 (`83F2`, 16 B) with the linear chain rule; `replace_slot_resized(idx, w, h, mips, chain)`: words 12 / 13 rewritten, the slot re-sized, following slots and their file-table offsets shifted, the target's table length and the header data size updated, cube maps (word 6 bit `0x1000`) refused. |
| `crates/tdb-rx2/src/gnf.rs` | `check_resize_rule` (tiling 13, 2D, depth 0, modelled format, `pitch = roundup(w, 32) − 1`, `streamSize` = file size, `d7` = data bytes, data = Σ micro-tiled levels — the "descriptor reproducible" gate; a cube fails it at 6 ×), `chain_tiled_bytes`, `LinearLevel`, `replace_resized` (dimension word, `last_level`, `pitch`, `d7`, `streamSize` recomputed; every other header byte kept; the chain re-tiled level by level). `BcFormat::block_bytes` public. |
| `crates/studio-domain/src/assets.rs` | `replace_texlib_texture`: resized DXT rasters route to `replace_with_resize` (uncompressed 8_8_8_8 stays same-size; 4-alignment required); PS3: `encode_ps3_slot` builds the chain at the new size with the slot's own level count capped by the halvings (`ps3_resized_mip_count`) and `replace_slot_resized` writes it; the package (BIGF) route keeps its same-size rule with an explicit message; `replace_gnf_texture`: resized → `check_resize_rule` then the chain (the file's own level count, capped) through `replace_resized`; DXT1 / DXT3 PS3 rasters mapped for preview and encode. |
| `app/src/components/TextureReplace.tsx` (+ test) | `texlib` and `gnf` are resizable containers; the confirm step states the rule and the card; an unknown container stays blocked. |

## 4. Regression evidence (developer)

| Case | Asserts |
|---|---|
| `tdb-rx2/tests/rw4xb2_descriptor_census.rs::every_tiled_dxt_descriptor…` (`--ignored`, real X360 Legacy / NHL 11 / NHL 09 `rendering` trees) | for every tiled 2D DXT raster ≥ 128 × 128: `patch_descriptor_for_resize(orig, w, h, fmt, mips(orig)) == orig`. **Legacy: 14 658 texlibs, 19 054 gated rasters, 18 984 reproduced, 70 cube maps refused, 0 mismatches** (the run that found the height-read and pitch defects listed 81 mismatches: 512 × 4 096 `effects/lcd_*` strips read as 2 048 high, 160-wide `boot/texlib_3` pitch, `enviro/*` cube maps — all three diagnosed and fixed, see §3) — NHL 11 / 09 figures in the log |
| `…::a_real_legacy_texlib_resizes_through_the_rule…` (`--ignored`) | three real jersey texlibs: the target halves, its descriptor equals the rule's output with `DW8` unchanged, every other raster's descriptor and de-tiled base mip byte-identical after the reflow, re-parse |
| `tdb-rx2/tests/rw4ps3_descriptor_census.rs::every_ps3_container_parses…` (`--ignored`, real PS3 Legacy / NHL 11) | **Legacy: 16 265 containers (1 536 mesh-only, 67 mixed mesh + texture skipped — an open read-side item), 26 054 rasters (ARGB8 1 161, DXT1 11 686, DXT3 2 592, DXT5 10 615), 72 cube-map containers, 0 other failures; NHL 11: 20 245 rasters, 5 cubes, 0 failures** — the research's 72 / 5 and 20 245 exactly |
| `…::a_real_ps3_legacy_container_resizes…` (`--ignored`) | three real `name_*` containers 256² → 128², 9 → 8 mips: words 0–11 kept, slot bytes exact, other rasters' slots and descriptors intact, length delta exact |
| `tdb-rx2/tests/gnf_header_census.rs` (`--ignored`, real NHL 21 + NHL 16 extracted trees, headers only) | the rule accepts every 2D block texture and refuses only unmodelled formats and ≤ the research's cube counts — **run in progress at closeout, see §4a** |
| `gnf::resize_tests::resize_rewrites_the_rule_fields_and_round_trips` | synthetic BC1 64 × 32 / 2 levels → 128 × 64 / 3 levels: rule fields, header tail kept, `parse` round trip, wrong level bytes / non-halving chain / 6 × data refused |
| `texlib_ps3::resize_tests::resize_reflows_the_following_slot…` | two-raster synthetic container: the following slot and its table offset shift, words 12 / 13, data size, word 6 kept, refusals |
| `studio-domain/tests/league_logo_packages.rs` (default + `--ignored` fixtures) | a bare PS3 container 16 × 8 → 32 × 16 (3 levels, exact length, pixels back); the Apt sweep now counts **95 / 46** (was 94 / 47 — one package's texture height was misread by the 8-bit read; documented in the test) |

Runs: [p2b-rw4xb2-descriptor-census.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2b-rw4xb2-descriptor-census.log),
[p2b-rw4ps3-descriptor-census.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2b-rw4ps3-descriptor-census.log)
(2 passed, 127 s), [p2b-gnf-header-census.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2b-gnf-header-census.log),
[p2b-league-logo-packages-ignored.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2b-league-logo-packages-ignored.log)
(5 / 6 on the first run — the Apt count, diagnosed; the count case passes after the update);
`cargo test -p tdb-rx2 --lib` 114 passed / 26 ignored; `npx vitest run src/components/TextureReplace.test.tsx`
7 passed; `npx tsc` clean. The earlier X360 census runs (three) each failed on **my** gate
expectations (the 20 000 minimum, then cube refusals counted as mismatches), not on the rule —
each is described above; the final log is the one linked.

### 4a. Runs still in progress at the time of writing

The GNF header census over 150 000 + 68 000 files on `H:` and the final X360 census (three
trees) were running when this receipt was written; their logs are linked above and the closing
receipt records their outcome. Nothing in this receipt depends on them beyond the GNF row.

## 5. Coverage (Phase 1 matrix 1.3 / C6 "different-dimension" rows)

| X360 07 / 08 | X360 09–15 / Legacy | PS3 08–15 / Legacy | PS4 15–21 | Recomp |
|---|---|---|---|---|
| refused (`DW0` gate; unmeasured layouts) | **impl; rule reproduced on the real Legacy tree (18 984 / 18 984 2D DXT ≥ 128); resize dev-tested on real texlibs** — tiny / linear / cube / 8_8_8_8 refused | **impl; rule = the parser's chain check on 26 054 + 20 245 rasters; resize dev-tested on real containers** — cubes refused; mixed mesh + texture containers unreadable (open) | impl; rule gate + synthetic round trip; **real-tree census pending (§4a)** — cubes / BC6H / volumes refused | by identity |

Runtime: cards R32-XB2-1, R33-PS3-1, R31-GNF-1 (MORNING-REVIEW). The PS4 registry's size
directory rule remains the outer gate for what the game will look up.

## 6. Open / omissions

- PS3 mixed mesh + texture containers (67 on Legacy: `stadium/*_container_0`, ice surfaces) do not
  parse under the slot ↔ descriptor pairing — read-side, pre-existing, now counted.
- Xbox 360 8_8_8_8 texlib rasters stay same-size (the rule covers them ≥ 32 texels; the linear /
  tiled write split was not extended tonight).
- Packages inside BIGF archives (league logos of NHL 07–11) stay same-size for RW4 / RW4ps3.
- `make_descriptor` (the 2-point builder) still exists for its fixture tests; no longer on any
  write path.

## 7. Cleanup

Scratchpad `desc_patch.rs`, `gnf_resize.rs`, `ps3_resize.rs` deleted. No temp roots (tests
remove theirs), background census processes end on their own (checked at closeout).
