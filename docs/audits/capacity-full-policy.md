# Capacity-full policy — boundary-insert safety + the preview blocker (Phase 5.0 Spike S4)

> **Status:** ✅ resolved 2026-07-26. **Gates** the Phase 5.2 preview blockers. **Provenance:**
> `crates/studio-domain/tests/capacity_boundary_insert.rs` + code read of
> `tdb_core::TdbFile::insert_record` and `studio_domain::capacity`. **Confidence:** high.

## Question (from the plan)

When a destination roster is full, is the boundary-insert safe, and what is the exact preview blocker
when the cart can't fit? Default proposal: boundary-insert to grow a full block while under the vanilla
ceiling; otherwise a storage blocker — never a silent drop.

## Two distinct limits

1. **Mechanical (table structure).** `create_player` places the slot via `eGlu` boundary-insert →
   `insert_record`. When the block is full (`current == max`) the insert **grows the table by one**:
   bumps `max_records`, shifts every downstream table's directory offset, regrows the file, re-parses.
   It only hard-errors (`CapacityExhausted`) at `u16::MAX`.
2. **Semantic (the vanilla ceiling).** The game runtime only accepts a roster up to the audited
   **ceiling** — GEN7 from `capacity::gen7_ceilings(game_id)` (the vanilla PS3 ROSTER-save
   `max_records`), PS4 from the attached main db's `max_records`. This is the same source Phase 4.4's
   `normalize_roster_capacity` grows each table up to at open, so after normalization
   `max_records == ceiling` and **headroom = ceiling − current_records**.

## Findings

1. **Boundary-insert is mechanically safe, including into a full block.** Proven on the GEN7 nhl14
   anchor, which ships **packed** (`ubPc.max == current`): the first `create_player` hits the
   grow-on-full branch, grows `max_records` by exactly one, adds exactly one bio, and yields a db that
   re-parses into a coherent roster model and passes `validate()`. No corruption.

2. **The write path has no ceiling awareness — it grows *past* the ceiling freely.** `insert_record`
   grows to `u16::MAX`; `txn.rs` itself notes "capacity growth past the ceiling" is still-to-come.
   So a large import would silently produce an **over-capacity** save the game rejects. **The ceiling
   gate must live in the Phase 5.2 preview**, not the write path.

3. **Headroom almost always exists.** GEN7 ceilings (e.g. nhl14: ubPc 6873, dSvy 6350, eGlu 10353)
   sit well above vanilla population (~5374 players), and Phase 4.4 pre-grows the mirror to the
   ceiling. A 25-player team-season is comfortably within headroom; the blocker is a whole-league /
   near-full edge, not the common path. `eGlu` (slots) is the roomiest; `ubPc` + companions bind first.

## Pinned policy (output)

- **Capacity projection in preview.** For each of the seven `capacity::ROSTER_TABLES`, project the
  cart's additions and require `current_records + additions ≤ ceiling`. Per new (Create) player:
  `+1` index (`ZBac`), `+1` bio (`ubPc`), `+1` AI companion (`dSvy` skater / `mHuy` goalie by kind),
  `+1` equip companion (`xmja`/`fMVl`; **skip on gen6** — no equip table), `+1` slot (`eGlu`). A
  **Match** (update existing) adds **zero**. The ceiling is table-wide, not per-team — a single team's
  block may grow freely as long as the table total stays under the ceiling.
- **Blocker, never silent drop or over-grow.** If any table would exceed its ceiling, emit a
  **storage blocker** and disable Commit. The import must never let `insert_record` grow a table past
  the ceiling.
- **Boundary-insert within headroom only.** Inside headroom, reuse the existing
  `create_player`/`eglu_placement` path unchanged (proven safe here).

## Pinned blocker wording (human-readable-surfaces rule)

Resolve the table to a friendly noun and the title to its real name; keep raw ids behind details:

> **Not enough roster space.** Importing these **{added}** players needs **{needed}** more
> {friendly_table} than **{title_name}** can hold — **{remaining}** of **{ceiling}** remain. Remove
> **{overflow}** player(s) from the cart, or free existing slots first (release or trade).

- `{friendly_table}`: `ubPc`→"player bios", `eGlu`→"roster slots", `dSvy`/`mHuy`→"rating records",
  `ZBac`→"roster index entries", `xmja`/`fMVl`→"equipment records". Report the **tightest** table
  (smallest remaining headroom) as the headline; list the others under details.
- Never surface the raw table id or `max_records` as the primary text.

## Residual notes

- A future "grow past ceiling on purpose" feature (raising the runtime cap) is **out of scope** — the
  RC treats the audited ceiling as hard. If pursued, it is its own verified change (the game must
  actually load the larger save), not an import detail.
- The projection must run on the **same lowered plan** the commit uses (Spike S3), so preview capacity
  numbers match what commit would write byte-for-byte.
