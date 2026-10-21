# Team Identity, Logos and Kits

**Evidence:** measured on 25 main-database anchors plus the genuine NHL 20, the Xbox 360 and PS3 loose trees and archive listings (Legacy and NHL 15 in depth), and the PS4 vault joins. Nothing is game-verified.

## The team row

| Field (`exhibitionteams`) | Meaning |
|---|---|
| `index` | the global team id every roster, jersey, equipment and award row points at; league rank by `index` is the schedule slot |
| `league`, `leaguegroup`, `conferencegroup`, `divisiongroup` | see [Leagues, Competitions and Schedules](Leagues-Competitions-and-Schedules) |
| `artid` (8 bits on NHL 08, 15 from 09) | **team art id**: names the crests, the logo set, the centre-ice logo, the puck and the kit files |
| `artabbr` (12 characters) | the suffix of every per-team dictionary key; not an asset key |
| `arenaid` | the arena row (Legacy: 187 rows; one arena is shared by 32 teams) |
| `audioid`, `audioid2` | team-name speech ids |
| `shortname`, `fullname`, `abbrname` | inline database text |
| colours | primary and secondary RGB, plus colour-id tables |
| `jerseyback*`, `jerseystyle`, `sockstyle`, `pantsstyle`, `font` | kit style selectors (meanings open) |
| `helmetbrand`, `gloves`, `goaliemask0..6_{starter,backup}` | team equipment defaults |
| `rivalteam`, `rivalteam2`, `ahlaffiliate` | links to other teams |

**The art id is a shared art key, not a row identity.** Winter Classic rows share their club's art id (Legacy records 216 to 219 share 19, 21, 23 and 29), the two EASHL rows share 255, the create-a-team placeholder 20 054 is shared by 30 rows on every gen7 title from NHL 10, and the PS4 alumni, circuit and generic rows share widely (NHL 21: 433 rows, 312 distinct ids, 600 shared by 45). Other columns named "artid" (player portraits, arenas, coaches, leagues, centre-ice logos) are separate namespaces.

Renaming a team means editing the database strings *and* the free text under its dictionary keys, which are built from `artabbr`: `NHLTeamName_`, `NHLCityName_`, `NHLTeamName_Abbr3_`, `TEAMLINE1_`, `TEAMLINE2_`, `TXT_NICKNAME_`, `TXT_NICKNAME_ALT_` (NHL 14 on), `X_XLA_TEAM_` (NHL 09 on), `NICKLINE2_`, with older and national teams under numeric keys. Joining on `abbrname` resolves only 134 of 222 Legacy rows (display `ARZ` against art key `PHX`); **`artabbr` is the key**. Which screen shows which key is open. Editing names, colours, abbreviations and audio ids is dependency-free; renumbering `index` touches thousands of rows and the schedule order and is refused; re-keying an art id is an unfinished contract because every kit and logo file carries internal raster names containing its own id tuple, and whether the game reads those names is open.

## Assets keyed by the art id

| Family | File grammar | Legacy coverage |
|---|---|---|
| front-end crest | `fe/ion/artassets/teamlogos/t{artid}.big`; small `teamlogossmall/s{artid}` from NHL 14; wide, calendar, dynasty, reflection and broadcast families by title | 217 of 218 rows; 101 art ids have files but no row |
| team logo set | `rendering/logo/texlib_{artid}_{y}_{z}` | 218 of 218; purpose open |
| centre-ice logo | `rendering/icesurface/centerlogo_{artid}_cm` | 196 of 218 (keyed by team, not arena) |
| puck | `rendering/puck/puck_{id}_cm` | team art ids and arena puck ids share one folder |
| kits | below | 698 of 698 stock rows |
| PS4 | vault packs keyed by `Team` (centre logos) and `(Team, UniformVariation, JerseyFit)` (jerseys) | typed joins; not writable |

## Kits (jerseys, pants, socks)

On NHL 11 to Legacy the `stockteamjerseys` row triple `(style, teamartid, variant)` is literally the file name: `rendering/jersey/texlib_{style}_{teamartid}_{variant}`, `rendering/pant/texlib_…`, `rendering/sock/sock_{style}_{teamartid}_{variant}_cm` (NHL 13 on), the nameplate `name_{s}_{a}_{v}_cm` and the front-end thumbnail `fe/ion/artassets/jerseys/jersey_{s}_{a}_{v}.big`. `style` is 0 (nineties and CCM) or 1 (Reebok and modern), `variant` 0 to 10, `jerseyindex` a per-team ordinal, `fevariant` equal to `variant` on every row; flags `isdefault`, `hastie`, `islightcolor`, `helmetlogo`, `helmetnumbers`, `islockable`; `locstring` the kit name key. Every row resolves all three texture families on every title (NHL 11: 627 rows; 12: 676; 13: 684; 14: 704; 15: 659; Legacy: 703), and each tree carries 26 to 322 spare files per family with no row, of which most are complete candidates. Re-pointing a row is safe only to a triple whose files all exist. NHL 11 and 12 ship **two sock files per kit** (`sock_{style}_{0|1}_…`), mostly clear mirrors, sometimes identical; which leg uses which is open. Custom-team rows (art id 20 054) have no kit files, so custom kits are built another way (open). NHL 09 and 10 have no jersey table despite 480 to 544 kit textures per family; NHL 08 keeps per-team kit counts in `stockteamjerseytable`.

PS3 NHL 12 and 13 keep kits only as `chunkzip`-wrapped `.rpsgl` members, jerseys in `nocacherender.big` and pants and socks in `cacherender.big`; the name grammar transfers from Xbox 360, the bytes do not. PS4 `stockteamjerseys` grows from 13 fields (NHL 15 and 16) to 31 (NHL 21), adding years, font, sock and pants styles, six logo-type fields, ordering, Champions League, casual and legend flags.

**Raster shapes** (Legacy and NHL 15, both consoles): a jersey texlib holds six rasters, the 1 024² DXT1 colour and specular maps, a normal map, and three **font maps** (`font_{s}_{a}_{v}_cm` 512² DXT5, `_nm`, `_sm` 512² DXT1); the nameplate is one 256² ARGB8 raster; pants 512² colour and normal with a 256² specular; socks 1 024×512 DXT1; the centre-ice logo 1 024² DXT3. On PS4 each jersey pack references the same ten maps through the vault.

## Jersey fonts and glyph coverage

There is no separate font file per team: the lettering and number maps are the three `font_…` rasters inside each kit's texlib, named with the kit's own tuple, and the nameplate file carries the name and number zones. The gen7 team `font` selector (3 bits) and the PS4 per-kit `font` column (from NHL 17) exist, but what each value selects is open. The nameplate layout the Studio's editor uses is a measured glyph atlas of those rasters.

Shipped interface fonts bound what text can be drawn: Xbox 360 NHL 09 three cuts covering 322 code points, 334 from NHL 10, 336 to 339 on NHL 12 to Legacy; PS4 NHL 15 twenty fonts covering 619, NHL 16 to 21 up to 1 138. The gen7 fonts lack eight Polish and Hungarian capitals (Ą Ć Ł Ń Ő Ś Ű Ż) that the language-neutral name file of NHL 12 to Legacy uses, and the CJK characters of the language-selection words; PS4 misses only format characters. Cyrillic and Czech are covered on PS4, Cyrillic also on gen7. The console's system-font fallback is open.

Last reviewed: 2026-10-20
