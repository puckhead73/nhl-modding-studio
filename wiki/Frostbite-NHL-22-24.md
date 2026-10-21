# Frostbite: NHL 22 to 24

**Evidence:** measured by read-only structural decoding of three installed PlayStation 4 titles (NHL 22 CUSA26280 v01.00, NHL 23 CUSA31046 v01.00, NHL 24 CUSA37933 v01.60 with its Patch overlay): all 245 735 bundle records parsed, every span bounds-checked, block framing and codec shares from a sample of 6 529 spans. Only uncompressed blocks were read; no asset was decoded end to end; nothing was verified at runtime. The NHL 24 install had been touched by a third-party tool; its index files match that tool's backup byte for byte, but its content files are not certified vanilla.

From NHL 22, EA's NHL games moved to the Frostbite engine. There is no TDB database, no `.big` archive and no loose asset tree; everything sits in content-addressed storage indexed by a layered table of contents. Studio opens these titles in a read-only [Frostbite Browser](Frostbite-Browser-NHL-22-24).

## Install layout

```
CUSAxxxxx/
├── package.mft, sce_sys/, sce_module/, Scripts/BootIni/MemoryInit.ini
├── Data/
│   ├── layout.toc            root index (D1CE01-wrapped DbObject)
│   ├── initfs_Gen4b          encrypted initial file system
│   ├── chunkmanifest         plain DbObject: install-chunk list (identical across the three titles)
│   └── Gen4b/                "Gen4b" is the PS4 platform tag
│       ├── eight superbundle indexes (.toc): globals, globalsfull, contentlaunchsb, contentsb, worldssb, movies, ui, levels/game/game
│       └── superbundlelayout/nhl_installpackage_00 … _06/cas_NN.cas   (roughly 1 GB each)
└── Patch/   (NHL 24 only) same shape as Data/
```

Seven install packages hold the superbundles: 00 (globals, ui), 01 (levels/game), 02 (contentlaunchsb) are always installed; 03 (movies), 04 (worldssb), 05 (contentsb), 06 (globalsfull) are the full-install group. Superbundles are not confined to their own package: on NHL 22, `contentsb` spans also land in packages 01, 02 and 04. `settings.bundleFormat` is 2651454669 on every title.

## The loading chain

```
layout.toc ── superBundles[] ──▶ Gen4b/<sb>.toc
   ├─ bundle table ──▶ bundle record ──▶ spans in cas files
   │                     span 0 = the bundle MANIFEST (ebx, res, chunk names)
   │                     spans 1..n = data of those entries, in manifest order
   └─ chunk table ──▶ (cas id, offset, size)      TOC-level chunks
cas data = blocks: [u32 BE decompressed size][u16 BE codec word][u16 BE stored size][payload]
```

**D1CE01 wrapper** (measured): `layout.toc`, `initfs_Gen4b` and every superbundle `.toc` start `00 D1 CE 01`; the payload begins at 0x22C. Bytes 0x08 to 0x107 look like a 256-byte signature (inferred); whether the runtime verifies it is open and decides whether indexes can be rewritten. Payloads are plain except `initfs`, which is one `{ encrypted: … }` object.

**DbObject encoding** (measured, every decode consumed its payload exactly): type byte, C-string name unless bit 7 is set, then a value: `01` list and `02` object (LEB128 size, entries, `00` terminator), `06` bool, `07` string (LEB128 length including NUL), `08`/`09` int32/int64 little-endian, `0B`/`0C` double, `0F` GUID, `10` SHA-1, `13` blob.

**`layout.toc`** holds `superBundles`, `fs`, `head`, and an `installManifest` with install chunks, groups and settings. NHL 24's `Patch/layout.toc` carries `base` equal to the Data `head` (981806), which links the layers; it also lists a `ui` superbundle whose Patch `.toc` is absent, which Studio reports rather than substituting Data.

**Superbundle `.toc`** (big-endian, offsets relative to 0x22C): a bundle hash map, a 16-byte bundle table (`u32 name word · u32 flags<<24 | record size · u64 record offset`, flag byte 0x40 throughout), the bundle records themselves (size, span count, then `u32 offset · u32 size` per span with a `u32 cas id` where flagged, then one flag byte per span), a chunk GUID table (20 bytes: GUID plus `flags<<24 | index`) and chunk data words (`cas id · offset · size`). Bundle names are **Huffman-compressed** in the TOC and the name word is a bit offset, not a hash; names decode on every examined TOC and are unique within it. The cas id word `00 RR PP CC` names the root (0 Data, 1 Patch), the install package and the cas number.

**Bundle manifest** (span 0): `u32 BE size`, then little-endian magic 0x9D798ED6, counts of ebx, res and chunk entries, string and meta offsets; then one SHA-1 per entry, the ebx and res name tables, res types, meta and ids, chunk GUIDs with logical offset and size, and the strings. Five bundles across the three titles are empty (zero entries).

## Data blocks and codecs

Each block is `u32 BE decompressed size · u16 BE codec word · u16 BE stored size`; the low nibble of the codec word carries bits 16 to 19 of the stored size. Blocks tile every sampled span exactly.

| Codec | Share of sampled blocks | Identification |
|---|---|---|
| 0x11 | about 92% | every payload starts `8C 06`; consistent with Oodle Kraken (inferred, corroborated externally, **not decoded by Studio**) |
| 0x00 | about 8% | stored, uncompressed |

Decoding and re-encoding codec 0x11 gates reading or changing almost anything. Payload types confirmed from uncompressed blocks: an unidentified recurring `48 00 00 0C` header (9 743 chunks), `PTCH` v7 structures (1 108), `CFX` Scaleform-style UI movies (654, inferred), Matroska/WebM video (292), `MVhd` EA video (107), `SBle` SND10 sound-bank indexes as on NHL 15 to 21 (69), `ST00` UTF-16 closed captions referencing `TXT_…` keys (68), and JSON and Lua path lists. `GNF ` texture headers and `RIFF … EBX EBXD` containers were seen inside compressed blocks.

## Catalogue sizes

| Title | Bundles | Distinct ebx | TOC chunks |
|---|---|---|---|
| NHL 22 | 43 214 | about 205 k | 132 571 |
| NHL 23 | 52 619 | about 245 k | 150 330 |
| NHL 24 Data | 129 715 | about 483 k | 190 594 (Data and Patch) |
| NHL 24 Patch | 20 187 | about 131 k | |

Superbundle roles shift between titles: NHL 22's `globalsfull` is essentially chunks only while NHL 23 and 24 put about 130 k ebx there, and NHL 24's `contentlaunchsb` grows to 79 k bundles on a new `ui/textures` tree. Never assume one title's placement for another. NHL 24 patch bundles point about 92% of their spans into Patch and 8% into Data; 13 457 of the 20 187 patch bundle names match a Data bundle of the same superbundle. Runtime overlay, deletion and fallback rules are open.

## Where content lives (by asset path)

| Family | Path convention | Label |
|---|---|---|
| Roster database (successor of `nhl_game.db`) | **not located**; candidate carriers are tiny `chunkfilecollector` ebx in `globals` that may collect legacy chunk files; reading them needs codec 0x11 | open |
| Localization | not located; captions reference `TXT_…` keys | open |
| Leagues, teams, kits | `content/character/skater/league/<league>/<team>/<brand>/<variant>/…` (380 dirs; jersey, pant, font, helmet-logo and font-placement assets, with `_color`, `_normal`, `_coeff` maps); goalie kits under `…/goalie/jersey/…` | measured names |
| Team logos | `content/common/logo/<team>/logo_<team>_*` (189 dirs); NHL 24 also `ui/textures/common/logos/…` | measured names |
| Player heads and hair | `content/character/skater/shared/head/licensed/<first_last>/…` (621 on NHL 22) and `…/head/generic/male_<n>/…`; hair under `…/head/hair/…` | measured names |
| Portraits | NHL 24: `ui/textures/common/playerportraits/p<id>` (42 k), the same `p<id>` naming as earlier generations; NHL 22 and 23 ship only capture settings | measured names |
| Arenas and ice | `content/worlds/stadiums/<name>_<id>/…` (75 dirs on NHL 22); the suffix as arena id is inferred | measured names |
| Boards, ads, scoreboards | `content/worlds/textures/boards/…`, `content/worlds/configs/adboards/<venue>/…`, `content/worlds/scoreboard/<venue>/…`; the on-screen scorebug is Lua plus UI | measured names |
| Equipment | `content/character/skater/player/<category>/…`, goalie gear under `…/goalie/<item>/…` | measured names |
| Audio | `sound/speech/{commentary,announcer,threes,…}/…`, `sound/sfx_fb/…`, `sound/music_fb/…`; banks are `SBle` | measured |
| Movies | WebM and `MVhd` under the movie trees | measured |

Likely res-type roles, from path names only: 0x6bde20ba texture-like, 0x49b156d4 mesh, 0xb2c465f6 audio wave, 0x0e2b02f7 shader, 0x428ec9d4 bundle reference tables, 0x063994c5 font.

## What Studio does with this

A read-only native reader parses the wrappers, DbObjects, superbundle indexes (including Huffman names), manifests and CAS spans with safety caps, refuses unknown variants, treats the encrypted `initfs` as opaque, and includes no codec 0x11 decoder. The browser lists Data and Patch separately, searches decoded bundle names, shows manifest metadata and TOC chunks, and exports raw CAS-framed bytes or decoded stored blocks. Not enabled: typed ebx, texture, mesh or audio viewers, roster editing, deployment, packaging, or a persistent catalogue.

## Open questions

Decoding codec 0x11; whether the runtime verifies the D1CE01 signature; patch-over-base precedence; the roster database's location and whether it is still TDB; localization; the ebx RIFF type system; the `48 00 00 0C` and `PTCH` payloads; whether stored replacement blocks are accepted; and whether an emulator can run these titles for verification.

Last reviewed: 2026-10-20
