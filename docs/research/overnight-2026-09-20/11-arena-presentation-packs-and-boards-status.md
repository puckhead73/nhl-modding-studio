# Handoff R18 — Arena presentation (PS4 15–21): which stadium packs are keyed by arena, team or catalogue id; what the numbered stadium vaults actually hold; where the boards live

Research lane, 20 September 2026 (Phase 2.5 / 5.2 follow-up after handoffs 07 and 10). Claim `research-r18-arena-packs`. Consumer:
developer 2.5 (arena details / ice), 5.2 (boards), the Creation-Zone presentation-id fields of handoff [06](../overnight-2026-09-19/06-ps4-team-identity-and-presentation-map.md)
§5. Nothing here is game-verified; nothing written; no launch; **no writer proposed**.

Machine-readable: [`data/r18-arena-packs.json`](data/r18-arena-packs.json) — per PS4 title: every `ge_nhl_stadium*` / banner / jumbotron /
lighting / conversion-kit class with its typed attributes, records, key-enum value sets, DB `arenaartid` coverage for the `ArenaId`-keyed
classes, examples with texture paths. Probe: `r18_arena_packs.py` (≈ 10 min).

## Plain-language summary

Handoff 07 (2026-09-19) found ~100 typed slots on each arena record for banners, ads, dashers, chairs and kits and left most of them
null. Tonight's census shows why: those slots are unset in the main registry (0 resolved on every level for every one of those
classes), and the packs are keyed on *their own* attributes instead — crowd chairs and scarves by `ArenaID` (55 packs, covering 43–44
of the 62 NHL 21 arena art ids; the rest share generic chairs), goal-light stanchions / dressing kits / net packs / lighting packs by
`ArenaID` for a handful of arenas, centre-ice logos and spotlight logos by `Team`, and the Creation-Zone catalogue packs (centre ads,
jumbotrons, stanchion materials, goal lights, entrance props, netting colour, dasher) by their own enum — the same numbers the team
row's presentation fields carry from NHL 18 on, which the developer already validates against catalogues. The per-stadium numbered
vaults are lighting data only (light rigs and 755 lightmap textures for Pittsburgh), not board art. The boards themselves are
per-arena meshes (`<arena>_rink_boards.mcd`, 61 of 80 stadium folders) whose material → texture binding is not in the mesh's strings
and not in the vault packs (one generic `dasher` pack) — the 63 shared `common/adboard_*` textures are bound by a mechanism not yet
located, which is the specific next step for 5.2.

## 1. Pack classes and their keys (observed; NHL 21 unless stated; other titles in the JSON)

| Class | Records (15 → 21) | Key attribute (type) | Coverage / notes |
|---|---|---|---|
| `ge_nhl_stadiumchairtexturepack` | — / — / 53 / 55 / 55 / 55 / 55 | `ArenaID` (`ArenaId`) | 44 / 62 DB arena art ids on 21 (38 / 39 on 17, 43 / 52 on 18, 43 / 60 on 20); the 11 unkeyed values are shared/frontend (`crowdchair_frontend` = 100) |
| `ge_nhl_stadiumchairscarftexturepack` | — / — / 54 / 55 … | `ArenaID` | 43 / 62 on 21 |
| `ge_nhl_stadiumdressingkit` | — … 21 / 21 / 21 / 25 (18–21) | `ArenaId` + `ConversionKitTier` + `Group` ref | 7–8 arenas (Threes / EASHL tiers, custom arenas), not NHL rinks |
| `ge_nhl_stadiumgoallightstanchion` | 13 (17) → 20 | `ArenaId` + `GoalLight` | 11–13 arenas; the other arenas use the catalogue `GoalLight` value |
| `ge_nhl_stadiumnetpack`, `ge_nhl_stadiumcrowdsecondarypack`, `ge_nhl_lightingpack` (19+/20+) | 9 / 6 / 12 | `ArenaId` + `RinkId` | 1–4 arenas (outdoor / Threes rinks) |
| `ge_nhl_stadiumcenterlogotexturepack` | 221 → 348 | `CenterLogo` (`Team`) | handoff 07 of 2026-09-19 (212 / 213 … 273 / 312) |
| `ge_nhl_stadiumspotlightlogotexturepack`, `ge_nhl_stadiumbapteamtexturepack` (21) | 4 / 32 | `Team` | spotlights for 4 teams; BAP locker-room logos for the 32 NHL clubs |
| `ge_nhl_stadiumcenteradtexturepack` | 147 → 219 | `CenterAd` + `AdSize` (catalogue enums) | matches the team row's centre-ad presentation fields (handoff 06 §5) — validation input |
| `ge_nhl_jumbotron` (16+), `ge_nhl_stadiumstanchionmaterial`, `ge_nhl_stadiumegoallight`, `ge_nhl_stadiumentranceprop`, `ge_nhl_stadiumnettingtexturepack`, `ge_nhl_stadiumdashertexturepack` | 10 / 3 / 3 / 15–20 / 2 / **1** | `Jumbotron`, `Stanchion`, `GoalLight`, `EntranceProp`, `NettingColor`, `Dasher` | the Creation-Zone catalogues (`scoreboard2` ↔ `Jumbotron`, `stanchionmaterial` ↔ `Stanchion`, …, handoff 06 §5); value-overlap joins to the team row's small-integer fields are non-discriminating (recorded, not used); **one** dasher pack → the boards are not per-arena vault packs |
| `ge_nhl_scbannertexturepack` (21), `ge_nhl_eashlbannertexturepack` (16+) | 40 / 8 | `ScBannerSeason` / `ScBannerRank` / `BannerLanguage`, `BannerType` | Stanley-Cup / EASHL banners, language-keyed (French twins) |
| `ge_level` slots for all of the above | 76 levels × ~100 slots | typed-null | **0 resolved** on every level in `gccollections` (measured) |

## 2. Numbered stadium vaults = lighting (observed on NHL 21 `20823504` = `custom_pittsburgh`)

Class census: `ge_texture` 755, `ge_linelight` 390, `ge_boxlight` 72, `ge_basicspotlight` 6, `ge_instance` 4, `ge_gameobject` 3,
`ge_lighting` 1; all 755 textures are `rendering\stadiums\custom_pittsburgh\lm_*` lightmaps (incl. `lm_…_prop_tunnel_adboard_85…95`).
So the association of handoff 07 §2 (vault ↔ level by name) is the **lighting** binding; no board/ad art, no arena-identity data.

## 3. Boards (5.2) — inventory and the missing link (observed)

> **Superseded by handoff [12](12-dasher-board-ads-binding-chain.md)** (same day): the dasher panels' art is a runtime decal list, the `adboard_*` textures are static building signage. The text below is the pre-decode state, kept as written.

`kettle/rendering/stadiums/`: 80 stadium folders; **61** carry `<arena>_rink_boards.mcd` (Boston 5.8 MB; also `_rink_glass`,
`_rink_netting`, `_banners`, `_animatedsigns`, `_scoreboard`, `_props`, `_railings`, `_stands*`, `_vipboxes`, `_zamboni`, `_roof` meshes and
a `.skel`), plus per-arena `lm_<arena>_adboard_N.gnf` lightmaps; `common/` holds 63 shared `adboard_*` textures (47 `_diff`, 7 `_norm`, 7
`_coeff`: `adboard_basic_{bauer,sherwood,vaughn,warrior}_long_*`, `adboard_bauer_{67x46,long,short,square}_diff`, `adboard_bestbuy_square`,
`adboard_brians_long_basic`, `adboard_adbd_swiss`, …). The board mesh's readable strings are shape names only
(`door_penaltybox_innerboard_1Shape`, …) — no texture paths — so the material → `adboard_*` binding lives in the MCD material blocks
(Studio's `tdb-rx2::mcd` reads the mesh; its material section was not decoded here) or in a material collection of the vault
(`ge_material_*` classes, 35 + 15 + … records, not decoded). **Next discriminating step:** decode one `_rink_boards.mcd` material table
(or the `ge_material_pbr_*` records) for Boston and check whether the referenced texture names are the `common/adboard_*` set; until
then 5.2 is "assets located, binding not decoded". **Interim observation (session, NHL 21):** the vault's string pool holds 634
`adboard*` strings, among them material names (`adboard_basic_bauer_square_mat`, `adboard_basic_timhortons_square_mat`, `adboard_glow_swiss_legacy_mat`,
…); 95 `ge_texture` records carry an `adboard` `FileRef`; their `(class, key)` references occur 236 times in the `.bin` heap (instance
data, not inline entries) and 0 times in `.vlt` inline entries — i.e. materials reference the ad textures through their instance blobs
(my blob-range mapping did not attribute them to records: the material instance layout exceeds the schema `instance_size`, or the
references sit in array attributes — unresolved); of the material collection keys, 3 (of the first 5 000 tested) occur as 32-bit words
in `boston_rink_boards.mcd`, no material *name* does. Next step stands: decode the MCD material table.

## 4. Contract input (proposal)

- Arena-keyed packs (chairs, scarves, kits, stanchion lights, nets): replace textures per arena through the pack's `ArenaID` record
  (paths in the JSON examples); an arena without a keyed pack uses a shared one — disclose, do not synthesise a record.
- Catalogue-keyed packs (centre ads, jumbotron, stanchion material, goal light, entrance prop, netting, dasher): the team row's
  presentation field selects the catalogue value; the pack per value holds the art — replacement is per value (shared across every
  team that selects it, disclose the sharers).
- Boards: blocked on the material binding (§3).
- Runtime: unchanged (nothing observed); a card only after the binding is decoded.

## 5. Unknowns (explicit)

1. Board material → texture binding. 2. The team-row presentation fields' exact enum per field (value overlap is non-discriminating;
handoff 06's catalogue validation stands). 3. Numbered-vault naming key (unchanged). 4. Runtime.

## 6. Reproduce

```
python local/runs/overnight-research-2026-09-20/probes/r18_arena_packs.py docs/research/overnight-2026-09-20/data/r18-arena-packs.json   (≈ 10 min)
```
Executed once (read-only) plus the session checks: `ge_level` reference census (0 resolved pack slots), the Pittsburgh vault class census
(a numbered-vault reader variant of `vlt_collections.Vault`, scratch, deleted), the `_rink_boards.mcd` string scan and the stadium folder
listing. **Not executed:** launches, writes, Studio binaries.
