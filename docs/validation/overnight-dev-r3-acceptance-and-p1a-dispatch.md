# R3 acceptance and P1-A schedule workflow dispatch

18 September 2026. R3 is accepted within its reviewed correction scope. Start a fresh Claude developer session for P1-A; the recovery correction series is complete and the next package has a different working context.

## Independent review

Read the lifecycle classification, durable-record reload, restore-intent ordering, process-probe gate and regressions. Independently reran:

- `cargo test -p studio-domain --lib -- iso_ in_place_save`: 23 passed, 1 ignored, 0 failed.
- `cargo test -p studio-api --test iso_route`: 5 passed, 1 ignored, 0 failed.
- DiscImagePanel and IsoOpenPanel component suites: 16 passed, 0 failed.

The completed-record mixture reproduction is now refused without replacing its bytes; stale in-memory intent does not override durable completion. Interrupted apply and restore remain recoverable. The deterministic API test uses the same fail-closed error path as production. No blocking findings in this correction. Broader checks in receipt 09 remain developer evidence; no live image or runtime test was performed here. Existing compiler/deprecation warnings do not invalidate these results.

R3 acceptance permits development to resume, not unattended live image writes. Image Write cards remain withheld pending a separately scoped user execution decision. In-place saves still lack a durable restart journal; do not advertise crash-safe coordinated saves. NHL21-E1 remains accepted and closed.

## Next prompt — P1-A: source schedule editing across titles

You are the sole Studio developer in a fresh session. Read the working agreement, canonical Phase 1 coverage matrix, research handoff `12-phase1-competition-structure-and-schedules.md` including its corrected slot rules and NHL 20 provenance, receipt 04, and the existing `schedules.rs` implementation. Preserve unrelated working-tree changes. Do not wait for localization research: this package is independent of LOC-W3.

Implement the matrix's P1-A schedule workflow across applicable targets before expanding into Phase 2:

1. Reconcile target availability against current research. NHL 07 has schedule variants; distinguish missing Studio support from absent game data. Use the real NHL 20 fixture, not the mislabelled NHL 19 anchor. Do not extrapolate platform qualification from shared schemas.
2. Provide a schedule-aware editor for home team, away team and date, with preview and post-edit validation. Resolve slots by rank within the league sorted by global team index, never by team ID or record order. Preserve/disclose unresolved table-to-league bindings; require an explicit validated mapping for imports where resolution is ambiguous. Handle zero-game members, All-Star rows, unequal per-team game counts, seasonal date wrap and each field's actual width. Do not invent a season year from a yearless table.
3. Implement documented CSV/JSON import/export with stable source identifiers and explicit destination team mapping. Validate the whole proposed change before applying it through existing edit-session/undo/pending/save paths. Refused input must leave bytes, pending state and history unchanged. Reject stale mappings after team ordering/membership changes. Preserve fields outside the requested edits; do not silently truncate, resize packed tables or overwrite unsupported rows.
4. Deliver deterministic generation for a supported, explicitly configured topology within proven table capacity, with a preview and validation. Where a title/topology cannot be supported from the available evidence, give a precise blocker and retain edit/import support. Do not claim stock scheduling rules or mode-local schedule survival from source-table validation. Do not fill unused rows without establishing how active rows are represented.
5. Test each applicable title/platform using existing fixtures, parameterized where schemas coincide. Include NHL 10/11 record-order exceptions, Legacy AHL unequal game counts, PS4 15 versus 16/17 versus 18+ layouts, real NHL 20, malformed imports, over-capacity refusal, undo/redo and copied-output reopen/restore. Keep private-fixture skips explicit. Generation tests must check schedule properties, not just repeat generator logic.
6. Update the operation-by-target coverage matrix and batched user review cards with exact tested scope. Correct stale research-status cells only where current handoffs establish facts; do not mark runtime acceptance from offline checks. Return a plain-language-first receipt, changed-file identities, actual checks and cleanup, then stop for coordinator review.

No live install/image writes, game/emulator launches, quarantine, staging, commits or pushes. Avoid new large build trees (E: had about 5.3 GB free at review). Reuse the existing cache for active work; remove superseded owned scratch with containment checks. Preserve active recovery material and unrelated files. The next receipt must name a concrete dependency for every retained temporary artifact.

## Roadmap position

Phase 1 remains active; P1-A is the next implementation package, followed by the remaining matrix packages. Accepted chunklzx/RW4ps3 research is queued for the asset work. LOC-W3 remains under separate contract correction. Phase 2's existing opening slice is preserved without expansion; Projects/Builds remain the final stretch feature.
