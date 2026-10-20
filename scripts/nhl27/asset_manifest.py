"""Read PS5 64-bit CAS references and bounded bundle manifests, without writes to games.

Usage: python scripts/nhl27/asset_manifest.py GAME_ROOT TOC_RELATIVE BUNDLE_REGEX OUTPUT
Only observed 0/0x80/0x84 span encodings are accepted. No patch precedence is inferred.
"""
import argparse
import json
import re
import struct
from pathlib import Path
from bundle_inventory import bundles
from dbobject import parse


def part(b, p, n):
    if p < 0 or n < 0 or p+n > len(b): raise ValueError('Range exceeds input')
    return b[p:p+n]


def spans(record):
    flags, count, start = struct.unpack_from('>III', part(record, 8, 12))
    bits = part(record, flags, count)
    p = start; current = None; out = []
    for flag in bits:
        if flag in (0x80, 0x84):
            value = struct.unpack('>Q', part(record, p, 8))[0]; p += 8
            current = (value >> 48, (value >> 16) & 0xffffffff, value & 0xffff)
        elif flag != 0: raise ValueError(f'Unsupported span flag {flag:#x}')
        if current is None: raise ValueError('Missing CAS identifier')
        off, size = struct.unpack('>II', part(record, p, 8)); p += 8
        out.append(dict(layer=current[0], persistent_index=current[1], cas=current[2], offset=off, size=size))
    if p != flags or flags+count != len(record): raise ValueError('Span consumption mismatch')
    return out


def manifest(data):
    size = struct.unpack('>I', part(data, 0, 4))[0]
    if size+4 != len(data): raise ValueError('Manifest size mismatch')
    b = data[4:]
    magic, total, ne, nr, nc, strings, meta, ml = struct.unpack('<8I', part(b, 0, 32))
    if magic != 0x9d798ed6 or total != ne+nr+nc: raise ValueError('Manifest header mismatch')
    part(b, meta, ml)
    ebx = 32+total*20; res = ebx+ne*8; types = res+nr*8
    metas = types+nr*4; ids = metas+nr*16; chunks = ids+nr*8
    end = chunks+nc*24; part(b, 0, end)
    if total and not end <= strings < len(b): raise ValueError('Bad string table')
    out = []
    for i in range(total):
        a = dict(sha1=part(b, 32+i*20, 20).hex())
        if i < ne+nr:
            j = i if i < ne else i-ne; p = (ebx if i < ne else res)+j*8
            off, n = struct.unpack('<II', part(b, p, 8)); off += strings
            part(b, off, 1); z = b.find(b'\0', off)
            if z < off or z-off > 8192: raise ValueError('Bad asset name')
            a.update(kind='ebx' if i < ne else 'res', name=b[off:z].decode('utf8'), size=n)
            if i >= ne:
                a.update(resource_type=hex(struct.unpack('<I',part(b,types+j*4,4))[0]),
                         resource_id=part(b,ids+j*8,8)[::-1].hex(), resource_meta=part(b,metas+j*16,16).hex())
        else:
            p = chunks+(i-ne-nr)*24
            logical, n = struct.unpack('<II', part(b,p+16,8))
            a.update(kind='chunk',name=part(b,p,16).hex(),size=n,logical_offset=logical)
        out.append(a)
    return out


def run(root, toc, pattern, output):
    layout = parse(root/'Data/layout.toc')
    mapping = {x['persistentIndex'] & 0xffffffff:x['name'] for x in layout['installManifest']['installChunks']}
    def read_span(s):
        if s['layer'] not in (0,1): raise ValueError('Unknown CAS layer')
        path = root/('Data' if s['layer']==0 else 'Patch')/'Ps5'/mapping[s['persistent_index']]/f"cas_{s['cas']:02d}.cas"
        if not path.resolve().is_relative_to(root.resolve()): raise ValueError('CAS escapes game root')
        if s['offset']+s['size'] > path.stat().st_size: raise ValueError('CAS span exceeds file')
        with path.open('rb') as f: f.seek(s['offset']); return f.read(s['size'])
    rows, info = bundles(root/toc); data = (root/toc).read_bytes()
    if data[:4] == bytes.fromhex('00d1ce01'): data=data[0x22c:]
    result=[]
    for row in rows:
        if not re.search(pattern,row['name'],re.I): continue
        if row['record_flag'] != 1: raise ValueError('Unsupported bundle record')
        ss=spans(part(data,row['record_offset'],row['record_size']))
        assets=manifest(read_span(ss[0]))
        if len(assets)+1 != len(ss): raise ValueError('Asset/span count mismatch')
        for a,s in zip(assets,ss[1:]): a['span']=s
        result.append(dict(bundle=row['name'],assets=assets))
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(json.dumps(dict(toc=toc,toc_sha256=info['sha256'],bundles=result),indent=2))
    print('bundles',len(result),'asset occurrences',sum(len(x['assets']) for x in result))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('root',type=Path);p.add_argument('toc');p.add_argument('pattern');p.add_argument('output',type=Path)
    a=p.parse_args();run(a.root,a.toc,a.pattern,a.output)
