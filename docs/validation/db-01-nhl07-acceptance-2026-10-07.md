# DB-01 — NHL 07 Xbox 360

Status: PASSED — user confirmed all verifications passed, 2026-10-07. DB-01 is closed for NHL07 Xbox 360. This sign-off supersedes the earlier pending-review notes below. Scope is existing-player editing. New-player creation/duplication/deletion belongs to DB-08. Release/sign and the required free-agent package support remain in DB-01 by explicit user direction.

## Completed work

- Recognized NHL07's draft-year and draft-pick fields in capabilities.
- Exposed Contract Amount in Contract Data, with the witnessed $25,000 storage unit. The editor accepts dollars; $5,000,000 writes 200.
- Validated complete calendar dates and the displayed 99 rating ceiling. Package replay validates the final birthday, avoiding invalid intermediate month/day combinations.
- Routed the player page's Main Team and Jersey Number edits through relational operations, preserving the corresponding roster and international memberships.
- Implemented NHL07's explicit free-agent pool: release from the primary club, retain additional memberships, sign into a vacant club slot, preserve pool order, refuse duplicate releases, invalid destinations and full rosters without partial changes. The original free-agent biographies use primary-club value zero; the UI identifies free agency from the explicit pool, not that ambiguous value alone.
- Added schema-3 package support for pool changes using player IDs. The installer validates the recipient's prior pool, stages roster changes and pool updates, and uses existing transactional installation/restore. Pool conflicts cannot be overridden by the ordinary cell-conflict option. Schema-2 packages remain supported. Qualification is limited to NHL07 Xbox 360.
- Added meaningful free-agency status changes to Project Overview, with player navigation. Table reordering does not appear as dozens of unrelated pool edits.
- Guarded unsupported new-player operations before mutation. They remain DB-08 work.
- Prepared an untouched review project at `E:\Repositories\nhl-modding-studio\local\DB01 Review` through production onboarding APIs. Existing QATest, game files and anchors were not changed.

## Automated evidence

- `cargo test -p studio-api --test nhl07_db01`: four integrated tests cover 18 biography/contract/draft fields; all exposed skater/goalie ratings and archetypes; invalid values; undo/redo; saved reopen; memberships; the player-page team/jersey field route; release/sign, including retained international membership; builds with growing and same-size pool changes; installation, conflicting recipient pools, and restore.
- Field-only installation matches the saved database byte-for-byte. Structural free-agency installation matches every live record in every table; unused capacity/physical layout can differ after growth and shrinkage. Restore is byte-identical to the original installation.
- `cargo test -p studio-domain --lib mod_package`: four tests passed, including malformed pool identities, schema qualification, old-schema compatibility, archive integrity and path validation.
- Existing `nhl07_default_roster` regression: five passed; one previously gated live-installation probe ignored. No live-installation probe was run.
- Frontend: three files / nine tests passed, including actual edit controls for salary, rating and birth-year units. TypeScript and production frontend build passed.
- Cross-title identity/rating regression: three tests passed over the anchor corpus. This run preceded the final calendar/free-agency changes; it does not qualify those changes on other titles.
- Legacy `onb003_corrections`: six passed, eight failed. The failures expect roster-only or combined main-db/roster packaging/deployment, paths removed from the default-database project workflow before this task. They are recorded, not represented as passing or as native NHL07 evidence. Their fixture/contract migration belongs to the separate Save Editor/package scope.

Logs are under `target/db01-*`. This record does not claim emulator acceptance for new free-agency behavior, contract interpretation, or every game mode.

## One-pass user verification

Use the prepared review project, not your production roster. If a listed control behaves differently, stop that item and report the field/action and result; other independent checks can continue.

1. **Open and inspect.** Mod Studio → Load Project → `E:\Repositories\nhl-modding-studio\local\DB01 Review`. Find Pavol Demitra. Expect Minnesota, #38, Slovakia as an additional team, Speed 92. Jason Allison should appear in Transactions' Free Agents pool. No unsupported active/secondary-position/potential controls should be invented for NHL07.
2. **Biography and contracts.** On Demitra's Player Data page, change First Name to `Pavel`; Birth Day to `15`, then Birth Month to `2`, then Birth Year to `1975`; Birthplace to `QA City`; Birth State to `QA State`; Height to `6'1"` (use the offered height control); Weight to `207`; Handedness to Right; Position to LW. Change Draft Year to `1994`, Draft Pick to `200`, Rookie on, Contract Amount to `5000000`, Contract Length to `3`, Two-way on. Confirm $5,000,000 is displayed. Change Nationality to Canada, verify it, then return it to Slovakia. These are test values, not roster corrections.
3. **Ratings and refusal.** Set Demitra's Speed to `93`; choose a different offered archetype and note it. Find David Aebischer and change one goalie rating by one point, noting its name/value. Try Demitra Speed `100`: it must be refused without changing 93. Try February 30: it must be refused without changing February 15. Undo and redo one accepted edit; confirm both the field and Overview follow it.
4. **Additional membership and transfer.** On Demitra, remove Slovakia from Additional Teams, confirm Minnesota remains his club, then Undo to restore Slovakia. Change Main Team to Vancouver and Jersey Number to `90` directly on Player Data. Verify the team rosters also reflect the move and Slovakia remains present with its own original number. Undo the number and transfer to return him to Minnesota before item 5; retain the earlier biography/rating edits.
5. **Release and sign.** In Transactions, select Minnesota and release Demitra to Free Agents. Confirm he leaves Minnesota, appears in Free Agents, and retains Slovakia. His Main Team must read Free agent, not Anaheim/Slovakia. Undo then redo the release. Sign him back to Minnesota (his released slot is available). Confirm he leaves the free-agent pool and returns to Minnesota. Next release Brian Rolston from Minnesota, and sign Jason Allison from Free Agents to Minnesota into the freed slot. Leave those changes in place for the game test. Finally move Demitra to Vancouver and set #90 again. This final state tests a release, a signing and a transfer in one build.
6. **Overview and persistence.** Check that Overview names the actual player edits and shows Rolston becoming a free agent and Allison becoming signed, with working player links. Save Session; close/reopen this review project. Verify Demitra's values, Aebischer's rating, Rolston's free-agent status and Allison's Minnesota membership remain. Reverting an edit must reduce the net change count, not increase it as an operation count.
7. **Build and install.** Give this review build a unique name/version in Project Overview and click BUILD. Close NHL07/Xenia. In Mod Manager select NHL07 Xbox 360, refresh/select the new build and Activate. Expect Installed; no structural-edit refusal. Use your existing extracted NHL07 installation.
8. **Game acceptance.** Start NHL07 from the extracted installation and choose Restore Default Rosters. Verify Pavel Demitra is on Vancouver #90 with Speed 93 and the visible biography/position/handedness values. Verify Brian Rolston is in Free Agents and Jason Allison is on Minnesota. Verify the changed goalie rating where the game's editor exposes it. Check Slovakia still contains Demitra. Play a short match to confirm the roster loads. For fields the game does not show, report “not exposed in game UI”; their storage/build checks are automated, and game-mode effects remain unverified.
9. **Restore.** Close NHL07, Deactivate the build in Mod Manager, restart the game and Restore Default Rosters again. Expect the original Pavol Demitra on Minnesota #38, original ratings, original Rolston membership and Jason Allison back in Free Agents.

Report PASS/FAIL for 1–9. DB-01 can close for NHL07 Xbox 360 after these checks pass. Unexposed field effects and other game modes remain explicitly unqualified; no PS3 target exists for NHL07.

Desktop build: final native build passed. Launched the rebuilt Studio with a responding visible window (PID 372172). Manual UI and emulator checks above remain for the user; launch is not acceptance evidence.


## User review follow-up — enum editing

- Item 1: PASS (user confirmed).
- Item 2: review in progress. User changed Birth State to Poland and reported that entering edit mode displayed a numeric code instead of a named selection. This is an edit-mode presentation defect, not a request to change stored enum values.
- Shared player/team/attribute editor now opens an accessible, full-width named dropdown with the current label selected. Removed the raw-number edit control from enum editing. The dense field/equipment editor uses the same dropdown. Unknown existing values are preserved explicitly; opening/cancelling never writes a replacement.
- Reviewed enum handling in the profile, raw field/equipment, attributes, lines, strategies and career-stat surfaces. Added enum priority over numeric inputs in career-stat cells and over numeric strategy sliders. Team/arena relationship selectors already use named choices. The Lines goalie roster-style exception remains because its skater vocabulary is not valid for goalie rows; it is not a confirmed goalie enum.
- NHL07's captured player view supplies 91 Birth State choices, five positions, two handedness choices, 27 nationality choices and ten skin-color choices. Existing IDs and title-specific vocabularies are retained (Poland = 86 in Birth State).
- Recheck: click Birth State, Handedness and Position. The active editor must show the current name, and opening its dropdown must show named alternatives. Select another name, then confirm the saved display matches. No number input should appear for a known enum field. Item 2 is not marked passed until user confirmation.

## User review follow-up — field responsiveness

The shared player/attribute, dense field and Main Team editors now display the submitted value immediately in its friendly units while the backend validates it. Writes are queued per project in submission order. Rejected values roll back with their error; a later rejected edit preserves an earlier accepted edit. Project-scoped refreshes are combined after a burst, unrelated previews remain cached, and revision-driven Overview refreshes are not duplicated. Save, build/archive, undo/redo and close wait for pending field writes.

Verification: 47 targeted frontend tests passed, including deferred-backend tests that assert the new label appears synchronously before IPC resolves, rapid-edit ordering, rollback, refresh coalescing, unaffected portrait queries, and save/build/undo/close barriers. TypeScript and production frontend build passed. Backend validation rules are unchanged; this is immediate UI feedback, not a claim that disk saving or database validation became instantaneous.

User recheck: enter a new name or rating and press Enter, then immediately edit another field. The submitted value should appear without waiting. Try an invalid rating; it should revert and report the error. Click Save Session immediately after a valid edit and reopen to confirm it was included. Existing DB-01 acceptance remains pending for the unfinished items.

## User review follow-up — Archetype on Player Data

Player Data's Archetype was a derived, read-only summary. It now uses the same editable style field and ratings-companion record as the Attributes tab, including the named dropdown and immediate field feedback. Missing companion records and noneditable fields remain protected.

Verification: 18 targeted frontend tests passed, including skater/goalie companion-record routing, record zero, named selection, immediate feedback and read-only fallbacks. TypeScript and production frontend build passed. User recheck: change Archetype on Player Data, confirm the same selection on Attributes, then save/reopen as part of item 6.

## User review follow-up — Undo and Redo access

Restored visible Undo/Redo controls before Save Session in the shared project header. Added Ctrl+Z for committed-edit undo and Ctrl+Y/Ctrl+Shift+Z for redo (Command equivalents also supported). Text inputs retain native text undo; modal dialogs do not trigger project history. History actions wait for queued field writes, refresh project views and counts, prevent duplicate in-flight requests, and display failures. Buttons follow the session's available undo/redo history.

Verification: 28 targeted frontend tests passed covering header buttons, shortcuts, text/modal exclusions, queued-edit ordering, failure/retry, save/close blocking and existing save/field behavior. TypeScript and production frontend build passed. User recheck: commit a field change, click Undo, then Redo; repeat using Ctrl+Z and Ctrl+Y after leaving the field editor. Confirm the field and Overview count follow each action.

## Post-sign-off workflow refinements

Undo/Redo now appear at the right of the contextual tab row. Successful builds offer Open containing folder (Explorer selects the generated package files) and Send to Mod Manager. Sent builds are registered persistently by their existing paths, alongside the downloaded-mod library, and remain in the Builds folder. The manager selects a compatible configured target or prompts for setup; sending does not activate a mod. An open Studio project retains its save/close controls and Return to Studio navigation.

Activating a different mod restores all restorable installs on the same game/platform/folder first, newest first to unwind any older stacks. A failed restore stops the switch before preflight or apply; other installation folders are untouched. Preflight runs again against the restored files. If the new package fails validation, the previous mod remains deactivated.

Automated verification: 38 targeted tests across five files passed; TypeScript and production frontend build passed. New native UI review: check button placement; build and try both result actions; activate a second mod and confirm only it is Installed, then deactivate it and verify original files. These new refinements are separate from the completed DB-01 sign-off.

## Overview warning diagnosis — 2026-10-07

Read-only comparison of `local/DB01 Review/baseline/db/NHL07NG.db` and its saved working database found exactly four bytes incorrectly classed as unknown. Free-agent table `VoEQ` still has 74 live records, but its capacity grew from 74 to 75, increasing the database from 4,189,776 to 4,189,780 bytes. The final checksum moved to offsets `0x3FEE50..0x3FEE53`. Recomputing all checksums reproduces both files exactly; neither has a checksum error. The former review compared absolute offsets against only the working record spans and omitted the final checksum from recognized integrity data.

The review now excludes each file's validated final checksum and compares remaining unknown regions after accounting for each file's own layout. The saved review project now reports zero unrecognized bytes. Invalid checksums remain visible. The fallback notice now reports the number of unrecognized bytes and explains that they are not included in the edit summary, instead of implying an edit occurred outside the permitted fields. No project database or anchor was modified by the diagnosis.

Regression coverage includes NHL07 free-agent capacity growth and reversal, damaged-checksum detection, and moved unknown regions with preserved/mutated contents. Diagnostic command: `cargo run -p studio-domain --example review_integrity -- <baseline.db> <working.db> <meta.xml> defs`. Evidence is in `target/overview-integrity-fixed.log`.

Validation results: the new NHL07 regression, existing save/rebind and NHL21 scalar-review tests, and unknown-region unit test passed. The broader NHL21 structural player-cloning test was stopped after several minutes and is not counted as passing. Native check and native build passed. Rebuilt Studio relaunched with a responding visible window; the revised comparison was verified read-only against the saved DB01 Review files before launch.
