# Handoff R13 — the stock NHL / AHL schedule formats, observed per era (quotas by division/conference relationship), and the 1.2 competition-structure status

Research lane, 20 September 2026 (Priority 1E of the overnight campaign). Claim `research-r13-schedule-formats`. Consumer: developer
**P1-A generation** ("NHL / Swiss / AHL blocked with the numbers — no whole round-robin; the stock division-weighted format is not
established"), P1-B. Builds on handoff [12](../overnight-2026-09-18/12-phase1-competition-structure-and-schedules.md) (slot = rank by
`index`, group ids) and the `schedule-slots` data. Nothing here is game-verified; nothing written; no launch; no new game-generated
saves (none were authorised tonight). **Observed data, not an algorithm:** every table below is what the shipped files contain; a
generator that reproduces the quotas is not thereby proven to be EA's.

Machine-readable: [`data/r13-schedule-structure.json`](data/r13-schedule-structure.json) — per anchor (25 main DBs; NHL 07 excluded, see
§3) and per NHL/AHL table: games, teams with games, games per team, pairs, **games per pair grouped by relationship** (same division /
same conference other division / other conference), home-away imbalance per pair, division sizes. Probe: `r13_schedule_structure.py`
(≈ 20 s).

## Plain-language summary

The generator can only build "everyone plays everyone k times" schedules, so NHL and AHL tables were blocked. The stock tables show
what a real NHL schedule is: a fixed quota per opponent *class* — for the 30-team six-division era (NHL 09–13) each club plays its four
division rivals 6 times, the ten other conference clubs 4 times, and the fifteen other-conference clubs once, except three of them
twice; for the four-division era (NHL 14–21) 4 or 5 times inside the division (7-team divisions), 3 times across the conference and
twice against the other conference. Those quotas are exact on every title, with the "which three twice / which pairs get the fifth
game" choice made evenly. The AHL tables are copies of real seasons with travel-based, uneven pairings (2–14 games per pair, five
Pacific clubs on 68 games in 2015-16): no class rule fits them, so AHL generation means importing a real schedule or a custom quota
table, not a formula. This gives the developer the per-era quota tables to generate against and a test (reproduce every pair count
and the home/away balance) to judge a generator; the order of games and their dates remain observed data only.

> **Note 2026-09-26 01:23 ([overnight-2026-09-26/05-schedule-provenance-and-swiss-format.md](../overnight-2026-09-26/05-schedule-provenance-and-swiss-format.md)):** the source labelled NHL 20 here is `anchors/ps4/nhl20`, a mislabeled NHL 19 base copy (`265e4a68…`). Re-measured on the real NHL 20 (`1e200478…`): the NHL 18–21 quota row is unchanged (confirmed); the AHL figures labelled NHL 20 are NHL 19's season. Swiss ("not re-analysed" in §2) is now measured: three groups of four, 6 games within, 4 across, 50 per club, ~~groups not stored in the DB~~ groups not encoded by the six inspected `exhibitionteams` group fields (corrected 2026-09-26 02:36; other locations unsearched). Also 2026-09-26 02:36: "real-season replicas" in §2 describes shape, not provenance — read "shipped irregular schedules, no general generation rule established".

## 1. NHL formats (observed on every anchor of the era; identical on Xbox 360 / PS3 / PS4 of the same title)

Pairs are counted over the 1 230 / 1 271 club games (the extra All-Star row on the 30-team titles resolves to one stray `1` pair and
is excluded from the quotas). Home/away: `imb 0` = the pair split its games evenly, `1` = one team hosted one more.

| Era (titles) | Teams / divisions | Same division | Same conference, other division | Other conference | Per-team sum | Home/away imbalance (pairs 0 / 1) |
|---|---|---|---|---|---|---|
| NHL 08 (2007-08) | 30 / 6 × 5 | **8** × 4 rivals (60 pairs) | **4** × 10 (150 pairs) | **1** × 10 of the 15 (150 pairs of 225: 5 opponents unplayed) | 32 + 40 + 10 = 82 | 210 / 151 |
| NHL 09, 10, 11, 12, 13 (2008–13) | 30 / 6 × 5 | **6** × 4 (60) | **4** × 10 (150) | **1** × 12 + **2** × 3 (180 pairs once, 45 twice) | 24 + 40 + 18 = 82 | 255 / 181 (NHL 10: 255 / 180) |
| NHL 14, 15, 16, 17 (2013–17) | 30 / 4 (7, 7, 8, 8) | **4 or 5** (48 / 50 or 46 / 52 pairs at 4 / 5): a 7-team division's club plays its 6 rivals 4 × 2 + 5 × 4 = 28, an 8-team division's club its 7 rivals 4 × 4 + 5 × 3 = 31 | **3** × the other division's 8 or 7 clubs (112 pairs) = 24 or 21 | **2** × 15 (224 pairs) = 30 | 28 + 24 + 30 = 82 or 31 + 21 + 30 = 82 | 271–273 / 163–165 |
| NHL 18, 19, 20, 21 (31 teams) | 31 / 4 (8, 7, 8, 8) | **4** × 94 pairs, **5** × 11 pairs (the 7-team division's extra games) | **3** × 120 pairs | **2** × 240 pairs | 82 on all 31 | 334 / 131 |
| `nhl32schedule` (18–21, 32-team layout) | 32 / 4 (8, 7, 9, 8) | 2 × 38 / 3 × 75 | 2 × 47 / 3 × 81 | 2 × 91 / 3 × 164 | 82 on all 32 | 176 / 320 — **not** the real-NHL format: an even 2/3 mix, no 4–5 division emphasis |
| `nhlfutureschedule` (14+) | as the title's `nhlschedule` (the second Franchise season): same quotas, the 4/5 split shifted (48/50 ↔ 46/52) | | | | | |

Facts a generator must reproduce for a "stock-shaped" NHL table (observed on all): every club 82 games; quotas exactly as above; the
uneven choices (which cross-conference opponents get the second game, which division pairs the fifth) spread so that every team's
total is 82 — the specific assignment in stock is the real season's and is not derivable from the DB; home/away split even or off by
one per pair (never 2 on NHL 09+; NHL 08 / AHL 08–09 have a few pairs at 2). Dates: 0-indexed month/day over October–April with the
per-month row counts of handoff 12; no date rule was tested here.

## 2. AHL tables (observed): real-season replicas, no class rule

`ahlschedule`: 29 teams × 80 games (NHL 08–10), 30 × 80 (11), 30 × 76 (12–15 + PS4 15), 25 × 76 + 5 × 68 (Legacy / PS4 16: the 2015-16
Pacific Division), 24 × 76 + 6 × 68 (17, 18), 24 × 76 + 7 × 68 (19–21). Games per pair range 1–14 with no relationship rule (e.g. NHL 21:
same division 4–14, conference 2–12, cross 2–4; NHL 12: 1–12 in every class; NHL 10: 2–12). `ahl31schedule` / `ahl32schedule` (PS4
18–21): 82 games on 31 / 32 slots with 30 / 61 / 31 pairs involving slots beyond the AHL's member count (the expansion placeholders
already disclosed by the developer) — those tables are the `nhl32`-style 2/3 mix, not real AHL seasons. **Conclusion for P1-A:** AHL
generation is either (a) import of a real season through the existing CSV/JSON route, or (b) a user-supplied per-pair quota table;
a formula would not be "stock-shaped". Swiss / European tables were not re-analysed here (handoff 12 has their round-robin structure).

## 3. Competition structure — status of the remaining 1.2 questions (no new evidence tonight; obstacles and next steps)

| Question | Status | Obstacle / next discriminating step |
|---|---|---|
| Group names for PS4 ids 77/78 (NHL 19–21), NHL 17 id mechanism | unchanged (P1-C record: shown by number) | the names are not in the DB, dictionary keys or `stockleagues`; next: search the PS4 executables that handoffs 17 / 22 already read (`p1bs_eboot_*` probes) for a group table carrying ids 77/78 with name keys — not done tonight; or a user card reading the on-screen conference/division names of a league that uses 77/78 |
| NHL 07 membership semantics | unchanged (handoff 12 §: thin `(table, division, teamid, index)`; league/conference/division on `stockteamtable`) | the R13 probe skips NHL 07 (no `league` field on `exhibitionteams`); a bounded next step is the same pair-class analysis with `stockteamtable` groups — not done tonight |
| Tournament tables ↔ league (`championsschedule` 32 slots, `spenglerschedule` 12, `wcschedule`, `worldchamp` / `worldjunior`) | unchanged (browse only) | the slot count exceeds the stem league's members, so slots are not ranks in that league; next: pair-count analysis per slot against every league's member count (a bounded probe, not run) |
| Older-title settings storage (NHL 08–11) | unchanged (research unknown) | no data file found by earlier scans; next: the `default.xex` string tables of 08/09 for the setting names (the NHL 10–15 catalogue lives in the executable) |
| Chosen-setting serialisation in saves | unchanged (P1-B / P1-BS: accepted NHL 15 stored-default contract; runtime selection unknown) | needs before/after save specimens produced in game — none may be generated tonight; the existing specimens show stock correspondence only, which does not prove edited-input propagation (unchanged) |
| Row-count / competition expansion | unchanged (no spare rows; engine tolerance unknown) | separate from the baseline roadmap; the schedule tables' fixed `max_records` and the 5/6-bit slot widths (handoff 12) bound any expansion statically |

## 4. Contract input for the developer (proposal)

Offer NHL generation as **"stock-shaped by era"**: a quota table per era (§1) selected from the title and the league's group layout
(divisions of 5 / 7 / 8 from the group ids), an even assignment of the uneven quotas, an even home/away split, dates spread over the
stock month histogram; validate a generated table by recomputing the §1 counts (the JSON is the oracle: same relationship-class
multiset, same games-per-team, imbalance ≤ 1). Refuse when the league's division layout matches no era row (e.g. an edited league with
three divisions) unless the user supplies quotas. AHL: import or user quotas only. Runtime (whether a non-stock assignment is accepted
by Season/Franchise) stays the P1-A card.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-20/probes/r13_schedule_structure.py docs/research/overnight-2026-09-20/data/r13-schedule-structure.json
```
Executed once (≈ 20 s, read-only over the 25 anchors with `nhlng.db` / `nhl_game.db`); the two NHL 07 anchors fail on the thin team
table (recorded). **Not executed:** launches, writes, Studio binaries.
