# Handoff R124 (P20) — Players 3.4 portrait and head reassignment: the evidence chain, what each title family adds (texture libraries from NHL 13, head registry tables from NHL 11 / 12, a `headtexture` twin on PS4 17–21), target sets, and separate listing / selection / reassignment tables with refusal conditions

Research lane, 27 September 2026, published 05:07 MDT (system clock). Prompt
[`research-20260926-after-receipt3.md`](../../plans/research-20260926-after-receipt3.md) package 3. Claim `research-p20-portrait-head-reassignment`.
Reuses 2026-09-21 pm handoff 11 (R53 gen7 containment, R57 portrait archive chain), handoff 20 (R63 PS4 portraits), 2026-09-20 handoff 08
(R15 PS4 vault head binding), 2026-09-22 pm handoff 06 (O-8 reading note) and Studio's `assets.rs` / `player_containment.rs` (read, not
edited). Read-only directory listings of the loose trees on `X:` and the anchors / genuine NHL 20 databases. **No head conversion, no
mesh work, nothing written, nothing launched.**

Machine-readable: [`data/r124-head-portrait-reassignment-inputs.json`](data/r124-head-portrait-reassignment-inputs.json).

## Plain-language summary

Every player row names its portrait with one number (`artid`) and its head model with another (`headid`), and on every title where the
files could be listed, each roster number points at a file of exactly that name. So Studio can already list which portraits and heads
exist and which are unused. Pointing a player at a different existing portrait or head would be a database edit, but this research cannot
prove what the game then draws: that is a naming match, not an observed selection. The checks show what such an edit must respect. From
NHL 13 on, every head the roster uses also has its own texture file, so a target head without one is outside what the games ship. From NHL
12 on, the database has a head table (hair cards) that covers every roster head except six. On PS4 NHL 17–21 a second field (`headtexture`)
always equals the head number, so the two must move together. One in-game test on a copy would settle whether a reassigned head and
portrait actually appear.

## 1. Evidence chain — representative source (Xbox 360 NHL Legacy, loose tree; PS3 Legacy identical sets)

| Edge | Observation | Class |
|---|---|---|
| player row → portrait id | `exhibitionplayerbiotable.artid` (`ubPc.lOnr`), 16 b | measured |
| portrait id → file | `fe/ion/artassets/playerheads/p{bucket}/p{artid}.big` (bucket by id range); 4 376 of 4 378 flagged rows have it (2 flagged without: Nagy, Colella); 127 flag-clear rows also have it | measured (naming; R53) |
| file → raster | BIGF → Apt record + RefPack → XPR2 (NHL 12–Legacy; RW4xb2 on 09–11) | measured (R57) |
| `hasportrait` | a stored display flag; contradicts files on 13 / Legacy (R53); 0 contradictions "flagged without file" on PS4 (R63) | measured; its effect is runtime |
| player row → head id | `headid` (`ubPc.pPaD`), 16 b | measured |
| head id → mesh | `rendering/player/player_{headid}.rx2` (PS3 `.rpsgl`); every roster id resolves (R53) | measured (naming) |
| head id → head textures | `rendering/player/texlib_{headid}.rx2`: **every roster-referenced head has one on NHL 13 → Legacy** (e.g. 5 632 / 5 632 rows on Legacy, 5 412 / 5 412 on NHL 13); none exist on NHL 09–12 | measured |
| head id → DB head registry | `stockplayerheads.headid` (fields `haircards_a`, `haircards_b`, plus `haircap` on 12 / 13): covers **every roster head id** on NHL 12, 13 and Legacy, all but 2 on NHL 14 (740, 8488) and 4 on NHL 15 (60007, 60026, 60086, 60138); absent on 09–11 | measured; consumption unobserved |
| head id → helmet placement | `stockplayerhelmetplacement.headid` (transform per helmet `type`) from NHL 11: 9–37 roster ids per title have **no** row (defaults presumably apply) | measured; optional in stock |
| bespoke vs pool | Studio: file-truth (a non-pool mesh exists), pool floor data-driven (60 000 if any roster id reaches it, else 10 000). **NHL 14's pool is 10 000–10 182** (171 shared pool ids, 5 111 rows on pool heads; no roster id ≥ 60 000) — Studio's rule gets this right; R53's summary sentence "60 000+ on NHL 14–Legacy" was wrong for NHL 14 (dated note added there) | measured |
| selection by the game | not observed: nothing here shows which file the runtime loads for a changed id | **runtime** |

## 2. Structural families (what changes between titles)

| Family | Portrait | Head mesh | Head textures | DB head tables | Pool floor | Coverage here |
|---|---|---|---|---|---|---|
| X360 NHL 07 / 08, PS3 08 | not censused | not censused | — | — | — | **not measured** |
| X360 NHL 09 | `p<id>.big` RW4xb2 | `head_<id>.rx2` (480) | none | none | 10 000 | loose tree |
| X360 NHL 10 / 11 | RW4xb2 (11: RefPack) | `player_<id>` (+ a few `head_<id>` on 10) | none | helmet placement from 11 | 10 000 | loose tree |
| X360 NHL 12 | RefPack → XPR2 | `player_<id>` | none | + `stockplayerheads` (with `haircap`) | 10 000 (to 30 101) | loose tree |
| X360 NHL 13 / 14 | XPR2 | `player_<id>` | **`texlib_<id>` appears** | both | **10 000** | loose tree |
| X360 NHL 15 / Legacy; PS3 15 / Legacy | XPR2 | `player_<id>` / `.rpsgl` | `texlib_<id>` | both (no `haircap`) | 60 000 | loose trees (PS3 = X360 sets) |
| PS3 NHL 08–14 | archive-held | archive-held | — | as X360 twin | — | **not walked** |
| Legacy Recompiled | — | — | — | — | — | **not measured** (not inherited) |
| PS4 NHL 15 / 16 | `.png` (15) / `playerportraits/p<id>.gnf` | vault `ge_nhl_skaterhead` via `SkaterNameAttribute` = `headid` | vault texture pack, same attribute | `stockplayerheads`, helmet placement tables present | 60 000 | vault (R15), portraits (R63) |
| PS4 NHL 17–21, genuine NHL 20 | `.gnf` | vault | **`headtexture` field = `headid` on every row** (6 128–7 576 rows) | tables absent | 60 000 | R15 / R63 + this scan |

## 3. Target sets and encodability (loose trees; per title in the JSON)

- Declared widths hold every shipped id: `artid` 14 b (≤ 16 383) on 09–14 against a largest portrait id ≤ 11 036; 16 b from NHL 15;
  `headid` 15 b on 09–13 (largest file 30 102), 16 b from 14 (largest 60 210). No shipped target is unstorable.
- Unused shipped targets: portraits 214 (09) → 4 724 (Legacy); head meshes 129 → 457, of which **with a texture library** 156 (13) → 238
  (Legacy); on 09–12 textures are not separate files.
- DB head registry vs files: `stockplayerheads` has 419–926 rows with no loose mesh (loose tree only; archive-held or absent — not
  resolved) and 182–236 loose meshes with no row.

## 4. Operation tables

**Portrait (`artid`)**

| Operation | Status | Evidence | Refusal / condition |
|---|---|---|---|
| List a player's portrait and the unused pool | supported by evidence (Studio implements listing, receipt 14) | naming + containment on every walked tree | "no portrait shipped for this player" when the file is absent; never derive or write `hasportrait` from a file check |
| Reassign to another **existing** shipped portrait | **data-level contract only**; selection unobserved | naming consistency; stock data already shares a portrait id between 2–3 rows on NHL 11 / 13 / 14 / Legacy, so sharing is not unprecedented | refuse a target id with no file in the title's own tree (or archive, where a reader exists); refuse ids beyond the declared width; disclose `hasportrait` as stored (do not flip it silently); PS3 Legacy: refuse or disclose when the target also exists in `patch.big` with different bytes (C0-AS7: 23 collisions; load precedence unobserved) |
| New portrait (new file / member) | out of scope here (texture / archive route) | R57 chain | — |

**Head (`headid`)**

| Operation | Status | Evidence | Refusal / condition |
|---|---|---|---|
| List a player's head and the unused meshes | supported (Studio: file-truth bespoke / pool / other) | naming + containment | "shipped without asset" for the NHL 09 five; PS4 NHL 15 `headid 10391` has no vault record |
| Reassign to another **existing** shipped head | **data-level contract only**; selection unobserved | naming consistency; pool heads are shared by design (5 000+ rows); two stock non-pool ids are shared by 2 rows (1005 on 10–14, 7797 on Legacy) | require a mesh file of that id; **NHL 13 → Legacy: also require `texlib_<id>`** (no roster head lacks one); **NHL 12 → Legacy: also require a `stockplayerheads` row** (six stock exceptions on 14 / 15 — warn, not refuse, if the coordinator prefers); helmet placement optional; ids within the declared width; **PS4 17–21: write `headtexture` with `headid` as one change** (never observed apart); PS4: target must exist as a vault `SkaterNameAttribute` value (R15 lists them); disclose when the target is another player's bespoke head (a duplicate face) |
| Pool ↔ bespoke change | same as above | file-truth classes | disclose class change; no id equality rule (NHL 14 pool ids collide with portrait ids on 5 014 rows — equality is not a bespoke signal) |
| New head (new mesh / vault collection) | **out of scope** — external head-conversion workstream | — | refuse |

## 5. Known, missing, shared and conflicting cases

- **Known:** every roster head id on NHL 10 → Legacy resolves to its mesh; NHL 13 → Legacy also to its texture library.
- **Missing:** NHL 09 — five bespoke heads and five portraits (Stastny, Staal, Malkin, Zherdev, Andersson …, R53); NHL 10 portrait
  Schenn; NHL 13 portrait Grigorenko; Legacy — two flagged rows without a portrait (Nagy, Colella); PS4 NHL 15 head 10391 (no vault record).
- **Shared:** pool heads (by design); non-pool 1005 (two rows, NHL 10–14) and 7797 (two rows, Legacy); portraits 5740 / 5746 (NHL 11), 5736
  (13), 5736 / 5750 (14), 2 / 7 / 12 (Legacy); PS4 NHL 18: 140 portrait ids on two rows (the R120 duplicate-player rows).
- **Conflicting:** `hasportrait` vs files (93 on 13, 127 + 2 on Legacy); PS3 Legacy `patch.big` portrait members colliding with loose files
  of different content (C0-AS7, 23); `stockplayerheads` rows without a loose mesh (419–926; location unresolved).

## 6. Current Studio behaviour (read, not executed)

Listing / classification are implemented (`player_containment.rs`, `assets.rs`; pool floor data-driven — correct on NHL 14). The Player
Data tab deliberately hides `artid`, `headid` and `hasportrait` (`PlayerDetail.tsx` `SUPPRESSED_MISC_BIO`); no reassignment writer exists
(phase-3 matrix row "reassignment: not impl"). The raw table editing path was not examined in this package.

## 7. Next discriminating observation (user card, proposed: P34-1)

On a **copy** of Xbox 360 NHL Legacy: set one skater's `headid` to an unused bespoke head that has a mesh, a texture library and a
`stockplayerheads` row, and his `artid` to an unused shipped portrait; play one game / open the roster screen. Expected under the naming
reading: the new face and portrait. Failure modes: old face (cached elsewhere), missing texture (texlib not selected by id), generic face
(registry row required elsewhere). Restore: the copy. Until then, the reassignment rows above stay "data-level contract only".

## 8. Reproduce

```
python -B local/runs/overnight-research-2026-09-26/probes/r124_head_portrait_reassignment_inputs.py docs/research/overnight-2026-09-21-pm/data/r53-gen7-player-asset-containment.json <out.json>
```
Executed twice (the first run used a fixed title list for the pool floor and was discarded; the published run uses Studio's data-driven rule).

## 9. Addendum 06:20 — the archive-held PS3 titles (R134, [`data/r134-ps3-archive-portrait-head-containment.json`](data/r134-ps3-archive-portrait-head-containment.json))

Member names from each PS3 title's archives (EB / BIG4; loose where present) joined to the anchor exactly as §3; probe
`r134_ps3_archive_portrait_head_containment.py`.

| PS3 title | Portraits (archive) | Head meshes (archive) | Separate head textures | Roster rows: head class / file | Portrait flag vs file | Unused heads / portraits |
|---|---|---|---|---|---|---|
| NHL 08 | 3 493 `playerheads/p<id>.big` (no bucket dir; `data0.big`) | 442 `head_<id>` (`data1.big`) | `head_<a>_<b>_<c>_<d>_cm` (433) — per-head colour textures (corrected in §10), `hair_*` | pool 2 464 with mesh; non-pool 162 with / **336 without** a `head_<id>` mesh (likely composed from parts) | no `hasportrait` column; 2 782 / 2 782 non-zero art ids have a file | 180 / 710 |
| NHL 09 | 2 920 (`fe.big`) | 480 `head_<id>` (`player.big`) | `hair_<id>_cm` 343 | pool 3 711; non-pool 252 / 5 missing (the same five as X360) | 1 / 1 078 / 2 705 as X360 | 129 / 214 |
| NHL 10 | 3 637 | 742 | **`head_<id>_cm` 522, `hair_<id>_cm` 394** (X360 has none) | all with mesh; **370** roster heads have a `head_<id>_cm` | as X360 | 372 / 679 |
| NHL 11 | 5 397 (`fe.big`) | 821 (`rendercache.big`) | `head_<id>_cm` 601, `hair_<id>_cm` 470 | all with mesh; 424 with `head_cm` | as X360 | 397 / 1 130 |
| NHL 12 | 6 158 (`nocacherender.big`) | 778 (`cacherender.big`) | `head_<id>_cm` 559, `hair_<id>_cm` 461 | all with mesh; 432 with `head_cm` | as X360 | 346 / 1 730 |
| NHL 13 | 6 823 | 817 | `texlib_<id>` 598 (as X360) | all with mesh; 442 with texlib | 93 flag-0-with-file as X360 | 375 / 2 433 |
| NHL 14 | 7 678 | 841 | `texlib_<id>` 622 | all with mesh; 441 with texlib | as X360 | 400 / 3 138 |

Consequences for §4: PS3 NHL 09, 10, 11, 13, 14 carry the X360 target sets (equal portrait and head-mesh counts; NHL 12: equal portraits, 778 head meshes vs 799 on X360); **PS3 NHL 10 → 12 add per-head
texture files `head_<id>_cm` / `hair_<id>_cm`** that X360 embeds — a PS3 head reassignment on those titles should check the `head_cm`
companion the same way NHL 13+ checks `texlib` (not every roster head has one: 370–432 do; whether the others fall back is runtime). PS3 NHL 08 (and X360 NHL 08, §10) keep bespoke heads as `head_<id>` meshes with per-head textures; the roster heads without a file are not
explained — reassignment there is limited as §10 states.

## 10. Addendum 06:22 — Xbox 360 NHL 07 / 08 heads and portraits (R134b, [`data/r134b-x360-07-08-heads.json`](data/r134b-x360-07-08-heads.json)); correction to §9

**Correction to §9 (NHL 08 row):** the `head_<a>_<b>_<c>_<d>_cm` textures are **not** composite parts — positions 2–4 take a single value on
every file, and position 1 equals a `head_<id>` mesh id: they are the per-head colour textures of the bespoke heads (NHL 08: 433 for 442
meshes; every one of the 262 roster heads with a mesh has one). Hair ships the same way (`hair_<id>_…`).

| Title | Portraits | Head meshes `head_<id>` | Per-head textures | Roster: heads / with mesh | Rows without a head file |
|---|---|---|---|---|---|
| X360 NHL 07 | 1 243 `playerheads/p<id>.big` (957 of 2 250 roster art ids have one) | 438 | `head_<id>_*_cm` 441 (+ `_am`, eye / eyelid / hair parts per head) | 1 349 distinct / 398 | **951 rows** (ids 226–10 500) |
| X360 NHL 08 (= PS3 NHL 08, identical counts) | 3 493 | 442 | 433 | 598 / 262 | **336 rows** (ids 3 000–4 429) |

The rows without a `head_<id>` file are the NHL 07 / 08 equivalent of later titles' pool heads, but no pool file family was found by name
(`commonhead_#` 2–4 files exist): how the game builds those faces is **not established**. Reassignment on NHL 07 / 08 is therefore limited
to bespoke heads that have both a `head_<id>` mesh and its `head_<id>_*_cm` texture; targets without files must be refused.
