"""Read-only cross-install manifest/payload comparison for NHL27 and NHL24.

Usage: python scripts/nhl27/compatibility.py OUTPUT_ROOT
Explicit local install paths prevent accidental layer fallback. Outputs are private.
"""
import collections
import ctypes
import hashlib
import json
import re
import struct
import sys
from pathlib import Path
from asset_manifest import part, spans, manifest
from bundle_inventory import bundles
from dbobject import parse

ROOTS={
 'nhl27':Path(r'W:\Downloads\NHL 27\PPSA34063-app0'),
 'nhl24_base':Path(r'F:\Games\NHL 24\game'),
 'nhl24_160':Path(r'F:\Games\NHL 24\game-UPDATE'),
}
PATTERNS={
 'animation':r'^af/assets/',
 'deke':r'^af/assets/.*deke',
 'presentation':r'^presentation/|^content/camera/',
 'commentary':r'^sound/speech/maincommentary/',
}


def ps4_spans(record):
    count=struct.unpack('>I',part(record,12,4))[0]
    flags=len(record)-count;p=32;current=None;out=[]
    for flag in part(record,flags,count):
        if flag==1:
            current=struct.unpack('>I',part(record,p,4))[0];p+=4
        elif flag!=0:raise ValueError('Unsupported PS4 flag')
        if current is None or current>>24 or (current>>16)&255>1:raise ValueError('Invalid PS4 identifier')
        off,size=struct.unpack('>II',part(record,p,8));p+=8
        out.append(dict(layer=current>>16,catalog=(current>>8)&255,cas=current&255,offset=off,size=size))
    if p!=flags:raise ValueError('PS4 span stream mismatch')
    return out


class Install:
    def __init__(self,label):
        self.label=label;self.root=ROOTS[label];self.ps5=label=='nhl27'
        self.platform='Ps5' if self.ps5 else 'Gen4b'
        self.mapping={}
        if self.ps5:
            for x in parse(self.root/'Data/layout.toc')['installManifest']['installChunks']:
                self.mapping[x['persistentIndex']&0xffffffff]=x['name']

    def read(self,s):
        if s['size']>256*1024*1024:raise ValueError('Span exceeds research limit')
        if s['layer'] not in (0,1):raise ValueError('Unsupported layer')
        root=ROOTS['nhl24_base'] if self.label=='nhl24_160' and s['layer']==0 else self.root
        layer='Patch' if s['layer'] else 'Data'
        folder=self.mapping[s['persistent_index']] if self.ps5 else f"superbundlelayout/nhl_installpackage_{s['catalog']:02d}"
        path=root/layer/self.platform/folder/f"cas_{s['cas']:02d}.cas"
        if not path.resolve().is_relative_to(root.resolve()):raise ValueError('Path escapes root')
        if s['offset']+s['size']>path.stat().st_size:raise ValueError('CAS span beyond EOF')
        with path.open('rb') as f:f.seek(s['offset']);return f.read(s['size'])

    def inventory(self,relative,pattern='.*'):
        path=self.root/relative;rows,info=bundles(path);b=path.read_bytes()
        if b[:4]==bytes.fromhex('00d1ce01'):b=b[0x22c:]
        out=[]
        for row in rows:
            if not re.search(pattern,row['name'],re.I):continue
            if row['record_flag']!=1:raise ValueError('Unsupported bundle record')
            r=part(b,row['record_offset'],row['record_size'])
            ss=spans(r) if self.ps5 else ps4_spans(r)
            assets=manifest(self.read(ss[0]))
            if len(assets)+1!=len(ss):raise ValueError('Manifest/span mismatch')
            for a,s in zip(assets,ss[1:]):a['span']=s
            out.append(dict(bundle=row['name'],assets=assets))
        return dict(toc=relative,toc_sha256=info['sha256'],bundles=out)


def frames(data, decoder=None):
    """Inspect CAS framing; optionally decode codec 0x11 with a supplied decoder."""
    p=0;rows=[];decoded=[];total=0
    while p<len(data):
        h=part(data,p,8);n=struct.unpack('>I',h[:4])[0]
        size=((h[5]&15)<<16)|int.from_bytes(h[6:8],'big')
        if h[5]&0xf0!=0x70 or not 0<n<=0x100000 or not size:raise ValueError('Unsupported CAS framing')
        total+=n
        if total>256*1024*1024:raise ValueError('Decoded asset exceeds research limit')
        payload=part(data,p+8,size)
        if h[4]==0:
            if size!=n:raise ValueError('Stored block length mismatch')
            decoded.append(payload)
        elif h[4]==0x11 and decoder is not None:
            block=decoder(payload,n)
            if len(block)!=n:raise ValueError('Decoder length mismatch')
            decoded.append(block)
        rows.append(dict(codec=h[4],stored=size,decoded=n));p+=8+size
    return rows,b''.join(decoded) if all(x['codec']==0 or (x['codec']==0x11 and decoder is not None) for x in rows) else None


class Oodle:
    """Use the already installed UEViewer decoder; no DLL download or distribution."""
    def __init__(self):
        self.path=Path(r'E:\Repositories\UEViewer\oo2core_8_win64.dll')
        self.dll=ctypes.CDLL(str(self.path));self.fn=self.dll.OodleLZ_Decompress
        self.fn.restype=ctypes.c_ssize_t
        self.fn.argtypes=[ctypes.c_void_p,ctypes.c_ssize_t,ctypes.c_void_p,ctypes.c_ssize_t,
            ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_void_p,ctypes.c_ssize_t,
            ctypes.c_void_p,ctypes.c_void_p,ctypes.c_void_p,ctypes.c_ssize_t,ctypes.c_int]
    def __call__(self,data,size):
        src=ctypes.create_string_buffer(data);dst=ctypes.create_string_buffer(size)
        n=self.fn(src,len(data),dst,size,1,0,0,None,0,None,None,None,0,3)
        if n!=size:raise ValueError(f'Oodle result {n}, expected {size}')
        return dst.raw


def run(out):
    out.mkdir(parents=True,exist_ok=True);indexes={};summaries={}
    for label in ROOTS:
        install=Install(label);layer='Patch' if label=='nhl24_160' else 'Data'
        paths=[f'{layer}/{install.platform}/levels/game/game.toc',f'{layer}/{install.platform}/globals.toc']
        if label=='nhl27':paths+=['Patch/Ps5/levels/game/game.toc','Patch/Ps5/globals.toc']
        index={}
        for relative in paths:
            result=install.inventory(relative)
            (out/f'{label}-{relative.replace("/","_")}.json').write_text(json.dumps(result))
            for bundle in result['bundles']:
                for a in bundle['assets']:
                    key=(a['kind'],a['name'])
                    # Keep every occurrence; selecting a representative below is not an effective-layer resolver.
                    index.setdefault(key,[])
                    index[key].append(dict(a,toc=relative,bundle=bundle['bundle']))
            print(label,relative,'bundles',len(result['bundles']),flush=True)
        indexes[label]=index
    # Compare each source occurrence with any available destination occurrence.
    report={}
    for group,pattern in PATTERNS.items():
        rows=[]
        for (kind,name),occ in indexes['nhl27'].items():
            if not re.search(pattern,name,re.I):continue
            old=indexes['nhl24_base'].get((kind,name),[])+indexes['nhl24_160'].get((kind,name),[])
            rows.append(dict(kind=kind,name=name,source_variants=len({a['sha1'] for a in occ}),
                source_hashes=sorted({a['sha1'] for a in occ}),target_hashes=sorted({a['sha1'] for a in old}),
                status='absent_in_sampled_target_manifests' if not old else 'shared_stored_hash' if {a['sha1'] for a in occ}&{a['sha1'] for a in old} else 'different_stored_bytes'))
        report[group]=dict(counts=dict(collections.Counter(x['status'] for x in rows)),items=rows)
    (out/'comparison.json').write_text(json.dumps(report,indent=2))
    # Payload probes include shared and source-only names, all occurrences, with format evidence.
    probes=[];decoder=Oodle()
    (out/'decoder.json').write_text(json.dumps(dict(path=str(decoder.path),sha256=hashlib.sha256(decoder.path.read_bytes()).hexdigest())))
    for label,index in indexes.items():
        install=Install(label)
        for group,pattern in PATTERNS.items():
            candidates=[(k,v) for k,v in index.items() if re.search(pattern,k[1],re.I)]
            chosen=dict(sorted(candidates,key=lambda x:(not ('database/' in x[0][1]),x[0]))[:30])
            for kind in ('ebx','res'):
                for key,occ in sorted(candidates):
                    if key[0]!=kind:continue
                    if key in indexes['nhl27'] and (key in indexes['nhl24_base'] or key in indexes['nhl24_160']):
                        chosen[key]=occ
                        if sum(k[0]==kind for k in chosen)>=60:break
                for key,occ in sorted((k,v) for k,v in candidates if k[0]==kind)[:10]:chosen[key]=occ
            for (kind,name),occ in chosen.items():
                for a in {x['sha1']:x for x in occ}.values():
                    raw=install.read(a['span']);row=dict(build=label,group=group,kind=kind,name=name,manifest_sha1=a['sha1'],toc=a['toc'],raw_sha256=hashlib.sha256(raw).hexdigest(),stored_sha1_matches=hashlib.sha1(raw).hexdigest()==a['sha1'])
                    try:
                        blocks,decoded=frames(raw,decoder);row['blocks']=blocks
                        if decoded is not None:
                            row.update(decoded_size=len(decoded),manifest_size_matches=len(decoded)==a['size'],decoded_sha256=hashlib.sha256(decoded).hexdigest(),header=decoded[:64].hex())
                            dest=out/'payloads'/label/(a['sha1']+'.bin');dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(decoded)
                            row['payload']=str(dest)
                    except ValueError as e:row['error']=str(e)
                    probes.append(row)
    (out/'payload-probes.json').write_text(json.dumps(probes,indent=2))
    print(json.dumps({k:v['counts'] for k,v in report.items()},indent=2))


if __name__=='__main__':run(Path(sys.argv[1]))
