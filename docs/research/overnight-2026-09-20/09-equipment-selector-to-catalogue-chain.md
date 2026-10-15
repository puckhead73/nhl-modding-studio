# Handoff R16 — Equipment (PS4 15–21): the database selector → vault catalogue → texture chain for sticks, gloves, helmets, skates, visors, tape and goalie gear

Research lane, 20 September 2026 (Priority 4 of the overnight campaign: 4.1–4.5). Claim `research-r16-equipment`. Consumer: developer
Phase 4 (catalog → label → DB assignment → geometry/material/texture → thumbnail → archive → runtime). Builds on handoff
[07](../overnight-2026-09-19/07-ps4-vault-collections-and-slug-binding.md) (reader) and today's [08](08-players-head-hair-binding-resolved.md).
Nothing here is game-verified; nothing written; no launch; **no writer proposed**. Gen7 equipment (handoff [30](../overnight-2026-09-18/30-phase3-4-portraits-equipment-thumbnails-xbox360.md)
of 2026-09-18) is not re-measured; this handoff is PS4-only.

Machine-readable: [`data/r16-equipment-chain.json`](data/r16-equipment-chain.json) — per PS4 title: every equipment vault class with its
typed attributes, enum value sets (with the collection names per value and texture paths on examples), every selector field of
`exhibitionskaterequipment` / `exhibitiongoalieequipment` with its value histogram, and the joins (semantic candidate by type name +
value coverage; value-only candidates; ambiguity flag). Probe: `r16_equipment_chain.py` (≈ 15 min for seven titles).

## Plain-language summary

Equipment on PS4 is selected per player by small numbers in the equipment tables (`stick`, `gloves`, `helmetbrand`, `skatebrand`,
`visortype`, tape styles; goalie `maskid`, `masktype`, `stick`, `blocker`, `trapper`, `pads`). The vault's equipment collections carry
typed enum attributes with the same meaning — `StickBrand`, `GloveBrand`, `HelmetBrand`, `SkateBrand`, `HelmetVisorType`,
`StickBladeTapeStyle`, `GoalieMaskTexture`, `GoalieStickBrand`, `GoalieBrands` — and every value a player uses exists in the catalogue on
every title (one sentinel `99` on `helmetbrand` from NHL 17, four mask ids on NHL 16, one on NHL 17 excepted). Each catalogue value
resolves to a named collection with its textures, so the chain database-number → collection → texture files is parsed, not guessed,
for all categories. The catalogues are larger than what players use (e.g. 233 stick brands, 51 in use on NHL 21): those spare entries
are the "existing entries" a player can be reassigned to without any asset work; a *new* brand means a new vault record — still
blocked on the vault writer. Geometry (which mesh a brand uses) is one reference further (`ge_gameobject` / model classes), not
measured here; thumbnails and the Creation-Zone catalogue text are separate registries (handoff 06 of 2026-09-19).

## 1. The chain per category (observed; NHL 21 counts, other titles in the JSON)

| Category | DB selector (`exhibitionskaterequipment` / `…goalieequipment`) | Vault enum type (class) | DB distinct → in catalogue | Catalogue values | Example collection → textures |
|---|---|---|---|---|---|
| Stick | `stick` | `StickBrand` (`ge_nhl_skatersticktexturepack`; also `StickHandedness`) | 51 → 51 (15: 19 / 20, missing 71; 16–20: all) | 233 (32 → 233 across 15 → 21) | `playerstick_right_nhl_buffalo_texpack` → `playerstick_nhl_buffalo_cm` |
| Stick tape | `stickbladetapestyle`, `stickshafttapestyle` | `StickBladeTapeStyle` / `StickShaftTapeStyle` (tape texture packs; + `OutdoorVariation`) | 5 → 5, 3 → 3 | 5, 3 | `playersticktape_blade_graphic2_smalltoe` |
| Gloves | `gloves` | `GloveBrand` (`ge_nhl_skaterglovestexturepack`) | 27 → 27 | 65 (22 → 65) | `glove_sherwood_ek15_texpack` |
| Helmet | `helmetbrand` | `HelmetBrand` (`ge_nhl_skaterhelmet` 350 records = brand × visor/flags; `…helmettexturepack` 554 = brand × `League` logo variant) | 12 → 11 (**99** unmatched on 17–21: sentinel, inferred) | 25 (13 → 25) | `helmet_jofaroman1_finnish_texturepack` |
| Visor | `visortype` | `HelmetVisorType` (on `ge_nhl_skaterhelmet`); `VisorColor` (`…visortexturepack`, 15 colours) | 6 → 6 | 7 | `helmet_visor_texturepack_visor_red` |
| Skates | `skatebrand` | `SkateBrand` (`ge_nhl_skaterskates` 585 records = brand × `SockType` × `PlayerType`; `…mergedskatestexturepack` 147) | 17 → 17 | 147 (14 → 147) | `merged_skate_od_graphic30_texpack` |
| Skate guards / sock tuck | `skateguard`, `skatesocktuck` | `SkateGuard` (5 / 7 values; 5 of 6 DB values covered), `SockType` (4 → 4) | | | |
| Goalie mask | `maskid`, `masktype` | `GoalieMaskTexture` (`ge_nhl_goaliehelmettexturepack` 991 records, 372 distinct values; + `GoalieHelmetBrand`) | 243 → 243 (16: 173 → 169, missing 173/198/208/261; 17: 216 → 215, missing 208) | 372 (256 → 372) | `goaliehelmet_itech_cincinnaticyclones_0_texpack` → `goaliehelmet_itech_cincinnaticyclones_0_cm` + shared `_am/_nm/_coeff` |
| Goalie stick | `stick` (goalie) | `GoalieStickBrand` (`ge_nhl_goaliesticktexturepack`; + `StickHandedness`) | 26 → 26 | 140 (29 → 140) | `goaliestick_right_nhl_minnesota_texpack` |
| Blocker / trapper / pads | `blocker`, `trapper`, `pads` | `GoalieBrands` — one shared enum on `ge_nhl_goalieblockertexturepack` (132 values), `…trappertexturepack` (130), `…goaliepadtexturepack` (42) + `…mergedpadtexturepack` (116) | 43 / 41 / 42 → all | 132 shared | `blocker_right_nhl_boston_texpack`, `goaliepad_nhl_vancouver_texpack` |

Value-set ambiguity: because every small id is contained in several enum ranges, a value-only join is ambiguous for most fields (the
JSON keeps the value-only candidates); the binding above is the **type-name** match confirmed by full coverage on 7 / 7 titles. The
sentinel-like unmatched values (`helmetbrand 99`, a few `maskid`s on 16/17) are recorded, not explained.

## 2. What "existing entry vs new entry" means here (proposal)

- **Assign an existing catalogue entry to a player:** a DB field edit validated against the title's enum value set (the JSON per
  title); spare values (catalogue − used) are legitimate targets — 182 stick brands, 38 glove brands, 130 skate brands, 129 mask
  textures on NHL 21 have no stock user.
- **Replace the look of an existing entry:** the texture paths referenced by its collection (per record in the JSON examples; the
  full list is one `Decoder` call away) → archive members → placement per handoffs 02/03; the collection itself is untouched.
- **New entry (new brand id):** a new vault collection with the enum attribute set — blocked on the vault writer (collection keys,
  Q-R7-1), exactly as for heads.
- Not covered: geometry/material binding (`ge_gameobject`, `ge_material_player_stick` etc.), Creation-Zone thumbnails and label text
  (registry types of handoff 06), gen7 equipment tables (`xmja`/`fMVl`, handoff 30) and colour zones (`*zone*color`, 15 zones per item —
  per-player recolouring, DB-only).

Runtime card **R16-EQ-1 (NHL 21 PS4):** set one skater's `stick` to a spare `StickBrand` value whose collection references a visibly
different texture — the new stick shows in the player's close-up = DB → vault → texture is the consumer path; unchanged = no cause
identified (cache, a model-level override, the value being a texture-only variant) — next control: a value used by another stock player.

## 3. Unknowns (explicit)

1. Geometry selection per brand (which `ge_gameobject` / mesh a `StickBrand` value uses) — one reference level not decoded here.
2. `helmetbrand 99` and the unmatched mask ids on 16/17. 3. The per-item `Int32` attributes (`f581f44e`, `fe3e0ed7`, …) and
`OutdoorVariation` on tape packs. 4. Runtime consumption (card). 5. Gen7 equipment (not measured tonight).

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-20/probes/r16_equipment_chain.py docs/research/overnight-2026-09-20/data/r16-equipment-chain.json [--titles nhl21]
```
Executed once over the seven titles (≈ 15 min, read-only) plus the class exploration in the session. **Not executed:** launches, writes,
Studio binaries.
