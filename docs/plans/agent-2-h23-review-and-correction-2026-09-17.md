# Agent 2 — H2.3 review: correction required

H2.3 is not accepted yet. H2.2 acceptance remains intact. Assign H2.3a below; versioned profiles and dependency delivery remain queued. Legacy evidence review remains pending and S7 is not authorized.

## Findings

**P1 — committed completion can be rewritten, contradicting the terminal event.** `export.py:235` persists a completed receipt through `end()`, but that call and the terminal event remain inside the broad exception handler. Injecting one KeyboardInterrupt immediately after publish's `stage_finished` rewrites the already completed receipt to cancelled and clears its artifacts. Injecting the interrupt immediately after `export_completed` has been written produces a cancelled receipt alongside an `export_completed` event, then raises `RuntimeError: event export_cancelled after terminal event export_completed`. The existing late-CANCEL test only writes the marker after `export()` returns and misses this boundary.

**P1 — Windows process ownership fails open and starts too late.** `lifecycle.py:138–149` catches Job Object creation/assignment failures, then permits execution with `tree_owned: false`. Injecting a Job Object creation failure allowed both stages to run and export to complete with four published artifacts. In addition, the child runs between Popen and assignment; this interval is scheduler-dependent, not bounded to microseconds. A child can spawn descendants before ownership, or the exporter can die before assignment. Job accounting cannot confirm cleanup of escaped descendants. Documenting this limitation does not satisfy the assigned owned-tree cancellation boundary.

Constructor failure paths also need resource cleanup: a job is created before Popen but is not closed if Popen raises. Treat this as part of the same ownership lifecycle correction.

Independent deterministic probe receipt: `C:/Users/puckh/.codex/visualizations/2026/09/17/01a0ad52-2750-7971-bfb3-6c5ca7e6edad/h23-review-zk0lcv46/review.json`. Probes use stand-ins and temporary files only. No converter source or accepted package was changed.

## Evidence

The four changed-file hashes match the handoff. Independently rehashed both preserved real H2.3 runs against phase 11: MCD, GNF and native audit are identical. Each preserved successful run has 14 events ending in `export_completed`. The preserved real cancellation receipt records cancelled, empty artifacts and zero active job processes; this is inspection of Claude's recorded run, not a new real-Blender cancellation.

Review tests use Blender's bundled Python 3.11 and redirect temporary directories into the permitted review workspace. The first combined run stalled at `test_spawn_failure_is_a_failed_receipt` while attempting to execute the deliberately invalid EXE; it was stopped. This case is not counted as independently passing. A bounded rerun excluding only that case passed all 31 selected tests in 39.139 seconds: 11 lifecycle cases plus all 20 native-export cases, including fresh real parity exports. No selected test skipped. The 47 worker/application tests are Claude-reported for H2.3, not rerun here.

## Prompt for Claude — H2.3a

You remain Agent 2, sole converter implementation writer. Work only in `E:\Projects\Repositories\nhl-head-mesh-converter` on existing `codex/research`. Preserve all work; no reset, clean, branch switch, staging, commit or push. No Studio, original assets, accepted packages or game installations may be modified.

Close these two H2.3 lifecycle defects as one bounded correction:

1. Establish one explicit completion commit point. After the completed receipt is atomically persisted, later KeyboardInterrupt, event-output failure or stdout failure must not rewrite that receipt or emit a contradictory cancellation/failure terminal. Before commitment, cancellation still wins. Make finalization robust to interruption between persistence and event delivery, including interruption immediately after the terminal line is written. Keep default CLI results and documented exit codes coherent with the committed result. If event delivery itself cannot succeed, preserve the authoritative receipt and report the delivery limitation without inventing a second terminal outcome.
2. Establish Windows Job Object ownership before allowing child code to execute. Use a launch mechanism that creates the child suspended and resumes only after successful assignment, or an equivalent atomic ownership mechanism. Preserve background/hidden execution and redirected logs. Job creation failure must refuse launch; assignment/resume failure must terminate and reap the not-yet-running child and close every acquired handle. Do not continue with unowned processes. Make construction exception-safe, including spawn failure and KeyboardInterrupt. Own resources before any interruption can strand a running child.
3. Confirm owned descendants are gone before advancing stages/finalizing, including when the immediate child exits first. Keep cleanup bounded and report cleanup failure accurately. Do not infer an empty process tree solely from the immediate child's exit. Keep abrupt-parent-death behavior truthful; do not claim a terminal receipt after an uncatchable kill.

Required regressions:

- Inject interrupts immediately before commitment, after the completed receipt replacement, after publish's stage-finished event and immediately after the completed terminal event. Assert persisted status/artifacts never roll back after commitment, no contradictory terminal is attempted, and CLI outcome matches the authoritative receipt. Include an event/stdout delivery error after commitment.
- Force Job Object creation, process creation, assignment and resume failures. Verify no payload executes before assignment, no unowned fallback, and no leaked child/job/thread handles. A deterministic spawn-failure test may mock the creation call; the deliberately invalid-EXE test stalled in Codex's review environment.
- Use a child that tries to spawn a descendant immediately and a deterministic barrier/delay around assignment. Prove child work cannot start before ownership. Exercise cancellation and abrupt-parent death; verify no surviving descendant. Also cover immediate-parent exit with a still-running descendant and verify bounded cleanup before proceeding.
- Preserve the existing lifecycle cases, all 20 native-export regressions, both real fixture parity cases, and existing H2.2 captured-input/pre-write behavior. Run broad worker tests only if shared worker behavior changes.

Writable scope: `src/application/lifecycle.py`, `src/application/export.py`, focused lifecycle tests/stand-ins and Claude report/status/handoff. Small launch helpers are allowed; no worker rewrite, UI, dependency framework, new adapters, geometry, Legacy S7 or installation work. Use full starting/final hashes and exact test commands in the handoff. Keep prior evidence; label superseded ownership/completion claims. Stop for Codex review after this correction.
