# Receipt 18 (P3.3-A) — where player text lives: biography strings, dictionary tokens and audio-name references, per anchor

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p33a-player-text`. Clock: 07:44 → 07:50 (MDT, 25 Sep 2026).

## 1. Plain-language summary

Phase 3.3 asks Studio to tell three kinds of player text apart — the biography strings written
straight into the player row, text that lives in a locale dictionary, and the numeric references
that pick a recorded name in the commentary banks — and never to assume every name is a dictionary
entry. Measured on all 27 anchors: first and last names, nicknames (PS4) and birth cities are
**inline biography strings**, populated for every player, valid UTF-8, never a `$` dictionary key —
with one measured exception: on PS4 NHL 17–21, eight template rows named "Joe NHL" store the birth
city as the token `$BURNABY`, which the title's own dictionary resolves to "Burnaby" (exact key on
NHL 17 / 18 / 20; NHL 19 / 21 have no dictionary anchor). The audio-name reference is a separate
numeric column on every title (`audioid`, 14 bits on NHL 07–13 and 16 bits from NHL 14; NHL 07
also carries `defaultaudioid`; NHL 21 adds `surnameid`, 16 bits) — the speech catalogue's name-call
chips already consume it (receipt 07). NHL 08–10 carry an empty `statename` string column beside
the birth-state enum. Studio's own labels (field names, positions, nationalities) come from
`defs/`, not from game dictionaries; team names on PS4 16–21 are dictionary keys and belong to
Phase 1.3's routes. No code changed: this package is the inventory the roadmap step asks for, with
the facts pinned.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/tests/player_text_columns.rs` (**new**) | per-anchor census of the bio string columns (width, filled, `$` tokens, non-ASCII, non-UTF-8) and the audio-reference columns; the eight `$BURNABY` rows named and their dictionary lookup shown |
| `docs/receipts/overnight-development-2026-09-24/LEDGER-PHASE3.md` | 3.3 section replaced with the measured inventory |
| `docs/validation/phase-3-coverage-matrix-2026-09-22.md` | 3.3 rows appended |

## 3. Evidence

[`p33a-player-text-columns.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p33a-player-text-columns.log)
— the 27-row table. Widths: names 160 B (NHL 07), 40 B (NHL 08–12), 25 B (NHL 13 → PS4 21); birth
city 160 / 40 / 30 (PS4 19–21) B; PS4 nickname 25 B, filled on 92–129 rows per title.

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-domain --test player_text_columns` | 2 passed |

## 5. Observations for research

- `$BURNABY` on eight "Joe NHL" template rows (PS4 17–21): does the console render the token or the
  dictionary text? Studio shows the raw token in the birthplace today (the card resolves team and
  arena keys through the attached dictionary, not birth cities) — a bounded consumer change if the
  template rows ever matter.
- NHL 08–10 `statename` (40 B) is empty on every row; the birth state is the enum column.

## 6. Cleanup

No scratch scripts; no temp roots; no processes.

## 7. Next action

Phase 3's developer-actionable rows are closed or research-owned (ledger). The next step in the
roadmap's order is **Phase 4.1 hockey sticks**: inventory the equipment fields per anchor family
(the skater / goalie equipment companions, stick model / tape / curve columns), then a bounded
sweep in the identity-sweep pattern (edit → tab → validate → save / reopen → undo) — unless the
morning review returns a defect.
