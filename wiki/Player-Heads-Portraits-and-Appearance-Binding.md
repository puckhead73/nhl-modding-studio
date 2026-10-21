# Player Heads, Portraits and Appearance Binding

**Evidence:** measured: gen7 file containment on the Xbox 360 NHL 07 and NHL 09 to Legacy trees, the PS3 NHL 08 to 14 archives and the PS3 NHL 15 and Legacy trees; the PS4 vault binding and portrait containment on all seven titles. Which file the game draws after any id change has never been observed (open).

## Gen7 (Xbox 360 and PS3)

Three fields of the player bio table (`ubPc`) carry appearance identity:

| Field | Meaning |
|---|---|
| `ubPc.lOnr` (`artid`) | **portrait id**, 14 bits on NHL 09 to 14, 16 from NHL 15 |
| `ubPc.pPaD` (`headid`) | **head model id**, 15 bits on NHL 09 to 13, 16 from NHL 14 |
| `ubPc.SvcL` (`hasportrait`) | a stored display flag, decoupled from heads |

A head is **bespoke** (authored for one player) when `pPaD == lOnr` on Legacy; the portable signal across all anchors is **uniqueness**: a head id used by exactly one person is bespoke whatever its value. Otherwise the player wears a shared **pool head**. Pool floors: 10 000 and up on NHL 09 to 14 (NHL 14's pool is 10 000 to 10 182; NHL 12 reaches 30 100) and **60 000 and up only from NHL 15**. On NHL 14 pool ids collide with portrait ids on 5 014 rows, so equality is no bespoke signal there.

**Portrait chain:** `fe/ion/artassets/playerheads/p{bucket}/p{artid}.big` with buckets `p0_4000`, `p4001_8000`, `p8001_12000` (flat on NHL 08), a matching `playerheadssmall` set, and four silhouette stand-ins at `silhouettes/p60000..60003.big`. The package is BIGF → Apt record plus texture member → RefPack → XPR2 on NHL 12 to Legacy, raw `RW4xb2` on NHL 09 and 10, RefPack → `RW4xb2` on NHL 11; PS3 portraits are DDS inside RefPack inside BIGF. On Xbox 360 the large portraits live in `data0.big` (NHL 08), `fe.big` (NHL 10), `nocacherender.big` (NHL 12) or `nocache.big` (NHL 13 to Legacy) and the small ones in `cache.big`.

**Head chain:** `rendering/player/player_{headid}.rx2` from NHL 10 (NHL 09 and some NHL 10 heads use `head_{headid}`; PS3 `.rpsgl`), plus `rendering/player/texlib_{headid}.rx2` **from NHL 13**, which every roster head has on NHL 13 to Legacy.

| Title | Players | Portrait archives | Head files | Bespoke | Pool rows | Flag set with file / flag clear with file | Unused portraits / heads |
|---|---|---|---|---|---|---|---|
| NHL 09 | 3 968 | 2 920 | 480 | 256 | 3 712 | 2 705 / 1 | 214 / 129 |
| NHL 10 | 3 933 | 3 637 | 742 | 270 | 3 663 | 2 954 / 4 | 679 / 372 |
| NHL 11 | 5 310 | 5 397 | 821 | 255 | 5 055 | 4 269 / 1 | 1 130 / 397 |
| NHL 12 | 5 395 | 6 158 | 795 | 253 | 5 142 | 4 427 / 1 | 1 730 / 363 |
| NHL 13 | 5 412 | 6 823 | 817 | 268 | 5 144 | 4 298 / **93** | 2 433 / 375 |
| NHL 14 | 5 374 | 7 678 | 841 | 262 | 5 112 | 4 542 / 0 | 3 138 / 400 |
| NHL 15 | 5 534 | 8 362 | 861 | 259 | 5 275 | 4 358 / 0 | 4 004 / 421 |
| Legacy | 5 632 | 9 224 | 904 | 246 | 5 385 | 4 376 / **127** | 4 724 / 457 |

The portrait flag contradicts the files on NHL 13 and Legacy (93 and 127 rows flagged clear have a file; two Legacy rows are flagged with none). **Never derive the flag from the files or the reverse.** Use three classes: bespoke, pool, and other (head id 0 or below the era floor), and exclude the four silhouettes from bucket counts.

Database head tables: `stockplayerheads.headid` (hair-card and haircap fields) covers every roster head on NHL 12, 13 and Legacy, misses a few on NHL 14 and 15, and is absent before NHL 12; `stockplayerhelmetplacement.headid` exists from NHL 11 with 9 to 37 roster ids lacking a row. How either is consumed is open. PS3 NHL 10 to 12 add per-head colour textures (`head_<id>_cm`, `hair_<id>_cm`) that Xbox 360 embeds.

**NHL 07 and 08:** bespoke heads are `head_<id>` meshes with per-head textures. On NHL 07, 950 of 2 250 bio rows have no head file anywhere (949 with head id equal to art id, mostly skin colour 1 or 2); on NHL 08, 336 rows in ids 3 000 to 4 429. No generic family covers them; a generic face chosen by skin colour is the hypothesis, untested.

**Layout by era:** hair ships as separate `hair_{id}_0_0_0.rx2` meshes on NHL 08 and 09 and is bundled inside the head (`haircap_`, `cardsa_`, `cardsb_` submeshes) from NHL 10 to Legacy; coach heads exist on NHL 07 to 09 only, staff reuse pool heads afterwards. Create-a-player is **texture-only** (eyebrow, black-eye and haircap swatches, 190 files per title); **facial hair is painted into the head texture** (zero facial-hair submeshes in every sampled decode). Head files per title grow from 441 (NHL 07) to 1 589 (Legacy).

## PS4 (NHL 15 to 21)

| Field | Meaning | Resolves to |
|---|---|---|
| `pPaD` (`headid`) | head | a vault head record → mesh and texture pack |
| `CbNl` (`headtexture`) | head texture set; equals `headid` on every row of NHL 17 to 21, so **the two are written together** | |
| `lOnr` (`artid`) | portrait | `nhl/ui/kettle_assets/playerportraits/p<artid>.gnf` (BC7 512×340); PNG under `ui/assets/portraits/` on NHL 15 |
| `RAXG` (`hairid`) | hair (11 bits) | `hair_<id>.mcd` with hat, toque and face-gear variants and four textures; 86 ids on NHL 21; bespoke-head players always carry 0 because their hair ships in the head set |
| `YQMS` (`eyebrowid`) | eyebrows (7 bits) | `head_eyebrows_<id>_{color,mask,alpha,normal}.gnf` |
| `SvcL` | portrait flag | |
| `KTDQ`, `uGEA`, `YVhA`, `Zufj` | skin, hair, eyebrow and eye colour codes | |
| `ofxH`, `OeNu`, `SZeY` | mascot flag, mascot head and body ids | |

**Head binding** is resolved at the data level: every `ge_nhl_skaterhead` collection and every head texture pack in the [PS4 Attrib/VLT Vault](PS4-Attrib-VLT-Vault) carries one typed enum attribute whose **value is the database `headid`**. The attribute is the binding; the slugified player name in the record is documentation (it matches only 362 of 395 bespoke players because of hyphens, nicknames and typos). The head record references the texture pack (`head_<name>_cm`, `hair_<name>_cm`, a common normal), a helmet-placement collection and the mesh object.

| Title | Vault head records | Database head ids → in vault | Rows with `artid == headid` |
|---|---|---|---|
| NHL 15 | 495 | 439 → 438 | 258 |
| NHL 16 | 544 | 448 → 448 | 247 |
| NHL 17 | 608 | 483 → 483 | 283 |
| NHL 18 | 638 | 492 → 491 (0 = no head) | 339 |
| NHL 19 | 698 | 536 → 535 | 326 |
| NHL 20 | 743 | 590 → 589 | 380 |
| NHL 21 | 766 | 605 → 604 | 394 |

NHL 21 head-id regimes: mascots 0 (44 rows); bespoke 20 to 20 406 (380 rows, 395 ids); pool 60 000 to 60 209 (6 823 rows, 209 ids); generic female 30 100 to 30 120 and referees 31 000 to 31 007 in the registry; 162 vault heads unused by any row; 44 908 ids unoccupied. Head ids are durable: the portrait id is 100% stable for shared players across NHL 17 to 21, the head id identical on 74 to 83% with every change being pool to bespoke.

PS4 portraits **never** flag a row that lacks a file; 1 000 to 1 600 rows per title have no portrait member and all have the flag clear. Coach, entity and mascot heads have their own CSV lists; the Creation Zone list `playerheads.csv` holds 229 rows on NHL 21 (211 male, 18 female) with asset id equal to render id.

## Reassignment contracts (data level; in-game selection open)

- **Portrait:** reassign only to an existing shipped portrait in the title's own tree or archive (stock data already shares a portrait among two or three rows on some titles); keep the id within the declared width; show the flag as stored and never flip it silently; on PS3 Legacy refuse or disclose a target that also exists in `patch.big` with different bytes (23 collisions; precedence open).
- **Head:** require a mesh of that id, plus `texlib_<id>` on NHL 13 to Legacy, a `stockplayerheads` row on NHL 12 to Legacy (six stock exceptions on 14 and 15), the colour companion on PS3 NHL 10 to 12, both mesh and texture on NHL 07 and 08; on PS4 the target must be a vault head value, written together with `headtexture` on NHL 17 to 21. Disclose duplicate faces and pool-to-bespoke class changes. On NHL 21 all 766 vault values are addressable.
- **New heads:** a new archive or container on gen7; on PS4 a new vault collection, **blocked** because collection keys cannot be derived.

Last reviewed: 2026-10-20
