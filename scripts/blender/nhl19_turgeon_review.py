"""Attach the 2K atlas to the repaired review scene; preserve native mesh data."""
from pathlib import Path
import bpy
from mathutils import Vector
root=Path(r'H:\TEMP\nhl19-turgeon');out=root/'revision2'
bpy.ops.wm.open_mainfile(filepath=str(out/'Turgeon_NHL19_Revision2.blend'))
head=bpy.data.objects['Turgeon_NHL19']
image=bpy.data.images.load(str(root/'diffuse2k/Turgeon_Diffuse_2K.png'),check_existing=False)
image.colorspace_settings.name='sRGB';image.pack()
for node in head.data.materials[0].node_tree.nodes:
    if node.type=='TEX_IMAGE':node.image=image
scene=bpy.context.scene;scene.display.shading.color_type='TEXTURE'
focus=Vector((0,-.02,1.78))
for name,direction in [('front',(0,-1,0)),('threequarter',(.65,-1,0)),('profile',(1,0,0)),('opposite_profile',(-1,0,0))]:
    scene.camera.location=focus+Vector(direction)
    scene.camera.rotation_euler=(focus-scene.camera.location).to_track_quat('-Z','Y').to_euler()
    scene.render.filepath=str(out/(name+'_2k.png'));bpy.ops.render.render(write_still=True)
bpy.ops.wm.save_as_mainfile(filepath=str(out/'Turgeon_NHL19_Revision2.blend'))
bpy.ops.object.select_all(action='DESELECT')
for ob in bpy.data.objects:
    if ob.type=='MESH' and not ob.hide_render:ob.select_set(True)
bpy.context.view_layer.objects.active=head
bpy.ops.export_scene.gltf(filepath=str(out/'Turgeon_NHL19_Revision2.glb'),use_selection=True,export_morph=False,export_animations=False)
