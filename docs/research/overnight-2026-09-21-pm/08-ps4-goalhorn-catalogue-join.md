# Handoff R50 — PS4 goal horns (Teams 2.7): `exhibitionarena.goalhorn` resolves through the shipped `goalhorns.csv` catalogue on NHL 17–21 (team rows by `audioid + 1`, generic rows by `audioid`), which names every horn; the catalogue → sound-bank binding stays open

Research lane, 21 September 2026 pm, roadmap step 2.7 PS4 (Teams audio; Q-R9-1 of the 2026-09-20 campaign "DB `goalhorn` → hornbank id
open"). Claim `research-r50-ps4-goalhorn-catalogue`. Read-only over the extracted PS4 trees (`creation_zone_csv/goalhorns.csv`, the
`audio/**/*.sbr` banks) and the anchor / install DBs; nothing game-verified; no writer.

Machine-readable: [`data/r50-ps4-goalhorn-csv-join.json`](data/r50-ps4-goalhorn-csv-join.json). Probe `r50_ps4_goalhorn_csv_join.py`
(≈ 2 min).

## Plain-language summary

The NHL 17–21 installs ship a plain text catalogue of goal horns (`goalhorns.csv`: 90 / 116 / 278 / 280 / 280 rows) with an identifier,
a name key, an art id and an `audioid`, and — from NHL 19 — the horn-builder's elements and flairs. The arena table's `goalhorn` cell
resolves into it: the 30 premade NHL club horns by `audioid + 1` (Anaheim 0 → 1 … Winnipeg 29 → 30, Vegas 30 → 31; the resolved row
names the arena's club on 30 of the 35 NHL-team arenas, the other five being the All-Star / shared-arena rows and the NY Rangers, whose
horn row is absent from the catalogue), and the default / generic horns by `audioid` directly (`41` = `AirHorn1` on 18–21, `31` = `AirHorn1`
on 17 — the value the AHL / CHL / European arenas carry). So the Team page can show a horn *name* and the horn-builder fields
(`goalhornelement1..3`, `goalhornflair`, blast / pause lengths, `goalhorniscustom`) with their catalogue labels on NHL 17–21. What is still
not found is which sound bank plays a given catalogue entry: no `.sbr` under `audio/` carries the team ids 1–31 or the generic ids 41–118
as sample ids, and `hornbank.sbr` carries only the element / flair range (500–686) plus unrelated ids — the horn *audio* for the DB value
lives somewhere this census did not read (the Harmony event system's `sfx_harmony_events.*` / `sfx_xml.big` `…crowdevents.horns.xml`
are the next files to open; the horns XML was read: the goal horn is a Harmony crowd event `H_goalHorn` creating a `GoalHornPatch` with parameters `GoalHornID` (enum) and `NumberOfLayers`, so the id → sample resolution sits in the patch data — `sfx_ptc.big` — not in a bank index). NHL 15 / 16 ship no catalogue (their `goalhorn` ids 0–136 stay as handoff 19 left them). The DB sentinel
690 (–334 signed; 11 arenas on 21, 690–693 on 19) resolves to nothing in the catalogue.

## 1. Catalogue (`nhl/data/common/gamemodes/creation_zone_csv/goalhorns.csv`)

| Title | rows | columns | classes | `audioid` range |
|---|---|---|---|---|
| NHL 17 | 90 | `index identifier artid audioid filter name` | none 1, premade team 29, generic 60 | 0–90 |
| NHL 18 | 116 | same (+ Vegas) | none 1, team 30, generic 85 | 0–125 |
| NHL 19 | 278 | `index identifier artid audioid name ispremade ishornelement isflare` | none 3, team 30, generic 85, **horn elements 63** (`HornElement_n`, `audioid` 500–686 step 3), **flairs 97** (`HornFlair_n`) | 0–686 |
| NHL 20 / 21 | 280 | + `tone` | none 5, team 30, generic 85, elements 63, flairs 97 | 0–686 |

Team rows: `identifier` = club (`AnaheimDucks` …), `artid` = the club's art id (Vegas 1000), `audioid` = team index (0–30; NY Rangers'
row is missing on every title), `name` = `$NHLTeamName_XXX`. Generic rows: `artid` 31, `name` = `$TXT_…` keys, `audioid` 31–90 (17) /
41–125 (18+). Elements / flairs: `artid` 31, `name` = `$TXT_HORNFLAIR_…`, `tone` −1 / 0 / 1 / 2.

## 2. Join to the DB (`exhibitionarena`; NHL 17 / 18 / 19 / 20 / 21 = 254 / 302 / 426 / 428 / 450 arena rows)

| Rule | NHL-club arenas whose resolved row names their club (of 32 on 17, 35 on 18–21) | rows resolving (of all arenas) |
|---|---|---|
| `goalhorn` = `audioid` | 0 | 254 / 294 / 417 / 414 / 435 (generic rows only) |
| **`goalhorn` = `audioid` + 1** (team rows) | **30 / 30 / 30 / 30 / 30** | 46 / 49 / 150 / 134 / 150 |
| `goalhorn` = `index` (+ 1) | 0 / 22 (17) … 21 | — |

Combined reading: **1 ≤ v ≤ 31 → premade club horn of team index v − 1 (catalogue row with `audioid` = v − 1); v = the default (31 on 17,
41 on 18+) and other generic values → catalogue row with `audioid` = v; v = 690 (…693) → no row (sentinel).** Unresolved under the combined
rule: the NY Rangers arena (v = 20, row absent — 1 / 1 / 4 / 3 / 3 arenas) and 690–693 (1–11 arenas). `periodhorn` is 0 on most rows (139 …
442) with the same small id space on 17 / 18 (32 / 45 / 61 / 27 … — not joined here). The NHL 19–21 horn-builder columns are constant on
the shipped rows (`goalhornelement1` 501, `goalhornflair` 0, `goalhorniscustom` 0, `goalhornblastsnum` 1) — the catalogue's element /
flair ids (500–686) are their value space.

## 3. Catalogue → bank (open)

For each title every `.sbr` under `audio/` was scanned for the catalogue's `audioid` values as `.SID`: the team ids 0–30 and generic ids
41–118 appear only as *coincidental* low ids in unrelated banks (`crowdgrainbank`, `reactionbank`, `ambiancesamplebank`, `fesfxbank` —
0–30 present because those banks number from 1); `hornbank.sbr` DSET 0 (413 records, `.SID` 56–751 on 21) holds 160 of the element /
flair ids 500–686 and 12–14 generic ids, none of the team ids. So the premade / generic horn audio is not addressed by these ids in any
SND10 bank read here; the `sfx_xml.big` member `nhlsfxevents.crowdevents.horns.xml` (12 559 B, read) defines the crowd event `H_goalHorn` → `GoalHornPatch` (player `Horns`, bindings `SfxMixer` / `Magma` / `CrowdMasterController` / `GameModesPatch`) with parameters **`GoalHornID` (Enum, CSIS parameter 0)** and **`NumberOfLayers` (Int32, parameter 1)**, and `H_periodHorn` → `PeriodlHornPatch` with `PeriodHornID`; the enum → sample resolution therefore lives in the Harmony patch data (`audio/sfx/sfx_ptc.big`, `sfx_harmony_events.{bin,csi}`), not opened tonight. Q-R9-1 stays open at that step; the runtime card of handoff 19 (2026-09-18) remains the discriminator for the audio.

## 4. Consequences (developer)

- Teams page (PS4 17–21): resolve `goalhorn` to the catalogue row and show `name` (a `$` dictionary key → the localized text) and
  `identifier`; show the sentinel 690 as "no catalogue horn (690)"; show the Rangers case as "catalogue row absent (20)". Reassignment =
  a DB cell edit validated against the catalogue's id set (team rows +1, generic rows direct) — **not implemented, not runtime-verified**.
- NHL 15 / 16: no catalogue; keep the raw id with handoff 19's Xbox-equal reading (15) and "unnamed" (16).
- Do not read the catalogue's `audioid` as an `.sbr` sample id (§3).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r50_ps4_goalhorn_csv_join.py docs/research/overnight-2026-09-21-pm/data/r50-ps4-goalhorn-csv-join.json   (≈ 2 min)
```
Executed once (final) after shell exploration of the NHL 21 CSV / DB / `hornbank.sbr` (logic stated above; the club-name match uses the
city prefix or nickname tokens, which is why the shared-arena All-Star rows and the two New York clubs score as misses).
