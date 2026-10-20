"""Compile supported NHL27 embedded tag trees into NHL24 native records.

Layouts are traced through NHL24 1.60 serializers 8e0f50, 8e10d0,
8e0bd0, 7e0760, 7de8d0, 7df720, 3b2dc0, 3b3410, 901280, 902640.
The caller owns identities and dependency resolution. Unknown tags fail closed.
"""
import hashlib
import struct

from deke_native_audit import native_id
from ant_controllers import LAYOUTS as CONTROLLER_LAYOUTS, INLINE_TYPES, emit_controller
from ant_interfaces import adjustment
from ant_databases import LAYOUTS as DATABASE_LAYOUTS, INLINE_TYPES as DATABASE_INLINE_TYPES, emit_database
from ant_ptm import LAYOUTS as PTM_LAYOUTS, INLINE_TYPES as PTM_INLINE_TYPES, INITIALIZED_TYPES, emit_ptm

LAYOUTS = {
    'MotionInfoTag': (0x18df8493, 40),
    'PTMKeepCurrentAnimBiasTag': (0x03c38016, 32),
    'BoolGameStateTagAsset': (0x4a11ba30, 56),
    'FloatGameStateTagAsset': (0x34de3e49, 56),
    'DefaultTagCollectionAsset': (0x3d4a3aec, 48),
    'TagCollectionSetAsset': (0x143af97a, 24),
}
LAYOUTS.update(CONTROLLER_LAYOUTS)
LAYOUTS.update(DATABASE_LAYOUTS)
LAYOUTS.update(PTM_LAYOUTS)
INLINE_TYPES = INLINE_TYPES | DATABASE_INLINE_TYPES | PTM_INLINE_TYPES
MONITORED = {'BoolGameStateTagAsset', 'FloatGameStateTagAsset'}


def allocate_id(partition, offset, occupied):
    """Allocate deterministic private IDs, checking collisions against both banks."""
    for salt in range(65536):
        key = f'nhl27-port-native-v1:{partition}:{offset}:{salt}'.encode()
        ident = int.from_bytes(hashlib.sha256(key).digest()[:8], 'little')
        if ident and ident not in occupied:
            occupied.add(ident)
            return ident
    raise ValueError('Unable to allocate collision-free internal object identity')


def compile_tree(fields, root, occupied, exported_ids=None):
    nodes = {i['offset']: i for i in fields['instances']}
    data, objects, internal, symbolic = bytearray(), [], [], []
    offsets, ids, pending = {}, {}, []
    requirements = []
    exported_ids = exported_ids or {}

    def node_type(ref):
        if not ref or 'internal_offset' not in ref:
            raise ValueError('Tag classification requires a local source instance')
        types = nodes[ref['internal_offset']]['type']
        if len(types) != 1 or types[0] not in LAYOUTS:
            raise ValueError(f'Unsupported native tag type: {types}')
        return types[0]

    def pointer(site, ref, interface=None):
        if ref is None:
            return
        if 'external' in ref:
            entry = dict(site=site, target_id=native_id(ref['external']['instance']))
            if interface is not None:
                entry['interface'] = interface
            symbolic.append(entry)
        elif 'internal_offset' in ref:
            pending.append((site, ref['internal_offset'], interface))
        else:
            raise ValueError('Unsupported tag reference')

    def array(site, count_site, values):
        if count_site is not None:
            struct.pack_into('<Q', data, count_site, len(values))
        if not values:
            return None
        data.extend(bytes((-len(data)) % 8))
        at = len(data)
        data.extend(bytes(8 * len(values)))
        internal.append(dict(site=site, destination=at))
        for index, ref in enumerate(values):
            pointer(at + index * 8, ref)
        return at

    def emit(source_offset):
        if source_offset in offsets:
            return offsets[source_offset]
        node = nodes[source_offset]
        kind = node_type(dict(internal_offset=source_offset))
        f = node['fields']
        th, size = LAYOUTS[kind]
        # Several controller records contain SIMD vectors. Sixteen-byte object
        # alignment also satisfies the smaller scalar/tag record requirements.
        data.extend(bytes((-len(data)) % 16))
        at = len(data)
        data.extend(bytes(size))
        offsets[source_offset] = at
        ident = exported_ids.get(source_offset)
        if ident is None:
            ident = allocate_id(fields['partition'], source_offset, occupied)
        else:
            if not 0 < ident < (1 << 64) or ident in ids.values():
                raise ValueError('Invalid or duplicate exported object identity')
            occupied.add(ident)
        ids[source_offset] = ident
        if kind in INLINE_TYPES:
            if source_offset in exported_ids:
                raise ValueError('Owned inline record cannot be an exported native object')
        else:
            objects.append(dict(id=ident, offset=at, size=size, type_hash=th,
                                name=f.get('Name') or None))
            if kind in INITIALIZED_TYPES:
                # Native master slot 6 (326c90) restores owned 504-byte database
                # vtables after relocation. Owned records have no object entry.
                objects[-1]['initialize'] = True
        if kind in PTM_LAYOUTS:
            requirements.extend(dict(source_offset=source_offset, **item)
                for item in emit_ptm(kind, f, at, data, pointer, array))
        elif kind in DATABASE_LAYOUTS:
            emit_database(kind, f, at, data, pointer, array)
        elif kind in CONTROLLER_LAYOUTS:
            emit_controller(kind, f, at, data, pointer, array)
        elif kind == 'TagCollectionSetAsset':
            array(at + 8, at + 16, f['TagCollections'])
        elif kind == 'DefaultTagCollectionAsset':
            tags = f['Tags']
            # Native base collection retains all tags; the derived collection
            # additionally indexes MonitoredTag descendants for enter/leave.
            monitored = [ref for ref in tags if node_type(ref) in MONITORED]
            array(at + 8, at + 16, tags)
            array(at + 24, at + 32, monitored)
            struct.pack_into('<?', data, at + 40, f['HasExitTags'])
        else:
            struct.pack_into('<fHB', data, at + 8,
                f['StartLinkOffset'], f['LinkFlags'], f['StartLinkUpdates'])
            struct.pack_into('<f', data, at + 16, f['EndLinkOffset'])
            if kind == 'MotionInfoTag':
                # Native serializer consumes ValuesRef only. The source enum
                # schema reference is compile-time metadata, not a native slot.
                array(at + 24, at + 32, f['ValuesRef'])
            elif kind == 'PTMKeepCurrentAnimBiasTag':
                struct.pack_into('<f', data, at + 24, f['Bias'])
            else:
                pointer(at + 24, f['TagEnable'])
                pointer(at + 32, f['GameStateValue'])
                struct.pack_into('<I', data, at + 40, f['ResetOnLeaveMode'])
                struct.pack_into('<I', data, at + 48, f['Mode'])
                struct.pack_into('<?' if kind == 'BoolGameStateTagAsset' else '<f',
                                 data, at + 52, f['Value'])
        return at

    node_type(root)
    emit(root['internal_offset'])
    cursor = 0
    while cursor < len(pending):
        site, source_offset, interface = pending[cursor]
        destination = emit(source_offset)
        delta = adjustment(LAYOUTS[node_type(dict(internal_offset=source_offset))][0], interface)
        if delta is not None:
            internal.append(dict(site=site, destination=destination + delta))
        cursor += 1
    return dict(data=bytes(data), objects=objects, internal_relocations=internal,
                symbolic_relocations=symbolic, root_id=ids[root['internal_offset']],
                source_offset_ids=ids, compatibility_requirements=requirements)
