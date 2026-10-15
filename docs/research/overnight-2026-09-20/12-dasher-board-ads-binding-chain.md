# Handoff R19 (rev. 2, corrected per Codex review F1/F2) — Dasher-board ads (PS4 15–21): the parsed vault structures around `<arena>_rink_boards`, the name associations that connect them, and the consumer readings that remain inferred

Research lane, 20 September 2026; **rev. 2** applies the coordinator review of the same day
([docs/validation/research-r19-r20-review-2026-09-20.md](../../validation/research-r19-r20-review-2026-09-20.md), F1/F2): the earlier "complete
static + runtime binding chain" wording is withdrawn — every record here was inspected *statically*; the components are runtime-*named*, no
runtime behaviour was observed, and the array-index reading is a candidate, not a traced consumer. Phase 5.2 boards; replaces the "binding
not decoded" state of handoff [11](11-arena-presentation-packs-and-boards-status.md) §3 with parsed references (Q-R18-1 → Q-R19-1). Claim `research-r19-board-ads-chain`. Consumer: developer 5.2 (boards / rink-side ads), 2.5 (arena presentation). Nothing here is
game-verified; nothing written; no launch; **no writer proposed** — §5 separates the read-only listing (developer-ready) from the candidate experiment (not a contract).

Machine-readable: [`data/r19-board-ads-chain.json`](data/r19-board-ads-chain.json) — per PS4 title: every `_rink_boards.mcd` model with its
ad-slot set and material textures, every `ge_imagegroup` with keys and decal list, every `<arena>_rink_boards` game object with its default
list and override groups, the length-consistency and sharing censuses, and (rev. 2) the `adboard_*` building-signage sharer census with typed
edges. Every record carries its vault key; the vault identity is the SHA-256 of `gccollections.{vlt,bin}` per title. Probe:
`r19_board_materials.py` rev. 2 (≈ 4 min for all titles). **Evidence classes used below:** *parsed* = typed RefSpec pointer resolved against
the vault record index; *name association* = equal strings, no pointer; *candidate reading* = an interpretation consistent with the parsed
data whose consumer (engine code) has not been traced.

## Plain-language summary

The rink boards' advertising is **not** painted into the board mesh or its materials. Each arena's board mesh has 38–42 numbered ad
materials (`boards_ads_mat_0 … _39`, one per panel), and every one of them points at the *same* blank base textures plus a transparent
"decal" slot. The actual sponsor logos are separate small textures (`common/boards_ads_decal_<sponsor>_diff`, 35 → 107 per title, incl.
every European league's set). The vault holds, on each arena's boards scene object, (1) a **decal-texture array** whose length equals
the arena's panel count (NHL 21: panel count + 1, last entry the blank base texture) and (2) **override groups** of decal textures keyed by
a game-mode number (and, from NHL 21, a board-variation number). The *parsed* facts are the arrays, their lengths and the records they
point at; that "element *i* is panel *i*" and that the game-mode groups "swap the panels" are **candidate readings** — the code that reads
these arrays was not traced, and a preload list, a remap or another selection rule is not excluded by matching lengths or by repeated
entries. On NHL 16–21 the array record is *shared*: on NHL 21 one record is reachable from 37 rinks (all 32 NHL arenas + Threes), which
matters for any future edit whatever the consumer turns out to be. The earlier `adboard_*` textures (handoff 11) turned out to be the *building* signage (stands, roof, scoreboard, VIP
boxes) bound through ordinary material records (typed edges; sharers now in the JSON). Unknown: the consumer of the arrays (indexing,
selection among groups, whether the decal lands in material slot `8aab7882`), the rule for short groups (a 6-logo group on a 40-panel
rink), and the GameMode / BoardVariation value names — the group families below are inferred from texture names only.

## 1. Static side (observed on all seven titles; identical structure)

`ge_model` (`FileRef` = `rendering\stadiums\<arena>\<arena>_rink_boards.mcd`) → attribute `962826a5` = RefSpec array (`u16 n | u16 n | u32 16 |
n × (class, key, 8 zero bytes)`) → `ge_material` records (`DebugName`, `Effect` → an effect record whose class names the shader) →
`ge_texture` (`FileRef`). For the ad panels:

| Title | boards models | with ad slots | slot counts (models) | max index | ad-material shader | ad-material textures (every ad material, every title) |
|---|---:|---:|---|---|---|---|
| 15 | 32 | 32 | 40 ×21, 38 ×8, 39 ×2, 42 ×1 | 39 ×31, 41 ×1 | `ge_material_enviro_advance_board_reflect` | `diffuseTexture`=`boards_ads_base_diff`, `normalTexture`=`…_base_norm`, `coeffTexture`=`…_base_coeff`, `93cd3f21`=`boards_ads_damage_diff`, **`8aab7882`=`boards_ads_decal_alpha_diff`** (blank decal) |
| 16 | 36 | 36 | 40 ×24, 38 ×9, 39 ×2, 42 ×1 | 39 ×35, 41 ×1 | same | same |
| 17 | 51 | 51 | 40 ×34, 38 ×10, 39 ×3, 42 ×4 | 39 ×47, 41 ×4 | same | same |
| 18 | 61 | 61 | 40 ×38, 38 ×14, 39 ×3, 42 ×6 | 39 ×55, 41 ×6 | same | same |
| 19 / 20 | 76 | 61 (15 outdoor kits: 0) | 40 ×36, 38 ×17, 39 ×2, 42 ×6 | 39 ×55, 41 ×6 | same (+ `…_reflect_glow_lcd` on 36 LED slots, `pbr_enviro_board_hotspot_static`) | same |
| 21 | 78 | 61 (17: 0) | 40 ×36, 38 ×17, 39 ×2, 42 ×6 | 39 ×55, 41 ×6 | same | same + `42fa8da0`=`helmet_cube_map`, `a94ed33d`=`default_white_lm` |

Slot indices run 0…max; 38/39-slot rinks skip a few indices (e.g. NYI/NSH/PIT lack two panels — `ad_slots_missing_below_max` in the JSON),
the six 42-slot rinks are the European arenas (0…41). Boston NHL 21: 65 materials, 40 ad slots 0…39, plus cap/metal/filler/glass/
`boards_ads_glass_1_boston` (the team-specific glass ad, statically bound). **Every ad material on every title binds the same base
textures** (`ad_material_texture_set` in the JSON) — the panel art is therefore not static.

The `common/adboard_*` textures of handoff 11 are bound to 56 `adboard_{basic,glow}_<brand>_<shape>_mat` materials (effect classes
`ge_material_enviro_advance_decal_glow`, `_enviro_advance`, `_enviro_basic`) used by 1–42 *building* meshes each (`_standslower`, `_standsupper`,
`_roof`, `_scoreboard`, `_animatedsigns`, `_vipboxes`, `_banners`; 13 of the 56 unused). That is ordinary static texture replacement (shared
across the arenas that use the material — sharers per material, with model FileRefs and textures, in the JSON `adboard_materials.sharers`,
rev. 2).

## 2. Runtime-named components (parsed statically; all seven titles)

`ge_gameobject` named `<arena>_rink_boards` (attribute `dc0edb22` = name; on 16–21 it is also listed in the `ge_instance` of the same name's
blob RefSpec array at blob+40; on NHL 15 the instance lists other objects, so the chain is driven from the game object) → `Schema` →
`ge_scene_schema` (component RefSpec array `b3b050be`, stored as a 20-byte inline entry the generic decoder skips — read via the record's
pointer slot) → components:

- **`ge_scene_runtimetexturecomponent`** — attribute `7968c7b3` = RefSpec array of `ge_texture` (`de04fabd` = parallel Text array, all null).
  **Parsed:** its length equals the boards model's ad-slot max + 1 on NHL 15–20 (32/32, 35/35, 51/51, 61/61, 61/61, 61/61 arenas whose
  folder has a boards model) and ad-slot max + 2 on NHL 21 (61/61) with `boards_ads_base_diff` last (72/72 lists). Entries repeat (a
  40-entry NHL list uses 13–16 distinct decals, the first 20 entries recur in the second 20). **Candidate reading:** element *i* → material
  `boards_ads_mat_i` (a per-panel default). **Not established:** the consumer; the repetition and length match are consistent with, but
  do not prove, per-panel indexing (a preload list with a separate placement rule, or a remap, would show the same lengths).
- **`ge_scene_runtimetextureoverridecomponent`** — attribute `ImageGroup` = RefSpec array of `ge_imagegroup`.
- `ge_imagegroup` — `ArenaId` (`ArenaId` enum; −2 on 143/147 NHL 21 records, real ids only for four outdoor rinks; attribute absent on 15–19),
  `BoardVariation` (`d79aea8d`, NHL 21 only), `GameMode` (`8e3162a7`), `MatchType` (`6996d852`, never set), `7968c7b3` = RefSpec array of
  decal textures. Lengths: 40/42 (equal to the full panel counts) and 1–14; what either length means to the consumer is not
  established (candidate: full-length = per-panel replacement; short = a set applied by an unknown rule).

| Title | `_rink_boards` game objects | with decal array | with override groups | image groups | distinct decals | decal-array sharing (distinct records / max arenas per record) |
|---|---:|---:|---:|---:|---:|---|
| 15 | 33 | 33 | 32 | 69 | 35 | 33 / 1 (one record per arena; 5 distinct lists) |
| 16 | 36 | 36 | 36 | 78 | 45 | 6 / 31 |
| 17 | 52 | 52 | 51 | 124 | 48 | 24 / 29 |
| 18 | 63 | 63 | 62 | 177 | 53 | 27 / 37 |
| 19 | 78 | 64 | 63 | 183 | 81 | 28 / 37 |
| 20 | 78 | 64 | 63 | 123 | 88 | 28 / 37 |
| 21 | 80 | 64 | 63 | 147 | 107 | 28 / 37 |

NHL 21 sharing (parsed edges game object → schema → component): record `05adb18f` (41 entries: adidas, bauer, bestbuy, brians, ccm, gatorade, sherwood_blk, stx, true, vaughn, warrior,
eanhl, nhl.com, nhl-the-app) is reachable from 37 arenas — all NHL rinks and the Threes tiers; `ahlgeneric` (AHL Live/Store/.com/Authentic, PHPA),
`frontend` (+ NBC), `world_cup_toronto` (Honda/Tim Hortons), `team_canada_training` (Team Canada), each `custom_*` and `eashl*` rink and
each Swiss arena (43 entries) have their own record. Earlier titles differ (NHL 16–18 carry a Canadian/US split with Tim Hortons /
Honda vs NBC lists; NHL 15 had per-arena records with Discover/NHL Network/EB Games) — all in the JSON.

Override groups by (GameMode, BoardVariation) on NHL 21, decal families (the *names in parentheses* are inferred from texture names only;
the GameMode / BoardVariation numbers are the parsed facts): (0,0) 10 groups
brand mix / AHL default ×4; (0,5…14) World-of-CHEL tier boards `boards_sc_{rookie,pro,allstar,superstar,elite}_{en,fr}` (single texture);
(1,0) 63 groups = the European league sets (Red Bull, Dell, Liiga/SHL/NL/DEL/EBEL/ELH …; 2–14 entries, plus 42-entry full sets for the
Swiss rinks); (2,0) 19 groups Honda (HUT); (2,1…4) HUT Rush 3v3/5v5 en/fr (7–8 entries incl. `boards_hutrush_*`); (5,0) 19 groups
Discover / Discover-GC (6 entries, attached to every NHL arena); (6,0) practice tier branding; (7,0) 7 groups Champions Hockey League /
Spengler Cup (42 entries, Swiss rinks); (8,0) one 40-entry AHL set. Each NHL arena's override component holds exactly `(5,0)` + `(2,0)`
[+ `(2,1…4)` on 21]; the Swiss and generic rinks hold the (1,·)/(7,·) sets; `ahlgeneric` holds (0,0) ×4 + (6,0) + (8,0).

## 3. Reconciliation with earlier findings

- Handoff 11 §3 "material → `adboard_*` binding not decoded": **resolved at the reference level** — `adboard_*` is building signage (typed
  material edges, §1); the dasher panels' materials carry no decal art, and the `boards_ads_decal_*` textures are referenced from the
  runtime-named components (§2) whose consumer is not traced. Handoff 11 §1's "one dasher pack" (`ge_nhl_stadiumdashertexturepack`) is the
  colourable kicker/dasher *strip* (`boards_dasher_kicker_colorable_*` recolour textures), not the ad panels.
- Handoff 07 (2026-09-19) §2's typed `ge_level` slots stay null; the boards chain does not pass through `ge_level` at all (level ↔ game object
  association is by name only, unchanged).
- Handoff 10 §3's 5.2 next step ("decode the `ge_level` banner/dasher pack classes") is superseded by this chain.

## 4. Unknowns (explicit)

1. **The consumer of the arrays** — whether the default list is indexed per panel at all, or preloaded and placed by another rule;
   the `de04fabd` Text arrays are null on every group and component, so no in-vault label helps. 2. How a short override group (1–14
   entries) relates to 38–42 panels. 3. GameMode / BoardVariation value names (the schema carries only the enum type names). 4. Whether
   the runtime writes a decal into material slot `8aab7882` (consistent with its `decal_alpha` default; not observed). 5. Precedence
   among groups sharing a key (four (0,0) groups on `ahlgeneric`). 6. Runtime acceptance of any edit (card R19-ADS-1, rev. 2).

## 4a. Consumer / indexing investigation after the review (string presence and schema text only; no code traced)

- **NHL 21 `eboot.bin`** (CUSA18182, 104 845 239 B) contains the attribute names `ImageGroup`, `BoardVariation`,
  `RuntimeTextureOverrideComponent`, `RuntimeTextureComponent` and the suffix `_rink_boards` (once each) and, next to `ImageGroup`, the
  shader-feature strings `NhlStadiumRenderComponent::StadiumBoardWrapFeature::BoardsWrapCBuffer` / `BoardsCB` / `TextureSwapEnabled`.
  That is *string presence*: it shows the arrays are consumed by engine code with a "board wrap" render feature nearby; it does not give
  the indexing rule. No `boards_ads_mat_%d`-style format string exists in the executable (0 hits for `boards_ads`, `ads_mat`, `_mat_%d`).
- **Scenario schema** (`nhl/data/common/gamemodes/service-defs/data/scenario/info.json`, NHL 21): the `ArenaHotSpot` customisation block
  carries `BlueLineAd1..2`, `CenterAd1..4`, `FaceoffCircleAd1..4`, `IcingLineAd1..4`, `NetAd1..2`, `ShowHondaAds`, `ShowVerizonAds` and
  `ArenaBoardVariation` (integer, optional, "for seasonal championships finals for the Home Team"); the eboot also lists
  `ArenaBoardVariation` among the HUT game-setup parameters. So on NHL 21 `BoardVariation` is a **per-game input** chosen by a mode
  service, not a static arena property — consistent with the `(0,5…14)` `boards_sc_<tier>_<lang>` and `(2,1…4)` HUT-Rush groups, and it
  explains why every NHL arena carries those groups. The value → group rule is still the engine's.
- **GameMode value names:** not present in the schema, the vault, the Lua enum files (`backgroundenums.lua` is a UI background table) or
  as `EA::AttribSys::Enums::GameMode::<name>` strings in the eboot (0 hits). The numbers stay numbers.
- **Conclusion:** static data does not resolve the indexing or the mode mapping; the corrected card R19-ADS-1 (rev. 2, §6) is the
  discriminating step and is deferred.

## 5. What is developer-ready now, and what is not (no writer; nothing accepted as an editor contract)

- **Read-only listing (developer-ready, source-bound):** per arena the model's ad materials (count, indices, textures), the decal
  array (record key, heap pointer, entries in file order as texture paths — labelled "entry *i*", not "panel *i*"), the override groups
  (keys, entries) and the sharers of each array record (NHL 21: 37 rinks on `05adb18f`). All of it is in the JSON with vault keys.
- **Decal texture replacement** (`common/boards_ads_decal_<x>`): the texture is an ordinary `ge_texture` FileRef; a replacement affects
  every array and group entry that references it (disclose) — its *visible* effect still depends on the untraced consumer.
- **Same-length RefSpec-element edit of the array:** a *candidate experiment* only (card R19-ADS-1); **not** an accepted per-panel editor
  contract. Any future writer proposal must carry every affected record (array owner, sharers) with source offsets.
- **Building signage** (`adboard_*`): material → effect → texture edges are typed and the sharer census is now reproducible (JSON
  `adboard_materials.sharers`: 43 used / 13 unused materials on NHL 21); a texture replacement there is the ordinary shared-texture case.

## 6. Reproduce

```
python local/runs/overnight-research-2026-09-20/probes/r19_board_materials.py docs/research/overnight-2026-09-20/data/r19-board-ads-chain.json   (≈ 4 min)
```
Executed (read-only) on `H:\Emulators\ShadPS4\Games\<CUSA…>` extracted trees (identities = `vault` SHA-256 of `gccollections.{vlt,bin}` per
title in the JSON); rev. 2 regenerated the JSON (probe docstring and `note` now match this handoff: no `ge_level` pointer chain is claimed —
level ↔ game object is a name association; the array is not called a preload set or an assignment). Session-only checks (not in the
probe): the heap owner attribution that located `ge_scene_schema`/`ge_gameobject`, the DB metadata grep for ad/board/sponsor fields (none;
only `boardtrimzone1..3` and scoreboard zones), the non-texture tree grep for board-ad identifiers (none outside vaults). **Not executed:**
launches, writes, Studio binaries.

**Card R19-ADS-1, rev. 2 (NHL 21 PS4, deferred; no launch or vault write is authorised by this research):**
- *Baseline:* stock `gccollections.{vlt,bin}` (SHA-256 in the JSON, title NHL 21 CUSA18182 extracted tree); one Play-Now game in Boston
  and one in a rink whose array record is **not** `05adb18f` (e.g. `custom_detroit`, own record) — photograph the full dasher circuit
  from the broadcast camera at the opening faceoff and record which sponsor appears on each visible panel, starting at the home-bench door
  and going clockwise (physical panel order is *not* known from the parsed labels; the photos define it).
- *Treatment A:* in a scratch copy, swap entries 0 and 1 of `05adb18f`'s `7968c7b3` array (two 16-byte RefSpecs; sharers listed in the
  JSON). *Treatment B:* the same swap on `custom_detroit`'s own array record. Confirm the active source (the copy the emulator loads —
  path and hash) before each run; roll back to stock after each run and re-verify the hash.
- *Observations:* A/Boston — which panels (if any) changed relative to the baseline photos; A/custom_detroit — whether anything changed
  (sharing control: it must not); B/custom_detroit — which panels changed; B/Boston — must not change.
- *Bounded verdicts:* changes in A/Boston and B/custom_detroit at exactly two panels each, none in the controls → the array is consumed
  per entry and the two records are the sources for those rinks (indexing rule still only partly known: which two panels). Changes at more
  than two panels or in a control → sharing/consumption differs from the reading. No change anywhere → *not* proof the array is unused:
  alternatives remain (an override group active for the mode played, a cache of the stock vault, a treatment the loader rejected, the
  entries not being the rendered decal) — next control: repeat with a *different* game mode and with the two entries replaced by a decal
  absent from every group (e.g. `boards_ads_decal_team_canada_diff`).
