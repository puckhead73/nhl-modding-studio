# Handoff R118 (P12) — Players 3.2: a cross-title continuity check of the six-bit rating offsets — ~~the same skaters' raw ratings do not move between NHL 07 → 08 → 09 → 10, jump by exactly **+14** on every shared field from NHL 10 to NHL 11, and stay put from NHL 11 to NHL 14;~~ **(wording corrected 2026-09-26 03:43)** the same skaters' raw rating differences have median 0 between NHL 07 → 08 → 09 → 10, a median of **+14** on every shared field from NHL 10 to NHL 11 (quartiles 11 / 14 / 14), and median 0 / +1 from NHL 11 to NHL 14; with NHL 07 (+50) and NHL 14 / 15 / Legacy (+36) verified in game, this is consistent with +50 on NHL 08–10 and +36 on NHL 11–13; controls reproduce the known +36 gap to PS4 as the median of every shared field (quartiles 36 / 36 / 38)

Research lane, 26 September 2026, published 03:09 MDT (system clock). Roadmap Phase 3.2 (after the League / Team queue). Claim
`research-p12-rating-offset-continuity`. Read-only over the X360 anchors 07–Legacy (PS3 byte-identical per title) and PS4 15 / 16.
Evidence class: **cross-title consistency (inferred)** — it assumes a real player's displayed ratings change little between consecutive
releases; it does not observe the game's display and does not promote any cell to game-verified.

> **Correction 2026-09-26 03:43 (review `research-20260926-r113-r118-review.md` items 1–2; raw results unchanged).** (1) These are **distributions**, not identical
> changes in every cell: "nothing moves", "everything moves up by 14" and "exactly 36" below are replaced by the measured medians and
> spread (e.g. NHL 10 → 11 skaters: every shared field has median +14, all-cell quartiles 11 / 14 / 14; X360 → PS4 NHL 15: median +36 on
> every field, quartiles 36 / 36 / 38). Individual players' changes differ; the median per field is the continuity evidence. Status stays
> **inferred**. (2) **Source of the endpoint observations:** Studio's encoding contract records NHL 07 +50 and NHL 14 / 15 / Legacy +36 as
> `InGame` — "read on a console by the earlier program" (`crates/studio-domain/src/encoding.rs` module note and the rating-offset evidence
> list; phase-3 coverage matrix, receipt 20 row "six-bit rating scale"). This research did not rerun them. (3) Goalie `consistency`: the
> example is corrected in §3 (NHL 08 follows the contract's +50 path), the value meaning is not assigned, and `floppiness` stays unresolved
> per title.

Machine-readable: [`data/r118-rating-offset-continuity.json`](data/r118-rating-offset-continuity.json) (per title: hash, join field,
matched skaters, rating fields; per pair: matched players, common fields, median / quartiles of all cell differences, median of player
medians, per-field medians).

## Plain-language summary

Older NHL titles store ratings as small numbers and add a fixed amount before showing them — 50 on some titles, 36 on others — and for
NHL 08–10 (and to a lesser degree 11–13) that amount had only been inferred. This check follows the same players from one year's game to
the next. If the added amount stayed the same, their stored numbers should barely move; if it changed, every stored number should move
by the difference. ~~That is exactly what happens: nothing moves from NHL 07 to NHL 10, everything moves up by 14 from NHL 10 to NHL 11
(50 minus 36), and nothing moves again up to NHL 14.~~ *(Corrected 2026-09-26 03:43:)* That is what the typical player shows: a median change
of 0 from NHL 07 to NHL 10, a median rise of 14 on every rating from NHL 10 to NHL 11 (50 minus 36), and a median of 0 or 1 up to NHL 14;
individual players still move up or down around those medians. The two ends are already confirmed in the game, so the middle titles fit both. As a
test of the method, the same comparison between the Xbox 360 and PS4 versions of NHL 15 shows the known gap of 36 as the median on every rating.

## 1. Results (skaters matched by first name, last name and calendar birth month / day; duplicates dropped)

| Pair | matched skaters | shared fields | median raw difference (all cells) | quartiles | per-field medians | reading |
|---|---|---|---|---|---|---|
| NHL 07 → 08 | 1 742 | 16 | **0** | 0 / 0 / 6 | 0 … 3 | same offset |
| NHL 08 → 09 | 2 022 | 23 | **0** | −2 / 0 / 0 | −1 … 0 | same offset |
| NHL 09 → 10 | 2 534 | 23 | **0** | 0 / 0 / 3 | 0 … 1 | same offset |
| NHL 10 → 11 | 2 735 | 20 | **+14** | 11 / 14 / 14 | **14 … 14** (every field) | offset drops by 14 |
| NHL 11 → 12 | 3 430 | 25 | +1 | 0 / 1 / 3 | 0 … 2 | same offset (season drift) |
| NHL 12 → 13 | 3 603 | 25 | **0** | 0 / 0 / 2 | 0 … 1 | same offset |
| NHL 13 → 14 | 3 859 | 25 | **0** | 0 / 0 / 1 | 0 … 0 | same offset |
| *control* NHL 14 → 15 | 3 473 | 25 | 0 | 0 / 0 / 2 | 0 … 0 | both +36 in game |
| *control* NHL 15 → Legacy | 3 685 | 25 | 0 | 0 / 0 / 3 | 0 … 0 | both +36 in game |
| *control* X360 NHL 15 → PS4 NHL 15 | 4 823 | 25 | **+36** | 36 / 36 / 38 | **36 … 36** | PS4 stores 0..99 (offset 0) |
| *control* Legacy → PS4 NHL 16 | 4 916 | 25 | +36 | 36 / 36 / 36 | 36 … 36 | same |
| *control* NHL 14 → PS4 NHL 15 | 3 483 | 25 | +37 | 36 / 37 / 40 | 36 … 39 | +36 plus a season's drift |

Fields matched by suffix (`p_*` on 07–13, `c_*` from 14); potential, fighting, style and overall excluded. Hashes per title in the JSON.

## 2. Reading (inferred)

Anchored at NHL 07 = +50 (in game): 07 → 10 differences of 0 give **+50 on NHL 08, 09, 10**. Anchored at NHL 14 = +36 (in game): 11 → 14
differences of 0 / +1 give **+36 on NHL 11, 12, 13**. The 10 → 11 step — a median of +14 on every shared field, all-cell quartiles 11 / 14 / 14 — closes the chain from both ends
(50 − 36 = 14). The controls show the method recovers a known offset gap as the per-field median (36 on every field; quartiles 36 / 36 / 38) and that same-offset pairs have a
median of 0 with quartiles within a few points.

**Not shown:** the display itself (a title could, in principle, add a different constant and also re-rate every player by the same amount —
implausible for a median +14 on every field, but not excluded statically); NHL 07 → 08 compares only 16 skater / 13 goalie fields; goalies: §2b.

## 2b. Goalies (added 03:12; `exhibitiongoalieai`, same matching; [`data/r118b-rating-offset-continuity-goalies.json`](data/r118b-rating-offset-continuity-goalies.json))

| Pair | matched goalies | fields | median | quartiles | per-field medians |
|---|---|---|---|---|---|
| NHL 07 → 08 | 197 | 13 | 0 | 0 / 0 / 4 | 0 … 0 |
| NHL 08 → 09 | 213 | 20 | 0 | 0 / 0 / 0 | 0, except `floppiness` −15 |
| NHL 09 → 10 | 285 | 20 | 0 | 0 / 0 / 2 | 0 … 0 |
| NHL 10 → 11 | 302 | 19 | **+14** | 12 / 14 / 15 | **14 … 14** (every field) |
| NHL 11 → 12 | 388 | 20 | +1 | 0 / 1 / 4 | 0 … 2 |
| NHL 12 → 13, 13 → 14, 14 → 15, 15 → Legacy | 383 / 391 / 349 / 372 | 20 | 0 | 0 / 0 / 0–2 | 0 … 0 |
| *control* X360 NHL 15 → PS4 NHL 15; Legacy → PS4 16 | 524 / 536 | 20 | +36 | 36 / 36 / 36–38 | 36 on 19 fields; **`consistency` 0** |

Goalies reproduce the skater result. Two fields do not behave like ratings: **`consistency`** stores only 0 / 1 / 2 on every title
measured (NHL 08–15, PS4 15, PS4 21), whether declared 2-bit 0..3 (NHL 09 / 10), 6-bit 0..63 (NHL 08, 11–15, Legacy) or 7-bit 0..99
(PS4) — three stored values on every source; their labels and the legal write domain are **not** established by this census; **`floppiness`** (NHL 07–10 only) stores 0..2 on NHL 07, 0..50 on NHL 08 and 0..40 on NHL 09 / 10 (median
shift −15 from 08 to 09) — its scale changes between titles. `puckplayfrequency` follows the +14 / +36 relation like a rating.

## 3. Developer-facing note

Studio's contract already uses +50 (NHL 07–10) and +36 (NHL 11–Legacy) (`studio_domain::encoding`; receipt 20 labels NHL 08–10
"inferred, provisional" and 11–13 "provisional"). This adds an independent static consistency witness for those labels; it does **not**
change the evidence level to game-verified. Suggested wording if shown: "cross-title consistent with NHL 07 / NHL 14 (in game)".

**Observation for the developer (added 03:12):** Studio's rating rule (`attributes.rs` `rating_offset`: prefix + declared 0..63 / 0..99,
excluding fighting / style / paddledown / potential) classifies goalie `consistency` as a rating wherever it is declared 0..63 or 0..99,
so the same goalie would show ~~36–38 on X360 / PS3 NHL 08 and 11–Legacy~~ **50–52 on NHL 08** (`view.rs` substitutes the contract's
six-bit offset, +50 there) and **36–38 on NHL 11–Legacy**, but 0–2 on PS4 (corrected 2026-09-26 03:43). The stored values are 0 / 1 / 2 everywhere
(stock census; not a label or write-domain result). **Routed to the developer** (queue C15): a focused classification investigation with
regression coverage across display, edit, import and derived calculations; do not assign meanings to the three values, clamp to 0..2, or
alter a fitted goalie Overall formula without separate justification. Goalie `floppiness` on NHL 08–10 (declared 0..63, stored up to 50) is likewise not established as a
36..99 rating and stays unresolved per title. Research does not edit the rule; the classification is the developer's decision.

```
python local/runs/overnight-research-2026-09-26/probes/r118_rating_offset_continuity.py <out.json>
```
Executed twice: the first run matched only `p_*` names and could not compare NHL 14 onward; the published run matches `p_*` / `c_*`
by suffix (NHL 07–13 rows identical in both runs). Output published through `safe_patch.py put`.
