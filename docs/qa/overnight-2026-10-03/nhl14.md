# NHL14 QA — Xbox360 and PS3 first pass

All twelve requested areas were reviewed on both platforms using owned copies. File edits, native texture replacement and database archive deployment/restore pass. **Edited gameplay remains unverified.** Xbox baseline crashes during loading; PS3 reaches its title prompt but automated Start input has not opened a usable menu. No NHL14 PS3 ISO was found.

Original games and emulators remain unchanged. Xbox evidence is under `H:/TEMP/nhl-studio-overnight-20261003/nhl14`. PS3 installation evidence is under `S:/TEMP/nhl-studio-overnight-20261003/nhl14`; edit, asset, project and runtime receipts are under `E:/Repositories/nhl-modding-studio/target/overnight-20261003/nhl14`.

| # | Requirement | Xbox360 | PS3 |
|---|---|---|---|
| 1 | ISO extraction/load | Pass: staged edit survives reopen; 5,374 players, 246 teams. | Unverified: recursive search of the supplied PS3 ROM folder found no NHL14 ISO. |
| 2 | Dumped-disc extraction | Complete after decoder repair: 42,028 assets, 17 archives, zero failures. | Supplied installation's packed archives verified on owned copy: 1,180 missing assets recovered, 40,716 retained, 17 archives complete. A pristine ISO-derived dump was unavailable. |
| 3 | Existing files/location | Independent X: installation imports into relocated H: project; source unchanged. | Relocated S: disc opens from disc root, PS3_GAME and USRDIR, each with 5,374 players and 246 teams. E: review project also opens. |
| 4 | League name/icon | QHL label persists; native league icon replacement passes. | Same edit and native icon checks pass. |
| 5 | Team biography | Name, city, actual arena link, venue name/city, primary and equipment RGB persist. Desktop QA Ducks rename survives Save/close/reopen. | Same file checks pass; desktop Honda Center link resolves. |
| 6 | Team logo | Native replacement changes decoded image; original and exact backup preserved. | Same replacement checks pass; desktop logo renders. |
| 7 | Center ice | centerlogo_0_cm native replacement passes. | Corresponding native rpsgl replacement passes. |
| 8 | Roster/lines/scratches | Release/sign/trade, unique first-line centre, scratch and jersey edits survive reopen. | Same checks pass. |
| 9 | Jersey/equipment colors | Native jersey replacement and RGB/tape pass; five-piece desktop kit, 10,708 triangles, ten maps. Seven assigned kits complete. | Native jersey slot 3 replacement and RGB/tape pass. Seven assigned kits complete; color/specular/normal and font previews verified visually. PS3 3D preview remains unsupported. |
| 10 | Player biography/attributes/portrait | Name, raw 49/display 85 aggression, career season and native portrait replacement pass. | Same; native portrait uses playerheadssmall family. |
| 11 | Player equipment | Shipped stick 18/24 and tape persist; equipment inventory resolves. | Same edit and inventory checks pass. |
| 12 | Localization | Sequential dictionary changes persist; one unsaved dictionary operation at a time remains a stated limitation. | Same checks and limitation. |

Texture tests exercise production preview, plan and import into fresh copies, compare decoded output and byte-exact backups. These establish file editing, not in-game asset selection. No compatible roster-save gameplay acceptance is claimed.

## Evidence and fixes

Xbox ISO copy produced 22 files / 6,766,594,263 bytes. Initial source-routes-1 finished in 1,775.87 seconds but its detailed receipt was partial: 41,187 assets / 14,221,075,116 bytes and an unreadable nested faceposer archive. The test summary incorrectly called zero known remaining members complete. The harness now requires both zero archive failures and a complete rescan; production preflight already correctly reported partial.

NHL14 faceposer uses EB version 0x00200400 with 16-byte entries and separate stored/decoded sizes. Added reader support after verifying all 841 real members have bounded, non-overlapping payloads and matching decoded lengths. Separate-size archive writes remain refused. The real-member regression passes; ten EB unit tests pass with one unrelated opt-in test ignored. Resume passes in 139.68 seconds: 841 members / 12,577,176 bytes written, 41,187 retained, 17 archives complete, no conflicts or failures. Initial receipt retained beside extraction-resume-1.json.

PS3 copy transferred 40,746 files / 18.741 GiB in 6:02 with no failures. Extraction reconciliation passes in 513.41 seconds: 1,180 members / 114,420,860 bytes written, 40,716 retained, no conflicts or failures. Receipt: target/overnight-20261003/nhl14-ps3-existing-receipt.json. All three folder-level discovery checks pass with matching database bytes and valid CRC. This is existing-installation acceptance, not ISO acceptance.

Xbox edit-roundtrip-2 passes in 23.76 seconds; PS3 ps3-edit-roundtrip-1 in 18.77 seconds. The initial Xbox harness assumed p_aggression; NHL14 uses c_aggression. Native asset/replacement tests pass in 26.75 seconds on Xbox (56 Anaheim textures) and 24.74 seconds on PS3 (82 textures). Final Xbox owned-source inventory passes in 9.41 seconds.

Copied cacheboot.big database deployment passes in 27.80 seconds on Xbox and 21.88 seconds on PS3: edited member matches the edited loose database with valid CRC, then both restore byte-exact. Xbox's initial wrong cache.big selection was refused before apply. PS3 archive/database input hashes matched the original before testing.

Desktop project creation passes on both platforms with blank deployment destinations. Uniform-tab remount picks up completed extraction: seven complete assigned kits and eight selector entries including Other. Bundled build passes with SHA256 `4CF051CC931431A32793F2B3EBFE55ABE54C1766601370C522B1084414FDDF9C`, including the decoder repair. PS3 visuals were verified in this build.

## Runtime and remaining limitations

Xenia edge crashes at PC 0x82751FAC/read 0x100000005. A fresh packed-only baseline copy passes in 108.03 seconds and repeats the same EA-intro crash without loose assets or edits. This rules out loose extraction as a prerequisite; the actual cause remains unresolved. Both logs are retained. Launching an isolated copy of the pre-existing Xenia master executable timed out at the desktop tool's app-approval gate, so no comparison result is claimed.

PS3 BLUS31179 in isolated RPCS3 reaches intro playback and the explicit Press Start prompt around 30 FPS. Configured Start (P), Cross (X), and a Return fallback did not establish usable menu input. Runtime log confirms active Keyboard handler and Start P; input timing or another cause remains unresolved. The baseline game and isolated emulator were closed; the original emulator remains untouched. Log: ps3-baseline-rpcs3-2.log. The first incorrectly located log-copy attempt left an empty file; it is not evidence.

Remaining blockers: missing PS3 ISO, Xbox baseline crash, PS3 automated menu input, PS3 3D preview, unqualified writes to separate-size nested EB archives, and single-pending localization. No edited gameplay is claimed. Earlier onboarding, career-year, native texture-preview and stale-uniform fixes remain covered by prior game reports; this pass does not assert they explain every Discord/GitHub report.
