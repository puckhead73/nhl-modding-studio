"""Convert unmatched DCT moves and trace their incoming controller dependencies.

Writes private, target-bound resources and evidence, never installed banks. Existing
CTD maps are selected by exact DOF/category/lane compatibility, not similar names.
The output is deliberately marked unregistered until native graph insertion exists.
"""
import argparse
from collections import defaultdict, Counter, deque
import hashlib
import json
from pathlib import Path
import struct

from animation_dct import decode, take, envelope, parse
from animation_fields import Fields
from compatibility import Install, Oodle, frames
from dct_reorder import permutation, reorder
from deke_native_audit import native_id
from native_ant import Bank
from deke_expanded_bank import SCALARS


def native_clip_fragment(resource, fields, root_id, ctd_id, dsl_id):
    """Compile known target records; binding fixups remain symbolic until linking."""
    cf = next(i['fields'] for i in fields['instances'] if 'ClipControllerAsset' in i['type'])
    af = next(i['fields'] for i in fields['instances'] if 'DctAnimationAsset' in i['type'])
    if any(cf[k] is not None for k in ('RigRef', 'DofCodecAnim')):
        raise ValueError('Clip has additional dependencies requiring native serialization')
    dependencies = []
    if cf['TagCollectionSetAsset'] is not None:
        dependencies.append(dict(site=16, source_reference=cf['TagCollectionSetAsset'],
            field='TagCollectionSetAsset', requires='Serialize and link complete tag/event subtree'))
    meta, _ = parse(resource)
    if (af['NumCompressedQuat'], af['NumCompressedVec3'], af['NumCompressedVecFloat']) != (
            meta['quaternions'], meta['vectors'], meta['float_vectors']):
        raise ValueError('Animation metadata and compressed dimensions disagree')
    env, body = envelope(resource)
    # Keep the animation 16-byte aligned, with its target-sized record intact.
    anim = 80
    data = bytearray(anim + 104)
    struct.pack_into('<4f6B', data, 48, *(cf[k] for k in SCALARS))
    struct.pack_into('<IIffH', data, anim + 16, af['CodecType'], af['AnimId'],
                     af['TrimOffset'], af['TrimmedDuration'], af['EndFrame'])
    struct.pack_into('<I', data, anim + 56, af['BaseFlags'])
    struct.pack_into('<I', data, anim + 72, len(env['key_times']))
    struct.pack_into('<I', data, anim + 88, len(body))
    struct.pack_into('<HH?', data, anim + 96, af['NumVec3'], af['NumFloat'], af['Cycle'])
    relocations = [(32, anim)]
    if env['key_times']:
        relocations.append((anim + 64, len(data)))
        data += struct.pack('<' + 'H' * len(env['key_times']), *env['key_times'])
    data += bytes((-len(data)) % 16)
    relocations.append((anim + 80, len(data)))
    data += body
    # Round-trip through the existing native clip reader without claiming this
    # relocatable fragment is a standalone bank accepted by the game loader.
    view = object.__new__(Bank)
    view.blob = data
    view.reloc = dict(relocations)
    view.by_id = {root_id: dict(offset=0, size=72, type_hash=1556467357),
                  -1: dict(offset=anim, size=104, type_hash=3842202299)}
    if view.clip(root_id)[2] != resource:
        raise ValueError('Native fragment resource round trip failed')
    return data, dict(
        objects=[dict(id=hex(root_id), offset=0, size=72, type_hash=1556467357),
                 dict(id=None, identity_policy='Allocate a unique bank-local ID at link time',
                      offset=anim, size=104, type_hash=3842202299)],
        internal_relocations=[dict(site=s, destination=d) for s, d in relocations],
        symbolic_relocations=[dict(site=anim + 40, target_id=hex(ctd_id)),
                              dict(site=anim + 48, target_id=hex(dsl_id))],
        unresolved_dependencies=dependencies,
        qualified_as='Unregistered relocatable records, not an installable bank')


def incoming(catalog):
    result = defaultdict(dict)
    for node in catalog:
        for partition in {p['partition'] for p in node['fixup']['imports']}:
            result[partition][node['asset']['sha1']] = node
    return result


def native_boundaries(partition, parents, identities, limit=4096):
    """Find first existing native ancestors along each reverse dependency path."""
    queue = deque([(partition, [])])
    seen = {partition}
    boundaries, dead_ends = [], []
    while queue:
        current, path = queue.popleft()
        nodes = sorted(parents[current].values(), key=lambda n: (n['name'], n['asset']['sha1']))
        if not nodes:
            dead_ends.append(dict(partition=current, path=path))
        for node in nodes:
            p = node['fixup']['partition']
            edge = dict(name=node['name'], partition=p, sha1=node['asset']['sha1'])
            next_path = path + [edge]
            if native_id(p) in identities:
                boundaries.append(dict(native_id=hex(native_id(p)), path=next_path))
            elif p not in seen:
                if len(seen) >= limit:
                    raise ValueError('Reverse dependency traversal exceeds explicit bound')
                seen.add(p)
                queue.append((p, next_path))
    return dict(existing_ancestor_paths=boundaries, dead_ends=dead_ends,
                visited_partitions=len(seen), qualification='Import reachability, not runtime move selection')


def compatible_map(source, dimensions, maps):
    candidates = []
    for ident, target in maps.items():
        try:
            order = permutation(source, target, *dimensions)
        except ValueError:
            continue
        # Prefer an exact channel order, then a stable ID, with no name heuristic.
        candidates.append((order != list(range(len(order))), ident, order))
    if not candidates:
        raise ValueError('No existing target CTD with compatible DOF categories and scalar lanes')
    _, ident, order = min(candidates)
    return ident, order, len(candidates)


def channel_categories(bank, maps):
    """Recover DOF kinds from target animations which actually use each CTD."""
    by_offset = {o['offset']: ident for ident, o in bank.by_id.items()}
    categories = defaultdict(set)
    for obj in bank.by_id.values():
        p = obj['offset']
        if (obj['type_hash'], obj['size']) == (3842202299, 104):
            body = bank.pointer(p + 80)
            _, q, v, f = struct.unpack_from('<4H', bank.blob, body)
        elif (obj['type_hash'], obj['size']) == (1531114270, 184):
            q, v, f, cq, cv, cf = struct.unpack_from('<6H', bank.blob, p + 108)
            q, v, f = q + cq, v + cv, f + cf
        else:
            continue
        ident = by_offset.get(bank.reloc.get(p + 40))
        if ident not in maps:
            continue
        dofs = maps[ident]
        if len(dofs) < q + v:
            raise ValueError('Target CTD shorter than rotation/vector channel count')
        for i, dof in enumerate(dofs):
            categories[dof].add('quaternion' if i < q else 'vector' if i < q + v else 'float')
    return categories


def run(root, stage_directory=None, output_name='new-moves'):
    dest = stage_directory or root / 'deke-trial'
    out = dest / output_name
    out.mkdir(exist_ok=True)
    stage = json.loads((dest / 'stage-report.json').read_text())
    catalog = json.loads((root / 'deke-trial/source-catalog.json').read_text())
    types = json.loads((root / 'source-type-descriptors.json').read_text())
    parents = incoming(catalog)
    banks = {name: Bank((root / 'ant/nhl24_160' / (name + '.bin')).read_bytes())
             for name in ('state', 'launch')}
    identities = set().union(*(b.by_id for b in banks.values()))
    launch = banks['launch']
    maps = {}
    for ident, obj in launch.by_id.items():
        if (obj['type_hash'], obj['size']) != (924757995, 32):
            continue
        p = obj['offset']
        count = struct.unpack_from('<I', launch.blob, p + 24)[0]
        if not 0 < count <= 16384:
            continue
        maps[ident] = list(struct.unpack('<' + 'I' * count,
            take(launch.blob, launch.pointer(p + 16), 4 * count)))
    bindings = defaultdict(list)
    known_categories = channel_categories(launch, maps)
    for b in stage['bindings']:
        bindings[b['partition']].append(b)
    install, decoder = Install('nhl27'), Oodle()
    exported = {}

    def export(node):
        a = node['asset']
        if a['sha1'] in exported:
            return exported[a['sha1']]
        raw = install.read(a['span'])
        if hashlib.sha1(raw).hexdigest() != a['sha1']:
            raise ValueError('Source stored resource hash mismatch')
        data = frames(raw, decoder)[1]
        if data is None or len(data) != a['size']:
            raise ValueError('Source decoded resource size mismatch')
        f = Fields(data, types).export()
        path = out / (a['sha1'] + '.fields.json')
        path.write_text(json.dumps(f, indent=2))
        exported[a['sha1']] = f
        return f

    rows = []
    for clip in stage['clips']:
        if 'partition' not in clip or native_id(clip['partition']) in identities:
            continue
        row = dict(name=clip['name'], partition=clip['partition'],
                   source_sha1=clip['resource_sha1'], parents=[])
        rows.append(row)
        row['integration_boundaries'] = native_boundaries(clip['partition'], parents, identities)
        for parent in sorted(parents[clip['partition']].values(), key=lambda x: x['name']):
            ident = native_id(parent['fixup']['partition'])
            fields = export(parent)
            row['parents'].append(dict(name=parent['name'], sha1=parent['asset']['sha1'],
                partition=parent['fixup']['partition'], span=parent['asset']['span'],
                types=sorted({t for i in fields['instances'] for t in i['type']}),
                fields=str(out / (parent['asset']['sha1'] + '.fields.json')),
                native_matches={name: b.by_id[ident] for name, b in banks.items() if ident in b.by_id},
                incoming_parents=[dict(name=n['name'], sha1=n['asset']['sha1'],
                    partition=n['fixup']['partition'])
                    for n in parents[parent['fixup']['partition']].values()]))
        try:
            prefix = dest / clip['resource_sha1']
            data = prefix.with_suffix('.resource').read_bytes()
            if hashlib.sha256(data).hexdigest() != clip['resource_sha256']:
                raise ValueError('Staged resource hash mismatch')
            fields = Fields(prefix.with_suffix('.ebx').read_bytes(), types).export()
            f = next(i['fields'] for i in fields['instances'] if 'DctAnimationAsset' in i['type'])
            refs = bindings[f['ChannelToDofAsset']['external']['partition']]
            choices = {tuple(next(i['fields']['0xec5cc5cd'] for i in
                Fields(Path(b['payload']).read_bytes(), types).export()['instances']
                if '0xec5cc5cd' in i['fields'])) for b in refs}
            if len(choices) != 1:
                raise ValueError('Missing or conflicting source CTD variants')
            source = list(choices.pop())
            dsl = native_id(f['DofSetList']['external']['instance'])
            if dsl not in launch.by_id:
                raise ValueError('Source DOF set is absent in target launch bank')
            meta, original, _ = decode(data)
            dimensions = tuple(meta[k] for k in ('quaternions', 'vectors', 'float_vectors'))
            try:
                ident, order, count = compatible_map(source, dimensions, maps)
                binding_status = 'existing_target_map'
            except ValueError:
                q, v, fv = dimensions
                # A missing map is not a license to delete channels. Retain the
                # source order only when every DOF/kind is observed in NHL24.
                order = permutation(source, source, q, v, fv)
                for i, dof in enumerate(source):
                    kind = 'quaternion' if i < q else 'vector' if i < q + v else 'float'
                    if kind not in known_categories[dof]:
                        raise ValueError(f'DOF {dof:#x} has no target {kind} evidence')
                ident = native_id(f['ChannelToDofAsset']['external']['instance'])
                if ident in identities:
                    raise ValueError('New CTD ID collides with existing target object')
                count = 0
                binding_status = 'new_target_map_requires_registration'
                # Emit the native 32-byte object plus its uint32 array. Vtable
                # and pointer slots stay zero; the bank loader owns their fixups.
                fragment = bytearray(32)
                struct.pack_into('<I', fragment, 24, len(source))
                fragment += struct.pack('<' + 'I' * len(source), *source)
                binding_path = out / (f'{ident:016x}.ctd-fragment.bin')
                binding_path.write_bytes(fragment)
                row['new_binding'] = dict(path=str(binding_path), object_size=32,
                    type_hash=924757995, sha256=hashlib.sha256(fragment).hexdigest(),
                    relocation=dict(site=16, destination=32), dofs=source,
                    target_dof_kind_evidence=True)
            if reorder(data, list(range(len(order)))) != data:
                raise ValueError('DCT identity round trip differs')
            converted = reorder(data, order)
            _, decoded, _ = decode(converted)
            if decoded != [[frame[i] for i in order] for frame in original]:
                raise ValueError('Decoded sample permutation differs')
            inverse = [order.index(i) for i in range(len(order))]
            if reorder(converted, inverse) != data:
                raise ValueError('Inverse conversion is not byte exact')
            path = out / (clip['resource_sha1'] + '.resource')
            path.write_bytes(converted)
            fragment, links = native_clip_fragment(converted, fields,
                native_id(clip['partition']), ident, dsl)
            fragment_path = out / (clip['resource_sha1'] + '.native-fragment.bin')
            fragment_path.write_bytes(fragment)
            (out / (clip['resource_sha1'] + '.fields.json')).write_text(json.dumps(fields, indent=2))
            row.update(status='converted_unregistered', binding_status=binding_status, path=str(path),
                sha256=hashlib.sha256(converted).hexdigest(), target_ctd_id=hex(ident),
                target_dsl_id=hex(dsl), compatible_target_ctd_count=count,
                channel_order=order, source_dofs=source, target_dofs=maps.get(ident, source),
                keyed_frames=meta['keys'], exact_decoded_permutation=True,
                inverse_byte_exact=True)
            row['native_fragment'] = dict(path=str(fragment_path),
                sha256=hashlib.sha256(fragment).hexdigest(), **links)
        except (ValueError, KeyError, StopIteration) as error:
            row.update(status='rejected', error=str(error))
    result = dict(status='conversion_stage_not_installable',
        target_bank_sha256={n: hashlib.sha256(b.data).hexdigest() for n, b in banks.items()},
        layer_policy='All explicit catalog occurrences retained; no last-wins source resolution',
        counts=dict(Counter(x['status'] for x in rows)), clips=rows)
    # Preserve every intermediate node on the discovered shortest integration
    # paths. This is a reverse path slice, not the full forward import closure.
    by_sha = {n['asset']['sha1']: n for n in catalog}
    route_nodes = {}
    for row in rows:
        for boundary in row['integration_boundaries']['existing_ancestor_paths']:
            for edge in boundary['path']:
                node = by_sha[edge['sha1']]
                f = export(node)
                ident = native_id(edge['partition'])
                route_nodes[edge['sha1']] = dict(**edge,
                    fields=str(out / (edge['sha1'] + '.fields.json')),
                    types=sorted({t for i in f['instances'] for t in i['type']}),
                    native_matches={name: b.by_id[ident] for name, b in banks.items() if ident in b.by_id})
    result['route_nodes'] = list(route_nodes.values())
    (out / 'report.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(dict(counts=result['counts'],
        parents_exported=len(exported),
        clips_with_existing_parents=sum(any(p['native_matches'] for p in x['parents']) for x in rows),
        rejected=[dict(name=x['name'], error=x['error']) for x in rows if 'error' in x]), indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('--stage-directory', type=Path)
    parser.add_argument('--output-name', default='new-moves')
    args = parser.parse_args()
    run(args.root, args.stage_directory, args.output_name)
