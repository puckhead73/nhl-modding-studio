# Head preview and Ignite MCD repairs — 20 October 2026

User examples: NHL19 Jaromir Jagr and Mike Bossy; missing face textures in
NHL16/NHL19; accented names not matching ASCII library slugs.

## Measured failures and fixes

The MCD reader previously assigned the first unused index buffer whose indices
fit a vertex count. This does not establish ownership. Repeated VIEW resources
and shared eye index buffers made that heuristic assign hair topology to heads.

The repaired reader follows each VIEW's explicit TLST references:

| TLST-relative offset | Meaning |
| --- | --- |
| +12 | Triangle count |
| +16 | VFMT ordinal, including split-stream descriptors in the ordinal table |
| +20 | Index-binding count (measured value 1) |
| +24 | Vertex-stream count |
| +28 onward | Vertex-stream ordinals followed by the index-buffer ordinal |

Counts, declared strides, stream bindings and index bounds are checked together.
Malformed explicit references fail rather than falling back to a different,
in-range buffer. The legacy heuristic remains only for containers with no TLST.

The old name scanner also read numeric GEOM resource headers in later VIEWs as
named tail records. A following VFMT tag plus its length could become `TMFVk`,
shifting subsequent names and mixing head/hair roles. Named records now require
a GEO revision marker. Exact duplicate base geometry across VIEWs is omitted;
distinct geometry and native split records remain intact.

For Jagr, the old head asset contained 1,789 edges longer than 10 cm in spurious
extra surfaces; Bossy contained 1,808. Corrected head subsets contain zero such
edges and retain the 6,014-triangle head plus two 100-triangle eyes. Hair remains
separate. The measurements are in
`local/mcd-fix-20261020/topology-comparison.json`.

Some Ignite face maps, including NHL16 Nik Antropov, store face RGB in a `_coeff`
texture rather than `_cm`. The decoded source was inspected. Preview selection
now uses this map when the corresponding part has no CM, only for Ignite titles.
Frostbite dual-coefficient/specular maps are not substituted as face colors.

Name searches fold Unicode combining marks and explicit Latin letters such as
ø/ł/æ before matching. `Jaromír Jágr` and `Jaromir Jagr` resolve identically.
The current mesh index's names and identity slugs are all ASCII.

## UI

- Texture thumbnails are in a native disclosure section, initially collapsed.
- The selector gathers bounded API batches into one newest-first scroll list;
  Previous/Next controls are removed.
- Regression coverage includes more than 100 results, accented queries,
  coefficient fallback, ordinary color-map preference and Frostbite exclusion.

## Validation and data handling

All 1,631 selected NHL16/NHL19 native MCD sources re-exported successfully.
Corrected catalogs preserve texture/source provenance and feed the exact-dedup
library compiler. Original game files were not modified. The audit directory
contains requests, per-file results, corrected catalogs and rebuild logs.

Nine MCD tests passed, including a synthetic case where two index buffers are
both in range but TLST explicitly binds them in reversed order, plus malformed
reference rejection. Thirteen focused frontend tests passed. All 78 cross-game
native sample reads passed without errors. Structural corpus
validation and publication receipts accompany the rebuilt library.

Published the verified library at `graphical database/headmeshes`. All 28,643
GLBs and 30,222 DDS files passed integrity/fingerprint checks with zero errors;
68,816 source appearances remain. The DDS content-addressed set is unchanged,
so its prior successful full Studio decoding audit is retained with that evidence.
Studio search, source selection, composed export, texture decoding, containment
and collision checks passed against the rebuilt library. Jagr and Bossy each
retain the correct face and both eye submeshes, with NHL19 provenance retained
after exact deduplication against matching other-game geometry.

The previous library is preserved at
`graphical database/headmeshes-before-mcd-fix-20261020`. Publication and example
receipts are in `local/mcd-fix-20261020/publication.json` and the live library's
`_metadata/repair-publication.json`. UI query/preview keys were advanced to avoid
retaining stale geometry after replacement. The running desktop executable was
not replaced; native extraction changes require its next rebuild/restart, while
the repaired library and development frontend are available immediately.

This repairs native extraction and previews. It does not qualify cross-game
native installation, skeleton retargeting or the paused overnight assignment.
