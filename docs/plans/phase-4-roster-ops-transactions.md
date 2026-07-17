# Phase 4 — Roster ops & transactions (implementation plan)

> **Status:** ✅ COMPLETE 2026-07-14 (all sub-phases 4.0–4.7 done). Authored after Phase 3
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

### Spike S3 — Capacity ceilings  →  ✅ RESOLVED 2026-07-13
**Result:** [docs/audits/roster-capacity.md](../audits/roster-capacity.md). Ceilings for all seven
roster-bearing tables (`ZBac`, `ubPc`, `dSvy`, `xmja`, `mHuy`, `fMVl`, `eGlu`) read from real anchor
bytes across every title/platform (all CRC-clean). **Key finding — the ceiling is `max_records`, and
where it lives differs by generation:** the **GEN7 main `nhlng.db` is packed** (`max == cur`) and does
*not* expose it — the ceiling lives in the vanilla **PS3 ROSTER save's `max_records`** (which ships ~2×
the shipped population as slack); on **PS4 the main `nhl_game.db` already carries the slack**, so its
`max_records` *is* the ceiling (== vanilla ROSTER `max`). Cross-validated by two exact identities
(PS3 NHL 15 ROSTER `max` ≡ PS4 NHL 15 main `max`; PS3 Legacy ≡ PS4 NHL 16) — the ceiling is a property
of the roster-content **year**, platform-independent. **Consequences for 4.4:** (a) PS4 derives the
ceiling at runtime from the attached main DB (no constants); GEN7 must carry the doc's ceiling table as
a `title→ceilings` constant (packed main DB can't supply it); (b) normalization grows **`max_records`,
not `current_records`** (pre-allocate slack, matching the vanilla ROSTER shape); (c) **no committed
anchor is trimmed below its ceiling**, so within the anchor set the pass is a no-op — 4.4 must verify
growth against a **synthetically-packed fixture** (pack `max` down to `cur`, normalize back).

### Spike S4 (minor) — Enum labels for status/style  →  ⏳ MOSTLY RESOLVED 2026-07-13
Confirmed via the **NHLViewNG EXE Delphi RTTI** (unit `uDatabaseTypes`, per-instance clean-room
exception — same path as captaincy):
- **`eGlu.QgFs` player style — ✅ CONFIRMED.** The Phase 5T Player Style mapping, **re-verified
  1-based** against the committed legacy anchor: dSvy.QgFs bucketed by position shows D=1..4, F=5..10,
  no code 0 (0=unset), and code 9 (modal forward) = Two-Way Forward. The EXE RTTI
  `TNHLPlayerAiStyle [0..10]` is NHLViewNG's own 0-based editor enum — it corroborates codes 1..8/10
  at a +1 offset but is not the field codec (carries an internal `psDangler` the 4-bit field never
  uses; disagrees at code 9). The offset is **field-specific**: goalie `mHuy.QgFs` is 0-based. Shipped
  [player-style.toml](../../defs/enums/player-style.toml) now binds the roster table (`eGlu`, the S4
  target), skater-AI (`dSvy`), and both career-mode mirrors.
- **`eGlu.hSZj` — ✅ RESOLVED.** 1-bit ⇒ a Delphi `Boolean` (dressed/active flag); no enum vocab.
- **`ZBac.RREB` — ✅ RESOLVED (follow-up): it is PLAYER TYPE, not roster status.** `ZBac` is EA's
  "exhibitionplayers" index table; `RREB` (meta name "table", 2-bit) is the player-source
  discriminator `TNHLPlayerType {Built-In, User-Created, Special, Developer}` — confirmed by EXE RTTI
  (the only 4-value enum that fits), the "table" meta name, and a 1:1 correlation with the pointed-to
  bio `active` flag in the anchor. Shipped [player-type.toml](../../defs/enums/player-type.toml). The
  **real** dressed/scratched status is the 1-bit `eGlu.hSZj` boolean (anchor split `{0:1092,1:4440}`);
  "In Minors" is not stored there.

  > ✅ **Bug this uncovered — FIXED (2026-07-13).** `set_roster_status` (which wrote `ZBac.RREB`) was
  > replaced by **`set_dressed`**, writing the real per-slot boolean **`eGlu.hSZj`** (`1`=dressed,
  > `0`=scratched — direction verified against the anchor: dressed rows hold line flags, scratched
  > hold none). The entity/DTO field `roster_status` became **`dressed: Option<bool>`**; `LinesTab`
  > now shows a Dressed/Scratched toggle (no In-Minors tab). The "in the minors" concept is now
  > purely a **front-end Depth-Chart identifier** — the Depth Chart's **"In the system"** section:
  > players under this team's contract/rights whose main team (`dXSB`) is elsewhere. `ZBac.RREB` is exposed only
  > as the confirmed Player Type enum. Cross-stack (txn/session/workspace/registry/command + DTO +
  > regenerated TS + `LinesTab`/`JerseyCaptaincyTab`/`teamPresentation`/Depth Chart); tests green.

Ops can write the codes regardless; this only governs whether the UI labels are shown as confirmed or
"meta-derived (unverified)". Non-blocking — ran alongside 4D.

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

**✅ DONE (2026-07-14).** Implemented per S3's findings ([roster-capacity.md](../audits/roster-capacity.md)):
the ceiling is an **allocation** target, so the pass grows **`max_records`, not `current_records`**
(the vanilla `cur < max` ROSTER shape — population is never invented).

1. **`tdb-core::grow_table_max(table, new_max)`** — `append_record`'s growth path generalized to a
   bulk pre-allocation: appends zeroed slack to the table's record region, shifts later directory
   offsets, updates `max_records`/`db_size`, recomputes CRCs; idempotent at/below the current
   allocation. Anchor-verified test alongside the other alloc primitives (`tests/alloc.rs`).
2. **`EditSession::grow_table_max`** (canonical id) — runs **outside** the undo history (a load-time
   repair, not a user edit); skips tables absent from the file's subset.
3. **`Workspace::normalize_roster_capacity()`** — roster mirror only (never the main DB). Ceiling
   per table = **max(GEN7 constant, attached main DB's `max_records`)**: the GEN7 `game_id →
   ceilings` table lives in `studio-domain::capacity` (traces to the audit doc — the packed GEN7
   main can't supply it), while the main-DB read is exact on PS4-style mains and a safe no-op floor
   on GEN7. Re-runs `analyze` after growth so `shared_tables`/`aligned` reflect the new shape.
   Called at workspace open in `Registry::open_workspace`, right after `set_game_id`.

*Verified* (`studio-domain/tests/capacity.rs`, on temp copies of the nhl14 PS3 anchors): a
**synthetically-packed fixture** (all seven tables' `max` cut to `cur`, the S3-mandated shape) opens,
normalizes to the audited ceilings with every populated record intact, stays aligned, saves
CRC-valid, and re-parses at the ceiling; a second pass is empty (idempotent); the vanilla anchor is a
byte-level no-op (not dirty); a post-normalization `create_player` consumes slack without growing the
roster file.

---

## Phase 4.5 — Service layer: commands, DTOs, convert (wire every 4.1-4.4 op)

**✅ DONE (2026-07-14).** Every 4.1–4.2 op already had its triplet (registered in both
`command_handler!` and `export_typescript`); the remaining 4.5 gap was the **Transactions pool
reader**. Added **`list_free_agents`** (registry → `#[tauri::command]` → `command_handler!`),
reusing the `PlayerSummaryDto` shape rather than a parallel reader: the free-agent pool is exactly
the `team_id == None` slice of `player_summaries` (Spike S1's occupancy model — off-roster ⇔ no
`eGlu` slot; team resolution already lives in the master-list builder). No new DTO, so bindings are
unchanged. The team-roster side the two-team panel needs is the existing `lines_view` per team.

*Verified* (`studio-api/tests/registry.rs::free_agent_pool_through_registry`): the pool equals the
master list's team-less slice; `release_to_fa` moves a player **into** the pool and `sign_from_fa`
moves them back **out**, with the master list resolving them onto their team again.

> ✅ **Bug uncovered & FIXED (2026-07-14) — `vfEq==0` occupancy key.** Building the reader's
> round-trip test surfaced a latent bug in **`sign_from_fa`/`add_membership`**: both picked the
> player's occupancy key via `player_index_keys(p).min()`. Some players carry a spurious `ZBac`
> index row with `vfEq == 0` (e.g. the first rostered player on the Legacy anchor has keys
> `{0, 1}`), so `.min()` returned `0`; `occupy_slot` wrote `XSWT = 0`, which `RosterModel::build`
> reads as a **vacant** slot — the sign silently no-op'd (player stayed off-roster). Fixed with a new
> `player_slot_index_key(p)` that excludes `0` **and** the `XSWT` all-ones sentinel (mirrors the
> model's slot-emptiness test) and takes the smallest remaining key. `delete_player`'s use of the
> full key set is unchanged. Regression-guarded in both `tdb-edit` (round-trip) and the registry
> test (master list resolves the signed player back onto the team).

---

## Phase 4.6 — Frontend: Transactions view + repair tools

**✅ DONE (2026-07-14).**

1. **`app/src/pages/Transactions.tsx`** — the two-team panel already shipped
   (move/copy/release-to-FA/delete/trade-pick); 4.6 finished it now that S1/4.2 are in:
   - **Free-agent pool + Sign.** Each column gains a **Free Agents** mode (a third toggle beside
     Players / Draft Picks) that lists the global pool from the new `list_free_agents` reader; the
     **Sign** action calls `sign_from_fa(player, otherTeam)` (installs the FA into the *other*
     column's team, mirroring how every verb acts relative to the opposite column). The pool query
     invalidates on every op, so release/sign move players across the boundary live.
   - **Trade (swap).** Each column reports its selected player up to the parent, which renders a
     central **⇄ Trade** control enabled only when both columns hold a rostered player on *different*
     teams; it calls `trade_slots(aTeam, aPlayer, bTeam, bPlayer)` (in-place swap, no free slot
     consumed).
   - Every action is a mutation with `onSuccess → invalidateQueries` (teams, players, free-agents,
     validation) + project-summary refresh (dirty); the backend's structured `Err(String)`
     (`TeamRosterFull`, dup, ambiguous) surfaces inline; destructive verbs (delete / release) confirm.
2. **Repair tools (Dashboard).** Finding groups now carry an inline repair affordance where a **real
   op** backs the category: the **duplicate-jersey** flavour of `dup_on_team` (keyed on
   `field == sVRt`) gets a **Fix duplicate jerseys** button running the whole-DB `autocorrect_jerseys`
   op (audited, undoable), then re-running the audit. Error categories with **no** automated op (the
   player-in-two-slots `dup_on_team`, `single_captain`, `companion_presence`, `towx_coherence`) are
   tagged **"Manual fix"** so the absence of a button is intentional, not an oversight. (A safe
   single-captain auto-demote needs the offending team+player ids, which the resolved `FindingDto`
   deliberately doesn't expose — deferred until the DTO carries them.)
3. **Player style / Player type / dressed** — already shipped with S4 (the enum bindings and the
   `set_dressed`/`eGlu.hSZj` repoint); no further 4.6 work.

*Verified:* `cd app && npm run build` (tsc + vite) clean; the op round-trips are covered by the
backend registry/txn tests (4.2/4.5). Functional in-app move/copy/trade/sign/release/delete driving
is exercised via those command-level tests; a live emulator round-trip stays gated per CLAUDE.md
rule 3 for the net-new (title, platform) matrix.

---

## Phase 4.7 — Final verification (gate sign-off)  ✅ DONE (2026-07-14)

1. **Functional:** two-team roster management (move/copy/trade/sign/release/delete + FA pool) is
   wired end-to-end through the audited, dual-file-mirrored ops; the command-level round-trips are
   green on the proven-title anchors (registry + txn tests). A live emulator round-trip for the
   **net-new** (title, platform) cells remains gated per CLAUDE.md rule 3.
2. **Tests:** `cargo test --workspace` green (every crate 0 failed; intermittent Windows
   Application-Control blocks on freshly-linked test binaries — os error 4551 — clear on relink and
   are not test failures). `cd app && npm run build` (tsc + vite) clean. `cargo clippy --workspace
   --all-targets` introduced **zero** new warnings (remaining notes are pre-existing, in crates this
   phase didn't touch — e.g. `find_record` dead-code in `txn.rs`, `div_ceil` in the formula/fhm
   crates). Capacity normalization has grow + idempotency + no-op tests; the FA reader and the
   `vfEq==0` fix have regression tests.
3. **Anti-pattern grep:** no op bypasses `with_repairing_transaction`; the new capacity pass is a
   load-time repair (grows `max` only, `cur`/alignment preserved) so it correctly sits outside the
   op history and the `fully_aligned` structural guard; capacity ceilings live **only** in
   `studio-domain::capacity` and trace to `roster-capacity.md` (workspace.rs references the function,
   never raw numbers); no generated bindings hand-edited (no DTOs added — `list_free_agents` reuses
   `PlayerSummaryDto`).
4. **Format-fact honesty:** S1–S4 provenance/confidence recorded in their audits; nothing unverified
   is asserted in-game-true. The `vfEq==0` occupancy-key discovery (4.5) is verified against the
   committed anchor.
5. Spikes recorded (S1–S4); no architectural assumption changed in DESIGN/ROADMAP beyond what the
   spike audits already captured.

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
- **Capacity ceilings** — ✅ resolved (S3, [roster-capacity.md](../audits/roster-capacity.md)): ceilings
  read from real bytes. GEN7 ceiling = vanilla PS3 ROSTER `max_records` (main DB is packed); PS4 ceiling
  = `nhl_game.db` `max_records`, runtime-derivable. Grow `max`, not `cur`. PS4 titles beyond the proven
  Legacy/14/15 set are real reads but still gated on a round-trip/in-game check before asserted working.
- **Enum labels (S4)** — all resolved. Player style (re-verified 1-based vs the anchor; bound to `eGlu`
  + career mirrors), captaincy, goalie style, and **`ZBac.RREB` = player type**
  ([player-type.toml](../../defs/enums/player-type.toml)) are confirmed; `hSZj` is the boolean dressed/scratched flag. The follow-up also uncovered
  (and **fixed**) a cross-stack bug — the dressed/scratched feature had been wired to `RREB` (player
  type); it is now the `set_dressed` op on `eGlu.hSZj`, and the "in the minors" concept is the Depth
  Chart's "In the system" section (front-end only). No enum labels remain "meta-derived".
