# NHL19 QA — first pass complete with blockers

The supplied PS4 ROM folder is empty, including hidden files. The installed CUSA11126 contains base and patch archives. A fresh copy of 2,408 files with packed archives and existing top-level loose files (33.637 GiB, excluding Extracted) is under F:/TEMP/nhl-studio-overnight-20261003/nhl19/ps4-source-routes-1/NHL 19/CUSA11126. Production extraction finished in 1,171.15 seconds: 28 archives, 152,659 members, 135,765 files written (76,070,559,803 bytes), 16,894 blocked members at 6,863 colliding paths, zero codec/archive failures. The completion assertion failed because the final state is blocked. No patch precedence is assumed.

Existing-file inventory opens 7,136 players and 419 teams. The English dictionary is intentionally not bundled; the installed dictionary was read/copied only into private QA output. Do not add it to anchors or distributions.

| Requirement | PS4 finding |
|---|---|
| 1. ISO extraction/load | BLOCKED: no ISO or PKG supplied for NHL19. Studio PS4 PKG onboarding remains a separate unfinished task. |
| 2. Dumped-disc extraction | PARTIAL: noncolliding members extracted; base/patch collisions remain blocked. Existing top-level loose DB files were included in the copy, so successful load does not prove packed-version resolution. |
| 3. Arbitrary existing files | Existing installation inventory and project import pass. Copied database/save/dictionary files reopen at arbitrary QA paths. Copied installation discovery opens 7,136 players / 419 teams. Its pre-existing loose DB hashes to the base inxp_00.big version, not a newly selected patch version. Relocated inventory passes in 19.93 seconds, including direct/parent folder discovery and real logo paths. |
| 4. League name/icon | League localization save/reopen and copied native icon replacement pass. |
| 5. Team biography | Database abbreviation, nickname, localized full name/city, arena association, arena name/city and packed frontend colors save/reopen. Direct roster-mode abbreviation also persists while main DB stays byte-identical. |
| 6. Team logo | Copied native GNF replacement, decode and backup checks pass. |
| 7. Center ice | Native ice_centerlogo_0_cm.gnf replacement passes; vault-based venue binding and full-rink reassignment remain unverified. |
| 8. Roster/lines/scratches | Compatible entity transactions pass on both files. Main DB line ownership and scratches persist. Direct roster-mode release/sign, scratch/jersey and complete line-view serialization pass on independently reopened DATA. See pair limitation below. |
| 9. Jersey/equipment colors | Native jersey GNF replacement and fifteen packed equipment-color zones pass; other fourteen zones preserved. Named outfit support exists; numeric database/3D binding remains unverified. |
| 10. Player biography/attributes/portrait | Main DB and direct roster-mode biography/rating save/reopen pass; native portrait replacement passes. Career insertion without unsupported inline team code passes in main DB. |
| 11. Player equipment | Stick/tape fields and copied native tape texture pass. NHL19's Creation Zone catalogue resolves actual equipment names, but rendered in-game equipment is not verified. |
| 12. Localization | League, city, full name and sampled dictionary text persist. Requires the private installed dictionary for this fixture. |

The supplied main DB and DATA roster are unaligned: player 3207 is on Anaheim in the main DB and San Diego in DATA. Generic record-index edits correctly remain main-only. Entity transactions require compatible memberships in both files and refuse incompatible operations. The broad edit test passes in 74.74 seconds with compatible players; this does not prove generic mirroring. A separate roster-mode test passes in 16.90 seconds (expanded second run, including mismatched transaction refusal and unchanged history), changing team/player/gear/roster fields in DATA, independently reopening it and proving the main database is byte-identical. No original file was changed.

Six native texture roundtrips pass in 15.98 seconds. The copied inxp_00.big nickname edit passes CRC, fresh decode and byte-exact restore in 332.41 seconds. Project import passes in 4.08 seconds with copied anchors, roster and private dictionary, owned F: assets and blank deployment destinations. Evidence is under target/overnight-20261003/nhl19; failed fixture attempts are retained alongside successful attempts.

Desktop Studio and emulator verification remain blocked by the Windows helper failure. These API, file and serialization checks are not edited-gameplay evidence. GitHub issue 1 and Discord onboarding/texture/career reports remain subject to the same runtime limitation; do not close them based solely on these results.

The unaligned-pair toolbar notice and source-switch popup previously implied edits always applied to both files. They now distinguish main-only field edits, compatible entity transactions and roster-only editing. The save tooltip also describes the selected routing. Critical collision inspection found seven distinct versions each of nhl_game.db and eng_us.db; receipt ps4-collisions-1.json records decoded SHA-256 values.

Trade-screen fix: team ID 0 (Anaheim) was incorrectly treated as a missing team by truthiness checks. Replaced these with explicit null checks in eligibility, action and tooltip. Two component tests reproduced the disabled button before the change and now pass, asserting trade_slots arguments with team 0 in either column. This shared fix applies across titles. The broader frontend suite previously passed 338 tests in 61 files; these two new cases are additional.



All twelve requirements have been reviewed. This is a completed first pass with extraction, asset-binding and desktop/runtime blockers, not full user acceptance. Workspace regression suite passes 13 tests. Latest frontend build index-DY41z5dr includes the trade and unaligned-file messages; the currently open Studio executable predates these changes.
