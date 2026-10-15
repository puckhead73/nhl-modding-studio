# Agent 2 — converter coordination and next assignment

This task manages Claude's converter assignments under `parallel-completion-plan-2026-09-16.md`. Claude remains the sole implementation writer. Codex owns planning and independent acceptance. This document assigns no Studio implementation work.

## Current disposition

- Read-only inspection confirms `E:\Projects\Repositories\nhl-head-mesh-converter` is on `codex/research` with no HEAD commit. Preserve the checkout.
- The explicit-input exporter handoff is received, not independently accepted. Its reported test totals and fixture parity have not been rerun by this task.
- Source inspection confirms that `export.py` exposes route inputs, while `build.py` and `audit_candidate.py` retain the old run-root restrictions. The native packer executable and import preconditions remain fixed in `build.py`.
- Legacy eye-evidence correction remains pending independent review. S7 is not authorized; no geometry candidate or readiness flag is promoted.

One active Claude package: H2.2 below. Codex's review queue is the explicit-input slice and Legacy correction, using the existing correction-review prompt. Next implementation after H2.2 review: export events/cancellation lifecycle. A Legacy repair can replace that next package only after an explicit review decision defining its scope. Versioned profiles, dependency/source baseline, remaining adapters and standalone delivery remain later completion work; Studio integration waits for converter completion.

## Prompt to give Claude

You are Agent 2, the sole implementation developer for the universal NHL head converter. Codex in the dedicated Agent 2 task-manager session assigns work and performs independent review. Another Claude session owns Studio.

Work only in `E:\Projects\Repositories\nhl-head-mesh-converter`.

Assignment H2.2: make the demonstrated guarded NHL21 export honor an explicit run root end to end, while preserving its native output and qualification boundaries.

Read `AGENTS.md`, the latest Claude/Codex status entries, `research/claude/app-export-explicit-inputs-2026-09-16-report.md`, `docs/CANONICAL_CONTRACT.md`, `src/application/export.py`, the existing native-export tests, and `experiments/codex/nhl21_package/build.py` plus `audit_candidate.py`. Use current source over historical ownership or next-step prose.

Confirm `codex/research` and record the starting source hashes for the files you touch. Preserve all existing work. Do not reset, clean, switch branches, stage, commit or push. Do not edit Studio, original assets, accepted packages or game installations. Studio dependencies are read-only; use isolated build outputs if needed.

The previous explicit-input slice is submitted for review, not independently accepted. Continuing this bounded application package does not accept that slice or the Legacy proposal.

### Implementation scope

1. Add explicit run-root, native packer executable and import-preconditions inputs through the exporter and existing native tools wherever consumed. Preserve demonstrated defaults. Distinguish the Python packer script from the native packer executable in CLI names and receipts. Narrow edits to the existing Codex native tools are explicitly assigned; do not build a replacement pipeline.
2. Validate the selected executable and preconditions before creating export output. Validate preconditions structure and target/donor agreement for the existing qualified route. Missing, malformed or incompatible explicit inputs must refuse cleanly; never silently fall back to defaults. Retain offline audit behavior without reading live archives.
3. Replace the historical location/name restriction with resolved containment beneath the selected existing run root and fresh-output enforcement. Apply the guard at the relevant direct tool entry points as well as the exporter. Preserve non-offline audit protections; do not broaden installation behavior. Keep current native asset names and the exact qualified donor/profile scope.
4. Record resolved inputs and hashes in the export receipt, including the native executable and preconditions. Check pinned input stability before successful completion. Restrict installation-plan artifact paths to regular files inside the native output directory; reject absolute, traversal, duplicate or escaped references. Publish the artifact list only after all required outputs validate; failure must not advertise completed deliverables.

Writable implementation scope: `src/application/export.py`, the existing `experiments/codex/nhl21_package/build.py` and `audit_candidate.py`, focused test files/helpers, and Claude's report/status/handoff. A small shared validation helper is allowed if needed. Preserve historical evidence and runs.

### Verification and exit

- Run focused exporter/native-tool tests and the existing application-interface regressions. Test missing/malformed/mismatched dependencies, existing destinations, containment escapes, artifact-path escapes and failures after packing. Verify refusal creates no export output and runtime failures retain an accurate failed receipt.
- Demonstrate both accepted Turgeon and Fedorov routes through the real tools under a fresh root outside `local/runs/codex`, but inside this repository, with a destination name independent of the old `nhl21-phase` convention. Pass the executable and preconditions explicitly. Do not write outside the repository to prove portability.
- Compare MCD/GNF bytes and native audit results with the preserved accepted-reference exports. Explain every receipt difference: implementation hashes and paths will legitimately change. Missing fixtures are an explicit coverage gap, never a pass. Preserve NHL19 references unchanged and report the checks actually performed.
- Record exact commands, tool versions, starting/final source hashes, output paths, changed files and remaining fixed dependencies. Separate structural parity from runtime acceptance.

Do not add progress events, redesign cancellation, implement the full profile schema, change geometry, start Legacy S7, run a broad discovery sweep, install anything in a game, or integrate Studio in this package. Events/cancellation are the next separately reviewed slice.

Return one compact implementation handoff with the result, tests actually run, parity evidence, failures/limitations and next useful action. Stop for this Codex task's review before committing or expanding scope. Legacy remains pending and all existing readiness flags retain their current values.
