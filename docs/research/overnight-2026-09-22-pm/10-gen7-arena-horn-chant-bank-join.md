# Handoff R78 — gen7 arena cue ids vs the numbered `audio/sfx` banks (Teams 2.7 gen7, ten trees NHL 10–Legacy, both consoles): `goalhorn` / `periodhorn` name `<id:03>_goal.abk` / `<id:03>_period.abk` for every NHL-club id (ids 1–29 → files on every title; the non-NHL ids 30–136 ship no loose bank — 11–14 values per title, listed), `chantaudioid` is only a **partial** candidate for `<id>_chant.abk` (23–24 values per title without a file, 66–80 banks no row names), and `cityaudioid` / `arenaaudioid` are **not** sfx-bank ids (negative, all families)

> **Correction note — 2026-09-24 (coordinator review item 3; [R79](11-correction-checkpoint-r75-r78-review.md)).** "`cityaudioid` / `arenaaudioid` are not sfx-bank ids" and "match nothing" are withdrawn: the §2 table records numerical hits on every family (NHL 15: 61 / 40 and 57 / 36 rows on chant / goal) alongside 104–127 distinct values outside every family. Restated: **no established binding from these tests**; the hits are reported as coincidences of small-integer id spaces; the searched scope was the loose `audio/sfx` numbered banks only; the alternate consumer is not identified by a missing loose file. The exact-match horn listing and its residuals stand; presence stays separate from selection / playback.

Research lane, 23 September 2026 (brief item 2, Teams 2.7 gen7; Q-R9-1 gen7 side, Q-R35-1). Claim `research-r78-gen7-arena-sfx-bank-join`.
Read-only: directory listings of `audio/sfx` on the ten gen7 trees with a loose `audio` folder (X360 10 / 11 / 12 / 13 / 14 / 15 / Legacy,
PS3 11 / 14 / 15 / Legacy; NHL 09 X360 ships no numbered banks; PS3 12 / 13 keep audio in archives — not listed here) and the committed
anchors' `exhibitionarena` rows; no bank opened; nothing game-verified.

Machine-readable: [`data/r78-gen7-arena-sfx-bank-join.json`](data/r78-gen7-arena-sfx-bank-join.json) — per tree the bank id sets per
family (3- and 4-digit stems apart), per column the value set and, per family, rows with a file, values without a file, files without a
row (lists). Probe `r78_gen7_arena_sfx_bank_join.py` (seconds).

## Plain-language summary

On the older consoles the crowd chants and the two horns live as one small bank file per number in the sound-effects folder. Checked
against every arena row of every title, the goal-horn and period-horn numbers name those files exactly for the NHL clubs (and the
2026-09-18 handoff already observed the same on NHL 15); the numbers used by the European and minor-league arenas (30–136) have no
file of their own in the loose folder — they are listed as residuals, not explained. The chant number matches a chant file for most
NHL arenas but fails for a quarter of its values, so it is a candidate only. The city and arena announcer numbers match nothing in
this folder — they are speech ids, as the 2026-09-21 handoff assumed, and that question stays open.

## 1. Banks (identical on all ten trees)

`<id:03>_goal.abk` × 31 (NHL 10–13) / 32 (14–Legacy): ids 0–29 (+ 100, 194 on 14+; 100 on 10–13); `<id:03>_period.abk` the same set;
`<id:03>_chant.abk` × 73 (ids 0–29, 37–71 …) plus **45 four-digit** `<id:04>_chant.abk` (4, 31, 119, 127, 170, 210, 218, 244, 264, 275,
293, 298, …) — a second chant numbering the 3-digit set does not contain.

## 2. Join (per column, `goal` / `period` / `chant` family; rows with a file · values without a file · files no row names)

| Tree | `goalhorn` → goal | `periodhorn` → period | `chantaudioid` → chant | `cityaudioid` (best family) | `arenaaudioid` (best) |
|---|---|---|---|---|---|
| X360 NHL 10 | 108 · 14 · 2 | 108 · 14 · 2 | 111 · 24 · 78 | 119 · 47 (chant) | 87 · 48 |
| X360 / PS3 NHL 11 | 168 · 14 · 2 | 168 · 14 · 2 | 171 · 24 · 78 | 81 · 101 | 81 · 101 |
| X360 NHL 12 | 171 · 13 · 2 | 171 · 12 · 2 | 172 · 23 · 79 | 83 · 101 | 81 · 102 |
| X360 NHL 13 | 141 · 13 · 2 | 141 · 12 · 2 | 148 · 23 · 79 | 58 · 102 | 54 · 104 |
| X360 / PS3 NHL 14 | 145 · 12 · 3 | 145 · 11 · 3 | 151 · 23 · 80 | 59 · 105 | 55 · 107 |
| X360 / PS3 NHL 15 | 146 · 12 · 3 | 146 · 11 · 3 | 152 · 23 · 80 | 61 · 104 | 57 · 106 |
| X360 / PS3 Legacy | 144 · 12 · 3 | 144 · 11 · 3 | 139 · 24 · 80 | 70 · 103 | 66 · 105 |

PS3 figures equal the X360 ones on every shared title (identical anchors and bank sets). NHL 15 residuals, exactly: `goalhorn` values
without a file **30, 31, 32, 33, 34, 35, 36, 38, 41, 45, 61, 136** (the DEL / SHL / Liiga / AHL-international ids of the 2026-09-18
handoff 19 §1); files no row names **0, 100, 194**; `chantaudioid` values without a file 30, 32, 33, 89, 90, 100–117, 136; chant files
no row names: 0 and the whole 37–71 range plus every four-digit bank. `cityaudioid` (155 distinct, 1–511) and `arenaaudioid` (157,
1–701) sit mostly outside every family — negative.

## 3. Consequence (developer; not Codex-accepted)

- **Arena page, gen7 (10–Legacy, loose trees):** resolve `goalhorn` → `audio/sfx/<id:03>_goal.abk` and `periodhorn` → `<id:03>_period.abk`
  by exact file presence; the 11–14 non-NHL values per title are "no loose horn bank for this id" (never invented; whether they are
  served from `sfx.abk` / an archive is open — Q-R78-1); files 0 / 100 / 194 are shipped-but-unreferenced. This closes the gen7 half of
  "DB `goalhorn` → hornbank id open" for the NHL clubs as a **file** binding; the bank's sample id inside the `.abk` (ABKC `S10A`
  table, 2026-09-20 handoff 02) is the next hop and is already decoded there.
- `chantaudioid` → `<id>_chant.abk`: **candidate**, shown with "chant bank (candidate; 23 values have no file)" and never as a binding;
  the four-digit chant set is a separate space with no column pointing at it.
- `cityaudioid` / `arenaaudioid`: **not** sfx banks — keep the 2026-09-21 handoff 07 reading (speech sample-group ids; tag open,
  Q-R35-1).
- PS3 12 / 13 (archive audio) and NHL 09 (no numbered banks): refuse with the reason.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-22-pm/probes/r78_gen7_arena_sfx_bank_join.py docs/research/overnight-2026-09-22-pm/data/r78-gen7-arena-sfx-bank-join.json
```
Executed once. Inputs: the ten trees' `audio/sfx` listings and `anchors/{xbox360,ps3}/<title>/databases/nhlng.db` + meta.
