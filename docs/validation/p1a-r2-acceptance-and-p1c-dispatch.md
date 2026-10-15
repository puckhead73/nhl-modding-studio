# P1-A-R2 acceptance and P1-C dispatch

18 September 2026. R2 accepted. P1-A's bounded offline schedule workflow is accepted with its documented fixed-row-count, generation-topology and unbound-tournament limitations. Runtime review remains separate. Start a fresh developer session for P1-C; P1-B-R1 stays with the research lane.

## Independent review

Reviewed the per-instance namespace creation in both constructors, ownership check before lookup, exact stored-ticket comparison and string transport through DTO/client/UI. Independently reran the domain cross-workspace ownership regression (1 passed), Registry schedule workflow (1 passed), and schedule card suite (9 passed). No blocking finding in the ticket correction. The previous independent R1 run covered all 12 then-current domain cases, including 26-target parity and real NHL 20; the full R2 suite totals remain developer evidence.

Verified recovery blob `337704f5b5bb79873d57ae013be2bdad724111fc` has SHA-256 `f6f7b1b8b43cfb3c6f2fea89a6c23a7f186e9fb9cd7e71fe3436e7c753ace5ab`, matching R1. Current workspace.rs SHA-256 is `4d82b3122b72d01c6e8bec63d481fdc54bcf6b9cc08af2b301ac7520db5ab7c1`, matching R2. The blob-to-current diff is confined to the namespace/ticket changes. No loss of R1 content found. Continue using transform-before-write (prefer apply_patch); never truncate a destination before a scripted transformation succeeds.

No new review scratch or builds were created beyond reuse of the shared cache. No live files, images or anchors were changed by this review. NHL21-E1 remains accepted and closed.

## Next Claude prompt — P1-C: PS4 competition groups

You are the sole Studio developer in a fresh session. Read the working agreement, canonical Phase 1 matrix, existing membership/group implementation, and research handoffs 12 and 17 with their latest provenance corrections. P1-B is a separate research lane correcting evidence claims; do not take it over or consume its unaccepted causal claims. Proceed independently with P1-C using the already accepted group research.

1. Add per-title PS4 competition-group catalogues and integrate them into existing league/team membership views and supported edit paths. Keep stockleagues identity, league membership and group namespaces distinct. Reuse existing gen7 machinery where applicable; do not build a second group editor.
2. Bind catalogue selection to title/schema/source provenance. Use the real NHL 20 fixture and correct NHL 19 base/patch sources. Every mapping carries its evidence level and source. Names resolve through that title's dictionary; absent dictionaries or keys fall back to raw IDs/keys with a clear reason.
3. Preserve unknown and inferred mappings honestly. NHL 17's executable array order is not its runtime ID order; NHL 18 Spengler mappings are inferred; IDs 77/78 remain unnamed. Do not silently promote sibling-title labels or inferred relationships to confirmed mappings. Browsing and unrelated edits must work on existing unknown values. Reject newly invalid assignments only where the target relationship is established; an incomplete catalogue must not manufacture referential errors or 'repair' existing data.
4. Where league/conference/division reassignment is already supported, integrate validated group choices, pending changes and undo/redo. Merely existing in a catalogue is not proof that a group is a valid conference/division for a particular league; preserve the correct role/parent relationships or keep the unsupported assignment read-only with its reason. Do not infer schedule rewriting from group changes.
5. Verify each PS4 title (15–21) plus relevant NHL 19 patch provenance. Cover confirmed mappings, raw unknowns, inferred disclosures, missing dictionaries, wrong-title/schema fallback, refused assignments with unchanged bytes/history/revision, and accepted edits through pending/undo/save-to-copy/reopen/restore. Rerun gen7 membership regressions to preserve existing behavior. Use fixtures and disposable copies only; include focused UI tests and regenerate/check bindings as needed.
6. Update per-target coverage and user review cards. Separate offline implementation checks from game acceptance. Return a plain-language-first receipt with exact tested scope, blockers, changed identities and cleanup, then stop for coordinator review.

No live deployments/image writes, game launches, quarantine, staging, commits or pushes. Reuse the shared cache; clean obsolete owned scratch with verified containment and keep active recovery material. Do not start P1-D or Phase 2 during this package.

## Roadmap

P1-A: bounded offline implementation accepted; visual/runtime cards remain for the user. P1-B: research correction active, rule storage/consumers unresolved. P1-C: next developer package, independent of that correction. LOC-W3 contract accepted for later P1-D implementation. Assets/audio remain later Phase 1 work; Projects/Builds remain stretch.
