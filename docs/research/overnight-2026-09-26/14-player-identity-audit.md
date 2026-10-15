# Handoff R120 (P15) — Players 3.1 identity audit across titles: weight, height, handedness and position hold their codes and units on every source; **nationality codes are renumbered between titles** — national-team rosters (title-owned) put Czech Republic at 79 / 81 / 82, Germany at 82 / 84 / 85, Switzerland at 89 / 95 / 97 and Japan at 96 / 105 / 108 / 109 across NHL 08 → 13 — so Studio's Legacy-anchored, title-free nationality labels are wrong on NHL 08–10 and for Japan on NHL 11 / 12; also a 7 → 8-bit widening on PS4 20 / 21 and 180 blank placeholder rows whose height would display as 4'6"

> **Correction 2026-09-27 04:48 ([00d](00d-checkpoint-r119-r121-review.md) §1–3):** the title's claim about Studio's labels on NHL 08 / 09 is **withdrawn** — R120b compared the codes with the shared `nationality.toml` only, but Studio's field view applies its NHL 07 country table (`GEN6_COUNTRIES`, gate: 3-bit `dSvy.mQoE`) on NHL 07–09, and that table has the Czech, German, Swiss and Japanese codes right. The Croatian / French / Slovak examples are **NHL 10 only**; NHL 11 / 12 Japan was a **missing** label. "All 21 teams match from NHL 13" means stable codes, each labelled — not exact country wording (code 83 = "England"). D2 / D3 reclassified (§5 note). The measured code shifts stand.

Research lane, 26 September 2026, published 03:54 MDT (system clock). Review `research-20260926-r113-r118-review.md` (Players 3.1
prompt). Claim `research-p15-identity-audit`. Read-only over the anchors and the private fixture copies; Studio source and `defs/` read,
not edited.

Machine-readable: [`data/r120-identity-audit.json`](data/r120-identity-audit.json) (per source: hashes, provenance, metadata, raw and
decoded distributions, national-roster codes, tournament-stat joins; per pair: matching statistics and field comparisons),
[`data/r120b-nationality-label-table.json`](data/r120b-nationality-label-table.json) (per-title code per national team vs Studio's enum
label), [`data/r120c-position-line-slots.json`](data/r120c-position-line-slots.json) (position codes per roster line slot; zero rows).

## Plain-language summary

The same players were followed from each release to the next, and each field was checked against something the game itself stores. Weight,
height, handedness and position mean the same thing on every title: players keep them from year to year except for real roster changes,
and the positions agree with the line slots the game puts players in. Nationality is different. The number used for a country shifts between
the older titles as countries are inserted into the list; the national teams' own rosters show it directly (the Czech national team's players
are 79 on NHL 08 / 09, 81 on NHL 10 and 82 from NHL 11). Studio labels these numbers with one list taken from NHL Legacy, so on NHL 08–10 it would
call Czech players "Croatian", Germans "French" and Swiss "Slovak", and on NHL 11 / 12 it would miss Japan. From NHL 13 on, Studio's list
matches every national team. Two smaller points: the newest PS4 titles store nationality in a wider field, and the older titles contain about
180 empty placeholder players whose zero height would show as 4 ft 6 in.

## 1. Coverage and identities

19 distinct sources (hashes and provenance in the JSON): X360 NHL 07–15 and Legacy (PS3 byte-identical per title, recorded as aliases), PS4 15,
16, 17, 18, NHL 19 base (`anchors/ps4/nhl19` = `anchors/ps4/nhl20`, alias), **real NHL 20** (`1e200478…`), NHL 21, and the patch variants NHL 15
PS4 patch 00 and NHL 19 patch 56. Not covered: Legacy Recompiled (no independent database), unheld patches. NHL 07 has no national-team league
(gen6 team model), so its nationality evidence is the tournament join only.

**Matching.** K1 = first + last name + calendar birth date (not a field under investigation; year base 1959 on the narrow NHL 07 / 08 year
field, 1900 elsewhere; month / day base per R116); duplicate keys dropped. K2 = `game_id`, reported as confidence only. Same-season and
same-title pairs are listed separately from adjacent seasons.

| Pair | kind | matched | unmatched A / B | K1 collision keys A / B | K2 agrees |
|---|---|---|---|---|---|
| X360 NHL 15 / PS4 NHL 15 | same season | 5 347 | 7 / 28 | 1 / 1 | 5 347 |
| Legacy / PS4 NHL 16 | same season | 5 452 | 0 / 0 | 1 / 1 | 5 452 |
| PS4 15 / patch 00 | same title | 5 375 | 0 / 0 | 1 / 1 | 5 375 |
| NHL 19 base / patch 56 | same title | 6 843 | 270 / 1 413 | 1 / 1 | 6 843 |
| adjacent seasons (15 pairs) | adjacent | 1 938 – 5 766 | 294 – 2 185 per side | 1 – 9, **PS4 18: 140** | all but 0 – 56 per pair (PS4 16 → 17: 4 204 / 4 260) |

The PS4 NHL 18 database holds 140 name + birth-date keys used twice (dropped from matching; not investigated further).

## 2. Weight and height (stable contract; continuity is supporting evidence only)

| Check | Result |
|---|---|
| decoded weight change (contract bases 150 → 120 at NHL 10 → 11) | median 0 lb on every pair, quartiles 0 / 0 / 0 on all but PS4 17 → 18 (0 / 0 / 6) and 19 → 20 (0 / 0 / 4); **raw** median +30 exactly at NHL 10 → 11 and 0 elsewhere — consistent with the base change |
| same-season / same-title pairs | weight and height identical for 100 % of matched players, except NHL 19 base → patch 56 (roster update: 75 % within 5 lb, 96 % within 1 in) |
| height change (raw + 54 in) | median 0 on every pair; 95–100 % within 1 in |
| metadata | weight 8 b 0..255, height 5 b 0..31 on every source |
| zero rows | NHL 08–15 and PS4 15: 180–185 rows with raw weight **and** height 0 (plus 1–17 weight-only zeros); all unnamed except 5 on NHL 09 and 1 on NHL 12 (Colton Heffley, on a roster); none on Legacy, PS4 16 or later (0–5 weight-only zeros) |

## 3. Handedness and position

Handedness 1 b on every source, values 0 / 1; same player keeps it 97.2–99.95 % across adjacent seasons, 100 % same-season; no systematic code
move. Position 3 b, values 0–4 on every source; 90.9–99.8 % kept across seasons (moves between wings and centre dominate), 100 % same-season;
no systematic move. **Title-owned check (R120c):** in the roster line slots, centre slots hold mostly code 0, left-wing slots 1, right-wing 2,
defence slots 3 (≥ 96 %), goalie slots **only** 4 — on all 19 sources, matching Studio's position enum (0 Centre, 1 LW, 2 RW, 3 D, 4 G).

## 4. Nationality

**Title-owned label evidence (R120b):** the international league's 21 national teams; each roster's players share one `intlcountry` code
(dominant share 0.78–1.00; below 0.90 only for Austria on Legacy / PS4 16 / 17, Great Britain on NHL 19 base and Germany on PS4 18 / NHL 19 base — 6 of 378 team-title cells). Code per team (NHL 08 / 09 → 10 → 11–12 → 13+; PS4 15–21 = 13+):

| Team | 08 / 09 | 10 | 11 / 12 | 13 → PS4 21 | Studio label at the 08 / 09 code | at the NHL 10 code |
|---|---|---|---|---|---|---|
| CAN, USA, FIN, SWE, DEN, NOR, BEL (Belarus), KAZ | 0, 14, 67, 69, 66, 68, 70, 72 | same | same | same | correct | correct |
| AUS (Austria) | 77 | 78 | 78 | 78 | Ukraine | Austria |
| RUS / UKR / LAT | 75 / 76 / 73 | 76 / 77 / 74 | same as 10 | same | Lithuania / Russia / (none) | correct |
| CZE | 79 | 81 | 82 | 82 | (none) | Croatia |
| GBR | 80 | 82 | 83 | 83 | (none) | Czech Republic |
| FRA | 81 | 83 | 84 | 84 | Croatia | England |
| GER | 82 | 84 | 85 | 85 | Czech Republic | France |
| ITA | 84 | 86 | 87 | 87 | France | Hungary |
| POL | 86 | 89 | 91 | 91 | Hungary | Netherlands |
| SVK (`SLV`) | 88 | 93 | 95 | 95 | (none) | Scotland |
| SUI (`SWI`) | 89 | 95 | 97 | 97 | Netherlands | Slovakia |
| JPN | 96 | 105 | **108** | 109 | Slovenia | (none) |

Cross-title player continuity agrees: 69.7 % / 69.8 % of matched players keep their code at NHL 09 → 10 and 10 → 11 (e.g. 81 → 83 → 84 for
27 players; 84 → 86 → 87 for 25), 99.2 % at 12 → 13 with the single systematic move 108 → 109 (26 players), ≥ 98 % elsewhere.

**Other facts:** the field is 7 b 0..127 through NHL 19 and **8 b 0..255 on the real NHL 20 and NHL 21** (stored values still ≤ 127, 255
unused); code 127 (Studio: "-- N/A --") is absent on NHL 07–09 (maximum code 96) and used on NHL 10 onward (7–305 players); at X360 NHL 15 →
PS4 NHL 15 (same season) 103 players with 127 on Xbox carry a real code on PS4, and at real NHL 20 → NHL 21 117 players move from 127 (81 to 0).
**Tournament stats:** `stockintltourneyplayerstats.country` equals the player's `intlcountry` on NHL 07 (1 415 / 1 425) and NHL 08 (1 337 / 1 340)
via `playerid = game_id`; from NHL 09 the join resolves only 25–60 % of rows and the unequal pairs follow the NHL 08 code table (CZE 79 vs 82,
RUS 75 vs 76) — **inferred:** the table was carried forward from the NHL 08 era; not used as label evidence after NHL 08.

Birthplace strings: 90.9–100 % unchanged across pairs (roster edits otherwise); not a code field.

## 5. Developer discrepancy table

> **Correction 2026-09-27 04:48 ([00d](00d-checkpoint-r119-r121-review.md) §1–3):** D1 applies to the shared TOML and the enum registry, not the NHL 07–09 field view (see the header note); NHL 10 wrong labels and NHL 11 / 12 missing Japan are the concrete cases (developer reports both fixed, receipt 28). **D2** is now a field-width fact plus consumer audit (no failing consumer found; 127 is not established as a sentinel on any title). **D3** is now a zero / placeholder presentation investigation with explicit placeholder criteria and the named rostered NHL 12 exception — not authority for `raw 0 = missing`, row hiding or a changed minimum height. D4 / D5 continuity supports the contract on these 19 sources; position is the separately measured result.

| # | Existing behaviour | Observed evidence | Confidence | Affected titles | Smallest regression / next check |
|---|---|---|---|---|---|
| D1 | `defs/enums/nationality.toml` binds `ubPc.Lelh` labels with no title scope (Legacy-anchored) | national-team rosters give per-title codes (§4 table); codes shift at NHL 09 → 10, 10 → 11 and 12 → 13 | high (title-owned roster evidence on every title, confirmed by player continuity) | NHL 08, 09, 10 (X360 / PS3): most European codes; NHL 11, 12: Japan (108), and possibly other codes that no national team covers; NHL 07: **unknown** (no national rosters; NHL 07 / 08 share the tournament code space) | scope the labels per title family; regression: on each anchor, a player of each national team renders that team's country (e.g. an NHL 10 Czech national-team player shows "Czech Republic", not "Croatia"); keep unlisted codes as raw numbers; do not infer labels for codes without roster evidence |
| D2 | nationality field treated as 7-bit with 127 = N/A | 8 b 0..255 on real NHL 20 and NHL 21; 127 still the stored "no country" value there; 127 unused on NHL 07–09 | measured (metadata + distributions) | PS4 20, 21; NHL 07–09 | edit-range check uses the title's declared width; keep 127 as N/A only where observed; confirm the 07–09 "no country" value before labelling one |
| D3 | height = raw + 54 in, no zero sentinel (weight raw 0 = "no weight") | 180–185 unnamed placeholder rows with weight 0 and height 0 on NHL 08–15 / PS4 15; 1 named rostered case on NHL 12 | measured | X360 / PS3 08–15, PS4 15 | show height raw 0 as "no height" like weight (or hide placeholder rows); regression on one placeholder row per affected title |
| D4 | position and handedness enums (title-free) | line slots confirm 0–4 on all 19 sources; handedness codes stable | high for position (title-owned); continuity only for handedness | none | no change; optional regression: goalie slots resolve to position 4 on every anchor |
| D5 | weight base 150 (07–10) / 120 (11+) | raw +30 median at NHL 10 → 11, decoded median 0 | continuity (supporting) | none | no change |

## 6. Residuals and boundaries

NHL 07 nationality labels (no national rosters); codes outside the 21 national teams (no title-owned label found — they stay numbers); the
meaning of 127 on 8-bit titles beyond "no country"; the 140 duplicate keys on PS4 NHL 18; whether the tournament-stats table is consumed at all.

```
python local/runs/overnight-research-2026-09-26/probes/r120_identity_audit.py <out.json>
python local/runs/overnight-research-2026-09-26/probes/r120b_nationality_label_table.py <r120.json> defs/enums/nationality.toml <out.json>
python local/runs/overnight-research-2026-09-26/probes/r120c_position_line_slots.py <out.json>
```
R120 executed three times (first run: roster join through `game_id`, wrong — rosters reference `exhibitionplayers.index`; second run cut off
by a pipe before writing; third run published). R120b / R120c once each on the published R120 output.

> **Correction 2026-09-27 04:48 ([00d](00d-checkpoint-r119-r121-review.md) §1–3):** birth-state country continuity supports investigation; it is not an independently recovered title-owned label table.

## 7. Addendum 03:55 — birth `state` codes (same data, `r120-identity-audit.json` → `pairs[].fields.state`)

`ubPc.state` (7 b; **8 b on real NHL 20 / NHL 21**) holds Canadian provinces and US states in 1–65 and, per Studio's
`defs/enums/birth-state.toml`, country fallbacks from 66 that "mirror" the nationality codes. Same-player continuity: the 1–65 region shows no
systematic move on any pair; the country region moves at exactly the nationality boundaries — NHL 09 → 10: 79 → 81 (354 players), 82 → 84 (123),
88 → 93 (75), 84 → 86, 81 → 83, 86 → 89; NHL 10 → 11: 81 → 82 (337), 95 → 97 (218), 84 → 85 (149), 86 → 87, 83 → 84; NHL 12 → 13: 108 → 109 (26);
share of players keeping their code 69.0 % / 69.6 % at those two steps, ≥ 97.9 % elsewhere. Code 127 is used for 450–514 players on X360 NHL 15,
Legacy, PS4 15 and 16, rare elsewhere. **D1 therefore extends to `birth-state.toml` codes ≥ 66** (title-free, Legacy-anchored): wrong labels for
the country region on NHL 08–10 and for 108 / 109 on NHL 11 / 12; the North-American region is unaffected. Evidence class: continuity, anchored
by the nationality result (no separate title-owned state table found).

## 8. Addendum 03:57 — the other title-free player enums (R121, continuity only; [`data/r121-unscoped-enum-continuity.json`](data/r121-unscoped-enum-continuity.json))

Studio enums without a `[scope]`: nationality, birth state (above), position, handedness (confirmed), **eye colour, skin colour, player
style, goalie style** (player type not examined here). Same matching as R120; no title-owned label table was found for these four.

| Field (metadata) | Continuity result | Reading |
|---|---|---|
| skater style (`p_style` 3 b on NHL 07–09, 4 b from NHL 10; `playerstyle` from NHL 14) | **systematic +1 shifts**: NHL 09 → 10 3 → 4 (781 players), 4 → 5 (240), 7 → 8 (44); NHL 10 → 11 4 → 5 (401), 5 → 6 (199), 7 → 8 (86); share unchanged 34 % / 25 %; 83–99 % on every other pair, no systematic move | **possible renumbering** at NHL 10 and 11 (values inserted); Studio's title-free `player-style.toml` may mislabel NHL 07–10 — not established without a label source |
| eye colour (`eyecolorcode` 3 b; 5 b on PS4 17–19, 4 b on PS4 20 / 21; absent on NHL 07) | bulk changes: NHL 12 → 13 2 → 1 (860 players), NHL 15 → Legacy and PS4 15 → 16 1 / 3 / 4 → 2 (≈ 1 640), PS4 16 → 17 2 → 0 (1 912), real NHL 20 → 21 0 → 2 (3 901); ≥ 95 % unchanged elsewhere | re-population or remapping — continuity cannot tell; labels per title **not established** |
| skin colour (`skincolor` 4 b) | NHL 07 → 08 3 → 1 (134 players; 46 % unchanged); Legacy / PS4 16 add codes 9 / 10; ≥ 91 % unchanged elsewhere | NHL 07 → 08 shift unexplained; otherwise stable |
| goalie style (`g_style` / `playerstyle` 2 b) | 74–100 % unchanged, no systematic move | no renumbering signal |

> **Correction 2026-09-27 04:48 ([00d](00d-checkpoint-r119-r121-review.md) §1–3):** R122 ([handoff 15](15-native-label-consumers.md)) inspected the actual consumers: the companion style field shows NHL 07's labels on NHL 07–09, a number on NHL 10–13 and `player-style.toml` from NHL 14; the roster table's `eGlu.QgFs` and the Lines roster rail use the Legacy labels on every title. The sentence "`player-style.toml` may mislabel NHL 07–10" is replaced by handoff 15's measured split. R121's JSON was regenerated with deterministic rankings (00d §4; counts unchanged).

Developer rows: **D6** skater style — before trusting labels on NHL 07–10, find a title-owned or in-game label check (e.g. one known player
per style per title); regression: none until then (keep raw numbers visible). **D7** eye colour — per-title labels unverified; same next step.
Evidence class for D6 / D7: continuity only (supporting, not proof).
