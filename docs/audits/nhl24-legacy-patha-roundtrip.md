# NHL24 → Legacy Path A conversion — round-trip verification (M2)

**Status:** `roundtrip_verified` (2026-08-06). In-game verification (`game_verified`) is
a separate manual gate and has **not** run yet.

## What ran

The full Path A pipeline (`tdb-headmesh`), on real locally-held data (none committed):

```
headmesh convert --source nhl24 --target nhllegacy-xbox360 --allow-unproven
  --head-glb  .../licensed/aaron_ekblad/components/head_aaron_ekblad_mesh.glb
  --weights   .../head_aaron_ekblad_mesh.weights.json
  --donor .../01_Research-BAK/rendering/player/player_67.rx2
  --donor-texlib .../texlib_67.rx2
  --head-texture .../textures/head_aaron_ekblad_color.dds
  [--hair-texture .../textures/hair_aaron_ekblad_color.dds]
  --head-id 30101 --out <dir> --collapse-haircap
```

Stages: GLB+sidecar extract (sections joined by vertex count, influences routed
`boneList`-space → skeleton space, `& 0x7FFF`) → 433→122 weight remap
(override → direct → ancestor-climb → drop; top-4 renormalized) → RW pack
(`tdb_rx2::pack::substitute_submesh` — first consumer; recomputed area-weighted
normals; typed GL→STORED UV flip; BLENDINDICES = palette × 3; haircap collapsed to a
hidden degenerate triangle) → texlib build (DDS decode → bilinear resize → full mip
chain to 16×16 → texpresso DXT1/DXT3 → `replace_with_resize`) → distortion QA battery.

## Gates passed

1. **Strict re-parse**: the packed `player_30101.rx2` round-trips through
   `parse_player_mesh` (strict skin policy) — enforced inside `pack_head_into_donor`,
   and re-verified from disk by the fingerprint tool.
2. **Texlib exact shape**: output is exactly **331 776 bytes** with head (512² DXT1)
   alone and with head + hair (256² DXT3) both replaced.
3. **UV carry-through cross-validation (the strongest signal):** the converted head's
   quantized-UV multiset hash is `abe6221dae083590` — **bit-identical to EA's own PS4
   Kettle build of the same player** (`CUSA00561 .../player/aaron_ekblad.mcd`, see
   `headmesh-uv-compat.md`). Our GL→STORED flip + u16 quantization lands exactly where
   EA's console pipeline landed the same source asset.
4. **Palette-usage containment**: a census of the converted head shows 27 slots, all
   inside the vanilla donor-used set (`legacy-head-donor-census.md`); **zero mass on the
   excluded garbage slots 5/20/49/80** — the historical shred/garbage-matrix failure
   modes are structurally excluded.
5. **Id-scheme validation**: head id checked against the target scheme (`rw-ppad`:
   below the 60000 pool floor) before any work; install plan emitted as data
   (`install-plan.json`), no game-tree writes.

## QA scorecard (first run, Ekblad)

| Metric | Value |
|---|---|
| Facial RMS / max | **1.89 mm / 13.42 mm** → verdict **Caution** |
| Retarget RMS / max | 5.71 mm / 28.80 mm |
| DOF-loss RMS | 2.14 mm |
| Battery | 160 poses (79 bones ≥ 0.5 mass × 2 axes @ 15°) |
| Remap | collapsed 818.3 mass · unmapped 0.0 · truncated 3.2 (79 verts) |

Interpretation: the dominant error source is the **known facial micro-bone collapse**
(818 weight mass ancestor-collapsed, mostly onto Head) — the exact defect the
bone-override workstream (`defs/appearance/bone-overrides.toml`, milestone M2c loop)
exists to burn down. Numbers are from **this** battery (single-bone ±15° X/Z poses, LBS)
and are comparable run-to-run, not to the prior tool's absolute baseline; the verdict
thresholds inherit the prior baseline's scale and may need recalibration once a few
heads have been judged in-game.

## Submesh alignment (added 2026-08-06, after first in-preview feedback)

Donor eyeballs at donor positions read **sunken** on the NHL24 head — the source
surface's eyelids are authored around the *source's* eye centers. Fix is measured, not
hand-tuned (`crates/tdb-headmesh/src/placement.rs`):

- source eye centers measured from the component's own `mat_eyes` geometry (left/right
  cluster centroids); donor `eyel_`/`eyer_` **translated onto them** post-pack via the
  same-count position patch (translation only — gaze rides the per-draw register slot,
  eye-raster UVs untouched), then re-parsed and verified on-target within 1.5 mm
  (fp16 slack) — a drifted patch is a hard error;
- donor `mouthbag_` translated by the **mouth-opening boundary-loop centroid delta**
  (source head vs donor head) — this replaces the prior effort's per-asset
  `--mouth-back-mm`-style levers with measurement.

Ekblad run: `eyel_` moved 10.27 mm, `eyer_` 6.67 mm, mouthbag 3.99 mm; all gates still
pass. Default-on; `--keep-donor-eyes` keeps donor placement for A/B checks. Shift
magnitudes are recorded in `qa.json` per run.

## Known limitations at this stage

- **Bald conversions**: hair-card grafting (base/medium/long variants) is not built —
  the donor haircap is collapsed and no cards ship. Hair variants are follow-up scope.
- Eyes/mouthbag are the donor's *geometry* (by design — the eye rule), now at
  source-measured positions; lashes are dropped (Legacy has no lash slot).
- Eyeball *size* is not scaled to the source (radius mismatch is not yet measured or
  corrected — revisit if in-game reads wrong).
- Only the `player_67` donor profile is exercised; `player_71` (long-hair) unmeasured.
- QA verdict "Caution" is expected until M2c overrides land.

## Matrix promotion

`nhl24 → nhllegacy-xbox360` promoted **unproven → roundtrip_verified** on this
evidence. `game_verified` requires the manual gate: install `player_<id>.rx2` +
`texlib_<id>.rx2` into a working copy's `rendering/player/`, set `ubPc.pPaD == ubPc.lOnr
== <id>` on a player, and verify the head renders + animates in-game.

## Reproduce

Command above; any NHL24 licensed component + a pristine donor pair works. Outputs:
`player_<id>.rx2`, `texlib_<id>.rx2`, `qa.json`, `install-plan.json`.
