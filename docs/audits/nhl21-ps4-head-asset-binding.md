# NHL 21 (PS4, CUSA18182) — how a player is bound to their head mesh and textures

**Status:** verified against real bytes except where explicitly marked *unresolved*.

**Ground truth:** `anchors/ps4/nhl21/databases/nhl_game.db` (+ `nhl_game-meta.xml`) and the installed
game tree at `H:\Emulators\ShadPS4\Games\CUSA18182` (its `databigs\Extracted\` tree, 169 095 files,
and the decrypted `eboot.bin` ELF).

**Method:** TDB v6 record decode of every `exhibitionplayerbiotable` / `customplayerbiotable` row;
filename census of the extracted asset tree; a first-principles decode of EA's Attrib/VLT asset
registry (`nhl/data/common/database/attrib/`); literal and format-string search of `eboot.bin`;
an exhaustive numeric-table scan of all 12 333 non-texture game files plus the eboot.

---

## 1. Summary of the binding chain

```
player row (ubPc / KdqM)
  |- pPaD  headid       --> head geometry + head texture set   (two regimes, section 3)
  |- CbNl  headtexture  --> head texture set                   (always == pPaD in vanilla, 3.1)
  |- lOnr  artid        --> ui/kettle_assets/playerportraits/p<artid>.gnf   (section 6)
  |- RAXG  hairid       --> rendering/characters/player/hair_<hairid>.*     (section 4)
  |- YQMS  eyebrowid    --> rendering/characters/player/head_eyebrows_<id>_*.gnf (section 5)
  |- KTDQ  skincolor / uGEA haircolorcode / YVhA eyebrowcolorcode / Zufj eyecolorcode
  |- playoffbeard{type,color,start,max,growthrate}  (VpIM/oUIh/NDxv/qgwv/Upwy)
  |- SvcL  hasportrait   (gates the portrait lookup)
  \- OeNu/SZeY mascotheadid / mascotbodyid  (mascot rows only, section 7)
```

The renderer's own property names, recovered from `eboot.bin` (offset `0x50ae233`), confirm the split:
`... HeadGeoId . HeadTexId . MascBodyId . MascHeadId ...`. `HeadGeoId`/`HeadTexId` are the runtime
counterparts of `pPaD`/`CbNl`.

## 2. Where the data lives

| Layer | Location |
|---|---|
| Player record | `nhl/data/common/gamemodes/database/t3db/nhl_game.db` — TDB v6, **little-endian**, 115 tables, full `name=` metadata in `nhl_game-meta.xml` (fields are self-describing: `headid`, `headtexture`, `hairid`, `eyebrowid`, `artid`, ...) |
| Head/hair assets | `nhl/data/kettle/rendering/characters/player/` (34 727 files) — `.mcd` meshes, `.gnf` textures |
| Portraits | `nhl/ui/kettle_assets/playerportraits/p<id>.gnf` |
| Asset registry | `nhl/data/common/database/attrib/` — EA **Attrib/VLT** vault (`gccollections.vlt` 32.6 MB + `gccollections.bin` 25.1 MB string/blob pool, `gcschema.*`) |
| Creation-Zone pick lists | `nhl/data/common/gamemodes/creation_zone_csv/playerheads.csv`, `appearancehairstyles.csv`, `appearanceeyebrowstyles.csv`, ... |
| Non-player entities | `nhl/data/common/gamemodes/kernel_entity_csv/{entityheads,coachheads,bapcoachheads}.csv` |

### 2.1 Attrib/VLT container (decoded here)

`*.vlt` is a chunk chain; each chunk is `tag(4, little-endian FourCC, stored reversed) | size(4,
**includes the 8-byte header**) | payload`:

| Chunk | Meaning |
|---|---|
| `Vers` | version stamp |
| `StrN` / `DepN` | string-pool + dependency descriptors (names the sibling `.bin`) |
| `DatN` | the record data; record payloads start immediately after the chunk header |
| `ExpN` | `count(u32)` then 16-byte entries `key(u32) \| tag(u32) \| size(u32) \| absoluteFileOffset(u32)` — verified: records tile `DatN` with **zero overlaps** |
| `PtrN` | 12-byte fixups `dataOffset(u32) \| type(u16) \| flags(u16) \| offsetIntoBin(u32)` — resolves every string/blob pointer |
| `EndC` | terminator |

`*.bin` is `StrE | size` followed by the NUL-separated string pool and 8-byte key blobs.

Collections are **name-addressed**: each collection has a 48-byte index record holding a pointer to
its name string and a pointer to an 8-byte `(parentNodeKey, collectionKey)` blob. Asset references
inside a collection are the Windows-style paths seen in the pool, e.g.
`rendering\characters\player\paul_stastny.mcd`, `rendering\characters\player\head_paul_stastny_cm`.

## 3. Head id — two regimes

Decoding all 7 247 `exhibitionplayerbiotable` (`ubPc`) rows:

| Regime | `pPaD` range | Rows | Distinct ids | Resolution |
|---|---|---|---|---|
| Mascot | `0` | 44 | 1 | head comes from `mascotheadid`/`mascotbodyid` instead (section 7) |
| **Bespoke** (scanned real player) | `20 ... 20 406` | 380 | 395 | collection `head_<player_name>` — **id→name link unresolved, section 8** |
| **Generic pool** | `60 000 ... 60 209` | 6 823 | 209 | collection `head_generic_player_<headid>` — **direct, verified** |

Additional id namespaces present in the registry but unused by vanilla `ubPc` rows:
`head_generic_female_<30100..30120>` (20) and `head_generic_referee_<31000..31007>` (8).

### 3.1 `headtexture` is redundant in vanilla data

`pPaD == CbNl` for **7 247 / 7 247** rows. The engine keeps them separate (`HeadGeoId` vs
`HeadTexId`), so they *can* diverge — a mod can point geometry and texture at different heads —
but no shipped row does.

`pPaD == lOnr` (headid == portrait/art id) for **394 / 424** bespoke rows, i.e. a bespoke player's
head id *is* their portrait id. Pool-head players always carry a distinct `lOnr`.

### 3.2 Generic pool heads — correspondence verified, *mechanism not*

```
ubPc.pPaD = 60051
  -> VLT collection   "head_generic_player_60051"          (211 collections, 60000-60210)
  -> mesh             characters/player/generic_player_60051.mcd
  -> texture          characters/player/head_generic_player_60051_cm.gnf
```

All 209 pool ids used by the roster are covered by an existing collection. The Creation-Zone picker
is driven by `creation_zone_csv/playerheads.csv`
(`index,identifier,name,filter,assetid,renderassetid,skintone,gender`): **229 data rows = 211 male
(`gender=1`) + 18 female (`gender=0`)**, `assetid == renderassetid` on all 229, spanning
`30100 … 60210`. `index` is only the row ordinal (dense `0…228`, file unsorted) — it is *not* an
id offset. The 211 male rows correspond 1:1 with the 211 `head_generic_player_*` collections.

> **Correction (do not read the id-in-the-name as the binding).** An earlier revision of this
> document called this chain "direct, verified". What is verified is the *correspondence* — a
> collection named `head_generic_player_<id>` exists for every pool id in use. It is **not** verified
> that the engine constructs that name. It almost certainly does not: `generic_player` occurs nowhere
> in `eboot.bin`, and the only other files in the game containing the substring are stadium lightmap
> tables (`ahlgeneric_player_tunnel`, `eurogeneric_playerexit`). The eboot's string table is intact
> and rich (`gccollections`, `gcschema`, `playerheads`, `nhl/data`, 2 013 `%s` and 1 556 `%d` format
> literals), so this negative is trustworthy rather than an artefact of a stripped binary. The id
> embedded in the pool-head name may be pure artist convention; pool and bespoke heads are bound by
> the same unknown mechanism (section 8).

### 3.3 Bespoke heads — asset set

```
mesh      characters/player/<first>_<last>.mcd          (532 name-shaped .mcd files)
textures  characters/player/head_<first>_<last>_cm.gnf  (527)
          characters/player/head_<first>_<last>_nm.gnf  (subset)
          characters/player/hair_<first>_<last>_cm.gnf  (526)
registry  collections  head_<first>_<last>  and  head_<first>_<last>_texpack   (528 pairs)
geometry  the head geometry collection binds skeleton_player.skel and the
          geo_headShape / geo_eyesShape / geo_eyelashesShape / geo_haircapShape sub-shapes
```

The artist naming is free-form and *not* a reliable derivation of the DB name: slugifying
`firstname_lastname` matches only **362 / 395** bespoke players. The misses are hyphen/period/accent
handling (`Marc-Edouard Vlasic` → `head_marc_edouard_vlasic` does not exist), nickname drift
(`Alex Ovechkin` vs `head_alexander_ovechkin`), and outright typos in EA's asset names
(`head_gabriel_lendeskog`, `head_wendel_clarke`, `head_valterri_filppula`). Name matching is
therefore a **heuristic, not the binding**.

## 4. Hair — direct id link (verified)

`ubPc.RAXG hairid` → `characters/player/hair_<hairid>.*`:

```
hair_41.mcd                  base mesh
hair_41_hat.mcd              hat-compatible variant
hair_41_toque.mcd            toque variant
hair_41_facegear.mcd  hair_41_hat_facegear.mcd  hair_41_toque_facegear.mcd
hair_41_aa.gnf  hair_41_ab.gnf  hair_41_color.gnf  hair_41_reflectance.gnf
```

86 hair ids exist (`0..~62` plus a `1000..1012` block). The registry additionally carries 3 010
`hair_<id>_<headgear>` permutation collections (`hair_41_hatcadet_strapback_halfmask`, ...) that pick
the correct hair cut for the worn headgear.

**Bespoke-head players always carry `hairid = 0`** (424/424) — their hair ships inside the bespoke
head asset set (`hair_<player>_cm.gnf` + the head's `geo_haircapShape`), not from the shared pool.
Pool-head players use 57 distinct hair ids (most common: 28, 45, 15, 2, 57).

## 5. Eyebrows — direct id link (verified)

`ubPc.YQMS eyebrowid` (observed 0-8) → `characters/player/head_eyebrows_<id>_{color,mask,alpha,normal}.gnf`
(ids 0-10 plus 100-103 present). Registry: `head_eyebrow_custom_<id>_texturepack` (15).
Tint comes from `YVhA eyebrowcolorcode`.

## 6. Portraits (verified)

`ubPc.lOnr artid` → `nhl/ui/kettle_assets/playerportraits/p<artid>.gnf`, gated by `SvcL hasportrait`
(1 for 6 469 rows, 0 for 778). HUT/legend card art uses the same id space under
`ui/kettle_assets/cards/{legend,master,heroes}/p<id>.gnf`.

## 7. Mascots and non-player entities

* Mascot rows: `pPaD = 0`, `ofxH ismascot = 1`; appearance comes from `OeNu mascotheadid` /
  `SZeY mascotbodyid`, listed in `creation_zone_csv/{mascothead,mascotbody}.csv` and rendered from
  `rendering/characters/mascot/`.
* Coaches: `coachhead_<name>.mcd` + `coachhead_<name>_{cm,sm}.gnf` and generic
  `coachhead_generic_<id>` / `coachhead_male_<id>`; driven by `kernel_entity_csv/coachheads.csv`
  and `bapcoachheads.csv` (`HeadId,PortraitId,SkinTone,IsFemale,BodyId`, 107 rows).
* Generated entities (BaP, staff): `kernel_entity_csv/entityheads.csv`, same 5 columns, HeadId 0-106,
  PortraitId 1000+.

## 8. Unresolved: the bespoke `headid` → `head_<name>` collection binding

For pool heads the id is embedded in the collection name, so the engine can build the name directly.
For the 395 bespoke head ids **no id→name binding was found anywhere in the shipped game data.**
What was checked and came back negative:

1. **No bespoke head id appears as a string** anywhere in `gccollections.bin` (297 675 strings) — the
   only numeric-suffixed collection prefixes in the whole registry are `head_generic_player`,
   `head_generic_female`, `head_generic_referee`, `coachhead_male`, `coachhead_generic`, `glass`.
2. **No numeric attribute** on the `head_<name>` collections. Their index records are pure
   name→key entries (48 bytes: key, type, name pointer, `(parentNode, collection)` blob pointer);
   only 55 head collections carry any attribute-override record at all, and none holds an id.
3. **The eboot builds no head asset names.** `eboot.bin` is a decrypted ELF with readable strings, but
   contains zero occurrences of `generic_player`, `skaterhead`, `playerportrait`, `texpack`, or any
   `head_%...` format string. (It does contain one built collection name for a different subsystem —
   `ge_nhl_customlogo` / `customlogo_common_1991140` — proving such literals *would* be visible.)
4. **No lookup table exists in any file.** A density scan for the 163 distinguishable bespoke ids
   (>= 8000) as 16-bit LE values across every non-texture file in the game plus the 104 MB eboot
   returned only mesh-vertex and Lua-bytecode false positives — no table.
5. **Only `gccollections` mentions heads at all** among the 300+ attrib files; the other numbered
   `*.vlt`/`*.bin` pairs are stadium data.
6. **No engine-side name construction, for either regime.** See the correction in 3.2 — the eboot
   builds no head collection name, generic or bespoke.
7. **No head list or index-ordered array anywhere in the registry.** Checked both representations:
   runs of consecutive inline `(nodeKey, collectionKey)` pairs in `gccollections.bin` (0 runs of
   length >= 8), and records whose *pointer* targets are head-collection blobs in `gccollections.vlt`
   (901 records reference a head collection; **every one is 1:1**, i.e. a collection's own index
   record). Nothing enumerates heads in id order.
8. **The collection key is not a function of the head id.** Recovered `(id, parentNode, collectionKey)`
   for all 211 pool heads: keys are non-linear in the id (210 distinct deltas across consecutive ids)
   and are not `crc32` or `fnv1a` of the collection name (0/211 for each). The keys look like EA's own
   name hash, which cannot be reconstructed from the id alone.

### 8.1 Bespoke head ids are a durable identity, not a per-build slot index

Cross-title comparison against the PS4 anchors, matching players by `(firstname, lastname)`:

| vs NHL 21 | shared players | `headid` identical | `artid` identical |
|---|---|---|---|
| NHL 17 | 291 | 214 (74 %) | **291 (100 %)** |
| NHL 18 | 303 | 233 (77 %) | **303 (100 %)** |
| NHL 19 | 356 | 297 (83 %) | **356 (100 %)** |
| NHL 20 | 356 | 297 (83 %) | **356 (100 %)** |

Classifying the NHL 20 → NHL 21 `headid` drift: 297 identical, 59 changed, and **all 59 are
pool → bespoke transitions** (a player who used a generic head got scanned). There are **zero**
bespoke → different-bespoke renumberings. Combined with `artid` being 100 % stable across five
titles and `pPaD == lOnr` on 394/424 bespoke rows, the conclusion is that a bespoke head id **is**
EA's durable global player-art id — the same number that names `playerportraits/p<id>.gnf`.

Consequence for modding: **there is no id namespace to invent.** Every player in the DB already
carries a unique, stable art id in `lOnr`. The open problem is purely how to *register* a head asset
against an id, not how to choose one.

### 8.2 What this means for asset work

`headid` must be treated as an **opaque but stable key**: safe to read, display and *reassign among
values already known to resolve*, but not to synthesise. Concretely:

* **Known-good addressable head slots: 606.** 395 bespoke ids in live use + 209 pool ids in live use
  + 2 pool slots that no player uses (`60051`, `60210`). Reassigning players among these and
  replacing the assets behind each is the ceiling reachable **without** solving the binding.
* **Unreachable: the ~133 bespoke head collections that no vanilla row references.** They exist in
  the registry, but their ids are unknown and cannot be derived (section 8, items 7–8).
* **Unreachable: any newly minted id.** Nothing indicates an arbitrary `headid` will resolve.

### 8.3 Head-id namespace ceiling

`pPaD headid` is **16 bits, metadata range `0..65535`, identical on every PS4 anchor (NHL 15–21)** —
65 536 representable values. Occupancy in NHL 21:

| Band | Size | Contents |
|---|---|---|
| `0` | 1 | mascot sentinel |
| `20 – 20 406` | 20 387 | bespoke / EA player-art ids (only 395 used — sparse, but the band grows every title: max 10 391 → 11 145 → 12 826 → 13 433 → 17 374 → 17 374 → 20 406 for NHL 15→21) |
| `30 100 – 30 120` | 21 | generic female |
| `31 000 – 31 007` | 8 | generic referee |
| `60 000 – 60 210` | 211 | generic male pool (209 in use) |

**44 908 ids are unoccupied.** Largest contiguous runs: `31 008–59 999` (28 992), `20 407–30 099`
(9 693), `60 211–65 535` (5 325), `30 121–30 999` (879). Extending the generic pool upward from its
current top gives **5 325 additional slots**; the whole free space comfortably exceeds the 7 247
player rows in `ubPc`.

The namespace is demonstrably not frozen: EA grew the male pool from a `60 182` ceiling in NHL 15 to
`60 210` in NHL 16 and held it there through NHL 21.

Two adjacent fields cap *before* `headid` does if unique per-player hair is also wanted via the
generic route: `RAXG hairid` is 11 bits (`-1..2046`, 86 assets exist) and `YQMS eyebrowid` is 7 bits
(`0..127`, 15 assets). Bespoke heads sidestep both — every bespoke row carries `hairid = 0` and ships
its hair inside the head asset set.

**These are addressable ids, not resolvable ones.** Only 606 are known to resolve to an asset
(section 8.2); every id beyond that depends on the unsolved binding *and* on being able to add
collections to `gccollections`. Any engine-side array cap on head ids is unknown and only the
disassembly in 8.4 can rule it in or out.

### 8.4 Remaining avenue

Data-side investigation is exhausted — eight independent probes, all negative. The one live avenue
left is code-side: **disassemble the `HeadGeoId` consumer in `eboot.bin`** (decrypted FreeBSD x86-64
ELF; the render-entity property table at `0x50ae233` naming `HeadGeoId`/`HeadTexId` is the entry
point) and read how the collection key is obtained from the id.

Scope note: if that path turns out to be a compiled table or hard-coded logic inside the eboot, then
minting new head slots requires **eboot patching**, which the roadmap currently lists as
deferred/out-of-scope, and which is PS4/ShadPS4-specific rather than portable across the title range.
That trade should be decided before any large cyberface programme is committed to.

## 9. Reproduction

The decode used a standalone TDB v6 reader (file header `0x18`, table header `0x28`, 16-byte field
descriptors, LSB-first bit packing for little-endian) matching `crates/tdb-core`, plus an Attrib/VLT
chunk walker implementing section 2.1. Both are ~120 lines and were run against the paths named at
the top of this document; no game file was modified.
