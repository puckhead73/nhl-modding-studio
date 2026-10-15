# Handoff R125 (P21) — Equipment 4.5 goalie gear: database value → catalogue / file → geometry / texture → consumer, per title family — NHL 08 / 09 paint is `maskcolor` (not `maskpattern`), masks are two-level from NHL 10 (team slots + goalie `maskid`), PS4 selection keys include helmet brand and handedness

Research lane, 27 September 2026, published 05:16 MDT (system clock). Prompt
[`research-20260926-after-receipt3.md`](../../plans/research-20260926-after-receipt3.md) package 4; answers the observation in developer
claim `dev-p45-goalie-gear` (receipt 31: "which asset or catalogue entry does each id select, per title"). Claim `research-p21-goalie-gear-chain`.
Reuses 2026-09-20 handoff 09 (R16 PS4 vault chain, genuine NHL 20 install), 2026-09-21 handoff 12 (gen7 file grammar), 2026-09-21 pm
handoff 10 (component sets, R74 note), 2026-09-18 handoff 30 (fe thumbnails). Read-only; archive **member names** only for NHL 08; no
extraction, no decoding of meshes, nothing written or launched.

Machine-readable: [`data/r125-gen7-goalie-gear-chain.json`](data/r125-gen7-goalie-gear-chain.json) (per title and DB column: every file
position containing ≥ 90 % of the values), [`data/r125b-goalie-mask-levels.json`](data/r125b-goalie-mask-levels.json) (NHL 08 / 09 mask
columns; team mask slots), [`data/r125c-ps4-goalie-selection-keys.json`](data/r125c-ps4-goalie-selection-keys.json) (PS4 key tuples).

## Plain-language summary

Studio can already edit every goalie gear number; what it lacks is what each number means. This package traces each one to the files or
catalogue records it names. On the older consoles the numbers match file names: from NHL 11 the mask shape and mask paint, stick, pads,
blocker and trapper each map to one numbered set of files. Two things are easy to get wrong. On NHL 08 and 09 the mask paint number sits
in a column called `maskcolor`; the column called `maskpattern` is zero for every goalie. And from NHL 10 each team also stores mask paints
for its starter and backup in up to seven uniform slots, so a goalie's own mask number is not the only mask input. On PS4 each number
names a catalogue record directly, but a mask needs the mask number and the helmet type together, and sticks, blockers and trappers exist
in left- and right-handed versions. None of this shows which item the game draws; it gives neutral numbered candidates Studio can show.

## 1. Chain per item and family (edge classes: **name** = file-name containment, **typed** = vault attribute join; runtime selection is unobserved everywhere)

| Item | NHL 08 (archive names) | NHL 09 | NHL 10 | NHL 11 | NHL 12 → Legacy (X360; PS3 15 / Legacy identical) | PS4 15 → 21 incl. genuine NHL 20 |
|---|---|---|---|---|---|---|
| mask shell | `masktype` (0–2) → `goaliehelmet_<a>…` / `helmet_1_0_<shell>` shells {0, 1, 2} — name, 3 / 3 | `masktype` → shells {0, 1, 2, 100, 101, 102}; 4 / 5 values (103, one goalie, has no shell) | `masktype` → `goaliehelmet_<shell>` {0–5}; 5 / 6 (99, one goalie) | `masktype` → shell n1: 5 / 5 of 7 | `masktype` → shell n1: 3–5 / 3–5 of 7–8 | `masktype` → `GoalieHelmetBrand` (0–7, + 8 from NHL 19) on `ge_nhl_goaliehelmet` (9 geometry records) — typed; **99** on 4–5 goalies (NHL 19–21) is not a brand |
| mask paint | **`maskcolor`** (8 b) → paint position of `goaliehelmet_<a>_<b>_<paint>_<d>`: 99 / 99 values; `maskpattern` = 0 on all 315 goalies; `hascustommask` = 0 on all | **`maskcolor`** (9 b): 149 / 149; `maskpattern` 0 on all 415; `hascustommask` 0 | `maskid` → paint n2: 172 / 172 of 241 | `maskid` → n2: 188 / 188 of 243 | `maskid` → n2: 173–185 / … of 246–280 (unique full candidate each title) | (`maskid`, `masktype`) → exactly one `ge_nhl_goaliehelmettexturepack` record for 98.2–100 % of goalies per title (§2; typed); up to 8 records per `maskid` (one per helmet brand) |
| team mask slots | — | — | `exhibitionteams.goaliemask0–5_{starter,backup}` (8 b): every non-zero value is a paint file id | slots 0–6 (8 b): all paint ids | slots 0–6 (9 b; measured on 13, 15, Legacy — 12 / 14 not run): all paint ids | slots 0–6 (+ 7–8 declared, all zero on 18–21); values in `GoalieMaskTexture` except **248, 249** on every title 16+ (and 173 / 198 / 208 / 261 on 16, 208 on 17) |
| `maskrank` | 0 / 1 — ambiguous (fits the 4th mask number and many 2-value positions) | 0–2 — ambiguous | absent | absent | absent | absent |
| pads | `pads` → `goaliepad` `texlib_<id>` 19 / 19 (texture-keyed); meshes shared | `texlib` 21 / 21 | `texlib` 17 / 18 (value 0 has no texlib) | `texlib` 32 / 32 | `goaliepad` id position: 28–34 / all (n2 on 12, n1 from 13) | `pads` → `GoalieBrands` in `ge_nhl_goaliemergedpadtexturepack` (all used values, every title); the non-merged pad class is a subset (0–42 values) — typed, two classes |
| blocker / trapper | `texlib_<id>` 18 / 18; mesh files `blocker_<s>_…` with s ∈ {0, 1} | `texlib` 20 / 20 | `texlib` 17 / 17 | `texlib` 33 / 33 and 32 / 32 | **n2** of `blocker_<s>_<id>` / `trapper_<s>_<id>`: all; **every id exists under s = 0 and s = 1** (NHL 08, 12 → Legacy) | `GoalieBrands` + `StickHandedness` (left / right pack per brand; every used brand has both) — typed |
| goalie stick | `stick` (declared 0..190, used 0–5) → `goaliestick` n1: 6 / 6 | 6 / 6 | 6 / 7 (25, one goalie, has no file) | 13 / 13 of 14 | 12–19 / all of 23–40 | `GoalieStickBrand` + `StickHandedness` (NHL 16–21 left / right per brand; NHL 15 packs carry no handedness) — typed |
| neck / throat protector | `neckprotector` 0 / 1 — no discriminating file position (`throatprotector_*` 6–9 files keyed by shell) | same | same | same | same | `ge_nhl_goaliethroatprotector`: 2–5 records keyed by `GoalieHelmetBrand` with `OnOff` = 1 only — candidate: throat protector geometry per helmet brand, `neckprotector` as the switch (unobserved) |
| geometry | shared meshes: pad / blocker / trapper styles, 8 mask shells, cages per shell (R74 / handoff 10) | same | same | same | same | `ge_nhl_goalieblocker` / `…trapper` 4 records (handedness × `OnOff`), `ge_nhl_goaliepad` 2, `ge_nhl_goaliehelmet` 9 (brand) — model references not decoded |
| thumbnail (front end) | not re-measured (handoff 30: `fe` packages `pads{N}`, `blocker{N}`, `trapper{N}`, `goaliestick{N}`, `mask{N}`, NHL 08 `maskpattern` 23 + `maskstyle` 3) | … | … | … | … | Creation-Zone registries (2026-09-19 handoff 06), not joined |

## 2. PS4 mask resolution by (`maskid`, `masktype`) — per goalie row

| Title | goalies | exactly one record | `maskid` exists only under another brand | `maskid` not in catalogue |
|---|---|---|---|---|
| NHL 15 | 559 | 559 | 0 | 0 |
| NHL 16 | 566 | 556 | 3 | 7 |
| NHL 17 | 601 | 596 | 0 | 5 |
| NHL 18 | 714 | 714 | 0 | 0 |
| NHL 19 | 747 | 743 | 4 | 0 |
| NHL 20 (genuine, CUSA15439) | 773 | 768 | 5 | 0 |
| NHL 21 | 749 | 744 | 5 | 0 |

The "another brand only" rows on 19–21 coincide in count with the `masktype = 99` rows. `maskid` alone is not a unique key (up to 8
records per value).

## 3. Candidate mappings for Studio (neutral numbered labels; nothing asserts what the game draws)

- **Display**, per title: "Mask shell `<masktype>` · paint `<maskid | maskcolor>`" with the matched file stem / vault `DebugName` behind
  disclosure (`goaliehelmet_<shell>_<paint>`, `goaliehelmet_itech_<team>_<n>_texpack`); pads / blocker / trapper / stick as "`<family>` id
  `<n>`" with the file stem or pack name; never a brand or team name as the primary label unless a title-owned dictionary supplies it.
- **Assignment validity** (existing entries only): the value must name an existing file / record **in this title**: NHL 08 / 09
  `maskcolor` ∈ paint ids, `masktype` ∈ shells; NHL 10+ `maskid` ∈ paint ids, `masktype` ∈ shells; PS4 the (`maskid`, `masktype`) pair must
  exist; pads / blocker / trapper / stick ∈ the family's id set (texlib ids on 08–11). Report, do not refuse, the shipped exceptions (NHL
  09 `masktype 103`, NHL 10 `masktype 99` / `stick 25` / `pads 0`, PS4 `masktype 99`, team slot values 248 / 249).
- **Team slots**: a goalie's mask may come from the team's slot for the uniform being worn; the goalie `maskid` edit is not the only mask
  input from NHL 10. How the two combine is not established (no join from goalie to slot measured here).
- **Handedness**: gen7 blocker / trapper files exist in two styles s = 0 / 1 for every id, and PS4 names its packs left / right. No
  goalie-equipment column stores a side on either generation; which player field selects it is not established.

## 4. Developer observations

| # | Observation | Targets | Smallest action |
|---|---|---|---|
| G1 | The 4.5 sweep (receipt 31) lists NHL 08 / 09 mask columns as `masktype`, `maskpattern`, `maskrank`; the paint candidate is **`maskcolor`**, and `hascustommask` also exists. Neither was named in the sweep | X360 / PS3 NHL 08, 09 | include `maskcolor` / `hascustommask` in the goalie-gear surface and regression; label `maskpattern` as "stored, 0 on every shipped goalie" |
| G2 | Team mask slots (`exhibitionteams.goaliemask<k>_{starter,backup}`) are an equipment input on NHL 10 → PS4 21 | all titles from NHL 10 | show them on the team's uniform / goalie surface as numbered paints; same validity as `maskid` |
| G3 | PS4 mask validity needs the (`maskid`, `masktype`) pair | PS4 15–21 | validate the pair against the title's catalogue; `masktype 99` stays a stored number |

## 5. Not established / next observation

Runtime selection of every item; how team slots and goalie `maskid` combine; which field chooses the blocker / trapper side; `maskrank`;
`masktype 99` / `helmetbrand 99` semantics; the PS3 NHL 08–14 archive-held gen7 trees (not walked); Recompiled (not measured). Optional user
card **G-MASK-1** (a copy of NHL 15 X360): set one goalie's `maskid` to an unused paint id whose file shows a clearly different design and
play a game — the new paint = the goalie-level edge is the consumer; unchanged = the team slot (or another input) wins for that uniform.

## 6. Reproduce

```
python -B local/runs/overnight-research-2026-09-26/probes/r125_gen7_goalie_gear_chain.py docs/research/overnight-2026-09-21-pm/data/r53-gen7-player-asset-containment.json <out.json>
python -B local/runs/overnight-research-2026-09-26/probes/r125b_goalie_mask_levels.py <out.json>
python -B local/runs/overnight-research-2026-09-26/probes/r125c_ps4_goalie_selection_keys.py <out.json>
```
Executed: R125 twice (a guard fix after the first run stopped on a one-number blocker stem), R125b twice (a JSON key fix), R125c once on NHL
21 as a trial and once on all seven titles.

## 7. Addendum 05:18 — skater gear on the remaining gen7 titles (R125d, [`data/r125d-gen7-skater-gear-positions.json`](data/r125d-gen7-skater-gear-positions.json))

Same containment test for `exhibitionskaterequipment` over `rendering/{glove, helmet, skate, stick, playerstick, visor}`, extending
2026-09-21 handoff 12 (NHL 11 / 13 / 15 / Legacy, reproduced here as controls) to Xbox 360 NHL 08 (archive member names), 09, 10, 12, 14:

| Column | NHL 08 | NHL 09 | NHL 10 | NHL 12 | NHL 14 | Reading |
|---|---|---|---|---|---|---|
| `gloves` | `glove` `texlib` 9 / 9 | `texlib` 11 / 12 (0 has none) | `texlib` 11 / 11 | `glove` n2 15 / 15 | `glove` n1 15 / 15 | texture-keyed through NHL 11, mesh-position keyed from 12 |
| `helmetbrand` | `helmet` n2 6 / 6 | n2 8 / 9 (15 has none) | n1 8 / 9 (15) | n1 7 / 8 (99 has none) | n1 9 / 9 | helmet id position; 15 / 99 = stored values without a file (cf. PS4 `helmetbrand 99`, R16) |
| `skatebrand` | `skate` `texlib` 9 / 9 | 12 / 12 | 8 / 8 | 7 / 7 | 9 / 9 | `texlib` n1 on every title |
| `stick` | `stick` `texlib` 12 / 12 | 20 / 20 | 14 / 14 | `playerstick` `texlib` 16 / 16 | 20 / 20 | `texlib` n1 on every title |
| `visortype` | 3 values (0–2) | 3 | 3 | 3 | 3 | ambiguous (fits several 3-value positions); handoff 12's `helmet/visor n1` reading stays a candidate |

Developer use: the same neutral "`<family>` id `<n>`" display and existing-id validity as §3, per title; the Legacy-only model names in
Studio stay Legacy-only (receipt 21). Which glove / helmet / skate / stick the game draws stays the in-game question (card V-71 covers the
edit path only).

## 8. Addendum 06:24 — archive-held PS3 NHL 08 → 14 (R125e, [`data/r125e-ps3-goalie-gear-archives.json`](data/r125e-ps3-goalie-gear-archives.json))

R125's test on archive member names (PS3 NHL 08 `data0` / `data1.big`; 09 / 10 / 11 `rendercache.big` (+ `rendernocache.big`); 12 → 14
`cacherender.big` (+ `renderboot.big` on 14)) reproduces the Xbox 360 table column for column: NHL 08 / 09 paint = **`maskcolor`** (99 / 99,
149 / 149 at the paint position), `maskid` → paint position 2 from NHL 10 (172 / 188 / 185 / 184 / 175 values all present); masktype →
shell position 1 (NHL 09: 103 has no shell; NHL 10: fits only the paint position because 99 has no shell, as on Xbox); pads / blocker /
trapper → `texlib_<id>` on 08 → 11 (NHL 10 pads value 0 has none) and id positions from 12 (both styles s = 0 / 1 for every id); goalie
stick → n1 (NHL 10: 25 has no file). So the C20 candidate mappings apply to the PS3 targets of NHL 08 → 14 as measured; PS3 NHL 15 /
Legacy were already covered with the loose trees (§1).
