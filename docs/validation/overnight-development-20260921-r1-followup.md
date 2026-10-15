# R1 coordinator review — 2026-09-22

Disposition: F2 swap completion ownership accepted offline for the reviewed cases. F1's original dimension truncation is fixed; one residual codec-boundary defect requires a small R2. No game/runtime acceptance is added and NHL 21 E1 remains closed. This is scoped acceptance, not acceptance of the entire overnight campaign.

Independent source review and checks:

- `cargo test -p tdb-rx2 --lib resize`: 4 passed, 4 private cases ignored.
- `cargo test -p studio-api --test loose_texture_route`: 2 passed, 5 private cases ignored, including the oversized public-route refusal and valid changed-dimension outputs.
- TeamIdentityCard Vitest: 10 passed, including cross-project/team failures and away-and-back ownership.
- 16 tests passed in total. The full suites, real-container sweeps and TypeScript were not independently rerun. The developer's missing-fixture failures remain recorded, not counted as passes. No real GNF resize evidence was added here.

## R2 finding — P2: size arithmetic still precedes the Xbox bound

`crates/tdb-rx2/src/texlib.rs`, `patch_descriptor_for_resize`, constructs `(min_dim, tile_texels, tiled_base)` before checking `DESC_MAX_DIM`. `tiled_buffer_size` multiplies aligned dimensions and block size unchecked.

Independent regression: start with a 56-byte descriptor containing big-endian dw0 `0x00400003` and dw7 `0x80000000`; call `patch_descriptor_for_resize(&descriptor, u32::MAX, u32::MAX, PixelFormat::Dxt5, 1)` and expect `Err`. In the debug test it instead panics at texlib.rs:1455, "attempt to multiply with overflow". No large buffer or source file was needed. This is a direct codec-API defect; the new public image-route gate guards that route, and no live corruption is claimed. The report's "before building anything"/checked-boundary claim is not yet true for all inputs to the codec.

## Prompt — same developer session, bounded R2

Finish OVERNIGHT-DEV-20260921-R2 before opening another feature. Read this review and receipt 16. Move dimension and level validation ahead of dimension-dependent size arithmetic in `patch_descriptor_for_resize`; use checked arithmetic where a reachable calculation is not bounded. Audit the ARGB/non-DXT branch of this same function as well. Keep the existing supported dimensions and layout rules unchanged.

Add permanent regressions for the exact DXT5 `u32::MAX` case, extreme width-only/height-only inputs and the non-DXT branch. Each must return a descriptive error without panic or allocation; retain the valid-max and original truncation regressions. Do not merely catch the panic. Run affected codec tests and the public loose-texture route tests. F2 is closed for this review: do not rework the UI or repeat unrelated suites unless the code changes require it.

Correct receipt 16's boundary-validation claim with a dated note and record fresh results. Preserve dirty work, shared caches, sources and fixtures; no launches/live writes/staging/commits/pushes. Clean only package-owned scratch. Return a plain-language-first receipt and stop for coordinator review. A new session is unnecessary for this narrow correction.

After R2 closure, the next bounded validation package should cover a real GNF changed-dimension round trip on disposable bytes and repair the two missing-fixture test inputs (with verified provenance). Reuse existing fixtures where possible. This closes an explicit evidence gap before further resize expansion; it is not a request for user runtime testing.

Reviewer temporary probe and its build artifacts were removed after recording the reproduction; no production source edits.
