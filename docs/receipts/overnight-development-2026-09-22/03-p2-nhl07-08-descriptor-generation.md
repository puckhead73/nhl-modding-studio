# Receipt 03 — Priority 2.1: the NHL 07 / 08 descriptor generation, and where its writer still refuses

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
HEAD `8d60a6d` (unchanged). Real game trees read **only**. Nothing staged, committed, pushed,
launched or written live. Claim: `dev-p1-resize-evidence` (same package).
Research consumed (unedited): 2026-09-21 pm handoff
[02](../../research/overnight-2026-09-21-pm/02-rw4xb2-slot-join-and-resize-bounds.md) §2–3 (R42 /
R44).

## 1. Plain-language summary

Xbox 360 textures on NHL 07 and NHL 08 carry a slightly different version marker in their header than
NHL 09 and later. Studio refused to touch them for that reason alone — "this generation has not been
measured". The research lane has since measured every size-dependent word on both games' entire
texture populations and found the rules identical to the later titles. So the refusal is gone, and
Studio now reads and re-derives those descriptors by the same rule — proven here by reproducing
every shipped descriptor on both real installs (NHL 08 read through the compression wrapper EA put
around each file).

Two things deliberately stay refused, each with its own reason instead of the old blanket one:

- **NHL 07's container layout.** Those files keep their pixels inside the header region at absolute
  offsets with no separate data region. The code that moves buffers around to make room for a
  bigger texture does not describe that layout, so a *different-size* replacement is refused by
  name. Same-size replacement is unaffected.
- **110 NHL 08 jersey files.** The home and away jersey libraries of 55 teams are compressed in a
  form that does not decode — and the copy inside the archive is byte-identical, so that is how EA
  shipped them, not a bad dump. They cannot be read or replaced by this route. The census asserts
  the count is exactly 110, so if a future change made some of them decodable the test would say so.

A side effect worth noting: 243 NHL 09 descriptors also carried the older marker and were being
refused. They now work.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-rx2/src/texlib.rs` | `DESC_MEASURED_DW0 = [0x0040_0003, 0x0000_0003]` replaces the single-constant gate in `patch_descriptor_for_resize`; the refusal text names both generations. `replace_with_resize` refuses a container parsed with `hdr_size == 0` (the NHL 07 table-at-EOF layout) with its own message, **before** any other work. |
| `crates/tdb-rx2/tests/rw4xb2_descriptor_census.rs` | two new `--ignored` cases: `the_nhl_07_and_08_generation_is_reproduced_by_the_same_rule` and `the_nhl_07_container_layout_is_still_refused_by_the_writer`; the existing census gained the `DW8` join assertion (receipt 02). |
| `crates/tdb-rx2/Cargo.toml` | dev-dependency on `tdb-archive-codec` so the census can unwrap NHL 08's RefPack streams (no cycle — `tdb-archive-codec` does not depend on `tdb-rx2`). |

## 3. Evidence

`cargo test -p tdb-rx2 --test rw4xb2_descriptor_census -- --ignored --nocapture --test-threads=1`
([r1-p2-rw4xb2-census-full.log](../../validation/evidence/overnight-development-2026-09-22/logs/r1-p2-rw4xb2-census-full.log)):

| Tree | Files | Containers | `DW8` join | Gated 2D | Reproduced | Cube maps refused | Undecodable |
|---|---|---|---|---|---|---|---|
| **NHL 07** | 11 552 | 10 628 | **10 637 / 10 637** | 9 390 | **9 388** | 2 (`environment/common/helmet.rx2`, `visor_and_trophy.rx2`) | — |
| **NHL 08** (through RefPack) | 8 940 | 7 557 | **10 592 / 10 592** | 9 744 | **9 744** | 0 | **110** (the measured jersey set) |

Cross-checks against research R42 that were **not** built into the test in advance and matched
anyway: NHL 07's cube-map count (R42 §2 lists "2 / 2"), and NHL 08's undecodable count (R42 §3 lists
110 `texlib_1_<team>_{0,1}` libraries). The NHL 08 figure *is* asserted (`assert_eq!(undecodable,
110)`).

`the_nhl_07_container_layout_is_still_refused_by_the_writer`: 20 real NHL 07 containers each refuse
a halving with the layout message, and each source file is re-read and asserted unchanged.

### A captured failure, diagnosed rather than rerun

The first pass of the NHL 07 case failed: 9 388 of 9 390 gated descriptors reproduced. The two were
the cube maps above — correctly refused by the rule's own `CubeMap` gate, which the test's
capacity-based pre-filter (`slot ≥ 6 × the 2D chain`) had not excluded because their slots are not a
clean 6 ×. The **test's accounting** was wrong, not the rule. It now counts cube refusals separately,
asserts `reproduced == gated − cubes`, and additionally asserts that **no other refusal class
appears** — which is a stronger check than the original assertion, not a weaker one.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p tdb-rx2 --lib -- --include-ignored` (fixture root set) | 144 passed, 0 failed, 0 ignored |
| `cargo test -p tdb-rx2 --test rw4xb2_descriptor_census -- --ignored` (5 cases, 5 real trees) | see §3 |
| `cargo test -p studio-api --test gnf_real_resize -- --ignored` | 3 passed (unaffected) |

## 5. Coverage effect (Phase 1 matrix 1.4, RW4xb2)

| Column | Before | After |
|---|---|---|
| X360 **07 / 08** shipped-descriptor reproduction | refused (`UnmeasuredGeneration`) | **impl+tested (dev)** — 9 388 / 9 390 (NHL 07, 2 cubes refused) and 9 744 / 9 744 (NHL 08) on the real trees |
| X360 **07** changed-dimension replacement | refused (generation) | **still refused** — now by container layout, with its own reason; a bounded follow-up (implement the table-at-EOF reflow) |
| X360 **08** changed-dimension replacement | refused (generation) | **not implemented** — the descriptor rule applies, but the loose route must unwrap and re-wrap the RefPack stream; not attempted here, and the 110 undecodable jersey libraries stay refused |
| X360 **09** | 12 816 reproduced + 243 refused as older-layout | **13 059 / 13 059** — the 243 were the same `DW0 = 3` generation |
| Loader acceptance, every column | not established | **unchanged** (card R32-XB2-1) |

## 6. Open / not done

- The NHL 07 container reflow (table at EOF, absolute offsets, no data region).
- The NHL 08 loose route's RefPack unwrap / re-wrap, and therefore any NHL 08 write.
- The 110 undecodable NHL 08 jersey libraries: refused; the transform is unidentified (research R42
  §3 ruled out a simple repeating XOR).
- `DW12`'s single documented exception (`ice/texlib_0.rx2`, 16 × 256 packed mips) is below the
  128-texel gate and is therefore refused as `TooSmall`, not reproduced — unchanged behaviour.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `p2_gen0708.py`, `p2_test0708.py` | deleted at checkpoint |
| Evidence `r1-p2-rw4xb2-census-full.log` | retained (the five-tree run; ~12 min to reproduce) |
| Background processes | the census; ended before this receipt closed |
| Real installs on `X:` | read-only; the writer-refusal case re-reads and asserts each file unchanged |

## 8. Next action

Priority 2.1 continues with the remaining packaging/asset route work that current research supports,
then Priority 2.2 (localization). The NHL 07 container reflow and the NHL 08 RefPack route are the
two named follow-ups from this slice.
