# NHL09 QA — first pass reviewed, acceptance partial

All twelve requirements have been reviewed on both platforms. Source loading, serialized edits and native texture checks pass with the limitations below. Xbox profile setup and PS3 menu input still block edited-game acceptance. Later continuation notes supersede the earlier pending states retained below.

Xbox360 first. Original ISO is read-only; all outputs are under target/overnight-20261003/nhl09.

ISO database staging/reopen passes with 3,968 players and 188 teams, including unchanged source database bytes. Full disc extraction/source routes remain running; 44 files / 5,392,474,216 bytes copied so far.

Copied anchor edit-roundtrip-1 passes: team name; independently selected arena name/city; release/sign/trade; unique first-line centre; scratches and jersey number; player name and aggression; stick and tape color; career insertion; localization; save/reopen/CRC. No gameplay verification. Asset replacements, desktop, remaining requirement limits and PS3 are pending.

## Xbox360 source and texture results

The initial source-route test completed copying/extraction but failed opening the shipped metadata: NHL09's sibling XML is RefPack-compressed. Project loading now decompresses recognized RefPack metadata in memory, preserving the file. Synthetic compressed metadata and malformed-input regression passes; the existing real dump then opens with the shipped schema, 3,968 players / 188 teams, all ten archives complete and zero remaining members. This supersedes the earlier pending extraction; the initial full test itself remains a recorded failure, followed by successful targeted verification.

Asset discovery passes (asset-observations-1). Six copied texture replacements pass (texture-roundtrip-2): team logo, league logo, player portrait, jersey color slot 2, stick, ice_000_cm. NHL09 uses rendering/icesurface/ice_000_cm.rx2, unlike NHL08's generic ice library; texture-roundtrip-1 stopped on that missing NHL08 path and is superseded. Changed decoded pixels, exact backups and unchanged input files checked. Jersey rewrites base plus three large mips but retains seven packed-tail levels; stick and ice rewrite base only. These are partial image replacements with explicit writer notes.

No stockteamjerseys table exists; texture editing is offered but new kit assignment is not established. Arena model files resolve through arenaartid, but team/home-arena link is absent and centre-ice binding remains unresolved. Raw equipment edits persist and catalogue files resolve; complete component-set contracts remain unmeasured. Desktop rebuild with the compressed-schema fix is underway; runtime still unverified.

Desktop import passed in C31F5018... with compressed sibling schema, 76 tables and 9,448 strings. All seven stock validation errors concern Centennial Canadiens extra captains; the audit now explains exact pre-existing findings using loaded/last-saved baseline comparison, retaining severity. Regression confirms all seven remain visible and a deliberate in-memory captain repair clears them. New wording is not yet in this desktop executable.

## Xbox360 checklist status

| # | Requirement | Evidence / remaining limit |
|---|---|---|
| 1 | ISO load | Direct ISO database staging/edit/reopen passes, original bytes preserved. |
| 2 | Dump extraction | 44 disc files / 5,392,474,216 bytes copied; ten archives complete. |
| 3 | Relocated files | Real relocated dump opens with shipped compressed metadata after fix; desktop project created. |
| 4 | League name/icon | NHL→QHL localization persists; league icon replacement passes. Runtime binding pending. |
| 5 | Team biography | Team name and independently selected venue name/city persist; team-arena link unresolved. |
| 6 | Team logo | Copied native replacement/backup/decode passes. |
| 7 | Center ice | Shipped ice_000_cm texture replacement passes; team-specific binding unresolved. |
| 8 | Roster/lines/scratches | Transactions, unique first-line centre, scratches and jersey state persist. |
| 9 | Uniforms/colors | 42 Anaheim textures, five desktop kits; jersey and tape-color edits persist. Four preview pieces, 8,038 triangles/eight maps; gloves untextured. Xbox small-mip limitations disclosed. New kit selection unmeasured. |
| 10 | Player data/portrait | Name, rating, career insertion and copied portrait replacement pass. |
| 11 | Equipment | Raw stick/tape changes persist, catalogue files resolve; full component sets unmeasured. |
| 12 | Localization | 9,448 strings imported, saved retext verified; one pending edit at a time. |

Runtime active in isolated portable Xenia/StudioQA using copied default.xex. Only generated copied DB/dictionary replaced with edit-roundtrip-1 outputs; .before-runtime-qa backups retained. Reached title screen, no edited gameplay accepted. PS3 not started.

Xbox360 first pass reviewed: all twelve items above have evidence or explicit limitations. Xenia advanced to the NHL09 local-profile prompt after focusing its game surface, but repeated observed A input did not advance setup; no compatible roster save or edited gameplay captured. Log saved to nhl09/xenia-profile-attempt.log. Runtime input/profile progression remains a blocker, rather than treating the title screen as proof. Proceed to NHL09 PS3.

PS3 source pass active: 53 files / 5,310,483,763 bytes copied through job API, database readable; archive extraction pending. PS3 copied database edit-roundtrip-1 passes all existing name/venue/roster/line/player/gear/career/localization save-reopen assertions. No PS3 runtime evidence yet.

## PS3 checklist and first-pass completion

Real ISO job/extraction/load PASS (174.72 s): 53 files / 5,310,483,763 bytes, 21,315 extracted assets / 3,077,323,841 bytes, ten archives complete, 3,968 players / 188 teams. Source DB bytes preserved. ps3-edit-roundtrip-1, ps3-asset-observations-1 and ps3-texture-roundtrip-1 all pass.

| # | Requirement | Evidence / remaining limit |
|---|---|---|
| 1 | ISO load | Decrypted ISO copied through the job API, database loaded; encrypted images unsupported. |
| 2 | Dump extraction | Copied disc's ten archives extracted completely. |
| 3 | Relocated files | Arbitrarily relocated PS3_GAME/USRDIR detected; desktop project imported compressed schema, 76 tables / 9,434 strings. |
| 4 | League name/icon | Saved QHL localization and native logo replacement reopen correctly; runtime binding unverified. |
| 5 | Team biography | Team name and independently selected venue name/city persist; home-arena association unresolved. |
| 6 | Team logo | Native replacement, backup and changed decoded image verified. |
| 7 | Center ice | ice_000_cm.rpsgl replacement passes; team-specific center-logo binding unresolved. |
| 8 | Roster/lines/scratches | Transactions, unique first-line centre, scratch and jersey state persist. |
| 9 | Uniforms/colors | 55 Anaheim textures, five desktop kits, jersey color slot 3 and equipment tape color edits pass. Native replacements report no partial-mip warning. PS3 mesh preview unsupported; kit assignment unmeasured. |
| 10 | Player data/portrait | Name, rating, career insertion and portrait replacement pass save/reopen. |
| 11 | Equipment | Raw stick/tape fields and native stick texture replacement pass; complete component contracts unmeasured. |
| 12 | Localization | 9,434 strings loaded; saved edits reopen. One pending operation at a time remains a limitation. |

Desktop build C159752AEFCFEBA34DB49E4A3E6793CA71A35CA063FB591489D08A10E69E95D9 verified five kits and pre-existing captain warnings correctly labelled. Follow-up source change explains unsupported PS3 3D preview instead of implying absent kit files; TypeScript passes, next build required to see this wording. Native PS3 bulk outfit export was fixed and validated against six NHL08 rasters (byte-identical PNG encoding to preview); seven outfit regressions pass.

Isolated RPCS3 runs the unedited NHL09 baseline through title and rink/attract rendering without NHL08's crash warning. Explicit Keyboard defaults saved in the isolated emulator only. Start/Cross inputs did not produce an observed usable menu; game stopped through its Exit Game confirmation. This is baseline boot/rendering evidence only: no edited gameplay, no compatible save capture. Original emulator, ISO and installed files unchanged. All twelve requirements reviewed; remaining runtime and binding limitations carry forward to follow-up. Continue NHL10 Xbox360.

Joliet follow-up: full ISO manifest's 53 filenames/lengths match the independent existing dump. Earlier primary-name copy lacked ESRB-E-nV.vp6 under its correct name; copied only that missing file into generated source-routes-1 from the read-only independent dump, SHA256 checked. Old alias retained. Runtime needs retesting after this correction; no edited gameplay acceptance.

Corrected NHL09 baseline retest reaches stable windowed title (~60FPS). Repeated Start attempts at the visible prompt did not enter an observed menu. Filename regression corrected, menu/input remains the blocker; no edited gameplay. Stop requested, then NHL10 PS3 baseline.
