# NHL13 QA — Xbox360 and PS3 first pass

All twelve requested surfaces were reviewed on both platforms using owned copies. Extraction, database edit/reopen, six native texture replacements and database archive apply/restore pass. **Edited gameplay remains unverified.** Xbox baseline crashes on loading; PS3 baseline renders its title/intro but automated input has not reached a usable menu.

Original ISO, extracted installation and emulator files remain unchanged. Evidence root: `H:/TEMP/nhl-studio-overnight-20261003/nhl13`.

| # | Requirement | Xbox360 | PS3 |
|---|---|---|---|
| 1 | ISO extraction/load | Pass: staged edit survives reopen; 5,412 players/248 teams. | Pass: decrypted ISO through production asynchronous onboarding job; DB matches independent dump. |
| 2 | Dumped-disc extraction | Pass: 37,571 assets/12,924,322,128 bytes,15 archives,zero failures. | Pass: 37,413 assets/11,602,103,926 bytes,15 archives,zero failures. |
| 3 | Existing files/location | Independent installation and relocated H: project open. | Relocated H: disc resolves through PS3_GAME/USRDIR; copied project opens. |
| 4 | League name/icon | QHL dictionary label and native icon replacement persist. | Same database and native icon checks pass. |
| 5 | Team biography | Name, city, actual arena foreign key, venue name/city, primary RGB and equipment RGB persist. | Same checks pass; desktop Honda Center link resolves. |
| 6 | Team logo | Native replacement decodes differently; original and backup preserved. | Same; desktop exposes six logo families. |
| 7 | Center ice | centerlogo_0_cm native replacement passes. | Native rpsgl replacement passes. |
| 8 | Roster/lines/scratches | Release/sign/trade, unique first-line centre, scratch and jersey edit save/reopen pass. | Same checks pass. |
| 9 | Jersey/equipment colors | Native jersey replacement and RGB/tape edits pass; desktop seven-piece kit renders. | Native jersey slot3 replacement and RGB/tape pass; 3D mesh preview unsupported. |
| 10 | Player biography/attributes/portrait | Name, scaled aggression, career season and portrait replacement pass. | Same; PS3 uses playerheadssmall family. |
| 11 | Player equipment | Shipped stick18/24 and tape persist; glove11-file measured set, helmet13 files including texlib. | Same equipment edit/inventory checks pass. |
| 12 | Localization | Sequential renames persist; only one unsaved dictionary operation can be held. | Same; limitation now explicitly visible in rename panel. |

All texture changes above use production preview/plan/import and a fresh output copy, then compare decoded output and byte-exact backups. These establish file editing, not game selection or rendering of the edited assets. No compatible roster-save gameplay acceptance is claimed.

## Evidence

- Xbox source-routes-1:20discfiles/6,313,424,119bytes; complete in1562.63s. PS3 ps3-source-routes-1:28discfiles/8,055,581,904bytes; complete in1137.22s. Both production extraction rescans report complete.
- Xbox edit-roundtrip-2:18.83s; PS3 ps3-edit-roundtrip-1:19.48s. Actual arena foreign key kOtt.buDa and primary RGB17/43/91 additionally verified after reopening.
- Xbox texture/inventory:2tests27.50s,61Anaheimtextures/seven assigned complete kits. PS3 ps3-texture-roundtrip-2 and ps3-asset-observations-2.json:2tests11.46s,64textures/seven complete kits while extraction finished. Original older PS3 dump lacked rendering files; initial failed texture-roundtrip-1 retained as incomplete-input evidence.
- Xbox archive-roundtrip-2:318.91s; PS3 ps3-archive-roundtrip-1:345.45s. Edited copied cache.big database member decodes exactly to edited loose DB with validCRC; archive and database restore byte-exact. Source preserved. First Xbox attempt selected boot.big incorrectly and was refused before apply.
- Xbox desktop:248teams,20leagues/18,826dictionarystrings,seven-piece uniform/19,536triangles/15maps visually rendered. PS3 desktop:248teams,arena link,six logo families,seven complete kit assignments. Final native jersey preview verification follows refreshed build.

## Fixes and runtime limits

Uniform asset queries had cached an empty scan indefinitely when files were extracted externally. Returning to the tab now rescans both outfit and kit inventory; changing asset-folder overrides also selects a distinct outfit cache. Two new component regressions plus native grouping test pass; production frontend and bundled desktop build pass. New build SHA256 B68745288A2F589BCFBFAC774DA352EEAA65D5A998D30AF2483BD5CECE46DC0F. This addresses another route to the empty-uniform feedback; it does not claim every prior report had this cause.

The dictionary rename panel now warns that holding another operation replaces the unsaved one. Save each operation before editing another string; batch pending localization is not implemented.

Xbox unedited baseline repeats PC0x82600994/read0x100000005 before and after complete extraction. Both Xenia logs are preserved. PS3 BLUS31003 in isolated RPCS3 reaches title/intro around30FPS; configured Start(P)/Cross(X) do not establish usable-menu access. Closed only the QA game afterward. No matching emulator cause established and no edited gameplay claimed. Original emulators remain untouched.

Final desktop verification on B6874528: PS3 jersey color/specular/normal maps and font maps render correctly, seven assigned kits complete (eighth selector entry is the Other asset catch-all). Full source receipt confirms complete extraction. First-pass review complete with runtime limits above.

