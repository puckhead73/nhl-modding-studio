# Handoff 01 (28 Sep) — R123 schedule reference, revision 2: odd-host rules validated by value, stored keys separate from positions, generated-vs-kept collisions

Research lane, 28 September 2026, published 04:46 MDT (system clock). Prompt package 2. Claim `research-q2-r123-reference-revision`.
Research-only reference validator: **not** Studio code, **not** the developer's generator and not the game's scheduling algorithm. The
developer owns production validation and generation (dispatch after receipt 45 §3). Nothing here edits or duplicates that work.

Machine-readable: [`data/r123r2-schedule-reference.json`](data/r123r2-schedule-reference.json). Probe:
`local/runs/overnight-research-2026-09-28/probes/r123r2_schedule_reference.py` (SHA-256 `e63339f3…`). Revision 1
(`overnight-research-2026-09-26/probes/r123_schedule_generation_contract.py`, `cd46f8fb…`) is **unchanged**. It remains the recorded
result the coordinator replayed on 28 Sep: 27 / 27 vectors, 144 tables, JSON-equal.

## Plain-language summary

The coordinator found that research's schedule checker accepted a nonsense "who hosts the extra game" rule. Revision 2 now refuses
anything but the three named rules or a complete per-pair list, and it no longer skips the host check when the rule is unknown. It also
treats a row's stored number separately from where the row sits. Studio must never renumber a kept row, and the checker now says so
directly. It also refuses a new game booked on the same date as a kept game of one of its clubs. Along the way, research found that
revision 1 never read the tables' stored number: it assumed number = position. That has now been measured. On all 144 shipped league
tables the stored number does equal the position, so the earlier statement happens to be true, but it had not been checked. For the
Austrian two-stage table, only the stored facts are recorded. What the second stage means to the game remains a question for an in-game
check.

## 1. What changed from revision 1

| Item | Revision 1 | Revision 2 |
|---|---|---|
| Odd-host rule (Q11) | presence only; the proposal check skipped the host test for an unknown rule | **by value**: `lower_slot`, `higher_slot`, `balance`, or `{"a,b": host}`. Q11 no rule with odd counts; **Q11r** unsupported text / type (incl. `'Lower_Slot'`, `' lower_slot'`, `''`, `1`, `true`, a list), or `balance` on a phased request; **Q11m** malformed / non-canonical key (`"0-2"`, `"2,0"`); **Q11h** host not one of the pair's two slots, or not an integer (`"2"`, `true`); **Q11k** an odd pair without a key; **Q11x** a key for an even pair or a non-participant |
| Unresolvable host in a proposal | silently not checked | **R5 failure** ("no resolvable host") |
| `balance` | accepted as arbitrary non-null text | a **transcription of the developer's documented algorithm** (`schedule_edit.rs` `OddHome::Balance`, file SHA-256 `2a72ed8d…`, read 28 Sep): home totals start at ⌊count / 2⌋ per pair; pairs i < j in participant order; host = the club with the lower (home games so far, slot) tuple; the host's total grows by one. Checking against it shows that a proposal follows that algorithm. It does **not** establish a game rule, and it assumes the developer's participant order equals the request's. |
| Row identity | `index == position` (R3) | row = **original position** + **stored key** (`index`). The request supplies `stored_keys` (the source's key per position) and kept rows carry `pos`. **R2** kept rows keep every stored value at the same position. **R3 (revised)** every row keeps the source's key of its position; a key is never rewritten to force key = position. **Q13** request-internal inconsistency (kept position out of range / duplicated / key ≠ source key). |
| Collisions | R9 generated vs generated | R9 unchanged; **R11** generated game on a date when one of its clubs plays a **kept** game (failure; stock mode warning); **R9-retained** kept-only double bookings = warning, never a failure |
| Date packing around kept rows | — | **not checked**. The developer's conservative packer (receipt 41) is an implementation choice, not an engine rule. |
| Duplicate JSON keys in a host map | — | invisible to any dict parser. A production reader must reject them at parse time. |

## 2. Vectors (executed 28 Sep)

- **Unchanged population:** the 27 revision-1 vectors run through revision 2 give the same expected codes (27 / 27).
- **New:** 29 / 29 revision-2 vectors give their expected codes and warnings.
  - **C-1:** the coordinator's case (2 clubs, 1 game, `odd_home='invalid-rule'`) → request **Q11r**.
  - **C-2:** the same proposal checked on its own → **R5** (no silent skip).
  - Six Q11r rule values, as listed in §1.
  - Map cases: Q11k, Q11x (even pair; non-participant), Q11h (outside the pair; text; bool), Q11m (+ Q11k).
  - `balance`: V5 valid, F-R5 flipped host, Q11r on phases.
  - V6: a rule with no odd pairs is harmless.
- **Discriminating non-positional case (V7):**
  - 8 rows with stored keys `[3, 0, 1, 2, 7, 4, 5, 6]` and a kept row at position 5 with key 4 (club 0 v a left-out club).
  - Keys kept → valid.
  - Keys rewritten to positions → **R3**.
  - Kept row moved one position → **R2 + R3** (+ R4, R6 from the displaced game).
  - Kept key ≠ source key → **Q13**.
  - A generated game of club 0 on the kept game's date → **R11**.
  - V8: a second kept row double-booking club 0 → warning **R9-retained** only.

## 3. Stock replay, revision 2 (same R113 inputs, SHA-256 `9579937f…`; 144 league tables, 17 distinct sources incl. genuine NHL 20 `1e200478…`)

| Measure | Result |
|---|---|
| Request / proposal failures | 0 (as in revision 1) |
| Stored `index` field present | 144 / 144 |
| Stored key = position | **144 / 144**. First measurement; revision 1 used the position as the key, so its R3 result was unmeasured. |
| Duplicate keys | 0 |
| Kept rows under the default participant rule (> 2 games) | **0 on every table**. Answers developer observation 41 Q5 for the league tables in scope: a kept row arises only when the user deselects clubs. The NHL / AHL / tournament tables are outside R113 and are not covered. |
| Relaxed checks, reported as warnings | R5 uneven splits on 24 tables, R9 double bookings on 4, date order on 4; **R11 and R9-retained on 0** (no kept rows) |
| Pair quotas and odd hosts | **derived from the same rows being checked** |

What this replay proves: reproducibility and internal consistency of the reference on shipped data, and the measured key = position fact.
It does **not** prove that strict generated-output invariants hold on stock data, which break R5 / R9 / order as listed. It does not prove
runtime semantics or a universal league format (corrects the "0 failures" shorthand; checkpoint 00 K6).

## 4. Austrian phased table — static facts only (phased generation stays withheld)

| Source (SHA-256) | Rows | Phase 0 (`round = 0`) | Phase 1 (`round = 1`) | Phase 1 pairs | Games per club in phase 1 |
|---|---|---|---|---|---|
| PS4 NHL 18 (`65eaa6e8…`) | 324 | positions 0–263 = 12 clubs, every pair 4 times; last date Jan 28 | positions **264–323**, contiguous; first date Feb 2 | 30 pairs × 2, **all inside the R113 groups** {ranks 0–5}, {6–11} | 10 each |
| PS4 NHL 19 base (`265e4a68…`) | 324 | 0–263; last Feb 3 | 264–323; first Feb 23 | 30 × 2 within | 10 each |
| PS4 NHL 19 patch 56 (`61332e66…`) | 324 | 0–263; last Feb 3 | 264–323; first **Feb 12** (differs from the base) | 30 × 2 within | 10 each |
| PS4 NHL 20 genuine CUSA15439 (`1e200478…`) | 270 | 0–219 = 11 clubs × 4; last Jan 26 | 220–269; first Feb 12 | 25 × 2 within {0–4}, {5–10} | 8 (ranks 0–4), 10 (ranks 5–10) |
| PS4 NHL 21 (`90ee1ec4…`) | 270 | as NHL 20 | as NHL 20 | as NHL 20 | as NHL 20 |

Dates are the stored 0-based month and day, shown as calendar dates.

**Outstanding semantics (runtime, not static):**
- whether the game plays the stored phase-1 opponents, or **re-forms the two groups from phase-0 standings** at run time (a placement /
  qualification round, as the real-world league does);
- whether `round = 1` changes standings, points or playoff qualification;
- whether the phase-1 group of a club follows its rank or its record.

No team-keyed column encodes the groups (R113 §2, inspected list). No stock continuity argument settles this. The smallest discriminator
is card P1-A: on a copied save, play or simulate through phase 0 with standings unlike rank order, then read the phase-1 opponents the
game schedules.

Consequence for the developer: keep the current refusal for tagged rows. A single-phase generation may keep the phase-1 rows as kept rows
only if the user deselects nothing and the phase-1 rows are preserved verbatim. Research gives no rule for regenerating them.

## 5. For the developer (production, not this reference)

1. Reject unknown odd-host rule values and incomplete / foreign / malformed per-pair maps (Q11r / Q11m / Q11h / Q11k / Q11x). Reject
   duplicate keys at parse time.
2. If `balance` is offered, state its algorithm and report the chosen hosts (the developer's preview already names them). Revision 2
   checks proposals against the transcription above. A divergence between Studio and the transcription is a finding for either side,
   not proof of a game rule.
3. Keep every stored key. Model row position and key separately even though every shipped table has key = position. The synthetic V7 case
   is the regression shape.
4. Treat R11 (generated vs kept) as a refusal. Report R9-retained as pre-existing. This matches receipt 41's behaviour as reported.
5. Correct `place_generated`'s "every stock table is date-sorted" comment. Revision 2 again finds 4 tables out of date order (named in
   the data: the Czech and QMJHL cases already listed by R123).

## 6. Reproduce

```
python -B local/runs/overnight-research-2026-09-28/probes/r123r2_schedule_reference.py docs/research/overnight-2026-09-26/data/r113-group-partitions.json <out.json>
python -B local/runs/overnight-research-2026-09-28/probes/r123r2_schedule_reference.py <r113.json> <out.json> --vectors-only
```
