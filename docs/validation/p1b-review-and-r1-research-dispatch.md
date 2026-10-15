# P1-B review — evidence boundaries and coverage correction

18 September 2026. Continue the same research session for P1-B-R1. The source/schema inventory is useful research input. The causal conclusions and resulting implementation/scope recommendations are not accepted as written. Developer P1-A-R2 remains active independently.

## Independent review

Read handoff 21, the inventory structure and the survey's comparison/calendar logic. Counted 29 main-DB source entries and 46 save source/mode records. Of these, 43 have populated `schedule_copy` results; `xbox360-nhl07-DYNASTY`, `xbox360-nhl08-DYNASTY` and `ps3-nhl08-DYNASTY` have zero schedule rows and no comparison in that block, despite populated raw `ULcv` tables. No extraction, emulator or runtime experiment was rerun by Codex.

## F1 — stock correspondence does not establish a causal consumer

The probe compares multisets of `(month, day, home-offset, away-offset)` from stock saves and source tables. This is strong evidence of correspondence; it does not establish the code path that produced the save, row-for-row identity of all fields, response to edited source data, or calendar derivation. The handoff explicitly says no consumer code was traced. Stock tables and executables could encode matching schedules/dates independently.

Replace the claims that P1-A edits are guaranteed to reach new modes, that the copy is a verified consumer, that shorter source tables necessarily copy fewer rows, and that calendar changes necessarily follow schedule changes. Label the measured matches/offsets observed, the initialization/copy/calculation mechanism inferred, and edited-input behavior unresolved. Preserve the useful slot translations, preprocessing differences and named candidate sources within their measured scope. Do not dispatch UI wording promising calendar effects from this evidence. Correct the plain summary's season-end `+1` versus body `-1` inconsistency; neither task's semantic name is established merely by its date.

## F2 — make specimen coverage and comparison outcomes reproducible

Reconcile the all-46 claim with the three empty comparison records. Either add the supported raw-table mapping to the read-only probe and publish actual comparison output for those specimens, or mark them decoded-container-only / schedule comparison pending. Do not equate successful inner-DB extraction with verified schedule contents.

For every compared specimen/tier, record the exact candidate source identity, compared fields, counts, unmatched source and destination tuples (with multiplicity), and separately justified transformations. A multiset intersection count is not full equality or an ordering proof. All-Star translation should be verified by the team mapping and residual comparison, not just inferred from out-of-range IDs. Distinguish regular season, preseason and save-specific rows. Retain legitimate exceptions rather than forcing one universal rule.

## F3 — narrow absence, storage and scope claims

Searching table/field names cannot establish that rules are nowhere in database data, nor that their only storage is profile/settings saves or executable constants. Generic key/value tables, opaque structures and unnamed fields remain possible. Describe the exact inspected schemas/paths and search method; use 'no identified main-DB representation' where appropriate. Settings identifier strings and matching salary-like values are leads, not a traced serialization location or fully mapped editing contract. Keep inferred state labels visibly inferred.

Do not remove league rules from the roadmap or classify their editor as optional solely because their representation may be outside the main DB. Phase 1 includes additional league data; storage location determines dependencies, not user scope. Return the rule-editing path as research-blocked with specific next discriminating work. A future settings/save writer may be needed, but no such implementation is dispatched here. Reconcile group-edit implementation status against the current coverage matrix rather than calling an existing operation unimplemented.

## F4 — make runtime cards distinguish competing explanations

- Keep changed-schedule and changed-calendar cards, but explicitly identify them as tests of currently inferred behavior. Use controlled before/after sources, observed resulting save rows and restoration; no launches in this research package.
- A shorter-season screen alone cannot distinguish `current_records` from `max_records`. Design a discriminating control (including the treatment and contents of spare records) or limit the verdict to what was actually observed.
- Matching a named UI event to an unnamed task date is only a candidate binding when dates coincide; resolve ambiguity with an additional control before calling it a verified type-to-name mapping.
- One Champions League save establishes a participant list and order for that setup, not the general selection rule. Separate those outcomes and propose a second controlled setup only if needed to distinguish hypotheses.
- Treat tournament phase/placeholder interpretations as inferred unless directly supported by a consumer or discriminating specimen. Permit future browsing of observed fields without inventing phase semantics.

## Next prompt and stop point

Execute P1-B-R1: correct handoff 21, inventory coverage, atlas/status and proposed developer packages/cards using the distinctions above. Reuse measurements; only narrowly scoped read-only decoding/comparison is needed to fill the three raw-table gaps if feasible. No new broad survey, app/defs/bindings edits, game launch, live write, staging, commit or push. Clean owned extracts with containment checks and retain only evidence with a named dependency. Publish a plain-language-first receipt with actual observed coverage and remaining inferences, then stop for coordinator review.

Phase 1.2 remains active. P1-A-R2 is the developer lane; P1-B-R1 is the research lane. LOC-W3's accepted contract and NHL21-E1's accepted runtime result are unchanged.
