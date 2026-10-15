# NHL19 MeshDataTransfer add-on comparison

Installed and enabled user-supplied MeshDataTransfer 2.0.9 in Blender 4.5.5 LTS.
Actual ZIP: `S:\Downloads\MeshDataTransfer_v2.0.9.zip`.
No game deployment or repository commits performed.

## Controlled test

Runner: `scripts/blender/nhl19_addon_transfer_compare.py`, reusing
`scripts/blender/nhl19_data_transfer_test.py`.

Inputs: `H:\TEMP\nhl19-eichel-glb\jack_eichel_native.glb` and
`S:\Downloads\Mogilny Test\alex_mogilny.glb`. Exact hashes in comparison.json.

Compared Blender's built-in Nearest Face Interpolated transfer of normalized XYZ
vertex-group channels with the add-on's actual `object.transfer_mesh_data` operator:
Closest, World, Shape, snap off. Both used identical mouth and eye sampling
translations, common mouth-anchored output coordinates, and the same regional blend.
Each method ran mouth-only and blended two-pass variants. Mouth landmarks remain
provisional; this is not an anatomically validated fit.

Outputs: `H:\TEMP\nhl19-addon-comparison`, with `builtin` and `addon` subfolders,
each containing the full Blender scene, both GLBs, report and twelve rendered views.
`comparison.png` shows built-in two-pass, add-on two-pass and source, left to right.

## Results

| Metric | Built-in | Add-on |
| --- | ---: | ---: |
| Identity-transfer maximum error, mm | 0.0000167 | 0.000597 |
| Mouth-only orientation reversals / collapsed triangles | 158 / 20 | 156 / 20 |
| Two-pass orientation reversals / collapsed triangles | 116 / 18 | 125 / 18 |
| Two-pass moved head vertices | 2982 | 2982 |

Between methods: mouth-only RMS difference 0.00337 mm, two-pass RMS 0.00465 mm;
maximum difference 0.11678 mm for both. Orientation reversal means a triangle's
normal has negative dot product against its donor normal; it is a diagnostic,
not a definitive self-intersection test. Counts near degeneracy are sensitive to
the tiny numeric differences and do not demonstrate a meaningful quality ranking.

Visual inspection confirms essentially the same result, including damaged nose,
lip and ear surfaces. Closest mode does not resolve anatomical correspondence.
Separate lip IDs survive, but mouthbag integrity and animated lip separation are
not established. Eyeballs/lashes use identical companion translations in both tests;
their placement is not an independent result of either transfer operator.

The add-on also offers Vertex ID mode, gated by equal vertex counts. The present
3157-vertex donor and 17400-record source do not qualify. Shared-template mapping
requires additional evidence about original source topology and vertex ordering.

## Export validation

The shared runner now explicitly bakes evaluated shape keys into temporary meshes
before exporting. Each of the four GLBs was reimported and its head position set
checked bidirectionally against SelectedResult with a 0.001 mm tolerance; all passed.
This check permits export vertex splitting/reordering, so it is not proof that GLB
vertex indices preserve native indices. Native donor topology remains in the .blend.
Use this new output folder for baked comparison GLBs.

Run (installed add-on must be readable by Blender):

```powershell
& 'C:\Program Files\Blender Foundation\Blender 4.5\blender.exe' --background --factory-startup --python-exit-code 1 --python scripts/blender/nhl19_addon_transfer_compare.py -- 'H:\TEMP\nhl19-eichel-glb\jack_eichel_native.glb' 'S:\Downloads\Mogilny Test\alex_mogilny.glb' 'H:\TEMP\nhl19-addon-comparison'
```
