# Receipt 02 — Priority 1: real GNF resize evidence, the private fixture root, and the `DW8` slot join as a check

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
HEAD `8d60a6d` (unchanged). Real game trees read **only**; every write went to a disposable temp copy
or to the private fixture root outside the repository. Nothing staged, committed, pushed, launched
or written live. Claim: `dev-p1-resize-evidence`.
Research consumed (unedited): 2026-09-21 pm handoff
[02](../../research/overnight-2026-09-21-pm/02-rw4xb2-slot-join-and-resize-bounds.md) §1–4 (R42 /
R47) and [03](../../research/overnight-2026-09-21-pm/03-gnf-per-title-census-and-resize-bounds.md)
(R43).

## 1. Plain-language summary

Three gaps the coordinator recorded are now closed.

**Real PS4 textures actually resize.** Until now every PS4 resize claim rested on containers Studio
built itself. Six real shipped textures from two different games (NHL 21 and NHL 16 — logos, a hat
normal map, a trophy map, in BC1, BC3 and BC5, both single-level and fully mipped) are now halved,
re-grown and re-read on throwaway copies, both directly through the codec and through the same
API route the app's "Replace…" dialog calls (`import_texture` → `texture_preview`; the dialog
itself is not exercised). The strongest result: halving one of these and growing it back
produces a header that is **byte-for-byte the shipped header** — the rule is not merely plausible,
it reproduces EA's own bytes. That is a statement about the header fields only: the regrown
*pixels* are the product of a lossy downscale then upscale and are not the shipped texels. Through
the API route the written file re-opens at its new size, the picture in it is the picture that was
imported (within the per-format bound the test states), and the automatic backup restores the
original exactly. The original game files were never written to.

> **Correction note (22 Sep 2026, coordinator review):** the two distinctions above — API route, not
> dialog; header reproduction, not pixel survival — were added to this paragraph; §3b already stated
> both and is unchanged.

**The test fixtures exist again.** Six tests were failing loudly because the private fixture folder
was empty — the two the coordinator named, plus four more. Each missing file was tracked down by the
provenance already written in the code (not by guessing at similar names), its contents checked
against the documented description, copied read-only, and pinned by hash. One of them had to be
unpacked out of a compressed archive member first; Studio's own reader then confirmed it. The
`tdb-rx2` suite goes from 118 passing with 26 loudly failing to **144 passing, none skipped**.

**A guess became a check.** The Xbox 360 descriptor holds a number that points at where the texture's
pixels live. Studio preserved it without being able to say what it meant. The research lane has now
measured it on every descriptor of ten games: it is the position of the pixel entry in the
container's table. Studio now *verifies* that before rewriting a container and refuses one that
disagrees, rather than trusting it. Descriptors that are not textures at all (a null format) are
refused by name as well.

**One reason got more honest.** Some PS4 files are two textures stored back to back. Studio refused
them, but blamed a size mismatch. It now says "multi-stream file" and explains that resizing one
stream would drop the other.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-api/tests/gnf_real_resize.rs` | **new** — three `--ignored` cases over six SHA-256-pinned real GNF sources: the shipped-shape gate, a codec halve → regrow round trip, and the public `import_texture` → `texture_preview` → restore route. |
| `crates/tdb-rx2/src/gnf.rs` | `GnfRuleRefusal::MultiStream` + `count_tiled_streams` — a file whose concatenated `GNF ` streams tile it exactly is refused as a multi-stream container, not as a `streamSize` mismatch (research 03 §2: NHL 15 ships 1 115, NHL 19 5 342). |
| `crates/tdb-rx2/src/texlib.rs` | `descriptor_data_entry_index` (`DW8` bits 12+) and `Texture::data_entry_index`; `replace_with_resize` refuses `SlotJoinMismatch`; `patch_descriptor_for_resize` refuses `NullFormatDescriptor`; a `debug_assert` that the patch preserves the index. |
| `crates/tdb-rx2/tests/rw4xb2_descriptor_census.rs` | the census now also asserts the `DW8` join on **every** descriptor it walks, not only the gated ones. |
| `docs/validation/c0-fx1-explicit-fixture-tests.md` | §9 addendum — the six fixtures, their originals, hashes, content checks and exactly which coverage each supplies. |

## 3. Evidence

### 3a. Real GNF sources (all pinned by SHA-256; originals read-only)

| Fixture | Title / origin | Shape |
|---|---|---|
| `ps4-gnf/nhl21-alphaheat.gnf` | NHL 21 CUSA18182 `…/rendering/textures/alphaheat.gnf` | 256² BC1, 9 levels |
| `ps4-gnf/nhl21-customlogo_46_diff.gnf` | NHL 21 `…/customlogos/` | 1024² BC3, 11 levels |
| `ps4-gnf/nhl21-customlogo_adirondack_thunder_normal.gnf` | NHL 21 `…/customlogos/` | 1024² BC5, 11 levels |
| `ps4-gnf/nhl21-customlogo_adirondack_thunder_recolor2.gnf` | NHL 21 `…/customlogos/` | 1024² BC1, **1 level** |
| `ps4-gnf/nhl16-hat_0_0_nm.gnf` | NHL 16 CUSA01909 `…/rendering/textures/` | 512² BC5, 10 levels |
| `ps4-gnf/nhl16-trophy_0_0_coeff.gnf` | NHL 16 `…/rendering/textures/` | 1024² BC3, 11 levels |

Hashes are in the test source and in the C0-FX1 addendum. Every case re-hashes its fixture at the
end and fails if it changed.

### 3b. What the three cases assert

| Case | Asserts |
|---|---|
| `every_pinned_source_passes_the_rule_at_its_shipped_shape` | `check_resize_rule` accepts each shipped header and reads the same shape the research census recorded; `parse` agrees; tiling 13 |
| `halving_a_real_gnf_through_the_codec_rewrites_only_the_rule_fields` | half-size write → header reads back at the requested size / level count; file length = `0x100 + chain`; **every header byte outside the rule's five fields is identical**; re-parse returns the level that was written; regrow to the shipped shape → **`out[..0x100] == original[..0x100]`, the shipped header reproduced exactly**, and the file length matches the shipped file |
| `a_real_gnf_changes_dimensions_through_the_public_route_and_restores_from_its_backup` | `texture_preview` before → `import_texture` (half size, backup on) → summary size/container; `.bak` is the shipped file byte-for-byte; the written file passes the rule at the new size; preserved header bytes identical; **re-opens** through `texture_preview` at the new size; decoded pixels are the imported image (per-format bound; BC5 carries two channels) and are **not** the shipped image; restoring from `.bak` is byte-exact and re-opens at the shipped size |

Result: **3 passed** (`NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test
gnf_real_resize -- --ignored`, 8.9 s) —
[r1-p1-gnf-real-resize.log](../../validation/evidence/overnight-development-2026-09-22/logs/r1-p1-gnf-real-resize.log).

### 3c. Fixture root populated (Priority 1B)

Six keys, each located by the provenance in `fixtures.rs` and content-verified — full table in the
[C0-FX1 §9 addendum](../../validation/c0-fx1-explicit-fixture-tests.md). The two the coordinator
named (`replace_with_resize_same_dims_round_trip`, `replace_with_resize_grows_earlier_slot`) pass.
`legacy/playerheadssmall_p1512.xpr2` had to be extracted from `p1512.big` member `194` and
RefPack-decompressed (8 455 → 32 864 B, matching the declared length); Studio's own `Xpr2Texture`
then reads it as 256×128 DXT5, which is the independent check on that extraction.

`cargo test -p tdb-rx2 --lib -- --include-ignored`: **144 passed, 0 failed, 0 ignored** (was 118
passed + 26 failing-for-missing-fixture).

### 3d. `DW8` slot join (Priority 1C)

Research R42 §1 measures `DW8`'s upper field (bits **12**+, not 8 — confirmed independently here on
`texlib_67.rx2`: `0x2052 >> 12 = 2`, the index of that texture's data entry) as the file-table index
of the paired pixel-data entry: 100 % on NHL 08–Legacy, 10 666 / 10 675 on NHL 07. Studio keeps
*preserving* the word — this reflow never changes the table's membership or order, so preservation
stays correct — but now **checks** it first and refuses `SlotJoinMismatch` otherwise, and refuses
null-format descriptors (the nine NHL 07 `fx/common/texture.xml.rx2` records). Exercised on real
multi-raster data by the `texlib_67` resize cases and at tree scale by the census (§4).

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p tdb-rx2 --lib -- --include-ignored` (fixture root set) | **144 passed, 0 failed, 0 ignored** |
| `cargo test -p studio-api --test gnf_real_resize -- --ignored` | **3 passed** |
| `cargo test -p studio-api --test loose_texture_route` | 2 passed, 5 ignored (private logo fixtures) |
| `cargo test -p tdb-rx2 --test rw4xb2_descriptor_census -- --ignored` (Legacy + NHL 11 + NHL 09, with the new join assertion) | CENSUS_RESULT |

## 5. Coverage effect

Phase 1 matrix row "1.4 Replace with different dimensions — GNF (PS4)": the *changed-dimension
replacement* column moves from "synthetic round trip only" to **impl+tested (dev) on real NHL 21 and
NHL 16 members through the codec and the public route, with the shipped header reproduced exactly on
a halve→regrow round trip**. Shipped-descriptor reproduction (the census) and **loader acceptance**
are unchanged: card R31-GNF-1 stays open, and packaging through the PS4 registry's size directory is
still a separate gate that is not claimed.

Not covered by a real file: **BC2, BC4, BC7 and BC6H** GNF members (BC7 remains synthetic-only;
BC6H has no encoder and stays refused), PS3 `RW4ps3`, and NHL 15 / 17 / 18 / 19 / 20 GNF sources.
Recorded as a gap — not implied by the two titles measured.

## 6. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `pick_gnf*.py`, `pick_bc7.py`, `extract_xpr2.py`, `check67.py`, `p1_gnf_multistream.py`, `p1c_dw8.py`, `p1c_census.py` | deleted at checkpoint (the extraction recipe is recorded in the C0-FX1 addendum, so the fixture is reproducible without the script) |
| Private fixture root additions (~5 MB GNF + ~1 MB gen7) | **retained** — named dependency of `gnf_real_resize.rs` and the 26 `tdb-rx2` `--ignored` cases; outside the repository, never committed |
| Temp roots `nhlms-gnf-real-route-<pid>` | removed by the test's `Drop` |
| Background processes | the census run; ended before this receipt was closed |
| Original game trees, shared caches, other lanes' files | untouched (read-only) |

## 7. Next action

Priority 2.1: lift the `UnmeasuredGeneration` (`DW0 = 3`) refusal for NHL 07 / 08 `RW4xb2` on the
evidence of research 02 §2–3 (same rules measured: 07 pitch 10 666 / 10 666, `DW12` 9 554 / 9 555;
08 RefPack-wrapped, 14 066 / 14 066), with the 110 undecodable NHL 08 jersey libraries refused by
name pattern plus decode failure.

---

## 8. Addendum — the census result (§4's `CENSUS_RESULT`)

`cargo test -p tdb-rx2 --test rw4xb2_descriptor_census -- --ignored --nocapture --test-threads=1`
([r1-p2-rw4xb2-census-full.log](../../validation/evidence/overnight-development-2026-09-22/logs/r1-p2-rw4xb2-census-full.log)):

| Tree | Gated rasters reproduced | **`DW8` slot join** |
|---|---|---|
| NHL Legacy X360 | 18 984 / 18 984 (unchanged) | **22 401 / 22 401** |
| NHL 11 X360 | 14 659 / 14 659 (unchanged) | **16 981 / 16 981** |
| NHL 09 X360 | **13 059 / 13 059** — was 12 816 reproduced + 243 refused as `UnmeasuredGeneration`; those 243 `DW0 = 3` descriptors are now reproduced by the lifted rule | **13 836 / 13 836** |

The join holds on **every** descriptor of all three trees — research R42 §1's 100 % result,
reproduced independently by Studio's own pairing. That is what turns `DW8` from a preserved unknown
into the checked refusal described in §3d.

A failure was captured and diagnosed rather than rerun: the first NHL 07 pass reported 9 388 of
9 390 gated descriptors reproduced. The two were `environment/common/helmet.rx2` and
`visor_and_trophy.rx2` — **cube maps**, correctly refused by the rule's own `CubeMap` gate, which
the test's capacity-based pre-filter had not excluded (their slot is not a clean 6 × the 2D chain).
Research R42 §2 independently lists "2 / 2" cube maps for NHL 07. The *test's accounting* was wrong,
not the rule; it now counts cube refusals separately and additionally asserts that no *other*
refusal class appears. See §9.
