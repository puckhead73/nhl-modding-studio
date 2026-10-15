# Handoff P1-B — competition rules, season/tournament/playoff settings and mode initialization (all anchor families)

Research lane, 18 September 2026. **Revision 3 (P1-B-R2)** after the coordinator reviews
[p1b-review-and-r1-research-dispatch](../../validation/p1b-review-and-r1-research-dispatch.md) and
[p1b-r1-review-and-r2-dispatch](../../validation/p1b-r1-review-and-r2-dispatch.md): observations,
inferred mechanisms and unresolved edited-input behaviour are now separated everywhere; specimen coverage is
reconciled (46 saves processed, 44 with a populated schedule comparison, 2 without a regular-season copy); absence claims are narrowed to
what was inspected; the rule-editing path is kept in scope as research-blocked; the cards are rewritten as tests
of inferred behaviour with controls and verdicts limited to the observations; residual evidence is complete (R2). Claim `p1b-competition-rules-and-mode-init`. Consumer: developer Phase 1.2
and Phase 2 (teams), C6.

Machine-readable:
- [`data/p1b-schedule-copy-comparison.json`](data/p1b-schedule-copy-comparison.json) — **the authority for §2**:
  per specimen and tier, the exact candidate source table(s), compared fields, row classes, the translation
  applied to mode team ids and how it was derived, multiset intersection, unmatched tuples on both sides with
  multiplicity, positional/`index` equality, All-Star residual through the team mapping (`p1b_schedule_compare.py`).
- [`data/competition-rules-inventory.json`](data/competition-rules-inventory.json) — source identities (29 main
  DBs, 46 saves with inner-DB hashes), per-title schemas of the mode/rules tables, tournament and expansion
  schedule digests, decoded `leaguestates` vectors, `leaguetasks` calendars with offsets, executable identifier
  sets (`p1b_mode_init_survey.py`; its own `schedule_copy` blocks are superseded by the comparison file — see
  the `notes` key). Probes: `save_db.py`, `p1b_lib.py`, `p1b_sources.py`.

No application code, defs or bindings changed; no launches; no game bytes stored (digests, row counts,
decoded small integers only). No consumer code was traced anywhere in this handoff.

**Evidence vocabulary (used in every table):** **observed** = read from anchors, committed saves or installed
executables; **inferred** = a reading of observed values or correspondences whose producing mechanism was not
traced; **unresolved** = behaviour on edited inputs, which no static comparison can establish.

## Plain-language summary

The one place where a competition rule turned up as ordinary, editable database data is the **NHL 15 PS4**
main database: a settings catalogue (`stocksettings` with per-context `stocksettingdefaults`) that lists the
mode-setup options — series length, tie-breaks, overtime-loss point, salary cap on/off, All-Star game, season
length, roster start — with their allowed ranges and **default** values. Those are defaults, not the value a
player picked when starting a mode; where the picked value is stored is still not located. On the other 28
main databases the same scan (every table and field name in the metadata) found no representation of these
rules; the leads there are option identifiers in the executables and a key/value state store inside the mode
saves whose numbers look like those options. The rule-editing path therefore stays in scope and is
*research-blocked*, with the catalogue as the next lead.

On initialization: 46 committed day-1 Season/Franchise/GM saves were processed (43 through metadata-named
tables, 3 NHL 07/08 through a raw table id); **44 hold a regular-season schedule copy** and each of those
**corresponds** to its stock source table — the same (date, home, away) tuples under the mode's own team
numbering — with three exception classes (the All-Star row's home/away swapped on NHL 13–15, NHL 08's
All-Star numbering, and a 31st-slot placeholder in the NHL 18 GM save); **2** (NHL 10 Franchise/GM) hold no
regular-season copy, so nothing is claimed for them. Franchise saves also carry rows that exist in no source
table (inferred: a generated preseason). The mode's calendar dates sit at fixed distances from the copied
schedule's first and last game. All of this is correspondence in *stock* data; that the game reads the source
table and computes the calendar from it is the natural inference and remains untested, and whether an *edited*
schedule reaches a new mode is exactly what the runtime cards are for. The tournament tables read like phase
templates with placeholder slots — an inferred reading, not a proven format.

## 0. Sources and coverage (identities in the JSON `sources` blocks)

- **Main databases (29)**: the 27 main-DB anchors + NHL 07 (`NHL07NG.db`), the real NHL 20 install DB
  (`E:\nhl-studio-fixtures\…\nhl20-ps4-cusa15439\inxp_00.big\nhl_game.db`, `1e200478…`) and NHL 19 patch 1.56.
  `anchors/ps4/nhl20` is listed as `ps4-nhl20-anchor(=nhl19-base)` and never used as NHL 20 evidence.
- **Mode saves (46)**: every committed SEASON / DYNASTY / BEAGM specimen under `anchors/*/*/savefiles` (Xbox
  360 07–Legacy, PS3 08–Legacy, PS4 15–21). Inner TDB extracted read-only from each container style (raw body
  at 0x1c/0x20/0x2c or 0x8d9bc on NHL 08, zlib at 0x30 on 14/15/Legacy incl. PS3 `DATA0000–3` multi-part,
  PS4 LZ4 frame — NHL 17+ nest `[01][frame]` sub-blocks — NHL 15 PS4 `chunkzip`). **Processing coverage: 46/46 specimens
  opened and run through the comparison (43 via metadata-named tables, 3 — NHL 07 Xbox, NHL 08 Xbox/PS3 — via
  the raw table `ULcv`). Comparison outcome: 44 with a populated regular-season copy and a recorded
  disposition; 2 (NHL 10 Xbox Franchise, NHL 10 PS3 Be-A-GM) with no regular-season copy** (one `round = 5`
  row; disposition "opened; no regular-season copy in specimen" — nothing is claimed for NHL 10 Franchise).
  Each specimen carries an explicit `disposition` string in the comparison JSON. Extraction is
  measurement only; it does not extend `tdb-savedata`.
- **Executables**: the seven PS4 `eboot.bin` (hashes = handoff 17 witnesses) for identifier sets; the user's
  decompressed Legacy Xbox image for the gen7 counterpart. Other gen7 executables were not decompressed.

## 1. Where each rule appears — inspected schemas, exact search, and what was found (observed unless marked)

**Search method.** For all 29 main DBs: every table name and every field name from the `-meta.xml` files was
scanned for `point`, `tie`, `overtime`, `playoff`, `round`, `series`, `season`, `qualif`, `seed`, `bracket`,
`cap`, `salary`, `calendar`, `deadline`, `date`, `phase`, `period`, `rule`, `setting`, `state`, `mode`;
matching tables were opened and their rows read. This finds *named* representations only: a generic key/value
table, an opaque blob or an unnamed field could still carry a rule — so every "absent" below reads **"no
identified main-DB representation"**, never "nowhere". The mode-side tables (`leaguestates`,
`leaguegamemodesvars`, `leaguerules`, `leaguetweaks`) *are* generic key/value or opaque stores and are listed
as such. The scan's rule-relevant hits are recorded in the inventory (`field_name_scan`); one of them is new
in revision 2: **the NHL 15 PS4 main DB carries a mode-setup settings catalogue** — `stocksettings` (223 rows:
`id, stringid, legacyid, default, min, max, size, style`), `stocksettingdefaults` (373 rows: `legacyid,
context, default`), `stocksettingcontext`, `stocksettinggroups`, `stocksettingdependencies` — with the rule
settings among them (`SeasonTieBreak` default 2, `SeasonOvertimeLoss` 1, `NHLSalaryCap` 1, `PlayoffSeriesLength` 3,
`PlayoffTieBreak` 0, `PlayoffPrelimSeriesLength` 3, `PlayoffSeedingType` 0, `HomeAwayType` 0, `TrophyType`,
`NumberOfTeams`, `NumberOfRoundRobins`, `RoundRobinTieBreak` 4, `TXT_DYNASTY_LENGTH`, `AllStarGame`,
`StartingRosters`, `BaseLeague`, …; full dump in the JSON `settings_catalogue_ps4_nhl15`). It is a catalogue of
*identifiers, ranges and defaults per context* — an identified main-DB representation on **that one title**
(no other of the 29 metadata files declares these tables); the value a player chose is still not located.
NHL 07/08 declare an `exhibitionstates` (`value/index/type`, 0 rows) and 07–10 `exhibitionrules`/
`exhibitiontweaks` (0 rows) in the main DB — the main-DB twins of the mode-side stores, empty in stock.

| Concern | Main DB — identified representation | Mode save — observed | Executable / settings — identifier leads (observed strings) | Evidence |
|---|---|---|---|---|
| **Season length / games per team** | the per-league `*schedule` tables: row count and per-slot game count (handoff 12 §1.3/1.4); NHL 07's four length variants | schedule copy (§2); `leaguestates` carries the constant `82` in **every tier**, including the AHL tier whose copied schedule has 76/68 games (so it is not a per-league game count) | gen7 FE names `TXT_DYNASTY_LENGTH`, `NumberOfRoundRobins`, `NumberOfTeams`; PS4 `DYNASTY_LENGTH`, `SEASON_MODE_LEAGUE_INDEX`, `MUD_LENGTH` | observed tables and copies; the `82` state's role is inferred |
| **Points / tie-break / OT loss** | no identified representation of the *rule values*; **NHL 15 PS4 only:** the settings catalogue's `TieBreak`/`SeasonTieBreak`/`PlayoffTieBreak`/`SeasonOvertimeLoss`/`RoundRobinTieBreak` rows (identifier, range, default per context) | small integers (0–3) in `leaguestates` next to the setting-like block (§3.1) — unlabelled | gen7 `SeasonTieBreak`, `PlayoffTieBreak`, `RoundRobinTieBreak`, `SeasonOvertimeLoss`, `RoundRobinOvertimeLLoss`, `TieBreak`; PS4 `DYNASTY_SEASON_TIE_BREAK`, `DYNASTY_PLAYOFFS_TIE_BREAK`, `DYNASTY_SEASON_OVERTIME_LOSS`, `GAME_MODES_FINALS_TIE_BREAK`, `TOURNAMENT_ROUND_ROBIN_TIE_BREAK`, `PLAYOFF_ELIMINATION_TIE_BREAK`, value ids `TIE_BREAK_NO_OT / _SINGLE_OT_5V5 / _SINGLE_OT_4V4 / _OT_3V3_THEN_SHOOTOUT / _5MIN_THEN_SHOOTOUT / _CONTINUOUS_OT / _SHOOTOUT` (17+); PS4 UI-contract flags `UseTies / UseOvertimeLosses / UseOvertimeWins` | identifiers observed; serialization location of the chosen values **not identified** |
| **Qualification / playoff teams, rounds, series length** | no identified representation of the rule values; **NHL 15 PS4 only:** catalogue rows `PlayoffSeriesLength` (default 3, range 0–4), `PlayoffPrelimSeriesLength`, `PlayoffSeedingType`, `HomeAwayType`, `NumberOfTeams`, `NumberOfRoundRobins`; `stockplayoffteamstats` (32 rows) and `stockseasonteamstats` (30; 62 on PS4 21) exist on every title — sized like the stock layout, contents are stat columns, no rule fields | `leaguestates` values `16`, `4`, six consecutive `3`s, `8` (§3.1, labels inferred); Franchise copies carry generated **preseason** rows, no playoff rows (§2) | gen7 `PlayoffSeriesLength`, `PlayoffPrelimSeriesLength`, `PlayoffSeedingType`, `HomeAwayType`, `TrophyType`; PS4 `PLAYOFF_NUMBER_TEAMS`, `GAME_MODES_NUMBER_PLAYOFF_TEAMS`, `PLAYOFF_SERIES_LENGTH`, `GAME_MODE_PLAYOFF_SERIES_LENGTH`, `PLAYOFF_ELIMINATION_SERIES_LENGTH`, `TOURNAMENT_ELIMINATION_*`, `PROAM_SERIES_LENGTH`, `BestOf3/5/7`, `GAMETYPE_PLAYOFF_ROUND1–4`, `PLAYOFF_TOTAL_ROUNDS` | identifiers observed; state labels inferred |
| **Divisions / conferences** | team-table group ids: NHL 08–11 local `conference` (1 bit) + `division` (3 bits); NHL 12–Legacy and PS4 `leaguegroup/conferencegroup/divisiongroup` (6 / 7 bits); NHL 07 on `stockteamtable` | `leagueteams` copies the same fields per copied team | group-name tables in the executables: Xbox 12–Legacy (handoff 12 §2), PS4 15–21 (handoff 17; ids 77/78 unknown); NHL 08–11 name lists without ids | observed; ids title-specific |
| **Tournament groups** | PS4 17+: `stockleagues` rows `WorldCup` (17: 8 teams, groups 44/45), `TXT_CHAMPIONS_LEAGUE` (18+: **8** stock teams, all group 64), `TXT_SPENGLER_CUP` (18+: 6 teams, groups 75/76); tables `wcschedule` (17), `championsschedule`, `spenglerschedule` (18+), `worldchampschedule`/`worldjuniorschedule` (20–21, 0 rows, capacity 100/60); gen7: no tournament tables (national teams are a league with the `TournamentFile/PoolA/PoolB/Playoffs` group block) | no tournament-mode save is committed | PS4 group ids 61–76 (handoff 17); strings `TXT_ROUND_OF_16/8`, `TXT_CHAMPIONS_LEAGUE_GROUP_STAGE_COMPLETE`, `TXT_SPENGLER_ADDITIONAL_GAMES_SCHEDULED`, `TOURNAMENT_MODE_LEAGUE_INDEX` | observed |
| **Calendar / phases** | no identified representation (`fe/nhlsched.bin` unidentified) | `leaguetasks` rows with Excel-serial dates (§3.2); NHL 15 PS4 `leaguescheduleperiods` declared, **empty** in the only NHL 15 GM specimen | PS4 `DAY_*` special-day ids (per-title lists in the JSON: 12 on 15, growing to 19 on 20/21) | tables/ids observed; task-type ↔ day-id binding **not established** |
| **Mode state / initialization** | `leaguestates`, `leaguetasks`, `leaguegamemodesvars` (`team, key, value(16-bit, base −32768), isvalid, type`), `leaguescheduletable`, `leaguetempscheduletable`, NHL 15 PS4 `leaguescheduleperiods` — declared with 0 rows in every main DB (gen7 `max_records` 0; PS4 600–800 / 250–512 / 100 / 4000–4500 / 2700–4000) | populated per §2–§3; `leaguegamemodesvars` is **0 rows in all 46 specimens** | gen7 `BaseLeague`, `DynastyStartPoint`, `StartingRosters`, `NHLSalaryCap`, `AllStarGame`; PS4 `DYNASTY_STARTING_ROSTERS`, `DYNASTY_SALARYCAP`, `DYNASTY_ALLSTARGAME`, `FANTASY_DRAFT_SETTING`, `DRAFT_PICK_OWNERSHIP`, `AUTHENTIC_CAP_PENALTIES`, `DYNASTY_EXPANSION_DRAFT_FLOW`, `DYNASTY_EXPANSION_CUSTOM_TEAM_TYPE` (18+) | observed |
| **Salary cap / floor / contract limits** | no identified representation of the figures (player salaries are data; league figures are not named anywhere); **NHL 15 PS4 only:** catalogue row `NHLSalaryCap` (on/off default 1) | a `leaguestates` quintet whose values equal the published CBA figures of the season (Legacy `575000 / 14280000 / 925000 / 52800000 / 71400000`; NHL 21 `700000 / 16300000 / 925000 / 60200000 / 81500000`); `leaguegmdata` per-team `salary`, `salarynextyear`, `cbaexpenseyear0–24`, `retainedtradesalaryyear0–24` | `NHLMaxSalaryCap`, `NHLMinSalaryCap`, `LeagueSalaryCap`, `MaxSalary`, `MinSalary` (PS4 UI contract) | values observed; labels inferred by matching |
| **Game rules / sliders (not competition rules)** | no identified representation | NHL 09/10 `leaguerules` (`level` + opaque 255-byte string) and `leaguetweaks` — 2 rows each; absent from NHL 11 on | `RULES_SETTING`, `PERIOD_LENGTH`, `OFFSIDES`, `ICING`, … | observed; outside Phase 1 |

**What this does and does not say.** The identifiers are leads to *where a setting is chosen*, and the
save values are leads to *where a chosen value lands*; the serialization of the chosen values at rest
(profile/settings saves are binary and were not parsed; PS4 `Profile0` records are opaque here) and the code
that applies them were **not** located. "Only in settings saves or executable constants" is therefore not a
finding; it is one hypothesis among others (a data file outside the DB, an unnamed field, a packed blob).

## 2. Mode-local schedule copies — observed correspondence on 44 of 46 specimens (mechanism inferred; edited-input behaviour unresolved)

Per-specimen dispositions are in the JSON (`disposition`); nothing is reported as "compared" for the two
NHL 10 Franchise specimens.

Copy table: `leaguescheduletable`; NHL 11 Franchise: `leaguetempscheduletable` (the main copy table has 1
row); NHL 07/08: raw table `ULcv` (same field raw ids as the source schedule — `XpWK round, fdgg status,
iwsK day, pLKJ month, qEfv index, qkhY home, rOMv away`, NHL 08 adds a 1-bit `jjMx league`; no metadata name).
Compared fields: `(month, day, home, away)` after translating mode team ids; `round`, `status` and `index`
reported separately. The comparison JSON holds the **complete** unmatched-tuple lists on both sides with
multiplicity (no slicing since R2), an exhaustive residual characterisation per candidate (`residual_copy_ids_involved`,
`residual_source_slots_involved`, `every_copy_residual_involves_unmapped_id`) and a per-specimen `disposition`.

**Translation of mode team ids (observed on 09–21 through `leagueteams`; inferred on 07/08).** On every
specimen with a metadata-named `leagueteams` table, each `home`/`away` value is a `leagueteams.index`, and
`leagueteams.teamid` equals the source team's global `exhibitionteams.index`, whose rank within its league (by
`index`, handoff 12 §1.2) is the source slot. Applying that mapping — not a constant offset — reproduces the
source tuples; the constant "+30 / +31" of revision 1 is the *consequence* of the mode numbering (NHL clubs
0–29/30, AHL 30–59 or 31–61, All-Star squads last), not a rule of its own. NHL 07's copy keeps the source slots
(5-bit field); NHL 08's copy has no usable `leagueteams` metadata, so its tiers were separated by slot range and
the AHL rows read with an offset of 30 — the unmatched lists show that reading is exact except the All-Star
row (below).

| Result (per tier, round-0 rows) | Specimens | Detail from the JSON |
|---|---|---|
| **Exact multiset correspondence, no unmatched tuple either side** | Season 10, 11, 12, Legacy, PS4 16–21 (incl. the real NHL 20 DB); Franchise/GM 09 (NHL+AHL), 11 (temp table, NHL+AHL), 12, Legacy, PS4 16, 17, 19, 20, 21 (NHL+AHL); NHL 07 (`nhlschedulelongnoleap`, 1 231/1 231, positional and `index` identity too) | e.g. Legacy Season `nhlschedule` 1 231/1 231, unmatched 0/0, positional 1 231, `index` equal 1 231 |
| **Exception A — All-Star row with home/away swapped** (copy `EAS`(60) home / `WES`(61) away where the source has 31 home / 30 away): 1 230/1 231 with exactly one unmatched tuple on each side | NHL 13, 14, 15 Season and Franchise (Xbox and PS3), PS4 NHL 15 GM | `[[0,26,30,31],1]` copy vs `[[0,26,31,30],1]` source on NHL 13; same shape on 14/15 |
| **Exception B — NHL 08 All-Star row numbered 59/60** (after the 29 AHL clubs, as on NHL 09), which the slot-range grouping puts in the AHL tier | NHL 08 Xbox and PS3 | NHL tier 1 230/1 230 + source `[0,26,30,31]` unmatched; AHL tier 1 160/1 160 + copy `[0,26,29,30]` unmatched — the same row |
| **Exception C — 31-slot AHL copy** matching `ahl31schedule` (1 189/1 271) with 82 unmatched tuples on each side; **exhaustively**, every one of the 82 copy residuals has exactly one side equal to mode id 61 (unmapped: `leagueteams[61].teamid` 497, league 16 `CUSTOM_TEAMS`, empty `artabbr`) and every one of the 82 source residuals has exactly one side equal to slot 30 (`every_copy_residual_involves_unmapped_id: true`; residual source slots 0–30) — a placeholder row present in a non-expansion save; `ahlschedule` (14/1 116) and `ahl32schedule` (9/1 312) do not match | PS4 NHL 18 GM only | JSON tier `1` of `ps4-nhl18-BEAGM`, full 82+82 lists |
| **No regular-season copy in the specimen** (disposition "opened; no regular-season copy") | NHL 10 Xbox Franchise, NHL 10 PS3 GM (1 row, `round 5`; the NHL 10 Season specimens correspond exactly, 1 230/1 230) | not a correspondence result |

**Row classes and ordering (observed).** Season copies keep the source order and `index` (positional equality
= all rows; `index` equal = all rows). Franchise/GM copies (12–21) hold three classes: round-0 rows = the NHL
and AHL source tuples, **merged into one date-sorted `index` sequence** (positional equality drops to 219–430
because of interleaving; `index` equality 0); rows with `round 6` (gen7, PS4 15–17) / `round 8` (PS4 18+) —
60 on NHL 12/13, 98 on 14–17, 105 on 18–21, dated 19 Sept–6 Oct, low-numbered slots first, present in no
source table (**save-specific rows; "generated preseason" is an inferred label**); no other classes (no playoff
rows in any day-1 save). `status` is 0 everywhere; `round` is 0 on every round-0 row in copy and source.

**What is established and what is not.** *Observed:* a stock day-1 save's schedule table equals the stock
source table under the team mapping, with exceptions A–C. *Inferred:* that the game builds the copy by reading
the source table at initialization (the same tuples could be produced by any code path that agrees with the
stock data). *Unresolved:* whether an **edited** source table is reflected in a new mode; row-for-row identity
of fields not compared (`round`/`status` were compared and are 0; nothing else exists in a source row); what
happens with fewer or more source rows (revision 1's "the copy is row-driven" is withdrawn — it is a
hypothesis for card P1B-3); how the mode chooses `ahl31schedule` on NHL 18 (the setting `DYNASTY_EXPANSION_*`
is a lead only). Studio must not word any UI text as "edits reach the mode" on this evidence.

## 3. Mode state store and calendar (layouts observed; every label inferred)

### 3.1 `leaguestates` — one integer per `(type, tier)`

Layout observed on NHL 09–21: `type` 8 bits, `tier` 1 → 2 → 3 bits (09 / 11 / 12+), `key` 9–10 bits
(= `type × 6 + tier` on 12+; 09–11 pack differently), `value` 31 or 32 bits — on PS4 18–21 declared with
`rangelow = −2 147 483 648`, so the word decodes as `raw + rangelow`. Tier 0 = the NHL, 1 = AHL, 2–4 = the CHL
leagues in Franchise (12+). Type numbering is **title-specific** (57 types on NHL 09 … 161 on NHL 21) and no
type-name table was found in the PS4 eboots or the Legacy image. The identifications below rest only on value
signatures across titles and are **inferred**; Xbox and PS3 specimens of one title agree on every type except
a few save-specific ones (NHL 10/13 types 3/4 — bit-packed; 09 `59`, 11–12 `51`, 13 `96`, 14/15 `101`, Legacy
`53`/`103`), which are treated as runtime state, not rules.

| Value signature | gen7 type (10 / 11–13 / 14–15 / Legacy) | PS4 (15 / 16–17 / 18–21) | Inferred reading |
|---|---|---|---|
| Excel-serial date = copied first game −3 / −20 | 0 | 0 | current mode date (relation to the copy observed on 44 specimens) |
| 2009 … 2020 | 14 / 13 / 13 / 13 | 13 / 14 / 23 | season year |
| `500000, 11360000, 900000, 40800000, 56800000` … `700000, 16300000, 925000, 60200000, 81500000` | 16–20 / 15–19 / 15–19 / 15–19 | 15–19 / 16–20 / 25–29 | league minimum salary, individual maximum, entry-level maximum, floor, ceiling (equal to that season's published CBA figures) |
| 30 / 31 | 21 / 20 / 20 / 20 | 20 / 21 / 30 | club count of the tier |
| 82 in every tier | 22 / 23 / 23 / 23+24 | 23 / 24+25 / 33+34 | a season-length setting or constant |
| 16 | 35 / 35 / 35 / 37 | 35 / 38 / 40+56 | playoff team count |
| 4 | 24 / 24 / 24 / 26 | 79 / 27 / 38+49 | playoff round count |
| six consecutive 3s | 29–34 / 29–34 / 29–34 / 31–36 | … | per-round series-length codes; 3 is exactly the NHL 15 PS4 catalogue default of `PlayoffSeriesLength` (range 0–4) — the code's meaning (best-of-7?) is still unknown |
| 59+60 / 60+61 / 62+63 | 62+63 / 53+54 / 53+54 / 55+56 | 53+54 / 56+57 / 74+75 | the All-Star squads' mode ids (matches §2) |
| 998, 1000000, 2147483647, `0x03020100`-style byte quads | various | various | sentinels / packed small vectors — unidentified |
| second Excel-serial date ≈ All-Star row −1 | 102 / 104 | 103 / 106 | an event date (14–17 only) |

### 3.2 `leaguetasks` — dated events (relations observed; derivation inferred; names not bound)

`(tier, type, startdate, enddate, index = type × 6 + tier)`, 16-bit Excel-serial dates, 8–47 rows. Type
numbers are title-specific. In the stock day-1 saves the dates stand in fixed relations to the copied schedule:
mode start = first game −3 (Season) / −20 (Franchise; −17 on 12/13; −7 NHL 15 PS4); one task at last game
**−1** on every title and mode (the plain summary of revision 1 said "+1" — wrong; the body's −1 is the
measurement); tasks at last game −45 and −41 (Season; one at −43 in Franchise); Franchise markers at first
game −7/−6/−4; on titles with an All-Star row, tasks at that row's date −4 and −3; year-long spans to 30 Sept
/ 11 Sept; and fixed month/days (17, 19, 21, 22, 29 June; 30 Aug; 1 and 8 Sept; 30 Dec and 20 June on PS4
18+). **These are relations in stock data.** That the engine computes the dates from the copied schedule is
the natural inference and is *untested*; that a task at "last game −45" is the trade deadline, or the one at
"−1" the regular-season end, is a candidate binding by date coincidence only — no task type is named by any
string found, and the PS4 `DAY_*` ids were not linked to type numbers (card P1B-4).

### 3.3 Mode-setup settings (identifier leads; storage not located)

Identifiers observed as strings — gen7 (Legacy image, FE names): `BaseLeague`, `DynastyStartPoint`,
`HomeAwayType`, `PlayoffSeedingType`, `PlayoffPrelimSeriesLength`, `PlayoffSeriesLength`, `PlayoffTieBreak`,
`SeasonTieBreak`, `SeasonOvertimeLoss`, `NHLSalaryCap`, `AllStarGame`, `StartingRosters`, `TrophyType`,
`NumberOfTeams`, `NumberOfRoundRobins`, `RoundRobinTieBreak`, `RoundRobinOvertimeLLoss`, `TXT_DYNASTY_LENGTH`;
PS4 NHL 17–21 (uppercase option ids, 62–116 per title in the JSON): `DYNASTY_*`, `PLAYOFF_*`, `GAME_MODE(S)_*`,
`TOURNAMENT_*`, `SEASON_MODE_*`, `TIE_BREAK_*`; NHL 15/16 PS4 eboots do not carry the uppercase set — on NHL 15
PS4 the catalogue is the main-DB `stocksettings` table instead (same FE names as the gen7 image, with
`legacyid`s, ranges and per-context defaults). Where the chosen values are serialized (profile/settings save,
another data file, the mode save only) was **not** located, and no editing contract follows from the
identifier list; the NHL 15 defaults are the one place where a rule *default* is ordinary editable DB data.

## 4. Tournament tables and expansion placeholders (fields observed; phase/placeholder semantics inferred)

| Table | Titles | Rows / capacity | Observed contents | Inferred reading (not consumer-backed) | Binding status |
|---|---|---|---|---|---|
| `wcschedule` | PS4 NHL 17 | 17 / 17, 8 B, 5-bit slots | `round 0`: 12 rows (Sept 17–22, 0-indexed month 8) over slots 0–7; `round 3`: 2 rows and `round 4`: 3 rows, all `8 vs 8` | slots 0–7 = the 8 `WorldCup` league teams by rank (league 14; pools 44/45); slot 8 = a "to be determined" placeholder for the knockout rows | **inferred, read-only**: a size match (8 slots, 8 league-14 teams) is not evidence of *which* team is which slot — the slot order needs a discriminating source/consumer (or a save from a played World Cup, card P1B-7) before any team reassignment; rows with slot 8 not bindable |
| `championsschedule` | PS4 18–21 (+19 patch) | 125 / 155, 4 B (8 B on 21), 5-bit (6-bit on 21) | `round 0`: 96 rows, slots 0–31, every pairing inside a block of four consecutive slots, 6 games per slot, Aug 28 → Oct; `round 3`: 16 rows over slots 0–15; `4`: 8 rows / 0–7; `5`: 4 rows / 0–3; `6`: 1 row / 0–1 (Feb) | group stage of 8 groups of 4 (`slot // 4` ↔ groups A–H, group ids 64–71), then two-leg round of 16, quarter-, semi-finals and a final whose slots are qualifier positions; the 32 participants are not a stock league — `TXT_CHAMPIONS_LEAGUE` (league 9) has 8 stock teams (AAL VOJ CARD GROD MNSK GREN STAV TYCH, all group 64), the rest would come from the licensed leagues at runtime | **unbound**: no static slot → team rule; dates and `round` values are plain fields that can be browsed |
| `spenglerschedule` | PS4 18–21 | 11 / 41, 4 B, 4-bit | 6 league-20 teams (TRI HCD CANS group 75; UFA TPS HCAP group 76); `round 0`: 6 rows using slots 0–9; `round 4`: 2 rows; `5`: 2 rows (slots 10/11); `6`: 1 row; Dec 26–31 | a 6-team cup whose day-2/3 and knockout rows reference game results (slots 6–11 as placeholders) | unbound (12 slots for 6 teams) |
| `worldchampschedule`, `worldjuniorschedule` | PS4 20–21 | 0 / 100 and 0 / 60 | declared, empty; no league rows | reserved | absent |
| `nhl32schedule`, `ahl32schedule` | PS4 18–21 | 1 312 / 1 372–1 373, 6-bit | 32 slots × 82, `round 0`, no All-Star row | slot 31 = the expansion club; which row fills it is a runtime question (§2 Exception C shows a placeholder `leagueteams` row in a non-expansion save) | research unknown |
| `ahl31schedule` | PS4 NHL 18 | 1 271 / 1 332 | 31 × 82 | the table the NHL 18 GM specimen's AHL copy corresponds to (§2) | — |

Consequence for validators: `round ≠ 0` occurs in shipped source tables (tournament tables) and in Franchise
copies; a warning keyed on it must be table-aware. A browser may show `round`, dates and slots as observed
without asserting phase names: the date/round correspondence in this table does **not** authorise labels such
as "round of 16" or "final" in a UI — those are this handoff's inferred readings.

## 5. Per-operation table (statuses reconciled with the Phase 1 coverage matrix)

| Operation | Source candidates | Joins / identifiers | Limits | Validation | Bounded path | Status (matrix) / evidence |
|---|---|---|---|---|---|---|
| Edit a season's games within the stock row count | the league's `*schedule` table | rank-by-`index` slots | gen7 packed, PS4 +60 spare | 12 §1.4 invariants | P1-A (accepted offline) | impl+tested, accepted offline; whether the mode reflects an edit: **unresolved** (card P1B-1) |
| Change the season length (row count) | same | same | growth = capacity package; fewer rows: engine behaviour unknown | — | none yet | research unknown (card P1B-3) |
| Move a team between leagues with validated groups | team group fields + per-title group catalogue | gen7b `GlobalGroups`, 08–11 `LocalBits`, PS4 per-title catalogues (`defs/leagues/groups/ps4-*.toml`, P1-C) | PS4 ids 43–60 (17) / 74–76 (18) inferred, 77/78 unknown | membership validated through the title's group table **and** the row fit (an id merely existing is not sufficient parent validation) | — | **impl+tested** per the coverage matrix (X360/PS3/PS4/Recomp; NHL 07 blocked); PS4 naming delivered by P1-C and **under correction** ([p1c record](../../validation/p1c-ps4-competition-groups-2026-09-18.md), [P1-C-R1](../../validation/p1c-review-and-r1-dispatch.md)) — the matrix and that dispatch are the authority, not this handoff |
| Rename a division / conference | dictionary keys (`NHLDivisionA–D`, `WesternConference`, …) | LOC-W1 | shared keys across leagues | consumer disclosure | LOC-W1 (done) | impl+tested (rename with disclosure) |
| Edit points / tiebreak / OT-loss, playoff teams / rounds / series length, salary cap | rule *values*: no identified main-DB representation; rule *defaults*: NHL 15 PS4 `stocksettings`/`stocksettingdefaults` (§1); leads: setting identifiers (§3.3), mode-save `leaguestates` values (§3.1) | — | unknown until the serialization is found | — | **research-blocked** (next steps in §6) | in scope (Phase 1 "additional league data"); blocked, not optional |
| Tournament participants (CHL 32, WC 8, Spengler 6) | `stockleagues` + team `league` for the stock subset; runtime for the rest | group ids 44/45, 61–76 | knockout rows are placeholders (inferred); WC slot order unknown | none established (a size match is not a slot map) | browse observed fields, read-only | research unknown (cards P1B-6, P1B-7) |
| Tournament dates / rounds | the tournament tables | none | — | table-aware `round` handling | date browse/edit of observed fields | not impl |
| Expansion (32nd team) | `nhl32`/`ahl32` + a mode-side placeholder | slot 31 ↔ placeholder id | PS4 18+ | — | none in Phase 1 | research unknown (card P1B-5) |
| Calendar events | no identified representation; relations to the schedule observed (§3.2) | — | — | — | none | inferred derivation; unresolved on edits (card P1B-2) |

## 6. Rule-editing path — research-blocked, with the discriminating next work

The league rules stay in Phase 1 scope; their storage decides *dependencies*, not whether the user gets an
editor. Next discriminating steps, all read-only and bounded, none dispatched here:

0. **Read the NHL 15 PS4 catalogue as the model.** `stocksettings.legacyid` ↔ `stocksettingdefaults(context)`
   gives identifier, range and default per mode context on one title; check whether NHL 16–21 moved it to
   `service-defs`/`nhl/data` (string search for `PlayoffSeriesLength`/`legacyid`) and whether gen7 carries a
   binary twin (the Legacy image's FE name cluster is the lead). A default edit on NHL 15 PS4 is the smallest
   possible rule-editing experiment (card material, not dispatched).
1. **Locate the serialized settings.** Take two profile/settings saves that differ in exactly one mode-setup
   choice (e.g. `PlayoffSeriesLength` best-of-5 vs best-of-7; a user test with restoration) and diff them;
   repeat for the resulting Franchise saves' `leaguestates` to bind the changed type number. This turns the
   identifier leads into a location and a value encoding for that title.
2. **Check the non-DB data files** the executables reference for rule tables (PS4 `service-defs`, `nhl/data`
   JSON/ION packages; gen7 `fe/*.bin` including `nhlsched.bin`) by string search for the identifiers of §3.3.
3. **Only then** decide whether a settings-save writer (a new codec: v5 Xbox/PS3 containers and PS4 `Profile0`
   are unsupported by `tdb-savedata`) or a mode-save edit is the dependency. Neither is dispatched.

## 7. Developer package leads (research-derived; statuses per the matrix; nothing dispatched; P1-B is research, not an implementation claim)

1. **P1-B-1 — schedule card wording (small).** Suggested text: "In the stock saves we inspected (44 of 46
   Season/Franchise saves, NHL 07–21), the mode's schedule table holds the same games as this table under the
   mode's own team numbering. Whether an edited table reaches a new mode has not been tested." No promise about
   the calendar. Make the `round ≠ 0` warning table-aware.
2. **P1-B-2 — tournament tables: browse observed fields (small).** Show `round`, dates and slots as stored;
   no phase names, no team binding for any tournament table (the World Cup 8 = 8 size match is not a slot
   map — pending card P1B-7).
3. **P1-B-3 — PS4 group names.** Delivered by P1-C and under correction (P1-C-R1); nothing further from
   research beyond ids 77/78 and the NHL 17 mechanism (handoff 17).
4. **P1-B-4 — mode-save inspector (optional, later).** Read-only; needs read-side v5 and nested-LZ4 support in
   `tdb-savedata` (framing documented in `save_db.py`); shows copied schedule through `leagueteams`, task
   dates, and `leaguestates` values labelled *inferred*.
5. **Rule editor** — blocked pending §6 (the NHL 15 PS4 catalogue is the next research lead, not a writer
   dispatch); kept on the roadmap.

## 8. User test cards — tests of inferred behaviour, with controls and bounded verdicts (no launches in this package)

Common protocol: work on a copy of the loose main DB (gen7) or a Studio-built archive (PS4); keep the
untouched original as the **control**; before the run confirm the treatment file has **valid CRCs and container
structure** and sits at the **placement the launch actually reads** (loader precedence is itself unresolved on
several families, atlas Q11 — a treatment at an inactive placement tests nothing); after the run **extract the
resulting save with `save_db.py` and compare it to the control save with `p1b_schedule_compare.py`**; restore
the original files. A verdict names the hypotheses the observation supports and the alternatives it leaves.

- **P1B-1 — does an edited source table reach a new mode?** Treatment: one title, `nhlschedule`, swap the
  opener's home/away *and* move one mid-season game by 7 days with P1-A; Season on treatment and control.
  *Treatment save carries both edited tuples, control does not* → supports "the copy reads this table at this
  placement". *Both equal stock* → does **not** establish an alternate source: the alternatives are an
  inactive placement, a cached/patch copy, a rejected treatment (CRC/structure) or a different source — next
  control: confirm placement with an unrelated visible edit in the same file (e.g. a team abbreviation) before
  re-running. *Refusal/crash* → treatment validity first.
- **P1B-2 — do calendar dates follow the schedule?** Treatment A: move the last regular-season game 14 days
  later (nothing else); treatment B: move only the first game; Season on each and on control; compare
  `leaguetasks`. *A task date shifts with the treatment* → supports dependency on that date **under that
  setup**. *A task date does not shift* → not thereby a constant: it may depend on another input (the other
  end of the schedule, a setting, the All-Star row) — record it as "not moved by this treatment" only.
- **P1B-3 — row count: `current_records` vs spare rows.** Treatments on one gen7 title: (a)
  `current_records` reduced by N, trailing N records intact; (b) same reduction, trailing records zeroed;
  both with recomputed CRCs. *Copy has `rows − N` in both* → supports "the loader honours the current count";
  it does not by itself show *direct* use of `current_records` (a derived length or a validity scan would look
  the same) — next control: (c) `current_records` unchanged with the trailing N rows zeroed. *Full count in (a),
  garbage or refusal in (b)* → supports reading to `max`. *Refusal in both* → does **not** prove packed-count
  enforcement: treatment validity (CRC, structure, placement) must be excluded first, and a refusal can come
  from any consistency check. Report the copy's row count and the trailing tuples, never the season screen alone.
- **P1B-4 — task type ↔ named event (candidate bindings only).** PS4 21 Franchise: record the calendar's
  named days; extract the save; a type whose date coincides is a *candidate*. Controls: a Season save
  (different task set) and the P1B-2 shifted treatment. **If more than one type still coincides after the
  controls, the binding stays ambiguous** — report the candidate set, never a bound name.
- **P1B-5 — expansion slot.** PS4 18–21 Franchise with the expansion draft enabled; extract; report which
  `leagueteams` row holds id 31 / 61 and whether the copy corresponds to `nhl32schedule`/`ahl32schedule`.
  Outcome = the binding for that setup only.
- **P1B-6 — Champions League participants.** PS4 18+: start a CHL tournament; extract; list the 32
  `leagueteams` rows in slot order and compare round-0 pairings to `slot // 4`. Outcome = the list and order
  **for that setup**; the selection rule is a separate question (a second setup only if the first suggests a
  standings rule).
- **P1B-7 — World Cup slot order (NHL 17 PS4).** Start the World Cup mode; extract; list the 8 `leagueteams`
  rows in slot order against the round-0 pairings of `wcschedule`. Outcome = the slot ↔ team order for that
  setup; until then the table stays read-only and unbound.

## 9. Reproduce

```
python probes/p1b_mode_init_survey.py    docs/research/overnight-2026-09-18/data/competition-rules-inventory.json
python probes/p1b_schedule_compare.py    docs/research/overnight-2026-09-18/data/p1b-schedule-copy-comparison.json
python probes/save_db.py "<save dir or file>"      # lists the inner-DB records of one specimen
```

## 10. Changes from revisions 1 and 2

**Rev. 3 (P1-B-R2):** summary reconciled with the findings (NHL 15 PS4 catalogue leads; defaults vs
player-selected values; 46 processed / 44 with a populated comparison / 2 without a regular-season copy; three
exception classes; "generated preseason" labelled inferred); processing coverage separated from correspondence
outcomes with per-specimen dispositions; residual lists published complete (no slicing) with an exhaustive
characterisation — the NHL 18 GM claim now rests on all 82 + 82 tuples; card verdicts narrowed to the
observations with named alternatives and next controls (validity, placement); World Cup binding kept inferred
and read-only with a pending card (P1B-7); phase names not authorised for UI; P1-A accepted offline and P1-C
delivered-under-correction per the matrix/dispatch; P1-B-1 wording rewritten.

**Rev. 2 (P1-B-R1):** 
Causal wording ("copies row for row", "verified consumer", "edits reach the mode", "row-driven copy",
"calendar derived from the schedule") replaced by observed correspondence + inferred mechanism + unresolved
edited-input behaviour; plain summary "+1" corrected to "−1"; coverage reconciled (46 opened, 44 with a copy,
3 raw-table comparisons added, 2 without a copy); exceptions A–C recorded from the unmatched lists instead of a
universal rule; absence claims narrowed to "no identified main-DB representation" with the search method
stated; settings/salary leads no longer called the storage location; rule editing returned to scope as
research-blocked with next steps; group edits reconciled with the matrix (impl+tested, not "not impl");
tournament phase/placeholder readings marked inferred; cards rewritten with controls and bounded verdicts;
the field-name scan is now actually recorded (inventory `field_name_scan`) and surfaced the NHL 15 PS4
settings catalogue.
