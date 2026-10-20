# NHL 24 arena library implementation roadmap

## Goal and research baseline

Build a local library containing every available arena and source-year presentation variant, then compile selected variants into the existing NHL 24 foundation. Teams must be able to share a venue while choosing independent presentation presets. Studio must preview, extract, edit, validate, assign, install and roll back these packages.

Research inputs: [reference catalog](../research/arena-library/arena-reference-catalog.md), [findings and limitations](../research/arena-library/findings.md), [machine-readable source/variant records](../research/arena-library/data/catalog.json), and [foundation identity snapshot](../research/arena-library/data/foundation-snapshot.json).

The source census and bounded format/video specimens are delivered. Full dependency closure, Frostbite arena/database registration, target scene writing and runtime playback remain engineering work. NHL 25 is unavailable. NHL 26 and 27 are independently inventoried rather than assumed compatible. No source arena is labeled NHL 24-installable merely because its files were found or previewed.

The destination is `F:\Games\NHL 24`, with current `game`, `game-UPDATE`, and `game-mods` layers. The existing moves-only executable/ANT modifications must survive arena package composition. This is a local expandable library; an occupied-slot swap is a diagnostic control, not the final deliverable.

## Work packages and release gates

Implementation update (2026-10-17): [NHL24 binding milestone](../research/arena-library/nhl24-binding-implementation.md) locates the live database and metadata through legacy collectors, resolves all 531 team-to-arena joins, and stages a separate arena database record with a one-team assignment. Byte-exact preservation and CRC checks pass. A1 is partially delivered; art-ID-to-scene routing and full dependency closure remain unresolved. A3's database-only proof shares Boston's art ID and is not installable or game-verified.

| Package | Work | Required output / exit gate |
|---|---|---|
| A0 — Source adapters | Turn the research readers into supported domain adapters for ISO, BIG4/BIGF, EB, Ignite vault, and Frostbite Data/Patch sources. Retain archive identity, patch occurrences and unknown variants. Include TOC-level chunks and metadata collectors. | Reproducible typed asset manifest; strict bounds and integrity failures; no source writes; source coverage and missing/ambiguous files visible |
| A1 — Arena identity and registry | Resolve NHL 07 selector tables; qualify NHL 08 art bindings; refresh Ignite vault joins live. For NHL 24, trace team → arena record/registry → scene, rink, presentation and mode dependencies. Locate the actual persistence representation and limits. | Machine-readable binding chains with typed keys, source evidence and reverse sharers. One existing NHL 24 team assignment round-trips without changing unrelated teams |
| A2 — Native NHL 24 control | Resolve one complete Boston NHL 24 scene, including world transforms, meshes/LODs, materials/textures, collision, crowd, lighting, screens and presentation. Export and reconstruct its target resources. | Complete dependency closure; strict reparse; original-versus-rebuilt scene comparison; installed control loads correctly before cross-engine imports begin |
| A3 — Additive registration | Allocate a new arena identity, scene/resource/chunk identifiers and required registry entries. Register a private clone of the control arena, leaving the original selectable. Persist assignments in supported modes. | Two distinct arena choices coexist; two teams can independently select them; save/reload works; capacity and unknown mode limitations documented |
| A4 — Conversion pilots | Port Boston from NHL 26, NHL 27, Ignite NHL 15 and Gen7 NHL 14 separately, using their original components. Add one international rink and one outdoor/custom scene as topology/lighting controls. | Per-family conversion reports; valid complete scenes; target materials and collision; source appearance comparison; no claimed support inferred from another platform or title |
| A5 — Presentation packages | Resolve jumbotron/power-ring controllers, event cues, board overrides, lights, cameras, entrances, audio and movie resources. Preserve authentic presets and create team-configurable overrides. | A complete pregame → gameplay → goal → intermission → end-of-game sequence with correct events, audio and skip handling |
| A6 — City intros | Map verified Ignite clips to cities; import one supported movie resource into NHL 24 and invoke it before the native arena intro. Translate frame tracks and qualify audio policy. | Normal playback, skip at several timestamps, missing clip fallback, no duplicate commentary/audio, one clean gameplay handoff |
| A7 — Studio and batch library | Integrate the proven adapters/compiler with Arena UI, package jobs, assignment previews, diagnostics and install receipts; batch-convert only supported component families. | End-to-end import/edit/assign/build/install/rollback workflow; failures isolated per package; every advertised capability backed by evidence |

A0–A1 resolve the critical unknowns. A2–A3 establish that the destination can support a library rather than only replacements. A4 and A5 have separate gates so an imported building cannot be presented as a complete authentic arena package. A6 can proceed once the target movie/sequence route is qualified; it does not need every mesh converter to finish.

## Library and API contracts

Use versioned serialized records in the domain layer, not frontend-only state:

- `VenueIdentity`: stable library UUID, verified names/aliases, location, venue kind, and reviewed relationships to historical buildings. Name similarity creates a review candidate, never an automatic physical-venue merge.
- `ArenaVariant`: UUID, venue identity or unresolved identity, game ID/title year/platform/build, source fingerprints, scene roots, arena/database binding evidence and capability status. Preserve all source-year variants even when underlying blobs deduplicate.
- `AssetManifest`: original logical name and identity; physical archive/member or TOC/CAS span; kind/native format; stored/decoded hashes where measured; dependency edges; scene ownership versus shared usage; unresolved references and evidence level.
- `PresentationPreset`: references to venue variant, team/league branding, ice/boards, screen content/controllers, lighting, cameras/intro sequence, audio and optional city video. An authentic preset is immutable; edits create a derived preset.
- `TeamArenaAssignment`: target build identity, team key, arena variant, presentation preset and supported mode scope. Preview includes affected database/resource records and shared-resource consequences.
- `ArenaBuildReceipt`: compiler/profile versions, source fingerprints, target identity, allocated IDs, changed assets and physical files, dependencies, validations, coexistence conflicts, predecessor receipt and rollback data.

Storage: content-addressed local blobs for immutable source payloads; versioned manifests for edits/presets; separate target outputs per NHL 24 build. Generated preview assets are disposable caches. Deduplication does not erase provenance or layer occurrences. Unknown native metadata is retained alongside interchange exports.

Add typed domain/API operations for source indexing, variant queries, dependency inspection, component preview/export, staged replacement, preset creation, assignment preview/staging, build validation, install planning, installation and rollback. Long operations use cancellable jobs with progress and durable diagnostics. Existing `ArenaViewDto`, asset cards and assignment validation remain entry points; extend them with library variant and capability references rather than changing current numeric arena IDs into cross-game identifiers.

Capability states are per operation and per component: indexed, readable, previewable, exportable, editable, compilable, installable and game-verified. An unresolved dependency blocks compilation of the affected package; a missing optional city video can use an explicit native-intro fallback. Never hide an incomplete package behind an overall green badge.

## Conversion and editing behavior

**Scene interchange.** GLB/glTF carries assembly hierarchy, component transforms, material slots, UVs, LOD references where representable and animations that have qualified decoders. Sidecar metadata retains collision, lightmap bindings, streaming/visibility, native shader parameters, cue graphs and unresolved data. Use native descriptors to determine coordinate system, handedness and units; record every conversion and its inverse. Validate rink dimensions and faceoff/goal/bench positions numerically.

**Materials.** Decode typed bindings first. Map source diffuse/color, normal, coefficient/roughness/specular/mask, opacity, emissive and lightmap inputs to measured NHL 24 shaders. Preserve color spaces, channel packing, UV sets and mip chains. A heuristic albedo match is allowed only as a labeled preview candidate, never a compile-time dependency. Unsupported shaders require an explicit conversion recipe or block the component. Default edits affect a private copy; updating a shared library asset requires an impact preview and an explicit user choice.

**Screens and lighting.** Separate physical screen mesh/UV layout, image/video content, animation/controller and event selection. Convert or reconstruct each part; a resized texture does not qualify a power-ring animation. Keep lighting rigs and baked lightmaps together. If source bake data cannot be represented correctly, mark the preset incomplete until a target lighting solution is authored and reviewed; do not silently substitute generic arena lighting.

**City video.** Preserve the source clip and metadata; package a target-compatible stream only after NHL 24 decoding is demonstrated. Default to muted source audio and native commentary, with per-preset audio selection. Convert `ShouldRender`, `CityOverlay` and `ScenicPBP` timing using stream timestamps. Cancellation clears queued events and releases resources. Missing/disabled clips continue with the native introduction. Do not assign `scenic_XX` to a city by numeric coincidence alone.

**Presentation graphs.** Translate source behavior into target-supported nodes/fields and explicit event mappings. Source-only camera, tween, visual-environment or audio nodes require adapters. Reuse the five previously qualified small camera-control conversions only within their tested signatures. No general raw NHL 27 graph or executable transplant is authorized by this roadmap.

Before importing NHL 27 jumbotron widgets, qualify the `EBXS` ECS-prefab fixup variant found in the current specimen pass. Its payload hash/length validates but the existing reader reports trailing EFIX data; preserve that failure as a targeted A0/A5 blocker.

## Studio workflow and installation

The Arenas list becomes a library view with filters for source game/year/platform, physical venue, venue kind, readiness, assignments and installed state. Detail areas: Overview; 3D Components; Materials; Screens; Lighting; Cameras/Sequences; Audio/City Intro; Database Bindings; Teams; Build/Install History. Display all year variants under the same reviewed venue identity, without discarding older versions.

The viewport supports orbit/free camera, component isolation, LOD inspection, texture-channel views, bounds/collision overlays and before/after comparisons. Native sequence playback is labeled separately from approximated preview playback. Unsupported components remain listed with the precise missing reader or binding.

Assignment flow: select team(s), variant and preset; preview compatibility, shared effects and mode scope; stage the change; validate/build; inspect install plan; activate the composed package. Save uncommitted edits in the project so switching variants does not lose work. Partial conversion jobs retain successful artifacts but do not activate partial arenas.

The package composer reads the **current combined overlay**, resolves source/update/overlay precedence using the foundation's measured loader behavior, and edits manifests/resources at asset granularity. Allocate unique GUIDs/RES IDs/chunk IDs and collision-free CAS placement; preserve every retained mod contribution. Preserve relocation, hashes, resource metadata, alignment and catalog/bundle registration requirements.

Stage a full combined overlay beside the active one. Require the game closed before activation. Verify staged files and all reachable changed resources, retain the predecessor overlay/receipt, then activate through the foundation's established directory mechanism. Recovery must detect interrupted staging and leave a usable prior overlay. A later package removal recomposes retained packages; it must not invoke the old moves-only rollback against a now-combined overlay.

## Test and acceptance matrix

1. **Reader integrity:** truncated archives, oversized counts, unsupported codecs, wrong hashes, invalid compressed lengths, out-of-range spans, ambiguous layers, missing imports and cycles. Assert explicit failure, not an empty inventory.
2. **Identity and catalog:** all catalog targets accounted for; NHL 25 missing; source-year variants preserved; same-name/different-build assets distinct; duplicate bundle appearances not counted as distinct venues; database aliases and unmatched rows visible.
3. **Round-trip scenes:** native control and each engine-family pilot; transforms, LODs, material slots, texture channels, collision, rink dimensions, lighting and crowd placement; compare rendered views and numeric properties.
4. **Assignments:** two teams sharing one venue with different branding/audio; reassignment without modifying the original venue or another team; persistence and supported mode coverage; ID exhaustion/conflicts fail safely.
5. **Presentation:** home/away changes, alternate teams/leagues, goals, penalties, intermissions, overtime, shootouts and game exit; camera clipping, screen UVs, event timing, lighting and audio synchronization.
6. **City intros:** native FPS/timing, audio/no-audio clips, skip before/after cue boundaries, repeated games, missing clips, disabled video and one-time gameplay handoff.
7. **Installation:** coexists with current moves-only package; protected originals unchanged; deterministic build from identical inputs; interrupted activation recovery; rollback restores predecessor hashes; removing arenas preserves unrelated mods.

Automated/static checks, launch checks and user gameplay verification are separate receipt fields. The first release requires at least one game-verified package from each advertised source family and explicit coverage for any advertised special rink type. Broader game modes remain unavailable until their assignment/persistence consumers are qualified.
