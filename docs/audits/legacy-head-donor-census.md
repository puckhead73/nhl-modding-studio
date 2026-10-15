# Legacy head palette donor-usage census

**Status:** first in-repo measurement (2026-08-06), 6 vanilla heads. Confirms the
prior-effort exclusion set with one contested slot recorded below.

## Why

Path A remap targets must stay inside the set of palette slots that vanilla assets of
the target class actually reference: a slot no vanilla head weights holds a **garbage
matrix**, and sending remapped mass there shreds the mesh in-game (the historical
"palette-5 Neck" failure). Name correlation alone cannot see this — usage can. The
census is a rerunnable engine stage (`headmesh census`, `tdb_headmesh::census`), not a
one-off script.

## Run of record

`headmesh census` over six pristine Legacy X360 heads from the cacherender extraction
(`player_67, player_71, player_20500, player_1151, player_671, player_676`):

- **39 palette slots carry weight** (of 122). Dominant mass: slot 8 Head (12064), slot
  16 Jaw (2574), slot 4 Spine3 (1168), then the facial-region slots 17–48 as mapped in
  `defs/appearance/facial-regions.toml`; slots 46–48/79 carry real neck-family mass
  despite arm-flavored bind names — palette naming is head-context-specific, as the
  prior effort warned.
- **Excluded slots 20 (Tongue), 49 (LeftArm), 80 (RightArm): zero mass.** Exclusions
  confirmed against usage.
- **Contested: slot 5 ("Neck", excluded) carries 8.61 mass (~0.04% of total)** — a
  handful of stray vertex references in this sample, against 353 mass on the preferred
  Neck slot 6. The exclusion stays (routing remap mass to 5 remains wrong; the slot's
  matrix is not part of the animated neck chain), but the prior blanket claim "no
  vanilla head references slot 5" is too strong. `Census::contested_exclusions` surfaces
  this automatically; keep an eye on it as the sample widens.

Prior-effort baseline for comparison: 37 used slots over 5 donors (different sample).
The 39-vs-37 delta is sample composition, not drift.

## Consequences

- `defs/appearance/legacy-bone-names.toml` keeps `exclude` on 5/20/49/80 and `prefer`
  on 6 (Neck) — all consistent with this census.
- Remap targeting (`tdb_headmesh::remap`) resolves through the exclusion-aware
  inversion, so no override or direct match can land on an excluded slot.
- Widen the census (goalie heads, the full bespoke set) before M2c adds new
  bone-override entries; each new override's target must be census-used.

## Reproduce

```
headmesh census <vanilla player_*.rx2 files...>
```
