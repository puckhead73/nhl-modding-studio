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

**Coverage note.** NHLViewNG metadata covers **NHL 09–16**; our scope is **NHL 07–21 plus Legacy
and Legacy Recompiled**. Metadata outside that range (NHL 07–08 and 17–21) did not come from
NHLViewNG's files and was established here. The `FieldMetaIndex` is keyed per `(title, platform)`
precisely so years plug in independently.

---

## Layer 2 — Entity model

Edits operate on typed entities backed by the join model, never on bare rows. The model below is
**verified against the anchors** (all games) — see
[audits/roster-relationships.md](audits/roster-relationships.md).

- **Player** — the `ubPc` (bio, key `wBIz`) row **plus** its `ZBac` index row **plus** companion
  rows that **share `ubPc.wBIz` 1:1**: `dSvy` (skater AI) + `xmja` (skater equipment) for skaters,
  `mHuy` (goalie AI) + `fMVl` (goalie equipment) for goalies. A Player is the whole cluster; its
  lifecycle (create/delete) owns every member. *Verified: all four companions key on `wBIz`.*
- **RosterSlot** — an `eGlu` (roster+lines) row, key `toWX`. Bridges to the player via
  **`eGlu.XSWT → ZBac.vfEq` (a meta-declared FK), then `ZBac.ykFq → ubPc.wBIz`**. Per-slot fields:
  `dXSB` (team → `kOtt.vfEq`, 8-bit), `sVRt` (jersey #, 7-bit), `mCcl` (captaincy, 2-bit),
  `QgFs` (player style, 4-bit), `hSZj` (roster status flag, 1-bit), plus **71 one-bit
  line-assignment flags** (NHL13+; ~60 in NHL11/12 before the OT lines were added) — the per-player
  "occupies slot S" activation flags the UI groups into the 19 line buckets
  (`C, LW, RW, LD, RD, G, PP5-1/2, PP4-1/2, PK4-1/2, PK3-1/2, SO, X, OT1/2/3`).
- **Team** — the `kOtt` (key `vfEq`) row.
- **Roster status** — `ZBac.RREB` (2-bit) holds the roster-player status enum. *Label→value mapping
  (Scratched/Dressed/In-Minors + default) is **candidate**, pending operator/data confirmation.*

> **Carried-forward invariant to (re)confirm:** the prior app modeled a per-team contiguous `toWX`
> block ("40-slot block alignment"). Our anchors confirm `toWX` is `eGlu`'s primary key, but the
> contiguous-block invariant itself is a prior-app claim **not yet re-verified** against our data —
> confirm with the record decoder before relying on it.

> **Status (shipped).** `tdb-relations::entity` ships the typed model — `RosterModel::build(db)`
> assembles `Player` (bio + companion cluster), `RosterSlot`, and `Team`, resolving the roster→player
> two-hop bridge and the team join by construction. *Verified against the PS3 NHL Legacy anchor:*
> 252 teams / 5632 players / 5532 slots; the skater↔goalie split (5066 / 566) matches the
> `dSvy`+`xmja` / `mHuy`+`fMVl` companion populations exactly; **0 companion-invariant violations**;
> 5531/5531 populated slots resolve to both a player and a team (100%). The `toWX` contiguous-block
> invariant above is still **not** modelled (slots are keyed, not yet block-validated).

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

> **Status (first increment).** `tdb-edit::txn::EditTransaction` ships the repairing-transaction
> harness — `with_repairing_transaction` does *snapshot → stage (+auto-repair) → audit shared
> predicate → commit / rollback*, mutating working bytes so operations **compose against pending
> state**. The first operation, **`set_captaincy`**, enforces the single-captain rule **by repair,
> not refusal**: promoting a Captain auto-demotes the prior one, and the post-condition reuses the
> save-time `single_captain` predicate. *Verified against the NHL Legacy anchor:* promote → prior
> captain demoted → exactly one captain → commit yields CRC-valid, audit-clean bytes; failed ops roll
> back untouched.
>
> **Record-allocation primitives now exist** (`tdb-core::append_record` / `remove_record`): append
> reuses a slack slot or grows a full table (insert one record, shift later tables' directory offsets,
> bump `db_size`/`max`/`current`, recompute CRCs); remove is swap-pop (relocate the tail into the gap,
> leave reusable slack) and returns the moved tail index so the caller can repoint references — the
> documented hazard. *Verified on the anchor: full-table growth preserves every other record and keeps
> all CRC scopes self-consistent.*
>
> **`create_player` / `delete_player` now ship** on the harness. `create_player(team, position)`
> allocates the whole cluster atomically — `ubPc` bio (freshly **minted unique** `wBIz`, `vjla`,
> `dXSB`), the kind-correct companion pair (`dSvy`+`xmja` / `mHuy`+`fMVl`), the `ZBac` index bridge,
> and an `eGlu` slot wired to the team — then the audit confirms the companion invariant.
> `delete_player` swap-pops the full cluster (slots → index rows → companions → bio); since every
> intra-cluster reference is **by value** (`wBIz`/`vfEq`), the relocated tail rows need no repointing.
> *Verified on the anchor with the full save-time predicate set (relations + enums): create/delete are
> CRC-valid and audit-clean; delete returns counts to baseline.* Note: id minting must pick the
> **smallest free non-sentinel** value — the 14-bit `wBIz`/`vfEq` spaces are nearly full and their
> top value is the all-ones "none" sentinel, so `max + 1` would alias it.
>
> **`move_player`, `set_jersey_number`, `set_roster_status` also ship.** `move_player(player,
> from_team, to_team)` moves one **roster membership**: `eGlu` is partitioned into contiguous,
> gap-free per-team blocks (verified — [audits/towx-blocks.md](audits/towx-blocks.md)), so a slot is a
> fixed *(team, slot#)* container. The op therefore **relocates the occupant** into a free slot in the
> destination team's block (transplanting `XSWT` + the per-slot fields) and vacates the source slot by
> zeroing its `XSWT` (the game's own empty-slot convention) — it does **not** retag `dXSB`, which would
> break block coherence. It errors `TeamRosterFull` when the destination block is full (block growth is
> a later capacity phase). It is keyed by `from_team` because a player can hold several memberships at
> once (verified: ~6 % occupy >1 slot, across different teams). Captaincy clears on arrival
> (single-captain repair); the bio's `dXSB` follows only when it matched `from_team`.
> `set_jersey_number` (`eGlu.sVRt`) is width- and **uniqueness-within-team** validated (a collision is
> a structured error — no unambiguous repair); `set_roster_status` writes the bridged `ZBac.RREB`
> 2-bit code (labels candidate). All verified audit-clean on the anchor.
> **Still deferred:** `assign_line_slot` — the 71 one-bit line flags are not yet mapped to positions,
> so wiring it now would be guesswork (verify-don't-guess); and `trade_slots` (a two-membership swap,
> the natural op when destination blocks are full).
>
> **Note (corrected):** an earlier increment of `move_player` retagged the slot `dXSB` (and moved
> *every* one of the player's memberships), which silently broke the per-team block invariant. That was
> fixed once the block layout was verified; the coherence predicate that would have caught it is a
> pending Layer-4 addition ([ROADMAP Phase 4](ROADMAP.md)).

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

> **Status (first increment).** `tdb-domain-validation` ships the shared-predicate backbone:
> `validate_invariants(db, relations, enums)` composes a whole-DB `IntegrityReport` from two
> predicates — `referential_integrity` (every confirmed FK resolves; dangling values are **warnings**,
> since the defs carry no machine-readable FK *totality* and real data holds documented partial-subset
> FKs / null sentinels — flag-don't-refuse) and `enum_vocabulary` (unlabelled-but-reachable codes are
> `Info`). The remaining predicates (dup-on-team, companion presence, `toWX` coherence, single-captain)
> and the `with_repairing_transaction` harness land as further increments, each a shared predicate so
> operation-time and save-time never drift.

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
  what the prior studio could do — **v1 (NHL 10/11)** and **PS4**. (Remaining container work is
  milestone C2 in the [completion roadmap](plans/project-completion-roadmap.md).) See
  [05-save-data.md](05-save-data.md).
  **Shipped:** `tdb-savedata` reads v1/v2/v4 containers (magic + save-type + variant discriminator),
  extracts the inner TDB (raw, or zlib-inflate for v4), and **repacks v2 (byte-identical) and v4
  (semantic round-trip + edit), recomputing both verified checksums** and preserving any trailing
  payload. v1 stays read-only (`tag_1c` un-reversed); PFD/AES and `DATA000N` multi-part are deferred.
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

- **Metadata files** — anchor `nhlng-meta.xml` delivered (nhl11→legacy); nhl10 has none.
- **Relationship/companion/line model** — ✅ **verified** against the anchors and folded into Layer 2
  ([audits/roster-relationships.md](audits/roster-relationships.md)). The validation/line-visibility
  staging audits were verified at the schema level; remaining audit docs (`edit-session-architecture`,
  view-models, `db-capacity-limits`) are not in-repo — bring in if/when you choose.
  **Shipped:** the confirmed FK edges now live as the `tdb-relations` registry (`defs/relationships/`),
  and the whole roster→player→team bridge chain re-verifies at ≥99.98 % resolution against the
  committed PS3 NHL Legacy anchor (see `tdb-domain-validation`'s referential-integrity audit).
- **Record decoder — ✅ SOLVED & verified.** The TDB v6 descriptor layout (`kind_code`, bit-offset,
  id, bit_width) and BE bit-packing are verified ([formats/tdb-v6-format.md](formats/tdb-v6-format.md));
  records decode to sane values. **Write-path CRCs also verified** (inner-DB CRC-32/MPEG-2; container
  checksum_a/b) — so validated writes are now possible for `.db` and v2/v4 saves.
- **Enums — value-distribution VERIFIED** for `eGlu.mCcl` (captaincy 0/1/2), position, skin, jersey
  (decoded from anchors and confirmed). Full mappings (incl. nationality/birth-state/style, and
  `ZBac.RREB` status) are in [formats/tdb-v6-format-spec.md §6](formats/tdb-v6-format-spec.md#6-confirmed-valuelabel-mappings)
  and [audits/roster-relationships.md](audits/roster-relationships.md).
  **Shipped:** the 19 confirmed vocabularies now live as the `tdb-enums` registry (`defs/enums/`),
  keyed per studio `(table, field)` so `dSvy.QgFs` (player style) and `mHuy.QgFs` (goalie style) do
  not collide; the registry validates enum membership at edit entry and renders labels on read.
- **Per-slot labels** — the 71 one-bit line-assignment flags → hockey-position names not fully mapped.
- **17–21 metadata** — net-new; not covered by NHLViewNG (ROADMAP Phase 10).
