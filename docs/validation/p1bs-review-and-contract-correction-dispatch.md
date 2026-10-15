# P1-B-SETTINGS review and next dispatch

Research session: continue the same session for the bounded corrections below. The catalogue inventory and four-site decoding finding are usable research inputs. The defaults writer contract is not yet accepted. P1-C-R1 remains the active developer correction; do not start a second implementation lane.

## Review evidence and disposition

Codex read handoff 22, the catalogue JSON and `tdb-defs` decoding/encoding implementation. The four reported sites are absent from `OFFSET_BINARY_SITES`. The current branches produce the reported wrong values: minimum raw 8 at four bits becomes -8 rather than 0; id raw 153 becomes 153 rather than 152; parent raw 37 becomes 37 rather than 36; subgroup order raw 0 becomes 0 rather than -1. This is a source-level confirmation, not a fresh compiled-parser run or independent repeat of the extraction probes. Research's arithmetic and executable witnesses justify a scoped correction; they do not establish a general rule for every negative-range field.

The evidence is useful but the summary overstates agreement: id agreement is 222/223 and all-six-field agreement is 186/223, not universal executable confirmation. Preserve those exceptions. The cross-title catalogue locations are leads with the recorded platform and source coverage, not proof of consumers or exhaustive absence elsewhere.

Two contract issues need correction before a writer consumes it:

1. Section 2 says a missing context means the global default applies, and promises the default offered when a screen opens, despite explicitly unknown precedence and consumption. Describe a stored catalogue default; retain the unknown effect. Section 5's executable-only assertion also exceeds the inspected-source evidence.
2. P1BS-1 treats unchanged candidate save fields as exclusion of that storage. Different encoding, deferred serialization, another context or an unapplied treatment remain possible. A matching numeric change supports a candidate association, not a complete serialization binding. Keep both verdicts bounded and specify the next discriminating control.

## Next Claude research prompt — P1-BS-R1

Continue this research session. Read this review and handoff 22. Work only in research-owned paths and its coordination claim; do not change application code, definitions or bindings.

1. Correct the summary's executable agreement and enumerate the exceptions from the existing JSON. Replace guaranteed default/precedence/consumer wording and exhaustive executable-only absence claims with the actual observed scope. Preserve the useful mapping; no broad new survey is needed.
2. Finalize the narrow defaults contract. Explicitly enumerate eligible numeric Phase 1 rows; exclude text/name rows, unresolved code bounds and ambiguous target identities. Preserve existing unrelated dangling joins and anomalies; reject newly introduced defects and ambiguous target selection rather than requiring a pristine stock database. State how a second valid edit remains eligible. Keep inferred labels visibly inferred alongside raw codes. Verify only the selected cell's bits plus individually accounted checksum bytes may differ, including other cells within its record.
3. Rewrite P1BS-1 with bounded positive and unchanged outcomes, placement/profile controls, and a second distinct treatment before proposing a selected-value binding. Leave all execution with the user. Neither static patch changes nor stock correspondence establish game consumption.
4. Supply a small four-site metadata applicability table: raw table/field IDs, bit widths, declared ranges, occurrences across available metadata, and witnessed versus unverified variants. Recommend the narrowest safe decoder scope. Do not generalize all negative ranges or repeat executable extraction already supported by evidence.
5. Reconcile the contract, operation table, tests and summary; update atlas/status and deliver a plain-language-first receipt with exact checks and cleanup. No launches, live writes, staging, commits, pushes or retained large scratch. Stop for review after this bounded correction.

## Queued developer package — P1-BS-1

After P1-C-R1 is reviewed and closed, take a focused decoding/encoding package before the catalogue reader or writer. Use the four-site applicability evidence; reproduce wrong decoding through the real parser on NHL 15 PS4 base and available patch fixtures. Fix only supported field/schema sites. Test raw-to-logical-to-raw identity, negative and sentinel boundaries, unchanged unrelated signed/sentinel fields, existing offset-binary sites and cross-title metadata collisions. Do not alter global negative-range semantics or enable a defaults editor. Report base/patch coverage and private-fixture skips honestly. This is queued scope, not permission to interrupt the current correction or run a concurrent developer.

P1-BS-2 (catalogue reader) follows that gate; the one-cell writer follows acceptance of the corrected contract. NHL 16–21 CSV packaging and gen7 executable-backed equivalents retain explicit per-target dependencies, rather than being declared permanently unsupported.

## Roadmap

Phase 1 remains active. P1-A is accepted offline; P1-C-R1 is the active developer correction. Settings research now identifies concrete default representations and a scoped parser defect; settings editing remains pending. LOC-W3 is accepted as a contract, with implementation still queued. NHL21-E1 remains accepted and closed. Phase 2 onward follows Phase 1 parity work; Projects/Builds remain final stretch work.
