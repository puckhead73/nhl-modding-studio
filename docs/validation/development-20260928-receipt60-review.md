# Coordinator review — receipts 46–60, bounded receipt-60 checkpoint (28 September 2026)

## Result

**Correction required before accepting Hockey Shop recovery or expanding that writer.** `WriteTxn::rollback` deletes a newly created backup even when restoring its original file fails. This can discard the surviving on-disk original at precisely the point recovery needs it. The normal copied-install cases pass, but do not cover this failure path.

No blocking defect found in the inspected grouped-schedule and encoding paths. Their selected suites passed independently, including genuine NHL 20. This is bounded offline evidence, not acceptance of every receipt 46–60, all audio/asset routes, desktop rendering or game behavior.

Next dispatch: [development-20260928-after-receipt60.md](../plans/development-20260928-after-receipt60.md).

## Finding F1 — preserve recovery material when rollback fails

Location: `crates/studio-api/src/hockey_shop.rs`, `WriteTxn::rollback`, lines 658–670 at review time. After restoration returns an error, the code records it in `left` but still calls `remove_file(bak)`.

Independent reproduction used a standalone Rust harness extracting the exact `WriteTxn`, path and backup helper source from the reviewed file, with only a minimal `ApiError::Workspace(String)` stand-in. It wrote an original scratch file, overwrote it with backup enabled, confirmed `.bak` held the original, then replaced the destination with a directory to force restoration failure. Output: `rollback_failed=true backup_survives=false`. The harness's assertions passed, proving the unwanted backup deletion. This did not launch the application or exercise the whole install API; it isolates the actual extracted transaction code. No original/game file was touched.

Required correction: only retire the backup after successful restoration; preserve and name every usable recovery artifact on failure. Test failed restoration as well as ordinary rollback, create-mode removal and an existing backup. Avoid deleting an unowned destination that blocks recovery. Surface backup/temp cleanup failures instead of claiming all work was restored/removed. Review partial writes and transaction-owned temporary-file cleanup within this bounded correction. No claim of crash recovery follows from in-memory rollback.

## Executed independent checks

Environment: `CARGO_INCREMENTAL=0`; `NHL_STUDIO_FIXTURES=E:\nhl-studio-fixtures` for Rust suites.

- `cargo test -p studio-api --test hockey_shop_title_spec -- --include-ignored`: **2 passed**, including real NHL 12 sync and applied create/overwrite on disposable copies (23.53 s).
- `cargo test -p studio-domain --test schedule_grouped --test encoding_contract -- --include-ignored`: **14 passed** (encoding 10, 24.18 s; grouped 4, 9.16 s). Includes genuine NHL 20. The Recompiled spelling test reaches the encoding conflict guard; it is not a full project-open/edit/save test.
- Current `verify.py check` against actual r59 zip into reviewer-owned scratch: **passed**.
- Same archive with `check --release`: **expected refusal**, naming missing MIT holder/withheld LICENSE and 53 artwork files with unrecorded rights. No negative-archive selftest or helper PCM suite rerun this turn.
- Extracted transaction harness: **rollback defect reproduced**, as above.

Inspected receipts 46/47/48/59/60, the artwork and licence configuration, Hockey Shop plan/apply/fingerprint/transaction implementation, related registry routing, the grouped request/post-check and source-bound stock-group code, encoding alias guard/test, and release verifier sections. No source edits, full workspace tests, frontend/CI checks, app/game launches, listening, publication or commits. Prior independent checks are not counted as new tests.

## Other review conclusions

- Receipt 47 corrects receipt 45's 44-id claim to **41 actual pads plus three misclassified meshes**. Use the corrected denominator. Existing write tests now exercise actual installation; the failure-recovery claim remains blocked by F1.
- Recompiled spelling alignment is justified by the catalogue's `legacyrecomp` identity and the measured encoding guard. Add a real catalogue/project identity regression through public edits, imports/pending and copied save/reopen for representative affected fields; do not broaden roster ceilings or other routes by alias alone.
- Single-phase grouped generation passed the selected Swiss/Finnish/genuine-NHL-20 cases, including declared host and persistence checks. Austrian phased writing remains withheld. UI/game acceptance is separate.
- r59 contains receipts through 59. The archive passed integrity verification and correctly describes itself as an internal review candidate. The `release_ready` field is a packaging eligibility signal, not the complete project acceptance/release decision.
- MIT choice remains settled. The holder name is still pending from the user. Artwork rights are unrecorded, which is not itself a legal conclusion about each image. An asynchronous question requested provenance or approval to replace undocumented artwork; do not mark rows first-party or delete/replace user branding without the answer. Do not repeat the earlier MIT-choice question.
- The report's headline “two release items” does not eliminate other required scope: board replacement/re-key, phased schedules, PS4 head work and remaining target acceptance still need their current dispositions. The next prompt must preserve the full agreed beta blocker list rather than equating package eligibility with feature completion.

## Evidence and cleanup

Source/archive hashes and measured results: `development-20260928-receipt60-evidence.json`. Reproduction source retained at `docs/validation/evidence/coordinator-r60-rollback.rs` for the developer's correction dependency; it is a standalone reviewer harness, not application code. All reviewer executables, extraction trees and scratch targets under `local/runs/coordinator-r60-review` were removed after exact containment/ownership/reparse checks. Existing test guards clean their copies. Shared caches, user bundles, toolchain and unowned H:\TEMP directories were not removed.

## User answer received during this review

The user permits the current assets in internal research/team builds, to be replaced before external release. This supersedes the pending artwork-choice question above. The rights/provenance inventory remains unrecorded where appropriate; internal bundling is allowed, external-release replacement remains a task. Decision recorded in the beta scope document and next dispatch. No publishing/sending occurred.
