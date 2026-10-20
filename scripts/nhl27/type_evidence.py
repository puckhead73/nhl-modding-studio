"""Resolve source types through relocated names and matching GUID/signature pairs.

Usage: python scripts/nhl27/type_evidence.py RESEARCH_ROOT
No target layout is inferred from an equal source type name.
"""
import collections
import json
import sqlite3
import struct
import sys
import uuid
from pathlib import Path
from asset_manifest import part
from riff_evidence import chunks
from research import ElfImage
from dbobject import parse


def run(root):
    source=Path(json.loads((root/'identity.json').read_text())['path']).parent
    initfs=parse(source/'Data/initfs_Ps5',keep_blobs=True)
    descriptor=next(x['$file']['payload'] for x in initfs if x.get('$file',{}).get('name')=='SharedTypeDescriptors.ebx')
    (root/'compatibility').mkdir(parents=True,exist_ok=True)
    (root/'compatibility/source-types.ebx').write_bytes(descriptor)
    b=chunks((root/'compatibility/source-types.ebx').read_bytes(),b'EBXT')['RFL2'];p=0
    def take(n):
        nonlocal p
        v=part(b,p,n);p+=n;return v
    def u():return struct.unpack('<I',take(4))[0]
    signatures=[]
    for _ in range(u()):signatures.append((str(uuid.UUID(bytes_le=take(16))),hex(u())))
    types=[struct.unpack('<IIHHHH',take(16)) for _ in range(u())]
    fields=[struct.unpack('<IIHH',take(12)) for _ in range(u())]
    names=collections.defaultdict(set);runtime={};field_names=collections.defaultdict(set)
    image=ElfImage(json.loads((root/'identity.json').read_text())['path'])
    expected=set(signatures)
    with sqlite3.connect(f'file:{(root/"index.sqlite").as_posix()}?mode=ro',uri=True) as db:
        reloc=dict(db.execute('SELECT site,addend FROM relocations WHERE type=8'))
        strings=dict(db.execute('SELECT va,text FROM strings'))
        for site,s in db.execute('SELECT r.site,s.text FROM relocations r JOIN strings s ON r.addend=s.va WHERE r.type=8'):
            try:
                raw=image.read(site,56);h=struct.unpack_from('<I',raw,8)[0]
                identity=(str(uuid.UUID(bytes_le=raw[16:32])),hex(struct.unpack_from('<I',raw,52)[0]))
                if identity not in expected:continue
                names[h].add(s);runtime[identity]=hex(site)
                kind=struct.unpack_from('<H',raw,12)[0]&0x1f0
                field_count=struct.unpack_from('<H',raw,50)[0]
                field_pointer=reloc.get(site+(64 if kind==0x60 else 104 if kind==0x40 else 0))
                if field_pointer and kind in (0x60,0x40) and field_count<10000:
                    for j in range(field_count):
                        at=field_pointer+j*32
                        fr=image.read(at,32);fh=struct.unpack_from('<I',fr,8)[0];off=struct.unpack_from('<I',fr,16)[0]
                        name=strings.get(reloc.get(at))
                        if name:field_names[(identity,fh,off)].add(name)
            except (ValueError,struct.error):continue
    rows=[]
    for i,(name,first,count,flags,size,align) in enumerate(types):
        if first+count>len(fields):raise ValueError('Field descriptor bounds')
        row=dict(index=i,name_hash=hex(name),name_candidates=sorted(names[name]),flags=hex(flags),size=size,alignment=align,
                 fields=[dict(name_hash=hex(h),name_candidates=sorted(names[h]),offset=o,flags=hex(f),type_ref=t) for h,o,f,t in fields[first:first+count]])
        if i<len(signatures):row.update(guid=signatures[i][0],signature=signatures[i][1],runtime_descriptor=runtime.get(signatures[i]))
        for f in row['fields']:
            f['name_candidates']=sorted(field_names.get((signatures[i],int(f['name_hash'],16),f['offset']),set()))
        rows.append(row)
    # Parse the remaining descriptor index tables rather than silently accepting a suffix.
    n=u();take(n*12);n=u();take(n*8);last=u()
    if last!=0 or p!=len(b):raise ValueError('Unsupported RFL2 suffix')
    (root/'compatibility/source-type-descriptors.json').write_text(json.dumps(rows,indent=2))
    print('types',len(types),'signatures',len(signatures),'fields',len(fields),'uniquely named',sum(len(r['name_candidates'])==1 for r in rows))


if __name__=='__main__':run(Path(sys.argv[1]))
