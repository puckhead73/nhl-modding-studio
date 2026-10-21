# Head UV Templates and Transfer

**Evidence:** measured UV-class comparisons on small samples (two to six heads per title); NHL 11 to 14 on Xbox 360 and NHL 07 to 09 are not measured. One NHL 24 to Legacy conversion round trip is verified offline (in game: open). UV-correspondence transfer onto an NHL 19 PS4 head is user-reported working in game (CUSA11126, APP_VER 01.60). Head-mesh conversion is being completed outside the Studio; this page records what is established.

## UV classes

Heads were compared by an order-independent hash of the head submesh's UV multiset (quantised to a 1/2048 grid) and by a 256² coverage raster scored as "agree ± 2": the worse direction of the fraction of covered texels within two texels of the other mask.

| Pair | agree ± 2 | raw IoU | Verdict |
|---|---|---|---|
| Legacy X360 and NHL 15 X360 | 1.0000 | 1.0000 | identical: one exact multiset (2 751 vertices, 5 098 triangles, 2 609 unique UVs) shared by bespoke heads and the generic donor |
| NHL 24 and PS4 NHL 15 | 0.9871 | 0.9288 | same class (3 157 vertices, 6 014 triangles) |
| NHL 10 and Legacy / NHL 15 X360 | 0.9688 | 0.9585 | close but distinct (NHL 10: 2 657 vertices, 5 032 triangles) |
| NHL 10 and the PS4 class | 0.9731 to 0.9758 | 0.92 to 0.96 | close but distinct |
| Legacy / NHL 15 X360 and NHL 24 | 0.9339 | 0.8908 | related, **not interchangeable** |
| Legacy / NHL 15 X360 and PS4 NHL 15 | 0.9437 | 0.9420 | related, not interchangeable |

Classes so far: `x360-rw-v2` (Legacy and NHL 15 on Xbox 360), `x360-rw-v1` (NHL 10 on Xbox 360), `kettle-fb` (NHL 24, PS4 NHL 15 and 16); everything else unmeasured. The community belief that "NHL 11 to 24 share one head UV map" is not confirmed as identity: the RenderWare and Kettle generations lay the face out similarly but islands differ beyond storage jitter. Whether NHL 10 is a re-cut or a refinement needs a visual pass.

UV-class equality gates **texture-only transplants**. A full conversion carries the source's UVs and texture together, so it is not blocked by class. Eye rasters on RenderWare targets are separate (a shared 128×128 eye texture with donor eyeballs kept).

## Shared-UV template transfer

The preferred correspondence method when source and destination share the anatomical UV template. The destination keeps its native vertex ids, connectivity, UVs, rig binding and container structure; supported donor vertices move to the corresponding source positions; UVs act as **correspondence keys** (lip to lip, eyelid to eyelid), not as data to copy. Equal vertex counts or ordering are not required.

1. Extract both meshes with node transforms applied exactly once; normalise units, UV convention and output frame; undo an importer's V flip once and never add an unverified one.
2. Match on position plus UV for triangle-soup sources (position-only welding erases seams): nearest source UV within 0.0005, ties within 0.0000001; tied candidates more than 0.1 mm apart are ambiguous and not auto-selected. These are the measured settings for the NHL 19 recipe, not universal constants.
3. Require support: identical mapped triangles and at least two shared incident edges. Keep separate records of matched, supported, ambiguous and unsupported vertices.
4. Copy supported positions in one output frame without re-aligning eyes or mouth to the donor's proportions.
5. Placement (from in-game review): align scalp maxima measured on the head surface only; preserve the donor's shoulders, chest and neck base exactly; blend through an explicitly chosen neck band; move the face rigidly. Neck thresholds are template-specific.
6. Keep the donor's mouthbag, attach its seam vertices to verified exterior counterparts without welding ids, and propagate displacement through native connectivity. Fit eyeballs and lashes to the new eyelids.
7. Recompute native normals, tangents and bounds; preserve opaque fields; transfer textures with mip chains in the native format.
8. Verify by re-parse, native counts and topology, seams, likeness in three views, a reversible install and an actual game test.

The reusable matcher reproduces the recorded NHL 19 mapping exactly: 3 013 supported donor vertices and 2 983 complete shared triangles, with synthetic regressions. One supplied head over a stock NHL 19 head was reported a full success in game; its source provenance was not established, so it is not recorded as a proven NHL 24 to NHL 19 conversion. A second import from an external character-creator export succeeded on its second revision using calibrated UV registration plus barycentric sampling for the diffuse texture only, after adding a retained native neck, a posterior scalp fit, bilateral ear locks and seam-normal correction. Open: barycentric sampling for different tessellation density inside the direct matcher; reverse conversion (many-to-one mappings are not invertible); clothing, hair and equipment UVs, which are unrelated to the head template.

## NHL 24 to Legacy (Xbox 360) conversion

A verified offline round trip, not an in-game result:

- **Weight remap from 433 to 122 bones:** override, then direct, then ancestor-climb, then drop; keep the top four influences and renormalise.
- **Pack** into a donor `player_<id>.rx2`: area-weighted normals, the UV flip from GL to stored space, blend indices as palette × 3, haircap collapsed.
- **Texlib rebuild** with a full mip chain down to 16×16, DXT1 head (512²) and optional DXT3 hair (256²), **exactly 331 776 bytes**.
- Gates passed: strict re-parse; the converted head's UV-multiset hash **bit-identical to EA's own PS4 build of the same player**; a palette census of 27 slots all inside the vanilla-used set with zero mass on slots 5, 20, 49 and 80; a head id below the 60 000 pool floor.
- First-run quality: facial RMS 1.89 mm (max 13.42 mm, "caution"); retarget RMS 5.71 mm; eyes and mouthbag translated onto source-measured centres and verified within 1.5 mm.
- Limits: no hair cards (bald conversions), lashes dropped, eyeball size not scaled, one donor profile tested.
- The manual in-game gate is to install the mesh and texlib and set the head id and portrait id to the same value (see [Player Heads, Portraits and Appearance Binding](Player-Heads-Portraits-and-Appearance-Binding)).

Last reviewed: 2026-10-20
