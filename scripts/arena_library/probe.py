"""Decode bounded arena/presentation specimens and city-video metadata.

python scripts/arena_library/probe.py [--only nhl15-ps4,nhl24-ps4]
Reads census locations; records hashes and decoder failures, never installs assets.
"""
import argparse
import collections
import gzip
import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

from census import REPO, write_json, category
from compatibility import frames, Oodle
from riff_evidence import fixup
from ps4_leaguelogos import chunkzip, gnf_info

ROOT=REPO/'docs/research/arena-library'


def assets(folder):
    path=folder/'assets.jsonl'
    if path.exists():f=path.open(encoding='utf8')
    else:f=gzip.open(path.with_suffix('.jsonl.gz'),'rt',encoding='utf8')
    with f:
        for line in f:yield json.loads(line)


def read(a,limit=32*1024*1024):
    size=a.get('span',{}).get('size',a['size'])
    if size>limit:raise ValueError(f'Payload above specimen limit: {size}')
    off=a.get('span',{}).get('offset',a.get('physical_offset',a.get('offset',0)))
    with open(a['physical'],'rb') as f:f.seek(off);raw=f.read(size)
    if len(raw)!=size:raise ValueError('Short payload read')
    return raw


def probe(folder,decoder):
    chosen={};city={};scripts={};budget=collections.Counter()
    for a in assets(folder):
        n=a['name'];kind=a['kind'];cat=category(n)
        if '/cityintro/' in n and kind!='chunk':city.setdefault(n,a)
        if kind=='chunk':continue
        if n.endswith(('.lua','.json','.csv','.xml')) and re.search('intro|pregame|movie|camera',n,re.I) and a['size']<1024*1024:
            scripts.setdefault(n,a)
        # Every source gets bounded specimens from each arena-related domain.
        domain='stadium_geometry' if '/stadium' in n and (n.endswith('.mcd') or 'mesh' in n) else cat
        relevant='/stadium' in n or cat in ('camera','presentation','screens','lighting','movies') or ('database' in n and 'arena' in n)
        key=(kind,domain)
        if relevant and budget[key]<6 and (kind,n) not in chosen and a['size']<=8*1024*1024:
            chosen[kind,n]=a;budget[key]+=1
    rows=[]
    for a in chosen.values():
        r={k:a[k] for k in ('name','kind','physical','size')};r['category']=category(a['name'])
        try:
            raw=read(a);r['stored_sha256']=hashlib.sha256(raw).hexdigest()
            if 'sha1' in a:
                r['manifest_sha1_match']=hashlib.sha1(raw).hexdigest()==a['sha1']
                if not r['manifest_sha1_match']:raise ValueError('Stored hash mismatch')
                blocks,b=frames(raw,decoder);r['block_codecs']=sorted({x['codec'] for x in blocks})
                if b is None:raise ValueError('Undecoded CAS codec')
                if len(b)!=a['size']:raise ValueError('Decoded size mismatch')
            elif raw[:8]==b'chunkzip':b,_=chunkzip(raw)
            else:b=raw
            r.update(decoded_sha256=hashlib.sha256(b).hexdigest(),decoded_size=len(b),header=b[:32].hex())
            if b[:4]==b'RIFF':r['fixup']=fixup(b)
            elif b[:4]==b'GNF ':r['gnf']=gnf_info(b)
            elif b[:8] in (b'chunklzx',b'chunkzip'):r['remaining_wrapper']=b[:8].decode()
            r['status']='parsed_fixup' if 'fixup'in r else 'decoded_header_only' if 'span'in a or raw[:8]==b'chunkzip' else 'raw_header_only'
            # Short string evidence only; not an inferred field schema.
            r['path_strings']=[s.decode('ascii') for s in re.findall(rb'[a-zA-Z0-9_./\\:-]{12,}',b) if b'/' in s or b'\\' in s][:20]
        except Exception as e:r['error']=str(e)
        rows.append(r)
    city_rows=[];sampled_video=False
    ffprobe=shutil.which('ffprobe')
    for n,a in sorted(city.items()):
        r={k:a[k] for k in ('name','kind','physical','size')}
        r['location']={k:a[k] for k in ('archive','physical_offset','span','toc','bundle') if k in a}
        try:
            if n.endswith('.json'):
                raw=read(a);b=frames(raw,decoder)[1] if 'span'in a else chunkzip(raw)[0] if raw[:8]==b'chunkzip' else raw
                if b is None:raise ValueError('Undecoded metadata')
                r['decoded_sha256']=hashlib.sha256(b).hexdigest()
                text=re.sub(r'/\*.*?\*/','',b.decode('utf-8-sig'),flags=re.S)
                r['frame_metadata']=json.loads(text)
            elif n.endswith(('.mkv','.webm')) and not sampled_video and 'scenic_' in n:
                raw=read(a,64*1024*1024);b=frames(raw,decoder)[1] if 'span'in a else chunkzip(raw)[0] if raw[:8]==b'chunkzip' else raw
                if b is None:raise ValueError('Undecoded movie')
                r['decoded_sha256']=hashlib.sha256(b).hexdigest();r['header']=b[:32].hex()
                if ffprobe:
                    p=subprocess.run([ffprobe,'-v','error','-show_streams','-show_format','-of','json','-i','pipe:0'],input=b,capture_output=True,timeout=45)
                    r['ffprobe']=json.loads(p.stdout);r['ffprobe_returncode']=p.returncode
                    if p.stderr:r['ffprobe_diagnostics']=p.stderr.decode(errors='replace')[:1000]
                sampled_video=True
        except Exception as e:r['error']=str(e)
        city_rows.append(r)
    text_hits=[]
    for n,a in list(sorted(scripts.items()))[:100]:
        try:
            raw=read(a);b=frames(raw,decoder)[1] if 'span'in a else chunkzip(raw)[0] if raw[:8]==b'chunkzip' else raw
            if b is None:continue
            lines=b.decode('utf8',errors='replace').splitlines()
            hits=[l.strip()[:250] for l in lines if re.search('cityintro|scenic_|CityOverlay|ShouldRender|pregame',l,re.I)]
            if hits:text_hits.append(dict(name=n,sha256=hashlib.sha256(b).hexdigest(),matches=hits[:15]))
        except Exception as e:text_hits.append(dict(name=n,error=str(e)))
    write_json(folder/'payload-evidence.json',dict(selection='Up to six unique names per kind/domain; one scenic video stream per source. Samples are not exhaustive codec qualification.',samples=rows,city_intro=city_rows,presentation_text=text_hits))
    print(folder.name,'samples',len(rows),'errors',sum('error'in r for r in rows),'city records',len(city_rows),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--only');a=p.parse_args()
    try:decoder=Oodle()
    except OSError:decoder=None
    for folder in sorted((ROOT/'data').iterdir()):
        if not folder.is_dir() or not (folder/'summary.json').exists():continue
        if a.only and folder.name not in a.only.split(','):continue
        if not (folder/'assets.jsonl').exists() and not (folder/'assets.jsonl.gz').exists():continue
        probe(folder,decoder)
