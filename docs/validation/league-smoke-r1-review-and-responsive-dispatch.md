# LEAGUE-SMOKE-R1 independent review and LEAGUE-RESP-1 dispatch

17 September 2026. **Accepted for acknowledged-close correctness. Start a new Claude session** at this package boundary. Responsiveness and in-game qualification remain open.

## Independent review

Inspected Registry close outcomes, the command, shared hook, shell button, picker escape integration and component regressions. Closure acknowledgement now precedes owner disposal/navigation. Busy, failed and transport failures retain ownership; the backend rechecks reservations before removal. No blocking correctness finding in this correction's reviewed scope.

Fresh checks: all 23 source SHA-256 values in the receipt matched; `cargo test --locked --offline -p studio-api --test copied_target r2_reservations_cover_aliasing_workspaces` passed (1); `cargo test --locked --offline -p studio-api --lib bindings` passed (25); `app/node_modules/.bin/vitest.cmd run src/components/CloseProject.test.tsx` passed (12). Cargo used the existing transferred target directory. Index empty. No application source changed by Codex.

Inspected developer screenshot 54: refusal names the running apply, Retry close and Cancel apply remain visible. Developer's real desktop/CDP observations are accepted as recorded, not claimed as an independent desktop rerun. Corruption escape has component evidence only. A screenshot does not establish timing; the ~32-second main-thread stall is the developer's measured result and remains a material open issue.

## Executable Claude prompt — LEAGUE-RESP-1

Act as sole application developer in E:/Repositories/nhl-modding-studio. Read this review, docs/plans/coordinator-developer-working-agreement.md, docs/plans/entity-editing-parity-roadmap.md, docs/receipts/league-smoke-r1-close-flow.md and docs/receipts/league-desktop-smoke.md. Preserve unrelated dirty work. LEAGUE-SMOKE-R1 is accepted; do not reopen feature scope or claim Phase 1.1 complete.

Objective: keep the desktop responsive and cancellation reachable during copied-target review/apply, including when Close/status/ordinary league editing contend with their locks. Address the measured ~32-second close stall and measure the review-time workspace-lock case. This is one bounded infrastructure package, not a migration of every command.

1. Trace the commands actually dispatched by the league/pending/copied-target screens, their execution threads and registry/workspace/engine lock order. Record a compact before/after map. Measure real desktop input/heartbeat delay separately from command response, cancellation acknowledgement and final cancellation settlement. Include ordinary league edit/read refresh as a baseline.
2. Move blocking work and lock waits off the UI thread for the affected paths. Make busy close/status/cancel responses prompt where safely possible. An async annotation alone does not resolve lock contention. Investigate long input hashing/packaging under the registry/workspace locks; shorten lock scope only with explicit reservations, stable snapshots and final revision/identity validation. Preserve the single authoritative snapshot, external-change detection, ordered locking, alias reservations, rollback, recovery and acknowledged-close semantics. Do not introduce stale cached status that claims completion or allow edits to race a save. If a deeper lock redesign exceeds this bounded package, finish safe dispatch improvements and document the exact remaining blocker rather than silently widening scope.
3. Add meaningful contention regressions using synchronization barriers/test seams where possible: Close during preflight/review, status and cancel while work is blocked, competing edits/reviews/aliased sessions, and project ownership after refusal/completion. Cover concurrency newly enabled by dispatch changes. A source-attribute guard alone is insufficient. Preserve the earlier copied-target, invalid-reference and close-flow regressions; run affected suites, bindings checks when contracts change, shell compile and frontend checks for touched surfaces.
4. Reproduce the real desktop flows on disposable copies using the established Tauri/CDP method. Exercise review, apply, Close refusal, cancellation and successful final close. Demonstrate the window accepts input during verification rather than freezing until a lock releases. Aim for close refusal/cancel acknowledgement within one second on this host; report all measured values and any miss honestly. Final safe cancellation can take longer: report that separately. Verify destination originals/restoration, absent orphan handles/reservations, source hashes, settings restoration and stopped processes. No in-game run or live install.
5. Use local/runs/league-resp-1 for new scratch. The cache and reproduction helpers below transfer specifically to reproducing the preflight/review contention baseline and validating this fix. Inventory and remove obsolete step scripts as you replace them; do not reuse the old settings snapshot (removed). Capture fresh pre-run settings and restore them. Clean fixtures/logs/helpers after their dependency ends; retain only explicitly enumerated review evidence/build dependencies. Never delete shared or unowned temporary directories by inference.
6. Deliver docs/receipts/league-resp-1.md with a short plain-language summary first, changed-source hashes, lock/dispatch decisions, exact tests and measurements, remaining limits, and absolute scratch ownership/disposition. No intermediate updates or preference questions needed. Stop for Codex review. Do not stage, commit, push, start Phase 1.2/1.3, or implement Projects/Builds.

## Cleanup and transferred ownership

Codex verified the disposable fixtures were absent and engine staging empty after review checks. All 56 desktop screenshots were copied with matching hashes into `docs/validation/evidence/league-desktop-smoke/`, with SHA256SUMS.txt; these are durable historical evidence for SMOKE-1/R1, not disposable builds. Receipt screenshot paths now resolve there by filename. Removed the obsolete `local/runs/league-smoke-1/` tree, including its settings snapshot, after containment/reparse checks and evidence preservation.

The following transfer to LEAGUE-RESP-1 for its immediate baseline reproduction, compilation and real desktop regression; remove when those dependencies and independent review finish:

- `E:\Repositories\nhl-modding-studio\local\runs\league-overnight-2026-09-17\target\` — shared series build cache; avoid a second 14 GB build.
- `H:\TEMP\claude\e--Repositories-nhl-modding-studio\e66483ae-2c51-4f71-9688-ef42374df4fa\scratchpad\smoke-setup\` — disposable archive fixture setup.
- `H:\TEMP\claude\e--Repositories-nhl-modding-studio\e66483ae-2c51-4f71-9688-ef42374df4fa\scratchpad\cdp\` — existing desktop reproduction driver; prune obsolete step scripts in the successor.

Unowned temporary folders are untouched. No live recovery backup was removed.

## Roadmap

Close-flow correction accepted; LEAGUE-RESP-1 is next. Phase 1.1 remains open for game acceptance, PS4 archive qualification and missing target fixtures. Later league steps and Phases 2–5 retain their order. Projects/Builds remain final optional stretch.
