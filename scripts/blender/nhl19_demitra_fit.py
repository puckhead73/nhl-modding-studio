"""CC5 -> NHL19: calibrated UV warp, source-surface sampling, native topology.
Development preview; native packing follows only after geometry review.
"""
import bpy,json,sys,math
import numpy as np
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.geometry import barycentric_transform
sys.path.insert(0,str(Path(__file__).parent))
from head_scalp_placement import place_head
root=Path(r'H:\TEMP\nhl19-demitra');out=root/'fit';out.mkdir(exist_ok=True)
native=json.loads((root/'vanilla/gaborik_native.json').read_text())
sources=json.loads((root/'source/source_geometry.json').read_text())
cc=next(m for m in sources if m['name']=='0')
donor=next(m for m in native['submeshes'] if m['name']=='geo_headShape')
# Anatomical atlas landmarks measured on both exported 1024 preview images.
# Stored top-down pixels here; the native sampler converts once to Blender UVs.
pairs=[((512,0),(512,42)),((0,0),(0,40)),((1024,0),(1024,40)),
 ((0,500),(0,500)),((1024,500),(1024,500)),((0,820),(0,840)),((1024,820),(1024,840)),
 ((512,160),(512,190)),((512,275),(512,308)),
 ((367,336),(393,373)),((438,336),(462,373)),((399,326),(429,361)),((399,345),(429,385)),
 ((587,336),(562,373)),((659,336),(631,373)),((624,326),(595,361)),((624,345),(595,385)),
 ((512,397),(512,413)),((468,449),(477,458)),((556,449),(547,458)),
 ((512,450),(512,459)),((512,480),(512,496)),
 ((436,529),(459,526)),((591,529),(566,526)),
 ((477,509),(485,511)),((512,508),(512,516)),((547,509),(539,511)),
 ((477,532),(485,528)),((512,532),(512,527)),((547,532),(539,528)),
 ((478,551),(485,545)),((512,558),(512,550)),((547,551),(539,545)),
 ((512,638),(512,645)),((360,637),(391,638)),((664,637),(633,638)),
 ((200,390),(222,403)),((820,390),(802,403)),
 ((194,337),(218,353)),((831,337),(806,353)),((224,464),(245,478)),((800,464),(779,478)),
 ((512,820),(512,820)),((256,810),(256,810)),((768,810),(768,810))]
anchors=np.array([a for a,b in pairs],float)/1024;targets=np.array([b for a,b in pairs],float)/1024
def kernel(a,b):
 r2=((a[:,None,:]-b[None,:,:])**2).sum(2)
 return r2*np.log(np.maximum(r2,1e-20))
n=len(anchors);P=np.c_[np.ones(n),anchors]
A=np.block([[kernel(anchors,anchors)+np.eye(n)*1e-9,P],[P.T,np.zeros((3,3))]])
coef=np.linalg.solve(A,np.vstack([targets,np.zeros((3,2))]))
def warp(uv):return kernel(uv,anchors)@coef[:n]+np.c_[np.ones(len(uv)),uv]@coef[n:]
sp=[Vector(p) for p in cc['positions']];suv=[Vector((u,v,0)) for u,v in cc['uv']]
uvtree=BVHTree.FromPolygons(suv,cc['triangles'],all_triangles=True)
dp=[Vector((p[0],-p[2],p[1])) for p in donor['positions']]
duv=np.array(donor['uv']);mapped=warp(duv)
# CC5 and native are metre-scale, with the same facing direction. Align crown
# and transverse centre; use donor eye depth to place the facial plane.
se=next(m for m in sources if m['name']=='cc_base_eye')
de=next(m for m in native['submeshes'] if m['name']=='geo_reyeShape')
source_eyes=np.array(se['positions']);donor_eyes=np.array([[p[0],-p[2],p[1]] for p in de['positions']])
shift=Vector((0,float(donor_eyes[:,1].mean()-source_eyes[:,1].mean()),max(p.z for p in dp)-max(p.z for p in sp)))
sampled=[];distances=[]
for i,uv in enumerate(mapped):
 q=Vector((float(uv[0]),float(1-uv[1]),0));hit,normal,ti,d=uvtree.find_nearest(q)
 ids=cc['triangles'][ti]
 p=barycentric_transform(hit,*[suv[j] for j in ids],*[sp[j] for j in ids])+shift
 sampled.append(p);distances.append(d)
# Donor-only oral islands retain their anatomy and follow the mouth displacement.
# Embedded lash strips have near-zero UV area: follow their nearest facial surface
# through source sampling, retaining the donor strip's local offset below.
mouth_ids=[i for i,(u,v) in enumerate(duv) if .43<u<.57 and .51<v<.52]
mouth_delta=sum((sampled[i]-dp[i] for i in mouth_ids),Vector())/max(1,len(mouth_ids))
oral=[i for i,(u,v) in enumerate(duv) if v>.84 and (u<.405 or u>.595) and dp[i].z>1.70]
for i in oral:sampled[i]=dp[i]+mouth_delta
# Coincident UVs can represent distinct inner/outer lip layers. Keep the native
# offset from the frontmost record; UV equality never licenses collapsing them.
uvgroups={}
for i,uv in enumerate(duv):uvgroups.setdefault(tuple(uv),[]).append(i)
for ids in uvgroups.values():
 if len(ids)>1:
  front=min(ids,key=lambda i:dp[i].y)
  for i in ids:
   if (dp[i]-dp[front]).length>1e-5:sampled[i]=sampled[front]+dp[i]-dp[front]
# Lock native seam duplicates to a common result without welding native IDs.
groups={}
for i,p in enumerate(dp):groups.setdefault(tuple(round(c,6) for c in p),[]).append(i)
for ids in groups.values():
 if len(ids)>1:
  p=sum((sampled[i] for i in ids),Vector())/len(ids)
  for i in ids:sampled[i]=p.copy()
placed,_,placement=place_head(dp,sampled,1.66,1.74)
# A local neck-only correction avoids atlas stretch at CC5's lower chart border.
# No eye, lip, nasal or oral geometry participates in this spatial projection.
neck_tris=[t for t in cc['triangles'] if .60<np.mean([1-cc['uv'][i][1] for i in t])<.83]
neck_tree=BVHTree.FromPolygons([p+shift for p in sp],neck_tris,all_triangles=True)
for i,p in enumerate(dp):
 if i not in oral and 1.66<p.z<1.735:
  hit,_,_,_=neck_tree.find_nearest(placed[i])
  w=min(1,max(0,(p.z-1.66)/.045));w=w*w*(3-2*w)
  placed[i]=placed[i].lerp(hit,w)
# Gaborik embeds narrow lash strips in the head stream. Advect their native
# offset from surrounding skin instead of flattening strips onto the UV slit.
lash_ids={i for i,(u,v) in enumerate(duv) if .3273<v<.3286 and (.367<u<.414 or .585<u<.633)}
skin_tris=[t for t in donor['triangles'] if not any(i in lash_ids for i in t)]
skin_tree=BVHTree.FromPolygons(dp,skin_tris,all_triangles=True)
for i in lash_ids:
 hit,_,ti,_=skin_tree.find_nearest(dp[i]);ids=skin_tris[ti]
 placed[i]=dp[i]+barycentric_transform(hit,*[dp[j] for j in ids],*[placed[j] for j in ids])-hit
bpy.ops.wm.read_factory_settings(use_empty=True)
def mesh_object(name,ps,tris,uv=None):
 m=bpy.data.meshes.new(name);m.from_pydata(ps,[],tris);m.update()
 if uv is not None:
  layer=m.uv_layers.new(name='UVMap')
  for l in m.loops:layer.data[l.index].uv=uv[l.vertex_index]
 ob=bpy.data.objects.new(name,m);bpy.context.collection.objects.link(ob)
 for p in m.polygons:p.use_smooth=True
 return ob
head=mesh_object('Demitra_NHL19',placed,donor['triangles'],[(u,1-v) for u,v in duv])
control=mesh_object('Gaborik_Control',dp,donor['triangles']);control.hide_render=True
reference=mesh_object('Demitra_CC5_Reference',[p+shift for p in sp],cc['triangles']);reference.hide_render=True
eye_objects=[]
for name,negative in [('geo_reyeShape',True),('geo_leyeShape',False)]:
 orig=next(m for m in native['submeshes'] if m['name']==name)
 old=np.array([[p[0],-p[2],p[1]] for p in orig['positions']])
 target=source_eyes[(source_eyes[:,0]<0)==negative]
 sc=(target.min(0)+target.max(0))/2+np.array(shift);dc=(old.min(0)+old.max(0))/2
 scale=(target.max(0)-target.min(0))/(old.max(0)-old.min(0))
 ps=(old-dc)*scale+sc
 eye_objects.append(mesh_object(name,ps,orig['triangles']))
scene=bpy.context.scene;scene.render.engine='BLENDER_WORKBENCH';scene.display.shading.light='STUDIO'
scene.render.resolution_x=720;scene.render.resolution_y=900;scene.render.resolution_percentage=100
cam=bpy.data.cameras.new('Review');scene.camera=bpy.data.objects.new('Review',cam);scene.collection.objects.link(scene.camera)
cam.type='ORTHO';cam.ortho_scale=.43;focus=Vector((0,-.02,1.78))
for view,d in [('front',Vector((0,-1,0))),('profile',Vector((1,0,0))),('threequarter',Vector((.65,-1,0)))]:
 scene.camera.location=focus+d;scene.camera.rotation_euler=(focus-scene.camera.location).to_track_quat('-Z','Y').to_euler()
 for tag,ob in [('fit',head),('source',reference)]:
  head.hide_render=ob!=head;reference.hide_render=ob!=reference
  scene.render.filepath=str(out/(view+'_'+tag+'.png'));bpy.ops.render.render(write_still=True)
head.hide_render=False;reference.hide_render=True
native['submeshes'][native['submeshes'].index(donor)]['positions']=[[p.x,p.z,-p.y] for p in placed]
(out/'fit_native.json').write_text(json.dumps(native))
(out/'uv_warp.json').write_text(json.dumps({'pairs_pixels':pairs,'coefficients':coef.tolist(),'source_shift':list(shift),'placement':placement,'oral_ids':oral,'max_uv_projection_distance':max(distances)},indent=2))
bpy.ops.wm.save_as_mainfile(filepath=str(out/'Demitra_UV_Fit.blend'))
print('FIT',len(dp),'source',len(sp),'maxUVgap',max(distances),'oral',len(oral),'shift',list(shift))
