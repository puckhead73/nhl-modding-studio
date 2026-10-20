"""Materialize embedded clip tags and report exact remaining native dependencies."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from ant_tags import allocate_id, compile_tree
from ant_controller_stage import exported_id_map
from animation_fields import Fields
from native_ant import Bank


def run(root):
    folder = root / 'deke-trial/closure-stage/new-moves'
    source = json.loads((folder / 'report.json').read_text())
    out = folder / 'tagged-fragments'
    out.mkdir(exist_ok=True)
    banks = [Bank((root / 'ant/nhl24_160' / (n + '.bin')).read_bytes()) for n in ('state', 'launch')]
    existing = set().union(*(b.by_id for b in banks))
    occupied = existing | {int(c['native_fragment']['objects'][0]['id'], 16) for c in source['clips']}
    occupied.update(int(c['target_ctd_id'], 16) for c in source['clips'])
    # Tags and controllers share the same final registry. Reserve every source
    # export and every staged controller ID before allocating private tag IDs.
    trial = root / 'deke-trial'
    graph = json.loads((trial / 'integration-closure.json').read_text())
    types = json.loads((root / 'source-type-descriptors.json').read_text())
    exports = {}
    for node in graph['nodes']:
        reader = Fields((trial / 'integration-fields' / (node['asset']['sha1'] + '.ebx')).read_bytes(), types)
        ids = exported_id_map(reader)
        exports[reader.fixup['partition']] = ids
        occupied.update(ids.values())
    controllers_path = trial / 'controller-fragments/report.json'
    if controllers_path.exists():
        controllers = json.loads(controllers_path.read_text())
        occupied.update(obj['id'] for row in controllers['partitions'] if 'fragment' in row
                        for obj in row['fragment']['objects'])
    rows, definitions = [], set(existing)
    for clip in sorted(source['clips'], key=lambda c: c['partition']):
        manifest = clip['native_fragment']
        row = dict(name=clip['name'], partition=clip['partition'])
        rows.append(row)
        data = Path(manifest['path']).read_bytes()
        if hashlib.sha256(data).hexdigest() != manifest['sha256']:
            raise ValueError('Clip fragment hash mismatch')
        fragment = {k: manifest[k] for k in ('objects', 'internal_relocations', 'symbolic_relocations')}
        for obj in fragment['objects']:
            obj['id'] = int(obj['id'], 16) if obj['id'] else allocate_id(clip['partition'], 'animation', occupied)
        for ref in fragment['symbolic_relocations']:
            ref['target_id'] = int(ref['target_id'], 16)
        fragment['path'], fragment['sha256'] = manifest['path'], manifest['sha256']
        parts = [fragment]
        try:
            for dep in manifest['unresolved_dependencies']:
                if dep['field'] != 'TagCollectionSetAsset':
                    raise ValueError('Unsupported clip dependency')
                fields = json.loads((folder / (clip['source_sha1'] + '.fields.json')).read_text())
                tree = compile_tree(fields, dep['source_reference'], occupied, exports[fields['partition']])
                payload = tree.pop('data')
                path = out / (clip['source_sha1'] + '.tags.bin')
                path.write_bytes(payload)
                tree.update(path=str(path), sha256=hashlib.sha256(payload).hexdigest())
                fragment['symbolic_relocations'].append(dict(site=dep['site'], target_id=tree['root_id']))
                parts.append(tree)
            row.update(status='serialized_unregistered', fragments=parts)
            definitions.update(o['id'] for part in parts for o in part['objects'])
        except ValueError as error:
            row.update(status='rejected', error=str(error))
    # New channel maps have their own emitted fragments. They are definitions,
    # not proof that a bank already contains or has initialized them.
    definitions.update(int(c['target_ctd_id'], 16) for c in source['clips'] if 'new_binding' in c)
    missing = Counter()
    for row in rows:
        if row['status'] == 'rejected':
            continue
        refs = {r['target_id'] for f in row['fragments'] for r in f['symbolic_relocations']}
        row['unresolved_native_ids'] = [hex(i) for i in sorted(refs - definitions)]
        missing.update(row['unresolved_native_ids'])
    report = dict(status='not_installable_controller_integration_pending',
        counts=dict(Counter(r['status'] for r in rows)), unresolved_native_ids=dict(missing), clips=rows,
        qualification='Native tag record serialization; selection graphs and new state producers are not implemented')
    (out / 'report.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(dict(counts=report['counts'], unresolved_native_ids=dict(missing)), indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    run(parser.parse_args().root)
