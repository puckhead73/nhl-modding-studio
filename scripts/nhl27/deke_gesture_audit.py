"""Recover fake-shot producers from hashed source assets and original target banks.

This audits configuration data, not gesture scheduler equivalence. No installation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import uuid

from animation_fields import Fields
from compatibility import Install, Oodle, frames
from native_ant import Bank


class Source:
    def __init__(self, root):
        self.catalog = json.loads((root / 'deke-trial/source-catalog.json').read_text())
        self.types = json.loads((root / 'source-type-descriptors.json').read_text())
        self.install, self.oodle = Install('nhl27'), Oodle()
        self.evidence = {}

    def partition(self, partition):
        nodes = [n for n in self.catalog if n.get('fixup', {}).get('partition') == partition]
        if not nodes or len({n['asset']['sha1'] for n in nodes}) != 1:
            raise ValueError(f'Absent or ambiguous source partition: {partition}')
        asset = nodes[0]['asset']
        stored = self.install.read(asset['span'])
        if hashlib.sha1(stored).hexdigest() != asset['sha1']:
            raise ValueError('Source payload hash mismatch')
        data = frames(stored, self.oodle)[1]
        if len(data) != asset['size']:
            raise ValueError('Source payload size mismatch')
        reader = Fields(data, self.types)
        result = reader.export()
        result['exported_instance_guids'] = {str(uuid.UUID(bytes_le=data[32+offset-16:32+offset])): offset
            for offset in reader.fixup['instance_offsets'][:reader.fixup['exported_instances']]}
        self.evidence[partition] = dict(stored_sha1=asset['sha1'],
            decoded_sha256=hashlib.sha256(data).hexdigest(), fields=result)
        return result

    def follow(self, reference):
        document = self.partition(reference['external']['partition'])
        instance = reference['external']['instance']
        if instance not in document['exported_instance_guids']:
            raise ValueError('Imported instance is not exported by its source partition')
        return dict(document, selected_offset=document['exported_instance_guids'][instance])


def selected(document):
    offset = document.get('selected_offset', document['instances'][0]['offset'])
    return next(n for n in document['instances'] if n['offset'] == offset)


def source_gesture(document):
    nodes = {n['offset']: n['fields'] for n in document['instances']}
    instance = selected(document)
    root = instance['fields']
    if instance['type'] == ['GamepadButtonsMatchAsset']:
        return {k: v for k, v in root.items() if k != 'Name'}
    if instance['type'] != ['GestureAsset']:
        raise ValueError('Unqualified gesture/matcher class')
    steps = []
    for ref in root['Steps']:
        step = nodes[ref['internal_offset']]
        matchers = []
        for ref in step['Matchers']:
            fields = nodes[ref['internal_offset']]
            matchers.append({k: v for k, v in fields.items() if k != 'Name'})
        steps.append(dict(MinTicks=step['MinTicks'], MaxTicks=step['MaxTicks'],
                          KeepMatching=step['KeepMatching'], Matchers=matchers))
    return dict(BufferSize=root['BufferSize'], CancelOnNotMatched=root['CancelOnNotMatched'], Steps=steps)


def native_array(bank, offset):
    count = struct.unpack_from('<Q', bank.blob, offset + 8)[0]
    if count > 1000:
        raise ValueError('Unexpected gesture array count')
    return [bank.pointer(bank.pointer(offset) + 8 * i) for i in range(count)] if count else []


def target_gesture(bank, ident):
    obj = bank.by_id[ident]
    if (obj['type_hash'], obj['size']) != (0xabfc18ac, 32):
        raise ValueError('Unexpected native GestureAsset layout')
    root = obj['offset']
    steps = []
    for step in native_array(bank, root + 16):
        matchers = []
        for matcher in native_array(bank, step + 8):
            values = struct.unpack_from('<iifffi?', bank.blob, matcher + 32)
            fields = dict(zip(('Button', 'MatchType', 'MinValue', 'MaxValue',
                               'Velocity', 'OutputValue', 'MatchValidState'), values))
            fields.update(BufferSize=struct.unpack_from('<f', bank.blob, matcher + 8)[0],
                          CancelOnNotMatched=bool(bank.blob[matcher + 12]))
            matchers.append(fields)
        minimum, maximum, keep = struct.unpack_from('<ff?', bank.blob, step + 24)
        steps.append(dict(MinTicks=minimum, MaxTicks=maximum, KeepMatching=keep, Matchers=matchers))
    return dict(BufferSize=struct.unpack_from('<f', bank.blob, root + 8)[0],
                CancelOnNotMatched=bool(bank.blob[root + 12]), Steps=steps)


def scalar_parameters(gesture):
    # External device/output references require separate identity qualification.
    if 'Steps' not in gesture:
        return {k: v for k, v in gesture.items() if k not in ('GamepadButtons', 'OutputTo')}
    return {**gesture, 'Steps': [{**s, 'Matchers': [
        {k: v for k, v in m.items() if k not in ('GamepadButtons', 'OutputTo')}
        for m in s['Matchers']]} for s in gesture['Steps']]}


def run(root):
    source = Source(root)
    bank = Bank((root / 'ant/nhl24_160/state.bin').read_bytes())
    expected = 'cc638f146bf85bed18cb9d945b7a567e8c37cd25482776fd73952efa316652b9'
    if hashlib.sha256(bank.data).hexdigest() != expected:
        raise ValueError('Requires the original qualified NHL24 state bank')
    config = source.partition('a231f1bf-4414-125a-a231-f1bf4414125a')['instances'][0]['fields']
    mappings = []
    for ref in config['0x07c6bcd3'] if '0x07c6bcd3' in config else config['0x7c6bcd3']:
        mapping = selected(source.follow(ref))['fields']
        signal = selected(source.follow(mapping['Signal']))['fields']
        gestures = [source_gesture(source.follow(g)) for g in mapping['Gesture']]
        mappings.append(dict(name=mapping['Name'], action=signal['ActionInput'],
                             gestures=gestures, enabled=mapping['Enabled']))
    target = target_gesture(bank, 0x39d0fba02d925576)
    default = next(m for m in mappings if m['action'] == 172)
    native_config = bank.by_id[0x125a4414a231f1bf]
    native_count = len(native_array(bank, native_config['offset'] + 24))
    report = dict(installed=False, target_state_sha256=expected, source_mappings=mappings,
        target_default_gesture=target,
        default_scalar_parameters_equal=scalar_parameters(default['gestures'][0]) == target,
        source_config_mapping_count=len(mappings), target_config_mapping_count=native_count,
        qualification='parameter_comparison_only_scheduler_and_configuration_composition_unqualified',
        source_evidence=source.evidence)
    destination = root / 'deke-trial/fake-shot-producer-audit.json'
    destination.write_text(json.dumps(report, indent=2))
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path, nargs='?', default=Path('local/nhl27-eboot/compatibility'))
    result = run(parser.parse_args().root)
    print(json.dumps({k: v for k, v in result.items() if k not in
                     ('source_evidence', 'source_mappings', 'target_default_gesture')}, indent=2))
