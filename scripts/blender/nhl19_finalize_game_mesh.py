"""Finalize the measured UV fit for a native static in-game experiment.
Blender --background --factory-startup --python-exit-code 1 --python this.py -- out
"""
import bpy,sys,os,json,argparse
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.geometry import barycentric_transform
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
import nhl19_data_transfer_test as util
from head_scalp_placement import place_head
parser=argparse.ArgumentParser()
parser.add_argument('out')
parser.add_argument('--texture')
parser.add_argument('--keep-mouth-height',action='store_true',help='Reproduce the first game test')
args=parser.parse_args(sys.argv[sys.argv.index('--')+1:])
out=args.out;os.makedirs(out,exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=r'H:\TEMP\nhl19-mogilny-uv-template-final\Mogilny_UV_Template_Test.blend')
donor=list(bpy.data.collections['01_EICHEL_CONTROL'].objects)
fitted=list(bpy.data.collections['09_UV_TEMPLATE_FIT'].objects)
source=list(bpy.data.collections['02_MOGILNY_REFERENCE'].objects)
obs,coll=util.duplicate(fitted,'12_GAME_TEST_FINAL')
head=util.find(obs,'geo_headShape');basehead=util.find(donor,'geo_headShape')
coords=[v.co.copy() for v in head.data.shape_keys.key_blocks['Mogilny_Template'].data]
head.shape_key_clear()
for v,p in zip(head.data.vertices,coords):v.co=p
with open(r'H:\TEMP\nhl19-mogilny-uv-template-final\report.json') as f:fit_report=json.load(f)
seams=[]
for ids in fit_report['coincident_groups_over_point1mm']:
    target=sum((head.data.vertices[i].co for i in ids),Vector())/len(ids)
    seams.append({'ids':ids,'max_adjustment_mm':max((head.data.vertices[i].co-target).length*1000 for i in ids)})
    for i in ids:head.data.vertices[i].co=target

shift=Vector();placement={'mode':'historical mouth alignment'}
if not args.keep_mouth_height:
    # Destination-specific lower neck/chest band; facial proportions stay rigid.
    placed,shift,placement=place_head([v.co.copy() for v in basehead.data.vertices],
                                     [v.co.copy() for v in head.data.vertices],1.68,1.72)
    for v,p in zip(head.data.vertices,placed):v.co=p

if args.texture:
    import numpy as np
    image=bpy.data.images.load(os.path.abspath(args.texture),check_existing=False)
    image.colorspace_settings.name='Non-Color'
    pixels=np.empty(len(image.pixels),np.float32);image.pixels.foreach_get(pixels)
    rgba=np.flipud(pixels.reshape(image.size[1],image.size[0],4)).copy()
    np.save(os.path.join(out,'source_rgba.npy'),rgba)
    image.colorspace_settings.name='sRGB';image.pack()
    material=bpy.data.materials.new('Mogilny supplied diffuse');material.use_nodes=True
    tex=material.node_tree.nodes.new('ShaderNodeTexImage');tex.image=image
    material.node_tree.links.new(tex.outputs['Color'],material.node_tree.nodes.get('Principled BSDF').inputs['Base Color'])
    head.data.materials.clear();head.data.materials.append(material)
def tree(ob):return BVHTree.FromPolygons([v.co for v in ob.data.vertices],[p.vertices[:] for p in ob.data.polygons])
db=tree(basehead);hb=tree(head)
report={'placement':placement,'seam_closures':seams,'hair':{},'eyes':{},'native_animation':'opaque data preserved, runtime validation required'}
# Lash roots follow the actual eyelid deformation rather than only eye translation.
lash=util.find(obs,'geo_eyelashesShape');oldlash=util.find(donor,'geo_eyelashesShape')
for v,old in zip(lash.data.vertices,oldlash.data.vertices):
    p,n,index,d= db.find_nearest(old.co)
    ids=basehead.data.polygons[index].vertices[:3]
    a,b,c=[basehead.data.vertices[i].co for i in ids]
    x,y,z=[head.data.vertices[i].co for i in ids]
    v.co=old.co+barycentric_transform(p,a,b,c,x,y,z)-p
# Match source eye bounds while retaining native sphere topology and UVs.
se=util.find(source,'head_3_1_eyes_mesh1')
for name,negative in [('geo_reyeShape',True),('geo_leyeShape',False)]:
    ob=util.find(obs,name);old=util.find(donor,name)
    src=[v.co for v in se.data.vertices if (v.co.x<0)==negative]
    dst=[v.co for v in old.data.vertices]
    sc=util.midpoint(src)+shift;dc=util.midpoint(dst)
    scale=Vector(tuple((max(p[k] for p in src)-min(p[k] for p in src))/(max(p[k] for p in dst)-min(p[k] for p in dst)) for k in range(3)))
    for v,p in zip(ob.data.vertices,dst):v.co=sc+Vector(tuple((p[k]-dc[k])*scale[k] for k in range(3)))
    report['eyes'][name]={'scale':list(scale),'centre':list(sc)}
# Radially conform all native hair layers to the new head, then shrink them inward.
# This preserves their vertices/material bindings rather than changing visibility flags.
centre=Vector((0,0,1.87))+shift
for name in ('geo_haircapShape','geo_cardsAShape','geo_cardsBShape'):
    ob=util.find(obs,name);clearances=[]
    for v in ob.data.vertices:
        direction=v.co-centre
        direction.z=max(.015,direction.z) # Fold hanging cards onto the scalp dome.
        direction.normalize();hit,n,index,d=hb.ray_cast(centre,direction,1.)
        if hit is None:raise RuntimeError('No scalp intersection for '+name)
        v.co=centre+(hit-centre)*.92
        # Positive radial clearance confirms the point lies behind the scalp hit.
        clearances.append((hit-v.co).length*1000)
    report['hair'][name]={'vertices':len(ob.data.vertices),'radial_scale':.92,'min_radial_clearance_mm':min(clearances)}
    assert min(clearances)>3

payload={'donor_sha256':'4da8f5d5c9eaa64f3aaa38b7e9e4cb0b0aea74b91a1884ff863bf0cc61243195','submeshes':[]}
for ob in obs:
    old=next(o for o in donor if o.name in ob.name)
    native_name=next(n for n in ('geo_headShape','geo_reyeShape','geo_leyeShape','geo_eyelashesShape','geo_haircapShape','geo_cardsAShape','geo_cardsBShape') if n in ob.name)
    assert len(ob.data.vertices)==len(old.data.vertices)
    assert [tuple(p.vertices) for p in ob.data.polygons]==[tuple(p.vertices) for p in old.data.polygons]
    # Preserve split shading boundaries by averaging per native vertex, not welding.
    util.select_only([ob],ob)
    if ob.data.has_custom_normals:bpy.ops.mesh.customdata_custom_splitnormals_clear()
    for p in ob.data.polygons:p.use_smooth=True
    ob.data.update();bpy.context.view_layer.update()
    uv=[None]*len(ob.data.vertices)
    for loop in ob.data.loops:
        t=ob.data.uv_layers.active.data[loop.index].uv
        uv[loop.vertex_index]=[t.x,1-t.y] # Back to stored glTF/native V.
    payload['submeshes'].append({'name':native_name,
      'original_positions':[list(util.native(v.co)) for v in old.data.vertices],
      'positions':[list(util.native(v.co)) for v in ob.data.vertices],
      'normals':[list(util.native(v.normal)) for v in ob.data.vertices],
      'uv':uv,'triangles':[list(p.vertices) for p in ob.data.polygons]})
report['head_quality']=util.normals_and_quality([v.co for v in basehead.data.vertices],[v.co for v in head.data.vertices],head.data.polygons)
assert report['head_quality']['collapsed_triangles']==0
with open(os.path.join(out,'native_mesh.json'),'w') as f:json.dump(payload,f)
with open(os.path.join(out,'mesh_report.json'),'w') as f:json.dump(report,f,indent=2)
util.select_only(obs,head)
bpy.ops.export_scene.gltf(filepath=os.path.join(out,'Mogilny_Bald_GameTest.glb'),use_selection=True,export_morph=False,export_animations=False)
scene=bpy.context.scene;focus=Vector((0,-.025,1.775));scene.camera.data.ortho_scale=.43
for ob in bpy.data.objects:
    if ob.type=='MESH':ob.hide_render=ob not in obs;ob.hide_set(ob not in obs)
for view,d in [('front',Vector((0,-1,0))),('threequarter',Vector((.65,-1,0))),('profile',Vector((1,0,0)))]:
    scene.camera.location=focus+d;scene.camera.rotation_euler=(focus-scene.camera.location).to_track_quat('-Z','Y').to_euler()
    scene.render.filepath=os.path.join(out,view+'_all_components.png');bpy.ops.render.render(write_still=True)
    # Identical camera and framing make crown and neck comparison reviewable.
    for ob in bpy.data.objects:
        if ob.type=='MESH':ob.hide_render=ob not in donor or any(n in ob.name for n in ('haircap','cardsA','cardsB'))
    scene.render.filepath=os.path.join(out,view+'_eichel.png');bpy.ops.render.render(write_still=True)
    for ob in bpy.data.objects:
        if ob.type=='MESH':ob.hide_render=ob not in obs
bpy.ops.wm.save_as_mainfile(filepath=os.path.join(out,'Mogilny_Bald_GameTest.blend'))
print('FINAL_MESH',json.dumps(report))
