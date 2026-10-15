# NHL19 CC5 Demitra revision 2: successful import

On 2026-09-11, after revision 2 installation and production readback, the user
reported: **"This was a successful import"**. This is user-confirmed in-game
success for the supplied CC5 Demitra fixture over Marian Gaborik, Slovakia,
NHL19 PS4 CUSA11126 / APP_VER 01.60. The roster name was not changed.

## Successful baseline

Private package: `H:\TEMP\nhl19-demitra\revision2`.
Source: `S:\Downloads\Mogilny Test\Demitra\Demitra.Fbx`.
Source SHA256: `a1b281236cb4dfef9dbac12568c88cf43e092c9087a6cc045b02bae04ff6a637`.

| Decoded asset | SHA256 |
|---|---|
| base / patch93 MCD | f9d470f47a9897243c3f47991475cff5e118f0ebc902d5b4321c24e057ab5d21 |
| patch91 MCD | 172049048add52c211f6a0fde980f6ec3f4ce545ac86c52742e562e8a8d528a4 |
| head diffuse GNF | 0c2787311429220cab4e5c0e84fe65a88a643fd0cbe895169b46c61fe932a9e0 |

The installer completed all three archives. All four live decoded payloads
matched the manifest. Native pack/reparse, chunkzip, all 11 GNF mips and virtual
archive append checks passed. `installation.json` and `live_readback.json` retain
the deployment evidence; `repair_report.json` records geometry invariants.
Restore this revision before restoring its original Demitra or Mogilny baseline.

## Lessons carried forward

- Registered UV sampling transfers likeness while retaining native topology,
  UVs and rig data; preserve source per-corner UV seams.
- UV equality does not permit collapsing inner/outer lip layers. Handle native
  mouthbag, eyes and embedded lashes explicitly.
- The first neck projection distorted the silhouette. Revision 2 preserves 484
  native lower-neck/chest vertices exactly and blends higher into the fitted head.
- Posterior scalp correspondence needs separate handling. Coincident seam positions
  alone do not guarantee smooth shading.
- Radial scalp fitting initially distorted an ear. Lock both ears explicitly:
  259 records per side, zero position delta against pre-repair Demitra; retain
  core ear normals and transition surrounding shading smoothly.
- Hide all three native hair layers inside the revised scalp; measured minimum
  clearance exceeds 7.7 mm. Crown matches the native donor.
- Check both profiles and rear solid views, not only textured front-face likeness.
  All six output components have zero newly collapsed triangles.

The only imported CC5 texture was the head diffuse, rebaked to native 1K BC1.
Native oral atlas islands and material bindings remain. CC5 normal and other PBR
maps were not imported; regenerated vertex normals/tangents are separate from
normal textures. Further material experiments should isolate one channel while
holding this successful geometry fixed.

No detailed per-animation checklist or new runtime screenshot/log bundle accompanied
the success report. This does not certify every CC5 variant or NHL title/platform.
The reproducible workflow and next-test requirements are in
[CC5-to-NHL19 transfer](../CC5_TO_NHL19_HEAD_TRANSFER.md).
