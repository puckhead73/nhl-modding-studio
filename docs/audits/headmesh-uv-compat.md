# Head-UV atlas compatibility across titles (HM-S1)

**Status:** first measurement pass complete (2026-08-06). Classes are **provisional** —
sample sizes below; NHL 11–14 X360 not yet measured (builds present but heads still
packed in `.big` archives).

## Why

The Appearance Studio's headmesh conversion matrix must know, per (title, platform),
whether head-texture atlas layouts are interchangeable. The working hypothesis ("NHL 11
through NHL 24 share one head UV map; pre-NHL11 differs") had **no recorded evidence** in
this repo or the prior app's material — so it is measured here, not assumed
(CLAUDE.md rule 3).

## Method

`headmesh uv-fingerprint` (crates/tdb-headmesh/src/uvfp.rs — the spike tool is the
permanent tool) decodes the head-role submesh of each input (`.rx2` via
`tdb_rx2::playermesh`, `.mcd` via `tdb_rx2::mcd`, `.glb` via `tdb_headmesh::glb`) and
reports:

- counts (vertices / triangles / unique quantized UVs) and UV bbox;
- an order-independent FNV-1a-64 hash of the UV multiset quantized to a 1/2048 grid
  (equal ⇒ identical atlas up to vertex reordering);
- a 256² **UV coverage raster** compared two ways: raw IoU, and `agree±2` — the worse
  direction of "fraction of set texels within 2 texels of the other mask". Raw IoU
  saturates (1-texel storage jitter between u16 and f32 UVs already costs ~7% on a head
  atlas); `agree±2` reads ≈ 1.0 for same-layout-with-jitter while moved or re-cut
  islands still fall out.

## Samples (all real, locally held game data — none committed)

| Game key | Files | Source tree |
|---|---|---|
| nhllegacy-xbox360 | player_67, player_71, player_20500, player_1151, player_671, player_676 | `Game Builds/XBOX/01_Research-BAK/rendering/player` (pristine cacherender extraction) |
| nhl15-xbox360 | player_10, player_1000 | `Pre-Legacy Games/NHL 15/NHL 15 (USA)/rendering/player` |
| nhl10-xbox360 | player_10, player_1000 | `Pre-Legacy Games/NHL 10/NHL 10 (USA)/rendering/player` |
| nhl24 | head_aaron_ekblad_mesh.glb, head_adam_fox_mesh.glb | `Reference Data/NHL24/.../head/licensed` (Frosty exports) |
| nhl15-ps4 | aaron_ekblad.mcd, ales_hemsky.mcd | ShadPS4 CUSA00561 extracted kettle tree |
| nhl16-ps4 | aaron_ekblad.mcd | ShadPS4 CUSA01909 extracted kettle tree |

## Findings

### Within-generation

- **Legacy X360 ≡ NHL 15 X360 — identical.** Bespoke heads on both share one exact UV
  multiset (hash `bf71204a8eab859f`, 2751 v / 5098 t / 2609 unique UVs); `agree±2 =
  1.0000`, raw IoU `1.0000`. The 2623-vert generic donor (player_67) has a different
  multiset (fewer duplicated verts) but the **same 2609 unique UVs and the same
  atlas** (`agree±2 = 1.0000` vs the bespoke heads).
- **PS4 15 ≡ PS4 16.** Same-player `.mcd` files are UV-identical across CUSA00561 and
  CUSA01909 (hash `abe6221dae083590`).
- **NHL24 ≡ PS4 Kettle.** Same player (Ekblad), same topology family (3157 v / 6014 t);
  multiset hashes differ only at quantization level (3141 vs 3143 unique UVs);
  `agree±2 = 0.9871`. One class: **`kettle-fb`**.

### Cross-generation

| Pair | agree±2 | raw IoU | verdict |
|---|---|---|---|
| Legacy X360 ↔ NHL 15 X360 | 1.0000 | 1.0000 | identical (`x360-rw-v2`) |
| NHL24 ↔ PS4 15 (same player) | 0.9871 | 0.9288 | same class (`kettle-fb`) |
| NHL 10 ↔ Legacy/15 X360 | 0.9688 | 0.9585 | close but distinct (`x360-rw-v1`) |
| NHL 10 ↔ kettle-fb | 0.9731–0.9758 | 0.92–0.96 | close but distinct |
| Legacy/15 X360 ↔ NHL24 | 0.9339 | 0.8908 | related, **not interchangeable** |
| Legacy/15 X360 ↔ PS4 15 | 0.9437 | 0.9420 | related, **not interchangeable** |

### What this means for the hypothesis

- "NHL 11–24 share one head UV map" is **not confirmed as identity**. The X360
  RenderWare generation (Legacy/15 measured; 11–14 pending) and the Kettle/Frostbite
  generation (PS4 15/16, NHL24) use **similar but distinct** head atlases (`agree±2 ≈
  0.93–0.94` — every generation lays the face out the same general way, but islands
  differ beyond storage jitter).
- This does **not** block cross-class conversion: the proven NHL24→Legacy pipeline
  carries the source's UVs *and* the source's texture into the target container as a
  unit. UV-class equality gates **texture-only transplants** (e.g. putting a
  Legacy-authored face texture on an NHL24-converted head), and that gate is now data
  (`defs/appearance/head-uv-classes.toml` + matrix `uv_class_*` fields).
- "Pre-NHL11 differs" is **directionally supported**: NHL 10 has its own topology
  (2657 v / 5032 t) and its atlas reads distinct from both neighbours, though it is far
  closer to everything than expected (0.96–0.97) — a visual wireframe pass should
  decide whether the difference is a re-cut or a refinement.

## Classes assigned (defs/appearance/head-uv-classes.toml)

- `x360-rw-v2` — nhllegacy-xbox360, nhl15-xbox360
- `x360-rw-v1` — nhl10-xbox360
- `kettle-fb` — nhl24, nhl15-ps4, nhl16-ps4
- everything else `unmeasured`

## Caveats + follow-ups

1. **Small samples** (2–6 heads per title). Widen per title before treating classes as
   settled; the fingerprints JSON of this run is reproducible with the command below.
2. **NHL 11–14 X360 unmeasured** — builds on hand still carry heads inside `.big`
   archives (`cacherender.big`); extract (tdb-archive-big) then fingerprint. This is the
   segment that decides whether `x360-rw-v2` extends back to NHL 11.
3. **NHL 07/08/09 unmeasured.**
4. **Visual wireframe pass pending** — `agree±2` says "how much", a rendered UV overlay
   says "where"; needed before any texture-transplant feature ships across the
   0.93–0.97 band.
5. GLB primitives from Frosty exports carry no material names (primitive name falls back
   to the mesh name); the tool fingerprints the largest head-role primitive per file,
   which excludes the 198-vert eye and 64-vert lash prims.
6. Eye rasters are a separate story on RW targets (shared 128×128 `eyes_<n>_cm`, donor
   eyeballs kept verbatim) and are out of scope for head-atlas classing.

## Reproduce

```
headmesh uv-fingerprint <head files...> --json out.json
```

Run of record: 2026-08-06, files listed above, fingerprints at
`uvfp-run1.json`/`uvfp-run2.json` (session scratchpad; re-run to regenerate — inputs are
local game data and stay out of the repo per the EA data policy).
