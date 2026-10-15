# Agent 2 — H2.4 accepted; H2.5 versioned export profile

## Acceptance

H2.4 including H2.4a is accepted for the demonstrated Windows/MSVC NHL21 export-tool build. The workspace-derived build-identity omission is closed. `identity_index` now feeds content identity, expected-manifest comparison, capture/snapshot validation and post-build drift checks; standalone manifests are generated from verified source bytes and captured workspace values. Version 1 expected manifests are explicitly rejected rather than silently upgraded.

Independent verification: all 15 `tests.adapters.test_native_build` tests passed in 19.030 seconds, no skips, including a real locked build and invocation of its executable. Review used Blender's bundled Python 3.11 and redirected temporary directories into the permitted review workspace. No converter source or Studio file was modified.

Reviewed SHA-256:

- `native_build.py`: `f44ed28ab69dbf5cc65c927f250d1a3f12be17c20148a4082899c42da9a7b03b`
- `test_native_build.py`: `d3ec8ae1896672b020cac5c17e88e2d53f25a1809c30b3f41660735352087189`

Independently rehashed preserved release-04 (`55b1da01b7c350c9c1ece3176f6fc735860a263c90d4119e78e6ec8bf6a8fe1e`) and release-05 (`388a642e98bcaf137f88550655de9467bd9b840235931b091a2e7130d32cd557`) executables against their built receipts. Both use source schema v2 and have empty drift reports; release-05 records successful expected-manifest matching. Both preserved `-h24a-fresh-01` exports have MCD/GNF/final-audit bytes equal to phase 11. Fresh compilation passed independently; fixture parity here was verified from Claude's preserved runs.

The acceptance concerns output reproducibility from recorded inputs, not executable identity across roots or standalone distribution. Studio's dirty local crate remains an external, content-pinned dependency. H2.2/H2.3 remain accepted. Keep the legacy exporter default unchanged; do not point it at whichever experimental build is newest. Legacy evidence correction remains pending independent review and S7 is not authorized.

## Claude prompt — H2.5

You remain Agent 2, the sole converter implementation developer. H2.4 is accepted. Implement one versioned application-facing export profile for the existing qualified NHL21 route, with explicit selection of a receipted native build. This is the sole active implementation package.

Work only in `E:\Projects\Repositories\nhl-head-mesh-converter` on existing `codex/research`. Read AGENTS.md, current statuses, the canonical contract, existing native profile and phase-3 handoff, accepted exporter, and H2.4 source/build receipts. Preserve all work: no reset, clean, branch switch, stage, commit or push. No Studio, original assets, accepted packages or game writes. Studio dependencies remain read-only.

### Scope and behavior

1. Implement and document a narrow `nhl-head-export-profile/1` contract plus validation for the demonstrated NHL21 PS4 Ekblad export route. It is an application export descriptor referencing the existing immutable native adapter profile, not a claim to implement the full proposed canonical `nhl-head-profile/1`. Document that mapping and remaining canonical gaps in Claude's contract proposals. Keep one profile, not a broad registry or plugin system.
2. The descriptor must carry stable profile ID/version, title/platform/title ID and known build attribution, supported source kind/strategy, the four required native input references with hashes, import-preconditions reference, native adapter identity, output expectations, and fixture-scoped validation/runtime evidence references. Preserve unknowns and existing limitations; no aggregate universal-support flag or invented build metadata. Profile identity and evidence may not be inferred from filenames alone.
3. Add an explicit exporter option to select this profile. Resolve references relative to the descriptor location or explicit documented bindings, independent of the caller's working directory. The new route must not require the historical `research/codex/nhl21-phase3-handoff.json` at execution time. Reuse the current native writer and validators; a small compatibility translation to its existing input shape is allowed after preflight. Never alter accepted profiles/fixtures to migrate them. Preserve the existing explicit-input/default CLI route as a compatibility path, with clear conflict rules instead of silently ignoring competing overrides.
4. Add explicit selection of a H2.4 build receipt for the native executable. Require a successful built receipt, valid v2 source-manifest reference and actual matching executable hash. Resolve references deterministically; no latest-build scanning, implicit build, or fallback to the old executable on an invalid explicit selection. Record the selected profile bytes/hash and build receipt/source identity/executable hash in the export receipt. Distinguish a locally validated build receipt from an independent review endorsement; it is provenance, not a signature.
5. Preserve preflight-before-output behavior, manifest/destination pin agreement, snapshot-based native packing, final input stability checks, native byte protections, events/cancellation and immutable completion. Wrong versions, unsupported routes, missing/changed pins, mismatched donor/profile identity or an invalid build selection must refuse cleanly before export output exists. Maintain native output names and the exact current donor scope.

### Demonstration and tests

- Add focused descriptor/build-selection tests for successful resolution, unknown schema/version/profile, malformed fields, missing/mismatched hashes, job/profile disagreement, failed/tampered build receipt or executable, and conflicting CLI options. Verify refusal does not create output.
- Demonstrate loading a relocated descriptor/input bundle from a different working directory using disposable copies, with originals untouched. Show the new execution path does not read the historical phase-3 handoff; use a controlled read guard or explicit input tracing instead of renaming that shared file.
- Export Turgeon and Fedorov through the new profile plus a validated build receipt. Require MCD/GNF/final-audit parity with phase 11; explain legitimate receipt changes. Re-run existing native-export/lifecycle tests because export input resolution is changing; include build-contract tests if their shared code changes.
- Keep runtime observations fixture-scoped. Deliver the schema/descriptor, runnable CLI examples, full changed-file hashes, exact tests and preserved run paths, remaining dependencies and canonical-profile gaps. Update Claude status/handoff with H2.4 acceptance.

Writable scope: exporter input/profile resolution and a small supporting module, a schema and one new export descriptor under appropriate converter directories, focused tests/helpers, and Claude documentation. Narrow compatibility edits to the existing packer are allowed only if required and must preserve output. No geometry changes, new donors/platforms, full canonical-scene rewrite, GUI, installer, Studio integration or Legacy S7. Stop for Codex review before committing or expanding scope.
