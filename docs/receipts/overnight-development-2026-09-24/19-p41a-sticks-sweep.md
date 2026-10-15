# Receipt 19 (P4.1-A) — hockey sticks: model, curve, flex and tape edited, saved, reopened and undone on every anchor

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p41a-sticks`. Clock: 07:50 → 08:00 (MDT, 25 Sep 2026).

## 1. Plain-language summary

Phase 4 opens with 4.1 hockey sticks. On all 27 anchors the skater's equipment companion row is
found (NHL 07 has none — the tab says so) and its stick columns are recorded: the model id (5 bits
on NHL 09, 7 on NHL 10 → PS4 16, 8 on PS4 17 / 18, 9 on PS4 19 / 20, 10 on PS4 21, 8 with range
190 on NHL 08), curve and flex (0..3; 0..15 on NHL 08), and the tape columns — one 4-bit tape
colour through PS4 15, then PS4 16's blade / shaft tape styles, PS4 17's 24-bit RGB tape colours,
PS4 19's tape types. Model, curve, flex and a tape column are edited through the ordinary validated
write, re-read on the Equipment tab's field list, refused by range when out of range, kept through
save → reopen on a disposable copy, and undone — on all 25 anchors that carry the companion.

One thing the sweep makes visible for review: the stick **catalogue labels** ("Easton Stealth S17"
and 74 others) were anchored on NHL Legacy PS3 texlibs, and the enum registry binds vocabularies
by table / field id with no title scope, so the same labels appear on every title from NHL 08
(range 0..190) to PS4 21 (0..1023). Whether id 17 is the same stick on NHL 10 or PS4 19 is not
established; the labels off Legacy are **provisional** and the ledger names per-title scoping of
the catalogue as the next contract question. No code changed in this package.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-api/tests/player_sticks.rs` (**new**) | the 27-anchor sweep (columns recorded; stick / curve / flex / tape edits; range refusal; save / reopen; undo; NHL 07 recorded as no companion) |
| `docs/validation/phase-4-coverage-matrix-2026-09-22.md` | 4.1 rows appended |

## 3. Evidence

[`p41a-player-sticks-sweep.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p41a-player-sticks-sweep.log)
— 27 rows; equipment field counts 15 (NHL 09 / 10), 16 (11–Legacy, PS4 15), 17 (NHL 08), 29–43
(PS4 16–21); every edit row "saved, reopened, 4 undone".

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-api --test player_sticks` (27 anchors) | **1 passed** (110 s) |

No source change; no other suite affected.

## 5. Limitations and observations

- Stick catalogue labels are Legacy-anchored and unscoped by title — provisional everywhere else
  (coordinator: a per-title vocabulary scope in the enum registry, or per-title catalogues, is a
  contract decision; the picker's labels are the human-readable surface at stake).
- Handedness / orientation (roadmap 4.1) lives on the bio row (`GRkp`, swept in 3.1), not on the
  equipment companion; on-ice appearance and the picker's model preview are runtime / desktop cards.
- Goalie sticks (4.5) were not swept here.

## 6. Cleanup

No scratch scripts; `nhlms-sticks-sweep-<pid>` removed on drop; no processes.

## 7. Next action

**4.2 gloves and 4.3 helmets** in the same sweep pattern (the companion's glove / helmet / visor
columns and their catalogues), then the coordinator's decision on title-scoped equipment
vocabularies — unless the morning review returns a defect.

## 8. Correction (25 Sep 2026, receipt 21)

The coordinator decided review item 8e: equipment vocabularies are title-scoped. The stick catalogue
(and the glove / helmet / skate / visor / tape / tuck ones) now labels a project opened as NHL Legacy
only; every other title in this sweep shows the stored id as a number with the reason. The sweep's
edits and round trips are unchanged (`player_sticks` still passes on 27 anchors); the "provisional
labels off Legacy" observation in §1 / §5 is closed — there are no labels off Legacy.
