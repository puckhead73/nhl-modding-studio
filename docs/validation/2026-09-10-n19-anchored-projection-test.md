# NHL19 anchored skin projection experiment

Final artifacts: `H:\TEMP\nhl19-anchored-projection-v3`.
Runner: `scripts/blender/nhl19_anchored_projection_test.py` in Blender 4.5.5 LTS.
Inputs are the preserved donor/reference collections from the previous built-in
comparison scene, with the source already in the common mouth-anchored frame,
and `E:\tmp\nhl19-mogilny\out\fit\masks.json`.

23 provisional facial anchors use eye bounds centres, front-surface ray samples
around lids, lips, nose and cheeks, and ear extrema. Additional shell anchors
constrain the lower boundary. These are geometric proposals, not artist-approved
anatomical correspondence. A regularized Gaussian displacement field fits the
donor, followed by nearest-skin projection within 8 mm, with normal and lip-side
guards. This is scripted geometry processing inside Blender, not the add-on.

Eyeballs, lashes and hair do not participate in projection; the target is only
Mogilny's head surface. The 494-vertex conservative oral-cavity mask, additional
occluded facial vertices and shell masks exclude 1223 head vertices in total.
Internals receive the smooth anchor deformation but no projection. This mask is
not an exact mouthbag segmentation. Eyeballs/lashes receive companion translations
to source eye centres and remain available in the full result.

The initial low-regularization fit failed (963 donor-relative normal reversals).
A rigid cavity freeze in v2 also created discontinuities (115 final reversals).
Final v3 regularizes the field at 0.05 and smoothly deforms the excluded interior,
preserving continuity with exterior skin. Earlier output folders remain diagnostic.

## Final measured result

- All 3157 donor head vertex indices and polygon connectivity preserved in Blender.
- 1458 vertices eligible for projection; global fold guard reduced projection to
  0.125 strength, making the maximum projected correction at most 1 mm.
- Anchor-only and final meshes: zero donor-relative triangle-normal reversals,
  zero triangles below the collapse threshold. Projection also introduces zero
  such failures relative to the anchored mesh. These checks are not a complete
  self-intersection or animation validation.
- Both GLBs reimported; every exported head position matched a selected-result
  position within 0.001 mm. This does not establish native index order in GLB.
- Visual inspection: previous nose/mouth tears resolved, ear surface improved;
  residual Eichel shape and eyelid/eyeball mismatch remain. Smoothness is not proof
  of accurate Mogilny likeness. Lip animation and cavity function remain unverified.

`comparison.png`: previous raw two-pass transfer, final projection, Mogilny source.
`Anchored_Projection_Test.blend`: donor, source, previous transfers, anchor-only,
final mesh, provisional landmark empties, exclusion group and reversible shape keys.
`08_PROJECTED.glb`: full static result, including companion meshes.
No game files deployed, no commits or pushes.

```powershell
& 'C:\Program Files\Blender Foundation\Blender 4.5\blender.exe' --background --factory-startup --python-exit-code 1 --python scripts/blender/nhl19_anchored_projection_test.py -- 'H:\TEMP\nhl19-anchored-projection-v3'
```
