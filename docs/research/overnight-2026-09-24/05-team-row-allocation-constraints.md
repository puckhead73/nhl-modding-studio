# Handoff R87 — team / league row allocation constraints (Leagues 1.2 creation / growth, Teams 2.2): measured on 25 anchors, **every gen7 main database is packed** — every table's capacity equals its row count (teams, arenas, kits, roster, schedules: headroom 0 on X360 / PS3 08–Legacy and on PS4 15–17's team table) — so a new team row cannot be allocated in place on those titles; PS4 18–21 carry headroom (5–100 team rows, 60–4 500 schedule rows) but their per-team stat tables (`stockseasonteamstats`, `stockplayoffteamstats`, career / record tables) are still full; the rows a new team would need per index-keyed table are measured (kits ≥ 1–2, goalie equipment 2, roster 20+, colours 1–2, one season and one playoff stats row for NHL clubs), together with free `index` values, arena coverage and the membership value spaces

Research lane, 24 September 2026, 05:47 → 05:52 MDT (clock corrected against file times) (brief item R4, League scope: "creation / growth, tournament participants, group
mappings and schedule row capacity: enumerate necessary references and allocation constraints"). Claim `research-r87-team-row-allocation`.
Read-only over `anchors/`. Nothing game-verified; no writer proposed; the engine's ability to grow a packed TDB table is a developer /
EDIT-MODEL question this handoff does not answer.

Machine-readable: [`data/r87-team-row-allocation.json`](data/r87-team-row-allocation.json). Probe `r87_team_row_allocation.py` (≈ 1 min).

## Plain-language summary

Adding a team is not one row: the roster, the kit table, the goalie-equipment table, the colours and the season and playoff stat
tables all carry rows per team, and each table has a fixed number of record slots in the database file. On every Xbox 360 and
PlayStation 3 title the shipped database uses every slot of every table — there is no spare row anywhere, not for a team, an arena, a
kit or a schedule game — so "create a team" on those titles means growing the database file, which is a different (engine) question
from editing rows. The PlayStation 4 databases from NHL 18 on have spare slots in most tables (five to a hundred team rows, sixty
schedule games per league), but their per-team statistics tables are full too. The free identity numbers exist everywhere (four on
Legacy, hundreds on PS4 21); the numbers are not the constraint, the slots are.

## 1. Capacity versus rows (`max_records` from each table header; headroom = capacity − rows)

| Anchors | `exhibitionteams` rows / capacity | free `index` values (declared range) | `exhibitionarena` rows / capacity | kits `stockteamjerseys` | roster `exhibitionrostertable` | schedules (per league table) |
|---|---|---|---|---|---|---|
| X360 / PS3 NHL 08 | 138 / 138 | 118 (0–255) | 138 / 138 | (no table) | 2 525 / 2 525 | headroom 0 |
| X360 / PS3 NHL 09 | 188 / 188 | 68 | 188 / 188 | (no table) | full | 0 |
| X360 / PS3 NHL 10 | 182 / 182 | 74 | 182 / 182 | (no table) | 3 719 / 3 719 | 0 |
| X360 / PS3 NHL 11 / 12 / 13 | 242 / 243 / 248, full | 14 / 13 / 8 | full | full | full | 0 |
| X360 / PS3 NHL 14 / 15 | 246 / 246 | 10 | 177–178, full | full | full | 0 |
| X360 / PS3 Legacy | 252 / 252 | 4 | 187 / 187 | 703 / 703 | 5 532 / 5 532 | 0 (19 schedule tables) |
| PS4 15 | 246 / 246 | 10 | 178 / 183 | 659 / 755 | 5 523 / 10 619 | 60 per league table; `leaguescheduletable` 0 / 4 000 |
| PS4 16 | 252 / 252 | 4 | 187 / 192 | 709 / 755 | 5 347 headroom | 60 |
| PS4 17 | 256 / 256 | 256 (0–511) | 256 / 261 | 70 headroom | 5 284 headroom | 60 |
| PS4 18 | 296 / 301 | 216 | 303 / 308 | 105 headroom | 6 129 headroom | 30–60 |
| PS4 19 / 20 | 419 / 519 | 605 (0–1 023) | 426 / 526 | 505 headroom | 7 239 headroom | 30–60; `leaguescheduletable` 4 500 |
| PS4 21 | 433 / 533 | 591 | 450 / 550 | 1 792 / 2 411 | 9 090 / 16 576 | 30–100 |

Every gen7 anchor: **all** tables measured have headroom 0 (21 / 21 anchors of X360 / PS3 08–Legacy and PS4 15–17 for the team table).
`exhibitionteams.index` is unique everywhere; `arenaid` values all reference an existing arena index on the titles that carry the field
(coverage 1.0 on 13–Legacy and PS4; 8 arenas shared by several teams on Legacy, 0 on PS4 21).

## 2. Rows a new team needs (index-keyed tables; per-team min / median / max over the teams that have rows; teams without rows)

| Table.field | gen7 witness (Legacy X360) | PS4 21 | reading |
|---|---|---|---|
| `stockteamjerseys.teamid` | 222 teams with rows, 30 without (the create-a-team block); 1 / 2 / 13 rows per team | 433 / 0; 2 / 2 / 78 | ≥ 1 kit row (2 typical) per real team — headroom 0 on gen7, 619 on PS4 21 |
| `stockgoalieequipment.team` | 222 / 30; 2 / 2 / 5 | 419 / 14; 2 / 2 / 5 | 2 rows per team (home / away) |
| `exhibitionrostertable.team` | 222 / 30; 20 / 25 / 35 | 424 / 9; 7 / 24 / 39 | a dressed roster (20+ players on gen7; 7 minimum seen on PS4) |
| `exhibitionplayerbiotable.team` | 215 / 37; 3 / 25 / 522 (522 = the free-agent pool) | 308 / 125 | players' `team` links; optional for a new team (teams without any player rows exist: 37 / 125) |
| `stockteamcolorstable.team` | 62 / 190; 2 / 3 / 6 | 66 / 367 | present for NHL / some leagues only — not required |
| `stockseasonteamstats.team`, `stockplayoffteamstats.team` | 30 + 32 rows, exactly 1 per NHL club; headroom 0 | 62 / 32 rows, headroom 0 | one row per **NHL** club; other leagues' teams have none — a new non-NHL team needs none; a new NHL club would need a slot that does not exist on any anchor |
| `stockretirednumbertable.team`, `stockawardstable.team`, `stockteamhistory` (PS4), career / record tables (PS4) | present for a subset of teams | headroom 0 on the PS4 21 career / record tables | optional history; full tables |
| `customteamjerseys.teamid`, `customteamequipmenttable.team` | 30 / 29 custom teams × 2 / 22 rows | 30 × 2 | the create-a-team block's own rows (fixed 30 / 22 slots) |
| `exhibitionteams.rivalteam` / `.ahlaffiliate` | per team, index-valued | same | a new team's values must be existing indexes (or the title's "none" value) |

## 3. Membership value spaces (what a new row must choose from; from the team table censuses)

- gen7 11–Legacy: `league` (16 distinct on Legacy: 0 NHL 32 rows, 1 AHL 30, 13 create-a-team 30, 11 / 8 / 9 … the European leagues,
  14 Winter Classic, 15 EASHL, 12 Legends) and `leaguegroup` (13 distinct); NHL 10: `league` (11), `conference` (0 / 1), `division` (0–5).
  `stockleagues` is absent on gen7 (league identity lives in the localization keys and the loader, 2026-09-18 handoff 23 / 2026-09-19 R3).
- PS4 15: `league` 15 distinct, `leaguegroup` 12; `stockleagues` 20 rows (`artid`, `leagueid`, `name`), full.
- PS4 21: `league` 23 distinct (23 = Circuit 97 rows, 0 NHL 35, 21 Alumni 33, 1 AHL 31, 15 EASHL 26, 13 create-a-team 22), `leaguegroup`
  19, `teamtype` 0 / 1 (3 rows); `stockleagues` 31 rows (+ `orderid`, `renderid`, colours), full.
- Competition groups (PS4 conferences / divisions) are the developer's P1-C catalogues; the tournament participant selection is
  code-resident (R55, card L-CHL-1) — unchanged.

## 4. Consequence (developer; not Codex-accepted)

- **Creation contract inputs, gen7 (X360 / PS3 08–Legacy):** no free record slot in any table → "create team" is **blocked at the
  container level** until the engine can grow a TDB table (relocating every following table; the roster-save ceiling memory of this
  repository says the gen7 main DB is packed to the save ceiling — a save-format constraint the developer must weigh); free `index`
  values (4–118) and 32 000+ free art ids are not the constraint. Refuse with "no record slot in `<table>` (capacity N = rows)".
- **PS4 15–17:** team table full (headroom 0) → same refusal for the team row; PS4 18–21: team, arena, kit, roster, goalie-equipment,
  retired-number and schedule slots exist (§1), but a new **NHL** club would also need a `stockseasonteamstats` / `stockplayoffteamstats`
  row (headroom 0) and, on 21, career / record rows — so "create a non-NHL team" is the only allocation the anchors can host without
  growth, and whether the runtime tolerates a club without stat rows is untested.
- **Row set for a new non-NHL team (PS4 18–21):** 1 `exhibitionteams` row (free index, valid `league` / `leaguegroup`, existing `arenaid`,
  `artid` free or shared by design — handoff 04), ≥ 1 `stockteamjerseys` row (art id + kit files!), 2 `stockgoalieequipment` rows, ≥ 7–20
  roster rows referencing existing players, optional colours; schedule rows only if the league's schedule must include it (P1-A's
  generator is bounded by the table's row count: 60 spare games per league table on PS4).
- **Kit files are the second constraint:** a new art id has no textures; the team must reuse an existing art id (shared key, handoff 04)
  or the kit / logo files must be copied — the art-id graph of handoff 04 lists the families.

```
python local/runs/overnight-research-2026-09-24/probes/r87_team_row_allocation.py docs/research/overnight-2026-09-24/data/r87-team-row-allocation.json
```
Executed once (25 anchors); the §1 / §2 figures were read back from the JSON.

> **Correction note 13:25 MDT (R88, [handoff 06](06-correction-checkpoint-campaign-review.md) §F1 / §F2):** (1) The "PS4 19 / 20" row of §1
> is an NHL 19 base measurement (`anchors/ps4/nhl20` is mislabeled); the real NHL 20 install: teams 416 / 517, arenas 428 / 529, kits
> 1 652 / 2 254, roster 8 613 / 15 735, `stockleagues` 31 / 31, `ahlschedule` 1 150 / 1 210, plus `worldchampschedule` 0 / 100 and
> `worldjuniorschedule` 0 / 60; NHL 19 patch 56 adds 4 team rows — handoff 06 §F1 (hashes for every input). "Every gen7 main database"
> means the 18 X360 / PS3 08–Legacy anchors; NHL 07, Recompiled and NHL 19 patches 51–55 were not measured. (2) §2's row "one row per
> **NHL** club … a new NHL club would need a slot that does not exist" and §4's "only a non-NHL team can be hosted" are **withdrawn**:
> `stockseasonteamstats` holds 30 / 31 NHL **and** (PS4 18+) 30 / 31 AHL rows, `stockplayoffteamstats` 16 NHL + 16 AHL from NHL 09, and
> the PS4 20 / 21 career tables hold 32 Swiss / other European teams — shipped-season participants, not a per-club requirement. The
> min / median / max columns of §2 are observed stock distributions, not required minima; the "index reference" class is a coverage
> candidate, not a foreign-key proof. Headroom 0 scopes a refusal to a **no-growth in-place allocation on the inspected file** only;
> Studio serialization, archive placement, game loading and mode initialization are separate, unresolved layers. Replacement statement
> and the proposed (not run) allocation experiment: handoff 06 §F2. No league-creation contract follows from this census.
