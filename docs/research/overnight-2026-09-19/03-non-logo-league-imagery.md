# Handoff R3 — non-logo league imagery (P1-G): consumers, id-keyed families, aliases, previews

Research lane, 19 September 2026 (queue entry R3 of the [overnight research brief](../../plans/overnight-research-2026-09-19.md)).
Claim `research-r3-league-imagery`. Consumer: developer **P1-G** (coverage-matrix row "Non-logo league imagery
inventory"), P1-E/P1-F (same codecs and packaging as league logos: handoffs [16](../overnight-2026-09-18/16-league-logo-packaging-cross-platform.md),
[02](02-packaging-size-change-requirements.md)). Nothing is game-verified; no launch; no writes to installs.

Machine-readable: [`data/r3-league-imagery.json`](data/r3-league-imagery.json) — `gen7_families` (4 335 members of
14 Xbox 360 / PS3 art families on 19 sources: path, id, stored size, texture kind/dims/format, decoded-content SHA-256),
`ps4.types` (per title: every league/competition-shaped type of the asset-path registry with its template and the ids
actually present), `ps4.members` (3 062 PS4 members with GNF headers and decoded hashes), `consumers` (Xbox ION screens
per family; PS4 layout/lua references per type), `previews`, `name_census`. Probes: `league_imagery_names.py`,
`league_imagery_survey.py`. Contact sheets: [`previews/`](previews/) (Xbox 360 Legacy, PS3 Legacy, Xbox NHL 10).

**Evidence vocabulary:** **observed** (read from the named files, incl. UI source text), **inferred** (a visual reading
of a preview or a naming pattern), **unknown**.

## Plain-language summary

Both generations keep a *registry* that tells the front end where an image lives for a given asset type and id, and
those registries are readable: on Xbox 360 / PS3 it is ActionScript in `boot.big` (`generalhelpers.big`,
`getArtAssetPath(family, id)` → `artAssets/<family>/<prefix><id>.big`), on PS4 it is a JSON file in `boot.big`
(`nhl/ui/definition.kettle.assetpaths`, `type → path template with # = id and a French-locale tag`). Reading them
answers P1-G without guessing from file names. Beyond the four league-logo families the *league-keyed* imagery is
small: on gen7 the playoff-tree **conference/league logos** (`divisionlogos/d0–d6`: NHL West/East, a generic globe,
AHL, SHL, Liiga …), the **trophies** (`trophies/t0–t6`, `trophiesbroadcast/t0–t4`) chosen per league by the playoff
and game-settings screens, and the MUD-mode league logos; on PS4 the same league art id also drives a `size512`
"large" logo, the collection logo, the Memorial Cup (`memcup` → `l99` in NHL 21) and the Champions Hockey League tile
(`l20`), while three small id-keyed families (`league_icon_0–4`, `rostermoves/logo-1–3`, NHL 15 `scouting/leagues/league-1–11`)
are registered but no UI text references them. Two aliasing facts matter for editing: **scouting-region logos are the
league logos** (region 0 = league art 11, region 1 = 12, … on every PS4 title, as separate PNG copies on NHL 15 and as
shared archive spans on 16–21), and `l100` is a byte-copy of `l5`. Online-lobby "league_*" images exist but are
consumed only by the online lobby screen. Everything decodes with the codecs Studio already has (XPR2 / DDS / GNF; PS3
08–11 `RW4ps3` via handoff 18), and previews for tomorrow's visual check are attached.

## 0. Sources (observed)

Xbox 360: the ten X: roots (handoff 01) — `data0.big` (07/08), `fe.big` (09–11), `cache.big` (12–Legacy) for art,
`boot.big` for the registry. PS3: `data0.big` (08), `fe.big` (09/10), loose `USRDIR/fe/…` (11–Legacy). PS4: every
`databigs/*.big` of the seven installs (witnesses in `ps4-install-witnesses.json`); registries:
NHL 15 `boot.big:nhl/ui/definition.assetpaths` (81 982 B), NHL 16 `boot.big:…definition.kettle.assetpaths`, NHL 17
`inxp_10.big:…definition.kettle.assetpaths`, NHL 18–21 `boot.big:…definition.kettle.assetpaths` (SHA-256 in the JSON).

## 1. The consumer registries (observed)

### 1.1 Xbox 360 / PS3 (NHL 11–Legacy; 07–10 not located)

`boot.big` → `fe/ion/game/pods/helpers/generalhelpers.big` member `1` (ActionScript). Its `getArtAssetPath` table
lists the family names the screens pass; the path is `artAssets/<family>/…` (family folder = lower-cased name; the
file prefix is per family: `d`, `t`, `l`, `f`, `a`). League-shaped entries: `leagueLogos`, `leagueLogosSmall`,
`leagueLogosWide`, `leagueLogosBig`, `divisionLogos`, `trophies`, `trophiesBroadcast`, `awards`, `flagsWide`,
`flagsSmall`, `countryFlag`, `onlineLeagueLogos(_sm)`, `onlineLeagueTrophies(_sm)`; NHL 11 adds `nationalFlags`,
`MiniFlags`. `mudLeagueLogos` and `lobbyimg` are **not** in the table (§2.4/§2.5). NHL 07–10 `boot.big` carry no
`generalhelpers.big`; their screens were not traced (the same family folders exist, §2).

Screens calling a family (Legacy; NHL 11 equivalent, full lists in `consumers.xbox360`):

| Family | Screens (`fe/ion/game/…`) | What the screen does with it (observed strings) |
|---|---|---|
| `divisionLogos` | `screens/dynasty/dm36_playofftree`, `screens/tournament/tm26_playofftree` | dm36: `mcWesternLogo` / `mcEasternLogo` / `mcAhlLogo` next to `StanleyCupPlayoffs` / `CalderCupPlayoffs`; tm26: `setDivisionLogo` left/right/middle from `m_arrLogoIDs` (`LOGO_ID` `split` with `divisionLogos` or `leagueLogosWide`) |
| `trophies` | dm36, tm26 (`TROPHY_ID`), `screens/settings/gamesettingdata` (`EASHL_LEAGUE_ASSET_ID`, `GAME_MODES_BASE_LEAGUE`, `getTournamentLeagueId` next to `leagueLogosWide`) | the trophy shown for the active league / tournament; the league→trophy id mapping is computed in game code (**unknown**, not in the screen) |
| `trophiesBroadcast` | `overlays/overlay_2` | in-game broadcast overlay |
| `leagueLogosWide` | `nhlpausestatdisplay`, `dm21_freeagents`, `gamesettingdata`, tm26 | league id from `GetLeagueIDFromGameModeID` / `ION_League` |
| `leagueLogosBig` / `Small` | `cardsmain`, `sa1_stickeralbum`, `rfacompensationscreen`, `playerbio` / `playerinfoheader`, `ctrosters`, `dm25_tradeplayer` | `ASSET_ID` of `ION_League.GetAllAttributes` |
| `flagsSmall`/`flagsWide` | scouting, player info, pick-a-player | `PLAYER_COUNTRY_ASSET_ID` — country, not league |

### 1.2 PS4 (NHL 15–21)

`nhl/ui/definition.kettle.assetpaths` (NHL 15: `definition.assetpaths`, PNG under `nhl/ui/assets/`) is a JSON map
`{"PARTS": {type: template}, "STATIC": {…}}`; `#` is replaced by the id (`self.mAssetPathService.GetPath("league_logo",
tostring(id))` in `overlays/vvms/leagueleaders/leagueleaders.lua`; layouts use `"type": "league_logo", "id": n` or bind
the id to `LeagueArtId`). The French tag is `LANG_TAG` on NHL 17 (explicit Lua: `assetpathlocatorutil.lua` maps
`FRE_FR → "_fr"`, else `""`) and `[loc FRE_FR]` on NHL 18–21 (engine-side; same observed `_fr` file set — **inferred**
same semantics). The `#`-keyed league/competition types and the ids present per title:

| Type | Template (kettle) | 15 | 16 | 17 | 18 | 19 | 20 | 21 | UI-text consumers (files) |
|---|---|---|---|---|---|---|---|---|---|
| `league_logo` (256²) / `collection_league_logo` (same file) | `leaguelogos/size256/l#` | 14 | 15 | 17 | 20 | 24 | 24 | 25 | 17–27 layouts; `collectionservices.lua` (`LogoId = item.LeagueArtId`) |
| `small_league_logo` (64²) | `leaguelogos/size64/l#` | 14 | 15 | 17 | 20 | 22 | 22 | 23 | 2–4 |
| `large_league_logo` (512²) | `leaguelogos/size512/l#` | — | — | 17 | 20 | 22 | 22 | 22 | `startnewseason`, `selectscenario` |
| `memorial_cup_logo` | `size256/memcup` (17–20) → **`size256/l99`** (21) | — | — | 1 | 1 | 1 | 1 | 1 | none in UI text; `beaprocreationoverview.lua` uses `LeagueLogoArtId = 99` for "MemorialCup" |
| `champions_logo` | `size256/l20` (fixed) | — | — | — | — | 1 | 1 | 1 | `homepage/vvms/tournamentoverlay.lua` (CHAMPIONSLEAGUE) |
| `gmc_league_icon` | `logos/leagueicons/league_icon_#` (36×36 / 36×16 / 32×32 BC7) | 5 (0–4) | 5 | 5 | 5 | 5 | 5 | 5 | **none found** |
| `league_roster` | `logos/rostermoves/logo-#` (76×36) | 3 (1–3) | 3 | 3 | 3 | 3 | 3 | 3 | none found |
| `league_icon` (15 only) | `gmconnected/scouting/leagues/league-#` | 11 (1–11) | — | | | | | | none found |
| `roster_league_logo` (15 only) | `rosters/viewroster/league_#` | 12 (0–7, 10–13) | — | | | | | | none found |
| `region-logo-big` / `-small` (`region-logo` on 15) | `logos/regionsLogos{Big,Small}/region-#` (256² / 64²) | 11 (0–10) | 11 | 11 | 11 | 11 | 11 | 11 | `scoutingscreen/assignmentinfo.layout`, `worldmap/marker.layout` |
| `calendar_league_logo`, `playnow_league_logo` (15/16) | fixed NHL files | 1 | 1 | — | | | | | none found |
| `homepage_worldcup_logo`, `worldcup_tile_text_line` (0–2), `tile_worldcup_logo_zoomed` | fixed / `homepage/worldcup_text/t_#` | — | — | ✓ | ✓ | ✓ | ✓ | ✓ | homepage tiles |
| `finals_logo(_fr)`, `background_stanleycup`, `stanley_cup_tall_logo` | fixed | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | next-game tiles, playoff trees |
| `country_flag_small/wide` | `countryflags/flags{small,wide}/f#` | 57 | 57 | 57 | 57 | 57 | 57 | 57 | country, not league |

"none found" = no `"type": "<name>"` and no `GetPath("<name>"` in any `.layout/.lua/.dd/.vvm/.json/.menu/.nav/.lreq`
member of `boot.big`, `inxp_00.big`, `inxp_10.big` (compiled `.gfx` Scaleform movies and the executable were not
searched — a registered type can still be consumed there). Full per-title type lists (85–122 league/competition-shaped
entries incl. HUT/EASHL/awards/playoff-tree art) are in `ps4.types`.

## 2. Id-keyed gen7 families (observed; previews = inferred labels)

| Family | Titles (ids) | Texture per era | Preview (Legacy Xbox) — **inferred** reading, for the user to confirm |
|---|---|---|---|
| `divisionlogos/d#` | 07: 0–2 · 08: 0–3 · 09/11–Legacy: 0–5 · 10: 0–6 (both platforms) | 256×256 ARGB8 (`RW4xb2` 0x86 on 08–11; NHL 07 d0 496×448); XPR2 `8_8_8_8` 12–Legacy; PS3 `RW4ps3` 08–11, DDS 32-bit 12–Legacy | d0 NHL Western Conference, d1 NHL Eastern Conference, d2 globe/puck (generic international), d3 AHL, d4 blue swoosh (Elitserien/SHL), d5 SM-liiga — i.e. the **playoff-tree conference/league logos**, keyed by an index the playoff screens compute (id ↔ league mapping **unknown**; NHL 10's d6 is a seventh) |
| `trophies/t#` | 07: 0–3 · 08/09: 0–4 · 10: 0–5 · 11–Legacy: 0–6 | 376×232 → 256×512 ARGB8 (t6 128×256) | t0 Stanley Cup, t1 Calder Cup, t2 DEL trophy (tall), t3 Le Mat (SHL), t4 Kanada-malja (Liiga), t5 tall Stanley Cup, t6 Calder Cup (wide) — the per-league championship trophies |
| `trophiesbroadcast/t#` | 10: 0 · 11: 0–3 · 12–Legacy: 0–4 | 256×256 ARGB8 | t0 Stanley Cup, t1 Calder Cup, t2/t3 gold European cups, t4 silver cup — broadcast-overlay variants |
| `mudleaguelogos/l#` | 13–Legacy: 0, 5 | 256×256 | MUD-mode league logos for ids 0 and 5 only (NHL, ?) |
| `awards/a#` | 07–13: 0–20 (18–19) · 14–Legacy: 0–94 (83) | 512×256 DXT5 | individual awards (13 + 8 + 5 members share bytes with each other on 14–Legacy — placeholder duplicates) — player awards, not league presentation |
| `flagssmall/flagswide/f#` | 39 → 57 ids (0–127) | 128×96 → 128×128 | country flags (player nationality / international teams) |
| `lobbyimg/league_{nhl,ahl,cze,del,elit}` (11–Legacy; 08–10 other names) | 5 named files | 256×256 | league banners **consumed only by the online lobby** (`ion_online/screens/online/lobby/room.big` references `lobbyimg`; no offline screen does) — not offline league presentation |
| `winterclassicmode/#`, `mainmenuwallimages/*` | 12–Legacy | 512×256 DXT5 | mode/menu art, not league-keyed |

Cross-title identity (decoded pixels): `divisionlogos` d0–d5 were redrawn for NHL 08, NHL 09 and NHL 12 and are
byte-identical 12→Legacy (both platforms); `trophies` t0–t4 redrawn 08, 09, 12, identical 12→Legacy; `trophiesbroadcast`
identical 12→Legacy; `mudleaguelogos` identical 13→Legacy. NHL 10 `trophies/t5` == `trophiesbroadcast/t0`.

## 3. Aliases and shared fallbacks (observed)

- **PS4 scouting regions reuse league logos.** Decoded-content equality on NHL 15 (separate PNG files) and NHL 21
  (shared archive spans, cf. handoff 02 §1.4): `region-0` = `l11`, `region-1` = `l12`, `region-2` = `l13`,
  `region-4` = `l6`, `region-5` = `l7`, `region-6` = `l10`, `region-7` = `l3`, `region-8` = `l2` on NHL 15 (all in
  both `size256`/`region-logo-big` and `size64`/`region-logo-small`); NHL 21 keeps `r0=l11, r1=l12, r7=l3, r8=l2`
  (the others diverged). Replacing one of those league logos in place on 16–21 changes the scouting region art;
  on NHL 15 it does not (separate files) — the review must name the sibling paths (handoff 02 §4.1).
- `l100` = `l5` byte-copy on every PS4 title (100 is the "same art as 5" alias used by the DB `artid`).
- NHL 21: `l99` = `memorial_cup_logo` (the Memorial Cup entered the league art id space); NHL 19–21 `champions_logo`
  = `l20` (Champions Hockey League).
- `league_logo` and `collection_league_logo` are the same template (one file, two types).
- Gen7 within-title aliases: only the `awards` placeholders and NHL 10 `t5`/`trophiesbroadcast t0`; no league family
  shares bytes with another.

## 4. Formats, packaging chains, smallest operations

Every family above sits in exactly the chains handoff 16 measured for league logos (same folders, same package
layout, same wrappers): X-A/X-B/X-C, P-A/P-B/P-C, S-A (PNG), S-B (chunkzip → GNF BC7, no mips). Same-dimension
replacement therefore reuses P1-E/P1-F unchanged; the preview/replace operation per family is "the league-logo
operation with another family folder and prefix". Dimensions that differ from the logo families (trophies
256×512 / 128×256; awards 512×256 DXT5; PS4 icons 36×36 …) are per-member facts in the JSON — a replacement must keep
them (the 288-byte Apt record on gen7 packages carries the quad size; handoff 16 §2). PS4 `_fr` twins are separate
paths (mostly dedup'd spans) — a replace must state whether the French variant is replaced too.

**Scoped absence (rev. 2 wording, review F4):** no offline screen references `lobbyimg`; no `selection tile` or `banner`
family keyed by league id exists **in the two inspected registries** (PS4 tiles are per mode/feature, gen7 has no tile family).
This is absence in those registries only — the compiled `.gfx` movies, the executables and any consumer that builds paths
itself were not searched, so it is not a product-wide "no such asset". **Inaccessible:** NHL 07–10
screen code (no `generalhelpers.big`; the family folders exist and the NHL 10 previews are attached); PS4 `.gfx`
movies and the executable (not searched); PS3 executables. **Unsearched:** PS3 08–11 previews (`RW4ps3` decode from
handoff 18 not wired into this probe), Recompiled (Xbox Legacy bytes, handoff 01).

## 5. Developer vectors and the P1-G matrix wording

- Inventory rows: `gen7_families` (paths/ids/dims/hashes per title) and `ps4.types` + `ps4.members` are the fixture
  for a "league imagery" browser; previews: `previews/contact-*.png` (17 sheets).
- Preview/replace cells: reuse the league-logo cells per chain; the only new parameters are family folder, prefix and
  expected dims/format (from the JSON). Suggested matrix wording for P1-G: "inventory delivered (research 03);
  preview/replace = league-logo operation parameterised by family; league-keyed families: `divisionlogos`,
  `trophies`, `trophiesbroadcast`, `mudleaguelogos` (gen7); `large_league_logo`, `memcup`/`l99`, `l20`, region logos
  (PS4); tiles/banners: no league-keyed entry in the inspected registries (`.gfx`, executables and other consumers
  unsearched — not a product-wide n/a)" (rev. 2 wording, review F4).
- Unknowns to carry: the gen7 league → `divisionlogos`/`trophies` id mapping (in game code); PS4 `league_icon_0–4`,
  `logo-1–3` consumers; whether `[loc FRE_FR]` falls back to the English file when a `_fr` twin is missing (every
  present id has one, so stock never exercises the fallback).

## 6. Cards (user, later; no review requested now)

- **R3-IMG-1 (NHL Legacy Xbox 360):** replace `divisionlogos/d0.big` (same dims) via the league-logo route; open the
  Season playoff tree — the Western Conference logo changes → confirms d0 ↔ West on this title; unchanged → the tree
  reads another family or the `cache:`/loose precedence question (2026-09-18 atlas Q1).
- **R3-IMG-2 (NHL 21 PS4) — rev. 2 wording (review F3):** replace `l11.gnf` with a private-copy append (handoff 02
  V-EB-SHARED); check the league logo *and* the scouting region-0 logo. Only the league logo changes → the retargeted row
  was consumed for that screen and the region screen drew other bytes (consistent with per-row retargeting on this
  title/screen pair; not a general loader rule). Both change → **not identified by this outcome alone**: the edit may have
  been placed in the shared slot, the region screen may alias the league path in its own mapping, or another consumer
  may resolve both through one block — the next discriminating control is the same edit with the region-0 row verified
  untouched in the written archive (offset/size/hash) and, separately, a region-only edit. Neither changes → the
  private copy was not consumed on this screen; cause not identified (placement, caching, another copy).

## 7. Reproduce, checks, cleanup

```
python local/runs/overnight-research-2026-09-19/probes/league_imagery_names.py <names.json>
python local/runs/overnight-research-2026-09-19/probes/league_imagery_survey.py <out.json> docs/research/overnight-2026-09-19/previews [--only families|registry|consumers|previews]
```
Executed (read-only): the name census over every archive of 26 sources; the survey (families 4 335 members, PS4
registry parse ×7 + 3 062 member headers, consumer grep of Xbox Legacy/NHL 11 ION packages and PS4 UI text of seven
titles, 17 contact sheets); ad-hoc greps for registry consumers. Not executed: any launch, any write, BC7 decode
(no PS4 previews), PS3 `RW4ps3` previews. Scratch under `local/runs/overnight-research-2026-09-19/r3-imagery/`
(survey parts, extracted registries) deleted after merging into `data/`.
