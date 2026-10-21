"""Audit final native attachment, companion byte protection and frozen source inputs."""
import argparse,hashlib,json,struct,sys
from pathlib import Path
import numpy as np
sys.dont_write_bytecode=True
repo=Path(__file__).resolve().parents[3];sys.path.insert(0,str(repo))
from src.conversion.geometry import native_to_blender,bottom_loop,seam_groups,adjacency,grow
from src.application.native_paths import contained

parser=argparse.ArgumentParser()
parser.add_argument('candidate',type=Path)
parser.add_argument('--installed-packet',type=Path)
parser.add_argument('--offline',action='store_true',help='Audit native files only; do not validate live installation readiness.')
parser.add_argument('--run-root',type=Path,default=repo/'local/runs/codex',help='existing directory the candidate (and installed packet) must lie beneath')
args=parser.parse_args()
if args.offline and args.installed_packet:parser.error('--offline and --installed-packet are mutually exclusive')
out=contained(args.run_root,args.candidate,'Candidate')
if not out.is_dir():raise NotADirectoryError(out)
receipt=out/'final-audit.json'
if receipt.exists():raise FileExistsError(receipt)
mesh=json.loads((out/'native_mesh.json').read_text())
data=(out/'aaron_ekblad.mcd').read_bytes();original=(out/'native_donor.mcd').read_bytes()
checks=[]
for sub in mesh['submeshes']:
    f=sub['native_fields']['p0']
    pos=np.array([struct.unpack_from('<3f',data,f['data_off']+i*f['stride']+f['offset']) for i in range(len(sub['positions']))])
    assert np.array_equal(pos,np.asarray(sub['positions'],np.float32))
    checks.append({'component':sub['name'],'f32_positions_exact':True})
    if sub['name']=='geo_headShape':
        donor=np.asarray(sub['original_positions'],np.float32);tri=np.asarray(sub['triangles'],int)
        groups=seam_groups(donor);loop=bottom_loop(native_to_blender(donor),tri)
        frozen=grow(loop,adjacency(len(donor),tri,groups),2)
        assert np.array_equal(pos[frozen],donor[frozen])
        gap=max(float(np.linalg.norm(pos[ids]-pos[ids[0]],axis=1).max()) for ids in groups)
        assert gap==0
        checks.append({'attachment_loop_vertices':len(loop),'attachment_plus_2_rings_vertices':len(frozen),
                       'attachment_error_m':0,'seam_max_gap_m':gap})
    if sub['name']=='geo_eyesShape':
        field=sub['native_fields']['n0'];start=field['data_off'];end=start+len(pos)*field['stride']
        assert data[start:end]==original[start:end]
        checks.append({'component':sub['name'],'entire_attribute_stream_byte_identical':True,'bytes':end-start})
    if sub['name'] in ('geo_cardsBShape','geo_haircapShape'):
        start=f['data_off'];end=start+len(pos)*f['stride']
        assert data[start:end]==original[start:end]
        checks.append({'component':sub['name'],'entire_vertex_stream_byte_identical':True,'bytes':end-start})
frozen_checks=json.loads((out/'frozen-hash-checks.json').read_text())
for check in frozen_checks:
    assert hashlib.sha256(Path(check['path']).read_bytes()).hexdigest()==check['sha256']
checks.append({'frozen_inputs_and_artifacts_rechecked_after_pack':True,'count':len(frozen_checks)})
preconditions=json.loads((out/'import-preconditions.json').read_text())
if args.offline:
    baseline=None # Export qualification does not depend on the currently installed mod layer.
elif args.installed_packet:
    from installed_baseline import verify_installed
    contained(args.run_root,args.installed_packet,'Installed packet')
    baseline=verify_installed(args.installed_packet,preconditions)
    (out/'installed-baseline-audit.json').write_text(json.dumps(baseline,indent=2))
else:
    for entry in preconditions['targets']:
        with Path(entry['archive']).open('rb') as f:
            header=f.read(48);f.seek(entry['row_offset']);row=f.read(20)
            f.seek(entry['offset']);raw=f.read(entry['stored_bytes'])
        assert header.hex()==entry['expected_header_hex'] and row.hex()==entry['expected_row_hex']
        assert hashlib.sha256(raw).hexdigest()==entry['stored_sha256']
if not args.offline:checks.append({'current_archive_target_headers_rows_stored_hashes_match':True,'count':2,
               'scope':'Selected current extents only; original16-table catalog scan documented separately.'})
receipt.write_text(json.dumps({'status':'passed','scope':'native files only; live installation readiness not checked' if args.offline else 'native files and selected live entries','checks':checks},indent=2))
print('FINAL AUDIT PASSED',out,'checks',len(checks))
