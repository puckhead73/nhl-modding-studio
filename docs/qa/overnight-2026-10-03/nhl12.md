# NHL12 QA — both platform first passes reviewed, with blockers

Original ISO and independent extracted game under X:/Games/Emulation/Roms/XBOX360/NHL 12 remain read-only. Bulk QA copies now live in H:/TEMP/nhl-studio-overnight-20261003/nhl12 because the project drive became nearly full. Reports and source remain in the repository.

ISO database staging/edit/save/reopen passed with 5,395 players and 243 teams. The disc copy wrote 17 files / 6,027,541,543 bytes; archive extraction is still running (session17190). Source route receipt is not complete yet.

Copied database edit-roundtrip-2 passes15.85s with team/league/independent venue edits, roster transactions, unique lines, scratches/jersey, player biography/rating/career, stick24/18 and tape color, sequential localization saves, CRC and reopen. First run also passed serialization but selected stick29, which is not shipped by NHL12; its result is superseded by the title-correct second run.

Asset/texture tests2/2 pass9.46s against the independent extracted installation. All six native texture replacement routes pass, including XPR2 portrait/logo containers, jersey color slot2, playerstick/texlib_24 and centerlogo_0_cm. First texture run stopped at the inherited NHL11 stick path; its earlier outputs are retained. Smaller Xbox mip limitations remain; inspect per-texture receipt before deployment. Observations find67 Anaheim textures and7 complete assigned kit rows.

Found a real equipment lookup defect: NHL12 glove filenames use glove_<style>_<model>_<map>, while the generic parser treated style0/1 as the model. Added unambiguous map-shape recognition and matching rename behavior; legacy filename forms remain covered. Stock glove45 now resolves22 files instead of zero. Helmet6 resolves12 separate files. Their single-texlib completeness checks were incorrect; both required sets are now explicitly unmeasured on NHL12 Xbox. All26 equipment regressions pass, including real texture/compositor checks.

Added an import-source folder scan to locate the database/dictionary from a dumped disc or existing installation, with extraction offered before import when packed. This does not fill installation/save deployment destinations. Frontend tests/build are in progress; desktop verification follows.

| # | Requirement | Xbox360 status |
|---|---|---|
| 1 | ISO extraction/load | Complete after decoder fixes; 37,291 assets, zero remaining. |
| 2 | Dumped-disc extraction | Relocated copied disc extraction complete; resume preserves 37,289 existing assets. |
| 3 | Existing files/location | Independent dump API checks and relocated H: desktop import pass. |
| 4 | League name/icon | Serialized QHL and native logo roundtrip pass. |
| 5 | Team biography | Team and independently selected venue names/city pass; team-arena association/individual palette unresolved. |
| 6 | Team logo | Native replacement/backup pass; runtime pending. |
| 7 | Center ice | Native centerlogo_0_cm replacement pass; team-arena association unresolved. |
| 8 | Roster/lines/scratches | Transaction, unique slot and scratch/jersey save/reopen pass; runtime pending. |
| 9 | Jersey/equipment colors | Actual color slot2 replacement/tape edit pass; small mip limitations, desktop pending. |
| 10 | Player biography/attributes/portrait | Name/rating/career plus portrait replacement pass. |
| 11 | Player equipment | Valid shipped stick24/18 persists; glove lookup fixed, component completeness unmeasured. |
| 12 | Localization | Sequential save/reopen pass; only one pending text operation supported. |

No edited gameplay verified. Finish Xbox desktop/runtime and extraction before PS3.

Further checks: source-folder scan is visually verified in bundled D8E8E7B4...; it fills db/nhlng.db and fe/loc/nhl_eng_us.db automatically from the relocated H: folder, accepts133tables/16787strings, and keeps installation/save fields blank. All21 related frontend tests pass (the final added OpenProject integration test is included). Edit-roundtrip-3 PASS16.80s additionally proves the team's dictionary city label changes to QA and survives save/reopen independently of venue city. Raw team-city column/home-arena link remains unresolved; the dictionary city is editable. Source extraction17190 remains active, processing render assets (not stalled).

Edit-roundtrip-4 PASS18.19s additionally changes glove zone1 RGB to17/43/91 through the atomic three-cell API, verifies those channels after reopen, and confirms every other zone is unchanged. This proves team equipment-color serialization beyond stick-tape color. It does not establish runtime tint rendering. Desktop project now created with no game/save/database deployment destinations; chosen Skip on the extraction screen to review DB while the separate QA extraction continues. At that screen preflight showed27,771/37,291 members present across12 archives; this is a partial checkpoint, not completion.
Extraction initial pass finished1785.39s:37,289assets/12,342,573,657bytes,12archives,2unsupported. Both investigated: crowd_29 uses stored chunklzx flag4; full decoded786592bytes now matches independent plaintext. Nested faceposer_983 uses version0x00100400 with9908stored/19484decoded; fixed bounds/decode, no independent plaintext available for that nested member. Writer refuses separate-size variant. Codec46/EB10unit checks pass; owned-copy resume is running and completion is not yet claimed. Desktop Anaheim uniforms visually renders jersey/head/pants,5pieces/12100triangles/9maps and7completekitrows; gloves remain untextured.
Xbox first-pass checklist reviewed with explicit limits. The unedited copied baseline crashes in isolated Xenia at PC0x8273DE9C, read0x100000005, before a usable menu. Log H:/TEMP/nhl-studio-overnight-20261003/nhl12/xenia-baseline.log retained; emulator closed. No edited gameplay accepted. Existing-file desktop import, all twelve API/asset surfaces and equipment-color persistence have been reviewed. Remaining acceptance limits are runtime, glove texture preview, unmeasured component completeness/home-arena association, small Xbox mip replacement, and single pending localization operation. Decoder recovery is still running against the owned copy; initial partial extraction receipt remains intact.
Both decoder regressions pass, including all778 nested faceposer members bounded/decoded with declared lengths. All50 existing real LZX vectors pass. This closes decoder investigation but does not yet claim the full resume job complete. Proceeding to PS3 while that owned-copy verification finishes.

Final Xbox extraction resume PASS304.32s:2recovered/806076bytes,37289retained,zero conflicts/unsupported/remaining,complete. This supersedes pending recovery notes above. PS3 edit-roundtrip-1 PASS16.96s including team dictionary city and atomic equipment RGB preservation. PS3 ISO source run52851 active; first dispatch failed linking because the Xbox resume still held the executable, retried after it completed. No source output was created by that failed dispatch.
PS3 ISO route blocked: supplied NHL 12 (USA) (En,Fr).iso has unreadable boot.big bytes (20AFEB95 prefix), while the independent installation has the expected EB0003 archive. Copying the filesystem is not game extraction acceptance. The image appears encrypted or damaged; no disc key was found adjacent to it. Added boot-container validation before creating the destination, with an actionable decrypted-ISO/existing-installation error. Synthetic regression passes; real refusal verification pending. Existing installation is being copied to a fresh owned QA folder for the remaining requirements. PS3 ISO remains unsupported for this supplied input; no user action is needed to continue testing the readable installation.
PS3 desktop project created at H:/TEMP/nhl-studio-overnight-20261003/nhl12/ps3 desktop project. Folder scan resolves an arbitrary disc parent to PS3_GAME/USRDIR and imports133tables/16409strings with installation/save destinations blank. Selected Skip while separate API extraction76184 runs; this is not asset extraction completion. Initial desktop preflight13337/37125members across12containers. Isolated RPCS3 boots the readable copied disc BLUS30791 and is compiling PPU modules; optional bundled3on3NHLArcade package skipped. No edited runtime result.

| # | PS3 requirement | Current evidence |
|---|---|---|
| 1 | ISO extraction/load | Blocked: supplied image unreadable/encrypted; early refusal now verified before output. |
| 2 | Dumped-disc extraction | Complete: 37,125 members across 12 archives; zero failures or conflicts. |
| 3 | Existing files/location | Relocated H: parent detected, DB/dictionary imported in desktop. |
| 4 | League name/icon | QHL serialization and native league icon replacement pass. |
| 5 | Team biography | Team/city/independent venue and equipment RGB save/reopen pass; home-arena relationship unresolved. |
| 6 | Team logo | Native replacement, changed pixels and preserved backup pass. |
| 7 | Center ice | Native center-logo replacement passes; arena relationship unresolved. |
| 8 | Roster/lines/scratches | Transaction and unique-line/scratch/jersey save/reopen pass. |
| 9 | Jersey/equipment colors | Atomic RGB/tape and native jersey replacement pass; desktop maps visually verified. |
| 10 | Player biography/attributes/portrait | Biography/ratings/career and small portrait replacement pass; large portrait unqualified. |
| 11 | Player equipment | Stick/tape persists; glove45 resolves22 files and helmet6 resolves12; completeness unmeasured. |
| 12 | Localization | Sequential text save/reopen pass; one pending text operation limit remains. |
PS3 baseline reached title/intro at roughly30FPS with the copied readable installation. Keyboard mapping confirmedPlayer1 Keyboard,StartP,CrossX; input sent but no usable menu established. Closed through ExitGame confirmation. This proves only startup/rendering, not edited gameplay or controller acceptance. Remaining extraction76184 stillactive.
PS3 native assets/textures2testsPASS10.52s.88Anaheimtextures,7completeassignedkits,0spares. Sixreplacement routes preserve source/backups and change decoded pixels: teamlogo,leaguelogo,smallportrait(playerheadssmall/p0_4000/p1.big),jerseycolor slot3,playerstick24,centerlogo_0_cm. Jerseycolor slot3 PNG visually verified. Large playerheads family absent in this dump; smallportrait is the qualified test. Glove45 has22files andhelmet6 has12; required-component checks corrected to unmeasured onPS3 too,4unit regressionsPASS. Runtime remains unverified. Full extraction stillrunning, so these observations are a checkpoint; final inventory may include further files.

Final PS3 evidence: copied-installation extraction PASS1013.09s, 23,788 files written/10,728,416,730 bytes plus13,337 retained;12archives,zero unsupported/conflicting/failed,complete.5395players243teams; source DB preserved. Final asset inventory PASS9.29s confirms88Anaheimtextures/seven complete kits. Bundled build AF75205529773457929FAEB5BACC6BB19DC06BAD608961C8F0D6044ADFEB8ECE desktop visually verifies jersey color/specular/normal maps and seven kits after reopening the project. PS3 3D kit preview remains unsupported with an explicit notice. All twelve requirements reviewed; ISO and runtime blockers remain, so this is a first-pass review, not full game acceptance. Originals unchanged. Proceed NHL13 Xbox360.

