# NHL11 QA — first pass complete with runtime blockers

Xbox360 source: `X:/Games/Emulation/Roms/XBOX360/NHL 11/NHL 11 (USA) (En,Fr).iso`, read-only. New copied targets under `target/overnight-20261003/nhl11`.

Direct ISO database staging, edit/save/reopen and original-byte preservation passed: 5,310 players / 242 teams. Full disc copy reports 19 files / 4,903,321,495 bytes; archive extraction still running in session1473.

Copied database edit-roundtrip-2 PASSED14.32s: league retext, team name, independently selected arena name/city, release/sign/trade, line uniqueness, scratch/jersey state, player name/rating, career row, stick/tape, save/reopen and CRC. NHL11 uses +36 rating encoding; the inherited NHL10 +50 test expectation was corrected, with stored49 displaying85. First attempt's outputs retained.

Desktop, native assets, runtime and PS3 checks remain pending. No edited gameplay accepted.

## Xbox360 checklist (extraction complete; baseline runtime blocked)

| # | Requirement | Evidence / limitation |
|---|---|---|
| 1 | ISO extraction/load | ISO staging, edit/reopen and 19-file copy pass; two codec bugs fixed and full extraction recovered. |
| 2 | Dumped-disc extraction | 30,635 extracted members after production resume; zero unsupported or remaining members. |
| 3 | Location-independent files | Copied relocated root imported through desktop; independent existing extracted installation read successfully. |
| 4 | League name/icon | QHL dictionary retext survives save; league logo native replacement passes. |
| 5 | Team biography | Team name and separately selected venue name/city persist. No measured home-arena link or separate team city/RGB fields. |
| 6 | Team logo | Native replacement and original backup verified. |
| 7 | Center ice | Native centerlogo_0_cm replacement passes; arena-to-home-team association unavailable. Packed small mips retained. |
| 8 | Roster/lines/scratches | Release/sign/trade, unique line slot, scratch/jersey state survive reopen. |
| 9 | Jersey/equipment colors | 51 Anaheim textures, five complete assigned kits, one incomplete spare. Jersey and tape-color writes pass; packed small mips retained. Gloves untextured in mesh preview. |
| 10 | Player biography/attributes/portrait | Name, 85 aggression, career row and native portrait edits pass. NHL11 +36 rating scale used. |
| 11 | Player equipment | Shipped stick18/29 and tape fields persist; native stick texture replacement passes. Helmet component completeness was falsely classified and is now unmeasured. |
| 12 | Localization | 14,920 strings imported; sequential edits save/reopen; one pending operation limitation remains. |

The API tests establish serialized edits, not runtime application. Neither title screens nor mesh previews establish edited gameplay.

Asset observations from the independent existing extracted installation (read-only) PASS11.64s: 51 Anaheim textures, five complete kit records, 14 leagues. Texture-roundtrip-2 PASS8.15s for team logo, p0_4000/p1.big portrait, league logo, jersey color slot2, stick29 and centerlogo_0_cm. Xbox jersey/center-ice large mips change but packed tails remain; stick is base-only, explicitly disclosed. Source files preserved. Helmet false-incomplete contract fixed for NHL11 Xbox: actual disc contains separate helmet/chinstrap meshes and tint maps, no texlib. Required set now honestly unmeasured, four component regressions PASS. Real observation recheck running session49085; source extraction session1473 still active.

Initial extraction completed755.55s with30,594 files/11,397,578,553 bytes and41 unsupported. Fixed LZX frame padding before new block headers and prevented a second alignment after their trees. All34 LZX failures match independent plaintext exactly; prior50 vectors PASS. Nested EB archives carry compressed size at+4 and decoded size at+8: reader now uses the compressed length for Gen7 variant0x400. All10+638+821 nested entries decode; synthetic bounds/read-only-writer regression PASS, EB suite10PASS+1ignored. Full copied extraction resume is underway (session76576), preserving the initial receipt. Desktop D412D91 shows six kit choices (five complete assigned rows plus one incomplete unassigned spare), five mesh pieces/13,032 triangles/nine maps; jersey/head/pants visible, gloves untextured. Imported131tables/14920dictionary strings. No edited gameplay.

Runtime before repaired extraction: isolated Xenia loading-screen crash PC0x82AE44A0, read0x100000005; log xenia-before-extraction-repair.log. This is provisional: repeat after extraction recovery before classifying runtime compatibility. Original game/emulator untouched. QA Studio closed for rebuild with the decoder and helmet fixes.

Production extraction recovery PASSED465.89s: wrote all41 missing members/184,735,556 bytes, retained30,594 files, zero conflicts/unsupported/unsafe/failed archives. Preflight complete, zero remaining. Receipt extraction-resume-2.json. Bundled desktop2573D018739B78CAD8B7B7EC8857DEA16BC9E3B2EF7D5C4D3F41DDC7E3588915 includes decoder/helmet fixes. Existing50-file LZX vector regression also passes after EB reader change. Xbox baseline repeat underway.

Xbox baseline repeat after complete extraction still crashes during loading at PC0x82AE44A0. Preserved xenia-after-extraction-repair.log; isolated emulator closed. This remains an unedited-baseline runtime blocker. All twelve Xbox requirements reviewed; serialized edits pass, edited gameplay remains unverified. Continue PS3.

## PS3 source and editing evidence

The decrypted ISO copies 39 files with full Joliet names; a source-to-owned-copy manifest comparison passes every filename and length (`ps3-disc-manifest.json`). This comparison is against our copied disc, not an independent extraction. The extracted main DB separately matches the independent pre-existing dump byte for byte.

Initial source test failed after the successful disc copy because NHL11 packs its database in boot.big. Corrected extraction order and reused that completed copy. Production extraction PASS319.10s:31,247 members/11,795,595,878 bytes across14 archives, zero conflicts/unsupported/failures, complete. Receipt `ps3-source-routes-1/receipt.json`. The failed initial test did not save its copy-job receipt; the final receipt explicitly discloses that limitation.

The same layout exposed an actual onboarding defect: Ps3DiscPanel rejected a missing loose database before container extraction. It now offers extraction on the copied USRDIR before importing, locks source fields during the job and rescans on Use extracted files. All6 frontend tests and production build pass. This is component/API evidence; the completed real extraction was run through the production backend, not the newly added panel.

Copied DB edit-roundtrip-1 PASS11.55s for league text, team/independent arena name-city, transactions, unique line slot, scratches/jersey, biography, +36 rating scale, career, stick/tape, localization, CRC/reopen. Native asset/texture tests2/2 PASS8.52s on the newly extracted copy:67 Anaheim textures,14 leagues; team/player/league logo, jersey color slot3, stick29 and centerlogo_0_cm replace/decode with exact original backups. The old independent extracted folder lacked stadium/icesurface; the new extraction recovered both. Jersey color slot3 visually verified against exported PNG.

PS3 helmets use the same separate chinstrap/helmet/tint layout as Xbox, so extended the false-incomplete fix to PS3. Four component regressions pass. Required component set remains unmeasured rather than falsely complete.

| # | Requirement | PS3 evidence / limitation |
|---|---|---|
| 1 | ISO extraction/load | Decrypted ISO copied, packed database extracted and opened; encrypted images unsupported. |
| 2 | Dumped-disc extraction | New relocated dump fully extracted,14 archives complete. |
| 3 | Location-independent files | Arbitrary copied root opens5310players/242teams; desktop import review accepts131tables/14630strings. |
| 4 | League name/icon | QHL retext and native league logo roundtrip pass. |
| 5 | Team biography | Name and independent venue name/city persist; team home-arena link and separate team city/RGB unresolved. |
| 6 | Team logo | Native replacement/decoded change/source backup pass. |
| 7 | Center ice | Recovered centerlogo_0_cm replacement passes; automatic arena/home-team association absent. |
| 8 | Roster/lines/scratches | Transactions/line uniqueness/scratch and jersey state survive reopen; no compatible runtime save captured. |
| 9 | Jersey/equipment colors | Actual jersey color slot3 replacement and tape-color field persist; PS3 mesh preview unsupported. |
| 10 | Player biography/attributes/portrait | Name/aggression/career plus native portrait roundtrip pass. |
| 11 | Player equipment | Stick18/29 and tape persist; stick texture replacement passes, helmet completeness unmeasured. |
| 12 | Localization | Copied dictionary retext/save/reopen passes; sequential-save limitation remains. |

Isolated RPCS3 baseline reaches NHL11 title/intro near60FPS, but menu input not yet accepted. No edited gameplay claim. Desktop build475C3A8B1B793F7BCE34DE4E899B7773E9F1594574D4F99EC76F41E7F86A4276 is creating the PS3 project; visual review pending.


Final PS3 desktop: project created and opened242teams; game/save/database deployment paths stay empty, asset root references only the owned QA copy. Uniforms shows6choices (5complete assignedrows +1incomplete spare), compact unsupported3D notice, jersey color/specular/normal and number-font maps visually verified. No original destinations. Baseline RPCS3 title/intro remained stable near60FPS but P/X input did not reach a usable menu. Exited through confirmation. No PS3 runtime edits attempted, no edited gameplay acceptance. All12 requirements reviewed with the limits above; proceed NHL12Xbox360.
