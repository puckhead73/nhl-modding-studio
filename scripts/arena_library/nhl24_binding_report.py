"""Generate live-TDB joins and explicit Boston EBX import evidence. No installs.

Requires nhl24_collectors.py and the source census. Name-based scene matching is
labeled as a candidate; missing imports block a complete scene clone.
"""
import collections
import hashlib
import json
import re
import sys
from pathlib import Path
from nhl24_collectors import REPO
from probe import assets, read, Oodle, frames
from riff_evidence import fixup

sys.path.insert(0, str(REPO / 'local/runs/overnight-research-2026-09-18/probes'))
from tdb import Tdb
from schema_dump import load_meta


def sha(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def run():
    root = REPO / 'local/arena-library/nhl24-resolved'
    folder = root / 'files/gamemodes/database/t3db'
    db = Tdb((folder / 'nhl_game.db').read_bytes())
    meta = load_meta(folder / 'nhl_game-meta.xml')
    tables = {}
    schemas = {}
    for t in db.tables:
        m = meta[t.id]
        if m['name'] not in ('exhibitionarena', 'exhibitionteams', 'customarena', 'leaguearena'):
            continue
        rows = []
        for i in range(t.current_records):
            row = {}
            for f in t.fields:
                fm = m['fields'][f.id]
                v = t.value(i, f)
                # Meta range_low is an encoding bias, not two's-complement.
                if fm['type'] == 'FIELDTYPE_INTEGER':
                    v += int(fm.get('lo', 0))
                elif f.kind == 0:
                    v = t.record_bytes(i)[f.bit_offset//8:(f.bit_offset+f.bit_width)//8].split(b'\0')[0].decode('cp1252')
                row[fm['name']] = v
            rows.append(row)
        tables[m['name']] = rows
        schemas[m['name']] = dict(table_id=t.id, current=t.current_records, capacity=t.max_records,
            fields=[dict(id=f.id, bit_offset=f.bit_offset, bit_width=f.bit_width, **m['fields'][f.id]) for f in t.fields])
    arenas = {a['index']: a for a in tables['exhibitionarena']}
    if len(arenas) != len(tables['exhibitionarena']):
        raise ValueError('Duplicate arena primary keys')
    joins = []
    sharers = collections.defaultdict(list)
    for team in tables['exhibitionteams']:
        arena = arenas.get(team['arenaid'])
        joins.append(dict(team_id=team['index'], team_name=team['teamname'], arena_id=team['arenaid'],
                          arena_art_id=arena['arenaartid'] if arena else None,
                          status='database_join' if arena else 'unresolved_arena_key'))
        sharers[team['arenaid']].append(team['index'])
    candidates = {}
    for a in assets(REPO / 'docs/research/arena-library/data/nhl24-ps4'):
        if a['kind'] == 'ebx' and ('/stadiums/boston_2/' in a['name'] or '/structures/boston/' in a['name']):
            candidates.setdefault((a['name'], a['sha1']), a)
    decoder = Oodle()
    nodes = []
    for a in candidates.values():
        n = {k:a[k] for k in ('name', 'sha1', 'size', 'physical', 'span', 'toc', 'bundle')}
        try:
            raw = read(a)
            if hashlib.sha1(raw).hexdigest() != a['sha1']:
                raise ValueError('Stored SHA1 mismatch')
            b = frames(raw, decoder)[1]
            if b is None or len(b) != a['size']:
                raise ValueError('Decoded size mismatch')
            n['fixup'] = fixup(b)
            n['decoded_sha256'] = hashlib.sha256(b).hexdigest()
        except (ValueError, KeyError, OSError) as e:
            n['error'] = str(e)
        nodes.append(n)
    partitions = collections.defaultdict(list)
    for n in nodes:
        if 'fixup' in n:
            partitions[n['fixup']['partition']].append(dict(name=n['name'], sha1=n['sha1']))
    edges = [dict(source_name=n['name'], source_sha1=n['sha1'], **ref,
                  targets=partitions.get(ref['partition'], []),
                  status='partition_candidates_instance_unverified' if ref['partition'] in partitions else 'outside_scanned_scene')
             for n in nodes if 'fixup' in n for ref in n['fixup']['imports']]
    baseline = json.loads((REPO / 'docs/research/arena-library/data/foundation-snapshot.json').read_text())
    foundation = [dict(path=x['path'], unchanged=sha(Path(baseline['root'])/x['path']) == x['sha256']) for x in baseline['files']]
    report = dict(database_sha256=sha(folder/'nhl_game.db'), metadata_sha256=sha(folder/'nhl_game-meta.xml'),
                  schemas=schemas, tables=tables, team_arena_bindings=joins, reverse_sharers=dict(sharers),
                  boston_scene_nodes=nodes, boston_import_edges=edges, foundation_checks=foundation,
                  complete_dependency_closure=False, scene_registration_resolved=False,
                  limitations=['arenaartid-to-scene routing requires runtime/schema evidence',
                               'import targets outside Boston subset are unresolved',
                               'resource/chunk references and imported instance identities require typed decoding',
                               'base/patch content variants retained; this graph does not select effective scene assets'])
    (root/'arena-bindings.json').write_text(json.dumps(report, indent=2), encoding='utf8')
    summary = dict(database_sha256=report['database_sha256'], tables={k:len(v) for k,v in tables.items()},
        joins=len(joins), unresolved_joins=sum(x['status']!='database_join' for x in joins),
        boston_ebx_variants=len(nodes), decode_errors=sum('error' in n for n in nodes),
        import_edges=len(edges), external_edges=sum(x['status']=='outside_scanned_scene' for x in edges),
        foundation_unchanged=all(x['unchanged'] for x in foundation),
        complete_dependency_closure=False, scene_registration_resolved=False)
    (root/'summary.json').write_text(json.dumps(summary, indent=2), encoding='utf8')
    print(json.dumps(summary))


if __name__ == '__main__':
    run()
