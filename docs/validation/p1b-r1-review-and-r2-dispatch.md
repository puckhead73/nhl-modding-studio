# P1-B-R1 review — final evidence presentation and test-card corrections

Continue the same research session for this bounded P1-B-R2 correction. The revised correspondence approach and NHL 15 PS4 settings catalogue are accepted research leads; they do not yet supply a rule writer or runtime qualification. Developer P1-C-R1 remains independent.

## Reviewed evidence

Read handoff 21 revision 2 and the comparison probe. Independently checked the JSON: 46 specimen records; 223 NHL 15 PS4 settings and 373 context-default rows. The NHL 18 GM AHL comparison reports 82 unmatched tuples on each side, but lists only 20 entries each. No extraction or game was run in this review.

## Required corrections

1. **Make the summary agree with the detailed findings.** It still says no representation was found in main-DB schemas and that every one of 46 saves corresponds to its source. Lead with the discovered NHL 15 settings/default catalogue, distinguish defaults from player-selected values, and state 46 processed / 44 with populated schedule comparisons / 2 without a regular-season copy. Describe the 43 metadata-named versus 3 raw-table paths as processing coverage, not 46 successful correspondence checks. Keep all three exception classes; the summary currently says two. Do not call save-specific rows generated preseason without the inference label. Update the suggested UI wording in package P1-B-1 too. Use explicit per-specimen disposition instead of `status = compared` for all specimens regardless of useful copy availability.

2. **Complete the residual evidence or label it honestly.** `p1b_schedule_compare.py` slices both unmatched lists to `[:20]`, while the handoff calls them complete. Store full compact residual lists with multiplicity, or supply a separate complete artifact and label the inline lists as samples with counts/hashes. In particular, the claim that all 82 NHL 18 AHL residuals involve the placeholder needs an exhaustive assertion/result, not just a 20-row sample. Preserve source identities and the field/translation scope. This needs only a targeted regeneration, not a broader survey.

3. **Keep experiment verdicts within the observations.** P1B-1: unchanged output does not prove an alternate source; inactive placement, caching or rejected treatment remain alternatives until controlled. P1B-2: a date that moves in a controlled treatment supports dependency under that setup; an unchanged date is not thereby a fixed constant. P1B-3: refusal in both treatments does not prove packed-count enforcement, and shorter output alone does not uniquely establish direct use of `current_records`; state which hypotheses are supported, which alternatives remain, and the next discriminating control. Require valid CRCs/container structure and known active placement for those conclusions. P1B-4: if task candidates still coincide after controls, keep them ambiguous rather than calling a name bound. No runtime execution is requested.

4. **Do not promote the World Cup slot mapping from a size match.** The table rightly labels rank-based binding inferred, but the package proposal offers it as an established editable team mapping because there are eight slots and eight teams. Keep raw browsing/date work distinct from confirmed team reassignment. Either provide discriminating source/consumer evidence for that order or retain inferred read-only labels and a pending binding test. Likewise date/source correspondence does not authorize semantic phase names.

5. **Refresh implementation references without overwriting developer status.** P1-A is accepted offline. P1-C has now been implemented but is under correction, not simply 'names not implemented'; point at the canonical matrix/current dispatch. Do not portray group-ID existence alone as sufficient parent validation. P1-B itself remains research, not an implementation completion claim.

## Next prompt and stop point

Execute P1-B-R2: reconcile the summary/status records, publish complete residual evidence, narrow card verdicts and the World Cup binding proposal, and return a short plain-language-first receipt. Preserve the new settings catalogue as the next research lead; do not expand into a settings writer or a new broad investigation during this correction. Research paths only, no application/defs/bindings edits, launches, live writes, staging, commits or pushes. Clean any owned temporary extracts with verified containment. Stop for review.

Roadmap: P1-A accepted offline; P1-C-R1 developer correction; P1-B-R2 research correction. LOC-W3 contract accepted, implementation queued. NHL21-E1 stays closed.
