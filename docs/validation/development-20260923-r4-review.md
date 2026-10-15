# R4 focused coordinator review — 2026-09-23

## Disposition

The backend changed-size mipped Xbox refusal passes focused review and public-route regression. The same-size partial policy is explicit. The earlier kit-completeness acceptance remains closed. Dictionary isolation and archive-preview seam checks passed; this is not independent acceptance of every real-tree sweep, desktop surface or runtime behavior.

Independent checks: loose_texture_route 3 passed/5 ignored; song_catalogue 2 passed/1 ignored; archive_member_preview 1 passed/1 ignored; TextureReplace/TeamKitsCard/SongCatalogueCard Vitest 18 passed. Total: 24 existing tests passed, 7 private cases ignored. A separate temporary UI regression failed as below. No production edits or live writes; probe removed after recording it.

## F1 — P2: disclosure can be bypassed while pending or failed

`app/src/components/TextureReplace.tsx` enables Write once the two previews load. `plan.isLoading` and plan failure are not part of the write gate; only a completed checked refusal blocks. Its existing test explicitly requires the button to remain enabled when the plan fails.

Independent reproduction: mock current preview as `texlib`, source at the same size, hold `texture_replace_plan` unresolved. Render and choose a source, wait for `texture-replace-plan-loading`. The Write button is enabled. The regression expecting it disabled fails. A same-size backend write is permitted and would perform partial replacement without its promised advance disclosure. The backend's resize refusal remains correct; this finding is about partial-operation review, not a bypass of that refusal.

For a target whose operation requires partial disclosure, wait for a current successful policy response before enabling Write. On failure keep retry/cancel available and do not silently proceed. Preserve explicitly unsupported pre-check behavior for other formats only when the backend response establishes it, rather than inferring it from a failed request. Bind the displayed policy to the current target/source selection. The current cache key uses paths/index/dimensions with infinite staleness, so also audit re-open/replacement-at-same-path freshness. A changed target must not apply a materially different partial policy than the one displayed; either bind the plan to source identity or revalidate and require refreshed review on drift.

## Next developer prompt — same session, extended continuation

Read this review and current repository instructions. Close F1 first with deferred-plan, failed-plan, retry, source/target-switch and stale-at-same-path regressions. Assert no import dispatch until the required current disclosure is shown. Replace the existing failure-allows-write expectation. Keep the authoritative backend refusal and independent non-Xbox routes intact. Verify failed/stale plans preserve files and backups; run affected UI/API checks. Publish a plain-language-first correction checkpoint, then continue automatically under the existing extended-session authorization.

Next implement Teams 2.5 PS4 venue listing using the **latest R73 contract**, not the superseded R68 reassignment proposal: list all matching levels with their folders, named ambiguities, missing folders and dependencies. Shared ids must never silently choose a record. Read-only discovery only; no arenaartid reassignment. Consume `docs/research/overnight-2026-09-22-pm/04-ps4-venue-listing-contract.md` and subsequent corrections/claims.

Then continue actionable roadmap parity, coordinating claims with research. Do not wait for a full Xbox packed-tail contract: those resized writes remain refused while independent work progresses. Dictionary labels remain attached-project-specific; no sibling fallback. Keep diagnostic audio distinct from verified game audio. Projects/Builds stays stretch.

No routine review stop between packages; checkpoint receipts/matrices/cards and continue as usage allows. No launches/live writes/staging/commits/pushes, no shared-cache cleanup, no zz_diag. Preserve accepted NHL 21 E1. End with exact coverage, actual checks, limitations, manifest-owned cleanup and one next task.
