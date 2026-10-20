"""Link fully resolved object fragments into NHL24 version-6 ANT memory images.

Only existing runtime types are allowed. Incomplete dependency manifests fail
closed. This module does not activate controller branches or install game files.
Outer +28 reserves a vector (adf850 -> ae13a0); +32 is an optional arena size
(ad74b0). Use zero to select the loader's existing general-allocator path instead
of inventing a fixed-arena capacity for a grown image. Native memory-image loads
already receive their image allocation separately from resource metadata.
"""
import struct
import bisect
import hashlib
from animation_dct import take
from native_ant import Bank
from ant_interfaces import adjustment


def signed_id(value):
    if not 0 <= value < 1 << 64:
        raise ValueError('Object identity exceeds uint64')
    return value if value < 1 << 63 else value - (1 << 64)


def link(bank, fragments, replace_ids=(), registered_types=(), external_types=None):
    """Each fragment supplies data, objects, relocations and symbolic references.

    Object records require id, offset, size, type_hash and optional name/initialize.
    Fragment offsets are local. All names and old offsets remain stable in blob;
    tables are sorted by signed ID and initialization indices are remapped.

    registered_types contains audited (hash, size) pairs already implemented
    by the target executable, not registrations for new executable classes.
    external_types maps identities supplied by another loaded bank to their
    concrete native type. The caller must qualify bank load order separately.
    """
    if not fragments:
        if replace_ids:
            raise ValueError('Replacement identities require replacement fragments')
        return bytes(bank.data)
    replace_ids = set(replace_ids)
    if not replace_ids <= set(bank.by_id):
        raise ValueError('Replacement identity is absent from original bank')
    blob = bytearray(bank.blob)
    objects = list(bank.objects)
    names = dict(zip((o[1] for o in objects), bank.names))
    initialized = {objects[i][1] for i in bank.values('indices', '<I')}
    identities = {ident: offset for offset, ident, typ in objects}
    identity_types = {ident: bank.hashes[typ] for offset, ident, typ in objects}
    types = {(h, size): i for i, (h, size) in enumerate(zip(bank.hashes, bank.sizes))}
    sizes, hashes = list(bank.sizes), list(bank.hashes)
    approved = set(registered_types)
    external_types = external_types or {}
    if len({h for h, _ in approved}) != len(approved):
        raise ValueError('Conflicting sizes for registered native type')
    for h, size in approved:
        if not 0 < h < (1 << 32) or not 0 < size < (1 << 32) or size % 8:
            raise ValueError('Invalid registered native type/layout')
        if h in hashes and sizes[hashes.index(h)] != size:
            raise ValueError('Registered type conflicts with existing native layout')
    for ident, h in external_types.items():
        signed_id(ident)
        if not 0 < h < (1 << 32):
            raise ValueError('Invalid external native type')
        if ident in identity_types and identity_types[ident] != h:
            raise ValueError('External identity conflicts with local native type')
    relocations = list(bank.relocations)
    pending = []
    inherited_external = []
    replaced = {}
    emitted = set()
    for fragment in fragments:
        if fragment.get('compatibility_requirements'):
            raise ValueError('Fragment has unqualified source/target runtime semantics')
        if fragment.get('unresolved_dependencies'):
            raise ValueError('Fragment contains unresolved tag/event dependencies')
        data = bytes(fragment['data'])
        blob += bytes((-len(blob)) % 16)
        base = len(blob)
        blob += data
        occupied = []
        for obj in fragment['objects']:
            ident = obj['id']
            if ident is None:
                raise ValueError('Fragment object has no allocated identity')
            signed_id(ident)
            if ident in emitted or (ident in identities and ident not in replace_ids):
                raise ValueError('Duplicate object identity during linking')
            at, size = obj['offset'], obj['size']
            take(data, at, size)
            if at % 8 or any(at < z and a < at + size for a, z in occupied):
                raise ValueError('Misaligned or overlapping fragment object')
            occupied.append((at, at + size))
            typ = types.get((obj['type_hash'], size))
            if typ is None and (obj['type_hash'], size) in approved:
                typ = len(sizes)
                types[(obj['type_hash'], size)] = typ
                sizes.append(size)
                hashes.append(obj['type_hash'])
            if typ is None:
                raise ValueError('Fragment type/layout is absent from target registry table')
            if ident in replace_ids:
                old = bank.by_id[ident]
                if old['type_hash'] != obj['type_hash'] or old['size'] != size:
                    raise ValueError('Replacement must preserve native type and interface layout')
                replaced[ident] = (old['offset'], old['offset'] + size, base + at)
                objects = [o for o in objects if o[1] != ident]
            emitted.add(ident)
            identities[ident] = base + at
            identity_types[ident] = obj['type_hash']
            objects.append((base + at, ident, typ))
            name = obj.get('name')
            if name is None:
                names[ident] = (1 << 64) - 1
            else:
                if '\0' in name:
                    raise ValueError('Embedded NUL in object name')
                names[ident] = len(blob)
                blob += name.encode('utf-8') + b'\0'
            if obj.get('initialize', False):
                initialized.add(ident)
        for ref in fragment.get('internal_relocations', []):
            site, dst = ref['site'], ref['destination']
            if any(take(data, site, 8)):
                raise ValueError('Relocation slot must be zero before loader fixup')
            take(data, dst, 1)
            relocations.append((base + site, base + dst))
        for ref in fragment.get('symbolic_relocations', []):
            site = ref['site']
            if any(take(data, site, 8)):
                raise ValueError('Symbolic relocation slot must be zero')
            pending.append((base + site, ref['target_id'], ref.get('interface')))
        # A native-record adaptation may retain an existing array or interface
        # pointer without reverse engineering its containing allocation. Require
        # the exact bank and original fixup site, never an unchecked destination.
        inherited = fragment.get('inherited_relocations', [])
        if inherited and fragment.get('template_bank_sha256') != hashlib.sha256(bank.data).hexdigest():
            raise ValueError('Inherited fixup template bank mismatch')
        original_external = {row[1]: row for row in bank.external}
        for ref in inherited:
            site, original = ref['site'], ref['source_site']
            if any(take(data, site, 8)):
                raise ValueError('Inherited relocation slot must be zero')
            if original in bank.reloc and original not in original_external:
                relocations.append((base + site, bank.reloc[original]))
            elif original in original_external and original not in bank.reloc:
                owner, _, ident, typ, flags = original_external[original]
                if owner != -1:
                    raise ValueError('Owned external fixup cannot be inherited')
                inherited_external.append((-1, base + site, ident, typ, flags))
            else:
                raise ValueError('Inherited source is not one unambiguous native fixup')
    if set(replaced) != replace_ids:
        raise ValueError('Not every requested replacement has a fragment')
    # Incoming references can address secondary interfaces inside an object.
    # Retarget the entire audited object range, preserving the interior delta.
    # Old outgoing storage remains inert in the preserved blob prefix.
    ranges = sorted(replaced.values())
    starts = [r[0] for r in ranges]

    def retarget(destination):
        index = bisect.bisect_right(starts, destination) - 1
        if index >= 0:
            lo, hi, new = ranges[index]
            if destination < hi:
                return new + destination - lo
        return destination

    relocations = [(site, retarget(dst)) for site, dst in relocations]
    external = [(retarget(owner) if owner != -1 else -1, site, ident, typ, flags)
                for owner, site, ident, typ, flags in bank.external]
    external.extend(inherited_external)
    for site, ident, interface in pending:
        if ident not in identities:
            if ident not in external_types:
                raise ValueError(f'Unresolved native identity {ident:#x}')
            concrete = external_types[ident]
            delta = adjustment(concrete, interface)
            if delta is not None:
                # Existing state-bank fixups use absent owner (-1), the
                # requested interface hash and flags=0. The loader performs
                # the cast; no foreign-bank blob offset is serialized here.
                external.append((-1, site, ident, concrete if interface is None else interface, 0))
            continue
        delta = adjustment(identity_types[ident], interface)
        if delta is not None:
            relocations.append((site, identities[ident] + delta))
    sites = set()
    # Original tables may contain duplicate identical relocation pairs; preserve
    # those, but never allow an appended fixup to reuse any original/new slot.
    for site, dst in bank.relocations:
        sites.add(site)
    sites.update(row[1] for row in bank.external)
    for site, dst in relocations[len(bank.relocations):]:
        if site in sites:
            raise ValueError('Multiple fixups address the same new pointer slot')
        sites.add(site)
    for _, site, _, _, _ in external[len(bank.external):]:
        if site in sites:
            raise ValueError('Multiple fixups address the same new pointer slot')
        sites.add(site)
    objects.sort(key=lambda o: signed_id(o[1]))
    indices = [i for i, obj in enumerate(objects) if obj[1] in initialized]
    count = len(objects)
    outer = list(struct.unpack_from('>9I', bank.data))
    old_count = len(bank.objects)
    extensions = bank.data[36 + 24 * old_count:bank.header]
    outer[0] = 36 + count * 24 + len(extensions)
    outer[2] = outer[3] = count
    # Preserve the original vector reserve hint: its vector grows dynamically.
    # Disable the optional fixed arena, rather than guessing a larger capacity.
    outer[8] = 0
    result = bytearray(struct.pack('>9I', *outer))
    result += b''.join(struct.pack('<QQ', ident, 0) for _, ident, _ in objects)
    result += b''.join(struct.pack('<Q', ident) for _, ident, _ in objects)
    result += extensions
    result += struct.pack('<12Q', 6, len(sizes), 8, len(hashes), 4,
        len(relocations), 16, count, 24, len(external), len(indices), len(blob))
    result += struct.pack('<' + 'Q' * len(sizes), *sizes)
    result += struct.pack('<' + 'I' * len(hashes), *hashes)
    result += b''.join(struct.pack('<QQ', *r) for r in relocations)
    result += b''.join(struct.pack('<QQQ', *o) for o in objects)
    result += b''.join(struct.pack('<qQQII', *row) for row in external)
    result += b''.join(struct.pack('<I', i) for i in indices)
    result += b''.join(struct.pack('<Q', names[ident]) for _, ident, _ in objects)
    result += blob
    check = Bank(result)
    if check.blob[:len(bank.blob)] != bank.blob:
        raise ValueError('Linker changed existing data')
    if set(check.by_id) != set(identities):
        raise ValueError('Linked identities do not round trip')
    return bytes(result)
