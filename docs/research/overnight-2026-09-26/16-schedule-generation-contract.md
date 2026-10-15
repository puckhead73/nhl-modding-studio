# Handoff R123 (P19) — League 1.2 grouped / pair-matrix schedule generation: explicit request and proposal invariants, refusal codes, a research-only reference validator with fixed vectors, and a replay on all 144 shipped league tables

Research lane, 27 September 2026, published 04:59 MDT (system clock). Prompt
[`research-20260926-after-receipt3.md`](../../plans/research-20260926-after-receipt3.md) package 1. Claim
`research-p19-schedule-generation-contract`. Builds on handoff 07 (R113 partitions / validator), handoff 08 (R114 layout, refusal list; 00c
matrix correction) and the developer ledger's current row "schedule generation — grouped European formats: **proposal**, needs an
implementation contract". Read-only; Studio's `schedule_edit.rs` read, not edited.

Machine-readable: [`data/r123-schedule-generation-contract.json`](data/r123-schedule-generation-contract.json) (27 vectors with expected and
actual codes and messages; per shipped table: participants, kept rows, round tags, slot width, request / proposal failures, stock facts).
Probe: `local/runs/overnight-research-2026-09-26/probes/r123_schedule_generation_contract.py` (the module docstring is the normative
statement of the invariants below).

## Plain-language summary

Handoff 08 described how grouped schedules (Swiss, Finnish, Austrian and similar) could be generated, but it stayed a proposal because the
rules were not written as checks a developer could run. This package writes them as two lists of numbered checks: one for what the user
asks for (groups and game counts, or a full table of games per pair) and one for the rows a generator produces. Each check comes with a small
example that passes and one that fails for exactly that reason. The same checks were run on every league schedule the games ship, 144
tables: none fails. Where the shipped tables do something a new schedule should not (uneven home / away splits, a club playing twice in a
day, rows out of date order) the checker reports it as a fact about the shipped table, not as damage. It still does not say which dates the
game would choose, how the game treats the Austrian final block, or how NHL divisions should be paired.

## 1. Request contract (inputs)

| Input | Meaning | Source in Studio today |
|---|---|---|
| `source` | database SHA-256 + table | the open project |
| `slot_map` | current members as slots (league rank by the team table's `index`) + fingerprint | `schedules::slot_map` |
| `mapping_fingerprint` | fingerprint the request was built against | as in the import format |
| `participants` | slots that play the generated rows; every row touching another slot is **kept verbatim at its position** | `GenerationConfig.participants` |
| `quota` | `partition` {groups, within, across} **or** `matrix` {counts: symmetric n × n over the participants} | new |
| `phases` | optional ordered blocks, each {round tag, quota}; later blocks dated after earlier ones (PS4 18–21 Austrian shape) | new |
| `odd_home` | who hosts the extra game of an odd pair count: `lower_slot`, `higher_slot` or an explicit per-pair map | new (handoff 08 §4 item 5) |
| `stock` | only for the stock-shaped option: the R113 contract's DB hash, fingerprint and whether the source is two-phase | R113 export (handoff 07) |
| `table` | `current_records`, kept rows, home / away bit width, whether `status` exists | `ScheduleModel` / layout |

Dates, matchday spacing and the season boundary stay as Studio already handles them; this contract adds only R9 / R10 below.

## 2. Invariants and refusal codes

**Request (refuse before building rows):**

| Code | Invariant | Refusal text (suggested) |
|---|---|---|
| Q1 | matrix square over the participants, zero diagonal | "the games table must list every participant once and no club against itself" |
| Q2 | every count a non-negative integer | "game counts must be whole numbers, zero or more" |
| Q3 | matrix symmetric | "A–B and B–A must hold the same number of games" |
| Q4 | total = upper-triangle sum over all phases = `current_records − kept rows` | "N games requested, the table holds M active rows; Studio does not change the row count" |
| Q5 | partition covers every participant exactly once, no empty group | "every participating club must be in exactly one group" |
| Q6 | participants distinct, ≥ 2, members of the current slot map | as today |
| Q7 | every slot fits the home / away width (4 b European tables, 5 b CHL / ECHL / NHL 07 international, 6 b ECHL on genuine NHL 20 / NHL 21) | "this table can address at most 2^w clubs" |
| Q8 | request fingerprint = current slot map fingerprint | as today (stale mapping) |
| Q9 | stock-shaped option only for the exact DB hash **and** fingerprint of its R113 contract | "the shipped layout belongs to <source>; enter groups and counts instead" |
| Q10 | a two-phase source (PS4 18–21 `austrianschedule`) is requested as phases, never flattened to one round tag | "this table ships a tagged final block; generate both phases or use your own counts" |
| Q11 | odd counts require an `odd_home` rule | "choose which club hosts the extra game" |
| Q12 | every participant has at least one game | "<club> would have no games" |

**Proposal (validate the built rows before planning the write):**

| Code | Invariant |
|---|---|
| R1 | exactly `current_records` rows |
| R2 | kept rows unchanged in value and position |
| R3 | `index` = row position |
| R4 | per phase, every unordered pair's count equals the request, including zero-count pairs |
| R5 | even count → home = away; odd count → the declared host has exactly one more home game |
| R6 | generated rows: no self game; both slots are participants |
| R7 | generated rows carry the round tag of their phase (single-phase requests: 0, as today) |
| R8 | `status` written (0) exactly where the field exists (absent on PS4 16+) |
| R9 | no slot plays twice on one date among generated rows (pre-existing kept rows: warning) |
| R10 | every row of phase k + 1 is dated after every row of phase k |

## 3. Vectors (executed 27 Sep; 27 / 27 as expected)

- **Valid:** V1 partition 2 × 2, within 2 / across 1, odd pairs hosted by the lower slot (8 rows); V2 matrix over 3 clubs
  `[[0,2,1],[2,0,1],[1,1,0]]` with a kept All-Star row at position 4 and explicit odd hosts (5 rows); V3 two-phase request — phase 0 round
  robin twice (12 rows, tag 0), phase 1 within-group twice (4 rows, tag 1, dated after), no `status` field.
- **Failing, one invariant each:** Q1 diagonal 1; Q2 a −1 entry and a 1.5 entry; Q3 asymmetric; Q4 `current_records` 6 for 4 games + 1 kept;
  Q5 a group missing a club; Q6 a participant outside the slot map; Q7 a 1-bit field; Q8 old fingerprint; Q9 stock option naming another
  DB hash; Q10 the two-phase request flattened to within 4 / across 2; Q11 odd counts without `odd_home`; Q12 a participant with an all-zero
  row; R2 kept row's day changed; R3 an index 9 at position 3; R4 one game moved to another pair; R5 an even pair 2–0 and an odd pair hosted
  by the undeclared side; R8 `status` on a PS4 16+ row; R9 two games of slot 0 on one date; R10 a phase-1 game dated inside phase 0.
- **Failing with a necessary second code** (recorded as expected): R1 a missing row also changes a pair count (R4); R6 a self game (R4);
  R7 a wrong round tag moves the row out of its phase (R4).

## 4. Replay on the shipped tables (stock mode)

17 distinct sources carry league schedule tables resolved by R113 (of its 19 distinct sources, NHL 08 / 09 resolve none); 144 tables — 54 round robins, 35 stock
partitions (including the five PS4 18–21 / genuine NHL 20 / NHL 19 patch 56 Austrian two-phase tables, requested as two phases), 55
irregular. Each table was checked as a proposal against a matrix request built from its **own** pair counts (participants = members with
more than two games, Studio's default; no league table has kept rows): **0 request failures, 0 proposal failures.** Facts about shipped data
(warnings, never refusals of the stock table):

| Fact | Tables |
|---|---|
| uneven home / away split for an even count, or an odd split other than ± 1 (R5 as a fact) | 24: every irregular `ohl` / `qmjhl` / `whl` / `echl` table where it occurs, the NHL 15 / PS4 15 irregular `finnishschedule`, and NHL 13 `czechschedule` (the 3 / 1 pair) |
| same-day double booking (R9 as a fact) | NHL 13 `swissschedule`; `finnishschedule` of X360 NHL 15, PS4 NHL 15 and its patch 00 |
| rows not in season-date order | `czechschedule` NHL 11 and 13; `qmjhlschedule` genuine NHL 20 and NHL 21 |
| Austrian round-1 block dated after round 0 (R10 holds) | all five two-phase tables |

So the checks accept the shipped data; they refuse only **new** rows with these properties. Refusing new input is not a claim that shipped
data is corrupt.

## 5. What is established and what is not

- **Established (static):** the invariants above hold on every shipped league table (with the listed facts); the stock-shaped totals of
  handoff 08 §2 follow from Q4; slot widths per table (R114); the Austrian two-phase shape and its tag.
- **Not established:** which dates the game would choose (Studio's spacing is its own choice); the runtime meaning of `round = 1` (Q10
  keeps the tag rather than guessing); any pair rule for the NHL division-weighted format (R13 quotas fix games per opponent class, not
  which pairs take the extra division game — a user-supplied matrix satisfying the quotas is allowed under this contract, a "stock NHL"
  option is not); tournaments; how a mode copies a source table into `leaguescheduletable`; runtime acceptance (card P1-A).
- **Developer-consumable scope:** a partition / matrix / phases generator for the European league tables and user-supplied matrices on any
  league table, refusing with Q / R codes; the stock-shaped option only under Q9 / Q10. The NHL table stays round-robin / import / user
  matrix. This is a research contract for coordinator acceptance; it is not accepted by being published.

```
python -B local/runs/overnight-research-2026-09-26/probes/r123_schedule_generation_contract.py docs/research/overnight-2026-09-26/data/r113-group-partitions.json <out.json>
```
Executed on the registered sources (anchors + private fixture copies of the genuine NHL 20, NHL 19 patch 56, NHL 15 PS4 patch 00).
