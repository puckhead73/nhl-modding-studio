"""Read the supplied CC5 FBX and record evaluated geometry/material/UV evidence."""
import bpy,json,sys,hashlib,argparse
from pathlib import Path
from mathutils import Vector
parser=argparse.ArgumentParser()
parser.add_argument('output')
parser.add_argument('--source',default=r'S:\Downloads\Mogilny Test\Demitra\Demitra.Fbx')
args=parser.parse_args(sys.argv[sys.argv.index('--')+1:])
root=Path(args.output);root.mkdir(parents=True,exist_ok=True)
source_file=Path(args.source);src=source_file.parent
bpy.ops.wm.read_factory_settings(use_empty=True)
# The supplied FBX's skin hierarchy trips Blender's armature_setup[None].
# Strip only Skin/Cluster objects and their connections in memory for static rest
# geometry import. Preserve meshes, node transforms, materials and blend shapes.
from io_scene_fbx import parse_fbx
original_parse=parse_fbx.parse
removed=[]
def static_parse(*args,**kwargs):
    elem,version=original_parse(*args,**kwargs)
    objects=next(e for e in elem.elems if e.id==b'Objects')
    ids={e.props[0] for e in objects.elems if e.id==b'Deformer' and e.props[2] in (b'Skin',b'Cluster')}
    removed.extend(sorted(ids))
    objects.elems[:]=[e for e in objects.elems if e.props[0] not in ids]
    connections=next(e for e in elem.elems if e.id==b'Connections')
    connections.elems[:]=[e for e in connections.elems if not any(p in ids for p in e.props[1:3])]
    return elem,version
parse_fbx.parse=static_parse
bpy.ops.import_scene.fbx(filepath=str(source_file))
report={'source_path':str(source_file),'source_sha256':hashlib.sha256(source_file.read_bytes()).hexdigest(),'removed_skin_cluster_ids':removed,'objects':[],'images':[]}
for ob in bpy.data.objects:
    item={'name':ob.name,'type':ob.type,'matrix_world':[list(r) for r in ob.matrix_world]}
    if ob.type=='MESH':
        ps=[ob.matrix_world@v.co for v in ob.data.vertices]
        item.update(vertices=len(ps),polygons=len(ob.data.polygons),bounds=[[min(p[k] for p in ps) for k in range(3)],[max(p[k] for p in ps) for k in range(3)]],uv_layers=[u.name for u in ob.data.uv_layers],shape_keys=[k.name for k in ob.data.shape_keys.key_blocks] if ob.data.shape_keys else [])
        item['materials']=[]
        for i,mat in enumerate(ob.data.materials):
            polys=[p for p in ob.data.polygons if p.material_index==i]
            ids={v for p in polys for v in p.vertices}
            uv=[ob.data.uv_layers.active.data[l].uv for p in polys for l in p.loop_indices]
            item['materials'].append({'slot':i,'name':mat.name,'vertices':len(ids),'polygons':len(polys),'uv_bounds':[[min(v[k] for v in uv) for k in range(2)],[max(v[k] for v in uv) for k in range(2)]] if uv else None})
    elif ob.type=='ARMATURE':item['bones']=[b.name for b in ob.data.bones]
    report['objects'].append(item)
for name in ['Std_Skin_Head_Diffuse.png','Std_Skin_Head_Normal.jpg']:
    im=bpy.data.images.load(str(src/(source_file.stem+'.fbm')/name),check_existing=True)
    report['images'].append({'name':name,'size':list(im.size)})
    preview=im.copy();preview.scale(1024,1024);preview.filepath_raw=str(root/(name+'.preview.png'));preview.file_format='PNG';preview.save()
(root/'inspection.json').write_text(json.dumps(report,indent=2))
bpy.ops.wm.save_as_mainfile(filepath=str(root/(source_file.stem+'_Source.blend')))
print('INSPECTED',json.dumps([{k:v for k,v in item.items() if k in ('name','type','vertices','bounds','materials')} for item in report['objects']]))
