"""Worker Blender stage: CC5 head-only FBX -> source_geometry.json (Studio matcher record schema), read-only on the source.

blender --background --factory-startup --python-exit-code 1 --python blender_extract_cc5.py -- <source.fbx> <out_dir>

Technique (same as the Studio's read-only inspect/probe scripts, reproduced, not copied wholesale): the head-only
export's rigid Skin/Cluster records trip Blender 4.5's armature_setup[None]; they are stripped in memory (they carry a
single 1.0 weight to one bone and no facial rig - research/claude/rig-comparison.md), the FBX is imported with default
options after factory settings, and each mesh object is written as per-(vertex, uv) deduplicated records in world
coordinates (metres, Blender Z up). Writes source_geometry.json + extract_receipt.json. Refuses an existing out_dir.
"""
import bpy, hashlib, json, sys, time
from pathlib import Path
args = sys.argv[sys.argv.index('--') + 1:]
src = Path(args[0]); out = Path(args[1])
if out.exists(): raise SystemExit(f'refusing to overwrite existing {out}')
if not src.exists(): raise SystemExit(f'missing source {src}')
t0 = time.time()
bpy.ops.wm.read_factory_settings(use_empty=True)
from io_scene_fbx import parse_fbx
original_parse = parse_fbx.parse; removed = []


def static_parse(*a, **kw):
    elem, version = original_parse(*a, **kw)
    objects = next(e for e in elem.elems if e.id == b'Objects')
    ids = {e.props[0] for e in objects.elems if e.id == b'Deformer' and e.props[2] in (b'Skin', b'Cluster')}
    removed.extend(sorted(ids))
    objects.elems[:] = [e for e in objects.elems if e.props[0] not in ids]
    connections = next(e for e in elem.elems if e.id == b'Connections')
    connections.elems[:] = [e for e in connections.elems if not any(p in ids for p in e.props[1:3])]
    return elem, version


parse_fbx.parse = static_parse
bpy.ops.import_scene.fbx(filepath=str(src))
result = []; census = []
for ob in bpy.data.objects:
    census.append({'name': ob.name, 'type': ob.type, 'matrix_world': [list(r) for r in ob.matrix_world]})
    if ob.type != 'MESH': continue
    original = [list(ob.matrix_world @ v.co) for v in ob.data.vertices]
    ps = []; uv = []; lookup = {}; loop_ids = {}
    uvl = ob.data.uv_layers.active.data
    for loop in ob.data.loops:
        t = tuple(uvl[loop.index].uv)
        key = (loop.vertex_index, *t)
        if key not in lookup: lookup[key] = len(ps); ps.append(original[loop.vertex_index]); uv.append(list(t))
        loop_ids[loop.index] = lookup[key]
    ob.data.calc_loop_triangles()
    tris = [[loop_ids[l] for l in t.loops] for t in ob.data.loop_triangles]
    result.append({'name': ob.name, 'positions': ps, 'uv': uv, 'triangles': tris, 'triangle_materials': [t.material_index for t in ob.data.loop_triangles]})
    census[-1].update({'vertices': len(ob.data.vertices), 'polygons': len(ob.data.polygons), 'records': len(ps), 'triangles': len(tris), 'materials': [m.name if m else None for m in ob.data.materials], 'shape_keys': [k.name for k in ob.data.shape_keys.key_blocks] if ob.data.shape_keys else []})
out.mkdir(parents=True)
(out / 'source_geometry.json').write_text(json.dumps(result))
rec = {'stage': 'extract_source_cc5', 'source': str(src), 'source_sha256': hashlib.sha256(src.read_bytes()).hexdigest(), 'blender': bpy.app.version_string, 'removed_skin_cluster_ids': removed, 'import_options': 'bpy.ops.import_scene.fbx defaults after read_factory_settings(use_empty=True)',
       'record_rule': 'per (vertex_index, uv) deduplicated loops, world-space positions, active UV layer (V-up), loop triangles', 'objects': census, 'output_sha256': hashlib.sha256((out / 'source_geometry.json').read_bytes()).hexdigest(), 'seconds': round(time.time() - t0, 1),
       'losses': ['rigid one-bone skin binding dropped (no facial rig in head-only exports)', 'materials/textures not carried (diffuse referenced by path in the job)', 'no morphs exist in head-only exports']}
(out / 'extract_receipt.json').write_text(json.dumps(rec, indent=1))
print('EXTRACTED', json.dumps([{k: v for k, v in c.items() if k in ('name', 'type', 'records', 'triangles')} for c in census]))
