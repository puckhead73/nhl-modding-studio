# P1-A review and R1 correction dispatch

18 September 2026. Continue the same developer session. P1-A is not closed; correct the preview lifecycle and finish applicable offline coverage before advancing.

## Independent checks

- Domain schedule integration tests: 10 passed, 1 private fixture ignored. The real NHL 20 result remains developer evidence in this review.
- API schedule integration test: 1 passed.
- Schedule card component tests: 4 passed.
- Read schedule proposal/application paths, generation, and component async ownership. A temporary independent anchor-backed, memory-only probe reproduced F1 below. No install, image or anchor was written. Probe source removed after execution; no runtime acceptance claimed.

## F1 — P1: Apply is not bound to the previewed change

`workspace.rs::schedule_edit_apply`, `schedule_import_apply` and `schedule_generate_apply` derive a fresh plan on Apply. `apply_schedule_plan` compares its fingerprint to the same current model used to derive that plan. Only import accepts an earlier slot fingerprint, and that fingerprint omits schedule contents. The API also re-reads the import path on Apply. Consequently the receipt's claim that preview/apply races are refused is false: a changed file, configuration or schedule can produce an unreviewed result.

Independent reproduction: open Legacy Xbox anchor in memory; export NHL schedule JSON; preview that import (a no-op); change game 0's day through `schedule_edit_apply`; apply the previously previewed import with its fingerprint. Apply succeeds and restores the old day, silently overwriting the intervening edit. Probe printed: `CONFIRMED: previewed no-op import overwrote a subsequent schedule edit with unchanged slot fingerprint`.

Bind every apply to the exact project/table, relevant baseline revision/content, operation/configuration, mapping and imported bytes that were previewed. A server-held review or independently validated content token is acceptable; a freshly recomputed slot fingerprint alone is not. Refuse stale baseline/input before any mutation and require a fresh preview. Snapshot the import or hash-check its bytes on Apply. Keep the apply transaction atomic and undoable.

Regressions: preview then schedule edit; team membership/reordering change; imported file changed without changing keys; configuration/mapping changed; foreign project/table review; ordinary unchanged review succeeds; each refusal preserves bytes, undo/redo, pending state and revision. Update the inaccurate receipt claim through a dated correction note.

## F2 — P1: late UI responses can revive stale previews or update the wrong context

`LeagueScheduleCard.tsx` increments `genRef` but never uses it to guard completions. Child preview callbacks unconditionally set plans. Editing inputs while a preview is pending clears the plan only temporarily: the older response can restore it and enable Apply for newer inputs. Import keeps the previous plan available while a new mapping preview is pending. Controls can also change during Apply. `afterApply` compares `forProject` to a captured `projectId`, not the current project after its summary request completes. The selected table is not reset/validated against the new league when `leagueCode` changes.

Bind child requests, responses, dialogs and summary refreshes to current project/league/table, input generation and mounted state. Invalidate reviews immediately on any relevant change; ignore stale responses; prevent input changes from producing a different confirmed operation during Apply. Clear or validate `picked` when context changes. A stale completion must not call the global project update with another project's summary. Reuse the established project-bound lifecycle patterns.

Add deferred-promise component tests: preview resolves after input change; previews resolve out of order; mapping changes while preview pending; project/league/table switch during preview/apply/summary refresh; file dialog returns after switch; double Apply. Verify both displayed state and exact dispatched arguments, not merely a disabled button.

## F3 — P2: finish offline parity before calling P1-A complete

The receipt explicitly leaves import/generation coverage missing on older Xbox/PS3 targets, generation apply missing on PS4, and NHL 07 import missing. Complete the applicable parameterized cases, and audit all target rows, including NHL 19's correct source and Recompiled's Studio route. Shared bytes may reuse data but do not substitute for platform/adapter integration evidence. Unsupported generation topologies may remain explicit arithmetic blockers; unbound tournaments stay research questions. Scope the summary to bound, supported source schedules rather than every league.

## Execution and stop point

Execute P1-A-R1, confined to these corrections and their tests. Preserve existing successful scheduling logic. No new phase, live writes, game launches, quarantine, staging, commits or pushes. Reuse the build cache and clean only obsolete owned artifacts with containment checks. Deliver a plain-language-first receipt with findings, exact checks, per-target coverage, changed identities and cleanup; stop for coordinator review.

Phase 1.2 remains active. Localization contract work continues independently. NHL21-E1 remains accepted and closed; no additional visual evidence is requested here.
