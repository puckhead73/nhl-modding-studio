# Phase 4 — Roster ops & transactions (implementation plan)

> **Status:** 📋 scoped 2026-06-30 (not started). Authored after Phase 3
> ([phase-3-ea-db-core-views.md](phase-3-ea-db-core-views.md), shipped 2026-06-28).
> **Gate (ROADMAP.md:38-40):** entity ops (repairing transactions), Transactions view, roster
> capacity normalization, repair tools — a usable two-team roster-management surface for the proven
> titles, every op audit-clean and dual-file mirrored.
>
> **The Phase 1 head start.** Most of the *engine* this phase needs already shipped: the
> repairing-transaction harness, record allocation, six entity ops, the dual-file `Workspace`
> mirror, and four Layer-4 predicates (see "What already exists"). Phase 4 is therefore **finishing
> the op set, completing the Layer-4 audit, adding capacity normalization, and building the
> Transactions frontend** — not building the transaction machinery.
>
> **Locked decisions:** (1) free-agent / hidden pools are handled **spike-first** — the data
> representation is reverse-engineered against anchors (Spike S1) *before* `free_agent` is scoped or
> coded; (2) every new entity op flows through the existing `Workspace` fan-out so the attached
> roster save mirrors automatically; (3) `assign_line_slot` stays **deferred** (the 71 line flags are
> not mapped to positions — wiring it is guesswork, ROADMAP verify-don't-guess).
>
> Each sub-phase is self-contained for execution in a fresh context. Frame tasks as **copy the cited
> pattern**, not "transform existing code". Verify format facts against in-repo anchors; never assert
> a binary-layout specific you have not confirmed (CLAUDE.md hard rule 3).

---

## What already exists (Phase 4 builds on this — do not rebuild)

| Capability | Location | Notes |
|---|---|---|
| `with_repairing_transaction` (snapshot → stage → auto-repair → audit → commit/rollback) | `tdb-edit/src/txn.rs` | audit currently runs `single_captain` + `companion_presence` |
| Record alloc: `append_record` / `remove_record` (swap-pop, full-table growth, CRC-safe); `CapacityExhausted`; `mint_id` (smallest-free non-sentinel) | `tdb-core/src/lib.rs`, `tdb-edit/src/txn.rs` | the only allocation primitives — no `tdb-expand` crate |
| Ops: `set_captaincy`, `create_player`, `delete_player`, `move_player`, `set_jersey_number`, `set_roster_status`, `set_field` | `tdb-edit/src/txn.rs:296-611`, `studio-domain/src/session.rs:319-346` | each audited + undoable |
| Dual-file mirror: attribute ops via `Workspace::fan_out_entity`; structural ops (`create`/`delete`) via explicit dual-create + `fully_aligned` rollback | `studio-domain/src/workspace.rs:206-291` | **every new op must use one of these two paths** |
| Layer-4: `referential_integrity` (warn), `enum_vocabulary` (info), `companion_presence` (error), `single_captain` (error); composed by `validate_invariants` | `tdb-domain-validation/src/lib.rs:179-318` | dup-on-team and `toWX` coherence **absent** |
| Commands + registry + DTOs for every shipped op; `OpOutcomeDto`/`CreatePlayerDto`; undo/redo/save | `studio-api/src/{commands,registry,convert,dto}.rs` | the triplet pattern to copy |
| Frontend: read-only Players/Teams/Arenas/Lines/Dashboard/PendingEdits; typed `client.ts`; bindings | `app/src/pages/`, `app/src/bindings/` | **no Transactions / roster-ops UI** |

### The two dual-file op shapes (pick the right one for every new op)
- **Attribute op** (mutates fields on existing records, keyed by entity id — no allocation):
  `Workspace::fan_out_entity(|s| s.the_op(...))` (`workspace.rs:206`). Applies to `nhlng`, mirrors to
  the roster by the same entity id, rolls `nhlng` back on mirror error. Use for: `trade_slots`,
  `hide`/`unhide`, `retire_jersey`, and `set_*` ops.
- **Structural op** (allocates/removes records → record indices move): copy `create_player`/
  `delete_player` (`workspace.rs:228-279`) — dual-create, then `fully_aligned()` re-check, `roll_back_both`
  on any divergence. Use for: `clone_player`, and `free_agent` **iff** the pool model turns out to
  move records (Spike S1 decides).

---

## Phase 4.0 — Discovery spikes (gate the rest; do before coding the dependent ops)

These are **data-RE tasks**, not coding tasks. Each produces a short audit note under `docs/audits/`
and either confirms a fact or leaves the dependent op deferred. Run them first; they unblock 4A/4B/4C.

### Spike S1 — Roster-pool model (main / free-agent / hidden)  →  ✅ RESOLVED 2026-06-30
**Result:** [docs/audits/roster-pools.md](../audits/roster-pools.md). Pool membership = **slot
presence at the index level**: a player is off-roster iff its `ZBac` index row is referenced by no
`eGlu` slot (556 such on the anchor). **No sentinel team** (largest roster = 35) and **no
unindexed-bio "hidden" pool** (0 bios lack a `ZBac` row). The free-agent-vs-hidden *sub-label* is a
**candidate** `ZBac.RREB` code (codes 1/2 appear only on unplaced rows) — labels unconfirmed, still
need operator confirmation (Spike S4). **Consequence:** `free_agent` (4.2) is an *occupancy* op
(unplace / place into a team block), not a field flag — and signing into a full block needs capacity
growth (S3 / 4.4).

### Spike S2 — `toWX` block invariant  →  ✅ RESOLVED 2026-06-30
**Result:** [docs/audits/towx-blocks.md](../audits/towx-blocks.md). Blocks are **REAL**: all 222
populated teams have contiguous record *and* `toWX` runs, zero overlaps/gaps — `eGlu` is partitioned
into per-team blocks that tile the table. Size is **variable 20–35** (the prior app's fixed "40" was
wrong in magnitude; the structure was right). Only **1 empty slot DB-wide** → blocks are full.
**Consequences (reshape the op model):** (a) a slot is a fixed per-team container — move/trade must
rewrite the **occupant (`XSWT`)**, not the slot's team (`dXSB`); (b) the old `move_player` retagged
`dXSB` and so *violated* block coherence — **fixed** as occupant relocation (4.1); the
`towx_coherence` predicate (4.3) still needs adding so the audit guards against regressions; (c)
signing into a full block requires block growth (capacity, 4.4).

### Spike S3 — Capacity ceilings  →  gates 4C normalization
**Question:** what are the per-(title, platform) full-size record counts for the roster-bearing tables
(`ubPc`, `ZBac`, `dSvy`, `xmja`, `mHuy`, `fMVl`, `eGlu`)? The ceiling is the **vanilla save's** table
size, against which a game-trimmed roster save is grown.
**Method:** read `current_records`/`max_records` for those tables from the bundled vanilla saves in
`anchors/` (e.g. `anchors/ps3/nhllegacy/`, `anchors/ps4/nhl15|nhl16/`); compare to a trimmed roster
save's counts to confirm growth is actually needed and by how much.
**Output:** `docs/audits/roster-capacity.md` — a ceiling table sourced from real bytes (no hardcoded
guesses). Feeds the normalization pass in 4C.

### Spike S4 (minor) — Enum labels for status/style
Confirm `ZBac.RREB` roster-status and `eGlu.QgFs` player-style value→label mappings (currently
**candidate**, [roster-relationships.md:70-76](../audits/roster-relationships.md)). Ops can write the
codes regardless; this only governs whether the UI labels are shown as confirmed or
"meta-derived (unverified)". Non-blocking — can run alongside 4D.

---

## Phase 4.1 — Backend: finish the on-team entity ops (no pool dependency)

The ops that need **no** Spike-S1 result. Each is: a `tdb-edit::txn` method on the repairing
transaction → a thin `EditSession` delegate (`session.rs` `self.run(|tx| …)`) → a `Workspace`
wrapper on the correct dual-file path → the `studio-api` triplet.

1. **`clone_player(source_player, to_team)` — ✅ DONE (2026-07-01).** Backs Transactions "copy".
   Structural op reusing `create_player`'s cluster allocation, but the bio + kind-correct companion
   pair are **deep-copied** from the source (new `tdb-core::copy_record` clones the whole record block —
   ratings, name, appearance, equipment — in one shot), then the identity is overridden: fresh minted
   `wBIz` (re-keying bio + companions), fresh `ZBac` index (`vfEq` → new `wBIz`), bio team set to
   `to_team`, and a block-aware `eGlu` slot (reuse/insert). The source's slot state (jersey/captaincy)
   is **not** copied — the clone gets a clean slot (shared `occupy_slot`, which also cleaned
   `create_player`/`sign_from_fa`'s reuse path). Source may be rostered or a free agent. **Dual-file:**
   `Workspace::clone_player` copies the `create_player` dual-create + `fully_aligned` guard. Wired
   through session/registry/commands (returns `CreatePlayerDto`). *Verified:* new cluster present,
   every fixed-bit bio field deep-copied except identity/team, companion invariant holds, goalie clone
   gets the goalie pair, audit-clean, counts +1 cluster; bad-args rejected.

2. **`move_player` re-modeled as occupant relocation — ✅ DONE (2026-06-30).** S2 proved `eGlu` is
   partitioned into full, contiguous per-team blocks, so a slot is a fixed *(team, slot#)* container
   and the old `move_player` (retag `dXSB`) corrupted blocks. Fixed: `move_player(player, from_team,
   to_team)` transplants the occupant (`XSWT` + per-slot fields) into a free destination-block slot,
   vacates the source by zeroing `XSWT`, never touches `dXSB`; keyed by `from_team` because a player
   holds multiple memberships (~6 %). Errors `TeamRosterFull` when the block is full. Signature change
   propagated through session/workspace/registry/commands; regression test asserts block coherence
   ([towx-blocks.md](../audits/towx-blocks.md)).

   **`trade_slots(a_team, a_player, b_team, b_player)` — ✅ DONE (2026-07-01).** Swaps two occupants in
   place (the general op when both blocks are full): each occupant (`XSWT` + per-slot fields) is
   captured and written into the *other* slot via the shared `capture_occupant`/`apply_occupant`
   primitives (refactored out of `transplant_slot`), leaving both slots' `dXSB`/`toWX` untouched — so
   blocks stay coherent and no free slot is consumed. Clears captaincy on both arrivals, follows each
   bio's team, and clears a jersey that collides on arrival (shared `clear_colliding_jersey`, also now
   used by `move_player`). Attribute op → `fan_out_entity`. Wired through
   session/workspace/registry/commands (reuses `OpOutcomeDto`). Tests: in-place swap leaves both blocks
   coherent + audit-clean, the colliding-jersey repair, and bad-arg rejection.

3. **`hide(player)` / `unhide(player)`.** Attribute op once S1/S4 confirm the backing field (likely a
   `ZBac.RREB` / status code or a pool move). If S1 shows hidden is a pool, fold into the pool op
   (4.2) instead. **Dual-file:** `fan_out_entity`. *Verify:* round-trips; status reads back.

4. **`retire_jersey(team, number)` / `unretire_jersey(team, number)` — ✅ DONE (2026-07-01).**
   Metadata check first (rule 3) **corrected the plan's assumption:** retired numbers are **not** a
   field set on `kOtt` — they live in a **separate table `wcSL`** ("stockretirednumbertable", verified
   against the meta *and* anchor bytes: 322 rows, each a `(dXSB team, JFiY number)` pair — e.g.
   Montreal's eleven retired numbers). So these are **structural** ops (append/remove a `wcSL` row),
   not attribute ops: `retire_jersey` appends the pair (idempotent — no-op if already retired,
   range-checks the 7-bit number, requires an existing team); `unretire_jersey` swap-pop-removes the
   matching row (idempotent). **Dual-file:** *not* `fan_out_entity` — `wcSL` may be absent from the
   roster save, so a new `Workspace::fan_out_shared` mirrors only when the table is shared+aligned
   (else `nhlng`-only, the `set_field` degrade pattern). Wired through session/registry/commands
   (reuses `OpOutcomeDto`). *Verified:* retire then un-retire round-trips the `wcSL` row count,
   audit-clean; idempotence and bad-args covered. (`JMxx` "leagueretirementtable" — named
   league-retired players — is a separate concept, out of scope here.)

**Copy-from:** op shape `set_captaincy`/`move_player` (`txn.rs:296,526`); `EditSession` delegate
(`session.rs:319-346`); `Workspace` attribute vs. structural paths (`workspace.rs:187-279`); the
`studio-api` triplet + dual registration in `command_handler!` (`lib.rs:27`) and `export_typescript`
(`lib.rs:54`).
**Anti-pattern guards:** every op through `with_repairing_transaction`; structural ops through the
`fully_aligned` guard (never a bare `fan_out_entity` for an op that allocates); `mint_id` picks
smallest-free non-sentinel; no hand-edited DTO TS.

---

## Phase 4.2 — Backend: `free_agent` (sign / release)  ⟵ S1 resolved: occupancy model

**✅ DONE (2026-07-01).** S1 confirmed off-roster = an indexed player (`ZBac` row) occupying no `eGlu`
slot — **no sentinel team, no unindexed-bio pool** — so the ops are occupancy changes on the fixed
per-team blocks (same primitives as `move_player`), **not** `dXSB` retags:
- **`release_to_fa(from_team, player)`** = vacate the player's slot on `from_team` via the shared
  `vacate_slot` (zero `XSWT` + clear captaincy so no phantom captain lingers). The slot stays in the
  block (now empty); bio/index rows untouched → the player is off-roster but keeps their identity.
  Attribute op (`fan_out_entity`). `ZBac.RREB` pool code intentionally **not** written (label still
  candidate — Spike S4; S1 shows membership is defined by slot presence, so unslotting suffices).
- **`sign_from_fa(player, to_team)`** = install the free agent's `ZBac.vfEq` into a **free slot in
  `to_team`'s block** (clean arrival state: captaincy cleared, jersey unassigned; bio team follows).
  Rejects an already-rostered player (`AlreadyRostered`). Blocks are full, so it errors
  `TeamRosterFull` when the target block has no free slot — on-demand block growth stays deferred to
  4.4 to keep sign an attribute op (`fan_out_entity`); the natural flow (release frees a slot, then
  sign into it) works today.

Wired through session/workspace/registry/commands (reuses `OpOutcomeDto`). *Verified:* release then
sign round-trips a player back onto a team, both blocks coherent, audit-clean incl. `towx_coherence` +
`dup_on_team`; full-team sign and already-rostered/unknown-team args rejected; undo via the shared
`run`/`fan_out_entity` path. **The FA-vs-hidden sub-label** stays gated on Spike S4 (candidate `RREB`
codes). *Also fixed here:* `move_player`'s source-vacate now clears captaincy too (was a latent
phantom-captain on the emptied slot), via the shared `vacate_slot`.

---

## Phase 4.3 — Backend: complete the Layer-4 invariant engine  ⟵ S2 informs the toWX predicate

Add the two predicates the EDIT-MODEL contract requires (EDIT-MODEL.md:166-189) to
`tdb-domain-validation`, each a `fn(&TdbFile, …) -> Vec<Finding>` composed into both the txn audit
(`txn.rs` `with_repairing_transaction`) and save-time `validate_invariants` (`lib.rs:309`) so
operation-time and save-time never drift:

1. **`dup_on_team` — ✅ DONE (2026-07-01).** Composed into both audit paths. Within one team: no
   player occupies two slots, and no two **occupied** slots share a non-sentinel jersey (jersey `0` =
   unassigned, exempt — verified: team 107 legitimately carries three `0`-jersey slots). Severity:
   **error**. Promotes the op-time `set_jersey_number` uniqueness check to a whole-DB invariant and
   adds the same-player guard. Surfaced (and fixed) a real gap: `move_player` brought the player's
   jersey into the destination team; it now **clears a colliding jersey to unassigned on arrival**
   (a repair, like the captaincy clear). `set_jersey_number` itself already blocks the interactive
   case.
2. **`towx_coherence` — ✅ DONE (2026-06-30).** Composed into both the txn audit and save-time
   `validate_invariants`. Enforces the **record-block** structure: each team's `eGlu` slots form a
   **contiguous record run** and the per-team blocks **tile with no overlap or gap** (Severity:
   **error**). **`toWX`-key contiguity is *not* enforced** — measurement showed `toWX` is a globally
   monotonic unique key with inter-block gaps, so per-team contiguity is a creation-order artifact that
   ordinary edits legitimately break (see [towx-blocks.md](../audits/towx-blocks.md) pt 4). This
   required making the structural ops block-aware first: **`create_player`/`delete_player` were
   re-modelled** (new `tdb-core` `insert_record` + `remove_record_ordered`; in-block slot placement;
   ordered `eGlu` removal) so they satisfy the predicate by construction — the NHLViewNG record model
   validated this ([nhlviewng-record-model.md](../audits/nhlviewng-record-model.md)). Remaining 4.3
   work: the **`dup_on_team`** predicate.

Copy the existing predicate shape (`single_captain`, `lib.rs:276`) and extend the `validate_invariants`
composition (`lib.rs:314-318`) and the txn audit set. *Verify:* synthetic dirty fixtures (dup-on-team,
broken slot) flag the right `Finding`; the clean anchor stays clean; ops that would create a dup now
roll back.

---

## Phase 4.4 — Backend: roster capacity normalization  ⟵ gated on Spike S3

Auto-grow trimmed roster-save tables to the vanilla ceilings (DESIGN.md:202) so edits never hit a
premature `CapacityExhausted`. No new crate — grow via `tdb-core::append_record`'s full-table-growth
path (verified to preserve other records + CRC scopes, EDIT-MODEL.md:140-142).

1. A `studio-domain` normalization pass: for each roster-bearing table, if
   `current_records < ceiling(title, platform)` (ceilings from S3's `roster-capacity.md`), grow to the
   ceiling. Run it at **workspace open** (`Workspace::open`, `workspace.rs:56`), after alignment
   analysis, before the session is handed out — and re-run `analyze` so `aligned`/`shared_tables`
   reflect the grown shape.
2. Make it idempotent (a vanilla-sized file is unchanged) and a no-op for `nhlng.db` (only roster
   saves are trimmed).

*Verify:* open a trimmed roster anchor → tables reach the ceiling, every pre-existing record intact,
CRC-valid, audit-clean; re-open the result → no further growth (idempotent); a `create_player` that
previously would exhaust capacity now succeeds.

---

## Phase 4.5 — Service layer: commands, DTOs, convert (wire every 4.1-4.4 op)

For each new op add the triplet (registry method → `convert::*_dto` → `#[tauri::command]`), registered
in **both** `command_handler!` (`lib.rs:27`) and `export_typescript` (`lib.rs:54`). Most ops reuse
`OpOutcomeDto`; `clone_player` returns a `CreatePlayerDto`-shaped result (new id + outcome). Add a
read command the Transactions UI needs: a roster-pool / team-roster listing that surfaces each
player's team, pool (post-S1), jersey, and status — extend the existing `team_view`/`lines_view`
readers rather than adding a parallel reader.
*Verify:* `cargo test -p studio-api` regenerates bindings (new `*Dto.ts`); a registry test exercises
each command end-to-end on the anchor.

---

## Phase 4.6 — Frontend: Transactions view + repair tools

1. **`app/src/pages/Transactions/`** (clone the Players page structure, Phase 3.4): a two-team panel
   (pick team A / team B), each showing its roster from the 4.5 reader; actions **move / copy / trade /
   release-to-FA / sign / delete**, each a `useMutation` over the matching command with
   `onSuccess` → `invalidateQueries` for the affected teams + `['validation']` + project summary
   (dirty). Surface the backend's structured `Err(String)` inline (capacity-full, dup, ambiguous).
   Free-agent column appears only once S1/4.2 ship; until then, hide it (don't fake it).
2. **Repair tools (Dashboard):** wire the existing `validate_project` → `IntegrityReportDto` findings
   to inline repair buttons that run the relevant op (e.g. demote-extra-captain via `set_captaincy`,
   companion/dup repairs once their ops exist). Only wire repairs backed by a real op; label the rest
   "manual".
3. Status/style labels rendered from S4 (confirmed) or badged "meta-derived (unverified)".

*Verify (functional gate):* with an anchor loaded, move/copy/trade/delete between two teams persist,
flip dirty, keep validation clean, and survive save→reload; an out-of-capacity sign and a dup-jersey
surface the backend error without corrupting state; `npm run build` + `cargo test --workspace` green.

---

## Phase 4.7 — Final verification (gate sign-off)

1. **Functional:** two-team roster management (move/copy/trade/delete, + FA if S1 shipped) works
   end-to-end on a proven-title anchor, dual-file mirrored, audit-clean, save→reload stable.
2. **Tests:** `cargo test --workspace` + `cargo clippy --workspace --all-targets` clean; `cd app &&
   npm run build` clean. New predicates have dirty-fixture tests; capacity normalization has an
   idempotency test.
3. **Anti-pattern grep:** no op bypassing `with_repairing_transaction`; no structural op skipping
   `fully_aligned`; no hardcoded capacity ceilings (must trace to `roster-capacity.md`); no
   hand-edited bindings.
4. **Format-fact honesty:** every spike output records provenance/confidence; nothing unverified
   (pool model, `toWX` blocks, enum labels, line flags) is asserted as in-game-true (CLAUDE.md rule 3).
5. Update ROADMAP/DESIGN notes if a spike changed an architectural assumption; record S1-S4 findings.

---

## Risks & deferrals

- **Roster-pool model (S1)** — ✅ resolved: off-roster = unplaced `ZBac` index (no sentinel team, no
  unindexed-bio pool). Residual risk: the FA-vs-hidden `RREB` sub-label is still candidate (Spike S4).
- **`toWX` block invariant (S2)** — ✅ resolved: real, contiguous, full, variable-size blocks. This
  invalidated the old `move_player` (retag `dXSB`), now **fixed** (occupant relocation, keyed by
  `from_team`); adding the `towx_coherence` predicate (4.3) remains load-bearing so the audit catches
  any future regression.
- **`assign_line_slot`** — deferred (71 line flags unmapped). Stays out until an emulator round-trip
  maps slot→position.
- **Capacity ceilings** — must come from real vanilla-save bytes (S3), never hardcoded; titles beyond
  the proven Legacy/14/15 set are not in-scope to *verify* here.
- **Enum labels** — status/style stay "meta-derived" in the UI until S4 confirms; ops write codes
  regardless.
