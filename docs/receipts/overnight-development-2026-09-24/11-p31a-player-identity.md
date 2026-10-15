# Receipt 11 (P3.1-A) — player identity data on every anchor; the weight base corrected for PS4 and NHL 08

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p31a-player-identity`. Clock: 05:50 → 06:15 (MDT, 25 Sep 2026).

## 1. Plain-language summary

Phase 3 opens with **3.1 identity data**: names, birth date, nationality and birthplace, position,
handedness, height and weight. A new sweep opens all 27 committed anchors, reads one rostered
skater's identity card, checks every value is composed into its human form from the per-title
probes (a real-looking date, `6'1"`, `190 lb`, enum labels), edits seven identity cells through the
ordinary validated write, saves to a disposable copy, reopens it, and undoes every edit — on every
anchor. Refusals read as text (a 200-byte name names the field and its width; an out-of-range
height names the range). The shipped names were also measured: **every non-ASCII name and city on
every anchor is valid UTF-8** (Luleå, Litoměřice, Düsseldorf), so Studio's UTF-8 writes match the
data; no title uses a single-byte code page.

The sweep found one real defect and it is fixed red → green: **the weight base**. `ubPc.sNZW`
stores pounds minus 150 on NHL 07–10 and minus 120 from NHL 11 on, including every PS4 title. The
code decided the base from the *rating-scale* probe, which fired the wrong way on PS4 (no 6-bit
rating field there → 150: Lasse Kukkonen showed 220 lb, and a staged 200 lb would have written a
value the game reads as 170) and on NHL 08 (its goalie table has a 6-bit value above 49 → 120:
Kukkonen showed 157 lb). The base is now probed from the weight column's own distribution and
pinned on all 27 anchors; Kukkonen reads 190 lb on every title that lists him (187 on NHL 08, EA's
own roster figure). Display and the mirrored edit offset both follow the corrected base, so the
historical import's weight lowering does too.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/assets.rs` | **new** `probe_weight_base(db, bio)`: median non-zero raw weight < 60 → 150, else 120 (fallback 120); doc records the 27-anchor measurement and why the rating probe is the wrong question |
| `crates/studio-domain/src/lib.rs` | export |
| `crates/studio-domain/src/view.rs` | both weight consumers (`read_field`'s display / edit-form closure; the header's formatted weight) use the probe; comments corrected (the old comment claimed +120 from NHL 10 — NHL 10 is a 150 title) |
| `crates/studio-domain/tests/weight_base.rs` (**new**) | the base per anchor (27); Kukkonen 190 lb / 6'1" on the 24 anchors that list him |
| `crates/studio-api/tests/player_identity.rs` (**new**) | the 27-anchor identity sweep (columns present; header composition; string encoding census; seven edits → header → validate → save / reopen → undo; refusal texts; per-family base pinned) and the raw weight distribution / six-bit census measurement table |

No DTO, binding or frontend change.

## 3. Evidence

- [`p31a-player-identity-sweep.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p31a-player-identity-sweep.log)
  — the per-anchor table (27 rows): identity columns absent **none** on every anchor; headers such
  as "Lasse Kukkonen — DOB 1981-09-18 · 6'1" · 190 lb · Left · D · Finland · Oulu, Finland"; encoding
  census `not-utf-8 0` on all 27; edits "last name, height, weight, birth day (offset 0 on 07–10,
  1 after), handedness, nationality, position · saved, reopened, 7 undone" on all 27.
- [`p31a-weight-base-measurement.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p31a-weight-base-measurement.log)
  — raw weight mean 44–46 (NHL 07–10) vs 69–73 (NHL 11–Legacy, PS4 15–21); six-bit `dSvy` census
  (PS4: 33 fields, max 27–37; NHL 07–10: max 49; 11–Legacy: max 63); Kukkonen raw 19 / 37–40 vs 19 / 70.
- [`p31a-weight-base-red.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p31a-weight-base-red.log)
  — with the old consumers: probe test green on 27, Kukkonen test **red** (NHL 08 "157 lb").
- [`p31a-weight-base-green.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p31a-weight-base-green.log)
  — after the fix: `weight_base` 2 passed; `nhl07_encodings` 5, `import_lower` 2, `birthdate_encoding` 8, `view` 16 passed.

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-api --test player_identity` (27 anchors, 7 edits each) | **2 passed** (147 s) |
| `cargo test -p studio-domain --test weight_base --test nhl07_encodings --test import_lower --test birthdate_encoding --test view` | 33 passed |
| `cargo test -p studio-domain --lib -- --skip zz_diag` | **222 passed, 0 failed, 2 ignored** |
| `rustfmt --check` on the two new test files | clean (both formatted) |
| `cargo clippy -p studio-domain -p studio-api --tests` (scan of touched files) | nothing new |

Repo-wide fmt / clippy remain the recorded C0.2 baseline defects; not re-measured. Bindings unchanged.

## 5. Coverage recorded (Phase 3 matrix, 3.1 rows)

| 3.1 | X360 07 / PS3 07 (research) | X360 08–10, PS3 08–10 | X360 11–Legacy, PS3 11–Legacy | PS4 15–21 |
|---|---|---|---|---|
| columns | all 12 identity columns present | same | same | same |
| encodings | weight base 150; birth month / day 1-indexed; height +54 | same (NHL 10 incl.) | weight 120; month / day 0-indexed | weight 120 (**was 150 — fixed**); 0-indexed |
| names | UTF-8 (no other code page) | UTF-8 | UTF-8 | UTF-8 |
| edit → save → reopen → undo | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev |

Recompiled: by identity with Legacy (no independent anchor). Runtime display / behaviour agreement
(the roadmap's 3.1 acceptance) is a console question — card V-55 / R-ID-1.

## 6. Limitations and observations

- The subject on each anchor is the first rostered skater with both names; goalies' identity cells
  go through the same `set_field` but were not swept separately (the position edit deliberately
  never crosses the skater / goalie boundary — that is a kind change, Phase 3.2).
- The UI does not surface a string field's byte width before the write; the refusal after the write
  does ("holds at most N bytes"). A pre-write cap would be a small `FieldViewDto` addition — not
  done here (no defect, a convenience).
- The rating-scale probe (`sixbit_rating_offset`) is untouched and still gates the rating offset;
  whether it is also wrong for PS4 ratings is outside 3.1 (Attributes are 3.2) — flagged for that step.
- The R1 review's hand-written weight comment ("+120 on NHL 10 → Legacy") was wrong for NHL 10;
  NHL 10 is a 150-base title by measurement (Kukkonen raw 40). Memory note updated.

## 7. Cleanup

Scratchpad scripts (`append_weight_probe.py`, `patch_weight_base.py`, `patch_weight_base_b.py`)
deleted by literal path; `nhlms-identity-sweep-<pid>` temp roots removed on drop (verified absent);
no processes.

## 8. Next action

**3.1 continued, then 3.2:** add the per-target UI-side pre-check for string width (optional), then
open **3.2 additional database entries** starting with the ratings scale per title (the
`sixbit_rating_offset` probe's PS4 behaviour is the first fact to measure, using the same 27-anchor
sweep pattern) — unless the morning review returns a League / Team / identity defect.

## 9. Correction (25 Sep 2026, receipt 20)

The development audit (F1) found that deciding the weight base from the weight column's median is
still a decision from editable cells: a roster bulk-edited to 170 lb on a base-120 title would be
re-read as base 150. The base now comes from the schema-family contract
(`studio_domain::encoding`); `probe_weight_base` is retained as a diagnostic only. The measurements
above stand; the mechanism described in §1–2 is superseded.
