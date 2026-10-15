# Phase 3 (Players) coverage — opened 22 September 2026

Developer-verified only; nothing here is Codex-accepted or game-verified. Same vocabulary as the
Phase 1 / Phase 2 matrices (impl+tested / impl, test missing / not impl / research unknown /
blocked / n/a by evidence). This file records the Phase 3 rows the R3 campaign touched; the rest of
Phase 3 (identity / joins, ratings, biographical text, equipment references, audio) keeps its
existing surfaces and is not restated here — see `docs/STATE-OF-THE-REPO.md` and the
[entity roadmap](../plans/entity-editing-parity-roadmap.md) Phase 3 table.

| Row | X360 07 / 08, PS3 08 | X360 09 | X360 10–15 / Legacy, PS3 11–15 / Legacy | PS4 15 | PS4 16–21 | Recomp | Receipt |
|---|---|---|---|---|---|---|---|
| 3.4 portrait / head containment (roster-level cross-tab; per-player file evidence) | reported **not measured** (contract) | **impl+tested, dev** — `head_<id>` naming; R53 reproduced (256 bespoke, 251 / 251 files, 214 / 129 unused, the five "shipped without asset" names) | **impl+tested, dev** — R53 reproduced on X360 13 / Legacy and PS3 Legacy (`.rpsgl`); 10 / 11 / 12 / 14 / 15 same contract, real trees not run | impl, dev — `assets/portraits/p<id>.png` rule (seam-tested on a synthetic tree); real tree not run | impl, dev — `kettle_assets/playerportraits/p<id>.gnf` rule (seam-tested); heads stated as vault data, not measured; real trees not run | by identity (Legacy) | 14 |
| 3.4 `hasportrait` shown as stored, cross-tabulated, never derived / written | column absent on 07 (reported) | impl+tested, dev (2 705 / 0 / 1 / 1 262) | impl+tested, dev (13: 93 clear-with-file; Legacy: 2 flagged-without-file, 127 clear-with-file) | impl, dev | impl, dev (R63: 0 flagged-without-file expected; not reproduced here) | by identity | 14 |
| 3.4 pool-head floor per file (10 000 on 09–13, 60 000 on 14+) | — | impl+tested (receipt 06) | impl+tested (receipt 06) | impl+tested | impl+tested | by identity | 06 |
| 3.4 portrait / head **reassignment** (existing shipped id) | not impl | not impl — the unused shipped pools are counted as the target set; no writer, no validation path | not impl | not impl | not impl | not impl | 14 |
| 3.4 new portrait / head (new archive / container) | not impl | not impl (portrait = BIGF + RefPack + `RW4xb2` / XPR2 per R53 §4; head = the external mesh work-stream) | not impl | not impl | not impl | not impl | — |

## 3.1 identity data — 25 September 2026 (receipt 11, developer-verified)

Sweep `crates/studio-api/tests/player_identity.rs` (27 anchors) and `crates/studio-domain/tests/weight_base.rs`.

| Row | X360 07 / PS3 07 (research) | X360 08–10, PS3 08–10 | X360 11–15 / Legacy, PS3 11–15 / Legacy | PS4 15 | PS4 16–21 | Recomp | Receipt |
|---|---|---|---|---|---|---|---|
| 3.1 identity columns present (first / last name, birth city / state, nationality, birth y / m / d, position, handedness, height, weight) | all 12 — impl+tested | all 12 — impl+tested | all 12 — impl+tested | all 12 — impl+tested | all 12 — impl+tested | by identity | 11 |
| 3.1 encodings probed from data (weight base; birth month / day index; height +54) | 150; 1-indexed; +54 — impl+tested | 150; 1-indexed (NHL 10 incl.); +54 — impl+tested | 120; 0-indexed; +54 — impl+tested | 120 (**was 150, fixed**); 0-indexed — impl+tested | 120 (**was 150, fixed**); 0-indexed — impl+tested | by identity | 11 |
| 3.1 shipped name / city strings | UTF-8 (no other code page) — measured | UTF-8 — measured | UTF-8 — measured | UTF-8 — measured | UTF-8 — measured | by identity | 11 |
| 3.1 edit → header → validate → save / reopen → undo (name, height, weight, birth day, handedness, nationality, position) | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | by identity | 11 |
| 3.1 refusals as text (too-long name names field + width; out-of-range height names the range) | impl+tested | impl+tested | impl+tested | impl+tested | impl+tested | by identity | 11 |
| 3.1 runtime agreement with game display (roadmap acceptance) | not measured (card V-55 / desktop; console open) | not measured | not measured | not measured | not measured | — | 11 |

## 3.2 additional database entries — 25 September 2026 (receipt 12, developer-verified; opened)

| Row | X360 07 / PS3 07 (research) | X360 08–10, PS3 08–10 | X360 11–13, PS3 11–13 | X360 14–Legacy, PS3 14–Legacy | PS4 15–21 | Recomp | Receipt |
|---|---|---|---|---|---|---|---|
| 3.2 roster-list Overall column (era rule = Attributes tab) | none shown (no overall field; formula uncalibrated) — impl+tested | none shown — impl+tested (NHL 09) | NHL 13: stored `p_overall`; 11 / 12: none — impl+tested | derived (calibrated fit) — impl+tested (NHL 15) | derived — impl+tested (NHL 17) | by identity | 12 |
| 3.2 rating scale (display = raw + offset) | +50 — probed from the distribution, **provisional** | +50 — probed, provisional (NHL 08 corrected from 36 by receipt 13) | +36 — probed (in-game verified 14 / 15 / Legacy by the earlier program; 11–13 provisional) | same | 0 (metadata 0..99) — impl+tested | 0 — impl+tested | by identity | 13 |
| 3.2 rating / style / potential edit → tab → validate → save / reopen → undo (skater + goalie) | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | by identity | 13 |
| 3.2 refusal by range (rating above the field's range) | impl+tested | impl+tested | impl+tested | impl+tested | impl+tested | impl+tested | by identity | 13 |
| 3.2 historical import (synthetic person → cluster; every probed encoding through the import writer; save / reopen) | refused in words (no slot key) — impl+tested | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | impl+tested, dev | by identity | 14 |
| 3.2 move / add membership into a full roster block refuses in words (team named; nothing moves; no undo entry) | impl+tested | impl+tested | impl+tested | impl+tested | impl+tested (abbreviation fallback without a dictionary) | impl+tested | by identity | 15 |
| 3.2 cap hit in dollars (unit measured against public cap hits) | ×$25 000 — impl+tested (Crosby $850 000) | ×$25 000 — impl+tested (Crosby $8 700 000 on 09 / 10; NHL 08 by unit inference) | ×$5 000 — impl+tested | ×$5 000 — impl+tested | ×$5 000 (14-bit confirmed: McDavid $12 500 000) — impl+tested | ×$5 000 — impl+tested | by identity | 17 |

## 3.3 localization — 25 September 2026 (receipt 18, developer-verified, read-only inventory)

| Row | X360 07 / PS3 07 (research) | X360 08–10, PS3 08–10 | X360 11–Legacy, PS3 11–Legacy | PS4 15–16 | PS4 17–21 | Recomp | Receipt |
|---|---|---|---|---|---|---|---|
| 3.3 names / birth city are inline biography strings (UTF-8, no dictionary keys) | measured (160 B) | measured (40 B; `statename` empty) | measured (40 → 25 B) | measured (+ nickname) | measured; **8 template rows `$BURNABY`** (dictionary-resolvable) | by identity | 18 |
| 3.3 audio-name reference column | `audioid` + `defaultaudioid` (14 bits) | `audioid` 14 bits | `audioid` 14 → 16 bits | `audioid` 16 bits | `audioid` 16 bits; NHL 21 `surnameid` | by identity | 18 |

## 3.1 / 3.2 encoding contract — 25 September 2026 (receipt 20, developer-verified; corrects the "probed from data" rows above)

| Item | X360 07 / PS3 07 (research) | X360 08–10, PS3 08–10 | X360 11–Legacy, PS3 11–Legacy | PS4 15–21 | Recomp | Receipt |
|---|---|---|---|---|---|---|
| encoding source | schema family (`yeardrafted` 6-bit) — impl+tested | schema family (`yeardrafted` 7 / 8-bit; `helmetbrand` 4 / 8-bit) — impl+tested | schema family (`draftyear`) — impl+tested | schema family (little-endian) — impl+tested; real NHL 20 fixture witness (`--ignored`) | by identity | 20 |
| weight base | 150 (in game) | 150 (measured) | 120 (measured) | 120 (measured) | by identity | 20 |
| six-bit rating scale | +50 (in game) | +50 (**inferred**, provisional) | +36 (in game 14 / 15 / Legacy) | stored 0..99 | by identity | 20 |
| birth month / day | 1-indexed | 1-indexed | 0-indexed | 0-indexed | by identity | 20 |
| cap-hit unit (independent public contracts matched) | ×$25 000 (6) | ×$25 000 (NHL 08: 7, NHL 09: 8, NHL 10: 11) | ×$5 000 (25) | ×$5 000 (24) | by identity | 20 |
| bulk edit across the old thresholds keeps the contract (display, edit form, re-applied value, undo / redo, save → reopen) | not run | impl+tested (X360 09) | impl+tested (PS3 15, X360 14, PS3 12) | impl+tested (PS4 17) | — | 20 |
| declared-title / schema conflict refuses encoded writes in words; unrelated cells still write | impl+tested (PS3 15 opened as nhl09) | | | | | 20 |
