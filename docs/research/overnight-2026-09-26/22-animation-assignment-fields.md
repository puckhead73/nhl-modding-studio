# Handoff R129 (P25) — Players 3.2 animation / stance / celebration assignments: the fields exist only on PS4, grow title by title, carry no title-owned labels, and NHL 21 reshapes `skatingstyle`

Research lane, 27 September 2026, published 05:38 MDT (system clock). Roadmap 3.2 ("supported animation / stance / celebration
assignments"). Claim `research-p25-animation-assignment-fields`. Read-only census over the 19 R113 sources (genuine NHL 20 included; the
committed `anchors/ps4/nhl20` = NHL 19 base copy, aliased); PS4 vault schemas searched for enum names. Nothing written.

Machine-readable: [`data/r129-animation-assignment-fields.json`](data/r129-animation-assignment-fields.json).

## Plain-language summary

None of the Xbox 360 / PS3 databases stores a player's celebration, stance or skating style. On PS4 these arrive one title at a time: a
goalie stance number from NHL 16 (used from NHL 17), a skater goal-celebration number from NHL 17 (used from NHL 18), a skating-style
number, and on NHL 19 onward some World-of-Chel emote and animation-pack numbers that every stock player leaves at 0. The games ship no
list of names for these numbers that research could find, so Studio can only show and edit them as numbers within their declared range.
One trap: NHL 21 turned the single skating-style number into a small 3-bit field plus twenty further fields, so a value cannot be carried
between NHL 20 and NHL 21 as if it were the same field.

## 1. Fields per title (skater / goalie AI and equipment tables; stored values)

| Field | PS4 15 | 16 | 17 | 18 | 19 (base, patch 56) | 20 (genuine) | 21 |
|---|---|---|---|---|---|---|---|
| goalie `stance` | — | 4 b, all 0 | 7 b, 41 values 0–40 | 7 b, 41 values 0–40 | same | 7 b, 40 values 0–40 | 7 b, **52 values 0–58** |
| skater `goalcelebration` | — | — | 8 b, all 0 | 8 b, 35 values 0–126 | 44 / 53 values 0–132 | 59 values 0–134 | 61 values 0–134 |
| skater `skatingstyle` | — | — | 8 b, all 0 | 8 b, all 0 | 8 b, values 2–4 | 8 b, values 2–4 | **3 b, values 1–3, plus `skatingstyle0 … skatingstyle19` (7 b each)** |
| skater `animpack` | — | — | — | — | 8 b, all 0 | all 0 | all 0 |
| resort emotes (`emotegoal`, `emotebighit`, `emotebigsave`, `emotegoalagainst`) | — | — | — | — | 12 b, all 0 | all 0 | all 0 |
| `introanimid` (resort player extras) | — | — | — | — | — | 8 b, values 82 / 130 | same |
| `mouthguardanim` (skater equipment) | — | — | — | 1 b, all 0 | all 0 | all 0 | all 0 |

Gen7 (X360 / PS3 NHL 07 → Legacy): no field of this family in any `exhibition*` table. Team-level intro / signature effects (NHL 17 on
`exhibitionarena`, NHL 18+ on `exhibitionteams`) are presentation fields (2026-09-21 pm handoff 09) — note one width drift:
`signatureintroeffect2id` is 4 b on the genuine NHL 20 and 8 b on the other titles. Player `introsongid` is covered by R54 (not a
song-catalogue id).

## 2. Labels and consumers

- **No label source found:** the PS4 vault schemas (`gcschema`, NHL 15–21) name no celebration / stance / skating-style enum; the typed
  equipment-style join used for gear (R16) does not exist for these fields. Not searched: the PS4 localisation dictionaries inside the
  archives and the executables.
- **Developer use:** numeric display / edit within the declared width, neutral labels, no cross-title transfer of `skatingstyle` between
  NHL ≤ 20 and NHL 21; values 0 everywhere on the emote / `animpack` fields are stock facts, not a statement that 0 means "none".
- Which animation the game plays for a value is runtime evidence (a card would compare two stock values on a copy).
