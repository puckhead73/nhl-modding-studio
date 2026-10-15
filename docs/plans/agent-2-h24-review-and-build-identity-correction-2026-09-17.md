# Agent 2 — H2.4 review and H2.4a correction

Disposition: one build-identity correction required before H2.4 acceptance. H2.2/H2.3 remain accepted. Keep the exporter's accepted default executable unchanged. A later profile/tool selection should reference an accepted build receipt explicitly, rather than hardcoding a new experiment directory or choosing the latest build implicitly.

## Independent evidence

All 11 `tests.adapters.test_native_build` tests independently passed in 15.123 seconds, including a real locked Cargo build from the snapshot and executable invocation. Used Blender's bundled Python 3.11 with temporary test directories redirected to the permitted review workspace. No selected tests skipped; no converter source or Studio writes were made by the reviewer.

Verified changed-file hashes:

- `native_build.py`: `402a6436d31aabaee66d477c241819db1f7865c25bd02052c1f63036b4b88d15`
- `test_native_build.py`: `b1125ebeb824b8fa30b1133f62a48d7ff63116874dc39e611eaa118cc7b8d1ab`

Independently rehashed the preserved release-03 executable: `3a5ed830b1c560a21344dd8d76bc44465b08fc4ab85b5af61a0b22184b24eb66`, matching its built receipt. Both preserved `-h24-fresh-02` exports have MCD/GNF/final-audit bytes identical to phase 11. This verifies Claude's preserved parity evidence; the independent real-build unit test built and invoked another fresh executable but did not itself re-export those fixtures. No bit-identical-across-roots claim is made.

## P2 — inherited workspace inputs are recorded but excluded from identity checks

`capture_dependency()` records `workspace_root_sha256` and `resolved_package_fields`. However, `manifest_content_hash()` and `compare_manifests()` include only converter/dependency file lists and the lockfile. The workspace manifest and resolved values are omitted, even though they determine the generated dependency Cargo.toml. The post-build drift check reuses that incomplete comparison.

Deterministic reproduction using a disposable dependency:

1. Capture a source manifest with workspace-inherited edition 2021.
2. Change only the disposable workspace root Cargo.toml to edition 2024.
3. Run snapshot-only with `--expect` pointing to the first manifest.

Observed: `status: snapshot`, `expected_manifest.matched: true`, identical `content_sha256`, different workspace hash, different resolved edition, and different generated dependency manifest hash. Expected: refuse before output creation because a build-defining input changed. The same omission can hide workspace drift during a real build.

Probe receipt and both snapshots: `C:/Users/puckh/.codex/visualizations/2026/09/17/01a0ad52-2750-7971-bfb3-6c5ca7e6edad/h24-review-mifwjrkq/review.json`. No real Studio files were changed for this probe.

## Claude prompt — H2.4a

You remain Agent 2, sole converter implementation writer. Work only in `E:\Projects\Repositories\nhl-head-mesh-converter` on existing `codex/research`. Preserve everything; no reset, clean, branch switch, staging, commit or push. Studio and all external dependency trees remain read-only. Preserve accepted executables, packages and game assets.

Codex passed all 11 H2.4 tests, including a fresh locked build, and verified release-03 plus both preserved parity exports. Close the build-identity omission:

1. Include all workspace-derived build inputs in the source identity, `--expect` comparison and post-build drift checks: the actual workspace manifest hash when used, inherited-field declarations, and resolved values that feed generated manifests. Keep identity independent of the workspace's absolute location so identical relocated inputs can match. Do not silently accept an older expected manifest that lacks required identity fields; document the schema/compatibility behavior.
2. Generate both standalone Cargo manifests from verified source bytes and the captured, verified workspace values. Check the source manifests even though their generated outputs differ intentionally. A change between capture and snapshot must not bypass validation merely because a Cargo.toml will be rewritten. Keep snapshot-only behavior subject to the same input-consistency requirements.
3. Add regressions changing only workspace edition or rust-version: `--expect` must refuse without creating output. Cover workspace mutation between capture and snapshot and during a stubbed build; neither may report matched/built against the wrong inputs. Cover identical relocated dependency/workspace contents retaining the intended identity. Use disposable fixtures only.
4. Rerun the build-contract suite including a fresh real locked build. Confirm a valid unchanged expected manifest still passes. Re-export both accepted fixtures with the freshly built executable and retain native parity. Report full hashes, commands and legitimate receipt changes.

Scope: `src/application/native_build.py`, focused tests/helpers and Claude report/status/handoff. Do not change exporter defaults, lifecycle or native algorithms. No broad build-system rewrite, new profiles, GUI, Studio integration or Legacy S7. Keep the exporter default at the H2.2-accepted executable; explicit `--packer-exe` remains the route for reviewed fresh builds. Stop for Codex review after the correction.

After H2.4 acceptance, the next application package remains versioned profile consolidation. Legacy evidence review remains separately pending; this package does not authorize geometry repair.
