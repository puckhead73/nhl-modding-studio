# Spike + design — retroactive roster-block repair

> **Status: SPIKE / NOT YET IMPLEMENTED.** This captures the design and the go/no-go verification
> protocol for an operation that *repairs* an already-corrupted `eGlu` roster-block layout. Prevention
> of new corruption already exists (block-aware move/trade/create/delete). This is about healing DBs
> that were corrupted by the old `dXSB`-retag `move_player` (or hand edits) **before** the fix.
> No structural-mutation code ships until the verification below passes against the anchor.

## Problem

The `towx_coherence` audit flags DBs like the user's "intentionally old and errored" one:

```
team 23 eGlu block is not a contiguous record run: gap between records 541 and 5609
(block spans 520..=5626, 40 slots)
```

Per [towx-blocks.md](../audits/towx-blocks.md): `eGlu` is partitioned into **disjoint, record-ordered,
per-team blocks that tile the whole table with no gaps** — the stable identity of a slot is
`(toWX, dXSB) = "team T's slot #k"`, and the variable content is `XSWT` (occupant) + jersey / captaincy
/ line flags. A slot carrying team 23's `dXSB` while sitting physically at record 5609 (far outside
23's real 520–541 region) is the exact fingerprint of the **old retag bug**: the physical block
position is fine; the `dXSB` *label* — and thus the tiling — is wrong.

## The core difficulty (why this is a spike, not a patch)

Once `dXSB` is corrupted, `dXSB` can no longer be trusted as the source of truth for block ownership.
A repair must re-derive **which team owns each physical block region** from something the corruption
did *not* touch. Candidate sources of truth, to be tested in the spike:

1. **`toWX` runs.** `toWX` is a globally monotonic unique key whose per-team runs also tile in record
   order (towx-blocks.md finding). A retag changes `dXSB` but (per the corrected op model) should not
   change `toWX`, so contiguous `toWX` runs may reveal the true block boundaries even where `dXSB`
   lies. **Unknown:** does the old buggy path leave `toWX` intact? Verify.
2. **Dominant `dXSB` per contiguous region.** Within each physical run, the untouched majority still
   carries the correct id; strays are the minority. **Unknown:** robust only if corruption is sparse.
3. **`kOtt` block-extent metadata**, if any exists independent of `eGlu`. **Unknown:** none found so
   far — confirm there is truly no per-team slot-range field on `kOtt`.

## Two repair shapes (the op must pick per finding, or offer both)

- **(A) Relabel-in-place** — the physical slot is where it belongs; only its `dXSB` is wrong. Rewrite
  `dXSB` back to the region's true owner. Occupant, `toWX`, jersey, etc. stay put. Cheapest, no record
  motion, preserves everything else. Correct when the *slot* was mis-tagged.
- **(B) Relocate-occupant** — the occupant genuinely belongs to the labeled team and should live in
  that team's real block. Re-run the corrected `move_player` semantics: transplant `XSWT` + per-slot
  fields into a free slot in the true team's block, vacate the stray slot (zero `XSWT`), restore the
  stray slot's `dXSB` to its physical owner. Correct when the *occupant* was mis-placed.

These produce different rosters. The observed finding (a lone stray at 5609) is most likely (A) — the
retag bug mislabels the slot without moving the player — but the spike must confirm which shape matches
the real corruption before we auto-apply either. Auto-guessing wrong silently re-shuffles a roster.

## Proposed algorithm (candidate, pending spike)

1. Build `RosterModel` over the working bytes.
2. Derive **true block ownership** per physical record region from the source of truth chosen by the
   spike (likely `toWX` runs, cross-checked against dominant `dXSB`).
3. For each slot whose `dXSB` ≠ region owner, classify as (A) or (B) using whether the occupant’s
   `ubPc.dXSB` (bio primary team) agrees with the label or the region.
4. Apply the minimal edit set: relabel (A) or relocate (B); never swap-pop, always ordered.
5. Re-run `towx_coherence` + full `validate_invariants` on the result **inside** the repair
   transaction; if not clean, **roll back** and surface a structured "could not auto-repair" error
   rather than committing a half-fix.

## Verification protocol (go/no-go — must pass before wiring in)

A spike test under `crates/tdb-edit/tests/` (or an `examples/` probe) against
`anchors/ps3/nhllegacy/databases/nhlng.db`:

1. **Reproduce** the exact corruption the fix removed — reuse the setup from
   `towx_coherence_flags_a_foreign_team_id_inside_a_block` (retag an interior slot to a foreign team;
   also construct the "stray far from block" shape) and fix CRCs.
2. **Confirm the source of truth survives**: assert `toWX` runs (or the chosen signal) still recover
   the correct block boundaries under the corruption.
3. **Run the candidate repair.** Assert:
   - `towx_coherence(&repaired)` is empty and the full audit is **write-clean**;
   - **roster occupancy is conserved** — the multiset of `(player_id, resolved_team)` memberships is
     unchanged for shape (A), and changed *only* for the deliberately-moved occupant for shape (B);
   - no unrelated team's block moved (diff the per-team record ranges);
   - CRCs valid; round-trips (`TdbFile::parse` clean).
4. **Idempotence**: repairing an already-clean anchor is a no-op (0 edits).
5. Only if all pass: promote to a Layer 3 op (`repair_roster_blocks`), expose via `studio-api`, and
   surface as an explicit, **previewable** "Repair roster structure" action (show the planned edits;
   never silent — reordering a roster is high-stakes).

## Risks / rollback

- Wrong source-of-truth → a "repair" that shuffles players. Mitigation: the in-transaction re-audit +
  roll-back gate, and the mandatory preview before commit.
- Dense corruption (dominant-`dXSB` heuristic fails) → detect low confidence and refuse rather than
  guess. Mitigation: require the primary source (`toWX`) to fully reconstruct the tiling; bail if it
  can't.

## Out of scope for this spike

Capacity normalization / block growth (Spike S3) — if a true team's block is full, shape (B) can't
place the occupant. Until block-grow exists, shape (B) on a full destination surfaces
`TeamRosterFull` and defers to a manual trade (same as `move_player` today).
