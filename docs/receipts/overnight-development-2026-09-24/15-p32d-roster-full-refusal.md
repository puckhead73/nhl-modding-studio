# Receipt 15 (P3.2-D) — a full roster block refuses a move or added membership in words, on every anchor

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p32d-roster-full-refusal`. Clock: 07:20 → 07:34 (MDT, 25 Sep 2026).

## 1. Plain-language summary

The Phase 3 ledger's oldest open gap was the case the operation matrix could not run on 16 anchors:
moving or adding a player into a team whose roster block has no free slot. The new sweep does it on
all 27 anchors — and every anchor's first other team refuses (the gen7 mains are packed; the PS4
mains keep their slack in the table's capacity, not in vacant team slots). The refusal used to read
`team 0 roster block is full (no free slot to relocate into)`: a raw team id, engine wording. It
now reads, for example, "Anaheim Ducks has no free roster slot, so the move was not applied: on this
title a team's roster block holds a fixed number of slots and every one is occupied. Release or move
a player off Anaheim Ducks first" — resolved at the domain layer, which has the name data. On PS4
NHL 16–21 without an attached dictionary the team name is a dictionary key Studio cannot resolve, so
the message leads with the abbreviation and says so ("ANA (team #0; name not in this project's
dictionary) …"). A refused move leaves the player where he was and no undo entry. The engine gained
one error variant, `Refused(String)`, displayed verbatim; the NHL 07 slot-key refusal of receipt 14
uses it too, so its "entity model error:" prefix is gone.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-edit/src/lib.rs` | `EditError::Refused(String)` — a plain-language refusal, displayed without prefix |
| `crates/studio-domain/src/session.rs` | `name_full_roster` maps `TeamRosterFull { team }` → `Refused` with the team's name / abbreviation / id fallback; applied on `move_player`, `sign_from_fa`, `add_membership`; `require_slot_key` now emits `Refused` |
| `crates/studio-api/tests/roster_full_refusal.rs` (**new**) | the 27-anchor sweep: refusal text starts with the team, nothing moves, no undo entry, add-membership refuses the same block |

No DTO, binding or frontend change (the UI shows the error string as before).

## 3. Evidence

- [`p32d-roster-full-red.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32d-roster-full-red.log)
  — before: "team 0 roster block is full (no free slot to relocate into)".
- [`p32d-roster-full-green.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32d-roster-full-green.log)
  — after: 27 rows "refused in words at Anaheim Ducks™ / ® …" (gen7, PS4 15) and "… at ANA …" (PS4
  16–21, abbreviation fallback); 27 / 27 refused at the first other team (0 teams with a free slot).

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-api --test roster_full_refusal` (27 anchors) | **1 passed** (47 s) |
| `cargo test -p tdb-edit` | 41 passed (6 targets) |
| `cargo test -p studio-domain --lib -- --skip zz_diag` | **222 passed, 0 failed, 2 ignored** |
| `cargo test -p studio-domain --test operation_matrix --test import_lower_sweep --test workspace --test session --test capacity_boundary_insert` | 27 passed (operation matrix 1 passed, no CSV drift, 384 s; import sweep 1; workspace 11; session 13; boundary insert 1) |
| `rustfmt` on the new test | formatted |

The operation-matrix CSV was **not** regenerated; the run guards drift only (the refused-clean rows
stay refused-clean — the variant changed, not the outcome).

## 5. Limitations

- PS4 16–21 team names need the project's attached dictionary; the fallback names the
  abbreviation and says why. The team list itself shows the same empty name without a dictionary —
  a Phase 1.3 surface, not changed here.
- The sweep tries the first 40 other teams and stops at the first refusal; on every anchor that is
  the first team, so "a team with a free slot takes him" was never exercised here (the operation
  matrix covers the accepted move on its 11 fixtured anchors).

## 6. Cleanup

Scratchpad `patch_roster_full.py`, `patch_refused.py` deleted by literal path; no temp roots (the
test writes nothing); no processes.

## 7. Next action

Per the ledger: **the string-width pre-check in the identity editor** (expose the inline string's
byte width on the field view so the name input is capped before the write), then the
contract-dollar granularity question for NHL 08–10 / PS4 18–21 — unless the morning review returns
a defect.
