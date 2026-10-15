"""Run matched built-in and MeshDataTransfer 2.0.9 shape-transfer experiments.

blender --background --python-exit-code 1 --python this.py -- donor.glb source.glb out
The add-on must already be installed. No source assets or add-on code are copied.
"""
import os
import sys
import json
import bpy
import addon_utils

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nhl19_data_transfer_test as test


def addon_positions(source, destination, sampling_shift, common_shift):
    # main() resets factory settings, so enable only this add-on when needed.
    if not hasattr(bpy.types.Object, 'mesh_data_transfer_object'):
        addon_utils.enable('MeshDataTransfer', default_set=False)
    src = source.copy()
    src.data = source.data.copy()
    dst = destination.copy()
    dst.data = destination.data.copy()
    for ob in (src, dst):
        bpy.context.scene.collection.objects.link(ob)
    # Bake the sampling translation to eliminate object-space ambiguity.
    for vertex in src.data.vertices:
        vertex.co += sampling_shift
    test.select_only([dst], dst)
    settings = dst.mesh_data_transfer_object
    settings.mesh_source = src
    settings.search_method = 'CLOSEST'
    settings.mesh_object_space = 'WORLD'
    settings.attributes_to_transfer = 'SHAPE'
    settings.transfer_shape_as_key = False
    settings.transfer_modified_source = False
    settings.snap_to_closest_shape = False
    settings.transfer_edit_selection = False
    settings.vertex_group_filter = ''
    bpy.context.view_layer.update()
    result = bpy.ops.object.transfer_mesh_data()
    if result != {'FINISHED'}:
        raise RuntimeError(f'Add-on transfer failed: {result}')
    # Re-express each sampled point in the shared mouth-anchored output frame.
    positions = [v.co.copy() + common_shift - sampling_shift for v in dst.data.vertices]
    for ob in (src, dst):
        mesh = ob.data
        bpy.data.objects.remove(ob, do_unlink=True)
        bpy.data.meshes.remove(mesh)
    return positions


def read_results(path):
    bpy.ops.wm.open_mainfile(filepath=os.path.join(path, 'Eichel_Mogilny_DataTransfer_Test.blend'))
    result = {}
    for label in ('03_ONE_PASS_MOUTH', '04_TWO_PASS_BLEND'):
        head = test.find(list(bpy.data.collections[label].objects), 'geo_headShape')
        result[label] = [v.co.copy() for v in head.data.shape_keys.key_blocks['SelectedResult'].data]
        # Verify exported GLB geometry against the chosen shape key, independent of ordering.
        obs, coll = test.import_glb(os.path.join(path, label+'.glb'), 'VERIFY_'+label)
        imported = test.find(obs, 'geo_headShape')
        from mathutils.kdtree import KDTree
        def distance(a, b):
            tree = KDTree(len(b))
            for i, p in enumerate(b): tree.insert(p, i)
            tree.balance()
            return max(tree.find(p)[2] for p in a)
        actual = [v.co.copy() for v in imported.data.vertices]
        error = max(distance(actual, result[label]), distance(result[label], actual))
        if error > 1e-6:
            raise RuntimeError(f'GLB differs from selected result: {label} {error}')
        print('VERIFIED_GLB', path, label, 'max_error_mm', error*1000)
    return result


if __name__ == '__main__':
    donor, source, root = sys.argv[sys.argv.index('--')+1:]
    builtin = os.path.join(root, 'builtin')
    addon = os.path.join(root, 'addon')
    sys.argv = [__file__, '--', donor, source, builtin]
    test.main()
    test.transfer_positions = addon_positions
    test.TRANSFER_METHOD = 'MeshDataTransfer 2.0.9 object.transfer_mesh_data: CLOSEST, WORLD, SHAPE; snap off; same shared-frame regional blend as built-in'
    sys.argv = [__file__, '--', donor, source, addon]
    test.main()
    a, b = read_results(builtin), read_results(addon)
    report = {}
    for label in a:
        distances = [(x-y).length*1000 for x,y in zip(a[label], b[label])]
        report[label] = {'max_difference_mm': max(distances),
                         'rms_difference_mm': (sum(d*d for d in distances)/len(distances))**.5}
    for label, path in [('builtin', builtin), ('addon', addon)]:
        with open(os.path.join(path, 'report.json')) as f: report[label] = json.load(f)
    with open(os.path.join(root, 'comparison.json'), 'w') as f: json.dump(report, f, indent=2)
    print('COMPARISON_COMPLETE', json.dumps(report))
