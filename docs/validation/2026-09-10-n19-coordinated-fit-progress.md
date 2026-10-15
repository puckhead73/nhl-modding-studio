# NHL19 coordinated eye-unit fit — implemented progress

User steering: the frozen fit was too close to Eichel, including eye placement. The current experiment therefore permits **coordinated translation of an eyeball, its head-surface socket/lid unit, and its lashes**, while retaining their internal geometry. This supersedes absolute eye-position freezing for this explicit experiment; it does not authorize independent eyeball nudges, arbitrary eyelid shrinkwrap, or a claim that native animation will follow the new position.

## Code and reproduction

Changed `crates/tdb-headmesh/examples/mcd_fit.rs` and added `crates/tdb-headmesh/examples/support/coordinated_fit.rs`. The existing conservative invocation remains available; coordinated work requires `--coordinated`.

```powershell
cargo run --release -p tdb-headmesh --example mcd_fit -- 'E:\tmp\nhl19-mogilny\out\eichel_from_game90.mcd' 'S:\Downloads\Mogilny Test\alex_mogilny.glb' 'H:\TEMP\nhl19-mogilny-coordinated-v2' --coordinated
```

Uses the same source alignment as H2b, preserving the comparison baseline. Donor and source eye targets now both use per-eye AABB midpoints; the older source triangle-soup centroid was affected by duplicate vertex weighting. This is a repeatable geometric statistic, not a measured anatomical pupil centre.

Harmonic displacement interpolation blends between translated eye units, the retained oral unit, and fixed shell anchors. It smooths displacement rather than the original surface. The exterior projection then acts around those units, with a shorter three-ring falloff. Side-facing surface points can be admitted with a nearby, similarly oriented source triangle; anatomical/coverage uncertainties still require review.

The former broad eye masks overlap each other at five bridge vertices and the oral mask at 25 transition vertices. The bridge overlaps blend; oral overlaps remain with the oral unit only after checking they are more than 25 mm from both eye centres. Core conflicts refuse the fit. IDs for the final units and editable surface are serialized in `coordinated.json`. These adjusted transition masks are experimental, not anatomically or game-qualified.

## Result

- 1,206 exterior vertices editable; source-distance RMS on that same set: **2.6904 mm donor → 0.6571 mm fitted**. This is a surface-fit metric, not an overall likeness/animation score.
- Right eye/socket translation magnitude **2.347 mm**, left **2.235 mm**. Packed eye AABB midpoint error: right 0 mm; left approximately 0.00000745 mm relative to the aligned source target.
- Core socket vertices receive exactly the same translation as their eye. Lashes translate according to their nearest donor eye. Eyes and lashes are now included in renders; previous H2b head-only images omitted those components.
- The source has no open mouth-window boundary after reference-only positional welding. **Mouth/cavity translation stayed zero.** All oral unit and shell anchors were verified unchanged after packing. Do not claim that the lip shape or mouth placement now matches Mogilny.
- The stronger unconstrained exterior projection produced 13 orientation reversals. Local backtracking reduced projection around 88 vertices; the retained candidate has **0 orientation reversals and 0 collapsed triangles** against the pristine donor. Those checks are not a full self-intersection/collision test.
- Native length, stream count, unit-position checks, structural reparse and writable-byte-span checks pass. Native normals/tangents and opaque animation data are still unchanged; this is a position-only offline diagnostic.
- Output SHA256: `61A5E42DDA9A7FCE4EE9AC909630AA41C60D7ECBF00F3DB6E7EE4F4125B6E315`.

Artifacts in `H:\TEMP\nhl19-mogilny-coordinated-v2`:

- `mogilny_COORDINATED_POSITION_ONLY.mcd`, `coordinated.json`, `fit_positions.json`, `run.log`.
- `front_comparison.png`, `profile_comparison.png`, `threequarter_comparison.png`, `cutaway_comparison.png`: columns are **Eichel / new fit / aligned Mogilny**.
- Individual `*_donor.png`, `*_fitted.png`, `*_source.png` views. The source uses a reference-only position weld for comparable smooth shading; native donor topology is unchanged.

Checks executed: `cargo test -p tdb-headmesh --example mcd_fit` (2 passed: rigid-unit/shell behavior, distinct coincident lip IDs); `cargo clippy -p tdb-headmesh --example mcd_fit -- -D warnings` (passed); targeted rustfmt on the two edited files; release command above (passed). Initial experiments that produced reversed triangles were not exported as native diagnostic assets. The earlier temporary folder is exploratory; use the `-v2` output above.

## Visual assessment and next work

Compared the front, profile and three-quarter renders. The cheek/jaw exterior fits more closely, and eye centres now align, but the **donor's eyelid aperture/brow shape and mouth anatomy still limit likeness**. Centre alignment alone is not aperture matching. Ear projection also warrants closer correspondence/penetration review before selecting a final fit.

Continue with explicit source eyelid/aperture and lip landmarks, local collision checks, and a texture preview. Any aperture reshaping must preserve the pocket as a coherent volume and be evaluated against its eyeball, not flatten the lid onto the source surface. A source lip landmark should come from inspected geometry/texture, not an invented boundary where the source is closed. Native facial binding, shading regeneration, GNF bake/mips, and runtime loading remain unresolved. No game file was deployed; no commit/push or capability promotion occurred.
