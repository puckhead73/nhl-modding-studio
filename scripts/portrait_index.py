"""Portable portrait index builder and read-only lookup CLI (Python stdlib only).

Build: python portrait_index.py build --root PATH --output PATH/portraits.sqlite --reference-root REPO/tmp
Query: python portrait_index.py query --database PATH --mode exact --last Zherdev --first Nikolai --dob 1984-11-05
Modes: exact, surname-dob, surname, dob, search. Search accepts --text and optional --dob.
"""
import argparse
import csv
import datetime as dt
import hashlib
import json
import re
import sqlite3
import time
import unicodedata
from collections import Counter, defaultdict
from pathlib import Path

SCHEMA = '''
PRAGMA foreign_keys=ON;
PRAGMA user_version=1;
CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE portraits(
 id INTEGER PRIMARY KEY, relative_path TEXT NOT NULL UNIQUE, player_slug TEXT NOT NULL,
 recorded_name TEXT NOT NULL, last_name TEXT, first_name TEXT, last_key TEXT, first_key TEXT,
 dob TEXT, recorded_dob TEXT, portrait_number INTEGER, identity_status TEXT NOT NULL,
 identity_evidence TEXT, source_label TEXT, source_type TEXT, games TEXT, mods_json TEXT,
 file_bytes INTEGER NOT NULL, file_mtime_ns INTEGER NOT NULL, manifest_sha256 TEXT,
 width INTEGER,height INTEGER, suggested_shard TEXT);
CREATE TABLE provenance(portrait_id INTEGER PRIMARY KEY REFERENCES portraits(id), manifest_row_json TEXT NOT NULL);
CREATE TABLE aliases(portrait_id INTEGER REFERENCES portraits(id),alias TEXT NOT NULL, PRIMARY KEY(portrait_id,alias));
CREATE TABLE issues(id INTEGER PRIMARY KEY,portrait_id INTEGER REFERENCES portraits(id),relative_path TEXT,kind TEXT NOT NULL,detail TEXT NOT NULL);
CREATE INDEX portrait_exact ON portraits(last_key,first_key,dob);
CREATE INDEX portrait_surname_dob ON portraits(last_key,dob);
CREATE INDEX portrait_dob ON portraits(dob);
CREATE INDEX portrait_slug ON portraits(player_slug);
CREATE VIRTUAL TABLE portrait_search USING fts5(recorded_name,aliases,slug,dob, tokenized_names,
 tokenize='unicode61 remove_diacritics 2',prefix='2 3 4');
'''

def key(value):
    value=value.casefold().translate(str.maketrans({'ø':'o','ł':'l','ß':'ss','æ':'ae','œ':'oe','đ':'d','ð':'d','þ':'th'}))
    return ''.join(c for c in unicodedata.normalize('NFKD',value) if c.isalnum())

def words(value):
    value=value.casefold().translate(str.maketrans({'ø':'o','ł':'l','ß':'ss','æ':'ae','œ':'oe','đ':'d','ð':'d','þ':'th'}))
    value=''.join(c for c in unicodedata.normalize('NFKD',value) if not unicodedata.combining(c))
    return '_'.join(re.findall(r'[^\W_]+',value))

def references(root):
    result=defaultdict(set)
    def add(last,first,birth,origin):
        slug=f'{words(last)}_{words(first)}_{birth:%d_%m_%Y}'
        result[slug].add((words(last).replace('_',' '),words(first).replace('_',' '),origin))
    if not root:return result
    for path in sorted((root/'portrait-export').glob('db*/players.json')):
        for row in json.loads(path.read_text(encoding='utf-8')):
            if row.get('table')=='customplayerbiotable' or not row.get('firstname') or not row.get('lastname'):continue
            try:birth=dt.date(row['year']+1900,row['month']+1,row['day']+1)
            except (ValueError,KeyError,TypeError):continue
            add(row['lastname'],row['firstname'],birth,f'game database: {path.name} ({path.parent.name})')
    for kind in ['skater','goalie']:
        path=root/'slug_restore_work'/f'nhl_{kind}_bios.json'
        if not path.exists():continue
        for row in json.loads(path.read_text(encoding='utf-8'))['response']['data']:
            last=row.get('lastName',''); full=row.get(kind+'FullName','')
            if not last or not full.endswith(last):continue
            first=full[:-len(last)].strip()
            if not first:continue
            try:birth=dt.date.fromisoformat(row['birthDate'][:10])
            except (ValueError,KeyError,TypeError):continue
            add(last,first,birth,f'cached NHL biography playerId={row["playerId"]}')
    return result

def parse_identity(slug,refs):
    match=re.fullmatch(r'(.+)_(\d{1,2})_(\d{1,2})_(\d{4})',slug)
    if not match:return {'name':slug.replace('_',' '),'last':None,'first':None,'dob':None,'raw_dob':'','status':'unparsed_identity','evidence':''}
    name,day,month,year=match.groups()
    try:birth=dt.date(int(year),int(month),int(day)).isoformat()
    except ValueError:birth=None
    pieces=name.split('_')
    candidates=refs.get(slug,set())
    boundaries={(a,b) for a,b,_ in candidates}
    if len(boundaries)==1:
        last,first=next(iter(boundaries)); status='structured_reference'; evidence='; '.join(sorted({c for _,_,c in candidates}))
    elif len(pieces)==2 and all(pieces) and not any(p.isdigit() for p in pieces):
        last,first=pieces;status='unambiguous_slug';evidence='two name components in recorded slug'
    else:
        last=first=None;status='ambiguous_name_boundary';evidence='Name boundary is not recoverable unambiguously from underscores'
    if birth is None:status='invalid_dob'
    return {'name':name.replace('_',' '),'last':last,'first':first,'dob':birth,'raw_dob':f'{day}_{month}_{year}','status':status,'evidence':evidence}

def number(value):
    try:return int(value)
    except (ValueError,TypeError):return None

def sha(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def build(root,output,reference_root=None):
    started=time.perf_counter();root=root.resolve();output=output.resolve()
    # Relative image paths are rooted at the index directory for portable installation.
    if output.parent!=root:raise ValueError('Place the index directly in the portrait library root')
    if output.exists():raise FileExistsError(f'Refusing to overwrite existing index: {output}')
    csv.field_size_limit(64*1024*1024)
    manifest=root/'portrait_sources.csv'; manifest_hash=sha(manifest)
    with manifest.open(encoding='utf-8-sig',newline='') as f:
        rows=list(csv.DictReader(f))
    byfile={r['portrait_file'].replace('\\','/'):r for r in rows}
    if len(rows)!=len(byfile):raise ValueError('Duplicate manifest paths')
    image_folder=root/'portraits' if (root/'portraits').is_dir() else root
    files={p.relative_to(root).as_posix():p for p in image_folder.rglob('*') if p.is_file() and '_TOOL' not in p.relative_to(root).parts and p.suffix.lower() in {'.png','.webp'}}
    refs=references(reference_root)
    print(f'Indexing {len(files):,} images; {len(refs):,} structured identity references.',flush=True)
    staging=output.with_suffix('.building.sqlite')
    if staging.exists():raise FileExistsError(staging)
    con=sqlite3.connect(staging);con.executescript(SCHEMA)
    counts=Counter()
    try:
        for i,(relative,path) in enumerate(sorted(files.items()),1):
            row=byfile.get(relative,{})
            stem=path.stem
            filematch=re.fullmatch(r'(.+_\d{1,2}_\d{1,2}_\d{4})(?:_(\d+))?',stem)
            # Preserve user filename identity over stale inherited provenance identity.
            slug=filematch[1] if filematch else row.get('player_slug') or stem
            identity=parse_identity(slug,refs)
            # Imports store explicit name boundaries, including compound names.
            if row.get('import_first_name') and row.get('import_last_name') and row.get('import_dob'):
                entered_date=dt.date.fromisoformat(row['import_dob'])
                entered_slug=f"{words(row['import_last_name'])}_{words(row['import_first_name'])}_{entered_date:%d_%m_%Y}"
                if entered_slug==slug:
                    identity.update(last=row['import_last_name'],first=row['import_first_name'],dob=entered_date.isoformat(),status='user_supplied_fields',evidence='Explicit importer name and DOB fields')
            issues=[]
            if row.get('player_slug') and row['player_slug']!=slug:issues.append(('filename_manifest_identity_difference',row['player_slug']))
            if not row:issues.append(('missing_manifest_entry','No provenance row found'))
            if identity['status'] not in ['structured_reference','unambiguous_slug','user_supplied_fields']:issues.append((identity['status'],identity['evidence'] or slug))
            # Malformed filename with a numeric name component must not inherit a valid-looking DOB identity.
            if any(t.isdigit() for t in identity['name'].split()):
                identity.update(last=None,first=None,dob=None,status='malformed_identity')
                issues.append(('malformed_identity','Unexpected numeric component in recorded name'))
            counts[identity['status']]+=1
            stat=path.stat();last=identity['last'];first=identity['first']
            values=(i,relative,slug,identity['name'],last,first,key(last) if last else None,key(first) if first else None,
                    identity['dob'],identity['raw_dob'],number(filematch[2]) if filematch else number(row.get('portrait_number')),
                    identity['status'],identity['evidence'],row.get('source_label',''),row.get('source_type',''),row.get('games',''),row.get('mods_json',''),
                    stat.st_size,stat.st_mtime_ns,row.get('representative_sha256',''),number(row.get('width')),number(row.get('height')),
                    key(last)[:2] if last else None)
            con.execute('INSERT INTO portraits VALUES('+','.join('?' for _ in values)+')',values)
            con.execute('INSERT INTO provenance VALUES(?,?)',(i,json.dumps(row,ensure_ascii=False)))
            aliases=set(filter(None,row.get('player_aliases','').split(';')))|set(filter(None,row.get('deduplication_identity_variants','').split(';')))
            aliases.discard(slug)
            con.executemany('INSERT INTO aliases VALUES(?,?)',[(i,a) for a in sorted(aliases)])
            text=' '.join([identity['name'],*(a.replace('_',' ') for a in sorted(aliases))])
            con.execute('INSERT INTO portrait_search(rowid,recorded_name,aliases,slug,dob,tokenized_names) VALUES(?,?,?,?,?,?)',
                        (i,identity['name'],' '.join(sorted(aliases)),slug,identity['dob'] or '',text+' '+key(identity['name'])))
            con.executemany('INSERT INTO issues(portrait_id,relative_path,kind,detail) VALUES(?,?,?,?)',[(i,relative,k,d) for k,d in issues])
            if i%20000==0:print(f'Indexed {i:,}/{len(files):,}',flush=True)
        for missing in sorted(set(byfile)-set(files)):
            con.execute('INSERT INTO issues(relative_path,kind,detail) VALUES(?,?,?)',(missing,'missing_image',json.dumps(byfile[missing],ensure_ascii=False)))
        meta={'schema_version':'1','library_version':dt.datetime.now(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ'),
              'created_utc':dt.datetime.now(dt.timezone.utc).isoformat(),'manifest_sha256':manifest_hash,'image_root':'.',
              'image_folder':'portraits' if image_folder!=root else '.',
              'normalization':'NFKD + casefold + explicit Latin transliteration; remove non-alphanumeric characters',
              'exact_match':'primary recorded identity only; aliases used only by search',
              'search':'FTS5 token-prefix AND search; optional ISO DOB filter',
              'image_hashes':'Manifest-supplied hashes retained, not recomputed by index build'}
        con.executemany('INSERT INTO metadata VALUES(?,?)',meta.items())
        con.execute("INSERT INTO portrait_search(portrait_search) VALUES('optimize')")
        con.commit();con.execute('ANALYZE');con.commit()
        assert con.execute('PRAGMA integrity_check').fetchone()[0]=='ok'
        assert not con.execute('PRAGMA foreign_key_check').fetchall()
        assert con.execute('SELECT count(*) FROM portraits').fetchone()[0]==len(files)
        assert con.execute('SELECT count(*) FROM portrait_search').fetchone()[0]==len(files)
        assert sha(manifest)==manifest_hash,'Manifest changed during build'
        issue_counts=dict(con.execute('SELECT kind,count(*) FROM issues GROUP BY kind'))
        con.close();staging.replace(output)
        report={'portraits':len(files),'manifest_rows':len(rows),'identity_status_counts':dict(counts),'issues':issue_counts,
                'sqlite_bytes':output.stat().st_size,'elapsed_seconds':time.perf_counter()-started,'integrity_check':'ok','foreign_key_check':'ok','manifest_sha256':manifest_hash}
        output.with_suffix('.build.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        print(json.dumps(report,indent=2),flush=True)
        return report
    finally:
        con.close()

def query(con,mode,last='',first='',dob=None,text='',limit=100,offset=0):
    if dob:dt.date.fromisoformat(dob)
    if mode in ['exact','surname-dob','surname'] and not key(last):raise ValueError('Last name is required')
    if mode=='exact' and not key(first):raise ValueError('First name is required')
    if mode in ['exact','surname-dob','dob'] and not dob:raise ValueError('ISO DOB is required')
    conditions=[];params=[];join=''
    if mode in ['exact','surname-dob','surname']:conditions.append('p.last_key=?');params.append(key(last))
    if mode=='exact':conditions.append('p.first_key=?');params.append(key(first))
    if dob:conditions.append('p.dob=?');params.append(dob)
    if mode=='search':
        tokens=re.findall(r'[^\W_]+',words(text))
        if not tokens:raise ValueError('Search text must contain a letter or number')
        # Quote tokens: user input is never interpreted as raw FTS operators.
        expression=' AND '.join('"'+t.replace('"','""')+'"*' for t in tokens)
        join=' JOIN portrait_search ON portrait_search.rowid=p.id'
        conditions.append('portrait_search MATCH ?');params.append(expression)
    if mode not in ['exact','surname-dob','surname','dob','search']:raise ValueError(mode)
    sql='SELECT p.* FROM portraits p'+join+' WHERE '+' AND '.join(conditions)+' ORDER BY p.last_key,p.first_key,p.dob,p.portrait_number,p.relative_path LIMIT ? OFFSET ?'
    return sql,params+[min(max(int(limit),1),1000),max(int(offset),0)]

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    sub=parser.add_subparsers(dest='command',required=True)
    b=sub.add_parser('build');b.add_argument('--root',type=Path,required=True);b.add_argument('--output',type=Path,required=True);b.add_argument('--reference-root',type=Path)
    q=sub.add_parser('query');q.add_argument('--database',type=Path,required=True);q.add_argument('--mode',choices=['exact','surname-dob','surname','dob','search'],default='exact')
    for name in ['last','first','dob','text']:q.add_argument('--'+name,default=None if name=='dob' else '')
    q.add_argument('--limit',type=int,default=100);q.add_argument('--offset',type=int,default=0)
    args=parser.parse_args()
    if args.command=='build':build(args.root,args.output,args.reference_root)
    else:
        connection=sqlite3.connect(args.database.resolve().as_uri()+'?mode=ro',uri=True);connection.row_factory=sqlite3.Row
        sql,params=query(connection,args.mode,args.last,args.first,args.dob,args.text,args.limit,args.offset)
        print(json.dumps([dict(r) for r in connection.execute(sql,params)],ensure_ascii=False,indent=2))
