# PS4 EA "RSF" mesh container (`.mcd` / `.skel` / `.rsf`)

The mesh/geometry format for the PS4 "Kettle" asset stack (NHL 15 → 21). Player
heads, hair, eyes, team logos, and stadium props are `.mcd` files; skeletons are
`.skel`; compiled shaders are `.rsf` — all share the same RSF container.

Status: **decode verified** against real NHL 15 **and NHL 16** files
(`crates/tdb-rx2/src/mcd.rs`). NHL 16 introduced two changes the decoder now
handles — a seven-field VFMT descriptor and split-stream (position / attribute)
submeshes; see [Version drift](#version-drift-nhl-15--16). Skeleton/bind-pose
parse and morph-target deltas are decoded structurally but not yet consumed (see
[Open items](#open-items)). All values little-endian (PS4).

This is the PS4 counterpart to the Xbox 360 / PS3 RenderWare `\x89RW4xb2` reader
(`crates/tdb-rx2/src/playermesh.rs`). Both decoders emit the same
`PlayerAsset` model, so the shared `write_glb` exporter and the `head_preview`
viewer path work for `.mcd` unchanged — only the binary decode differs.

## Container header

```
0x00  "RSF\0"   magic (52 53 46 00)
0x04  u32       version/flags = 0
0x08  u32       0x30  — a constant format tag, NOT a directory offset
0x0C  u32       0
0x10  chunk stream …
```

There is **no central directory**. The body is a sequential/nested stream of
chunks whose tags are stored as **reversed (little-endian) FourCCs**:

| on-disk | tag  | meaning                                   |
| ------- | ---- | ----------------------------------------- |
| `MOEG`  | GEOM | one geometry block (a submesh / morph)    |
| `TMFV`  | VFMT | self-describing vertex format (ASCII)     |
| `MRTS`  | STRM | interleaved vertex stream                 |
| `XDNI`  | INDX | index buffer                              |
| `HSEM`  | MESH | submesh record (tail)                     |
| `TSLT`  | TLST | primitive/triangle count (tail)           |
| `TIEW`  | VIEW | LOD/view container (player heads)         |
| `FRGS`  | —    | scene-graph root (`.skel` only)           |

Chunk bodies are **byte-packed** (not 4-byte aligned). Each geometry sub-chunk is
immediately preceded by a `01 00 00 00` child-count word.

Two submesh layouts occur:
- **Interleaved** (logos, simple meshes): `VFMT → STRM → INDX` per submesh.
- **Struct-of-arrays** (player heads, under a `TIEW`): all VFMTs clustered up
  front, then all STRMs, then all INDX. Multi-LOD heads repeat the whole set once
  per LOD.

Because of the two layouts, the decoder walks a **STRM cursor** in file order and
matches each STRM to a VFMT by its lead (stream-0) **stride**, not by adjacency;
VFMTs are taken forward-first with reuse (the two eye submeshes share one format).
A VFMT that names more than one stream (NHL 16 split streams) then consumes one
further consecutive STRM per additional stream. Each INDX is assigned to whichever
geometry it validates against (every index `< vertex_count`).

## VFMT — vertex format (verified)

`TMFV | u32 strlen | ASCII` (no NUL inside the counted length). The string is
space-separated attributes, each of colon-delimited fields. The **field count is
version-dependent** (see [Version drift](#version-drift-nhl-15--16)):

```
NHL 15 (5 fields):  name : byteOffset(hex) : streamIdx(hex) : 0001 : countType
NHL 16 (7 fields):  name : byteOffset(hex) : streamIdx(hex) : 0001 : 0000 : 00 : countType
```

e.g. NHL 15 `p0:00:00:0001:3f32 n0:0C:00:0001:4s16n t0:14:00:0001:2f16`
     NHL 16 `p0:00:00:0001:0000:00:3f32 n0:0C:00:0001:0000:00:4f16 …`

The decoder reads **name** from field 0, **byteOffset** from field 1, **streamIdx**
from field 2, and **countType** from the **last** field (version-agnostic).

- **name** → semantic: `p0`=POSITION, `n0`=NORMAL, `g0`=TANGENT,
  `t0`/`t1`=TEXCOORD0/1, `t7`=BLENDINDICES, `t8`=BLENDWEIGHTS. (There are no
  `i0`/`w0` tokens — skinning rides on `t7`/`t8`.)
- **byteOffset** — hex offset of the attribute within **its stream's** vertex
  record (verified).
- **streamIdx** — which vertex buffer the attribute is sourced from. `00` for a
  fully-interleaved submesh; `01` for the attribute half of a split-stream one.
- **countType** — a component-count digit + component format: `f32`, `f16`,
  `s16n` (int16 normalized, `/32767`), `u8n` (uint8; raw for indices, `/255` for
  weights). Per-stream stride = `max(offset + count·size)` over that stream's attrs.

Observed layouts:

| kind            | stride | attributes                                              |
| --------------- | ------ | ------------------------------------------------------- |
| logo/decal      | 24     | `p0` 3f32 · `n0` 4s16n · `t0` 2f16                       |
| head base       | 32     | `p0` 3f32 · `n0` 4f16 · `g0` 4f16 · `t0` 2f16            |
| head + uv2      | 36     | …+ `t1` 2f16                                             |
| head skinned    | 40     | …+ `t7` 4u8n (4 bone indices) · `t8` 4u8n (4 weights)    |
| eye             | 36     | `p0` 3f32 · `n0` 4f16 · `g0` 4f16 · `t0` 4f16            |
| morph delta     | 16     | one of `p0` 4f32 / `t7` 4f32 / `t8` 4f32 (single attr)   |

Skinning is thus **4 influences per vertex**; weights are `u8/255`.

## Version drift (NHL 15 → 16)

Verified by hex-walking `bobby_ryan.mcd` (NHL 16, CUSA01909). Two changes; the
decoder handles both and NHL 15 still parses unchanged:

1. **Seven-field VFMT.** NHL 16 inserts two fields (`…:0000:00:`) before the
   `countType`, so the component type is no longer at a fixed index — read it from
   the last field. This alone was the `NoGeometry` failure: with `countType`
   pinned to field 4, every attribute failed to parse and no VFMT survived.
2. **Split-stream submeshes.** NHL 16 splits the eyes into a **position stream**
   (stride 12, `p0` only) and a separate **attribute stream** (stride 24,
   `n0`/`g0`/`t0`/`t1`), tied together by the per-attribute `streamIdx`. The
   combined VFMT names both streams; the two STRMs (equal vertex count) sit
   consecutively. NHL 15 was always single-stream/fully-interleaved.

A real NHL 16 head decodes to four submeshes: a stride-40 skinned face (3285
verts), a stride-32 block (677 verts), and two split-stream eyes (66 verts each).

## STRM — vertex stream (verified)

```
tag  "MRTS"
+0x04  u8      (pad)
+0x05  u32     = 2   (stream index? constant)
+0x09  u32     vertex count
+0x0D  u32     stride (bytes)
+0x11  u32     0
+0x15  interleaved vertex bytes …
```

Deinterleave per the paired VFMT. Verified: `logos_boston` vertex 0 decodes to
position ≈ (0.0120, −0.5103, 0.0235), unit-length normal, UV in [0,1].

## INDX — index buffer (verified)

```
tag  "XDNI"
+0x09  u32     index count
+0x0D  ASCII   reversed token: "61DI"→ID16 (u16) · "23DI"→ID32 (u32)
              index data begins just after the token (variable 1–4 byte pad)
```

Primitive is a **triangle list** (not a strip). Baked-on-grid logos carry ~36 %
degenerate (collapsed) triangles — expected, filtered downstream as needed. The
decoder locates the data start by validating that every index is `< vertex_count`.

## Tail — submesh names + AABB (verified)

After all geometry, a tail block carries per submesh: `HSEM`(MESH) → `TSLT`(TLST,
u32 triangle count) → `MOEG` marker → **NUL-terminated ASCII name** → **6-float
AABB** (min.xyz, max.xyz). Names are Maya shape nodes: `logo_bostonShape`,
`geo_headShape`, `geo_haircapShape`, `hair_cardsShape`, `geo_leyeShape`,
`geo_reyeShape`. These names drive the preview's texture binding and blend mode
(the same name-keyword heuristics the RW4 path uses). Names are associated to
geometry in file order; a name containing a lowercase letter distinguishes a real
name from an all-caps chunk tag.

## `.skel` — skeleton (structure verified, framing partial)

`RSF\0` → `FRGS` → a typed scene-graph of length-prefixed strings. Node =
`u32 nameLen | typeName` with types `SGNodeGroup`, `SGNodeTransform`,
`SGNodeLocator`, `SGNodeJoint`; hierarchy is by nesting (group child counts). Each
`SGNodeJoint` holds `translate`/`rotate`(euler radians)/`scale` as 3×f32. Bone
names are fully recoverable (`Spine1`, `LeftShoulder`, finger chains, facial
joints). The `t7` blend indices reference `.skel` joint order.

## Morph targets / blend shapes

Head meshes carry ~60 named facial morphs (`RightEye`, `MiddleNose`, `ChinPuff`,
`UpperLip`, …) plus additive combos (`swelling_nose_Additive_swelling_geo_headShape`).
Deltas are stored as standalone single-attribute streams (`p0:4f32` position
deltas, with `t7:4f32`/`t8:4f32`), keyed by morph name in the tail. The base
decoder skips these (they are not full submeshes).

## Open items

- **Skeleton → GLB joints.** `.skel` TRS parse is understood; wiring it into a
  `BonePalette` (inverse-bind matrices, `write_glb` joints/weights) is a
  follow-up. Today `.mcd` decodes rigid (empty palette) — correct geometry, no
  rig. Blend indices/weights are still decoded on the vertices.
- **Multi-LOD selection.** Player heads may carry several LODs; the decoder
  currently returns all of them. A future pass can keep only LOD0 via the `TIEW`
  view table.
- **Index reuse across submeshes.** Paired eyes with identical topology may share
  one INDX; the greedy validator assigns it to the first eye, leaving the second
  without triangles. Low impact (geometry is intact); revisit with the `TIEW`
  table.
- **Write-back.** Read-only today. An RSF re-emit (for mesh replacement) is a
  separate effort, mirroring the RW4 `container.rs`/`pack.rs` write path.

## Related

- Decoder: `crates/tdb-rx2/src/mcd.rs`
- Shared model + GLB writer: `crates/tdb-rx2/src/playermesh.rs`
- Dispatch (`.mcd` vs `.rx2`): `crates/studio-domain/src/assets.rs` `parse_head_mesh`
- Preview: `crates/studio-domain/src/head_preview.rs`
- Texture companion (PS4 GNF): see the GNF texture plan.
