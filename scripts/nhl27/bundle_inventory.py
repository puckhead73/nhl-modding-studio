"""Read bounded PS4/PS5 TOC bundle names without guessing asset span encodings.

Usage: python scripts/nhl27/bundle_inventory.py GAME_ROOT Gen4b|Ps5 OUTPUT
Retains every layer occurrence. Patch-name overlap is a candidate override,
not proof of complete resource-level precedence or payload compatibility.
"""
import argparse
import hashlib
import json
import struct
from pathlib import Path


def bundles(path):
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if data[:4] == bytes.fromhex('00d1ce01'): data = data[0x22c:]
    elif data[:3] == bytes.fromhex('00d1ce'): raise ValueError('Unsupported wrapper')
    if len(data) < 60: raise ValueError('Short TOC')
    w = struct.unpack_from('>15I', data)
    if w[0] != 60: raise ValueError('Not a superbundle TOC')
    def span(p, n):
        if p < 0 or n < 0 or p+n > len(data): raise ValueError('TOC range exceeds file')
        return data[p:p+n]
    entries = span(w[1], w[2]*16)
    if w[11] & 4 and w[2]:
        tree = struct.unpack('>'+'i'*w[13], span(w[14], w[13]*4))
        words = struct.unpack('>'+'I'*w[12], span(w[8], w[12]*4))
        if len(tree)<2 or len(tree)%2: raise ValueError('Invalid name tree')
    def name(start):
        if not w[11]&4:
            p = w[8]+start
            end = data.find(b'\0', p)
            if end < p or end-p > 4096: raise ValueError('Invalid name')
            return data[p:end].decode('ascii')
        bit = start; chars = []
        while len(chars)<4096:
            node = len(tree)//2-1
            while True:
                if bit >= len(words)*32: raise ValueError('Name bitstream overflow')
                branch = (words[bit//32] >> (bit%32))&1; bit += 1
                v = tree[node*2+branch]
                if v < 0:
                    c = ~v
                    if c == 0: return ''.join(chars)
                    if c > 127: raise ValueError('Non-ASCII name')
                    chars.append(chr(c)); break
                if v >= node: raise ValueError('Invalid tree edge')
                node = v
        raise ValueError('Oversized name')
    rows = []
    for ordinal, (nw, sz, off) in enumerate(struct.iter_unpack('>IIQ', entries)):
        row = {'ordinal': ordinal, 'name': name(nw), 'record_flag': sz>>30,
               'record_offset': off, 'record_size': sz&0x3fffffff}
        if sz>>30 == 1:
            r = span(off, sz&0x3fffffff)
            if len(r)<32: raise ValueError('Short bundle record')
            count = struct.unpack_from('>I',r,12)[0]
            if count>len(r)-32: raise ValueError('Invalid span flags count')
            row['span_count'] = count
            row['span_flag_values'] = sorted(set(r[len(r)-count:])) if count else []
        rows.append(row)
    return rows, {'sha256':digest,'bundles':len(rows),'chunks':w[5],
                  'chunk_location_words':w[10], 'toc_bytes':len(data)}


def run(root, platform, out):
    out.mkdir(parents=True, exist_ok=True)
    metadata = []; errors = []; total=0
    with (out/'bundles.jsonl').open('w',encoding='utf8') as f:
        for layer in ['Data','Patch']:
            for path in sorted((root/layer/platform).rglob('*.toc')):
                try:
                    rows, info = bundles(path)
                    rel = path.relative_to(root).as_posix()
                    metadata.append(dict(info,path=rel,layer=layer))
                    for row in rows:
                        f.write(json.dumps(dict(row,layer=layer,toc=rel))+'\n')
                    total += len(rows)
                    print(rel, len(rows), flush=True)
                except (ValueError,struct.error,UnicodeError) as e:
                    errors.append({'path':str(path),'error':str(e)})
    (out/'summary.json').write_text(json.dumps({'root':str(root),'platform':platform,
        'bundle_occurrences':total,'tocs':metadata,'errors':errors},indent=2))
    print('TOTAL',total,'ERRORS',len(errors),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('root',type=Path);p.add_argument('platform',choices=['Gen4b','Ps5']);p.add_argument('output',type=Path)
    a=p.parse_args();run(a.root,a.platform,a.output)
