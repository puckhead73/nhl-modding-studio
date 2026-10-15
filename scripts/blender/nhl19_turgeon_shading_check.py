"""Compare custom normals with geometric normals and isolate scalp from hair."""
import bpy
import json,numpy as np
from pathlib import Path
root=Path(r'H:\TEMP\nhl19-turgeon\revision2')
m=next(m for m in json.loads((root/'native_mesh.json').read_text())['submeshes'] if m['name']=='geo_headShape')
a=np.array(m['original_positions']);b=np.array(m['positions']);uv=np.array(m['uv'])
d=np.linalg.norm(a[:,None,:]-a[None,:,:],axis=2);np.fill_diagonal(d,1)
pairs=np.argwhere(np.triu(d<.00001,1));gaps=np.linalg.norm(b[pairs[:,0]]-b[pairs[:,1]],axis=1)
for j in np.argsort(gaps)[-12:]: print('NEAR_SEAM',pairs[j].tolist(),float(gaps[j]),a[pairs[j]].tolist(),uv[pairs[j]].tolist())
bpy.ops.wm.open_mainfile(filepath=str(root/'Turgeon_NHL19_Revision2.blend'))
head=bpy.data.objects['Turgeon_NHL19']
for ob in bpy.data.objects:
    if ob.type=='MESH': ob.hide_render=ob!=head
from mathutils import Vector
focus=Vector((0,-.02,1.78));scene=bpy.context.scene
scene.camera.location=focus+Vector((1,0,0))
scene.camera.rotation_euler=(focus-scene.camera.location).to_track_quat('-Z','Y').to_euler()
scene.render.filepath=str(root/'head_only_custom.png');bpy.ops.render.render(write_still=True)
head.data.normals_split_custom_set([(0,0,0)]*len(head.data.loops))
scene.render.filepath=str(root/'head_only_geometric.png');bpy.ops.render.render(write_still=True)
