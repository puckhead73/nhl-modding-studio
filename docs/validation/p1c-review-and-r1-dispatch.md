# P1-C review and R1 correction dispatch

Continue the same developer session. P1-C remains open for the corrections below. P1-B research remains separate.

## Independent checks

All eight `studio-domain --test ps4_groups -- --include-ignored` cases passed with `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures`, including real NHL 20 and NHL 19 patch fixtures. All four LeagueMembershipCard component tests passed. Broader suites remain developer evidence. Two temporary, memory-only anchor probes reproduced F1/F2; their source and owned build outputs were removed. No live writes or launches.

## F1 — P1: valid league IDs still form invalid parent combinations

`ResolvedGroups::validate_membership` checks each conference/division against its league independently. There is no division-to-conference relationship. An independent NHL 21 probe called `set_team_membership(BOS, 0, Some(6), Some(10))`: it succeeded, changed the revision and assigned Western Conference plus Atlantic Division. The catalogue itself names those IDs. Pool slots are similarly accepted independently, and conference IDs are allowed in the division slot for every team based on All-Star exceptions.

Encode per-title witnessed parent relationships and explicit exceptional arrangements. Filter division options by selected conference and reject newly invalid combinations in the backend. Do not generalize one All-Star arrangement to ordinary clubs, or allow cross-pool triples merely because both pools belong to the league. Preserve existing unusual values for browsing/unrelated edits; where a relationship is unresolved, disclose that limitation and do not invent a validated choice. Include domain/API/UI regressions for the reproduced case, legitimate pairs, pool consistency, special rows and unchanged-on-refusal. Audit gen7 behavior without substituting PS4 assumptions.

## F2 — P1: row fit is not reliable title provenance

`FitReport::accepted` uses `fit > conflict + misplaced` and ignores unknown rows in that comparison. An independent probe opened the NHL 21 anchor with these claims:

- NHL 16: accepted, 135/433 fit, 99 conflicts, 199 unknown.
- NHL 17: accepted, 147/433 fit, 113 conflicts, 173 unknown.
- NHL 18: accepted, 299/433 fit, 0 conflicts, 134 unknown.
- NHL 19: accepted, all 433 fit (shared structure is not title identity).

The wrong-title test exercises only the NHL 15 mismatch, so the receipt's general guarantee is not established. Separate authenticated/recorded source provenance from compatibility heuristics. A claimed game ID plus majority row agreement must not be described as verified title identity. Use known source identities/registration and a reliable supported structural discriminator where available; explicitly ambiguous inputs must not silently gain another title's labels or evidence. Preserve legitimate user edits and known patches rather than enforcing whole-file stock hashes as the only admissible input. Add cross-title matrix tests including the reproduced NHL 21-as-16/17 cases, legitimate edited files and patch variants; scope fallback/evidence labels honestly where identical structures cannot distinguish titles.

## F3 — P1: membership summary completion still crosses context

`LeagueMembershipCard` guards result display with the generation, but after awaiting `project_summary`, it checks `forTeam === teamId` using the callback's captured team ID. That remains true for the old request after a mounted card switches projects; no current-project guard is checked before `update(summary)`. Use current context refs/generation after the await, as in the corrected schedule card. Add a deferred test: complete the move, leave its summary pending, switch project/team, resolve the old summary, assert no foreign global update or notice. Include late errors and ordinary same-context completion.

## Bake script and provenance decisions

Promote the bake script to a repository-owned script as part of R1: it is the reproducible source of seven maintained catalogues, not disposable scratch. Give it documented input/output paths, deterministic UTF-8 output and a check mode that refuses drift without rewriting. Keep generation decisions (including majority-derived relationships) marked as inference, separate from the traced key lookup evidence. Do not overwrite maintained metadata before a complete transformation succeeds.

Resolve the league-22 wording against each source's actual key and record base/patch differences explicitly; do not use research wording as authority over the witnessed DB. Exercise the known NHL 15 patch fixture if available, or keep that variant explicitly untested. After promotion/review, delete obsolete package patch scripts/logs with containment checks. Do not delete reported P1-A temp directories without checking ownership and any recovery dependency. The long-running unrelated diagnostic is not evidence of a full clean domain run; retain the receipt's precise excluded-test accounting.

## Next prompt

Execute P1-C-R1: fix F1–F3, promote and verify the bake tool, reconcile source notes and run focused affected tests plus gen7 membership regressions. Update the receipt with a dated correction and per-target evidence. No P1-D/Phase 2 expansion, live writes, game launches, staging, commits or pushes. Deliver a plain-language-first receipt and stop for coordinator review.

P1-A remains accepted offline. P1-C is the developer correction; P1-B remains research-owned. NHL21-E1 remains accepted and closed.
