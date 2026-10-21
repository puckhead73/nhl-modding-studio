# Logos and League Imagery

**Evidence:** measured: league-logo packaging on ten Xbox 360 sources, nine PS3 sources and seven PS4 installs; the gen7 code-to-logo tables read from each game; team-logo joins on Legacy. What each logo index depicts is inferred from decoded Legacy art. Which screen draws which size family, loader precedence and how the interface behaves when a texture's size differs from its quad are open.

## The league art id

Gen7 league logos are **not** addressed by the league code. A small table compiled into each game maps league code to a **logo index**, which is a family-wide enum; PS4 stores the same enum in the database as `stockleagues.artid`.

| Title | Code to logo index, in code order |
|---|---|
| NHL 09 | 0 1 2 3 6 7 8 4 9 5 |
| NHL 10 | 0 1 2 3 6 7 10 4 9 5 5 |
| NHL 11 | 0 1 2 3 6 7 10 4 12 13 11 5 5 14 |
| NHL 12, 13, 14, 15 (identical) | 0 1 2 3 6 7 10 4 12 13 11 14 5 5 0 14 0 0 0 0 |
| Legacy | 0 1 2 3 6 7 10 15 4 12 13 11 14 5 0 5 14 0 0 0 0 5 |

NHL 15 PS4's twenty `stockleagues` rows carry the NHL 15 Xbox 360 table byte for byte; NHL 21 extends it with 17 ECHL, 18 EBEL, 19 Allsvenskan, 20 Champions League, 21 Spengler, 22 3-on-3, 23 USA, 24 Russia and 25 NHL Alumni. `renderid` is a second, different index.

| Index | Art (inferred from Legacy) |
|---|---|
| 0 | NHL shield (also Winter Classic, US, Russia, Rest of World, HUT) |
| 1 | AHL |
| 2 | SHL / Elitserien (art changes per title, index stable) |
| 3 | Liiga |
| 4 | national teams |
| 5 | EA SPORTS tile (Custom, EASHL, Legend) |
| 6 | DEL |
| 7 | Czech Extraliga |
| 8 | Superliga (NHL 09 and 10 only) |
| 9 | Historic (NHL 09 and 10 only) |
| 10 | Swiss National League (from NHL 10) |
| 11 | WHL |
| 12 | OHL |
| 13 | QMJHL |
| 14 | CHL (also Prospects) |
| 15 | Norway flag (Legacy only) |

The `mudleaguelogos` and `onlineleaguelogos*` families are separate and must not be folded into this map.

## Package chains per platform

| Chain | Titles | Archive and package | Texture |
|---|---|---|---|
| X-A | X360 NHL 07 to 10 | BIG4 archive (`data0.big` on 07 and 08, `fe.big` on 09 and 10); entry = RefPack'd BIGF with four members | `RW4xb2` uncompressed inside |
| X-B | X360 NHL 11 | `fe.big` (EB); BIGF with six members, 64-byte aligned | RefPack → `RW4xb2` |
| X-C | X360 NHL 12 to Legacy, and the Recompilation | `cache.big` (EB); BIGF with six members | RefPack → XPR2 |
| P-A | PS3 NHL 08 | `data0.big` BIG4; BIGF with six members, raw texture | `RW4ps3` |
| P-B | PS3 NHL 09 to 11 | `fe.big` BIG4 (NHL 11 loose on disc) | RefPack → `RW4ps3` |
| P-C | PS3 NHL 12 to Legacy | loose on disc and as `cache.big` members (same bytes) | RefPack → DDS |
| S-A | PS4 NHL 15 | `inxp10.big`; `patch_inxp00.big` overrides one logo | PNG under `nhl/ui/assets/leaguelogos/size{64,256}/l{artid}.png` |
| S-B | PS4 NHL 16 to 21 | `inxp_10.big` (NHL 19: `inxp_10-010.big`) | `chunkzip` → GNF BC7, no mips, at `nhl/ui/kettle_assets/leaguelogos/size{64,256,512}/l{artid}.gnf`, with `_fr` French variants from NHL 17 |

PS4 file sizes: 4 352 bytes at 64, 65 792 at 256, 262 400 at 512. The X-C packages are identical across Xbox 360 NHL 13, 14, 15 and Legacy; NHL 12 differs (Elitserien-era art). On Xbox 360 the archive is the only copy on disc; on PS3 NHL 12 to Legacy loose and archive copies coexist and are identical; on PS4 the archive plus patch overrides are the only copies. Which copy the engine loads is open.

## Inside a gen7 package

Member `0` is the UI object (`Apt Data`, 240 to 264 bytes); member `1` an Apt constant file; `sg1` and `sg2` empty segment markers; the texture member; and a **288-byte geometry record** holding six vertices at offset 96 (stride 32) that describe the **display quad**, as big-endian floats: 64.0 for `leaguelogos`, 256.0 for `big`, 128.0 × 64.0 for `small`. Member `0` carries the same width and height twice more. The numbering of the texture and record members differs per family and is referenced from member `0`, so members are never renumbered. The record is the display size, not the texture size: NHL 07 draws a 100×100 quad from a 128-wide slot.

## Replacement

**Same size** (recommended first): re-encode with the original header so the pixel data size stays fixed, then re-wrap; only the RefPack or chunkzip length changes. Only 6 of 38 unchanged-content RefPack re-encodes fit their original slots, so the real encoded size decides whether the member stays in place or the package and archive are reflowed.

**Texture size changes, display size kept:** rewrite the texture container keeping the source's level count and leave the eight quad floats untouched. **Display size changes:** rewrite the floats. Refused: non-power-of-two sizes on gen7, format swaps between ARGB and DXT5, packages whose floats cannot be located, display-crop quads (886 of the NHL 08 to 11 Xbox 360 packages, including every calendar logo) and size-word mismatches. On PS4 a mismatch between the logo and its `size*` directory is not caught statically.

## Team logos

Team crests are keyed by the team's **art id** (`kOtt.lOnr`), not its row key: on Legacy 219 of 252 teams have an art id different from their index, and 348 distinct logo ids exist for 252 teams. Front-end crests sit at `fe/ion/artassets/teamlogos/t{artid}.big` (small crests under `teamlogossmall/s{artid}` from NHL 14), with wide, calendar, dynasty, reflection and broadcast families by title. The `rendering/logo/texlib_{artid}_{y}_{z}` family ("team logo set") holds exactly three DXT5 rasters per member (256×128, 128×64, 128×128); its tuples overlap the kit tuples without being a grid or superset, and no readable file names it, so its purpose (kit decals, boards, front end) is open. Never conflate the team art id with the player portrait id, which also uses a field named `lOnr`. See [Team Identity, Logos and Kits](Team-Identity-Logos-and-Kits).

Last reviewed: 2026-10-20
