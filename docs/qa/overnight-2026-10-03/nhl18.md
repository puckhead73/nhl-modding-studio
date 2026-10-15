# NHL18 QA — first pass complete with blockers

Installed inventory loads 7,236 players and 296 teams, including 120 Anaheim outfit textures. Supplied RAR archives ultimately contain a 25,124,798,464-byte PS4 PKG, not an ISO. Direct PKG import remains unsupported. A fresh F: copy of the packed installation (50 files / 23.437 GiB, excluding Extracted) has been extracted through Studio in 1,072.36 seconds: 13 archives / 120,591 files / 66,539,139,264 bytes / zero failures, collisions or remaining members. The extracted database and dictionary load 7,236 players and 296 teams.

Six native GNF replacements pass in 10.11 seconds, with changed decoded images, dimensions retained, byte-identical backups and unchanged originals. NHL18 uses teamlogos/large/t0.gnf; the NHL17 fullsizeflat path is absent. The initial failed fixture-path attempt is retained. Project import passes in 4.68 seconds with copied anchors/roster, owned F: assets and blank deployment destinations.

| Requirement | Current PS4 finding |
|---|---|
| 1. ISO extraction/load | BLOCKED: no supplied ISO; direct import of the supplied PS4 PKG is unsupported. |
| 2. Dumped-disc extraction | PASS extraction of the copied packed installation. Standalone PKG extraction and direct database load also pass; Studio PKG onboarding is still missing. |
| 3. Arbitrary existing files | PASS independent installed inventory and imported project; fresh extracted database and relocated assets pass in 29.45 seconds; extracted DB bytes match the independent installation. |
| 4. League name/icon | Native icon replacement passes; localization persistence passes. |
| 5. Team biography | PASS actual abbreviation, nickname, localized name/city, arena linkage/biography and packed frontend colors saved and reopened. |
| 6. Team logo | PASS copied native large/t0.gnf replacement/preview/reopen, with source and backup preserved. |
| 7. Center ice | PARTIAL: ice_centerlogo_0_cm.gnf replacement passes. Venue key 0 has two vault candidates; binding and full rink reassignment remain unverified. |
| 8. Roster/lines/scratches | PASS single-membership skater release/sign, trade, line-slot uniqueness, scratches and jersey number save/reopen, with independently opened saved roster. Releasing a club membership leaves national/all-star memberships intact; such a player is not eligible for the current free-agent signing operation. See limitation below. |
| 9. Jersey/equipment colors | Native jersey replacement passes. Fifteen packed equipment zones supported by the NHL17 fix; NHL18 persistence passes, with other zones unchanged. PS4 3D kit binding remains unverified. |
| 10. Player biography/attributes/portrait | Native portrait replacement passes; skater biography/rating/career save-reopen passes. |
| 11. Player equipment | Native stick-tape texture replacement passes; stick and packed tape-field persistence passes. |
| 12. Localization | PASS league/city/full-name re-encode and save/reopen. |

Roster limitation: player 1213 occupies Anaheim, Pacific Division All-Stars and USA slots. Releasing only Anaheim correctly preserves the other two slots, but Studio's free-agent pool requires zero memberships, so sign_from_fa rejects him as already rostered. The roundtrip fixture is being corrected to use a skater with one membership. This does not resolve the broader workflow of signing a club-free player who retains national/all-star memberships. No secondary memberships were silently removed. A second fixture attempt selected a goalie with fMVl equipment and was rejected by the skater-specific harness; it is not a production equipment failure.

The third edit attempt exposed a real career-season bug: NHL18's tsSy table omits the inline team-code field lyxL. Add Season now omits this field only when the caller supplies an empty code; a nonempty code is rejected rather than silently lost. The schema capability is exposed to the form, which hides the unavailable team-code input. Four career/equipment frontend tests and the production frontend build pass. The complete NHL18 save/reopen test passes in 80.06 seconds, including refusal of a nonempty team code without adding a row, then successful empty-code row insertion. The older NHL17 schema regression passes in 65.87 seconds.

Evidence and copied edits: target/overnight-20261003/nhl18. Bulk copy: F:/TEMP/nhl-studio-overnight-20261003/nhl18/ps4-source-routes-1/NHL 18/CUSA07569. Original game/save/archive files remain read-only. Archive nickname edit/CRC/decode/restore passes in 306.22 seconds on the copied inxp_00.big, including byte-exact restoration.

Desktop Studio and emulator verification remain blocked by the Windows helper error. No NHL18 edited gameplay is claimed. Shared onboarding and uniform fixes address parts of GitHub issue 1 and the Discord feedback, but these component/backend results do not establish full runtime acceptance.

Package-import investigation: the [standalone shadPS4Plus extractor](https://github.com/AzaharPlus/shadPS4Plus) documents a direct input/output CLI. The Windows 1.1 release ZIP was downloaded only into the QA tools folder and matched GitHub's published SHA-256 `a6a6dee84c1283949245c868e580ff8d0049d0e73df9de3096460ee26c6f04d7`. Its base-package probe returns 101 for the supplied NHL18 PKG. Extraction into a separate fresh pkg-source-routes-1 folder finished at 58 of 58 entries. Whole-file hash comparison finds 49 of 50 files equal, including all thirteen game archives and eboot.bin. The sole difference is the final four bytes (offsets 528–531) of the 532-byte sce_sys/npbind.dat; cause and runtime impact are unverified. A separate test reads the database and dictionary directly from the PKG-extracted inxp_00.big and opens 7,236 players / 296 teams in Studio in 25.05 seconds. The tool is not integrated into Studio. The CLI's source returns zero even for some failures, so exit status alone will not be accepted as success. No installed emulator was replaced.



