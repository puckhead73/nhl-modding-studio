# NHL16 QA — first pass complete with blockers

Independent installed files load 5,632 players and 252 teams. Supplied NHL 16.rar contains a game dump (54 entries), not a disc ISO; it was unpacked to a fresh F: QA folder. Production game-container extraction passed in 531.93 seconds: 11 archives, 79,773 files, 41,016,941,052 bytes, zero collisions/failures/remaining files. The extracted database and dictionary load 5,632 players and 252 teams.

Mirrored database/roster editing passed in 62.94 seconds after adapting the QA harness to actual NHL16 fields. NHL16 has `shortname` and `nickname`, not the earlier `fullname`; it also has separate stick-blade tape RGB and `feprimarycolor_*` fields. The test changes these actual fields, the `NHLTeamName_ANA` dictionary entry, team city, arena FK, arena biography, equipment RGB, player biography/rating, roster transactions/lines/scratches and career data. Save/reopen succeeds, the saved roster independently contains the team-name display, traded-in player and rating, and original roster bytes are unchanged. Initial harness failures are retained as attempts 1 and 2; successful output is ps4-roster-roundtrip-3.

Six copied GNF replacements passed in 9.98 seconds: league logo, team logo, portrait (`kettle_assets/playerportraits/p1.gnf`), jersey, goalie-stick tape and center-logo image. Decoded outputs change, dimensions remain valid and original/backups are preserved. Project import passes in 4.54 seconds with copied database/dictionary/roster, owned F: assets and blank destinations.

| Requirement | PS4 finding |
|---|---|
| 1. ISO extraction/load | NOT VERIFIED: supplied source is a RAR game dump; no NHL16 ISO found in the PS4 ROM folder. |
| 2. Dumped-disc extraction | PASS fresh relocated RAR dump, all 11 game archives extracted with zero failures. |
| 3. Arbitrary existing files | PASS independent installation, freshly extracted F: dump and imported project. Direct game-folder and parent-folder discovery resolve the same database. Relocated asset inventory passes in 20.51 seconds. |
| 4. League name/icon | PASS dictionary rename/save/reopen and copied native GNF replacement. New loose-logo resolver finds installed assets. |
| 5. Team biography | PASS shortname, nickname, full-name dictionary entry, localized city, arena FK/biography and frontend/equipment RGB. Which name source each in-game screen reads remains unverified; no invented fullname alias. |
| 6. Team logo | PASS copied GNF preview/replace/reopen with preserved source/backup. Runtime unverified. |
| 7. Center ice | PARTIAL: centerlogo_0_cm.gnf replacement passes. Venue key 0 has five vault candidates, so binding/reassignment is ambiguous and not promoted to a verified choice. Full rink editing/runtime selection unverified. |
| 8. Roster/lines/scratches | PASS database/roster release/sign/trade, unique first-line center, scratch flag and jersey number persistence. Saved roster opened independently. |
| 9. Jersey/equipment colors | PASS native GNF replacement and actual RGB fields. PS4 kit-file completeness is unmeasured; no gen7 completeness claim. |
| 10. Player biography/attributes/portrait | PASS biography, direct rating 85, career and native portrait replacement. Saved roster independently checked. |
| 11. Player equipment | PASS stick and blade-tape red channel persistence, native equipment GNF replacement; catalogue is enum-only for this title. Runtime appearance unverified. |
| 12. Localization | PASS league, generic text, city and full-team-name re-encode/save/reopen. Each pending operation saved before the next. |

Evidence: target/overnight-20261003/nhl16/ps4-observations-1.json, ps4-roster-roundtrip-3 and ps4-texture-roundtrip-1. Bulk game copy: F:/TEMP/nhl-studio-overnight-20261003/nhl16/ps4-source-routes-1/NHL 16/CUSA01909. Original installation/RAR/save files remain read-only.

Copied inxp_00.big deployment passed in 239.87 seconds: actual short-name edit, decoded member/database agreement, CRC validation and byte-exact restore; original archive preserved.

Fixed the PS4 uniform browser: named GNF families previously fell into Other textures while an empty numeric jersey registration was selected by default. Studio now groups matching named maps and initially selects an available texture family after discovery finishes. Registered jerseys remain independently selectable; no numeric registration or 3D binding is invented. The unverified 3D notice is compact and no longer incorrectly asks users to link an already linked folder. Five focused tests pass, including delayed discovery and retained manual selection; TypeScript/Vite production build passes. This is not yet verified in the running desktop bundle.

Desktop/runtime verification remains blocked by the Windows helper's repeated `foreground window did not report a process id` failure. No NHL16 emulator run or edited gameplay is claimed. All twelve requirements have been reviewed; this is not full gameplay acceptance. ISO coverage and edited runtime remain unverified. Final source and relocated inventory receipts are ps4-source-receipt-1.json and ps4-relocated-observations-1.json.

