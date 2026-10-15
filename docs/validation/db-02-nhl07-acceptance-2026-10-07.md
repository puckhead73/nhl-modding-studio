# DB-02 — NHL 07 Xbox 360

Status: PASSED — user sign-off, 2026-10-07, for NHL 07 Xbox 360. All original DB-02 checks and the follow-up drag-and-drop, immediate edit feedback and Overview database-revert checks passed. DB-02 is closed for its supported existing-team scope. Earlier pending-review wording below is retained as verification history and superseded by this sign-off. Documented unsupported fields, record lifecycle, file/archive/structural reverts and other games/platforms remain outside this acceptance.

## Scope and completed work

| Area | NHL07 implementation / qualification |
| --- | --- |
| Team identity | Full Name, Short Name and Abbreviation edit the joined stock team record. Save/reopen and package install/restore verified. Localization-wide name propagation remains LOC-01; individual game screens need native review. |
| Location and home arena | Added a named Home Arena dropdown targeting the actual `StEO.arenaid` stock-arena key. Arena City edits the selected stock arena's city. The UI explains that the city is shared by all teams using that arena. Unknown venue keys are refused, including bulk/package writes. |
| Team colors | The NHL07 stock team record has no team-color/RGB fields. No color controls are invented. Kit/equipment/art color work remains AA-02/AA-03. |
| Roster and depth chart | Existing membership-driven depth chart, roster count, player transfers and additional memberships retained; DB-01 regression suite rerun. |
| Lines / Best lines | Replaced successive single-cell requests with one validated backend roster operation. Swaps and Best lines are one Undo step. Cross-team rows, invalid field/value writes, duplicate occupants, scratched players and goalie/skater slot mismatches are refused before mutation. |
| Dressed / scratched | Scratching atomically clears that player's line assignments and captaincy. One Undo restores all of them. Dressing alone does not silently rebuild lines. |
| Captain / alternates | Replacing a role holder is one transaction; an invalid selection cannot first demote the previous holder. A team cannot gain a second captain through this operation. |
| Jersey numbers | Duplicate numbers are refused without clearing the existing holder. Explicit assignment is limited to 0–99 (0 clears the number). NHL07 primary-team jersey changes retain the biography mirror. |
| Strategies | All nine strategy fields exposed by this title were changed through their actual named choices and verified through save/reopen/build/install/restore. Gameplay effects still require native checks. |
| Draft picks / retired numbers | The default database lacks `qHav` and `wcSL`. Draft-pick trading is not applicable here; unsupported retired-number checkboxes are hidden. API refusals verified. |
| Team record lifecycle | Creation/duplication/deletion is not qualified for this target. No new team-record writer or structural package support is claimed. Direct edits to the stock/custom team selector are now read-only and refused, preventing a team from being accidentally redirected to another identity. Native Create-a-Team work remains DB-09. |
| Project Overview | Strategy and NHL07 home-arena changes group under their team; shared arena metadata groups under Shared changes → Arenas. Existing cumulative review and field links retained. |

Line, captaincy and jersey edits participate in the pending-write queue, so Save/Build/Undo/Close wait for them. The new roster command and dressed-state command run away from the native UI thread.

## Automated evidence

- `cargo test -p studio-api --test nhl07_db02`: three production-API tests cover inventory, identity/location/arena/strategies, invalid arena and selector refusals, atomic line and captaincy edits, scratch/undo, invalid goalie/cross-team/field/value requests, jersey conflicts, unsupported draft picks/retirement, and package installation/restoration.
- Two independent package round trips compare the installed database byte-for-byte with the saved working database, then verify exact restoration to vanilla. All writes use disposable folders.
- `cargo test -p studio-api --test nhl07_db01`: all four prior player-editing qualification tests passed with the new roster/venue behavior.
- Frontend tests cover one-command Best lines, one-command scratching, atomic captain replacement, absent retired-number controls, queued writes, enum editors and session controls.
- Field-profile tests include the new NHL07 venue selector and other committed title anchors.
- Native check and frontend production build are required before the review executable is handed over.

Final results: all 3 DB-02 API tests, 4 DB-01 API regressions, 8 field-profile tests and 36 targeted frontend tests passed. Native check, native build and frontend production build passed. The fresh DB02 Review project was created through production onboarding and saved clean. Rebuilt Studio was launched for user review. Existing Vite chunk-size/deprecation and Rust unused-code warnings do not represent new test failures. At that initial handoff, native game acceptance was pending; the user subsequently passed item 9 as recorded below.

## One-pass user verification

Use Mod Studio → Load Project and select `E:\Repositories\nhl-modding-studio\local\DB02 Review`. This is a fresh vanilla project with no game-source folder linked; the database-only tests below do not require one. Use your existing NHL07 Xbox 360 installation in Mod Manager for the game test. Keep each accepted edit unless a step explicitly says Undo.

1. **Baseline and team identity.** Open Teams → Minnesota → Team Data. Expect Minnesota Wild, 27 roster members, Xcel Energy Center and St. Paul, MN. Change Full Name to `Minnesota QA`, Short Name to `Minnesota Test`, and Abbreviation to `QAM`. Confirm the team list/header updates. Undo and redo the abbreviation once. Name rendering on every in-game screen is checked separately in step 9.

2. **Venue and city.** Before changing arenas, set Arena City to `QA City`; confirm the arena description updates, then Undo to restore `St. Paul, MN`. Open Home Arena and select Vancouver's `General Motors Place` entry (use its Vancouver city label to identify it). Confirm Minnesota's displayed venue changes. Undo once and expect Xcel Energy Center; Redo and expect the Vancouver venue again. This changes the venue selection, not its art assets.

3. **Lines and depth chart.** Confirm the Depth Chart lists Minnesota's roster. On Lines → Even Strength, swap Pavol Demitra (line 1 center) with Pierre-Marc Bouchard (line 3 center) by dragging one line card onto the other. Both cards should exchange places. Undo once must restore both; Redo must reapply both. Click Best lines, then Undo once to restore the pre-Best-lines arrangement. Drag a goalie onto a skater slot: expect refusal with no partial change. This verifies the editor; whether the game presents every stored situational assignment is checked in step 9.

4. **Captaincy and scratching.** On Jersey Numbers & Captaincy, change Captain from Wes Walz to Pavol Demitra. Undo once must restore Walz; Redo restores Demitra. On Lines, set Demitra to Scratched. Expect him off all lines and absent from captaincy. Undo once must restore his dressed state, captaincy and exact line assignments. Leave him dressed and captain. Dressing a scratched player alone intentionally does not auto-assign lines.

5. **Jerseys and roster transactions.** Drag an already-used jersey number onto Demitra: expect an error and no changes to either player. Assign him unused number `91`. Confirm his player page also shows 91. In Transactions move a different player from Minnesota to another NHL club, confirm both rosters and counts update, then Undo to return that player. Keep Demitra on Minnesota. No retired-number checkboxes or Draft Picks tab should appear for this default-database target.

6. **Strategies.** On Minnesota → Strategies, change Defensive Pressure from Protect Net to another named choice and note that choice. Undo and Redo; verify the label each time. Change one offensive-line strategy and one special-teams strategy, noting the final names. The complete set of nine fields has automated storage/package coverage; these representative changes are for native UI/game verification.

7. **Overview, save and reopen.** Confirm the team name, arena, line, captaincy, jersey and strategy changes are summarized; edit links return to the relevant entity. Save Session, close the project and reopen DB02 Review. Verify the final name, Vancouver arena, line arrangement, Demitra captain/#91 and chosen strategies. Undo history is session-only; do not expect the previous session's Undo stack to survive reopening.

8. **Build and install.** In Project Overview use a distinct build name such as `DB02 QA`, version `1.0`. BUILD, then Send to Mod Manager. Close NHL07/Xenia. Select the NHL07 Xbox 360 installation and Activate this build. If another mod is active, it should deactivate first. Confirm Installed. The build contains only this fresh review project's edits.

9. **Native game and restore.** Launch NHL07 from the extracted installation and select Restore Default Rosters. Verify Minnesota's roster, Demitra wearing #91 as captain, and the edited lines/strategies wherever NHL07 exposes them. Check which screens display `Minnesota QA`/`QAM`; report any that retain the original name so we can distinguish database names from localization. Start a Minnesota home exhibition and confirm the Vancouver venue loads, then play briefly. Report any field that is not exposed in the game UI rather than marking its behavior proven. Close the game, Deactivate in Mod Manager, restart and Restore Default Rosters; verify original team/arena/roster behavior returns.

Report PASS/FAIL for 1–9, with any screen-specific name or strategy differences. DB-02 can close for supported existing-team editing on NHL07 Xbox 360 after this run; it does not qualify other platforms, newly created teams, asset colors or every game mode.

## Acceptance follow-up — 2026-10-07

User passed items **1, 2, 4, 6, 7, 8 and 9**, including save/reopen, packaging, installation and the in-game result. Items **3 and 5** reported the same drag failure; the remaining behavior from those items is not being reclassified as a failure.

Changes:
- Disabled Tauri's native drop interception on the main Windows webview, which otherwise prevents HTML line-card and jersey-number drops. File import continues through the existing pickers.
- Team edits display pending values immediately. Failed writes restore the previous display. Line/status/style/jersey/captaincy and strategy changes share the serialized save/undo barrier and coalesced project refresh. Transfers, releases, signings and trades update roster lists immediately.
- Moved roster writes and expensive roster/team read commands off the native UI thread so their work cannot hold the window's event loop.
- Added a per-edit × in Project Overview. Database field reverts use the immutable project-creation baseline, survive save/reopen, remain undoable, and preserve unrelated fields. Dependent roster rows, primary jersey mirrors and free-agent membership restore atomically. File/localization/structural change summaries are not yet supported by this new database revert command; attempting one reports that it must be reverted in its editor, without changing anything.

Focused remaining user review:
1. Lines: drag one occupied skater card onto another and verify the swap. Undo/redo it. Drag a skater from the roster rail onto a card. Attempt a goalie onto a skater slot and expect refusal with the original lineup restored.
2. Jersey Numbers & Captaincy: drag an unused number onto a player. Expect an immediate display update. Try an already assigned number and expect refusal with both original assignments intact. Change captain and one strategy; both should display the new selection immediately.
3. Overview: make two unrelated field edits. Click × for one and confirm only it disappears, its editor shows the original value, and the header edit count updates. Undo should restore that edit. Save/reopen, then repeat × on the saved edit.
4. Overview roster revert: release or transfer a player, find that player's roster/team change and click ×. Confirm the original team membership returns and no duplicate/free-agent residue remains. Related roster changes are reverted together to keep a valid database.

No repeat of the already-passed native-game acceptance is requested for these UI fixes; the revised revert output is also covered by automated save/reopen and build/install regression checks.

Follow-up automation: 7 DB-02 API tests, 4 DB-01 API regressions, and 29 frontend tests passed (40 total). The frontend production build and native debug build passed. Tests cover pending feedback before native completion, refusal rollback, HTML card/number drops, the Windows configuration, saved/reopened selective reverts, linked roster changes, secondary-team jersey independence, trades, and Save/Undo write barriers. Logs are in `target/db02-final-revert-tests.log`, `target/db02-followup-api-tests.log`, `target/db02-followup-ui-tests.log`, `target/db02-followup-ui-build.log`, and `target/db02-followup-native-build.log`.
