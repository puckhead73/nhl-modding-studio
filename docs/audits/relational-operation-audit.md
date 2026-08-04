# Relational operation audit (Phase 3)

> **Status:** established 2026-08-02 (Phase 3 of the
> [Full-game QA and editability plan](../plans/full-game-qa-editability.md)).
> **Automated gate:** `studio_domain::operation_matrix::operation_matrix_over_all_anchors`
> (`crates/studio-domain/tests/operation_matrix.rs`).
> **Machine-readable companion:** [`data/operation-matrix.csv`](data/operation-matrix.csv) — the
> applicability + outcome of every implemented roster operation on every one of the 27 main-DB
> anchors, drift-guarded by the same test.

Phase 2 proved every *scalar codec* read/write-inverts. Phase 3 proves the *structural* half of
`db-operations`: the high-level entity and roster operations — the ones that create records, move
occupancy between team blocks, and maintain relational invariants — run through the exact
`EditSession` seam the UI/API calls, hold every invariant, save/reopen cleanly, and undo to the anchor
byte-for-byte; and where a schema does not carry an operation, it is **refused cleanly**, never
half-committed.

## What "the same path the UI calls" means

Every proof drives `EditSession`'s operation methods — `create_player`, `clone_player`,
`delete_player`, `move_player`, `trade_slots`, `release_to_fa`/`sign_from_fa`,
`add_membership`/`remove_membership`, `set_captaincy`, `set_jersey_number`,
`autocorrect_duplicate_jerseys`, `set_dressed`, `retire_jersey`/`unretire_jersey`,
`repair_roster_blocks`, `trade_draft_pick`, `add_career_season`/`delete_career_row`, and the strategy
`set_field` — which are exactly what `studio_api::Registry` (and therefore the Tauri commands and the
React roster/team surfaces) invoke. Each runs inside the Layer-3 **repairing transaction** (delta-
audited, undoable). So a green matrix row means the production operation path, not a test-only
shortcut, satisfies the contract on that schema.

Anchors are immutable: each session opens a temporary copy, and every operation is undone before the
next runs, so the session returns to the anchor bytes between operations. `git status --short
anchors/` stays clean.

## Applicability is derived from the schema, never the title id

An operation is *applicable* to an anchor when the loaded database carries the tables and fields that
operation writes — probed off the file, exactly as the plan requires (never inferred from `game_id`).
The gen6 title (**NHL 07**) is where this matters: its schema is materially thinner, so several
operations are legitimately **not applicable** and must refuse rather than corrupt:

| Operation | Requires | NHL 07 (gen6) |
|---|---|---|
| create / clone player | `eGlu.toWX` slot key **and** the equipment companion tables (`xmja`/`fMVl`) | ✗ no `toWX`, no equipment tables → refused |
| delete player | the equipment companion tables it tears down | ✗ refused |
| move / trade / release / sign / membership | the `eGlu` occupancy pointer (`XSWT`) | ✓ occupancy-only ops apply |
| captaincy / jersey / dressed | the per-slot field (`mCcl` / `sVRt` / `hSZj`) | ✓ all present → apply |
| retire / un-retire number | the retired-number table (`wcSL`) | ✗ absent → refused |
| trade draft pick | the draft-pick table (`qHav`) | ✗ absent → refused |
| career add / delete | the career table carrying every field the add writes — year (`qFnd`), league flag (`tsSy.xMjj`), team code (`tsSy.lyxL`) | ✗ league field absent → refused |
| strategy edit | the strategy table (`EeYo`) | ✓ present → applies |

The distinction the matrix draws is precise: **occupancy relocation** (move/trade/release/sign/extra
membership) touches only the `XSWT` pointer and the bio's team, so it works on gen6, whereas **cluster
lifecycle** (create/clone/delete) allocates or tears down the full record cluster — bio + kind-correct
companions + index + slot — and so needs the modern (fat) schema. Both facts are proven, not assumed.

## The invariant battery (after every applied operation)

The plan's post-operation invariants reduce, at the engine level, to the shared save-time audit
(`tdb_domain_validation::validate_invariants`) plus a structural re-parse. After each applied
operation the harness asserts:

1. **No newly introduced audit finding** vs the anchor baseline. The comparison is
   **record-independent** (an insert/remove shifts every later record index, so a record-keyed
   comparison would flag identical pre-existing findings as new) and count-bounded: the set of
   `(predicate, team, player)` Error keys introduces no new entity, and the total Error count does not
   rise. This single predicate subsumes the plan's list — `referential_integrity` (no dangling FK),
   `companion_presence` (correct companion cardinality), `single_captain` (exactly zero/one captain),
   `towx_coherence` (team-block alignment / slot ownership), and `dup_on_team` (no duplicate roster
   membership or jersey) — because those *are* the audit's predicates.
2. **The entity model rebuilds** from the saved bytes (`RosterModel::build`), so the relational graph
   is still assemblable.
3. **The saved DB re-parses** with a valid header CRC and size, and a second no-op save is
   byte-identical (deterministic serialization).
4. **The operation is reversible** — undoing every entry it pushed restores the working bytes to the
   anchor exactly, and the session is no longer dirty.

Some anchors ship **pre-existing** Error findings (e.g. a vanilla team with more than one captain).
The baseline-relative comparison is what lets the matrix prove an operation "introduces no new
finding" without falsely demanding the operation *fix* unrelated vanilla data — matching the engine's
own delta-audit, which rolls back only on an *increase* in a predicate's count.

## Deterministic representatives

Entity representatives are chosen from a freshly-built model in a stable order (players/slots sorted by
team then record), so selection is byte-identical across runs and hosts — which is what lets
`operation-matrix.csv` be drift-guarded. Two selection subtleties the matrix encodes:

- **release-then-sign** targets a **single-membership** player: PS4 rosters carry players who hold
  several memberships (all-star / national rosters), and releasing one team's slot would leave such a
  player rostered elsewhere — not a free agent — so `sign_from_fa` would correctly refuse. The op is
  proven on a player whose one slot the release genuinely frees.
- **move / extra-membership** target a player who is **not already** a member of the destination team,
  so relocating them into its open slot cannot manufacture a duplicate membership.

When an applicable operation has no suitable representative on a given anchor, the row is recorded
`no-fixture` rather than forced — an honest gap, not a synthetic pass. In practice this is **move** and
**extra-membership** on the fully-packed gen7 PS3/Xbox rosters (NHL 07–14), which ship no vacant `eGlu`
slot for the player to move *into*; NHL 15 / Legacy and every PS4 roster ship an open slot and pass.
Relocation is still proven on those packed anchors by **trade-slots** (which needs no free slot) and
**release-then-sign** (which frees its own).

Two verified schema facts fall out of the applicability probe and are worth recording:
- **draft picks** (`qHav`) are absent on NHL 07 and the early gen7 titles NHL 08–10 (PS3/Xbox) — the
  era gap already noted in the cross-title drift audit — so `trade-draft-pick` refuses there;
- the **career-season table drops the inline team-code field (`tsSy.lyxL`) on PS4 NHL 18–21**, so the
  stock career-add path is not applicable on those late gen8 titles (it remains applicable on PS4
  NHL 15–17, which still carry it).

## Compound operations

Four rows exercise an operation and its inverse as a pair, so the net roster is coherent and every
step is reversible: **release → sign** (free a slot, sign the player back into it), **add → remove
membership**, **retire → un-retire** a number, and **career add → delete** a season row. The invariant
battery runs on the final state; undo unwinds both steps back to the anchor.

## The eight-per-anchor operation set

Player lifecycle — create skater, create goalie, clone skater, clone goalie, delete player, move
player, trade slots, release-then-sign, add-then-remove membership. Team/roster lifecycle — set
captaincy (single-captain by construction), set jersey (unique within team), autocorrect duplicate
jerseys, set dressed/scratched, retire-then-un-retire a number, repair roster blocks, trade a draft
pick, career add-then-delete, strategy edit (with the `JUfQ` career mirror). Endianness (big-endian
PS3/Xbox 360, little-endian PS4) is covered by running the set over both anchor kinds.

## Negative gate (schema-absent operations)

For every operation a schema does **not** carry, the harness still *invokes* it and asserts a **clean
refusal**: the call returns `Err`, and neither the working bytes nor the undo history changes. This is
the plan's "non-applicable operations are absent/disabled, never allowed to fail halfway through a
transaction" — proven on real gen6 data, not asserted from a capability flag.

## QA-matrix promotion

`db-operations` for every main-DB anchor stays `edit-roundtrip-pass`, but its `automated_test` now
cites this relational gate and its evidence lists **both** `mutation-matrix.csv` and
`operation-matrix.csv`. The `blocker` drops the "structural roster-op matrix is Phase 3" caveat: the
only outstanding axis for db-operations is **in-game acceptance** (`game-verified`), which requires the
manual emulator/console campaign, not another automated gate.
