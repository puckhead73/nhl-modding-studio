# Handoff R46 — `CTX0` record header words identified across the two file versions: the v3.1 `w0 / w1 / w2` are the v4.5 `w1 / w4.hi / w4.lo`; candidate labels (priority, window, kind) from value structure and the module XML — no runtime claim

Research lane, 21 September 2026 pm, roadmap step 1.5 / 2.7 / 3.5 gen7 (prompt step 4: "investigate header / window fields against XML
and crowd siblings"; the 2026-09-21 closing receipt's next-task item 2). Claim `research-r46-ctx0-header-words`. Read-only over the loose
`audio/context` files of X360 NHL 11–Legacy and PS3 NHL 11 / 14 / 15 / Legacy; nothing game-verified; no writer. The bytecode stays out of
scope.

Machine-readable: [`data/r46-ctx0-header-words-census.json`](data/r46-ctx0-header-words-census.json). Probe
`r46_ctx0_header_words_census.py` (≈ 40 s; imports the r41 and r24 parsers).

## Plain-language summary

Both context-file versions carry three undecoded numbers per record. Laying the NHL 11 / 12 records next to the NHL 13–Legacy records
shows they are the same three numbers in different slots — the values, their spread and how they pair with "has a trigger" match exactly,
and the crowd file (identical records on every title from NHL 14 on) makes the identification word-for-word. The first is a priority-like
number (100–800 for ordinary lines, 2 000 for cup presentations, 5 001 for injury stories, 9 999 at the top); the second is a time-like
number (0, 10, 30, 60, 120, 180, 240, 300 … up to 6 000); the third is a small kind code: 1 for every record that has a trigger, and other
codes for the trigger-less "follow", "passive" and "output" records. The module XML names a priority decay rate on NHL 11 / 12 and
"windows" / an evaluation time on 13+, which is consistent with these readings but does not fix the units. The developer's listing can
label the three words *priority (candidate)*, *window (candidate, unit unknown)* and *kind* with the observed code table; nothing about
selection at runtime follows from this.

## 1. Identification (values per set; v3.1 = NHL 11 / 12 by the r41 parser, v4.5 = NHL 13–Legacy by the r24 parser)

| Concept | v3.1 word | v4.5 word | `pbp_eng` values (NHL 11 · NHL 15) | `crowds` values (every title) | Cross-tab |
|---|---|---|---|---|---|
| **A** priority-like | `w0` | `w1` | 1–6 000 (173 distinct; top 100 / 120 / 200 / 400 / 300 / 101 / 140 / 800) · 10–9 999 (224 distinct; top 400 / 10 / 100 / 200 / 120 / 150 / 101 / 500) | 100 / 200 / 300 / 500 / 600 / 250 / 120 / 400 (129 distinct) | cup presentations 2 000 / 2 005, injury stories 5 001, dynamic intros 995–997; `+1` variants (101, 401) on a subset |
| **B** time-like | `w1` | `w4` high u16 | 0–600 (19 distinct; 60 / 300 / 30 / 180 / 120 / 240 / 0 / 10) · 0–6 000 (32 distinct; same top values + 200 / 600) | 10 (1 062) / 0 (87) / 30 (15) | 0 on every trigger-less record of kinds 2 / 3 (v3.1) |
| **C** kind | `w2` | `w4` low u16 | 1 = 2 752 (2 699 with a trigger, 53 without), 2 = 40, 3 = 41 (no trigger) · 1 = 3 815 (all with a trigger), 20 = 35, 25 = 9, 28 = 7 (no trigger) | 1 = 1 118 (1 078 with a trigger), 4 = 46 (no trigger; all `c_output` items) | kind ≠ 1 ⇒ no trigger on every set |
| flag | — (`w3` = `w4` = 0 always) | `w3` = `0x01000000` on 126 / 128 / 203 / 289 records (13 / 14 / 15 / Legacy) | cup / playoff "won" and game-over records (`Playoff_End_Stanley_Cup_Won_OT`, `Gameover_Won_Stanley_Cup` …) | 0 | no v3.1 counterpart |
| constants | — | `w2` byte 3 = 100 on every record; byte 2 = 1 on 22 Legacy records (the slot-w11 block carriers) | | | unchanged from handoff 01 |

Pairs (A, B) recur identically across versions — (120, 180), (100, 60), (800, 240), (300, 300), (400, 60) are the top pairs on NHL 11 and
NHL 13–Legacy alike — and the `crowds` records are **equal in (name, A, B, C) on NHL 14 / 15 / Legacy, both platforms (1 164 / 1 164)**;
NHL 11 / 12 / 13 crowds differ from Legacy on 1 071–1 076 records (value changes, same names), so the field identity is fixed by the
pbp distributions and the 88–93 equal crowd records, not by a byte-identical file.

Kind codes by example: v3.1 `2` = `*_Follow` contexts (`Cup_End_Special_Stanley_Team_Photo_Follow`, `Acmem_Follow_*`), `3` = `NONIS` /
`DYN_*` intro contexts; v4.5 `20` = follow / fight / save "C" contexts, `25` = `Allow_Passives_Dynasty`, `PP_Setup_C_*`, `Time_Last_Minute_*`,
`28` = further `PP_Setup_C` / `PK_SETUP` / `Time_Last_Minute` contexts; crowds `4` = the 46 `XCITE_*` / `ANTICIPATION_*` output contexts.
The v3.1 `1`-without-trigger records (53) are the `*_C` / `*_2man` save and setup contexts that carry kind 20 / 25 / 28 on 15+.

## 2. Module XML attributes (per title; `pbp_eng` / `crowds`)

| Titles | Attributes |
|---|---|
| NHL 11 / 12 (v3.1) | `maxNumContexts` 2 800 / 1 200, `maxNumGroups` 120 / 100, `maxNumValidContexts` 500, `maxNumTriggeredContexts` 300, `numPassives` 200 / 100, **`priorityDecayRate` 1 000 000**, `maxContextsToEvaluate` 500 |
| NHL 13–Legacy (v4.5) | `maxNumValidContexts` 1 000 (crowds 500 on PS3 14 / 15 / Legacy, 1 000 on X360), `maxNumTriggeredContexts` 500 / 300, `numPassives` 200 / 100, **`maxContextEvaluationTime` 30 / 3**, **`maxNumWindows` 25** |

Consistency (not proof): the NHL 11 XML names a *priority* with a decay rate, and word A ranges like a priority; 13+ names *windows* and an
evaluation time, and word B is time-like. NHL 11's 2 833 / NHL 12's 3 339 `pbp_eng` records exceed `maxNumContexts` 2 800 — so that
attribute is not a hard record cap of the file (or counts something else); recorded, not explained. `numPassives` 200 vs the 81 / 51
trigger-less records: not a count match either.

## 3. What the listing may say (contract rev. 2c input)

Show per record: **priority (candidate)** = A; **window (candidate; unit not established — 10 / 30 / 60 / 300 read like seconds but no
observation ties them to a clock)** = B; **kind** = C with the code table above (1 triggered; 2 / 20 follow; 3 / 25 / 28 passive-type
(candidate); crowds 4 output); the v4.5 `w3` flag as "won / game-over flag (candidate)"; everything else raw. Do not order or filter by A
as if it were the engine's choice.

## 4. Not claimed

Units of B; whether A is compared across triggered contexts or decayed; what kinds 25 vs 28 distinguish; any runtime selection rule
(Q-R20-1 / Q-R24-1 remain open on the semantics); the extra words and the Legacy slot-w11 block (unchanged).

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r46_ctx0_header_words_census.py docs/research/overnight-2026-09-21-pm/data/r46-ctx0-header-words-census.json   (≈ 40 s)
```
Executed twice (the crowd identity test was corrected between runs to ignore the v3.1 trees the v4.5 parser refuses). Inputs: the
`audio/context/{pbp_eng,crowds}_contexts.bin` + `_contextmoduledef.xml` of the r41 / r24 tree tables.
