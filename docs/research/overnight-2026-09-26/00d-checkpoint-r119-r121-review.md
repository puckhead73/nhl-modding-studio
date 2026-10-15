# Checkpoint 00d — corrections from the coordinator's R119–R121 review

Research lane, 27 September 2026, published 04:47 MDT (system clock). Review
[`docs/validation/research-20260926-r119-r121-review.md`](../../validation/research-20260926-r119-r121-review.md); continuation prompt
[`docs/plans/research-20260926-after-receipt3.md`](../../plans/research-20260926-after-receipt3.md). Claim `research-p17-checkpoint-r119-r121`.
Developer receipts 28–31 (`docs/receipts/overnight-development-2026-09-24/`) were read first; they are developer reports, not accepted
results, and nothing here re-requests work they report as delivered.

## Plain-language summary

My C16 table compared the game's nationality numbers with one shared label file and called the result "what Studio shows". That was wrong
for NHL 08 / 09: Studio's player page swaps in its NHL 07 country table on every title with NHL 07's database shape, and that table already
has the right Czech, German, Swiss and Japanese entries. The measured number shifts between titles stand; the claim about what Studio
displayed on NHL 08 / 09 is withdrawn. NHL 10's wrong labels and NHL 11 / 12's missing Japan were real, and the developer reports both
fixed (receipt 28). "Every national team matches from NHL 13" is narrowed: the teams' codes are stable, but a roster only shows which number
a team uses, not the exact country wording (the British code is labelled "England"). The wider field on the newest PS4 titles and the
zero-height rows are now questions to investigate, not defects. One probe's rankings are now deterministic; the numbers did not change.

## 1. C16 D1 — the shared TOML is not Studio's resolver (withdrawn: "NHL 08 / 09 field views use the Legacy labels")

Current source path, read 27 Sep (not executed; no desktop observation):

| Consumer | Path | NHL 07–09 (3-bit `dSvy.mQoE`) | NHL 10–12 | NHL 13 → PS4 21 |
|---|---|---|---|---|
| Field view / dropdown, nationality `ubPc.Lelh` | `view.rs::read_field` → `attributes.rs::apply_gen6_geo_vocab` (gate `is_gen6_style_schema` = `dSvy.mQoE` bit width 3, a schema test, not a title id) | `GEN6_COUNTRIES` (27 codes; incl. 79 Czech Republic, 82 Germany, 89 Switzerland, 96 Japan) replaces the TOML list | per-title TOML (`nationality-nhl10.toml`, `-nhl11-12.toml`, receipt 28) | `nationality.toml`, now scoped NHL 13–21 + Legacy |
| Birth state `ubPc.QvxI` ≥ 66 | same override; since receipt 28 also `apply_title_birth_state_countries` | `GEN6_COUNTRIES` nation range; DC / Delaware swap (22 / 23) | title nationality labels, ≤ 65 preserved, continuity note | `birth-state.toml` |
| Import label → code | enum registry (`EnumRegistry::for_field`), not the field-view override | registry uses `nationality-nhl07.toml` (NHL 07) / `-nhl08-09.toml` (21 national-team codes) | per-title TOML | `nationality.toml` |

Descriptor widths (coordinator, independently; also R122 below): NHL 07 / 08 / 09 `mQoE` 3 b, NHL 10 4 b. Therefore:

- **Withdrawn:** "on NHL 08–10 Studio would call Czech players Croatian, Germans French and Swiss Slovak" (handoff 14 summary, RECEIPT-3,
  STATUS 03:54, queue C16). Those three examples are **NHL 10 only** (81 Croatia, 84 France, 95 Slovakia in the shared file). The raw TOML
  comparison on NHL 08 / 09 gave different values (Czech 79 unlabelled, German 82 "Czech Republic", Swiss 89 "Netherlands"), and the
  effective field view overrides them with `GEN6_COUNTRIES`.
- **Kept:** the measured title-to-title code shifts (R120 / R120b JSON unchanged, independently reproduced by the coordinator); NHL 10's
  concrete wrong labels; NHL 11 / 12 code 108 **missing** (no label — distinct from an incorrect label).
- **Delivered since (developer-reported, receipt 28 / 31, not re-requested):** per-title nationality files for NHL 08–09 / 10 / 11–12,
  unlabelled codes shown as numbers, birth-state country region following the title, a stored-code entry box for unlabelled in-range
  codes, the British code worded "England" from NHL 10.
- **Remaining consumer observation (not a defect claim):** on NHL 08 / 09 the field view shows `GEN6_COUNTRIES` (NHL 07 evidence, 27
  codes, e.g. 74 Lithuania, 85 Northern Ireland) while the import registry holds only the 21 national-team codes of
  `nationality-nhl08-09.toml` — display and import resolve different sets on the same title. Receipt 28 §4 already notes this for review;
  whether NHL 07's non-national-team entries apply to NHL 08 / 09 has no NHL 08 / 09 title-owned evidence either way.

## 2. Summary examples and "all 21 teams after NHL 13" (narrowed)

- Replace "from NHL 13 on, Studio's list matches every national team" with: **from NHL 13 onward each national team's code is stable and
  every one of the 21 codes has a label; roster membership fixes which code a team uses, not the exact country wording.** R120b itself
  records code 83 as "England" where its abbreviation map says "Great Britain" (`GBR`); national-team participation does not settle that
  distinction. No rename of 83 is proposed; unmeasured codes are not filled.
- Preserved ambiguity: `AUS` (Austria vs Australia) and `BEL` (Belarus vs Belgium) are decided by the rosters' players, not the abbreviation;
  mixed rosters (dominant share < 0.90: Austria on Legacy / PS4 16 / 17, Great Britain on NHL 19 base, Germany on PS4 18 / NHL 19 base) stay
  reported as mixed.

## 3. D2 and D3 reclassified; D1 birth-state and continuity scope

| Row | Was | Now |
|---|---|---|
| D2 | "edit-range check uses the title's declared width; 127 = N/A only where observed" (implied a 7-bit editor limit) | **Field-width fact + consumer audit.** `intlcountry` / `state` are declared 0..255 on the genuine NHL 20 and NHL 21 (measured). No failing consumer was identified by research; the developer reports 200 round-trips and 256 is refused on both (receipt 28 §3, `identity_ranges_nhl20_21`). Code 127's presence (NHL 10 on) / absence (NHL 07–09) is a population fact; it does **not** establish a sentinel contract on any title. |
| D3 | "show height raw 0 as 'no height' like weight (or hide placeholder rows)" | **Zero / placeholder presentation investigation.** Measured: 180–185 unnamed rows with weight 0 and height 0 on NHL 08–15 / PS4 15, plus the named, rostered NHL 12 exception (Colton Heffley, height raw 0). Raw 0 is also the minimum encodable height (54 in). Frequency does not establish `raw 0 = missing`; no global reinterpretation, row hiding or change to the minimum height is proposed. Placeholder criteria, if the developer investigates, must be explicit (e.g. empty name **and** weight 0 **and** no roster / contract reference) and must keep the named exception visible and editable. |
| D1 (birth state) | "D1 extends to `birth-state.toml` codes ≥ 66" | Birth-state country codes move at the same boundaries (continuity, R120 §7) — support for investigation, **not** an independently recovered title-owned label table. |
| D4 / D5 | weight / height / handedness / position "confirmed" | Position stays the stronger, separately measured result (roster line slots, R120c). Weight / height / handedness continuity **supports** the existing contract; it does not prove unchanged meaning for every field, value and title, and covers only the 19 listed sources. |

## 4. R121 determinism (reproducibility correction; no substantive change)

`r121_unscoped_enum_continuity.py` now sorts the matched keys and ranks every `Counter` by count descending, ties broken by code order.
Regenerated [`data/r121-unscoped-enum-continuity.json`](data/r121-unscoped-enum-continuity.json); the previous output is kept as
[`data/superseded-r121-unscoped-enum-continuity-20260926.json`](data/superseded-r121-unscoped-enum-continuity-20260926.json).

Comparison (executed 27 Sep): per-source metadata and distributions identical (19 sources); 19 pairs with identical `matched` counts; every
field-pair record has identical `n`, unchanged share, set of flagged source codes with player counts, and count profiles of `maps_to` and
`top_changes`. 45 field-pair records are byte-identical; 30 differ only in the order of equal-count entries or which equal-count entry fills
the last retained slot; one top destination changed and it is a tie (NHL 11 → 12 goalie style, code 0: 7 → code 0 and 7 → code 1). Two
consecutive runs of the corrected probe are byte-identical. The D6 / D7 conclusions are unchanged.

## 5. C15 disposition (updated)

Developer receipt 27 corrected the goalie `consistency` classification (shown as the stored 0 / 1 / 2 on every title, tooltip says the
meaning is not established); the coordinator's bounded offline review passed. **Open:** the numeric meanings of 0 / 1 / 2 and the separate
goalie Overall calibration question (receipt 28 §5 proposes +36 on PS4 NHL 15 / 16 only; not implemented). 00c's description of NHL 08
50–52 / NHL 11–Legacy 36–38 is a historical description of the pre-receipt-27 behaviour, not a fresh unfixed application issue.

R119 stands with its qualifiers: record total = present + empty slots on every archive-backed PS3 title, and Xbox 360 NHL 09's 4 902 counts
the archive and loose copies (2 × 2 451).

## 6. New consumer evidence gathered for these corrections (R122, carried into handoff 15)

[`data/r122-style-consumer-scope.json`](data/r122-style-consumer-scope.json): which vocabulary the current rules attach to each style field
per source. Short form: the companion skater / goalie style shows the NHL 07 labels on NHL 07–09 (gen6 gate), **a number on NHL 10–13**
(`dSvy.mQoE` / `mHuy.wJus` 4 / 2 b, no binding, no synthetic vocabulary), and `player-style.toml` / `goalie-style.toml` from NHL 14; the
roster table's `eGlu.QgFs` is bound to the Legacy-anchored `player-style.toml` on **every** title. So R121's "Studio's title-free
`player-style.toml` may mislabel NHL 07–10" is corrected: on the companion field it does not apply before NHL 14; on the roster field it
applies everywhere. Handoff 15 publishes the discriminating evidence.

## Updated records

Dated notes: handoff 14 (header, summary, §4, §5 D1–D3, §7, §8), RECEIPT-3, STATUS, `overnight-2026-09-25/DEVELOPER-QUEUE.md` C15 / C16,
claims p15 / p16. Raw evidence (R120 / R120b / R120c JSON) unchanged.
