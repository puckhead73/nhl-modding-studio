# DB-04 — Advanced Team and League Editing

Status: mapped NHL07 Xbox360 organization and source-schedule workflows implemented, 2026-10-08. **PASSED — final user sign-off, 2026-10-11.** All review checks and four presentation/CSV refinements passed. DB-04 is closed for the verified NHL07 Xbox360 scope. [Evidence, mapping limits and exact review steps](../validation/db-04-nhl07-review-2026-10-08.md). Naming/presentation/affiliation mappings remain unqualified.

Target: NHL 07 Xbox 360, editing the project's default game database and any explicitly supported companion files. MS-01/02/03 and supported DB-01/02/03 have user sign-off. Save Editor and other games/platforms require separate qualification.

## Outcome

An author can reorganize existing teams into supported leagues/conferences/divisions, edit supported competition identities and existing presentation assignments, and edit or generate source schedules. Changes must remain coherent through Overview, Undo/Redo, save/reopen, developer project sharing, build, installation and restoration. Native acceptance covers newly created game modes, not merely a successful database write.

## Starting evidence

- Existing League detail, Membership, Schedule, imagery and dictionary tools provide reusable code; their presence does not establish NHL07 acceptance.
- NHL07 has a thin team index and separate stock/custom team data. Metadata includes league, conference and division values. The inventory must identify every authoritative field and related copy before writing them.
- Existing schedule code recognizes NHL07 schedule-length variants and international schedule templates. Home/away values can be league-local slots rather than global team IDs. Correct team resolution after membership changes is a central requirement.
- Older research contains candidate league/group labels, unresolved dictionary handling, and research-only NHL07 PS3 entries. Recheck current code and NHL07 Xbox360 evidence; do not treat those old rows as current support claims. NHL07 has no PS3 target.

## Implementation sequence

| Step | Work | Completion evidence |
| --- | --- | --- |
| DB04-01 — NHL07 support inventory | Trace team membership, conferences/divisions, affiliations, competition identity, schedule variants and presentation references using current anchors and source files. Record schema locations, valid choices, ownership, capacity and suspected hardcoded constraints. | A support matrix distinguishes confirmed mappings, absent records, unresolved meanings and runtime dependencies. Existing tools and old research are reconciled. |
| DB04-02 — Team membership | Complete named league/conference/division selectors and any confirmed affiliation controls. Apply dependent membership values atomically; offer team swaps where a fixed competition size requires them. Preview affected schedule/team references and validate group combinations. | No raw numeric guessing, stale team lists, partial moves or silent reassignment of fixtures. Invalid combinations produce a specific explanation without mutation. |
| DB04-03 — Competition identity and structure | Present the actual league → conference → division → team hierarchy. Support names/abbreviations where NHL07 stores editable values or traced localization keys. Expose shared-name effects and identify fixed versus editable structure. | Correct ownership, readable choices, persistent identity edits and packaging of every changed companion file. Unrelated localization-key propagation remains LOC-01. No fake editor for nonexistent records. |
| DB04-04 — Team Schedule calendar | Add a Schedule subtab to each team's detail page, populated from the selected source schedule variant. Display a navigable monthly calendar with opponent logos/names and explicit home/away labels. Clicking a date opens an add/edit/remove-game dialog. Implement required schedule row insertion/deletion and packaging as part of this feature. Preserve special fixtures and distinguish source templates from mode-local schedules. | Dates and opponents are readable; missing logos have a text fallback. Adding, removing, rescheduling or changing an opponent updates every affected team's calendar atomically. Invalid dates, self-play, unresolved slots and slot bounds refused; unrelated fixtures preserved. |
| DB04-05 — League CSV import/export and generation | Provide Export CSV and Import CSV in the league Schedule view for the selected schedule variant. Use one row per fixture, readable dates/team names and stable team identifiers with explicit mapping for ambiguous inputs. Preview additions, removals, changes and conflicts before applying. Support demonstrated generator constraints such as participants, season length, dates, home/away balance and group matchup frequency. Include necessary schedule-specific row-count/package support. | CSV round trip with no duplicated reciprocal games; all affected team calendars refresh after import/generation. Deterministic generation under tested constraints; cancel leaves no changes; one accepted operation is undoable as a unit. Unsupported layouts/capacity requests are refused explicitly. |
| DB04-06 — Shared fixtures and membership consistency | Represent each game once in the league schedule and derive both teams' calendars from that fixture. Keep intended opponents stable when local slot order changes. Handle league moves, swaps and regenerated schedules together, with an explicit resolution path for affected schedules. Do not silently regenerate a user's custom schedule. | No separately maintained home/away copies can diverge. Old and new opponents update after a matchup change; Undo/Redo and selective revert restore all affected calendars together. Reordering membership cannot silently change opponents. |
| DB04-07 — League presentation assignments | Qualify existing league logo/imagery and commentary assignments where NHL07 exposes them. Use known labels or traced source references and accurate availability states. | Assignments target the intended competition without replacing a shared file unintentionally. Asset existence, preview support and native playback/rendering are separate results. New imagery/audio authoring remains AA/AUD work. |
| DB04-08 — Project integration and responsiveness | Integrate every supported operation into the write queue, immediate UI feedback, rollback, Undo/Redo, saves/autosave and build barriers. Group league/team/schedule edits coherently in Overview; provide links and linked × reverts. Include required dictionary or structural schedule changes in sharing and release packages. | No stranded edits or partially restored relationships. Large schedule batches remain responsive and produce concise summaries rather than thousands of expanded rows. |
| DB04-09 — Qualification and handoff | Exercise production APIs and UI behavior with disposable NHL07 projects, then prepare a controlled review project/build and one consolidated user verification run. | Automated evidence, exact original/changed teams and fixtures, expected results, applicable game modes and explicit remaining limitations. |

## Calendar and CSV interaction requirements

### Team → Schedule

- Monthly calendar with previous/next month navigation and a clearly identified competition/schedule variant. A team with no fixtures still has an empty calendar and an Add Game path where the selected template supports it.
- If NHL07 stores month/day without a year, expose the season/calendar-year context explicitly for weekday placement and leap-year handling; do not silently assume the current real-world year or imply an unstored year was written to the database.
- Each game shows the opponent logo when available, the opponent name or abbreviation, and **Home — vs** or **Away — @**. Color can reinforce the distinction but cannot be the only indication. Missing logos fall back to text and never prevent schedule editing.
- Clicking any date opens a dialog listing all games on that date. Add Game selects the opponent and Home/Away, with that date preselected. Existing games can be removed or edited, including moving the date, changing the opponent or switching home/away. Cancel makes no changes.
- Multiple games on one day remain visible rather than hiding one. Check conflicts for both participants and explain the specific issue; enforce verified game/competition restrictions without inventing universal scheduling rules.
- A fixture is shared: adding A hosting B appears as Home for A and Away for B. Removing it removes it from both. Changing B to C removes the entry from B and adds it to C. Rescheduling moves both entries. These are single undoable operations, including any required backing-table changes.
- Refresh the affected calendars, league schedule, counts and Overview from the same committed fixture state. Use immediate feedback and rollback on rejected writes, consistent with earlier work packages.

### League → Schedule → CSV

- CSV is the primary user-facing schedule interchange format, usable in Excel or Google Sheets. Export the selected competition and schedule variant, not an accidental mixture of season-length templates.
- One row represents one game, including date fields, home/away team identifiers and readable names. Carry sufficient competition/variant identity to detect an incorrect import target. Use human-facing dates (not internal zero-based date values); finalize the precise versioned column contract during inventory.
- Import previews the selected variant's proposed schedule before replacing it. Show added, removed and changed fixtures, unresolved teams, invalid dates, capacity issues and conflicts. No silent append/merge and no automatic creation of duplicate reciprocal games.
- Match teams through identifiers with names for readability. Require explicit resolution when identifiers or names are ambiguous; do not guess based on row order or unqualified league-local slots.
- Preserve required round/special-fixture metadata on export/import where the selected template supports it. Handle quoting, Unicode, BOM and common spreadsheet line endings. Report errors using CSV row numbers and understandable column names.
- Applying the import is one atomic history operation and refreshes every affected team calendar. Cancelling or failing validation leaves the project unchanged. Generated schedules feed the same shared fixture/calendar model.

## Scope boundaries

- Existing-team reorganization and supported source schedules belong to DB-04. New custom-team creation and full Create-a-Team remain DB-09.
- Existing competition limits must be identified and enforced. Creating extra leagues, enlarging hardcoded competitions or changing executable limits is not promised by a writable database field; document any such dependency separately.
- Affiliation editing is included only where NHL07 has a traced relationship. Do not infer minor-league features from later games.
- Targeted league/group naming includes necessary verified dictionary edits and their release packaging. A general localization glossary and automatic propagation across all keys remain LOC-01.
- Schedule-specific structural packaging, if required for supported generation/import, is DB-04 work. The earlier player-record lifecycle deferral to DB-08 does not automatically defer schedule functionality.
- Existing Dynasty/Season saves may have copied their own membership and schedule data. DB-04 acceptance must use appropriate newly created modes. Restore Default Rosters alone is not evidence that a source schedule was consumed.
- Native rendering/playback and competition progression cannot be certified by parsing or build round trips.

## Automated verification before handoff

1. Resolve NHL07 team/group/schedule keys independently; test slot boundaries, unknown values and correct stock/custom joins.
2. Exercise valid membership moves/swaps and reject invalid combinations without partial writes.
3. Verify fixtures retain intended teams across membership changes; cover special slots and ambiguous imports.
4. Validate supported date variants, leap/non-leap templates, counts, home/away balance and generator constraints. Preserve unrelated games.
5. Round-trip league CSV; test quoting, Unicode, spreadsheet-compatible encodings, wrong variant, ambiguous teams, malformed dates, duplicate fixtures, over-capacity and cancelled inputs. Verify one row per game, not one row per participant.
6. Test atomic Undo/Redo and Overview × for coupled membership/schedule/identity changes while unrelated edits remain.
7. Verify immediate UI feedback, rejected-write rollback, queued Save/Build/Close and responsive large operations.
8. Save/reopen, export/import a developer project, build and activate into disposable game copies, then restore exact original bytes. Include every affected companion file, not only the main DB.
9. Run relevant DB-01/02/03 regressions. Prepare the native review fixture without modifying QATest or the live installation.
10. Test team-calendar month/season transitions, leap-year context, multiple games per date, home/away presentation, logo fallback and day-dialog cancellation.
11. Add/remove/reschedule/change an opponent through the calendar and verify old/new opponents, league totals and Overview update together. Repeat through Undo/Redo, selective revert, save/reopen, CSV import and build/install; verify row insertions/deletions survive packaging.

## One-pass user review to prepare

Exact teams, fixture dates, option labels and modes will be selected from the completed inventory and supplied with the review build.

| Check | User actions | Expected result |
| --- | --- | --- |
| Team organization | Apply a supplied conference/division change and a supported league move or swap. | Team and league views agree; controls remain named and responsive; affected schedules are handled explicitly. |
| Competition identity | Change a supported league/group identity and inspect any shared-name notice. | Correct project labels, coherent Overview and no unintended renames. |
| Team calendar | Open Team → Schedule; navigate months and inspect a home and away fixture. Click an empty day to add a game, then open the opponent's calendar. Edit the date/opponent and remove the game. Cancel one day dialog. | Opponent logo/name and Home/Away are clear; all affected calendars update together; removed fixtures disappear from both teams; cancel changes nothing. |
| League CSV and generation | Export the selected league schedule to CSV, edit supplied fixtures in a spreadsheet and import with preview. Cancel one import, reject one invalid file, then apply a valid import and inspect both teams' calendars. Preview/apply a supported generated schedule. | One CSV row per game; clear changes/errors; no changes on cancellation/refusal; accepted imports/generation populate all affected calendars without duplicates. |
| History and persistence | Undo/redo, selectively revert a linked change, save/reopen, export/import the project. | Entire operations remain coherent and unrelated work survives. |
| Presentation | Exercise supported league imagery/commentary assignments. | Correct reference status and expected available previews; native behavior assessed separately. |
| Native competition | Build/activate, restart NHL07 and start the specified new Season/Dynasty/tournament mode. Check membership, standings groups and opening fixtures; simulate through a supplied checkpoint. | The game consumes the intended source changes and the tested competition progresses correctly. Untested full-season/playoff behavior remains unverified. |
| Restore | Deactivate, restart and create a fresh comparison mode. | Original memberships, identities, schedule and presentation return. Previously created saves are not expected to rewrite themselves. |

## Completion rule

Complete the implementation and all available automated verification before one consolidated handoff, following the established work-package workflow. Request user action only for an actual dependency, such as releasing the executable for replacement. Final reporting must identify any required functionality still blocked, absent or unqualified rather than silently redefining it as completed. DB-04 closes only after user acceptance of the explicitly supported NHL07 Xbox360 scope.
