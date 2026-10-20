"""Move a flat indexed portrait library into two-letter folders, with resumable audit.

Only current relative paths change. Filenames, identity records, image bytes and
historical provenance remain intact. Backups live outside the distributed library.
"""
import argparse,csv,hashlib,json,re,shutil,sqlite3,time
from collections import Counter
from pathlib import Path

def sha(path):
    with path.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def run(root,report):
    root=root.resolve(); report=report.resolve()
    assert root not in report.parents and root!=report
    csv.field_size_limit(64*1024*1024)
    planfile=report/'move_plan.json'
    manifest=root/'portrait_sources.csv';database=root/'portraits.sqlite'
    if (report/'completion.json').exists():raise RuntimeError('Already completed')
    if not planfile.exists():
        report.mkdir(parents=True,exist_ok=False)
        con=sqlite3.connect(database); con.row_factory=sqlite3.Row
        indexed=list(con.execute('SELECT id,relative_path,player_slug,suggested_shard,file_bytes,file_mtime_ns FROM portraits'))
        con.close()
        actual={p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() and p.suffix.lower() in {'.png','.webp'}}
        assert actual=={r['relative_path'] for r in indexed},'Index/file mismatch'
        moves=[]
        for r in indexed:
            old=r['relative_path']; assert '/' not in old,'Expected flat input'
            prefix=(r['suggested_shard'] or re.sub('[^a-z0-9]','',r['player_slug'].split('_')[0].lower())[:2])
            assert re.fullmatch('[a-z0-9]{1,2}',prefix),old
            new=f'{prefix}/{Path(old).name}'
            assert not (root/new).exists(),new
            stat=(root/old).stat()
            assert stat.st_size==r['file_bytes'] and stat.st_mtime_ns==r['file_mtime_ns'],f'Changed file: {old}'
            moves.append({'id':r['id'],'old':old,'new':new,'bytes':stat.st_size,'mtime_ns':stat.st_mtime_ns})
        assert len({m['new'].casefold() for m in moves})==len(moves)
        for name in ['portrait_sources.csv','portraits.sqlite','portraits.build.json','portraits.validation.json','portrait_index_README.txt']:
            if (root/name).exists():shutil.copy2(root/name,report/name)
        plan={'root':str(root),'manifest_sha256':sha(manifest),'database_sha256':sha(database),'moves':moves}
        planfile.write_text(json.dumps(plan),encoding='utf-8')
    plan=json.loads(planfile.read_text(encoding='utf-8'))
    assert plan['root']==str(root)
    assert sha(manifest)==plan['manifest_sha256'],'Manifest changed; inspect before resuming'
    assert sha(database)==plan['database_sha256'],'Index changed; inspect before resuming'
    moves=plan['moves'];mapping={m['old']:m['new'] for m in moves}
    # Validate every resolved source and target is inside the intended library.
    for m in moves:
        assert root in (root/m['old']).resolve().parents
        assert root in (root/m['new']).resolve().parents
    for directory in sorted({str(Path(m['new']).parent) for m in moves}):(root/directory).mkdir(exist_ok=True)
    start=time.perf_counter()
    for i,m in enumerate(moves,1):
        source=root/m['old'];dest=root/m['new']
        if source.exists():
            assert not dest.exists(),m
            stat=source.stat()
            assert (stat.st_size,stat.st_mtime_ns)==(m['bytes'],m['mtime_ns'])
            source.rename(dest) # Same-volume rename; no image decode/re-encode.
        stat=dest.stat()
        assert not source.exists() and (stat.st_size,stat.st_mtime_ns)==(m['bytes'],m['mtime_ns'])
        if i%10000==0:print(f'Moved and verified {i:,}/{len(moves):,}',flush=True)
    staged_manifest=report/'portrait_sources.updated.csv'
    with manifest.open(encoding='utf-8-sig',newline='') as fi,staged_manifest.open('w',encoding='utf-8-sig',newline='') as fo:
        reader=csv.DictReader(fi);writer=csv.DictWriter(fo,fieldnames=reader.fieldnames);writer.writeheader()
        for row in reader:
            row['portrait_file']=mapping.get(row['portrait_file'],row['portrait_file'])
            writer.writerow(row)
    staged_db=report/'portraits.updated.sqlite'
    shutil.copy2(database,staged_db)
    con=sqlite3.connect(staged_db)
    with con:
        for m in moves:
            con.execute('UPDATE portraits SET relative_path=?,suggested_shard=? WHERE id=? AND relative_path=?',(m['new'],m['new'].split('/')[0],m['id'],m['old']))
            original=json.loads(con.execute('SELECT manifest_row_json FROM provenance WHERE portrait_id=?',(m['id'],)).fetchone()[0])
            original['portrait_file']=m['new']
            con.execute('UPDATE provenance SET manifest_row_json=? WHERE portrait_id=?',(json.dumps(original,ensure_ascii=False),m['id']))
            con.execute('UPDATE issues SET relative_path=? WHERE portrait_id=?',(m['new'],m['id']))
        # User explicitly confirmed this portrait was removed due to its background.
        con.execute("UPDATE issues SET kind='intentional_exclusion',detail=? WHERE kind='missing_image' AND relative_path=?",('User removed this portrait because it had a background; do not restore. Original row retained in backed-up manifest.','adamcik_marian_09_03_2001_01.png'))
        con.execute("UPDATE metadata SET value=? WHERE key='manifest_sha256'",(sha(staged_manifest),))
        con.execute('INSERT OR REPLACE INTO metadata VALUES(?,?)',('storage_layout','two-letter surname folders; ambiguous names use first two recorded slug letters'))
    assert con.execute('PRAGMA integrity_check').fetchone()[0]=='ok'
    assert not con.execute('PRAGMA foreign_key_check').fetchall()
    paths={r[0] for r in con.execute('SELECT relative_path FROM portraits')}
    actual={p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() and p.suffix.lower() in {'.png','.webp'}}
    assert paths==actual==set(mapping.values())
    assert con.execute('SELECT count(*) FROM portrait_search').fetchone()[0]==len(moves)
    counts=dict(con.execute('SELECT kind,count(*) FROM issues GROUP BY kind'))
    con.close()
    # Publish prepared files after verifying the entire move set.
    shutil.copy2(staged_manifest,root/'portrait_sources.sharding.csv')
    shutil.copy2(staged_db,root/'portraits.sharding.sqlite')
    (root/'portrait_sources.sharding.csv').replace(manifest)
    (root/'portraits.sharding.sqlite').replace(database)
    for filename in ['portraits.build.json','portraits.validation.json']:
        p=root/filename
        if p.exists():
            data=json.loads(p.read_text(encoding='utf-8'));data.update(storage_layout='two-letter surname folders',issues=counts,manifest_sha256=sha(manifest),sharded_paths_verified=len(moves))
            p.write_text(json.dumps(data,indent=2),encoding='utf-8')
    readme=root/'portrait_index_README.txt'
    if readme.exists():
        text=readme.read_text(encoding='utf-8')
        text=text.replace('The current folder structure and all filenames are unchanged. The index supports\nsubfolders when rebuilt, so optional two-letter sharding can be performed later.','Images are stored in two-letter surname folders, e.g. zh/zherdev_...png.\nFilenames are unchanged; ambiguous names use the first two recorded slug letters.')
        readme.write_text(text,encoding='utf-8')
    summary={'moved':len(moves),'folders':len({m['new'].split('/')[0] for m in moves}),'folder_counts':dict(sorted(Counter(m['new'].split('/')[0] for m in moves).items())),'all_paths_verified':True,'image_bytes_unchanged':'same-volume renames; sizes and mtimes verified','index_integrity':'ok','issues':counts,'seconds':time.perf_counter()-start}
    (report/'completion.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
    print(json.dumps({k:v for k,v in summary.items() if k!='folder_counts'},indent=2),flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',type=Path,required=True);p.add_argument('--report',type=Path,required=True)
    a=p.parse_args();run(a.root,a.report)
