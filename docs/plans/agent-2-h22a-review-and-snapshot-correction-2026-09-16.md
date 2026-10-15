# Agent 2 H2.2a review and final input-consistency correction

Disposition: original pre-write rejection defect closed; H2.2 input-consistency requirement still incomplete. Assign H2.2b below before events/cancellation. Legacy review and S7 authorization are unchanged.

## Independent verification

The six inspected source hashes match the handoff. Branch is `codex/research`. Reviewed `build.py` SHA-256: `a870b186762a850b095bb147ede0e5f98c7e704c573a8f447c3af845efc7003f`; test SHA-256: `d24f80d032ae7a2ab4d67156a99fad41fd26152d394a09b32c2f46a7432d5473`.

All 19 tests in `tests.adapters.test_native_export` independently pass, no skips, 21.276 seconds. This includes real Blender direct-entry rejection for wrong donor hash, wrong texture hash, missing texture target and duplicate donor target, plus fresh Turgeon/Fedorov parity exports. Used Blender's bundled Python 3.11 with temporary test directories redirected to the permitted review workspace, as in the prior review. No converter implementation files changed. Broad suites were not rerun because the change is confined to the packer and its regression.

## Remaining finding — captured inputs are not consistently consumed

The H2.2a prompt required reusing validated data so validation and execution cannot disagree. The new `pinned` dictionary correctly feeds output copies and Python profile/exchange parsing, but the handoff's broader no-second-read claim is not true:

- `build.py:45–46`: preconditions bytes are captured, then `load_preconditions(path)` rereads the file. Target validation can therefore use bytes different from those later copied and hashed in the receipt.
- `build.py:71`: donor bytes are reread from the original source instead of `pinned['native_donor']`.
- `build.py:105` and `:131`: native `pack` and `verify` receive original donor/profile paths, rather than the verified copies already written into the fresh output.
- `build.py:130`: texture bytes are reread from the original source instead of the captured texture.

These are source-confirmed reads, not a claim that a real fixture changed during this review. If a source changes after the initial hash verification, execution may use bytes different from the recorded copies or fail after output creation. Exporter's final stability check is useful, but direct `build.py` must also consume the verified inputs as assigned.

## Prompt for Claude — H2.2b

You remain Agent 2, sole converter implementation writer. Work only in `E:\Projects\Repositories\nhl-head-mesh-converter` on the existing `codex/research` checkout. Preserve everything; no reset, clean, branch switch, stage, commit or push. No Studio, original-asset, accepted-package or game-install writes.

Codex independently passed all 19 H2.2a tests and closes the original pre-write preconditions rejection defect. Finish the previously assigned validated-input reuse requirement:

1. Parse and validate import preconditions from the same captured bytes later copied and hashed. Preserve the existing path-based helper for other callers if needed; share structural validation rather than duplicating it.
2. Use captured donor and texture bytes throughout Python packing. Pass the verified donor/profile copies in the fresh output to native `pack` and `verify`. Preserve original source paths in provenance. Audit all consumers of the four exchange pins so no later original-source read undermines the captured inputs. Do not broaden this into a general dependency snapshot framework.
3. Add a focused regression that makes original exchange-pin inputs unavailable or different after capture, using disposable copies only, and verifies that downstream processing consumes the verified snapshot or refuses before consuming changed bytes. Assert native subprocess arguments use the verified copies. Include a preconditions test proving the validated bytes are those hashed and copied. Exercise the build path, not merely a helper disconnected from it.
4. Rerun the 19 focused tests plus new regressions and both fixture parity cases. MCD/GNF/native-audit parity must remain exact. Record legitimate receipt/path changes and full starting/final source hashes. Correct the handoff's no-second-read statement without overwriting historical evidence.

Writable scope: existing `build.py`, `src/application/native_paths.py` only as needed for byte-based preconditions validation, focused tests/helpers, and Claude report/status/handoff. Existing defaults and pre-write refusal behavior must remain. No events/cancellation, geometry changes, Legacy S7, new profiles or Studio integration. Stop for Codex review afterward.
