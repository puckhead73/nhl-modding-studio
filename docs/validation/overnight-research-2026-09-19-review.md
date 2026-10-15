# Overnight research review — 19 September 2026

Use a fresh research session at this campaign boundary. The inventories and vectors are useful research inputs, but several proposed rules and runtime verdicts need the bounded corrections below. This is not blanket acceptance of handoffs 02–06 as writer contracts. The developer's current STATUS now reports D4 complete, so this review is input to its subsequent independent review, not an assumption that D4 is still active. Do not interrupt or overwrite developer work.

## Checks performed by Codex

Read the campaign STATUS and handoffs, concentrating on packaging preservation and proposed operations/cards, and inspected the packaging/encoder JSON. No source extraction, codec execution, image review, game launch or implementation tests were repeated. Preview appearance and research-run round trips retain their researcher-verified evidence level.

The published data contains 118 EB records: 11 gen7, 17 NHL 15 PS4, 90 later PS4. Exactly **four** have a declared-size mismatch: NHL 15 `game20.big`, `inxp20.big`, `patch_inxp00.big`, and NHL 16 `game_90.big`. The encoder data contains 18 inner RefPack, 8 outer RefPack, 12 other RefPack and 30 chunkzip vectors. The package array has 129 entries; reconcile its per-case population with the prose's 116 measured-package claim, rather than assuming the array length is the same measurement denominator.

## Findings to correct before consuming proposed rules

### F1 — RefPack placement must use actual encoded size and measured slot capacity

Handoff 02 §5 says no same-dimension replacement fits its slot. Three of its own unchanged-content outer re-encode vectors contradict that universal claim:

| Vector | Growth | Available slot slack |
|---|---:|---:|
| NHL 10 Xbox leaguelogossmall/l0 | 25 B | 38 B |
| NHL 09 Xbox leaguelogossmall/l0 | 15 B | 39 B |
| NHL 08 Xbox leaguelogossmall/l0 | 15 B | 30 B |

Moreover, compression of stock input does not bound arbitrary edited pixels. Size change is not synonymous with relocation. Keep the useful 38/38 larger-stock-reencode observation; remove the universal relocation conclusion. Decide placement from each actual encoded replacement, safe reusable gap bytes and surrounding spans. Do not skip valid fit-in-slot cases or require an encoder-ratio improvement before offering them.

### F2 — Preserve source anomalies; do not infer safe growth from counts or gap heads

Correct “tenth”/“all 10” anomalies to the four identified records, or publish the additional distinct sources if another population was meant. Zero hole heads do not establish that every byte of those holes is zero. A stale declared-size archive can support a bounded preserve-length operation only after full relevant layout/span checks; one shipped specimen does not justify unconditional acceptance of every mismatched archive of that version.

The BIG4 fit rule says to zero-fill to the alignment boundary while acknowledging a nonzero gap in a source. Never erase unexplained gap bytes or treat them as free capacity. Define safe capacity from verified bytes and overlap constraints; preserve unknown bytes or refuse/relocate. Inner-package re-planning also needs source-derived preservation rather than assuming all unmeasured gaps are zero. Separate observed counts, checked spans and proposed policy.

### F3 — Runtime cards still make unsupported causal exclusions

Rewrite R2-BIG4-1: both stock-looking outcomes do not exclude placement/caching/rejection; a successful unpatched-sidecar run does not prove the sidecar is never consulted. Rewrite R2-LZX-1: unchanged art without a source-selection control does not prove the replacement wrapper was consumed, and a failure is not uniquely wrapper rejection. Preserve positive observations but state their scope and next discriminating same-title control.

Similarly, R3's both-changed outcome does not alone identify shared-block loader behavior; mapping aliases or another consumer remain possible. R4 audio differences between different leagues/setups support an association, not proof of the named event firing without controlled isolation. No new visual tests are required now; these are corrections to future cards.

### F4 — Bound inventories and research leads to what was inspected

Handoff 05's “no text anywhere” exceeds its actual search: members no larger than 8 MB, nested traversal two levels, selected strings/encodings and named loose trees. Record skipped size/depth/format/encoding cases and say no matching source was located there. Do not promote a 13-specimen texture result to every texture on the title without coverage evidence.

Handoff 03 registries support absence of a league-keyed tile/banner entry in those registries, not product-wide n/a while `.gfx`, executables and other consumers remain unsearched. Audio identifiers are important evidence against the old blanket absence claim, but event→sample→codec bindings remain unresolved. A discovered fixture does not make a developer operation “tested”; update research availability separately.

Handoff 06 finds collection names and asset paths in `gccollections.bin`; that is a valuable candidate resolver. Without decoded records/references it does not establish where the team/arena/jersey-to-slug binding resides. Label that binding unresolved, with `ErtS` the next bounded investigation. Do not authorize arena id editing solely from catalogue membership: the field→catalogue relation and per-field applicability still need evidence.

## Immediate guidance for the developer review

Do not rewrite a completed D4 from this research receipt alone. Audit its claims against F1/F2 and actual tests. Preserve shared-span siblings through explicit private-copy or group-scoped operations; account for `.big`/`.bh` and embedded-index dependencies atomically, or refuse unsupported cascades. The coordinator's prior compression and paired-settings constraints still apply. Research evidence does not independently qualify any new writer, unknown header word or game loader behavior.

## Next Claude research prompt

Start a fresh research session. Read this review, the 19 September atlas/status and both overnight lane statuses. Research-owned paths only; no application/defs/bindings/matrix edits, launches, live writes, staging, commits or pushes.

1. Claim a distinct correction item. Apply F1–F4 using existing data first. Publish a reproducible count/placement audit: encoder vector joined to source slot, per-vector fit versus growth, distinct declared-size anomalies, package populations and skipped cases. Do not repeat bulk extraction without a specific missing measurement. Reconcile summaries, contracts, cards, atlas and machine-readable prose; retain measured data and mark superseded conclusions explicitly. Publish this correction checkpoint before continuing so the developer review has a stable input. No intermediate user review stop is needed.
2. Then claim the next unowned Teams research item: bounded `ErtS` collection parsing. Start with NHL 21 and NHL 15, locate record boundaries, string/reference tables, ids and collection membership with bounds and source hashes. Prove several actual links from named collections to asset paths, distinguishing string co-occurrence from a parsed reference. Only then test the proposed DB-id→collection→slug chain. Cross-check all available PS4 variants without assuming identical layouts. Keep unresolved fields opaque and do not propose a writer.
3. If `ErtS` reaches an evidence/tool blocker, publish that boundary and move to the already queued PS4 team-localization/gen7 join comparison; do not spend the session in repeated blind searches. Respect existing developer/research claims and do not duplicate active work.
4. Clean obsolete owned `rev2_edit.py`/`rev3_edit.py` and regenerable scratch after preserving the useful logs/results; keep probes and durable evidence. List exact retained artifacts and dependencies. Receipt begins with a plain-language summary, then independently reproducible checks, limitations and cleanup. Stop for coordinator review after the correction checkpoint and bounded next investigation are delivered.

## Roadmap

Phase 1 implementation has advanced in the developer lane but awaits its own review. Research now supplies useful packaging, non-logo imagery, league-audio and NHL 07 texture evidence; it does not close those product cells. Teams research can continue ahead without advancing the implementation phase. P1-C and NHL21-E1 retain their accepted scopes. Projects/Builds remain final stretch work.
