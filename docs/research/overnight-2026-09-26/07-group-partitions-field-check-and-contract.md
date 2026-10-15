# Handoff R113 (P7) — League 1.2 schedule groups: the Finnish, Austrian and Swiss partitions checked against every table of each title's database (no team-keyed column encodes them); a reproducible stock-partition export with validator and controls covering 19 distinct sources (all 27 main-database anchors, the real NHL 20, NHL 19 patch 56 and NHL 15 PS4 patch 00); partitions change from title to title and must never be carried across

Research lane, 26 September 2026, published 02:53 MDT (system clock). Review `research-20260926-r107-r112-review.md`, queue items 1–3.
Claim `research-p7-group-partitions`. Read-only over the anchors and the private fixture root (`E:\nhl-studio-fixtures`); nothing
written outside research paths. Observed data about shipped tables — not a rule the game follows, not an implementation contract, nothing
game-verified.

Machine-readable: [`data/r113-group-partitions.json`](data/r113-group-partitions.json) (per source: hashes, every league table's contract
or irregular summary, field scans, cross-source comparison), [`data/r113b-stock-partition-import-table.json`](data/r113b-stock-partition-import-table.json)
(compact import table, derived from the former without a database read).

## Plain-language summary

Some European leagues ship with schedules in which clubs are split into small groups and play group-mates more often. The review asked
whether the game records those groups anywhere in its database, and for a reliable way to export them. Every table of every title was
searched for a column that tells the groups apart; none that refers to teams does. A handful of accidental matches came from row numbers
lining up with team numbers, and they are listed so nobody mistakes them for a group field. That is a statement about what was searched,
not proof the groups are stored nowhere. The export now records, for each title, exactly which clubs form each group, a fingerprint of how
schedule positions map to clubs, and a checker that recomputes every pairing, total and home/away split. Deliberately broken copies —
a missing game, a club moved to the wrong group, a reshuffled team list, a bad slot, a team playing itself — are all caught. The groups
change between titles (the Swiss groups differ on almost every release, even where the team list is the same), so each title needs its own
export. Two shipped oddities turned up: NHL 15's Finnish table is two groups of seven with one game assigned to the wrong opponent, and
NHL 13's Czech table has one pairing whose home and away games are split three to one.

## 1. Coverage and identities

| Source (label) | `db` SHA-256 | provenance | aliases (byte-identical) |
|---|---|---|---|
| `xbox360-nhl07` | `67a00bea…` | anchor; gen6 — club attributes read from `stockteamtable` (its `exhibitionteams` has no league field) | `ps3-nhl07` (research-only copy) |
| `xbox360-nhl08` / `-nhl09` | `587204c3…` / `0f7c826b…` | anchors — **no league-stem schedule tables** (only `nhlschedule`, `ahlschedule`, `leaguescheduletable`) | PS3 08 / 09 |
| `xbox360-nhl10` … `-nhl15`, `-nhllegacy` | `e87b8c80…`, `461f8d64…`, `94b31ff2…`, `64460a15…`, `84e1bdff…`, `028ae06a…`, `105d2186…` | anchors | PS3 of the same title |
| `ps4-nhl15` | `ae9d1d56…` | anchor | — |
| `ps4-nhl15-patch00` | `abe36f58…` | CUSA00561 `patch_inxp00.big`; base meta (no table id missing) | — |
| `ps4-nhl16` / `17` / `18` | `7d95f1db…` / `de48b02b…` / `65eaa6e8…` | anchors | — |
| `ps4-nhl19-base` | `265e4a68…` | anchor `ps4/nhl19` | `anchors/ps4/nhl20` (**mislabeled** NHL 19 base) |
| `ps4-nhl19-patch56` | `61332e66…` | CUSA11126 `patch_inxp_56.big`; base meta | — |
| `ps4-nhl20-real` | `1e200478…` | CUSA15439 `inxp_00.big` (01.00, no patch archives held) | — |
| `ps4-nhl21` | `90ee1ec4…` | anchor | — |

**Not covered (named):** NHL Legacy Recompiled (no independent database anchor; not assumed equal to Legacy); PS4 NHL 16 / 17 / 18 / 21
patch databases and any NHL 20 patch (not held); X360 / PS3 title updates (not held). `ps4-nhl17 wcschedule` (World Cup, 17 rows) is a
tournament table and is left to the tournament package. Meta hashes per source are in the JSON.

## 2. Field check (review item 1) — inspected and uninspected locations

**Inspected, on every source with a partition league (35 league scans over 19 sources; 2 644 table scans, 126 784 column tests):** every
table of the database; in each, every column whose values include **every** member club's `index` (or every member's `artid`) was taken
as a key, and every other column of that table was tested for inducing exactly the partition (one value per member, equal inside a group,
different across groups). The six named group fields — `conference`, `division` (NHL 07–11) and `conferencegroup`, `divisiongroup`,
`leaguegroup` (NHL 12+) on the team table — hold **one value for all members of the league** on every source (e.g. Swiss 19 on NHL 12 / 13,
Finnish 21 and Austrian 62 on PS4 17–21), so none distinguishes the groups; a `group` column does not exist on these titles.

| Result | count | detail |
|---|---|---|
| columns encoding the partition through a **team-reference key** (`exhibitionteams.index`, `…teamid`, `…team`, `…artid`, `stockteamjerseys.teamartid`, 17 such key columns in all) | **0** | on every source and league |
| columns encoding it through a **non-team key** | 21 | all on the two-group Austrian tables of PS4 18 / NHL 19 base / patch 56: `stockteamequipmenttable` zone colours and `stockawardstable.year / league` (NHL 18), `tweetstexts.logo / tweetid`, `swedishschedule.day`, `czechschedule.day`, `austrianschedule.day` — every key is a **row number** (`index` 0–4 875 in the equipment table, whose team reference is its `team` column; tweet / award / schedule row ids). The Austrian groups are the first 6 (or 5) clubs by team `index` versus the rest (indices 252–257 / 258–263 on NHL 18), so any table whose row blocks change at the same row number matches by coincidence. Not group fields. |

**Uninspected (preserved alternatives):** relations where a club has several rows with different values (a group value must be single per
member to be tested); keys by schedule slot / rank, by name or abbreviation strings, or by any id that does not cover every member;
combinations of two or more columns; localization databases (`eng_us.db`, `nhl_*_*.db`); save files (roster / Franchise / Season
mirrors); PS4 attribute vaults (`gccollections` / `gcschema`); archive configuration files; executable / code tables. The result is
"no inspected column encodes the partition", not "the partition is stored nowhere". A schedule-derived export does not depend on settling
this.

## 3. Stock partitions (review items 2–3) — by club abbreviation; ranks and team `index` values in the JSON

Round robins (every pair the same count) are included in the export as one-member groups; all 54 conform.

| League table | Sources (identical partition by club) | model · quotas | groups |
|---|---|---|---|
| `swissschedule` | NHL 10 / 11 / 12 / 13 | 3 × 4 · 6 / 4 | {BERN, BIEL, FRI, GEN} {HCAP, HCL, SCL, ZUG} {HCD, KLOT, RAP, ZSC} |
| | NHL 14 | 3 × 4 · 6 / 4 | {BERN, BIEL, FRI, GEN} {HCAP, HCL, LAU, ZUG} {HCD, KLOT, RAP, ZSC} |
| | NHL 15 X360 / PS3, PS4 15, PS4 15 patch 00 | 3 × 4 · 6 / 4 | {BERN, HCAP, HCL, ZUG} {BIEL, FRI, GEN, LAU} {HCD, KLOT, RAP, ZSC} |
| | Legacy, PS4 16 | 3 × 4 · 6 / 4 | {BERN, FRI, GEN, LAU} {BIEL, HCAP, HCL, ZUG} {HCD, KLOT, SCL, ZSC} |
| | PS4 17 | 3 × 4 · 6 / 4 | {BERN, KLOT, SCL, ZSC} {BIEL, FRI, GEN, LAU} {HCAP, HCD, HCL, ZUG} |
| | PS4 18 | 3 × 4 · 6 / 4 | {BERN, FRI, GEN, LAU} {BIEL, KLOT, SCL, ZSC} {HCAP, HCD, HCL, ZUG} |
| | NHL 19 base, NHL 19 patch 56 | 3 × 4 · 6 / 4 | {BERN, RAP, SCL, ZSC} {BIEL, FRI, GEN, LAU} {HCAP, HCD, HCL, ZUG} |
| | real NHL 20, NHL 21 | 3 × 4 · 6 / 4 | {BERN, FRI, GEN, LAU} {BIEL, SCL, ZSC, ZUG} {HCAP, HCD, HCL, RAP} |
| `intlelitserienschedule` | NHL 07 | 3 × 4 · 6 / 4 | 12 clubs (ranks / names in the JSON) |
| `finnishschedule` | NHL 10, NHL 13 / 14 | 2 × 7 · **5 / 4** | {ASS, BLU, IFK, JOK, KAR, LUK, TPS} {HPK, ILV, JYP, KAL, PEL, SAI, TAP} |
| | NHL 11 / 12 | 2 × 7 · 5 / 4 | {ASS, HPK, ILV, KAR, LUK, TAP, TPS} {BLU, IFK, JOK, JYP, KAL, PEL, SAI} |
| | NHL 15 (all four copies) | **irregular** — nearest partition 2 × 7 · 5 / 4 with **one displaced game** (§4) | — |
| | Legacy, PS4 16 | 5 × 3 · 6 / 4 | {ASS, LUK, TPS} {BLU, HPK, IFK} {ILV, TAP, VAA} {JYP, KAL, KAR} {KOU, PEL, SAI} |
| | PS4 17, 18, NHL 19 base / patch 56, real NHL 20, NHL 21 | 5 × 3 · 6 / 4 | {ASS, LUK, TPS} {HPK, IFK, KAR} {ILV, TAP, VAA} {JUK, JYP, KAL} {KOU, PEL, SAI} |
| `austrianschedule` (PS4 18–21 only) | PS4 18, NHL 19 base / patch 56 | 6 + 6 · 6 / 4 | {BOLZ, DORN, GRAZ, INNS, KLAG, SZEK} {LINZ, SALZ, VIEN, VILL, ZAGR, ZNOJ} |
| | real NHL 20 | 5 + 6 · 6 / 4 | {BOLZ, DORN, GRAZ, INNS, SZEK} {KLAG, LINZ, SALZ, VIEN, VILL, ZNOJ} |
| | NHL 21 | 5 + 6 · 6 / 4 | {BOLZ, DORN, GRAZ, INNS, SZEK} {BRAT, KLAG, LINZ, SALZ, VIEN, VILL} |

Round-robin shapes across titles (new beyond R112): `swedishschedule` is 12 clubs × 5 per pair on NHL 10–15 (gen7 and PS4 15), 14 × 4 from
Legacy / PS4 16; `germanschedule` 15 × 4 on NHL 10 / 11, 14 × 4 afterwards; NHL 07 `intldelschedule` 14 × 4; `czechschedule` 14 × 4 and
`swedish2ndschedule` (PS4 18+) 14 × 4 throughout.

**Same mapping ≠ same groups.** The rank → club fingerprint of the Swiss league is identical on NHL 19 base and NHL 21 (`3616bbdc…`) and
on PS4 16 and 17 (`492b9dac…`), yet the groups differ. A partition belongs to one stock table; it is not a property of the team list.

## 4. Validator contract and controls (review item 2)

Per contract: members `{rank, index, artid, artabbr, abbrname, name}` (rank = position by team `index` within the league — Studio's slot
convention); `mapping_fingerprint` = SHA-256 of the canonical JSON `[[rank, index, artabbr], …]`; groups as ranks, as `index` values and as
abbreviations; quotas; `clique_complete` per group, **computed** from the schedule graph (all 35 partitions complete; the R112 `all_cliques`
placeholder is superseded); DB and meta SHA-256.

`validate(contract, current_members, games)` fails on: fingerprint mismatch (stale mapping); groups not covering ranks 0…n−1 exactly once;
any slot outside 0…n−1; any self game; any member with no game; **every unordered pair** (zero-count pairs included) off its quota;
per-team totals off; directional home / away counts off (even quota: exactly half each way; odd: differ by one).

| Check | result |
|---|---|
| stock tables conforming | **88 / 89** contracts; the exception is NHL 13 `czechschedule` (X360 = PS3): ranks 6 v 13 (HC VERVA Litvínov v PSG Zlín) play 3 home / 1 away — a shipped imbalance, reported not repaired |
| controls detected (dropped game, swapped membership, stale mapping, out-of-range slot, self game) | **89 / 89** contracts, every control (swapped membership only where a contract has more than one multi-club group) |
| clique completeness | computed, 35 / 35 partitions complete |

Outliers with a near-partition (diagnostic `nearest_partition`, not a model):

- **NHL 15 `finnishschedule`** (X360 / PS3 / PS4 15 / PS4 15 patch 00, 406 rows, 14 clubs): the pairs with ≥ 5 games form two complete
  groups of seven; exactly two pairs deviate — ranks 0 v 3 play **6** (expected 5) and ranks 3 v 9 play **3** (expected 4); totals 57 / 58 / 59
  (rank 9 / all others / rank 0); one same-day double booking (R114). A single game assigned to the wrong opponent.
- **NHL 07 `intlsmliigaschedule`** (13 clubs, 364 rows): groups 5 / 4 / 4 at 6 / 4 with four pairs at **8** (ranks 1 v 12, 2 v 5, 3 v 9, 4 v 11)
  — no two-quota model; import or user quotas.
- Junior leagues and ECHL: 49–202 deviating pairs from any partition — no small-deviation structure (irregular, import or user quotas).

## 5. Correction to R111b (handoff 05 §2)

Handoff 05 wrote "3 cliques of 4 (same slot structure …)" for NHL 19 base, real NHL 20 and NHL 21. The **shape** is the same; the
**memberships are not**: NHL 19 base's groups differ from NHL 20 / NHL 21's (table above), and Legacy's differ from both. A dated note
is placed on handoff 05.

## 6. Developer-facing boundary

The import table is a research export: per source, stock partition + quotas + mapping fingerprint + validator. It supports a
"stock-shaped" option only on the exact source whose hashes it names; any other title, patch or edited team list needs its own export.
The generator handoff (bounded contract, capacity, dates / order, refusals) is [08-schedule-generator-handoff.md](08-schedule-generator-handoff.md).
No implementation contract is claimed and nothing here says how the game initializes a season from these tables.

```
python local/runs/overnight-research-2026-09-26/probes/r113_group_partitions.py <out.json>
python local/runs/overnight-research-2026-09-26/probes/r113b_partition_import_table.py docs/research/overnight-2026-09-26/data/r113-group-partitions.json <out.json>
```
Both executed; outputs published through `safe_patch.py put`. Runtime ≈ 100 s (reads 29 database files).
