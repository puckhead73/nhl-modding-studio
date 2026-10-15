# Handoff R7 — the PS4 Attrib/VLT collection registry (`gccollections`), parsed: where the team/arena/jersey → asset-slug binding lives

Research lane, 19 September 2026 (item 2 of the [research correction and continuation dispatch](../../validation/overnight-research-2026-09-19-review.md)).
Claim `research-r7-vault-collections`. Consumer: Teams research (handoff [06](06-ps4-team-identity-and-presentation-map.md) §4/§4b, Q-R6-1),
later Phase 2 arena/ice/kit operations, and the NHL 21 cyberface program's head-binding question ([docs/audits/nhl21-ps4-head-asset-binding.md](../../audits/nhl21-ps4-head-asset-binding.md) §8).
Nothing here is game-verified; no install, archive or anchor was written; no game launched; **no writer is proposed.**

Machine-readable: [`data/r7-ps4-vault-collections.json`](data/r7-ps4-vault-collections.json) (10.7 MB; per title: file identities, container facts,
schema census, the decoded Teams-relevant collection sets, reference accounting, the DB joins). Probe:
`local/runs/overnight-research-2026-09-19/probes/vlt_collections.py` (read-only; ≈ 5 min for all seven titles; its module docstring is the
format record).

**Evidence vocabulary:** **observed** = read from the named files; **inferred** = a reading whose producing mechanism is not traced;
**unknown** = not established. Everything about what the *game* does with these records is unknown.

## Plain-language summary

Handoff 06 found a file (`gccollections.bin`, which it called an `ErtS` container) whose text mentions both the collection names and the
texture paths for on-ice logos, stadiums and kits, and asked whether that file is where the game turns a team or arena id into the
slug-named files it draws. This handoff parses the file properly instead of reading its strings. It is one half of EA's *Attrib/VLT*
vault: the `.vlt` sibling holds one record per asset collection (154 720 on NHL 21) with a class, a typed attribute block and
typed references to other collections; the `.bin` holds the strings and the attribute data those records point at; and a small
`gcschema` pair defines the 778 classes and their attributes. The string keys are Bob Jenkins' `lookup2` hash with a fixed seed, which
lets the class names, type names and a few dozen attribute names be recovered and checked. With that, the binding is explicit and
parsed rather than guessed: every arena row in the database has a `ge_level` collection whose `ID` attribute equals the row's
`arenaartid` (all 32–62 ids on every title, NHL 15–21); the centre-ice logo packs carry the team art id as a typed `Team` attribute and
point, through a typed reference, at the texture collection whose `FileRef` is the slug-named file — and in 66 of NHL 21's 348 packs
the file's number is *not* the team number, which shows the reference is the binding and the name is not; the jersey texture packs
carry `Team`, `UniformVariation` and `JerseyFit`, which equal the database's `teamartid`, `variant` and `style`, and each pack holds
ten typed references to the slug-named jersey, font, name-bar and helmet-logo textures. What is still open is narrower: some database
triples match two packs, the game's selection rule and runtime consumption are unobserved, most attribute names are still hashes, and
the collection keys themselves are not derived from the names.

## 0. Sources (observed)

| Title | Install | `gccollections.vlt` / `.bin` SHA-256 (prefix) · size | `gcschema.vlt` / `.bin` | DB joined |
|---|---|---|---|---|
| 15 | CUSA00561 | `d1ec595c…` 14 761 408 / `0684ef97…` 13 462 336 | 26 880 / 106 648 | `anchors/ps4/nhl15/databases/nhl_game.db` `ae9d1d56…` |
| 16 | CUSA01909 | `202035cf…` 13 325 888 / `b1217f4a…` 10 523 544 | 49 952 / 179 848 | anchor `7d95f1db…` |
| 17 | CUSA04326 | `df39aeaa…` 17 666 480 / `2da87ee8…` 14 211 032 | 53 904 / 201 320 | anchor `de48b02b…` |
| 18 | CUSA07569 | `0a5e019d…` 21 672 784 / `cf0cfab2…` 17 928 824 | 54 416 / 206 760 | anchor `65eaa6e8…` |
| 19 | CUSA11126 | `6e537bb7…` 25 875 856 / `aece3f7a…` 21 075 832 | 60 304 / 232 640 | anchor `265e4a68…` |
| 20 | CUSA15439 | `31a2727f…` 30 651 104 / `e5266477…` 23 839 624 | 61 520 / 238 848 | real install `inxp_00.big:…/nhl_game.db` `1e200478…` |
| 21 | CUSA18182 | `dd4ceff0…` 32 620 256 / `d668bbe1…` 25 113 928 | 63 424 / 248 040 | anchor `90ee1ec4…` |

All read from the already-extracted `databigs/Extracted/nhl/data/common/database/attrib/` trees (the `boot.big` members handoff 06
hashed: the `.bin` prefixes match its table). Full hashes are in the JSON. The NHL 21 pair is the one
[docs/audits/nhl21-ps4-head-asset-binding.md](../../audits/nhl21-ps4-head-asset-binding.md) §2.1 decoded to chunk level; this handoff
starts from that record and goes to attribute level.

## 1. Container format (observed on all seven titles)

Correction to handoff 06 §4b first: **`ErtS` is not a container format.** It is the reversed FourCC `StrE` of the first chunk of
`gccollections.bin`, the string pool. The registry is the pair `gccollections.vlt` (records) + `gccollections.bin` (strings + data heap),
described by `gcschema.vlt` + `gcschema.bin`. The atlas Q-R6-1 wording was corrected in the checkpoint; this section is the format record.

| Piece | Layout (little-endian) | Verified |
|---|---|---|
| Chunk chain (both `.vlt` files) | `tag(4, FourCC stored reversed) \| size(4, includes the 8-byte header) \| payload`: `Vers`, `StrN`, `DepN` (names `gccollections.vlt`/`.bin`), `DatN`, `ExpN`, `PtrN`, `EndC` | 7/7 |
| `.bin` | `StrE` (NUL-separated string pool, 79 437 … 127 198 strings) \| `TofC` (count u32, then `(classKey, collectionKey)` u32 pairs) \| `EndC` \| **unframed heap** (instance blobs, `RefSpec`s, vectors; 18.1 MB on NHL 21) | 7/7 |
| `ExpN` | count u32, then 16-byte rows `expKey \| tag \| size \| absoluteVltOffset`; one tag value per file; records tile `DatN` with 0/4-byte alignment gaps only (gap histogram = {0, 4} on all seven; the JSON's `records_tile_datn` flag is false on 16/21 only because of trailing padding at the end of `DatN`) | 7/7 |
| `TofC` | `TofC[i] == (classKey, collectionKey)` of `ExpN[i]`'s record on every row (93 880 … 154 720) | 7/7 |
| `PtrN` | 12-byte fixups `slotOffset \| type u16 \| flags u16 \| targetOffset`; **two segments** introduced by `(0,2,1,0)` = slots inside the `.bin` and `(0,2,0,0)` = slots inside the `.vlt`; every target is a `.bin` offset (strings, `TofC`, or heap); `(0,0,0,0)` ends the list (NHL 15 carries two end markers); every slot is 8-aligned and zero in the file (877 063 / 877 066 on NHL 21 — the three markers are the exceptions) | 7/7 |
| Collection record (`.vlt`) | `collectionKey u32 \| classKey u32 \| 0 \| nInline u32 \| 1 \| nInline \| nTypes u16 \| u16 \| 0 \| ptr(8) → instance blob in the heap \| nTypes type keys \| zero pad to 8 \| nInline × 24-byte inline entries `attrKey \| 0 \| value or ptr (8) \| typeIndex u16 \| flags u16 \| 0`` | decoded on the sets below; the two `nInline` words were equal on every record dumped by hand (not asserted programmatically) |
| Instance blob | attributes at the offsets/sizes the schema class gives; `Text` = 8-byte pointer slot → string; `RefSpec` = 16 bytes `classKey \| collectionKey \| 8 × 0`; enums = i32; `Vector4` = 4 × f32 | all sets |
| Inline entry values | scalar/enum inline in the 8-byte value; `RefSpec`/`Vector`/`Transform`/`Text` through a fixup slot; **array attributes** (schema `count == 65535`) carry two count words there (not decoded further) | all sets |
| Schema (`gcschema.vlt`) | 778 (NHL 21) / 326 (NHL 15) class records of 48 bytes `classKey \| … \| attrCount @8 \| ptr@16 → descriptors \| instanceSize u16 @40`; descriptor = `attrKey u32 \| typeKey u32 \| offset u16 \| size u16 \| count u16 \| flags u8 \| code u8`; flag `0x04/0x05` = value lives in the record's inline entries, `0x06/0x26` = in the blob, `count 0xFFFF` = array | 7/7 (type keys 185/185 resolved on NHL 21) |
| **Key function** | `key = lookup2(bytes, initval 0xABCDEF00)` (Bob Jenkins 1996). Verified: every `TofC` class key resolves to a schema class name (258/258 … 476/476 per title), every attribute type key resolves to a type-name string (185/185), and 81 attribute keys resolve to plain PascalCase names (e.g. `DebugName`, `Team`, `ArenaID`, `RinkId`, `CenterLogo`, `CenterLine`, `IceLine`, `FileRef`, `UniformVariation`, `JerseyFit`, `HeadTeam`, `BodyTeam`, `JerseyTexturePack`) | observed |
| Collection keys | **not** `lookup2(DebugName)` under any init/prefix tried (0/13 candidates); opaque here (the head audit's item 8 stands) | observed negative |

Vault version words differ per title (`Vers` payload `316058a0…` on 15 → `79ffffde…` on 21); the layout above did not change across
them. Numbered sibling vaults (`<decimal>.vlt/.bin`, 0 on NHL 15, 39 on 16 … 81 on 21) hold per-stadium content (lightmaps,
banners, light rigs — e.g. `20823504` = `custom_pittsburgh`); their file names are neither a `ge_level` collection key nor
`lookup2` of the level name (0/8 forms tried), and **no `ge_level` reference points into them** — every typed reference in the decoded
sets resolves inside `gccollections` (§3). Their naming key is unknown.

## 2. Schema classes and attributes that matter for Teams (observed)

| Class (`ge_…`) | Records 15 → 21 | Attributes recovered (name : type @offset) | Purpose (inferred from names) |
|---|---|---|---|
| `ge_level` | 33 → 76 | `DebugName`:Text; `ID`:ArenaId (15–21); `ArenaID`:ArenaId (17+); `RinkId`:RinkID; `CenterLogo`:Team; `CenterLine`:CenterLine; `IceLine`:IceLineTeamLeague; ≈100 typed `RefSpec`s (banner/centre-ad/chair/dasher/dressing-kit/entrance-prop/goal-light/jumbotron/… texture packs and kits; most null on a given level) — 260 inline attributes on NHL 21, 44 on NHL 15 | the arena/stadium definition |
| `ge_nhl_stadiumcenterlogotexturepack` | 221 → 348 | `RefSpec`→`ge_texture` @0; `DebugName` @16; `CenterlogoStyle` @24; `CenterLogo`:**Team** @28; one flag-4 Int32 | centre-ice logo pack per team art id |
| `ge_nhl_stadiumcenterlinetexturepack` | 33 → 145 | `RefSpec`→`ge_texture`; `DebugName`; `CenterLine`:CenterLine | centre-line art by id |
| `ge_nhl_stadiumicelinetexturepack` | 4 → 45 | `RefSpec`→`ge_texture`; `DebugName`; `IceLine`; `ThreesRinkGraphic` | ice-line art |
| `ge_nhl_skaterjerseytexturepack` | 1 014 → 1 464 | 11 `RefSpec`s (10 → `ge_texture`, 1 → `ge_nhl_skaterjerseytextureplacement`); `DebugName` @192; `UniformVariation` @200; **`Team` @204**; `JerseyFit` @208; `OnOff` @212; one flag-4 Int32 | the kit texture set for (team, variation, fit) |
| `ge_texture` | 48 501 → 54 820 | `FileRef`:Text @0 (the slug-named path, no extension); Bool @8 | the asset path leaf |
| `ge_nhl_stadiumdressingkit` | 25 (21) | `DebugName`; `ArenaId`; `Group`→`ge_group`; `Lighting`; `ConversionKitTier` | per-arena dressing |
| `ge_arenanamemapping` / `ge_sponsornamemapping` | 0 records | `Names`:Text[]; Int32[] | defined, unused in `gccollections` |
| `ge_nhl_mascot` | 1 | `HeadTeam`/`BodyTeam`/`JerseyTeam`/`PantTeam`/`SockTeam`:Team (1000) | placeholder |

Attribute-name coverage is low overall (81 of 2 828 keys on NHL 21) but complete for the fields used above; every other attribute is
reported by its key hash and left opaque. Enum *value* names (which `Team` value is which team) are not stored in either vault; the
values are checked against the database below.

## 3. Parsed links, not string co-occurrence (observed)

A link counts as parsed when it is a typed `RefSpec` `(classKey, collectionKey)` whose target record exists in the same vault. Accounting
over every reference in the decoded sets (levels, centre-logo/line/ice-line packs, jersey packs, dressing kits, mascot):

| Title | resolved | typed-null (class set, key 0) | dangling |
|---|---:|---:|---:|
| 15 / 16 / 17 / 18 / 19 / 20 / 21 | 11 429 / 9 068 / 10 494 / 13 129 / 14 993 / 15 159 / 16 699 | 594 / 1 313 / 3 737 / 3 379 / 3 779 / 4 069 / 7 945 | **0 on every title** |

Named chains (NHL 21 unless stated; every one is in the JSON):

1. `ge_nhl_stadiumcenterlogotexturepack` `centerlogo_boston` (`CenterLogo` = Team **2**) → `RefSpec` → `ge_texture` `ce2f845e…` →
   `FileRef` `rendering\stadiums\common\ice_centerlogo_2_cm`.
2. `centerlogo_47_texpack` (Team 47) → `ge_texture` `5485549e` → `FileRef` `rendering\stadiums\common\ice_centerlogo_501_cm`, shared
   with `centerlogo_46…53_texpack` — **66 of 348 packs point at a texture whose number differs from the pack's team number** (282 agree).
   The name-derived rule of handoff 06 (`centerlogo_{artid}` → `ice_centerlogo_{artid}`) would resolve those 66 wrongly; the reference is
   the binding.
3. `centerline_32_texpack` (`CenterLine` 32) → `ice_cline_32_cm`; level `boston` carries `CenterLine` 32, `CenterLogo` 2, `ID` 2.
4. `jersey_rbk_boston_four_texpack` (`Team` 2, `UniformVariation` 4, `JerseyFit` 1) → ten textures:
   `jersey_rbk_boston_four_{cm,nm,coeff}`, `font_rbk_boston_four_{cm,nm,sm}`, `name_rbk_boston_four`, `helmetlogo_rbk_boston_four_0`,
   `helmetlogo_blank_{1,2}` (all under `rendering\characters\player\`), plus one placement collection. Boston has 16 packs on NHL 21:
   variations 0–4 for fit 1 (`rbk`), 0–3 for fit 0 (`nineties`), 0–4 + 20 + 21 for fit 2 (`adidas`).
5. `ge_level` `boston` (NHL 15) → `ge_crowdplacement` `boston_crowd.crowd`; NHL 21 levels resolve a `ge_crowdplacement` and a
   `ge_nisoverridesgroup` and leave ~104 typed slots null.
6. `dressingkit_threes_5_tier1` (`ArenaId` 520) → `ge_group` `threes_5_dressing_tier1`; level `threes_5` has `ID`/`ArenaID` 520.

## 4. The DB-id → collection → slug chain, tested (observed joins; anchors / real NHL 20 DB)

| Join | 15 | 16 | 17 | 18 | 19 | 20 | 21 |
|---|---|---|---|---|---|---|---|
| `exhibitionarena.arenaartid` ↔ `ge_level.ID` | **32/32** | **33/33** | **39/39** | **52/52** | **56/56** | **60/60** | **62/62** |
| same via `ArenaID` (17+) | — | — | 33/39 | 46/52 | 50/56 | 55/60 | 57/62 |
| `exhibitionteams.artid` ↔ centre-logo `CenterLogo` | 212/213 | 216/218 | 251/256 | 268/289 | 273/307 | 270/304 | 273/312 |
| `stockteamjerseys (teamartid, variant)` ↔ jersey `(Team, UniformVariation)` | 565/565 | 599/600 | 703/711 | 837/839 | 878/900 | 880/937 | 955/1 027 |
| `(teamartid, variant, style)` ↔ `(Team, UniformVariation, JerseyFit)` — distinct DB triples matched / DB triples; matched triples with **2** packs | 657/657; 0 | (not run) | 802/810; 53 | 996/998; 24 | (not run) | (not run) | 1 176/1 250; 88 |
| `exhibitionteams.centerline` ↔ centre-line `CenterLine` | field absent | absent | absent | 60/60 | 60/60 (119 missing) | 60/60 (119) | 60/60 (119) |

Readings:

- **Arena → level is complete and keyed by `ID`, not `ArenaID`.** Every DB `arenaartid` has at least one `ge_level` record with
  `ID == arenaartid` on every title; `ArenaID` (NHL 17+) equals `ID` on the NHL arenas but is 0 on the Swiss / common-ice / World-Cup /
  training levels whose `ID` (321, 325, 326, 329, 331, 552, 800, 36) the DB uses. NHL 17's `minnesota` and `ottawa` levels have
  `ArenaID` 0 and `ID` 14 / 20. Levels without a DB row: `frontend` (100/101), the EASHL tiers (512–515), the `custom_*` builder arenas
  (701–709), `common_ice_nhl` (552), `outdoor_*` (900–910, NHL 19+). Level `DebugName` matches the arena's city on every one of the 80 joined NHL 21 rows the JSON keeps
  (`Honda Center` → `anaheim`, `Bell Centre` → `montreal`, …; three Toronto-keyed levels: `toronto`, `world_cup_toronto`,
  `team_canada_training`; all AHL rows share `ID` 50 `ahlgeneric`).
- **Team art id → centre-logo pack is the `Team` enum value.** 1 / 2 / 5 / 21 / 34 / 34 / 39 DB art ids (mostly national and
  historic teams) have no pack; 9–40 packs carry `Team` values that no DB row uses (e.g. −1, 30, 31, 54–61, 999/1000/1001 placeholders).
  Whether an unmatched team draws a fallback pack is unknown.
- **Kit textures: `(teamartid, variant, style)` is the DB key and `(Team, UniformVariation, JerseyFit)` the collection key**
  (`style` 0/1/2 and `JerseyFit` 0/1/2 have the same value sets — 0 = `nineties`, 1 = `rbk`, 2 = `adidas` by pack name). NHL 15 maps
  1:1 on all 657 triples. Later titles leave 24–88 matched triples with **two** packs (the same triple appears with two `DebugName`s —
  era/alternate duplicates) and 2–74 DB triples without a pack (spare-kit `teamartid` 255 / 30220 / 30221; NHL 21's `variant` 10 /
  `style` 2 rows). The remaining discriminator — the pack's flag-4 Int32 (`bb93351e`), a DB year/`variantlegend` field, or a runtime
  rule — is **unknown**. This does not decide the NHL 20/21 thumbnail fourth component (handoff 06 Q-R6-2): thumbnails are UI files
  outside this registry.
- **Centre line:** the team row's `centerline` (18+) is the `CenterLine` enum of a `…centerlinetexturepack`; value 119 (NHL 19–21) has
  no pack.

Everything above is a static correspondence between two shipped data sets; the vault's class/attribute names make the *purpose* of each
link explicit, but **no runtime consumption was observed** and no statement about what the game does with an edited vault or DB id is made.

## 5. Cross-title differences (observed)

- Layout and hash: identical on 15–21 (chunk set, record layout, two-segment `PtrN`, `TofC`, key function); version words differ.
- `ge_level` grows from 44 attributes (NHL 15: `ID`, `CenterLogo`, `CenterLine`, `IceLine`, crowd placement, 18 null refs) to 260
  (NHL 21: adds `ArenaID`, `RinkId`, dozens of typed texture-pack/kit references, banner/centre-ad enums). `ArenaID` appears on 17+.
- Class count 326 → 778; `ge_nhl_stadiumdressingkit`, EASHL/Threes classes and the `ge_arenanamemapping` definition appear later.
- Per-stadium sibling vaults appear on NHL 16 (39 files) and grow to 81 on NHL 21; NHL 15 has none (its levels reference only crowd
  placements inside `gccollections`).
- The database side: `exhibitionteams.centerline` exists from NHL 18 (handoff 06); NHL 20 is the real install DB (anchor is the NHL 19
  base, memory/handoff note).

## 6. What this changes in earlier records

- Handoff 06 §4b (rev. 2 wording already applied): the binding is **resolved at the data level** for arenas (`ID`), centre-ice logos
  (`Team`), centre lines (`CenterLine`) and kits (`Team`/`UniformVariation`/`JerseyFit`); it lives in `gccollections.vlt` records as typed
  attributes and references, with the slug in the referenced `ge_texture.FileRef`. "Collection lookup keyed by ids, or the executable
  composes a slug" → the former, by explicit attributes.
- [docs/audits/nhl21-ps4-head-asset-binding.md](../../audits/nhl21-ps4-head-asset-binding.md) §2.1 (not edited — audit path): its
  "48-byte index record with a name pointer and an `(parentNodeKey, collectionKey)` blob pointer" is the `ge_texture`/`ge_swappart` record
  shape seen from outside; the name it saw is the `DebugName`/`FileRef` Text attribute and the "blob" is the instance. Its §8 negative
  (no head-id attribute on `head_<name>` collections; keys not a function of the id) is consistent with this decode and is **not**
  overturned: this handoff did not decode `ge_nhl_skaterhead` and does not claim the head binding.
- Handoff 06 §5 item 3 (arena presentation ids): unchanged — catalogue membership is validation input only; this handoff adds that the
  *arena* itself is keyed by `ID` in the vault, which is a second validation input, not an authorisation.

## 7. Unknowns (explicit)

1. Runtime consumption of any record; loader behaviour on an edited vault; whether the game builds collection names or keys itself.
2. Collection-key derivation (not `lookup2` of the DebugName); the `ExpN` key per record; the numbered sibling-vault naming key.
3. 2 747 of 2 828 attribute names (NHL 21) — reported as hashes; the flag-4 Int32 on texture packs; enum value names.
4. The selection rule when a `(Team, UniformVariation, JerseyFit)` triple has two packs; the NHL 21 `variant` 10 rows.
5. Head/hair collections (`ge_nhl_skaterhead*`, 766 records) — not decoded here; the bespoke head-id binding stays open.
6. The `PtrN` type/flag values other than `(3, 1)`; the array attribute layout; the trailing `DatN` padding on NHL 16/21.

## 8. Not proposed

No writer, no Studio operation, no coverage-matrix promotion. The safe research follow-ups are: decode `ge_nhl_skaterhead` /
`ge_nhl_skaterheadtexturepack` with this reader (bounded, same vault); resolve more attribute names by hashing the schema's class-name
vocabulary; identify the numbered vault key; then the queued PS4 team-localization / gen7 join comparison.

## 9. Reproduce, checks, cleanup

```
python local/runs/overnight-research-2026-09-19/probes/vlt_collections.py docs/research/overnight-2026-09-19/data/r7-ps4-vault-collections.json [--titles nhl21,nhl15]
```

Executed (read-only): the probe over all seven titles (≈ 5 min); ad-hoc checks in the session scratchpad — hash-function search
(`lookup2`/`oaat`/`fnv1`/`fnv1a`/`djb2`/`crc32` × inits against schema class keys: only `lookup2(0xABCDEF00)` hits), attribute-name
brute force (≈ 180 k candidate strings, 81 hits incl. `DebugName`), record/heap dumps that established the record layout, the
collection-key-derivation negative (13 forms), the numbered-vault name negative (8 forms), the reference-into-sibling-vault test
(0 hits after the typed-null correction), the `(teamartid, variant, style)` triple join on 15/17/18/21. **Not executed:** launches, writes,
any Studio binary. Scratchpad exploration scripts (`vlt.py`, `schema.py`, `decode.py`, `t1–t9.py`, `r7-test.json`) were deleted after
the durable probe reproduced their results; nothing under `local/runs/…/r7/` was created.
