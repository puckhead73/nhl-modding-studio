# Overnight development 2026-09-21 — coordinator review R1

Focused independent review, 2026-09-21. This is not acceptance of the entire campaign or a rerun of its private-fixture sweeps. No production code changed, game launched, or installation written. NHL 21's previously user-accepted E1 remains accepted.

## Disposition

The P0 colour settlement correction passes this focused source/test review. Further campaign acceptance is withheld for the affected resize and team-identity UI paths below. Other packages remain developer-verified pending their relevant review; absence of a finding here is not full acceptance.

## F1 — P1: resize writers silently truncate dimensions

`crates/tdb-rx2/src/gnf.rs`, `replace_resized`, masks requested dimensions into 14-bit fields without first rejecting dimensions that cannot be represented. `crates/tdb-rx2/src/texlib.rs`, `patch_descriptor_for_resize`, does the equivalent for 13-bit dimensions. Checking the original descriptor does not validate the requested output.

Independent in-memory regressions failed against the delivered code:

- GNF: construct a valid 32×32 BC1 source with `encode_gnf`, set texture type to 9, pitch to 32, and stream/data lengths consistently. `check_resize_rule` accepts it. Request one 16400×4 level containing 32800 bytes. `replace_resized` returns `Ok`, but the output header records width **16**.
- Xbox descriptor: use a 56-byte descriptor with big-endian dw0 `0x00400003` and dw7 `0x80000000`. Request 8196×128 DXT1, one mip through `patch_descriptor_for_resize`. It returns `Ok`, but dw9 records width **4**.

These demonstrate malformed output from the codec APIs, not observed game corruption. Validate representability before allocation/encoding: width, height, pitch, mip count, size fields and checked size arithmetic. Do not silently clamp or mask invalid requests into another value. Reparse successful output and verify requested dimensions/mips and computed payload relationships. Cover valid boundaries and first invalid values on both axes, plus refusal through the public editing route with bytes/history/pending state unchanged. Apply the same audit to the other newly enabled resize formats without claiming an unproven defect in them.

## F2 — P2: failed art-ID swap leaks into another project

`app/src/components/TeamIdentityCard.tsx`, mutation `onError`, checks the team id but not the captured issuing project/generation. The success path's ownership checks do not protect failure callbacks.

Independent deferred-promise test reproduced: mount project 7/team 3, choose team 4 and dispatch swap; switch the same component to project 8/team 3; wait for project 8's dependency query; reject the original swap with `project-seven-only-error`. The project 8 card displays that error. The expectation of no error fails.

Carry immutable dispatch identity through success, failure, settlement and summary refresh. Test cross-project same-team failure, switching away and back, unmount, successful completion and failed summary refresh. Assert exact dispatched arguments and UI ownership. Do not repair only the visible comparison while leaving callbacks dependent on later render values.

## Independent checks

- `cargo test -p tdb-rx2 --lib resize`: 2 passed, 4 ignored.
- `cargo test -p tdb-archive-big --lib big4`: 14 passed.
- `cargo test -p tdb-archive-codec --lib chunklzx`: 2 passed.
- Focused existing ColorField, TeamIdentityCard and TextureReplace Vitest suites: 33 passed.
- Reviewer resize tests: 2 failures reproducing F1.
- Reviewer TeamIdentityCard copy: 3 existing cases passed; new failure reproduces F2.

51 existing focused tests passed; four private tests were ignored. This does not independently confirm the campaign's aggregate, descriptor censuses, real-archive sweeps or runtime behavior. Temporary reviewer probes are removed after recording these reproductions.

## Evidence and ownership discipline

Shipped-descriptor reproduction is valuable format evidence; it is not equivalent to changed-dimension end-to-end replacement or loader acceptance. Preserve those distinctions in both matrices and the morning cards.

The campaign describes removing old test binaries from a shared cache by age. Age is not ownership. Future cleanup must use an explicit package-owned artifact manifest; do not remove another lane's or the shared cache's files merely because they are old. Do not attempt to undo this by rebuilding everything.

## Fresh developer dispatch — OVERNIGHT-DEV-20260921-R1

Read repository instructions, this review, receipts 01, 06, 11 and 15 of `docs/receipts/overnight-development-2026-09-21/`, and the current matrices. Preserve the dirty working tree and all unrelated lane files. This is a bounded correction package; do not begin new roadmap features until it is delivered for coordinator review.

1. Reproduce F1 and F2 with permanent failing regressions before fixing them. Record the red evidence concisely.
2. Fix output representability and checked arithmetic at the authoritative codec boundary. Trace the public resize route and prove refusal leaves existing edits and source bytes intact. Retain existing format/layout guarantees.
3. Fix all TeamIdentityCard completion ownership paths using immutable request context; add deferred-result tests for the contexts above. Preserve the successful swap's owning-project refresh and one-operation undo behavior.
4. Run affected codec/domain/API tests, affected UI suites, TypeScript, and bindings verification if signatures change. Run meaningful changed-dimension round trips in addition to unchanged shipped-descriptor checks. Do not substitute repeated green runs for investigating a captured failure.
5. Reconcile only the coverage claims affected by these fixes. Keep runtime cards deferred for the user; do not reopen the accepted NHL 21 E1 test. No live writes, launches, commits, staging or pushes.
6. Clean only this package's manifest-owned temporary artifacts after preserving useful evidence. Do not clean shared caches or other lanes' files. Finish with a plain-language summary, exact changed paths, regression evidence, checks/skips, remaining limitations and cleanup manifest; stop for coordinator review.

## Roadmap snapshot

Leagues have substantially broader offline implementation, including reflow/cascade, localization, assets and commentary inspection, with remaining format/consumer and runtime gates explicit. Teams have identity/name/asset work underway; art-ID swap is a constrained operation, not unrestricted identity reassignment. Players have recorded-name listing; equipment has file containment checks. These are partial Phase 3/4 slices, not phase completion. Miscellaneous work remains largely research-led; Projects/Builds remains the final stretch feature.
