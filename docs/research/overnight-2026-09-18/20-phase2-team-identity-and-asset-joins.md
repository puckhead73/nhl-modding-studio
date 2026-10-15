# Handoff P2-X — teams: identity fields, arena/ice/jersey/logo joins and asset name grammars (Xbox 360 family)

Research lane, overnight 2026-09-18. Claim `p2-team-asset-joins-xbox360`. Consumer: developer Phase 2 (2.1 identity,
2.2 links, 2.4 logos, 2.5 arena/ice, 2.6 kits, 2.7 audio ids). Machine-readable:
[`data/nhllegacy-team-asset-joins.json`](data/nhllegacy-team-asset-joins.json) (coverage of every join below
on Legacy Xbox 360), the per-family member CSVs `data/xbox360-assets-*.csv` (every matching member in every
archive of the ten titles: teamlogo, rendering/logo, rendering/ice, jersey, puck, board, horn, overlay, arena)
and the existing field-level schema diff (`docs/audits/data/schema-diff.csv`, not redone). Probes:
`asset_grep.py`, `team_asset_joins.py`, `schema_dump.py`.

Evidence: **observed** joins are counted from the anchor DB against the X: archive directory tables
(no member decoded except where stated); everything about which screen consumes which family is **unknown**.

## Plain-language summary

On the Xbox 360 titles the team row (`exhibitionteams`, `kOtt`) carries three foreign "art" keys that drive
almost everything visual and audible: **`artid`** (the team art id — FE logos `t{artid}.big` in five
size/context families, rink logos `rendering/logo/texlib_{artid}_{a}_{b}.rx2`, centre-ice logo
`centerlogo_{artid}_cm.rx2`, and the second component of every jersey asset), **`arenaid`** (row of
`exhibitionarena`, whose own `arenaartid`, `bouncemap`, `centerline`, `puck`, `goalhorn`, `periodhorn`,
`arenaaudioid`, `cityaudioid`, `chantaudioid` select the rink textures, puck and audio cues), and
**`audioid`/`audioid2`** (team name/PA speech). Jersey entries are rows of `stockteamjerseys` keyed
`(style, teamartid, variant)`, and that triple *is* the filename of both the FE thumbnail and the on-ice
textures — 698 of 698 stock kit rows resolve to assets on Legacy, with ~200 extra kit textures shipped but
unreferenced (candidates for "add a jersey entry" without new art). All joins are complete on Legacy except
one sentinel (`artid 255` = no art) and the AHL teams' centre-ice logos (none shipped). The naming grammars
are stable from NHL 08 to Legacy with two era breaks: ice textures switch from per-arena `ice_{NNN}_{n}_cm`
(08–10) to component textures (11+), and the jersey-entry table changes shape three times (08 / 09–10 / 11+).

## 1. Team row (`exhibitionteams`) — the keys that matter (Legacy; widths from the anchor)

| Field | Width | Meaning (observed / inferred) | Joins |
|---|---|---|---|
| `index` | 8 | position within the league — the schedule slot (P1.2-X) | schedules |
| `league` | 4 | league code (P1.1-X) | key tables, logo index |
| `leaguegroup` / `conferencegroup` / `divisiongroup` | 6/6/6 | global group ids (12–Legacy) → executable group table (P1.2-X) | — |
| `artid` | 15 | team art id | `fe/ion/artassets/teamlogos{,small,wide,calendar,dynasty,reflection}/t{artid}.big` (small uses `s{artid}`), `rendering/logo/texlib_{artid}_{a}_{b}.rx2`, `rendering/icesurface/centerlogo_{artid}_cm.rx2`, `rendering/puck/puck_{artid}_cm.rx2` (see §3), jerseys' middle component |
| `artabbr` | 96 (12 chars) | art abbreviation string | **the dictionary key suffix** for every per-team text family (`NHLTeamName_{artabbr}` …) — see P2.3-X; not observed as an asset filename key |
| `arenaid` | 8 | row index into `exhibitionarena` (187 rows; shared: arena 181 by 32 teams, 60 by 16, 59 by 14 — generic/custom venues) | arena joins below |
| `audioid` / `audioid2` | 11 / 12 | team speech ids | audio banks (not surveyed) |
| `primarycolor_*`, `secondarycolor_*` | 8×6 | RGB team colours; more in `stockteamcolorstable` (`team`, `colorid` 0..15, RGB) | UI / kit recolour |
| `jerseybackhome/away` (5), `jerseystyle` (1), `sockstyle` (3), `pantsstyle` (3), `font` (3) | | kit style selectors | jersey grammar |
| `helmetbrand`, `gloves`, `goaliemask{0..6}_{starter,backup}` (9 each) | | equipment/mask ids | Phase 4 |
| `rivalteam`, `rivalteam2` (8, −1 = none), `ahlaffiliate` (6) | | links to other team rows | 2.2 |
| `active`, `table` (1) | | flags | — |
| `shortname` (40 ch), `fullname` (50 ch), `abbrname` (12 ch) | inline strings | display names (direct DB text — but see localization: PS3/Xbox dictionaries also carry `NHLCityName_*`/`NHLTeamName_*` keys on NHL 07; consumer split untested) | 2.3 |

Earlier titles: NHL 08 `artid` is 8 bits (12–Legacy: 15); NHL 10 adds `jerseynew/old/retro/download/downloadretro`
counts on the team row; NHL 08 keeps them in `stockteamjerseytable` (per team: `jerseynew`, `jerseyold`,
`jerseydownload`, 4 bits each). Exact per-title field sets: `schema-diff.csv`.

## 2. Arena row (`exhibitionarena`) — 07 → Legacy

| Field | 10 | Legacy | Join (Legacy, observed) |
|---|---|---|---|
| `cityname` (40 ch), `arenaname` (40/45 ch) | ✓ | ✓ | display text |
| `arenaartid` | 9 | 10 | `rendering/icesurface/ice_hotspot_{arenaartid}_{n}_cm` (3 arenas only); `icesurface_{N}_bm` matches `bouncemap`, not `arenaartid` (34/52) |
| `bouncemap` | — | 10 | `rendering/icesurface/icesurface_{bouncemap}_bm.rx2` — **35/35** |
| `centerline` | — | 6 | `rendering/icesurface/ice_cline_{centerline}_cm.rx2` — **30/30** |
| `centericelogo` | — | 10 | constant on all rows; centre-ice art is keyed by the **team's** `artid` (`centerlogo_{artid}_cm`, 196/218; the 22 unmatched are AHL/other art ids with no centre logo shipped) |
| `puck` | 9 | 10 | `rendering/puck/puck_{puck}_cm.rx2` — 36/36; the same folder also holds `puck_{teamartid}` for 217 team art ids (team-branded pucks) — two id spaces share one folder |
| `powerring`, `stanchion`, `arenatype` | — / — / 1 | 10 / 1 / 1 | not joined tonight |
| `goalhorn`, `periodhorn` | 9 | 9 | audio cue ids → `audio/sfx/horns.abk` + `aemshorn.csi` (event defs; every title 08–Legacy has exactly these two members) — binding not decoded |
| `arenaaudioid`, `cityaudioid`, `chantaudioid` | 9 | 10 / 11 / 9 | speech/crowd banks (`audio/speech/*`, `audio/context/crowds_*`) — not decoded |
| `maxattendance`, `avgattendance` | 15 / 7 | 17 / 7 | numeric |

NHL 07 additionally has `stockarenatable` (96) / `customarenatable`; NHL 08–10 ice is `rendering/icesurface/ice_{arenaartid:03}_cm` (08) / `ice_{arenaartid:03}_{n}_cm` (10, `n` 0..2 — *inferred* to be `arenaartid`; not joined tonight).

## 3. Asset name grammars (from the archive directory tables; counts = Legacy)

| Family | Path grammar | Keyed by | Count (Legacy / 12 / 10 / 08) |
|---|---|---|---|
| FE team logo | `fe/ion/artassets/teamlogos/t{artid}.big` (+ `tblank`, `tcustom`); `teamlogossmall/s{artid}.big` (12 and earlier: `t{artid}`); `teamlogoswide/`, `teamlogoscalendar/`, `teamlogosdynasty/`, `teamlogosreflection/` (Legacy, 30); NHL 08 `teamlogosbroadcast/` | `artid` | 320 / 303 / 217 / 136 per family |
| Rink logo (in-game) | `rendering/logo/texlib_{artid}_{a}_{b}.rx2` — `a ∈ {0,1}`, `b` 0..≥6. **Decoded tonight** (chunklzx → RW4 texlib, see handoff 40): each file holds three DXT5 rasters `logo_{id}_{a}_{b}_{0,1,2}` (256×128, 128×64, 128×128; only raster 0 carries visible art in the specimens); `texlib_27_0_0` = current Maple Leafs crest, `texlib_27_1_6` = a **vintage** Leafs crest, `texlib_27_0_3` = fully transparent placeholder, `texlib_0_0_0` = Ducks wordmark → **inferred:** `a` = logo set (0 current, 1 vintage/alternate), `b` = variant slot within the set, empty slots shipped as blanks. Previews: `previews/rinklogos/contact-nhllegacy-x360-rinklogos.png` (user check pending) | `artid` (218/218) | 1926 / 1701 / 808 / 485 |
| Centre-ice logo | `rendering/icesurface/centerlogo_{artid}_cm.rx2` | `artid` | 231 / 213 / — / — |
| Ice components | `icesurface_{bouncemap}_bm`, `ice_cline_{centerline}_cm`, `ice_lines_{n}_cm`, `ice_lightmap_{n}_lm`, `ice_hotspot_{arenaartid}_{n}_cm` | arena fields | 37 / 22 / — / — |
| Ice (08–10) | `ice_{NNN}_cm` (08), `ice_{NNN}_{n}_cm` (10) | per arena (inferred) | — / — / 224 / 93 |
| Jersey thumbnail | `fe/ion/artassets/jerseys/jersey_{style}_{teamartid}_{variant}.big` (+ `custom`, `customhome`, `customaway`, `locked`) | `stockteamjerseys` triple (698/698) | 910 / 970 / 606 / 364 |
| Jersey on-ice | `rendering/jersey/name_{s}_{a}_{v}_cm.rx2` (nameplate/number map) + `texlib_{s}_{a}_{v}.rx2` (kit textures); geometry `jersey_{…6 ints…}.rx2` (60), `jerseytie_*`, `shirt_*`; `rendering/czjersey/` (30), `rendering/referee/` (45) | triple (698/698 both) | 1883 / 2011 / 1157 / 1304 |
| Puck | `rendering/puck/puck_{id}_cm.rx2` (+ base/shadow) | `arena.puck` **and** team `artid` | 252 / 215 / 143 / 106 |
| Boards | `rendering/board/board_shadow_{0,1}` only; rink-side ads are `rendering/adboard/` (NHL 08/10, 1 member) and `rendering/prop/` (12+, 4) — the "boards/advertising" family for 5.2 is *not* per-arena in these archives | — | 2 |
| Horns | `audio/sfx/horns.abk` + `audio/sfx/aemshorn.csi` (NHL 08: `audio/sfx/ram/horns.abk` + `events/aemshorn.csi`) | `goalhorn`/`periodhorn` ids | 2 |
| Overlays (scorebug etc.) | `fe/ion/game/overlays/overlay_{n}.big` (63) + `fe/ion/game/components/…`; no member is named "scorebug" | — | 63 / 61 / 53 / 24 |

Exceptions worth encoding: NHL 10/08 `rendering/jersey/jersey_{9 ints}` and `jerseytie_{8 ints}` (longer
geometry keys than 12+'s 6/3); NHL 08 adds `name_*_nm` normal maps; `teamlogossmall` switches from `t{N}` (≤12)
to `s{N}` (14+ — 13 not checked). Every title 08–Legacy keeps `jersey_{s}_{a}_{v}` for the FE thumbnail.

## 4. Safe candidate operations for development (Phase 2)

1. **Team identity** (2.1): `shortname/fullname/abbrname`, colours, `rivalteam*`, `ahlaffiliate` are plain
   record fields; `artid`/`arenaid`/`audioid` re-pointing is a *reference* edit that must validate against the
   asset sets above (the JSON gives the exact id sets per family) — a resolver can report "logo present in
   5/5 FE families, rink logo (0,0)+(0,1) present, centre-ice present/absent".
2. **Kits** (2.6): a jersey entry = one `stockteamjerseys` row + the three asset triples; the 200+ shipped
   but unreferenced kit triples (`asset_only` lists in the JSON, e.g. `(0,68,0)`, `(0,73,6)`) are the safe pool
   for "add an alternate" tests without new art; on 09/10 the entry count lives on the team row instead.
3. **Arena/ice** (2.5): `arenaid` reassignment + `bouncemap`/`centerline` selection are validated by the
   35/35 and 30/30 joins; centre-ice logo follows the *team*, not the arena.
4. **Rink logo suffix**: keep both `a` sets and all `b` maps per art id as one unit when replacing.

## 5. Not established

- Runtime consumer of each FE logo family; `artabbr` use; `powerring`/`stanchion`; `a`/`b` semantics.
- Audio cue → bank member binding (`horns.abk` internals; `.csi` event defs) — next research topic.
- PS3 uses the same DB and the same file names (checked for league logos only, P1.4-X); PS4 not surveyed here.
