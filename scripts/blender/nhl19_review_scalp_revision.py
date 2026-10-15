"""Review the prepared revision without editing native assets. Blender --python ... -- root"""
import bpy, sys, json
import numpy as np
from pathlib import Path
from mathutils import Vector

root=Path(sys.argv[sys.argv.index('--')+1])
old=json.loads(Path(r'H:\TEMP\nhl19-mogilny-game-test\native_mesh.json').read_text())
new=json.loads((root/'native_mesh.json').read_text())
oldhead=next(m for m in old['submeshes'] if m['name']=='geo_headShape')
head=next(m for m in new['submeshes'] if m['name']=='geo_headShape')
placement=json.loads((root/'mesh_report.json').read_text())['placement']
a=np.array(head['original_positions']);b=np.array(head['positions']);prior=np.array(oldhead['positions'])
tri=np.array(head['triangles'])
def normals(p):return np.cross(p[tri[:,1]]-p[tri[:,0]],p[tri[:,2]]-p[tri[:,0]])
base_n=normals(a)
flips=lambda p:set(np.flatnonzero(np.sum(base_n*normals(p),axis=1)<0).tolist())
frozen=placement['frozen_native_ids']
assert np.array_equal(a[frozen],b[frozen])
assert flips(b)==flips(prior),'New orientation reversals introduced'
rigid=a[:,1]>=placement['neck_top_z']
shift=np.array([0,placement['translation'][2],0])
assert np.max(np.abs(b[rigid]-prior[rigid]-shift))<3e-7
assert max(a[:,1])==max(b[:,1])
report={'frozen_vertices_exact':len(frozen),'unchanged_orientation_reversal_ids':sorted(flips(b)),
        'rigid_upper_vertices':int(rigid.sum()),'max_upper_translation_error_m':float(np.max(np.abs(b[rigid]-prior[rigid]-shift)))}
bpy.ops.wm.open_mainfile(filepath=str(root/'Mogilny_Bald_GameTest.blend'))
scene=bpy.context.scene
scene.display.shading.color_type='TEXTURE'
scene.display.shading.light='STUDIO'
focus=Vector((0,-.025,1.775))
for name,d in [('front',Vector((0,-1,0))),('threequarter',Vector((.65,-1,0))),('profile',Vector((1,0,0)))]:
    scene.camera.location=focus+d
    scene.camera.rotation_euler=(focus-scene.camera.location).to_track_quat('-Z','Y').to_euler()
    scene.render.filepath=str(root/(name+'_textured.png'));bpy.ops.render.render(write_still=True)
# Compare actual independently decoded BC1 against the supplied 2K pixels.
im=bpy.data.images.load(str(root/'Mogilny_Texture'/'head_jack_eichel_cm.png'))
im.colorspace_settings.name='Non-Color'
p=np.empty(len(im.pixels),np.float32);im.pixels.foreach_get(p)
decoded=np.flipud(p.reshape(im.size[1],im.size[0],4))
source=np.load(root/'source_rgba.npy')
report['diffuse_rms_0_to_1']=float(np.sqrt(np.mean((decoded[:,:,:3]-source[:,:,:3])**2)))
report['diffuse_dimensions']=list(im.size)
(root/'revision_checks.json').write_text(json.dumps(report,indent=2))
print('REVISION_CHECKS',json.dumps(report))
