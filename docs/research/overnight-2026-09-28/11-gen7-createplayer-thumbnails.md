# Handoff 11 (28 Sep) — gen7 create-player equipment thumbnails: no data edge found; pads and blockers match their render id by picture content (NHL 12 → Legacy); stored values are a numbered-file candidate

Research lane, 28 September 2026, published 13:38 MDT (system clock). After-receipt-5 prompt §3 (receipt 5 §8). Claim
`research-q9-gen7-createplayer-thumbnails`. Consumes the 18 Sep handoff 30 (thumbnail locations), R125 / R125d (gear value → render file
position) and receipt 47 (NHL 12: `goaliepad_0_{0,2,3}` are shared meshes, not pads 0 / 2 / 3).
- Read-only on the X360 NHL 10–Legacy and PS3 NHL 15 / Legacy loose trees and the committed anchors.
- Decoded images existed only in memory and in scratch contact sheets. The sheets were deleted.
- Nothing launched.

Machine-readable:
- [`data/r143-gen7-createplayer-thumbnails.json`](data/r143-gen7-createplayer-thumbnails.json) (`43c3c48b…`): thumbnail sets, database values, shift controls, render relations, missing values;
- [`data/r143b-thumbnail-render-shift.json`](data/r143b-thumbnail-render-shift.json) (`200323c0…`): render id set vs thumbnail set at shifts −3 … +3, duplicate thumbnails;
- [`data/r143d-visual-checks.json`](data/r143d-visual-checks.json): the picture-content checks and the failed colour-histogram method.

Probes in `local/runs/overnight-research-2026-09-28/probes/`: `r143a_ion_strings.py` (`8a0c387e…`), `r143_gen7_createplayer_thumbnails.py`
(`fda9bc69…`), `r143b_thumbnail_render_shift.py` (`2c4da45a…`), `r143c_thumbnail_content_match.py` (`596928a9…`),
`r143d_thumbnail_tint_sheet.py` (`e1fb6c50…`), `r143e_sheet_patterns.py` (`763988f2…`).

## Plain-language summary

The create-player screens show a small picture for each pad, blocker, glove, and so on. Studio would like to show the same picture
for the gear number a player has stored. No table in the game's data says "gear N uses picture M".
- The front-end screen asks the game code for its lists (names, picture ids, stored values) as separate arrays. So in principle the
  picture number and the stored number could differ.
- What can be shown instead is that the numbers line up:
  - the pictures are numbered like the gear's render files;
  - for pads and blockers on NHL 12 → Legacy, **picture N shows the same design as render texture N**, and never its neighbour's,
    in every sample checked.
- Stored goalie pad, blocker and trapper values on those titles all have a picture of the same number.
- **Masks are different.** Only 88 mask numbers have pictures. Most stored mask numbers have no picture. *(Reworded 15:15, [12](12-reconcile-after-receipt6.md) §3: the absence of a picture does not show that a number is a team mask.)*
- NHL 10 / 11 could not be checked this way.

This is a numbered-file candidate backed by a picture comparison. It is not a data edge. It says nothing about what the game draws in play.

## 1. Where an explicit edge was looked for (bounded, all negative)

| Source | Finding |
|---|---|
| `fe/ion/game/screens/createplayer/cpequipmentimagepickerpanel.big` (Apt script, X360 Legacy) | The panel calls `ION_CreatePlayer.GetAttribute` for per-setting arrays: `DISPLAY`, **`ASSET_ID`**, `LOGO_ID`, `STATUS`, `BOOSTS`, `TASKS`, `COSTS`, `UNIQUENESS`, `VALUE`, `defaultIndex`, over `EQUIPMENT_{GLOVES, HELMET, TRAPPER_STYLE, PAD_STYLE, BLOCKER_STYLE, GOALIE_STICK, GOALIE_MASK}`. The image path = `createPlayer/` + the asset key (`getArtAssetPath`). The arrays come from native code (disassembly is out of scope). **Asset id and value are separate arrays by design.** |
| `fe/equipatrib.bin` (598 B) | Byte-identical on X360 NHL 08 → Legacy (SHA-256 `25af70ba…`), while thumbnail counts range from 14 to 106. It cannot be the per-title picture list; its content reads as rating curves. |
| `fe/beapro.bin`, `fe/dime/*.xml`, `fe/dynamicsettings.txt` | numeric progression tables / store configuration; no equipment ids |
| `attribdb/feattribdb.*`, `renddb.*` | hashed Attrib vaults, with no readable class or field names. Not decoded (no schema), so this is an open lead only through a gen7 vault schema. |
| main database (`nhlng-meta.xml`, Legacy) | per-player / team / stock equipment rows only; **no gear catalogue table** |
| thumbnail packages (`pads<N>.big` …) | `Apt` stub + XPR2 picture (RW4 on NHL 10 / 11); no name text |

## 2. Thumbnail inventory vs database values (per title; shift controls)

`T` = thumbnail ids; `V` = distinct stored values; fit = |V ∩ T| at shift 0. Full rows, including PS3 15 / Legacy, which equal their X360
twins, are in R143.

| Family (DB column) | NHL 10 | NHL 11 | NHL 12 | NHL 13 | NHL 14 | NHL 15 | Legacy |
|---|---|---|---|---|---|---|---|
| pads (`exhibitiongoalieequipment.pads`) | T 38; 15 / 18 (26, 50 have none) | T 56; 32 / 32 | 64; 29 / 29 | 73; 30 / 30 | 80; 34 / 34 | 87; 28 / 28 | 92; 29 / 29 |
| blocker | 37; 14 / 17 (**1** on 30 rows, 24, 50 have none) | 56; 33 / 33 | 65; 28 / 28 | 74; 31 / 31 | 81; 34 / 34 | 88; 29 / 29 | 93; 29 / 29 |
| trapper | 37; 14 / 17 (**1** on 32 rows, 24, 50 have none) | 56; 32 / 32 | 63; 31 / 31 | 72; 28 / 28 | 79; 31 / 31 | 86; 26 / 26 | 91; 26 / 26 |
| goalie stick (`.stick`) | 50; 7 / 7 | 50; 13 / 13 | 50; 14 / 14 | 39; 17 / 17 | 44; 17 / 17 | 48; 19 / 19 | 54; 12 / 12 |
| mask (`.maskid`) | 81; 52 / 172 | 88; 71 / 188 | 88; 68 / 185 | 88; 65 / 184 | 88; 60 / 175 | 88; 70 / 182 | 88; 59 / 173 |
| gloves (`exhibitionskaterequipment.gloves`) | 32; 6 / 11 (23, 24, 25, 27 on 106–221 rows have none) | 47; 14 / 14 | 56; 15 / 15 | 58; 13 / 13 | 68; 15 / 15 | 74; 15 / 15 | 83; 14 / 14 |
| helmet (`.helmetbrand`) | 24; 8 / 9 (15 has none) | 28; 7 / 7 | 29; 7 / 8 | 30; 9 / 9 | 32; 9 / 9 | 34; 10 / 10 | 38; 10 / 10 |
| skates (`.skatebrand`) | 27; 7 / 8 (22) | 35; 8 / 8 | 37; 7 / 7 | 42; 10 / 10 | 48; 9 / 9 | 52; 9 / 9 | 58; 8 / 8 |
| sticks (`.stick`) | 44; 14 / 14 | 56; 14 / 14 | 66; 16 / 16 | 76; 16 / 16 | 88; 20 / 20 | 101; 19 / 19 | 106; 19 / 19 |
| visor (`.visortype`) | `visor0–2`, 3 / 3 | same | same | same | same | same | same |

**Shift control (R143 `shift_hits`):** the stored values also fit at ±1 / ±2 nearly as well, e.g. Legacy pads 28 / 28 / 29 / 29 / 29
for k = −2 … +2. The thumbnail ids are dense runs (1 … 84 with gaps, plus 100 … 109), so **value-set containment does not discriminate
N from N ± 1**, and the table alone is not evidence of identity. The same holds for the render sets (R143b: shift 0 = 1.0, best other
shift 0.94–0.98).

## 3. The discriminator that works: picture content (R143d / R143e)

- The render diffuse `_0_dm` textures are grey detail maps, and the `_0_tm` tint masks carry the design as red / green / blue recolour
  regions. So a **colour** comparison is invalid. R143c's histogram test gave top-1 = N on 2 / 67, which is chance; it is recorded as a
  failed method, not as evidence against the join.
- A **shape** comparison works. Research placed thumbnail N beside the tint masks of N − 1, N and N + 1, then inspected them:

| Title / family | Render file | Match at N | Match at N ± 1 | Undetermined |
|---|---|---|---|---|
| X360 Legacy pads | `goaliepad_<N>_0_tm` | 11, 12, 31, 44, 45, 46 (+ 20 plausible) | none | 40, 60, 77 (plain designs) |
| X360 Legacy blocker | `blocker_0_<N>_0_tm` | 11, 12, 31, 44 | none | — |
| X360 NHL 12 pads | `fegoaliepad_0_<N>_0_tm` | 11, 12, 31, 44 | none | — |
| X360 Legacy trapper (added 13:39) | `trapper_0_<N>_0_tm` | 31, 44 plausible only (bands; circle logo) | none | 11, 12 (net-dominated designs) |
| X360 NHL 10 pads | `texlib_<N>` | — | — | not discriminable: RW4 thumbnail not rendered by the research decoder; texlib = detail map |

Examples:
- pads / blocker 11 is the Vaughn zigzag;
- 12 is a red "Brians" flame;
- 31 is Sher-Wood stripes;
- 44 is the KOHO crossing design.

These are qualitative judgements, by research, on 14 clear samples. They are not a pixel metric.

## 4. Per-family result (developer-consumable, NHL 12 → Legacy X360; PS3 15 / Legacy by identical sets)

*Confidence per title and family, added 15:15 after the receipt-6 review ([12](12-reconcile-after-receipt6.md) §3). The picture checks cover only X360 Legacy pads / blocker (and trapper, plausible only) and X360 NHL 12 pads. NHL 13 / 14 / 15 and PS3 15 / Legacy rest on **equal id sets only**, which do not prove identical pictures. Nothing here is a stored-value selection semantic. The reproducible review set is listed in [`data/r143f-visual-review-set.json`](data/r143f-visual-review-set.json).*

| Family | Edge class | Contract |
|---|---|---|
| pads / blocker / trapper | **numbered-file candidate, content-corroborated** (pads and blocker sampled; trapper by set only) | Show `createplayer/<pads\|blocker\|trapper>/<family><N>.big` for stored value N, labelled a thumbnail candidate. Every stored value has one (0 missing on 12 → Legacy). |
| goalie stick, gloves, helmet, skates, sticks | **numbered-file candidate by set only** (no content check) | Same display, with lower confidence. Folder / prefix per §5. |
| mask (`maskid`) | **partial**: the 88 thumbnails ⊆ paint ids (`goaliehelmet` position 2); 112–120 of 172–188 stored values (NHL 10 → Legacy) (including every value-0 row) have **no** thumbnail | Show a thumbnail only when `mask<N>.big` exists; otherwise, neutrally, "no picture available" *(wording corrected 15:15: absence does not establish team-mask status)*. Team slots: 70–77 of 218–230 distinct non-zero values have one. |
| visor | 3 thumbnails, 3 values; ambiguous (every shift fits) | no join claimed |
| NHL 10 / 11 (texlib era) | NHL 10 values miss thumbnails (blocker 1 on 30 rows, gloves 23–27 on 106–221 rows); NHL 11 set fits, content unchecked | no join claimed; show nothing or "unverified" |

**Duplicates and exceptions.**
- `pads37 = pads41`, `blocker37 = blocker41` and `trapper37 = trapper41` are byte-identical on NHL 13 → Legacy.
- `gloves0 = gloves1` from NHL 11 onwards.
- One stray `gloves72.big` sits in Legacy `createplayer/blocker/`.
- Legacy `goaliestick9999.big` exists. It is outside every stored range; it is not an id.

**Receipt 47.** On NHL 12, `pads2.big` / `pads3.big` exist, but `goaliepad_0_2` / `_0_3` are shared meshes, not pads 2 / 3. A thumbnail's
existence does not make a mesh file a pad. The shared-mesh exclusion stays. No handedness is inferred: blocker / trapper thumbnails are
per id, and the s = 0 / 1 files are not distinguished.

## 5. Implementation impact (observation for the developer; research changed nothing)

`crates/studio-domain/src/equipment_slots.rs` `SLOT_SPECS.thumbnail_prefix`: **no code reads this field today**, so nothing is wrong at
runtime. Two of its values name paths that match **no** file on any measured title:
- `ion/artassets/createplayer/sticks/goaliestick` → actual `createplayer/goaliestick/goaliestick<N>.big` (the `sticks/` folder holds only
  `stick<N>`);
- `ion/artassets/createplayer/skates/skates` → actual `createplayer/skates/skate<N>.big`.

A future consumer must fix these first. The candidate status of §4 then applies. Queue C38.

## 6. Not established / next

- What the game shows in play.
- The `ASSET_ID` array's source: native code, or a hashed vault. Either way it is not static-readable without disassembly or a gen7
  vault schema.
- NHL 10 / 11 thumbnails.
- Trapper, glove, helmet, skate and stick content checks. The same sheet method applies (`_0_tm` for trapper; texlib-era families
  need another design-carrying map).
- PS3 NHL 08–14 archive-held thumbnails (not walked).
- No user card is needed for the thumbnails themselves. They are presentation only.
