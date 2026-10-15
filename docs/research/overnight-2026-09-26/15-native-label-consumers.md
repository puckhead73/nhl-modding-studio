# Handoff R122 (P18) — native style and country labels: what each Studio consumer attaches per title, and the title-owned evidence that constrains it

Research lane, 27 September 2026, published 04:54 MDT (system clock). Prompt
[`research-20260926-after-receipt3.md`](../../plans/research-20260926-after-receipt3.md) package 2. Claim `research-p18-native-label-consumers`.
Read-only over the anchors, their dictionaries and the private fixture copies; Studio source and `defs/` read at the current working tree,
not edited, not executed.

Machine-readable: [`data/r122-style-consumer-scope.json`](data/r122-style-consumer-scope.json) (per source: every style / nationality /
birth-state field, Studio id, width, values, the vocabulary route of the current rules),
[`data/r122b-roster-style-agreement.json`](data/r122b-roster-style-agreement.json) (roster `eGlu.QgFs` vs companion style per player),
[`data/r122c-style-position-split.json`](data/r122c-style-position-split.json) (style code × position),
[`data/r122d-style-transition-matrix.json`](data/r122d-style-transition-matrix.json) (full transition matrices NHL 08→09, 09→10, 10→11,
13→14 and each title's dictionary style-label set).

## Plain-language summary

Skater styles are numbered differently in three eras. NHL 07–09 have 7 styles (2 for defencemen), NHL 10 has 9 (3 for defencemen) and NHL 11
onward have 10 (4 for defencemen); the games' own text files list exactly that many style names per era, and the players' positions split the
numbers the same way. Studio shows the NHL 07 names on the player's style field for NHL 07–09 (which fits), a plain number on NHL 10–13, and
the NHL Legacy names from NHL 14 on. But the team Lines page and the roster table's style cell use the NHL Legacy names on **every** title. On
NHL 08 / 09 the same player's style would therefore read, say, "Grinder" on the player page and "Enforcer Defenseman" on the Lines page, and
on NHL 10 about a thousand forwards would be labelled "Two-Way Defenseman". That is a developer observation, measured from the data and the
source code, not seen on screen. For countries, the NHL 07 table needs nothing new except the single inferred Belgian entry, which a
birthplace (Antwerpen) supports; four small NHL 08 / 09 codes remain candidates only.

## 1. Sources and method

- **Databases:** the 19 distinct sources of R120 (hashes in each JSON); PS3 copies alias their Xbox 360 twins; the committed
  `anchors/ps4/nhl20` is reported as the NHL 19 base copy; the genuine NHL 20 is the private fixture `nhl20-ps4-cusa15439`.
- **Dictionaries (title-owned text):** `anchors/xbox360/{nhl08, nhl09, nhl10, nhl11, nhl13, nhl14}/databases/nhl_eng_us.db`
  (`LanguageStrings`: `stringid` key, `sourcetext`, `hashid`; rows are ordered by `hashid`), hashes in `r122d` JSON; NHL 07 read in the same
  pass (same 7 style keys as 08 / 09, not written to JSON).
- **Consumers read (current tree):** `studio-domain/src/view.rs` (`read_field`, `CompanionReader::style_label`, lines-view `style` cell),
  `attributes.rs` (`is_gen6_style_schema`, `synthetic_enum`, `restrict_style_options`, `apply_gen6_geo_vocab`,
  `apply_title_birth_state_countries`), `import.rs` (`push_semantic_int` for `QgFs`), `app/src/components/LinesTab.tsx` roster rail,
  `app/src/lib/teamPresentation.ts` (`PLAYER_STYLE_LABELS`, `GOALIE_STYLE_LABELS`), `defs/enums/player-style.toml`, `goalie-style.toml`,
  `nationality*.toml`, `birth-state.toml`.
- **Identifier note:** gen7 metadata keys are byte-reversed relative to Studio's canonical ids (`yvSd` = `dSvy`, `EoQm` = `mQoE`); the first
  R122 run compared uncanonicalised keys and was discarded before publication.

## 2. Skater style — title-owned structure

| Era (sources) | Stored field | Codes used | Defence codes (≥ 0.9 D) | Forward codes | Dictionary style keys (count) |
|---|---|---|---|---|---|
| NHL 07 / 08 / 09 | `dSvy.mQoE` `p_style` 3 b | 1–7 | 1, 2 (NHL 08 / 09: code 1 mixed, 152 / 161 forwards) | 3–7 | defensive / offensive defenseman, grinder, playmaker, sniper, power forward, dangler (7) |
| NHL 10 | `dSvy.mQoE` 4 b | 1–9 | 1 (mixed), 2, **3** | **4–9** | + enforcer defenseman, enforcer (9; both shown as "TOUGH GUY") |
| NHL 11 / 12 / 13 | `dSvy.mQoE` 4 b | 1–10 | 1 (mixed), 2–4 | 5–10 | + two-way defenseman ("2 WAY DEFENDER"), two-way forward (11 keys; dangler key still present) |
| NHL 14 → PS4 21, genuine NHL 20 | `dSvy.QgFs` `playerstyle` 4 b | 1–10 | 1–4 (1 mixed on 14 / 15 / PS4 15) | 5–10 | NHL 14 same 11 keys |

Transitions (same players, `r122d`): **NHL 09 → 10 is a clean insertion** — 1→1 (92 %), 2→2 (98 %), 3→4 (86 %), 4→5 (94 %), 5→6 (95 %),
6→7 (93 %), 7→8 (96 %); the new NHL 10 codes receive only minority flows (code 3: 16 from code 1, 1 from code 2; code 9: 58 from code 3 = Grinder, 3 from code 1, 2 from code 6). **NHL 10 → 11** re-rates heavily but keeps the
pattern 4→5, 5→6, 6→7, 7→8, 9→10 as each row's largest cell, with **8→7** (28 of 50) and many players moving into the new codes 4 and 9.
**NHL 13 → 14** (field changes `p_style` → `playerstyle`): diagonal 88–98 % on every code. NHL 08 → 09: diagonal 78–94 %.

**Candidate table (class: inferred — continuity joined to title-owned label sets and position splits; not a mapping contract):**

| Code | NHL 07–09 | NHL 10 (candidate) | NHL 11 → PS4 21 |
|---|---|---|---|
| 1 | Defensive Defenseman | Defensive Defenseman | Defensive Defenseman |
| 2 | Offensive Defenseman | Offensive Defenseman | Offensive Defenseman |
| 3 | Grinder | Enforcer Defenseman ("Tough Guy") | Enforcer Defenseman |
| 4 | Playmaker | Grinder | Two-Way Defenseman |
| 5 | Sniper | Playmaker | Grinder |
| 6 | Power Forward | Sniper | Playmaker |
| 7 | Dangler | Power Forward | Sniper |
| 8 | — | Dangler | Power Forward |
| 9 | — | Enforcer ("Tough Guy") | Two-Way Forward |
| 10 | — | — | Enforcer Forward |

Evidence per column: NHL 07 = Studio's existing in-game-verified vocabulary (`skater_style_options`, named players); NHL 08 / 09 carry it by
continuity plus the identical dictionary set and position split. NHL 11 onward = Studio's `player-style.toml` (operator-confirmed on NHL
Legacy) carried back by continuity (13→14 diagonal ≥ 88 %), identical split and dictionary set (measured on 11, 13, 14). NHL 10 = the only
assignment consistent with (a) the 09→10 insertion at codes 3 and 9, (b) the two new dictionary keys being one defence and one forward role
and (c) the defence / forward split; the 10→11 moves (4→5 Grinder, 5→6 Playmaker, 6→7 Sniper, 7→8 Power Forward, 9→10 Enforcer, Dangler 8→7
Sniper) then agree with NHL 11's labels. Still inferred: no NHL 10 in-game label was observed. Displayed wording differs from Studio's
("TOUGH GUY", "2 WAY DEFENDER"); wording is not proposed here.

**Negative scope:** dictionaries are keyed by name and ordered by hash; no numeric style index, option table or code-ordered list was found
in them. The code → key table is expected in the executable, which was not examined in this bounded package. Goalie style (0 / 1 / 2; its dictionary names were not searched) shows no renumbering signal (R121).

## 3. What the current consumers attach (static reading of the source rules; `r122` JSON)

| Consumer | NHL 07–09 | NHL 10 | NHL 11–13 | NHL 14 → PS4 21 |
|---|---|---|---|---|
| Player page style field / Attributes (companion `dSvy.mQoE` / `QgFs`, `mHuy.wJus` / `QgFs`) | NHL 07 synthetic vocabulary (gate: 3-bit `mQoE`); dropdown filtered by position 1–2 / 3–7 — **matches the measured split** | number (no binding, no synthetic vocabulary) | number | `player-style.toml` / `goalie-style.toml`; filter 1–4 / 5–10 — matches |
| Player header / list `style_label` (`CompanionReader::style_label`) | synthetic label | none | none | TOML label |
| Roster table cell `eGlu.QgFs` (lines view `style`, generic field view) | **`player-style.toml` (Legacy labels)** | **Legacy labels** | Legacy labels | Legacy labels |
| Lines roster-rail archetype dropdown (`LinesTab.tsx`, frontend constant `PLAYER_STYLE_LABELS` = Legacy 1–10; goalies `GOALIE_STYLE_LABELS`) | Legacy labels, options 1–10, no position filter | same | same | same |
| Import (`import.rs`, corpus style → companion `QgFs`) | skipped: "field absent on this title" (companion has `mQoE`) | skipped | skipped | written as a Legacy code |

Roster cell vs companion (`r122b`, skaters): equal for 98.7–98.8 % on NHL 08 / 09, 99.86–100 % on NHL 10 → PS4 21; **73 % on NHL 07**, where
roster codes 4 / 5 / 6 sit against companion 5 / 3 / 4 for 164 / 194 / 192 players (the NHL 07 roster cell's meaning is unresolved).

## 4. Developer observations (consumer-scoped; no application change made)

| # | Observation | Targets | Evidence | Smallest next action (developer / coordinator) |
|---|---|---|---|---|
| S1 | The same player's style reads differently on the player page and on the Lines page: e.g. code 3 = "Grinder" (companion, NHL 07 vocabulary) vs "Enforcer Defenseman" (roster cell / rail, Legacy). The rail also offers codes 8–10, beyond the 3-bit field's 0..7 (whether the write path refuses them was not executed) | X360 / PS3 NHL 08, 09 (and NHL 07, with the 73 % caveat) | measured agreement (98.7 %), position split, current source | Regression: an NHL 08 / 09 skater with companion code 3 shows the same label in both places; rail options limited to the field's declared range. Do not attach Legacy labels to `eGlu.QgFs` where the gen6 gate holds; on NHL 07 show the number until the roster cell's meaning is settled |
| S2 | Legacy labels on NHL 10's roster cell / rail contradict the title's own split: code 4 is used only by forwards (1 070) but reads "Two-Way Defenseman"; codes 5–9 are shifted by one against the candidate | X360 / PS3 NHL 10 | position split (title-owned), 09→10 insertion | Scope `player-style.toml` (and the rail constant) to the titles where the split and label set match (NHL 11 onward, see S3); show NHL 10 as numbers unless the coordinator accepts the inferred NHL 10 table |
| S3 | Companion style shows a number on NHL 11–13 although split, dictionary set and continuity match the Legacy vocabulary | NHL 11, 12, 13 | inferred (continuity + title-owned split / label set) | Decision for the coordinator: keep numbers (missing label, not wrong), or scope the Legacy table to NHL 11–13 with this handoff as provenance |
| S4 | Goalie roster cell ≠ goalie style for stand-up goalies from NHL 10: roster codes are only 1 / 2; `g_style` 0 goalies carry roster 1 (7–67 per source), so the rail's goalie labels would call them "Hybrid" | NHL 10 → PS4 21 | measured (`r122b` goalie cross-tables) | Observation only; roster goalie cell meaning unresolved — do not relabel |

## 5. Nationality — NHL 07 provenance reconciled, residual candidates

- NHL 07 stores 22 nationality codes: NHL 08's 21 national-team codes (NHL 07 → 08 continuity 99.6 %, no systematic move) plus **78**
  (one player, Mike Pellegrims, birthplace string "Antwerpen"). `GEN6_COUNTRIES` agrees with all 21; its user-confirmed entries are not
  re-examined. 78 = Belgium stays **inferred** — the birthplace supports it but a birthplace is not a nationality label.
- Codes 74, 85, 92, 100, 101 in `GEN6_COUNTRIES` are not used by NHL 07's nationality field (they are birth-state evidence, as the code
  comments say).
- NHL 08 / 09 codes outside the national teams (tiny): **74** Dainius Zubrus, Arturas Katulis (birthplace Elektrenai) — field view shows
  "Lithuania" via `GEN6_COUNTRIES`; **78** Pellegrims — "Belgium" (inferred entry carried); **83** Janos Vas (Dunaujvaros) and **90** Anze
  Kopitar, Robert Kristan (Jesenice) — shown as numbers. Hungarian / Slovenian are **candidates** from birthplace strings only; nothing is
  proposed for the vocabulary.
- **Next discriminating observation (optional user card, low priority):** NHL 08 in game, the nationality shown for Anze Kopitar (code 90)
  and Janos Vas (code 83).

## 6. Residuals

Eye colour / skin colour: still continuity flags (D7); the dictionaries were not searched for them in this package (bounded). NHL 07's
roster style cell (73 % agreement) is unresolved. The executable-held style / country code tables were not examined.

## 7. Addendum 05:41 — eye colour (D7), bounded dictionary check (negative)

Dictionaries searched: X360 NHL 12 (`354a1905…`), X360 Legacy (`f00926ce…`), PS4 NHL 17 (`035b3b30…`) `LanguageStrings`; the committed PS4 NHL 21
anchor has no English dictionary. Each holds an `eyecolor` / `Eye Color` key and colour words keyed by name: **all seven labels Studio's
`eye-color.toml` uses** (Blue, Light Blue, Brown, Light Brown, Hazel, Green, Yellow) are present, together with hair words (Black, Grey,
Blonde, and Dark Brown on PS4 17). The strings are shared by the hair / facial-hair / eye lists and carry no numeric index, so they neither
confirm nor contradict the code → label table; stored code sets (0–5 on Legacy / PS4 16–18, 0–6 on NHL 08–15, a single 6 on PS4 19–21)
do not separate the seven labels either. D7 stays a continuity flag; the next discriminating observation is an in-game check of one player
per code on one title (optional user card). No further static search planned.
