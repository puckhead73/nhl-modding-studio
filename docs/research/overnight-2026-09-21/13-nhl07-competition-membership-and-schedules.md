# Handoff R39 — NHL 07 (gen6 schema) competition membership and schedule structure, measured on both `NHL07NG.db` anchors

Research lane, 21 September 2026, Priority 1.5 ("NHL 07 membership semantics", carried open since the 2026-09-18 handoff 12 and the
2026-09-20 handoff 06 §3). Claim `research-r39-nhl07-competition`. Read-only over the two repo anchors (X360 = PS3, identical results);
nothing game-verified; no writer. NHL 07 Xbox 360 is a shipping target; NHL 07 PS3 is research-only.

Machine-readable: [`data/r39-nhl07-competition-membership.json`](data/r39-nhl07-competition-membership.json). Probe
`r39_nhl07_competition_membership.py` (≈ 2 s).

## Plain-language summary

NHL 07 keeps its competition structure on the team rows themselves: `stockteamtable` carries `league`, `conference` and `division` for
its 93 rows (NHL 32 incl. the two All-Star rows, 21 national teams, Elitserien 12, SM-liiga 13, DEL 14, Davos 1), and the thin
`exhibitionteams` selector is a stock-only identity map (every row `table 0 / division 0 / teamid = index`; the custom table is empty).
The seven schedule tables reference teams by **rank inside their league** — the order of that league's rows in `stockteamtable` — and
their NHL quotas are exactly the 2007-08 format the later titles use (8 games per division rival, 4 per conference rival, 1 against
10 of the 15 other-conference clubs), with a 58-game (2 / 2 / 2) and a 29-game (1 / 1 / 1) short season and one All-Star row. So the
Leagues page can show NHL 07's leagues, conferences and divisions from `stockteamtable` alone, its membership editing is a `league` /
`conference` / `division` cell edit with the schedule-rank dependency the other titles already have, and the era quota table of the
2026-09-20 handoff 06 gains an NHL 07 row identical to NHL 08's.

## 1. Membership (both anchors)

| League id | Teams | `stockteamtable` index range | Conference / division groups | Names |
|---|---|---|---|---|
| 0 | 32 | 0–31 | conf 0: div 3 (CHI CBJ DET NSH STL), 4 (CGY COL EDM MIN VAN), 5 (ANA DAL LA PHX SJ); conf 1: div 0 (BOS BUF MTL OTT TOR), 1 (NJ NYI NYR PHI PIT), 2 (ATL CAR FLA TB WSH); conf 0 / div 0: `EAS` 30, `WES` 31 (All-Star) | NHL 2006-07 |
| 1 | 21 | 32–52 | conf 1 / div 0 | national teams (Austria … USA) |
| 2 | 12 | 53–64 | 0 / 0 | Elitserien |
| 3 | 13 | 65–77 | 0 / 0 | SM-liiga |
| 4 | 14 | 78–91 | 0 / 0 | DEL |
| 5 | 1 | 92 | 0 / 0 | Davos (Spengler host; no schedule table) |

`exhibitionteams` (93 rows, `max_records` 93): `table` = 0 and `division` = 0 on every row, `teamid` = `index`; `customteamtable` has 0
rows. So on NHL 07 the selector adds nothing to the stock table — the `table` / `division` words are the custom-team hook (disc 0 = stock,
1 = custom, as the 2026-09-18 record said) and are unused in the stock DB. League / conference / division ids have no name table in the DB
(the later `stockleagues` does not exist); the names come from the dictionary by the UI's own keys (not traced here).

## 2. Schedules (identical on X360 / PS3; `index` column = row order; `round` = 0 and `status` = 0 everywhere; months 0-indexed)

| Table | Rows = `max_records` | League | Ids | Games per team | Pair quotas | Home / away imbalance |
|---|---|---|---|---|---|---|
| `nhlschedulelongleap` = `nhlschedulelongnoleap` (byte-identical rows; February days 0–27 in both) | 1 231 | 0 | 0–31 (rank = index) | 82 × 30, 1 × 2 (All-Star `WES` v `EAS`, row 727) | **division × 8** (60 pairs), **conference × 4** (150), **cross × 1** (150 of 225) — NHL 08's format | 0: 210, 1: 70 |
| `nhlschedulemedium` | 871 | 0 | 0–31 | 58 × 30 + All-Star | division × 2, conference × 2, cross × 2 (all 225) | — |
| `nhlschedulesmall` | 436 | 0 | 0–31 | 29 × 30 + All-Star | division × 1, conference × 1, cross × 1 | — |
| `intldelschedule` | 364 | 4 | 0–13 (league rank) | 52 × 14 | every pair × 4 | 0: 91 |
| `intlelitserienschedule` | 300 | 2 | 0–11 | 50 × 12 | 48 pairs × 4, 18 pairs × 6 | 0: 66 |
| `intlsmliigaschedule` | 364 | 3 | 0–12 | 56 × 13 | 56 pairs × 4, 18 pairs × 6, 4 pairs × 8 | 0: 78 |

`leaguescheduletable` (the Dynasty copy) has `max_records` 0 in the stock DB. No table schedules league 1 (international) or 5.

## 3. Contract input

- **Leagues page (1.1 / 1.2), NHL 07:** groups = `stockteamtable.(league, conference, division)`; labels by the UI's dictionary keys or
  a derived label ("League 2 — 12 teams: Brynäs …") as for the PS4 pseudo-leagues (handoff 08); `exhibitionteams` is listed as the stock
  selector, not as a membership table.
- **Membership edit:** a `league` / `conference` / `division` cell edit on `stockteamtable`; the schedule dependency is the league rank
  (moving a team between leagues re-ranks both leagues → refuse or re-map as the accepted P1 contract does for the later titles); the
  All-Star rows 30 / 31 must stay in league 0 (the long / medium / small tables address them).
- **Schedule generation:** NHL 07 = the NHL 08 era row (8 / 4 / 1 quota, 82 games) plus the 58- and 29-game short-season quotas
  (2 / 2 / 2, 1 / 1 / 1); the European tables are real-season replicas (uneven 4 / 6 / 8 pairs) like the AHL tables of handoff 06 §2.
- **Not claimed:** the dictionary keys of the league / division names, the `day` indexing origin (0-indexed by the February range 0–27),
  runtime honouring of an edited membership, the international tournament's team selection (21 nations, no table).

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r39_nhl07_competition_membership.py docs/research/overnight-2026-09-21/data/r39-nhl07-competition-membership.json   (≈ 2 s)
```
