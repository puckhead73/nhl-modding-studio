"""Read-only source census; writes research artifacts only beneath --output.

Run: python scripts/arena_library/census.py [--only nhl26,nhl27] [--output PATH]
Uses existing repository research readers, with their identities recorded in the report.
No source extraction, game mutation, effective-patch guess, or runtime claims.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
import struct
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
sys.path.insert(0, str(REPO / 'local/runs/overnight-research-2026-09-18/probes'))
sys.path.insert(0, str(REPO / 'scripts/nhl27'))
from xiso_list import find_base, read_dir
from tdb import Tdb
from schema_dump import load_meta
from bundle_inventory import bundles
from asset_manifest import manifest, spans, part
from compatibility import ps4_spans
from dbobject import parse as dbobject

RELATED = re.compile(r'stadium|arena|jumbotron|powerring|scoreboard|scoreclock|rink|icesurface|ice_|banner|crowd|lighting|lightmap|camera|presentation|pregame|pre_game|intro|movie|/nis[/.]|goallight|goalhorn|chant|entrance|stanchion', re.I)
PS4 = dict(zip(range(15, 25), ['CUSA00561','CUSA01909','CUSA04326','CUSA07569','CUSA11126','CUSA15439','CUSA18182','CUSA26280','CUSA31046','CUSA37933']))


def sha(path):
    with Path(path).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False, default=lambda x: x.hex() if isinstance(x, bytes) else str(x)), encoding='utf8')


def source_list():
    result = []
    iso_root = Path('X:/Games/Emulation/Roms/XBOX360/ISOs')
    ps3_root = Path('X:/Games/Emulation/Roms/Sony Playstation 3')
    for year in [*range(7,16), 'legacy']:
        title = 'nhllegacy' if year == 'legacy' else f'nhl{year:02}'
        display = 'Legacy' if year == 'legacy' else f'{year:02}'
        found = sorted(iso_root.glob(f'NHL {display}*.iso'))
        result.append(dict(id=f'{title}-xbox360', title=title, year=2016 if year=='legacy' else 2000+year, platform='xbox360', engine='gen7', root=str(found[0]) if len(found)==1 else None))
        if year != 7:
            folder = ps3_root / ('NHL Legacy - Vanilla' if year=='legacy' else f'NHL {year:02}')
            roots = sorted([*folder.glob('PS3_GAME/USRDIR'),*folder.glob('*/PS3_GAME/USRDIR')]) if folder.exists() else []
            result.append(dict(id=f'{title}-ps3', title=title, year=2016 if year=='legacy' else 2000+year, platform='ps3', engine='gen7', root=str(roots[0]) if len(roots)==1 else None))
    result.append(dict(id='legacyrecomp-recomp', title='legacyrecomp', year=None, source_game_year=2016, platform='recomp', engine='gen7', root='F:/Games/NHL Legacy/game'))
    for year,cusa in PS4.items():
        root = f'H:/Emulators/ShadPS4/Games/{cusa}' if year!=24 else 'F:/Games/NHL 24/game'
        result.append(dict(id=f'nhl{year}-ps4',title=f'nhl{year}',year=2000+year,platform='ps4',engine='ignite' if year<=21 else 'frostbite',root=root, update_root='F:/Games/NHL 24/game-UPDATE' if year==24 else None))
    for year,root in [(25,None),(26,'S:/NHL26/PPSA26785-app0'),(27,'W:/Downloads/NHL 27/PPSA34063-app0')]:
        result.append(dict(id=f'nhl{year}-ps5',title=f'nhl{year}',year=2000+year,platform='ps5',engine='frostbite',root=root))
    return result


def arena_token(name):
    n=name.lower().replace('\\','/')
    m=re.search(r'(?:rendering/stadiums|content/worlds/stadiums)/([^/]+)/', n)
    if m:return m[1]
    m=re.search(r'rendering/stadium/(stadium_\d+|ice_[^/]+?)(?:_container|\.)', n)
    return m[1] if m else None


def category(name):
    n=name.lower()
    for label,pat in [('movies',r'movie|\.vp6$|\.webm$|\.bik$'),('camera',r'camera'),('lighting',r'lightmap|/lighting/|/lm_|_lm\.'),('screens',r'powerring|jumbotron|animatedsign|scoreboard|scoreclock'),('audio',r'goalhorn|chant|/sound/|/audio/'),('presentation',r'presentation|pregame|intro|/nis[/.]'),('crowd',r'crowd|chair|stands'),('materials',r'material'),('textures',r'\.gnf$|texture|_texgr$'),('geometry',r'\.mcd$|mesh|stadium_\d+_container')]:
        if re.search(pat,n):return label
    return 'other_related'


def eb_entries(f, base, size):
    """Bounded EB directory read, also works inside an ISO without staging the archive."""
    def read(off,n):
        if off<0 or n<0 or off+n>size:raise ValueError('EB directory outside archive')
        f.seek(base+off);b=f.read(n)
        if len(b)!=n:raise ValueError('Short EB read')
        return b
    h=read(0,48)
    if h[:4] in (b'BIG4',b'BIGF'):
        count,index_size=struct.unpack_from('>II',h,8)
        if count>2000000 or index_size<16 or index_size>64*1024*1024:raise ValueError('Invalid BIG directory size')
        index=read(16,min(size-16,index_size+65536));p=0
        for i in range(count):
            off,length=struct.unpack_from('>II',index,p);p+=8;end=index.find(b'\0',p)
            if end<0 or end-p>8192:raise ValueError('Invalid BIG member name')
            name=index[p:end].decode('latin1').replace('\\','/');p=end+1
            if off+length>size:raise ValueError('BIG member outside archive')
            yield dict(name=name,offset=off,size=length,archive_version=h[:4].decode())
        return
    if h[:4]!=b'EB\0\3':raise ValueError('Not EB archive')
    count,version,no=struct.unpack_from('>III',h,4)
    if version not in (0x400,0x100400,0x110400,0x200400):raise ValueError(f'Unsupported EB version {version:x}')
    ns,ds=h[20:22];nd=struct.unpack_from('>H',h,22)[0]
    stride=20 if version==0x110400 else 16
    if count>2000000 or ns<3:raise ValueError('Invalid EB directory count/slot')
    table=read(48,count*stride);names=read(no,count*ns)
    dirs=read((no+count*ns+15)&~15,nd*ds)
    values=[dirs[i*ds:(i+1)*ds].split(b'\0')[0].decode('latin1') for i in range(nd)]
    for i in range(count):
        block,compressed,length=struct.unpack_from('>III',table,i*stride)
        declared=length
        if stride==16 and compressed:length=compressed
        rec=names[i*ns:(i+1)*ns];di=struct.unpack_from('>H',rec)[0]
        if di>=len(values):raise ValueError('Invalid EB directory index')
        name=values[di]+'/'+rec[2:].split(b'\0')[0].decode('latin1')
        off=block<<4
        if off+length>size:raise ValueError('EB member outside archive')
        yield dict(name=name,offset=off,size=length,declared_length_word=declared,compressed_length_word=compressed,archive_version=f'{version:08x}')


def database(source):
    plat='xbox360' if source['platform']=='recomp' else source['platform']
    title='nhllegacy' if source['platform']=='recomp' else source['title']
    folder=REPO/'anchors'/plat/title/'databases'
    candidates=[folder/'nhl_game.db', folder/'nhlng.db', folder/'NHL07NG.db']
    p=next((p for p in candidates if p.exists()),None)
    if not p:return dict(status='no_TDB_anchor',arenas=[],teams=[])
    meta_path=p.with_name(p.stem+'-meta.xml')
    db=Tdb(p.read_bytes());meta=load_meta(meta_path);out={};schemas={}
    for name in ['exhibitionarena','exhibitionteams','customarena','stockarenatable','customarenatable']:
        short=next((k for k,v in meta.items() if v['name']==name),None)
        t=next((t for t in db.tables if t.raw.decode('latin1')==short),None)
        if not t:continue
        schemas[name]=dict(table_shortname=short,fields=meta[short]['fields'])
        names={f.id:meta[short]['fields'].get(f.raw.decode('latin1'),{}).get('name',f.id) for f in t.fields}
        rows=[{names[k]:v for k,v in r.items()} for r in t.rows()]
        if name=='exhibitionteams':
            rows=[{k:v for k,v in r.items() if re.search('arena|artid|index|name|city|powerring|scoreboard|intro|goal|chant|center|stanchion|banner|mascot',str(k),re.I)} for r in rows]
        out[name]=rows
    selectors=out.pop('exhibitionarena',[])
    resolved=[]
    for row in selectors:
        if 'arenaname' not in row and 'arenaid' in row:
            table={0:'stockarenatable',1:'customarenatable'}.get(row.get('table'))
            target=next((r for r in out.get(table,[]) if r.get('index')==row['arenaid']),None)
            if target:resolved.append(dict(target,selector=row,selector_source_table=table));continue
        resolved.append(row)
    return dict(status='anchor_parsed_not_live_database',path=str(p.relative_to(REPO)),sha256=sha(p),meta_sha256=sha(meta_path),field_schemas=schemas,arenas=resolved,teams=out.pop('exhibitionteams',[]),**out)


def legacy_scan(source, emit, report):
    root=Path(source['root']);physical=[]
    if root.is_file():
        with root.open('rb') as f:
            base,layout=find_base(f);f.seek(base+0x10000+20);sector,size=struct.unpack('<II',f.read(8));entries=[]
            read_dir(f,base,sector,size,'',entries)
        report['iso_layout']=layout
        physical=[dict(name=e['path'],path=root,offset=base+e['sector']*2048,size=e['size']) for e in entries if not e['attr']&16]
    else:
        scanroot=root/'databigs' if source['engine']=='ignite' else root
        # Skip previously extracted duplicates; archive members remain the authoritative listing.
        # Installed Gen7 trees include hundreds of thousands of prior extraction products.
        # Census the shipped archive layer, not those derived loose duplicates.
        paths=list(scanroot.glob('*'))
        if source['engine']!='ignite':
            for sub in ('gamedata','gamedata/anim','audio/speech','movie','movies'):
                paths.extend((scanroot/sub).glob('*'))
        physical=[]
        for p in paths:
            if not p.is_file():continue
            name=p.relative_to(scanroot).as_posix()
            if name.endswith('.nhlms-extracted.json') or not (RELATED.search(name) or p.suffix.lower() in ('.big','.viv')):continue
            physical.append(dict(name=name,path=p,offset=0,size=p.stat().st_size))
        report['loose_tree_policy']='Archive layer plus direct movie/gamedata/speech containers; prior extracted rendering trees excluded.'
    report['physical_files']=len(physical);report['archives']=[]
    signatures=set()
    for item in physical:
        name=item['name'];p=item['path'];base=item['offset'];size=item['size']
        if name.lower().endswith(('.big','.viv')):
            with p.open('rb') as f:
                f.seek(base);magic=f.read(8)
                if magic[:4] not in (b'EB\0\3',b'BIG4',b'BIGF'):
                    if RELATED.search(name):emit(dict(name=name,kind='opaque_container',physical=str(p),offset=base,size=size,magic=magic.hex()))
                    report.setdefault('other_archives',[]).append(dict(name=name,magic=magic.hex(),size=size))
                    continue
                try:
                    entries=list(eb_entries(f,base,size))
                    # Hash the directory description, not gigabytes of untouched payload.
                    index_hash=hashlib.sha256(json.dumps(entries,sort_keys=True).encode()).hexdigest()
                    report['archives'].append(dict(name=name,physical=str(p),offset=base,size=size,members=len(entries),index_sha256=index_hash))
                    for e in entries:
                        if not (RELATED.search(e['name']) or RELATED.search(name)):continue
                        signature_key=(name,Path(e['name']).suffix,category(e['name']))
                        magic=None
                        if signature_key not in signatures:
                            f.seek(base+e['offset']);magic=f.read(min(8,e['size'])).hex();signatures.add(signature_key)
                        emit(dict(e,kind='archive_member',archive=name,physical=str(p),physical_offset=base+e['offset'],magic=magic))
                except (ValueError,struct.error,IndexError) as e:report['errors'].append(dict(container=name,error=str(e)))
        elif RELATED.search(name):
            signature_key=('loose',Path(name).suffix,category(name));magic=None
            if signature_key not in signatures:
                with p.open('rb') as f:f.seek(base);magic=f.read(min(8,size)).hex()
                signatures.add(signature_key)
            emit(dict(name=name,kind='iso_file' if root.is_file() else 'loose_file',physical=str(p),offset=base,size=size,magic=magic))


def frost_scan(source, emit, report):
    ps5=source['platform']=='ps5';platform='Ps5' if ps5 else 'Gen4b';root=Path(source['root'])
    roots=[root]+([Path(source['update_root'])] if source.get('update_root') else [])
    mapping={}
    if ps5:
        for r in roots:
            for layer in ['Data','Patch']:
                p=r/layer/'layout.toc'
                if p.exists():
                    try:
                        for c in dbobject(p).get('installManifest',{}).get('installChunks',[]):mapping[c['persistentIndex']&0xffffffff]=c['name']
                    except (ValueError,KeyError) as e:report['errors'].append(dict(path=str(p),error=str(e)))
    locations={}
    def location(s):
        if s['layer'] not in (0,1):raise ValueError('Unknown CAS layer')
        key=(s['layer'],s.get('persistent_index',s.get('catalog')),s['cas'])
        if key in locations:
            p,length=locations[key]
            if s['offset']+s['size']>length:raise ValueError('CAS span outside file')
            return p
        layer='Patch' if s['layer'] else 'Data'
        rr=roots[-1] if s['layer'] else root
        folder=mapping[s['persistent_index']] if ps5 else f"superbundlelayout/nhl_installpackage_{s['catalog']:02d}"
        p=rr/layer/platform/folder/f"cas_{s['cas']:02d}.cas"
        if not p.resolve().is_relative_to(rr.resolve()):raise ValueError('CAS path escapes root')
        length=p.stat().st_size;locations[key]=(p,length)
        if s['offset']+s['size']>length:raise ValueError('CAS span outside file')
        return p
    report['tocs']=[];report['manifest_bundles']=0
    for rr in roots:
        for layer in ['Data','Patch']:
            for toc in sorted((rr/layer/platform).rglob('*.toc')):
                rel=toc.relative_to(rr).as_posix()
                try:rows,info=bundles(toc)
                except (ValueError,struct.error) as e:
                    report['errors'].append(dict(path=str(toc),error=str(e)));continue
                report['tocs'].append(dict(path=str(toc),**info))
                data=toc.read_bytes();data=data[0x22c:] if data[:4]==bytes.fromhex('00d1ce01') else data
                for row in rows:
                    # All world/movie bundles plus related and core registration/config bundles.
                    whole=toc.stem in ('worldssb','movies')
                    if not (whole or RELATED.search(row['name']) or toc.stem in ('globals','globalsfull','game')):continue
                    try:
                        if row['record_flag']!=1:raise ValueError('Unsupported bundle record')
                        ss=(spans if ps5 else ps4_spans)(part(data,row['record_offset'],row['record_size']))
                        mp=location(ss[0])
                        with mp.open('rb') as f:f.seek(ss[0]['offset']);mb=f.read(ss[0]['size'])
                        assets=manifest(mb)
                        if len(assets)+1!=len(ss):raise ValueError('Manifest/span mismatch')
                        report['manifest_bundles']+=1
                        manifest_hash=hashlib.sha256(mb).hexdigest()
                        for a,s in zip(assets,ss[1:]):
                            if not (whole or RELATED.search(row['name']) or RELATED.search(a['name'])):continue
                            p=location(s)
                            emit(dict(a,span=s,physical=str(p),toc=rel,toc_root=str(rr),toc_sha256=info['sha256'],bundle=row['name'],manifest_sha256=manifest_hash,evidence='manifest_parsed_payload_not_decoded'))
                    except (OSError,ValueError,KeyError,struct.error) as e:report['errors'].append(dict(toc=rel,bundle=row['name'],error=str(e)))
                print(source['id'],rel,'manifests',report['manifest_bundles'],flush=True)


def run(source, output):
    folder=output/'data'/source['id'];folder.mkdir(parents=True,exist_ok=True)
    report=dict(source=source,errors=[],asset_occurrences=0,groups=0,category_counts={},status='scanned')
    if not source['root'] or not Path(source['root']).exists():
        report['status']='source_unavailable';write_json(folder/'summary.json',report);return report
    try:db=database(source)
    except Exception as e:db=dict(status='parse_failed',error=str(e),arenas=[],teams=[])
    write_json(folder/'database.json',db)
    groups=collections.defaultdict(lambda:dict(names=set(),counts=collections.Counter(),occurrences=0))
    cats=collections.Counter();movies={}
    with (folder/'assets.jsonl').open('w',encoding='utf8') as out:
        def emit(a):
            a['category']=category(a['name']);a['venue_token']=arena_token(a['name']) or arena_token(a.get('bundle',''))
            if a['category']=='other_related' and a.get('archive'):a['category']=category(a['archive'])
            a['source_id']=source['id']
            out.write(json.dumps(a,ensure_ascii=False)+'\n');report['asset_occurrences']+=1;cats[a['category']]+=1
            if a['venue_token']:
                g=groups[a['venue_token']];g['names'].add(a['name']);g['counts'][a['category']]+=1;g['occurrences']+=1
            if a['category']=='movies':movies.setdefault(a['name'],a)
        if source['engine']=='frostbite':frost_scan(source,emit,report)
        else:legacy_scan(source,emit,report)
    write_json(folder/'movies.json',list(movies.values()))
    levels=json.loads((REPO/'defs/arena/ps4-venue-levels.json').read_text(encoding='utf8'))['titles'].get(source['title'],{}).get('levels',[]) if source['engine']=='ignite' else []
    variants=[];mapped=set()
    for token,g in sorted(groups.items()):
        level_ids=[l['id'] for l in levels if l.get('dir')==token]
        art=int(token[8:]) if re.fullmatch(r'stadium_\d+',token) else None
        hits=[r for r in db.get('arenas',[]) if (art is not None and r.get('arenaartid')==art) or (level_ids and r.get('arenaartid') in level_ids)]
        for r in hits:mapped.add(db['arenas'].index(r))
        # A suffix number is deliberately NOT a database join for Frostbite.
        v=dict(source_id=source['id'],game_year=source['year'],token=token,classification='shared_environment' if token in ('common','common_ice_int') or token.startswith('ice_') else 'venue_asset_group',unique_names=len(g['names']),occurrences=g['occurrences'],categories=dict(g['counts']),database_rows=hits,join_evidence='historical_vault_level_id' if level_ids else ('candidate_arenaartid_filename' if source['title']=='nhl08' else 'arenaartid_filename') if hits else 'unresolved',runtime_verified=False)
        variants.append(v)
        safe=re.sub(r'[^a-zA-Z0-9_.-]','_',token)
        (folder/f'{safe}.files.txt').write_text('\n'.join(sorted(g['names']))+'\n',encoding='utf8')
    write_json(folder/'venues.json',variants)
    report.update(groups=len(variants),category_counts=dict(cats),database_status=db['status'],database_arena_rows=len(db.get('arenas',[])),mapped_database_rows=len(mapped),unmapped_database_row_ordinals=[i for i in range(len(db.get('arenas',[]))) if i not in mapped],movie_names=len(movies))
    report['status']='scanned_with_errors' if report['errors'] else 'scanned'
    write_json(folder/'summary.json',report)
    print(source['id'],'DONE',report['asset_occurrences'],'assets',len(variants),'groups',len(report['errors']),'errors',flush=True)
    return report


def render(output):
    summaries=[];all_variants=[]
    for s in source_list():
        p=output/'data'/s['id']/'summary.json'
        if p.exists():
            r=json.loads(p.read_text(encoding='utf8'));summaries.append(r)
            v=p.with_name('venues.json')
            if v.exists():all_variants.extend(json.loads(v.read_text(encoding='utf8')))
    write_json(output/'data/catalog.json',dict(schema='arena-research-catalog/1',sources=summaries,variants=all_variants))
    lines=['# Arena reference catalog','', 'Generated from source directory and manifest reads. Game year means the title year, not a claimed release date. Legacy is the NHL 16-era source (2016); Recompiled retains that provenance without inventing a release year.', '', '**These are venue asset groups, not a verified count of unique physical arenas.** Database aliases, shared environments, spare assets, and historical versions are retained. Frostbite directory suffixes are not treated as database keys. Every group links its complete observed logical filename list; each source assets.jsonl carries physical archive/CAS locations and occurrences. Payload decoding and runtime qualification are separate.', '', '| Source | Game year | Status | Asset occurrences | Venue/shared groups | Arena DB rows | Mapped rows | Errors |','|---|---:|---|---:|---:|---:|---:|---:|']
    for r in summaries:
        s=r['source'];sid=s['id'];lines.append(f"| [{sid}](data/{sid}/summary.json) | {s['year'] or 'Legacy-derived'} | {r['status']} | {r.get('asset_occurrences',0)} | {r.get('groups',0)} | {r.get('database_arena_rows',0)} | {r.get('mapped_database_rows',0)} | {len(r['errors'])} |")
    for r in summaries:
        sid=r['source']['id'];asset_file='assets.jsonl.gz' if (output/'data'/sid/'assets.jsonl.gz').exists() else 'assets.jsonl'
        lines+=['',f'## {sid}', '',f"Source: `{r['source']['root']}`",'',f'[Full asset locations](data/{sid}/{asset_file}) · [Database fields and rows](data/{sid}/database.json) · [Movie candidates](data/{sid}/movies.json)' if r['status']!='source_unavailable' else 'Source unavailable; no arena names or filenames fabricated.','', '| Venue token / source variant | Database arena names | Join evidence | Names | Complete filename list |','|---|---|---|---:|---|']
        for v in (v for v in all_variants if v['source_id']==sid):
            names='; '.join(sorted({str(x.get('arenaname','unnamed')) for x in v['database_rows']})).replace('|','/')
            safe=re.sub(r'[^a-zA-Z0-9_.-]','_',v['token'])
            lines.append(f"| `{v['token']}` | {names or 'Unresolved / no joined row'} | {v['join_evidence']} | {v['unique_names']} | [{safe}.files.txt](data/{sid}/{safe}.files.txt) |")
        db_path=output/'data'/sid/'database.json'
        if db_path.exists() and r.get('unmapped_database_row_ordinals'):
            db=json.loads(db_path.read_text(encoding='utf8'))
            lines+=['','### Database entries without a qualified venue-file join','','These records are retained even when the asset group cannot be assigned safely. They are not additional confirmed unique meshes.','','| Row | Arena name | City | Art / selector |','|---:|---|---|---|']
            for ordinal in r['unmapped_database_row_ordinals']:
                row=db['arenas'][ordinal];name=str(row.get('arenaname','Unnamed')).replace('|','/');city=str(row.get('cityname','')).replace('|','/')
                lines.append(f"| {ordinal} | {name} | {city} | `{row.get('arenaartid',row.get('selector',{}).get('arenaid','unresolved'))}` |")
    (output/'arena-reference-catalog.md').write_text('\n'.join(lines)+'\n',encoding='utf8')
    inputs=[*sorted(Path(__file__).parent.glob('*.py')),REPO/'scripts/nhl27/bundle_inventory.py',REPO/'scripts/nhl27/asset_manifest.py',REPO/'scripts/nhl27/compatibility.py',REPO/'scripts/nhl27/riff_evidence.py',REPO/'local/runs/overnight-research-2026-09-18/probes/xiso_list.py',REPO/'local/runs/overnight-research-2026-09-18/probes/tdb.py',REPO/'local/runs/overnight-research-2026-09-18/probes/schema_dump.py',REPO/'local/runs/overnight-research-2026-09-18/probes/ps4_leaguelogos.py',REPO/'defs/arena/ps4-venue-levels.json']
    write_json(output/'data/tool-provenance.json',[dict(path=str(p.relative_to(REPO)),sha256=sha(p)) for p in inputs])


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--only');p.add_argument('--render-only',action='store_true');p.add_argument('--output',type=Path,default=REPO/'docs/research/arena-library');a=p.parse_args()
    a.output.mkdir(parents=True,exist_ok=True)
    if not a.render_only:
        for s in source_list():
            if not a.only or s['title'] in a.only.split(',') or s['id'] in a.only.split(','):run(s,a.output)
    render(a.output)
