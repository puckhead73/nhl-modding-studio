# Bulk-import history cost — the batching decision (Phase 5.0 Spike S3)

> **Status:** ✅ measured 2026-07-26. **Gates** the Phase 5.2 pipeline design (`lower()` / commit seam).
> **Provenance:** `crates/studio-domain/tests/import_history_cost.rs` (`#[ignore]` benchmark
> `import_history_cost`), measured on the nhl14/ps3 (2.7 MB) and nhl21/ps4 (10.7 MB) anchors.
> **Confidence:** high on the memory result (order-of-magnitude); time is indicative.

## Question (from the plan)

`EditSession::run` is the unit of undo history: it parses the working buffer
(`EditTransaction::begin`), serializes a new one (`commit`), and **retains the pre-op full-DB
snapshot** on the undo stack (`std::mem::replace(&mut self.working, new_bytes)` pushed to `undo`). So
**N per-op writes retain N full-DB copies**, and a mirrored `Workspace` runs two sessions (nhlng +
roster), doubling it. A cart of N players × (create + ~15 field writes) ≈ 16N ops. Does this blow up,
and what is the batching seam?

## Measurement

Identical work — write every skater rating cell on one companion record — done two ways:

- **Path A (per-op):** one `set_field` per cell → one `run()` each → N undo entries.
- **Path B (batched):** all cells in one `set_cells_int` → a single `run()` → one undo entry.

| anchor | db MB | cells | per-op ms | undo depth | **per-op retained MB** | batch ms | depth | **batch retained MB** |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| nhl14/ps3 | 2.7 | 25 | 4320 | 25 | **67.2** | 3807 | 1 | **2.7** |
| nhl21/ps4 | 10.7 | 26 | 11794 | 26 | **277.9** | 5918 | 1 | **10.7** |

Extrapolated to a **25-player team-season on a mirrored workspace** (≈16 ops/player × 2 sessions):

| path | nhl14/ps3 retained | nhl21/ps4 retained |
|---|---:|---:|
| per-op | **≈ 2.10 GB** | **≈ 8.35 GB** |
| batched (1 commit) | ≈ 5.4 MB | ≈ 21.4 MB |

## Findings

1. **Retained memory is the killer, and it is order-of-magnitude.** The undo stack keeps one full-DB
   snapshot **per op**. A single-player, 25-cell edit already retains 67–278 MB; a 25-player mirrored
   import on the per-op path retains **2–8 GB** — OOM territory on the 11 MB PS4 db. Batched into one
   commit it is a single snapshot (single-digit MB). This alone forces batching.

2. **Time also scales with op count.** Each `run()` re-parses the whole db, rebuilds the entity model
   for the audit (`with_repairing_transaction`), and re-serializes — ~170 ms/op on ps3, ~450 ms/op on
   ps4. 25 cells for **one** player already costs 4–12 s per-op; a full team-season × 2 sessions on the
   per-op path is **minutes**. Batched pays the parse/audit/serialize **once**.

3. **The batch primitive already exists.** `set_cells_int` runs many cell writes inside one `run()` →
   one undo entry → one retained snapshot. It proves the pattern but is scoped to a single
   `(table, record)`. (Its `prepare` loop also parses the db per cell — a purpose-built import batch
   should parse once, which would cut Path B's time further; the memory result already stands.)

## Decision (output)

- **One import commit = one `run()` / one `EditTransaction` / one undo entry / one retained snapshot.**
  The 5.2 `lower()` must stage the **whole cart** — every `create_player` and every field write across
  bio + companion + slot records — inside a **single** `with_repairing_transaction`, then commit once.
  Never one `run()` per field or per player.
- **New seam for 5.2:** an import-scoped batch on `EditSession` / `Workspace` (e.g.
  `apply_import(lowered) -> OpOutcome`) that executes the lowered edit list against one
  `EditTransaction` and commits once. Building blocks (`EditTransaction::create_player`,
  `set_cell_int`) already exist; this is composition + a single commit, not new write logic.
- **Preview uses the same single-transaction `lower()`** against a throwaway snapshot (no commit), so
  preview is as cheap as one op and stays byte-identical to commit (the 5.2 golden-test invariant).
- **Undo granularity:** a committed import is **one** history entry (undo reverts the whole cart), not
  N. This matches the plan's "undo across a commit is out of scope."

## Residual notes

- The audit runs **once** for the whole batched cart (`with_repairing_transaction` audits after the
  op). That is the intended semantics for preview/commit — the four predicates evaluate the final
  state, not each intermediate edit.
- Unbounded undo growth during *normal* interactive editing (each edit retains a full snapshot) is a
  pre-existing, separate concern — not triggered by import once batched. Out of S3 scope; flag if
  memory pressure shows up elsewhere.
