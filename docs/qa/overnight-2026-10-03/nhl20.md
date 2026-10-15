# NHL20 QA — first pass complete with blockers

The supplied PS4 base package is 30,665,408,512 bytes. The previously qualified standalone extractor completed 58/58 entries into F:/TEMP/nhl-studio-overnight-20261003/nhl20/pkg-source-routes-1/CUSA15439. All thirteen extracted BIG archives match the existing installation by whole-file SHA-256. No extraction errors were logged. Studio's production archive extraction completed on that package output in 1,322.52 seconds: 13 archives, 170,756 files, zero failures, blocked members or remaining members. The freshly extracted DB/dictionary opens 7,576 players and 416 teams. This establishes a standalone package preparation route; Studio PKG onboarding is still missing.

The committed NHL20 anchors are the previously documented NHL19 aliases and are already excluded from bundled baselines. QA uses the genuine installed NHL20 database (SHA-256 1e200478…), metadata and dictionary read-only. It opens 7,576 players and 416 teams. No vanilla roster save is supplied: the provided career/profile/season saves are not substituted for a roster container. Originals and bundled provenance policy remain unchanged.

| Requirement | PS4 finding |
|---|---|
| 1. ISO extraction/load | No ISO supplied. Standalone extraction of the supplied PKG passes; Studio PKG onboarding remains unfinished. |
| 2. Dumped-disc extraction | PASS production extraction on freshly unpacked PKG files, with complete state and no unresolved members. |
| 3. Arbitrary existing files | Existing installation inventory and fresh imported project pass. Copied genuine DB/dictionary save/reopen pass. Relocated inventory passes in 33.47 seconds: direct/parent discovery agrees, real assets resolve, and the package-extracted DB is byte-identical to the independently installed DB. |
| 4. League name/icon | League localization persistence and native league-logo replacement pass. |
| 5. Team biography | Abbreviation, nickname, localized full name/city, arena association, arena name/city and frontend packed color save/reopen pass in the main DB. |
| 6. Team logo | Copied large/t0.gnf replacement, decode, dimensions and backup checks pass. |
| 7. Center ice | Native ice_centerlogo_0_cm.gnf replacement passes. Vault venue binding/full-rink reassignment remain unverified. |
| 8. Roster/lines/scratches | Main DB release/sign, trade, line ownership, scratch/jersey save/reopen pass. No compatible vanilla roster save supplied, so no roster-container verification. |
| 9. Jersey/equipment colors | Native jersey replacement and fifteen packed equipment color zones persist. Ambiguous current/alumni names initially hid all Anaheim outfits; explicit candidate browsing now exposes 130 files without assigning kit ownership. |
| 10. Player biography/attributes/portrait | Main DB biography, rating and career-season edits persist; copied native portrait replacement passes. |
| 11. Player equipment | Stick and packed tape fields persist; copied native stick-tape texture replacement passes. Actual rendering unverified. |
| 12. Localization | League/city/full-name and sampled text save/reopen pass using the genuine installed dictionary. |

Main-only editing passes in 54.60 seconds. Six native GNF replacement roundtrips pass in 10.41 seconds. Imported project creation passes in 2.84 seconds with blank deployment destinations. Copied inxp_00.big database edit, CRC/fresh decode and byte-exact restore pass in 362.95 seconds. Evidence is under target/overnight-20261003/nhl20.

Uniform finding: current Anaheim (0) and Ducks Alumni (285) share the city/name prefix and ANA abbreviation. The conservative automatic resolver intentionally drops ambiguous keys. The new opt-in filename-candidate view labels those matches as unverified and explains that a texture may be shared; the export resolver still refuses ambiguous ownership. A synthetic test additionally caught a prefix bug where an explicitly named alumni texture matched the shorter current-team slug. Matching now includes the outfit-piece delimiter to keep those teams separate. Eight focused frontend tests pass; all 41 PS4 domain tests pass, including both ambiguous-name and exact-team separation checks. The real API candidate check returned 130 files; frontend build index-u26U6oMD passes.

Desktop Studio and emulator checks remain blocked by the Windows helper. No NHL20 edited gameplay is claimed. GitHub issue 1 and Discord texture/onboarding feedback are addressed only to the scope of these fixes and backend/component evidence.


Additional package checks: all 13 game archives and 32 of 33 sampled root/sce_sys/sce_module files match the installed copy, including eboot.bin and param.sfo. As with NHL18, the 532-byte sce_sys/npbind.dat differs; cause and runtime impact remain unverified. This comparison is not a runtime-equivalence claim.



All twelve requirements have been reviewed. The complete archive extraction wrote 87,211,937,771 bytes. Studio PKG onboarding, compatible roster-save input and edited-gameplay verification remain unresolved; this first pass is not full runtime acceptance.
