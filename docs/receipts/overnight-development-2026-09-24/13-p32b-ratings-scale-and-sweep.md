# Receipt 13 (P3.2-B) — ratings, style and potential on every anchor; the six-bit scale probe narrowed to ratings

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p32b-ratings-sweep`. Clock: 06:45 → 07:03 (MDT, 25 Sep 2026).

## 1. Plain-language summary

The second 3.2 package sweeps **ratings, player / goalie style and potential range** on all 27
anchors, one skater and one goalie each: every rating cell shows raw + the title's scale offset
within 1..99, one rating is edited to 88, the style to another eligible archetype and the potential
where the title has one, an out-of-range rating refuses by range, validation gains no error, the
edits survive save → reopen on a disposable copy, and undo restores them.

The sweep found the **scale probe** wrong on NHL 08. Xbox 360 / PS3 titles store ratings in six
bits and the metadata cannot say whether the display is raw + 36 (NHL 11 → Legacy) or raw + 50
(NHL 07–10); the code inferred it from "any six-bit value above 49 anywhere in the two companion
tables". NHL 08's goalie table holds `g_positioning` = 50 on 33 of 2 774 rows, and the old probe
also read non-rating six-bit fields (`potential`, traits), so NHL 08 was shown on the 36 scale
(every rating 14 too low: Kukkonen-era skaters topping out at 85) while NHL 07 / 09 / 10 sat on 50.
The probe now counts only rating-classified cells over a bounded sample and calls the scale 36 only
when more than 1 % of them exceed 49 (NHL 11 → Legacy: 2–4 %; NHL 07–10: 0–0.05 %). The roster
list's per-table reader uses the same probe (it previously stamped 36 on every six-bit title).
The Attributes tab's caption now reads the offset it was given ("raw + 50 → 50..99" on NHL 07–10;
it used to say "stored 0..99" there) and keeps the raw companion table id behind a tooltip.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/view.rs` | `sixbit_rating_offset(db, meta)`: rating-classified six-bit cells only, ≤ 1 024 records per table, > 1 % above 49 → 36 else 50; `CompanionReader::build` probes once per table for its display offsets |
| `crates/studio-domain/tests/rating_scale_probe.rs` (**new**) | per-anchor maxima (rating vs other six-bit fields), the count above 49, the rule asserted on 20 gen7 anchors; NHL 08's 33 cells pinned |
| `crates/studio-api/tests/player_ratings.rs` (**new**) | the 27-anchor ratings / style / potential sweep (skater + goalie) with save / reopen / undo and refusal text |
| `app/src/components/AttributesTab.tsx` | caption from the stamped offset; table id behind `title` |
| `app/src/components/AttributesTab.test.tsx` (**new**) | 36 / 50 / 0 captions; no raw id in text |

No DTO or binding change.

## 3. Evidence

- [`p32b-rating-scale-measurement.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32b-rating-scale-measurement.log)
  — the pre-fix measurement table: dSvy / mHuy rating maxima 49 on NHL 07–10 (NHL 08 mHuy 50 via
  `g_positioning`, 33 cells), 63 on NHL 11–Legacy with thousands of cells above 49; PS4 no six-bit ratings.
- [`p32b-rating-scale-green.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32b-rating-scale-green.log)
  — after the narrowed probe: `rating_scale_probe` 1, `nhl07_encodings` 8, `ratings_write_codec` 3, `view` 16, `roster_overall_column` 4 passed.
- [`p32b-player-ratings-sweep.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p32b-player-ratings-sweep.log)
  — the 27-row table (scale per anchor; skater and goalie edit rows). The first run's NHL 08 failure
  ("Speed edit offset left 36 right 50") is the red record for the probe.

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-api --test player_ratings` (27 anchors × skater + goalie) | **1 passed — 27 anchors, 54 subjects (230 s)** |
| `cargo test -p studio-domain --test rating_scale_probe --test nhl07_encodings --test ratings_write_codec --test view --test roster_overall_column` | 32 passed |
| `npx vitest run` (full) | **37 files, 234 passed** (AttributesTab 3) |
| `npx tsc --noEmit` | clean |

Repo-wide fmt / clippy remain the recorded C0.2 baseline defects. `cargo test -p studio-domain --lib -- --skip zz_diag` after this package: **222 passed, 0 failed, 2 ignored**.

## 5. Coverage recorded (Phase 3 matrix, 3.2 rows)

Rating scale per family: NHL 07–10 raw + 50 (**probed, provisional** — the population argument:
no rating above 49 on 07 / 09 / 10 and a single 50-valued goalie field on 08; in-game agreement is
a console question), NHL 11 → Legacy raw + 36 (in-game verified for 14 / 15 / Legacy by the earlier
program, provisional 11–13), PS4 stored 0..99 (metadata). Edit → save → reopen → undo:
impl+tested on all 27 for one rating, the style (where the vocabulary is editable) and the
potential range (where present).

## 6. Observations for research

- NHL 08 goalie `g_positioning` = raw 50 on 33 rows (display 100 on the 50 scale): a clamp or an
  authoring overflow — which does the console show?
- Whether NHL 07–10 display ratings as raw + 50 is inferred from the distribution, not read in
  game; a single screenshot of a known player's rating on NHL 09 / 10 would close it.

## 7. Cleanup

Scratchpad `patch_sixbit.py` deleted by literal path; `nhlms-ratings-sweep-<pid>` temp roots removed
on drop; no processes.

## 8. Next action

3.2 continues with the **ledger of the remaining 3.2 operations** (companion records, roster
operations, contracts / draft / career, historical import, bulk edit) against the existing
operation matrix and the Phase 3 code — then the first gap it names.

## 9. Correction (25 Sep 2026, receipt 20)

The 1 % rule over rating cells is a decision from editable values (audit F1): lowering the sampled
ratings below 50 on a 36-scale title would move the scale to 50 on the next read and write. The
scale now comes from the schema-family contract (`studio_domain::encoding`: +50 on NHL 07–10 —
NHL 07 in game, 08–10 `Inferred`; +36 on NHL 11–Legacy; PS4 stored 0..99);
`sixbit_rating_population_offset` is a diagnostic only. The measurements above stand.
