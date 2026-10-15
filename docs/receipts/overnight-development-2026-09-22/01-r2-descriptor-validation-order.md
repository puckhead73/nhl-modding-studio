# Receipt 01 — OVERNIGHT-DEV-20260921-R2: dimension validation ahead of size arithmetic

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
HEAD `8d60a6d` (unchanged; 721 working-tree entries at start, 187 modified — preserved). Nothing
staged, committed, pushed, launched or written live. Dispatch:
[R1 followup review](../../validation/overnight-development-20260921-r1-followup.md).
Claim: `dev-r2-descriptor-validation-order`.

## 1. Plain-language summary

The Xbox 360 descriptor writer checked that a requested size fits the header **after** it had already
worked out how many bytes that size would occupy. For an absurd request — the reviewer's
`u32::MAX × u32::MAX` DXT5 — that calculation overflowed and the program crashed instead of politely
refusing. The check now happens first: nothing about the requested size is measured, allocated or
multiplied until the width, height and mip count are known to fit. The size calculations themselves
were also given checked forms, so if any future caller reaches them with unvalidated numbers they
return "not representable" rather than overflowing. Nothing about which sizes are *accepted* changed —
the largest valid descriptor on each branch still works and reads back exactly as before.

## 2. The defect and the fix

`patch_descriptor_for_resize` built `(min_dim, tile_texels, tiled_base)` in one binding, so
`tiled_buffer_size(u32::MAX, u32::MAX, 16)` ran before `DESC_MAX_DIM` was consulted and panicked on
the `aligned_w * aligned_h * bytes_per_block` product.

| Before | After |
|---|---|
| tuple binding computes `tiled_base` → `TooSmall` check → representability checks → arithmetic | **validation block** (`DESC_MAX_DIM` width, height; `DESC_MAX_MIPS`; `min_dim` floor; mips ≤ halvings) with no dimension-dependent arithmetic in it → **arithmetic block** on dimensions now known to be 128…8 192 (32…8 192 uncompressed) |
| `tiled_buffer_size` / `tiled_buffer_size_texels` multiply unchecked | `try_tiled_buffer_size` / `try_tiled_buffer_size_texels` (new, `Option<usize>`, `checked_add`/`checked_mul` throughout); the infallible forms now delegate and panic with a message naming the contract, so a future out-of-contract caller is unambiguous rather than a bare overflow |
| `debug_assert!(pitch <= 0x1FF)` — vanishes in release | `DW7` pitch over 9 bits returns `Unrepresentable` (unreachable under the dimension bound; no longer relies on a debug build to say so) |
| `DW12` mip page already used `u32::try_from(...).filter(...)` | unchanged |

Both branches were audited: the DXT branch (`tiled_buffer_size`, block grid × block bytes) and the
non-DXT `8_8_8_8` branch (`tiled_buffer_size_texels`, texel grid × 4) now take the same checked path
after the same validation. The other callers of the infallible helpers were audited and are bounded
by construction: `xpr.rs:267` parses `width ≤ 0x2000` / `height ≤ 2048` out of `DW9`, and
`xpr.rs:378/402` run after `replace_resized`'s measured-width allow-list.

## 3. Red evidence

`texlib::tests::patch_descriptor_refuses_extreme_dimensions_without_overflowing`, added before the
fix, failed with a panic inside `patch_descriptor_for_resize` — the reviewer's exact reproduction
([r2-red-descriptor-overflow.log](../../validation/evidence/overnight-development-2026-09-22/logs/r2-red-descriptor-overflow.log)).
The coordinator's probe recorded it as "attempt to multiply with overflow" at texlib.rs:1455; with the
checked helpers in place the same call site panics with the named contract message instead. The
defect reproduced is the same: a panic where an `Err` was contracted.

The regression is permanent and covers, on all four accepted formats (Dxt1 / Dxt3 / Dxt5 / Argb8888):

- the verbatim `(u32::MAX, u32::MAX, Dxt5, 1)` case → `Unrepresentable { what: "width" }`;
- width-only (`u32::MAX × 128`), height-only (`128 × u32::MAX`), `u32::MAX − 1` square, and the
  first invalid square (`0x2001`) → refusal naming the offending axis;
- an extreme mip count alongside an extreme size → refusal on a bound;
- the valid maxima (`8 192²` DXT5, `8 192 × 128` DXT1, `8 192 × 32` `8_8_8_8`) still succeed and their
  `DW9` reads back exactly — the R1 behaviour is retained;
- `try_tiled_buffer_size*` return `None` on `u32::MAX` and agree with the infallible forms on valid
  input.

The R1 truncation regressions (`patch_descriptor_refuses_unrepresentable_dimensions_instead_of_masking_them`,
`replace_with_resize_refuses_before_building_and_round_trips_the_boundary`, the GNF pair and the
public-route case) are untouched and still pass.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p tdb-rx2 --lib` | **118 passed, 0 failed, 26 ignored** (117 + the new regression) |
| `cargo test -p studio-api --test loose_texture_route` | **2 passed, 5 ignored** (private fixtures) |
| `cargo clippy -p tdb-rx2 --lib` | my R1/R2 code clean after fixing two `manual_range_contains` in `gnf.rs::encode_gnf`; one `manual_range_contains` remains at `xpr.rs:360` (another package's working-tree code — recorded, not repaired) |
| `cargo fmt -p tdb-rx2 -- --check` | 22 diffs in `texlib.rs` (was ~24): the code **added in R2** is rustfmt-clean; the remainder are receipt-11-era dense lines inside and around this function and the test module's one-line asserts — part of the repo-wide C0.2 baseline, not repaired here |

Not rerun (F2 is closed for this review; no code they cover changed): the Vitest suites, TypeScript,
the real-tree censuses, the full Rust aggregate.

## 5. Receipt 16 correction

A dated note was appended to
[receipt 16 §4](../overnight-development-2026-09-21/16-r1-resize-representability-and-swap-ownership.md):
its "validate before allocate" claim was true of the *masking* defect but not of every input to the
codec, because the size arithmetic preceded the bound. The claim now reads as corrected by this
package, with the reviewer's reproduction named.

## 6. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `r2_red.py`, `r2_helpers.py`, `r2_fix.py` | deleted at checkpoint |
| Evidence `r2-red-descriptor-overflow.log` | retained |
| Temp roots / processes | none created |
| Shared caches, other lanes' files, fixtures, sources | untouched |

## 7. Next action

Priority 1A: real GNF changed-dimension round trips on disposable copies of hash-identified NHL 21
and NHL 16 sources, consuming research handoff 03 (per-title GNF census and bounds, incl. the NHL 20
identity and the NHL 15 two-stream refusal).
