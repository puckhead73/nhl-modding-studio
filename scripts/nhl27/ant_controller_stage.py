"""Emit supported controller partitions with preserved exported identities.

This stage never installs incomplete graphs. Its report lists uncompiled types
and unresolved references so the full bank linker can require closure.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import uuid

from animation_fields import Fields
from ant_tags import compile_tree, LAYOUTS
from deke_native_audit import native_id
from native_ant import Bank


def exported_id_map(reader):
    result = {}
    for offset in reader.fixup['instance_offsets'][:reader.fixup['exported_instances']]:
        # Only exported instances have a GUID immediately before their record.
        guid = uuid.UUID(bytes_le=reader.data[32 + offset - 16:32 + offset])
        ident = native_id(str(guid))
        if not ident or ident in result.values():
            raise ValueError('Invalid/colliding exported GUID identity')
        result[offset] = ident
    if not result or next(iter(result.values())) != native_id(reader.fixup['partition']):
        raise ValueError('Root instance identity does not match source partition')
    return result


def run(root):
    trial = root / 'deke-trial'
    graph = json.loads((trial / 'integration-closure.json').read_text())
    types = json.loads((root / 'source-type-descriptors.json').read_text())
    banks = [Bank((root / 'ant/nhl24_160' / (name + '.bin')).read_bytes()) for name in ('state', 'launch')]
    occupied = set().union(*(b.by_id for b in banks))
    # Reserve every exported identity before allocating private internal IDs.
    readers = {}
    for node in graph['nodes']:
        path = trial / 'integration-fields' / (node['asset']['sha1'] + '.ebx')
        reader = Fields(path.read_bytes(), types)
        ids = exported_id_map(reader)
        occupied.update(ids.values())
        readers[node['asset']['sha1']] = (reader, ids)
    out = trial / 'controller-fragments'
    out.mkdir(exist_ok=True)
    categories = {}
    source_objects = {}
    for reader, ids in readers.values():
        for instance in reader.export()['instances']:
            if instance['offset'] in ids:
                source_objects[ids[instance['offset']]] = instance
            if instance['type'] == ['DebugCategoryAsset'] and instance['offset'] in ids:
                categories[ids[instance['offset']]] = instance['fields']['CategoryId']
    for bank in banks:
        for ident, obj in bank.by_id.items():
            if obj['type_hash'] == 0xe9329a24 and ident not in categories:
                import struct
                categories[ident] = struct.unpack_from('<I', bank.blob, obj['offset'] + 8)[0]
    rows = []
    for node in graph['nodes']:
        reader, ids = readers[node['asset']['sha1']]
        fields = reader.export()
        first = fields['instances'][0]
        if any(t in first['type'] for t in ('ClipControllerAsset', 'ChannelToDofAsset')):
            continue  # Their qualified compressed-resource writer is separate.
        row = dict(name=node['name'], partition=fields['partition'], source_sha1=node['asset']['sha1'])
        rows.append(row)
        try:
            for instance in fields['instances']:
                kind = instance['type'][0]
                f = instance['fields']
                if kind == 'PTMWeights':
                    ref = f['TweakAsset']
                    if ref is None:
                        verified = True
                    else:
                        target = (source_objects[native_id(ref['external']['instance'])] if 'external' in ref
                                  else next(i for i in fields['instances'] if i['offset'] == ref['internal_offset']))
                        tf = target['fields']
                        verified = (target['type'] == ['PTMWeightsTweakAsset']
                            and set(tf) == {'Name', 'FeatureComponentWeightsToolOverride', 'KeepCurrentAnimBiasOverride'}
                            and tf['FeatureComponentWeightsToolOverride'] == []
                            and tf['KeepCurrentAnimBiasOverride'] == -1.0)
                    f['_native_tweak_defaults_verified'] = verified
                if kind in ('ContextDatabase', 'PriorityDatabase'):
                    names = ['EntriesV'] + (['PrioritiesV'] if kind == 'PriorityDatabase' else [])
                    f['_native_raw_arrays'] = {n: reader.raw_array(instance['offset'], n) for n in names}
                if kind in ('ContextDatabaseAsset', 'PriorityDatabaseAsset'):
                    ref = f['DebugCategory']
                    if ref is None:
                        category = 0
                    elif 'external' in ref:
                        category = categories[native_id(ref['external']['instance'])]
                    else:
                        target = next(i for i in fields['instances'] if i['offset'] == ref['internal_offset'])
                        if target['type'] != ['DebugCategoryAsset']:
                            raise ValueError('Unexpected inline debug category type')
                        category = target['fields']['CategoryId']
                    f['_native_debug_category_id'] = category
            result = compile_tree(fields, dict(internal_offset=first['offset']), occupied, ids)
            payload = result.pop('data')
            path = out / (node['asset']['sha1'] + '.bin')
            path.write_bytes(payload)
            result.update(path=str(path), sha256=hashlib.sha256(payload).hexdigest())
            row.update(status='serialized_unregistered', fragment=result)
        except (ValueError, KeyError) as error:
            row.update(status='unimplemented', error=str(error))
    report = dict(status='controller_graph_incomplete_not_installed',
        counts=dict(Counter(r['status'] for r in rows)),
        blockers=dict(Counter(r['error'] for r in rows if 'error' in r)), partitions=rows)
    (out / 'report.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(dict(counts=report['counts'], blockers=report['blockers']), indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    run(parser.parse_args().root)
