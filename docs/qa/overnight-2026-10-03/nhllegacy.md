# NHL Legacy QA — Xbox360 and PS3 reviewed

The ISO staging route opens 5,632 players and 252 teams; a player edit survives save/reopen without modifying the source ISO. The materialized disc contains 171 files / 6,450,973,987 bytes. Full extraction completed in 875.59 seconds: 45,845 assets / 13,372,626,424 bytes, 17 archives, zero failed/conflicting/blocked/unsupported members, complete preflight.

Database editing passed in 25.60 seconds. The stronger mirrored-roster pass succeeded in 36.36 seconds: database and raw roster container save/reopen, valid header CRCs, independent opening of the saved roster with edited team, roster membership and rating, and byte-identical original save. Native asset inventory and six replacement/backups checks passed in 43.28 seconds. A separate preview assembly check passed in 20.07 seconds: 7 pieces, 19,926 triangles and 15 textures, finite positions, valid triangle indexes and valid material references. This is geometry/API evidence, not desktop visual or gameplay evidence.

## Xbox360 checklist

| Requirement | Finding |
|---|---|
| 1. ISO extraction/load | PASS for production ISO staging/edit/save/reopen; 5,632 players and 252 teams. Full copied-disc extraction is complete. |
| 2. Dumped-disc extraction | PASS: relocated disc unpacked completely; 17 archives, zero remaining members. |
| 3. Arbitrary existing files | PASS for independent extracted dump and production import project; source inputs copied, linked assets point to the owned S: dump and destinations remain blank. |
| 4. League name/icon | PASS at file level: NHL to QHL dictionary edit and native icon replacement; source/backups retained. Runtime unverified. |
| 5. Team biography | PASS name, localized city, arena foreign key, arena name/city and primary/equipment RGB save/reopen. |
| 6. Team logo | PASS copied native replacement with changed decoded content; runtime unverified. |
| 7. Center ice | PASS for copied centerlogo_0_cm.rx2 replacement. Complete rink art and runtime selection unverified. |
| 8. Roster/lines/scratches | PASS release/sign/trade, unique first-line center, scratch flag and jersey number persistence, including the saved roster container. |
| 9. Jersey/equipment colors | PASS native jersey replacement and equipment RGB. 77 textures and 9 complete assigned kits. Preview geometry assembles; desktop unavailable. |
| 10. Player biography/attributes/portrait | PASS biography, aggression raw 49/display 85, career and native portrait roundtrips. Independently opened roster contains the edited rating. |
| 11. Player equipment | PASS stick/tape selections and native stick texture replacement. Stock gloves resolve with measured contract. Runtime appearance unverified. |
| 12. Localization | PASS dictionary re-encode/save/reopen. The explicit one-pending-operation limitation remains. |

Actual cacheboot.big deployment/restore passes in 26.54 seconds: decoded edited member matches saved DB, CRC valid, both copied DB/archive restored byte-for-byte, source unchanged. Fresh project creation passes in 1.53 seconds. Receipts and copied edit targets: target/overnight-20261003/nhllegacy. Bulk disc output: S:/TEMP/nhl-studio-overnight-20261003/nhllegacy/source-routes-1.

Runtime/desktop blocker carried from the preceding PS4 check: the Windows helper failed both initial and recovered-window attempts with `foreground window did not report a process id`. No Legacy emulator launch or edited gameplay is claimed. Existing original emulator and Studio windows were not changed. PS3 review has not started.
Xbox360 first pass reviews all twelve requirements with the runtime/desktop limitation above. Source receipt: S:/TEMP/nhl-studio-overnight-20261003/nhllegacy/source-routes-1/source-routes.json. Proceed to PS3.

## PS3 checks

Mirrored database/roster edit/save/reopen passes in 32.13 seconds, including independently reopening the saved roster and checking its team name, traded-in membership and rating. Native asset inventory and six copied replacements pass in 45.65 seconds: 104 textures and nine complete assigned kit rows. Decoded jersey color slot 3 was visually inspected as the Anaheim jersey; this is a raster check, not desktop verification. Project creation passes in 1.55 seconds with copied sources, owned asset root and blank destinations. Copied cacheboot.big apply/decode/CRC/restore passes in 26.09 seconds, with original source preserved.

| Requirement | PS3 finding |
|---|---|
| 1. ISO extraction/load | PASS: 30 disc files / 6,962,261,494 bytes copied; extracted DB matches independent vanilla dump and opens with dictionary, 5,632 players / 252 teams. |
| 2. Dumped-disc extraction | PASS: 45,714 assets / 11,934,427,298 bytes from all 17 archives, complete preflight, zero failures/conflicts/blocked/unsupported members. |
| 3. Arbitrary existing files | PASS using independent vanilla dump and imported project, with source files preserved. |
| 4. League name/icon | PASS dictionary rename/save/reopen and native logo replacement; runtime unverified. |
| 5. Team biography | PASS name, localized city, arena FK, arena name/city, primary/equipment RGB. |
| 6. Team logo | PASS copied native replacement/backups; runtime unverified. |
| 7. Center ice | PASS copied native center-logo replacement; whole-rink/runtime selection unverified. |
| 8. Roster/lines/scratches | PASS database and roster serialization for transactions, unique first-line assignment, scratch flag and number. |
| 9. Jersey/equipment colors | PASS native replacement, RGB edits and nine complete kits. Correct decoded jersey color raster inspected. PS3 3D assembly remains unsupported. |
| 10. Player biography/attributes/portrait | PASS biography, rating, career and portrait replacement; saved roster independently checked. |
| 11. Player equipment | PASS stick/tape fields and native equipment texture replacement, stock asset discovery. |
| 12. Localization | PASS dictionary review/re-encode/save/reopen; one pending dictionary operation remains. |

No PS3 Legacy emulator run or edited gameplay is claimed: Windows helper access remains unavailable after its initial/recovery failures. ISO/extraction passed in 364.39 seconds. Receipt: S:/TEMP/nhl-studio-overnight-20261003/nhllegacy/ps3-source-routes-1/receipt.json. All twelve checks have been reviewed with the stated runtime limitation; proceed to NHL16 PS4.
