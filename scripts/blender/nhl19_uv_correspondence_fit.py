"""Test shared UV-template correspondence, with topology evidence before fitting.
Blender --background --factory-startup --python-exit-code 1 --python this.py -- out
No donor welding/reindexing; source position/UV equivalence is diagnostic only.
"""
import bpy, sys, os, json, hashlib
import numpy as np
from mathutils import Vector
from mathutils.kdtree import KDTree
from mathutils.bvhtree import BVHTree
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
import nhl19_data_transfer_test as util

out=sys.argv[sys.argv.index('--')+1];os.makedirs(out,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=r'H:\TEMP\nhl19-addon-comparison\builtin\Eichel_Mogilny_DataTransfer_Test.blend')
donor=list(bpy.data.collections['01_EICHEL_CONTROL'].objects)
source=list(bpy.data.collections['02_MOGILNY_REFERENCE'].objects)
head=util.find(donor,'geo_headShape');ref=util.find(source,'head_3_0_head_mesh0')
original=[v.co.copy() for v in head.data.vertices]
polygons=[tuple(p.vertices) for p in head.data.polygons]

def records(ob):
    points=[];uvs=[];tris=[];lookup={}
    for p in ob.data.polygons:
        face=[]
        for l in p.loop_indices:
            co=ob.data.vertices[ob.data.loops[l].vertex_index].co
            uv=ob.data.uv_layers.active.data[l].uv
            key=tuple(co)+tuple(uv)
            if key not in lookup:
                lookup[key]=len(points);points.append(co.copy());uvs.append(uv.copy())
            face.append(lookup[key])
        tris.append(tuple(face))
    return points,uvs,tris
sp,suv,st=records(ref)
tree=KDTree(len(suv))
for i,uv in enumerate(suv):tree.insert((uv.x,uv.y,0),i)
tree.balance()
duv={}
for p in head.data.polygons:
    for l in p.loop_indices:
        i=head.data.loops[l].vertex_index;uv=head.data.uv_layers.active.data[l].uv.copy()
        assert i not in duv or (duv[i]-uv).length<1e-7
        duv[i]=uv
matches={};ambiguous=[];uv_dist={}
for i,uv in duv.items():
    nearest=tree.find((uv.x,uv.y,0))[2]
    if nearest>.0005:continue
    candidates=tree.find_range((uv.x,uv.y,0),nearest+1e-7)
    if not candidates:continue
    candidates.sort(key=lambda r:r[2]);sid=candidates[0][1]
    if max((sp[j]-sp[sid]).length for _,j,_ in candidates)>.0001:
        ambiguous.append(i);continue
    matches[i]=sid;uv_dist[i]=candidates[0][2]
# Cross-check entire triangles, not merely similar texture coordinates.
source_faces={tuple(sorted(t)) for t in st}
verified_faces=[];disagree=[]
for f in polygons:
    if all(i in matches for i in f):
        if tuple(sorted(matches[i] for i in f)) in source_faces:verified_faces.append(f)
        else:disagree.append(f)
trusted={i for f in verified_faces for i in f}
# A triangulated quad may use the other diagonal. Confirm shared neighboring edges
# as additional evidence, without requiring each triangle diagonal to agree.
source_edges={tuple(sorted((a,b))) for f in st for a,b in zip(f,f[1:]+f[:1])}
donor_neighbors=[set() for _ in original]
for f in polygons:
    for a,b in zip(f,f[1:]+f[:1]):donor_neighbors[a].add(b);donor_neighbors[b].add(a)
edge_supported={i for i,sid in matches.items() if sum(
    j in matches and tuple(sorted((sid,matches[j]))) in source_edges
    for j in donor_neighbors[i])>=2}
trusted.update(edge_supported)
report={'source_records':len(ref.data.vertices),'source_position_uv_records':len(sp),
 'donor_records':len(original),'uv_tolerance':.0005,'uv_matches':len(matches),
 'ambiguous_uv_vertices':ambiguous,'verified_shared_triangles':len(verified_faces),
 'fully_matched_nonshared_triangles':len(disagree),'topology_supported_vertices':len(trusted),
 'shared_edge_supported_vertices':len(edge_supported)}
print('CORRESPONDENCE',json.dumps(report),flush=True)
if len(trusted)<2000:raise RuntimeError('Insufficient shared-template evidence')

# All topology-supported points take the actual Mogilny coordinate at full strength.
# Unsupported NHL19 vertices inherit a harmonic displacement through donor edges.
# This moves donor-only interior geometry without projecting it onto exterior skin.
adj=[set() for _ in original]
for f in polygons:
    for i,j in zip(f,f[1:]+f[:1]):adj[i].add(j);adj[j].add(i)
fixed={i:sp[matches[i]]-original[i] for i in trusted}
# Attach unsupported cavity/rim copies to a verified coincident exterior point.
# Keep their IDs and connectivity separate; do not weld upper and lower lip loops.
coincident={}
for i,p in enumerate(original):coincident.setdefault(tuple(p),[]).append(i)
seam_attached=[]
for ids in coincident.values():
    known=[i for i in ids if i in trusted]
    if known and max((sp[matches[i]]-sp[matches[known[0]]]).length for i in known)<1e-5:
        for i in ids:
            if i not in trusted:
                fixed[i]=fixed[known[0]].copy();seam_attached.append(i)
report['unsupported_seam_vertices_attached']=seam_attached
# The lower attachment remains native, with a transition below the facial region.
with open(r'E:\tmp\nhl19-mogilny\out\fit\masks.json') as f:old_masks=json.load(f)
shell={i for i in old_masks['masks']['shell_boundary']['ids'] if original[i].z<1.70}
for i in shell:fixed[i]=Vector()
disp=[fixed.get(i,Vector()).copy() for i in range(len(original))]
for iteration in range(800):
    old=[d.copy() for d in disp]
    for i in range(len(original)):
        if i not in fixed and adj[i]:disp[i]=sum((old[j] for j in adj[i]),Vector())/len(adj[i])
fitted=[]
for i,(p,d) in enumerate(zip(original,disp)):
    weight=util.smoothstep(1.66,1.72,p.z)
    fitted.append(p+d*weight)
report['unsupported_vertex_ids']=sorted(set(range(len(original)))-trusted)
report['quality']=util.normals_and_quality(original,fitted,head.data.polygons)
report['max_supported_face_error_mm']=max((fitted[i]-sp[matches[i]]).length*1000 for i in trusted if original[i].z>1.72 and i not in shell)
report['matched_uv_max_error']=max(uv_dist.values())
report['matched_native_to_source']={str(i):matches[i] for i in sorted(trusted)}
report['coincident_groups_over_point1mm']=[ids for ids in coincident.values() if len(ids)>1 and max((fitted[i]-fitted[ids[0]]).length for i in ids)>.0001]
# Measure source-to-result facial surface distance, independent of the matched list.
source_bvh=BVHTree.FromPolygons(sp,st)
fit_bvh=BVHTree.FromPolygons(fitted,polygons)
surface_errors=[]
for xyz in set(tuple(p) for p in sp):
    p=Vector(xyz)
    if p.z<1.72 or abs(p.x)>.085 or p.y>-.025:continue
    front=source_bvh.ray_cast(Vector((p.x,-1,p.z)),Vector((0,1,0)),2)[0]
    if front is not None and (front-p).length<.0002:
        surface_errors.append(fit_bvh.find_nearest(p)[3]*1000)
report['source_visible_face_to_fit_mm']={'samples':len(surface_errors),
 'rms':float(np.sqrt(np.mean(np.square(surface_errors)))),
 'p95':float(np.quantile(surface_errors,.95)), 'max':max(surface_errors)}
report['source_glb_sha256']=hashlib.sha256(open(r'S:\Downloads\Mogilny Test\alex_mogilny.glb','rb').read()).hexdigest()
report['donor_glb_sha256']=hashlib.sha256(open(r'H:\TEMP\nhl19-eichel-glb\jack_eichel_native.glb','rb').read()).hexdigest()

obs,coll=util.duplicate(donor,'09_UV_TEMPLATE_FIT');h=util.find(obs,'geo_headShape')
h.shape_key_add(name='Basis');key=h.shape_key_add(name='Mogilny_Template');key.value=1
for v,p in zip(key.data,fitted):v.co=p
for name,ids in [('VerifiedCorrespondence',trusted),('DonorOnlyOrUnverified',set(range(len(original)))-trusted)]:
    g=h.vertex_groups.new(name=name)
    if ids:g.add(sorted(ids),1,'REPLACE')
deyes=[util.midpoint([v.co for v in util.find(donor,n).data.vertices]) for n in ('geo_reyeShape','geo_leyeShape')]
seyes=util.find(source,'head_3_1_eyes_mesh1')
scentres=[util.midpoint([v.co for v in seyes.data.vertices if (v.co.x<0)==neg]) for neg in (True,False)]
for name,a,b in zip(('geo_reyeShape','geo_leyeShape'),deyes,scentres):
    for v in util.find(obs,name).data.vertices:v.co+=b-a
for v in util.find(obs,'geo_eyelashesShape').data.vertices:
    j=min(range(2),key=lambda j:(v.co-deyes[j]).length);v.co+=scentres[j]-deyes[j]
bpy.context.view_layer.update();dg=bpy.context.evaluated_depsgraph_get();baked=[]
def smooth_geometry(mesh):
    # Recompute preview/export normals from the new shape, replacing donor normals.
    sums={}
    for p in mesh.polygons:
        a,b,c=[mesh.vertices[i].co for i in p.vertices[:3]]
        n=(b-a).cross(c-a)
        for i in p.vertices:
            key=tuple(mesh.vertices[i].co);sums[key]=sums.get(key,Vector())+n
        p.use_smooth=True
    normals=[sums[tuple(v.co)].normalized() for v in mesh.vertices]
    mesh.normals_split_custom_set_from_vertices(normals)
for ob in obs:
    mesh=bpy.data.meshes.new_from_object(ob.evaluated_get(dg))
    smooth_geometry(mesh)
    copy=bpy.data.objects.new(ob.name+'_BAKED',mesh);bpy.context.scene.collection.objects.link(copy);baked.append(copy)
util.select_only(baked,util.find(baked,'geo_headShape'))
glb=os.path.join(out,'Mogilny_NHL19_UV_Template.glb')
bpy.ops.export_scene.gltf(filepath=glb,use_selection=True,export_morph=False,export_animations=False)
fitted_preview,preview_coll=util.duplicate(baked,'11_FITTED_GEOMETRY_NORMALS')
for ob in baked:bpy.data.objects.remove(ob,do_unlink=True)
check,_=util.import_glb(glb,'VERIFY_EXPORT');checkhead=util.find(check,'geo_headShape')
def point_error(a,b):
    t=KDTree(len(b))
    for i,p in enumerate(b):t.insert(p,i)
    t.balance();return max(t.find(p)[2] for p in a)
actual=[v.co.copy() for v in checkhead.data.vertices]
report['export_bidirectional_error_mm']=max(point_error(actual,fitted),point_error(fitted,actual))*1000
assert report['export_bidirectional_error_mm']<.001
for ob in check:bpy.data.objects.remove(ob,do_unlink=True)
assert [tuple(p.vertices) for p in h.data.polygons]==polygons and len(h.data.vertices)==3157
report['native_topology_unchanged']=True

# Smooth the source reference on a temporary preview copy only, so the reference
# and donor are not compared with faceted versus smooth shading.
preview,pc=util.duplicate(source,'10_SMOOTH_REFERENCE_PREVIEW')
for ob in preview:
    util.select_only([ob],ob)
    bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.mesh.remove_doubles(threshold=.0000001);bpy.ops.object.mode_set(mode='OBJECT')
    if ob.data.has_custom_normals:
        bpy.ops.mesh.customdata_custom_splitnormals_clear()
    for p in ob.data.polygons:p.use_smooth=True
for ob in preview:smooth_geometry(ob.data)
groups=[('donor',donor),('fitted',fitted_preview),('source',preview)]
scene=bpy.context.scene;centre=Vector((0,-.065,1.815))
scene.camera.data.ortho_scale=.34
for view,direction in [('front',Vector((0,-1,0))),('threequarter',Vector((.65,-1,0))),('profile',Vector((1,0,0)))]:
    scene.camera.location=centre+direction
    scene.camera.rotation_euler=(centre-scene.camera.location).to_track_quat('-Z','Y').to_euler()
    for tag,visible in groups:
        for ob in bpy.data.objects:
            if ob.type=='MESH':
                ob.hide_render=ob not in visible or any(t in ob.name for t in ('cardsA','cardsB','haircap','eyelashes'))
                ob.hide_set(False)
        scene.render.filepath=os.path.join(out,view+'_'+tag+'.png');bpy.ops.render.render(write_still=True)
for ob in bpy.data.objects:
    if ob.type=='MESH':ob.hide_set(ob not in obs);ob.hide_render=ob not in obs
report['limitations']='Static geometric correspondence; eye radius/lashes and donor-only interior need review. No rig or animation validation. Source preview alone is welded for shading; donor never welded.'
with open(os.path.join(out,'report.json'),'w') as f:json.dump(report,f,indent=2)
bpy.data.texts.new('UV_TEMPLATE_REPORT').write(json.dumps(report,indent=2))
bpy.ops.wm.save_as_mainfile(filepath=os.path.join(out,'Mogilny_UV_Template_Test.blend'))
print('UV_FIT_COMPLETE',json.dumps({k:v for k,v in report.items() if k not in ('matched_native_to_source','unsupported_vertex_ids')}))
