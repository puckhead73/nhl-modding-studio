"""Build private 1.60 animation/metadata candidates with bounded buffer repacking."""
import hashlib
import json
import struct
import sys
from pathlib import Path
from native_ant import Bank
from animation_dct import envelope
from ant_buffer_pool import repack
from deke_native_audit import native_id
from vbr_stage import envelope as vbr_envelope

SCALARS=['TimeScale','0x314b9906','Distance','TickOffset','DeltaTrajectory',
         'DeltaTrajectoryRotationMask','OnlyActivateModifiedDofs','IsLooping','PlaybackMode','DofMode']
VBR_FLOATS=['QuatMin','TrajMin','Vec3Min','FloatMin','QuatMax','TrajMax','Vec3Max','FloatMax',
            'VectorOffsetScale','FloatOffsetScale','0x7960773d']
VBR_SHORTS=['QuaternionCount','Vector3Count','FloatCount','ConstQuaternionCount','ConstVector3Count',
            'ConstFloatCount','KeyTimeSize','NumKeys','ConstChanMapSize','ConstPaletteSize',
            'VectorOffsetSize','FloatOffsetSize','Flags']


def run(root,name):
    d=root/'deke-trial';pre='' if name=='state' else name+'-'
    bank=Bank((root/'ant/nhl24_160'/f'{name}.bin').read_bytes());replacements={};edits=[];checks=[]
    audit=next(x for x in json.loads((d/(pre+'native-comparison.json')).read_text())['banks'] if x['build']=='nhl24_160')
    converted={x['source_sha1']:x for b in json.loads((d/(pre+'reorder-report.json')).read_text()) if b['build']=='nhl24_160' for x in b['clips']}
    def metadata(controller,anim,fields,cf):
        if sum(dst==anim for _,dst in bank.relocations)!=1:raise ValueError('Shared animation metadata')
        edits.extend([(controller+48,struct.pack('<4f6B',*(cf[k] for k in SCALARS))),
            (anim+24,struct.pack('<ffH',fields['TrimOffset'],fields['TrimmedDuration'],fields['EndFrame'])),
            (anim+56,struct.pack('<I',fields['BaseFlags']))])
    for clip in audit['clips']:
        c=converted[clip['source_sha1']]
        if 'output' not in c:continue
        if not c['same_dof_set_id']:raise ValueError('Unqualified DOF set')
        data=Path(c['output']).read_bytes()
        if hashlib.sha256(data).hexdigest()!=c['sha256']:raise ValueError('Converted data changed')
        p=clip['animation']['offset'];f=next(x['fields'] for x in clip['source_fields']['instances'] if 'DctAnimationAsset' in x['type'])
        cf=next(x['fields'] for x in clip['source_fields']['instances'] if 'ClipControllerAsset' in x['type'])
        metadata(clip['controller']['offset'],p,f,cf)
        env,body=envelope(data);replacements[p+80]=body
        if env['key_times']:replacements[p+64]=struct.pack('<'+'H'*len(env['key_times']),*env['key_times'])
        edits.extend([(p+72,struct.pack('<I',len(env['key_times']))),(p+88,struct.pack('<I',len(body))),
                      (p+96,struct.pack('<HH?',f['NumVec3'],f['NumFloat'],f['Cycle']))])
        checks.append(dict(codec='DCT',id=int(clip['root_id'],16),name=clip['name'],expected=data,animation=p))
    if name=='launch':
        for row in json.loads((d/'vbr/stage.json').read_text()):
            prefix=Path(row['prefix']);f=json.loads(prefix.with_suffix('.remapped.fields.json').read_text())
            ident=native_id(row['fields']['partition']);obj=bank.by_id[ident];p=bank.pointer(obj['offset']+32)
            if not any(o['offset']==p and o['size']==184 and o['type_hash']==1531114270 for o in bank.by_id.values()):
                raise ValueError('Unqualified VBR object')
            dsl=bank.pointer(p+48);source_dsl=native_id(f['DofSetList']['external']['instance'])
            if bank.by_id[source_dsl]['offset']!=dsl:raise ValueError('VBR DOF set differs')
            metadata(obj['offset'],p,f,row['fields']['instances'][0]['fields'])
            data=prefix.with_suffix('.remapped.resource').read_bytes();env,pal,keys,stream=vbr_envelope(data)
            for offset,payload in [(136,pal),(152,keys),(168,stream)]:replacements[p+offset]=payload
            edits.extend([(p+64,struct.pack('<11f13H',*(f[k] for k in VBR_FLOATS+VBR_SHORTS))),
                          (p+144,struct.pack('<Q',len(pal)//4)),(p+160,struct.pack('<Q',len(keys)//2)),
                          (p+176,struct.pack('<Q',len(stream)))])
            checks.append(dict(codec='VBR',id=ident,name=row['name'],expected=(pal,keys,stream),animation=p))
    output,pool=repack(bank,replacements);parsed=Bank(output);base=parsed.sections['blob'][0]
    for at,payload in edits:output[base+at:base+at+len(payload)]=payload
    parsed=Bank(output)
    for item in checks:
        if item['codec']=='DCT':
            if parsed.clip(item['id'])[2]!=item['expected']:raise ValueError('DCT bank round trip failed')
        else:
            p=item['animation']
            for offset,payload in zip((136,152,168),item['expected']):
                at=parsed.pointer(p+offset)
                if parsed.blob[at:at+len(payload)]!=payload:raise ValueError('VBR bank round trip failed')
    # The opaque outer header and entire image allocation retain their original sizes.
    if output[:bank.header]!=bank.data[:bank.header] or len(parsed.blob)!=len(bank.blob):raise ValueError('Outer layout changed')
    out=d/f'nhl24_160-{name}-expanded.bank';out.write_bytes(output)
    report=dict(bank=name,path=str(out),sha256=hashlib.sha256(output).hexdigest(),
                original_sha256=hashlib.sha256(bank.data).hexdigest(),pool=pool,
                dct=sum(x['codec']=='DCT' for x in checks),vbr=sum(x['codec']=='VBR' for x in checks),
                status='private_animation_metadata_candidate_gameplay_unverified',
                clips=[{k:v for k,v in x.items() if k!='expected'} for x in checks])
    (d/f'{name}-expanded-report.json').write_text(json.dumps(report,indent=2))
    print(json.dumps({k:v for k,v in report.items() if k!='clips'},indent=2))


if __name__=='__main__':run(Path(sys.argv[1]),sys.argv[2])
