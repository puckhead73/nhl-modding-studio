# NHL19 head import playbook: Mogilny and Demitra

**Reference brief — 2026-09-11.** This records the actual successful methods,
their inputs, implementation, corrections and deployment dependencies. Start here
for the next head; do not repeat the early projection/data-transfer experiments.

## 1. Results and method selection

| Case | Source | NHL19 donor | Successful correspondence | Current material result |
|---|---|---|---|---|
| Alexander Mogilny | Supplied GLB, anatomically compatible NHL UV template | Jack Eichel | Direct UV matching, checked against source connectivity | Original import and later crown-corrected mesh user-confirmed; separate 2K texture installed, but its individual runtime acceptance was not explicitly recorded |
| Pavol Demitra | CC5 FBX, different UV atlas | Marian Gaborik, Slovakia | Calibrated UV warp plus barycentric source-surface sampling; repaired neck/rear scalp; protected ears | Revision 2 mesh and subsequent 2K diffuse both explicitly user-confirmed successful |

Both workflows preserve the **NHL19 destination topology, vertex records, UVs,
component bindings and opaque rig data**. They move destination vertices into the
source shape. They do not install a CC5 rig or replace the native index buffers.
Vertex counts need not match. Equal counts alone would not prove correspondence.

The user's NHL10–NHL24 shared-head-UV observation makes template transfer a useful
starting point. It does not prove universal container, shader, rig or UV compatibility.
Qualify each new title/platform/template separately. The successful native writer
here is for measured NHL19 PS4 containers, not every game in that range.

## 2. Environment and preservation

- Repository: `E:\Repositories\nhl-modding-studio`.
- Blender: `C:\Program Files\Blender Foundation\Blender 4.5\blender.exe`
  (recorded runtime 4.5.5 LTS).
- NumPy-capable Python:
  `C:\Program Files\Blender Foundation\Blender 4.5\4.5\python\bin\python.exe`.
- Game: `H:\Emulators\ShadPS4\Games\CUSA11126`, NHL19 PS4 APP_VER 01.60.
- Emulator used: `C:\shadPS4\shadPS4.exe`.
- Private sources: `S:\Downloads\Mogilny Test` and its `Demitra` subdirectory.
- Scratch/results: `H:\TEMP` and the earlier `E:\tmp\nhl19-mogilny`.

Keep source files unchanged. Save hashes, original archive entries, native record
order, geometry/UV ledgers and installation receipts. The repository already had
unrelated changes; do not clean/reset it to run these workflows.

**Installed directories are immutable historical packages.** Do not rerun a
hardcoded fixture script into them. Copy/adapt scripts into a new scratch root for
the next head, or use the saved native JSON to reproduce packing in a fresh output.
The scripts are working fixture recipes, not a single parameterized conversion CLI.

## 3. Extract the real destination assets first

1. Identify the player and appearance routing in the current roster/database.
2. Enumerate all matching archive entries with `eb_find`, including patches.
3. Extract each occurrence separately with production `eb_extract`; record archive,
   entry index, path, stored offset/length, decoded hash and revision.
4. Inspect original MCD structure with `mcd_donor_inspect`; reconcile stream ownership,
   formats, vertex counts and indices with the display decoder.
5. Export original textures with `export_gnf_maps`, retaining original GNF and
   full-mip DDS alongside PNG/channel previews.

The initial third-party `Extracted/jack_eichel.mcd` was three concatenated copies,
not one valid vanilla asset. The real Eichel container is **1,043,168 bytes with
one VIEW and seven submeshes**. Never infer multiple LODs or faces from that dump.
Base/patch93 and patch91 have different facial-name ordering; pack each revision
from its own original bytes. The facial-name table helps identify anatomy but is
not a substitute for verified per-vertex masks.

| Donor | Base MCD entry | Patch91 | Patch93 | Head diffuse in base |
|---|---:|---:|---:|---:|
| Eichel | 62195 | 2608 | 3036 | 33208 |
| Gaborik | 15972 | 601 | 690 | 69297 |

Archive names are `game_90-008.big`, `patch_game_91.big`, `patch_game_93.big`.
Entry numbers are evidence for this installed build, not universal constants.
Gaborik's container is **978,768 bytes, six components**: head 3285v/6014t with
embedded lashes, eyes 66v/100t each, cap 580v/1040t, cardsA 98v/92t, cardsB 434v/408t.
Eichel's head has 3157v/6014t and a separate eyelash component. The two donors
demonstrate why raw vertex count cannot be assumed even within one game.

Gaborik was selected through Slovakia team ID 151, player ID 792 and appearance
value 289. His roster name stayed Marian Gaborik; Demitra replaces his appearance
wherever that asset is used, including Slovakia.

### Coordinate contract

Native NHL XYZ is Y-up; Blender is Z-up. The recipes use:

```text
native (x,y,z) -> Blender (x,-z,y)
Blender (x,y,z) -> native (x,z,-y)
```

Apply source object/node transforms once. Record units and UV orientation explicitly.
The Blender recipes convert stored/native V to Blender V once and reverse it once
when emitting native JSON. Do not apply additional guessed axis or green-channel flips.
GLB is for source input/review; the **native-order JSON**, not an exported/reimported
GLB index order, drives native packing.

## 4. Mogilny: successful shared-UV workflow

### Inputs and dependencies

- Source GLB hash:
  `e983e37218e99aab383fca349229b33899fc0cdc8923385707f6ee712d2d2b29`.
- Exported Eichel full GLB hash:
  `4db21491c8f738126a1975632c71f837fe6a9a3844ea2960320972ae881d1ee1`.
- Prepared comparison scene:
  `H:\TEMP\nhl19-addon-comparison\builtin\Eichel_Mogilny_DataTransfer_Test.blend`.
  This supplies the unmodified control/reference collections and common frame.
- UV fit:
  `H:\TEMP\nhl19-mogilny-uv-template-final\Mogilny_UV_Template_Test.blend`,
  `report.json` and comparison PNGs.
- Native donors:
  `E:\tmp\nhl19-mogilny\out\eichel_from_game90.mcd` and `eichel_from_p91.mcd`.

The comparison scene's filename reflects an earlier experiment. The successful
fitter reads `01_EICHEL_CONTROL` and `02_MOGILNY_REFERENCE`, not the rejected
Data Transfer result. If rebuilding that scene is necessary,
`nhl19_data_transfer_test.py` imports the full Eichel/source GLBs, applies node
transforms, and creates those collections. Its transfer experiments are not the
successful correspondence method and the external MeshDataTransfer add-on is not
required by the successful UV workflow.

### Procedure

1. Read donor `geo_headShape` and source `head_3_0_head_mesh0` from the prepared scene.
2. Build a source position-plus-UV representation from face corners. The GLB has
   17,400 triangle-soup records, 2,924 distinct positions and 3,041 position/UV
   combinations. This source-only representation must not weld native records.
3. Match each native UV to the nearest source UV within **0.0005**. Treat candidates
   within **1e-7** of the nearest distance as ties; reject positional ambiguity
   exceeding **0.1 mm**.
4. Check source triangle/edge connectivity. Require at least two mapped incident
   source edges for direct positional support. Different quad diagonals are allowed
   when shared-edge evidence supports the correspondence.
5. Copy supported source coordinates at full strength. The recorded result is 3018
   unambiguous UV matches, 21 rejected ambiguous vertices, 2983 complete mapped
   source triangles and **3013 directly supported native vertices**.
6. Resolve the remaining **144** vertices by donor-graph displacement interpolation.
   Attach 21 unsupported seam vertices to verified coincident exterior counterparts;
   preserve native IDs and the separate lip/interior connections.
7. Do not freeze the historical `shell_boundary` group wholesale: it included scalp
   seams and caused a crown notch. Use actual lower-attachment anatomy.
8. Finalize four residual coincident groups (maximum correction 0.420 mm), fit the
   native eyeballs to source eye bounds, and move lash roots with the eyelid surface
   through barycentric donor-to-fitted displacement.
9. Align the actual head crown to Eichel and retain his lower neck/chest. The first
   import used a mouth-anchored frame; in-game helmet/collar review superseded that
   placement. Final settings: donor crown **1.9450141191 m**, upper translation
   **−0.0178362131 m**, 280 vertices at native/Blender-up height <=1.68 m fixed,
   smoothstep blend over **1.68–1.72 m**. Preserve source facial proportions.
10. Conform Eichel's cap and both card layers radially to the final scalp; fold
    hanging cards toward the dome and shrink to **92%** radius. Minimum measured
    clearance exceeds **6.27 mm**. Hide by geometry, not viewport flags or deletion.
11. Recompute normals/tangents and bounds; emit all seven components in native order.

### Implementations and results

- `scripts/blender/uv_template_match.py`: reusable correspondence helper; JSON
  donor/source positions, UVs and triangles -> mapping/evidence. It reproduced the
  recorded 3013 supported vertices/2983 shared triangles on the real fixture.
- `scripts/blender/nhl19_uv_correspondence_fit.py`: full recorded fit.
- `scripts/blender/nhl19_finalize_game_mesh.py`: finishing; crown alignment is the
  default. `--keep-mouth-height` reproduces only the historical first placement.
- `scripts/blender/head_scalp_placement.py`: reusable crown/lower-neck placement.

Initial successful package: `H:\TEMP\nhl19-mogilny-game-test`.
User-confirmed corrected mesh: `H:\TEMP\nhl19-mogilny-scalp-mesh-only`.
Later installed diffuse: `H:\TEMP\nhl19-mogilny-texture-only-2k`.
The combined `nhl19-mogilny-scalp-2k` launch failed and was restored; it is not a
known-good deployment package. The later isolated mesh succeeded. Do not infer
that the earlier combined crash proved a particular GNF field was wrong.

Mogilny initially used a supplied 1K diffuse. The later texture input was
`S:\Downloads\Mogilny Test\head_alex_mogilny_cm.png.png` (2K). Eichel's original
512-square BC5_SNORM normal remained unchanged; no source normal was imported.
Original maps and channel previews are retained under the scalp/2K package's
`Eichel_Textures` directory. There was no separately named Eichel head specular map.

## 5. Demitra: successful CC5 workflow

### Inputs

`S:\Downloads\Mogilny Test\Demitra\Demitra.Fbx`, SHA256
`a1b281236cb4dfef9dbac12568c88cf43e092c9087a6cc045b02bae04ff6a637`.
Material metadata is `Demitra.json`; images live under `Demitra.fbm` and
`textures/Demitra/Demitra`. The populated head is object `0`, material
`Std_Skin_Head`: 4119 original vertices/8196 triangles. Separate source eyes,
teeth and tongue exist; empty body material slots do not make this a full-body export.

Scratch root: `H:\TEMP\nhl19-demitra` with `source`, `vanilla`, `fit`, `package`,
`revision2` and `diffuse2k` stages. The initial `package` is a dependency of the
repair script; **revision2 is the successful geometry**, with `diffuse2k` above it.

### Procedure and exact script order

1. **Inspect/import** — `nhl19_demitra_inspect.py -- <source-output-directory>`.
   Blender's FBX importer hit `armature_setup[None]` on this source. The wrapper
   removes eight Skin/Cluster objects and their connections from the parsed FBX
   **in memory**, preserving transforms, geometry and material data. It leaves the
   source file untouched. Apply its 0.01 object scale once. This is a static rest
   mesh import, not CC5 animation/weight retargeting.
2. **Measure donor** — run `mcd_donor_inspect` on both original Gaborik revisions.
   Preserve `vanilla/gaborik_native.json` and `gaborik_patch91_native.json`.
3. **Extract source UV records** — `nhl19_demitra_uv_probe.py`. Preserve separate
   records for `(source vertex ID, exact corner UV)`; the head becomes 4282 sampling
   records. A previous per-vertex last-UV assignment tore the side of the face.
4. **Register the two atlases** — `nhl19_demitra_fit.py`. Its measured landmark pairs
   cover eyes, nose, lip curves, ears, forehead and bounds. Fit a thin-plate-spline
   warp from NHL atlas coordinates to CC5 atlas coordinates. The ledger stores
   landmark pairs, coefficients, source shift and anatomy IDs in `fit/uv_warp.json`.
   This is calibrated for this pair, not an interchangeable CC5 preset.
5. **Sample geometry** — warp native UVs into the CC5 UV triangles and barycentrically
   interpolate source positions. Record nearest-chart fallbacks rather than treating
   them as proven anatomical matches. Source Blender shift is approximately
   `(0, −0.0608217567, +0.1853127480)` metres, based on eye depth and crown.
6. **Handle special anatomy** — retain 534 native oral records with mouth-following
   displacement. Distinct inner/outer lip records with equal UVs retain their native
   offsets. Keep native eye topology but fit source eye bounds. Embedded native lash
   strips follow nearby skin; do not flatten their near-zero-area UV strip.
7. **Initial finalization/bake** — `nhl19_demitra_finalize.py`. Bake the 4K diffuse
   through the same UV warp to native 1K, retaining native oral atlas islands. Produce
   `package/native_mesh.json`, the review scene and quality report. This is an
   intermediate stage: its original neck/back defects require the next repair.
8. **Measure initial fit** — `nhl19_demitra_qc.py`. The recorded facial-surface
   distances were median 0.159 mm/p95 0.699 mm; 10,000 facial UV Jacobian samples
   were positive. These metrics did not catch the neck/back problems and are not
   a whole-head or animation certificate.
9. **Mandatory successful repair** — `nhl19_demitra_repair.py`. Read the initial
   package, write `revision2`, and apply all of the following:
   - Preserve **484** Gaborik lower-neck/chest vertices exactly through 1.735 m;
     blend to the fitted jaw by 1.775 m and nape by 1.820 m. The earlier
     chart-border neck projection moved some vertices over 3 cm and is rejected.
   - Use spatial radial correspondence to the CC5 surface on the posterior scalp,
     where the two atlas boundaries are not homologous. Exclude oral/ear anatomy.
   - Lock **259 records per ear**, including two triangle-neighbour rings, to the
     pre-repair Demitra fit: zero positional change on both sides. Retain core ear
     normals, with surrounding shading blended into the repaired scalp. Radial
     fitting had distorted an ear before this explicit protection was added.
   - Keep native duplicate seam positions coincident and smooth their normals.
     An apparent seam can be shape/shading even when there is no open positional gap.
   - Match crown exactly to **1.9478691816 m**. Refit all three hair layers inside
     the revised head at **90%** radius, minimum clearance greater than **7.7 mm**.
   - Check zero newly collapsed triangles in all six components. Save both side
     profiles and rear solid views in addition to front views.
10. **Pack and install revision2**, then validate in game. The user confirmed success.
11. **Upgrade diffuse without refitting** — `nhl19_demitra_diffuse_2k.py` rebakes the
    original 4K CC5 diffuse through the unchanged ledger at **2048²**. It does not
    upscale the 1K atlas. The builder's `texture_only: true` recipe changes one
    Gaborik diffuse entry in the base archive; no MCD or patch entry is replaced.
    All 12 mips passed; live MCD hashes remained identical. The user confirmed 2K success.

### Material boundary

The **only imported CC5 texture is head diffuse**. Native oral atlas islands and
NHL eye/material bindings remain. CC5 normal, roughness, metallic, AO/ORM,
specular, micro-normal, SSS, transmission and wrinkle maps were not imported.
Generated vertex normals/tangents are mesh attributes, not imported normal textures.

The subsequent normal investigation found no player-specific Gaborik normal in
the installed archive tables. Shared normals exist, but their binding to Gaborik
was not established. The user explicitly stopped that test. **No shared or
player-specific normal was replaced.** Do not resume it as an unfinished requirement.

## 6. Native packing and texture encoding

The validated native path is `mcd_game_test_pack` followed by
`nhl19_build_test_package.py`, not Blender's GLB export alone.

1. Native JSON carries `donor_sha256` and each component's name, original positions,
   new positions, normals, native triangles and UVs.
2. The Rust packer accepts registered donor fingerprints. It compares original
   positions, triangles and UVs against that revision before writing new positions
   and bounds. Unknown donor profiles need structural qualification first.
3. Pack base and patch91 separately. Base geometry also serves patch93, but patch91
   must preserve its own opaque bytes and facial-name ordering.
4. The Python builder writes measured f16 normal/tangent XYZ fields. Preserve native
   fourth components and all unrelated bytes; do not reinterpret native W as a
   generic glTF tangent convention.
5. Diffuse uses BC1. Build mipmaps with linear-light averaging, encode blocks and
   micro-tile them into GNF. Native 1K uses 11 levels/701696 bytes; 2K uses 12 levels/
   2798848 bytes. For 2K, update only measured size-dependent header fields: width,
   height, pitch, mip count and total/data size. Keep format/numeric/tile mode.
6. Chunkzip uses 131072-byte chunks for these replacement assets. Preserve the
   original header/per-chunk conventions and verify production decoding. The PS4
   alignment fix uses direct alignment to the next header offset congruent to 8
   modulo 16; the older stepping loop could never land correctly for arbitrary
   compressed lengths. Do not generalize observed flags to untested assets.

### Runnable offline repack of successful Demitra geometry

This recreates positional MCDs in a fresh folder without modifying installed assets.
Run from the repository. It does not rebuild the fitted surface or install anything.

```powershell
$ErrorActionPreference = 'Stop'
$repack = Join-Path $env:TEMP ('demitra-repack-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $repack | Out-Null
cargo build -p tdb-rx2 --example mcd_game_test_pack
if ($LASTEXITCODE -ne 0) { throw 'Packer build failed' }
$native = 'H:\TEMP\nhl19-demitra\revision2\native_mesh.json'
$donors = @{
  base = 'H:\TEMP\nhl19-demitra\vanilla\game_90-008#15972_marian_gaborik.mcd'
  patch91 = 'H:\TEMP\nhl19-demitra\vanilla\patch_game_91#601_marian_gaborik.mcd'
}
foreach ($revision in 'base','patch91') {
  & '.\target\debug\examples\mcd_game_test_pack.exe' $donors[$revision] $native (Join-Path $repack $revision)
  if ($LASTEXITCODE -ne 0) { throw "Packing failed: $revision" }
}
Write-Output "Positional MCDs and layout receipts: $repack"
```

To build a new installation package, also supply its `native_mesh.json`,
`source_rgba.npy`, positional MCDs/layouts and a reviewed recipe. The recipe selects
`slug`, original `donors`, `vanilla`, current full `baseline_sha256`, optional
`previous_package`, `normal_map` status and optional `texture_only`.
Use `nhl19_build_test_package.py <new-directory> --recipe <recipe.json>`.
Do not simply reuse historical archive fingerprints against a later installed state.
For another donor layered over existing modifications, use current fingerprints
but do not set `previous_package` to an unrelated player's package: decoded-entry
checks require the previous package to contain the entries being revised.

## 7. Validation, installation and rollback

Before installation:

1. Assert native record counts, original IDs, UVs, triangles and opaque-byte limits.
2. Check source likeness, lip-layer separation, eyes/lashes, both ears, both profiles,
   rear seam shading, neck/shoulder attachment and crown. Render all hair components
   enabled to demonstrate actual geometric hiding.
3. Run `verify_nhl19_test_package` for production MCD/chunkzip/GNF comparisons.
4. Run `eb_verify_append` for every proposed entry and confirm unrelated table rows
   are unchanged. Validate full live archive hashes against the accepted baseline.
5. Close shadPS4. Run the package's `Manage-Test.ps1 apply` or
   `nhl19_test_install.py <package> apply`; obtain filesystem approval where required.
6. **Wait for `COMPLETE apply`.** Full pre/post hashing of the ~13.5 GB base archive
   takes minutes. Do not launch NHL19 while installation is still running.
7. Read installed entries back with production `eb_extract`, compare hashes to the
   package, and record `live_readback.json`. For texture-only revisions, confirm
   the successful mesh payloads remain identical.
8. Record the user's runtime result against the exact package. Inspect the real
   player model, not a UI portrait: lighting, detail/mips, blinking, mouth movement,
   head turns and helmet/collar clearance. Do not claim unreported individual checks.

Archive installation appends aligned payloads, retargets only selected file-table
rows, and updates archive lengths. It preserves original payload bytes. Restoration
uses recorded original rows/header/length and verifies the restored full hash.
It does not depend on an unproven loose-file override.

The two unidentified archive row words were preserved, not reverse-engineered as
content checksums. Their stability supported the bounded replacement experiment;
do not invent recomputation rules.

### Current rollback dependency chain, newest first

1. `H:\TEMP\nhl19-demitra\diffuse2k` -> Demitra revision2 with 1K diffuse.
2. `H:\TEMP\nhl19-demitra\revision2` -> initial Demitra package.
3. `H:\TEMP\nhl19-demitra\package` -> pre-Demitra archives with Mogilny installed.
4. `H:\TEMP\nhl19-mogilny-texture-only-2k` -> corrected Mogilny mesh with 1K diffuse.
5. `H:\TEMP\nhl19-mogilny-scalp-mesh-only` -> original successful Mogilny package.
6. `H:\TEMP\nhl19-mogilny-game-test` -> its recorded original archive baseline.

Read actual receipts before any restore. Older receipts still say `installed` when
newer packages are layered above them; they are not permission to restore out of
order. A full-hash mismatch is a stop condition, not a reason to bypass validation.

## 8. What to reuse, and what to measure again

Reuse the algorithms, native writer/codec, validation tools and successful ledgers
for identical signatures. For a new head, make a new scratch root, record input
hashes, identify the donor, compare UV charts and choose direct matching or calibrated
sampling. Re-measure neck bands, ears, lips, oral islands, lashes, crown and material
availability. Do not reuse fixture-specific heights or vertex masks by game name alone.

Rejected routes were global nearest-surface/shrinkwrap fitting and independent
mouth/eye-aligned passes that compromised likeness or anatomy. Removing eyeballs
and mouthbag from a projection did not by itself solve correspondence. They are
useful diagnostic experiments, not prerequisites to repeat for every head.

Minimum retained handoff set: source files/hashes; vanilla donors; prepared source
scene; native-order geometry JSON; UV mapping/coefficients; anatomy IDs and thresholds;
final review scene/GLB and solid views; original/converted textures; pack/layout
reports; package, installation and live-readback manifests; runtime acceptance;
and the exact scripts used. Losing a prepared scene is a dependency loss even if
the final GLB survives.

## 9. Detailed references

The next CC5 fixture, [Turgeon over Bouwmeester](validation/2026-09-11-n19-turgeon-cc5.md),
reused the saved calibration after exact source and destination UV/index parity
checks. It also demonstrated that identical topology does **not** qualify another
donor's neck-height thresholds: Gaborik's cutoff froze Bouwmeester's lower jaw.
Anatomical placement must still be measured per donor. Blais was rejected as the
target because his generic head is shared; the user approved a unique Blues head.

Frozen reference sources are saved at
`H:\TEMP\nhl19-head-import-reference-2026-09-11`, preserving relative paths.
The [hash manifest](validation/2026-09-11-head-import-reference-manifest.json)
records 26 files: fitting/baking/packing scripts, measured writer code, relevant
archive helpers, Cargo.lock and this brief. This is a source reference snapshot,
not a standalone Rust workspace or a backup of the private meshes. Keep the
private fixture scenes and installation packages listed above alongside it.

- [Shared-template method](MESH_UV_TEMPLATE_TRANSFER.md)
- [Mogilny UV-fit measurements](validation/2026-09-10-n19-mogilny-uv-template-fit.md)
- [Mogilny original installation](validation/2026-09-10-n19-mogilny-game-test-package.md)
- [Mogilny crown/texture history](validation/2026-09-10-n19-mogilny-scalp-2k.md)
- [CC5 Demitra detailed workflow](CC5_TO_NHL19_HEAD_TRANSFER.md)
- [Demitra revision2 success](validation/2026-09-11-n19-demitra-cc5-success.md)
- [Demitra 2K diffuse](validation/2026-09-11-n19-demitra-diffuse-2k.md)

Historical receipts can contain pre-test status paragraphs. The dated user outcome,
exact successful revision and later installation receipts take precedence over
those historical descriptions. This brief records Demitra's 2K success and the
explicit cancellation of the normal-map test.
