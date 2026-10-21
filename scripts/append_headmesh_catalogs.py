"""Stage patch catalogs into an existing library, preserving all existing paths.

python scripts/append_headmesh_catalogs.py LIBRARY SOURCES.json NEW_LIBRARY
Immutable GLB/DDS files are hardlinked when possible; SQLite and metadata are copied.
The live library is never edited. Run verify_headmesh_library.py before publication.
"""
import argparse
import collections
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3

from headmesh_library import identity, load_glb, pack_subset, role, sha, year


def link_or_copy(source, destination):
    try:
        os.link(source, destination)
    except OSError:
        shutil.copy2(source, destination)


def append_library(library, sources, destination):
    library, destination = Path(library).resolve(), Path(destination).resolve()
    if destination.exists() or destination.is_relative_to(library):
        raise ValueError('Choose a new sibling staging directory')
    def stage_file(source, target):
        if Path(source).suffix in ('.glb', '.dds'):
            # Fail rather than silently duplicate the entire existing corpus.
            os.link(source, target)
        else:
            shutil.copy2(source, target)
    shutil.copytree(library, destination, copy_function=stage_file)
    # A staged library must not inherit a misleading successful verification receipt.
    (destination / 'verification.json').unlink(missing_ok=True)
    db = sqlite3.connect(destination / 'headmeshes.sqlite')
    db.row_factory = sqlite3.Row
    known = collections.defaultdict(set)
    for row in db.execute('SELECT DISTINCT identity_slug FROM appearances WHERE identity_slug IS NOT NULL'):
        slug = row[0]
        known['_'.join(slug.split('_')[:-3])].add(slug)
    geometry = {(r['class'], r['geometry_sha256']): r for r in db.execute('SELECT * FROM assets')}
    ledger = json.loads((destination / 'sources.json').read_text())
    existing_catalogs = {r['sha256'] for r in ledger}
    report = collections.Counter()
    for source in sorted(sources, key=lambda s: year(json.loads(Path(s['catalog']).read_text())['game']), reverse=True):
        path = Path(source['catalog']).resolve()
        catalog = json.loads(path.read_text(encoding='utf-8-sig'))
        digest = sha(path.read_bytes())
        if digest in existing_catalogs:
            continue
        if catalog.get('validate_only'):
            raise ValueError('Validation-only catalog has no assets')
        layer = catalog.get('layer', 'Data')
        texture_cache = {}
        print(f"Appending {catalog['game']} {layer}: {len(catalog['entries'])} entries", flush=True)
        for original in catalog['entries']:
            entry = dict(original, layer=layer, patch_version=source.get('patch_version'))
            slug = identity(entry)
            if not slug and '/licensed/' in entry['source']:
                parts = entry['source'].split('/licensed/')[1].split('/')[0].split('_')
                candidates = set()
                for split in range(1, len(parts)):
                    candidates.update(known['_'.join(parts[split:] + parts[:split])])
                if len(candidates) == 1:
                    slug = candidates.pop()
            textures = []
            for relative in entry.get('textures', []):
                if relative not in texture_cache:
                    native = (path.parent / relative).resolve()
                    if not native.is_relative_to(path.parent):
                        raise ValueError('Texture escapes catalog')
                    data = native.read_bytes()
                    if data[:4] != b'DDS ':
                        raise ValueError(f'Invalid DDS {native}')
                    h = sha(data)
                    target = Path('textures') / h[:2] / (h + '.dds')
                    output = destination / target
                    if not output.exists():
                        output.parent.mkdir(parents=True, exist_ok=True)
                        link_or_copy(native, output)
                        report['new_textures'] += 1
                    texture_cache[relative] = target.as_posix()
                textures.append({'name': Path(relative).name, 'file': texture_cache[relative]})
            native = (path.parent / entry['mesh_file']).resolve()
            if not native.is_relative_to(path.parent):
                raise ValueError('Mesh escapes catalog')
            doc, binary = load_glb(native)
            groups = collections.defaultdict(list)
            for index, mesh in enumerate(doc['meshes']):
                kind = role(mesh.get('name', ''))
                cls = entry['class'] if entry['class'] != 'head' else kind if kind in ('hair', 'facialhair') else 'head'
                groups[cls].append(index)
            for cls, indices in groups.items():
                packed, fingerprint, subset = pack_subset(doc, binary, indices)
                key = cls, fingerprint
                if key not in geometry:
                    stem = slug or re.sub(r'[^a-z0-9_]', '_', entry['name'].lower())
                    folder = cls if slug or entry['origin'] != 'player' else f'Unresolved/{cls}'
                    ordinal = 1
                    while True:
                        relative = f'{folder}/{stem[:2]}/{stem}_{ordinal:02d}.glb'
                        output = destination / relative
                        if not output.exists():
                            break
                        ordinal += 1
                    output.parent.mkdir(parents=True, exist_ok=True)
                    output.write_bytes(packed)
                    aid = sha(f'{cls}:{fingerprint}'.encode())
                    db.execute('INSERT INTO assets VALUES (?,?,?,?,?,?,?,?,?,?)',
                        (aid, cls, stem, relative, entry['game'], source['platform'], entry['origin'], entry.get('lod', 0),
                         json.dumps([m.get('name', '') for m in subset['meshes']]), fingerprint))
                    geometry[key] = {'id': aid, 'game': entry['game'], 'platform': source['platform']}
                    report['new_meshes'] += 1
                else:
                    report['deduplicated_meshes'] += 1
                aid = geometry[key]['id']
                preferred = geometry[key]
                if year(entry['game']) > year(preferred['game']) or (entry['game'] == preferred['game'] and source['platform'] == 'xbox360' and preferred['platform'] == 'ps3'):
                    db.execute('UPDATE assets SET game=?,platform=?,origin=?,lod=? WHERE id=?',
                               (entry['game'], source['platform'], entry['origin'], entry.get('lod', 0), aid))
                    geometry[key] = {'id': aid, 'game': entry['game'], 'platform': source['platform']}
                db.execute('INSERT INTO appearances(asset_id,identity_slug,game,platform,origin,source,source_catalog,entry_json,textures_json,lod) VALUES (?,?,?,?,?,?,?,?,?,?)',
                    (aid, slug, entry['game'], source['platform'], entry['origin'], entry['source'], str(path), json.dumps(entry), json.dumps(textures), entry.get('lod', 0)))
                report['added_appearances'] += 1
        ledger.append({'catalog': str(path), 'sha256': digest, 'platform': source['platform'], 'game': catalog['game'],
                       'layer': layer, 'patch_version': source.get('patch_version'), 'source_entries': len(catalog['entries'])})
        existing_catalogs.add(digest)
        db.commit()
    totals = {f'unique_{r[0]}': r[1] for r in db.execute('SELECT class,count(*) FROM assets GROUP BY class')}
    totals['source_appearances'] = db.execute('SELECT count(*) FROM appearances').fetchone()[0]
    totals['duplicate_meshes'] = totals['source_appearances'] - len(geometry)
    totals['unique_texture_files'] = len(list((destination / 'textures').rglob('*.dds')))
    unresolved = [dict(r) for r in db.execute("SELECT p.asset_id,a.stem AS name,p.source FROM appearances p JOIN assets a ON a.id=p.asset_id WHERE p.identity_slug IS NULL AND p.origin='player'")]
    totals['unresolved_appearances'] = len(unresolved)
    for filename, value in [('summary.json', totals), ('sources.json', ledger), ('unresolved.json', unresolved), ('patch-import.json', dict(report))]:
        (destination / filename).write_text(json.dumps(value, indent=2), encoding='utf8')
    db.execute('ANALYZE')
    db.close()
    print(json.dumps(dict(report), indent=2), flush=True)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('library', type=Path)
    p.add_argument('sources', type=Path)
    p.add_argument('destination', type=Path)
    a = p.parse_args()
    append_library(a.library, json.loads(a.sources.read_text()), a.destination)
