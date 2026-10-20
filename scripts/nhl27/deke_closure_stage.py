"""Stage all DCT clips reachable through the new-move integration roots.

No filename filter or implicit layer precedence: conflicting partition/resource
variants fail closed. Native-boundary correspondence is not semantic equivalence.
This produces inputs for deke_new_moves.py, never an activated game package.
"""
import argparse
from collections import defaultdict, deque
import hashlib
import json
from pathlib import Path

from animation_dct import parse
from animation_fields import Fields
from compatibility import Install, Oodle, frames
from deke_native_audit import native_id
from native_ant import Bank


def closure(catalog, roots, native_ids):
    by_partition = defaultdict(dict)
    for node in catalog:
        by_partition[node['fixup']['partition']][node['asset']['sha1']] = node
    roots = set(roots)
    # A known partition root does not imply all its exported instances exist
    # in NHL24. NHL27 adds enumerators inside otherwise familiar partitions.
    queue = deque((part, part) for part in sorted(roots))
    seen, nodes, reused = set(), [], set()
    missing, ambiguous = [], []
    while queue:
        part, instance = queue.popleft()
        if part in seen:
            continue
        if part not in roots and native_id(instance) in native_ids:
            reused.add(part)
            continue
        seen.add(part)
        reused.discard(part)
        choices = by_partition.get(part, {})
        if not choices:
            missing.append(part)
            continue
        if len(choices) != 1:
            ambiguous.append(dict(partition=part, hashes=sorted(choices)))
            continue
        node = next(iter(choices.values()))
        nodes.append(node)
        queue.extend(sorted({(i['partition'], i['instance']) for i in node['fixup']['imports']}))
    return dict(roots=sorted(roots), nodes=nodes, reused_partitions=sorted(reused),
                missing=missing, ambiguous=ambiguous)


def unique_resource(resources, name, resource_id):
    choices = {r['sha1']: r for r in resources
               if r['name'] == name and int(r['resource_id'], 16) == resource_id}
    if len(choices) != 1:
        raise ValueError(f'Resource identity has {len(choices)} payload variants: {name}')
    return next(iter(choices.values()))


def run(root):
    trial = root / 'deke-trial'
    out = trial / 'closure-stage'
    out.mkdir(exist_ok=True)
    catalog = json.loads((trial / 'source-catalog.json').read_text())
    moves = json.loads((trial / 'new-moves/report.json').read_text())
    roots = {b['path'][-1]['partition'] for c in moves['clips']
             for b in c['integration_boundaries']['existing_ancestor_paths']}
    banks = [Bank((root / 'ant/nhl24_160' / (n + '.bin')).read_bytes())
             for n in ('state', 'launch')]
    graph = closure(catalog, roots, set().union(*(b.by_id for b in banks)))
    (out / 'closure.json').write_text(json.dumps(graph, indent=2))
    if graph['missing'] or graph['ambiguous']:
        raise ValueError('Incomplete/ambiguous import closure; inspect closure.json')
    field_directory = trial / 'integration-fields'
    field_directory.mkdir(exist_ok=True)
    types = json.loads((root / 'source-type-descriptors.json').read_text())
    resources = []
    for path in sorted(root.glob('nhl27-*.toc.json')):
        inv = json.loads(path.read_text())
        resources.extend(a for bundle in inv['bundles'] for a in bundle['assets']
                         if a['kind'] == 'res')
    install, decoder = Install('nhl27'), Oodle()

    def read(asset):
        raw = install.read(asset['span'])
        if hashlib.sha1(raw).hexdigest() != asset['sha1']:
            raise ValueError('Stored resource hash mismatch')
        data = frames(raw, decoder)[1]
        if data is None or len(data) != asset['size']:
            raise ValueError('Decoded resource size mismatch')
        return data

    rows, needed, exports = [], set(), {}
    for node in graph['nodes']:
        data = read(node['asset'])
        fields = Fields(data, types).export()
        field_path = field_directory / (node['asset']['sha1'] + '.ebx')
        field_path.write_bytes(data)
        field_path.with_suffix('.json').write_text(json.dumps(fields, indent=2))
        exports[node['fixup']['partition']] = (node, data, fields)
        animations = [i['fields'] for i in fields['instances'] if 'DctAnimationAsset' in i['type']]
        if not animations:
            continue
        if len(animations) != 1:
            raise ValueError('Expected exactly one DCT animation per clip partition')
        anim = animations[0]
        res = unique_resource(resources, node['name'], anim['Resource'])
        payload = read(res)
        meta, _ = parse(payload)
        if tuple(anim[k] for k in ('NumCompressedQuat', 'NumCompressedVec3', 'NumCompressedVecFloat')) != tuple(meta[k] for k in ('quaternions', 'vectors', 'float_vectors')):
            raise ValueError('Resource and EBX channel dimensions differ')
        prefix = out / res['sha1']
        prefix.with_suffix('.ebx').write_bytes(data)
        prefix.with_suffix('.resource').write_bytes(payload)
        needed.add(anim['ChannelToDofAsset']['external']['partition'])
        rows.append(dict(name=node['name'], partition=node['fixup']['partition'],
            resource_sha1=res['sha1'], ebx_sha1=node['asset']['sha1'],
            resource_sha256=hashlib.sha256(payload).hexdigest(), metadata=meta))
    bindings = []
    for part in sorted(needed):
        # A CTD may be behind a native identity boundary; decode its exact source
        # definition anyway because equal IDs do not establish equal channel order.
        variants = {n['asset']['sha1']: n for n in catalog if n['fixup']['partition'] == part}
        if len(variants) != 1:
            raise ValueError('Missing or conflicting CTD source definitions')
        node = next(iter(variants.values()))
        data = exports[part][1] if part in exports else read(node['asset'])
        path = out / (node['asset']['sha1'] + '-binding.ebx')
        path.write_bytes(data)
        bindings.append(dict(partition=part, payload=str(path), sha1=node['asset']['sha1']))
    report = dict(status='staged_not_registered', clips=rows, bindings=bindings,
        source_catalog_sha256=hashlib.sha256((trial / 'source-catalog.json').read_bytes()).hexdigest(),
        native_boundary_policy='Identity correspondence only; controller semantics require separate verification')
    (out / 'stage-report.json').write_text(json.dumps(report, indent=2))
    # Publish the controller graph only after every referenced source payload
    # has passed its stored/decoded size and hash checks and has been exported.
    (trial / 'integration-closure.json').write_text(json.dumps(graph, indent=2))
    print(json.dumps(dict(partitions=len(graph['nodes']), native_boundaries=len(graph['reused_partitions']),
                          clips=len(rows), channel_maps=len(bindings)), indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    run(parser.parse_args().root)
