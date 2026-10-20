"""Compare source/target shared field layouts using GUIDs and field name hashes.

Usage: python scripts/nhl27/schema_compare.py COMPATIBILITY_ROOT
Requires target_initfs.ps1 and type_evidence.py outputs. No schema rewriting.
"""
import json
import collections
import struct
import sys
import uuid
from pathlib import Path
from asset_manifest import part
from riff_evidence import chunks


def shared(data,known=()):
    cs=chunks(data,b'EBXT');tag='RFL2' if 'RFL2'in cs else 'REFL';b=cs[tag];p=0
    def take(n):
        nonlocal p
        v=part(b,p,n);p+=n;return v
    def u():return struct.unpack('<I',take(4))[0]
    sig=[(str(uuid.UUID(bytes_le=take(16))),hex(u())) for _ in range(u())]
    ts=[struct.unpack('<IIHHHH',take(16)) for _ in range(u())]
    fs=[struct.unpack('<IIHH',take(12)) for _ in range(u())]
    take(u()*12);take(u()*8)
    if tag=='RFL2' and u()!=0:raise ValueError('Unsupported RFL2 suffix')
    if p!=len(b) or len(sig)<len(ts) or any(x not in known for x in sig[len(ts):]):raise ValueError('Descriptor count/length mismatch')
    rows=[]
    for i,(h,first,n,flags,size,align) in enumerate(ts):
        if first+n>len(fs):raise ValueError('Field bounds')
        fields=[]
        for fh,off,f,ref in fs[first:first+n]:
            if ref!=65535 and ref>=len(sig):raise ValueError('Referenced type bounds')
            fields.append(dict(hash=hex(fh),offset=off,flags=hex(f),type_guid=sig[ref][0] if ref!=65535 else None))
        rows.append(dict(guid=sig[i][0],signature=sig[i][1],name_hash=hex(h),size=size,alignment=align,flags=hex(flags),fields=fields))
    return rows


def run(root):
    source=shared((root/'source-types.ebx').read_bytes());base=shared((root/'target-base-SharedTypeDescriptors.ebx').read_bytes())
    patch=shared((root/'target-patch-SharedTypeDescriptors_patch.ebx').read_bytes(),{(x['guid'],x['signature']) for x in base})
    names={(x['guid'],x['signature']):x for x in json.loads((root/'source-type-descriptors.json').read_text())}
    target_names=collections.defaultdict(set)
    candidate_file=root/'target-field-name-candidates.json'
    if candidate_file.exists():
        for x in json.loads(candidate_file.read_text()):target_names[(x['build'],x['hash'])].add(x['name'])
    comparisons=[]
    # Preserve signature variants instead of assuming patch records replace all versions of a GUID.
    for build,rows in [('nhl24_base',base),('nhl24_160',base+patch)]:
        target={}
        for x in rows:target.setdefault(x['guid'],{})[x['signature']]=x
        for s in source:
            named=names.get((s['guid'],s['signature']),{});nm=named.get('name_candidates',[])
            fnames={x['name_hash']:x['name_candidates'] for x in named.get('fields',[])}
            candidates=list(target.get(s['guid'],{}).values())
            if not candidates:
                comparisons.append(dict(target=build,name_candidates=nm,guid=s['guid'],source_signature=s['signature'],status='type_guid_absent',source_size=s['size']));continue
            for t in candidates:
                a={x['hash']:x for x in s['fields']};b={x['hash']:x for x in t['fields']}
                changes=[]
                for h in sorted(a.keys()|b.keys()):
                    if a.get(h)==b.get(h):continue
                    changes.append(dict(field_hash=h,name_candidates=fnames.get(h,[]),source=a.get(h),target=b.get(h)))
                named_fields={}
                for side,fields in [('source',s['fields']),('target',t['fields'])]:
                    for f in fields:
                        ns=fnames.get(f['hash'],[]) if side=='source' else sorted(target_names.get((build,f['hash']),set()))
                        if f['offset']==0 and f['type_guid'] and int(f['flags'],16)>>5==0:ns=['$base']
                        if len(ns)==1:named_fields.setdefault(ns[0],{})[side]=f
                comparisons.append(dict(target=build,name_candidates=nm,guid=s['guid'],source_signature=s['signature'],target_signature=t['signature'],
                    status='signature_equal' if s['signature']==t['signature'] else 'signature_changed',source_size=s['size'],target_size=t['size'],
                    source_alignment=s['alignment'],target_alignment=t['alignment'],field_changes=changes,named_field_correspondence=named_fields))
    (root/'target-types-base.json').write_text(json.dumps(base,indent=2));(root/'target-types-patch.json').write_text(json.dumps(patch,indent=2));(root/'schema-comparison.json').write_text(json.dumps(comparisons,indent=2))
    print('source types',len(source),'target base',len(base),'patch',len(patch),'comparisons',len(comparisons))


if __name__=='__main__':run(Path(sys.argv[1]))
