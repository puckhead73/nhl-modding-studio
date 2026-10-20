"""Adapt source action references to the eight native Boolean assets.

Reusing the existing objects preserves their context property allocation and
reset interfaces. Copying a serialized Boolean would allocate a separate
property and would not receive the game's native producer writes. References
remain blocked from installation until producer and scheduling parity is met.
"""
import copy
import hashlib

from ant_interfaces import adjustment
from deke_native_audit import native_id


def bindings(audit, state, launch):
    for key, bank in [('target_state_bank_sha256', state), ('target_bank_sha256', launch)]:
        if hashlib.sha256(bank.data).hexdigest() != audit[key]:
            raise ValueError('Action adapter bank hash mismatch')
    mapping = {}
    for row in audit['candidates']:
        source = native_id(row['source_partition'])
        target = int(row['target_id'], 16)
        obj = launch.by_id[target]
        if (obj['type_hash'], obj['size']) != (0x33e57bd6, 104):
            raise ValueError('Action adapter target is not a native Boolean')
        # Source ActionInputBool's default getter returns zero. The native
        # BoolAsset default getter reads +0x58, independently of reset flags.
        if launch.blob[obj['offset'] + 88] != 0:
            raise ValueError('Action adapter default value differs')
        flags = tuple(row['source_flags'][k] for k in (
            'DeactivateAfterUpdateEnabled', 'DeactivateAfterTransitionEnabled',
            'DeactivateAfterUpdateValue'))
        actual = tuple(bool(v) for v in launch.blob[obj['offset']+96:obj['offset']+99])
        if flags != actual:
            raise ValueError('Action adapter reset semantics differ')
        if source in mapping:
            raise ValueError('Repeated source action identity')
        mapping[source] = dict(target_id=target, action=row['action'], type_hash=obj['type_hash'])
    if len(mapping) != 8 or len({r['action'] for r in mapping.values()}) != 8:
        raise ValueError('Expected exactly eight distinct action adapters')
    return mapping


def adapt(fragment, mapping):
    """Return a separate fragment manifest; its binary payload stays unchanged."""
    result = copy.deepcopy(fragment)
    used = {}
    for ref in result.get('symbolic_relocations', []):
        source = ref['target_id']
        if source not in mapping:
            continue
        binding = mapping[source]
        # Optional missing interfaces remain null under the native linker's
        # existing policy; unknown interfaces fail rather than being invented.
        adjustment(binding['type_hash'], ref.get('interface'))
        ref['target_id'] = binding['target_id']
        used[source] = binding
    if used:
        requirements = result.setdefault('compatibility_requirements', [])
        requirements.extend(dict(kind='action_producer_and_scheduling_parity',
            action=binding['action'], source_id=hex(source), target_id=hex(binding['target_id']))
            for source, binding in sorted(used.items()))
        result['action_reference_adapters'] = [dict(source_id=hex(source), **binding)
                                               for source, binding in sorted(used.items())]
    return result
