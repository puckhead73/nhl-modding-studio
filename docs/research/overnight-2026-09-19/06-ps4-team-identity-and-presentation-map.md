# Handoff R6-T1 — PS4 NHL 15–21 team identity, arena and kit tables: cross-title field map and asset joins

Research lane, 19 September 2026 (queue entry R6, Teams step 1 — identity and related DB entries/joins + asset
selection on the PS4 family, the family handoff 20 did not cover). Claim `research-r6-ps4-teams`. Consumer:
developer Phase 2 (2.1 identity, 2.2 links, 2.4 logos, 2.5 arena/ice, 2.6 kits, 2.7 audio ids) when it starts;
nothing here is authorised for implementation and nothing is game-verified. Reuses handoff 20 (Xbox 360 joins) as
the comparison baseline, handoff 03 (PS4 asset-path registry) as the consumer map, handoff 19 (horn ids).

Machine-readable: [`data/r6-ps4-team-field-map.json`](data/r6-ps4-team-field-map.json) — per title (anchor DBs for
15–19/21, the real CUSA15439 install member + its own `nhl_game-meta.xml` for NHL 20): `exhibitionteams`,
`exhibitionarena`, `stockteamjerseys`, `stockteamcolorstable`, `stockleagues`, `customteams`, `customarena`,
`customteamjerseys`, `stockteamhistory` — every field (short id, meta name, kind, width, offset, meta range), per-field
value statistics (distinct count, min/max, value list when ≤ 64 distinct), the registry's `#`-keyed team/arena/kit
types with the ids present, and id-set joins; [`data/r6-ps4-jersey-joins.json`](data/r6-ps4-jersey-joins.json) —
jersey-row triples vs thumbnail files per title. Probe: `ps4_team_field_map.py` (+ an ad-hoc jersey join, §4).

**Evidence vocabulary:** observed (read from the DB/meta/archives), inferred (reading of a name), unknown.
Signedness follows the metadata's declared range; three fields read negative under it where handoff 19 established
unsigned ids (`goalhorn` 690 → −334 on 19–21, `mascothead/body` −23, `ahlaffiliate` −31..−2 on 15–18) — treat those
as raw bit patterns until the developer's decoder decides (the NHL 15 offset-binary lesson of handoff 22 applies).

## Plain-language summary

On PS4 the team row grows from 55 fields (NHL 15) to 183 (NHL 21) and, from NHL 18, absorbs almost all arena
presentation (ice, centre line, puck, power ring, stanchions, mascot, goal/win/power-play songs, signature effects)
that Xbox and PS4 15–17 keep on the arena row; the arena row in turn gains the NHL 19+ custom-horn builder and, in
NHL 20, `isoutdoor`/`timeofday`. Team identity itself stays simple: `artid` selects every front-end logo family
through the registry (`teamlogos/<size>/t#`, 8–10 families on 17–21; 251/256 … 304/312 team art ids have a logo, the
misses are placeholder ids), `arenaid` points at the arena row, `arenaartid` selects the arena picture (`arenas/a#`,
30/32 stock NHL ids), and a jersey is still the `(style, teamartid, variant)` triple of the Xbox era — PS4 16–19 name
the thumbnails exactly that way (697/698 … 1 087/1 091 triples have a file), NHL 19 adds an `xl` set, and NHL 20/21
append a fourth number (0–19) that is not a `stockteamjerseys` field. Creation-Zone catalogues (seat pattern,
netting colour, scoreboard, stanchion material, centre-ice logo layout, intro/goal effects …) are `#`-keyed image
sets whose ids match the small value sets of the corresponding team/arena fields, so those fields are validatable
against shipped art. In-game textures on PS4 are named by *slug* (`pant_rbk_dallas_fourth`, `stadiums/toronto/`),
and no DB field carries that slug — the id-to-slug binding for on-ice art is unknown and is the next Teams
question.

## 1. Table sizes and field migration (observed)

| Table | 15 | 16 | 17 | 18 | 19 | 20 | 21 | Notes |
|---|---|---|---|---|---|---|---|---|
| `exhibitionteams` rows / fields | 246 / 55 | 252 / 104 | 256 / 97 | 296 / 198 | 419 / 182 | 416 / 182 | 433 / 183 | 16: + 59 presentation fields (mascot, stanchion colours/animations, FE/overlay/pres colour triples as `_r/_g/_b`); 17: colours collapse to packed 24-bit words (`feprimarycolor` …), + franchise economics, `logostyle1–3`, `cityname`; 18: + 101 (arena presentation moved here: `bouncemap`, `centerline`, `puck`, `powerring`, `centericelogo*`, `goalsongid`, `winsongid`, `powerplaysongid`, `intromusicid`, `chantaudioid`, `mascotcustom*` zones, `goalspotlight*`); 19: + `forceteamcolors`, `showmascot`, `stick`; 21: + `goalcall` |
| `exhibitionarena` rows / fields | 178 / 18 | 187 / 18 | 256 / 124 | 303 / 60 | 426 / 65 | 428 / 67 | 450 / 67 | 17 is the outlier (124: presentation on the arena row, six seat/roof/scoreboard zones); 18 moves 64 of them to the team row; 19: + 11 custom-horn fields (`goalhorniscustom`, `goalhornelement1–3`, `goalhornflair`, blasts/pauses); 20: + `isoutdoor`, `timeofday` |
| `stockteamjerseys` rows / fields | 659 / 13 | 703 / 13 | 813 / 28 | 1 030 / 29 | 1 532 / 30 | 1 652 / 31 | 1 792 / 31 | 17: + `yearstart/yearend`, `font`, `sockstyle`, `pantsstyle`, `jerseyback`, six logo-type fields, `orderid`, `active`, `variantleadingscorer`; 18 `ischampionsleague`; 19 `iscasual`; 20 `variantlegend` |
| `stockteamcolorstable` | 115 (`team, colorid, index, color_r/g/b`) | 205 | 205 (`color` packed) | 215 | 217 | 217 | 217 | 16 colour slots per team on Xbox → packed word from 17 |
| `stockleagues` | 20 (`leagueid, artid, name`) | 22 (+ `renderid`) | 24 (+ `orderid`) | 26 | 29 | 31 | 31 (+ `primarycolor`, `secondarycolor` on 21) | |
| `customteams` / `customarena` / `customteamjerseys` | — / — / 60 | — / — / 60 | 50 / 50 / 150 | 55 / 55 / 165 | 55 / 55 / 165 | 55 / 55 / 165 (+ `stockteamhistory` 162) | 55 / 55 / 165 (+ 163) | same shapes as the stock tables |

Removed with 15→16: `fullname`, `shortname` (16 keeps `shortname`, 17 drops it for `cityname`/`teamname`/`nickname`),
`jerseybackhome/away`, `jerseystyle`, the six `primary/secondarycolor_*` bytes. Exact per-title lists: JSON
`tables.*.fields`.

## 2. Identity and link fields (observed value spaces)

| Field | 15 | 17 | 21 | Join / meaning |
|---|---|---|---|---|
| `artid` (15 bits) | 213 distinct, 0–20 054 | 256, 0–30 391 | 312, 0–30 398 | team art id → all `teamlogos/*/t#` families (§3); jerseys' middle component; `stockteamjerseys.teamartid` (255/284/299 distinct) |
| `arenaid` | 178 distinct = 1:1 rows | 256 | 433, max 696 | row index into `exhibitionarena` |
| `audioid` / `audioid2` | 212 / all 0 | 247 / {0, 690?} | 293 / 293 | PBP speech ids (bank index undecoded, handoff 19); `audioid2` becomes populated from 18 |
| `league` | 15 codes 0–14 | 16 (0–14, 17) | 23 (0–15, 17–23) | `stockleagues.leagueid` (handoff 12/17) |
| `ahlaffiliate`, `rivalteam`, `rivalteam2` | 31 / 56 / – | 31 / 56 | 32 / 62 | team-row references; sign convention per meta (see header) |
| `artabbr`, `abbrname`, `cityname`/`teamname`/`nickname` (17+), `shortname`/`fullname` (15) | strings | | | display text on the row; localization keys via `artabbr` as on Xbox (handoff 23 — PS4 not re-measured here) |
| `coachheadid` | 30000–30019 | same (+0) | same | `characters/coach/coachhead_*` (slug-named on 21) |
| `helmetbrand`, `gloves`, `stick` (19+) | 6 / 8 / – | 5 / 7 | 8 / 12 / {0} | equipment brand ids (Phase 4) |
| `mascothead` / `mascotbody` (16+) | – | 27 ids 0–30 | 31 ids | `creationzone/appearance/mascotheads/#` (25 files) / `mascotbodies/#` (12): 22 / 9 of the 27–31 DB ids have a catalogue file |
| `logostyle1–3` (17+) | – | all 0 | {0, 73, 81, 243} / {0, 73, 245, 817} | **unknown** (not a registry id set) |
| `goalsongid`, `winsongid`, `powerplaysongid`, `intromusicid` (18+) | – | – | {0, 1, 72, 154 810} / {0, 1, 72, 169 403, 235 706} / {0, 1, 113 007, 164 509} / {0} | 25-bit values — large ones look like hashes/ids into the music bank; **unknown** binding (handoff 04 §3) |
| arena `arenaartid` | 32 (0–29, 50, 51) | 39 (+ 325–800) | 62 (+ 3xx, 5xx, 7xx, 9xx) | `arenas/a#` picture (§3); in-game stadium folders are slug-named |
| arena `goalhorn` / `periodhorn` | 42 / 41 ids 0–136 | 32 (0–31) / 41 | 33 (1–31, 41, 690) / {0,1,2} | handoff 19 |
| arena `arenaaudioid` / `cityaudioid` / `chantaudioid` | 158 / 156 / 61 | 155 / 155 / 62 | 157 / 158 / (team row) | speech/crowd ids |
| arena `scoreboardid` (17+) | – | {1, 2} | {1, 2, 4, 7, 8, 9, 11} | `creationzone/arena/scoreboard2/#` (12 files; 11 missing on 18–21) |
| arena `seatpatternid`, `nettingcolorid`, `architectureid`, `goallightid` (17+) | – | {0,2} / {0} / {0} / {0,1,3} | {0–4} / {0,1} / {0} / {−4,0,1,2,3} | `seatpattern/#` (6), `nettingcolor/#` (3), `basearchitecture/#` (33 → 26 ids ≥ 512 on 19+), `goal/#` (8 on 17–18, 1 on 19+) |
| team `centericelogotype` / `centericelogostyle` / `stanchionmaterialid` / `stanchionintroeffectid` (18+) | – | – | {0,1,2} / {0,1,2} / {0,1,3} / {0,1,4} | `centericelogo/#`, `centericelogolayout/#` (9), `stanchionmaterial/#` (5), `introstanchioneffecttype/#` (9) |
| team `centerline` / `bouncemap` / `puck` (18+) | – | – | 88 ids 0–146 / 31 / 32 | `creationzone/arena/centericeline/#` (105 files; 0–999 with 999 = "none" placeholder observed across catalogues); `arena_ice/#` (4 on 18, 1 on 19+) — bouncemap/puck ids no longer match a catalogue on 19+ (in-game textures are slug-named) |

## 3. Registry joins (observed; `#` types from `definition.kettle.assetpaths`, handoff 03 §1.2)

| Family | 15 (PNG) | 16 | 17 | 18 | 19 | 20 | 21 | team `artid` coverage |
|---|---|---|---|---|---|---|---|---|
| FE team logos `teamlogos/<size>/t#` | `halfsizeflat` 217, `fullsizeflat` 218, `jerseylogo` 217, `iconsizeflat` 23 | + `overlaylogo`, `overlaylogohalf` (225–226) | `img://LogoCro*` processors in front of `fullsizeflat`/`halfsizeflat`/`smallsizeflat`/`fullsizebw`/`overlaylogo(half)` (263–264) | renamed `large`/`medium`/`extrasmall`/`largemono`/`extralargemono`/`mediumglossy`/`smallglossy` (306–307) | + `extrasmallmono` (330–332) | 335–337 | + `small`, `mediummono/t_#` (348–351) | 212/213 · 216/218 · 251/256 · 281/289 · 299/307 · 296/304 · 304/312 — missing ids are `900`, `10390`, `10399`, `10497–10499`, `30220/30221`, `30391`, `30398` (placeholder/custom-team ids, inferred) |
| Arena picture `arenas/a#` (17+) | – | `cards/arenas/arena#` 31 | 31 | 44 | 44 | 48 | 52 | `arenaartid`: 30/39 … 32/62 (the 3xx/5xx/7xx/9xx generic ids have no picture) |
| Jersey thumbnails `teamjerseys/jersey_{style}_{teamartid}_{variant}` (§4) | `assets/…` (not `#`-keyed in the registry; `jersey_asset` = `teamjerseys/#`) | 1 207 files | 1 326 | 1 584 | 1 749 + `teamjerseysxl` 1 270 | 4-component (§4) | 4-component | — |
| Mascot catalogues (18+) | – | – | – | heads 25, bodies 12 (+ `creationzone_XL` 22/9 on 19+) | | | | 22/27–31 heads, 9/27–31 bodies |
| Team background `bgs/teambackground/t#` (17) / `cards/arenas/arena#` (16) | – | 31 | 31 | – | – | – | – | NHL-only (31) |

`img://LogoCro<Size>/` prefixes (17+) are image-processing directives in front of the path (crop/resize by the
UI image service — inferred from the name); the file is the path after the prefix.

## 4. Jerseys (observed)

- Rows keep the Xbox triple `(style, teamartid, variant)` plus `fevariant` (= `variant` on every stock row measured)
  and `jerseyindex`; from 17 the row also carries `yearstart/yearend`, `font`, `sockstyle`, `pantsstyle`,
  `jerseyback`, six logo-type selectors, `orderid`, `active`.
- Thumbnail grammar: `nhl/ui/kettle_assets/teamjerseys/jersey_{style}_{teamartid}_{variant}.gnf` on 16–19 (697/698,
  796/810, 988/998, 1 087/1 091 distinct row triples have a file; 510–662 shipped triples have no row — the same
  "spare kit" pool as on Xbox, handoff 20 §4); NHL 19 adds `teamjerseysxl/` (1 270 files, same triples); **NHL 20/21**
  switch both sets to `jersey_{style}_{teamartid}_{variant}_{n}.gnf` with `n` ∈ 0–19 and 2–8 files per triple
  (1 883 / 2 044 + 1 406 / 1 574 files): `n` equals **no `stockteamjerseys` field** (every candidate field scores the
  same 613/626 trivial matches); the per-triple fan-out suggests a UI-side render variant (pattern/colourway or
  size), **unknown**. Duplicate row triples appear from 19 (271–280 rows share a triple, disambiguated by
  `jerseyindex`/`orderid`/`yearstart` — inferred).
- On-ice kit textures are slug-named (`characters/player/pant_rbk_dallas_fourth_coeff.gnf`,
  `jersey_nineties_japan_away_cm.gnf`) — no numeric join from the row; the resolver that maps
  `(teamartid, variant, yearstart…)` to a slug lives outside the DB (**unknown**; candidate: the `.mcd`/material
  tables or the executable). This is the first Teams question for the next research slot.

### 4b. Where the slug binding lives (observed, late addition)

PS4 15–21 ship a **collection database** `boot.big:nhl/data/common/database/attrib/gccollections.bin` (chunkzip →
`ErtS` container; decoded 13.5 MB on 15 … 25.1 MB on 21; SHA-256 prefixes in the table below). Rev. 2 (review F4): `ErtS`
is the reversed FourCC `StrE` — the `.bin` is the **string/blob pool** of EA's Attrib/VLT vault whose record side is the
sibling `gccollections.vlt` (`Vers`/`StrN`/`DepN`/`DatN`/`ExpN`/`PtrN`/`EndC` chunks), already decoded to record level in
[docs/audits/nhl21-ps4-head-asset-binding.md](../../audits/nhl21-ps4-head-asset-binding.md) §2.1 for NHL 21; the bounded
next item is the parsed reference structure (handoff 07), not the container framing. Its string vocabulary
names asset *collections* and the texture/model paths they bundle. Some collection families are keyed by the DB ids
(`centerlogo_{artid}_texpack`: 190 / 199 / 227 / 255 / 268 / 271 / 279 ids on 15–21 — 240 of NHL 21's 312 team art ids
have one; `centerline_{id}_texpack`: 33 → 145, the `centerline` field's space; `iceline_#`, `czjersey_#`,
`head_generic_player_#`, `hair_#_*`), others by slug (`{stadium}_rink_boards`, `{stadium}_crowd.crowd`,
`mascot_body_{slug}`, `skate_nhl_{slug}_{tuck}`, `jersey_nineties_champions_{n}_{home,away}_texpack`,
`pant_rbk_{slug}_{variant}` paths). No collection is named by the jersey triple. So the team/arena row → slug
binding for on-ice kits and stadiums is **unresolved**: the string vocabulary shows collection names and asset paths
co-occurring in one pool, which makes the vault a candidate resolver, but without parsed records/references it does not
establish where the team/arena/jersey → slug binding resides (rev. 2 wording, review F4) — the parsed reference structure is
the bounded next research item (handoff 07), not a Studio task. It is
the same family as the head-id → collection question of the NHL 21 cyberface program. The Xbox 360 `attribdb`
members (`renddb.bin` `StrE`, `*.vlt` `Vers`) carry no such collection names — gen7 keys its in-game art by id in the
file names (handoff 20), so the mechanism is PS4-only.

| Title | `gccollections.bin` stored → decoded | SHA-256 (decoded, prefix) | strings | `centerlogo_#` | `centerline_#` |
|---|---|---|---|---|---|
| 15 | 1 886 927 → 13 462 336 | `0684ef97…` | 113 074 | 190 | 33 |
| 16 | 1 354 325 → 10 523 544 | `b1217f4a…` | 86 386 | 199 | 36 |
| 17 | 1 748 201 → 14 211 032 | `2da87ee8…` | 103 438 | 227 | 76 |
| 18 | 2 173 225 → 17 928 824 | `cf0cfab2…` | 131 841 | 255 | 105 |
| 19 | 2 586 394 → 21 075 832 | `aece3f7a…` | 155 821 | 268 | 120 |
| 20 | 2 936 164 → 23 839 624 | `e5266477…` | 172 904 | 271 | 136 |
| 21 | 3 091 303 → 25 113 928 | `d668bbe1…` | 181 806 | 279 | 145 |

## 5. Safe candidate operations (proposal; Phase 2 — not authorised)

1. **Identity text/links** (2.1/2.2): `cityname`/`teamname`/`nickname`/`abbrname` (17+; `shortname`/`fullname` on 15),
   `rivalteam*`, `ahlaffiliate` — plain fields; validation ranges from the JSON.
2. **`artid` re-point** (2.4): validate against the per-title logo family id sets (JSON `registry_team_types`);
   report which families hold the id (all 8–10 or the 23-member `iconsizeflat` subset).
3. **Arena presentation ids** (2.5, 17+): every `*id` field with a Creation-Zone catalogue is validatable against
   that catalogue's id set (`seatpatternid` ∈ {0..4} ⊂ 6 files, `scoreboardid` ∈ 12, `stanchionmaterialid` ∈ 5 …);
   `999` placeholders exist in the catalogues and never in stock rows. Rev. 2 (review F4): catalogue membership is a
   *validation* input only — it does not authorise editing these fields; the field → catalogue relation (which csv column
   the engine reads, whether the catalogue or the vault is the consumer) and per-field applicability still need evidence.
4. **Kits** (2.6): row triple ↔ thumbnail join exactly as Xbox on 16–19; on 20/21 the thumbnail set is a superset
   keyed by an extra component (browse-only until `n` is understood).
5. **Audio ids** (2.7): `goalhorn` per handoff 19; `goalsongid`/`winsongid`/`powerplaysongid` are 25-bit values with
   four/five stock distinct values — do not offer editing until the music-bank binding is known.

## 6. Unknowns (explicit)

1. Slug binding for on-ice/stadium assets — inside `gccollections.bin` (`ErtS`, §4b), format undecoded; 2. `logostyle1–3`, `goal/win/powerplaysongid` id spaces;
3. the NHL 20/21 fourth jersey-thumbnail component; 4. the meaning of `centericelogo` (11-bit, 18+) vs
`centericelogotype/style`; 5. PS4 team localization keys (`artabbr`-suffixed keys as on Xbox — not re-measured);
6. runtime consumption of any of it.

## 7. Reproduce, checks, cleanup

`python local/runs/overnight-research-2026-09-19/probes/ps4_team_field_map.py <out.json>` (≈ 4 min; extracts the
real NHL 20 `nhl_game-meta.xml` to lane scratch, deleted after the run). Ad-hoc python: jersey triple joins per
title, the fourth-component field test, value-set listings. **Not executed:** launches, writes. Scratch
`local/runs/…/r6/` deleted after copying the two JSON files to `data/`.
