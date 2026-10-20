"""Check exact EBX partition/instance imports within the decoded sample.

This is deliberately not an effective-layer resolver. Missing sample nodes are
unresolved, never proof of missing game content. No installed files are written.
Usage: python scripts/nhl27/dependency_probe.py COMPATIBILITY_ROOT
"""
import collections
import json
import struct
import sys
import uuid
from pathlib import Path
from asset_manifest import part
from riff_evidence import chunks, fixup


def exports(data):
    f=fixup(data,details=True)
    if f['exported_instances']>len(f['instance_offsets']):
        raise ValueError('Export count exceeds instance count')
    # EBXD starts at an absolute RIFF offset; the object payload aligns to 16.
    chunks(data)
    p=12
    while part(data,p,4)!=b'EBXD':
        size=struct.unpack('<I',part(data,p+4,4))[0]
        p+=8+size+size%2
    size=struct.unpack('<I',part(data,p+4,4))[0]
    payload=(p+8+15)&~15
    end=p+8+size
    result=[]
    for offset in f['instance_offsets'][:f['exported_instances']]:
        at=payload+offset
        if at-16<payload or at+2>end:
            raise ValueError('Export offset outside EBXD')
        result.append(str(uuid.UUID(bytes_le=part(data,at-16,16))))
    if len(set(result))!=len(result):
        raise ValueError('Duplicate exported instance GUID')
    return f,result


def resolve(imports,nodes):
    """Retain every matching variant; do not pick a patch winner."""
    out=[]
    for ref in imports:
        candidates=nodes.get(ref['partition'],[])
        exact=[n for n in candidates if ref['instance'] in n['exports']]
        out.append(dict(ref,status='exact_instance_in_sample' if exact else
            'partition_only_in_sample' if candidates else 'partition_not_in_sample',
            candidates=[dict(name=n['name'],sha1=n['sha1']) for n in exact]))
    return out


def run(root):
    probes=json.loads((root/'payload-probes.json').read_text())
    probes+=json.loads((root/'presentation/probes.json').read_text())
    nodes=[];seen=set();errors=[]
    for row in probes:
        if row['kind']!='ebx' or 'payload' not in row:continue
        identity=(row['build'],row['manifest_sha1'])
        if identity in seen:continue
        seen.add(identity)
        try:
            f,ex=exports(Path(row['payload']).read_bytes())
            nodes.append(dict(build=row['build'],name=row['name'],sha1=row['manifest_sha1'],
                partition=f['partition'],exports=ex,imports=f['imports']))
        except (ValueError,KeyError) as e:
            errors.append(dict(build=row['build'],name=row['name'],error=str(e)))
    indexes={}
    for n in nodes:indexes.setdefault(n['build'],{}).setdefault(n['partition'],[]).append(n)
    rows=[]
    for n in nodes:
        if n['build']!='nhl27':continue
        for target in ['nhl27','nhl24_base','nhl24_160']:
            rows.append(dict(name=n['name'],source_sha1=n['sha1'],target=target,
                dependencies=resolve(n['imports'],indexes.get(target,{}))))
    counts={b:dict(collections.Counter(d['status'] for r in rows if r['target']==b
        for d in r['dependencies'])) for b in indexes}
    report=dict(scope='Decoded sample only; layer precedence and transitive closure unqualified',
        node_counts=dict(collections.Counter(n['build'] for n in nodes)),counts=counts,
        errors=errors,nodes=nodes,comparisons=rows)
    (root/'dependency-evidence.json').write_text(json.dumps(report,indent=2))
    print(json.dumps({k:v for k,v in report.items() if k not in ['nodes','comparisons']},indent=2))


if __name__=='__main__':run(Path(sys.argv[1]))
