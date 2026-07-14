# Audit — roster-pool model: free agents & hidden (Spike S1, VERIFIED structure)

> **Status: structure VERIFIED** by decode of `anchors/ps3/nhllegacy/databases/nhlng.db` (PS3 NHL
> Legacy), 2026-06-30, via `RosterModel::build` + a direct `ZBac` pass. The **pool-membership
> mechanism is confirmed** (slot presence, model A). Resolves the open pool question for Phase 4.2
> `free_agent`. **Update (Spike S4, 2026-07-13):** `ZBac.RREB` — previously read here as a
> "roster-status" sub-label — is actually the **player-type** discriminator (`TNHLPlayerType`); see
> the resolved section below. It is unrelated to pool membership.

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

## `ZBac.RREB` is PLAYER TYPE, not roster status — RESOLVED (Spike S4 follow-up, 2026-07-13)

Earlier revisions of this audit read `ZBac.RREB` as a "roster status" enum. **That was wrong.**
`ZBac` (binary `caBZ`) is EA's **"exhibitionplayers" index table** — `index` (`vfEq`), `playerid`
(`ykFq`), and **`table` (`RREB`, 2-bit, meta name literally "table")**. `RREB` is the **player
source/type** discriminator, confirmed three ways ([player-type.toml](../../defs/enums/player-type.toml)):

| RREB code | placed | unplaced | pointed-to bio `active` | `TNHLPlayerType` |
|---:|---:|---:|:--|:--|
| 0 | 5487 | 361 | **all active=1** | `plrBuiltIn` → Built-In |
| 1 | 0 | 180 | **all active=0** | `plrUserCreated` → User-Created |
| 2 | 0 | 13 | active=1 | `plrSpecial` → Special |
| 3 | 44 | 2 | active=1 | `plrDeveloper` → Developer |

1. **EXE RTTI** (unit `uDatabaseTypes`) has exactly one 4-value `[0..3]` enum that fits the 2-bit
   field: `TNHLPlayerType {plrBuiltIn, plrUserCreated, plrSpecial, plrDeveloper}`. The roster-status
   (`TNHLRosterPlayerStatus`) and contract-status enums are **3-valued** and cannot fit.
2. **Meta name "table"** = *which player table the index points into* = the player's source.
3. **Data:** the code correlates **1:1 with the pointed-to bio `active` flag** (code 0 = all active
   built-ins; code 1 = all-inactive created-player-pool slots). A roster status would never do that.

The **real roster status** (dressed vs. scratched) is the separate **1-bit boolean `eGlu.hSZj`**
(meta name "rosterstatus"; anchor split `{0:1092, 1:4440}`). "In Minors" (the 3rd
`TNHLRosterPlayerStatus` value) is **not** stored on that 1-bit field — it is a structural/editor
concept. `TNHLPlayerContractStatus {Unrestricted, Signed, Unsigned}` was also recovered from the EXE
but has no confirmed field home here.

> ✅ **Cross-stack mislabeling bug this uncovered — FIXED.** The `set_roster_status` op (which wrote
> `ZBac.RREB`) and the `LinesTab`/`JerseyCaptaincyTab` "dressed/scratched/in-minors" UI were rebuilt:
> lineup status is now the **`set_dressed`** op on the real per-slot boolean **`eGlu.hSZj`**
> (`1`=dressed, `0`=scratched), the entity/DTO field is `dressed: Option<bool>`, and the "in the
> minors" concept is now a **Depth-Chart-only front-end identifier** — the "In the system" section
> (players under this team's contract/rights whose main team `dXSB` is elsewhere). `ZBac.RREB` is
> exposed only as the Player Type enum. See the Phase 4 plan's Spike S4 section.

## Implications for Phase 4.2 (`free_agent`) and 4.1 (`hide`/`unhide`)

- **`free_agent` is fundamentally a slot occupancy change, not a field flag** (consistent with the
  [`toWX`-block finding](towx-blocks.md)):
  - *release to pool* = clear the player's slot `XSWT` (the slot becomes the empty kind we observe).
    (Do **not** touch `RREB` — it is player type, unrelated to pool membership.)
  - *sign from pool* = place the player's `ZBac.vfEq` into a free slot in the target team's block —
    blocks are full, so this generally needs block growth (capacity normalization, Spike S3 / 4.4).
- **`hide`/`unhide`** is the unplace/place mechanism (slot presence, per S1) — **not** an `RREB`
  sub-status. The earlier "fold into RREB" idea is dropped now that RREB is player type.

## Reproduction

`RosterModel::build` over the anchor for slots; a direct `ZBac` pass reading `vfEq`/`ykFq`/`RREB`;
partition rows by whether `vfEq` ∈ {slot `index_key`} and histogram `RREB` on each side; join
`ykFq` → `ubPc.wBIz` and read the bio `active` flag per `RREB` code (the 1:1 correlation that
identifies `RREB` as player type). (Spike probes removed after capture.)
