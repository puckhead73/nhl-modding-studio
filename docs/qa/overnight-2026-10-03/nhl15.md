# NHL15 QA — Xbox360, PS3 and PS4 findings

ISO staging with edit/save/reopen passes: 5,534 players and 246 teams. Original ISO unchanged. Full disc copy produced 170 files / 6,509,370,129 bytes; complete extraction is recorded under S:/TEMP/nhl-studio-overnight-20261003/nhl15/source-routes-1.

Copied database edit/reopen passes in 23.10 seconds, including league/localization, team biography and arena link, primary/equipment colors, player transactions/lines/scratches, player biography/attributes/career and equipment. Native asset and six replacement tests pass in 34.87 seconds using read-only independent input: 77 Anaheim textures, nine complete kit rows, changed decoded replacements and preserved source/backups. No edited gameplay is claimed.

Production project creation passes in 1.73 seconds at target/overnight-20261003/nhl15/desktop project. Database/dictionary are imported copies; assets point only to the owned S: dump and deployment destinations are blank. Desktop and archive deployment checks are detailed below. PS3 and PS4 results follow below.

## Xbox360 checklist (reviewed with runtime blocker)

| Requirement | Finding |
|---|---|
| 1. ISO extraction/load | PASS: production ISO staging, player edit/save/reopen; 5,534 players and 246 teams. |
| 2. Dumped-disc extraction | PASS: 170 files / 6,509,370,129 bytes materialized from ISO; all 17 archives decoded into 42,034 assets / 12,630,312,112 bytes. Zero failures, conflicts, unsupported or remaining members; complete preflight. |
| 3. Arbitrary existing files | PASS: independent dump imports and relocated owned dump opens with dictionary; project database and dictionary are copied. |
| 4. League name/icon | PASS at file level: NHL renamed QHL and reopened; native icon replacement decoded and backup preserved. In-game display unverified. |
| 5. Team biography | PASS: name, localized city, actual arena foreign key, primary and equipment RGB values survive save/reopen. Desktop displays Anaheim, Honda Center and logo. |
| 6. Team logo | PASS: native replacement on copied package; original and backup preserved. Runtime unverified. |
| 7. Center ice | PASS for centerlogo_0_cm replacement. This does not establish complete rink-surface art editing or runtime selection. |
| 8. Roster/lines/scratches | PASS: release, sign, trade, unique first-line center, scratch flag and jersey-number persistence. Gameplay unverified. |
| 9. Jersey/equipment colors | PASS: native texture and equipment RGB roundtrips. Desktop assembles 5 pieces / 10,708 triangles / 10 maps; 9 of 9 assigned kit rows complete. |
| 10. Player biography/attributes/portrait | PASS: biography, c_aggression raw 49 / displayed 85, Crosby 2007-08 career and portrait replacement. Runtime unverified. |
| 11. Player equipment | PASS: stick and tape selections persist; discovered stock equipment assets. No claim that the game equips them. |
| 12. Localization | PASS: dictionary review, re-encode and save/reopen. Only one dictionary operation can currently be pending; UI explicitly warns to save each before another. |

Source test completed in 759.34 seconds. Receipt: `S:/TEMP/nhl-studio-overnight-20261003/nhl15/source-routes-1/source-routes.json`. Real cacheboot.big apply/decode/CRC/restore passed in 25.16 seconds; receipt: `target/overnight-20261003/nhl15/archive-roundtrip-1/qa-receipt.json`. Database and archive restore byte-for-byte. Desktop build SHA256: 4CF051CC931431A32793F2B3EBFE55ABE54C1766601370C522B1084414FDDF9C.


The final Xbox baseline, after complete extraction, repeats the unedited EA-intro crash at PC 0x82762974 (read 0x0000000100000005). Both attempts are logged under target/overnight-20261003/nhl15. No edited gameplay is verified, and the cause is not established. All twelve Xbox requirements have been reviewed with this runtime limitation; continue to PS3 rather than treating launch as acceptance.

## PS3 checks

Independent existing files load, copied database edits/save/reopen pass in 22.56 seconds, and native assets plus six replacement roundtrips pass in 18.60 seconds: 104 Anaheim textures and nine complete assigned kits. Real cacheboot.big apply/decode/CRC/restore passes in 26.17 seconds. Fresh project creation passes in 1.85 seconds; Studio opens 246 teams with Anaheim biography, arena link and logo. All deployment destinations are blank.

PS3 ISO extraction PASSED in 377.36 seconds: 30 disc files / 7,212,236,128 bytes, 41,902 decoded assets / 11,366,320,679 bytes, 17 archives, no failures/conflicts/unsupported members, complete preflight. Loaded 5,534 players / 246 teams, dictionary attached, database matches the independent dump. Receipt: S:/TEMP/nhl-studio-overnight-20261003/nhl15/ps3-source-routes-1/receipt.json. Studio native jersey color/specular/normal maps are visually correct; nine assigned kits complete (selector also includes Other). PS3 3D assembly remains unsupported and explicitly labelled. Baseline reaches Press Start at roughly 30 FPS but repeated mapped P and X input do not open a menu. No edited gameplay verified.


| Requirement | PS3 finding |
|---|---|
| 1. ISO extraction/load | PASS using supplied decrypted ISO and production asynchronous disc-extraction API; DB matches independent dump. |
| 2. Dumped-disc extraction | PASS: packed archives extracted from the materialized relocated disc; all 17 complete. |
| 3. Arbitrary existing files | PASS: independent X: dump and owned S: disc detected, copied project opens with dictionary and 246 teams. |
| 4. League name/icon | PASS for rename/save/reopen and decoded native icon replacement on copies; runtime unverified. |
| 5. Team biography | PASS for name, localized city, arena FK, primary/equipment colors; desktop shows biography, arena and logo. |
| 6. Team logo | PASS copied native replacement and preserved original/backup; runtime unverified. |
| 7. Center ice | PASS for native center-logo image replacement. Full rink surface/runtime selection unverified. |
| 8. Roster/lines/scratches | PASS for release/sign/trade, first-line center uniqueness, scratch flag and number persistence. Gameplay unverified. |
| 9. Jersey/equipment colors | PASS for native texture and RGB persistence. All nine kits complete; actual jersey color/specular/normal previews verified. PS3 3D assembly unsupported. |
| 10. Player biography/attributes/portrait | PASS for biography, aggression, career and native portrait roundtrips. Runtime unverified. |
| 11. Player equipment | PASS for stick/tape selections and asset discovery; equipped runtime appearance unverified. |
| 12. Localization | PASS dictionary re-encode/save/reopen; single pending-operation limitation remains explicit. |

Owned ISO-extracted asset inventory also passes independently in 22.72 seconds. PS3 evidence lives under target/overnight-20261003/nhl15/ps3-*; native test inputs were read-only and replacements used fresh copied targets. Baseline runtime is blocked at the Start prompt; all twelve PS3 requirements reviewed with the limitations above. Proceed to PS4.

## PS4 checks

The base database opens with 5,555 players and 246 teams. Native file replacement passes for league PNG, team PNG, portrait PNG, jersey GNF, equipment GNF and center-logo GNF, each on a new copy with changed decoded content and preserved original/backup (7.97 seconds). Database edits passed in 31.54 seconds; after implementing the NHL15 chunkzip roster codec, mirrored database/DATA editing and independent reopening of DATA passed in 37.59 seconds. The actual fresh-project workflow also passed: project save/reopen edits the roster as its primary file and leaves the main database unchanged (11.40 seconds).

Two production fixes resulted: PS4 logo discovery now resolves `databigs/Extracted` and other supported loose layouts while keeping canonical archive member names for archive edits; NHL15 roster DATA now decodes and repacks native 128 KiB chunkzip records. The baseline inventory now includes this previously rejected roster and its companion files. Save support remains experimental: record headers and other records are preserved, but emulator/console acceptance and companion-file semantics are unverified. All 15 save-codec tests pass, including later-title LZ4 regressions.

| Requirement | PS4 finding |
|---|---|
| 1. ISO extraction/load | NOT VERIFIED: available NHL15 source is a RAR containing an extracted CUSA00561 game dump, not an ISO. No NHL15 ISO was found in the PS4 ROM folder. Universal PS4 ISO onboarding is not established. |
| 2. Dumped-disc extraction | BLOCKED/PARTIAL: fresh owned RAR dump produces 70,116 assets / 45,745,468,835 bytes from 17 archives without codec failures, but 3,487 members remain blocked across 1,743 colliding paths. Database and dictionary discovery therefore fail; this is not a successful import. |
| 3. Arbitrary existing files | PASS for direct anchor/database imports and the existing extracted installation. Import project created with copied DB, dictionary and roster, arbitrary owned asset root, blank destinations. The partially extracted fresh dump cannot yet supply its database automatically. |
| 4. League name/icon | PASS at file/API level: dictionary rename/save/reopen and PNG replacement. Fixed extracted-logo path resolution; Rust four-layout test and five frontend tests pass. New desktop visual blocked by Windows helper failure. |
| 5. Team biography | PASS for name, city, arena foreign key, primary/equipment RGB and save/reopen. Roster independently contains team rename after mirrored API save. |
| 6. Team logo | PASS for copied native PNG replacement and resolved loose path. Archive member names remain unchanged. In-game appearance unverified. |
| 7. Center ice | PARTIAL: copied centerlogo_0_cm.gnf replacement passes. With defs loaded, Anaheim level 0 uniquely references center-logo key 0, center-line 13 and ice-line 0. Venue reassignment/full rink editing remains unsupported; runtime selection unverified. |
| 8. Roster/lines/scratches | PASS at API/serialization level: release/sign/trade, unique first-line center, scratch/number persistence. NHL15 DATA codec fixed; independently reopened DATA contains roster and attribute changes. No gameplay verification. |
| 9. Jersey/equipment colors | PASS for jersey GNF replacement and database equipment RGB. Gen7 kit completeness contract is unmeasured for this PS4 title; no new 3D/desktop acceptance claim. |
| 10. Player biography/attributes/portrait | PASS for biography, rating 85 stored directly, career and PNG portrait roundtrips. Roster DATA independently reopened with edited rating. |
| 11. Player equipment | PASS for stick/tape persistence and GNF equipment replacement. All five PS4 catalogue families resolve as enum-only for this title; it ships no Creation Zone list. No runtime equipment appearance claim. |
| 12. Localization | PASS for dictionary review/re-encode/save/reopen. One pending dictionary operation remains the explicit limitation. Base/patch dictionary selection remains unresolved during fresh extraction. |

Collision comparison reads both versions without selecting or overwriting either: only eight groups are byte-identical; 1,735 genuinely differ. Base inxp00.big database and dictionary match the committed anchors; patch_inxp00.big contains different versions. Existing format research also leaves runtime base/patch precedence unresolved. A future explicit source-selection workflow or verified precedence is needed; naming alone was not used to overwrite conflicting content.

Receipts under target/overnight-20261003/nhl15: ps4-source-receipt-1.json (failed discovery and partial extraction), ps4-collisions-1.json, ps4-observations-2.json (real logo paths), ps4-observations-3.json (defs-aware venues/equipment), ps4-texture-roundtrip-1, ps4-edit-roundtrip-1 and ps4-roster-roundtrip-1. Bulk outputs are under S:/TEMP/nhl-studio-overnight-20261003/nhl15/ps4-source-routes-1. Originals were read-only.

Desktop/runtime limit: the Windows helper returned `foreground window did not report a process id` on both the initial attempt and the recovered-window retry. No further desktop input was issued. The open QA build 93918A1035F9D301F26991671A523685A4CC290B66A4262C2038F2DC94F84D94 includes logo changes but predates the roster codec. No NHL15 PS4 emulator run or edited gameplay is verified. GitHub issue 1 and Discord onboarding/uniform/arena complaints remain traced through the shared fixes and earlier game reports; these PS4 fixes do not close all of those reports.

Copied PS4 base inxp00.big deployment passed in 174.06 seconds: actual team-name edit applied through the transaction API, decoded archive member equals the edited database and passes its header CRC, then both archive and loose DB restore byte-for-byte. Source archive remains unchanged. Receipt: target/overnight-20261003/nhl15/ps4-archive-roundtrip-1/qa-receipt.json. This checks an explicitly selected base archive; it does not resolve runtime patch precedence.

All twelve requirements have now been reviewed on each applicable NHL15 platform. Remaining blockers above are carried forward; this is a completed first pass, not full user acceptance. Next: the 2015 Legacy Xbox360/PS3 release, then NHL16 PS4.

Follow-up onboarding polish from the partial PS4 extraction: completion now awaits the fresh preflight before notifying its parent, displays unresolved-file completion explicitly, and disables another no-op extraction when all remaining files are blocked. Five focused frontend tests and production frontend build pass. The broader project-workspace suite passes all eight tests, including creating every available shipping baseline.
