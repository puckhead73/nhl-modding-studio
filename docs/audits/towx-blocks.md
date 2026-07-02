# Audit — `toWX` roster-block layout (Spike S2, VERIFIED)

> **Status: VERIFIED** by decode of the committed anchor `anchors/ps3/nhllegacy/databases/nhlng.db`
> (PS3 NHL Legacy), 2026-06-30, via `RosterModel::build` (the verified entity graph). Resolves the
> open `toWX` contiguous-block question carried in [EDIT-MODEL.md](../EDIT-MODEL.md) (lines 96-105).

## Question

Are `eGlu` roster slots laid out as **per-team contiguous `toWX` runs** (each team owns a contiguous
block of slot rows), or are they keyed rows in no particular order? The prior app asserted a fixed
**"40-slot block per team"**; our repo had not re-verified it.

## Finding — blocks are REAL, contiguous, and tightly tiled; size is VARIABLE (not 40)

Decoded totals: **252** `kOtt` teams, **5632** `ubPc` bios, **5532** `eGlu` slots (5531 populated, **1
empty in the entire table**). 222 distinct teams appear on slots (30 `kOtt` teams have zero slots —
all-star/classic/placeholder entries).

Grouping every slot by its resolved team (`dXSB`) and sorting:

- **222 / 222 teams have a contiguous *record* range.**
- **222 / 222 teams have a contiguous *`toWX` key* range.**
- **0 record-range overlaps and 0 gaps between teams** → the `eGlu` table is partitioned into
  disjoint, record-ordered, per-team blocks that **tile the whole table with no gaps**. Example:
  team 0 = records 0–23, team 1 = 24–48, team 2 = 49–69, team 3 = 70–93, … each team's block begins
  exactly where the previous ended.
- **Block size is variable: 20–35 slots/team**, clustered 23–26 (distribution: 20→14 teams, 21→14,
  22→10, 23→31, 24→34, 25→43, 26→25, 27→10, 28→17, 29→9, 30→4, 31→3, 32→4, 33→1, 34→2, 35→1).
  The prior app's fixed **40** is **wrong in magnitude**; the *contiguous-block structure* it implied
  is correct.
- `toWX` key order tracks record order within each block (both contiguous and aligned).

## Implications for Phase 4 ops (important — reframes move/trade and flags a likely bug)

1. **A roster slot is a fixed, per-team container.** Because blocks are contiguous, tile without
   gaps, and are essentially full (1 empty slot DB-wide), the stable per-slot identity is
   **(`toWX`, `dXSB`) = "team T's slot #k"**. The variable content is **`XSWT`** (which player's
   `ZBac` index occupies the slot) plus jersey/captaincy/line flags.
2. **The earlier `move_player` (retag `dXSB`) was buggy on this layout — now FIXED.** Writing team B's
   id onto a slot that sits *physically inside team A's contiguous block* breaks the block invariant.
   The save-time audit missed it because the `toWX`-coherence predicate does not exist yet — exactly
   the gap Spike S2 surfaced. `move_player` has been re-modeled as an **occupant relocation** (keyed by
   `from_team`; transplants `XSWT` + per-slot fields into a free destination-block slot, vacates the
   source by zeroing `XSWT`, never touches `dXSB`) with a regression test asserting block coherence.
   The coherence *predicate* (Phase 4.3) is still to be added to the shared audit.
3. **Correct op model:**
   - *trade* (B is full) = **swap the `XSWT` occupants** (and jersey/captaincy/flags) between a slot
     in A's block and a slot in B's block — no `dXSB` change.
   - *sign from pool* into team T = place the player's `ZBac.vfEq` into a **free slot in T's block**;
     since blocks are full this generally requires **growing T's block** (record insert → shifts all
     later blocks) — i.e. it depends on capacity normalization (Spike S3 / Phase 4.4).
   - *release to pool* = clear a slot's `XSWT` (unplace), leaving an empty slot in the block (the one
     empty slot observed is exactly this shape).
4. **`towx_coherence` predicate (Phase 4.3) — IMPLEMENTED** (`tdb-domain-validation`, composed into
   both the txn audit and save-time `validate_invariants`). It enforces the **record-block** structure
   only: each team's slots form a contiguous *record* run and the blocks tile without overlap/gap
   (`Severity::Error`). **`toWX`-key contiguity is deliberately *not* enforced** — a follow-up
   measurement showed `toWX` is a globally monotonic *unique key with inter-block gaps* (spans
   `0..=8861` over 5532 records), so its per-team contiguity is a creation-order artifact: an ordinary
   mid-block delete leaves a key hole and a freshly-minted slot key is not block-adjacent. Enforcing it
   would cry wolf on every edit and force a risky global renumber. The record-block structure is the
   invariant the game actually reads a roster by, and the one the old `dXSB`-retag `move_player`
   (and swap-pop delete / append-at-end create) violated.
5. **Structural ops are now block-aware** (see [nhlviewng-record-model.md](nhlviewng-record-model.md)):
   `delete_player` ordered-removes the `eGlu` slot (slide up, no swap-pop), `create_player` places the
   new slot *inside* the team's block (reuse a vacant in-block slot, else `insert_record` at the block
   boundary), and `move_player` relocates the occupant — all preserving the tiling by construction.

## Reproduction

`RosterModel::build` over the anchor; group `slots` by `team_id`; per team check
`(max_record − min_record + 1) == count` and `(max_toWX − min_toWX + 1) == count`; sort teams by
`min_record` and check adjacent ranges for overlap/gap. (Spike example removed after capture; logic is
trivial to re-add under `crates/tdb-relations/examples/` if needed.)
