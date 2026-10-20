"""Generate the city-intro reference and cross-year namespace comparison from evidence."""
import json
import struct
from pathlib import Path
from census import REPO, write_json, sha, source_list

ROOT=REPO/'docs/research/arena-library'


def run():
    rows=[];summaries=[];sets={}
    for folder in sorted((ROOT/'data').iterdir()):
        if not folder.is_dir():continue
        p=folder/'payload-evidence.json'
        if p.exists():
            evidence=json.loads(p.read_text(encoding='utf8'));city=evidence['city_intro']
            rows.extend(dict(source_id=folder.name,**a) for a in city)
            summaries.append(dict(source_id=folder.name,city_path_records=len(city),video_filenames=sum(a['name'].endswith(('.mkv','.webm','.vp6','.bik')) for a in city),parsed_frame_sidecars=sum('frame_metadata'in a for a in city),video_stream_samples=sum('ffprobe'in a for a in city),payload_samples=len(evidence['samples']),payload_errors=sum('error'in a for a in evidence['samples']),city_errors=sum('error'in a for a in city)))
        p=folder/'venues.json'
        if p.exists():sets[folder.name]={a['token'] for a in json.loads(p.read_text(encoding='utf8'))}
    comparisons=[]
    for old,new in [('nhl24-ps4','nhl26-ps5'),('nhl26-ps5','nhl27-ps5')]:
        comparisons.append(dict(source=old,target=new,added_tokens=sorted(sets[new]-sets[old]),removed_tokens=sorted(sets[old]-sets[new]),shared_tokens=len(sets[new]&sets[old]),meaning='Namespace comparison only; same token does not mean identical geometry or presentation.'))
    write_json(ROOT/'data/presentation-summary.json',dict(sources=summaries,namespace_comparisons=comparisons))
    write_json(ROOT/'data/city-intro-records.json',rows)
    lines=['# City-intro file reference','','Exact cityintro-path names retained by the census/probe. Numeric scenic indices are **not yet verified city IDs**. Video metadata applies only to the listed sampled stream; logo and binding-table records are not videos. See [findings](findings.md) for the proposed NHL 24 sequence and audio policy.','','| Source | Cityintro-path records | Video filenames | Parsed frame sidecars | Stream samples |','|---|---:|---:|---:|---:|']
    for s in summaries:
        lines.append(f"| {s['source_id']} | {s['city_path_records']} | {s['video_filenames']} | {s['parsed_frame_sidecars']} | {s['video_stream_samples']} |")
    for source in sorted({r['source_id'] for r in rows}):
        lines+=['',f'## {source}','',f'[Physical locations and payload evidence](data/{source}/payload-evidence.json)','','| Exact logical filename | Bytes reported by source | Evidence |','|---|---:|---|']
        for r in (r for r in rows if r['source_id']==source):
            status='metadata parsed' if 'frame_metadata'in r else 'stream probed' if 'ffprobe'in r else 'indexed'
            if 'error'in r:status='unresolved: '+r['error']
            lines.append(f"| `{r['name']}` | {r['size']} | {status} |")
    (ROOT/'city-intro-reference.md').write_text('\n'.join(lines)+'\n',encoding='utf8')
    builds=[]
    for source in source_list():
        result=dict(source_id=source['id'],metadata=[])
        if source['root'] and Path(source['root']).is_dir():
            root=Path(source['root']);roots=[root]+([Path(source['update_root'])] if source.get('update_root') else [])
            for r in roots:
                for path in [r/'sce_sys/param.json',r/'sce_sys/param.sfo',r.parent/'PARAM.SFO']:
                    if not path.exists():continue
                    row=dict(path=str(path),sha256=sha(path))
                    try:
                        if path.suffix=='.json':
                            d=json.loads(path.read_text(encoding='utf8'))
                            row['fields']={k:v for k,v in d.items() if k in ('titleId','contentId','contentVersion','masterVersion','requiredSystemSoftwareVersion','applicationCategoryType')}
                        else:
                            b=path.read_bytes()
                            if b[:4]!=b'\0PSF':raise ValueError('Unsupported SFO')
                            keys,values,count=struct.unpack_from('<III',b,8);fields={}
                            if count>4096:raise ValueError('SFO count out of bounds')
                            for i in range(count):
                                ko,fmt,length,capacity,vo=struct.unpack_from('<HHIII',b,20+16*i)
                                start=keys+ko;end=b.find(b'\0',start)
                                if end<start or values+vo+length>len(b):raise ValueError('SFO field out of bounds')
                                key=b[start:end].decode('ascii')
                                if key in ('TITLE','TITLE_ID','APP_VER','VERSION','CONTENT_ID'):
                                    value=b[values+vo:values+vo+length].rstrip(b'\0')
                                    try:fields[key]=value.decode('utf8')
                                    except UnicodeDecodeError:
                                        fields[key]=value.decode('cp1252')
                                        row.setdefault('encoding_notes',{})[key]='cp1252 fallback; exact bytes retained'
                                        row.setdefault('field_bytes',{})[key]=value.hex()
                            row['fields']=fields
                    except (ValueError,struct.error) as e:row['error']=str(e)
                    result['metadata'].append(row)
        result['status']='metadata_observed' if result['metadata'] else 'build_version_unresolved_or_source_unavailable'
        builds.append(result)
    write_json(ROOT/'data/source-builds.json',builds)
    print('city-path records',len(rows),'sources with samples',len(summaries))


if __name__=='__main__':run()
