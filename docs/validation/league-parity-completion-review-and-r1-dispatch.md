# LEAGUE-PARITY-COMPLETION review — R1 corrections

18 September 2026. Disposition: implementation delivered; consolidated acceptance pending the corrections below. Continue the same Claude session because these concern the new lifecycle and integration code. NHL21-E1 remains accepted and closed by the user; none of these findings reopens it.

## Independent evidence

Codex read the completion receipt, relevant implementation, tests, deployment/quarantine helpers and runtime cards. All 132 entries in the package SHA256SUMS.txt matched. The index was empty. The reported 977 Rust tests, private/real-archive cases, desktop run and 57 frontend tests remain developer evidence: this review did not rerun those suites or launch an app/game.

On a disposable fixture, Codex represented an interruption after the first quarantine move: first.db existed only in quarantine, second.db only in the game directory, both with their recorded hashes. Running the unmodified helper's restore mode returned exit 2, `derived state: MIXED` and `REFUSED: mixed or unknown state; nothing moved`. Neither file was restored/moved. This is an independently reproduced failure. An initial PowerShell invocation was blocked by the local script execution policy; the successful reproduction used a process-scoped ExecutionPolicy Bypass, without changing machine policy.

Other findings below are source-confirmed control-flow defects, not claimed runtime reproductions. Reviewer-owned two-file fixtures and logs were removed after recording this result; live installations were untouched.

## Required corrections

### F1 — P1: dictionary validation occurs after database writes

`crates/studio-domain/src/workspace.rs:1465`: `save_in_place` writes the primary database immediately, then the roster/sidecars, and only afterward checks the attached dictionary's expected identity at lines 1491–1505. With both a database edit and pending rename, an externally changed dictionary causes an error after the database has already been changed. The current test uses an unchanged database, so it cannot establish its comment that nothing was written. A subsequent dictionary write failure can likewise split the save.

Preflight the complete save set before mutation and use coordinated recoverable publication for database/roster/dictionary writes. Reuse transaction machinery where applicable; do not solve only the early identity check while leaving partial write failures undisclosed. Add a regression with an actual DB edit plus pending dictionary rename and changed dictionary, asserting all destination identities and session baselines/pending edits remain unchanged on refusal. Test a later dictionary publication failure and demonstrate rollback or explicit recoverable state, never a generic refusal hiding earlier writes.

### F2 — P1: failed dictionary reload silently discards edits

`workspace.rs:553–601`: reload prepares only DB/roster first, adopts them, clears history, then clears the pending dictionary edit before trying to load its dictionary. A missing/unreadable/invalid dictionary is swallowed by `Err(_)`, detaches the dictionary, and the method succeeds. This contradicts the method's two-phase failure-preserves-edits contract.

Prepare and validate the dictionary alongside the other files before adopting any reload. On failure return an actionable error while preserving working bytes, attached dictionary, pending rename, history and revision. Explicitly dropping a dictionary must be a separate intentional action. Cover missing and CRC-invalid dictionaries with concurrent DB edits, then a successful reload after the file is restored.

### F3 — P1: late rename responses can overwrite another project's summary

`app/src/components/DictionaryRenamePanel.tsx:28–50`: mutation callbacks await invalidation and then call the unconditional global `update(await project_summary(oldProjectId))`. `ProjectProvider.update` accepts that summary for whichever project is now open while retaining its existing dbPath/install. Remounting the panel with a key does not cancel an already-running async callback. Closing A and opening B while the summary request is pending can leave B's paths paired with A's summary/id. Preview responses also have no input/dictionary generation binding, and editing either input leaves the old preview displayed.

Bind callbacks and summary publication to the current project/generation, including after every await; reject foreign summaries at the shared update boundary if appropriate. Clear or invalidate previews when their inputs/dictionary identity change and ignore delayed obsolete results. Use deferred component tests for switch/close/unmount during set/clear/summary requests, and changing inputs while a preview is pending. Confirm B's summary, paths and install stay together and previews describe current inputs.

### F4 — P1: quarantine cannot restore an interrupted two-file operation

`local/runs/league-parity-batch/quarantine.ps1:72–108`: any combination other than all-in/all-out becomes MIXED, and both directions refuse. The helper itself can leave this state if the process stops, a later move fails, or the emulator starts between moves. Restore must work when one verified original is already home and the other is safely quarantined.

Derive recoverable states per file and make restore idempotent across interruptions in both directions. Preserve unknown files; reject overwrites; keep verified originals recoverable without depending on a final manifest write. Validate containment and ancestor reparse points for mutation paths. Add interruption cases after each move, repeated recovery, emulator-start interruption, and unknown destination preservation. Have status predict the actual recovery plan. No live quarantine is authorized by this correction package.

### F5 — P2: Xbox-only trace evidence is displayed as proven on PS3/Recompiled

`defs/leagues/gen7-nhllegacy.toml` is shared across Legacy PS3, Xbox 360 and Recompiled but marks the bindings `traced`. `crates/studio-domain/src/leagues.rs:607` copies that evidence directly, and the PS3 test in `studio-api/tests/dictionary_rename.rs` explicitly expects `traced`. The desktop record even describes the traced pill on PS3. This contradicts the receipt's correct limitation that only the Legacy Xbox 360 executable table was traced.

Resolve binding evidence by actual target/platform and source witness. Preserve shared mappings if useful, but PS3/Recompiled must remain hypothesis/by-reference as appropriate until independently established. Add target-specific assertions and correct the desktop/matrix wording without rewriting historical screenshots. Do not claim game hash-collision resolution or consumers beyond the evidence; NHL 07's missing expected key family leaves its label source unresolved, not an exhaustive proof that no dictionary text can supply a league label.

## Coordinator decisions and execution prompt

Execute F1–F5 sequentially as LEAGUE-PARITY-COMPLETION-R1 with developer checkpoints and one receipt. Keep the candidate preparation work and retained rollback artifacts; regenerate only candidates actually affected, with fresh identities and audits. Run focused regressions and relevant final checks, including bindings if DTOs change. Preserve unrelated work, originals and private fixtures. No staging, commits, pushes, live replacements, quarantine or game launches.

REG-1's private, source-identified fixtures are sufficient for this direct-editing campaign. Canonical adapter promotion and physical immutable-anchor relabelling are deferred to a separately bounded registry maintenance package; do not use them as a reason to stop the active corrections or mix in a canonical rewrite. The recorded provenance corrections remain authoritative now.

NHL 19 quarantine remains a runtime-session decision. NHL 12's ISO route and missing Xbox installations remain explicit blockers, not passes. Maintain one consolidated runtime checklist across the prepared titles; no repetitive NHL 21 qualification or per-title preparation review loop.

Begin the receipt with the plain-language summary, then findings dispositions, actual checks, changed paths/hashes, per-target readiness and cleanup. Remove obsolete correction fixtures/builds after recording durable evidence, retain only named review/runtime dependencies, and stop for Codex review.
