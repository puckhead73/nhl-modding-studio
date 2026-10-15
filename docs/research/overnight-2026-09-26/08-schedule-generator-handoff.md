# Handoff R114 (P8) — League 1.2 schedule generator handoff (bounded): stock-shaped versus user-supplied quotas, exact row capacity, team mapping, date / order facts, refusal conditions and the validation oracle; new facts — the PS4 Austrian table is **two-phase** (its `round` field marks a final within-group block, so "round is always 0" does not hold there), stock tables are not always in date order, and two shipped tables contain same-day double bookings

Research lane, 26 September 2026, published 02:55 MDT (system clock). Review `research-20260926-r107-r112-review.md`, queue item 4.
Claim `research-p7-group-partitions` (same package line). Inputs: R113 export, R114 layout probe (new), R13 NHL quotas, and a read-only
reading of Studio's current generator (`crates/studio-domain/src/schedule_edit.rs`, module header and `generation_options` /
`plan_generation`; not edited). Proposal only — not an implementation contract; no claim that a generated table initializes a mode.

Machine-readable: [`data/r114-schedule-table-layout.json`](data/r114-schedule-table-layout.json) (per distinct source and league table:
`current_records` / `max_records`, fields and bit widths, `round` / `status` value sets, `index` = position, season-date order, first / last
date, distinct dates, same-day double bookings).

## Plain-language summary

Studio can already build a schedule in which every pair of clubs meets equally often, and it never changes how many games a table holds.
This handoff says what it would take to also build the grouped formats (Swiss, Finnish, Austrian and the NHL 07 Swedish table) either in
the shipped shape of one exact title or from groups and game counts the user chooses, and when to refuse. The numbers must add up
exactly to the games already in the table. Three details matter. The PS4 Austrian table holds everyone playing everyone
four times, then a final block of two more games inside each group, and it tags that block with a `round` value Studio currently always
writes as zero — what the tag does in the game is not verified (wording corrected 2026-09-26 03:43). Some shipped tables are not sorted by date, and two contain a club playing twice on one day, so a checker must not reject
the stock table itself for those. The NHL quotas from 20 September give the number of games per kind of opponent, but not which pairs
get the extra games or on what dates; that remains a choice, not a rule. Whether the game plays a generated table correctly is for the
user to check in a game.

> **Correction 2026-09-26 03:43 (review `research-20260926-r113-r118-review.md` item 3):** matrix arithmetic in §2 corrected (upper-triangle sum; zero diagonal,
> non-negative integers, symmetry). Stock-source anomalies (NHL 13 Czech 3 / 1 pair, NHL 15 Finnish displaced game, double bookings) are
> facts about shipped tables and stay distinct from the validity rules for generated rows. The Austrian `round = 1` block is measured;
> its runtime phase semantics are unverified. The data and validator support further design; they do not authorize an implementation.

## 1. What exists (read from Studio, not changed)

Balanced round robin for chosen participants; slot = rank within the league by the team table's `index`; slot-map fingerprint, stale
mapping refused; exactly `current_records` rows (rows involving non-participants — the All-Star row — kept verbatim); `round` and `status`
written as 0; yearless dates, matchdays spaced from the current table's span; import (`nhl-studio-schedule/1`, CSV / JSON) with a
destination mapping and fingerprint; export.

## 2. Two quota sources

| Option | Input | Where valid | Rows required |
|---|---|---|---|
| **Stock-shaped** | the R113 contract of **this exact source** (DB SHA-256 match): groups, within / across quotas, mapping fingerprint | only when the loaded database's league members produce the same fingerprint as the contract; otherwise refuse and offer the user-supplied option | `within · W + across · (P − W)`, with `P = n(n−1)/2`, `W = Σ g(g−1)/2` over the groups |
| **User-supplied** | a partition of the current members (any group sizes) plus within / across counts — or, as a general form, a symmetric per-pair count matrix | any league table with the game layout | same formula; for a matrix, total games = the **upper-triangle sum** (half the full off-diagonal sum), with zero diagonal, non-negative integer counts and symmetry required (corrected 2026-09-26 03:43) |

Stock values that satisfy the formula (R113): Swiss / NHL 07 Elitserien 3 × 4 · 6 / 4 → 6·18 + 4·48 = **300**; Finnish 5 × 3 · 6 / 4 → 6·15 +
4·90 = **450**; Finnish 2 × 7 · 5 / 4 (NHL 10–14) → 5·42 + 4·49 = **406**; Austrian 6 + 6 · 6 / 4 → 6·30 + 4·36 = **324**; Austrian 5 + 6 → 6·25 +
4·30 = **270**; round robins `k · P` (12 × 5 → 330, 14 × 4 → 364, 15 × 4 → 420). Irregular tables (OHL, QMJHL, WHL, ECHL, NHL 15 Finnish, NHL 07
SM-liiga) have **no stock-shaped option** — import or a user-supplied matrix.

**NHL** stays separate: R13's era quotas (NHL 18–21 row confirmed on the real NHL 20) fix games per opponent class, not which pairs take
the fifth division game or the dates. A quota-satisfying NHL table is a generated allocation, not a reproduction of the stock algorithm.

## 3. Capacity and layout (R114)

- **Row count:** every gen7 league table is packed (`current_records = max_records`); PS4 tables carry spare rows (European league tables
  +60 on PS4 15–17, +30 on PS4 18–21; CHL / ECHL +60; NHL 19 base / patch 56 `whlschedule` 748 / 852) whose "active" representation is not
  established — keep Studio's rule: exactly `current_records` rows.
- **Slot widths (R114):** home / away are **4-bit** on every European league table measured (Swedish, Swedish 2nd, German, Czech, Swiss,
  Finnish, Austrian — at most 16 clubs, slots 0–15), 5-bit on the CHL / ECHL tables and the three NHL 07 international tables, 6-bit on
  `echlschedule` of the real NHL 20 and NHL 21; refuse n above 2^width.
- **`index` = row position** on every table measured.
- **`round`:** 0 on every row of every league table **except `austrianschedule` on PS4 18–21: 0 / 1**. The `round = 1` rows are a
  contiguous final block (rows 264–323 on PS4 18 / NHL 19, 220–269 on NHL 20 / 21) containing only within-group games, two per within pair
  (10 per club in the six-club group, 8 in the five-club group); the `round = 0` block is a four-game round robin of all clubs (4 per
  pair). The runtime meaning of `round` is not established. Studio's "written as 0 (every stock row)" note does not hold for this table.
- **`status`:** 0 on gen7 and PS4 15; the field is absent on PS4 16+ (Studio already writes it only when present).
- **Date order:** rows are in season-date order except `czechschedule` NHL 11 and NHL 13 and `qmjhlschedule` real NHL 20 / NHL 21 (no
  evidence that order matters at run time). Seasons start in stored month 8 (September; ECHL 9) and end in stored months 1–3.
- **Same-day double bookings in stock:** NHL 13 `swissschedule` (X360 = PS3) 21 slot-dates; NHL 15 `finnishschedule` 1 (the displaced game).
  Every other league table: 0.
- **Group games in the season:** Swiss and Finnish within-group games are spread through the season (at most 12 of 108 / 5 of 210 fall after
  the last cross-group game); only the Austrian table has a marked final phase.

## 4. Proposed refusal conditions (for a partition generator)

1. Stock-shaped: the source hash or the mapping fingerprint differs from the contract (edited team list, patch, other title).
2. The quota formula does not equal `current_records − kept rows` (no row-count change, as today).
3. A partition that does not cover every participant exactly once, or n above the slot width.
4. Austrian (PS4 18–21): either write the two-phase shape explicitly (phase 0 `round = 0` four-game round robin; phase 1 `round = 1`
   two-game within-group round robin dated after phase 0) **or** refuse the stock-shaped option until that choice is made; writing all
   rows with `round = 0` changes a stock value whose meaning is unknown.
5. Odd quotas (5 per pair on Finnish 2 × 7 and the 12 × 5 Swedish tables): home / away must differ by exactly one per pair (the stock tables
   do); the generator states which club gets the extra home game.
6. Dates past the season boundary (as today).

## 5. Validation oracle

R113 `validate(contract, members, games)` — fingerprint, partition cover, every unordered pair including zero-count pairs, participants,
self games, slot range, per-team totals, directional home / away — plus these R114 layout checks: `index` = position, `round` per the rule
above, `status` only where the field exists, no same-day double booking **among generated rows** (a warning, not a refusal, for pre-existing
stock rows, as Studio's touched-rows rule already does). Controls: the five R113 controls (dropped game, swapped membership, stale mapping,
out-of-range slot, self game) must fail validation.

## 6. Still outside this handoff

Runtime acceptance (the existing P1-A card: a generated table loads and plays a season correctly — user's evidence only); how a mode copies
a source table into `leaguescheduletable`; the stock date-allocation algorithm; tournaments (`championsschedule`, `spenglerschedule`,
`wcschedule`, `worldchamp` / `worldjunior`; next package); any title or patch not listed in R113 §1.

```
python local/runs/overnight-research-2026-09-26/probes/r114_schedule_table_layout.py <out.json>
```
Executed once; output published through `safe_patch.py put`.
