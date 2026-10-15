# LEAGUE-SMOKE-1 review and close-flow correction

Claude session: continue the same session. One bounded defect remains in the Close wiring introduced during smoke testing.

Reviewed by Codex, 17 September 2026. The reported real-desktop evidence is useful and is not reclassified as mocked testing. Package acceptance is withheld for the close/refusal issue below. This review did not independently repeat the CDP desktop run or execute a game.

## Evidence reviewed

Read docs/receipts/league-desktop-smoke.md, the seven command execution attributes, command_execution_context.rs, the panel status changes, both newly wired Close paths and Registry::close_project. Visually inspected screenshots 47-fix-status-pill.png and 49-fix-cancel-outcome.png. They show the prepared-review state and the cancellation request followed by an aborted transaction with unchanged files. The reported 0.64 seconds is cancellation acknowledgement; terminal abort was reported around 40 seconds, not instantaneous cancellation.

Fresh independent checks, using local/runs/league-overnight-2026-09-17/target:

- `cargo test --locked --offline -p studio-api --test command_execution_context --test league_copied_target --test leagues`: 1 + 7 + 3 passed.
- `cargo test --locked --offline -p studio-domain --test leagues`: 7 passed.
- From app, local `vitest.cmd run src/components/CopiedTargetPanel.test.tsx`: 10 passed; Vite emitted deprecation warnings, not failures.
- `cargo test --locked --offline -p studio-api --test copied_target r2_reservations_cover_aliasing_workspaces`: 1 passed, explicitly confirming close_project returns false during apply and succeeds again after it finishes.

Total: 29 tests passed. The execution-context test guards source declarations; it does not independently prove responsiveness. Desktop timing/IPC behavior remains developer-observed evidence, supported by the supplied screenshots/log. No full-suite, release-build, native-picker or in-game claim is made by this review.

Reviewed candidate hashes:

- crates/studio-api/src/commands.rs: b03b731ade432ad44eda5192d4cb2dad2f37d86fe098c6b46f48ef3e9bc5560a
- app/src/AppShell.tsx: 7f82e57d0dc06ef659c4cc13591a6ed430f56a34e4bd9aa805489ee957af67da
- app/src/pages/OpenProject.tsx: 62fcbf40763e47ed6ae51b6273a71a7c4321dba7995f85cb417b769d480515a5

The smoke receipt omits a complete changed-source hash manifest. Include it in the correction receipt; fixture hashes are not a substitute for candidate source hashes.

## Blocking finding — Close ignores backend refusal (P2)

AppShell.tsx:113–120 fires `void call("close_project", ...)`, immediately clears the project context and navigates to the picker. OpenProject.tsx:40–44 does the equivalent on the corruption-guard escape. Neither awaits the result or handles rejection.

Registry::close_project explicitly returns false when a reservation/in-flight operation needs the session, and also when prepared transaction disposal fails. The existing API regression independently confirms the busy refusal. Since apply now runs off the main thread, the user can actually click Close while the operation runs. The backend correctly keeps the workspace, but the UI removes its owner and hides its running progress/cancel controls. The operation may continue writing the copied tree while the picker suggests the session has closed. A failed abort can likewise leave an open session silently orphaned from the UI. An IPC failure becomes an unhandled rejected promise.

This finding combines inspected frontend control flow with a freshly executed backend refusal regression; no new desktop reproduction is claimed. It is distinct from asking for a new unsaved-edits confirmation policy.

Required: complete the close transition only when server-side closure is acknowledged. While pending, prevent duplicate close requests. A refused close or transport failure must retain a usable owner and expose a reason/retry; for a running save, preserve access to cancellation/completion. Do not work around this by forcibly deleting the session or automatically discarding edits. Handle the corruption-escape path without redirect loops or losing track of a failed-close session. Explicitly define how already-closed/idempotent cases are distinguished from a busy refusal, since the current boolean API conflates them; use existing status/ownership or a small typed outcome if necessary.

## Scoped responsiveness follow-up

The seven async declarations address the observed apply/cancel failure. Do not claim they establish a universal lock-free UI: copied_target_status still acquires registry locks, and review packaging holds the workspace mutex while staging. A synchronous status/edit/close command waiting on that mutex can still block the main thread. The developer already records broader stalls. Keep this as a separate bounded measurement/command-dispatch task after close-flow acceptance; do not blanket-convert all commands or promise that an async attribute alone fixes lock contention.

## Executable prompt — LEAGUE-SMOKE-R1

Continue as sole Studio developer in E:/Repositories/nhl-modding-studio. Read this review, the smoke receipt and the working agreement. Preserve unrelated dirty work and the validated cancel/status/message fixes.

1. Correct both Close paths so navigation/context disposal follows acknowledged server closure. Preserve ownership on refusal/error and provide a retryable user-facing state. Keep active apply controls accessible. Support corruption escape without redirect loops. Handle project-switch/late-response ownership and already-closed cases explicitly. No new unsaved-change confirmation requirement is imposed by this package.
2. Add component tests with deferred close responses: true/success, false/busy, rejected IPC, repeated click, and late response after a project switch. Cover the corruption escape. Verify context and route do not discard the wrong session or abandon a live operation. Keep the backend close-during-apply regression passing.
3. Re-exercise Close during an actual copied-target operation in the real desktop using the established smoke method on disposable fixtures. Confirm refused close leaves progress/cancel accessible, then cancel/wait and close successfully. Observe backend session release through a test/status seam, not just absence of a page. Also exercise ordinary prepared-review close. Distinguish desktop observations from mocked tests; if tooling is no longer available, state the missing gate precisely.
4. Run affected component/API checks, frontend type/build and binding/shell checks if contracts change. Do not re-run multi-gigabyte campaigns without a relevant change or unresolved concern. Correct the source comment claiming synchronous status/cancel never wait on these operations; document actual lock constraints. No broad responsiveness rewrite, new phase, in-game test or live install.
5. Deliver a plain-language-first receipt with complete changed-source hashes, exact checks, observed close outcomes, remaining limitations and exact scratch ownership. No intermediate updates or preference questions needed. Stop for Codex review. No staging, commits or pushes.

## Cleanup and roadmap

Independently confirmed fixtures/ is absent under local/runs/league-smoke-1, 51 screenshots remain, and the engine staging root is empty after review tests. No application source was modified by Codex. The index is empty.

Retain the screenshots and settings snapshot for this active smoke correction review. The existing target cache transfers specifically to compiling/launching LEAGUE-SMOKE-R1 and rerunning its checks. Document exact absolute paths for the retained setup crate/CDP driver rather than `H:/TEMP/…`; retain only what the correction needs and clean obsolete package helpers after durable evidence is saved. Leave unowned temp directories untouched. No package-closeout cleanup is claimed while the correction remains active.

Next: close-flow correction and acceptance, then a bounded responsiveness task or separately authorized runtime test plan. Phase 1.1 remains open for target-specific game evidence. Projects/Builds remain final optional stretch.
