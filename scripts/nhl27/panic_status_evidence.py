"""Qualify the deke-on-knees status mapping through its signal producer.

The certificate covers only availability comparisons with status 14/17, not
translation of the entire actor-state enum or full producer behavior.
"""
import json
from pathlib import Path

from deke_runtime_evidence import checked_image, SOURCE_HASH, TARGET_HASH, snapshot
from native_ant import Bank
from research import ElfImage, SelfImage


def qualify(root, source, target):
    source_sites = {
        0x389015: '498bbf380e0000',  # Serialized SignalDekeOnKneesEnter field.
        0x38902b: 'ff90d0000000',    # Native interface resolution in source.
        0x389034: '488981200e0000',  # Runtime pointer publication.
        0x480e6e7: '488b80200e0000',
        0x480e6ee: '4885c07403c60001',
        0x480e6f8: 'c783c000000011000000',
        0x455e811: '83bbc000000011',
        0x455e827: '83bbc00000000e',
    }
    target_sites = {
        0x403aca: '488bbb68120000',
        0x404fce: 'ff9088000000',
        0x404fd7: 'e900ebffff',     # Return to the runtime publication below.
        0x403adc: '48898170120000',
        0x2bf105c: '488b8070120000',
        0x2bf1063: '4885c07403c60001',
        0x2bf106d: 'c783ac00000012000000',
        0x27c0e07: '418b9dac00000083fb12',
        0x27c0e2a: '83fb0e',
    }
    for image, sites in ((source, source_sites), (target, target_sites)):
        for address, expected in sites.items():
            if image.read(address, len(bytes.fromhex(expected))).hex() != expected:
                raise ValueError(f'Unqualified status instruction at {address:#x}')
    descriptors = json.loads((root / 'source-type-descriptors.json').read_text())
    source_type = next(t for t in descriptors if t['name_candidates'] == ['AnimAssetRI_GSBoolAssets'])
    source_field = next(f for f in source_type['fields'] if f['offset'] == 0xe38)
    if source_field['name_candidates'] != ['SignalDekeOnKneesEnter']:
        raise ValueError('Source signal identity differs')
    state = Bank((root / 'ant/nhl24_160/state.bin').read_bytes())
    launch = Bank((root / 'ant/nhl24_160/launch.bin').read_bytes())
    gs = state.by_id[0x30b7d024f81ea1df]
    refs = [r for r in state.external if r[1] == gs['offset'] + 0x1268]
    if len(refs) != 1 or launch.by_id[refs[0][2]]['name'] != 'SignalDekeOnKneesEnter':
        raise ValueError('Native signal identity differs')
    return dict(scope='availability_status_predicates_only', installed=False,
        source_status=17, target_status=18, signal='SignalDekeOnKneesEnter',
        source_asset_field='0xe38', source_runtime_pointer='0xe20',
        target_asset_field='0x1268', target_runtime_pointer='0x1270',
        source_sites=[snapshot(source, a, len(bytes.fromhex(b))) for a, b in source_sites.items()],
        target_sites=[snapshot(target, a, len(bytes.fromhex(b))) for a, b in target_sites.items()],
        limitations=['No general enum translation', 'No whole-producer behavioral parity claim',
                    'Status 14 correspondence is limited to the paired availability consumers'])


def main():
    root = Path('local/nhl27-eboot/compatibility')
    source = checked_image(ElfImage, Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'), SOURCE_HASH)
    target = checked_image(SelfImage, Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'), TARGET_HASH)
    report = qualify(root, source, target)
    report.update(source_sha256=SOURCE_HASH, target_sha256=TARGET_HASH)
    out = root / 'deke-trial/panic-status-evidence.json'
    out.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(path=str(out), source_status=17, target_status=18, installed=False)))


if __name__ == '__main__':
    main()
