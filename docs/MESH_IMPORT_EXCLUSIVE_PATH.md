# THE EXCLUSIVE PATH — importing an arbitrary head MESH into NHL Legacy

**Scope update, 2026-09-10:** For NHL heads sharing an anatomical UV template,
start with [shared-UV template transfer](MESH_UV_TEMPLATE_TRANSFER.md), following
the successful Mogilny-to-NHL19 game test. The shrinkwrap and frozen-eye rules
below describe the historical arbitrary-head/FaceBuilder fallback without verified
UV correspondence. They are not blanket constraints on shared-template conversion.
Both methods retain the destination's native UVs and topology.

**This is the only validated way to import an arbitrary head mesh (FaceBuilder /
sculpt / scan — a mesh with NO Legacy rigging) into NHL Legacy. Use this path
exactly. Other approaches (transferring the target's UVs, flipping V, rigid
weights, skipping alignment) have repeatedly produced offset/upside-down/garbage
results — they are WRONG. Validated in-game 2026-06-25 (the "suckow_mike" head).**

## What it does (and why)

We do NOT ship the target mesh. We **morph a Legacy donor head onto the target's
shape** with a Blender shrinkwrap, then bake the new positions back into the
donor's `head_` slot. The output therefore keeps, by construction:
- the donor's **Legacy topology** (same vertex order),
- the donor's **animated weights** (the face rig — jaw/lips/eyes), and
- the donor's **default Legacy head UV layout**.

Only the vertex POSITIONS change (to the target's shape). That is the whole
trick — everything that has to stay Legacy stays Legacy; only the shape moves.

**Normals are recomputed from the morphed surface** (`reshape-head` does this
automatically). The donor's stored normals describe the OLD shape, so they must
be regenerated — area-weighted smooth normals over the donor winding, verified to
match the donor's in-game orientation (mean dot 0.98 vs stored). Earlier builds
passed empty normals → every normal collapsed to flat `[0,0,1]` → **banded /
streaky head shading in-game** (the classic symptom). If you see streaky face
shadowing, the normals were zeroed; re-run with a build that recomputes them.

## ☢️ THE UV RULE (this is what every failed attempt got wrong)

1. **KEEP the donor's default Legacy head UV layout.** Pass `--keep-uv` to
   `reshape-head`. Do **NOT** transfer the target mesh's UVs onto the morph —
   that offsets the texture (the target's UV islands ≠ the Legacy template).
2. **The head texture MUST be authored for the default Legacy head UV layout.**
   (e.g. `suckow_mike_07_12_1987_head_color.dds` was.)
3. **With `--keep-uv`, do NOT flip V.** The donor UVs are already in the rx2's
   native (Xbox, top-left) orientation. Flipping makes it upside-down.
   - (`--flip-v` is ONLY for the non-exclusive case of transferring the target's
     own UVs, which we do NOT do here. Don't.)

## ☢️ THE EYE RULE — FREEZE the eye region (locked 2026-06-25, in-game verified)

The **player_67 donor `head_` has only TWO holes: the neck perimeter and the
mouth.** Its **eyes and nose are CLOSED surfaces**, and the eye area is a
carefully **rigged eyelid POCKET** — the inner upper-lid folds back to form the
socket cavity the separate eyeball submesh (`eyel_`/`eyer_`, 66 verts, uniform
Head-bone) shows through and animates against. **This pocket is the single most
fragile thing in the whole import. Do NOT let the shrinkwrap touch it.**

If you wrap the eye region onto the target, the inner upper-lid fold gets pulled
down/flat onto the target surface → the pocket COLLAPSES. In-game the upper lid
then clips through the eyeball no matter where you place it; pushing the eyeball
forward to clear the collapsed lid only makes it bulge and clip the cheeks.
(Filling the target's eye holes + subdividing helps the *transition ring* but
does NOT save the pocket — only freezing does.)

**THE FIX (do all three):**
1. **FREEZE the donor eye region.** In the morph vertex group, force weight 0 for
   any donor vert within `R_IN≈16 mm` of either donor eyeball center
   (`(0.0341,1.8357,0.0907)`, `(−0.0312,1.8345,0.0910)`), ramping to full morph by
   `R_OUT≈28 mm`. Verify: donor-vs-morph displacement must be **0.00 mm inside
   R_IN**. This keeps the donor's working lids/pockets verbatim; only the
   surrounding face (brow, cheeks, nose, forehead) wraps to the target.
2. **FILL the target's eye holes + Catmull-Clark SUBDIVIDE it (×2).** Coarse
   FaceBuilder targets (`FBHead` ~1386 faces, open eye holes) otherwise facet the
   transition ring and suck its verts toward the hole rims. Leave the target's
   neck + mouth holes open (they match the donor's).
3. **Eyeballs stay at DONOR position — do NOT nudge them.** The frozen donor
   socket + the donor eyeball are a self-consistent unit that already works
   in-game. Reshape from the pristine donor and ship `eyel_`/`eyer_` untouched.
   (Nudging eyeballs only "works" as a hack against a collapsed pocket — fix the
   pocket instead.) `nudge-slot` remains available for genuine per-eye tweaks.

Reference run: `wrap5.py` (eye-hole fill + subdiv + boundary-freeze + **eye-region
freeze** + wrap), then `reshape-head --keep-uv --collapse-haircap` from the
pristine `player_67.rx2`. No eyeball nudge.

## Tools / prerequisites

- `cargo build --release --bin rigaudit --bin export_submesh_obj`
- Blender 4.5 (`C:\Program Files\Blender Foundation\Blender 4.5\blender.exe`)
- A clean Legacy donor `player_*.rx2` (a 5-submesh head). **player_67** is the
  generic donor used here — its `head_` has 2623 verts and the standard Legacy
  head UV template. Path used: `…/01_Research-BAK/rendering/player/player_67.rx2`
  (+ `texlib_67.rx2`).

## Coordinate frame (verified)

Everything is **Y-up, face +Z**, in the Legacy head frame:
`X[-0.14,0.14]  Y[1.54,1.94]  Z[-0.12,0.14]`. Blender's `wm.obj_import` PRESERVES
this frame (no axis remap), so `vertex.co` written straight to the `.pv` is
already correct — no conversion. The morphed `.pv` must read Y≈1.54–1.94.

## Steps

### 1. Get the target as OBJ, ALIGNED to the Legacy head frame
Convert FBX→OBJ in Blender if needed (`import_scene.fbx` → `wm.obj_export`).
Then **align** the target so its bbox overlays the donor head bbox above
(Y-up, face +Z, scaled to ~the donor's size). Feature alignment (eyes/nose) beats
a raw bbox fit. A bbox auto-fit (uniform scale by height + center) is an OK
starting point but verify Y≈1.6–1.94. **A mis-aligned target → garbage morph.**

### 2. Export the Legacy donor head
```
export_submesh_obj <donor>/player_67.rx2 <work>/donor head
# -> <work>/donor/head.obj  (2623 verts, UVs, the Legacy head + UV template)
```

### 3. Blender: shrinkwrap the donor onto the aligned target
Run `blender --background --python wrap.py`. The script (below):
- imports `donor/head.obj` + the aligned target OBJ,
- builds a **boundary-freeze falloff** vertex group: the `head_` mesh's BOUNDARY
  LOOPS are the eye sockets / nostrils / mouth opening / neck — freeze them
  (weight 0) so the holes keep donor shape (eyeballs/mouthbag still fit), ramping
  to 1 over `N=8` rings (BFS edge-distance) for a smooth transition,
- **Shrinkwrap** modifier, `wrap_method='NEAREST_SURFACEPOINT'`, `vertex_group='morph'`,
- **Smooth** modifier (factor 0.5, iterations 4, same group) to de-noise,
- writes `morphed.pv` = per-vertex `x y z u v` in donor vertex order (positions
  are what matter; UV columns are dummies — `--keep-uv` ignores them).

```python
import bpy, bmesh, collections
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.wm.obj_import(filepath=r"<work>/donor/head.obj")
donor=[o for o in bpy.context.selected_objects if o.type=='MESH'][0]
bpy.ops.wm.obj_import(filepath=r"<aligned_target>.obj")
target=[o for o in bpy.context.selected_objects if o.type=='MESH'][0]
bpy.context.view_layer.objects.active=donor
bm=bmesh.new(); bm.from_mesh(donor.data)
boundary=set(v.index for e in bm.edges if len(e.link_faces)==1 for v in e.verts)
adj=collections.defaultdict(list)
for e in bm.edges:
    a,b=e.verts[0].index,e.verts[1].index; adj[a].append(b); adj[b].append(a)
bm.free()
dist={i:10**9 for i in range(len(donor.data.vertices))}; dq=collections.deque()
for b in boundary: dist[b]=0; dq.append(b)
while dq:
    u=dq.popleft()
    for w in adj[u]:
        if dist[w]>dist[u]+1: dist[w]=dist[u]+1; dq.append(w)
N=8
vg=donor.vertex_groups.new(name="morph")
for vi,d in dist.items(): vg.add([vi], min(d/N,1.0), 'REPLACE')
sw=donor.modifiers.new("sw","SHRINKWRAP"); sw.target=target
sw.wrap_method='NEAREST_SURFACEPOINT'; sw.vertex_group="morph"
bpy.ops.object.modifier_apply(modifier="sw")
sm=donor.modifiers.new("sm","SMOOTH"); sm.factor=0.5; sm.iterations=4; sm.vertex_group="morph"
bpy.ops.object.modifier_apply(modifier="sm")
with open(r"<work>/morphed.pv","w") as f:
    for v in donor.data.vertices:
        c=v.co; f.write(f"{c.x:.6f} {c.y:.6f} {c.z:.6f} 0 0\n")
```
Sanity: `morphed.pv` must have 2623 lines and Y≈1.54–1.94.

### 4. Reshape the donor head_ to the morph (KEEP donor UVs + weights)
```
rigaudit reshape-head <donor>/player_67.rx2 \
  --pv <work>/morphed.pv --slot head \
  --keep-uv --collapse-haircap \
  --out <corpus>/base/player_<slug>.rx2
```
- `--keep-uv` → donor's default Legacy head UVs (NOT the morph's). **Required.**
- NO `--flip-v`.
- `--collapse-haircap` → bald (the morphed `head_` already carries the full head
  shape; eyes/mouthbag stay donor at the frozen sockets).

### 5. Texlib (vanilla-sized, mips regenerated from the new texture)
```
rx2tex replace --reencode --resize 512 512 --format DXT1 <donor>/texlib_67.rx2 1 \
  <slug>_head_color.dds <corpus>/base/texlib_<slug>.rx2
```
Reflow the head texture at the slot's VANILLA dims (512 DXT1) so the FULL mip
chain is regenerated from the new texture → output is exactly 331776 bytes
(byte-size + descriptor identical to vanilla), and distant LODs now show the
player's own texture (not the donor's). NEVER **upscale** the dims (e.g. 1024)
for the standard texlib — that bloats it (~1.6MB) and crashes the game. Regen at
the vanilla dims is safe (verified 331776 B, same buffers/mip_start_pages as
vanilla). (Older runs used `--keep-tail`, which kept the donor's mip tail — base
mip correct but distant LODs showed the donor; superseded by this reflow.)
Texlib rasters keep the donor id (`head_67_cm`); retarget to the player's head-id
slot at assignment (rename files + equal-length raster-id rewrite).

## Verify
- `morphed.pv`: 2623 lines, Y≈1.54–1.94.
- `player_<slug>.rx2` re-parses (rigaudit prints "wrote …").
- `texlib_<slug>.rx2` == 331776 bytes.
- In-game: face upright, texture aligned (no offset, not upside-down).

## Pitfalls = the exact failures other sessions hit
| symptom | cause | fix |
|---|---|---|
| texture OFFSET | transferred the TARGET's UVs | use `--keep-uv` (donor Legacy UVs) |
| texture UPSIDE-DOWN | flipped V with donor UVs | with `--keep-uv`, do NOT `--flip-v` |
| head rotated/lying down | wrote non-Y-up coords | write `vertex.co` straight (Blender keeps Y-up); pv Y must be ~1.7 |
| garbage / melted morph | target not aligned to Legacy frame | align target bbox to the donor head bbox first |
| collapsed eyes/nostrils/mouth/neck | no boundary-freeze | the BFS boundary falloff group (N=8) is mandatory |
| static face (no animation) | rigid weights | `reshape-head` keeps the DONOR weights — never rigid |
| streaky / banded face shadowing | normals zeroed to flat `[0,0,1]` | `reshape-head` now recomputes smooth normals from the morph (build ≥2026-06-25) |
| texlib crashes game | UPSCALED dims (1024) → ~1.6MB bloat | reflow at VANILLA dims `--resize 512 512` (still 331776 b) — do NOT upscale |

See `tools/`/`Working Assets/Output/mike_wrap/` for the reference run
(`wrap_final.py`, `morphed_mike.pv`). This supersedes any other mesh-import
instructions for arbitrary (un-rigged) head meshes.
