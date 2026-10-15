# Receipt 01 (P0) — Correction checkpoint: the texture write is bound to the plan the operator saw

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (unchanged; the working
tree was already dirty from the 18–23 September campaigns and is preserved). Nothing staged,
committed, pushed, launched or written live; every write below happened in a disposable temp root.
Dispatch: [the 24 September coordinator review](../../validation/development-20260924-disclosure-venue-review.md),
F1, and the overnight brief's P0. Claim: `dev-p0-reviewed-texture-write`
(`local/coordination/overnight-2026-09-24/claims/`). Clock: started 04:54, checkpoint 05:20 (MDT).

## 1. Plain-language summary

The coordinator reproduced a real gap: the Replace… dialog showed a plan for a file, the file at
that path changed to a different kind of raster, and the write still went through — now as a
*partial* replacement (seven small "packed-tail" mip levels kept the old image) that the operator
had never been shown. Rechecking whether the write was *allowed* did not help, because the new
operation was also allowed; it was just a different operation.

The plan is now a **server-held review**. When the dialog asks what a write would do, the backend
reads the target and the source itself and remembers their exact bytes (a hash), the raster, the
selection (install root and path) and the policy it displayed. The write must name that review.
Before any backup or mutation the backend reads the target and the source **once more, once**,
compares them to the review, recomputes the policy from those very bytes, refuses on any
difference, and hands those same bytes to the writer. A refused write leaves the file and its
backup exactly as they were, and the dialog re-plans so that a fresh review is *displayed* — it
never re-plans and writes in one step. A review is spent by the write that names it, so a second
click cannot reuse it. The nameplate editor's Save (which bakes its own image) now goes through the
same two steps and shows the same disclosure before its Write.

The reproduction is recorded red (`p0-f1-red.log`) and green (`p0-f1-green.log`).

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-api/src/registry/texture_review.rs` (**new**, sha256 `e0ef4f60…`) | the review state (bounded to 8 held reviews), `texture_replace_plan` / `texture_replace_plan_bytes` (plan + hold), `import_texture` / `import_texture_bytes` (take the review; check install root, path, raster, resolved target, target length + sha256, source path + sha256 or held image, dimensions; recompute the policy from the consumed bytes; compare the writer's notes to the reviewed notes; then backup + write) |
| `crates/studio-api/src/registry.rs` | the three old functions removed; `texture_reviews` state field; `mod texture_review` |
| `crates/studio-api/src/dto.rs` | `TextureReplacePlanDto` + `review_id: u64`, `source_width`, `source_height` |
| `crates/studio-api/src/commands.rs`, `src/lib.rs` | `import_texture(+review_id)`, `import_texture_bytes(review_id, no PNG)`, `texture_replace_plan(source_image_path)`, **new** `texture_replace_plan_bytes` registered |
| `app/src/bindings/TextureReplacePlanDto.ts` | regenerated (`--write`); `--check` in sync at 148 roots |
| `app/src/lib/client.ts` | the four command shapes |
| `app/src/components/TextureReplace.tsx` (sha256 `e8950ce6…`) | the plan query sends the source path (backend decodes); Write passes `planData.review_id`; a write error re-fetches the plan (fresh review displayed, Write disabled while fetching, never auto-written); **new** `useReviewedBytesWrite` hook + `ReviewedPlanNotes` for the bytes route |
| `app/src/components/NameplateEditor.tsx` (sha256 `c308b133…`) | Save = bake → `texture_replace_plan_bytes` → footer shows refusal / partial-replacement notes / "no pre-check" → **Write texture** names the review (with **Bake again**); a refusal never offers Write; errors drop the review |
| `crates/studio-api/tests/texture_review_binding.rs` (**new**, sha256 `c01a1ed8…`) | 8 cases, §4 |
| `crates/studio-api/tests/loose_texture_route.rs`, `tests/registry.rs`, `tests/gnf_real_resize.rs` | callers go through `reviewed_import` (plan, then write under that review); the F1 boundary case plans from real source files; the stale-plan case now asserts the review's drift refusal (before any writer rule) |
| `app/src/components/TextureReplace.test.tsx` | plan mocks carry `review_id` / source dims; write args assert `reviewId`; **new** drift case for the dialog and 3 cases for the bytes hook |

## 3. Reproduction (the coordinator's steps, now red → green)

Red, previous route (`p0-f1-red.log`): plan for the 512² single-level raster → `SHOWN []`;
file replaced at the same path by the 10-level raster; `import_texture` → **succeeds**,
`APPLIED ["Partial replacement (Xbox 360 mip chain): …7 packed-tail levels…"]; BYTES_CHANGED true`.

Green (`texture_review_binding::f1_an_allowed_to_allowed_policy_change_refuses_and_leaves_target_and_backup_alone`):
the same steps refuse with *"The reviewed write was not performed: the texture file … changed since
the plan was shown … Nothing was written and no backup was made; review the replacement again"*;
target bytes equal the replaced file, no `.bak`; a fresh plan shows the partial-replacement note
under a new review id, and only that review writes; the result repeats the reviewed note word for word.

## 4. Regressions (all developer-run, all green)

| Required by the brief | Case |
|---|---|
| allowed→allowed policy drift | `f1_an_allowed_to_allowed_policy_change_refuses_…` |
| allowed→refused drift | `an_allowed_to_refused_change_refuses_from_the_review_not_only_from_the_writer` (+ writing with a review that is itself a refusal refuses with that reason) |
| same-path source replacement, unchanged dimensions (+ removed source) | `a_source_replaced_at_the_same_path_with_unchanged_dimensions_refuses` |
| unchanged-plan success; repeated dispatch; refusal preserves target and backup | `an_unchanged_plan_writes_once_and_the_consumed_review_cannot_write_again` |
| target / raster / project (install root) / source selection changes; unknown + evicted review | `a_review_is_bound_to_its_target_raster_install_root_and_source` |
| bytes route (nameplate): review holds the image; drift; cross-route misuse; refusal; repeat | `the_bytes_route_reviews_the_baked_image_and_refuses_drift_and_cross_route_use` |
| containers without a pre-check keep their route, not as a bypass | `a_container_without_a_pre_check_still_writes_under_its_review_and_still_refuses_target_drift` (GNF) |
| failed plans hold no review | `a_plan_that_cannot_read_its_inputs_holds_no_review` |
| dialog: loading / failure / retry / cancel / duplicate dispatch / switched source / same-path re-open | the 13 pre-existing vitest cases, migrated |
| dialog: the write names the displayed review; a drift refusal re-plans, displays the fresh review, dispatches nothing on its own; the next Write names the new id | `names the displayed review in the write, and after a drift refusal re-plans …` |
| bytes hook: stage → disclosure → one write under the id, spent review; refused review never writes; drift drops the review; failed pre-check; superseded stage; target change | the 3 `useReviewedBytesWrite` cases |
| existing partial-write masks and changed-size mipped Xbox refusals | `loose_texture_route::a_changed_size_write_into_a_mipped_xbox_raster_…` unchanged in substance, green |

## 5. Checks run

[`p0-f1-green.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p0-f1-green.log):

| Run | Result |
|---|---|
| `cargo test -p studio-api --test texture_review_binding` | **8 passed** |
| `cargo test -p studio-api --test loose_texture_route` | 3 passed, 5 ignored (private fixtures) |
| `cargo test -p studio-api --test registry` | 7 passed |
| `cargo test -p studio-api --lib` (incl. the bindings drift test) | 31 passed |
| `export-bindings --check` | in sync, 148 roots |
| `npx vitest run` (full) | **31 files, 213 passed** (TextureReplace 17: 13 migrated + 4 new) |
| `npx tsc --noEmit` | clean |
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test loose_texture_route --test gnf_real_resize -- --ignored` | see [`p0-f1-private-fixtures.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p0-f1-private-fixtures.log) (§7) |

`zz_diag` was not invoked. Pre-existing repo-wide fmt / clippy failures (C0.2) are untouched and
not re-measured here.

## 6. Decisions and limitations

- **Review, not signature.** The review is held in the registry (like the disc-image plan of R2 F2)
  rather than encoded in a token: it also holds the nameplate editor's baked image, so the bytes
  route transfers the PNG once and cannot be handed a different image than the one reviewed.
- **Consumed on use.** A review is removed by the write that names it, success or refusal. A
  refused write therefore always forces a fresh displayed plan. At most 8 reviews are held; older
  ones are evicted and refuse as "not current".
- **The redundant policy comparison is deliberate.** With equal target bytes, equal source and
  equal raster the recomputed policy cannot differ; it is compared anyway (and the writer's notes
  against the reviewed notes) so the contract "the result repeats the plan" is enforced, not assumed.
- **What is not bound:** the review does not detect a target replaced *after* the write's read —
  there is no lock; the read–compare–write is not atomic against a concurrent external writer
  (the same limit the disc-image route states). The dialog's automatic re-plan after a refusal is
  a fresh *display*; no write follows without a click.
- **Xbox 360 packed tail** stays as before: same-size writes into mipped rasters are disclosed
  partial replacements; changed-size writes refuse. H1 remains a candidate.
- No desktop run: the dialog and the nameplate footer are jsdom-verified only (cards in
  `MORNING-REVIEW.md`, V-45 / V-46).

## 7. Private-fixture sweep

Recorded in `p0-f1-private-fixtures.log` when the background run finishes; summarized in
`STATUS.md`. A case that fails for a missing fixture is a failure, not a pass.

## 8. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `patch_p0_registry.py`, `patch_p0_tests.py`, `patch_p0_frontend.py`, `patch_p0_fe_tests.py` | kept until the campaign closeout receipt is written (they document the mechanical edits), then deleted |
| Temp roots `nhlms-review-*`, `nhlms-c1-loose-*`, `studio_api_import_texture` | removed by the tests' `Drop` / explicit cleanup |
| Background processes | the private-fixture cargo run (§7) — finishes on its own; none left at closeout |

## 9. Next action

P1: the League / Team completion ledger from the two coverage matrices and the code (brief P1),
then the first P2 package it selects.
