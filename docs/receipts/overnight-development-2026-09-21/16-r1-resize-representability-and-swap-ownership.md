# Receipt 16 — OVERNIGHT-DEV-20260921-R1: resize representability (F1) and swap completion ownership (F2)

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`; the dirty working tree of the campaign is preserved. Nothing staged, committed,
pushed, launched or written live; real trees read only, writes in memory or on temp copies.
Dispatch: [review R1](../../validation/overnight-development-2026-09-21-review-r1.md). Bounded
correction package — no roadmap feature was started.

## 1. Plain-language summary

Two things the review found wrong are fixed, each first reproduced by a test that failed for the
reason the review gave.

**Sizes that do not fit.** A texture's header stores its width and height in a fixed number of bits.
When someone asked for a size bigger than the field can hold, the writers on Xbox 360 and PS4 quietly
chopped the number to whatever bits fit — a 16 400-wide PS4 request was written as 16 wide, an
8 196-wide Xbox 360 request as 4 wide — and reported success. Now the writer checks that the
requested width, height and number of mip levels can actually be stored **before** it builds anything,
refuses with a message naming the value and the limit if not, and after writing re-reads its own
output to confirm it says what was asked. The refusal reaches the user through the normal Replace
dialog route with the file, its backup and the current texture untouched. The PS3 writer already
refused sizes over 16 bits; it gained the same level-count bound, overflow checks and read-back
(no defect was found there — an audit, not a fix). Sizes that do fit — including the largest one on
each axis — work and read back exactly.

**A failed swap from the wrong project.** The Team page's art-id swap remembered which team it was
started for but not which project. If a swap failed after the user had moved to a different project
that happened to show a team with the same number, the error appeared on that other project's card.
Every completion of a swap — success, failure, and the refresh of the owning project's summary or
its failure — now carries the exact project, team, partner and card generation it was dispatched
with, and only acts on the card that still matches. The successful swap still refreshes its own
project's summary and is still one undoable step.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-rx2/src/gnf.rs` | `replace_resized`: representability before allocation (`GNF_MAX_DIM` 16 384, `GNF_MAX_LEVELS` 16, levels ≤ halvings, u32 `streamSize`), no masking, output re-read through `check_resize_rule` and compared with the request; `GnfError::Unrepresentable`; `encode_gnf` now emits a rule-conformant one-level header (type 9, pitch, `streamSize`, `d7`) so synthetic fixtures take the resize route; tests. |
| `crates/tdb-rx2/src/texlib.rs` | `patch_descriptor_for_resize`: `DESC_MAX_DIM` 8 192, `DESC_MAX_MIPS` 16, mips ≥ 1 and ≤ halvings, 20-bit mip page checked; masks removed; `DescriptorRuleRefusal::Unrepresentable`. `replace_with_resize`: descriptor derived **first** (before any buffer), the chain must halve, u32 slot check, output re-parsed and compared with the request. `encode_synthetic_texlib` (public synthetic builder for tests). Tests. |
| `crates/tdb-rx2/src/texlib_ps3.rs` | `replace_slot_resized` audit: level bound, u32 slot / file length checks, output re-parsed and compared (16-bit gate was already present). |
| `crates/studio-domain/src/assets.rs` | `replace_gnf_texture` / `replace_texlib_texture`: an early gate on the codec's constants so an unrepresentable request is refused before the image is block-compressed (the codec stays the authority). |
| `crates/studio-api/tests/loose_texture_route.rs` | `an_unrepresentable_resize_is_refused_through_import_texture_and_the_target_is_untouched` — the public route on both families. |
| `app/src/components/TeamIdentityCard.tsx` | `SwapContext` as the mutation's variables; `ownsCard` (generation + mounted) / `ownsProject` for every completion path; `pending` owned by the dispatching card; `mountedRef` re-armed on mount. |
| `app/src/components/TeamIdentityCard.test.tsx` | 7 new deferred-result cases (10 total). |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md`, `docs/validation/phase-2-coverage-matrix-2026-09-20.md` | "Correction R1" addenda (the earlier rows stand as the earlier accounting). |
| `docs/validation/evidence/overnight-development-2026-09-21/logs/r1-*.log` | red and green logs (§3, §5). |

No DTO, binding or command signature changed.

## 3. Red evidence (before the fixes)

| Regression | Red reason | Log |
|---|---|---|
| `gnf::resize_tests::resize_refuses_unrepresentable_dimensions_instead_of_masking_them` | `replace_resized` returned `Ok` for 16 400 × 4; the T# dimension word read `0F C0 00 00` = width **16**, height 4 | `r1-f1-gnf-red.log` |
| `texlib::tests::patch_descriptor_refuses_unrepresentable_dimensions_instead_of_masking_them` | `Ok` for 8 196 × 128; `DW9` = `00 0F E0 03` = width **4**, height 128 | `r1-f1-texlib-red.log` |
| `texlib::tests::replace_with_resize_refuses_before_building_and_round_trips_the_boundary` | the container writer accepted the 8 196-wide chain | same log |
| `TeamIdentityCard.test.tsx` — "a swap failing after the card moved to another project (same team id)" | `project-seven-only-error` rendered on project 8's card (the reviewer's reproduction) | `r1-f2-teamidentity-red.log` |
| — "… moved to another team (same project)" | `team-three-only-error` rendered on team 5's card | same |
| — "switching away and back before a completion / failure lands" | the returned-to card was disabled as "pending" on the stale request; the stale failure rendered | same |

The two remaining new UI cases (unmount; summary-refresh failure on the owning card / after a project
move) passed against the delivered code and are kept as guards.

## 4. Key decisions

- **Refuse, never clamp.** Every bound is checked against the *requested* value; the error names the
  value and the field's range. The bounds are the fields' own (14 / 13 bits, 4 bits), plus "no more
  levels than the size halves to" — a chain of extra 1 × 1 levels is not a layout any shipped file
  carries, and the domain already caps by `full_mip_count`.
- **Validate before allocate; re-read after write.** **Corrected 22 Sep 2026 (R2, receipt
  [01](../overnight-development-2026-09-22/01-r2-descriptor-validation-order.md)):** this was true of
  the masking defect but *not* of every input to the codec — `patch_descriptor_for_resize` still
  computed the tile-aligned size before consulting `DESC_MAX_DIM`, so `(u32::MAX, u32::MAX, Dxt5, 1)`
  panicked on a multiplication overflow instead of refusing (coordinator followup review, 2026-09-22).
  Validation now precedes all dimension-dependent arithmetic and the size helpers have checked forms.
  The original text follows as written. Both writers derive / check the header first and
  re-parse their output (GNF through `check_resize_rule`, texlib through `parse`, PS3 through `parse`)
  comparing size, format and level count with the request. A writer that cannot reproduce its own
  request returns an error, not bytes.
- **`encode_gnf` is rule-conformant now.** It was documented as a synthetic builder writing "only the
  fields `parse` reads"; it now also writes the type / pitch / size words, so the API-route test can
  resize a synthetic GNF. Existing same-size tests compare the header they built against itself and
  are unaffected (aggregate green, §5).
- **F2: identity in the variables.** The dispatch context is passed to `mutate` and reaches every
  callback as the mutation's variables — immutable, per request — instead of closure values that
  react-query refreshes on each render. Same-project-different-team and away-and-back are both
  handled by the generation (bumped with the reset on every project / team change); the owning
  project's summary refresh keys on the project alone, as before.
- **Not changed:** the runtime cards R31-GNF-1 / R32-XB2-1 / R33-PS3-1 / T-4 (deferred for the user);
  NHL 21 E1; XPR2 `replace_resized` (audited: explicit allow-list of measured widths and a 2 048
  height cap — no masking, nothing to fix).

## 5. Checks run

| Run | Result | Log |
|---|---|---|
| `cargo test -p tdb-rx2 --lib` | 117 passed, 0 failed, 26 ignored | (in the aggregate) |
| `cargo test -p studio-api --test loose_texture_route` | 2 passed, 5 ignored (private fixtures) | (in the aggregate) |
| `cargo test --no-fail-fast -p tdb-rx2 -p studio-domain -p studio-api` | **621 passed, 0 failed, 80 ignored, exit 0** (one run) | `r1-aggregate-default.log` |
| `--ignored` real changed-dimension round trips: `rw4xb2_descriptor_census::a_real_legacy_texlib_resizes…` (three real Legacy X360 jersey texlibs halved in memory, other rasters byte-identical) and `rw4ps3_descriptor_census::a_real_ps3_legacy_container_resizes…` (three real Legacy PS3 containers 256² → 128²); `studio-domain --test league_logo_packages --include-ignored` (9 passed, incl. `dimension_change_rewrites_texture_and_apt_geometry_on_the_resizable_families` and the private gen7 / PS4 logo sweeps) | 1 + 1 + 9 passed | `r1-real-resize-roundtrips.log` |
| `tdb-rx2 --lib replace_with_resize -- --ignored` (`replace_with_resize_same_dims_round_trip`, `…grows_earlier_slot`) | **missing-fixture** — `legacy/texlib_67.rx2` is not under `E:
hl-studio-fixtures` (2 failed as C0-FX1 designs: selected + absent = fail, not skip); the same real-container round trips ran on the X: install above | same log |
| `npx tsc --noEmit` | clean | — |
| `npx vitest run src/components/TeamIdentityCard.test.tsx` | 10 passed (was 4 failed / 6 passed red) | `r1-f2-teamidentity-red.log` (red) |
| `npx vitest run` | 26 files, 182 passed (175 + 7) | `r1-frontend-full.log` |
| bindings | not run — no DTO / signature changed (`export_bindings` unit tests passed in the aggregate) | — |

Skips: the real-tree descriptor censuses (10–66 min each) were **not** rerun — the rule fields they
reproduce are untouched by this package (only refusals and the read-back were added); the
`--ignored` `tdb-rx2` private-fixture cases were not selected. No captured failure was replaced by a
rerun: the two harness timing corrections in the F2 tests (waiting for the new project's dependency
query; tolerating the `invalidateQueries` refetch) are described in §3's log and changed no
expectation.

## 6. Remaining limitations

- Runtime acceptance of any resized texture is still unestablished on all three families (cards
  unchanged). Shipped-descriptor reproduction, changed-dimension replacement (dev) and loader
  acceptance are stated separately in the Phase 1 addendum.
- No real GNF was resized in this package (synthetic sources through the public route); receipt 11's
  GNF claim was synthetic too.
- PS3 mixed mesh + texture containers, Xbox 360 8_8_8_8 resizes and packages inside BIGF stay as
  receipt 11 left them.
- The UI does not pre-announce the size bounds before the write; the refusal text does.

## 7. Cleanup manifest (package-owned only)

| Artifact | Disposition |
|---|---|
| Scratchpad `fix_texlib.py`, `fix_texlib2.py`, `fix_domain2.py`, `add_route_test.py`, `f2_tests.py`, `f2_fix.py` | deleted at closeout |
| Temp roots `nhlms-c1-loose-r1-f1-<pid>` (the route test's `Tmp`) | removed by the test's `Drop` |
| Background processes | none left (the aggregate / real-fixture runs ended) |
| Shared caches (`target/`, `app/node_modules`) and other lanes' files | not touched |
| Evidence logs `r1-*.log` | retained |
