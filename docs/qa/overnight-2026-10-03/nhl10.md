# NHL10 QA — first pass reviewed, acceptance partial

All twelve requirements have been reviewed on both platforms. Source loading, serialized edits and native texture checks pass with the limitations below. Xbox baseline crashes and PS3 menu input still block edited-game acceptance. Later continuation notes supersede earlier pending states retained below.

Xbox360 source pass completed (203.47s): read-only ISO X:/Games/Emulation/Roms/XBOX360/NHL 10/NHL 10 (USA).iso; 69 files / 5,273,702,346 bytes copied to source-routes-1/relocated disc dump; 22,095 extracted assets / 2,924,949,102 bytes. All 14 archives complete, zero remaining. Direct ISO staging/edit/reopen and relocated folder loading pass with 3,933 players / 182 teams; original ISO DB preserved.

Database edit-roundtrip-2 passes (9.40s): team name, independently selected venue name/city, release/sign/trade, unique first-line centre, scratches/jersey, player name/rating/career, stick ID and tape color, sequential localization save/reopen/CRC. Stick now uses an actual shipped catalogue ID (29→18), rather than the earlier raw-ID-only test. Home-arena binding is absent; team identity exposes fullname/shortname/abbreviation but no mapped separate city or RGB team-color fields. Do not claim those unmapped fields edited.

Asset observations pass: 42 Anaheim uniform textures; no stockteamjerseys selector table, so new kit assignment remains unmeasured. Six texture roundtrips pass (texture-roundtrip-2, 10.84s), including actual jersey color slot 2, stick texlib_29 and ice_000_0_cm. Earlier texture-roundtrip-1 passed four files then failed on the NHL09-only stick filename; it is superseded. Backup/input preservation and changed decode checked. Xbox partial-mip notes remain applicable. Complete equipment component contracts and center-ice team binding remain unmeasured.

Desktop/runtime and PS3 checks pending. No edited gameplay accepted. Bundled desktop build 046611EF0F681F6139563BB1F054BE6A2611BF3F4F3CE410B7EE50FC5BE2BB51 is running. A prior development-mode build showed localhost connection failure because it lacked tauri/custom-protocol; rebuilt with that feature, bundled welcome screen now confirmed. This was a QA build invocation issue.

Desktop Xbox check: 86 tables/12,505 strings, 182 teams, five Anaheim kits, 42 discovered textures. Preview has five pieces / 9,666 triangles / nine maps: jersey/head/pants/helmet textured; gloves untextured. Seven stock audit errors are multiple captains. Source assets are linked to the generated copy; all save/deployment destinations blank. For runtime probing only, copied DB/dictionary replaced with edit-roundtrip-2 outputs and .before-runtime-qa backups retained. Source-route receipt predates this generated-target mutation; original ISO/dump unchanged. Packed archive priority is not yet verified.

## Xbox360 checklist

| # | Requirement | Evidence / remaining limit |
|---|---|---|
| 1 | ISO extraction/load | Direct ISO database stage/edit/reopen passes; source preserved. |
| 2 | Dump extraction | 69 copied disc files, 22,095 extracted assets, 14 complete archives. |
| 3 | Location independence | Relocated dump loaded with shipped metadata; desktop project imported. |
| 4 | League name/icon | QHL text save/reopen and native logo replacement pass; runtime binding unverified. |
| 5 | Team biography | Name and separate venue name/city persist. Separate team city/RGB fields unmapped; home-arena link unresolved. |
| 6 | Team logo | Native replacement and backup/changed decode pass. |
| 7 | Center ice | ice_000_0_cm native replacement passes; team-specific center-logo association unmeasured. |
| 8 | Roster/lines/scratches | Transactions, line uniqueness, scratch and jersey states persist. |
| 9 | Jersey/equipment colors | Five desktop kits, 42 textures; jersey and tape color roundtrips pass. Xbox packed small mips retained; gloves preview untextured; kit selection unmeasured. |
| 10 | Player biography/attributes/portrait | Name, rating, career and portrait roundtrips pass. |
| 11 | Equipment | Shipped stick IDs 29/18 and texture replacement tested; complete component sets unmeasured. |
| 12 | Localization | 12,505 strings imported; sequential edits saved/reopened; multiple pending edits unsupported. |

Runtime probe started in isolated portable Xenia with the copied executable. No edited gameplay accepted; PS3 begins after this runtime check is recorded.

Xbox first pass reviewed: edited loose-file probe and restored baseline both crash before the title screen in isolated Xenia, at PC 0x82A0C1F0. Logs xenia-edited-crash.log and xenia-baseline-crash.log retained; baseline DB hash equals its pre-runtime backup. No evidence establishes an edit-induced crash. Copied runtime DB/dictionary now baseline; edited outputs retained with .edited-runtime-qa suffix. All twelve items reviewed with explicit limits; continue NHL10 PS3. Original games/emulators untouched.

Joliet correction verified against real NHL10 ISO: ps3-source-routes-2 PASS176.61s, 83 disc files/5,480,453,074 bytes, 22,125 assets/2,697,223,519 bytes, 14 archives complete, 3,933 players/182 teams. All 83 ISO filenames/lengths also match independent existing dump. PS3 edit-roundtrip-1 PASS11.50s; PS3 asset observations and six texture roundtrips PASS11.38s combined. Full PS3 checklist/Desktop/runtime still pending while checking extraction-fix regressions in NHL08/09.

PS3 ISO/Joliet reference: https://www.ecma-international.org/wp-content/uploads/ECMA-119_4th_edition_june_2019.pdf (7.5 and Annex C). Full filename selection is required: primary aliases can replace punctuation, truncate extensions and hide sibling metadata. The fix is present in bundled E983219F0804075527E5E8EF03D13A3135A9CAB89B510DEEF829B1A7469094D4.

## PS3 checklist

| # | Requirement | Evidence / remaining limit |
|---|---|---|
| 1 | ISO extraction/load | Joliet-aware job API pass; all 83 full filenames and lengths match independent dump. |
| 2 | Dump extraction | 22,125 assets extracted from 14 supported archives; complete status. |
| 3 | Location independence | Relocated disc root detected and imported through desktop with sibling schema. |
| 4 | League name/icon | QHL retext and native logo replacement pass; runtime binding unverified. |
| 5 | Team biography | Name and separate venue name/city persist; home-arena link and separate city/RGB fields unmapped. |
| 6 | Team logo | Native replacement, backup and changed pixels verified. |
| 7 | Center ice | ice_000_0_cm.rpsgl replacement passes; team-specific logo binding unmeasured. |
| 8 | Roster/lines/scratches | Transactions, unique line slot, scratch/jersey save-reopen pass. |
| 9 | Jersey/equipment colors | 55 Anaheim textures; jersey slot3 and tape color edits persist. Native writer reports no partial-mip note. 3D preview unsupported; kit selection unmeasured. |
| 10 | Player data/portrait | Name, rating, career and portrait roundtrips pass. |
| 11 | Equipment | Raw stick/tape and native stick replacement pass; component-set binding unmeasured. |
| 12 | Localization | 12,559 strings imported; sequential edits reopen; one pending operation limitation remains. |

Desktop import created a project with no installation/save deployment destination. Remaining checks: inspect uniform controls and isolated baseline runtime. No gameplay or archive-priority acceptance claimed.

Desktop follow-up: bundled D412D91F62CDE6754CADAC14FC509940D9A0BF7EA1C3704A67D79209CC3EC1CC visually verifies a compact readable PS3 unsupported-mesh notice. Five Anaheim kits and jersey color/specular/normal previews are present. Frontend and bundled desktop builds pass. Isolated baseline reaches NHL10 title/intro at about60FPS; input acceptance under investigation, no edited gameplay.

First pass complete with limits: all twelve items reviewed on both platforms. PS3 baseline renders title and intro at ~60FPS, but no usable menu was verified using Return, Cross or a UI-confirmed Start=P mapping. Isolated RPCS3 global keyboard Start is now P. No edited PS3 runtime probe or gameplay acceptance. Xbox baseline and edited copies both crash before title. Remaining work includes runtime input/compatible saves, PS3 mesh decoding, Xbox small mips, measured kit/arena-art assignments and localization edit queuing. Continue NHL11 Xbox360.
