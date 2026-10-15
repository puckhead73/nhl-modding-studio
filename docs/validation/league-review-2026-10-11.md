# NHL 07 Xbox 360 — Leagues review, 11 October 2026

Status: implemented portions are ready for user review. This is a direct-observation task, not the start of DB-07. Structural expansion and unresolved presentation mappings remain open.

## Implemented

- Identity uses the project's own dictionary for NHL and National. Conference/division labels resolve through the same dictionary. Other competitions retain the existing stock-membership labels instead of inventing dictionary bindings.
- Competition names and presentation lists the NHL name, two conference names and six division names, or the National name. A read-only batched lookup shows the current dictionary, including pending edits. Edit text opens the existing dictionary editor. Existing dictionary edit/save limitations remain unchanged.
- Competition structure is a team table with league, conference and division selectors. Apply commits a same-league change; Cancel discards the row draft. A cross-league change requires choosing a team to exchange and uses the existing atomic membership/schedule swap. All-Star teams remain protected.
- Removed the three routine notes beneath league logos. Actual read/replacement failures, missing-file status and shared-logo warnings remain visible.

## Evidence and unresolved research

The anchor nhl_eng_us.db has 3,952 rows. NHL and National resolve, as do WesternConference, EasternConference and Northeast/Atlantic/Southeast/Central/Northwest/PacificDivision. The executable literal block near 0x820b3650 following FE::CLeagueIonAdapter contains NHL, NATIONAL, CHTM_LEAGUE_SWEDISH, CHTM_LEAGUE_FINNISH, CHTM_LEAGUE_GERMAN and CUSTOM TEAMS. Literal ordering is a hypothesis, not a traced code-to-key consumer. Code 5 is not assigned CUSTOM TEAMS: its stock member is DAV, and no such binding is established.

The three European CHTM keys are absent from this dictionary by key and hash. The optional read-only main.big probe traversed supported BIG members and RefPack decompression without finding its listed text needles. This narrow string scan does not rule out hashes, other encodings, resources in other containers or executable-generated labels. Reproduce with cargo run -p studio-api --example nhl07_league_dictionary; current output is target/nhl07-league-dictionary.log. Earlier evidence: docs/research/overnight-2026-09-18/10-phase1-league-identity-xbox360.md.

Remaining work:

1. Trace XEX references/consumers of that literal block and hash references in the remaining ION resources; compare alternate locale resources if available. Establish European names and the actual screens reading each NHL/National/group key before claiming in-game rename coverage.
2. Trace league-logo and commentary consumers. NHL07's logo index is currently inferred from package counts, not a verified editable assignment. This pass does not qualify new logo mappings or commentary editing.
3. Establish league/conference/division lifecycle and whole-division reassignment. NHL07 carries membership on HqzU.xMjj (3 bits), rasz (1 bit) and xgeX (3 bits), with no editable league/conference entity table. Both conference values are used; the supported NHL conference/division pairs are fixed by stock membership. Adding/removing records or changing the grouping needs executable consumers plus standings, schedule and playoff behavior researched, followed by persistence/build support and runtime tests. Team reassignment is implemented; adding/removing leagues/conferences and moving an entire division between conferences are NOT implemented.

## Automated verification

- 16 UI tests pass across NHL07 league review, LeagueDetail and LeagueMembershipCard: table Apply/Cancel, valid division choices, cross-league destination selection, refusal preservation, protected All-Stars, batched name reads, unresolved competitions, normal logo notes hidden and real errors retained.
- Six existing native DB04 regressions pass: calendar changes, CSV, schedule variants/generation, membership swaps, package/install/restore.
- Additional anchor-backed native test passes: actual dictionary names and evidence, group choices, unresolved European bindings, no edits from lookup, pending rename visibility and restoration after clearing it.
- TypeScript/production UI build and desktop build checked for this review. Compiler warnings retained in target/league-review-*.log.

## User review

1. Open an NHL07 project, then Leagues > NHL. Confirm the Identity card has the name/team count and Competition names shows the league, two conferences and six divisions.
2. Click Edit text on a division. Confirm the editor opens with the corresponding dictionary key. Cancel/clear any temporary dictionary change before continuing. This is Studio presentation verification; no new in-game rename compatibility is claimed.
3. In Competition structure, change one ordinary team's conference. Confirm division choices follow it. Click Cancel and verify the original selection returns. Repeat, Apply, then Undo and confirm restoration.
4. Change an ordinary team's league. Confirm Swap is unavailable until another team is selected. Choose a team, Swap, verify both memberships and schedules, then Undo. All-Star rows must remain protected.
5. Open National and a European competition. National should show its dictionary text; unresolved European name bindings should say so rather than claim a resolved mapping.
6. Check League logo: routine bullet notes are gone; genuine missing-file or read errors still display.

User acceptance: pending. No live game files were deployed by this task.
