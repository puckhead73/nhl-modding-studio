"""Blender 4.5 native Mesh Data Transfer position experiment.

Run with blender --background --python this.py -- <Eichel.glb> <Mogilny.glb> <out>.
Uses Blender's actual object.data_transfer operator, Nearest Face Interpolated.
Positions are encoded as three temporary [0,1] vertex-group channels because
Blender's built-in Data Transfer does not directly offer a POSITION data type.
No external add-on, shrinkwrap, custom BVH nearest-point solver, or game writes.
"""
import bpy
import hashlib
import json
import math
import os
import sys
from mathutils import Vector, Matrix

TRANSFER_METHOD = 'Actual Blender object.data_transfer: VGROUP_WEIGHTS, POLYINTERP_NEAREST; normalized XYZ channels decoded to positions'


def native(v):
    return Vector((v.x, v.z, -v.y))


def blender(v):
    return Vector((v[0], -v[2], v[1]))


def select_only(objects, active):
    bpy.ops.object.select_all(action='DESELECT')
    for ob in objects:
        ob.hide_set(False)
        ob.select_set(True)
    bpy.context.view_layer.objects.active = active


def import_glb(path, collection_name):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path, merge_vertices=False)
    added = [o for o in bpy.data.objects if o not in before and o.type == 'MESH']
    collection = bpy.data.collections.new(collection_name)
    bpy.context.scene.collection.children.link(collection)
    for ob in added:
        # Bake import nodes exactly once into Z-up Blender world coordinates.
        transform = ob.matrix_world.copy()
        ob.parent = None
        ob.data.transform(transform)
        ob.matrix_world = Matrix.Identity(4)
        for old in list(ob.users_collection):
            old.objects.unlink(ob)
        collection.objects.link(ob)
    return added, collection


def duplicate(objects, name):
    coll = bpy.data.collections.new(name)
    bpy.context.scene.collection.children.link(coll)
    result = []
    for ob in objects:
        copy = ob.copy()
        copy.data = ob.data.copy()
        copy.name = name + '__' + ob.name
        coll.objects.link(copy)
        result.append(copy)
    return result, coll


def find(objects, token):
    return next(o for o in objects if token in o.name)


def midpoint(points):
    return Vector(tuple((min(p[k] for p in points) + max(p[k] for p in points)) * .5
                        for k in range(3)))


def smoothstep(a, b, x):
    t = max(0., min(1., (x-a)/(b-a)))
    return t*t*(3.-2.*t)


def transfer_positions(source, destination, sampling_shift, common_shift):
    """Find matches in sampling alignment, transfer coordinates in common frame."""
    src = source.copy()
    src.data = source.data.copy()
    bpy.context.scene.collection.objects.link(src)
    src.name = 'TEMP_transfer_source'
    dst = destination.copy()
    dst.data = destination.data.copy()
    bpy.context.scene.collection.objects.link(dst)
    dst.name = 'TEMP_transfer_destination'
    src.vertex_groups.clear()
    dst.vertex_groups.clear()
    original = [v.co.copy() for v in source.data.vertices]
    output = [p + common_shift for p in original]
    lo = Vector(tuple(min(p[k] for p in output) - .01 for k in range(3)))
    extent = Vector(tuple(max(p[k] for p in output) - lo[k] + .01 for k in range(3)))
    for k, name in enumerate(('DT_position_x', 'DT_position_y', 'DT_position_z')):
        group = src.vertex_groups.new(name=name)
        for i, p in enumerate(output):
            group.add([i], (p[k]-lo[k])/extent[k], 'REPLACE')
    src.location = sampling_shift
    bpy.context.view_layer.update()
    select_only([dst, src], src)
    result = bpy.ops.object.data_transfer(
        data_type='VGROUP_WEIGHTS', use_create=True,
        vert_mapping='POLYINTERP_NEAREST',
        use_object_transform=True, use_auto_transform=False,
        use_max_distance=False, layers_select_src='ALL', layers_select_dst='NAME',
        mix_mode='REPLACE', mix_factor=1.0)
    if result != {'FINISHED'}:
        raise RuntimeError('Blender Data Transfer failed: ' + str(result))
    groups = [dst.vertex_groups[n] for n in ('DT_position_x', 'DT_position_y', 'DT_position_z')]
    positions = []
    for v in dst.data.vertices:
        # Strict positive source channel values: missing groups cannot mimic valid zero.
        positions.append(Vector(tuple(lo[k]+groups[k].weight(v.index)*extent[k] for k in range(3))))
    bpy.data.objects.remove(src, do_unlink=True)
    bpy.data.objects.remove(dst, do_unlink=True)
    return positions


def normals_and_quality(original, result, polygons):
    reversed_count = collapsed = 0
    for poly in polygons:
        for j in range(1, len(poly.vertices)-1):
            ids = (poly.vertices[0], poly.vertices[j], poly.vertices[j+1])
            a,b,c = (original[i] for i in ids)
            d,e,f = (result[i] for i in ids)
            n0 = (b-a).cross(c-a)
            n1 = (e-d).cross(f-d)
            if n1.length < 1e-12:
                collapsed += 1
            elif n0.dot(n1) < 0:
                reversed_count += 1
    return {'orientation_reversals': reversed_count, 'collapsed_triangles': collapsed}


def main():
    args = sys.argv[sys.argv.index('--')+1:]
    if len(args) != 3:
        raise RuntimeError('Expected Eichel.glb Mogilny.glb output-directory')
    eichel_path, mogilny_path, out = args
    os.makedirs(out, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    donor, donor_coll = import_glb(eichel_path, '01_EICHEL_CONTROL')
    source, source_coll = import_glb(mogilny_path, '02_MOGILNY_REFERENCE')
    head = find(donor, 'geo_headShape')
    reference = find(source, 'head_3_0_head_mesh0')
    source_eyes = find(source, 'head_3_1_eyes_mesh1')
    original = [v.co.copy() for v in head.data.vertices]
    if len(original) != 3157:
        raise RuntimeError(f'Blender import changed expected head count: {len(original)}')
    # Donor landmark from the verified native frontmost midline lip-rim point.
    # Source landmark is a measured provisional closed-lip midline point. Both are
    # exposed as named empties and stored in the report for manual correction.
    donor_mouth = blender((-.000764121, 1.773568511, .112065010))
    source_mouth = blender((-.0002895, 1.653094, .136195))
    re = find(donor, 'geo_reyeShape')
    le = find(donor, 'geo_leyeShape')
    dr = midpoint([v.co for v in re.data.vertices])
    dl = midpoint([v.co for v in le.data.vertices])
    sr = midpoint([v.co for v in source_eyes.data.vertices if v.co.x < 0])
    sl = midpoint([v.co for v in source_eyes.data.vertices if v.co.x >= 0])
    de = (dr+dl)*.5
    se = (sr+sl)*.5
    mouth_shift = donor_mouth-source_mouth
    eye_shift = de-se
    common_shift = mouth_shift.copy()  # Retains Mogilny's mouth-to-eye length.
    report = {
        'blender_version': bpy.app.version_string,
        'method': TRANSFER_METHOD,
        'source_sha256': hashlib.sha256(open(mogilny_path,'rb').read()).hexdigest(),
        'donor_glb_sha256': hashlib.sha256(open(eichel_path,'rb').read()).hexdigest(),
        'mouth_landmark_status': 'provisional measured midline points; exposed as editable scene markers, not an automatic anatomical solution',
        'donor_mouth_native': list(native(donor_mouth)),
        'source_mouth_native': list(native(source_mouth)),
        'donor_eye_mid_native': list(native(de)),
        'source_eye_mid_native': list(native(se)),
        'donor_mouth_to_eye_height_mm': (de.z-donor_mouth.z)*1000,
        'source_mouth_to_eye_height_mm': (se.z-source_mouth.z)*1000,
        'mouth_sampling_translation_native': list(native(mouth_shift)),
        'eye_sampling_translation_native': list(native(eye_shift)),
        'common_output_translation_native': list(native(common_shift)),
        'preservation': 'native donor topology/UVs and vertex count; no rig/weights inferred; raw geometry-transfer study, not game-ready',
    }
    print('DT_LANDMARKS '+json.dumps(report), flush=True)
    # Verify the coordinate-channel process against an identity transfer first.
    identity = transfer_positions(head, head, Vector(), Vector())
    identity_error = max((a-b).length for a,b in zip(original,identity))
    report['identity_transfer_max_error_mm'] = identity_error*1000
    if identity_error > 1e-5:
        raise RuntimeError(f'Coordinate Data Transfer identity failed: {identity_error} metres')
    lower = transfer_positions(reference, head, mouth_shift, common_shift)
    upper = transfer_positions(reference, head, eye_shift, common_shift)
    # Blend over the middle 35%-75% of the donor mouth-to-eye span.
    blend_lo = donor_mouth.z + (de.z-donor_mouth.z)*.35
    blend_hi = donor_mouth.z + (de.z-donor_mouth.z)*.75
    region = [smoothstep(blend_lo,blend_hi,p.z) for p in original]
    # Retain the lower body shell that has no matching source surface. This raw study
    # deliberately does not freeze the old Eichel facial masks, which would conceal
    # the strengths and defects of the requested direct-transfer method.
    coverage = [smoothstep(donor_mouth.z-.13, donor_mouth.z-.075,p.z) for p in original]
    single = [p.lerp(a,w) for p,a,w in zip(original,lower,coverage)]
    blended = [p.lerp(a.lerp(b,t),w) for p,a,b,t,w in zip(original,lower,upper,region,coverage)]
    report['blend_band_native_y'] = [blend_lo,blend_hi]
    report['one_pass'] = normals_and_quality(original,single,head.data.polygons)
    report['two_pass'] = normals_and_quality(original,blended,head.data.polygons)
    report['one_vs_two_max_mm'] = max((a-b).length for a,b in zip(single,blended))*1000
    report['two_pass_moved_vertices'] = sum((a-b).length>1e-7 for a,b in zip(original,blended))
    # Explicitly measure lip-seam merging: source has closed/welded regions; retaining
    # donor IDs is necessary but does not prove successful future lip separation.
    groups = {}
    for i,p in enumerate(original):
        groups.setdefault(tuple(p),[]).append(i)
    coincident = [ids for ids in groups.values() if len(ids)>1]
    report['coincident_donor_groups'] = len(coincident)
    report['two_pass_max_coincident_group_spread_mm'] = max(
        ((blended[i]-blended[ids[0]]).length*1000 for ids in coincident for i in ids),default=0.)
    print('DT_RESULT '+json.dumps(report),flush=True)

    variants = []
    for label,positions in [('03_ONE_PASS_MOUTH',single),('04_TWO_PASS_BLEND',blended)]:
        obs,coll = duplicate(donor,label)
        h = find(obs,'geo_headShape')
        h.shape_key_add(name='Basis')
        # Keep pass results reversible and inspectable as shape keys.
        for name,coords in [('MouthAlignedTransfer',lower),('EyeAlignedTransfer',upper),('SelectedResult',positions)]:
            key = h.shape_key_add(name=name)
            for v,p in zip(key.data,coords): v.co = p
            key.value = 1. if name=='SelectedResult' else 0.
        g = h.vertex_groups.new(name='UpperPassBlend')
        for i,w in enumerate(region): g.add([i],w,'REPLACE')
        # Native companions follow source eye centres in the common frame. The head
        # transfer is raw, so aperture fit still requires visual assessment.
        for eye_name,from_c,to_c in [('geo_reyeShape',dr,sr+common_shift),('geo_leyeShape',dl,sl+common_shift)]:
            ob=find(obs,eye_name)
            delta=to_c-from_c
            for v in ob.data.vertices: v.co += delta
        lashes=find(obs,'geo_eyelashesShape')
        for v in lashes.data.vertices:
            v.co += (sr+common_shift-dr) if (v.co-dr).length<(v.co-dl).length else (sl+common_shift-dl)
        variants.append((label,obs,coll))
    # Move the reference into the common output frame, not the sampling alignments.
    for ob in source:
        for v in ob.data.vertices: v.co += common_shift
    marker_coll=bpy.data.collections.new('05_LANDMARKS_AND_NOTES')
    bpy.context.scene.collection.children.link(marker_coll)
    for label,p in [('Donor_Mouth',donor_mouth),('Mogilny_Mouth',source_mouth+common_shift),
                    ('Donor_EyeMid',de),('Mogilny_EyeMid',se+common_shift)]:
        ob=bpy.data.objects.new(label,None);marker_coll.objects.link(ob)
        ob.empty_display_type='SPHERE';ob.empty_display_size=.002;ob.location=p
    text=bpy.data.texts.new('READ_ME_Data_Transfer_Test')
    text.write(json.dumps(report,indent=2)+'\n\nRaw diagnostic. Shape keys are reversible. Both sampling passes transfer coordinates in the same mouth-anchored frame. No native asset deployment.\n')
    # Export evaluated selected results, not additive Basis + three shape keys.
    for label,obs,coll in variants:
        # Explicitly bake evaluated shape keys: export_morph=False alone can export Basis.
        bpy.context.view_layer.update()
        depsgraph = bpy.context.evaluated_depsgraph_get()
        baked = []
        for ob in obs:
            mesh = bpy.data.meshes.new_from_object(ob.evaluated_get(depsgraph))
            copy = bpy.data.objects.new(ob.name + '_BAKED', mesh)
            scene_collection = bpy.context.scene.collection
            scene_collection.objects.link(copy)
            copy.matrix_world = ob.matrix_world.copy()
            baked.append(copy)
        select_only(baked,find(baked,'geo_headShape'))
        bpy.ops.export_scene.gltf(filepath=os.path.join(out,label+'.glb'),export_format='GLB',
            use_selection=True,export_apply=True,export_morph=False,export_animations=False)
        for ob in baked:
            mesh = ob.data
            bpy.data.objects.remove(ob, do_unlink=True)
            bpy.data.meshes.remove(mesh)

    scene=bpy.context.scene
    scene.render.engine='BLENDER_WORKBENCH'
    scene.display.shading.light='STUDIO'
    scene.display.shading.studio_light='paint.sl'
    scene.display.shading.color_type='SINGLE'
    scene.display.shading.single_color=(.65,.65,.62)
    scene.display.shading.show_shadows=True
    scene.display.shading.show_cavity=True
    scene.display.shading.cavity_type='BOTH'
    scene.display.shading.background_type='WORLD'
    scene.world=bpy.data.worlds.new('World')
    scene.world.color=(.035,.035,.035)
    scene.render.resolution_x=700;scene.render.resolution_y=900;scene.render.resolution_percentage=100
    scene.render.image_settings.file_format='PNG'
    camera_data=bpy.data.cameras.new('ComparisonCamera');camera=bpy.data.objects.new('ComparisonCamera',camera_data)
    scene.collection.objects.link(camera);scene.camera=camera;camera_data.type='ORTHO';camera_data.ortho_scale=.37
    centre=Vector((0.,-.065,de.z-.025))
    groups_to_render=[('eichel',donor),('one_pass',variants[0][1]),('two_pass',variants[1][1]),('mogilny',source)]
    # Smooth reference-only normals for fair untextured comparison; donor indices stay
    # intact. Imported source normals are otherwise per-corner from the supplied GLB.
    for ob in donor+source+[o for _,obs,_ in variants for o in obs]:
        for p in ob.data.polygons:p.use_smooth=True
    for view,direction in [('front',Vector((0,-1,0))),('threequarter',Vector((.65,-1,0))),('profile',Vector((1,0,0)))]:
        camera.location=centre+direction
        camera.rotation_euler=(centre-camera.location).to_track_quat('-Z','Y').to_euler()
        for tag,visible in groups_to_render:
            for _,obs in groups_to_render:
                for ob in obs:ob.hide_render=ob not in visible
            # Hair cards/cap are retained in GLBs, hidden for the face comparison.
            for ob in visible:
                if any(x in ob.name for x in ('cardsA','cardsB','haircap')):ob.hide_render=True
            scene.render.filepath=os.path.join(out,view+'_'+tag+'.png')
            bpy.ops.render.render(write_still=True)
            print('RENDERED '+scene.render.filepath,flush=True)
    for tag,obs in groups_to_render:
        for ob in obs:
            ob.hide_render=tag!='two_pass'
            ob.hide_set(tag!='two_pass')
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(out,'Eichel_Mogilny_DataTransfer_Test.blend'))
    with open(os.path.join(out,'report.json'),'w') as f:json.dump(report,f,indent=2)
    print('DATA_TRANSFER_TEST_COMPLETE '+out,flush=True)


if __name__=='__main__':
    main()
