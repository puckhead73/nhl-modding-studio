# CC5 head conversion to NHL19: Demitra over Gaborik

**Successful in-game fixture, 2026-09-11:** the user reported "This was a successful
import" after installing **revision 2**. Use that revision as the next-test baseline,
not the initial package with the neck/rear seam defects. See the
[success receipt](validation/2026-09-11-n19-demitra-cc5-success.md).
The subsequent **2K diffuse also succeeded in game**. The normal-map experiment
was explicitly cancelled because no player-specific Gaborik normal was found.
For the consolidated handoff, start with the [import playbook](NHL19_HEAD_IMPORT_PLAYBOOK.md).

## Selected approach

Keep the destination-native head topology, UVs, component bindings and opaque rig
data. Register NHL UV landmarks to the CC5 atlas, sample CC5 surface positions
barycentrically in that registered UV space, and rebake the diffuse through the
same mapping. This extends the shared-template Mogilny method to different UV
layouts; it does not assume equal counts, vertex order or identical UV coordinates.

Direct CC5 rig/geometry replacement is not used: the existing Path C code targets
RW/Legacy containers and does not establish a safe NHL19 PS4 topology/rig rewrite.
Global nearest-surface projection is unsuitable around lips, eyelids and oral
geometry. The successful revision uses spatial correspondence on the posterior
scalp and native lash attachment, protects both ears, and retains/blends the native
neck instead of projecting its chart border. Source normals cannot be transplanted by warping RGB:
their tangent frame and the destination shader must be accounted for separately.

## Source assessment

Supplied `S:\Downloads\Mogilny Test\Demitra\Demitra.Fbx`, SHA256
`a1b281236cb4dfef9dbac12568c88cf43e092c9087a6cc045b02bae04ff6a637`.
CC5 metadata is `Demitra.json`; material paths resolve under `Demitra.fbm` and
`textures/Demitra/Demitra`. This is a head export, not a full body: the main object
has only populated `Std_Skin_Head` polygons despite retaining body material slots.

- Head: 4,119 original vertices, 8,196 triangles, 4,282 position/UV records after
  preserving discontinuous per-corner UV seams for sampling.
- Separate eyes/corneas, teeth and tongue; no hair object is required for baldness.
- Main diffuse and normal maps: 4096x4096. Metadata also supplies roughness,
  metallic, AO, ORM, specular/micro-normal/SSS/transmission and wrinkle masks.
  These CC5 shader inputs are not all NHL19 material slots.
- Source rest geometry uses a 0.01 FBX object scale, applied once in extraction.
  Head world bounds are approximately X [-0.1134,0.1140], Y [-0.0908,0.1282],
  Z [1.4129,1.7626] metres, facing Blender -Y.
- Blender 4.5's FBX importer fails at `armature_setup[None]` on the supplied skin
  hierarchy. The read-only importer wrapper removes eight Skin/Cluster objects
  and their connections **in memory**, retaining geometry, transforms, materials
  and other deformation records. The original FBX remains unchanged. Native NHL
  rig data is retained, so this static import does not claim CC5 weight retargeting.

## Donor selection and measured layout

Marian Gaborik is on Slovakia's roster in the latest supplied NHL19 main database:
team `kOtt.vfEq=151`, roster `eGlu.dXSB=151`, player-index bridge
`eGlu.XSWT -> ZBac.vfEq -> ZBac.ykFq -> ubPc.wBIz=792`, player row 141.
His bespoke appearance value 289 is unique in the player-bio table. Asset routing
resolves to `marian_gaborik.mcd`; all 25 installed archives were enumerated.
This replaces his appearance wherever that asset is used, not his roster name.

Gaborik has six native components: head 3,285v/6,014t (including lash records),
two eyes 66v/100t each, haircap 580v/1,040t, cardsA 98v/92t, cardsB 434v/408t.
The eye components share the measured index buffer at offset 354963. Structural
declarations and original display vertex order agree. His 978,768-byte container
has two revisions, registered as separate fingerprinted native-writer profiles:

| Revision | SHA256 |
|---|---|
| base + patch93 | e9e316e70a870b2116b8777f51f9c58038a99ce71be11f0fbf29365afca11b1b |
| patch91 | 22b823733bd9a4c508677831000c59fc896938fe41da7fbf542f913e5e161203 |

The named player textures are head diffuse and hair diffuse/specular-labelled map.
There is no separately named Gaborik head normal in this archive scan. No new
shared normal/PBR binding is invented; existing native material bindings remain.

## Fitting and texture decisions

The saved thin-plate-spline calibration uses measured atlas landmarks around the
eyes, nose, lips, ears, forehead and chart bounds. It is a first measured CC5/NHL
calibration, not an automatically qualified mapping for every CC5 export.

- Preserve per-corner source UV seams. Collapsing these to one UV per source vertex
  caused surface tearing in the initial prototype and was corrected before packing.
- Sample the registered source UV triangles, with explicit nearest-chart fallback
  recorded by distance. The largest fallback is at chart edges, not proof of full
  exact correspondence for all native vertices.
- Preserve distinct native inner/outer lip records even where UVs coincide. Keep
  their native offset rather than collapsing both layers to the same source sample.
- Retain native oral geometry with mouth-following displacement and its original
  texture islands. The separate CC5 teeth/tongue assets remain source references.
- Match source eye bounds with native eye topology. Embedded lash strips follow
  the displaced skin while retaining their native offset.
- Align crown to donor Z 1.9478691816 m. The successful revision preserves 484
  native lower-neck/chest vertices through Z=1.735 m, blending up to 1.775 m
  under the jaw and 1.820 m at the nape. The earlier 1.66-1.74 m blend plus
  chart-border projection distorted the neck and is superseded.
- Fit the posterior scalp spatially where atlas correspondence breaks down.
  Lock both ears and two neighbour rings (259 records per side) to the pre-repair
  Demitra fit. Preserve core ear normals and blend surrounding shading. Repair
  coincident scalp seam shading without welding native records.
- Conform all three hair layers to the scalp and shrink radially to 90%, retaining
  native components and bindings. Their clearance is recorded in `mesh_report.json`.
- Bake the supplied 4K diffuse to Gaborik's existing 1024x1024 BC1 GNF with all
  11 mip levels and unchanged native texture header. The first test isolates the
  new CC5 conversion from texture-resolution/container changes. Full-resolution
  source maps are retained for future material/resolution work.
- Recompute native f16 normal/tangent XYZ and bounds; preserve original W values,
  UVs, topology, record order and other bytes.

## Evidence and limits

No new collapsed triangles in any revision-2 output component. The following
source-distance and UV metrics describe the initial fit, not a new measurement of
the repaired posterior/neck: the UV registration Jacobian
is positive at all 10,000 sampled facial-chart points (minimum 0.3370). For 2,224
front-face triangles, source-surface distance is median 0.159 mm, p95 0.699 mm,
maximum 2.830 mm. The 57 opposing nearest-source-normal comparisons all lie in
the oral slit, where retained native interior layers are not the CC5 exterior.
201 head triangle orientations differ from vanilla Gaborik; that comparison is
recorded, not interpreted as 201 proven foldovers. Hair orientations change as
the layers are folded inside the scalp.

Both native revisions passed profile-gated packing and production reparse.
All four replacement payloads passed production chunkzip decoding, native
position/topology/normal comparisons and 11-level GNF detile checks. Four virtual
EB append checks confirmed unrelated rows remain unchanged. All 31 mcd_write
unit tests passed after adding the measured donor profiles.

The user confirmed successful import in game for revision 2. Individual blink,
mouth-opening and equipment cases were not separately reported. This document
does not qualify arbitrary CC5 rigs or every NHL title/platform.

## Reproduction and private artifacts

Run in order with Blender 4.5 headless:
`nhl19_demitra_inspect.py`, `nhl19_demitra_uv_probe.py`, `nhl19_demitra_fit.py`,
`nhl19_demitra_finalize.py`, `nhl19_demitra_qc.py` (under `scripts/blender`).
Then run `nhl19_demitra_repair.py`: its output in `revision2` is the successful
final geometry. Omitting this step reproduces the superseded first fit.
The fit script contains the calibrated landmarks; `uv_warp.json` records them,
coefficients, alignment and anatomy selections. These are fixture scripts, not a
generic one-click product route.

Use `mcd_game_test_pack` for each donor revision, then
`scripts/nhl19_build_test_package.py <package> --recipe <recipe.json>`.
The explicit recipe supplies donor paths, player slug and accepted live archive
fingerprints; the historical Eichel invocation remains supported.

Private source inspection, roster evidence and vanilla donors:
`H:\TEMP\nhl19-demitra\{source,vanilla}`. Review scene:
`H:\TEMP\nhl19-demitra\revision2\Demitra_NHL19_Revision2.blend` / `.glb`;
both profiles, rear views, native meshes and repair report accompany the package.
The initial `package` and `fit` directories remain required historical inputs.
The installer receipt is authoritative for deployment status. Restore Demitra
before restoring earlier Mogilny packages, because their full-archive fingerprints
intentionally reject out-of-order restoration.

## Revision 2: neck, rear scalp and ear protection (2026-09-11)

User review exposed neck distortion and rear scalp shading/shape seams that the
initial face-distance checks did not catch. The original package was installed;
revision 2 is a separate review/package directory, not an installation receipt.

`scripts/blender/nhl19_demitra_repair.py` reads that original package and writes
`H:\TEMP\nhl19-demitra\revision2`. It freezes 484 native lower-neck/chest vertices
through native Y=1.735, blends higher under the jaw and through Y=1.820 at the
nape, and replaces posterior UV extrapolation with CC5 spatial correspondence.
Exact native positional seam duplicates retain equal coordinates and share smooth
normals. Crown height remains exactly the donor height. Hair is reconformed inside
the revised scalp, with minimum clearance greater than 7.7 mm.

Both ears are explicitly protected from the posterior correction: 259 records per
side, including two triangle-neighbour rings, match the original Demitra package
positions exactly (0 mm delta). Core ear normals also match that reference exactly;
the surrounding shading blends into the corrected scalp. Native IDs, UVs and
triangles are unchanged. These comparisons are against the pre-repair Demitra fit,
not against the differently tessellated CC5 source. The per-side record lists and
assertions are recorded in `repair_report.json`. Both profile renders and rear
solid views are required review artifacts; front-face metrics alone are insufficient.

The archive recipe can supply `previous_package` alongside a donor-specific
recipe. This preserves exact live-baseline and decoded-payload checks when making
a revision of a non-Eichel replacement. Restore revisions newest first.

Revision 2 was installed on 2026-09-11 after explicit user approval. The installer
completed all three archives; production EB/chunkzip readback of all four live
payloads matched the revision manifest. Receipts: `revision2/installation.json`
and `revision2/live_readback.json`. The user subsequently confirmed successful
import in game; detailed per-animation coverage was not separately recorded.

## Material scope and next test

Only **CC5 head diffuse** was imported as a source texture. It was rebaked from
4096 square to the native 1024-square BC1 atlas with 11 mips. Native oral texture
islands were retained. CC5 eye/teeth/tongue textures were not imported. Neither
revision imported CC5 normal, roughness, metallic, AO/ORM, specular, micro-normal,
SSS, transmission or wrinkle maps. Native NHL material bindings remain in place;
this does not mean NHL19 rendered without its existing material inputs.

Vertex normals and tangents were regenerated for the new geometry (with the ear
normal preservation described above). These are mesh attributes, not imported
normal-map pixels. No separately named Gaborik head normal was found, so the
effective shader's normal/material resource was not established. The user cancelled
this experiment; no normal replacement was installed and it is not pending work.

For another head, clone the fixture workflow into a new scratch directory; keep
the successful revision and its rollback metadata intact. Verify the new source's
UV/topology signatures before reusing landmarks. Re-measure the donor layout,
neck band, oral/lash/ear masks and crown. Check solid front, both profiles and rear
views before packing; face-distance scores alone missed the first neck defect.
Use the successful material scope initially. A separate material experiment should
hold this successful geometry and diffuse fixed, establish the native resource
binding, and then test one rebaked tangent-space normal map or one mapped material
channel at a time. Do not copy CC5 roughness into an unidentified NHL specular slot
or promote raw RGB normal warping to a correct tangent-space bake.
