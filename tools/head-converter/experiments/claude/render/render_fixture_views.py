"""Render donor / fitted / CC5-source solid views with identical framing (Blender 4.5, Workbench, cavity on).

blender --background --factory-startup --python-exit-code 1 --python render_fixture_views.py -- <native_mesh.json> <source_geometry.json> <uv_warp.json> <out_dir> [label]

Views: front, left profile, right profile, rear, left under-ear jaw, right under-ear jaw, underside (chin-up), top.
Left/right are the character's sides (character faces -Y; character's left is +X).
Writes <view>_<mesh>.png and a manifest. No source assets are modified.
"""
import bpy, json, sys
from pathlib import Path
from mathutils import Vector
args = sys.argv[sys.argv.index('--') + 1:]
native = json.loads(Path(args[0]).read_text()); source = json.loads(Path(args[1]).read_text()); ledger = json.loads(Path(args[2]).read_text())
out = Path(args[3]); label = args[4] if len(args) > 4 else 'fit'
if (out / 'render_manifest.json').exists(): raise SystemExit(f'refusing to overwrite existing renders in {out}')
out.mkdir(parents=True, exist_ok=True)
head = next(m for m in native['submeshes'] if m['name'] == 'geo_headShape')
cc = next(m for m in source if m['name'] == '0'); shift = Vector(ledger['source_shift'])
bpy.ops.wm.read_factory_settings(use_empty=True)
def mesh_object(name, ps, tris):
    m = bpy.data.meshes.new(name); m.from_pydata(ps, [], tris); m.update()
    for p in m.polygons: p.use_smooth = True
    ob = bpy.data.objects.new(name, m); bpy.context.collection.objects.link(ob); return ob
tb = lambda p: Vector((p[0], -p[2], p[1]))
objs = {'donor': mesh_object('donor', [tb(p) for p in head['original_positions']], head['triangles']),
        'fit': mesh_object('fit', [tb(p) for p in head['positions']], head['triangles']),
        'source': mesh_object('source', [Vector(p) + shift for p in cc['positions']], cc['triangles'])}
# Also include the fitted eyes for the fit view so sockets are not empty
for m in native['submeshes']:
    if 'eyeShape' in m['name']:
        e = mesh_object('fit_' + m['name'], [tb(p) for p in m['positions']], m['triangles']); e['group'] = 'fit'
        d = mesh_object('donor_' + m['name'], [tb(p) for p in m['original_positions']], m['triangles']); d['group'] = 'donor'
for k, ob in objs.items(): ob['group'] = k
scene = bpy.context.scene; scene.render.engine = 'BLENDER_WORKBENCH'
scene.display.shading.light = 'STUDIO'; scene.display.shading.color_type = 'SINGLE'; scene.display.shading.single_color = (.7, .7, .7)
scene.display.shading.show_cavity = True; scene.display.shading.cavity_type = 'BOTH'; scene.display.shading.curvature_ridge_factor = 1.5; scene.display.shading.curvature_valley_factor = 1.5
scene.render.resolution_x = 640; scene.render.resolution_y = 800; scene.render.resolution_percentage = 100; scene.render.film_transparent = False
cam = bpy.data.cameras.new('cam'); cam.type = 'ORTHO'; cam.ortho_scale = .42
camob = bpy.data.objects.new('cam', cam); scene.collection.objects.link(camob); scene.camera = camob
focus = Vector((0, -.01, 1.77))
views = {'front': (0, -1, 0), 'left_profile': (1, 0, 0), 'right_profile': (-1, 0, 0), 'rear': (0, 1, 0),
         'left_jaw': (1, .35, -.45), 'right_jaw': (-1, .35, -.45), 'underside': (0, -.6, -1), 'top': (0, .0001, 1),
         'left_rear_threequarter': (.8, 1, .1), 'right_rear_threequarter': (-.8, 1, .1)}
manifest = []
for vname, d in views.items():
    d = Vector(d).normalized(); camob.location = focus + d
    camob.rotation_euler = (focus - camob.location).to_track_quat('-Z', 'Y').to_euler()
    for group in ('donor', 'fit', 'source'):
        for ob in bpy.data.objects:
            if ob.type == 'MESH': ob.hide_render = ob.get('group') != group
        scene.render.filepath = str(out / f'{vname}_{group}.png'); bpy.ops.render.render(write_still=True)
        manifest.append({'view': vname, 'mesh': group, 'file': f'{vname}_{group}.png', 'direction': list(d)})
(out / 'render_manifest.json').write_text(json.dumps({'label': label, 'focus': list(focus), 'ortho_scale': cam.ortho_scale, 'renders': manifest}, indent=1))
print('RENDERED', len(manifest))
