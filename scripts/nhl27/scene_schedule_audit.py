"""Compare supplied source scene scheduling with the original native ANT bank.

This records asset-level ordering only; it does not infer executable callback
equivalence or authorize installing a reordered matrix.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

from deke_gesture_audit import Source, selected
from deke_native_audit import native_id
from native_ant import Bank

MATRIX = '00f58745-6672-fd34-00f5-87456672fd34'
TARGET_HASH = '6e5c2b215b8f064ec6982a6fdf141eb577d483ea7678e66ab702f0cb39f94d14'


def native_matrix(bank):
    root = bank.by_id[native_id(MATRIX)]
    if (root['type_hash'], root['size']) != (1085191674, 48):
        raise ValueError('Unqualified native scene matrix layout')
    by_offset = {o['offset']: ident for ident, o in bank.by_id.items()}
    external = {site: ident for _, site, ident, _, _ in bank.external}

    def reference(site):
        if site in bank.reloc:
            return by_offset[bank.pointer(site)]
        return external[site]

    def array(site, stride):
        count = struct.unpack_from('<Q', bank.blob, site + 8)[0]
        if count > 1024:
            raise ValueError('Unexpected scene matrix array length')
        if not count:
            return []
        start = bank.pointer(site)
        if start + count * stride > len(bank.blob):
            raise ValueError('Scene matrix array outside bank')
        return [start + i * stride for i in range(count)]

    columns = [reference(p) for p in array(root['offset'] + 32, 8)]
    rows = []
    # Serializer 0x8f8ce0: Items are 24-byte records, cells are byte Booleans.
    for index, item in enumerate(array(root['offset'] + 8, 24)):
        ident = reference(item)
        cells = [bank.blob[p] for p in array(item + 8, 1)]
        if len(cells) != len(columns) or any(c not in (0, 1) for c in cells):
            raise ValueError('Invalid scene matrix Boolean cells')
        obj = bank.by_id[ident]
        rows.append(dict(index=index, id=ident, name=obj['name'],
                         type_hash=obj['type_hash'], columns=[i for i, c in enumerate(cells) if c]))
    return dict(columns=columns, items=rows)


def run(root, output):
    source = Source(root)
    # Apply the supplied install's patch precedence, while retaining rejection
    # of conflicting records in the same layer in Source.partition().
    layers = {}
    for node in source.catalog:
        partition = node.get('fixup', {}).get('partition')
        layers[partition] = max(layers.get(partition, -1), node['asset']['span']['layer'])
    source.catalog = [n for n in source.catalog if n['asset']['span']['layer'] ==
                      layers[n.get('fixup', {}).get('partition')]]
    document = source.partition(MATRIX)
    fields = selected(document)['fields']
    rows = []
    for index, item in enumerate(fields['Items']):
        op = selected(source.follow(item['SceneOp']))
        rows.append(dict(index=index, id=native_id(item['SceneOp']['external']['instance']),
                         name=op['fields']['Name'], type=op['type'],
                         columns=[i for i, c in enumerate(item['Cells']) if c]))
    columns = [native_id(r['external']['instance']) for r in fields['EntityAssets']]
    data = (root / 'ant/nhl24_160/launch.bin').read_bytes()
    if hashlib.sha256(data).hexdigest() != TARGET_HASH:
        raise ValueError('Original native launch bank hash mismatch')
    target = native_matrix(Bank(data))
    if columns != target['columns']:
        raise ValueError('Scene columns have different identities; index comparison is invalid')
    source_ids, target_ids = {r['id'] for r in rows}, {r['id'] for r in target['items']}
    by_target = {r['id']: r for r in target['items']}
    report = dict(target_sha256=TARGET_HASH, installed=False, callback_parity=False,
        layer_policy='Highest supplied catalog span.layer per partition; same-layer disagreement rejected',
        source=dict(columns=columns, items=rows), target=target,
        source_only=[r for r in rows if r['id'] not in target_ids],
        target_only=[r for r in target['items'] if r['id'] not in source_ids],
        changed_cells=[dict(id=r['id'], name=r['name'], source=r['columns'],
                           target=by_target[r['id']]['columns']) for r in rows
                       if r['id'] in by_target and r['columns'] != by_target[r['id']]['columns']],
        source_evidence=source.evidence)
    output.write_text(json.dumps(report, indent=2))
    print(json.dumps({k: len(report[k]) for k in ('source_only', 'target_only', 'changed_cells')}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('local/nhl27-eboot/compatibility'))
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    run(args.root, args.output or args.root / 'deke-trial/scene-schedule-audit.json')
