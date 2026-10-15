# Frostbite NHL file structure — NHL 22, 23 and 24 (PS4)

Follow-up, 14 October: the [implementation review and roadmap](../plans/frostbite-implementation-2026-10-14.md)
records the new native reader/browser, full structural audit, and measured Huffman bundle-name
decoding. The report below remains the original 13 October research snapshot; its statements
about implementation status and unresolved name words describe that earlier state.

Status: **read-only structural research, 13 October 2026.** Nothing here is implemented in
Studio, and it promotes no support level. Everything marked **observed** was decoded from the
real installs listed below and checked for self-consistency. **Inferred** means a strong
pattern that has not been decoded end to end. **Unknown** is stated as such. The probe scripts
and per-title summaries are in [docs/research/frostbite-2026-10-13/](../research/frostbite-2026-10-13/).

Clean-room note: this was established from the installed bytes alone. No external Frostbite
tool or its source was consulted. The terms *superbundle*, *bundle*, *ebx*, *res*, *chunk*,
*toc* and *cas* are used because the game's own files use them (`layout.toc` keys, file names,
asset paths).

NHL 25–27 (PS5) are **not covered**: no PS5 data was available.

## 1. Summary

NHL 22–24 abandon the Ignite/Kettle layout entirely: there are no `*.big` archives, no loose
`nhl_game.db`, and no folder tree of assets. An install is about 90–160 files:

- **`layout.toc`** names 8 *superbundles* and 7 *install packages*.
- Each superbundle has a binary **`.toc`** index of **bundles** and **chunks**.
- All content is packed into about 34–60 **`cas_NN.cas`** files of roughly 1 GB each, split
  across those install packages.

Each bundle's **manifest** (stored in a cas file) lists named assets by path:
- **ebx:** typed asset/config objects;
- **res:** resources such as textures, meshes and shaders;
- **chunks:** large binary payloads.

Across the three titles we decoded **every** bundle manifest: 43 214 (NHL 22), 52 619
(NHL 23), 129 715 (NHL 24) and 20 187 (NHL 24 patch). That gives a full asset-path catalogue
of 205 k / 245 k / 483 k distinct ebx names. So **we now know where every named asset lives**:
which superbundle, bundle, cas file, offset and size.

What blocks reading the *contents*: about 92% of data blocks use codec `0x11`, whose stream
header (`8C 06`) is consistent with **Oodle Kraken** (inferred). Studio has no decoder for it.
Uncompressed blocks (about 8%) confirm several payload types directly: Scaleform-style UI
movies, WebM/EA video, `SBle` audio banks, caption tracks, Lua script lists and JSON. GNF
headers appear as literal bytes inside compressed texture data.

**Where the roster database is (the old `nhl_game.db`) remains unknown.** It is not an
uncompressed chunk. The candidate carrier is the `globals/chunkfilecollector*` family (see §6).

## 2. Sources and provenance

| Title | Install (shadPS4) | Content ID · version | Notes |
|---|---|---|---|
| NHL 22 | `H:\Emulators\ShadPS4\Games\CUSA26280` | `EP0006-CUSA26280_00-NHLPS4HOCKEY2022` · 01.00 (`gd`) | vanilla base |
| NHL 23 | `H:\Emulators\ShadPS4\Games\CUSA31046` | `EP0006-CUSA31046_00-NHLPS4HOCKEY2023` · 01.00 (`gd`) | vanilla base |
| NHL 24 | `H:\Emulators\ShadPS4\Games\CUSA37933` | `UP0006-CUSA37933_00-NHLPS4HOCKEY2024` · **01.60** (`gp`) | base + `Patch/` overlay; contains `__FMTBackup/` from a third-party mod tool (6/4/2026) |

The NHL 24 install had been touched by a mod tool. Its index files (every `.toc`,
`layout.toc`, `initfs_Gen4b`, `chunkmanifest`, for both `Data` and `Patch`) are byte-identical
to the tool's backup, so **no index-level modification is active**. The `.cas` files have no
backup, so their content is not certified vanilla. Duplicate copies exist under
`H:\Emulators\games\PS4\` and were not used.

Index-file SHA-256 prefixes (first 16 hex):

| File | NHL 22 | NHL 23 | NHL 24 `Data` | NHL 24 `Patch` |
|---|---|---|---|---|
| `eboot.bin` | `399d840264bf2b99` | `834d73c02fc7f18e` | `5e726421109f865c` | — |
| `layout.toc` | `95ab6337e9834961` | `c54a7b933fafd6bb` | `d65e019d02d3d5b7` | `1b07b9a063185097` |
| `initfs_Gen4b` | `9d1b1e952a605ed7` | `29fa6a848e4b91a2` | `2ab2b844b4133895` | `9dd64540c57ff62f` |
| `chunkmanifest` | `99256bc09cf8641d` | `99256bc09cf8641d` | `99256bc09cf8641d` | — |
| `globals.toc` | `93081e25e8acda99` | `a908e00931b61ddf` | `23a3f06bf745f96d` | `9f5baf68a04de8ac` |
| `contentsb.toc` | `d5348cec26862b45` | `77f1682983c019e2` | `600f8afe0c988e77` | `d58f12ee17c69418` |
| `contentlaunchsb.toc` | `65c3d9b543f62f38` | `806164b3c030f08e` | `84be7a30406f062c` | `82bb1f6f3c32a91c` |

## 3. Install layout (observed)

```text
CUSAxxxxx/
├── eboot.bin                      166–188 MB executable
├── Engine.BuildInfo.prx, Engine.Render.Core2.PlatformPs4.prx, EAWebKit.prx (22/23)
├── package.mft                    ProductCode / GameRegion / CrossButton / SaveIcon
├── sce_discmap.plt (+ _patch.plt on 24), sce_module/, sce_sys/ (param.sfo, icons, trophy)
├── Scripts/BootIni/MemoryInit.ini memory budget settings (plain text)
├── Scripts/ini/*_versionInfo*.ini NHL 23 only: skuVersion / onlineVersion (plain text)
├── Data/
│   ├── layout.toc                 ROOT index (D1CE01-wrapped DbObject)
│   ├── initfs_Gen4b               ENCRYPTED initial file system (D1CE01 + "encrypted" blob)
│   ├── chunkmanifest              plain DbObject: install-chunk list
│   └── Gen4b/                     "Gen4b" = PS4 platform tag
│       ├── globals.toc  globalsfull.toc  contentlaunchsb.toc  contentsb.toc
│       ├── worldssb.toc  movies.toc  ui.toc  levels/game/game.toc     (8 superbundle indexes)
│       └── superbundlelayout/nhl_installpackage_00 … _06/cas_01.cas … cas_NN.cas
└── Patch/ (NHL 24 only)           same shape as Data/, delta content; layout.toc has "base"
```

Install package → superbundle → cas files (identical shape on all three titles; sizes differ):

| Package | Superbundles | Always installed | NHL 22 cas files | NHL 24 cas files (Data / Patch) |
|---|---|---|---|---|
| 00 | `globals`, `ui` | yes | 2 | 2 / 1 |
| 01 | `levels/game/game` | yes | 1 | 2 / 1 |
| 02 | `contentlaunchsb` | yes | 6 | 7 / 4 |
| 03 | `movies` | no | 5 | 4 / 2 |
| 04 | `worldssb` | no | 4 | 4 / 3 |
| 05 | `contentsb` | no | 13 | 16 / 12 |
| 06 | `globalsfull` | no | 3 | 1 / 1 |

Packages 00–02 form `SuperBundleLayout/NHL_LaunchGroup`; 03–06 form `NHL_FullInstallGroup`.
`settings.bundleFormat` is `2651454669` on every title. A superbundle's content is **not**
confined to its own package. On NHL 22, `contentsb` (package 05) spans also land in packages
01, 02 and 04; `worldssb` (04) in 01 and 02; and `globalsfull` TOC chunks partly in 00.
Optional packages reuse data already shipped in earlier ones, so a replacement must account
for every bundle that shares a span.

## 4. Loading chain (observed)

```text
layout.toc ──superBundles[]──▶ Gen4b/<sb>.toc (binary)
                                   ├─ bundle table ─▶ bundle record (in the .toc) ─▶ spans in cas files
                                   │                      span 0 = bundle MANIFEST ─▶ ebx[] res[] chunk[] names
                                   │                      spans 1..n = the data of those entries, in order
                                   └─ chunk table ──▶ (cas id, offset, size)  "toc-level" chunks
cas data = run of blocks: [u32 BE decompressed size][u16 BE codec word][u16 BE stored size][payload]
```

### 4.1 `D1CE01` wrapper

`layout.toc`, `initfs_Gen4b` and every superbundle `.toc` begin with `00 D1 CE 01`. The payload
starts at **`0x22C`**. Bytes `0x08–0x107` look like a 256-byte signature. At `0x108` there is
an ASCII hex string (`xa37dd45ffe100bfffcc9753aabac325f07cb3fa…`, identical on every file and
title). The payload is plaintext except in `initfs_Gen4b`, whose payload is a single
`{ encrypted: <blob> }` object.

**Open:** whether the runtime verifies the signature. This is decisive for any edit that
rewrites a `.toc`.

### 4.2 DbObject encoding (used by `layout.toc`, `chunkmanifest`)

This is a type byte, a name (a C string, absent when bit 7 is set), and a value. Each decode
consumed its payload exactly, which is the verification.

| Type (low 5 bits) | Value |
|---|---|
| `01` list / `02` object | LEB128 byte size, then entries, `00` terminator |
| `06` | bool (1 byte) |
| `07` | string: LEB128 length incl. NUL |
| `08` / `09` | int32 / int64 LE |
| `0B`, `0C` | 8-byte LE double (`0C` seen as `Version: 3.0` in `chunkmanifest`) |
| `0F` / `10` | 16-byte GUID / 20-byte SHA-1 |
| `13` | blob: LEB128 length |

`layout.toc` keys are `superBundles[{name}]`, `fs: ["initfs_Gen4b"]`, `head` (an int) and
`installManifest`. The manifest holds `installChunks[{id, name, installBundle, alwaysInstalled,
superbundles, requiredChunks, estimatedSize, persistentIndex, …}]`, `installGroups` and
`settings.bundleFormat`.

**Patch link (observed):** NHL 24 `Patch/layout.toc` has `base: 981806`, which equals
`Data/layout.toc` `head: 981806`.

### 4.3 Superbundle `.toc` (binary, big-endian, offsets relative to `0x22C`)

The region arithmetic closes exactly on every file tested.

| Off | Meaning | Evidence |
|---|---|---|
| `+00` | bundle hash map offset (always `0x3C`) | map = `count × 4` bytes, ends at `+04` |
| `+04` | bundle table offset | 16 bytes per bundle exactly |
| `+08` | bundle count | |
| `+0C` | chunk hash map offset | `count × 4` bytes, ends at `+10` |
| `+10` | chunk GUID table offset | 20 bytes per chunk exactly |
| `+14` | chunk count | |
| `+18` (= `+1C`, `+24`) | chunk data offset | `u32[3 × chunk count]` |
| `+28` | chunk data word count | always `3 × chunk count` |
| `+20`, `+2C`, `+30`, `+34`, `+38` | **unresolved** | `+38` points at a signed-int table |

- **Bundle entry (16 B):** `u32 nameword · u32 (flags << 24 | record size) · u64 record offset`.
  The flag byte is `0x40` on every observed entry. **`nameword` is not a stable identity**:
  only 53 of 20 187 NHL 24 patch bundles share one with a base bundle, and all 53 differ in
  content. No bundle-name strings exist in any `.toc`, so bundle naming is **unknown**.
- **Bundle record** (at `offset + 8`, inside the `.toc`):
  `u32 size · u32 span count · u32 ×3 (0x20 each) · u32 0` is followed by a span stream. Each
  span is `u32 offset · u32 size`, preceded by a `u32 cas id` whenever its flag is set. The
  stream is followed by **one flag byte per span**. The entry's size word ends exactly at the
  last flag byte. Validated on all 245 735 records of the four trees. All 245 735 parse, and
  the manifests of all but 5 decode; those 5 are empty bundles (below).
- **Chunk GUID entry (20 B):** `GUID[16] · u32 (flags << 24 | index)`. The index is in `u32`
  units into the chunk data, and is always a multiple of 3.
- **Chunk data (12 B):** `u32 cas id · u32 offset · u32 size`.
- **cas id word:** `00 RR PP CC` = root (**0 = `Data`, 1 = `Patch`**, absolute), install
  package, cas number. For example `0x010203` is `Patch/…/nhl_installpackage_02/cas_03.cas`.

### 4.4 Bundle manifest (span 0 of each record)

`u32 BE size` is followed by little-endian fields: `magic 0x9D798ED6`, `total`, `ebx count`,
`res count`, `chunk count`, `strings offset`, `meta offset`, `meta size`.

After the header come:
- `total` SHA-1s;
- `ebx[] = (name off, size)`;
- `res[] = (name off, size)`, then `res type u32[]`, `res meta[16][]` and `res id u64[]`;
- `chunks[] = (GUID[16], logical offset, logical size)`;
- the strings.

The counts always sum to `total`. Five bundles are **empty** (zero entries, strings offset
`0xFFFFFFFF`).

**Observed:** for a sampled bundle, spans 1..n are exactly the ebx data then chunk data, in
manifest order, and the block sizes match the manifest sizes.

### 4.5 Data blocks and codecs

A block is `u32 BE decompressed size · u16 BE codec word · u16 BE stored size`. The codec word
is `CC 7H`: `CC` = codec, the low nibble `H` = bits 16–19 of the stored size. On 6 529 sampled
spans the blocks tile each span exactly (an earlier 16-bit reading did not).

| Codec | Share of sampled blocks | Identification |
|---|---|---|
| `0x11` | ~92% | every payload starts `8C 06`, **consistent with Oodle Kraken (inferred, not decoded)** |
| `0x00` | ~8% | stored (uncompressed) |

No other codec was seen. **Decoding (and re-encoding) `0x11` is the gating capability for
reading or modifying almost anything** (§7).

### 4.6 Payload types confirmed from uncompressed blocks (NHL 22 TOC-level chunks)

| Leading bytes | Count | Reading |
|---|---|---|
| `48 00 00 0C` | 9 743 | **unidentified** recurring binary header (mostly `levels/game` chunks) |
| `PTCH` v7 | 1 108 | **unidentified**; a GUID-tagged "patch" structure in `levels/game`, possibly sound-graph data |
| `CFX` + version `0F/10/11/1C` | 654 | consistent with compressed Scaleform UI movies (**inferred**) |
| `1A 45 DF A3` | 292 | Matroska/WebM video (movies) |
| `MVhd` | 107 | EA video header (movies) |
| `SBle` | 69 | the SND10 bank index already decoded for NHL 15–21 |
| `ST00` | 68 | closed-caption tracks: UTF-16 text referencing `TXT_…` keys, `CC` |
| `[{"URN":…` / `[ "/lua/…" ]` | 36 | JSON job/telemetry records; Lua script path lists (e.g. `/lua/scoreboxutil/penaltyutil.lua`) |

Inside codec-`0x11` blocks, literal `GNF ` headers appear right after Lua script-list chunks.
That is consistent with **PS4 textures being stored as GNF** (inferred), the format Studio
already decodes and encodes for NHL 15–21. The compressed asset data starts with
`RIFF … EBX EBXD`, a RIFF-framed ebx container (observed in a partially literal block).

## 5. Catalogue sizes (observed)

| Superbundle | NHL 22 bundles / ebx / res / bundle chunks / toc chunks | NHL 23 | NHL 24 base | NHL 24 patch (bundles / ebx) |
|---|---|---|---|---|
| `contentlaunchsb` | 9 678 / 65 k / 42 k / 28 k / 13.9 k | 10 258 / 122 k | 78 819 / 340 k | 8 074 / 120 k |
| `contentsb` | 27 582 / 298 k / 86 k / 70 k / 45.1 k | 35 648 / 201 k | 44 249 / 253 k | 10 723 / 100 k |
| `globals` | 42 / 941 / 5.3 k / 88 / 23.9 k | 43 / 985 | 42 / 974 | 4 / 936 |
| `globalsfull` | 1 / 1 / 1 / 1 / **38.3 k** | **800 / 129 k** | 767 / 139 k | 467 / 138 k |
| `levels/game/game` | 1 / 6.8 k / 5.4 k / 1.4 k / 1.7 k | 1 / 7.7 k | 1 / 7.4 k | 1 / 7.4 k |
| `movies` | 389 / 1.3 k / 6 / 0 / 427 | 339 / 1.2 k | 345 / 1.2 k | 8 / 27 |
| `ui` | 9 / 134 / 132 / 1 / 0 | 9 / 146 | 10 / 158 | — |
| `worldssb` | 5 512 / 153 k / 69 k / 70 k / 9.2 k | 5 521 / 161 k | 5 482 / 174 k | 910 / 152 k |
| **Total** | 43 214 bundles · 205 k distinct ebx | 52 619 · 245 k | 129 715 · 483 k | 20 187 · 131 k |

The **roles of superbundles shift between titles.** NHL 22 `globalsfull` is essentially
chunks-only, while NHL 23/24 put about 130 k ebx there. NHL 24 `contentlaunchsb` grows to
79 k bundles because of a new `ui/textures` tree (about 200 k ebx: Ultimate Team cards, player
portraits, banners). **Never assume one title's placement for another.**

NHL 24 patch: the patch `.toc`s list complete bundle records that point at both trees: about
92% of spans go to `Patch` cas files and about 8% to base `Data` cas files, a delta
re-declaration. **How the runtime chooses a patch bundle over a base bundle is unknown**,
because `nameword` does not match across trees.

## 6. Where Studio's content families live (observed asset paths; NHL 22 unless noted)

| Family | Asset path convention (ebx) | Notes |
|---|---|---|
| **Roster database** | **not located** | No TDB in names or uncompressed chunks. Candidate: `globals/chunkfilecollector`, `…uikettle`, `…uikettle_nhluidata`, `…uicommon(_nhluidata)`. These are tiny ebx (332–382 B) that by name collect "chunk files", likely legacy Kettle-era files stored in the 23–38 k TOC-level chunks of `globals`/`globalsfull`. Needs codec `0x11` to read the collector. |
| **Localization** | **not located** as assets | Only `content/common/configs/mappings/worlds_banner_language_mapping`. Captions reference `TXT_…` keys like the Kettle dictionaries. Likely in the same chunk-file collection (unverified). |
| Leagues / team identity | `content/character/skater/league/<league>/<team>/<brand>/<variant>/…` (380 league/team dirs on NHL 22) | Leagues seen: `nhl`, `ahl`, `echl`, `chl`, `elite_swedish`, `elite_swiss`, `elite_finnish`, `wc`, … |
| Kits / jerseys | same tree: `jersey_*`, `pant_*`, `font_*`, `helmetlogo_*`, `jersey_font_placement_*` (`…_color`/`_normal`/`_coeff`) | Goalie: `content/character/skater/goalie/jersey/<brand>/…` with cloth assets. Shared weaves: `…/league/shared/textile/…` |
| Team logos | `content/common/logo/<team>/logo_<team>_*` (189 dirs) | Also custom-logo fonts `content/common/logo/font_*`. NHL 24 adds `ui/textures/common/logos/…` |
| Player heads | `content/character/skater/shared/head/licensed/<first_last>/…` (621 on NHL 22); generic: `…/shared/head/generic/male_<n>/…` | Mesh: `components/head_<name>_mesh`; textures `textures_player_head_brt`; per-head `playerhelmet_placement` |
| Hair | `content/character/skater/shared/head/hair/male_<n>/<style>/…`; licensed `…/licensed/<name>/components/hair_<name>_*` | `meshvariationdb_gen4b` per style |
| Portraits | NHL 24: `ui/textures/common/playerportraits/p<id>` (42 k) | Same `p<id>` naming as Kettle-era portraits; NHL 22/23 have only capture settings (`content/ui/*portraitcapturesettings`) |
| Arenas / ice | `content/worlds/stadiums/<team>_<id>/…` (75 dirs on NHL 22, e.g. `anaheim_0`, `tampa_26`, `custombuffalo_700`, `outdoortier1_900`); rink `content/worlds/rink/…` | Lighting per arena `…/lighting/…_authentic`. The `<name>_<id>` suffix likely matches the arena id (unverified) |
| Boards / ads / glass | `content/worlds/textures/boards/…`, `content/worlds/configs/adboards/<venue>/…`, `content/worlds/components/structures/<venue>/rink_boards` | |
| Scoreboard / scorebug | Arena boards: `content/worlds/scoreboard/<venue>/…`, `…/scoreboard/textures/<team>/…`. On-screen scorebug: Lua (`/lua/scoreboxutil/…`) + UI | |
| Equipment | `content/character/skater/player/<category>/…` (skates, sticks, gloves, helmets, casual wear), goalie gear `content/character/skater/goalie/<item>/<brand or nhl/team>/…` | 100–195 k matching names per title |
| UI | `ui/assets/…`, `ui/features/…`, `ui/fonts/…` (font res type `0x063994c5`), NHL 24 `ui/textures/…` | UI movies appear Scaleform-like (§4.6) |
| Audio | `sound/speech/{commentary,announcer,threes,fe_bap,fe_woc,gauntlet}/…`, `sound/sfx_fb/…`, `sound/music_fb/…`, `sound/nhlharmony/…` | Banks are `SBle` (SND10). Commentary databases at `sound/speech/*/database/*` |
| Movies | `nhl/data/common/movie/…`, `nhl/data/kettle/movie/…` | WebM / EA `MVhd` payloads |
| Crowd / bench / NIS | `content/character/crowd/…`, `content/character/bench/…`, `content/nis/…` | |

Most common res types by count (names inferred from asset paths, not decoded):
- `0x6bde20ba` (151 k): texture-like (`…_color`, `…_normal`);
- `0x49b156d4`: mesh;
- `0xb2c465f6`: audio wave;
- `0x0e2b02f7`: shader bytecode;
- `0x428ec9d4`: bundle reference tables.

## 7. What this means for modifying these games

1. **Codec `0x11` is the first gate.** Without a decoder, Studio can map and copy data but
   cannot read ebx, textures, meshes or the database. Without an encoder, edits would have to
   be stored with codec `0x00`; whether the runtime accepts that is **unknown** but plausible,
   since about 8% of shipped blocks are already stored. The decoder is a project decision.
   The options include licensing the vendor library or a clean-room implementation. This
   document does not choose.
2. **TOC signature.** Every `.toc` and `layout.toc` carries a 256-byte header region that looks
   like a signature. Whether shadPS4 and the console verify it decides whether Studio can
   rewrite indexes or must avoid them.
3. **Patch-overlay strategy.** The game already supports a `Patch/` tree that re-declares
   bundles and points at both trees by absolute root id. An overlay that writes new cas data
   plus re-declared records is the natural non-destructive install route. Its precedence rule
   is unknown (§5).
4. **Encrypted `initfs`.** Contents unknown. Unlikely to be needed for asset/DB edits, but may
   hold configuration Studio would want to change.
5. **Continuity with Kettle-era knowledge.** Captions use `TXT_…` keys, portraits keep `p<id>`
   names, audio banks use `SBle`, and textures look like GNF. Existing Studio decoders for
   GNF, SND10 and possibly TDB (if the database survives as a legacy file) can likely be
   reused once codec `0x11` is solved.

## 8. Open questions (each should become a research or runtime card)

| ID | Question | Why it matters |
|---|---|---|
| FB-Q1 | Is codec `0x11` Oodle Kraken, and how will Studio decode/encode it? | Gates all content reads and writes |
| FB-Q2 | Does the runtime verify the `D1CE01` signature on `.toc`/`layout.toc`? | Decides whether indexes can be rewritten |
| FB-Q3 | How is a patch bundle matched to its base bundle? What do header words `+20/+2C/+30/+34/+38` hold (bundle names/hashes)? | Overlay install and bundle naming |
| FB-Q4 | Where is the roster database (legacy chunk files via `chunkfilecollector`?) and is it still TDB? | Core of Studio's editing model |
| FB-Q5 | Where is localization, and what is its format? | Phase 1.3/2.3/3.3 equivalents |
| FB-Q6 | ebx RIFF format and type system (type GUIDs, field layouts) | Reading any ebx (kits, arenas, config) |
| FB-Q7 | Texture res header + chunk = GNF? (res type `0x6bde20ba`) | Logos, kits, portraits, ice |
| FB-Q8 | Identify the `48 00 00 0C` and `PTCH` payloads | Large share of `levels/game` chunks |
| FB-Q9 | Does the runtime accept codec-`0x00` (stored) replacement blocks? | Write path without an encoder |
| FB-Q10 | Is shadPS4 able to run NHL 22–24 for runtime verification? | Every runtime card |
| FB-Q11 | PS5 (NHL 25–27) layout | No data examined |

## 9. Reproducing

Scripts in [docs/research/frostbite-2026-10-13/probes/](../research/frostbite-2026-10-13/probes/)
are read-only Python and need no dependencies (run with `python -I`):

- `fbdump.py obj <file>` decodes a DbObject (`layout.toc`, `chunkmanifest`).
- `tocprobe.py <toc>` prints the superbundle header and region arithmetic.
- `bundleprobe.py <cas> <offset>` decodes one bundle manifest.
- `inventory.py <install> <Data|Patch> <out> <tag>` writes the full catalogue (`.jsonl`,
  100–220 MB per title, kept out of the repo) and a summary.
- `analyze.py`, `codecs.py` and `findsig.py` produce the folder/family tables, codec tallies
  and signature census.

The per-title summaries, analyses and decoded layouts are in
[docs/research/frostbite-2026-10-13/data/](../research/frostbite-2026-10-13/data/).
