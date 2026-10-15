# NHL19 Mogilny: UV-template correspondence fit

Artifacts: `H:\TEMP\nhl19-mogilny-uv-template-final`.
Runner: `scripts/blender/nhl19_uv_correspondence_fit.py`.
Status: offline geometry experiment with markedly closer source likeness; not a
validated native import or animation result. No game deployment, commits or pushes.

## Finding that changed the approach

The supplied Mogilny GLB has 17400 triangle-soup records, 2924 exact distinct
positions and 3041 position/UV combinations. The NHL19 donor has 3157 records,
3031 exact positions and 3157 position/UV combinations. Equal original index order
cannot be inferred. However, the UV layouts share extensive correspondence.

An initial UV radius of 0.00001 found 2874 unambiguous matches and 2641 complete
shared triangles. The final radius is 0.0005; candidates are selected at the
nearest UV distance, with ties within 0.0000001. If tied candidates differ by more
than 0.1 mm in source position, the vertex is rejected as ambiguous.

Final evidence:

- 3018 unambiguous UV matches, 21 ambiguous UV vertices explicitly rejected.
- 2983 complete mapped triangles also exist in the source.
- 3013 donor vertices have at least two mapped neighboring edges present in the
  source. These receive the source coordinates directly.
- 2744 fully mapped donor triangles do not occur verbatim in the source. Shared
  edges allow differing triangulation; this is not proof of identical full topology.
- 144 remaining vertices use donor-graph displacement interpolation, including
  21 unsupported seam vertices explicitly attached to coincident verified exterior
  counterparts. Separate native IDs and lip-loop connectivity are retained.

The UV tolerance accommodates observed near-matches up to 0.0004887; rounding is
a plausible cause, not an established description of the source export history.
No source vertices were welded or reordered to fit the donor. Source position/UV
deduplication is only a correspondence representation. The source-only render copy
is welded for comparable shading; output donor topology is preserved.

## Fitting and correction

Mapped facial vertices receive Mogilny coordinates at full strength, in the common
mouth-anchored frame from the comparison fixture. There is no weakened projection
or anatomical-landmark interpolation for these vertices. The lower attachment
remains native, with a vertical blend from Blender Z 1.66 to 1.72 metres.

The old `shell_boundary` mask includes scalp seam vertices, not solely the lower
attachment. Freezing the entire mask created a crown notch. This experiment only
freezes those mask members below Z 1.70. Prior masks should not be treated as exact
anatomical labels or reused as unconditional shell constraints.

Eyeballs and lashes retain donor topology and move by the difference in per-eye
centres. Haircap/cards remain donor geometry. Their final fit is still outstanding;
hair and lashes are hidden in face-comparison renders. Preview/export normals are
recomputed from geometry so donor custom normals do not drive the comparison.

## Validation and practical limits

- Native head retains 3157 vertices and exact polygon connectivity in Blender.
- Reimported GLB has zero bidirectional positional-set error against the selected
  shape-key result; GLB exporter index preservation is not implied.
- 1290 source vertices visible from the front, above Z 1.72, within X +/-0.085 and
  Y below -0.025, sampled against the fitted surface: RMS 0.11488 mm, max 1.78849 mm,
  95th percentile zero within measured floating-point precision. This is a sampled
  one-way facial distance, not a whole-mesh Hausdorff or animation metric.
- 38 donor-relative triangle-normal reversals, zero collapsed triangles. A changed
  normal direction is not by itself an inverted or self-intersecting triangle.
- Four originally coincident groups retain >0.1 mm spread; IDs are in report.json.
  Interior continuity and animated upper/lower lip separation need further review.
- Front, three-quarter and profile were inspected against identically framed source
  renders. Facial outline and features now closely follow the supplied Mogilny mesh.
  Geometry resemblance does not establish rig compatibility or in-game quality.

## Review files

- `front_comparison.png`, `threequarter_comparison.png`, `profile_comparison.png`:
  original Eichel, fitted NHL19 topology, supplied Mogilny surface.
- `Mogilny_NHL19_UV_Template.glb`: full static geometry with recomputed normals.
- `Mogilny_UV_Template_Test.blend`: original and fitted collections, reversible
  `Mogilny_Template` shape key, mapping groups, source and fitted preview copies.
- `report.json`: hashes, mapping by donor index, rejected/unsupported vertex IDs,
  diagnostics and checks. Mapping source IDs refer to the script's deterministic
  position/UV representation, not original game vertex IDs.

This establishes a concrete template-correspondence route for this pair. Reuse for
other heads/games requires validating their UV and connectivity profiles first.

```powershell
& 'C:\Program Files\Blender Foundation\Blender 4.5\blender.exe' --background --factory-startup --python-exit-code 1 --python scripts/blender/nhl19_uv_correspondence_fit.py -- 'H:\TEMP\nhl19-mogilny-uv-template-final'
```
