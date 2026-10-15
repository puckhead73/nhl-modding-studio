# Receipt 17 (P3.2-F) — contract dollars read in dollars on NHL 08–10; the PS4 18–21 unit confirmed

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p32f-contract-units`. Clock: 07:38 → 07:44 (MDT, 25 Sep 2026).

## 1. Plain-language summary

The player card's Cap Hit showed a bare stored number on NHL 08, 09 and 10 ("salary scale unconfirmed
for this game"), and on PS4 NHL 18–21 it assumed the ×$5 000 unit of the earlier titles because the
field had grown to 14 bits. Both are now measured against contracts whose cap hits are public
knowledge, read straight off the anchors: Sidney Crosby's stored value is 348 on NHL 09 and NHL 10 —
exactly $8 700 000 at ×$25 000, the same unit NHL 07 uses (his NHL 07 value, 34, is his $850 000
entry-level salary); Connor McDavid's is 2 500 on PS4 NHL 19 / 20 / 21 — exactly $12 500 000 at
×$5 000; Crosby reads $8 700 000 on every title from NHL 09 to PS4 NHL 21. The 10-bit titles now
show dollars, and the 14-bit unit is pinned rather than assumed. Twenty-eight known cap hits are
asserted through the player card.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/view.rs` | `cap_hit_scale`: 9- **and 10-bit** `contractdollars` → ×$25 000; the ≥ 13-bit rule now documented as measured for 14 bits |
| `crates/studio-domain/tests/contract_scale_probe.rs` (**new**) | the raw-value table per anchor (Crosby / Ovechkin / McDavid × candidate units) and 28 pinned cap hits through `player_view` |

No DTO, binding or frontend change (the card already rendered dollars where the scale was known;
the "unconfirmed" note now no longer appears on NHL 08–10).

## 3. Evidence

- [`p32f-contract-scale-measurement.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32f-contract-scale-measurement.log)
  — per anchor: bits, raw, raw × 5 000 / 10 000 / 25 000, known cap hit. NHL 09 / 10 Crosby 348 →
  $8 700 000 at ×25 000; NHL 11–PS4 21 Crosby 1 740 → $8 700 000 at ×5 000; PS4 19–21 McDavid 2 500 →
  $12 500 000 at ×5 000; NHL 07 Crosby 34 → $850 000; NHL 08 Crosby 280 → $7 000 000 (EA's own
  pre-extension figure — not a public cap hit, so not asserted); Ovechkin 1 907 → $9 535 000 (EA
  rounds his $9 538 462 down to the unit).
- [`p32f-contract-scale-green.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32f-contract-scale-green.log)
  — `contract_scale_probe` 2 passed (28 cap hits); `view` 8; `nhl07_encodings` 8; `field_profile` 16.

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-domain --test contract_scale_probe --test view --test nhl07_encodings --test field_profile` | 34 passed |
| `cargo test -p studio-api --test player_containment` | 2 passed |
| `cargo test -p studio-domain --lib -- --skip zz_diag` | **222 passed, 0 failed, 2 ignored** |

## 5. Observations for research

- NHL 08 stores Crosby at 280 × $25 000 = $7 000 000 — a figure EA authored before his $8.7M
  extension took effect; the ×$25 000 unit is inferred from NHL 07 / 09 / 10 and the field's width,
  not from a public NHL 08 number.
- Ovechkin's $9 538 462 is stored as 1 907 (×$5 000 = $9 535 000) on PS4 17–20 and 1 908 on PS4 21
  ($9 540 000): EA rounds to the unit, down then up.

## 6. Cleanup

No scratch scripts (in-line edits); no temp roots; no processes.

## 7. Next action

Per the ledger, the remaining Phase 3.2 items are research questions (NHL 07–10 rating scale in game;
`yeardrafted` base on NHL 07 / 10; PS4 animation vocabularies; NHL 10–13 `p_style` vocabulary) or
scope questions (player bulk edit). The next developer step is **Phase 3.3 localization inventory**
(which player-related strings are dictionary-resolved per title) following the roadmap's order —
unless the morning review returns a defect.

## 8. Correction (25 Sep 2026, receipt 20)

- The "28 public cap hits" were three players observed across anchors, not 28 independent
  contracts. Receipt 20's candidate table matches 6–25 distinct public contracts per family.
- **NHL 08's unit is now measured** (Iginla $7.0M, Lidstrom $7.6M, Luongo $6.75M, Pronger $6.25M,
  Richards $7.8M, Thornton $6.667M, Kovalchuk $6.389M all read at ×$25 000); §5's explanation of
  Crosby's stored 280 as "EA's pre-extension figure" is **withdrawn** — it is EA's own figure and
  matches neither his 2007-08 cap hit nor his extension; it is recorded as a non-match.
- The unit is bound to the schema family (`studio_domain::encoding`), not to the field's bit width.
