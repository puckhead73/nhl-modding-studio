# Correction checkpoint — review of R113–R118 (`docs/validation/research-20260926-r113-r118-review.md`)

Research lane, 26 September 2026, published 03:44 MDT (system clock at publication). Wording and routing corrections plus one developer
observation; no new measurement, no raw result changed. Claim `research-p13-checkpoint-r113-r118`.

## Plain-language summary

Three things are corrected. The rating comparison across releases reported typical (median) changes, and some sentences made them
sound as if every stored number moved by the same amount; they now give the medians and the spread, and name where the two confirmed
end points come from. The goalie "consistency" finding is routed to the developer with a corrected example — on NHL 08 Studio would show
50 to 52, not 36 to 38 — and without guessing what the three stored values mean. The schedule handoff's arithmetic for a free-form table
of games per pair is fixed: count each pair once. Separately, the developer noticed that two handoffs from 25 September still called the
archive sound-bank files compressed; they now carry the same correction as the earlier note.

## Corrections placed (dated; originals struck through or kept)

| Item | Where |
|---|---|
| 1. R118 distributions, not identical per-cell changes; endpoint source = Studio encoding contract `InGame` evidence ("read on a console by the earlier program", `encoding.rs` module note and rating-offset evidence list; phase-3 matrix receipt 20 row); status stays inferred | handoff 12 (title, header note, summary, §2 text); RECEIPT-2 (summary, row 12); queue C15 |
| 2. Goalie `consistency`: `NON_RATING_SUFFIXES` lacks it, `rating_offset` classifies 0..63 / 0..99 declarations as ratings, `view.rs` substitutes the contract offset → NHL 08 50–52, NHL 11–Legacy 36–38, PS4 0–2; routed for a focused classification investigation with regression across display, edit, import and derived calculations; no meanings, no clamp, no silent goalie-Overall change; `floppiness` unresolved per title | handoff 12 (§2b, §3); queue C15; RECEIPT-2 |
| 3. Matrix total = upper-triangle sum (half the off-diagonal sum); zero diagonal, non-negative integers, symmetry; stock anomalies ≠ generated validity; Austrian `round = 1` block measured, runtime phase semantics unverified; data + validator support design, not implementation | handoff 08 (§2 table, header note, summary); RECEIPT-2 summary |
| Developer observation (claim `dev-p48-archive-sound-banks`, 1 of 2): R98 §1 prose and R100 table still labelled `.abk` members RefPack / chunkzip | 2026-09-25 handoff 11 (R98) §1 closing sentence; handoff 13 (R100) two table cells |

The second p48 observation (R98 per-member record totals exceed R82's present-record totals: 2 619 / 2 772 / 4 734 / 5 160 vs 2 594 /
2 748 / 4 697 / 5 117) is taken as the next bounded package.

## Tooling note

`safe_patch.py` was rewritten with the Write tool after a shell edit left it unparseable (no document was written by the broken copy).
It now normalizes line endings: anchors match regardless of CRLF / LF, output keeps the target's majority ending (LF for new files), and
four files that earlier patches had left with mixed endings (handoffs 09 and 12, STATUS, the queue) were rewritten with uniform endings,
content unchanged.
