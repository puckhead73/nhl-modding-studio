# Handoff R48 — PS4 play-by-play, typed evidence: the events carry integer `player_ID` (NHL 15–19) / `surname_ID` (NHL 20–21) parameters, the name phrases consume them, and their sample pools are the keyword-tagged name-call samples; the DB column that supplies the value is still not identified (one discriminating test, negative); NHL 20's shipped keyword table is a 2-keyword stub

Research lane, 21 September 2026 pm, roadmap step 3.5 PS4 (prompt step 4: "for PS4 players, investigate typed event parameters and static
consumer references; no further plain set-overlap argument as proof of a binding"). Claim `research-r48-ps4-typed-params`. Read-only over the
extracted PS4 trees (`audio/speech/eng`, seven titles), the anchors / install DBs and one executable string scan; nothing game-verified; no
writer.

Machine-readable: [`data/r48-ps4-speech-event-params.json`](data/r48-ps4-speech-event-params.json) (per title: parameter table with types,
event usage, phrase-item usage, every name-carrying phrase with its sample group and keyword coverage). Probe
`r48_ps4_speech_event_params_and_name_phrases.py` (≈ 6 min; reads the 2026-09-20 `pbp_sentences.py` / `snd10_sbr.py` / `r14_audio_keywords.py`
readers).

## Plain-language summary

The PS4 commentary's event definitions are now parsed with their parameter *types* and *names*. Every title declares a plain-integer
parameter for the player being named — called `player_ID` on NHL 15–19 (778–1 468 events use it) and `surname_ID` on NHL 20–21 (158 /
262 events; `player_ID` survives on one event) — next to `team_ID`, `city_ID`, `voiceID` and, on 21, `gender_*` enums for pronouns. In the
sentence files, the phrases that actually *say* a name reference exactly those parameters (41–57 phrases on 16–19, 5 / 15 on 20 / 21), and
their sample groups are the keyword-tagged name-call pools: on NHL 21 all 4 709 samples of the three `surname_ID` groups carry a keyword, 2 465
distinct keywords, only 29 of them team / arena keywords. So the runtime value put into `surname_ID` / `player_ID` is matched against the
keyword table, exactly as the established team binding works. What remains open is *where that value comes from*: the one test tonight
that could discriminate (players whose `audioid` is a small number — do they land in the surname pools' keywords more than chance?) says
**no** (381 of 906 against 416 expected by chance; Stastny / Staal / Letang / Kessel are outside), the surname-id and offset hypotheses
remain at chance, and the executable carries no surname string table. The developer's refusal stands; the next discriminator is a code-level
one. Two per-title facts fall out: NHL 15's sentence file (version 4) carries no phrase parameters at all, and the NHL 20 install
ships a **2-keyword** `pbp_keywords.bin` (744 bytes; the only copy in its archives), so on that install no name-call — team or player — can be
keyword-driven the way it is on 16–19 / 21.

## 1. `0TVE` layout (little-endian 'KETT' form of gen7 `EVT0`; parsed on all 7 titles, 0 odd records)

```
'0TVETTEK' | u32 2 | u32 namespace | u32 n events | u32 g params | u32 0x28 | u32 0
0x20  u64 → g × u64 → parameter record: u32 type (3 = integer, 1 / 2 = enum) | u32 id | u32 enum count | u32 0 ×3 | u64 enum table ptr | name
0x28  n × u64 → event record (96-byte fixed part): u64 size (= 4 × count) | u16 count | u16 flag (0 / 1) | zeros | u16 ns, u16 id @+0x28 | zeros | count × u32 param id @+0x60 | name
```

| Title | events | params | player parameter (type, events using it) | `team_ID` events | keyword table | sentences named as events / phrases / items |
|---|---|---|---|---|---|---|
| NHL 15 | 3 714 | 109 | `player_ID` (int, 1 222) | 1 557 | 2 121 keywords | 434 / 976 / **0** (`0CPS` v4: inline 16-byte phrase records, no parameter items — §5) |
| NHL 16 | 4 187 | 102 | `player_ID` (int, 1 442) | 1 701 | 3 052 | 520 / 1 105 / 255 |
| NHL 17 | 4 953 | 104 | `player_ID` (int, 1 468) | 1 751 | 3 111 | 803 / 1 699 / 219 |
| NHL 18 | 2 583 | 71 | `player_ID` (int, 778) | 828 | 3 195 | 685 / 2 049 / 241 |
| NHL 19 | 2 897 | 72 | `player_ID` (int, 826) | 877 | 3 195 | 1 734 / 5 332 / 5 860 |
| NHL 20 (install) | 1 415 | 20 | **`surname_ID`** (int, 158); `player_ID` 1 | 286 | **2 keywords (744 B stub; `inxp_40.big` holds the only copy)** | 349 / 699 / 410 |
| NHL 21 | 2 147 | 29 | **`surname_ID`** (int, 262); `player_ID` 1 (`GOAL_PLAYER_SPECIFIC`) | 362 | 5 363 | 1 417 / 3 071 / 2 849 |

NHL 20 / 21 also declare `city_ID` (249 / 320 events), `voiceID` (2), `custom_goal_call`, `line_nickname` and the `gender*` enums (20: one
`gender`; 21: `gender_primary / secondary / goalie / goalie_secondary`, 5 values each); 16–19 declare `team_home / team_away`,
`AI_team_ID`, `AI_goalie_ID`, `rivalry_ID` and 60–80 gameplay enums. The parameter *id* is stable across titles (`team_ID` 1, `player_ID` 2).

## 2. Which parameters the phrases consume (phrase items = `u8 i | u8 i+1 | u8 i | 0 | 1 | ff | 0 0 | u32 tag`, `i` = index into the event's parameter list)

NHL 21 (2 849 items): `get_sample_tuning` 1 515, `gender_primary` 889, `gender_goalie` 206, `gender_secondary` 159, **`surname_ID` 15**,
`pickup_id` 11, `period` 9, …, `team_ID` 3, `team_away_ID` 3, `team_home_ID` 3, `custom_goal_call` 2, `line_nickname` 2. NHL 16–19: `player_ID`
41 / 48 / 20 / 57, `team_ID` 6 / 9 / 18 / 38, then the `language_BIT / drill_BIT / platform_BIT` selectors. The 32-bit item tag differs per
(event, parameter) — 1 423 distinct tags for `get_sample_tuning` on 21, 3 for `surname_ID` — so it is not a parameter id; its namespace stays open.

## 3. Name-phrase sample pools vs the keyword table (per title, union over the phrases' DSETs)

| Title | parameter | phrases | DSETs | samples | keyword-tagged | distinct keywords per DSET | team / arena keywords among them |
|---|---|---|---|---|---|---|---|
| NHL 16 | `player_ID` | 41 | 10 | 24 864 | 24 857 | 30 … 1 080 | 663 |
| | `team_ID` | 6 | 5 | 4 620 | 4 563 | 0 … 553 | 22 |
| NHL 17 | `player_ID` | 48 | 11 | 21 636 | 19 030 | 30 … 1 080 | 532 |
| NHL 18 | `player_ID` | 20 | 9 | 23 723 | 20 770 | 32 … 1 112 | 594 |
| NHL 19 | `player_ID` | 57 | 13 | 25 085 | 20 334 | 0 … 1 112 | 642 |
| | `team_ID` | 38 | 30 | 7 273 | 5 827 | 0 … 574 | 55 |
| NHL 20 | `surname_ID` | 5 | 2 | 2 120 | **0** | 0 | 0 |
| | `team_ID` | 2 | 2 | 632 | **0** | 0 | 0 |
| NHL 21 | `surname_ID` | 15 | 3 (`0x21c8d7aa` 2 697, `0x1ca509d1` 1 943, `0x39b091b2` 69 samples — all in `pbp_samples_inxp.sbr`) | 4 709 | **4 709** | 2 453 / 1 814 / 49 | 14 / 13 / 2 |
| | `team_ID`, `team_home_ID`, `team_away_ID` | 3 + 3 + 3 | 3 + 1 + 1 | 239 + 31 + 31 | 189 + 31 + 31 | 31–38 | 21–28 |

Reading: the sample groups of the name phrases are (near-)completely keyword-tagged with hundreds to thousands of distinct keywords, and the
team phrases' groups are tagged by the DB-bound team keywords (2026-09-20 handoff 07 §3). This is *typed* evidence that the value of
`player_ID` / `surname_ID` at runtime is a keyword id — the same mechanism as teams — not a set-overlap argument about a DB column. On
NHL 16–19 the `player_ID` pools also carry 532–663 team / arena keywords (the pools mix player and team name-calls); on 21 the surname
pools are almost purely non-team.

## 4. Where the value comes from — tested tonight, all negative (population NHL 21 unless stated)

| Test | Result |
|---|---|
| `exhibitionplayerbiotable.audioid ≤ 5 363` players (906) ∈ the surname-pool keyword set (2 465 of 5 363) | **381 vs 416 expected by chance**; NHL-league players 84 in / 138 out; Stastny 1292, Staal 1297, Letang 1298, Kessel 1309 … outside — `audioid` is not the surname keyword (a discriminating test, unlike H1's dense membership) |
| `playernames.surnameid` (13 848 distinct on surname rows; 5 298 ≤ 5 363) of rows with `audioid` (2 006) vs without | 929 / 2 006 vs 4 515 / 12 023 in the keyword set = exactly the `≤ 5 363` fraction — chance |
| `playernames.audioid + K` (2 006 rows, 30 004–34 796) and `bio.audioid + K` into the surname-pool keyword set, K ∈ [−35 000, 0] | best 1 242 vs 922 expected (plateau of contiguous runs), 1 436 vs 3 320 — no offset |
| surname strings in `eboot.bin` (104.8 MB): `Stastny`, `Kessel`, `Crosby`, `Ovechkin` (three casings) | none as a name table (the `Crosby` / `Ovechkin` hits are animation / neural-net model identifiers) |
| name text or hash in the bank DSET records | none: the name-call DSET fields are `.DUR .OFF .SBS .SID` only |

So the keyword id for a player is produced by code from something not observed statically — candidates (untested): a table inside another
data file not yet searched, or a hash / index computed at roster load. `bio.surnameid → playernames.surnameid` resolves 7 171 / 7 247 (not
the row index: `playernames.surnameid` is a surname identity 1–14 052, `index` is the row), which is the join the display should use for
the name text; it is not a keyword id.

## 5. Consequences (developer)

- **PS4 player-name listing / reassignment stays "no established binding"** (queue row 14 of 2026-09-21 unchanged); the typed chain above
  can be shown as "the commentary names players through the `surname_ID` (20–21) / `player_ID` (15–19) parameter against the keyword pools" —
  informative, not editable.
- **NHL 20 (install, unpatched):** the keyword table is a 2-keyword stub and no name-phrase sample is tagged → the *team* name-call listing
  of P3 (keyword-driven) has no data on this install; list "no keyword table shipped (2 keywords)" rather than empty; do not infer NHL 20
  from NHL 19 / 21. Whether a patch adds the table is unknown (the user's install has none).
- **NHL 15:** its `0CPS` sentence file is **version 4** (21: version 5): the phrase list is an inline array of 16-byte records `u32 type │
  u32 dsetKey │ u32 historyKey │ u32 0` (the 2026-09-20 parser's u64 "phrase pointers" are in fact these consecutive records) with **no
  parameter items at all** — the phrase → event-parameter binding is not expressed in NHL 15's data (measured on `GOAL_SCORED_4_UNANSWERED`,
  `GOAL_SCORED_IN_OFF_SKATE`; closes Q-R48-1 at the data level: refuse the name-phrase view on 15 as "no phrase parameters in this file
  version", not as unparsed).
- Card **R40-AUD-1 rev. 2** (USER-CARDS) is the runtime discriminator; its treatment P can now name the *typed* expectation: if the
  commentary's player name changes with the edited `audioid`, the code maps `audioid → keyword`; if not, the mapping is elsewhere.

## 6. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r48_ps4_speech_event_params_and_name_phrases.py docs/research/overnight-2026-09-21-pm/data/r48-ps4-speech-event-params.json   (≈ 6 min)
```
The §4 tests were shell scratch over the same readers (logic stated in the table; not retained); the NHL 20 keyword-member search used
`eb_list.py --grep pbp_keywords` over the 13 install archives (only `inxp_40.big` entry 18, 744 B; the `gauntlet` / `threes` variants are other
modes). Executed once (final) after two development runs (a sentence-file bounds fix).
