# Receipt 01 (R4) — Correction checkpoint: the Xbox 360 write boundary and the audit's claims (F1 / F2)

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Real Xbox 360 trees on `X:` read **only**;
every write went to memory or a disposable temp copy. Nothing staged, committed, pushed, launched or
written live. Dispatch: [the coordinator review of receipts 10–17](../../validation/overnight-development-20260922-receipts10-17-review.md),
fresh-session items 1 (F1 / F2). Claim: `dev-r4-xbox-write-boundary`. The accepted kit correction
(receipt 10) was not reopened.

## 1. Plain-language summary

The coordinator found two problems with how Studio handled Xbox 360 texture mip chains after
receipt 11. First, a *changed-size* replacement still wrote the whole chain, including the small
"packed tail" levels whose layout nobody has established, and merely attached a note saying so.
Second, the audit that justified the rest of the route overstated itself: it called every large
level "verified", relabelled two failing levels "borderline", cited a withdrawn research measurement
and an old code comment as support, and did not say exactly which formats, shapes and titles it had
looked at.

Both are corrected here.

- **The write boundary.** A changed-size replacement of an Xbox 360 raster that has a mip chain is
  now **refused** before anything is backed up or written, with the reason; the chain is not
  quietly shortened to slip past the rule. A single-level raster still changes size (one level
  re-emitted). A same-size replacement is now explicitly a **partial** replacement: the base and —
  only for rasters inside the audited population — the large levels are rewritten, every packed-tail
  level keeps its shipped bytes, and the dialog says so **before** the write in exactly the words
  the result repeats. Rasters outside the audited population (other formats, shapes, descriptor
  forms, cubes, single level) get a base-only write and are told which property excluded them. The
  writer reports the exact byte ranges it wrote, and tests check that every other byte is
  untouched.
- **The audit's claims.** The audit test now applies one declared classifier to every level and
  reports the classes it yields — *matched-decisive*, *matched-nondecisive*, *unmatched*, *zero* —
  with unrounded decision values, per-format / per-shape totals and the titles run. It was re-run
  across 15 families on 8 Xbox 360 titles (4 700 rasters). The large levels: 9 185, of which 7 224
  matched-decisive, 1 953 matched-nondecisive and 8 unmatched (two jersey images recurring on four
  titles, just over the threshold, each beating both controls — recorded as data, still unmatched).
  The packed tail: 4 228 of 14 100 unmatched. The audit record says the comparison is an approximate
  box-filter match, RGB only, and never claims what the GPU reads. The handoff 17 citation is gone;
  the debug-blue comment survives only as an attributed historical note whose original evidence is
  not in this repository.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-rx2/src/texlib.rs` | `LargeLevelSupport` + `Texlib::large_level_support` (the applicability gate), `AUDITED_MIN_DIM` / `AUDITED_MAX_DIM`, `Texlib::declared_mip_count`, `Texlib::header_size`; `KeepTailWrite { support, written_ranges, … }` (the byte-write mask); `replace_large_levels_keep_tail` takes the base-only route outside the gate; handoff 17 citation removed; debug-blue text reduced to an attributed historical note. Unit cases: the mask / preservation assertions added to `keep_tail_writer_rewrites_only_the_content_verified_levels`; **new** `keep_tail_writer_falls_back_to_base_only_outside_the_audited_population` (128² chain; 512² chain with the NHL 07 / 08 form) |
| `crates/tdb-rx2/tests/rw4xb2_mip_placement.rs` | the declared classifier (`classify`, `MATCH_THRESHOLD`, `CONTROL_MARGIN`), unrounded values, per-shape tallies, `unmatched … beat both controls` as data, 15 families on 8 titles, grand totals |
| `crates/studio-domain/src/assets.rs` | `Xb2DxtPolicy`, `xb2_dxt_policy`, `xb2_policy_notes`; `replace_texlib_texture_noted` decides the policy before encoding — refuses a changed size on a mipped raster, re-emits one level for a single-level raster, and refuses if the writer's outcome ever differs from the policy's prediction (so the disclosure cannot describe a write that did not happen); `plan_texture_replace` / `TextureReplacePlan` |
| `crates/studio-domain/src/lib.rs` | re-exports |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | `TextureReplacePlanDto`, `Registry::texture_replace_plan`, the `texture_replace_plan` command (export roots 142 → 143) |
| `crates/studio-api/tests/loose_texture_route.rs` | **new** `a_changed_size_write_into_a_mipped_xbox_raster_is_refused_before_any_mutation_and_the_same_size_write_is_partial` |
| `app/src/bindings/TextureReplacePlanDto.ts` | **generated** (`--write`; `--check` in sync) |
| `app/src/components/TextureReplace.tsx` (+ `.test.tsx`), `app/src/lib/client.ts` | the dialog asks the plan once both previews are decoded; a refusal disables Write with the reason; partial-replacement notes appear before Write and again with the result; an unavailable plan is stated and does not override the writer's rules. Three new Vitest cases |
| `docs/validation/xbox360-mip-layout-audit-2026-09-22.md` | **rewritten** with a supersession note: restated classifier and results, exact sampled titles / formats / shapes, applicability gate + byte-write mask (§5), retained-outside-the-flow codec writers (§6), the 22 Sep run kept for the record (§8) |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | Correction R4 rows |
| `docs/receipts/overnight-development-2026-09-22/11-r3-xbox360-mip-layout-audit.md`, `MORNING-REVIEW.md` | dated correction notes (receipt 11 §1 / §3 / §5 claims; card V-31; card R32-XB2-1 rev. 3) |

## 3. Red / green

| Case | Before | After |
|---|---|---|
| Changed size on a 10-level 512² DXT1 raster through `Registry::import_texture` / `import_texture_bytes` | wrote the whole chain, tail under the unqualified model, note attached | **refused** with the reason; target bytes identical, no `.bak`, `texture_preview` unchanged; the plan carries the same refusal; 1024² refused the same way (chain never shortened) |
| Same size on the same raster | keep-tail write, result note predicted what the console shows at distance | partial write; plan note = result note word for word; `.bak` = original; bytes before the slot and after the three large levels identical to the shipped file; other raster untouched; declarations / dimensions / format / slot capacity unchanged; base reads back; no runtime prediction in the note |
| Changed size on a single-level 128² raster | full chain generated | one level re-emitted, declared count stays 1, disclosed |
| A 128² mipped chain / a 512² chain with `DW0 = 0x00000003` through the keep-tail writer | large levels rewritten as if audited | **base-only**, reason names the shape / descriptor form; one written range; every other byte preserved |
| The audit's two NHL 11 large levels over the threshold | "borderline, not contradictions" | *unmatched*, beating controls recorded as data |

## 4. Evidence

- Placement audit, restated: [r4-xb2-mip-placement-audit-restated.log](../../validation/evidence/overnight-development-2026-09-23/logs/r4-xb2-mip-placement-audit-restated.log)
  — 15 families, 8 titles, 4 700 rasters, 832 s; per-family and per-shape tables in the audit record §3.
- Public-route regression: `cargo test -p studio-api --test loose_texture_route` — 3 passed, 5
  ignored (private fixtures).
- Codec: `cargo test -p tdb-rx2 --lib` — 120 passed, 26 ignored.
- Dialog: `npx vitest run src/components/TextureReplace.test.tsx` — 10 passed; `npx tsc --noEmit` clean.
- Closeout regression: [r4-closeout-regression.log](../../validation/evidence/overnight-development-2026-09-23/logs/r4-closeout-regression.log) (see [STATUS](STATUS.md) for the totals).

## 5. Coverage effect (Phase 1 matrix 1.4, Xbox 360 `RW4xb2`)

| Row | 22 Sep | 23 Sep |
|---|---|---|
| Same-size DXT replacement | "base + large levels at content-verified offsets; tail kept; at gameplay distance the previous image shows" | **partial replacement, impl+tested (dev)**: base + large levels inside the audited population (§5 of the audit record), base only outside it, tail kept; disclosed before Apply and in the result as *not a full-image replacement*; byte-write mask asserted |
| Changed-size DXT replacement, mipped raster | written under the sequential model with a note; card R32-XB2-1 rev. 2 | **refused before any mutation**; the card's changed-size step no longer exists for mipped rasters (rev. 3 note) |
| Changed-size DXT replacement, single-level raster | as above | one level re-emitted; runtime acceptance still card R32-XB2-1 |
| Audit claim | "every large level verified; 2 borderline" | 9 185 large levels: 7 224 matched-decisive / 1 953 matched-nondecisive / 8 unmatched; tail 4 228 / 14 100 unmatched; RGB only; DXT1 / 3 / 5; 256² / 512² / 1024²; 8 titles |

## 6. Not done / limitations

- The packed-tail layout is not derived (research Q-R65-1; research handoff 02's H1 candidate is
  not consumed — the review's F1 for the research lane is open).
- The gate is by descriptor form, not title; NHL 12–15 and NHL 09 / 10 rasters are now in the
  audit population, `fe/` / `logo/` rasters and non-square shapes are not.
- The pre-Apply plan is the writer's own policy for Xbox 360 DXT rasters only; other containers
  report `checked: false` and their writer rules still decide at write time (the codec's own
  descriptor-rule refusals — tiny, cube, NHL 07 — are returned by the write, not predicted by the
  plan).
- Nothing here is game-verified; a same-size partial replacement's appearance is not observed.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad patch scripts (`patch_*.py`) and the `locprobe` cargo project | deleted at closeout (see STATUS §cleanup) |
| Temp roots `nhlms-c1-loose-f1-boundary-*` | removed on drop; verified absent |
| Evidence logs `r4-xb2-mip-placement-audit-restated.log`, `r4-closeout-regression.log` | retained |
| Background processes | none |

## 8. Next action

Item 2 of the dispatch — title-text keys on the song and arena cards ([receipt 02](02-r4-title-text-keys.md)).
