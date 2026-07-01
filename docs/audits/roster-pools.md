# Audit — roster-pool model: free agents & hidden (Spike S1, VERIFIED structure)

> **Status: structure VERIFIED** by decode of `anchors/ps3/nhllegacy/databases/nhlng.db` (PS3 NHL
> Legacy), 2026-06-30, via `RosterModel::build` + a direct `ZBac` pass. The **pool-membership
> mechanism is confirmed**; the **free-agent-vs-hidden sub-label is still candidate** (needs operator
> confirmation — Spike S4 overlap). Resolves the open pool question for Phase 4.2 `free_agent`.

## Question

How are the three NHLViewNG pools — **main roster / free agents / hidden** (`LoadRoster` /
`LoadFreeAgentsRoster` / `LoadHiddenRoster`) — represented in the bytes? Candidates were: a sentinel
"free-agent" team in `eGlu.dXSB` (B), a status code (C), or slot-presence (A).

## Finding — pool membership = slot presence (model A), at the `ZBac`-index level

Decoded counts on the anchor:

- **`kOtt` teams: 252; `ubPc` bios: 5632; `eGlu` slots: 5532** (5531 populated).
- **No oversized "holding" team.** Largest team roster = 35 slots; team-size buckets are 11–25 (146
  teams) and 26–40 (76 teams). → the **sentinel-team model (B) is FALSE** — free agents are *not*
  parked on a special `kOtt` team.
- **`ZBac` index rows: 6087** (more than the 5632 bios → some bios carry multiple index rows).
  - **5531** `ZBac.vfEq` are referenced by an `eGlu` slot (placed/rostered).
  - **556** `ZBac` rows are **indexed but referenced by no slot** — the unplaced pool.
  - **0** bios have *no* `ZBac` row → "hidden" is **not** an unindexed-bio mechanism; every bio is
    indexed.
- **522** bios occupy no populated slot (slightly < 556 because a bio with multiple index rows can be
  placed via one and unplaced via another).

**Conclusion:** a player is "off-roster" (free agent / hidden) **iff its `ZBac` index row is
referenced by no `eGlu` slot.** Pool membership is structural (slot presence), confirming model A — but
at the index→slot hop, not bio→slot. There is **no sentinel team and no unindexed-bio pool.**

## Free-agent vs hidden sub-label — CANDIDATE (`ZBac.RREB`), not yet confirmed

`ZBac.RREB` (2-bit roster-status enum) distribution:

| RREB code | placed rows | unplaced rows |
|---:|---:|---:|
| 0 | 5487 | 361 |
| 1 | 0 | 180 |
| 2 | 0 | 13 |
| 3 | 44 | 2 |

Reading:
- Among **placed** players RREB is **only 0 or 3** → `0` = active/dressed, `3` = a bench/scratch/minors
  status (label unconfirmed).
- Codes **1 and 2 appear only on unplaced players** (180 and 13) → the strongest **candidate** that
  the off-roster pool is itself sub-divided by RREB (e.g. `1` = free agent, `2`/`3` = hidden/other),
  but the value→label mapping is **NOT confirmed** and must come from operator/NHLViewNG ground truth
  (Spike S4). Do not assert these labels as in-game-true (CLAUDE.md rule 3).

## Implications for Phase 4.2 (`free_agent`) and 4.1 (`hide`/`unhide`)

- **`free_agent` is fundamentally a slot occupancy change, not a field flag** (consistent with the
  [`toWX`-block finding](towx-blocks.md)):
  - *release to pool* = clear the player's slot `XSWT` (the slot becomes the empty kind we observe),
    optionally set the pool `RREB` code.
  - *sign from pool* = place the player's `ZBac.vfEq` into a free slot in the target team's block —
    blocks are full, so this generally needs block growth (capacity normalization, Spike S3 / 4.4).
- **`hide`/`unhide`** is most likely the same unplace/place mechanism distinguished by the (candidate)
  `RREB` code rather than a separate field — fold into the pool ops once the RREB labels are confirmed,
  rather than scoping a distinct field op.
- The on-team `set_roster_status` op already writes `ZBac.RREB`; the same field carries the pool
  sub-status — so the plumbing exists, only the label semantics are open.

## Reproduction

`RosterModel::build` over the anchor for slots; a direct `ZBac` pass reading `vfEq`/`ykFq`/`RREB`;
partition `ZBac` rows by whether their `vfEq` ∈ {slot `index_key`}; histogram `RREB` on each side.
(Spike example removed after capture.)
