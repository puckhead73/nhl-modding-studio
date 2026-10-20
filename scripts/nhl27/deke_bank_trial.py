"""Build private, fixed-size motion-only ANT trial banks from verified conversions.

Preserves all object identities, graph references and relocations. The explicit
--port-metadata mode also transfers source timing, playback flags and trim fields.
Rejects insufficient capacity, shared buffers, changed metadata and conflicting writes.
These decoded resources are NOT install packages and contain no NHL27 gameplay logic.
Usage: python scripts/nhl27/deke_bank_trial.py COMPATIBILITY_ROOT
"""
import hashlib
import json
import struct
import sys
from pathlib import Path
from animation_dct import envelope, decode
from native_ant import Bank


def run(root,bank_name='state',port_metadata=False):
    suffix='' if bank_name=='state' else bank_name+'-'
    output_suffix=suffix+('metadata-' if port_metadata else '')
    dest=root/'deke-trial';audit=json.loads((dest/(suffix+'native-comparison.json')).read_text())
    reordered=json.loads((dest/(suffix+'reorder-report.json')).read_text());reports=[]
    for build in audit['banks']:
        name=build['build'];original=(root/'ant'/name/(bank_name+'.bin')).read_bytes();bank=Bank(original)
        if hashlib.sha256(original).hexdigest()!=build['sha256']:raise ValueError('Native audit hash changed')
        candidates={x['source_sha1']:x for b in reordered if b['build']==name for x in b['clips']}
        output=bytearray(original);writes=[];rows=[]
        def exclusive(start,size,owner):
            # No other serialized pointer may point into a buffer we replace.
            if any(start<=d<start+size and s!=owner for s,d in bank.relocations):
                raise ValueError('Replacement range has another relocation reference')
            if any(start<x[0]+bank.sizes[x[2]] and x[0]<start+size for x in bank.objects):
                raise ValueError('Replacement buffer overlaps a registered object')
        for clip in build['clips']:
            row=dict(name=clip['name'],source_sha1=clip['source_sha1'])
            try:
                c=candidates[clip['source_sha1']]
                if not c.get('fits_existing_buffers'):raise ValueError('Requires bank allocation growth')
                if not c['same_dof_set_id']:raise ValueError('Different DOF set identity')
                source=Path(c['output']).read_bytes()
                if hashlib.sha256(source).hexdigest()!=c['sha256']:raise ValueError('Converted resource changed')
                obj,anim,old=bank.clip(int(clip['root_id'],16));p=anim['offset']
                f=next(x['fields'] for x in clip['source_fields']['instances'] if 'DctAnimationAsset' in x['type'])
                controller=next(x['fields'] for x in clip['source_fields']['instances'] if 'ClipControllerAsset' in x['type'])
                scalar_names=['TimeScale','0x314b9906','Distance','TickOffset','DeltaTrajectory',
                    'DeltaTrajectoryRotationMask','OnlyActivateModifiedDofs','IsLooping','PlaybackMode','DofMode']
                expected=struct.pack('<4f6B',*(controller[k] for k in scalar_names))
                metadata_edits=[]
                if port_metadata:
                    metadata_edits=[(obj['offset']+48,expected),
                        (p+24,struct.pack('<ffH',f['TrimOffset'],f['TrimmedDuration'],f['EndFrame'])),
                        (p+56,struct.pack('<I',f['BaseFlags'])),
                        (p+100,struct.pack('<?',f['Cycle']))]
                if not port_metadata and bank.blob[obj['offset']+48:obj['offset']+70]!=expected:
                    raise ValueError('Controller timing/flags differ; requires explicit port')
                if not port_metadata and bank.blob[p+24:p+34]!=struct.pack('<ffH',f['TrimOffset'],f['TrimmedDuration'],f['EndFrame']):
                    raise ValueError('Animation trim/end frame differs')
                if bank.blob[p+96:p+100]!=struct.pack('<HH',f['NumVec3'],f['NumFloat']):
                    raise ValueError('Native channel counts/cycle differ')
                if not port_metadata and bank.blob[p+100:p+101]!=struct.pack('<?',f['Cycle']):
                    raise ValueError('Native cycle differs')
                if not port_metadata and struct.unpack_from('<I',bank.blob,p+56)[0]!=f['BaseFlags']:
                    raise ValueError('Native animation base flags differ')
                if sum(d==p for _,d in bank.relocations)!=1:
                    raise ValueError('Animation object shared by multiple controllers')
                env,body=envelope(source);oldenv,_=envelope(old)
                body_at=bank.pointer(p+80);exclusive(body_at,oldenv['body_size'],p+80)
                edits=metadata_edits+[(body_at,body),(p+88,struct.pack('<I',len(body))),
                       (p+72,struct.pack('<I',len(env['key_times'])))]
                if env['key_times']:
                    keys_at=bank.pointer(p+64);exclusive(keys_at,len(oldenv['key_times'])*2,p+64)
                    edits.append((keys_at,struct.pack('<'+'H'*len(env['key_times']),*env['key_times'])))
                planned=[(bank.sections['blob'][0]+at,data) for at,data in edits]
                for at,data in planned:
                    if any(at<a+len(d) and a<at+len(data) for a,d in writes):
                        raise ValueError('Overlapping replacement writes')
                for at,data in planned:output[at:at+len(data)]=data
                writes.extend(planned);row.update(status='included_motion_and_metadata' if port_metadata else 'included_motion_only',metadata_ported=port_metadata,root_id=clip['root_id'],
                    converted_sha256=c['sha256'],writes=[dict(offset=a,size=len(d)) for a,d in planned])
            except (ValueError,KeyError,StopIteration) as exc:row.update(status='excluded',reason=str(exc))
            rows.append(row)
        # Reopen the complete output and compare every included clip to its converted resource.
        verified=Bank(bytes(output))
        for row in rows:
            if not row['status'].startswith('included_'):continue
            actual=verified.clip(int(row['root_id'],16))[2]
            expected=Path(candidates[row['source_sha1']]['output']).read_bytes()
            if actual!=expected:raise ValueError('Native bank reconstruction differs from converted resource')
            decode(actual)
        for at,data in writes:
            if bytes(output[at:at+len(data)])!=data:raise ValueError('Declared write failed verification')
        # Verify that undoing exactly the declared writes recovers the original bank.
        undo=bytearray(output)
        for at,data in writes:undo[at:at+len(data)]=original[at:at+len(data)]
        if bytes(undo)!=original:raise ValueError('Undeclared output mutation')
        path=dest/(name+'-'+output_suffix+'motion-only-trial.bank');path.write_bytes(output)
        report=dict(build=name,status='private_decoded_bank_not_installable',output=str(path),
            source_sha256=hashlib.sha256(original).hexdigest(),output_sha256=hashlib.sha256(output).hexdigest(),
            unchanged_size=len(output)==len(original),included=sum(x['status'].startswith('included_') for x in rows),
            clips=rows,limitations=['No CAS/manifest install packaging','No new-move graphs or gameplay logic',
                'No gameplay validation or NHL27 behavior-parity claim'])
        reports.append(report)
    (dest/(output_suffix+'bank-trial-report.json')).write_text(json.dumps(reports,indent=2))
    print(json.dumps([dict(build=x['build'],included=x['included'],sha256=x['output_sha256']) for x in reports],indent=2))


if __name__=='__main__':run(Path(sys.argv[1]),sys.argv[2] if len(sys.argv)>2 else 'state','--port-metadata' in sys.argv[2:])
