"""Offline landmark-guided outer-skin projection, retaining native donor topology.
Run in Blender background with --python-exit-code 1 --python this.py -- output_dir.
Landmarks/masks are geometric proposals, not verified facial rig labels.
"""
import bpy, sys, os, json
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.kdtree import KDTree
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nhl19_data_transfer_test as util

out = sys.argv[sys.argv.index('--')+1]
os.makedirs(out, exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=r'H:\TEMP\nhl19-addon-comparison\builtin\Eichel_Mogilny_DataTransfer_Test.blend')
donor = list(bpy.data.collections['01_EICHEL_CONTROL'].objects)
source = list(bpy.data.collections['02_MOGILNY_REFERENCE'].objects)
head = util.find(donor, 'geo_headShape')
target = util.find(source, 'head_3_0_head_mesh0')
original = [v.co.copy() for v in head.data.vertices]
triangles = [tuple(p.vertices) for p in head.data.polygons]
with open(r'E:\tmp\nhl19-mogilny\out\fit\masks.json') as f: masks = json.load(f)
assert masks['vertex_count'] == len(original) == 3157
mask = lambda name: set(masks['masks'][name]['ids'])
cavity = mask('oral_cavity')
shell = mask('shell_boundary') | mask('shell_transition')
def bvh(ob):
    return BVHTree.FromPolygons([v.co for v in ob.data.vertices], [p.vertices[:] for p in ob.data.polygons])
db, sb = bvh(head), bvh(target)
def front(tree, x, z):
    p, n, index, distance = tree.ray_cast(Vector((x,-1,z)), Vector((0,1,0)), 2)
    return p
dm = Vector((-.000764121,-.112065010,1.773568511))
sm = dm.copy()
eyes_d = [util.midpoint([v.co for v in util.find(donor,name).data.vertices]) for name in ('geo_reyeShape','geo_leyeShape')]
eyes_s_ob = util.find(source,'head_3_1_eyes_mesh1')
eyes_s = [util.midpoint([v.co for v in eyes_s_ob.data.vertices if (v.co.x < 0)==negative]) for negative in (True,False)]
de, se = sum(eyes_d, Vector())/2, sum(eyes_s, Vector())/2
anchors = []
def anchor(name, a, b):
    if a is not None and b is not None: anchors.append((name,a.copy(),b.copy()))
for side,(a,b) in enumerate(zip(eyes_d,eyes_s)):
    anchor(f'eye_centre_{side}',a,b)
    for label,dx,dz in [('inner',-.012 if side else .012,0),('outer',.014 if side else -.014,0),('upper',0,.006),('lower',0,-.005)]:
        anchor(f'eyelid_{side}_{label}',front(db,a.x+dx,a.z+dz),front(sb,b.x+dx,b.z+dz))
for label,dx,dz in [('upper_lip',0,.002),('lower_lip',0,-.002),('mouth_left',-.024,0),('mouth_right',.024,0),('chin',0,-.038)]:
    anchor(label,front(db,dm.x+dx,dm.z+dz),front(sb,sm.x+dx,sm.z+dz))
for label,dx,fraction in [('nose_tip',0,.48),('nostril_left',-.013,.34),('nostril_right',.013,.34),('bridge',0,.9),('cheek_left',-.05,.45),('cheek_right',.05,.45)]:
    anchor(label,front(db,dm.x+dx,dm.z+(de.z-dm.z)*fraction),front(sb,sm.x+dx,sm.z+(se.z-sm.z)*fraction))
# Ear extrema are provisional landmarks; no claim that an extreme is a homologous vertex.
for sign in (-1,1):
    def ear(ob, eye):
        return max((v.co for v in ob.data.vertices if eye.z-.05<v.co.z<eye.z+.015), key=lambda p: sign*p.x)
    anchor(f'ear_{sign}',ear(head,de),ear(target,se))
for i in sorted(shell)[::12]: anchor(f'shell_{i}',original[i],original[i])
centres = np.array([a for _,a,_ in anchors])
delta = np.array([b-a for _,a,b in anchors])
radius=.045
def kernel(a,b): return np.exp(-np.sum((a[:,None,:]-b[None,:,:])**2,axis=2)/(2*radius**2))
weights = np.linalg.solve(kernel(centres,centres)+np.eye(len(centres))*.05,delta)
displacements = kernel(np.array(original),centres)@weights
anchored = [p+Vector(d) for p,d in zip(original,displacements)]
# Interiors receive only the regularized anchor deformation, never projection.
# The conservative cavity mask includes other occluded facial surfaces; freezing
# it rigidly would tear its transition to the moving exterior.
for i,p in enumerate(original):
    anchored[i]=p.lerp(anchored[i],util.smoothstep(dm.z-.10,dm.z-.05,p.z))
for i in shell: anchored[i] = original[i].copy()
# Exclude the conservative oral cavity mask and any vertex hidden behind donor front
# skin in the facial box. This also protects nostril interiors from nearest-skin collapse.
excluded = set(cavity)|shell
for i,p in enumerate(original):
    if abs(p.x)<.073 and dm.z-.045<p.z<de.z+.023:
        hit=front(db,p.x,p.z)
        if hit is not None and p.y-hit.y>.0015: excluded.add(i)
eligible=[]; projected=[p.copy() for p in anchored]
for i,p in enumerate(anchored):
    if i in excluded or p.z<dm.z-.065: continue
    hit,n,face,distance=sb.find_nearest(p)
    if hit is None or distance>.008: continue
    # Keep upper/lower lip proposals on their respective side of the mouth seam.
    if abs(p.x-dm.x)<.032 and abs(p.z-dm.z)<.012:
        if (p.z-dm.z)*(hit.z-sm.z)<0: continue
    # Reject a local projection with a strongly opposing target surface normal.
    if head.data.vertices[i].normal.dot(n)<.25: continue
    projected[i]=hit.copy();eligible.append(i)
# Bound fold creation relative to the anchored mesh using a global line search.
alpha=1.
def bad(a,b):
    return util.normals_and_quality(a,b,head.data.polygons)
while alpha>1/128:
    candidate=[a.lerp(b,alpha) for a,b in zip(anchored,projected)]
    q=bad(anchored,candidate)
    if q['orientation_reversals']==0 and q['collapsed_triangles']==0: break
    alpha*=.5
projected=[a.lerp(b,alpha) for a,b in zip(anchored,projected)]
report={'method':'Gaussian landmark displacement then skin-only bounded nearest-surface projection',
 'landmark_status':'provisional geometric ray samples, eye bounds centres and ear extrema; manual validation still needed',
 'anchors':[{'name':n,'donor':list(a),'target':list(b)} for n,a,b in anchors],
 'oral_cavity_mask_count':len(cavity),'excluded_total':len(excluded),'projection_vertices':len(eligible),
 'projection_limit_mm':8,'projection_blend_after_fold_guard':alpha,
 'anchored_vs_donor':bad(original,anchored),'projected_vs_donor':bad(original,projected),
 'projection_vs_anchored':bad(anchored,projected),'native_vertex_count':len(original)}
coll=bpy.data.collections.new('06_ANATOMICAL_ANCHORS');bpy.context.scene.collection.children.link(coll)
for name,a,b in anchors:
    if name.startswith('shell'):continue
    for suffix,p in [('donor',a),('target',b)]:
        ob=bpy.data.objects.new(name+'_'+suffix,None);coll.objects.link(ob)
        ob.location=p;ob.empty_display_type='SPHERE';ob.empty_display_size=.0015
variants=[]
for label,positions in [('07_ANCHORED',anchored),('08_PROJECTED',projected)]:
    obs,coll=util.duplicate(donor,label);h=util.find(obs,'geo_headShape')
    h.shape_key_add(name='Basis');key=h.shape_key_add(name='SelectedResult');key.value=1
    for v,p in zip(key.data,positions):v.co=p
    g=h.vertex_groups.new(name='ExcludedFromProjection')
    g.add(sorted(excluded),1,'REPLACE')
    for name,a,b in zip(('geo_reyeShape','geo_leyeShape'),eyes_d,eyes_s):
        for v in util.find(obs,name).data.vertices:v.co+=b-a
    for v in util.find(obs,'geo_eyelashesShape').data.vertices:
        j=min(range(2),key=lambda j:(v.co-eyes_d[j]).length);v.co+=eyes_s[j]-eyes_d[j]
    variants.append((label,obs))
    bpy.context.view_layer.update();dg=bpy.context.evaluated_depsgraph_get();baked=[]
    for ob in obs:
        mesh=bpy.data.meshes.new_from_object(ob.evaluated_get(dg))
        copy=bpy.data.objects.new(ob.name+'_BAKED',mesh);bpy.context.scene.collection.objects.link(copy);baked.append(copy)
    util.select_only(baked,util.find(baked,'geo_headShape'))
    path=os.path.join(out,label+'.glb')
    bpy.ops.export_scene.gltf(filepath=path,use_selection=True,export_morph=False,export_animations=False)
    for ob in baked:bpy.data.objects.remove(ob,do_unlink=True)
    check,checkcoll=util.import_glb(path,'VERIFY');checkhead=util.find(check,'geo_headShape')
    tree=KDTree(len(positions))
    for i,p in enumerate(positions):tree.insert(p,i)
    tree.balance();err=max(tree.find(v.co)[2] for v in checkhead.data.vertices)
    assert err<1e-6
    report[label+'_export_max_error_mm']=err*1000
    for ob in check:bpy.data.objects.remove(ob,do_unlink=True)
raw=list(bpy.data.collections['04_TWO_PASS_BLEND'].objects)
scene=bpy.context.scene
groups=[('raw',raw),('anchored',variants[0][1]),('projected',variants[1][1]),('source',source)]
centre=Vector((0,-.065,de.z-.025))
for view,direction in [('front',Vector((0,-1,0))),('threequarter',Vector((.65,-1,0))),('profile',Vector((1,0,0)))]:
    scene.camera.location=centre+direction
    scene.camera.rotation_euler=(centre-scene.camera.location).to_track_quat('-Z','Y').to_euler()
    for tag,visible in groups:
        for ob in bpy.data.objects:
            if ob.type=='MESH':
                ob.hide_render=ob not in visible or any(t in ob.name for t in ('cardsA','cardsB','haircap'))
                ob.hide_set(False)
        scene.render.filepath=os.path.join(out,view+'_'+tag+'.png')
        bpy.ops.render.render(write_still=True)
for ob in bpy.data.objects:
    if ob.type=='MESH':ob.hide_set(ob not in variants[1][1]);ob.hide_render=ob not in variants[1][1]
report['topology_unchanged']=all([tuple(p.vertices) for p in util.find(obs,'geo_headShape').data.polygons]==triangles for _,obs in variants)
assert report['topology_unchanged']
with open(os.path.join(out,'report.json'),'w') as f:json.dump(report,f,indent=2)
bpy.data.texts.new('ANCHORED_PROJECTION_REPORT').write(json.dumps(report,indent=2))
bpy.ops.wm.save_as_mainfile(filepath=os.path.join(out,'Anchored_Projection_Test.blend'))
print('PROJECTION_COMPLETE',json.dumps({k:v for k,v in report.items() if k!='anchors'}))
