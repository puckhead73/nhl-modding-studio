# Shared-UV template transfer for NHL meshes

For the complete Mogilny/Demitra handoff, successful revisions, dependencies and
deployment sequence, start with the [NHL19 head import playbook](NHL19_HEAD_IMPORT_PLAYBOOK.md).

Preferred correspondence strategy for compatible NHL head templates, saved after
the user-reported successful Mogilny-over-Eichel NHL19 PS4 game test on 2026-09-10.
Research/application workflow scope requested by the user: NHL10 through NHL24,
import and export in either direction. The user reports a shared head UV layout
across these games. Validate each encountered asset/template; this report does not
establish that every mesh category, platform, rig or container is interchangeable.

## Decision

For an NHL head whose source and destination share the anatomical UV template,
try UV correspondence before spatial shrinkwrap, nearest-surface fitting or
manually frozen donor-face fitting. Keep the destination-native vertex IDs,
connectivity, UVs, rig/binding data and container structure. Move supported donor
vertices to the corresponding source coordinates at full strength.

The UVs identify anatomical locations: lip maps to lip, eyelid to eyelid, nose to
nose. They are used as correspondence keys, not copied over the destination UVs.
This avoids compressing a longer source face back into donor eye/mouth proportions.
Equal vertex counts and equal vertex ordering are not prerequisites.

## Reusable code and successful recipe

- `scripts/blender/uv_template_match.py`: reusable matcher and JSON command-line
  entry point. It outputs correspondence/evidence only; no native asset mutation.
- `scripts/tests/test_uv_template_match.py`: five synthetic regression cases.
- `scripts/blender/nhl19_uv_correspondence_fit.py`: the original complete NHL19
  fitting recipe, with measured fixture paths, anatomical/attachment choices and
  preview/export steps. Kept intact as the reproducible successful experiment.
- `scripts/blender/nhl19_finalize_game_mesh.py`: seam, eyes/lashes, hidden-hair
  finishing and native-order export. Now defaults to crown alignment; use
  `--keep-mouth-height` to reproduce the first game's placement.
- `scripts/blender/head_scalp_placement.py`: reusable post-correspondence placement;
  matches the actual head crown, preserves the donor lower attachment, and blends
  through a destination-specific neck band. Two synthetic placement tests.
- `crates/tdb-rx2/examples/mcd_game_test_pack.rs`: profile-gated NHL19 native writer.
- `scripts/nhl19_build_test_package.py` and `scripts/nhl19_test_install.py`: exact
  NHL19 texture/archive packaging and reversible installation, not generic writers.

Reusable matcher invocation:

```powershell
& 'C:\Program Files\Blender Foundation\Blender 4.5\blender.exe' --background --factory-startup --python-exit-code 1 --python scripts/blender/uv_template_match.py -- input.json mapping.json
```

Input JSON has `donor` and `source` objects, each with `positions` (XYZ per record),
`uv` (UV per record), and `triangles` (three record indices). Optional `options`
passes `uv_tolerance`, `tie_epsilon`, `position_ambiguity`. Normalize UV convention
before matching. Position ambiguity uses metres. Native donors must retain their
original split records; a per-corner-UV adapter must preserve the mapping back to
native IDs. Source record IDs in the ledger refer to the supplied source arrays.

The reusable matcher reproduces the recorded NHL19 mapping exactly: 3013 supported
donor vertices, 2983 complete shared triangles. This parity check was executed on
the real fixture; the synthetic tests cover reordered source IDs, changed source
shape, opposite quad diagonals, UV rounding, ambiguous overlapping surfaces and
invalid geometry. The standalone helper is not yet a universal product-CLI route.

## Matching algorithm

1. Extract both meshes with node transforms applied exactly once; retain mesh roles,
   native IDs, topology, UVs, normal/tangent conventions and source fingerprints.
2. Normalize units, UV convention and intended output frame explicitly. An importer's
   V conversion is undone once when packing; never add an unverified V flip.
3. For triangle-soup sources, create a deterministic position-plus-UV representation
   for matching. Position-only welding can erase lip/interior or UV seams. Do not
   weld or reorder the native destination.
4. Find the nearest source UV. The NHL19 recipe admits a distance of 0.0005 with
   ties within 0.0000001; tied points differing by over 0.1 mm in source position
   are ambiguous and are not automatically selected. These are measured recipe
   settings, not universal constants.
5. Check supporting source connectivity: identical mapped triangles and at least
   two shared incident edges. Differing quad diagonals can retain valid edge evidence.
   Record matched, supported, ambiguous and unsupported native IDs separately.
6. Copy supported source positions directly. Use one output coordinate frame for
   the whole head. Do not align mouth and eyes independently into the old donor
   proportions after finding the correspondence.
   **Placement rule (2026-09-10 in-game review): align scalp maxima.** After applying
   transforms and matching, translate the incoming head so its highest scalp point
   matches the donor's highest scalp point along the target's up axis. Measure the
   head surface, excluding hair, cards, equipment and unrelated objects. Do not use
   an object origin or lock the mouth height. Preserve the donor shoulders, chest
   and base of neck exactly; blend donor positions into the translated fit through
   an explicitly selected neck band. Move the facial region rigidly, preserving
   eye-to-mouth proportions and mouthbag attachment. Eyes/lashes and hidden hair
   must follow the placed head. Neck thresholds are template-specific, not universal.
7. Handle unsupported native geometry explicitly: preserve donor-only mouthbag
   structure, attach its seam vertices to verified exterior counterparts without
   welding IDs, and propagate displacement through native connectivity. Validate
   the attachment region anatomically. The historical NHL19 `shell_boundary` mask
   also contained scalp seams and must not be frozen wholesale.
8. Fit eyeballs and lashes to the newly placed eyelids as companion geometry. The
   NHL19 test used source eye bounds and donor-surface displacement for lashes.
   A generic rule to freeze eyes at donor coordinates does not apply to this method.
9. Recompute native-format normals/tangents and required bounds; preserve measured
   fourth-component conventions, secondary UVs and opaque fields. Pack through a
   validated destination-specific adapter. Retained rig bytes still require runtime
   testing after changing rest geometry.
10. Transfer template-compatible textures with their mip chains and appropriate
    native format. Treat absent normal maps and changed shader conventions explicitly.
11. Verify reparse, native IDs/counts/topology, shading, seam behavior, source likeness
    in three views, and reversible archive installation. Test the actual target game.

## Where the method generalizes

For the supplied CC5 Demitra export, see
[CC5-to-NHL19 transfer](CC5_TO_NHL19_HEAD_TRANSFER.md): calibrated UV registration
and barycentric source-surface sampling extend this workflow to different atlases.
That fixture preserves Gaborik's native layout and rebakes the diffuse; it does
not imply direct equality of CC5 and NHL UVs or interchangeable shader maps.
Demitra **revision 2** was user-confirmed successful in NHL19 on 2026-09-11;
see the [success receipt](validation/2026-09-11-n19-demitra-cc5-success.md).
Its required refinements are a retained native neck, posterior scalp fitting,
explicit bilateral ear locks and seam-normal correction. Only the CC5 diffuse
texture was imported; other CC5 shader maps remain a separate test.

Shared anatomical UVs provide a reusable coordinate system across different head
shapes, counts and record ordering. A fixed per-template mapping can be reused once
its native layout, UV/topology signatures and source representation are verified.
Reverse conversion uses the reverse destination's topology and its own mapping;
an existing many-to-one mapping is not automatically invertible.

If two meshes share the UV layout but have different tessellation/density, direct
UV-vertex matching may leave too much unsupported. The next extension is
**barycentric sampling inside source UV triangles**, restricted to the correct UV
island/anatomical region, with overlap and degeneracy checks. That extension was
not required for the original Mogilny NHL19 experiment and is not implemented in the
saved direct matcher. Do not silently substitute 3D nearest-surface fitting.
The separate Demitra CC5 fixture implements registered barycentric sampling with
explicit anatomy-specific exceptions; it does not change the direct matcher's contract.

UV islands may overlap or be mirrored, variants may use another atlas, interior
regions may be absent, and clothing/hair/equipment meshes may have unrelated UVs.
The user's shared-head-template observation does not imply a single UV layout for
every asset class. Container encoding, rig channels, material/shader meaning,
compression and installation remain title/platform-specific.

## Evidence and next use

Confirmed case: supplied Mogilny GLB to NHL19 PS4 CUSA11126 APP_VER 01.60, replacing
Jack Eichel. The user reported **"Full success"** after the installed package was
delivered. The source GLB's original game provenance was not established, so this
is not recorded as a proven NHL24-to-NHL19 conversion. No per-animation checklist,
runtime screenshot/log bundle or exact shadPS4 build was supplied with that report.

See the [UV fit receipt](validation/2026-09-10-n19-mogilny-uv-template-fit.md) and
[native installation receipt](validation/2026-09-10-n19-mogilny-game-test-package.md).
Successful private artifacts, correspondence ledger, native files and rollback
receipt remain in `H:\TEMP\nhl19-mogilny-game-test` and the earlier fit folders.
Keep the installed package's restoration metadata while the test is installed.

The scalp/2K refinement is recorded in
[the revision receipt](validation/2026-09-10-n19-mogilny-scalp-2k.md).
For Eichel, the finalizer keeps native vertices at Blender Z <= 1.68 m exact,
blends over 1.68-1.72 m, then translates the upper region by -0.0178362131 m.
The final crown is 1.9450141191 m. Do not transplant these heights into other
games/players without selecting their anatomical attachment region.

For each next game/platform/template, record extraction/packing support, UV and
connectivity coverage, unsupported anatomy and a runtime receipt. This document
sets the preferred research workflow; it does not bulk-promote capability matrices
or assert NHL10-24 native import/export is already implemented everywhere.
