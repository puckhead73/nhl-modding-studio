# Agent 2 — H2.2 accepted; H2.3 export lifecycle assignment

## Review decision

H2.2, including corrections H2.2a and H2.2b, is accepted for the demonstrated guarded NHL21 export route. The original pre-write refusal defect and the assigned captured-input consumption defect are closed. This accepts the cumulative explicit-input implementation in its fixture-scoped form, not universal portability, distribution readiness or new runtime qualification.

Independent review confirms preconditions are parsed from their captured bytes; the four exchange pins feed the recorded copies and Python processing; native pack/verify use the verified donor/profile output copies. Candidate mesh/atlas bytes are captured at their artifact hash checks. The source path remains provenance rather than the native packer's input.

All 20 tests in `tests.adapters.test_native_export` passed independently in 25.062 seconds, no skips. They include four direct pre-write rejection cases, the real Blender tampered-source build followed by offline audit, and fresh Turgeon/Fedorov parity exports against phase 11. Tests used Blender's bundled Python 3.11; temporary directories were redirected to the permitted review workspace, without changing test or implementation source. The mutation harness proves successful consumption despite corruption after capture; it logs first-read mutations, not a complete count of every possible file access. No broader read-once claim is required for acceptance.

Reviewed branch: `codex/research`. Hashes match the handoff:

| File | SHA-256 |
|---|---|
| `build.py` | `d50e6fe6103551bcef3e0aeae58bdbaf97841bc466d6fc99d0b36ea13b2b3edc` |
| `native_paths.py` | `0aef054d1996cee19f745318512ce7afcbd3f4aeb98a804ae90f9aa84dab04c3` |
| `test_native_export.py` | `69d222063dd108bacf097384607b5da9e8bda6959f7bea871cd26b335c3547aa` |
| `blender_tamper_harness.py` | `c96a212cf8123f3cb9384df75ca549dc65f2fa438b5b0ec0adac11f8db3c3866` |

The prior independent 47 application-interface/worker/conversion tests remain earlier evidence, not a new rerun. No converter source changes, commits or installations were performed by this review. Legacy's eye-evidence correction remains pending; S7 is not authorized. NHL19 references gain no new runtime qualification.

## Prompt for Claude — H2.3

You are Agent 2, the sole converter implementation developer. This dedicated Codex task manages assignments and independent acceptance. H2.2 is now accepted for its demonstrated NHL21 route; the sole active implementation package is H2.3, export events and cancellation.

Work only in `E:\Projects\Repositories\nhl-head-mesh-converter`. Read `AGENTS.md`, current Claude/Codex status, this acceptance decision, `src/application/export.py`, the event/cancellation implementation in `src/conversion/job.py`, relevant existing lifecycle tests, and the canonical contract's job/cancellation requirements. Use current ownership over historical roadmap instructions.

Confirm `codex/research`; record starting source hashes. Preserve the unborn checkout and all untracked work. No reset, clean, branch switch, staging, commit or push. No Studio, original-asset, accepted-package or live-game writes. Existing Studio dependencies remain read-only.

### Deliverable

A caller can observe export stages, request cancellation during packing or audit, and receive a consistent terminal receipt without surviving export-owned child processes or completed artifacts advertised for an unsuccessful run. Preserve H2.2's explicit inputs, path guards, captured native inputs and byte-identical successful outputs.

1. Add a versioned, documented `events.jsonl` stream inside each fresh export directory. Use monotonic sequence numbers, an export/run identifier, elapsed time, event name, stage and status as applicable. Flush events while work runs. Report packing, auditing and final validation/publication boundaries with subprocess start/exit details. Stage-level progress is sufficient; do not invent percentage estimates. Reuse established worker event conventions where compatible, without silently changing that existing contract.
2. Add optional JSON-lines stdout events for callers; keep subprocess diagnostics in logs and preserve the existing default CLI result behavior. Define refusal, failure and cancellation exit behavior explicitly. Preflight refusal must still create no output directory; an optional stdout refusal event is sufficient there.
3. Support the established `CANCEL` file convention inside the export directory and KeyboardInterrupt. Check before launching each subprocess, poll during execution, and check again before final promotion. Define the completion/cancellation boundary so a request observed before completion prevents publication. Cancellation after a completed run does not rewrite it. Preserve logs, partial native files and existing evidence; failed/cancelled receipts must have an empty published artifact list.
4. Own and terminate the launched process tree on cancellation, then wait for cleanup before reporting cancellation. Packing can involve Blender plus the native packer: killing only Blender is insufficient. On Windows use a reliable mechanism scoped to this export's processes, with bounded cleanup and escalation; never kill by executable name or affect unrelated Blender/Python sessions. Preserve hidden/background invocation. If cleanup cannot be confirmed, record an explicit cleanup failure rather than a successful cancellation claim.
5. Keep receipt and event outcomes consistent: at most one terminal event on handled completion/failure/cancellation, final receipt persisted before its terminal event, no later stages after cancellation, and no completed status until existing validation/pin/hash checks finish. Do not promise a terminal receipt after an uncatchable process kill. Document abrupt-parent-death behavior; partial/running output must never count as completed. If the selected Windows process ownership mechanism supports cleanup on parent death, test it, but do not build a recovery daemon or resumable job framework.

Writable scope: `src/application/export.py`, a small application lifecycle/process helper if useful, focused tests/stand-ins, and Claude documentation. Inspect the worker for reuse; its current `_kill_active` only kills the immediate child and is not sufficient proof of tree cleanup. Do not refactor the entire worker. Changes to a shared helper require the relevant worker regressions. Native geometry/packing logic should remain unchanged.

### Verification and stopping conditions

- Use controlled stand-ins to test success, spawn failure, nonzero pack/audit exit, cancellation during pack, cancellation during audit, cancellation between stages and cancellation before publication. Assert event ordering, terminal uniqueness, receipt agreement, retained partial evidence and empty published artifacts on unsuccessful runs.
- Test a pack stand-in that launches a long-lived descendant. Verify both processes exit within the documented cleanup bound and stop writing after cancellation. Synchronize tests with explicit started/ready signals rather than sleep-only races. Check KeyboardInterrupt cleanup as well as CANCEL-file cleanup. Keep unrelated processes untouched.
- Retain all 20 H2.2 native-export regressions and rerun real Turgeon/Fedorov parity through the completed export path. Show the new event sequence for at least one real export. Runtime acceptance remains separate.
- Run existing application-interface/worker tests if shared behavior changes. Report missing fixtures and any platform behavior not exercised; do not count them as passes.
- Provide exact commands, tool versions, full source hashes, changed files, event examples, cancellation/process-cleanup evidence, parity results, and remaining lifecycle limitations. Keep the report compact and update Claude status/handoff with the H2.2 acceptance reference.

Non-goals: GUI, Studio integration, new profiles/adapters, geometry changes, Legacy S7, broad discovery, distribution packaging, resume/restart infrastructure or live installation. Stop if reliable process ownership requires a larger architecture change; return the concrete blocker and smallest proposed adjustment. Otherwise finish this bounded package and stop for Codex review before committing or expanding scope.
