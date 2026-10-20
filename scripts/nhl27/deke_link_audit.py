"""Audit the combined staged graph before any bank/package installation.

This checks actual payload hashes, overlapping records, identities, relocation
slots, dependency closure, native interface adjustments and replacement types.
It does not equate a serializable graph with executable behavior parity.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path

from ant_interfaces import adjustment
from native_ant import Bank


def run(root, action_adapters=False):
    trial = root / 'deke-trial'
    controllers = json.loads((trial / 'controller-fragments/report.json').read_text())
    clips = json.loads((trial / 'closure-stage/new-moves/tagged-fragments/report.json').read_text())
    moves = json.loads((trial / 'closure-stage/new-moves/report.json').read_text())
    banks = {n: Bank((root / 'ant/nhl24_160' / (n + '.bin')).read_bytes()) for n in ('state', 'launch')}
    existing = defaultdict(list)
    for bank_name, bank in banks.items():
        for ident, obj in bank.by_id.items():
            existing[ident].append(dict(bank=bank_name, **obj))
    fragments = [(row['name'], row['fragment']) for row in controllers['partitions'] if 'fragment' in row]
    panic_path = trial / 'panic-runtime-fragments.json'
    if panic_path.exists():
        fragments.extend((row['name'], row['fragment'])
                         for row in json.loads(panic_path.read_text())['fragments'])
    checking_path = trial / 'panic-checking-fragments/report.json'
    if checking_path.exists():
        fragments.extend((row['name'], row['fragment'])
                         for row in json.loads(checking_path.read_text())['fragments'])
    node_path = trial / 'panic-node-fragment.json'
    if node_path.exists():
        fragments.append(('PTM skating node activation', json.loads(node_path.read_text())))
    fragments.extend((row['name'], fragment) for row in clips['clips'] if 'fragments' in row for fragment in row['fragments'])
    bindings = {}
    for clip in moves['clips']:
        if 'new_binding' not in clip:
            continue
        ident, binding = int(clip['target_ctd_id'], 16), clip['new_binding']
        if ident in bindings:
            if bindings[ident]['sha256'] != binding['sha256']:
                raise ValueError('Conflicting new channel-map definitions')
            continue
        bindings[ident] = binding
        fragments.append(('new channel map', dict(path=binding['path'], sha256=binding['sha256'],
            objects=[dict(id=ident, offset=0, size=binding['object_size'], type_hash=binding['type_hash'])],
            internal_relocations=[binding['relocation']], symbolic_relocations=[])))
    if action_adapters:
        from ant_action_adapters import bindings as action_bindings, adapt
        mapping = action_bindings(json.loads((trial / 'input-adapter-audit.json').read_text()),
                                  banks['state'], banks['launch'])
        fragments = [(name, adapt(fragment, mapping)) for name, fragment in fragments]
        gesture_path = trial / 'fake-shot-gesture-fragment.json'
        if gesture_path.exists():
            fragments.append(('source R3 fake-shot mapping', json.loads(gesture_path.read_text())))
        (trial / 'action-adapted-fragments.json').write_text(json.dumps(
            dict(installed=False, fragments=[dict(name=name, fragment=f) for name, f in fragments]), indent=2))
    definitions, errors, pending, replacements = {}, [], [], []
    compatibility_requirements = []
    native_types = set().union(*(set(zip(b.hashes, b.sizes)) for b in banks.values()))
    new_types = Counter()
    for name, fragment in fragments:
        compatibility_requirements.extend(dict(source_name=name, **item)
            for item in fragment.get('compatibility_requirements', []))
        data = Path(fragment['path']).read_bytes()
        if hashlib.sha256(data).hexdigest() != fragment['sha256']:
            raise ValueError(f'Payload hash mismatch: {name}')
        spans = []
        for obj in fragment['objects']:
            ident, at, size = obj['id'], obj['offset'], obj['size']
            if not isinstance(ident, int) or not 0 < ident < (1 << 64):
                raise ValueError('Invalid staged native identity')
            if at < 0 or at % 8 or size <= 0 or at + size > len(data):
                raise ValueError('Object outside payload or misaligned')
            if any(at < end and begin < at + size for begin, end in spans):
                raise ValueError('Overlapping staged objects')
            spans.append((at, at + size))
            if ident in definitions:
                errors.append(dict(kind='duplicate_definition', id=hex(ident), name=name))
            definitions[ident] = dict(obj, source_name=name)
            if (obj['type_hash'], size) not in native_types:
                new_types[(obj['type_hash'], size)] += 1
            for old in existing[ident]:
                if (old['type_hash'], old['size']) != (obj['type_hash'], size):
                    errors.append(dict(kind='replacement_type_mismatch', id=hex(ident), name=name, bank=old['bank']))
                else:
                    replacements.append(dict(id=hex(ident), name=name, bank=old['bank']))
        sites = set()
        for ref in (fragment.get('internal_relocations', []) + fragment.get('symbolic_relocations', [])
                    + fragment.get('inherited_relocations', [])):
            site = ref['site']
            if site < 0 or site + 8 > len(data) or any(data[site:site + 8]) or site in sites:
                raise ValueError('Invalid, nonzero or repeated pointer relocation slot')
            sites.add(site)
            if 'destination' in ref:
                if not 0 <= ref['destination'] < len(data):
                    raise ValueError('Internal relocation outside payload')
            elif 'source_site' in ref:
                bank = banks.get(fragment.get('target_bank'))
                if bank is None or hashlib.sha256(bank.data).hexdigest() != fragment.get('template_bank_sha256'):
                    raise ValueError('Inherited relocation template bank mismatch')
                source_site = ref['source_site']
                external_sites = {row[1] for row in bank.external}
                if (source_site in bank.reloc) == (source_site in external_sites):
                    raise ValueError('Inherited source is not one unambiguous native fixup')
            else:
                pending.append((name, ref))
    missing = Counter()
    interface_errors = Counter()
    for name, ref in pending:
        ident = ref['target_id']
        target = definitions.get(ident)
        if target is None and existing[ident]:
            target = existing[ident][0]
        if target is None:
            missing[hex(ident)] += 1
            continue
        try:
            adjustment(target['type_hash'], ref.get('interface'))
        except ValueError as error:
            interface_errors[str(error)] += 1
    report = dict(status='incomplete_not_installable', fragment_count=len(fragments),
        defined_objects=len(definitions), replacements=replacements,
        unresolved_native_ids=dict(missing), interface_blockers=dict(interface_errors), errors=errors,
        compatibility_requirements=compatibility_requirements,
        types_requiring_registry_extension=[dict(type_hash=hex(h), size=s, objects=n) for (h, s), n in sorted(new_types.items())],
        uncompiled_partitions=[dict(name=r['name'], error=r['error']) for r in controllers['partitions'] if 'error' in r],
        executable_behavior_qualified=False, installed=False)
    name = 'action-adapted-link-audit.json' if action_adapters else 'combined-link-audit.json'
    (trial / name).write_text(json.dumps(report, indent=2))
    print(json.dumps(dict(fragments=len(fragments), defined_objects=len(definitions),
        replacements=len(replacements), unresolved_native_ids=dict(missing),
        interface_blockers=dict(interface_errors), errors=errors,
        compatibility_requirements=len(compatibility_requirements),
        registry_extensions=report['types_requiring_registry_extension']), indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('--action-adapters', action='store_true')
    args = parser.parse_args()
    run(args.root, args.action_adapters)
