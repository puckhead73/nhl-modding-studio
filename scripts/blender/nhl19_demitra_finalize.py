"""Finish Demitra's native-order meshes, hide hair, and bake the registered atlas."""
import bpy,json,sys
import numpy as np
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree
root=Path(r'H:\TEMP\nhl19-demitra');out=root/'package';out.mkdir(exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(root/'fit/Demitra_UV_Fit.blend'))
native=json.loads((root/'vanilla/gaborik_native.json').read_text())
ledger=json.loads((root/'fit/uv_warp.json').read_text())
head=bpy.data.objects['Demitra_NHL19'];obs=[head];hair_report={}
tree=BVHTree.FromPolygons([v.co for v in head.data.vertices],[list(p.vertices) for p in head.data.polygons],all_triangles=True)
centre=Vector((0,-.02,1.87))
for m in native['submeshes']:
 if m['name']=='geo_headShape':continue
 if 'eyeShape' in m['name']:ob=bpy.data.objects[m['name']]
 else:
  ps=[Vector((p[0],-p[2],p[1])) for p in m['positions']];clear=[]
  for i,p in enumerate(ps):
   d=p-centre;d.z=max(.015,d.z);d.normalize()
   hit,_,_,_=tree.ray_cast(centre,d,1)
   if hit is None:raise RuntimeError('Missing scalp intersection')
   ps[i]=centre+(hit-centre)*.90;clear.append((hit-ps[i]).length)
  mesh=bpy.data.meshes.new(m['name']);mesh.from_pydata(ps,[],m['triangles']);mesh.update()
  ob=bpy.data.objects.new(m['name'],mesh);bpy.context.collection.objects.link(ob)
  hair_report[m['name']]={'count':len(ps),'minimum_clearance_mm':min(clear)*1000}
  assert min(clear)>.003
 obs.append(ob)
 if not ob.data.uv_layers:
  uv=ob.data.uv_layers.new(name='UVMap')
  for l in ob.data.loops:u,v=m['uv'][l.vertex_index];uv.data[l.index].uv=(u,1-v)

def image_array(path):
 im=bpy.data.images.load(str(path),check_existing=False);im.colorspace_settings.name='Non-Color'
 ar=np.empty(len(im.pixels),np.float32);im.pixels.foreach_get(ar)
 return np.flipud(ar.reshape(im.size[1],im.size[0],4)).copy()
src=image_array(Path(r'S:\Downloads\Mogilny Test\Demitra\Demitra.fbm\Std_Skin_Head_Diffuse.png'))
old=image_array(root/'vanilla/decoded/game_90-008#69297_head_marian_gaborik_cm.png')
anchors=np.array([a for a,b in ledger['pairs_pixels']],float)/1024;coef=np.array(ledger['coefficients']);n=len(anchors)
def warp(p):
 r2=((p[:,None,:]-anchors[None,:,:])**2).sum(2)
 return (r2*np.log(np.maximum(r2,1e-20)))@coef[:n]+np.c_[np.ones(len(p)),p]@coef[n:]
size=1024;atlas=np.empty((size,size,4),np.float32)
for y0 in range(0,size,16):
 yy,xx=np.mgrid[y0:y0+16,0:size];q=np.c_[(xx.ravel()+.5)/size,(yy.ravel()+.5)/size]
 mapped=warp(q);xy=np.clip(mapped*np.array([src.shape[1],src.shape[0]])-.5,0,np.array([src.shape[1]-1.001,src.shape[0]-1.001]))
 x=xy[:,0].astype(int);y=xy[:,1].astype(int);f=xy-np.c_[x,y]
 values=(src[y,x]*(1-f[:,0,None])+src[y,x+1]*f[:,0,None])*(1-f[:,1,None])+(src[y+1,x]*(1-f[:,0,None])+src[y+1,x+1]*f[:,0,None])*f[:,1,None]
 atlas[y0:y0+16]=values.reshape(16,size,4)
# NHL oral islands use its retained teeth/mouthbag. Never paint CC5 skin over them.
yy,xx=np.mgrid[:size,:size];interior=(yy/size>.84)&((xx/size<.405)|(xx/size>.595))
atlas[interior]=old[interior];atlas[:,:,3]=1
np.save(out/'source_rgba.npy',atlas)
im=bpy.data.images.new('Demitra_NHL19_Diffuse',width=size,height=size,alpha=True)
im.colorspace_settings.name='Non-Color';im.pixels.foreach_set(np.flipud(atlas).ravel());im.file_format='PNG';im.filepath_raw=str(out/'Demitra_NHL19_Diffuse.png');im.save()
im.colorspace_settings.name='sRGB';im.pack()
mat=bpy.data.materials.new('Demitra_NHL19');mat.use_nodes=True
node=mat.node_tree.nodes.new('ShaderNodeTexImage');node.image=im
mat.node_tree.links.new(node.outputs['Color'],mat.node_tree.nodes.get('Principled BSDF').inputs['Base Color'])
head.data.materials.clear();head.data.materials.append(mat)
report={'hair':hair_report,'placement':ledger['placement'],'texture_size':size,'normal_map':'source CC5 tangent normals retained for evaluation; no unique Gaborik normal asset found','quality':{}}
for m in native['submeshes']:
 ob=head if m['name']=='geo_headShape' else bpy.data.objects[m['name']]
 for p in ob.data.polygons:p.use_smooth=True
 ob.data.update();bpy.context.view_layer.update()
 m['positions']=[[v.co.x,v.co.z,-v.co.y] for v in ob.data.vertices]
 m['normals']=[[v.normal.x,v.normal.z,-v.normal.y] for v in ob.data.vertices]
 assert len(m['positions'])==len(m['original_positions'])
 assert [list(p.vertices) for p in ob.data.polygons]==m['triangles']
 oldp=np.array(m['original_positions']);newp=np.array(m['positions']);ts=np.array(m['triangles'])
 def crosses(p):return np.cross(p[ts[:,1]]-p[ts[:,0]],p[ts[:,2]]-p[ts[:,0]])
 a=crosses(oldp);b=crosses(newp)
 collapsed=int(np.sum((np.linalg.norm(a,axis=1)>1e-10)&(np.linalg.norm(b,axis=1)<1e-10)))
 report['quality'][m['name']]={'new_collapsed_triangles':collapsed,'orientation_changes':int(np.sum(np.sum(a*b,axis=1)<0))}
 if m['name']=='geo_headShape':
  report['frozen_lower_vertices']=int(np.sum(oldp[:,1]<=1.66))
  assert np.array_equal(oldp[oldp[:,1]<=1.66],newp[oldp[:,1]<=1.66])
  report['final_crown']=float(newp[:,1].max());report['donor_crown']=float(oldp[:,1].max())
  assert abs(newp[:,1].max()-oldp[:,1].max())<1e-6
(out/'native_mesh.json').write_text(json.dumps(native))
(out/'mesh_report.json').write_text(json.dumps(report,indent=2))
for ob in bpy.data.objects:
 if ob.type=='MESH':ob.hide_render=ob not in obs;ob.hide_set(ob not in obs)
bpy.ops.object.select_all(action='DESELECT')
for ob in obs:ob.select_set(True)
bpy.context.view_layer.objects.active=head
bpy.ops.export_scene.gltf(filepath=str(out/'Demitra_NHL19.glb'),use_selection=True,export_animations=False,export_morph=False)
scene=bpy.context.scene;scene.display.shading.color_type='TEXTURE';focus=Vector((0,-.02,1.78))
for name,d in [('front',Vector((0,-1,0))),('threequarter',Vector((.65,-1,0))),('profile',Vector((1,0,0)))]:
 scene.camera.location=focus+d;scene.camera.rotation_euler=(focus-scene.camera.location).to_track_quat('-Z','Y').to_euler()
 scene.render.filepath=str(out/(name+'_textured.png'));bpy.ops.render.render(write_still=True)
bpy.ops.wm.save_as_mainfile(filepath=str(out/'Demitra_NHL19.blend'))
print('FINAL',json.dumps(report))
