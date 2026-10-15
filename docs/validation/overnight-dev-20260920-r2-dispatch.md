# R1 review — scoped acceptance and colour reapply correction

Continue in the same developer session for OVERNIGHT-DEV-20260920-R2. No new features in this bounded correction.

## Review result

The archive ownership guard addresses the reproduced contradictory-scope defect. Source inspection confirms planning checks effective row ownership before placement; domain checks preserve existing pending edits. The original colour target-switch fix also passes its regression. This is scoped acceptance of those corrections, not the entire overnight campaign.

Independent checks: engine contradictory-ownership test passed; domain asset_member_undo suite 4 passed; ColorField and FieldProfile suites 14 passed; API asset_member_route 5 passed, 2 ignored. Total: 24 passed, 2 ignored. No private-fixture sweeps, desktop/game launch or live write.

## F1 — P2: successful colour value remains suppressed after undo

ColorField.tsx keeps the successful hex in `inFlight.current` and only clears it on error. A later authoritative value change reseeds text but never releases that guard. Consequently the same hex cannot be reapplied after undo unless another value is committed or the editor remounts.

Independent jsdom reproduction using the existing test helpers:

1. Mount the same project/table/record/field with packed colour 0xdd9b26.
2. Enter #123456 and blur; await successful summary refresh and enabled input.
3. Rerender the same target with authoritative value 0x123456 (save/edit acknowledgement).
4. Rerender it with 0xdd9b26 (undo).
5. Enter #123456 and blur again.

Expected two set_cells_int calls; observed one. The added regression failed, while the existing thirteen colour tests passed. The temporary reviewer test was removed; no production source was modified.

Separate actual request-in-flight state from temporary duplicate suppression while awaiting authoritative refresh. Clear/reconcile suppression once the stored value acknowledges the successful edit and when a later authoritative undo/reload changes it. Retain captured mutation arguments and target-bound drafts. Do not simply remove suppression and reintroduce Enter/blur duplicates before refetch.

Required tests: edit A→B, acknowledge B, undo to A, reapply B; authoritative same-target value changes; repeated Enter/blur before acknowledgement; pending request with a changed draft; failed mutation retry. Assert exact dispatch counts, args and final displayed state. Rerun colour/profile/asset UI suites and tsc. No backend/binding changes needed unless the actual fix requires them.

## Reporting

Begin with a plain-language receipt, then fix/test evidence and cleanup. Stop for Codex review. Phase 1 remains open; provisional Phase 2 accounting does not imply feature acceptance. Canonical rebuild, BIG4/replan, resize writers and broader team coverage still require their independent review. Existing NHL21-E1 acceptance remains closed.
