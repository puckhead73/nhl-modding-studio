"""Render a fitted head with a given atlas PNG (Blender 4.5 Workbench, texture mode).
blender --background --factory-startup --python-exit-code 1 --python render_textured.py -- <native_mesh.json> <atlas.png> <out_dir> <tag>
"""
import bpy, json, sys
from pathlib import Path
from mathutils import Vector
args = sys.argv[sys.argv.index('--') + 1:]
native = json.loads(Path(args[0]).read_text()); atlas = Path(args[1]); out = Path(args[2]); tag = args[3]
if any(out.glob(f'*_{tag}.png')): raise SystemExit(f'refusing to overwrite existing renders for tag {tag} in {out}')
out.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
head = next(m for m in native['submeshes'] if m['name'] == 'geo_headShape')
m = bpy.data.meshes.new('head'); m.from_pydata([Vector((p[0], -p[2], p[1])) for p in head['positions']], [], head['triangles']); m.update()
uv = m.uv_layers.new(name='UVMap')
for l in m.loops: u, v = head['uv'][l.vertex_index]; uv.data[l.index].uv = (u, 1 - v)
for p in m.polygons: p.use_smooth = True
ob = bpy.data.objects.new('head', m); bpy.context.collection.objects.link(ob)
im = bpy.data.images.load(str(atlas)); mat = bpy.data.materials.new('atlas'); mat.use_nodes = True
tex = mat.node_tree.nodes.new('ShaderNodeTexImage'); tex.image = im
mat.node_tree.links.new(tex.outputs['Color'], mat.node_tree.nodes['Principled BSDF'].inputs['Base Color']); m.materials.append(mat)
for mm in native['submeshes']:
    if 'eyeShape' in mm['name'] or 'eyesShape' in mm['name']:
        e = bpy.data.meshes.new(mm['name']); e.from_pydata([Vector((p[0], -p[2], p[1])) for p in mm['positions']], [], mm['triangles']); e.update()
        for p in e.polygons: p.use_smooth = True
        bpy.context.collection.objects.link(bpy.data.objects.new(mm['name'], e))
scene = bpy.context.scene; scene.render.engine = 'BLENDER_WORKBENCH'; scene.display.shading.light = 'STUDIO'; scene.display.shading.color_type = 'TEXTURE'
scene.render.resolution_x = 640; scene.render.resolution_y = 800; scene.render.film_transparent = False
cam = bpy.data.cameras.new('cam'); cam.type = 'ORTHO'; cam.ortho_scale = .42; camob = bpy.data.objects.new('cam', cam); scene.collection.objects.link(camob); scene.camera = camob
focus = Vector((0, -.01, 1.77))
for name, d in {'front': (0, -1, 0), 'left_profile': (1, 0, 0), 'right_profile': (-1, 0, 0), 'rear': (0, 1, 0), 'rear_top': (0, 1, .8), 'left_rear': (.8, 1, .1)}.items():
    d = Vector(d).normalized(); camob.location = focus + d; camob.rotation_euler = (focus - camob.location).to_track_quat('-Z', 'Y').to_euler()
    scene.render.filepath = str(out / f'{name}_{tag}.png'); bpy.ops.render.render(write_still=True)
print('TEXTURED', tag)
