"""Stage sampled DCT clips as float32 channels with verified source metadata.

No target bank is emitted. Dependency matches are GUID based, never name only.
Usage: python scripts/nhl27/animation_stage.py COMPATIBILITY_ROOT
"""
import hashlib
import json
import re
import struct
import sys
import uuid
from pathlib import Path
from animation_dct import decode,take
from compatibility import Install,frames,Oodle
from riff_evidence import fixup

DCT='3a770f61-3d12-3ca2-cdae-b384a29b07cd'


def bank_ids(data,allow_external=False):
    header,version,count,again,z1,z2,z3,names,size=struct.unpack('>9I',take(data,0,36))
    # Launch banks append two 16-byte external tables after the object ID tables.
    # Preserve these opaque tables; their fields are not object IDs.
    extra=32*z1 if allow_external and z1==z2 and z3==0 else 0
    if version!=3 or count!=again or (not extra and (z1,z2,z3)!=(0,0,0)) or header!=36+24*count+extra:
        raise ValueError('Unsupported native bank header')
    take(data,0,header)
    ids=[]
    for i in range(count):
        key=take(data,36+i*16,8)
        if any(take(data,44+i*16,8)) or key!=take(data,36+count*16+i*8,8):
            raise ValueError('Native bank ID tables disagree')
        ids.append(int.from_bytes(key,'little'))
    if len(set(ids))!=count:raise ValueError('Duplicate native bank ID')
    return set(ids)


def run(root,scan_all=False,all_dekes=False):
    dest=root/('deke-trial' if all_dekes else 'animation');dest.mkdir(exist_ok=True);ins=Install('nhl27');decoder=Oodle()
    inventories={}
    for layer in ['Data','Patch']:
        for toc in ['levels/game/game.toc','globals.toc']:
            relative=f'{layer}/Ps5/{toc}'
            inv=json.loads((root/('nhl27-'+relative.replace('/','_')+'.json')).read_text())
            inventories[relative]={(a['kind'],a['name']):a for b in inv['bundles'] for a in b['assets']}
    def read(a):
        raw=ins.read(a['span']);_,b=frames(raw,decoder)
        if hashlib.sha1(raw).hexdigest()!=a['sha1'] or b is None or len(b)!=a['size']:
            raise ValueError('Manifest hash/size mismatch')
        return b
    probes=json.loads((root/'payload-probes.json').read_text())
    selected={(p['name'],p['manifest_sha1']):p for p in probes if p['build']=='nhl27' and p['kind']=='res'
        and p['group'] in ['animation','deke']}
    if all_dekes:
        selected={}
        for toc,assets in inventories.items():
            for a in assets.values():
                if a['kind']=='res' and a.get('resource_type')=='0x4088bf7e' and re.search(r'deke|/dk[0-9]',a['name'],re.I):
                    selected[a['name'],a['sha1']]=dict(name=a['name'],manifest_sha1=a['sha1'],toc=toc)
    rows=[];needed=set()
    for (name,sha1),probe in sorted(selected.items()):
        layer=probe['toc'];assets=inventories[layer]
        row=dict(name=name,layer=layer,layer_policy='Explicit sampled occurrence; no effective-layer claim')
        try:
            res=assets['res',name];ebx=assets['ebx',name]
            if res['sha1']!=sha1:raise ValueError('Sample occurrence hash mismatch')
            data=read(res);e=read(ebx);f=fixup(e,True)
            # These source EBX samples have EBXD first with payload aligned at 32.
            if e[12:16]!=b'EBXD':raise ValueError('Unsupported EBXD placement')
            types=list(f['types']);instances=[]
            for offset in f['instance_offsets']:
                at=32+offset;idx=struct.unpack('<H',take(e,at,2))[0]
                if idx>=len(types):raise ValueError('Invalid local type index')
                if types[idx]==DCT:instances.append(at)
            if len(instances)!=1:raise ValueError('Expected one DCT object')
            if f['types'][DCT]!='0x3821ab8':raise ValueError('Unsupported DCT EBX schema')
            at=instances[0]
            if struct.unpack('<I',take(e,at+40,4))[0]!=0x44435420:raise ValueError('Not DCT codec')
            rid=struct.unpack('<Q',take(e,at+88,8))[0]
            if rid!=int(res['resource_id'],16):raise ValueError('EBX/resource ID mismatch')
            m,motion,checks=decode(data)
            q,v,floats=struct.unpack('<IHH',take(e,at+100,8))
            if (q,v,floats)!=(m['quaternions'],m['vectors'],m['float_vectors']):
                raise ValueError('EBX/DCT channel count mismatch')
            values=[x for frame in motion for channel in frame for x in channel]
            raw=struct.pack('<'+'f'*len(values),*values)
            back=struct.unpack('<'+'f'*len(values),raw)
            checks['float32_max_abs_error']=max(abs(a-b) for a,b in zip(values,back))
            prefix=dest/res['sha1'];prefix.with_suffix('.f32').write_bytes(raw)
            prefix.with_suffix('.ebx').write_bytes(e)
            prefix.with_suffix('.resource').write_bytes(data)
            needed.update(i['partition'] for i in f['imports'])
            row.update(status='neutral_motion_exported_not_target_ready',resource_sha1=res['sha1'],
                ebx_sha1=ebx['sha1'],resource_sha256=hashlib.sha256(data).hexdigest(),partition=f['partition'],
                imports=f['imports'],metadata=m,checks=checks,output=str(prefix.with_suffix('.f32')),
                output_sha256=hashlib.sha256(raw).hexdigest(),
                layout='little-endian float32 [key][quaternion, vector, float-vector channel][x,y,z,w]',
                end_frame=struct.unpack('<H',take(e,at+56,2))[0],
                trimmed_duration=struct.unpack('<f',take(e,at+52,4))[0])
            prefix.with_suffix('.json').write_text(json.dumps(row,indent=2))
        except (ValueError,KeyError) as err:row.update(status='rejected',error=str(err))
        rows.append(row)
    # Bounded candidate pass over named rig/channel definitions, excluding arena rigs.
    bindings=[];scanned=0;errors=[];catalog=[]
    candidates={a['sha1']:a for assets in inventories.values() for a in assets.values()}
    for a in candidates.values():
        kind,name=a['kind'],a['name']
        if kind!='ebx':continue
        if not all_dekes and (not name.startswith('af/') or '/arenas/' in name):continue
        if not scan_all and not re.search(r'/rigs/|dof|channel|layout',name,re.I):continue
        scanned+=1
        if scanned%10000==0:print('Dependency candidates inspected',scanned,flush=True)
        try:
            b=read(a);f=fixup(b)
            if all_dekes:catalog.append(dict(name=name,asset=a,fixup=f))
            if f['partition'] not in needed:continue
            out=dest/(a['sha1']+'-binding.ebx');out.write_bytes(b)
            bindings.append(dict(name=name,sha1=a['sha1'],partition=f['partition'],fixup=f,payload=str(out)))
        except (ValueError,KeyError) as err:errors.append(dict(name=name,error=str(err)))
    native=[]
    for build in ['nhl24_base','nhl24_160']:
        bank=(root/'ant'/build/'state.bin').read_bytes();ids=bank_ids(bank)
        native.append(dict(build=build,sha256=hashlib.sha256(bank).hexdigest(),id_count=len(ids),
            clip_root_id_matches=[r['name'] for r in rows if 'partition' in r and
                int.from_bytes(uuid.UUID(r['partition']).bytes_le[:8],'little') in ids],
            status='Identifier correspondence only; native object layout not qualified'))
    report=dict(clips=rows,native_banks=native,binding_candidates_scanned=scanned,bindings=bindings,binding_errors=errors,
        unresolved_direct_partitions=sorted(needed-{x['partition'] for x in bindings}),
        limitations=['Unbound channels: skeleton topology and names not decoded',
            'Target ANT record writer and engine evaluation unqualified',
            'No install changes; no runtime or new-move claim'])
    (dest/'stage-report.json').write_text(json.dumps(report,indent=2))
    if all_dekes:
        (dest/'source-catalog.json').write_text(json.dumps(catalog))
    print(json.dumps(dict(clips=len(rows),exported=sum(x['status'].startswith('neutral') for x in rows),
        rejected=[dict(name=x['name'],error=x['error']) for x in rows if 'error'in x],
        bindings_scanned=scanned,bindings=len(bindings),binding_errors=len(errors),
        unresolved=len(report['unresolved_direct_partitions'])),indent=2))


if __name__=='__main__':run(Path(sys.argv[1]),'--scan-all-af' in sys.argv[2:],'--all-dekes' in sys.argv[2:])
