# Handoff R112 (P6) — every remaining league schedule table classified on the real NHL 20, NHL 21 and X360 Legacy: four leagues are plain four-game round robins (SHL, Allsvenskan, DEL, Czech: 14 clubs, 52 games), three follow a **group rule** — 6 games inside a group, 4 across (Swiss 3 × 4 clubs, Finnish 5 × 3, Austrian 6 + 5) — and four are ~~irregular real-season replicas~~ **shipped irregular schedules with no general generation rule established by this survey (wording corrected 2026-09-26 02:35)** (ECHL, OHL, QMJHL, WHL), completing the schedule-generator evidence begun by R13 and R111

Research lane, 26 September 2026, 01:24 → 01:26 MDT (real clock). Receipt next task. Claim `research-p6-league-schedule-formats`. Read-only
over the hash-checked real NHL 20 copy (`1e200478…`), the NHL 21 anchor (`90ee1ec4…`) and the X360 Legacy anchor (`105d2186…`). League
resolution: on PS4 by the title's own `stockleagues.name` key; on Legacy (no `stockleagues`) by one named member club. Observed data, not an
algorithm; nothing game-verified.

> **Correction 2026-09-26 02:35 (review `research-20260926-r107-r112-review.md` findings 1 and 2; raw results unchanged).** (1) "Swiss
> groups not in any DB field" is withdrawn: R111b inspected six `exhibitionteams` group fields only, so the result is "none of the inspected
> fields encodes the partition". The Finnish / Austrian check and the list of inspected versus uninspected locations are in
> [07-group-partitions-field-check-and-contract.md](07-group-partitions-field-check-and-contract.md). (2) Uneven opponent counts neither
> prove that a table copies a real season nor rule out generation: "real-season replica" below reads **shipped irregular schedule, no
> general generation rule established by this survey**; import and explicit user quotas remain available. The 4 / 6-game partitions
> ("group rule") are **stock-shaped generation inputs on the measured sources**, not universal league rules and not evidence of game
> initialization behaviour. Dates, game order and row counts stay observed data under the existing schedule contract.

Machine-readable: [`data/r112-league-schedule-formats.json`](data/r112-league-schedule-formats.json) (probe `r112_league_schedule_formats.py`).

## Plain-language summary

The rest of the leagues' shipped schedules fall into three kinds. Four European leagues simply have every club play every other club four
times, split evenly home and away — exactly what Studio's existing round-robin generator can already produce. Three leagues use groups:
clubs in the same small group meet six times, everyone else four times (Switzerland in three groups of four, Finland in five groups of three,
Austria in a group of six and a group of five). ~~The junior leagues and the ECHL copy real seasons with uneven pairings, so, like the AHL,
they can only be imported or given explicit counts. The Swiss groups are not stored in any database field; whether the Finnish and
Austrian groups are was not checked.~~ *(Corrected 2026-09-26 02:35:)* The junior leagues and the ECHL ship uneven pairings for which this
survey found no general rule; like the AHL, importing them or giving explicit counts are the available paths. The groups are what the
shipped schedules show on the measured titles, not a rule the game is known to follow; for where the groups are (and are not) recorded,
see handoff 07.

## 1. Classification (identical on real NHL 20 and NHL 21; Legacy where the table exists)

| Table (league id; resolved by) | clubs | games · per club | pair counts | home / away | class |
|---|---|---|---|---|---|
| `swedishschedule` (2; `elitserien` / club Frölunda) | 14 | 364 · 52 | 4 × 91 | all even | **round robin, 4 per pair** (PS4 20 / 21, Legacy) |
| `swedish2ndschedule` (8; `Allsvenskan`) | 14 | 364 · 52 | 4 × 91 | even | round robin ×4 (PS4 only) |
| `germanschedule` (4; `TXT_LEAGUE_DEL` / club Eisbären) | 14 | 364 · 52 | 4 × 91 | even | round robin ×4 |
| `czechschedule` (5; `TXT_LEAGUE_O2_EXTRALIGA` / club Sparta) | 14 | 364 · 52 | 4 × 91 | even | round robin ×4 |
| `swissschedule` (6) | 12 | 300 · 50 | 4 × 48, 6 × 18 | even | **group rule**: 3 groups of 4 (R111b: ~~groups not in any DB field~~ not encoded by the six inspected `exhibitionteams` group fields) |
| `finnishschedule` (3; `SMLIIGA` / club Tappara) | 15 | 450 · 60 | 4 × 90, 6 × 15 | even | **group rule**: the 6-game pairs form 5 components of 3 (15 edges = 5 × 3, complete groups); 2 × 6 + 12 × 4 = 60 |
| `austrianschedule` (7; `TXT_LEAGUE_EBEL`) | 11 | 270 · 48 (5 clubs) / 50 (6) | 4 × 30, 6 × 25 | even | **group rule**: components of 6 and 5 (25 edges = 15 + 10, complete groups); 5 × 6 + 5 × 4 = 50, 4 × 6 + 6 × 4 = 48 (PS4 only) |
| `echlschedule` (15) | 26 | 936 · 72 | 1–18 per pair | imbalance 0–4 | ~~real-season replica~~ shipped irregular, no rule established |
| `ohlschedule` (11 PS4 / 9 Legacy) | 20 | 680 · 68 | PS4: 1–8; Legacy: 2 / 4 / 6 / 8 | PS4 even on 184 / 190; Legacy all even | ~~real-season replica~~ shipped irregular, no rule established (Legacy's even multiples still vary by pair — no class rule) |
| `qmjhlschedule` (12 / 10) | 18 | 612 · 68 | 2–10 | mostly even | ~~real-season replica~~ shipped irregular, no rule established |
| `whlschedule` (13 / 11) | 22 | 748 · 68 (PS4) / 792 · 72 (Legacy) | 1–12 | uneven | ~~real-season replica~~ shipped irregular, no rule established |

No table has slots beyond its league's member count. Game order and dates remain observed data.

## 2. Developer-facing contract (proposal; validation oracle = recompute the pair counts above)

- **Round robin ×4:** SHL, Allsvenskan, DEL, Czech — the existing balanced round-robin generator with k = 4, even home / away.
- **Group rule:** Swiss (3 × 4), Finnish (5 × 3), Austrian (6 + 5): a partition into groups (imported from the stock table or supplied by the
  user), 6 games within a group, 4 across, even home / away; validate by recomputing per-club totals and the two pair counts. ~~Whether the
  Finnish / Austrian groups are recorded in a DB field was not checked (Swiss: not recorded).~~ Field check and the reproducible
  stock-partition contract: handoff 07 (corrected 2026-09-26 02:35). This proposal is not an implementation contract.
- **~~Replicas~~ Irregular shipped schedules:** ECHL, OHL, QMJHL, WHL — import or user quotas (as AHL); no general rule established.
- NHL: R13 era quotas (confirmed on the real NHL 20 in handoff 05). Runtime acceptance stays the P1-A card.

```
python local/runs/overnight-research-2026-09-26/probes/r112_league_schedule_formats.py docs/research/overnight-2026-09-26/data/r112-league-schedule-formats.json
```
Executed once. The probe's `all_cliques` field is a placeholder (always null); completeness of the groups is shown by the edge-count
arithmetic in the table, not by that field.
