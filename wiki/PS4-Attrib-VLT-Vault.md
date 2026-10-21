# PS4 Attrib/VLT Vault

**Evidence:** measured: the container format, key function and typed joins on all seven PS4 titles (NHL 20 from the genuine install database). Runtime consumption of any vault record has never been observed, and no vault writer exists.

The PS4 titles bind database ids to asset files through a typed registry EA calls the **attribute vault**: `gccollections.vlt` (records) with `gccollections.bin` (string pool, table of contents and heap), described by `gcschema.vlt` and `.bin`. The four files live at `nhl/data/common/database/attrib/` inside `boot.big`, identical on every install; no patch archive carries them. Numbered sibling vaults hold per-stadium content (0 on NHL 15, 39 on NHL 16, 81 on NHL 21) with an unknown naming key. A vault is read only for the title it ships with; never borrow another title's.

## Container

All little-endian, identical layout from NHL 15 to 21. Both `.vlt` files are a chunk chain of `tag (reversed four-character code) | size | payload`: `Vers`, `StrN`, `DepN`, `DatN`, `ExpN`, `PtrN`, `EndC`. The `.bin` holds `StrE` (79 437 to 127 198 strings), `TofC` (count, then `(classKey, collectionKey)` pairs), `EndC` and an unframed heap. `ExpN` rows (`expKey | tag | size | absolute offset`, 16 bytes) tile `DatN` and match `TofC` row for row (93 880 to 154 720). `PtrN` holds 12-byte fixups (`slotOffset | type | flags | targetOffset`) in two segments for slots in the `.bin` and in the `.vlt`; every target is a `.bin` offset.

A **collection record** is a header (`collectionKey | classKey | 0 | nInline | 1 | nInline | nTypes | … | pointer to instance blob`), the type keys, then 24-byte inline entries `attrKey | 0 | value-or-pointer | typeIndex | flags | 0`. Values: text as a pointer to a string; a **reference as 16 bytes `classKey | collectionKey | zeros`**; enums as i32; vectors as four floats; arrays partly decoded. The schema holds 326 (NHL 15) to 778 (NHL 21) classes in 48-byte records with per-attribute descriptors; flags say whether a value is inline or in the blob.

**Keys** are `lookup2(bytes, 0xABCDEF00)`, the public 1996 hash: every class key (258 to 476 per title) and every type key resolves; 81 attribute names are resolved (`DebugName`, `Team`, `ArenaID`, `RinkId`, `CenterLogo`, `CenterLine`, `IceLine`, `FileRef`, `UniformVariation`, `JerseyFit`, `JerseyTexturePack`, `IsTopScorer`), and about 2 747 of 2 828 remain hashes. **Collection keys are not the hash of the debug name**; they are opaque, which is what blocks creating new records. Typed references resolve with zero dangling on every title (NHL 21: 16 699 resolved, 7 945 typed-null).

## Classes and joins

| Class | Records (NHL 15 → 21) | Role |
|---|---|---|
| `ge_level` | 33 → 76 | venues: `ID`, `ArenaID` (from 17), `RinkId`, centre logo, centre line and ice line packs, about 100 texture-pack references |
| `ge_nhl_stadiumcenterlogotexturepack` | 221 → 348 | `CenterLogo` is a `Team` enum; points to a texture |
| `ge_nhl_skaterjerseytexturepack` | 1 014 → 1 464 | ten texture references plus a placement; `Team`, `UniformVariation`, `JerseyFit`, `IsTopScorer` |
| `ge_texture` | 48 501 → 54 820 | `FileRef` = the slug path without extension (`nhl/data/kettle/<FileRef>.gnf`) |

| Join | 15 | 16 | 17 | 18 | 19 | 20 | 21 |
|---|---|---|---|---|---|---|---|
| arena `arenaartid` ↔ `ge_level.ID` | 32/32 | 33/33 | 39/39 | 52/52 | 56/56 | 60/60 | 62/62 |
| team `artid` ↔ centre-logo `CenterLogo` | 212/213 | 216/218 | 251/256 | 268/289 | 273/307 | 270/304 | 273/312 |
| team `centerline` ↔ `CenterLine` | | | | 60/60 | 60/60 | 60/60 | 60/60 |

Arenas are keyed by `ID`, not `ArenaID`. **The reference is the binding, not the slug name**: on NHL 21, 66 of 348 centre-logo packs point at a texture whose number differs from the team number. Level ids are not unique on NHL 16 to 21 (id 0 is shared by several names), and re-pointing an arena's art id would swap ice, rink and crowd packs at once, so the Studio lists venues read-only.

**Jerseys.** The database key `stockteamjerseys (teamartid, variant, style)` matches the pack key `(Team, UniformVariation, JerseyFit)` with style 0 nineties, 1 rbk, 2 adidas. Matched triples run from 657 of 657 (NHL 15) to 1 176 of 1 250 (NHL 21), with 74 database-only and 198 pack-only triples on NHL 21. Where a triple has two packs the discriminator is `IsTopScorer` (every pair differs in it, except one NHL 19 pair where both are 0), so the static pack key is `(Team, UniformVariation, JerseyFit, IsTopScorer)`; when the top-scorer pack is drawn is a runtime decision. Each jersey pack references jersey colour, normal and coefficient maps, three font maps, a name bar and a helmet logo.

**Equipment.** The binding is a **type-name match** between a database field and a vault enum type, confirmed by full value coverage on all seven titles:

| Field | Vault enum (class) | NHL 21 distinct → in catalogue | Catalogue values |
|---|---|---|---|
| `stick` | `StickBrand` | 51 → 51 | 233 |
| `stickbladetapestyle`, `stickshafttapestyle` | tape styles | 5 → 5, 3 → 3 | 5, 3 |
| `gloves` | `GloveBrand` | 27 → 27 | 65 |
| `helmetbrand` | `HelmetBrand` | 12 → 11 (**99 unmatched** on 17 to 21) | 25 |
| `visortype` | `HelmetVisorType` | 6 → 6 | 7 |
| `skatebrand` | `SkateBrand` | 17 → 17 | 147 |
| goalie `maskid`, `masktype` | `GoalieMaskTexture` (991 packs) | 243 → 243 | 372 |
| goalie `stick` | `GoalieStickBrand` | 26 → 26 | 140 |
| `blocker`, `trapper`, `pads` | `GoalieBrands` (one shared enum) | all | 132 |

Spare catalogue values are legitimate reassignment targets (NHL 21: 182 sticks, 38 gloves, 130 skates, 129 masks unused); a new brand needs a new vault record, which is blocked. Geometry per brand is not decoded. The Creation Zone CSVs (NHL 17 to 21) list the same ids with `$` dictionary-key names and thumbnails; see [Equipment and Goalie Gear](Equipment-and-Goalie-Gear).

## The gen7 front-end vault (`feattribdb`)

The Xbox 360 and PS3 titles carry a big-endian relative with **64-bit keys**, empty string tables (names are hashes only) and 47 records of one type; 40 parse consistently. Xbox 360 NHL 15 and Legacy ship byte-identical files yet different create-player catalogues, so complete per-title lists cannot be encoded solely in those bytes; where the lists live is open.

Last reviewed: 2026-10-20
