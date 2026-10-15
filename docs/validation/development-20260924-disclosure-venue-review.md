# Coordinator review — disclosure gate and venue listing, 2026-09-24

## Disposition

Same developer session. Pending/failed-plan gating and R73 read-only venue discovery pass focused review. One P2 disclosure-freshness defect remains before the Replace flow is closed. This does not reopen the accepted kit fix or NHL 21 E1. No blanket acceptance of unrerun fixture sweeps or runtime behavior.

Independent checks: TextureReplace/ArenaAssetsCard 17 tests passed; studio-api arena_assets 4 passed. One additional coordinator regression failed as described below. No production changes, launches or live writes.

## F1 — P2: an allowed-to-allowed target change writes an undisclosed partial result

The existing stale-plan regression covers only allowed-to-refused drift. The actual public import route takes no reviewed identity/policy token, so rechecking whether today's write is allowed does not establish that its partial behavior was reviewed.

Reproduction in a temporary copy of loose_texture_route.rs, using its existing synthetic fixtures:

1. Write `flat` (512-square single-level raster plus the test's second raster) to a disposable target.
2. Request texture_replace_plan for a 512-square source, raster 0. Result: allowed, notes empty.
3. Replace that target at the same path with `stored` (512-square, 10 mip levels).
4. Call import_texture with the existing green 512-square source, backup enabled, raster 0.
5. Import succeeds and changes bytes. Its result now says partial replacement: base plus two smaller levels rewritten, seven packed-tail levels retain the previous image. The assertion that this unreviewed policy change must refuse fails.

Actual probe output: `SHOWN []; APPLIED ["Partial replacement (Xbox 360 mip chain): ..."]; BYTES_CHANGED true`. Thus post-write disclosure does not fulfill the prior review requirement. Backend resize safety remains intact.

## Next developer prompt — correction, then extended continuation

Continue in this session. Read this review and current repository instructions. Close F1 first; do not stop at another allowed-to-refused test. Bind the reviewed operation to the target bytes, selected raster, source and applicable policy. Use a backend-held review or validated identity contract, with comparison before backup/mutation against the same byte snapshots the writer consumes. A materially changed operation must refuse and require a fresh displayed plan; do not simply refresh and automatically write. Audit both file and byte import entry points and all callers. Preserve explicit unsupported-precheck behavior without letting it bypass a required disclosure.

Add the reproduction above as red/green, plus same-path source changes, unchanged-plan success, ownership/selection changes and refusal preserving target/backup. Keep loading, error, retry, cancel and duplicate-dispatch coverage. Avoid making a second disk read reintroduce drift after validation. Regenerate bindings if the contract changes. Publish a plain-language correction receipt with actual test results, then continue automatically as usage allows.

Next: Teams 2.7 diagnostic audio, starting with PS4 NHL 15–17 EA-MP3 under the latest corrected research contract. Evaluate the pure-Rust Layer III dependency for license, supported framing, malformed-input behavior, sample counts and maintained implementation; record the choice. Keep frame extraction, MP3 decoding and candidate embedded-PCM assembly separately testable. Candidate assembly is diagnostic, not audibly verified or game-qualified. Preserve unresolved vector cases and defer listening to the user. No replacement/encoder or live audio writes. Follow with gen7 EALayer3 only within R77's measured coverage and later ready contracts; do not apply the PS4 grammar to gen7.

Before consuming a handoff, check its latest correction and shared claim. Reconcile the PS3 NHL 12/13 kit-member wrapper as chunkzip per R76; do not keep a chunklzx-encoder blocker without inspecting the actual route. Venue results must remain explicitly from the baked, provenance-bound research table plus live folder status, not a live vault parse; no reassignment.

Maintain title/platform parity per operation. If research blocks one operation, record the exact missing contract and continue another ready roadmap item without waiting or inventing a writer. Keep Xbox mipped resize refused; H1 is still a candidate. Projects/Builds remains the final stretch feature.

No routine approval stops between packages. Checkpoint receipts, claims, coverage and morning cards. No staging, commits, pushes, launches or live/source writes. Exclude zz_diag explicitly; preserve shared caches and other lanes' files. Clean only owned temporary artifacts after their dependency ends. Finish with layman's summary, exact coverage/checks, unresolved limits and one next task.

## Roadmap

Teams 2.5 gains tested read-only PS4 venue discovery; kits retain their accepted inspection coverage. Teams 2.7 diagnostic previews are next. Phase 1 still has named writer/format and runtime gaps; listing coverage does not close editing coverage. Players/equipment/miscellaneous have partial inspection foundations. User runtime acceptance remains the user's decision.
