# Agent 2 H2.2 review — bounded correction required

Disposition: return for H2.2a; do not begin export events/cancellation yet. Legacy eye-evidence correction remains independently unreviewed, S7 unauthorized, and readiness unchanged.

## Finding

P2: the direct native packer checks target agreement after producing output. In `E:/Projects/Repositories/nhl-head-mesh-converter/experiments/codex/nhl21_package/build.py`, output creation is line 47, `validation.json` is written at line 141, and `preconditions_target` first runs at line 150. The exporter performs this check before writing, but the assigned direct-entry-point boundary does not.

Independently reproduced through Blender 4.5.5 with the real Turgeon handoff and a copied preconditions file whose first target's decoded hash was replaced with 64 zeroes. Exit code was 1, but the output directory contained the MCD, GNF, mip outputs and `validation.json`. Expected: reject the mismatched target before creating the output directory or starting native packing.

Review evidence is preserved at `C:/Users/puckh/.codex/visualizations/2026/09/17/01a0ad52-2750-7971-bfb3-6c5ca7e6edad/h22-review-e0u_yby7/review.json`. That receipt records the original execution location; the complete scratch directory was subsequently moved into this review workspace. No converter source, accepted fixture or live game was modified.

## Verified evidence

- Branch remains `codex/research`. All six source hashes match the report's abbreviated final hashes.
- Independently rehashed both preserved H2.2 runs against their phase-11 files: MCD, GNF and `final-audit.json` are byte-identical for Turgeon and Fedorov.
- All 18 native-export tests passed independently, including fresh real-tool exports of both fixtures. Review used Blender's bundled Python 3.11 and redirected temporary test directories into the permitted review workspace. Initial unmodified test runs were stopped after a traceback located a sandbox-related retry loop in `tempfile.mkdtemp`; those interrupted runs are not test passes.
- The 47 application-interface/worker/conversion tests also passed: combined run 65 tests, 37.439 seconds, no skips. Modules: `tests.adapters.test_native_export`, `tests.conversion.test_app_interface`, `tests.conversion.test_worker`, `tests.conversion.test_conversion`.
- Structural parity does not add runtime acceptance. The reported 243 Legacy/other-adapter tests have not been rerun in this review.

## Claude prompt — H2.2a

You remain Agent 2, sole converter implementation writer. Work only in `E:\Projects\Repositories\nhl-head-mesh-converter`, on the existing `codex/research` checkout. Preserve all work. Do not reset, clean, switch branches, stage, commit or push. No Studio, original-asset, accepted-package or live-game edits.

H2.2 review requires one bounded correction before acceptance: direct `build.py` validates import-preconditions target agreement only after writing its native outputs and `validation.json`. Codex reproduced this using the real Turgeon fixture and a structurally valid preconditions file with a wrong donor hash; packing completed before the final ValueError.

1. Move exchange/profile pin verification and the existing donor/texture preconditions-target agreement checks before the first output-directory creation and before invoking the native packer. Validate against the same pinned profile used for packing. Reuse the validated data; avoid separate reads that could make validation and execution disagree. Preserve native behavior, default paths and existing protections.
2. Add a direct `build.py` regression covering wrong donor hash, wrong texture hash, missing required target and duplicate required target. Each must refuse before output exists and before the native executable runs. Test through the real entry point; an exporter-only or helper-only test does not cover this finding. Keep fixture absence explicit.
3. Rerun focused native-export tests and both accepted-fixture parity cases. Confirm unchanged MCD/GNF/native audit results. Do not repeat unrelated broad suites unless changes justify it.
4. Report full starting/final SHA-256 values for touched files, exact commands and results, and the reproduction outcome. Update Claude's report/status/handoff; do not rewrite historical evidence.

Scope: existing `build.py`, a shared helper only if necessary, focused tests/helpers and Claude documentation. No events/cancellation implementation, geometry work, Legacy S7, schema redesign or Studio integration. Stop for this Codex task's review after the correction. The subsequent events/cancellation package remains queued.
