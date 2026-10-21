# Head extraction — 19–20 October 2026

## Plain-language summary

NHL07's extraction gate pointed at an ISO storage directory. The gate now includes source-folder controls for selecting the extracted game. Export prioritizes the full appearance catalog, including player, generic/CAP, coach and referee assets, with geometry in GLB and textures in DDS.

NHL22–24 now have a native Frostbite exporter. All 20 requested base game/platform combinations passed offline corpus checks, plus NHL24 Patch. These checks establish extraction and file validity, not in-game rig or animation compatibility.

## Ownership and integration

Implementation base: `ad12f4b6`; isolated checkout: `C:\Users\puckh\.codex\worktrees\headmesh-extraction\nhl-modding-studio`.

Main integration baseline: `3cb0cf0c`. The user confirmed Claude finished its commit pass and moved to wiki work. Package changes were integrated without staging or committing, preserving the newer portrait code and unrelated wiki edits. Three conflicts were reviewed earlier versions of this same extraction package; the final qualified versions replaced them. Preimages, hashes and the integration plan are retained in `E:\Repositories\nhl-modding-studio\local\headmesh-integration-20261020`.

Original games and ISOs remain read-only. Process inspection before the integration build found no running `nhl-studio.exe`.

## NHL07 diagnosis

The screenshot selected `X:\Games\Emulation\Roms\XBOX360\ISOs\Archives\NHL 07 (USA)`, which contains an ISO, not extracted rendering files. The verified extracted root is:

`F:\NHL Installations\Xbox 360\NHL07\NHL 07 (USA)`

The scanner supports `rendering/player/player_<id>/head_<id>.rx2`. Scanning that copy with its own NHL07 anchor found 2,250 named rows, 267 bespoke player meshes and 957 portraits. Select this root using **Game source folders → Extracted assets source folder**, now available directly on the Assets gate.

## Behavior and limits

- The full catalog is available independently of roster-only bespoke head counts.
- Sweep shared/CAP and staff folders, including `player_common` and PS4 `common`; preserve same-ID variants and LODs with distinct filenames and texture namespaces.
- Staff body containers yield only explicitly identified head/hair components. Anonymous body geometry is not classified as a head.
- Exclude equipment/clothing maps and meshes; classify standalone facial hair separately.
- Export shared eye maps and CAP overlays without meshes into `shared-textures/` with a source/game manifest.
- Catalog, roster and file-browser map exports share DDS encoding. GNF portraits remain PNG. Native BC blocks are preserved, including Xbox BC5 normal maps.
- DDS contains the full-resolution **base mip**, not the complete mip chain.
- GLBs contain geometry and available UVs, not skeletons/skin weights or inferred material bindings.
- Repeated biography rows for the same source are deduplicated. Different sources and colliding map labels receive distinct output names. Decode/worker errors are reported.
- Catalog export runs off the UI thread. Existing catalog paths and source provenance remain readable.

## Frostbite implementation

The user explicitly authorized `E:\Repositories\UEViewer\oo2core_8_win64.dll`: “Yes go ahehad and use it.” Only that DLL was loaded; no external tool source was read or copied. It is not bundled with Studio. The browser requires selection of a separately installed decoder.

Measured MeshSet handling covers both observed header sizes, all LODs, float3/half4 positions, half2/float2 UVs, external chunks and inline geometry. UV-less depth sections remain without invented UVs. DDS supports the observed native RGBA8/BC1/BC3/BC4/BC5/BC7 formats after PS4 microtile conversion. Bounds, finite coordinates and index references are checked.

Export is explicit about game and Data/Patch layer. Resources retain names, SHA-1, bundle and archive/span provenance, plus resolved chunk sources. Patch requires its matching Data head and may resolve an exact referenced chunk from that Data layer. Missing Patch TOCs generate warnings, not guessed resource overlays. Output must be a new folder outside the game.

## Qualification

Durable per-title results are in `docs/audits/head-extraction-20261019/`.

RenderWare/Ignite: **17 targets, 24,551 GLBs and 40,287 DDS**, zero failures. Xbox fixtures were staged from actual ISO appearance members and inflated with the normal onboarding codecs. PS4 NHL15–21 were read from extracted Kettle trees. All generated files passed independent GLB header/chunk/buffer bounds and DDS format/dimension/payload checks.

| Frostbite title/layer | LOD meshes | DDS textures | Evidence |
|---|---:|---:|---|
| NHL22 Data | 3,081 | 3,959 | Full-corpus serialization validation |
| NHL23 Data | 3,279 | 4,318 | Files written and independently structurally checked |
| NHL24 Data | 3,436 | 4,575 | Full-corpus serialization validation |
| NHL24 Patch | 3,475 | 3,625 | Full-corpus serialization validation |

All four runs reported zero failures. NHL24 Patch reports an absent `Patch/Gen4b/ui.toc` warning; no base UI replacement was inferred. Validation-only runs execute the same serializers without persisting GLB/DDS files; summary fields still use `_written`, so consult the evidence column.

NHL23 actual output files and manifest references passed independent checks. Representative textures covering all eight encountered DXGI variants were decoded and visually reviewed. This does not establish visual correctness for every asset. Initial 44 MeshSet samples produced 48 LOD exports; 192 additional variant samples produced 192 LOD exports and 290 sections.

Focused regression checks cover variant preservation, map collisions, equipment exclusion, standalone facial hair, DDS block preservation, portrait/map dispatch, both inline MeshSet headers with half vertices and invalid indices, the roster-empty catalog UI, and Frostbite export command arguments/results. Main checkout verification logs are in `local/headmesh-integration-20261020`. Generated DTOs were not copied from the older checkout; the drift check uses current main DTOs.

## Reproduction

```powershell
cargo test -p studio-domain --lib head_ --offline
cargo test -p studio-domain --lib appearance_texture --offline
cargo test -p studio-domain --lib file_export_keeps_gnf --offline
cargo test -p tdb-frostbite --offline
npm --prefix app run test -- --run src/pages/AssetExport.test.tsx src/lib/assetRoot.test.ts src/pages/FrostbiteBrowser.test.tsx
npm --prefix app run build
cargo run -p studio-api --bin export-bindings --offline -- --check
cargo run -p tdb-frostbite --bin frostbite-appearance --offline -- ROOT Data nhl24 NEW_OUTPUT OODLE_DLL --validate-only
```

Omit `--validate-only` for actual Frostbite output. Use Patch only with matching Data. The existing `appearance_audit` example and `scripts/check_appearance_exports.py` reproduce the RenderWare/Ignite checks; `scripts/stage_appearance_fixtures.py` stages a new owned Xbox fixture directory.

## Artifacts and disposition

- Isolated `local/appearance-audit-20261019`: owned fixtures, catalogs, metadata, logs and representative image checks retained for review/reproduction. Earlier failed trial folders are diagnostic history, not final qualification evidence.
- Main `local/appearance-audit-20261019`: original package inventory/probe material, retained as evidence.
- Main `local/headmesh-integration-20261020`: integration backups, plan and verification logs; retained for recovery and review.
- Isolated dependencies/build outputs remain available for reproduction; no shared cache was deleted.
- No extracted game assets or decoder DLL are included in tracked changes. Private exports are not installed into the user's headmesh database.

Main integration checks: head-focused Rust suite 42 passed / 1 ignored; DDS 2 passed; file dispatch 1 passed; Frostbite 4 unit + 9 integration passed; frontend 6 passed in 3 suites; TypeScript/Vite build passed; all 364 current DTO bindings match. Some mount-gated legacy tests can return early when their old mount path is absent; the separately recorded real-corpus runs establish target coverage. Initial sandbox temporary-file permission failures were rerun successfully outside the sandbox.

Desktop integration build passed: cargo build -p nhl-studio --features tauri/custom-protocol --offline. Launched target/debug/nhl-studio.exe (PID 25992); launch logs are retained in the main integration audit folder. Interactive export acceptance remains for the user.
