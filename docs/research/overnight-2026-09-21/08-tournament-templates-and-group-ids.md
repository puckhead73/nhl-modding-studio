# Handoff R36 — Competition structure (Phase 1.2), two tabled questions closed at the data level: tournament schedules are slot templates, and the PS4 group ids 62–64 / 75–78 are the tournament, alumni and template pseudo-leagues

Research lane, 21 September 2026. Claim `research-r36-tournament-groups`. Read-only over the repo anchors (PS3 / Xbox 360 08–Legacy, PS4
15–21; the NHL 20 anchor is the NHL 19 base — the finding below is identical on 19 / 20 / 21 so no install DB was needed). Nothing
game-verified; no writer.

> **Note 2026-09-26 03:01 ([overnight-2026-09-26/09-tournament-schedule-slots.md](../overnight-2026-09-26/09-tournament-schedule-slots.md) §5):** "resolved at runtime (group standings)" holds as an interpretation for knockout seeds only; round-0 slots are participant slots with an unresolved binding (World Cup: consistent join with the group ids). "No per-slot team binding to edit" is overstated (not established either way). The 24 domestic CHL participants' "mechanism outside the database tables (not found here)" is superseded: `stockteamjerseys.ischampionsleague` marks exactly 32 clubs (8 guests + 24 domestic) on every PS4 18–21 source — a candidate, selection by it not established.

Machine-readable: [`data/r36-tournament-templates-and-groups.json`](data/r36-tournament-templates-and-groups.json). Probe
`local/runs/overnight-research-2026-09-21/probes/r36_tournament_templates_and_groups.py` (≈ 1 s).

## Plain-language summary

Two of the 1.2 questions that the 2026-09-20 handoff 06 §3 left with "next step: a bounded probe" are answered by the anchors themselves.
(1) The tournament schedule tables (`championsschedule`, `spenglerschedule`, NHL 17's `wcschedule`) carry a `round` column and their
`home` / `away` values are **template slots**, not team ranks: the Champions Hockey League table is 32 group slots × 6 games (8 groups of 4,
each pair twice) followed by round-of-16 / quarter / semi (16 / 8 / 4 slots × 2 legs) and a one-game final on re-numbered seed slots; the
Spengler Cup table is 6 group games over 10 placeholder ids, then 2 + 2 + 1 knock-out games on other placeholders. The participants of
these slots are resolved at runtime (group standings), so "slot count exceeds the league's members" was the wrong expectation — there is no
per-slot team binding to edit. What the database does bind: the tournament-only clubs (`league` 9 = 8 CHL clubs from leagues the game does
not model; `league` 20 = the 6 Spengler clubs); the other 24 CHL participants (clubs of the modelled European leagues) are chosen by a
mechanism outside the database tables (not found here). (2) The group ids the P1-C work showed "by number": 62 = EBEL (league 7), 63 =
Allsvenskan (8), 64 = CHL guests (9), 75 / 76 = the two Spengler Cup groups (league 20, 3 clubs each), 77 = the NHL Alumni + All-Time teams
(leagues 21 / 22, one flat group), 78 = the Pro-Am template teams (league 23, `1T01`…). None of these needs a displayed conference /
division name — they are single-group or tournament pseudo-leagues — which is why no dictionary key exists for them.

## 1. Tournament templates (measured on the PS4 anchors; absent on gen7 except NHL 11 / 12's `fcc_tournaments`, 75 rows, no `round`)

| Table (titles) | Rows | Round 0 | Later rounds | Slot ids |
|---|---|---|---|---|
| `championsschedule` (18–21) | 125 | 96 games, 32 slots × 6 games, 48 pairs × 2 (home / away) → 8 groups of 4 | round 3: 16 games, 16 slots × 2 (8 pairs × 2); round 4: 8 / 8 × 2; round 5: 4 / 4 × 2; round 6: 1 game, 2 slots | 0–31 in round 0; 0–15 / 0–7 / 0–3 / 0–1 later (re-numbered seeds) |
| `spenglerschedule` (18–21) | 11 | 6 games over 10 slot ids (8 ids once, 2 ids twice; 6 distinct pairs) — two groups of 3, single round robin, with placeholder ids | round 4: 2 games (ids 1 2 4 5); round 5: 2 (ids 0 3 10 11); round 6: 1 (0 1) | 0–11 |
| `wcschedule` (17 only) | 17 | 12 games, 8 slots × 3 (two groups of 4, single round robin) | round 3: 2 games; round 4: 3 games | 0–8 |
| `worldchampschedule`, `worldjuniorschedule` (21) | 0 (declared, empty) | — | — | — |

Months: the CHL games sit in months 7–11 (+ 0–1 for the final) — the calendar is in the template. Fields on every table: `away day home
index month round`.

## 2. Group ids ≥ 60 (PS4 19–21; 18 has 62–64 and a single 75)

| Group id | League id (stockleagues name) | Clubs (NHL 21) | Reading |
|---|---|---|---|
| 62 | 7 `TXT_LEAGUE_EBEL` | 11 (BOLZ, DORN, GRZ, KAC, …) | one flat league (conference = division = 62) |
| 63 | 8 `Allsvenskan` | 14 | flat |
| 64 | 9 `TXT_CHAMPIONS_LEAGUE` | 8 (AAL, SON, CAR, GROD, MSK, GRE, STAV, TYC) | the CHL clubs from unmodelled leagues; the other 24 CHL slots are domestic clubs (binding not in the DB) |
| 75 / 76 | 20 `TXT_SPENGLER_CUP` | 3 + 3 (TRI, HCD, CAN / UFA, TPS, AMB) | the two Spengler groups |
| 77 | 21 `TXT_LEAGUE_NHL_ALUMNI` + 22 `TXT_LEAGUE_NHL_ALL_TIME` | 37 (ANA, ARZ, BOS, … ALL) | one flat pseudo-group shared by two leagues |
| 78 | 23 `TXT_LEAGUE_PROAM` | 97 (`1T01` … template teams) | template / creation teams |

`stockleagues.leagueid` → name (31 rows on NHL 21, incl. `CHL` 26 with 0 members, `TXT_USA_LEAGUE` 27, `TXT_RUSSIA_LEAGUE` 28, `TXT_ROW_LEAGUE`
29, `HUT` 30) is in the JSON per anchor, as are the member counts per league id.

## 3. Consequences for the developer

- **1.2 tournament tables:** browse-only stays right; a "participant slot" editor is not a thing — the editable objects are the template
  rows (`round`, `month` / `day`, slot ids) and the tournament-league memberships (`exhibitionteams.league` 9 / 20 and their group ids). Changing
  the *domestic* CHL participants has no DB handle (open, §4).
- **P1-C group names 77 / 78:** show them as "NHL Alumni / All-Time (single group)" and "Pro-Am template teams (single group)" derived from the
  member leagues — a rule, not a dictionary key; 75 / 76 as "Spengler Cup group 1 / 2"; 62–64 as the league name. Not "unknown".
- **Membership validation:** a club moved into league 9 / 20 must also carry the group id of that pseudo-league (64; 75 or 76); the
  spare Spengler slot ids and the NHL 18 all-75 placeholders (`SC1`–`SC6`) show the template pre-dates the club assignment.

## 4. Open (Q-R36-1)

The selector of the 24 domestic CHL participants and of the World Championship rosters (no DB table; candidates: the executable's
settings / CSV catalogue of P1-BS, or runtime standings); NHL 11 / 12 `fcc_tournaments` (75 rows, fields in the JSON — not analysed);
whether an edited template row (e.g. a changed `month`) is honoured (runtime, card only if a tournament edit is ever built).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r36_tournament_templates_and_groups.py docs/research/overnight-2026-09-21/data/r36-tournament-templates-and-groups.json   (≈ 1 s)
```
Executed once over 27 anchor DBs; two scratch scripts for the inventory and slot analysis were removed.
