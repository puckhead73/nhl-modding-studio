# R97–R101 coordinator review — 25 September

## Disposition

Continue the same research session. R97 substantially answers the previous direction audit. Retain the archive framing, diagnostic decode and span-census evidence. R99/R101 do not yet establish a rename or complete selector contract. Correct the conclusions below before a developer consumes them, then continue without an intermediate review stop.

Review scope: handoffs 10–13, current queue/checkpoint, R101 probe and published JSON, R99 span-check source. No corpus or decoder rerun, application tests, game launch or live write. This is a focused evidence/contract review, not independent certification of every reported measurement.

## F1 — R99 archive rename exceeds the inspected structure

No overlapping member spans in the inspected gen7 EB archives and matching path hashes are useful facts. They do not establish “name-table string + hash word and nothing else” for an arbitrary rename. Changing digit count/path byte length may change name-table storage and downstream layout. Name references, capacity, ordering/index constraints and destination path/hash collisions need explicit treatment. Absence of shared payload spans is not absence of logical aliases or internal references.

Narrow the result to measured entry fields and per-archive payload spans. Separate a same-byte-length proposed rename from a changed-length name-table rebuild. Enumerate archive-name storage/offsets, entry ordering, duplicate/colliding destination policy and any other dependent structures before proposing either operation. Preserve runtime lookup uncertainty. Keep PS4 renames refused and scope the zero-sharing finding to the six title/platform trees actually inspected (four distinct titles), not all gen7 EB files or future edited archives.

Remove “last unmeasured element” for in-payload references from CHECKPOINT: destination occupancy, other-title coverage, naming/layout rules, transactional recovery and runtime behavior remain separate requirements. In-payload research is still a useful next task.

## F2 — R101 position-set matches do not uniquely establish selectors

The R101 JSON preserves ambiguity that the handoff's “settled” wording loses:

- NHL 15 on both platforms has equal triple matches for `(style, teamartid, variant)` and `(style, teamartid, fevariant)` (657 matches / 683 asset tuples).
- Rink-logo position y matches `style`, `islightcolor` and `isdefault` equally on most inspected trees; all share the set {0,1}. Marginal set equality cannot distinguish their meaning.
- The probe directly compares DB triples to jersey names, not every family. Its `members` count is a set of numeric tuples, so it does not preserve duplicate member/path identities across archives.

Keep the observed filename structure and prior established kit bindings. Identify exactly which earlier independent evidence establishes each semantic slot; otherwise label it a candidate. For new nameplate/rink mappings, provide per-row full-tuple matches with source paths, hashes, unmatched records, alternative columns and multiplicity. Use rows where competing candidate fields differ or another independent reference. A matching jersey/nameplate tuple set supports a naming association, not runtime selection. Do not weaken already established jersey contracts merely because this new probe is less discriminating.

For PS3 NHL 11/12 socks, consult the existing developer kit resolver and R74/R76 measured contracts first. The developer already resolves required sock components on real trees. Distinguish a gap in this probe's regex from an unresolved production mapping; don't restart settled research or discard the four-number residual.

## F3 — keep diagnostic controls and residuals exact

R100 supports the unchanged diagnostic route on three tested Xbox NHL 09 streams. Same counts/RMS/peak after a header rewrite do not prove a decoder ignores those fields universally, and the RIFF sample-count control tests a different field from packet-header controls. Narrow the decoder-internal explanation or substantiate it with implementation evidence. Report the three-vector result, preserve the 87 residual middle blocks, and do not generalize structural acceptance from selected decode success. Byte-identical PCM comparison would strengthen the particular rewrite control without proving universal behavior.

R96 has 12 of 14 loud PCM frames meeting the stated >=0.95 threshold; two informative exceptions remain. “Match established on loud frames” is too broad. Carry those two exceptions and the quiet/uninformative population into the consumption contract. Keep candidate start assembly diagnostic. This requires precise residual reporting, not another wholesale census.

## Next prompt — correction checkpoint and extended continuation

Read this review and the original overnight research brief. Correct F1–F3 in research-owned handoffs, queue, claims and checkpoint, preserving raw evidence. Make the active developer queue unambiguous: replace superseded consumption wording in current rows with links to preserved historical text, repair its duplicated row 6 and separate current instructions from dated history. Publish a short checkpoint, then continue automatically.

Next inspect in-payload references for team-art/kit members on Legacy Xbox and PS3. Use the actual container/codec reader and enumerate parsed reference fields where possible. A negative string/hash search is only a scoped negative result, not proof that no references exist. Preserve path/hash aliases and destination occupancy in the team-art namespace. Integrate the new evidence into an inspection graph; do not promote a writer contract until the independent rename/layout/recovery gaps above are closed.

In parallel with the work sequence—not another application writer—coordinate research claims with the developer. Reuse its existing sock/component evidence and test discriminating nameplate/rink hypotheses. If these require runtime, write a bounded card and proceed to another ready League/Team blocker. Extend archived audio only where it answers a specific residual; avoid repeat sweeps without new predicates.

No application/defs/bindings or source-asset changes, no launches, staging, commits or pushes. Continue the authorized nonblocking overnight queue as usage allows. Cleanup only owned temporary artifacts after preserving concise evidence. New results remain researcher-verified pending review; user runtime acceptance is separate.

## Roadmap

League/Team audio gains archive-backed framing and representative diagnostics. Team-art research now has useful span and filename inventories. Safe re-key/rename and audio replacement remain open; the research is reducing concrete dependencies rather than closing those writers yet.
