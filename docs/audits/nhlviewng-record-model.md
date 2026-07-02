# Audit — NHLViewNG's TDB record add/delete model (reverse-engineered)

> **Status: reverse-engineered** from the prior app binary at the user's explicit direction
> (clean-room rule 1 per-instance exception, 2026-06-30). Source:
> `…/Reference Data/Database Definitions/nhlviewng_unpacked.exe` — a native i386 **Delphi** PE (not
> .NET; the `.exe` is the packer-stripped "unpacked" build). Findings come from RTTI method-name →
> code-pointer resolution + capstone disassembly (no symbols, no source). Addresses are `ImageBase
> 0x400000`. **This is analysis, not copied code** — it records *what the proven app does* so we can
> decide our own implementation.
>
> **Why this exists:** composing the new `towx_coherence` predicate (Phase 4.3) exposed that our
> `create_player` (append-at-end) and `delete_player` (swap-pop) produce block-incoherent `eGlu` —
> the same corruption class as the old `move_player` `dXSB`-retag bug. Before rebuilding those ops we
> looked at how NHLViewNG — which shipped working Legacy/14/15 roster edits — actually manages records.

## What NHLViewNG does

The TDB layer is a set of Delphi classes in units `uAbstractTDB` / `uAbstractTDBMetadata` /
`uAbstractTDBHuffman`: `TtdbDatabase`, `TtdbTable`, `TtdbTableAccessor`, `TtdbField`,
`TtdbFieldIndex`, plus `TtdbMeta*`. Row management lives on the table accessor. Its in-memory table
header (`[self+0x0c]`) carries four 16-bit counters (so **record numbers are `u16`, max `0xFFFF`**):

| header offset | meaning | accessor |
|---|---|---|
| `+0x14` | **record count** (live length) | `0x6873cc` |
| `+0x28` | **capacity / max** | `0x68852c` |
| `+0x3c` | **free (deleted) count** | `0x686ebc` |
| `+0x50` | **free-list head** (first reusable slot) | `0x6875a4` |

This is a **free-list allocator with tombstones** — not the append/swap-pop pair we built.

### `AddRecord` (`0x685ea4`)
1. Read `freeCount` (`+0x3c`). **If a freed slot exists → reuse it** (`0x6878f0` re-inits the slot in
   place) instead of growing.
2. Else **append**: if `count == capacity` and `capacity == 0xFFFF` → raise (range error `0x40a008`);
   otherwise `count = count + 1` (virtual setter `[vmt+8]`).
3. Common tail initialises the (reused or new) slot's fields.

### `DeleteRecord(recno, track: bool)` (`0x6868b4`)
Walks the record's fields and **captures the removed field values** into a map/list
(`0x4b5730/0x4b5918/0x4b5548`, backed by the `TRemovedFieldNameTDB` structure) — an undo/journal — then
frees the slot. **No `memmove`, no shift**: the record stays in place, logically tombstoned. The
`track` bool gates whether removed values are recorded.

### `RemoveRecord(recno)` (`0x687788`)
The physical variant: computes `dst = recSize*recno + dataBase`, then **`Move`s the `count-recno-1`
trailing records up by one** (`0x684778`) to close the gap, `count = count - 1`, and fixes the free
list. This is an **ordered left-shift compaction** — it preserves record *order* and the contiguity of
everything after the hole. It is emphatically **not swap-pop**.

### `Compact` (`0x686378`)
A reclaim pass that collapses tombstoned/free slots (the physical counterpart to `DeleteRecord`).

## Why this matters for block coherence

`eGlu` is partitioned into contiguous per-team blocks that tile the table
([towx-blocks.md](towx-blocks.md)). Against that layout:

- **Swap-pop (our `remove_record`) is the *only* one of these strategies that breaks blocks** — it
  drops the table's tail record into the freed mid-block slot, tagging a foreign team inside a block.
  NHLViewNG never does this.
- **Tombstone-in-place (`DeleteRecord`) is block-safe**: the slot keeps its `dXSB`/`toWX`, just emptied
  (`XSWT=0`). The block stays contiguous with a hole. This is *exactly* the **1 empty slot** observed
  DB-wide in the anchor, and exactly the "release-to-pool = clear `XSWT`" model from S2.
- **Ordered-shift compaction (`RemoveRecord`) is block-safe**: closing the hole shifts later blocks
  down by one index but each stays contiguous and the table still tiles.
- **Add** stays block-safe *iff* the reused free slot (or the insert point) is **inside the target
  team's block**. NHLViewNG's generic `AddRecord` reuses any free slot / appends; the block-correct
  slot choice is a roster-level policy on top of it (grow the block = ordered *insert* at the block
  boundary, the inverse of `RemoveRecord`'s shift).

## Scope — what it takes to mimic this directly

1. **`tdb-core`: add ordered structural primitives** alongside the existing swap-pop `remove_record`:
   - `remove_record_ordered(table, index)` — `memmove` the trailing records up one, `count-1`, CRC
     rescope. (Mirror of the growth path already proven in `append_record`.)
   - `insert_record(table, index)` — `memmove` the tail down one to open a hole at `index`, `count+1`.
     This is the primitive we lack today and the one block growth needs.
   - Keep swap-pop for callers that genuinely don't care about order (non-block tables), but **make
     block-bearing tables use the ordered variants.**
2. **`tdb-edit`: re-model the two structural ops the NHLViewNG way.**
   - `delete_player`: for the `eGlu` slot, **tombstone-in-place** (zero `XSWT` + per-slot fields,
     leave the record) so the block stays whole — this is the same primitive `release_to_fa` (4.2)
     needs. The bio/index/companion rows in the *non-block* tables can still be removed (order there is
     irrelevant; keep swap-pop or switch to ordered — decide once, consistently, re: the roster-save
     mirror alignment).
   - `create_player` / roster-add: **reuse a free (empty) slot inside the target team's block** if one
     exists; else **`insert_record` at the block's end boundary** to grow the block (shifting later
     blocks), then wire the occupant. This is the Phase 4.4 capacity work made concrete.
3. **Compose `towx_coherence`** into `validate_invariants` *and* the txn audit once 1–2 land — at which
   point create/delete are coherent by construction and the predicate guards every op.
4. **(Optional) tombstone bookkeeping.** NHLViewNG tracks a free-list head + free count in the table
   header and a removed-values journal. We don't need the journal (we snapshot for undo already), and
   we don't persist a free list — but a *save-time* pass can treat "empty `eGlu` slot" (`XSWT==0`) as
   the reuse pool, matching the on-disk shape EA ships.

## Issues / risks

- **Record-index churn.** `insert_record` / `remove_record_ordered` shift indices for every later
  record. Anything holding raw record indices across a structural op must re-resolve. Our ops key by
  value (`wBIz`/`vfEq`/team id), so this is safe *within* an op, but the **roster-save mirror**
  (record-index-aligned subset of `nhlng.db`) must apply the *same* ordered shift to stay aligned —
  verify `fully_aligned()` after ordered inserts/removes, same guard as today.
- **`u16` ceiling.** NHLViewNG caps record numbers at `0xFFFF`. Our tables are far below that, but the
  capacity-normalization pass (4.4) should assert it, not silently wrap.
- **Tombstone vs. compaction choice must be consistent per table.** If `delete_player` tombstones
  `eGlu` but a later `Compact`-style pass reclaims it, the mirror and any cached model must be rebuilt.
  Pick one policy (recommend: tombstone-in-place for `eGlu`, matching the vanilla "1 empty slot"), and
  only compact deliberately.
- **Not a licence to copy.** These are behavioural facts about record management, independently
  re-derivable from our own format work; no prior-app code or data is imported. Any specifics beyond
  what's stated here (e.g. the exact Huffman/serialisation path) remain unverified and must not be
  asserted.

## Recommendation

The RE **validated the strict reading of `towx_coherence`**: the fix was not to weaken the predicate
but to make the structural ops block-preserving, as NHLViewNG does.

## What was implemented (2026-06-30)

- **`tdb-core`:** `insert_record(table, index)` (open a hole, shift the tail down, `count+1`;
  slack-reuse or table-growth like `append_record`) and `remove_record_ordered(table, index)` (slide
  the tail up, zero the freed slot, `count-1`) — the order-preserving counterparts to
  `append_record` / swap-pop `remove_record`. Unit-tested against the anchor incl. an
  insert↔ordered-remove round-trip.
- **`tdb-edit`:** `delete_player` now **ordered-removes** the `eGlu` slot (`remove_matching_ordered`)
  while the non-block cluster tables keep swap-pop; `create_player` places the new slot **inside the
  team's block** — reuse a vacant in-block slot, else `insert_record` at the block-end boundary
  (`eglu_placement`). Both keep every block contiguous and tiling.
- **Predicate:** `towx_coherence` is **composed into both** the txn audit and save-time
  `validate_invariants`, scoped to the **record-block** structure (contiguity + tiling by `dXSB`).
  Per-team `toWX`-key contiguity was found to be a vanilla artifact (globally monotonic unique key with
  gaps) and is intentionally not enforced — see [towx-blocks.md](towx-blocks.md).

**Deviations from the NHLViewNG model (deliberate):** we use *ordered-shift* (not tombstone-in-place)
for `delete_player`, so a full cluster delete returns record counts to baseline; tombstone-in-place is
reserved for the future `release_to_fa` (4.2), where the player stays in the DB. We do not persist a
free-list/free-count header or a removed-values journal — undo is handled by the transaction snapshot,
and the single vanilla vacant slot is discovered from the model (`XSWT==0`) rather than tracked.

**Still open (out of this change):** block growth at the `0xFFFF` record ceiling and capacity
normalization (4.4); `trade_slots` / `free_agent` (4.1–4.2); the `dup_on_team` predicate (4.3).
