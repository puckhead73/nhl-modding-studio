# Consolidated receipt 2 — research lane, 26 September 2026 (02:32 → 03:13 MDT, system clock)

Continuation under [research-20260926-r107-r112-review.md](../../validation/research-20260926-r107-r112-review.md). Receipt 1
([RECEIPT.md](RECEIPT.md)) covers 01:06–01:26 and carries a dated pointer here. Everything below is researcher-verified and pending
review; nothing is game-verified; no writer exists; runtime and visual acceptance remain the user's.

## Plain-language results

The three over-strong statements the review found are corrected where they appeared, with the originals struck through. The
schedule work then went further: every table of every title's database was searched for a field that records the European league groups
(none that refers to teams does), and the groups are now exported per title with a checker that recomputes every pairing and catches
deliberately broken copies. The groups change from title to title, so none can be carried across. A bounded handoff for a group schedule
generator names the exact numbers that must add up, when to refuse, and three new facts: the PS4 Austrian table ends with a block of
group games tagged as a later round (its in-game meaning is not verified), the European tables allow at most 16 clubs, and some shipped tables are out of date order or book a club twice
on one day. The tournament tables were re-read: the World Cup positions line up with the teams and their groups, the Champions League
has a database candidate for its 32 participants (a jersey flag), and the Spengler Cup's extra numbers read as "winner or loser of the
opening game"; none of it allows editing. The developer's question about birth-date ranges is answered and agrees with Studio. The
developer's ledger is lined up with the research. Finally, following the same players across releases supports, as a consistency check on
median changes, the rating offsets Studio uses on the older titles — and shows that the goalie "consistency" value holds only 0, 1 or 2 in
the shipped files although Studio treats it as a rating (wording corrected 2026-09-26 03:42, [00c](00c-checkpoint-r113-r118-review.md)).

## Packages

| # | Handoff | Result | Evidence class |
|---|---|---|---|
| 0 | [00b-checkpoint-r107-r112-review.md](00b-checkpoint-r107-r112-review.md) | findings 1–3 corrected (dated notes in handoffs 03 / 05 / 06, R13 note, queue C11 / C12, RECEIPT, claims p3 / p5 / p6); validated-sibling writer `safe_patch.py` adopted | presentation |
| 7 | [07-group-partitions-field-check-and-contract.md](07-group-partitions-field-check-and-contract.md) | 19 distinct sources; 0 team-keyed columns encode a partition (21 row-number coincidences listed; uninspected locations listed); 89 contracts, 88 conform (NHL 13 Czech 3 / 1 pair), 89 / 89 controls detected; partitions differ per title; NHL 15 Finnish = 2 × 7 with one displaced game; R111b "same slot structure" corrected | measured |
| 8 | [08-schedule-generator-handoff.md](08-schedule-generator-handoff.md) | stock-shaped vs user-supplied quotas; formula; capacity; **Austrian two-phase `round`**; 4-bit slots; date-order / double-booking exceptions; refusals; oracle | proposal (not an implementation contract) |
| 9 | [09-tournament-schedule-slots.md](09-tournament-schedule-slots.md) | slot classes; WC consistent join; CHL `ischampionsleague` = 32 clubs (candidate); Spengler R-WL reading; J1 / J2 conflict on NHL 21; R36 corrected; `fcc_tournaments` = Ultimate Team name catalogue | measured / interpretations labelled |
| 10 | [10-date-field-declared-ranges.md](10-date-field-declared-ranges.md) | developer observation confirmed; stored 1-based NHL 07–10, 0-based NHL 11+; agrees with the contract; encodable ≠ valid | measured |
| 11 | [11-ledger-reconciliation.md](11-ledger-reconciliation.md) | ledger rows vs ready research; two withdrawn citations; save-side schedule copy already researched (P1-B) | coordination |
| 12 | [12-rating-offset-continuity.md](12-rating-offset-continuity.md) | skaters and goalies: median raw difference 0 (07→10), median +14 on every shared field (10→11; quartiles 11 / 14 / 14), 0 / +1 (11→14); controls median +36 per field; goalie `consistency` stores 0 / 1 / 2 but is classified as a rating by Studio (NHL 08 would show 50–52, 11–Legacy 36–38, PS4 0–2); `floppiness` changes scale | inferred (continuity) + measured value ranges |

## Corrections placed (dated; nothing deleted)

Handoffs 03, 05 (twice: F1 / F2 wording and membership), 06; 2026-09-20 handoff 06 (R13 note); 2026-09-21 handoff 08 (R36); RECEIPT.md;
queue rows C11 / C12; claims p3 / p5 / p6. Memory note on birth-date indexing corrected (NHL 07–09 also 1-based).

## Current developer queue

[`overnight-2026-09-25/DEVELOPER-QUEUE.md`](../overnight-2026-09-25/DEVELOPER-QUEUE.md): rows C1–C15 (C13 stock partitions +
generator handoff, C14 tournament browse labels, C15 rating-offset witness + goalie `consistency` observation added today; C11 / C12
reworded). Suggested developer order unchanged at the top (handoff 11 §2): archive-backed banks (C10), chant wording (C5), ledger
wording refresh.

## Cards (user; none blocks research)

Carried: L-LOGO-1, K-NAME-1, T-CHANT-1 rev. 3, A-SPX-1 rev. 3, A-AST-1, A-LOOP-1, T-ARTID-1, H-HORN-1 rev. 2, P1-A / P1B-3 (schedule
edits reaching a mode). Optional discriminating tests named in handoff 09 §6 (CHL jersey flag; Spengler group ids on NHL 21), on a
disposable copy.

## Claims

`local/coordination/overnight-2026-09-26/claims/`: research p1–p7, p9–p12 all `ready` (P8 was delivered under p7). No developer claim
active; developer receipt 23 consumed (its observation answered in handoff 10).

## Cleanup

Scratchpad empty; no background process; no `*.tmp-*` sibling left. Kept: probes r113 / r113b / r114 / r115 / r116 / r118 and the helpers
`safe_patch.py` / `claim_note.py` under `local/runs/overnight-research-2026-09-26/probes/`; JSON outputs under `data/`. Nothing written
to anchors, defs, bindings, application or developer paths; the private fixture root was only read. Nothing staged, committed or pushed.
Every time stamp in today's continuation documents was taken from the system clock at writing.

## One exact next task

Players 3.1 identity encodings, cross-title: extend the R118 same-player matching to the identity fields — weight, height, handedness,
position, `intlcountry` / birthplace — over adjacent titles (NHL 07 → PS4 21), to give an independent witness for the weight base change
(150 → 120) and to detect any per-title code-table change in nationality or position that the encoding contract does not yet cover;
report measured shifts, outliers and the fields where continuity cannot be assumed.
