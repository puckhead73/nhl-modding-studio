# EDIT-MODEL — the database editing contract

> **Canonical, binding.** This document codifies the database-editing model the engine **must**
> implement. It is the spec ROADMAP Phase 1–4 work is judged against. It is derived from the
> in-repo reference reports — [nhlviewng-functional-report.md](nhlviewng-functional-report.md) and
> [studio-vs-nhlviewng-comparison.md](studio-vs-nhlviewng-comparison.md) — and the architecture in
> [DESIGN.md](DESIGN.md). No other repositories were consulted (clean-room; see CLAUDE.md Hard rule 1).

## North star

> **Make illegal states unrepresentable.** Edits target *entities*, are validated against *per-field
> metadata at the moment of entry*, and structural operations maintain referential integrity *by
> construction*. The engine never accepts an edit it would later have to detect and refuse.

This is the explicit inversion of the prior studio's failed model (raw `FieldRef` staged blindly,
validated post-hoc, refusing rather than repairing). We are greenfield, so we build the correct model
from day one and carry none of that debt. There is **no** "stage a raw cell and judge it later" path.

## Field-id basis (read this first)

The 4-char table/field ids used below (`ubPc`, `dSvy`, `eGlu`, `ZBac`, `kOtt`, `toWX`, `mCcl`,
`QgFs`, `RREB`, `dXSB`, `XSWT`, `sVRt`, `xmja`, `mHuy`, `fMVl`, …) come from the two reference reports,
which derive them from NHLViewNG's metadata XML and prior audits. Treat them as **strong signals to
confirm against the user-provided metadata**, not as parser truth. Anything the reports flag as
unconfirmed (e.g. the `eGlu.QgFs` archetype label binding) stays marked unconfirmed here until
verified. Per [DESIGN.md](DESIGN.md) and CLAUDE.md, do not assert a binary-layout specific as fact
until confirmed against material in this repo or real, locally-held data.

---

## Layer 1 — FieldMeta backbone (metadata-driven validation)

Every editable cell is governed by typed per-field metadata. This is the validation backbone; without
it the edit layer is blind.

```
FieldMeta {
  table: RawId,          // e.g. ubPc, dSvy, eGlu, kOtt
  field: RawId,
  name: String,          // human display name
  kind: FieldType,       // INTEGER, STRING/varchar, enum, fk, ...
  bit_depth: u32,        // storage width
  range_low: i64,        // EA legal minimum (NOT merely the bit-width min)
  range_high: i64,       // EA legal maximum
  updatable: bool,       // writable flag — non-updatable fields are read-only
  fk_parent: Option<FkRef>,   // referential target, if any
  save_groups: SaveGroupSet,  // which save-context(s) include this field
  control_type: ControlType,  // TextBox | Dropdown(enum) | ... (UI hint)
}
```

**Source & ingestion.** The metadata files are **user-provided** into this repo (per-game, in the
shape of NHLViewNG's `nhlng-meta.xml` / `NHL 14 (EA DB Editor).xml`, or a converted form). The engine
ingests them into a typed `FieldMetaIndex` keyed by `(title, platform, table, field)`. The ingestion
loader is designed now; it consumes whatever the user drops in. The engine **must not** fetch these
from any other repository.

**Edit-entry rules (enforced by `apply_field_edit`).** An edit is rejected at entry if any holds:
1. the field is **not `updatable`**;
2. the value is outside **`[range_low, range_high]`** (range is the EA legal range, *not* the bit
   width — this removes the entire "valid-width but illegal-value" corruption class);
3. the value violates the field `kind` (bad enum member, malformed varchar, etc.);
4. an `fk_parent` edit would point at a non-existent / illegal target.

Rejection is a **structured, actionable error**, never a silently-swallowed `error_count: 1`.

**Coverage note.** NHLViewNG metadata covers **NHL 09–16**; our target is **10–21**. Metadata for
**17–21** is net-new (ROADMAP Phase 10) and will not come from NHLViewNG's files. The `FieldMetaIndex`
is keyed per `(title, platform)` precisely so years plug in independently.

---

## Layer 2 — Entity model

Edits operate on typed entities backed by the documented join model, never on bare rows.

- **Player** — the `ubPc` bio row **plus** its `ZBac` bridge **plus** companion rows
  (`dSvy`/`xmja` skater-AI, `mHuy`/`fMVl` goalie companions). A Player is the whole cluster, not just
  the bio row; its lifecycle (create/delete) owns every member.
- **RosterSlot** — an `eGlu` row inside a team's 40-slot `toWX` block, carrying the per-slot
  attributes (`dXSB` team, `XSWT` player-instance, `sVRt` jersey, `mCcl` captaincy, `QgFs` archetype
  *[label binding unconfirmed]*) and the **19 line-group 1-bit position flags**
  (`C, LW, RW, LD, RD, G, PP5-1/2, PP4-1/2, PK4-1/2, PK3-1/2, SO, X, OT1/2/3`).
- **Team** — the `kOtt` row.
- **Roster status** — `ZBac.RREB` tri-state `0/1/3` = Dressed / Scratched / In-Minors.

The `toWX` block alignment (a player's slots must stay within its team's contiguous block) is an
entity-level invariant, not a free-field edit.

---

## Layer 3 — Transactional operations

All structural change goes through typed operations, each of which **maintains its invariants by
construction** and **composes against pending session state, not on-disk** (closing cross-command
holes such as dup-on-team spread across two commands).

| Operation | Invariant it guarantees |
|---|---|
| **create_player** | Allocates the instance **and** creates companion rows (`dSvy`/`xmja`, `mHuy`/`fMVl`) **and** the `ZBac` bridge **and** wires it onto the target team — atomically. Never a bare reuse of a conveniently-empty slot. |
| **move_player / trade** | Atomic instance move between two teams; appearance follows the trade (the `keepcustomstickiftraded` semantics). Two-team operation, not a one-directional field rewrite. |
| **delete_player** | Removes the full cluster without orphaning `ZBac`/companions; guards the swap-pop tail-row hazard (a deleted row must not silently relocate an unrelated tail row into a live reference). |
| **assign_line_slot** | Sets the 1-bit `eGlu` position flag while keeping `toWX` block alignment and line-group coherence. |
| **set_roster_status / set_captaincy / set_archetype** | `ZBac.RREB` tri-state; `eGlu.mCcl` with the **single-captain rule**; `eGlu.QgFs` *[binding unconfirmed]*. |
| **set_jersey_number** | Validated, unique-where-required jersey assignment. |
| **field edit (attributes, equipment, appearance, …)** | Goes through Layer 1 (`apply_field_edit`) — metadata-validated at entry. |

**Roster scopes.** Operations are aware of the three roster pools: **main**, **free agents**,
**hidden** (NHLViewNG's `LoadRoster` / `LoadFreeAgentsRoster` / `LoadHiddenRoster`).

---

## Layer 4 — Invariant engine (auto-repair, not refuse)

A single `validate_invariants(session)` shares its predicates (`slot_supports_kind`,
`instance_to_player_map`, companion maps, null/sentinel sets, dup-on-team, dangling-FK, `toWX`
coherence) between **operation time** and **save time** so the two never drift.

Policy, in order:
1. **Prevent at entry** — Layer 1/3 reject illegal edits before they are accepted.
2. **Auto-repair** — where a structural edit leaves a fixable gap (missing bridge/companion,
   `toWX` block misalignment), the operation **creates/realigns it** inside the transaction
   (`with_repairing_transaction`: snapshot → stage → auto-repair → audit → commit/rollback).
3. **Structured error only when repair is ambiguous** — surface an actionable error; never silently
   swallow, never a bare refusal where a repair was possible.

This is the decisive difference from the prior studio, which **blocked** where NHLViewNG **repairs**,
and which read **on-disk** state (missing pending composition).

---

## Validation tiers (how the layers map)

Three explicit tiers, all sharing predicates with the invariant engine:

1. **Stage / edit-entry** — per-edit: `kind` / `bit_depth` / `[range_low, range_high]` / enum / FK /
   `updatable`, via `FieldMeta` (Layer 1).
2. **Operation time** — entity invariants inside repairing transactions (Layers 3–4).
3. **Save time** — whole-DB integrity audit (companion model, roster-block coherence, dup-on-team,
   dangling FKs) before bytes are written.

## Save model

- **One re-serialize.** Edits live in memory against the relational/entity model; on save the whole
  container is re-serialized (header preserved, payload recompressed, integrity tags recomputed).
- **Backup + integrity.** Automatic `.bak` before overwrite; CRC/checksum/magic/endian framing owned
  by the container codec.
- **Container parity (target).** Read/write across plain TDB, PS3 (de/encrypt), Xbox360, and — beyond
  what the prior studio could do — **v1 (NHL 10/11)** and **PS4**. (PS4 is ROADMAP Phase 9; v1 write
  is Phase 8.) See [05-save-data.md](05-save-data.md).
- **External-edit guard.** SHA check before overwrite so a file changed underneath us is not clobbered.

---

## What binds the build

1. The engine's public edit API is **entity + metadata first**. No raw-`FieldRef`-then-validate path
   ships.
2. `FieldMeta` ingestion consumes **user-provided** metadata; the loader is built to a documented
   schema and tested with synthetic metadata until real files land.
3. Every structural operation in Layer 3 is implemented as a repairing transaction with its invariant,
   and shares predicates with the save-time audit.
4. Field ids and any unconfirmed bindings (e.g. `eGlu.QgFs`) are **verified against the provided
   metadata before being relied upon**, per the verify-don't-guess rule.

## Open items

- **Metadata files** — pending user delivery into the repo (Layer 1 source).
- **Referenced-but-absent docs** — the reports cite `nhlviewng-validation-audit.md`,
  `nhlviewng-line-visibility-audit.md`, `nhlviewng-semantic-audit.md`, `edit-session-architecture.md`,
  `roster-view-model.md`, `player-view-model.md`, `db-capacity-limits.md`. They hold load-bearing
  detail (validation contract, `eGlu`/`ZBac` line storage, capacity ceilings) and are **not in this
  repo**; not fetched (clean-room). Bring in if/when you choose.
- **`eGlu.QgFs` archetype label binding** — unconfirmed in the source reports.
- **17–21 metadata** — net-new; not covered by NHLViewNG (ROADMAP Phase 10).
