# Receipt 12 (P3.2-A) — the roster list's Overall column obeys the Attributes tab's era rule

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p32a-roster-overall-column`. Clock: 06:15 → 06:45 (MDT, 25 Sep 2026).

## 1. Plain-language summary

Phase 3.2 opens by closing a recorded defect rather than adding a surface. The Overall number in
the roster list (team page depth chart, lines page, contracts tab) was computed with EA's rating
regression on **every** title, although that regression is an NHL 14 / 15-era fit: the Attributes
tab already refuses to print it on the older `p_` / `g_` ratings vocabulary and shows the stored
value instead. So on NHL 13 the list showed a player at 62 while his own tab said 74 (stored), and on
NHL 07–12, which carry no Overall field and too few of the formula's inputs, the list printed
numbers such as 1 or 6. The list now follows the tab's rule exactly: the derived Overall on the
calibrated `c_*` era when enough inputs resolve, otherwise the stored Overall when the title has one,
otherwise nothing. Goalies were already reading the stored value and are unchanged. Proven red →
green on eight anchors; the NHL 14–21 values are unchanged.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/view.rs` | `CompanionReader.calibrated` (every rating on the `c_*` vocabulary); **new** `CompanionReader::skater_overall` = derived (calibrated + coverage) → stored `overall` → `None`; `player_summaries` uses it |
| `crates/studio-domain/tests/roster_overall_column.rs` (**new**) | per-skater equality of the list's Overall with the tab's `computed_overall` (NHL 15 PS3, NHL 17 PS4) or `stored_overall` (NHL 13 both consoles); no value on NHL 07 / 09 / 10 / 11 / 12 |

No DTO, binding or frontend change (the column already renders `—` for no value).

## 3. Evidence

- [`p32a-roster-overall-red.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32a-roster-overall-red.log)
  — before: NHL 07 list `Some(1)`, NHL 10 `Some(6)`, NHL 13 `Some(62)` vs stored `Some(74)`; the
  calibrated-era case already green.
- [`p32a-roster-overall-green.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32a-roster-overall-green.log)
  — after: `roster_overall_column` 4 passed; `view` 16; `overall_formula` 4 (the per-anchor
  calibration table unchanged); `ea_slug_index` 3; `formula_real` (real fixture) ignored.

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-domain --test roster_overall_column --test view --test overall_formula --test formula_real --test ea_slug_index` | 27 passed, 1 ignored |
| `cargo test -p studio-domain --lib -- --skip zz_diag` | **222 passed, 0 failed, 2 ignored** |
| `rustfmt --check` on the new test | clean |

Repo-wide fmt / clippy remain the recorded C0.2 baseline defects.

## 5. Limitations

- The list does not say *which* Overall it shows (derived or stored); the Attributes tab does. A
  small "(stored)" hint on pre-NHL 14 titles would be UI-only — not done here.
- Goalie Overall in the list stays the stored value on every era (the tab computes it on the `c_*`
  era); the two can differ by the tab's measured 0–4 points there. Unchanged behaviour, recorded.

## 6. Cleanup

Scratchpad script `patch_overall_column.py` deleted by literal path; no temp roots; no processes.

## 7. Next action

3.2 continues with **ratings, style and potential edits swept per anchor** (offset 50 / 36 / 0 by
family, edit → tab → validate → save / reopen → undo — the identity sweep pattern on `dSvy` / `mHuy`),
then the 3.2 ledger of the remaining operations (companion records, roster ops, historical import,
bulk edit) against the existing operation matrix — unless the morning review returns a defect.
