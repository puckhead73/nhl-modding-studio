# Studio vs. NHLViewNG — Comparison & Redesign Direction

Function-by-function comparison of how **NHLViewNG** manages database/roster editing versus how the **NHL
Database Studio** does today, the root-cause diagnosis of the recurring edit/save errors, and the redesign
direction (metadata-driven invariant model) adopted for the parity branch.

> Companions: [`nhlviewng-functional-report.md`](nhlviewng-functional-report.md) (what NHLViewNG does),
> [`nhlviewng-validation-audit.md`](nhlviewng-validation-audit.md) (its validation contract),
> [`nhlviewng-line-visibility-audit.md`](nhlviewng-line-visibility-audit.md) (line storage),
> [`edit-session-architecture.md`](edit-session-architecture.md),
> [`roster-view-model.md`](roster-view-model.md), [`db-capacity-limits.md`](db-capacity-limits.md).

## Studio architecture today (summary)

Edits flow through a **read-only `EditSession` overlay** (`crates/tdb-edit/src/{model,session}.rs`) of raw
`FieldRef = (table, record, field)` edits plus sequenced `RowOp`s. Edits are **staged blindly** (never
rejected at entry) and validated **post-hoc** by a mix of a narrow staging guard
(`crates/tdb-gui-api/src/integrity_guard.rs`) and a whole-DB domain audit at `save_project()`
(`tdb-domain-validation`). Containers are read/written by `crates/tdb-savedata` (`extract.rs` / `pack.rs`).
The low-level byte-writer (`apply_to_file_bytes()`, 4-pass: header → row-ops → per-cell → varchar pool) is
solid and stays.

## Function-by-function

| Area | NHLViewNG | Studio today | Fundamental gap |
|---|---|---|---|
| **Edit unit** | Logical entity field, validated by per-field metadata (width/range/`update`/FK). | Raw `FieldRef`; **always staged, never rejected**; range checked only against bit width, not EA `rangelow/high`; no `update`-flag concept. | **No metadata layer** — illegal-but-in-width values and edits to non-updatable fields are accepted, surfacing as in-game corruption. |
| **Referential integrity** | Maintained *by construction* (create wires instance; trade moves it). | Maintained *reactively*: guards in `integrity_guard.rs` (position-flip companions, ZBac/eGlu/VoEQ pointers, dup-on-team) **block** but don't repair; full check only at save. | Studio **refuses** where NHLViewNG **repairs**; many guards read **on-disk** state, missing pending-edit composition (e.g. dup-on-team across two commands). |
| **Create player** | One action: new instance + companions + team wiring. | Roster-Builder "Duplicate": `find_free_player_slot_for_kind()` + `assign_player_to_team()`. **No auto-create** of companion rows (`dSvy/xmja` or `mHuy/fMVl`) or **ZBac bridge**; relies on a pre-existing complete free slot. | No true "new player" — only *reuse of a conveniently-complete empty slot*. Bridge/companion **lifecycle unmanaged**. |
| **Move / trade** | `btnTradeMove/Copy` moves the instance atomically between two teams. | `set_main_team()` rewrites `ubPc.dXSB/dbBW` + eGlu wiring; **no two-team trade UI**; no copy-with-appearance. | Single-direction move exists; **no trade workspace**, no appearance-follows-trade. |
| **Delete player** | `DatabaseDeletePlayer`. | `hide_player()` (move to hidden pool) or `apply_row_delete()` guarded by `SwapPopWouldOrphan`. | Real row-delete **orphans ZBac/companions** (swap-pop moves a different tail row in); studio mostly avoids true delete. |
| **Roster / lines** | Full 19-group line editor with captaincy/archetype/jersey + roster status (Dressed/Scratched/In-Minors). | **Lines tab now authors lines** (`TeamDetailPanel.tsx`): situation-bucketed 19-group layout with drag-drop place/swap (1-bit eGlu slot flags), "Best lines" auto-fill, **3-state roster status** (`ZBac.RREB` 0/1/3), **inline captaincy** (`eGlu.mCcl`, single-captain rule + C/A card badge), **per-slot archetype** (`eGlu.QgFs`, provisional labels — binding unconfirmed). **Jersey-number edit still missing.** `eGlu.toWX` block alignment now realigned by `entity::repair` (Phase 2c) but slot-bit line edits go through the raw edit session, not the repairing transaction. | Studio **can now author lines** (status/captaincy/archetype shipped 2026-06-25). Remaining: jersey-number edit; `eGlu.QgFs` label mapping unconfirmed; line-slot bit edits not yet wrapped in `with_repairing_transaction`. |
| **Save / containers** | PS3 (de/encrypt) + PS4 + Xbox360 + plain TDB; v1..v4; `.bak`; integrity tags. | `extract.rs`/`pack.rs`: v4 read+write, v2 read-only-writeable, **v1 (NHL10/11) read-only**, **no PS4**; integrity tags recomputed; **no automatic `.bak`**. | Studio **cannot write** the oldest Xbox saves or any PS4; no pre-overwrite backup. |
| **Schema source** | Per-game `db/nhlng-meta.xml` + RCDATA → 09–16 from one model. | `app/src-tauri/resources/games/*.toml` (`schema_id` selectors) + `defs/` TOML overlays; **NHL10 has no meta** → rating edits impossible. | Schema knowledge is **partial and hand-curated**; missing the per-field width/range/`update`/FK that NHLViewNG gets free from `nhlng-meta.xml`. |
| **Edit→save** | In-memory relational edits → one re-serialize. | `EditSession` → `apply_to_file_bytes()` (solid byte-writer). | **Byte-writer is good and stays**; the problem is everything *above* it (validation/lifecycle), not the write path. |

## Root-cause diagnosis

The recurring errors — out-of-block `toWX`, swallowed integrity errors, "N integrity error(s)" save walls,
dangling FKs, capacity overflow, position-flip orphans — share **one** root cause:

> **Edits are accepted against raw fields with no metadata and no entity lifecycle, then judged after the
> fact.** The system can represent illegal states, so it spends its effort *detecting* them late (and often
> *refusing* rather than fixing). NHLViewNG never reaches these states because its **metadata + entity model
> makes illegal states unrepresentable**, and its structural operations maintain consistency by construction.

The fix is **not** more post-hoc guards. It is to move validation and structural maintenance to **edit-entry
time, driven by ingested per-field metadata, operating on entities** rather than raw cells.

## Redesign direction — metadata-driven invariant model

Keep the proven byte-writer (`tdb-edit::apply_to_file_bytes`) and container codec (`tdb-savedata`). Replace the
reactive validation layer with three new layers on top:

1. **Field-metadata backbone.** Ingest each game's `nhlng-meta.xml` into a typed `FieldMeta`
   `{ table, field, name, type, bit_depth, range_low, range_high, updatable, fk_parent, save_groups }` (reuse
   the existing `definition_flavor = "nhlng-meta"` hook in `games/*.toml`). `apply_field_edit()` consults it:
   reject non-`updatable` fields, enforce `[range_low, range_high]` (not just bit width), resolve FK targets.
   Removes the whole "valid-width but illegal-value" corruption class.

2. **Entity layer.** Typed entities backed by the documented join model
   ([`roster-view-model.md`](roster-view-model.md), [`player-view-model.md`](player-view-model.md)):
   **Player** (ubPc + ZBac bridge + companions), **RosterSlot** (eGlu row in a team's 40-slot `toWX` block +
   line-group flags), **Team** (kOtt). Edits become **transactional operations** —
   `create_player`, `move_player` / `trade`, `delete_player`, `assign_line_slot` — each maintaining its
   invariants and composing against **pending session state**, not on-disk (closing the cross-command holes).

3. **Invariant engine.** A single `validate_invariants(session)` sharing predicates with
   `tdb-domain-validation` (`slot_supports_kind`, `instance_to_player_map`, companion maps,
   `PLAYER_NULL_SENTINELS`) so staging-time and save-time never drift — but invoked **at operation time** and
   able to **auto-repair** (create the missing bridge/companion, realign the `toWX` block) instead of blocking.
   Where repair is ambiguous, surface a structured, *actionable* error (no more swallowed `error_count:1`).

Container parity (`tdb-savedata`): add **v1 (NHL10/11) write**, **PS4 read/write**, and **automatic `.bak`**
before overwrite in `save_project()`.

See the parity roadmap (Phases 0–4) and the per-file change list in the plan
`C:\Users\puckh\.claude\plans\i-want-to-do-functional-biscuit.md`.
