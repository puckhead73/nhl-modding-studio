"""Bounded RIFF EBX fixup inspection; no typed object deserialization or writing.

Format cross-check: FrostyToolsuite/FrostySdk/IO/RiffEbx/EbxFixup.cs.
Usage: python scripts/nhl27/riff_evidence.py COMPATIBILITY_ROOT
"""
import collections
import json
import struct
import sys
import uuid
from pathlib import Path
from asset_manifest import part


def chunks(data, form=b'EBX\0'):
    accepted=(b'EBX\0',b'EBXS') if form==b'EBX\0' else (form,)
    if part(data,0,4)!=b'RIFF' or part(data,8,4) not in accepted:raise ValueError('Unsupported RIFF form')
    if struct.unpack_from('<I',data,4)[0]+8!=len(data):raise ValueError('RIFF length mismatch')
    p=12;out={}
    while p<len(data):
        tag=part(data,p,4).decode('ascii');n=struct.unpack('<I',part(data,p+4,4))[0]
        if tag in out:raise ValueError('Duplicate RIFF chunk')
        out[tag]=part(data,p+8,n);p+=8+n+n%2
    if p!=len(data):raise ValueError('RIFF padding mismatch')
    return out


def fixup(data, details=False):
    cs=chunks(data);b=cs['EFIX'];p=0
    def take(n):
        nonlocal p
        v=part(b,p,n);p+=n;return v
    def u32():return struct.unpack('<I',take(4))[0]
    def array(width):return [v for v in (take(width) for _ in range(u32()))]
    def nums():return [struct.unpack('<I',v)[0] for v in array(4)]
    guid=lambda v:str(uuid.UUID(bytes_le=v))
    result={'partition':guid(take(16))}
    types=[guid(v) for v in array(16)];sigs=nums()
    if len(sigs)>len(types):raise ValueError('More signatures than types')
    result['types']=dict(zip(types,[hex(v) for v in sigs]+[None]*(len(types)-len(sigs))))
    result['exported_instances']=u32()
    instances=nums();pointers=nums();resources=nums()
    imports=array(32);import_offsets=nums();type_offsets=nums()
    result.update(instance_count=len(instances),pointer_count=len(pointers),resource_ref_count=len(resources),
        imports=[dict(partition=guid(v[:16]),instance=guid(v[16:])) for v in imports],
        import_offset_count=len(import_offsets),type_offset_count=len(type_offsets),
        array_offset=u32(),boxed_offset=u32(),string_offset=u32())
    # Observed 2021+ form ends with one reserved word.
    result['reserved']=u32() if len(b)-p==4 else None
    if p!=len(b):raise ValueError('EFIX trailing bytes')
    result['chunk_sizes']={k:len(v) for k,v in cs.items()}
    if details:
        result['instance_offsets']=instances
        result['pointer_offsets']=pointers
        result['resource_offsets']=resources
    return result


def run(root):
    probes=json.loads((root/'payload-probes.json').read_text());rows=[]
    for a in probes:
        if a['kind']!='ebx' or 'payload' not in a:continue
        row={k:a[k] for k in ['build','name','manifest_sha1','group','decoded_sha256']}
        try:row['fixup']=fixup(Path(a['payload']).read_bytes())
        except (ValueError,KeyError) as e:row['error']=str(e)
        rows.append(row)
    (root/'riff-fixups.json').write_text(json.dumps(rows,indent=2))
    comparison=[]
    for a in rows:
        if a['build']!='nhl27' or 'fixup' not in a:continue
        for b in rows:
            if b['build']=='nhl27' or a['name']!=b['name'] or 'fixup' not in b:continue
            x=a['fixup']['types'];y=b['fixup']['types']
            comparison.append(dict(name=a['name'],target=b['build'],source_hash=a['manifest_sha1'],target_hash=b['manifest_sha1'],
                same_decoded_bytes=a['decoded_sha256']==b['decoded_sha256'],same_partition=a['fixup']['partition']==b['fixup']['partition'],
                matching_types=sorted(k for k in x if x[k] is not None and y.get(k)==x[k]),
                changed_signatures={k:dict(source=x[k],target=y[k]) for k in x if k in y and x[k] is not None and y[k] is not None and x[k]!=y[k]},
                unresolved_signatures=sorted(k for k in x if x[k] is None or (k in y and y[k] is None)),
                source_only_types={k:v for k,v in x.items() if k not in y}))
    (root/'riff-comparison.json').write_text(json.dumps(comparison,indent=2))
    print('parsed',sum('fixup'in x for x in rows),'errors',collections.Counter(x['error'] for x in rows if 'error'in x),'paired',len(comparison))


if __name__=='__main__':run(Path(sys.argv[1]))
