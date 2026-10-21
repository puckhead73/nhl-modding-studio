# Head asset library and conversion investigation — 20 October 2026

This work implements a compiled head asset library and Historical/EA Appearance
browser. It does **not** complete universal native game-to-game head replacement.
No original game installations were modified, and no in-game animation test was
performed during this run.

## Compiled corpus

The library is at `graphical database/headmeshes`, with a SQLite index,
content-addressed DDS textures, GLBs split into head/hair/facial-hair classes,
`sources.json`, `summary.json`, `unresolved.json`, and `provenance.csv`.

| Contents | Count |
| --- | ---: |
| Unique head meshes | 15,369 |
| Unique hair meshes | 13,714 |
| Unique facial-hair meshes | 331 |
| Total unique meshes | 29,414 |
| Unique DDS files | 30,222 |
| Source appearances retained | 69,905 |
| Duplicate mesh occurrences consolidated | 40,491 |
| Unresolved player appearances | 5,413 |

Thirty-one selected source catalogs cover Xbox 360 NHL07–15 and Legacy,
PS3 NHL08–15 and Legacy, PS4 NHL15–24, and PS5 NHL26–27. These counts describe
the selected exports, not every update/DLC revision of every game.
PS5 sources are the Data layer; Patch resources were inventoried separately.

Mesh identity includes exact decoded accessor bytes, topology, UVs, submesh
roles, and any preserved native vertex channels. No rounding or perceptual
matching is used. Reordered but geometrically equivalent surfaces may remain
separate. Newer games win canonical selection; Xbox 360 wins over PS3 within
the same title. Every source appearance and its texture set remains indexed.
Heads retain native coordinates and split records. Bespoke hair cards are
separated into a single hair asset containing their available card submeshes.

Known player identities use the player slug and a numbered variant. Unambiguous
licensed resource names can resolve against known full names; missing/ambiguous
identities remain unresolved. Generic and staff assets retain descriptive names.
A post-build classification correction moved 549 bench/referee appearance
records into staff categories without changing mesh or DDS contents.

## Extraction findings

- NHL09 PS3 early EDGE layouts require separate stride/UV offsets: `(4,14)`
  uses stride 17, UV offset 9; `(3,15)` uses stride 13, UV offset 9.
  All nine final PS3 exports completed with zero reported asset failures.
  Early EDGE native writing remains unsupported.
- NHL26 and NHL27 PS5 use different section/declaration and LOD offsets.
  Four measured skinned/rigid layouts have regression coverage, including
  inline and streamed geometry. Final exports produced 4,073 GLBs / 6,227 DDS
  for NHL26 and 4,356 GLBs / 6,880 DDS for NHL27, with zero reported failures.
- Complete streamed texture spans come from TOC placement records, rather than
  the shorter bundle range at the same CAS offset. PS5 export handles measured
  swizzles and padded small textures. DDS exports contain the base mip.
- Texture associations use the asset family and the actual character-shared
  texture folder. A generic `/common/` substring incorrectly included other
  players' folders; it was corrected and regression-tested.
- The explicitly authorized Oodle DLL was used as a decoder. No other
  UEViewer source was used.

## Studio functionality

Both Historical and EA player views expose an Appearance library panel:

- Indexed name/slug and class search, source texture-set selection, LOD/source
  information, and the shared interactive mesh viewer.
- Independent submesh visibility and optional separate hair/facial-hair
  companions.
- Per-person local saved selections, restored on this computer.
- Portable GLB/DDS package export with source provenance, including optional
  hair and facial-hair packages.

Shared geometry can have different player textures. Selection prefers the
searched identity's appearance instead of blindly using the first source.
Texture/material binding is currently inferred from map names, not a complete
native EBX/shader binding implementation. Remembering an appearance does not
write a game asset or change the historical database.

## Conversion and rig evidence

The user's prior converter is imported under `tools/head-converter`, with
original source hashes. NumPy scalar half conversion, boolean neck-mask logic,
and inherited workspace-license handling were corrected. Its anatomy worker
remains a calibrated CC5-to-NHL21 route, not an arbitrary GLB-to-any-game route.

The imported native packer builds against the current Studio codec. A measured
NHL21 donor no-op export/repack is byte-identical. Replaying the original Turgeon
anatomy export stops at its missing `TurgeonTest.Fbx`; source fingerprint checks
were retained.

The shared-UV measurement covers 31 representative heads and 961 directed
source/destination comparisons. Exact source position+UV duplicates are
collapsed only for correspondence; native destination records are preserved.
Some titles use a fallback player, explicitly recorded in the matrix.
High UV agreement is evidence for correspondence, **not** native writer,
mouth/eye placement, rig, or animation qualification. Early Xbox/PS3 samples
show materially different mappings; blindly applying the modern atlas is not
supported by the measurements.

Native bind sampling covers 78 assets across 26 non-Frostbite combinations.
Xbox sample bind counts change from 131 to 140, 123, then 122 across generations.
Absent exported PS3/PS4 bind data is not evidence of a zero-bone rig.
The NHL26/NHL27 player skeleton resource is byte-identical and decodes into
434 named model-space bind bones. Preserved PS5 index/weight channels were
audited against that roster. NHL26 has 8,249 compatible-channel sections,
97 other-layout sections and six without skin channels; NHL27 has 7,306
compatible-channel sections and six without skin channels. Other layouts
are recorded rather than guessed. Hierarchy/animation retargeting is unqualified.

## Validation and reproducibility

Private receipts are under `local/headmesh-overnight-20261020`.
Useful entry points:

```powershell
python scripts/headmesh_library.py sources.json NEW_LIBRARY
python scripts/verify_headmesh_library.py NEW_LIBRARY
cargo run -p studio-domain --example head_library_texture_audit --offline -- NEW_LIBRARY NEW_REPORT.json
cargo run -p studio-domain --example head_library_smoke --offline -- NEW_EXPORT_DIRECTORY
python -m unittest discover -s scripts/tests -p test_ps5_head_export.py -v
python -m unittest discover -s scripts/tests -p test_headmesh_library.py -v
```

The corpus verifier checks SQLite integrity, references, GLB fingerprints,
finite attributes, triangle bounds and DDS hashes. The separate decoder audit
uses Studio's DDS decoder. Their machine-readable results live with the
compiled library. Both passed: all 29,414 meshes, 69,905 appearance records,
and 30,222 texture hashes passed structural validation; all 30,222 DDS files
decoded successfully. All 31 source-catalog hashes also match their receipts.

The verified library was promoted at 06:13 local time. The previous library
is preserved at `graphical database/headmeshes-before-20261020`. The final
corpus passed the backend search/composed-export/texture/containment/collision
smoke check before promotion.

Other completed checks: 99 existing headmesh engine tests; 69 imported
converter tests run (66 passed, three skipped); three library compiler tests; three PS5
exporter tests; three Appearance UI tests; backend check, frontend production
build and desktop debug build. Full game runtime/visual qualification is
separate from these automated results.

## Remaining acceptance work

1. Calibrate old-to-modern UV correspondence and texture rebakes, including
   mouthbag, eyes, scalp and additional islands; test actual transformed heads.
2. Complete destination-specific native writers and rig/animation adapters
   across the requested platforms and titles. Preserve protected native data
   and validate changed packages through native re-decode and game testing.
3. Complete arbitrary external GLB/DDS import and native application from the
   Appearance panel through Studio's pending-change/recovery system.
4. Qualify hair/facial-hair replacement and old-game 3D facial-hair support.
   A preview composition alone does not establish that a target renderer or
   native container can install it.
5. Validate native material associations and authored normals where only raw
   channels are currently preserved, and complete patch-specific coverage.

The universal replacement outcome remains open. No conversion capability
matrix was promoted based only on library export, shared UVs, or a no-op pack.
