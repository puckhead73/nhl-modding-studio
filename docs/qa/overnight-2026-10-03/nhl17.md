# NHL17 QA — first pass complete with blockers

Read-only installed inventory loads 6,128 players and 256 teams and resolves 100 Anaheim outfit textures. The supplied ROM folder contains a ZIP and its extracted PS4 PKG (20,813,578,240 bytes); neither is an ISO. Studio has no direct PS4 PKG import implementation. This route remains a blocker, not an extraction success.

A fresh owned copy of the installed game, excluding its Extracted directory, contains 49 files / 19.416 GiB. Production container extraction passed in 741.61 seconds on F: with 12 archives, 96,408 files, 51,101,954,069 bytes and zero collisions/failures/remaining files. The extracted database and dictionary open 6,128 players and 256 teams. An initial sandbox directory-creation denial was resolved with approved access to this copied target; no original was changed.

The real edit test found that NHL17 stores fifteen equipment color zones as packed 24-bit values. Studio incorrectly read nine missing RGB triplets as zero and attempted writes to nonexistent fields. The domain now reads the packed fields, the DTO carries their write targets, and the equipment editor preserves the other channels when changing one channel. Older split-channel schemas retain their existing path. Seven focused frontend tests and the TypeScript/Vite build pass. Mirrored database/roster save-reopen passed in 66.36 seconds: the edited first zone decodes as RGB (17,43,91), all other zones stay unchanged, and the saved DATA independently contains the roster/name/rating edits.

Also corrected PS4 source import retaining the nested database directory as its assets root after discovery. The wizard now retains the detected game folder, so textures remain discoverable. Seventeen onboarding/source-panel tests pass, including the new integration regression checking the asset root sent to import review. Desktop verification remains unavailable.

| Requirement | Current PS4 finding |
|---|---|
| 1. ISO extraction/load | BLOCKED: no supplied ISO; supplied PKG import unsupported. |
| 2. Dumped-disc extraction | PASS copied packed-install extraction; this does not establish PKG extraction. |
| 3. Arbitrary existing files | PASS installed inventory and project import with copied anchors and relocated F: asset link. PASS relocated asset qualification in 36.94 seconds; direct/parent-folder discovery agrees and freshly extracted DB bytes match the independent installation. |
| 4. League name/icon | Native GNF replacement passes; dictionary rename/save/reopen passes. |
| 5. Team biography | PASS actual abbreviation, nickname, localized full name, city, arena and packed frontend color persistence. No shortname field in this schema. |
| 6. Team logo | PASS copied native GNF replacement with preview/reopen and backup preservation. |
| 7. Center ice | PARTIAL: native ice_centerlogo_0_cm.gnf replacement passes. Venue key 0 has multiple vault candidates; runtime binding and full rink reassignment remain unverified. |
| 8. Roster/lines/scratches | PASS mirrored database and RosterCustom0/DATA release/sign/trade, line-slot uniqueness, scratches and jersey numbers; DATA independently reopened. |
| 9. Jersey/equipment colors | Native jersey GNF replacement passes; fifteen-zone production fix implemented and frontend tested, persistence passes with other zones unchanged. Named texture groups are available; PS4 3D kit binding remains unverified. |
| 10. Player biography/attributes/portrait | Native portrait replacement passes; biography, rating 85 and career save/reopen passes. |
| 11. Player equipment | Native stick-tape texture replacement passes; stick and packed blade-tape value persistence passes. |
| 12. Localization | PASS league/city/full-name re-encode and save/reopen. |

Six native replacements pass in 9.74 seconds, with decoded image changes, dimensions retained, backups byte-equal to source and original files unchanged. The first attempt stopped at the NHL16 center-logo filename; NHL17 actually uses ice_centerlogo_0_cm.gnf. Retained failed attempts are not successful receipts.

Project import passes in 4.26 seconds with copied database, metadata, dictionary and roster; deployment destinations are blank. Archive edit/decode/CRC/restore verification passes in 225.00 seconds on a copied inxp_00.big, with byte-exact restoration and original source preserved. The NHL16 split-channel save/reopen regression also passes in 61.90 seconds after the packed-color fix. Evidence lives under target/overnight-20261003/nhl17; bulk target is F:/TEMP/nhl-studio-overnight-20261003/nhl17/ps4-source-routes-1/NHL 17/CUSA04326.

Desktop helper failure still prevents native Studio/emulator QA. No NHL17 edited gameplay or updated bundled-desktop visual verification is claimed. GitHub issue 1 and Discord onboarding/uniform findings remain covered by shared fixes; this title's findings do not establish that every reported runtime symptom is resolved.




