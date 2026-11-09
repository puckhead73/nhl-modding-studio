# Head catalog extraction — 19–20 October 2026

## Plain-language summary

NHL07's reported extraction gate was caused by the selected source pointing to an ISO storage folder, not its extracted game. The Assets gate now provides the existing source-folder controls directly, and Export puts the full appearance catalog ahead of the roster-only export. The catalog preserves mesh variants, includes shared/CAP and staff assets, and exports DDS textures.

NHL22–24 are a separate, unfinished Frostbite conversion task. Their head resources have been inventoried, but the existing Studio implementation cannot convert their compressed MeshSet resources into GLB. Do not describe those titles as supported by this change.

## Scope and ownership

- Base revision: `ad12f4b6`.
- Implementation checkout: `C:\Users\puckh\.codex\worktrees\headmesh-extraction\nhl-modding-studio`.
- The user confirmed Claude was preparing commits in the original checkout. Tracked edits were removed there during that work; implementation continued in this isolated worktree.
- No staging, commits, installation updates or Studio restarts were performed for this package.
- Original games and ISO files are read-only inputs. The screenshot shows unsaved Studio state; the running instance was left alone.

## NHL07 source diagnosis

The supplied screenshot expected:

`X:\Games\Emulation\Roms\XBOX360\ISOs\Archives\NHL 07 (USA)\rendering`

That folder contains `NHL 07 (USA).iso`, not an extracted rendering tree. The working extracted source is:

`F:\NHL Installations\Xbox 360\NHL07\NHL 07 (USA)`

The existing scanner resolves NHL07's nested `rendering/player/player_<id>/head_<id>.rx2` layout. A direct scan of the extracted source and NHL07 anchor found 2,250 named player rows, 267 bespoke player meshes and 957 portraits. No alternate-title database was used to label assets.

Use **Game source folders → Extracted assets source folder** to select the extracted game root. The same controls are now available on the Assets gate, without returning to Project Overview.

## Changes

- The full appearance catalog remains independent of the number of roster rows with exportable bespoke heads, and is now the first Export card.
- Sweep all existing appearance folders, including the previously omitted `player_common` and PS4 `common` folders.
- Preserve same-ID head and hairstyle variants under distinct source-derived names. Distinct sources also reserve distinct texture namespaces.
- Inspect staff body containers for explicitly named head/eye/hair components. Body-only and anonymously named body geometry is not invented into a head.
- Exclude PS4 goalie equipment, jaw protectors, padding, earguards, shirts, outdoor clothing and hand components that share the player folder.
- Export standalone beard/facial-hair geometry as `facialhair`, rather than counting it as a second head.
- Recognize additional shared texture suffixes, including `snm`, `ssm`, `ism`, `ctm` and `cpm`.
- Catalog, roster and file-browser texture exports share a DDS encoder. BC1/2/3 blocks retain legacy DDS headers; BC4/5/7 retain their blocks in DX10 DDS. Other supported raster formats use uncompressed RGBA DDS.
- DDS output currently contains the **base mip**, matching the existing source-reader interface. This is not a full-mip-chain exporter.
- GLBs contain geometry and UVs. They are not rig/skeleton conversion exports.
- Shared eye maps and CAP overlays without standalone meshes are exported under `shared-textures/`, with `supplemental-textures.json` recording their source files and game. No speculative material binding is added.
- Duplicate map labels get distinct filenames. Texture decode errors and catalog-worker failures are reported.
- Repeated database rows for the same player and source produce one export; distinct sources for that identity receive separate output names. Numeric head-map lookups exclude jersey-name and equipment maps.
- GNF portraits still export as PNG; texture maps export as DDS.
- The catalog command runs off the Tauri UI thread.

The existing `catalog.json` format remains readable. Mesh entries retain source-container and texture-source provenance, plus relative exported mesh and texture paths. Existing catalogs with PNG paths remain valid.

## Qualification

The cross-title runner stages appearance members from the actual Xbox ISOs. It uses the same EB `chunkzip`/`chunklzx` decompression as onboarding before asset decoding. NHL08's whole-file RefPack remains handled by the existing asset reader. PS4 NHL15–21 are read from their extracted Kettle trees.

Every generated GLB is independently checked for magic/version/length, JSON and binary chunks, and buffer-view bounds. Every DDS is checked for magic/header size, dimensions, FourCC/DXGI format and exact base-mip payload length. Physical output counts must match the export summary.

See the adjacent audit directory for the final per-title results and inventories:

`docs/audits/head-extraction-20261019/`

Focused tests cover variant preservation, map-label collisions, DDS block preservation for all six GNF BC formats, PNG portrait versus DDS texture dispatch, source-folder gear classification, standalone beard classification and the roster-empty catalog UI. These checks establish extraction/file structure, not visual acceptance of every model or in-game animation compatibility.

The NHL18 qualification exposed duplicate biography rows and unrelated `name_nineties_champions_*` jersey maps matching generic head IDs. Both were corrected before the final run. Earlier PS4 trial catalogs are not final evidence; the corrected PS4 run writes to `local/appearance-audit-20261019/qualified-ps4`.

## Frostbite NHL22–24

The inventory preserves resource name, SHA-1, resource metadata, archive, layer, offset, stored size, bundle and TOC. Counts are distinct **resource names**, including hairstyle/context components; they are not counts of unique players or deduplicated head shapes. Base and Patch occurrences are not silently flattened into an assumed effective installation.

Each title has licensed and generic skater heads, bench/coach heads, referee heads, scalp hair and beard resources. NHL24 additionally has a `special` head family. Sampled MeshSet payloads use CAS codec `0x11`.

Remaining prerequisites:

1. An explicitly authorized, separately installed Oodle decoder provider.
2. Measured MeshSet vertex declarations, LOD/submesh layouts and resource/chunk joins.
3. Measured texture resource/chunk layout and PS4 texture conversion.
4. GLB/DDS regression fixtures and whole-corpus verification before exposing a supported export action.

The user explicitly authorized using `E:\Repositories\UEViewer\oo2core_8_win64.dll` on 20 October: “Yes go ahehad and use it.” It was loaded only after that authorization. No external tool source was read or copied, and the DLL is not bundled with Studio. Initial decoding succeeded on 44 MeshSet samples across the three titles; stream/chunk qualification is in progress.

## Artifacts and cleanup

- Owned review/test root: `<implementation checkout>\local\appearance-audit-20261019`.
- `xbox-fixtures`: appearance-only copies of shipped archive members, inflated in place only after an explicit containment check. Retained for reproducible review.
- Per-title catalogs, summaries and logs under that root: test outputs, retained for independent review; not installed into the user's headmesh database.
- Obsolete PS4 test catalogs were removed after stopping the owned audit processes; originals were untouched.
- Original inventory/probe root: `E:\Repositories\nhl-modding-studio\local\appearance-audit-20261019`. Read-only archive research results remain there.
- Initial helper files created before isolation remain untracked in the original checkout. Do not remove or integrate them while Claude is using that checkout without coordinating ownership.
- The isolated frontend dependencies and Cargo target are retained for building/reviewing this package. No shared build cache was deleted.

## Reproduction

From the implementation checkout:

```powershell
cargo test -p studio-domain --lib head_ --offline
cargo test -p studio-domain --lib appearance_texture --offline
cargo test -p studio-domain --lib file_export_keeps_gnf --offline
npm --prefix app run test -- --run src/pages/AssetExport.test.tsx src/lib/assetRoot.test.ts
npm --prefix app run build
cargo check -p studio-api --offline
cargo run -p studio-api --bin export-bindings --offline -- --check
cargo build -p studio-domain --example appearance_audit --offline
```

Archive inventory uses this repository's existing research readers. In the isolated checkout, pass `--research-repository E:\Repositories\nhl-modding-studio` to `scripts/appearance_inventory.py`. Stage a **new** fixture directory with `scripts/stage_appearance_fixtures.py`, then run `scripts/check_appearance_exports.py <audit-directory>`; optional second argument `xbox` or `ps4` narrows the run.

Integration into Claude's checkout remains a separate coordinated step.
