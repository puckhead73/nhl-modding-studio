# Overnight developer campaign: first review and R1 correction dispatch

Start a fresh developer session for this bounded correction package. The campaign is delivered, not yet independently accepted in full. This first review prioritized the new write routes; it is not a claim that every changed file, codec or parity cell has been audited. Keep the remainder of the campaign available for subsequent review; do not begin new feature work yet.

## Independent checks and findings

Codex read the final receipt/receipt 07, traced the asset preview/pending/copied-target route and UI, and inspected settings CSV parsing and localization growth. Executed:

- `studio-api`: asset_member_route **3 passed**, including the real NHL 21 private fixture; dictionary_growth **3 passed**; settings_csv_route **2 passed**. All selected cases ran, no ignored cases remained in this invocation.
- `tdb-defs --test settings_sites`: **9 passed, 1 ignored** (patch fixture not selected in this separate command).
- `tdb-loc --test growth`: **9 passed**.
- Additional reviewer reproduction below: **failed as expected**, exposing lost prior pending state.

Total existing tests independently rerun: **26 passed, 1 ignored**. No full workspace suite, frontend rerun, desktop/game launch or live install. Existing compiler warnings remain. Index empty at review. The prior P1-C and NHL21-E1 acceptances remain unchanged.

### F1 — P1: archive replacement UI has no async context ownership or summary refresh

`app/src/components/ArchiveMemberReplace.tsx` holds picker/preview/apply state without mounted/context/generation guards. The picker may complete after a project/install/member switch; preview results may belong to a previous target; apply success unconditionally changes displayed state and invokes the current callback. Apply uses the current project id with whatever ticket remains displayed. Backend project-bound tickets prevent a foreign ticket from being accepted, but do not make the UI ownership correct.

The success handler also ignores the returned `summary`; both callers only refetch their logo/imagery query. Consequently the global project summary can remain clean after the first texture edit, even though the backend is dirty. This is a source-level finding, not a claimed desktop reproduction.

Bind picker, preview, apply and completion to project instance/id, install root and selected member plus a generation. Clear obsolete displayed state, ignore late responses, and prevent duplicate dispatch while applying. Define cancel/close during apply truthfully: hiding a dialog does not cancel an already dispatched backend mutation. Update the project summary and pending-query state only for the owning current project. Reuse the corrected ownership patterns already present in the schedule/membership flows.

Required deferred-promise component tests: picker return after context switch; preview return after switch/cancel/new selection; apply completion after project switch/unmount; successful same-project apply updates dirty/summary/pending; repeated Apply; close during apply. Assert exact dispatched arguments and global state, not only rendered text. Backend foreign/stale ticket protection must remain intact.

### F2 — P2: superseding a pending texture discards the history needed for undo

In `Workspace::asset_member_apply_review`, `retain` removes the prior pending plan for the same archive/member, then pushes the new plan. `undo_asset_member_edit` simply pops it. Independent Registry reproduction using the existing synthetic copied-root fixture:

1. Preview/apply image A (seed 42) for L0.
2. Preview/apply image B (seed 77) for that same member.
3. Call `asset_member_undo`.

Observed `dirty=false, pending=0`. A was lost; undoing B returns to stock rather than restoring A. The current Pending Edits wording says “Drop the last texture replacement”, but the API and delivered receipt describe undo; preserve the user's preceding applied edit rather than silently discard it.

Keep operation history separate from the coalesced replacement map. Undo the latest apply restores the prior plan for that member and its position/identity as appropriate; copied-target output uses only the effective latest replacement for each member. Test A→B→undo→save produces A, interleaved edits of different members, and state/revision/stale-review invariants. All copies must restore exactly. Do not regress shared-span private-copy behavior.

### F3 — P2: narrow campaign coverage claims before acceptance

“Everything else … impl+tested or n/a” is not a substitute for per-operation evidence. Distinguish loose same-dimension replacement from archive placement, reader/codec success from complete UI flow, measured families from every installed build, and researched absence from n/a. Preserve actual implementation gaps as not implemented. Audio event identifiers disprove broad no-league-audio conclusions but do not establish sample replacement. Reconcile with `overnight-research-2026-09-19-review.md`; do not import its refuted universal RefPack relocation claim or expand archive-mismatch acceptance from counts alone.

This does not request a full reimplementation or another exhaustive suite. Correct the final receipt/matrix where their claims exceed the cited tests, and identify remaining review areas explicitly. Desktop gates remain user-review pending.

## Next Claude prompt — OVERNIGHT-DEV-R1

Read this review and the campaign receipts. Implement F1–F2 with the focused regressions; reconcile F3. Sole developer owns application changes; preserve unrelated dirty files. No new Phase 1 features, Phase 2, canonical rebuild or dimension-changing work until this correction returns for review.

Run affected UI/domain/API suites and the real private-copy case; rerun binding checks only if DTOs change. Preserve planned bytes, revision ownership, copied-target recovery, non-target archive rows and sibling payloads. Begin the receipt with a plain-language summary, then findings, changed identities, exact test results, omissions and cleanup. Stop for Codex review after the bounded package. No launches, live writes, staging, commits or pushes.

## Reviewer cleanup and roadmap

The temporary reviewer `codex_overnight_probe.rs` and its specifically identified build artifacts are removed after recording the reproduction. The test's disposable fixture was removed by its Drop guard; shared cache preserved.

Phase 1 made substantial implementation progress, but the campaign remains under review. Remaining feature gaps include dictionary tree rebuild, different-dimension assets and gen7 changed-length placement. Keep those queued behind correction/review. Research can continue independently; Projects/Builds remain final stretch work.
