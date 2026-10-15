# RPSGL2 review — 11 October 2026

Initial request: assess RPSGL2 as a route to PS3 mesh parity while the user reviews Legacy onboarding. The initial review below was research only. The subsequent authorized native integration is recorded at the end of this document.

## Conclusion

**11 October visual-QA correction:** RPSGL2's EDGE unit-vector decoder omits the `1/sqrt(2)` scale for the two stored components. Native decoding originally inherited this defect. A position-matched Getzlaf (head 1092) Xbox/PS3 comparison identified it: correcting the scale reduced normal-component mismatches above 0.1 from 2,394 to 1 across 2,768 matched head vertices. Native decoding and normal/tangent encoding now use inverse matching scales. The external reader remains a reference for positions, UVs and indices, but its EDGE normals are no longer treated as a correctness oracle. A fixed packed-vector witness and encode/decode tests cover this correction; native no-op/edit checks still pass on 13 real mesh fixtures.

The developer's import/export claim is supported for a bounded set of PS3 formats. This is a useful reference for native PS3 mesh decoding and topology-preserving editing, not full arbitrary mesh replacement or automatic Xbox/PS3 conversion.

## Evidence

Reviewed `RPSGL2.zip_pass_123/RPSGL2` (README identifies version 1.9), particularly `rpsgl.py`, `equipment.py`, `edge_codec.py`, `containers.py`, `fbxio.py`, and supplied tests. Ran `probe.py` using NumPy 2.3.5/Pillow 12.3.0 against the read-only Legacy Vanilla installation at `X:/Games/Emulation/Roms/Sony Playstation 3/NHL Legacy - Vanilla/PS3_GAME/USRDIR/rendering`. Exports and edited copies were temporary; source bytes were checked unchanged. Exact sample names, hashes, counts and results are in `results.json`.

14 selected samples passed: seven player files (66/67/71/76/80/88/89), skate, helmet, glove, stick, trapper, goalie cage, and stadium_0_container_0. This is a sample, not a complete asset census.

- All 14: unchanged save was byte-identical; OBJ and binary FBX export/reimport retained triangle indices and positions within format precision.
- All 13 EDGE models: unchanged OBJ/FBX reimport produced byte-identical RPSGL; moving one vertex by 1/16384 survived save/reopen, with all bytes outside vertex attributes unchanged. Reordered triangles and out-of-range positions were rejected atomically.
- Compound player files contained seven or nine EDGE parts; helmet sample contained two.
- Stadium export contained 85,467 vertices and 58,087 triangles. No stadium import was tested or claimed.
- Initial probe selected a helmet diffuse texture as a mesh; corrected filename filtering. Initial arena OBJ precision assertion was too strict: the exporter writes ten significant digits, producing a maximum absolute coordinate error of 0.0000040625 on this sample. Final probe explicitly records error and uses a corresponding relative tolerance. FBX positions were exact. These were probe corrections, not source-library fixes.

The developer's supplied tests use an absent hard-coded Downloads fixture directory. Their claimed full sample counts were not treated as local results.

## Capabilities and limits

| Asset path | Supported by reviewed code | Limits |
| --- | --- | --- |
| EDGE 0x20050 player/equipment | Read segmented/compound geometry; OBJ/FBX export; rewrite position, normal and first UV in existing slots | Original vertex order/count and exact triangles required; fixed-point position range; preserved skinning, additional UVs, culling/motion metadata |
| Static 0x20009 trophy profile | Geometry editing and resized GPU buffers, including uint32 indices | Strict trophy_0 section/layout checks; inspected in source, not independently tested on a trophy |
| Stadium/ice containers | Geometry export; textured preview using mapped diffuse/lightmaps/vertex colors | Geometry import disabled; stored coordinates, incomplete scene transforms/shader reproduction |
| Interchange | OBJ and binary FBX | No exported skeleton, weights or animation; FBX import rejects deformers; merged geometry does not carry full game materials |

Equipment external texture assignment is not automatic in RPSGL2. Fixed-topology imports cannot install arbitrary high-poly heads, new hair cards, or differently rigged meshes. Preserving existing bytes is not proof that the game accepts edits. Large deformations retain stale culling/motion metadata. Normal-only edits deserve an additional tangent-basis test: tangent regeneration currently depends on position/UV changes.

## Studio integration points and recommended order

1. Add a native RW4ps3/EDGE parser beside `crates/tdb-rx2/src/playermesh.rs`; that existing parser explicitly requires RW4xb2. Preserve segment identity, all opaque streams and section references. Use the Python implementation as a comparison reference and validate hostile/truncated input independently.
2. Route PS3 preview through `kit_preview.rs` and `equipment_piece.rs`; the latter explicitly skips PS3 mesh files. Remove the PS3 blocker in `KitMeshViewer.tsx` only after the native route works. Reuse Studio's existing PS3 texture support (`texlib_ps3.rs`) and resolve actual equipment materials, rather than accepting RPSGL2's solid-only equipment preview.
3. Add OBJ/GLB export using Studio's existing conventions, preserving a sidecar of segment/vertex identities. FBX can remain an optional interchange route; a bundled Python GUI is unnecessary.
4. Add a guarded same-topology writer, then connect it to session edits, undo/redo, Overview, save/reopen, build packages and Mod Manager restoration. Do not enable new topology for skinned meshes based on the trophy writer.
5. Verify edited heads and representative skater/goalie equipment in RPCS3, including textures, animation, clipping and restore. Extend the format census by title before claiming PS3-wide support. Arbitrary topology, skeleton changes and automatic cross-platform asset conversion remain separate work.

`edge_codec.py` credits Hancapo/fivefury under Unlicense. The supplied folder has no top-level license and references an absent THIRD_PARTY_LICENSES.txt. Record the full source permissions/provenance before copying third-party implementation into a distributable build; this did not block local research.

## Native integration — 11 October 2026

Implemented an independent Rust codec in `crates/tdb-rx2/src/ps3_mesh.rs`, dispatched by the existing player/equipment geometry pipeline. Studio now discovers PS3 mesh containers, resolves skater/goalie equipment and head/eye textures, enables its existing uniform preview, and exports geometry through the existing asset GLB route. No Python runtime or RPSGL2 GUI is bundled.

The native `ps3-mesh` command exports OBJ/GLB and imports OBJ into existing EDGE vertex slots. Positions, normals and first UV are editable; tangents regenerate for geometry, UV or normal changes. Original part/vertex counts and triangle order are required. Unsupported layouts, nonfinite values and positions outside the original fixed-point range are refused. Output must be a new file. Skinning and opaque metadata remain in the source container; skeleton/animation interchange is not implemented. Static stadium geometry exports but cannot be imported. Large deformations still need culling/motion investigation.

```powershell
cargo run -p tdb-rx2 --bin ps3-mesh -- export input.rpsgl output.obj
cargo run -p tdb-rx2 --bin ps3-mesh -- export input.rpsgl output.glb
cargo run -p tdb-rx2 --bin ps3-mesh -- import input.rpsgl edited.obj output.rpsgl
```

Verification completed:

- 130 native codec regression tests passed; 26 optional fixture tests ignored.
- Three explicit real-fixture tests passed: 13 EDGE files, static stadium export/refusal, and all 14 files compared vertex-by-vertex and triangle-by-triangle against the independent reader. Includes no-op byte identity, position/normal/UV edits, malformed input and topology/range refusal.
- Nine kit preview regression tests and three frontend texture tests passed. TypeScript/Vite production build and native Studio debug build succeeded.
- The full Studio API assembled textured Legacy PS3 skater and goalie kits from a disposable project. Every tested head/equipment mesh resolved a material except the intentionally untextured metal goalie cage. Actual installations were read-only.
- Native CLI OBJ export/import produced a byte-identical player container; attempting to reuse its output path was refused.

Remaining qualification and integration: inspect Legacy PS3 heads and skater/goalie uniforms in Studio; validate modified files in RPCS3 for animation, lighting and clipping. Native OBJ import has **not** been connected to project undo/redo, Overview, save/reopen, `.nhlmod` builds or Mod Manager restoration. That adapter is required before distributing mesh edits through Studio projects. No FBX importer, resized skinned topology, trophy resizing, automatic platform conversion or all-title PS3 parity is claimed. The earlier tester ZIP is unchanged.
