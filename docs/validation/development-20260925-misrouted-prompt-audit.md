# Developer direction audit — 25 September 2026

## Disposition

Continue the same developer session for correctness work first. The misplaced prompt correctly targets Studio, prioritizes write-affecting conversions, preserves user files and requires title-scoped equipment labels. Those directions are consistent with this project. Keep the work; do not roll back packages wholesale.

Two prompt instructions need replacement: its ban on writes to “application paths” contradicts the implementation assignment, and its finite equipment queue does not restore the original League/Team-first extended work selection. Application-source changes inside the Studio working tree are authorized for this developer; installed applications, games, anchors and user originals remain protected. Later-phase work was allowed as fallback, but newly ready League/Team work takes priority after correctness fixes.

This audit inspected the pasted prompt, receipts 11/13/17/19 and campaign ledgers/closeout, current weight/rating/cap-hit source and associated tests. No tests were run in this audit; no production files changed. It is not independent acceptance of packages 01–19, the reviewed texture writer, dependency selection or audio implementation. No successor package receipt was present in this campaign directory at inspection time. The index is empty.

## F1 — P1: interpretation is derived from editable values

`assets.rs::probe_weight_base` recalculates a median of current nonzero weights: below 60 selects +150, otherwise +120, with an unqualified +120 fallback for empty input. `view.rs::sixbit_rating_offset` samples current rating cells and selects +36 only when more than 1% exceed 49; otherwise +50. These results feed display and write conversion.

These thresholds may distinguish stock fixtures, but cannot identify the encoding of arbitrary modded rosters. For example, in a known +120 file a roster of 170-lb players has raw weights 50 and would be interpreted with +150. In a known +36 file lowering ratings until <=1% of sampled cells exceed 49 changes the inferred offset to +50. The stored format did not change. The rating function's comment that ranges are disjoint is also incorrect: the raw ranges overlap. This is a code-inspection finding, not a freshly executed public-route reproduction.

Replace content-driven authoritative selection with a stable, provenance-backed encoding contract keyed to the verified title/platform/schema/version as required. Schema identity alone is insufficient where identical schemas carry different meanings. Resolve unknown/contradictory sources explicitly; do not silently choose a default or learn the encoding again from edited values on reopen. Merely caching the first heuristic result in memory does not solve reopen or imported-file behavior.

Before fixing, add red/green regressions on disposable known-source files for threshold-crossing bulk edits/import, sparse/all-zero populations, records reordered across the rating sample window, unrelated edits, undo/redo, save/reopen and the same human-value reapplication. Verify display and all writers/import lowering use one resolved contract and preserve unrelated bytes. Keep measured offsets distinguished from inferred family mappings; do not invent corroboration for uncertain NHL 08–10 semantics.

The foreign prompt's request for ambiguous-population tests is good, but an explicit fallback from the same mutable distribution would not close this architectural defect.

## F2 — provenance and contract-dollar evidence

The test sweeps enumerate 27 committed anchors, including the known NHL 19 base copy under `ps4/nhl20`. Do not describe these as 27 independently verified title/platform implementations. Add the real NHL 20 fixture with source/meta hashes and explicitly distinguish base/patch fixtures and Recompiled adapter coverage.

The “28 public cap hits” test is repeated observations of a small number of players across anchors, not 28 independent contract references. Receipt 17 explicitly infers NHL 08 from neighboring games/field width while its summary removes uncertainty on NHL 08–10. Preserve that distinction. Public salary, base salary, AAV/cap hit, contract year and authored game data need not coincide; remove the asserted explanation for EA's NHL 08 $7M value unless sourced.

`cap_hit_scale` currently generalizes by bit width, including every width >=13. Bound supported encodings to witnessed contracts rather than width alone. Pin independent source references for multiple identified records per family, and preserve raw/uncertain presentation where semantics remain unresolved. Do not automatically repair older user files. Determine affected operations from code history/receipts, acknowledging that the current uncommitted tree may not support precise build-version attribution.

## Equipment decision affirmed, with one boundary

Scope labels by title and where needed platform/version/source fit. Shared definitions are fine with measured equivalence. Unknown stored ids remain raw and survive unrelated edits. Separate schema-valid ids, observed ids, asset-backed ids and named ids.

Do not turn “not in the shipped catalogue” into proof that an id is invalid: modders may add assets. Refuse values known incompatible or disallowed by the operation's contract; for unknown candidates use the established raw/asset-validation workflow with explicit uncertainty, rather than silently borrowing labels or banning all non-stock values. A label association alone cannot authorize a write.

## Corrected next developer prompt — same session, extended continuation

Read this audit and `docs/plans/overnight-development-2026-09-24-leagues-teams.md`. This audit replaces conflicting instructions in the misplaced shadPS4-project prompt. You are authorized to edit Studio source, definitions, tests and generated bindings within this repository; do not edit installed applications, games, anchors, user originals or research-owned evidence. No staging, commits, pushes or launches.

First reproduce and fix F1 with stable encoding resolution and public display/edit/import/save/reopen tests. Resolve F2's provenance and uncertainty handling in the same bounded sequence. Preserve previous correctness fixes; do not restore the old rating-driven weight heuristic. Publish a plain-language-first checkpoint with actual tests and exact remaining uncertainties; continue without routine review stops.

Then implement title/source-scoped equipment resolution across the existing stick/glove/helmet surfaces, honoring the unknown-id distinction above. Do not expand Phase 4 breadth before reviewing newly actionable League/Team dependencies.

Refresh the completion ledger against corrected research through R88–R95 and the coordinator's `research-20260925-misrouted-prompt-audit.md`. Consume ready ABKC/AST inspection and diagnostic contracts within their measured populations; do not consume the unsupported chant-selection or silent-placeholder wording. Confirm the latest correction claims before using them. New codecs need their own documented decoder decision; format research does not qualify an encoder or writer.

Prioritize remaining League/Team user-facing integration and parity over gloves/helmets if ready dependencies now exist. Team re-key and creation remain unfinished contracts; Xbox mipped resize remains refused. If all actionable League/Team work is blocked, resume Phase 4.2 gloves then 4.3 helmets and later roadmap items under the original nonblocking queue. Record exact blockers and coordinate active research claims rather than waiting or duplicating work.

Use disposable copies, meaningful per-target tests and generated bindings. Exclude zz_diag explicitly. Preserve shared caches and unrelated dirty files. Clean only owned scratch after evidence is saved, retain artifacts only with a named dependency, and finish with exact coverage, remaining blockers, morning cards and one next task. New work stays developer-verified pending coordinator review; user runtime acceptance remains authoritative.

## Roadmap

Players/equipment gained useful coverage and exposed genuine baseline bugs, but conversion stability now takes precedence. League/Team audio research has advanced beyond the developer closeout's stale blockers, so returning to those ready operations is appropriate after correctness. No phase completion is granted by this audit.
