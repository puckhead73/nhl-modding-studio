"""Extend the native PTMSkating node's activation arrays with the source signal.

Keep the target node's transitions and controller pointers. Inherited fixups
are tied to the exact original launch bank, including secondary interfaces.
"""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from native_ant import Bank

NODE = 0xf5a748790f0b7f3d
ACTIVE = 0xe0833ad3a430af00
NODE_ACTIVE = 0x9f2ed2b822171119
LAUNCH_SHA256 = '6e5c2b215b8f064ec6982a6fdf141eb577d483ea7678e66ab702f0cb39f94d14'


def compile_fragment(bank, source):
    digest = hashlib.sha256(bank.data).hexdigest()
    if digest != LAUNCH_SHA256:
        raise ValueError('PTM node template bank mismatch')
    node = bank.by_id[NODE]
    if (node['type_hash'], node['size']) != (0x184a86dd, 208):
        raise ValueError('Unexpected native PTM node layout')
    fields = source['instances'][0]['fields']
    if source['partition'] != '0f0b7f3d-4879-f5a7-0f0b-7f3d4879f5a7':
        raise ValueError('Unexpected source PTM node identity')
    expected = ['a430af00-3ad3-e083-a430-af003ad3e083',
                '22171119-d2b8-9f2e-2217-1119d2b89f2e']
    for name in ('ActiveBoolAssets', 'ActiveFloatAssets', 'ActiveDeactivationAssets'):
        if [v['external']['instance'] for v in fields[name]] != expected:
            raise ValueError('Source activation list changed')
    at = node['offset']
    data = bytearray(bank.blob[at:at+208])
    internal, symbolic, inherited = [], [], []
    for site, _ in bank.relocations:
        if at <= site < at+208 and site-at not in (40, 56, 72):
            inherited.append(dict(site=site-at, source_site=site))
    for _, site, _, _, _ in bank.external:
        if at <= site < at+208:
            raise ValueError('Unexpected external node field')
    for field, interface, delta in ((40, 0xbcc2c469, 64),
                                     (56, None, None), (72, 0x28185be6, 72)):
        original = bank.pointer(at+field)
        if struct.unpack_from('<Q', bank.blob, at+field+8)[0] != 1:
            raise ValueError('Native activation list changed')
        expected_pointer = None if delta is None else bank.by_id[ACTIVE]['offset']+delta
        if bank.reloc.get(original) != expected_pointer or any(bank.blob[original:original+8]):
            raise ValueError('Native activation interface changed')
        start = len(data)
        data.extend(bytes(16))
        internal.append(dict(site=field, destination=start))
        struct.pack_into('<Q', data, field+8, 2)
        # Both BoolAssets lack the float interface; preserve null array entries.
        if interface is not None:
            symbolic.extend(dict(site=start+8*i, target_id=ident, interface=interface)
                            for i, ident in enumerate((ACTIVE, NODE_ACTIVE)))
    return dict(data=bytes(data), template_bank_sha256=digest, target_bank='launch',
        objects=[dict(id=NODE, offset=0, size=208, type_hash=0x184a86dd, name=node['name'])],
        internal_relocations=internal, symbolic_relocations=symbolic,
        inherited_relocations=inherited, replace_ids=[NODE],
        compatibility_requirements=[dict(kind='panic_node_lifecycle_required',
            reason='Qualify node activation/deactivation consumer semantics before installation')])


def run(root):
    trial = root/'deke-trial'
    bank = Bank((root/'ant/nhl24_160/launch.bin').read_bytes())
    source = json.loads((trial/'panic-bindings/c827ee683e2173877b33d00157f71f666875ef05.json').read_text())
    result = compile_fragment(bank, source)
    data = result.pop('data')
    path = trial/'panic-node-fragment.bin'
    path.write_bytes(data)
    result.update(path=str(path), sha256=hashlib.sha256(data).hexdigest(), installed=False)
    (trial/'panic-node-fragment.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(dict(bytes=len(data), inherited=len(result['inherited_relocations']), installed=False)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    run(parser.parse_args().root)
