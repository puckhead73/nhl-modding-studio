# Agent 2 — H2.3a review; H2.3b cleanup correction

Disposition: the demonstrated completion rollback and unowned-launch defects are closed. H2.3 still needs the exceptional descendant-cleanup correction below. H2.2 remains accepted. Legacy evidence review remains pending; S7 is not authorized.

## Independent checks

All four changed-file hashes match the H2.3a handoff. Independently ran 45 selected tests in 54.557 seconds, all passing: 25 lifecycle tests and all 20 native-export tests, including real Turgeon/Fedorov parity. Excluded only the deliberately invalid-EXE spawn test that stalled in the previous review environment. The new deterministic process-creation-failure test passed. Used Blender's bundled Python 3.11 and redirected temporary test directories into the permitted review workspace; no converter source edits. Worker tests were not rerun.

The tests independently exercise committed-receipt preservation, injected ownership failures, suspended launch, cancellation with descendants and normal lingering-descendant cleanup. These are meaningful fixes; the remaining defect concerns exceptions inside the cleanup path itself.

## P1 — exception during descendant drain leaves a live owned tree

`src/application/export.py:208` calls `proc.wait_tree()` after the immediate child exits. If that raises KeyboardInterrupt or OSError, execution skips `proc.close()` at line 211. The outer handler at line 279 only cleans up when the immediate child's `poll()` is None, so it skips this still-owned job. The cancellation receipt then reports `confirmed: true` and `no subprocess was running` at line 290 despite a surviving descendant.

Reproduced twice using existing `pack_orphan` stand-in with a live heartbeat descendant, injecting one exception from `wait_tree()`:

| Injection | Receipt | Immediate child | Descendant after export returns | Job handle |
|---|---|---|---|---|
| KeyboardInterrupt | cancelled; cleanup incorrectly confirmed | exited | alive | open |
| OSError (job query failure) | failed | exited | alive | open |

The reviewer explicitly terminated and drained the probe jobs afterward. Probe receipt: `C:/Users/puckh/.codex/visualizations/2026/09/17/01a0ad52-2750-7971-bfb3-6c5ca7e6edad/h23a-review-gl6u87mk/review.json`. No game or accepted fixture was involved.

## Prompt for Claude — H2.3b

You remain Agent 2, sole implementation writer. Work only in `E:\Projects\Repositories\nhl-head-mesh-converter` on existing `codex/research`. Preserve everything; no reset, clean, switch, stage, commit or push. No Studio, originals, accepted packages or live-game writes.

Codex independently passed 45 selected H2.3a tests and closes the prior completion rollback and unowned-launch findings. Finish one bounded exceptional-cleanup correction:

1. Make subprocess ownership cleanup exception-safe across polling, descendant draining, accounting queries and subprocess-exit event delivery. An owned job requires cleanup even when its immediate child has exited. Use unconditional resource finalization so an exception in `wait_tree()` or event delivery cannot bypass job closure. Preserve the initiating failure/cancellation and record the actual cleanup result.
2. On KeyboardInterrupt or query failure while descendants may still exist, terminate the owned tree, attempt bounded confirmation, and always release owned handles. If emptiness cannot be confirmed, report cleanup failure; never substitute `no subprocess was running` based only on the immediate child. Apply this truthfully to both failed and cancelled runs. Keep the accepted completion commit boundary intact.
3. Add deterministic regressions using `pack_orphan`: interrupt during descendant drain; job-accounting OSError during drain; and event-delivery failure after immediate-child exit. Confirm no descendant continues heartbeats after export returns, handles close, no later stage starts on failure, artifacts stay empty, and receipt cleanup agrees with measured termination. Cover persistent confirmation failure separately: terminate/close still happens, but the receipt must not claim confirmed cleanup.
4. Rerun lifecycle/native-export tests and both real fixture parity cases. Keep the deterministic spawn-failure coverage; do not spend time diagnosing Codex's invalid-EXE test environment. Report full changed-file hashes and exact results, preserving historical evidence.

Related reporting detail to correct while touching lifecycle finalization: `EventLog.terminal` denotes a terminal event claimed before the write, yet the sidecar uses it as `terminal_event_written`. Distinguish attempted/claimed from successfully delivered (or unknown on interrupted I/O); a failed stream write must not be reported as a delivered terminal event. Add a focused terminal-write failure assertion. This does not change the authoritative completed receipt.

Scope: `src/application/export.py`, `src/application/lifecycle.py` only as needed, focused lifecycle tests/helpers, and Claude report/status/handoff. No worker rewrite, profile expansion, Legacy geometry, UI, installation or unrelated hardening. Stop for Codex review afterward.
