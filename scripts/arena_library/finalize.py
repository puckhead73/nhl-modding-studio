"""Compress generated indexes, verify catalog consistency and snapshot foundation identity.

Only generated plain indexes beneath the repository research folder are removed,
after an independently read gzip stream matches their SHA-256 and byte count.
"""
import collections
from datetime import datetime, timezone
import gzip
import hashlib
import json
import re
from pathlib import Path
from census import REPO, render, sha, write_json

ROOT=REPO/'docs/research/arena-library'


def stream_digest(f):
    h=hashlib.sha256();length=0
    while b:=f.read(1024*1024):h.update(b);length+=len(b)
    return h.hexdigest(),length


def run():
    results=[]
    for folder in sorted((ROOT/'data').iterdir()):
        if not folder.is_dir():continue
        summary_path=folder/'summary.json'
        if not summary_path.exists():raise ValueError(f'Incomplete census: {folder.name}')
        summary=json.loads(summary_path.read_text(encoding='utf8'))
        if summary['status']=='source_unavailable':continue
        plain=folder/'assets.jsonl';packed=folder/'assets.jsonl.gz'
        if plain.exists():
            with plain.open('rb') as f:expected=stream_digest(f)
            with plain.open('rb') as src, packed.open('wb') as raw, gzip.GzipFile(filename='',mode='wb',fileobj=raw,mtime=0,compresslevel=6) as dst:
                while b:=src.read(1024*1024):dst.write(b)
            with gzip.open(packed,'rb') as f:actual=stream_digest(f)
            if expected!=actual:raise ValueError('Compressed index differs')
            # This exact file is our generated output, confined to ROOT.
            if not plain.resolve().is_relative_to(ROOT.resolve()):raise ValueError('Output escaped research folder')
            plain.unlink()
        count=0;groups=collections.defaultdict(set);types=collections.Counter();seen=set();movie_count=0
        with gzip.open(packed,'rt',encoding='utf8') as f:
            for line in f:
                a=json.loads(line);count+=1
                if a['source_id']!=folder.name:raise ValueError('Cross-source asset contamination')
                if a['venue_token']:groups[a['venue_token']].add(a['name'])
                if 'resource_type'in a:types[a['resource_type']]+=1
                seen.add((a['kind'],a['name'],a.get('sha1')))
                if a['category']=='movies':movie_count+=1
        if count!=summary['asset_occurrences']:raise ValueError(f'Occurrence mismatch: {folder.name}')
        venues=json.loads((folder/'venues.json').read_text(encoding='utf8'))
        for v in venues:
            token=v['token'];safe=re.sub(r'[^a-zA-Z0-9_.-]','_',token)
            names=set((folder/f'{safe}.files.txt').read_text(encoding='utf8').splitlines())
            if names!=groups[token] or len(names)!=v['unique_names']:raise ValueError(f'File list mismatch: {folder.name}/{token}')
            if v['runtime_verified']:raise ValueError('Research incorrectly claims runtime verification')
        result=dict(source_id=folder.name,occurrences=count,distinct_kind_name_stored_hash=len(seen),venue_groups=len(venues),compressed_index_sha256=sha(packed),compressed_bytes=packed.stat().st_size,resource_types=dict(types),errors=len(summary['errors']),catalog_consistent=True)
        results.append(result);print(folder.name,count,'verified',flush=True)
    render(ROOT)
    write_json(ROOT/'data/validation.json',dict(checked_utc=datetime.now(timezone.utc).isoformat(),sources=results,total_occurrences=sum(r['occurrences'] for r in results),total_venue_groups=sum(r['venue_groups'] for r in results),all_catalog_checks_passed=True))
    target=Path('F:/Games/NHL 24');paths=[target/'nhl24.exe',target/'game/eboot.bin',target/'game-UPDATE/eboot.bin',target/'nhl27-moves-only/installation.json',target/'nhl27-moves-only/package-receipt.json']
    paths+=sorted((target/'game-mods').rglob('*'))
    write_json(ROOT/'data/foundation-snapshot.json',dict(root=str(target),read_utc=datetime.now(timezone.utc).isoformat(),source_files_modified=False,files=[dict(path=p.relative_to(target).as_posix(),bytes=p.stat().st_size,sha256=sha(p)) for p in paths if p.is_file()],scope='Identity snapshot only; not a fresh installation or gameplay verification.'))


if __name__=='__main__':run()
