# Arena assets and presentation: findings

## What this pass establishes

The [catalog](arena-reference-catalog.md) is generated from current ISO/archive/TOC reads, not a hand-written list of expected venues. It covers available title/platform sources from NHL 07 through NHL 27 plus Legacy and Recompiled, with NHL 25 explicitly unavailable. It preserves source-year variants, unnamed asset groups, shared environments, and database rows without a proven art binding.

The asset census is broader than arena folders: it includes related camera, crowd, lighting, presentation, screen, movie, and audio names. Frostbite world/movie bundles are inspected in full, including manifest chunks; relevant other bundles and core globals/game registrations are inspected as well. This is a broad inventory, **not an exhaustive resolved semantic dependency graph**. Unnamed resources, dynamically selected assets, TOC-level chunks outside bundle manifests, and runtime consumers still need the work below.

No cross-generation arena was compiled or installed during this research. The implementation roadmap treats preview, conversion, registration, and runtime playback as separate gates.

### Existing Studio capabilities to extend

| Area | Existing implementation / evidence | Missing for the requested workflow |
|---|---|---|
| Arena entity editing | Arena list/detail, metadata-driven fields, team relationships, arena asset and venue cards | Cross-game venue identity, year variants and independently selectable presentation presets |
| Gen7 venue art | `venue_art` lists material/raster associations and previews qualified textures on its measured ten title/platform combinations | General assembled scene preview, full material editing, and target NHL 24 compilation; extending the measured list requires evidence |
| Ignite assets | MCD/GNF readers and vault research, directory/level listing, texture tooling | Complete assembled arena scene, live dependency graph and qualified cross-engine writes |
| Frostbite access | Repository TOC/CAS readers plus NHL 24/27 research tools; NHL 26's separate research viewer | Supported arena profile, typed registry/data joins, full materials, complete scene compiler and additive registration |
| Foundation installation | A concrete moves-only overlay/receipt and verification path | Multi-package composition and rollback that retain unrelated modifications |

The proposed interface reuses these entry points; it does not assume an already finished arena importer exists behind the current Arena page.

## Container and format map

| Family | Located representation | Extraction / interpretation | Remaining qualification |
|---|---|---|---|
| NHL 07–10, Xbox 360 / PS3 where supported | ISO XDVDFS or installed archives; BIG4/BIGF with embedded directories; RX2/RPSGL venue packages | Read archive members using exact offsets/lengths; unwrap measured RefPack/LZX variants before parsing mesh/raster data | Early scene, material, animation, and database relationships need per-title checks; a later RX2 writer is not automatically valid here |
| NHL 11–Legacy | EB archives, RX2/RPSGL scene packages; NIS and face-animation archives | EB 16-byte rows can have compressed length at +4 and decoded length at +8. Read the compressed length when nonzero. EB faceposer version `0x00200400` is separately accepted | Resolve complete scene and NIS graph; distinguish static textures from runtime screen controllers |
| Legacy Recompiled | Current `F:\Games\NHL Legacy\game` archive layer | Reuse Gen7 member readers, retain this install's provenance | Historical byte equality with a prior Legacy copy is not proof that today's modified installs are identical |
| Ignite NHL 15–21 PS4 | `nhl/data/kettle/rendering/stadiums/<venue>/...`; MCD meshes, GNF textures, skeleton resources; VLT/BIN attribute collections | Read scene/model/material/texture references through the vault, not filename proximity; decode chunkzip wrappers where present | Assemble every component, transforms, LODs, crowd/collision/lighting dependencies; qualify animation/controller semantics |
| Frostbite NHL 22–24 PS4 | Data/Patch `Gen4b` TOCs and CAS; RIFF EBX, RES, bundle and TOC chunks | Preserve per-layer manifests, resource types/IDs, stored SHA-1, physical spans and logical names. Bounded decoded specimens verify hashes and sizes | Resolve typed fields, TOC-level chunk dependencies, complete scene imports, actual registry/DB selection, and target writers |
| NHL 26–27 PS5 | Data/Patch `Ps5` TOCs/CAS; 64-bit CAS locators with persistent install-package IDs | Resolve locators through layout metadata; use PS5-specific EBX/resource/texture interpretation | PS5-to-PS4 resource layouts, material/shader support and presentation node conversion remain independent requirements |

The EB compressed-length distinction was checked against `crates/tdb-archive-big/src/eb.rs`; using +8 indiscriminately crosses adjacent members and can run past the archive. The census now bounds reads by the actual stored span. Other unrecognized archive families are retained in source summaries rather than treated as empty.

### Arena components and their ownership

| Component | Observed source organization | Port obligation |
|---|---|---|
| Building, rink, glass, netting, props, roof, stands, rails, VIP boxes | Gen7 `stadium_<art>_container_<n>`; Ignite separate `<venue>_*` MCDs; Frostbite scene/mesh/resource/chunk graph | Preserve assembly transforms, units, bounds, LODs, material sections, visibility and collision; inspect every component, not just the arena bowl |
| Materials and textures | Gen7 material/raster parameter blocks; Ignite typed vault references to GNF; Frostbite material EBX and texture RES/chunks | Decode actual bindings; preserve packed channels, alpha, color space, mipmaps, UV sets and emissive behavior |
| Ice, center logos, banners, boards | Mixed arena, team, league, and mode selection | Separate venue identity from branding; show shared-resource impact before editing |
| Jumbotron and power ring | Gen7 texlibs and arena materials; Ignite numbered common textures, scoreboard/animated-sign meshes and catalog packs; Frostbite screen materials and controller resources | Trace animation source, UV/layout mapping, event selection, timing and display controller. Screen textures are not a complete animation package |
| Lighting | Ignite numbered stadium vaults contain rigs/lightmaps; Frostbite per-world lighting and visual-environment assets | Preserve lightmap UVs and transforms; translate light/probe/environment settings to target-supported structures. Do not assume source shader/lightmap equivalence |
| Cameras and pregame | Camera resources, NIS/override groups, presentation graphs/timelines, movies | Resolve paths, rigs, lenses, cuts, actors, events, skip logic, audio priorities and gameplay handoff |
| Crowd and arena audio | Placement/appearance resources; goal/period horn, chant and intro-song references | Preserve seating placements and event associations; permit team-specific audio overrides independently of geometry |

## Database joins: established versus unresolved

**NHL 07:** `exhibitionarena` is a selector. `arenaid` points to the `index` key in `stockarenatable` or `customarenatable`, selected by `table`. `exhibitionarena.index` relates to the exhibition team; it is not automatically a venue mesh ID. The catalog exports resolved selector rows plus the underlying tables. The `roof`, `walls`, `jumbotron`, `icelogo`, dressing and sound selectors must be retained independently. The link from these selectors to a complete stadium package is still unqualified.

**Later Gen7:** previous measured work establishes `arenaartid` → `rendering/stadium/stadium_<id>_container_<n>.(rx2|rpsgl)` for NHL 09–Legacy and jumbotron `texlib_<id>` for NHL 12–Legacy. The new census checks current names. NHL 08 numeric filename matches remain candidates until independently qualified. Center logos and many banners are team-art keyed. `bouncemap` is not given a proven board-physics interpretation merely because of its label; earlier evidence only establishes candidate ID overlap with ice texture families.

**Ignite:** the preserved research map joins arena art IDs to VLT level IDs and named venue directories. Multiple levels can share an ID. Venue changes can also affect `ArenaID`, `RinkId`, center logo/line, crowd placement and NIS overrides. NHL 18 onward moves many presentation controls into team records, so copying an arena row is insufficient. The census reports historical mapping provenance and unresolved rows rather than pretending that directory suffixes are selectors.

**Frostbite:** bundle paths such as `content/worlds/stadiums/boston_2/...` establish an asset namespace, not a proven arena-table foreign key. A source-specific typed registry/DB read and team-to-scene consumer trace is required before allocating new IDs or claiming team assignment. Existing NHL 24 executable/type research can narrow this investigation; a same-name property or type GUID is not enough. No TDB anchor was substituted for a Frostbite database.

Supporting prior research: [Gen7 arena joins](../overnight-2026-09-21-pm/19-gen7-arena-venue-asset-join.md), [Ignite venue inventory](../overnight-2026-09-21-pm/21-ps4-arena-venue-asset-inventory.md), [venue listing contract](../overnight-2026-09-22-pm/04-ps4-venue-listing-contract.md), and [team/presentation field migration](../overnight-2026-09-19/06-ps4-team-identity-and-presentation-map.md). Earlier NHL 20 anchor findings may describe a superseded NHL 19-derived anchor; this census records the current NHL 20 database hash and does not reuse old row counts as current facts.

### Board ads are a useful warning about shared dependencies

Ignite board meshes bind blank base textures and decal slots. Separate arrays and override groups contain the sponsor logos; the older `common/adboard_*` family is largely building signage. One NHL 21 decal array was historically reachable from 37 rinks. Typed references are parsed evidence, while “array position equals panel position” and game-mode selection remained consumer hypotheses. An arena-specific edit must clone/rebind the relevant shared dependency when requested, not overwrite a globally shared sponsor array. See the corrected [board binding study](../overnight-2026-09-20/12-dasher-board-ads-binding-chain.md).

## City videos: concrete evidence and NHL 24 route

The live Ignite indexes contain `nhl/data/common/movie/cityintro/scenic_XX_YY.mkv` and companion `.json` files. The sidecars are JSON with C-style comments; after removing comments they parse as frame/value tracks. For sampled `scenic_00_01`, tracks are:

- `ShouldRender`: enabled at frame 0.
- `CityOverlay`: off at 0, on at 30, off at 360.
- `ScenicPBP`: off at 0, on at 60.

The same numbered clip is byte-identical in the inspected NHL 15, 16 and 17 copies: decoded SHA-256 `351cec81f4a9a035e2355190ae6181ff5ad68925efa77cd73e3e515d42b69d2d`. `ffprobe` identifies VP8, 1600×900, Vorbis 48 kHz audio, duration 13.580233 seconds. The NHL 18/19 sample has SHA-256 `0512317b5f75aaeba940852007ca3d89f7d788b8909f77de85cb24127f347b11`, VP8 1600×900, duration 13.012999 seconds, and no audio stream. The reported video rate is approximately 29.97 fps; frame metadata must be mapped against actual timestamps, not rounded duration alone.

These codec results apply to the sampled clip, not every movie. Full filename inventories and parsed sidecars are in each source's `movies.json` and `payload-evidence.json`. NHL 20/21 retain only two cityintro-path records in this pass and no `.mkv` city clips there; that is a path-census result, not proof that every possible city shot is absent elsewhere. `XX` has not been certified as a city or team database key. Clip-to-city assignments must be verified from selector data or visual review.

NHL 24 has Frostbite movie infrastructure, and previous NHL 24/27 research identifies `MovieTexture2Asset`, `VideoStream`, `MovieFrameInfo`, presentation tracks and event graphs. The first implementation should register one city clip through a **native NHL 24 movie resource**, invoke it before the existing arena introduction, and translate the sidecar events into target-supported actions. Do not copy Ignite sidecars into the game tree and assume they are consumed.

This pass also finds six `cityintro`-path records in NHL 24, but they are `ui/textures/common/logos/nbc/cityintro/logo_0` / `logo_1` and their binding-table/blueprint companions—not the scenic videos. NHL 26 has the corresponding six logo records. Their survival is useful context for overlay artwork, not proof that the old city-movie sequence is still wired up. Sampled NHL 24 movie transition EBX records expose imports and parse correctly; full stream registration and pregame event consumers remain unqualified.

Default preset behavior: native NHL 24 commentary, source video audio disabled unless explicitly selected, target city-overlay timing, and clean return to the arena introduction. Authentic-source presets may enable source audio when compatible. Skip must cancel pending overlay/commentary events, stop audio, release video resources and hand off once. Missing or incompatible clips fall through to the native intro. If target nodes cannot invoke the sequence, first trace the missing consumer; only then specify a narrow executable adapter.

Stream inspection follows the available [FFmpeg tools](https://ffmpeg.org/ffprobe.html). Codec/container readability in FFmpeg is not evidence of NHL 24 playback support.

## NHL 26/27 reuse and conversion constraints

The completed named asset grouping contains 75 stadium/environment tokens in NHL 24 and 80 each in NHL 26/27. Compared with NHL 24, the later sources add `arcade1_600`, `default_no_hypestage`, `frontend_woc_103`, `utah_30`, and `utahgeneric_30`; none of NHL 24's tokens disappear. NHL 26 and 27 have the same token set in this census. These are namespace comparisons, not proof that all same-token arenas are unchanged. The preliminary 81-token bundle-only count also included a bundle-name token not recovered as a separate named asset group; the final asset-based catalog is the reporting baseline.

NHL 26's existing research project at `S:\NHL26\research` supplies useful container, texture and mesh code. Its Asset Studio README explicitly limits the initial catalog to `contentsb.toc`, uses heuristic albedo binding, and documents unsupported mesh/tiling cases. Character preview success therefore does not qualify assembled arena previews. Extend scanning to world bundles and use typed material references before adopting those results.

The prior [NHL 27 compatibility study](../nhl27-port-compatibility-2026-10-15.md) establishes changed presentation type signatures and sampled PS5 texture headers of 184 bytes versus 132 bytes on PS4. Five small camera controls normalize to target-identical payloads; that narrow result is not a universal EBX converter. Source-only graph nodes, material shaders, wave resources, texture layouts and camera lens/tween structures need explicit adaptation or a documented unsupported result.

The new NHL 27 specimen pass also encounters a concrete additional blocker: `content/worlds/arenagraphics/widgets/arena_graphics_jumbotron_team_name_widget_base_ecsprefab.nopolicy` has a valid stored hash and decoded length, but the existing RIFF fixup reader rejects its `EBXS` EFIX trailing data. The failed structural decode is retained in `data/nhl27-ps5/payload-evidence.json`. Screen-widget/ECS-prefab support therefore needs its own measured reader variant; removing the trailing bytes to force a parse would not establish compatibility.

Arena meshes should be reconstructed as NHL 24-native scenes with target materials. Baking a preview into GLB preserves an editable interchange representation, not native lighting, collision, streaming or presentation behavior. Keep source EBX/VLT metadata and unresolved fields alongside each exported component.

## Foundation installation constraints

The existing moves-only installer and receipt establish a `game-mods` overlay ahead of `game-UPDATE` and `game`. Its installer refuses an existing overlay and its verifier expects exactly the receipt's file inventory. It cannot simply be reused to add arena files to this already-modified foundation.

The arena writer needs a package composer that starts from the current overlay and preserves its executable and ANT changes, reconciles shared TOC/CAS edits at asset level, reserves collision-free identifiers and storage, and creates a combined receipt. Original game and update data remain protected. Rollback must remove only the arena package contribution or restore the preceding combined overlay; the old moves-only rollback is not a general multi-package uninstaller.

## Remaining research gates

1. Complete typed Frostbite arena registration/database resolution, including registry and TOC-level chunk payloads.
2. Expand sampled EBX import evidence into a complete closure for each pilot scene; resolve external resources/chunks and shared dependencies.
3. Qualify all arena mesh/material/lighting variants and screen controller formats; do not extrapolate from head meshes.
4. Resolve NHL 07 dressing-to-scene mapping and independently qualify NHL 08 asset selector semantics.
5. Verify city-index selection, cinematic camera/timeline consumers and skip behavior in NHL 24.
6. Establish additive arena registration, limits, persistence, mode coverage and runtime results.

These gates are explicit roadmap tasks. The filename census and format specimens do not conceal them behind a “fully portable” label.
