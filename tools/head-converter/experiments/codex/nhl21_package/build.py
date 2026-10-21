"""Pinned NHL21 native shading/texture writer; experimental assets only, never installs."""
import argparse,bpy,hashlib,json,struct,subprocess,sys
from pathlib import Path
import numpy as np
sys.path.insert(0,str(Path(__file__).parent))
from math_texture import normals_tangents,texture_package
from shading_proof import positive_uniform_scale_translation
from layout_fields import resolve_fields

repo=Path(__file__).resolve().parents[3];sys.path.insert(0,str(repo))
from src.application.native_paths import fresh_output_beneath,existing_file,parse_preconditions,preconditions_target
sha=lambda b:hashlib.sha256(b).hexdigest()
parser=argparse.ArgumentParser()
parser.add_argument('--exchange',type=Path,required=True)
parser.add_argument('--input',type=Path)
parser.add_argument('--atlas',type=Path,required=True)
parser.add_argument('--out',type=Path,required=True)
parser.add_argument('--diagnostic',action='store_true')
parser.add_argument('--candidate-handoff',type=Path)
# Explicit route inputs (defaults = the demonstrated Ekblad route). No silent fallback: a given path must exist.
parser.add_argument('--run-root',type=Path,default=repo/'local/runs/codex',help='existing directory --out must lie beneath')
parser.add_argument('--packer-exe',type=Path,default=repo/'local/runs/codex/nhl21-candidate-build/debug/nhl21-candidate-pack.exe',help='native packer executable (pack/verify)')
parser.add_argument('--preconditions',type=Path,default=repo/'research/codex/nhl21-phase3-import-preconditions.json',help='import-preconditions JSON for the qualified archive targets')
args=parser.parse_args(sys.argv[sys.argv.index('--')+1:])
candidate=None;frozen_checks=[];candidate_role=None;captured={}   # candidate artifact bytes, hash-verified once and consumed as captured
if args.candidate_handoff:
    candidate=json.loads(args.candidate_handoff.read_text())
    assert candidate['status']=='ready_for_experimental_packing' and args.input and not args.diagnostic
    for item in candidate['inputs'].values():
        digest=sha(Path(item['path']).read_bytes());assert digest==item['sha256'],item['path']
        frozen_checks.append({'path':item['path'],'sha256':digest,'matches':True})
    for pathkey,hashkey in [('native_mesh_json','native_mesh_sha256'),('atlas_png','atlas_sha256'),('eyes_placed_variant_json','eyes_placed_variant_sha256')]:
        # A reviewed handoff may promote placed eyes to its primary mesh.
        if pathkey == 'eyes_placed_variant_json' and pathkey not in candidate['artifacts']:
            assert hashkey not in candidate['artifacts']
            continue
        path=Path(candidate['artifacts'][pathkey]);raw=path.read_bytes();digest=sha(raw)
        assert digest==candidate['artifacts'][hashkey]
        captured[path.resolve()]=raw
        frozen_checks.append({'path':str(path),'sha256':digest,'matches':True})
        if args.input.resolve()==path.resolve():candidate_role=pathkey
    assert candidate_role in ('native_mesh_json','eyes_placed_variant_json')
    assert args.atlas.resolve()==Path(candidate['artifacts']['atlas_png']).resolve()
out=fresh_output_beneath(args.run_root,args.out)
exe=existing_file(args.packer_exe,'Native packer executable')
preconditions_path=existing_file(args.preconditions,'Import preconditions','.json');preconditions_raw=preconditions_path.read_bytes()
preconditions=parse_preconditions(preconditions_raw,preconditions_path)   # validated from the bytes that are copied and hashed below
# Exchange pins and the preconditions' donor/texture target agreement are verified BEFORE any output exists or the native
# packer runs; the bytes verified here (`pinned`) are the ones copied, parsed, packed and verified below - the original source paths
# are recorded in `sources` for provenance only and are never read again.
handoff=json.loads(args.exchange.read_text())
paths={key:Path(handoff[key]['path']) for key in ('exchange','native_donor','profile','texture')}
pinned={}
for key,path in paths.items():
    data=path.read_bytes();assert sha(data)==handoff[key]['sha256'],key;pinned[key]=data
original=json.loads(pinned['exchange']);profile=json.loads(pinned['profile'])
required_targets=[(profile['archive_provenance']['asset_path'],profile['donor']['sha256']),
 (profile['diffuse']['provenance']['asset_path'],profile['diffuse']['provenance']['decoded_sha256'])]
for entry,expected in required_targets:preconditions_target(preconditions,entry,expected)
out.mkdir(exist_ok=False)
if candidate:
    (out/'claude-handoff.json').write_bytes(args.candidate_handoff.read_bytes())
    (out/'frozen-hash-checks.json').write_text(json.dumps(frozen_checks,indent=2))
sources={}
for key,path in paths.items():
    data=pinned[key];dest=out/(key+path.suffix);dest.write_bytes(data);sources[key]={'source':str(path),'sha256':sha(data),'copy':str(dest)}
input_raw=captured.get(args.input.resolve()) if args.input else None
if args.input and input_raw is None:input_raw=args.input.read_bytes()   # diagnostic/no-handoff runs have no hash-verified capture
mesh=json.loads(input_raw) if args.input else json.loads(pinned['exchange'])
assert mesh['donor_sha256']==profile['donor']['sha256']
if args.diagnostic:
    assert not args.input
    head=next(m for m in mesh['submeshes'] if m['name']==profile['head_submesh'])
    head['positions'][profile['diagnostic']['vertex']][2]+=.001
donor=pinned['native_donor']
donor_copy=Path(sources['native_donor']['copy']);profile_copy=Path(sources['profile']['copy'])
assert len(mesh['submeshes'])==len(original['submeshes'])
byname={m['name']:m for m in original['submeshes']}
assert len({m['name'] for m in mesh['submeshes']})==len(byname)
shading={};reports={}
for m in mesh['submeshes']:
    reference=byname[m['name']]
    pinned_submesh=next(s for s in profile['layout']['submeshes'] if s['name']==m['name'])
    fields=resolve_fields(profile,pinned_submesh)
    assert reference['native_fields']==fields
    for key in ('original_positions','triangles','uv'):
        assert np.array_equal(m[key],reference[key]),(m['name'],key)
    # Byte addresses come only from our pinned exchange, never candidate metadata.
    m['native_fields']=fields
    m['positions']=np.asarray(m['positions'],np.float32).tolist()
    assert np.isfinite(m['positions']).all()
    if np.array_equal(m['positions'],reference['positions']):
        n=np.asarray(reference['normals']);t=np.asarray(reference['tangents']);active=np.zeros(len(n),bool)
        report={'entire_component_unchanged':True,'native_shading_preserved_vertices':len(n),'shading_regenerated_vertices':0}
    else:
        n,t,report,active=normals_tangents(m,reference)
    if m['name']=='geo_eyesShape':
        proofs=[]
        for component in reference['connected_components']:
            ids=component['vertex_ids']
            proof=positive_uniform_scale_translation(np.asarray(reference['positions'])[ids],np.asarray(m['positions'])[ids])
            proofs.append({'vertices':len(ids),'proof':proof})
            if proof['accepted']:
                active[ids]=False;n[ids]=np.asarray(reference['normals'])[ids]
        report['component_uniform_transform_proofs']=proofs
        report['native_shading_preserved_vertices']=int((~active).sum())
        report['shading_regenerated_vertices']=int(active.sum())
    m['normals']=n.tolist();shading[m['name']]=(n,t,active);reports[m['name']]=report
(out/'native_mesh.json').write_text(json.dumps(mesh,indent=2))
subprocess.run([str(exe),'pack',str(donor_copy),str(profile_copy),str(out/'native_mesh.json'),str(out/'positions.mcd')],check=True)
positioned=(out/'positions.mcd').read_bytes();data=bytearray(positioned)
allowed=np.zeros(len(data),bool)
for m in mesh['submeshes']:
    n,t,active=shading[m['name']]
    for usage,values in [('n0',n),('g0',t)]:
        field=m['native_fields'][usage]
        assert field['encoding']=='4f16'
        for i,value in enumerate(values):
            off=field['data_off']+i*field['stride']+field['offset']
            if active[i]:
                data[off:off+6]=struct.pack('<3e',*value);allowed[off:off+6]=True
            else:assert data[off:off+8]==donor[off:off+8]
            assert data[off+6:off+8]==donor[off+6:off+8]
    # Native UV and skin/opaque record fields are not rewritten.
    for usage in ('t0','t1','t7','t8'):
        if usage not in m['native_fields']:continue
        f=m['native_fields'][usage]
        for i in range(len(m['positions'])):
            off=f['data_off']+i*f['stride']+f['offset']
            assert data[off:off+4]==donor[off:off+4]
before=np.frombuffer(positioned,np.uint8);after=np.frombuffer(data,np.uint8)
assert np.array_equal(before[~allowed],after[~allowed])
(out/'aaron_ekblad.mcd').write_bytes(data)
atlas=captured.get(args.atlas.resolve()) or args.atlas.read_bytes();(out/'atlas.png').write_bytes(atlas)
texture,tex_report=texture_package(pinned['texture'],out/'atlas.png',out)
verification=subprocess.run([str(exe),'verify',str(donor_copy),str(profile_copy),str(out/'native_mesh.json'),
 str(out/'aaron_ekblad.mcd'),str(texture),str(out/'diffuse_mip_0.bc1')],capture_output=True,text=True,check=True)
receipt={'status':'diagnostic qualification only' if args.diagnostic else 'experimental native candidate; not installed',
 'candidate_handoff':{'source':str(args.candidate_handoff.resolve()),'sha256':sha(args.candidate_handoff.read_bytes()),
                      'artifact_role':candidate_role,'geometry_classification':candidate['geometry_classification'],
                      'gates':candidate['gates'],'limitations':candidate['limitations']} if candidate else None,
 'sources':sources,'input':{'path':str(args.input.resolve()) if args.input else None,'sha256':sha(input_raw) if args.input else None},
 'atlas':{'path':str(args.atlas.resolve()),'sha256':sha(atlas)},
 'native':{'sha256':sha(data),'bytes':len(data),'changed_position_stage_bytes':sum(a!=b for a,b in zip(donor,positioned)),
 'changed_shading_stage_bytes':int((before!=after).sum()),'shading_mask_only':True,'UV_skin_opaque_fields_preserved':True,
 'verification':verification.stdout.strip(),'native_pack_verify_inputs':{'donor':str(donor_copy),'profile':str(profile_copy)}},'shading':reports,'texture':tex_report,
 'toolchain':{'blender':bpy.app.version_string,'python':sys.version,'numpy':np.__version__},
 'route_inputs':{'run_root':str(args.run_root.resolve()),'packer_exe':{'path':str(exe),'sha256':sha(exe.read_bytes())},'preconditions':{'path':str(preconditions_path),'sha256':sha(preconditions_raw)}},
 'implementation_hashes':{str(p):sha(p.read_bytes()) for p in [Path(__file__),Path(__file__).parent/'math_texture.py',Path(__file__).parent/'src/main.rs',Path(__file__).parent/'Cargo.lock',exe]},
 'limitations':['Exact Ekblad donor only; no other NHL21 donor qualified.','No topology/UV/rig/material binding changes.',
  'Original native normal/PBR textures retained; tangent-W shader meaning unqualified.',
  'Anatomy masks, source likeness, facial motion, eye pivots, hair/helmet and game testing require separate evidence.']}
(out/'validation.json').write_text(json.dumps(receipt,indent=2))
plan={'status':'prepared native files only; not installed','title':'NHL21 PS4 CUSA18182',
 'replacements':[{'archive':profile['archive_provenance']['archive'],'entry':profile['archive_provenance']['asset_path'],
 'file':'aaron_ekblad.mcd','expected_decoded_sha256':profile['donor']['sha256']},
 {'archive':profile['diffuse']['provenance']['archive'],'entry':profile['diffuse']['provenance']['asset_path'],
 'file':texture.name,'expected_decoded_sha256':profile['diffuse']['provenance']['decoded_sha256']}],
 'requirements':['Re-extract current entries and match expected donor hashes before any separately authorized import.',
 'Back up complete archives, preserve existing modifications, test static views, jaw/lip/blink/gaze and helmet states.']}
assert [(r['entry'],r['expected_decoded_sha256']) for r in plan['replacements']]==required_targets  # same targets verified before packing
(out/'import-preconditions.json').write_bytes(preconditions_raw)
plan['current_archive_preconditions']={'file':'import-preconditions.json','source':str(preconditions_path),'sha256':sha(preconditions_raw)}
(out/'installation-plan.json').write_text(json.dumps(plan,indent=2))
print('NHL21_NATIVE_COMPLETE',out)
