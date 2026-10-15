# Handoff P1-B-SETTINGS — the mode-setup settings catalogue: NHL 15 PS4 layout, contexts, defaults, cross-title equivalents and a bounded defaults-editing contract

Research lane, 18 September 2026, fresh session under
[p1b-r2-acceptance-and-settings-research-dispatch](../../validation/p1b-r2-acceptance-and-settings-research-dispatch.md).
Claim `p1bs-settings-catalogue-defaults`. Consumer: developer Phase 1.2 (rules / season / playoff settings,
"research-blocked" in the [coverage matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md)), C6.
Supersedes nothing in handoff 21; it resolves its §6 step 0 and turns the catalogue lead into a contract proposal.
**Revision 2 (P1-BS-R1)** after [p1bs-review-and-contract-correction-dispatch](../../validation/p1bs-review-and-contract-correction-dispatch.md): executable agreement stated with its exceptions, guaranteed default/precedence/consumer wording removed, the contract narrowed to enumerated rows with preservation and eligibility rules, card P1BS-1 rewritten with bounded verdicts, four-site metadata applicability added (§1.9). Changes listed in §10.
**Revision 3 (P1-BS-CSV, 2026-09-19)** after [p1bs-r1-acceptance-and-csv-parity-dispatch](../../validation/p1bs-r1-acceptance-and-csv-parity-dispatch.md): the acceptance clarifications applied as wording only (§2.1 count, §2.4 baseline, §2.5 checksum scope, §1.9 `NVBd` distinction, §4 profile control); NHL 16–21 now handled by [overnight-2026-09-19 handoff 01](../overnight-2026-09-19/01-ps4-settings-csv-defaults-parity-and-packaging.md). Measured data unchanged. Changes listed in §11.

Machine-readable (all under [`data/`](data/)):
- [`p1bs-nhl15-ps4-settings-catalogue.json`](data/p1bs-nhl15-ps4-settings-catalogue.json) — **authority for §1**: the
  five NHL 15 PS4 tables (anchor + patch 01.06) with the offset-binary value, the raw bits of every negative-rangelow
  field, Studio's current reading where it differs, every join check, the patch diff, the executable descriptor table
  and the eboot choice-label arrays.
- [`p1bs-settings-cross-title-map.json`](data/p1bs-settings-cross-title-map.json) — **authority for §3**: per Phase 1
  league/competition setting (37 rows, keyed by the NHL 15 `legacyid` lineage id + FE name) the representation on
  every title/platform, the NHL 15 context ids with their NHL 16/21 names, the group ids with their NHL 16 names.
- [`p1bs-ps4-setting-csv-catalogues.json`](data/p1bs-ps4-setting-csv-catalogues.json) — the NHL 16–21 `setting_csv`
  members normalised (settings, contexts, groups, choice lists, ranges, dependencies, Phase 1 per-context defaults).
- [`p1bs-gen7-setting-tables.json`](data/p1bs-gen7-setting-tables.json) — the gen7 executable descriptor tables
  (NHL 10–15, Legacy) and the negative result for NHL 07/08/09.

Probes (`local/runs/overnight-research-2026-09-18/probes/`): `p1bs_catalogue.py`, `p1bs_eboot_runs.py`,
`p1bs_eboot_setting_table.py`, `p1bs_ps4_xtitle.py`, `p1bs_gen7_setting_table.py`, `p1bs_csv_catalogue.py`,
`p1bs_build_map.py`. No application code, defs or bindings changed; no launches; no source or image written; no
game bytes retained (identifiers, small integers, dictionary keys and hashes only).

**Evidence vocabulary:** **observed** = read from the named anchor, install member or executable; **inferred** =
a reading of observed values whose producing mechanism was not traced (label order, context names by id lineage,
which copy wins); **unknown** = not established (consumer code, selected-value storage, precedence).

## Plain-language summary

The NHL 15 PS4 database really does carry the game's settings catalogue: 223 setting definitions (identifier,
range, allowed-choice count, default), 373 per-screen defaults, and the screen/group layout that decides which
settings appear where. Reading it correctly needed one fix in how the bytes are interpreted: four of its columns
store values as "raw plus the declared minimum", not as signed numbers — the internal size arithmetic shows it on
213 of 223 rows, and a compiled copy of the catalogue inside the game executable agrees with the setting identity
column on 222 of 223 rows (186 rows agree on every numeric column; the differences are listed). Studio's parser
currently reads those four columns wrongly. With that fixed, the
league and competition settings (playoff series length, season and playoff tie-breaks, overtime-loss point,
salary cap on/off, playoff team count and seeding, home/away pattern, trophy, All-Star game, starting rosters,
franchise length, base league) are fully described, and their displayed choices ("Best of 7", "5 Min then
Shootout", …) can be named — from the executable on NHL 15/16 (order inferred) and from the game's own data on
NHL 17–21 (explicit).

Every PS4 title from NHL 16 on keeps the same catalogue as plain CSV files inside `inxp_00.big` with the same
setting and screen numbering; NHL 10–15 and Legacy on Xbox 360 keep a compiled copy of the setting definitions
(no per-screen defaults found there) in the executable; for NHL 07–09 none of the identifiers was found in the
executables or the loose files that were searched. EA itself changed these tables in the NHL 15 patch (it
re-pointed the tournament team-count setting and added a tournament screen); that shows the catalogue is
maintained as data, not that the game reads any particular row. A first, deliberately narrow editing contract is
proposed: change one *stored catalogue default* of one league/competition setting in one screen context on
NHL 15 PS4, byte-exact except the edited cell and the checksums. Whether the game shows or uses that stored
default, where a player's *chosen* value is stored, and whether a screen default or the global default wins when
both exist are all unknown; a one-setting-at-a-time user test card with bounded outcomes is prepared for that and
is not authorised here.

## 0. Sources (hashes)

| Source | Identity |
|---|---|
| NHL 15 PS4 base DB (= anchor `anchors/ps4/nhl15/databases/nhl_game.db`, = `inxp00.big` entry 0) | `ae9d1d56bf896b83b3376cc179e5bf3a6986cb594880f43de8a81e5b7e5078d8`, 16 981 156 B, little-endian; meta `nhl_game-meta.xml` |
| NHL 15 PS4 patch 01.06 DB (`H:\Emulators\ShadPS4\Games\CUSA00561\databigs\patch_inxp00.big` entry 0, same path) | `abe36f58fd0a65ab074aff13f83f2e671b707afc539ee57c01df128200c320ff`, 16 981 204 B |
| NHL 15 PS4 `eboot.bin` | `37df821a…` (handoff 17 witness) |
| NHL 16–21 PS4 `inxp_00.big` members `nhl/data/common/gamemodes/setting_csv/*.csv` (9–12 per title, chunkzip except the three tiny plain ones) | per-member plain-text SHA-256 in the CSV catalogue JSON `members` blocks |
| NHL 16–21 PS4 `eboot.bin` | handoff 17 witnesses |
| Xbox 360 `default.xex` → flat images (base `0x82000000`, `xex_image.py`) | NHL 10 `cc98dbec…`, 11 `70e46586…`, 12 `8842a990…`, 13 `5ef121e5…`, 14 `27d86df1…`, 15 `4ad8d308…`, Legacy `5a720c17…` (full hashes in the gen7 JSON); NHL 07/08/09 images generated, no FE setting names in them |
| NHL 15 PS4 `eng_us.db` (anchor) | key/label lookups only |

## 1. NHL 15 PS4 catalogue — layouts, decoding, joins, contexts, defaults (observed unless marked)

### 1.1 Exact layouts

All five tables: `alloc_type 2`, `max_records == current_records` (packed), no `update="True"` on any field, no
`savegroups` (they are never carried into a save container), key field = the first listed. Bit offsets are
LSB-first within the little-endian record.

| Table (raw id) | Rows | Record | Fields `name @bit_offset:width [rangelow..rangehigh]` |
|---|---|---|---|
| `stocksettings` (`fRvu`) | 223 | 56 B (447 bits) | `stringid @0:400` (inline string, 50 B) · `legacyid @400:8 [0..255]` **key** · `default @408:3 [0..7]` · `size @411:5 [0..31]` · `id @416:10 [-1..511]` · `style @426:2 [0..3]` · `max @428:5 [0..31]` · `min @433:4 [-8..7]` |
| `stocksettingdefaults` (`gZtX`) | 373 (379 in patch) | 4 B (31 bits) | `legacyid @0:8` · `context @8:6 [0..63]` **key** · `default @14:4 [0..10]` |
| `stocksettingcontext` (`HwRX`) | 197 (201) | 4 B | `context @0:6` **key** · `subgroupparent @6:7 [-1..126]` · `group @13:7 [0..127]` · `order @20:3 [0..7]` · `subgrouporder @23:5 [-1..15]` |
| `stocksettinggroups` (`Oreg`) | 702 (704) | 4 B | `legacyid @0:8` · `group @8:7 [0..127]` **key** · `order @15:7 [0..127]` |
| `stocksettingdependencies` (`FzWx`) | 25 (25) | 4 B | `dependentlegacyid @0:8` · `legacyid @8:8` **key** · `order @16:4 [0..15]` |

Every table is stored **sorted by its key field** in both the base and the patch DB (§1.6).

### 1.2 Decoding — raw bits, offset-binary value, Studio's reading

Four fields have a negative `rangelow`. Three independent observations establish that they store
**offset-binary** values, `value = raw + rangelow`, not two's complement:

1. `stocksettings.size == max − min + 1` holds on **213/223** rows under the offset reading and on **0/223** under
   two's complement (the 10 exceptions are rows with `size 0` — non-enumerated entries — plus `ScoreClockOverlay`,
   `PuckShadow`, `TXT_MUD_MAX_USERS`; see the JSON). `PeriodLength` reads `min 3, max 20, size 18, default 5`;
   `PlayerAdvantage`/`GoalieAdvantage` read `min −7, max 7, size 15`.
2. The NHL 15 `eboot.bin` carries a **static descriptor table** of the same 223 settings (§1.7): its `id` equals
   the DB `id` under the offset reading on **222/223** rows (0/223 under two's complement; the exception is
   `legacyid 222 TXT_LEAGUETYPE`, an empty executable record with id −1, no name, style/max/size 0 against DB
   id 209, 0..9, size 10) and its `min` equals the DB `min` under the offset reading on **222/223** rows
   (exception `PeriodLength`: DB 3, executable 4), including −7 for the two advantage rows. The executable is a
   witness for `id` and `min`; it says nothing about `subgroupparent`/`subgrouporder` (observation 3) and it is
   not a consumer trace.
3. `stocksettingcontext.subgroupparent` raw 37 → value 36 = the group that the six gameplay slider sub-groups hang
   under, and raw 75/76 → 74/75 = the two container groups of the main settings screen; NHL 16's `group.csv`
   names exactly these parents (`GAMEPLAY_DEFAULT` 36, `RULES_AND_GAMEPLAY_DEFAULT` 74,
   `VOLUME_AND_VISUAL_DEFAULT` 75). Raw 0 → −1 = "no parent".

**Studio's parser today** (`tdb-defs` `FieldMeta::decode_raw`: two's complement when the declared range fits the
width, all-ones `−1` sentinel when `rangelow == −1` and it does not) misreads all four:

| Field | Studio rule | Studio result | Correct (offset) |
|---|---|---|---|
| `stocksettings.min` (4 bits, −8..7) | two's complement | raw 8 → **−8** on 219 rows (should be 0); raw 11 → −5 (should be 3) | `raw − 8` |
| `stocksettings.id` (10 bits, −1..511) | two's complement (range fits) | raw (e.g. 153 for PlayoffSeriesLength) | `raw − 1` (152, = executable id) |
| `stocksettingcontext.subgroupparent` (7 bits, −1..126) | −1 sentinel (raw 127), else raw | raw 37 → 37 | `raw − 1` (36) |
| `stocksettingcontext.subgrouporder` (5 bits, −1..15) | two's complement | raw 0 → 0 | `raw − 1` (−1) |

This is a **parser cross-check finding for the developer** (confirmed at source level by the coordinator's
review; not fixed here): `tdb-defs` keeps an anchored `OFFSET_BINARY_SITES` list (two career plus/minus sites);
these four `(table, field)` sites belong on it with the scope in §1.9, and nothing here establishes a rule for
any other negative-rangelow field. Every `value` in the JSON is the offset reading; `*_raw` keeps
the bits.

**`default` and the value/index question.** `stocksettings.default` (3 bits) and `stocksettingdefaults.default`
(4 bits) are consistent with **values** (the volume settings are 7 in the 3-bit column and 10 in the 4-bit
per-context column — a saturated 3-bit copy of 10; `PeriodLength` default 5 = 5 minutes; `TXT_DYNASTY_LENGTH`
default 0 = "25" = EA's default). A value-vs-index ambiguity remains only for rows with `min ≠ 0`
(`PeriodLength`, `TXT_MUD_MAX_USERS`, the two advantage sliders); **every Phase 1 league/competition setting has
`min 0`, so code = value = index there** and the ambiguity is moot for this contract. `TWOLINESOFFSIDE`,
`ControlIndicator`, `BRIGHTNESS/CONTRAST/SCREEN_ANGLE/SKILL` carry a global default outside their own range
(observed; not a rule setting).

### 1.3 Identifier joins (all checked; details in `analysis_anchor`)

- `legacyid` is the row key: 0–222 contiguous, unique. `id` is the **setting identity** and is shared by rows
  that expose the same setting on different screens (`PlayoffSeriesLength` ×3 → id 152; `TieBreak`,
  `SeasonTieBreak`, `RoundRobinTieBreak` → id 18; `StartingRosters` ×3 → 154; `TXT_LEAGUETYPE` ×4 → 209;
  `SeasonOvertimeLoss` / `RoundRobinOvertimeLLoss` → 155). `SHOOTING_CONTROL_TOGGLE` has id −1. 11 rows are named
  `NULL` (reserved), 14 `stringid`s are duplicated.
- `stocksettingdefaults`: 13 contexts (0, 1, 3, 5, 20, 30, 47, 48, 49–52, 54), 139 distinct `legacyid`s, no
  duplicate `(context, legacyid)`; **one dangling reference** — `legacyid 236` (four rows, contexts 49–52; no such
  setting); **four contexts without layout rows** (49–52). 289 per-context defaults equal the global default,
  **80 differ** (e.g. `NHLSalaryCap` global 0 but 1 in contexts 3 and 5; volumes 10 vs 7; `Camera` 7 vs 1).
- `stocksettingcontext`: 53 contexts, 197 rows; every `group` exists in `stocksettinggroups` except **36** (a
  container: its children are the six sub-groups with `subgroupparent 36`); container groups 74/75 also have no
  member rows.
- `stocksettinggroups`: 76 groups, no duplicate `(group, legacyid)`, one duplicate `(group, order)` = (80, 94);
  59 settings belong to no group (sliders' second variants, the `NULL` rows, …).
- `stocksettingdependencies`: 25 rows, all ids resolve; Phase 1-relevant: `TXT_LEAGUETYPE` (115) → 13 dependents
  in order (`NumberOfTeams` 122, `PlayoffSeedingType` 119, `PlayoffSeriesLength` 116, `TrophyType` 117,
  `HomeAwayType` 120, `PlayoffPrelimSeriesLength` 118, `TXT_LEAGUETYPE` 121, `PlayoffTieBreak` 73, `Offsides`,
  `Icing`, `Injuries`, `Penalties`, `TXT_FIGHTING`); `PlayoffSeedingType` (119) → `NumberOfTeams` (122);
  `DynastyStartPoint` (123) → `StartingRosters` (67). The dependency semantics (which choice enables what) are
  **unknown**; the table only says "re-evaluate the dependent when this changes" (inferred).
- The executable's DB-schema name list (a 796-entry string-pointer array at VA `0x3c2a598`, table name followed
  by its columns alphabetically) contains all five tables and all their columns — the game binds these tables
  by name (observed binding; consumer code not traced).

### 1.4 Contexts (screens) and groups — names by id lineage (inferred)

NHL 15 stores no context or group names. NHL 16's `context.csv`/`group.csv` and NHL 21's `context.csv` use the
**same numbering** (checked structurally: NHL 16 group 36 = `GAMEPLAY_DEFAULT` with parent 74, groups 50–58 with
parent 36, group 5 = `QUICK_DYNASTY_FLOW` holds exactly the NHL 15 group-5 members, etc.). Names below are
therefore by-reference, not observed on NHL 15.

| NHL 15 context | NHL 16 key / NHL 21 name | Phase 1 content on NHL 15 | Defaults rows (base / patch) |
|---|---|---|---|
| 0 | `FROM_PLAY_NOW_FLOW` / `DISPLAY_PLAY_NOW_MODE` | game rules only | 107 / 107 |
| 1 | `FROM_MAIN_MENU` / `DISPLAY_MAIN_MENU` | — | 35 |
| **3** | `FROM_DYNASTY_FLOW` / `DISPLAY_FRANCHISE_CREATION_CLASIC` | group 5 (`TXT_DYNASTY_LENGTH`, `NHLSalaryCap`, cap penalties, draft-pick ownership, …) + 27 (rules) + 15 (`PlayoffSeriesLength` 72, `PlayoffTieBreak` 73, `SeasonOvertimeLoss` 71, `SeasonTieBreak` 68) | 26 / 26 |
| **5** | `FROM_MUD_CREATION_FLOW` (online GM connected) / — | group 2 + 25 + 13 (same rule rows + `TXT_MUD_LENGTH`, `StartingRosters` 212, `TXT_ROSTER_TYPE`) | 25 |
| 9 | `FROM_TOURNAMENT_EA_FLOW` / — | group 8 (`NumberOfTeams` 82, `NumberOfRoundRobins` 83, `PlayoffSeriesLength` 86) + 30 + 16 | 0 / **6 (added by the patch)** |
| 10, 11, 12 | tournament Memorial / MTL / Euro flows | groups 9/8/7 + 30 + 72/16 | 0 |
| **30** | `FROM_BE_A_PRO_FLOW` / `DISPLAY_BE_A_PRO_CREATION` | group 6 + 27 + 17 (the four season/playoff rule rows) | 17 |
| **34** | `FROM_PLAYOFF_MODE_FLOW` / `DISPLAY_PLAYOFF_CREATION` | group 10 (`HomeAwayType`, `TXT_LEAGUETYPE` 121, `PlayoffPrelimSeriesLength`, `PlayoffSeriesLength` 116 and 86, `DynastyStartPoint`, `TrophyType`) + 30 + 18 | **0** — global defaults only |
| **37** | `FROM_SEASON_MODE_FLOW` / `DISPLAY_SEASON_CREATION` | group 11 (`BaseLeague`, `PlayoffSeriesLength` 86, `TXT_DYNASTY_CPU_TRADES`, …) + 30 + 73 | **0** — global defaults only |
| 47 | `FROM_ONLINE_PLAY_NOW_FLOW` / `DISPLAY_PLAY_NOW_ONLINE_MODE` | — | 25 |
| 49–52 | `INTERNATIONAL/NHL/CHL/NO_SETTINGS` / `DEFAULT_LEAGUE_RULES_*` | **game-rule presets** (Offsides, Icing, TWOLINESOFFSIDE, Injuries, TieBreak, Penalties, Fighting + the dangling 236) — no layout rows | 8 each |
| 53, 54 | `FROM_GLOBAL`, `FROM_ONLINE_RESET` | — | 0, 88 |

Full per-context tables: `contexts_nhl15` / `groups_nhl15` in the map JSON. **Precedence between a context
default and the global `stocksettings.default` is unknown**, and so is whether either is what a creation screen
offers (no consumer traced). The Season and Playoff creation contexts have *no* defaults rows at all while
Franchise/MUD/BAP repeat the rule rows; that pattern is compatible with several mechanisms (context rows override
a global fallback; contexts without rows read the executable table; the screen reads a persisted profile) and
selects none of them.

### 1.5 Per-context defaults of the Phase 1 settings (observed, base DB; codes, with the label the code maps to under §1.7)

| legacyid · FE name | id | min..max (size) | global default | ctx 3 (Franchise) | ctx 5 (MUD) | ctx 30 (BAP) | ctx 9 (patch) |
|---|---|---|---|---|---|---|---|
| 68 `SeasonTieBreak` | 18 | 0..5 (6) | 2 = `5MinThenShootout` | 2 | 2 | 2 | — |
| 71 `SeasonOvertimeLoss` | 155 | 0..1 (2) | 1 = `1Point` | 1 | 1 | 1 | — |
| 72 `PlayoffSeriesLength` | 152 | 0..4 (5) | 3 = `BestOf7` | 3 | 3 | 3 | — |
| 73 `PlayoffTieBreak` | 156 | 0..2 (3) | 0 = `ContinuousOT` | 0 | 0 | 0 | 0 |
| 70 `NHLSalaryCap` | 162 | 0..1 (2) | **0** = OFF | **1** | **1** | — | — |
| 77 `TXT_DYNASTY_LENGTH` | 190 | 0..4 (5) | 0 = "25" | 0 | — | — | — |
| 69 `AllStarGame` | 153 | 0..2 (3) | 1 = `WorldVsNA` | — | — | — | — |
| 67 / 85 / 212 `StartingRosters` | 154 | 0..2 / 0..2 / 0..1 | 0 = `current` | — | — (212: 0) | — | — |
| 86 `PlayoffSeriesLength` (tournament/season) | 152 | 0..4 | 2 = `BestOf5` | — | — | — | 3 |
| 116 `PlayoffSeriesLength` (playoff mode) | 152 | 0..4 | 3 | — | — | — | — |
| 87 `PlayoffTieBreak` (tournament) | 156 | 0..2 | 1 = `Shootout` | — | — | — | — |
| 88 `RoundRobinTieBreak` | 18 | 0..5 | 4 = `SingleOt5on5` | — | — | — | — |
| 89 `RoundRobinOvertimeLLoss` | 155 | 0..1 | 0 | — | — | — | — |
| 82 `NumberOfTeams` (tournament) | 158 → **186 in patch** | 0..1 (2) → **0..3 (4)** | 0 | — | — | — | 3 |
| 122 `NumberOfTeams` (playoff mode) | 186 | 0..4 (5) | 4 | — | — | — | — |
| 83 `NumberOfRoundRobins` | 159 | 0..6 (7) | 4 | — | — | — | — |
| 118 `PlayoffPrelimSeriesLength` | 182 | 0..4 | 3 | — | — | — | — |
| 119 `PlayoffSeedingType` | 174 | 0..4 (5) | 0 = `TXT_PLAYOFFTYPE_DIVISIONAL` | — | — | — | — |
| 120 `HomeAwayType` | 176 | 0..2 (3) | 0 = `TXT_HOMEAWAY_STANDARD` | — | — | — | — |
| 117 `TrophyType` | 196 | 0..3 (4) | 0 = `StanleyCup` | — | — | — | — |
| 125 `BaseLeague` | 207 | 0..13 (14) | 0 = NHL | — | — | — | — |
| 115 / 121 / 179 / 222 `TXT_LEAGUETYPE` | 209 | 0..10 / 0..10 / 0..9 / 0..9 | 0 | — | (179: 0) | — | — |
| 123 `DynastyStartPoint` | 197 | 0..1 | 0 | — | — | — | — |
| 15 `TieBreak` (game rule, Play Now) | 18 | 0..5 | 2 | ctx 0/1/49–52 | | | |

`—` = no defaults row for that context (global default is the only stored default). The three `TXT_MUD_*` rows,
`TXT_DRAFT_PICK_OWNERSHIP`, `TXT_AUTHENTIC_CAP_PENALTIES`, `FantasyDraft`, `LeagueName`/`TournamentName` (size 0,
free text) are in the JSON.

### 1.6 Patch 01.06 vs base — EA edits the catalogue as data (observed)

| Table | Change |
|---|---|
| `stocksettings` | one row changed: `legacyid 82 NumberOfTeams` re-pointed from id 158 (2 choices "8/16") to id **186** (4 choices "2/4/8/16"): `max 1→3`, `size 2→4` |
| `stocksettingdefaults` | +6 rows, all context **9** (SkillLevel 1, PeriodLength 5, PlayoffTieBreak 0, NumberOfTeams 3, PlayoffSeriesLength 86 = 3, GameStylePreset 1), inserted at records 193–198 (key order) |
| `stocksettingcontext` | context 9 loses groups 16/30, context 13 gains the six gameplay sub-groups (+6 −2 rows) |
| `stocksettinggroups` | groups 8, 30, 71 re-membered / re-ordered, group 80 order shift (+18 −16 rows) |
| `stocksettingdependencies` | identical |

Row order of unchanged rows is preserved; new rows sit at their key position; `max_records` grows with
`current_records` (the tables stay packed). This coincides with the NHL 15 "Playoff mode" post-launch content
(inferred). It shows the catalogue is **maintained as data**; a static patch change does not establish that the
game consumes any row, and no consumer was traced.

### 1.7 The executable's copy and the displayed choices (observed table; code → label binding inferred)

The NHL 15 eboot holds a **48-byte-record descriptor table** at VA `0x3d60080`, 223 records in `legacyid`
order — `{id i32, pad, name char*, style u32, min i32, max u32, default u32, size u32, pad, labels char**}` —
i.e. a compiled twin of `stocksettings`. **186/223** records equal the DB row on all six numeric fields
(`id, style, min, max, default, size`). The 37 exceptions, all enumerated in `executable_descriptor_table.db_vs_exe`:
26 rows differ in `default` (Phase 1 rows: exe `SeasonTieBreak` 4, `PlayoffSeriesLength` 72 → 2,
`SeasonOvertimeLoss` 0, `PlayoffTieBreak` 73 → 1, `TXT_DYNASTY_CPU_TRADES` 0, `LeagueName`/`TournamentName` 4,
`TieBreak` 4 vs DB 2/3/1/0/1/0/2); 14 in `max` and 12 in `size` (`Camera` 0..10/11, `PeriodLength` 4..17,
`ScoreClockOverlay`, `PuckShadow`, `ShootoutCamera`, `PRESET`, `TXT_BAP_CAMERA`, `NumberOfTeams` 122 (exe 0..3/4,
default 3), `TXT_LAST_PLAYED_MODE_ID`, `GameStylePreset` (exe all 0), `TXT_ONLINE_*_CAMERA`, `TXT_GOALIE_CAMERA`,
`TXT_MUD_MAX_USERS`); one `min` (`PeriodLength`); one `id`/`style` (the empty record 222). Which copy the game
uses — or whether it merges them — is **unknown**. The DB defaults being the NHL-authentic values and the DB
being patch-maintained are observations, not evidence of consumption.

The `labels` pointer of each record addresses an array of dictionary keys (arrays are contiguous; the first
`size` entries belong to the record). For the Phase 1 settings the arrays are:

| Setting (id) | Codes → dictionary keys (NHL 15 eboot; NHL 16 eboot identical) | Confirmation |
|---|---|---|
| `PlayoffSeriesLength` (152) | 0 `SingleKnockout` · 1 `BestOf3` · 2 `BestOf5` · 3 `BestOf7` · 4 `Authentic` | NHL 17–21 `choice.csv` list `DYNASTY_PLAYOFFS_LENGTH`/`PLAYOFF_SERIES_LENGTH` has exactly these values |
| `TieBreak` / `SeasonTieBreak` / `RoundRobinTieBreak` (18) | 0 `ContinuousOT` · 1 `Shootout` · 2 `5MinThenShootout` · 3 `SingleOt4on4` · 4 `SingleOt5on5` · 5 `NoOvertime` | NHL 17+ inserts `OT3ThenShootout` at 2 and adds `authentic` 7 — **the codes shift from NHL 17 on** |
| `PlayoffTieBreak` (156) | 0 `ContinuousOT` · 1 `Shootout` · 2 `5MinThenShootout` | NHL 17+: 0/1/2 `OT3ThenShootout`/3 `5MinThenShootout` |
| `SeasonOvertimeLoss` / `RoundRobinOvertimeLLoss` (155) | 0 `0Points` · 1 `1Point` | same on 17–21 |
| `StartingRosters` (154) | 0 `current` · 1 `default` · 2 `defaultdraft` | 17–21: 0/1 only |
| `AllStarGame` (153) | 0 `none` · 1 `WorldVsNA` · 2 `EastVsWest` | — |
| `TXT_DYNASTY_LENGTH` (190) | 0 "25" · 1 "5" · 2 "10" · 3 "15" · 4 "20" | same on 17–21 (`DYNASTY_LENGTH`) |
| `NumberOfTeams` (186 / 158) | 0 "2" · 1 "4" · 2 "8" · 3 "16" / 0 "8" · 1 "16" | same on 17–21 (`PLAYOFF_NUM_TEAMS`) |
| `NumberOfRoundRobins` (159) | 0 "1" … 6 "7" | — |
| `HomeAwayType` (176) | 0 `TXT_HOMEAWAY_STANDARD` · 1 `_ALTERNATE` · 2 `_REVERSE_ALTERNATE` | — |
| `TrophyType` (196) | 0 `StanleyCup` · 1 `CalderCup` · 2 `EACup` · 3 `MemorialCup` | — |
| `PlayoffSeedingType` (174) | 0 `TXT_PLAYOFFTYPE_DIVISIONAL` · 1 `TXT_PLAYOFF_CONFERENCE_RESEED` · 2 `_CONFERENCE_BRACKET` · 3 `_LEAGUE_RESEED` · 4 `_LEAGUE_BRACKET` | — |
| `BaseLeague` (207) | 0 `NHL` · 1 `AHL` · 2 `elitserien` · 3 `SMLIIGA` · 4 `TXT_LEAGUE_DEL` · 5 `TXT_LEAGUE_O2_EXTRALIGA` · 6 `TXT_LEAGUE_SWISS_LEAGUE` · 7 `OHL` · 8 `QMJHL` · 9 `WHL` · 10 `CUSTOM` (array of 11; DB size 14 — **mismatch, unresolved**) | — |
| `NHLSalaryCap` (162), `DynastyStartPoint` (197), … | 0 `OFF` · 1 `ON` (generic toggle array) | — |
| `Rules` (0, game rules) | 0 `Custom` · 1 `NHL` · 2 `International` · 3 `CHL` · 4 `Off` | matches the preset contexts 49–52 |

`stringid` itself is the dictionary key of the setting's label (`SeasonTieBreak` = "Season Tie Break",
`SeasonTieBreak_Desc` / `SEASONTIEBREAK_DESC` its description) — observed by key equality in `eng_us.db`.
The **code → label binding is array order** (inferred); the NHL 17–21 explicit lists agree wherever the
vocabulary did not change, which is the strongest available corroboration short of tracing the UI code.

### 1.8 Mode-save cross-reference (observed values; bindings are candidates only)

In the NHL 15 PS4 and Xbox 360 Be-A-GM specimens, `leaguestates` types 29–34 are six `3`s = the code of
`PlayoffSeriesLength` default 3 (`BestOf7`) per round; types 46/47 are `2` = `SeasonTieBreak` default 2; types
41–43 are `1`. The Season specimens carry the same values although the Season context has no defaults rows
(global `NHLSalaryCap` 0, `SeasonTieBreak` 2). These coincidences bind nothing: on NHL 18–21 the same six `3`s
occur while the NHL 18+ default of `DYNASTY_PLAYOFFS_LENGTH` is 4 (`Authentic`), so the store may hold *resolved*
per-round lengths rather than the setting code. Binding a `leaguestates` type to a setting needs the card in §4.

### 1.9 Four-site metadata applicability (for the queued decoder correction P1-BS-1)

Scan: every metadata file available to this lane — the 27 anchor `*-meta.xml` main-DB files plus the loose
localization metadata, the NHL 07 file, and the private-fixture metadata under `E:\nhl-studio-fixtures\databases`
(NHL 08/09 PS3, NHL 19/20/21 PS4) — 55 files; plus the NHL 15 PS4 patch 01.06 `nhl_game-meta.xml` read out of
`patch_inxp00.big` (identical bytes to the anchor file, `d9cf1be4…`).

| Site (table raw id . field raw id) | Metadata name | Width | Declared range | Occurrences of the *(table, field)* pair | Field raw id elsewhere | Witness for the offset reading |
|---|---|---|---|---|---|---|
| `fRvu.SbEu` | `stocksettings.min` | 4 | −8..7 | NHL 15 PS4 only (base = patch metadata) | none | size arithmetic 213/223; executable `min` 222/223 |
| `fRvu.CkjP` | `stocksettings.id` | 10 | −1..511 | NHL 15 PS4 only | **`qbPU.CkjP` = `leaguesimpopups.id`** on NHL 15 PS4 (14 bits, 0..16383 — non-negative, unaffected, but the raw field id collides) | executable `id` 222/223 |
| `HwRX.QHRR` | `stocksettingcontext.subgroupparent` | 7 | −1..126 | NHL 15 PS4 only | none | NHL 16 `group.csv` parent ids (36/74/75) by lineage — inferred witness |
| `HwRX.NVBd` | `stocksettingcontext.subgrouporder` | 5 | −1..15 | NHL 15 PS4 only | none | structural only (raw 0 on every non-subgroup row, raw 2–7 on the six subgroups) — **unverified against any external witness** |

Variants: none of the five catalogue tables or the four field ids occurs in any other title's metadata (PS4
16–21 dropped the tables; gen7/PS3 never had them). Witnessed = the NHL 15 PS4 base and patch DBs (same
metadata, same table shapes except row counts). Unverified = any other NHL 15 PS4 build (only 01.00 base and
01.06 patch exist on this machine).

**Narrowest safe decoder scope (recommendation).** Register exactly the four `(table raw id, field raw id)`
pairs above, keyed by table *and* field (the `CkjP` collision in `leaguesimpopups` shows a field-id-only key would
be wrong), and scope them to metadata whose `stocksettings`/`stocksettingcontext` field set matches the NHL 15
PS4 declaration (same widths and ranges) — not to a title name, so the patch DB and any unseen NHL 15 build with
identical metadata are covered while a differently declared variant is not. `subgrouporder` may be listed with
its witness marked *structural only*, or left as-is if the developer prefers only externally witnessed sites;
either choice must be recorded. No other negative-rangelow field changes. *(rev. 3)* `HwRX.NVBd` stays a
**structurally inferred** offset interpretation, unlike the executable-witnessed `stocksettings.min`/`id`; the decoder
package may apply the scoped interpretation, but its evidence and code commentary must preserve that distinction —
no claim that every field was externally verified and no general negative-range rewrite. Table-plus-field keying
and a matching declaration shape are required, with a regression for the unrelated `qbPU.CkjP`.

## 2. Contract proposal — editing one stored league/competition default (NHL 15 PS4)

A proposal for the developer, not authorisation to implement. It edits a **stored catalogue default** — a cell in
the main database — and claims nothing about what any screen offers, what precedence applies, or what a mode
uses; those remain unknown (§1.4, §1.7, §4). No game-rule sliders, no profile/save editing, no row creation.

### 2.1 Eligible target rows (enumerated; nothing else is offered)

A target is a pair *(legacyid, context)*; `context = global` addresses `stocksettings.default`, otherwise
`stocksettingdefaults.default` of the row `(context, legacyid)`, which must exist in the opened DB. Eligibility is
by **row identity and resolved bounds**, evaluated on the opened file (§2.4), not by the file being stock.

| legacyid · FE name | Role (by NHL 16 key) | Bounds (offset reading) | Codes → labels (§1.7; **inferred**, shown next to the raw code) | Context targets that exist in stock |
|---|---|---|---|---|
| 68 `SeasonTieBreak` | `DYNASTY_SEASON_TIE_BREAK` | 0..5 | 0 ContinuousOT · 1 Shootout · 2 5MinThenShootout · 3 SingleOt4on4 · 4 SingleOt5on5 · 5 NoOvertime | global, 3, 5, 30 |
| 71 `SeasonOvertimeLoss` | `DYNASTY_SEASON_OVERTIME_LOSS` | 0..1 | 0 0Points · 1 1Point | global, 3, 5, 30 |
| 72 `PlayoffSeriesLength` | `DYNASTY_PLAYOFFS_LENGTH` | 0..4 | 0 SingleKnockout · 1 BestOf3 · 2 BestOf5 · 3 BestOf7 · 4 Authentic | global, 3, 5, 30 |
| 73 `PlayoffTieBreak` | `DYNASTY_PLAYOFFS_TIE_BREAK` | 0..2 | 0 ContinuousOT · 1 Shootout · 2 5MinThenShootout | global, 3, 5, 30 (+ 9 in the patch copy) |
| 70 `NHLSalaryCap` | `DYNASTY_SALARYCAP` | 0..1 | 0 OFF · 1 ON | global, 3, 5 |
| 77 `TXT_DYNASTY_LENGTH` | `DYNASTY_LENGTH` | 0..4 | 0 "25" · 1 "5" · 2 "10" · 3 "15" · 4 "20" | global, 3 |
| 69 `AllStarGame` | `DYNASTY_ALLSTARGAME` | 0..2 | 0 none · 1 WorldVsNA · 2 EastVsWest | global |
| 86 `PlayoffSeriesLength` | `TOURNAMENT_ELIMINATION_SERIES_LENGTH` | 0..4 | as 72 | global (+ 9 in the patch copy) |
| 116 `PlayoffSeriesLength` | `GAME_MODE_PLAYOFF_SERIES_LENGTH` | 0..4 | as 72 | global |
| 87 `PlayoffTieBreak` | `TOURNAMENT_ELIMINATION_TIE_BREAK` | 0..2 | as 73 | global |
| 88 `RoundRobinTieBreak` | `TOURNAMENT_ROUND_ROBIN_TIE_BREAK` | 0..5 | as 68 | global |
| 89 `RoundRobinOvertimeLLoss` | `TOURNAMENT_ROUND_ROBIN_OVERTIME_LOSS` | 0..1 | as 71 | global |
| 83 `NumberOfRoundRobins` | `TOURNAMENT_NUMBER_ROUND_ROBINS` | 0..6 | 0 "1" … 6 "7" | global |
| 118 `PlayoffPrelimSeriesLength` | `GAME_MODE_PRELIMINARY_SERIES_LENGTH` | 0..4 | as 72 | global |
| 119 `PlayoffSeedingType` | `GAME_MODE_SEEDING_TYPE` | 0..4 | 0 DIVISIONAL · 1 CONFERENCE_RESEED · 2 CONFERENCE_BRACKET · 3 LEAGUE_RESEED · 4 LEAGUE_BRACKET | global |
| 120 `HomeAwayType` | `GAME_MODE_HOME_AND_AWAY_SCHEDULE` | 0..2 | 0 STANDARD · 1 ALTERNATE · 2 REVERSE_ALTERNATE | global |
| 117 `TrophyType` | `GAME_MODE_CUP_TYPE` | 0..3 | 0 StanleyCup · 1 CalderCup · 2 EACup · 3 MemorialCup | global |
| 67 / 85 `StartingRosters` | `DYNASTY_` / `TOURNAMENT_STARTING_ROSTERS` | 0..2 | 0 current · 1 default · 2 defaultdraft | global |
| 212 `StartingRosters` | `MUD_STARTING_ROSTERS` | 0..1 | 0 current · 1 default | global, 5 |

*(rev. 3)* The table lists **21 distinct legacy ids across 20 displayed rows** (67 and 85 share a line): 67, 68, 69, 70,
71, 72, 73, 77, 83, 85, 86, 87, 88, 89, 116, 117, 118, 119, 120, 123, 212. These explicit ids, their expected row
names and the bounds validated on the opened file govern eligibility; a row count never removes an id.
| 123 `DynastyStartPoint` | `GAME_MODES_SEASON_START_PERIOD` | 0..1 | 0 · 1 (labels not bound — shown as raw codes only) | global |

**Excluded** (with the reason, shown to the user if they look for the row): 74 `LeagueName`, 81 `TournamentName`
(`size 0`, free-text rows); 82 and 122 `NumberOfTeams` (bounds differ between the base and patch copies for 82;
DB `size 5` vs a 4-entry label array for 122 — unresolved code bounds); 125 `BaseLeague` (11-entry array vs
`size 14`); 115 / 121 / 179 / 222 `TXT_LEAGUETYPE` (no label array bound; 222 is empty in the executable);
15 `TieBreak` and every Play Now / rules-preset context (game rules, out of Phase 1); 84, 153, 190, 203, 213,
217 (MUD / draft / cap-penalty rows — not league/competition rules or labels unbound); every row named `NULL`;
every row whose `id` is shared with an excluded row *only if* the user targets it through the sibling list
(siblings are informational, never targets).

### 2.2 Selection and target identity

The user picks one eligible row and one of its existing context targets. The review shows: the raw code and the
inferred label side by side for the current and the new value (label pill "inferred from the executable's label
array"); the global default and every stored per-context default of the same `legacyid`; the sibling rows with
the same `id` marked "not changed". A target whose `(context, legacyid)` row does not exist is refused: "no stored
default for this setting on this screen; adding one is a table-growth operation that is not offered" — the
refusal makes **no** statement about which default the game would use. A target that resolves to more than one
record (duplicate key — never the case in stock, but checked) is refused as ambiguous.

### 2.3 Encoding and bounds

The written value is the code; both `default` fields have `rangelow 0`, so raw = code. Allowed: `0 ≤ code ≤ max`
of the row's `stocksettings` entry **as decoded with the offset reading**, `code < size`, and, for the global
3-bit column, `code ≤ 7` (refused otherwise with the reason; the per-context 4-bit column allows ≤ 10 by
metadata but the eligible rows never exceed 6). A code equal to the current value is a no-op review.

### 2.4 Eligibility of the opened file (not "pristine stock")

Before offering a target the operation decodes the five tables and evaluates: (a) the target row exists exactly
once; (b) the row's `stocksettings` entry decodes with `size == max − min + 1` and `min == 0`; (c) the five
tables keep their declared shapes (field widths/ranges of §1.1, packed `max == current`, key-sorted). Pre-existing
stock anomalies are **preserved and ignored**: the dangling `legacyid 236` defaults rows, contexts 49–52 without
layout rows, groups 36/74/75 without members, the duplicate `(80, 94)` order, the `size 0` rows, the 59 settings
in no group. They are reported once as "stock anomalies (unchanged)". The operation **rejects** only defects it
would introduce or that make the target ambiguous: an out-of-bounds code, a duplicate target key, a target row
missing, a shape mismatch in the two tables it writes to.

**A second valid edit stays eligible.** Eligibility is evaluated against the opened bytes, so a file that already
carries one or more reviewed catalogue edits (or any unrelated Studio edit) qualifies as long as (a)–(c) hold for
the new target; baseline validation excludes exactly the cells recorded in earlier reviews, as LOC-W3 does for
its rows. Two edits to the same cell in one review collapse to the last value. *(rev. 3)* Prior reviewed edits are
part of the current baseline, **not** an exemption from bounds or target-identity validation: every new edit
re-validates the current target and the file identity, unrelated existing anomalies are preserved, previously
edited cells are not blanket-skipped, and reopening an edited file is eligible without retaining old review tickets.

### 2.5 Preservation — the only bytes that may differ

The record holding the cell is 4 bytes (`stocksettingdefaults`, three packed fields) or 56 bytes
(`stocksettings`, eight fields); the write changes **only the bits of the selected field** inside that record.
Verification after the write, on the whole file: (1) every byte outside the target record and outside the
checksum words is identical; (2) inside the target record, every bit outside the selected field's
`[bit_offset, bit_offset + width)` is identical (mask compare — the other two/seven fields of the record are
covered by this rule, not by "the record may change"); (3) the checksum words that legitimately change are
accounted **individually by file offset** *(rev. 3, tightened)*: for a same-length cell edit on a valid baseline the
table headers and the file header are unchanged, so **their CRCs must remain byte-identical** even if the serializer
recomputes them — a changed `header_crc` (`+0x24`) or file-header hash (`0x14`) is a failed write, not an accounted
difference; **only the checksum that covers the edited body may change**: the `prior_crc` (`+0x00`) of the *next*
physical table, or the EOF CRC if the edited body is the last one. Its exact offset is derived from the table layout
and its new value verified independently of the writer (`tdb-core::TdbFile::recompute_crcs` scopes). Unrelated
invalid checksums already present in the baseline are neither repaired nor allowed to change. When the DB is written
back as an archive member — nothing else in the archive (member length unchanged, every other member byte-identical).
Any other difference fails the write and rolls back.
Row count, order, `max_records`, and every other table are byte-identical by (1).

### 2.6 Review description, undo, copied output, placement

- Pending-review text (human-readable; ids behind disclosure): "NHL 15 PS4 — stored catalogue default of
  *Playoff Series Length* for the Franchise creation screen (context 3) changed from code 3 (*Best of 7*, label
  inferred) to code 2 (*Best of 5*). The global default (3) and the Tournament/Playoff-mode rows are unchanged.
  Whether this stored default is what the game offers or applies has not been established."
- Undo: cell-level inverse from the review; restores byte identity of the record and the same checksum words.
- Copied output: hash-compare the copied DB member against the source outside the accounted offsets; re-open
  through Studio and assert decoded equality except the one cell; member length and all other members unchanged.
- Placement: NHL 15 PS4 has two DB copies (`inxp00.big` base `ae9d…`, `patch_inxp00.big` `abe3…`); the edit
  applies to the copy the project opened; the review names the other copy and states that which copy the
  installed game reads is unresolved (atlas Q11). The copies differ in the tournament rows (§1.6), so a
  base-copy project must not be described as the patched game.

### 2.7 Refused in this contract

Adding/removing defaults, context or group rows (growth); editing `stocksettings` `min/max/size/id/style`,
`stocksettingcontext`, `stocksettinggroups`, `stocksettingdependencies`; any executable, profile or save write;
any wording that a stored default is "the default offered" or "the value used"; NHL 16–21 (a CSV field
replacement inside a chunkzip member is not same-length — dependency: chunkzip member repack/re-plan, P1-F; *(rev. 3)*
the per-title CSV contract, packaging requirements and vectors are in
[overnight-2026-09-19 handoff 01](../overnight-2026-09-19/01-ps4-settings-csv-defaults-parity-and-packaging.md));
gen7/PS3/Recompiled (no DB or data-file representation found in the inspected sources — dependency: a data-side
representation or an executable-backed route with its own evidence); NHL 07–09.

### 2.8 Metadata note

None of the five tables declares `update="True"`; the NHL 21 `stockleagues.name` rename (E1) already edits a
static non-`update` table under the direct-editing model, so EDIT-MODEL entry rule 1 needs the same deliberate,
recorded exemption here (developer decision).

## 3. Cross-title equivalents (observed presence; meanings per title keep their own provenance)

| Title / platform | Where the catalogue is | Setting identity | Ranges / defaults | Per-context defaults | Choice labels | Evidence |
|---|---|---|---|---|---|---|
| **PS4 NHL 15** | main DB tables (§1); executable twin at `0x3d60080` | `legacyid` (row) + `id` (setting) | DB (offset reading) | `stocksettingdefaults` (13 contexts; +ctx 9 in patch) | eboot arrays (inferred order) | observed |
| **PS4 NHL 16** | `inxp_00.big` `nhl/data/common/gamemodes/setting_csv/`: `setting.csv` (267 rows: `id,key,name,string_id,description_id,is_choice,min_value,max_value,default_value,remap_id`), `context.csv` (61, named), `contextsetting.csv` (570: `context_id,setting_id,min_value,max_value,default_value,choices,is_enabled`), `contextgroup`, `group` (89, named, `parent_id`), `settinggroup`, `choice.csv` (placeholder keys `CHOICE_n`), `stocksettingdependencies.csv` (25, same rows as NHL 15) | `id` = NHL 15 `legacyid` (all 37 Phase 1 rows present; 36 with the same FE name, legacyid 222 renamed `TXT_NOT_IN_USED`); `key` = the uppercase id (`DYNASTY_SEASON_TIE_BREAK`, …); `remap_id` is a renumbered space (98 = NHL 15 id, 76 = NHL 15 raw id, 48 neither — not usable as a decoding witness) | CSV | CSV (`min/max/choices` overrides per context) | eboot arrays identical to NHL 15 (`OT43ThenShootout` appears in one extra array) | observed |
| **PS4 NHL 17** | same folder: `setting.csv` (351: `id,name,legacy_id,is_global,is_choice,choice_to_range,name_loc,description_loc,extra_loc`), `context.csv` (102), `contextsetting.csv` (8 306: `context_id,setting_id,is_enabled,default_value,potential_values`), `contextgroup`, `group` (64), `groupsetting`, `choice.csv` (299: `id,choicelist_id,name,value,name_loc,…`), `choicelist.csv` (71), `range.csv` (12), `settingservice.csv` | same ids; global default = context **−2 `_DEFAULT_`** row | CSV | CSV (every context, `is_enabled`) | **explicit** (`choice.value` → `name_loc`) | observed |
| **PS4 NHL 18 / 19 / 20 / 21** | same folder, `setting.csv` gains `default_value,potential_values,usage,audio_id` (376 / 389 / 409 / 427 rows); contexts 111 / 111 / 117 / 120; `contextsetting` 8 944 / 10 232 / 10 703 / 10 969 | same ids (`DYNASTY_PLAYOFFS_LENGTH` 72, `PLAYOFF_ELIMINATION_SERIES_LENGTH` 86, `GAME_MODE_PLAYOFF_SERIES_LENGTH` 116, …): 26 of the 37 NHL 15 Phase 1 rows present with the same id and FE name (from NHL 17 on); absent: `LeagueName` 74, `TournamentName` 81, `NumberOfRoundRobins` 83, `TXT_ROUND_ROBIN_PLAY_TYPE` 84, `RoundRobinOvertimeLLoss` 89, `TXT_LEAGUETYPE` 115/222, `PlayoffPrelimSeriesLength` 118, `PlayoffSeedingType` 119, `DynastyStartPoint` 123, `BaseLeague` 125; new rows 281–285 (`TOURNAMENT_ELIMINATION_*`), 372/373 (expansion) | CSV; `setting.default_value == contextsetting[−2]` on every Phase 1 row | CSV, incl. `is_visible` | explicit; tie-break vocabulary **shifted** (§1.7) | observed; NHL 19 patch DBs not checked for CSV changes (the patch archives were not opened) |
| **Xbox 360 NHL 10–15, Legacy** | `default.xex` static descriptor table (32-byte BE records `{id, name*, style, min, max, default, size, labels*}`; 184/187/190/210/222/222/222 records at file offsets `0x13324f8` / `0x151c018` / `0x14c420` / `0x415b8` / `0x4efc0` / `0x4f478` / `0x50fb0`) | same FE names in the same order; `id` is **title-specific** (`PlayoffSeriesLength` 140/147/148/150/152/152/152; `SeasonTieBreak` 18 everywhere); record index drifts by 1–2 | executable only; defaults are the "compiled" ones (`SeasonTieBreak` 4, `PlayoffSeriesLength` 2/2/3 — Legacy has 2 for `SeasonTieBreak`) | **not located** (no DB tables; no file under `fe/` contains the identifiers; the label pointers address runtime-initialised memory) | not statically resolvable | observed table; per-context defaults unknown |
| **PS3 NHL 10–15, Legacy** | by reference to the Xbox executable (PS3 executables encrypted; PS3 DBs carry no catalogue tables — 29-metadata scan, handoff 21) | — | — | — | — | by-reference |
| **Legacy Recompiled** | by reference to Xbox Legacy (same DB; its executable is the recompiled one and was not inspected) | — | — | — | — | by-reference |
| **Xbox 360 / PS3 NHL 07, 08, 09** | **none found**: none of the FE setting names or label keys exists in the three executables; NHL 09/10 mode saves carry an opaque `leaguerules` string (handoff 21) | unknown | unknown | unknown | unknown | gap |

Per-setting detail (ids, min/max/default per title, per-context rows, choice lists) is in the map JSON.
Shared parsing: NHL 16 and NHL 17–21 need two CSV schema variants; NHL 15 the DB reader; gen7 the executable
table reader (read-only). Title-specific meanings that must not be shared: the tie-break code vocabulary
(15/16 vs 17+), `StartingRosters` (3 vs 2 choices), `NumberOfTeams` (2 vs 4 choices, patch-dependent on NHL 15),
`BaseLeague` (11-entry array vs size 13–14), every `id` on gen7.

## 4. Selected-value serialization — what the static specimens support, and the minimal card

- **No committed specimen isolates a chosen value.** The Franchise/Season saves carry `leaguestates` integers
  whose coincidences with defaults are listed in §1.8; nothing distinguishes "setting code" from "resolved
  value", and the Season specimens (a context with no stored defaults) show the same numbers.
- **Profile files.** gen7 ships `fe/profile/defaultprofile.dat` (Legacy: 1 365 980 B, `35f1b1ff…`) — a
  container whose first payload (offset 16) is a **132-byte TDB with one table** (`exhibitiondlcnew`, 1 row) and
  whose remainder is opaque (two zlib streams at 977 101 / 994 999, no further `DB` header). The committed
  `PROFILE` saves (v5 containers, unsupported by `tdb-savedata`) were not parsed. The chosen-value store is
  **not located**; "profile save", "mode save only" and "a data file outside the DB" remain open hypotheses.
- **Static consumers.** The executable schema list (§1.3) binds the DB tables by name; the descriptor table
  (§1.7) carries defaults too; neither the patch changes (§1.6) nor the stock correspondences (§1.8) establish
  that the game consumes a row, and neither says where a selection is written.

**Card P1BS-1 — one setting at a time (user execution only; not authorised here).** Title: NHL 15 PS4 (the only
title where the default is DB data and the DB member edit is same-length). All treatments on copies; restore
both archive copies from their recorded hashes afterwards. Record, for every run: which copy was edited, the CRC
validity of the treatment, the creation screen's displayed option *before touching it*, and the extracted day-1
save (`save_db.py`, `leaguestates` types 29–34, 41–43, 46–49 and the full type list).

- **Controls first.** C0: stock install, fresh profile (or a profile that has never opened the Franchise creation
  screen), Franchise creation → record the displayed options and a day-1 save. C1: same with the user's existing
  profile — *(rev. 3)* if C0 and C1 differ, that **supports a profile-associated difference**, not proof of a particular
  persisted-choice mechanism: treat it as a control result that later treatments must match (same profile state),
  not as a located setting store; every later verdict is conditional on the fresh-profile control.
- **Treatment A (context row).** In the copy the game is believed to read (patch first), set
  `stocksettingdefaults (context 3, legacyid 72).default` 3 → 2; nothing else. Fresh profile, Franchise creation
  without touching the series-length option, day-1 save.
- **Treatment B (global row; separate run, stock context row).** Same on `stocksettings (72).default` 3 → 2.
- **Treatment C (second, distinct setting; separate run).** `stocksettingdefaults (3, 71).default` 1 → 0
  (`SeasonOvertimeLoss` 1 Point → 0 Points); compare `leaguestates` types 41–43 and 46–49 against C0.

Bounded verdicts (each holds for that placement, profile state and title only):
- *Displayed option changed to Best of 5* under A or B → supports "the creation screen offers this stored
  default at this placement" for the edited row; A-changed/B-unchanged (or the reverse) is a precedence
  observation for this one setting, not a rule.
- *Displayed option unchanged* → **does not exclude** the DB: the other copy may be read, a persisted profile
  choice may mask it, the executable table may be preferred for this row, or the treatment may have been
  rejected — next control: repeat on the other copy, then with the fresh-profile C0, then verify the treatment
  bytes were loaded (an unrelated visible edit in the same file).
- *A `leaguestates` type changes by exactly the treatment delta* (29–34 to 2 in A/B; a 41–49 type to 0 in C) →
  a **candidate association** between that type and that setting for that mode, not a serialization binding:
  different encodings, deferred writes at a later phase, or a resolved value that merely coincides remain
  possible — next discriminating control: a third treatment value (e.g. code 1) on the same setting; if the
  type follows the code a second time, the association is upgraded to *observed correspondence*, still short of
  a traced binding.
- *No `leaguestates` type changes* → **does not exclude** save-side storage: the value may be serialized in
  another table, another encoding, or only after a game is played — next control: play one game, save, repeat
  the comparison; then diff the full save, not only `leaguestates`.

Nothing about edited rules *taking effect in play* is claimed by any treatment; Treatment C exists so that a
selected-value binding is never proposed from a single setting.

## 5. Operations — supported / refused / unknown

| Operation | Status | Basis |
|---|---|---|
| Read and display the NHL 15 PS4 catalogue (settings, per-context defaults, screens/groups by lineage name, dependencies) with raw codes and inferred labels | **supportable** (read-only) after the four-site decoding correction (P1-BS-1) | §1 |
| Read and display the NHL 16–21 catalogue from the CSV members, with explicit labels on 17–21 | supportable now (read-only; chunkzip decode exists) | §3 |
| Read the gen7 executable table (ranges/defaults per title) | supportable, read-only, executable-derived (no DB path) | §3 |
| Edit one **stored** league/competition default (global or one existing context row) on NHL 15 PS4, eligible rows of §2.1 only | **proposed contract** (§2, not yet accepted); what the game offers/uses is unknown (card P1BS-1) | §2 |
| Same on NHL 16–21 | not offered until the native-framed chunkzip encoder and the text-member API route exist (dependencies D1/D2 of [overnight-2026-09-19 handoff 01](../overnight-2026-09-19/01-ps4-settings-csv-defaults-parity-and-packaging.md), shared with P1-F); the per-title CSV contract is defined there *(rev. 3)* | §2.7, 09-19/01 |
| Same on gen7 / PS3 / Recompiled | not offered — no DB or data-file representation was found in the inspected sources (executable table on 10–Legacy; Legacy loose `fe/` tree searched; archives not content-searched; PS3 executables not inspected); dependency: a data-side representation or an executable-backed route with its own evidence | §3 |
| Edit ranges (`min/max/size`), `id`, dependencies, contexts, groups | refused (unverifiable without a consumer; growth) | §2.7 |
| Add a defaults row for a context that has none (e.g. Season/Playoff creation) | refused (table growth; precedence unknown) | §2.2 |
| Edit the value a mode *uses* (chosen value) | unknown location — research-blocked, card P1BS-1 | §4 |
| NHL 07–09 anything | unknown (no catalogue found in the executables or searched loose files) | §3 |

## 6. Smallest implementation packages (research-derived; nothing dispatched)

1. **P1-BS-1 — decoding correction (small, `tdb-defs`).** Add the four NHL 15 PS4 sites to
   `OFFSET_BINARY_SITES` (or introduce a per-title site list) with a regression test on the anchor
   (`stocksettings` size-consistency 213/223; `subgroupparent` raw 37 → 36); open a separate question on the
   general rule for negative-rangelow fields.
2. **P1-BS-2 — catalogue reader + card (medium).** NHL 15 PS4: decode the five tables, join, present the Phase 1
   settings with lineage names, labels (inferred pill), per-context defaults and dependencies; NHL 16–21: parse
   the two CSV schema variants out of `inxp_00.big`; gen7: read the executable table (opt-in, read-only).
3. **P1-BS-3 — NHL 15 PS4 default edit (medium).** The §2 contract on the existing same-length DB member routes,
   pending review wording as in §2 (6), undo, copied-output verification, placement disclosure.
4. **P1-BS-4 — NHL 16–21 default edit.** After P1-F (chunkzip repack); text-field replacement with a CSV
   round-trip proof.

## 7. Evidence-specific test cases (for the packages above)

- **T1 decoding (P1-BS-1).** On `ae9d1d56…` and `abe36f58…` through the real parser: `stocksettings` rows decode
  with `size == max − min + 1` on exactly the 213 rows listed in the JSON; `PlayerAdvantage.min == −7`;
  `id(PlayoffSeriesLength) == 152` on legacyids 72/86/116; `subgroupparent` values ∈ {−1, 36, 74, 75};
  `subgrouporder` ∈ {−1, 1..6}; raw → logical → raw identity on all four sites for every raw value 0..2^w−1;
  `leaguesimpopups.id` (`qbPU.CkjP`, 14-bit, 0..16383) unchanged; the two existing offset-binary sites and an
  unrelated two's-complement slider unchanged.
- **T2 joins.** Reproduce §1.3 exactly (dangling 236 ×4; contexts 49–52 without rows; group 36/74/75 without
  members; duplicate (80, 94); 25 dependencies).
- **T3 patch shape.** On `abe36f58…`: 223/379/201/704/25 rows; legacyid 82 → id 186, max 3, size 4; six
  context-9 defaults at records 193–198.
- **T4 single-cell edit round trip (NHL 15 PS4).** Edit `(3, 72)` 3 → 2 on a temp copy: whole-file diff = the
  selected field's bits only (mask compare inside the 4-byte record: `legacyid` and `context` bits unchanged) +
  the individually listed checksum offsets; re-decode equals source except that cell; the §2.4 stock anomalies
  are reported unchanged; undo restores byte identity; the archive-member route keeps member length and every
  other member's hash. Repeat on the patch DB (`abe36f58…`) and on a copy that already carries the first edit
  (second target `(3, 71)` 1 → 0) — the second edit is eligible and the first cell is preserved.
- **T5 refusals.** `(37, 72)` (no row) refused with the growth reason and no precedence claim; code 5 for
  `PlayoffSeriesLength` refused (max 4); a global code > 7 refused (3-bit column); excluded rows 74, 82, 122, 125,
  115 refused by name with their §2.1 reason; a synthetic duplicate `(3, 72)` key refused as ambiguous; a
  synthetic `stocksettings` row with `size != max − min + 1` refused; an NHL 16 target refused with the
  repack dependency.
- **T6 CSV catalogues.** NHL 16 `setting.csv` id ↔ NHL 15 `legacyid` on all 37 Phase 1 rows (36 same FE name); NHL 17–21 26 rows present with the same id and FE name; NHL 17
  global default from context −2; NHL 18–21 `setting.default_value == contextsetting[−2].default_value` on
  every Phase 1 row; `DYNASTY_PLAYOFFS_LENGTH` choice values 0–4 with the listed keys.
- **T7 labels.** NHL 15 eboot `37df821a…`: record 72's labels pointer resolves to
  `[SingleKnockout, BestOf3, BestOf5, BestOf7, Authentic]`; record 68's to the six-entry tie-break array.
- **T8 gen7.** Legacy image `5a720c17…`: table at `0x50fb0`, 222 records, record 72 = `PlayoffSeriesLength`
  id 152 min 0 max 4 default 2 size 5; NHL 10 image: record 70, id 140.

## 8. Reproduce

```
python probes/p1bs_catalogue.py anchors/ps4/nhl15/databases/nhl_game.db anchors/ps4/nhl15/databases/nhl_game-meta.xml <out.json>
python probes/eb_list.py "H:\Emulators\ShadPS4\Games\CUSA00561\databigs\patch_inxp00.big" --extract nhl/data/common/gamemodes/database/t3db/nhl_game.db <tmp.db>
python probes/p1bs_eboot_setting_table.py "H:\Emulators\ShadPS4\Games\CUSA00561\eboot.bin" 3d60080 48 232 --json <out.json>
python probes/p1bs_eboot_runs.py <eboot> --window 3bef000:3bf1000 --window 3c0b800:3c0c400 --window 3d5f600:3d60600 --around stocksettings
python probes/p1bs_ps4_xtitle.py "H:\Emulators\ShadPS4\Games\CUSA18182\eboot.bin"
python probes/xex_image.py "<default.xex>" <image.bin> && python probes/p1bs_gen7_setting_table.py <image.bin>
python probes/p1bs_csv_catalogue.py <extracted setting_csv root> <out.json>   # extraction: eb_list.parse + ps4_leaguelogos.chunkzip on the setting_csv members
python probes/p1bs_build_map.py <scratch> <out dir>
```

## 9. Unknowns (explicit)

1. Consumer code: nothing traces which copy (DB vs executable table on NHL 15) the screens read, nor
   global-vs-context precedence.
2. Where the chosen value is persisted (profile container v5 / PS4 `Profile0` / mode save) — card P1BS-1.
3. `leaguestates` type ↔ setting binding (candidates only).
4. `default` value-vs-index for `min ≠ 0` rows (moot for Phase 1).
5. `BaseLeague` array (11) vs size 14; `NumberOfTeams` 122 size 5 vs 4 labels; `PuckShadow` size 3 vs 5 labels.
6. gen7 per-context defaults and label arrays (runtime-initialised); NHL 07–09 settings system.
7. Whether `tdb-defs`' two's-complement default is wrong for *all* EA negative-rangelow fields.
8. *(closed rev. 3)* NHL 19 patch archives (01.60 install: `patch_inxp_51–56`, `patch_game_91–96`) carry no `setting_csv`
   member; the real NHL 20 install (`CUSA15439`) is the census source; one stored copy per title —
   [overnight-2026-09-19 handoff 01](../overnight-2026-09-19/01-ps4-settings-csv-defaults-parity-and-packaging.md) §1.
9. Which of the two NHL 15 PS4 DB copies (base / patch) the installed game reads (atlas Q11).
10. Whether any creation screen shows a stored catalogue default at all (card P1BS-1).

## 10. Changes from revision 1 (P1-BS-R1)

Executable agreement stated as 222/223 (`id`), 222/223 (`min`), 186/223 (all six) with every exception
enumerated (§1.2, §1.7); the patch diff and the stock correspondences no longer offered as evidence of
consumption (§1.6, §4); precedence sentence lists the competing mechanisms without choosing (§1.4); the contract
rewritten (§2): enumerated eligible rows with exclusions and reasons, target identity and ambiguity rules,
eligibility of the opened file with stock anomalies preserved and only introduced defects rejected, second-edit
eligibility, bit-level preservation inside the record with individually accounted checksum offsets, "stored
catalogue default" wording, no "global applies"/"default offered" claims; card P1BS-1 rewritten with profile and
placement controls, a second distinct treatment, and bounded positive/unchanged verdicts that never exclude a
storage hypothesis (§4); §1.9 four-site metadata applicability (55 metadata files + patch metadata; `CkjP`
collision; narrowest decoder scope); §5/§7/§9 reconciled. The data files are unchanged.

## 11. Changes from revision 2 (P1-BS-CSV, 2026-09-19 — wording only, no measured data changed)

§2.1: the eligible table is stated as 21 distinct legacy ids across 20 displayed rows, ids governing. §2.4: prior
reviewed edits are baseline, not an exemption — re-validation of target identity and bounds on every edit, no old
review ticket needed to reopen, previously edited cells not blanket-skipped. §2.5: header CRCs (`+0x24`, `0x14`) must
stay byte-identical on a same-length cell edit; only the covering body CRC (next table's `prior_crc` or the EOF CRC)
may change, at an independently derived and verified offset; unrelated invalid checksums untouched. §1.9: `HwRX.NVBd`
kept as structurally inferred, distinct from the executable-witnessed sites; table-plus-field keying, declaration
shape and the `qbPU.CkjP` regression required; no universal-verification claim. §4: a fresh/existing-profile
difference is a profile-associated control result, not a located store. §2.7 / §5 / §9 item 8: NHL 16–21 routed to
[overnight-2026-09-19 handoff 01](../overnight-2026-09-19/01-ps4-settings-csv-defaults-parity-and-packaging.md)
(NHL 19 patch / NHL 20 source gaps closed there). `data/p1bs-nhl15-ps4-settings-catalogue.json` `readings.value`
corrected from "every `id` match[es] the executable" to 222/223 (`id`), 222/223 (`min`), 186/223 (all six) with the
exceptions named; no numeric field of the JSON changed.
