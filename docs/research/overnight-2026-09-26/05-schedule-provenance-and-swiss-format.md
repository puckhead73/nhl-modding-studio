# Handoff R111 (P5) — League 1.2 schedule generation: R13's NHL quotas re-witnessed on the **real** NHL 20 database (identical to NHL 19 base and NHL 21), R13's "NHL 20" AHL row corrected (it was NHL 19's season); the **Swiss** `swissschedule` format measured for the first time — 12 clubs, 50 games, three groups of four playing 6 times inside the group and 4 times across, identical on X360 Legacy, NHL 19 base, real NHL 20 and NHL 21, with the groups ~~**not** recorded in any database field~~ **not encoded by the six inspected `exhibitionteams` group fields (correction 2026-09-26 02:35)**; and a coordination note that the developer ledger still marks this row "format not established"

Research lane, 26 September 2026, 01:18 → 01:23 MDT (real clock). Review package 5 (gap ledger: missing per-title evidence for implemented
paths). Claim `research-p5-schedule-provenance-swiss`. Read-only over the anchors and the hash-checked real NHL 20 copy. Observed data,
not an algorithm; nothing game-verified.

> **Correction 2026-09-26 02:35 (review `research-20260926-r107-r112-review.md` findings 1 and 2; raw results unchanged).** (1) R111b
> inspected only the `exhibitionteams` fields `conference`, `division`, `conferencegroup`, `divisiongroup`, `group` and `leaguegroup`
> (whichever exist on the title). It supports "none of these inspected fields encodes the observed partition", **not** "not recorded
> in any database field": no other table or field was searched. Inspected and uninspected locations are listed in
> [07-group-partitions-field-check-and-contract.md](07-group-partitions-field-check-and-contract.md). (2) The three groups are the
> shape of the shipped `swissschedule` on the four measured sources — a **stock-shaped generation input**, not a universal league rule
> and not evidence of how the game initializes a season. The AHL wording "replicate real seasons" below is withdrawn as provenance:
> those tables are **shipped irregular schedules for which this survey established no general generation rule**; import and explicit
> user quotas remain the available paths. Game order, dates and row counts remain observed data only.

Machine-readable: [`data/r111-schedule-provenance-and-swiss.json`](data/r111-schedule-provenance-and-swiss.json) (R13's `analyse` run
unchanged per source, plus every schedule table's row count), [`data/r111b-swiss-schedule.json`](data/r111b-swiss-schedule.json).

## Plain-language summary

Studio can generate simple "everyone plays everyone" schedules but refuses the NHL, AHL and Swiss formats because, as the developer's
ledger says, "the stock format is not established". For the NHL it was established on 20 September (handoff R13): a fixed number of games
per opponent class, by era. That record had one flaw — its NHL 20 row came from the file that is really NHL 19 — and the real NHL 20 now
confirms the same NHL quotas. The Swiss league had never been measured: its twelve clubs are split into three groups of four; each club
plays its three group-mates six times and the other eight clubs four times, for fifty games, with every pairing split evenly between home
and away, on every title checked. ~~The three groups are not stored anywhere in the database — all Swiss clubs share one group number — so
a generator would have to take the grouping from the shipped schedule or ask the user.~~ *(Corrected 2026-09-26 02:35:)* None of the six
team-table group fields that were inspected tells the groups apart — all Swiss clubs share one group number there; other places in the
database were not searched. A generator can take the grouping from the shipped schedule or ask the user.

## 1. Provenance correction (R13 row "NHL 18, 19, 20, 21")

| Source (SHA-256) | `nhlschedule` games; per-pair quotas (same division / same conference / other conference); home-away imbalance 0 / 1 | `ahlschedule` games; same-division pair counts |
|---|---|---|
| **real NHL 20** (`1e200478…`, CUSA15439 copy) | 1 271; 4 × 94 + 5 × 11 / 3 × 120 / 2 × 240; 334 / 131 | 1 150; 4 × 11, 6 × 19, 8 × 42, 10 × 7, 12 × 25, 14 × 1 |
| NHL 19 base (`265e4a68…`, `anchors/ps4/nhl20`) | identical to real NHL 20 | 1 150; 4 × 10, 6 × 22, 8 × 40, 10 × 17, 12 × 14, 14 × 2 — **a different season** |
| NHL 21 (`90ee1ec4…`) | identical | 1 150; 4 × 13, 6 × 19, 8 × 36, … |

R13's NHL quota row for 18–21 **stands**, now with a witnessed NHL 20 source. R13's AHL figures labelled NHL 20 are NHL 19's (the AHL
tables ~~replicate real seasons, so they~~ differ per title — irregular shipped schedules, no general rule established; wording corrected 2026-09-26 02:35); the conclusion "no class rule; import or user quotas" is unchanged. Other
per-title differences seen here (NHL 19 base vs real NHL 20): `echlschedule` 972 → 936 rows, `austrianschedule` 324 → 270, real NHL 20
adds empty `worldjuniorschedule` / `worldchampschedule` (as R88 recorded).

## 2. Swiss format (`swissschedule`; league resolved by the club HC Davos — full name on X360 Legacy; on PS4 the name fields are empty (dictionary-keyed) and the team abbreviation (`HCD` / `DAV`) resolved it)

| Source | league id · clubs | games · per club | pairs by games | home / away | groups of the 6-game pairs | DB group fields |
|---|---|---|---|---|---|---|
| X360 Legacy (`105d2186…`) | 6 · 12 | 300 · 50 on all 12 | 4 × 48, 6 × 18 | every pair even | 3 disjoint cliques of 4: {Ambri-Piotta, Biel, Lugano, Zug}, {Bern, Fribourg-Gottéron, Genève-Servette, Lausanne}, {Davos, Kloten, SCL Tigers, ZSC Lions} | conference / division / league group = 15 for all 12 |
| NHL 19 base, real NHL 20, NHL 21 | 6 · 12 | 300 · 50 | 4 × 48, 6 × 18 | even | 3 cliques of 4 (~~same slot structure~~ same shape, **different memberships**: NHL 19 base ≠ real NHL 20 = NHL 21, and both ≠ Legacy — corrected 2026-09-26 02:53, [07 §3 / §5](07-group-partitions-field-check-and-contract.md); names are dictionary keys on PS4) | group = 20 for all 12 |

**Observed rule (all four sources):** 12 clubs in three groups of four; 6 games (3 home, 3 away) against each group-mate, 4 games (2 / 2)
against each other club; 3 × 6 + 8 × 4 = 50. ~~The grouping is a property of the stock schedule only.~~ The grouping is observed in the stock schedule; none of the six inspected
`exhibitionteams` group fields encodes it (other locations unsearched; corrected 2026-09-26 02:35). Game order and dates are observed data.

## 3. Developer-facing contract (proposal; generator validation oracle)

- **NHL:** R13 §1–§4 as published (era quota tables; validate by recomputing the pair-class multiset, 82 games per club, imbalance ≤ 1);
  the NHL 18–21 row is confirmed on the real NHL 20.
- **Swiss:** "stock-shaped" generation = a partition of the league's 12 slots into three groups of four (import the partition from the
  title's stock `swissschedule`, or let the user supply one; ~~the database has no field for it~~ none of the inspected group fields encodes it — corrected 2026-09-26 02:35), 6 games within a group and 4 across, every
  pair split evenly; validate by recomputing: 50 per club, pair counts 4 × 48 + 6 × 18, groups = 3 cliques of 4, imbalance 0. Refuse when
  the league does not have exactly 12 members, unless the user supplies the quotas.
- **AHL:** unchanged (import or user quotas).
- Runtime acceptance of a generated table stays the existing P1-A card.

## 4. Coordination: the developer ledger lags the research

`docs/receipts/overnight-development-2026-09-24/LEDGER.md` (developer-owned; not edited here) still reads: row 1.2 schedule generation
"NHL / Swiss / AHL blocked — the stock division-weighted format is not established" (NHL: established by R13 on 2026-09-20; Swiss: this
handoff); ledger refresh row "2.1 art-id re-key … archive entry = name + djb2 hash word" (withdrawn by handoff 14 §F1); and "1.2 / 2.2
creation … PS4 18–21 non-NHL only" (withdrawn by 2026-09-24 handoff 06 §F2). Recorded as queue row C11 for the developer's next ledger
refresh.

```
python local/runs/overnight-research-2026-09-26/probes/r111_schedule_provenance_and_swiss.py docs/research/overnight-2026-09-26/data/r111-schedule-provenance-and-swiss.json
python local/runs/overnight-research-2026-09-26/probes/r111b_swiss_schedule.py docs/research/overnight-2026-09-26/data/r111b-swiss-schedule.json
```
Executed: r111 × 1 (its `stockleagues` stem match leaves Swiss unresolved — superseded by r111b's club-name resolution); r111b × 5
(encoding fix; PS4 abbreviation fallback; a NUL byte introduced by a shell edit, removed; group-graph and DB-group checks added).
