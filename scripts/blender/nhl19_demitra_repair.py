"""Inspect and repair the Demitra fixture's neck and split scalp seam.

Keeps native record IDs; writes a separate review revision, never the live game.
Run with Blender 4.5 --background --python-exit-code 1 --python this_file.
"""
import json
from pathlib import Path
import bpy
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

ROOT = Path(r'H:\TEMP\nhl19-demitra')
OUT = ROOT / 'revision2'
OUT.mkdir(exist_ok=True)
native = json.loads((ROOT / 'package/native_mesh.json').read_text())
head = next(m for m in native['submeshes'] if m['name'] == 'geo_headShape')
old = np.array(head['original_positions'])
fitted = np.array(head['positions'])
original_normals = np.array(head['normals'])
# Protect both complete ear UV regions and their attached triangle neighbours.
# Explicit record sets make this an exact invariant, not a soft spatial guess.
uv = np.array(head['uv'])
ear_core = (((uv[:, 0] > .16) & (uv[:, 0] < .26)) | ((uv[:, 0] > .74) & (uv[:, 0] < .84))) & (uv[:, 1] > .31) & (uv[:, 1] < .48) & (np.abs(old[:, 0]) > .065)
ear_ids = set(np.flatnonzero(ear_core).tolist())
for _ in range(2):
    neighbours = {i for triangle in head['triangles'] if any(j in ear_ids for j in triangle) for i in triangle}
    ear_ids.update(neighbours)
ear_ids = np.array(sorted(ear_ids), dtype=int)
# The posterior CC5 chart is not homologous at the NHL wrap seam. Use spatial
# radial correspondence only there, leaving the registered face untouched.
sources = json.loads((ROOT / 'source/source_geometry.json').read_text())
source = next(m for m in sources if m['name'] == '0')
ledger = json.loads((ROOT / 'fit/uv_warp.json').read_text())
shift = np.array(ledger['source_shift'])
sp = np.array(source['positions']) + shift
tree = BVHTree.FromPolygons([Vector(p) for p in sp], source['triangles'], all_triangles=True)
centre = Vector((0, -.018, 1.855))
result = fitted.copy()
def smooth(t):
    t = np.clip(t, 0, 1)
    return t * t * (3 - 2 * t)
rear_weight = 1 - smooth((old[:, 2] - .005) / .040)
# Ear folds are multi-layer surfaces; radial scalp fitting must not touch them.
ear_protection = smooth((np.abs(old[:, 0]) - .045) / .015) * (1 - smooth((np.abs(old[:, 1] - 1.825) - .035) / .020))
rear_weight *= 1 - ear_protection
rear_weight[ear_ids] = 0
rear_weight[ledger['oral_ids']] = 0
rear_count = 0
for i in np.flatnonzero((rear_weight > 0) & (old[:, 1] > 1.735)):
    p = Vector((old[i, 0], -old[i, 2], old[i, 1]))
    direction = (p - centre).normalized()
    hit, _, _, _ = tree.ray_cast(centre, direction, .5)
    if hit is None:
        hit, _, _, _ = tree.find_nearest(p)
    target = np.array((hit.x, hit.z, -hit.y))
    result[i] = fitted[i] * (1 - rear_weight[i]) + target * rear_weight[i]
    rear_count += 1

# Keep the actual neck, shoulder and chest intact, with a longer transition on
# the nape than beneath the chin. Never project the neck across unrelated UVs.
neck_top = 1.775 + .045 * rear_weight
weight = smooth((old[:, 1] - 1.735) / (neck_top - 1.735))
weight[ledger['oral_ids']] = 1
weight[ear_ids] = 1
result = old + (result - old) * weight[:, None]
# Restore the crown exactly after the posterior correspondence adjustment.
result[:, 1] += (old[:, 1].max() - result[:, 1].max()) * smooth((old[:, 1] - 1.90) / .03)
groups = {}
for i, p in enumerate(old):
    groups.setdefault(tuple(p), []).append(i)
for ids in groups.values():
    if len(ids) > 1:
        result[ids] = result[ids].mean(axis=0)
result[ear_ids] = fitted[ear_ids]

bpy.ops.wm.open_mainfile(filepath=str(ROOT / 'package/Demitra_NHL19.blend'))
ob = bpy.data.objects['Demitra_NHL19']
for v, p in zip(ob.data.vertices, result):
    v.co = (p[0], -p[2], p[1])
ob.data.update()
# Duplicate seam records need a common shading normal, not just equal XYZ.
# Accumulate unnormalized triangle normals across the positional equivalence
# classes. This preserves native topology and the separate UV records.
ts = np.array(head['triangles'])
area = np.cross(result[ts[:, 1]] - result[ts[:, 0]], result[ts[:, 2]] - result[ts[:, 0]])
normals = np.zeros_like(result)
for corner in range(3):
    np.add.at(normals, ts[:, corner], area)
for ids in groups.values():
    if len(ids) > 1:
        normals[ids] = normals[ids].sum(axis=0)
normals /= np.maximum(np.linalg.norm(normals, axis=1)[:, None], 1e-20)
ear_distance = np.linalg.norm(old[:, None, :] - old[ear_core][None, :, :], axis=2).min(axis=1)
normal_weight = 1 - smooth(ear_distance / .025)
normals = normals * (1 - normal_weight[:, None]) + original_normals * normal_weight[:, None]
normals /= np.maximum(np.linalg.norm(normals, axis=1)[:, None], 1e-20)
normals[ear_core] = original_normals[ear_core]
ob.data.normals_split_custom_set_from_vertices([(n[0], -n[2], n[1]) for n in normals])
head['positions'] = result.tolist()
head['normals'] = normals.tolist()
head_tree = BVHTree.FromPolygons([v.co for v in ob.data.vertices], head['triangles'], all_triangles=True)
hair_clearance = {}
for mesh in native['submeshes']:
    if 'cards' not in mesh['name'] and 'haircap' not in mesh['name']:
        continue
    hair = bpy.data.objects[mesh['name']]
    gaps = []
    for vertex, p in zip(hair.data.vertices, mesh['original_positions']):
        d = Vector((p[0], -p[2], p[1])) - centre
        d.z = max(.015, d.z)
        d.normalize()
        hit, _, _, _ = head_tree.ray_cast(centre, d, 1)
        assert hit is not None
        vertex.co = centre + (hit - centre) * .90
        gaps.append((hit - vertex.co).length * 1000)
    hair.data.update()
    mesh['positions'] = [[v.co.x, v.co.z, -v.co.y] for v in hair.data.vertices]
    mesh['normals'] = [[v.normal.x, v.normal.z, -v.normal.y] for v in hair.data.vertices]
    hair_clearance[mesh['name']] = min(gaps)
    assert min(gaps) > 3
quality = {}
for mesh in native['submeshes']:
    tris = np.array(mesh['triangles'])
    def triangle_areas(positions):
        p = np.array(positions)
        return np.linalg.norm(np.cross(p[tris[:, 1]] - p[tris[:, 0]], p[tris[:, 2]] - p[tris[:, 0]]), axis=1)
    collapsed = int(np.sum((triangle_areas(mesh['original_positions']) > 1e-10) & (triangle_areas(mesh['positions']) < 1e-10)))
    assert collapsed == 0, (mesh['name'], collapsed)
    quality[mesh['name']] = {'new_collapsed_triangles': collapsed}
(OUT / 'native_mesh.json').write_text(json.dumps(native))
report = {
    'posterior_spatial_vertices': rear_count,
    'frozen_neck_vertices': int(np.sum(weight == 0)),
    'frozen_neck_max_error': float(np.max(np.abs(result[weight == 0] - old[weight == 0]))),
    'crown': float(result[:, 1].max()),
    'donor_crown': float(old[:, 1].max()),
    'seam_max_gap': max(float(np.linalg.norm(result[ids] - result[ids[0]], axis=1).max()) for ids in groups.values()),
    'hair_minimum_clearance_mm': hair_clearance,
    'quality': quality,
    'ears': {},
}
for label, sign in [('negative_x', -1), ('positive_x', 1)]:
    ids = ear_ids[old[ear_ids, 0] * sign > 0]
    assert len(ids) > 0
    assert np.array_equal(result[ids], fitted[ids])
    core_ids = ids[ear_core[ids]]
    assert np.array_equal(normals[core_ids], original_normals[core_ids])
    report['ears'][label] = {'protected_ids': ids.tolist(), 'vertices': len(ids), 'normal_locked_ids': core_ids.tolist(), 'max_position_change_mm': float(np.linalg.norm(result[ids] - fitted[ids], axis=1).max() * 1000), 'max_ear_core_normal_change': float(np.abs(normals[core_ids] - original_normals[core_ids]).max()), 'reference': str(ROOT / 'package/native_mesh.json')}
assert report['frozen_neck_max_error'] == 0
assert abs(report['crown'] - report['donor_crown']) < 1e-6
(OUT / 'repair_report.json').write_text(json.dumps(report, indent=2))
scene = bpy.context.scene
scene.display.shading.color_type = 'SINGLE'
scene.display.shading.single_color = (.65, .65, .65)
scene.display.shading.show_cavity = True
focus = Vector((0, -.02, 1.78))
for name, direction in [('profile', (1, 0, 0)), ('opposite_profile', (-1, 0, 0)), ('rear', (0, 1, 0)), ('rear_threequarter', (.7, 1, 0)), ('front', (0, -1, 0))]:
    scene.camera.location = focus + Vector(direction)
    scene.camera.rotation_euler = (focus - scene.camera.location).to_track_quat('-Z', 'Y').to_euler()
    scene.render.filepath = str(OUT / (name + '_solid.png'))
    bpy.ops.render.render(write_still=True)
bpy.ops.wm.save_as_mainfile(filepath=str(OUT / 'Demitra_NHL19_Revision2.blend'))
bpy.ops.object.select_all(action='DESELECT')
for mesh in native['submeshes']:
    target = ob if mesh['name'] == 'geo_headShape' else bpy.data.objects[mesh['name']]
    target.select_set(True)
bpy.context.view_layer.objects.active = ob
bpy.ops.export_scene.gltf(filepath=str(OUT / 'Demitra_NHL19_Revision2.glb'), use_selection=True, export_animations=False, export_morph=False)
print('REPAIR: neck, crown, seam, both ear locks and all component area checks passed; see repair_report.json')
