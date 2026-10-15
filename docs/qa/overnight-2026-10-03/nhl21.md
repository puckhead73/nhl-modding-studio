# NHL21 QA — first pass complete with blockers

The supplied PS4 RAR contains a 34,857,746,432-byte base PKG. Standalone package extraction completed (63/63); Studio's production archive extraction passed in 1,516.07 seconds on that fresh owned F: copy: 16 archives, 169,094 files, 98,361,235,571 bytes, zero failures or blocked members. The fresh database opens 7,247 players and 433 teams. Direct PKG onboarding has now passed a real NHL20 package job; release bundling and desktop qualification remain unfinished.

Read-only inventory of the genuine installed CUSA18182 database opens 7,247 players and 433 teams in 29.81 seconds. The title's English dictionary is a private installed fixture and is copied only into QA projects, not bundled. No vanilla roster container is supplied; career/profile/season saves are not substituted. Main-database editing passes in 63.30 seconds. Six native GNF replacements pass in 12.54 seconds, with decoded-image, dimensions, backup and original-preservation checks. Imported project creation passes in 3.63 seconds with blank deployment destinations.

| Requirement | PS4 finding |
|---|---|
| 1. ISO extraction/load | No ISO supplied; supplied PKG extraction completed. Studio PKG onboarding passed with NHL20; NHL21 uses the standalone extraction followed by production archive extraction. |
| 2. Dumped-disc extraction | Complete: all 16 archives extracted with zero remaining members or codec failures. |
| 3. Arbitrary existing files | Existing installation, copied project import/save/reopen, and relocated package output pass. Direct game folder and parent folder resolve the same DB; fresh DB is byte-identical to installed DB. Relocated qualification passes in 32.34 seconds. |
| 4. League name/icon | Localization save/reopen and native league-logo replacement pass. |
| 5. Team biography | Main DB abbreviation, nickname, localized full name/city, arena association, arena name/city and packed frontend colors persist. |
| 6. Team logo | Native large/t0.gnf replacement passes. |
| 7. Center ice | Native ice_centerlogo_0_cm.gnf replacement passes; vault-based venue ownership and full-rink reassignment remain unverified. |
| 8. Roster/lines/scratches | Main DB release/sign, trade, line-slot uniqueness, scratches/jersey persist. Roster-container verification blocked by missing compatible vanilla roster save. |
| 9. Jersey/equipment colors | Native jersey replacement and fifteen packed equipment color zones persist. Numeric kit/3D bindings remain unverified; explicit filename-candidate browsing passes on the relocated output, including the nineties Anaheim away jersey. Filename matches remain unverified team ownership. |
| 10. Player biography/attributes/portrait | Main DB biography/rating/career save-reopen and native portrait replacement pass. |
| 11. Player equipment | Main DB stick/tape fields and native tape texture replacement pass; actual rendered equipment unverified. |
| 12. Localization | League/city/full-name and sampled text persist using the genuine installed dictionary. |

Archive apply/restore passed in 553.01 seconds on a copied inxp_00.big, including nickname change, CRC/decode verification and byte-exact restoration. Evidence is under target/overnight-20261003/nhl21. Originals are read-only; bulk package output is F:/TEMP/nhl-studio-overnight-20261003/nhl21/pkg-source-routes-1/CUSA18182.

The package and installed game are not byte-identical: 15 of 16 BIG archives match, but game_90.big differs (installed 17,046,806,716 bytes; package 17,045,142,304 bytes). Both directories contain 92,322 members. Two entries differ in offset/size: aaron_ekblad.mcd and head_aaron_ekblad_cm.gnf under nhl/data/kettle/rendering/characters/player/. This establishes a pre-existing source difference, not its cause or equality of the remaining payloads. sce_sys/npbind.dat also differs. Neither source was modified by the comparison; installed assets are not labeled vanilla.

Desktop Studio and emulator verification remain blocked by the Windows helper. No NHL21 edited gameplay is claimed. Shared fixes from the earlier game passes address portions of GitHub issue 1 and Discord feedback, with the same remaining runtime qualification limits.


