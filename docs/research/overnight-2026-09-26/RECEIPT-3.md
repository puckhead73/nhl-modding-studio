# Consolidated receipt 3 — research lane, 26 September 2026 (03:40 → 03:57 MDT, system clock)

Continuation under [research-20260926-r113-r118-review.md](../../validation/research-20260926-r113-r118-review.md). Receipts 1 and 2 cover
01:06–03:13. Everything below is researcher-verified and pending review; nothing is game-verified; no writer exists; runtime, visual and
listening acceptance remain the user's.

## Plain-language results

The three review corrections are published: the rating comparison now reports typical changes and their spread and names where the
confirmed end points came from; the goalie "consistency" issue is handed to the developer with the corrected NHL 08 example and no guessed
meanings; and the schedule arithmetic for a free-form table counts each pair once. Two developer questions from last night's sound-bank work
are answered — two older reports still called the bank files compressed (now corrected), and the two record totals differ only because one
counted empty slots. The main new result is from following players across releases: weight, height, handedness and position mean the same on
every title (positions agree with the line slots the game uses), but **the numbers used for countries change between the older titles** —
the national teams' own rosters prove it — so Studio's single list of country names is wrong on NHL 08–10 and for Japan on NHL 11 / 12, and
the same applies to the country part of the birth-state list. Continuity also flags a possible shift in skater-style numbers at NHL 10 / 11 and
unexplained bulk changes in eye colour, both still unproven.

> **Correction 2026-09-27 04:48 ([00d](00d-checkpoint-r119-r121-review.md)):** "Studio's single list … is wrong on NHL 08–10" is withdrawn for NHL 08 / 09 — Studio's player page uses its NHL 07 country table there, which is right for those teams. The wrong labels were NHL 10's (Czech → "Croatia", German → "France", Swiss → "Slovakia") and NHL 11 / 12's missing Japan; the developer reports both fixed (receipt 28). The code shifts themselves stand.

## Packages

| # | Handoff | Result | Evidence class |
|---|---|---|---|
| 0 | [00c-checkpoint-r113-r118-review.md](00c-checkpoint-r113-r118-review.md) | review items 1–3 (handoffs 08 / 12, RECEIPT-2, queue C15); developer p48 observation 1 (R98 / R100 `.abk` labels); `safe_patch.py` rewritten (line-ending aware; four mixed files repaired) | presentation |
| 13 | [13-abk-record-total-reconciliation.md](13-abk-record-total-reconciliation.md) | R98 slot counts = R82 present + absent on every tree (25 / 24 / 37 / 43; X360 09 copies double-counted); note on R82 handoff 01 | derived (exact) |
| 14 | [14-player-identity-audit.md](14-player-identity-audit.md) | 19 sources incl. real NHL 20 and patches; weight / height / handedness / position stable (position confirmed by line slots); **nationality codes renumbered at NHL 09 → 10, 10 → 11, 12 → 13** (national-team rosters, title-owned); 8-bit field on PS4 20 / 21; placeholder rows with height 0; birth-state country codes follow nationality (§7); skater style / eye colour continuity flags (§8); discrepancy rows D1–D7 | measured (title-owned for nationality and position) + continuity |

## Developer handoff (queue [`overnight-2026-09-25/DEVELOPER-QUEUE.md`](../overnight-2026-09-25/DEVELOPER-QUEUE.md), rows C1–C16)

- **C16 (new):** D1 per-title scoping of `nationality.toml` and `birth-state.toml` codes ≥ 66 with named regressions (e.g. an NHL 10 Czech
  national-team player renders "Czech Republic", not "Croatia"); D2 width-aware nationality edit range (8 b on real NHL 20 / NHL 21); D3 height
  raw 0 as "no height"; D6 / D7 skater style and eye colour labels unverified per title (continuity only).
  **Correction 2026-09-27 04:48 ([00d](00d-checkpoint-r119-r121-review.md)):** D1 is NHL 10 (wrong) and NHL 11 / 12 (Japan missing) only — NHL 08 / 09 field views already used the NHL 07 table; delivered per receipt 28. D2 → field-width fact + consumer audit; D3 → zero / placeholder presentation investigation (no `raw 0 = missing`, no row hiding).
- **C15 (reworded):** goalie `consistency` classification investigation with regression across display, edit, import and derived calculations;
  NHL 08 would show 50–52, NHL 11–Legacy 36–38, PS4 0–2; no meanings, clamp or silent Overall change.
  **Correction 2026-09-27 04:48 ([00d](00d-checkpoint-r119-r121-review.md)):** classification corrected by developer receipt 27 and passed the coordinator's offline review; numeric meanings and the goalie Overall calibration remain open. The 50–52 / 36–38 figures describe pre-receipt-27 behaviour.
- Suggested order otherwise unchanged (handoff 11 §2); receipts 24–26 (archive banks, loop previews, nameplate association) consumed.

## Cards (user; none blocks research)

Carried: L-LOGO-1, K-NAME-1, T-CHANT-1 rev. 3, A-SPX-1 rev. 3, A-AST-1, A-LOOP-1, T-ARTID-1, H-HORN-1 rev. 2, P1-A / P1B-3. Optional
(handoff 14 §8): one known player per skater style per title on NHL 07–11, viewed in the game, would settle D6.

## Claims

Research p13–p16 `ready` (p1–p12 unchanged). Developer claims p48–p50 complete; both p48 observations answered.

## Residuals

NHL 07 nationality labels (no national rosters); codes outside the 21 national teams; the 140 duplicate name + birth-date keys on PS4 NHL 18;
tournament-stat `playerid` after NHL 08 (inferred carried-forward table); D6 / D7 labels.

## Reproduction

```
python local/runs/overnight-research-2026-09-26/probes/r120_identity_audit.py <out.json>
python local/runs/overnight-research-2026-09-26/probes/r120b_nationality_label_table.py <r120.json> defs/enums/nationality.toml <out.json>
python local/runs/overnight-research-2026-09-26/probes/r120c_position_line_slots.py <out.json>
python local/runs/overnight-research-2026-09-26/probes/r121_unscoped_enum_continuity.py <out.json>
```

## Cleanup

Scratchpad empty; no background process; no `*.tmp-*` sibling. Kept: probes r120 / r120b / r120c / r121 and the helpers; JSON under `data/`.
Nothing written to anchors, defs, bindings, application or developer paths; fixtures only read. Nothing staged, committed or pushed. Times
from the system clock. One process slip, corrected: a probe run piped into `head` stopped before writing its JSON and a derived table was
briefly computed from the stale file; the probe was rerun in full before anything was published.

## One exact next task

Look for title-owned label evidence for the skater-style codes (D6) and for the NHL 07 nationality codes: search each title's own data —
localisation dictionaries and create-player / front-end option lists in code order — statically; if none exists, record D6 / D7 as needing the
optional in-game card and move on to Players 3.4 (portrait / head reassignment inputs) in roadmap order.
