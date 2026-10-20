"""Prove a narrow EBX container conversion against exact NHL24 reference bytes.

Only removes the observed extra zero EFIX word when *all* other chunks match
the reference. Rejects changed layouts/content. Never patches an installation.
Usage: python scripts/nhl27/container_probe.py COMPATIBILITY_ROOT
"""
import hashlib
import json
import struct
import sys
from pathlib import Path
from riff_evidence import chunks


def normalize(source,target):
    a=chunks(source);b=chunks(target)
    if source[8:12]!=target[8:12] or list(a)!=list(b):raise ValueError('Different form/chunk order')
    if a['EFIX'][-4:]!=bytes(4) or a['EFIX'][:-4]!=b['EFIX']:raise ValueError('EFIX differs beyond reserved word')
    if any(a[k]!=b[k] for k in a if k!='EFIX'):raise ValueError('Payload chunks differ')
    a['EFIX']=a['EFIX'][:-4]
    body=source[8:12]+b''.join(k.encode()+struct.pack('<I',len(v))+v+bytes(len(v)%2) for k,v in a.items())
    result=b'RIFF'+struct.pack('<I',len(body))+body
    if result!=target:raise ValueError('Conversion does not exactly reproduce target')
    return result


def run(root):
    probes=json.loads((root/'payload-probes.json').read_text());results=[]
    source=[x for x in probes if x['build']=='nhl27' and x['kind']=='ebx' and 'payload' in x]
    target=[x for x in probes if x['build']!='nhl27' and x['kind']=='ebx' and 'payload' in x]
    for a in source:
        for b in target:
            if a['name']!=b['name']:continue
            row=dict(name=a['name'],target=b['build'],source_hash=a['manifest_sha1'],target_hash=b['manifest_sha1'])
            try:
                output=normalize(Path(a['payload']).read_bytes(),Path(b['payload']).read_bytes())
                dest=root/'normalized-controls'/b['build']/(a['manifest_sha1']+'.ebx');dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(output)
                row.update(result='byte_exact_target_control',sha256=hashlib.sha256(output).hexdigest(),output=str(dest))
            except ValueError as e:row.update(result='rejected',reason=str(e))
            results.append(row)
    (root/'container-validation.json').write_text(json.dumps(results,indent=2))
    print('paired',len(results),'exact controls',sum(x['result']=='byte_exact_target_control' for x in results))


if __name__=='__main__':run(Path(sys.argv[1]))
