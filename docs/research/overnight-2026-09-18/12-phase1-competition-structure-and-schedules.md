# Handoff P1.2-X — competition structure (groups) and source schedules, all anchor families

Research lane, overnight 2026-09-18. Claim `p1.2-competition-groups-and-schedules`. Consumer: developer Phase 1.2
(membership links, divisions/conferences, schedule sources, mode initialization) and C6 schedule tools.
Machine-readable: [`data/schedule-tables-survey.json`](data/schedule-tables-survey.json) (every `*schedule*`
table on all 27 main-DB anchors + NHL 07: fields, widths, row counts, value ranges, games per team slot,
rows per month, first/last rows) and [`data/xbox360-competition-groups.json`](data/xbox360-competition-groups.json)
(the executable group tables for NHL 12–Legacy with dictionary texts, plus the team table's group-id usage per
league). Probes: `schedule_survey.py`, `sched_slots.py`, `group_table.py`, `schema_dump.py`.

Evidence: **observed** on the anchors and executables; **inferred** where marked.

## Plain-language summary

The "source schedule" the roadmap asks for is inside the main database on every title: one table per
league (`nhlschedule`, `ahlschedule`, `ohlschedule`, `qmjhlschedule`, `whlschedule`, `swedishschedule`,
`finnishschedule`, `germanschedule`, `czechschedule`, `swissschedule`, later `echlschedule`, `austrianschedule`,
…), one row per game with a 0-indexed month/day, a running game `index`, and `home`/`away` given as the team's
**rank within its league** (its 0-based position among the league's members sorted by the team table's global
`index` field — *not* the `index` value itself and not the team id; §1.2). The NHL table holds
1 230 games plus one All-Star row on the 30-team titles (`1231` rows), 1 271 rows on the 31-team titles (NHL 18–21),
and a separate `nhl32schedule` (1 312) is shipped from NHL 18 for the 32-team layout. Every title also ships
`nhlfutureschedule` (the second Franchise season), `favoriteteamschedule`, and short `*scheduledebug` tables
(12 games per team). The mode-local copy is `leaguescheduletable` / `leaguetempscheduletable` (0 rows in the
stock DB, 8 fields, up to 4 500 rows on PS4). NHL 07 alone ships alternative season lengths as separate
tables (`nhlschedulesmall` 29 games, `nhlschedulemedium` 58, `nhlschedulelongleap/noleap` 82). The loose
`fe/nhlsched.bin` (1 310 B, differs per title) is **not** the schedule and stays unidentified.

Conferences and divisions: on NHL 12 → Legacy the team table carries three *global group ids*
(`leaguegroup`, `conferencegroup`, `divisiongroup`) that index a group table baked into the executable —
sequential (dictionary key, league art id) pairs listing each league, its conferences and its divisions
(NHL: `NHL`, `WesternConference`, `EasternConference`, `NHLDivisionA..D` on 14+ / six named divisions on 12–13;
AHL, OHL, QMJHL, WHL likewise; the European leagues are single groups; the national-team "league" points at a
tournament block `TournamentFile / PoolA / PoolB / Playoffs / ConferenceA-B / DivisionA-D`). The ids are
**title-specific** (52 groups on NHL 12/13, 50 on 14/15, 49 on Legacy, with shifted positions), so a team row's
group values do not transfer between titles. NHL 08–11 instead store a per-league local `conference` (1 bit)
and `division` (3 bits) on the team; the executable still lists the group names, without ids.

## 1. Source schedule tables (observed)

### 1.1 Row layout

| Family | Record | Fields (bit offset/width) | Notes |
|---|---|---|---|
| gen7 Xbox 360 / PS3, NHL 08–Legacy — NHL/AHL/CHL tables | 8 B | `round` 0/3 · `status` 3/2 · `day` 5/5 · `month` 10/4 · `index` 14/11 · `home` 25/5 · `away` 30/5 | `round`/`status` always 0 in stock; 5-bit slots (0..31) |
| same — European tables | 4 B | `round` 0/3 · `status` 3/2 · `day` 5/5 · `month` 10/4 · `index` 14/9 · `home` 23/4 · `away` 27/4 | 4-bit slots (≤ 16 teams) |
| NHL 07 (gen6) | 8 B | as gen7 but `status` 3 bits (`day` 6/5, `month` 11/4, `index` 15/11, `home` 26/5, `away` 31/5) | European tables also 8 B, 5-bit slots |
| PS4 NHL 15 | 8 B / 4 B | **gen7 layout** (`status` present, 5-bit / 4-bit slots) | corrected R1: NHL 15 PS4 = the gen7 record, not the 16+ one |
| PS4 NHL 16, 17 | 8 B / 4 B | `round` 0/3 · `day` 3/5 · `month` 8/4 · `index` 12/11 · `home` 23/**5** · `away` 28/**5** (European 4 B: `index` 12/9, slots 21/4, 25/4) | no `status`; still 5-bit slots |
| PS4 NHL 18–21 | 8 B / 4 B | as 16/17 but `home` 23/**6** · `away` 29/**6** on the 8 B tables (European 4 B tables keep 4-bit slots; `championsschedule` 5-bit on 18–20, 6-bit 8 B on 21) | 6-bit slots (≤ 64 teams) from NHL 18 |
| `leaguescheduletable` (save-side) | 8 B | 8 fields | 0 rows in stock DBs; PS4 max 4 500 |

`month`/`day` are 0-indexed (Legacy opener: month 9 day 6 = **October 7, 2015**, MTL@TOR; last row month 3
day 8 = April 9, 2016 — both match the real 2015-16 NHL calendar). `index` = 0..N-1 in row order.

### 1.2 `home` / `away` = the team's rank within its league, ordered by the team table's global `index` (verified — corrected 2026-09-18 R1)

**Correction (R1 item 2).** The first version of this section said "slot = the team table's `index` field".
That is only true for the NHL, whose teams happen to occupy `index` 0–31. The `index` field of
`exhibitionteams` (`kOtt`) is **global and unique across the whole table** (Legacy Xbox 360: NHL 0–31,
AHL 32–61, SHL 62–75, …; NHL 21 PS4: 433 teams, 433 distinct values, up to 696). A schedule slot is the
**0-based rank of the team among its league's members when those members are sorted by `index`** — the
formula developer receipt 04 implemented. Per-anchor member lists, ranks and games-per-slot for all 25 anchors
with the field: [`data/schedule-slots/slots-<platform>-<title>.json`](data/schedule-slots/) (probe
`sched_slots_all.py`; pair counts `sched_pairs.py`).

Evidence, all **observed** on the anchors:

| Check | Title / table | Result |
|---|---|---|
| NHL opener | Legacy X360 `nhlschedule` row 0: home 27 / away 15 | ranks 27 = TOR, 15 = MTL — the real 2015-10-07 opener; rows 1–3 CHI–NYR, CGY–VAN, LA–SJ likewise |
| **Non-NHL league, real-world game counts** | Legacy X360 (and PS3) `ahlschedule`: slots with 68 games = `1, 14, 21, 22, 24`; all others 76 | ranks by `index` → **BAK, ONT, SD, SJ, STO** = exactly the five California clubs of the 2015-16 AHL Pacific Division, which played 68 games while the other 25 played 76. Raw `index` (33, 46, 53, 54, 56) is out of range for the 5-bit slot, which rules out "slot = raw index". |
| Rank by `index` vs rank by **record order** (they differ only where a team row is stored out of `index` order) | NHL 11 X360/PS3 OHL: Mississauga has `index` 158 but is record 162 (stored after Owen Sound) | `sched_pairs.py ohlschedule`: slot 8 plays slots 2 (BRA) and 9 (NIA) 8×, 0 (BAR) and 18 (SBY) 6× — the Central Division pattern — so slot 8 = **MISS = rank by `index`**; slots 9–12 = NIA, OSH, OTT, OS follow the same rule (division rivals 8×). Rank by record order would put NIA at 8 — wrong. |
| same | NHL 10 X360/PS3 AHL: `index` 59 = WBS (record 60), 60 = WOR (record 59) | slot 27 plays ADK/HER 10×, ALB/BNG/NRF/SYR 8× (East Division) → **WBS**; slot 28 plays LOW/POR/PRO/SPR 10× (Atlantic) → **WOR** — rank by `index` again. |
| Every other anchor | 23 of the 25 anchors have record order == `index` order inside every league | both readings coincide there; the rule above is the one that also holds on the two exceptions |

Exceptions and invariants (observed):

- The two All-Star squads (`index` 30/31 on the 30-team titles) are league-0 members and occupy slots 30/31;
  the single All-Star row is the only game they have. NHL 10 has no All-Star row (1 230 rows).
- PS4 NHL 18–21: league 0 holds **35** rows — 31 clubs (`index` 0–30, incl. VGK) plus four All-Star division
  squads `ATL/CEN/MET/PAC` (`index` 31–34) — but `nhlschedule` uses slots 0–30 only (31 × 82, no All-Star row);
  `nhl32schedule` uses 32 slots (0–31, 82 each), so slot 31 there is the 32nd-club placeholder (the `ATL` row
  by rank). A validator must therefore allow league members with **zero** games, and a 32-team generator must
  know which row is the expansion placeholder — runtime question.
- `index` is not always contiguous per league on PS4 (NHL 18/21 leagues 17 and 19 interleave in 386–399);
  rank remains well-defined and those leagues have no schedule table.
- 4-bit slots (European tables) cap a league at 16 teams; 5-bit at 32; PS4 6-bit at 64. `championsschedule`
  (NHL 18+, 33-team league 21) uses slots 0–31 only.
- NHL 07 (gen6): `exhibitionteams` is only a thin `(table, division, teamid, index)` list; `league`, `index`,
  `conference`, `division` live on `stockteamtable` (93 rows: NHL 0–31 incl. `EAS`/`WES` All-Star slots 30/31,
  national 32–52, Elitserien 53–64, SM-liiga 65–77, DEL 78–91, `DAV` 92). Rank-by-`index` reproduces it:
  `nhlschedulelongnoleap` slot 0 (ANA) plays slots 9/13/22/25 (DAL/LA/PHX/SJ — the 2006-07 Pacific Division)
  8× each; slots 30/31 have one game (each other).

So a schedule editor must: (1) sort the league's members by `index`; (2) map slot ↔ rank; (3) never write
the `index` value itself into `home`/`away`; (4) re-derive every slot if a team is added to, removed from or
re-ordered within a league (changing a team's `index` or `league` silently re-labels the whole schedule).

**NHL 20 provenance (follow-up 2026-09-18):** every "PS4 18–21" statement above that was measured on
`anchors/ps4/nhl20` describes the **NHL 19 base** DB (`265e4a68…`), which that anchor actually is. The real
NHL 20 DB (CUSA15439 `inxp_00.big`, `1e200478…`) was surveyed separately —
`data/schedule-slots/slots-ps4-nhl20-install-1e200478.json` — and differs from the NHL 19 copy in exactly
the season-specific tables: `austrianschedule` 270 rows / 11 slots / 48–50 games (was 324 / 12 / 54),
`echlschedule` 936 rows / 26 slots (was 972 / 27); layouts, slot widths and the rank rule are unchanged (the
NHL 20 numbers coincide with NHL 21's). The `schedule-tables-survey.json` row `nhl20-ps4` and the previous
`slots-ps4-nhl20.json` (removed) were the NHL 19 copy; treat them as NHL 19 evidence.

### 1.3 Row counts (from the survey; full list in the JSON)

| Table | 07 | 08/09 | 10 | 11 | 12 | 13 | 14/15/Legacy | PS4 15–17 | PS4 18–21 |
|---|---|---|---|---|---|---|---|---|---|
| `nhlschedule` | small 436 / medium 871 / long 1231 (leap/noleap) | 1231 | **1230** | 1231 | 1231 | 1231 | 1231 | 1231 | **1271** (+ `nhl32schedule` 1312, `ahl31/32schedule`) |
| `nhlfutureschedule` | — | — | — | — | — | — (13: —) | 1231 (14+) | 1231 | 1271 |
| `favoriteteamschedule` | — | — | — | — | — | 1231 | 14: **0**, 15/Legacy: 1231 | 1231 | 1271 |
| `ahlschedule` | — | 1160 | 1160 | 1200 | 1140 | 1140 | 1140 / 1140 / 1120 | 1140 / 1120 / 1116 | 1116–1150 |
| `ohl/qmjhl/whl` | — | — | — | 680 / 612 / 792 | 680 / 578 / 792 | 680 / 612 / 792 | same | same | 680 / 612 / 748 |
| `swedish/finnish/german/czech/swiss` | intl* (300/364/364, 8 B) | — | 330/406/420/364/300 | same | 330/406/364/364/300 | same | Legacy: 364/450/364/364/300 | 16+: 364/450/364/364/300 | + `swedish2nd` 364, `austrian` 324/270, `champions` 125, `spengler` 11, `echl` 972/936, `wc` 17 (17), `worldchamps`/`worldjunior` 0 (21) |
| `*scheduledebug` | — | — | — | nhl 181, ahl 180 | + ohl 120, qmjhl 102/108, whl 132 | same | same (ahl 175 on Legacy) | same | nhl/ahl 465–466 |

NHL 10's `nhlschedule` has 1 230 rows (no All-Star row that year — real 2009-10 had no ASG (Olympics)); that is
the kind of per-title exception a generator must respect.

### 1.4 Editing / import / generation requirements (R1 follow-on; observed on the anchors, see `data/schedule-tables-survey.json`)

**Bindings.** A source table binds to a league by name stem (`nhl`, `ahl`, `ohl`, `qmjhl`, `whl`, `swedish`,
`finnish`, `german`, `czech`, `swiss`, `echl`, `austrian`, `swedish2nd`, `champions`, `spengler`, `worldchamps`,
`worldjunior`, NHL 07 `intl<league>`), and each row binds two teams by **rank within that league by global
`index`** (§1.2). Nothing in the DB links a table to a league id — the binding is the executable's; Studio's
stem match is the same inference and must stay disclosed.

**Invariants observed in every stock table** (a validator should enforce them; a generator must produce them):
`index` = 0..N-1 in row order; rows sorted by season date (months 8/9 → 3, 0-indexed); `home ≠ away`; every
club slot has the league's game count (82/76/68/60/56/55/52/50/48…, per-title — the AHL split 68/76 and
NHL 21 Austrian 48/50 show that *unequal* counts are legitimate); home/away balanced per slot; `round` and
`status` are 0 in every stock row (their runtime meaning — playoff round? played flag? — is **unknown**; a
generator should write 0); slot fields sized 4/5/6 bits per family cap the league at 16/32/64 teams;
All-Star squads (slots 30/31 on 30-team gen7 titles) have exactly the single All-Star row, none on NHL 10,
none on PS4 18+ (31-team layouts); PS4 18+ league 0 also carries four division squads with no games at all.

**Fixed-capacity limits.** gen7 (Xbox 360 / PS3, NHL 07–Legacy): **every schedule table is packed**
(`max_records == rows`, zero spare rows, e.g. `nhlschedule` 1231/1231), so a season with *more* games than
stock needs table growth in the loose main DB (a capacity question for `tdb-edit`, not a slot question), and
a season with *fewer* games either truncates the row count (untested: does the engine read `current` or
`max`?) or has to keep 1231 rows with duplicated/filler games. NHL 07 avoids the question by shipping four
fixed-length tables. PS4: **+60 spare rows** on nearly every table (`nhlschedule` 1231/1291, 1271/1331,
`nhl32schedule` 1312/1372, `ahlschedule` up to 1150/1331 on 19/20, `whlschedule` 748/852 on 19–21,
`favoriteteamschedule` +200/+400), so small extensions fit without growth. `nhlfutureschedule` mirrors
`nhlschedule`'s size (Franchise second season) and must be regenerated with it.

**Mode-specific copies.** `leaguescheduletable` (8 fields: + `league` 3 bits, wider `index` 12/13 bits,
`home`/`away` 7 bits gen7 / 8 bits PS4 — i.e. the mode copy addresses teams by a **wider, league-tagged
id**, not the 5-bit rank) and `leaguetempscheduletable` are 0 rows in every stock DB (PS4 capacity 4000/4500
and 2700/4000); NHL 15 PS4 adds `leaguescheduleperiods` (12 B: `begindate`/`enddate` 16-bit, `leagueid`,
`periodid`, `scheduleid`, `currentphase`). How Season/Franchise fills them from the source table is a save
question (the Xbox `SEASON`/`DYNASTY` fixtures are the specimens) — **not measured**.

**Remaining unknowns.** `round`/`status` semantics; `fe/nhlsched.bin`; engine behaviour on a shorter table;
whether the game re-derives the 32nd slot from `nhl32schedule` or from a team flag; PS4 `worldchamps`/
`worldjunior` (NHL 21, 0 rows, 100/60 capacity) and `wcschedule` (NHL 17 only, 17 rows — 2016 World Cup) formats; whether the executable bounds
games-per-team (NHL 21 Austrian 48/50 suggests it does not).

### 1.5 Unresolved (original list)

- `fe/nhlsched.bin` (1 310 B on 09–Legacy, 1 382 on NHL 08; different bytes per title; not in any archive) —
  role unknown (candidates: scheduler rule/permutation seed; unrelated). Not needed to edit `nhlschedule`.
- How a new Season/Franchise copies `nhlschedule` into `leaguescheduletable` (which fields the extra column
  carries) is a mode-initialization question — runtime/save inspection tomorrow (the Xbox `SEASON`/`DYNASTY`
  save fixtures under `anchors/xbox360/*/savefiles` are the specimens).
- Whether the game accepts a schedule with fewer rows than the table's `max_records`/current (custom season
  length, Q03a.1c) — untested; NHL 07's separate small/medium tables suggest the engine expects a full table
  per length rather than a truncated one.

## 2. Competition groups (conferences / divisions)

### 2.1 NHL 12 → Legacy (Xbox 360 executables; PS3 by identity of the DB fields)

Group table = sequential `(dictionary key, league art id)` pairs; `group_id` is the row position. Team table
fields `leaguegroup`, `conferencegroup`, `divisiongroup` hold `group_id`s. Cross-check on every title: league 0
teams have `leaguegroup 0`, `conferencegroup {1,2}`, `divisiongroup {3..8}` (12/13) or `{3..6}` (14/15/Legacy);
AHL teams `leaguegroup 9` (12/13) / `7` (14+), etc.; single-group leagues repeat the league's own id in all three
fields; national teams use the tournament block; custom teams the last id. The All-Star rows use the
*conference* ids as their `divisiongroup` (1/2).

| Title | VA | Groups | NHL divisions (id: key → text) |
|---|---|---|---|
| NHL 12 | `0x837b5040` | 52 | 3 Northwest · 4 Pacific · 5 Central · 6 Southeast · 7 Atlantic · 8 Northeast |
| NHL 13 | `0x8383d250` | 52 | same six |
| NHL 14 | `0x839f3428` | 50 | 3 `NHLDivisionA` "Division A" · 4 B · 5 C · 6 D (the dictionary still says A–D) |
| NHL 15 | see JSON | 50 | 3 A "Pacific" · 4 B "Central" · 5 C "Atlantic" · 6 D "Metropolitan" |
| Legacy | `0x83a150c8` | 49 | same as 15; Norway single group 19 shifts OHL→20, QMJHL→27, WHL→31, Tournament→38, Custom→48 |

Consequence for Phase 2 (teams): moving/creating a team must set the three group ids from *that title's*
table; a value copied from another title is silently wrong. Division/conference **names** are dictionary keys
(`NHLDivisionA..D`, `WesternConference`, `AtlanticDivision`, …), so renaming a division is a LOC-W1-class
operation; renaming a *conference* changes every league that reuses `WesternConference`/`EasternConference`
(NHL, AHL, OHL, WHL share the keys).

### 2.2 NHL 08–11

Team fields `conference` (1 bit, local 0/1) and `division` (3 bits, local 0..7); no group ids. The executables
carry the same group-name list (`NHL, WesternConference, EasternConference, NorthwestDivision, …`) as plain
pointer arrays (NHL 11 `0x834cef70`; NHL 10 `0x8333f948`), presumably indexed per league by local conference/
division number — the exact local→name binding was **not traced** (needs the consuming code). NHL 09/07: no
such table found by the same search; their team fields are the same local pair.

### 2.3 PS4

`stockleagues` (`fsLK`) carries `leagueid`, `name` (the key), `artid` (= the family-wide league art id, see
P1.4-X), `renderid`, colours and `orderid`. `exhibitionteams` (`kOtt`) carries the same three global group
fields as gen7b, 7 bits each (NHL 21: league 0 → `leaguegroup 1`, `conferencegroup {6,7}`,
`divisiongroup {8,9,10,11}`; AHL → 12 / {13,14} / {15..18}; European leagues single group ids 19–23) — but
**no group table exists in `nhl_game.db`** (no table or field family beyond `*group` on the team row), so
the PS4 group table also lives in the executable. **Update (R1 follow-on):** the seven `eboot.bin` files are
plaintext (fake-signed) and the table was located and resolved — see handoff
[17](17-ps4-groups-and-consumer-leads.md) and `data/ps4-competition-groups.json`. PS4 group ids are
numbered differently from gen7b (NHL is group 1, not 0) — one more "do not copy group values across titles".

## 3. What development can do now

1. Schedule browse/validate for every gen7 and PS4 target from the DB alone: parse the league's table, resolve
   slots as rank-by-`index` within the league (§1.2), render dates from 0-indexed month/day, validate 82/52/… games per slot, keep
   the All-Star row where present. Round-trip is an ordinary fixed-bit record edit (`tdb-edit`).
2. Expose conference/division membership on NHL 12–Legacy as human-readable groups via the group table (bake
   `xbox360-competition-groups.json` into `defs/` or trace at runtime from the executable) — with the per-title
   caveat above.
3. Treat the NHL 07 length variants and NHL 10's 1 230-row table as the two known exceptions to "1 231 rows".

## 4. Discriminating checks queued for tomorrow (runtime)

- Start a Season on a title after swapping two games' dates in `nhlschedule` and confirm the calendar reflects
  it (proves the source table and the slot mapping in one shot).
- Inspect a fresh `SEASON`/`DYNASTY` save's `leaguescheduletable` against the source table (fixtures exist).
