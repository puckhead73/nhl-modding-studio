# R2 review — colour acknowledgement/settlement ordering

Continue in the same developer session for OVERNIGHT-DEV-20260920-R3. One bounded P2 correction remains; no new features.

## Independent checks

The original edit/acknowledgement/undo/reapply regression passes. All 17 existing colour cases passed in the reviewer harness. A fresh full frontend run passed 150/150 (20 files); complete output is retained at `docs/validation/evidence/overnight-development-2026-09-20/codex-r2-frontend.log`. This does not explain the developer's earlier unattributed failure. Do not run repeatedly until green; retain complete failure output if it recurs.

An additional deferred-summary reproduction failed as below. No production source edits, desktop/game launch, live writes or backend tests. Temporary reviewer test removed.

## F1 — P2: settlement rearms suppression after an authoritative change

`ColorField.tsx` clears `awaitingAck` on authoritative colour changes, but `onSettled` unconditionally assigns the just-completed successful value back into it. Settlement occurs after the asynchronous invalidation and summary refresh. An acknowledgement and undo received during that interval are forgotten.

Reproduction using the existing component-test helpers:

1. Gate `project_summary`, mount packed colour A = 0xdd9b26.
2. Enter B = #123456, blur; allow set_cells_int to succeed, hold the summary response.
3. Rerender the same target with authoritative B, then authoritative A (undo), while summary is pending.
4. Resolve summary `{project_id: 7, dirty: false}`; wait for input to re-enable.
5. Enter B and blur again.

Expected two set_cells_int dispatches; observed one. The later onSettled overwrote the earlier acknowledgement reconciliation. This is component evidence, not a claimed desktop reproduction.

Bind suppression to the particular write and authoritative-state generation. A late completion must not reinstate a guard invalidated by newer authoritative state. For example, track the generation at dispatch and compare it at settlement; keep the exact implementation consistent with target-bound editor semantics. Preserve duplicate suppression before any authoritative acknowledgement and single pending-write dispatch. Do not fix this by removing duplicate protection.

Tests must cover acknowledgement before settlement, acknowledgement plus undo before settlement, authoritative other value before settlement, ordinary acknowledgement after settlement, no-ack duplicate Enter/blur, and failed mutation retry. Use deferred invalidation/summary responses and assert dispatch counts/arguments. Include the original R1 target-switch and R2 undo/reapply cases.

Run focused UI suites, type check, then one complete frontend run with its full output retained. If it fails, preserve the named failure and diagnose before further reruns. Never delete the only diagnostic evidence of a failure while it remains unexplained. No backend/binding work is requested.

Receipt: plain-language summary, deterministic race regression, tests and cleanup. Stop for coordinator review. Archive-scope acceptance remains unchanged; Phase 1 and broader overnight implementation review remain open. NHL21-E1 stays closed.
