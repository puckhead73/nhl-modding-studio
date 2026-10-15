# Receipt 14 (P3.2-C) — the historical import proven on every anchor; NHL 07's refusal in words; the Phase 3 ledger

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p32c-import-sweep`. Clock: 07:03 → 07:20 (MDT, 25 Sep 2026).

## 1. Plain-language summary

The roadmap's 3.2 acceptance says imported and created players must retain coherent records on
every target. The import round trip had been proven on two anchors (NHL 14, NHL 10); it is now
proven on all 27: one synthetic person is lowered through the historical-import path on each
anchor, so every per-title encoding this campaign pinned (weight base 150 / 120, birth day index,
rating scale 50 / 36 / 0, height + 54) flows through the import writer and reads back exactly; the
new player sits on the target roster, the whole-database audit is no worse than the anchor's, and
the player is found again after saving to a disposable copy and reopening. NHL 07 cannot take a new
roster slot (its roster table has no slot-key column); it refused before, but with the engine's raw
field id (`unknown field "toWX" in table "eGlu"`). Create, clone and import now refuse **before**
the transaction with the reason in words.

The package also delivers the **Phase 3 ledger** ([LEDGER-PHASE3.md](LEDGER-PHASE3.md)): every 3.1 /
3.2 operation per family with its state and the concrete missing work, built from the operation
matrix (read only), the coverage matrix and the code. Its oldest actionable offline gap is the
packed-roster refusal for move / add-membership, untested by the operation matrix on 16 anchors.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/session.rs` | `require_slot_key(what)` — refuses create / clone / import-create in words when the roster-lines table lacks the slot-key column; called from `create_player`, `clone_player`, `apply_import` (Create ops only) |
| `crates/studio-domain/tests/import_lower_sweep.rs` (**new**) | 27-anchor import round trip with save / reopen; NHL 07 refusal text asserted (no raw id) |
| `docs/receipts/overnight-development-2026-09-24/LEDGER-PHASE3.md` (**new**) | the Phase 3 ledger |
| `docs/validation/phase-3-coverage-matrix-2026-09-22.md` | 3.2 import row appended |

No DTO, binding or frontend change.

## 3. Evidence

- [`p32c-import-sweep.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32c-import-sweep.log)
  — 27 rows: "created, 6'1" / 195 lb / 1990-05-15 / #91 / rating 88 read back (offset Some(50) |
  Some(36) | None), audit N → N, saved, reopened" on 25 anchors; NHL 07 both consoles "refused
  clean — … no slot-key column … not supported here". Audit error counts unchanged on every anchor
  (NHL 09 / 10 carry 7 pre-existing errors, PS4 NHL 18 one).

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-domain --test import_lower_sweep --test nhl07_encodings --test session --test import_lower` | 22 passed |
| `cargo test -p studio-domain --test operation_matrix --test workspace --test capacity --test capacity_boundary_insert --test mutation_matrix` | **18 passed across the five targets (operation matrix 1 passed, no CSV drift, 334 s)** |
| `cargo test -p studio-api --test roster_builder` | 1 passed |
| `rustfmt --check` on the new test | clean |

The operation-matrix CSV was **not** regenerated (`QA_WRITE` unset); the run guards drift only.

## 5. Limitations

- ~~The refusal is typed as an entity-model error, so the API prefixes it "entity model error:"~~ —
  closed by receipt 15 (`EditError::Refused`, displayed verbatim).
- The synthetic sweep proves the encodings and the cluster, not the corpus join per title
  (`import_corpus_e2e` remains NHL 14 only).

## 6. Cleanup

Scratchpad `patch_slot_key.py` deleted by literal path; `nhlms-import-sweep-<pid>` removed on drop.

## 7. Next action

Per the ledger: **the packed-roster refusal for move / add-membership asserted per family** (a
bounded case filling a team to its ceiling), then the string-width pre-check, then the
contract-dollar granularity question for NHL 08–10 / PS4 18–21 (research unless a known contract
anchors it) — unless the morning review returns a defect.
