# Research lane review and next dispatch — 18 September 2026

Disposition: accepted as useful research input, with the corrections below before treating the atlas as current cross-title authority. It is not complete research coverage of all roadmap phases, game acceptance or independent verification of every probe. Continue the research session if available. Developer OVERNIGHT-DEV-R2 remains active and unchanged; this dispatch assigns no application changes to research.

## Review evidence

Codex read the atlas/status, chunklzx handoff/vector README, PS3 container cross-check and relevant schedule/localization material; compared the chunklzx claims to `crates/tdb-archive-codec/src/lib.rs`. The current walker does derive window bits from the chunk-size word and pass the prefixed payload directly to LZX, with the reported stepping alignment logic. These are credible implementation leads. Decoder output hashes were not independently rerun in this review.

The vector README explicitly says its texlib rows predate the final-frame rule, including a zero-padded 200704-byte result and a 196608-byte unpadded result. It instructs readers to rerun for current hashes. Handoff 40 describes final full-length output without padding, so these artifacts are not yet a single consistent regression reference.

Current E: free space measured by Codex was 5,997,043,712 bytes (about 5.6 GiB), below the earlier 7.6 GB report. No attribution to either lane is established by that measurement; do not delete another owner's builds or runtime recovery sources.

## Research corrections — R1

1. Regenerate the small chunklzx specimen set with the final decoder. Publish one machine-readable vector manifest with full archive/input/output SHA-256, exact source/member/offset, input/output lengths, chunk/subframe counts, final-frame sizes, decoder script identity and plaintext comparison where available. Separate legacy padded/incomplete results as superseded evidence. Ensure README, handoff, sibling hash file and manifests agree. Reproduce the three plaintext matches and the final texlib lengths without padding; do not claim family-wide decoder qualification from Legacy/NHL 12 samples. Add bounded NHL 11/13/14/15 samples where available, covering multiple chunks and terminal-frame variants. Retain compact extraction instructions rather than decrypted executables or large archives. Deliver required decoder behavior and malformed-input cases; developer implements after R2.
2. Reconcile schedule semantics with developer receipt 04: the proposed slot resolution is rank within a league by the team's global `index`, not the raw global index. Check a non-NHL league and document the exact ordering/evidence and any exceptions. Correct handoff 12, atlas and status; do not leave contradictory formulas for the next developer.
3. Narrow absence/equivalence claims. Audio filename scans establish no separately league-named bank among inspected paths, not absence of league-related sounds inside generic banks; PS4 remains unknown. A PS3 header/tree scan establishes observed layout/member families, not byte-identical contents or original-disc provenance for every loose file. Keep known PS3/Xbox dictionary differences explicit, not hidden behind a blanket identity claim. Scope NHL 07 negative findings to inspected data. Cite actual source hashes for any stronger assertion.
4. Describe `_compiled` as a discovery/path convention supplied by the user plus the specific inspected-instance observations. Do not generalize absence in the inspected Recompiled installation to other builds or demand a user investigation before continuing shared Xbox format work. Publish supported candidate root layouts and provenance; only the developer changes discovery.

## Continue useful research after corrections

Use the existing lane ownership and exclusive claim protocol. Remain in research-owned paths, read application code only, and publish incremental handoffs without waiting on developer R2. Focus on unresolved Phase 1 parity first:

- Actual PS4 league-logo members and payload variants, PS3 DDS and Xbox RW4 package examples, native archive/compression and size-change constraints. Give developer exact read/replace/repack candidates and validations, not just filename grammars. Distinguish archive-backed from loose-only routes and leave loader precedence as a runtime question.
- Supported source-schedule editing/import/generation requirements: table/slot bindings, membership dependencies, invariants, fixed-capacity limits, mode-specific copies and remaining unknowns. Do not equate one NHL example with all tables.
- PS4 league/group-related data and audio consumer leads; inspect generic event/bank references where practical rather than using member-name absence as closure. Static inspection only; clearly separate executable table discovery from tracing a consumer.
- Localization operation/locale coverage and constraints per source-verified family; identify unmeasured targets and title-specific alphabets. Distinguish fixed allocation constraints from a claim that no safe future growth algorithm exists.

When these are externally blocked or sufficiently handed off, continue Teams, Players, Equipment and Miscellaneous research in roadmap order, filling tables/fields/assets/consumer links rather than only counting thumbnails. Skip actively claimed investigations; choose independent remaining work. Do not re-research Recompiled's identical Xbox formats or touch converter-owned implementation.

No visual reviews, emulator/game launches, live modifications, fixture-registry edits, staging, commits or pushes. Leave visual/audio questions in the consolidated morning queue; the user is the acceptance authority. No immediate user decision is needed for the above work. Do not notify or wait at every handoff; use STATUS and claims.

Before new extraction, check space and bound output. Clean your own obsolete scratch/dependencies only after confirming ownership and ended dependencies. Keep necessary small vectors/probes durable; do not trigger a full game extraction or create another workspace build cache. Report remaining scratch with exact purpose/owner.

At completion or usage limit, provide a plain-language summary, correction dispositions, freshly executed checks versus inherited results, full vector-manifest identity, new handoff list, unresolved gaps and cleanup. Research readiness does not close the developer's Phase 1 gate. The developer should consume the corrected codec handoff after its active write-integrity corrections are reviewed, not mix unrelated codec work into R2.
