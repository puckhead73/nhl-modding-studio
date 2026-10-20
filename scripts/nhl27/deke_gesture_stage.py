"""Stage the source R3 fake-shot mapping in native NHL24 records.

The new mapping precedes LB+R3, matching the source configuration. Existing
device and matcher objects are reused only after exact identity/parameter
checks. Scheduler parity remains an explicit installation requirement.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

from deke_gesture_audit import scalar_parameters
from deke_native_audit import native_id
from native_ant import Bank

CONFIG = 0x125a4414a231f1bf
OLD_MAP = 0x8f23521674141119
NEW_MAP = 0xc694f89799aa0dcf
R3_MATCHER = 0x26b5b333cfaff536
FAKE_SHOT = 0x4da454479d17303f
DEVICE = 0x3b14b222f8865e0f


def compile_fragment(state, launch, audit):
    if hashlib.sha256(state.data).hexdigest() != audit['target_state_sha256']:
        raise ValueError('Gesture template bank hash mismatch')
    if hashlib.sha256(launch.data).hexdigest() != '6e5c2b215b8f064ec6982a6fdf141eb577d483ea7678e66ab702f0cb39f94d14':
        raise ValueError('Gesture launch bank hash mismatch')
    evidence = audit['source_evidence']
    source_config = evidence['a231f1bf-4414-125a-a231-f1bf4414125a']['fields']['instances'][0]['fields']
    source_map = evidence['99aa0dcf-f897-c694-99aa-0dcff897c694']['fields']['instances'][0]['fields']
    wanted = scalar_parameters(next(m for m in audit['source_mappings'] if m['action'] == 171)['gestures'][0])
    native_matcher = state.by_id[R3_MATCHER]
    if (native_matcher['type_hash'], native_matcher['size']) != (1517677578, 64):
        raise ValueError('R3 exported identity is not the native matcher')
    matcher = native_matcher['offset']
    actual = dict(zip(('Button', 'MatchType', 'MinValue', 'MaxValue', 'Velocity',
        'OutputValue', 'MatchValidState'), struct.unpack_from('<iifffi?', state.blob, matcher+32)))
    actual.update(BufferSize=struct.unpack_from('<f', state.blob, matcher+8)[0],
                  CancelOnNotMatched=bool(state.blob[matcher+12]))
    if wanted != actual:
        raise ValueError('Source R3 matcher differs from native R3 matcher')
    source_ref = source_map['Gesture'][0]['external']
    document = evidence[source_ref['partition']]['fields']
    selected_offset = document['exported_instance_guids'][source_ref['instance']]
    source_matcher = next(n for n in document['instances'] if n['offset'] == selected_offset)
    if source_matcher['type'] != ['GamepadButtonsMatchAsset'] or native_id(source_ref['instance']) != R3_MATCHER:
        raise ValueError('Mapping must reference the exported matcher, not the partition root')
    if source_matcher['fields']['OutputTo'] is not None or any(state.blob[matcher+24:matcher+32]):
        raise ValueError('Unqualified matcher output binding')
    if native_id(source_matcher['fields']['GamepadButtons']['external']['instance']) != 0x9fa44a43cb57cf5a:
        raise ValueError('Source matcher gamepad identity differs')
    if source_map['Enabled'] is not None or source_map['DeviceList'] != [dict(external=dict(
            partition='f8865e0f-b222-3b14-f886-5e0fb2223b14', instance='f8865e0f-b222-3b14-f886-5e0fb2223b14'))]:
        raise ValueError('Source gesture mapping device/enable difference')
    if [native_id(x['external']['instance']) for x in source_config['0x7c6bcd3']] != [NEW_MAP, OLD_MAP]:
        raise ValueError('Source gesture mapping order changed')
    signal = launch.by_id[FAKE_SHOT]
    if signal['type_hash'] != 0x33e57bd6 or launch.blob[signal['offset']+96:signal['offset']+99] != b'\0\1\0':
        raise ValueError('Native FakeShot signal/reset semantics differ')
    by_offset = {o['offset']: ident for ident, o in state.by_id.items()}
    external = {site: (ident, typ) for _, site, ident, typ, _ in state.external}

    def original_reference(site):
        if site in state.reloc:
            return dict(target_id=by_offset[state.pointer(site)])
        ident, typ = external[site]
        return dict(target_id=ident, interface=typ)

    config = state.by_id[CONFIG]
    children_base = state.pointer(config['offset'] + 8)
    children = [original_reference(children_base + 8*i) for i in range(2)]
    if [r['target_id'] for r in children] != [native_id(x['external']['instance']) for x in source_config['0xc185b7d2']]:
        raise ValueError('Source/native gesture child configuration differs')
    if source_config['0x107b11fb'] is not None or source_config['0xc5a006ad'] is not None or source_config['0xaa6e6638'] or source_config['0xf8ea2935']:
        raise ValueError('Source config has unsupported non-default controls')
    if any(state.blob[config['offset']+40:config['offset']+80]):
        raise ValueError('Native config has non-default controls')
    if external.get(matcher+16) != (0x9fa44a43cb57cf5a, 0x7f321f9d):
        raise ValueError('Native matcher uses a different gamepad asset')
    occupied = set(state.by_id) | set(launch.by_id)
    if NEW_MAP in occupied:
        raise ValueError('Source mapping identity already exists')
    data = bytearray(144)
    data[:80] = state.blob[config['offset']:config['offset']+80]
    original_map = state.by_id[OLD_MAP]
    data[80:144] = state.blob[original_map['offset']:original_map['offset']+64]
    internal, symbolic = [], []

    def array(site, entries):
        at = len(data)
        data.extend(bytes(8*len(entries)))
        internal.append(dict(site=site, destination=at))
        struct.pack_into('<Q', data, site+8, len(entries))
        symbolic.extend(dict(site=at+8*i, **ref) for i, ref in enumerate(entries))

    array(8, children)
    array(24, [dict(target_id=NEW_MAP), dict(target_id=OLD_MAP)])
    symbolic.append(dict(site=88, target_id=FAKE_SHOT, interface=0x0aeca644))
    array(104, [dict(target_id=DEVICE, interface=0xe47c8a0c)])
    array(120, [dict(target_id=R3_MATCHER)])
    symbolic.append(dict(site=136, target_id=CONFIG))
    objects = [dict(id=ident, offset=at, size=size, type_hash=typ, name=name)
        for ident, at, size, typ, name in (
            (CONFIG, 0, 80, 0xe1d14d4e, config['name']),
            (NEW_MAP, 80, 64, 0x86fc1dac, source_map['Name']))]
    return dict(data=bytes(data), objects=objects, internal_relocations=internal,
        symbolic_relocations=symbolic, replace_ids=[CONFIG],
        compatibility_requirements=[dict(type='GestureToSignalMappingAsset',
            field='Signal', reason='Action 171 native producer and gesture scheduling parity must be qualified')])


def run(root):
    trial = root / 'deke-trial'
    state = Bank((root / 'ant/nhl24_160/state.bin').read_bytes())
    launch = Bank((root / 'ant/nhl24_160/launch.bin').read_bytes())
    audit = json.loads((trial / 'fake-shot-producer-audit.json').read_text())
    result = compile_fragment(state, launch, audit)
    data = result.pop('data')
    path = trial / 'fake-shot-gesture-fragment.bin'
    path.write_bytes(data)
    result.update(path=str(path), sha256=hashlib.sha256(data).hexdigest(), installed=False)
    (trial / 'fake-shot-gesture-fragment.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(dict(objects=len(result['objects']), bytes=len(data), installed=False)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path, nargs='?', default=Path('local/nhl27-eboot/compatibility'))
    run(parser.parse_args().root)
