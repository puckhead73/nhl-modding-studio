# Arenas, Boards and Venues

**Evidence:** measured: arena tables on all 27 anchors; venue packages on the Xbox 360 NHL 09 to Legacy and PS3 NHL 12 to Legacy trees and archives; PS4 vault level records and venue folders on NHL 15 to 21. The PS3 board-art writer contract is accepted for implementation on disposable copies only. Nothing is in-game.

## The arena table

Core fields from NHL 08 to PS4 NHL 21: `arenaname`, `cityname`, `arenaartid`, `arenatype` (0 NHL ice, 1 international), `arenaaudioid`, `cityaudioid`, `goalhorn`, `periodhorn`, `maxattendance` (2 000 to 65 000; 65 000 marks the Legacy outdoor rows). Gen7 adds `avgattendance`, `chantaudioid` and `puck` (NHL 08 to 17), `centerline` (NHL 11 to 17 and Legacy), `bouncemap`, `powerring` and `centericelogo` (NHL 13 to 17 and Legacy) and `stanchion` (NHL 13 to 21). NHL 08 carries a one-off rink-dressing set (ice logo, ice ads, board ad and trim, door logo, jumbotron, roof, walls, seat tiers, crowd type, ice quality); NHL 17 a 57-field signature set. PS4 NHL 17 to 21 add 24-bit colour zones for seats, walls, tunnel, roof, railings, stairs and scoreboard (non-zero only on created arenas), scoreboard, seat-pattern, goal-light and netting ids, from NHL 19 the horn-builder fields, and on NHL 21 `isoutdoor` and `timeofday`. NHL 07 is a thin selector into a 96-row stock table or a custom table; its link to a stadium package is open.

The audio id columns coincide numerically with horn and chant bank ids on many rows but match no bank on 104 to 127 distinct values; **no binding is established** for `cityaudioid` and `arenaaudioid`. `goalhorn` and `periodhorn` map exactly to the numbered horn banks ([Goal Horns, Chants and Music](Goal-Horns-Chants-and-Music)).

## Gen7 venue assets

`arenaartid` **is the venue file name**: `rendering/stadium/stadium_{arenaartid}_container_{n}` on NHL 09 to Legacy and `rendering/jumbotron/texlib_{arenaartid}` from NHL 12. Every arena row resolves a stadium package on every title (NHL 09: 188 rows; Legacy: 187), and each tree ships 4 to 14 spare venues no row names. Named ice bases (`ice_nhl`, `ice_intl`, `ice_nhl_full_lm`, earlier `ice_ahl` and `ice_practice`) are selected by something other than an id (open); NHL 08 to 10 ice was per arena. Centre-ice logos and rafter banners are keyed by **team** art id (191 and 210 of 218 on Legacy) and are shown through the home team; NHL 09 and 11 have no home-arena link on the team row. `bouncemap` → `icesurface_{id}_bm` resolves on every row and is a strong candidate; `centerline` and the ice line and lightmap files are not established.

Legacy examples from the venue census: `stadium_0` Honda Center, `stadium_2` TD Garden, `stadium_19` Madison Square Garden, `stadium_27` Air Canada Centre, the outdoor `stadium_700` to `703`, Swiss rinks in the 320s; shared generic venues `stadium_145` and `155` (AHL), `494` (junior), `498` and `507` (European), the EA SPORTS Arena variants and the EASHL arena `510`.

## PS4 venues

The chain is `arenaartid` → vault `ge_level.ID` → the record's debug name → the folder `rendering/stadiums/<name>/` ([PS4 Attrib/VLT Vault](PS4-Attrib-VLT-Vault)). It is a name match with provenance, not a documented selector. Each title has 80 arena rows; 79 are unambiguous on NHL 15 and 17 to 21, 77 on NHL 16, where Honda Center's id 0 maps to several levels and the Barclays and Rogers ids map to two folders each. A level record binds the centre logo, centre line and ice line packs, `RinkId` (19 to 21), `ArenaID` (from 17, which differs from `ID` on non-NHL venues and must not be used as a key), crowd placement and on NHL 21 cut-scene override groups; re-pointing an arena's art id would swap all of them, so Studio lists venues read-only. NHL 15 to 19 ship city-intro clips (`scenic_XX_YY.mkv`, 83 to 88 per title, VP8 1 600×900 with overlay sidecars) whose `XX` is not a verified city key; NHL 20 and 21 keep only two overlay records.

## Board ads

**Gen7:** board-ad pictures are rasters inside each venue package (Xbox 360 NHL 10: 44 packages and 105 rasters; NHL 14 to Legacy and PS3 14 to Legacy: 64 packages, 139 rasters, 88 shared by several materials). Xbox 360 board rasters are DXT1 plus one 512×128 DXT3 per title from NHL 13, at 256 to 1 024 pixels; the PS3 rasters are plain DXT1, DXT3 or DXT5 whose base bytes equal the Xbox 360 base after untiling and byte swap. No checksum covers the package (a bounded negative). Each venue package exists as a loose plus archive pair on Xbox 360 NHL 11 to Legacy and PS3 14 to Legacy, archive-only on PS3 11 to 13 and the Recompilation, and unmeasured on Xbox 360 NHL 10.

The accepted PS3 replacement contract: same size and format, DXT1 or DXT3 only, rewrite the full mip chain of that one data entry and nothing else, refuse an entry that overlaps another or is owned by several descriptors, list every material sharing the raster, update both copies of a loose-plus-archive pair together or refuse, and re-pack the archive because the compressed member length changes. Xbox 360 stays gated on the packed mip tail and a re-wrap qualification. Copy precedence and in-game appearance are open.

**PS4:** board ads are not in the board mesh art. Each rink's `_rink_boards` mesh has 38 to 42 ad materials bound to blank base textures plus a transparent decal slot; sponsor art lives in `common/boards_ads_decal_<sponsor>_diff` textures (35 on NHL 15, 107 on NHL 21), and each boards scene object holds a decal array (length = panel maximum + 1) with override groups keyed by game mode and, on NHL 21, board variation. "Entry *i* = panel *i*" is a candidate; on NHL 21 one array record is shared by 37 rinks, so editing it affects all of them. The `adboard_*` textures are building signage, not boards.

## What Studio offers

The arena page edits identity, presentation and audio fields, lists venue parameters by class, shows the joined files read-only, and on PS3 replaces a board raster as a pending change ([Arenas](Arenas)). Creating or duplicating arenas, and re-pointing PS4 venues, are not offered.

Last reviewed: 2026-10-20
