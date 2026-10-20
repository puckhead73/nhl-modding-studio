"""Check candidate ActionInputBool adapters against target Boolean reset semantics.

This is an audit, not an alias installer. Matching names/flags do not establish
that the executable produces the same signal on the same input/frame.
"""
import argparse
import hashlib
import json
from pathlib import Path
from native_ant import Bank

# Explicit semantic candidates; do not use fuzzy matching or enum ordinals from
# a different title as executable offsets.
CANDIDATES = {
    224: 'SignalSpin360CW', 223: 'SignalSpin360CCW',
    221: 'SignalShotRelease', 172: 'FakeShotDefault',
    144: 'bgs_psk_lacrosse_prepare_forehand', 219: 'SignalGlideFwdDeke',
    218: 'SignalEnterLoosePuckDeking', 139: 'bgs_psk_forward_betweenLegsDeke',
}
FLAGS = ('DeactivateAfterUpdateEnabled', 'DeactivateAfterTransitionEnabled',
         'DeactivateAfterUpdateValue')


def run(root):
    trial = root / 'deke-trial'
    bank_path = root / 'ant/nhl24_160/launch.bin'
    bank = Bank(bank_path.read_bytes())
    state = Bank((root / 'ant/nhl24_160/state.bin').read_bytes())
    gs = state.by_id[0x30b7d024f81ea1df]
    if gs['type_hash'] != 0x16a79c0c or gs['name'] != 'AnimAssetRI_GSBoolAssets':
        raise ValueError('Unexpected NHL24 game-state Boolean binding record')
    rows = []
    for path in sorted((trial / 'integration-fields').glob('*.json')):
        source = json.loads(path.read_text())
        for instance in source.get('instances', []):
            if 'ActionInputBoolAsset' not in instance['type']:
                continue
            f = instance['fields']
            name = CANDIDATES.get(f['ActionInput'])
            matches = [(ident, o) for ident, o in bank.by_id.items() if o['name'] == name]
            if name is None or len(matches) != 1:
                raise ValueError('No unique explicit target signal candidate')
            ident, obj = matches[0]
            if (obj['type_hash'], obj['size']) != (870677462, 104):
                raise ValueError('Candidate is not the qualified target BoolAsset layout')
            # NHL24 serializer 0x8e8a40 writes these three bytes at +96..98.
            flags = {k: bool(bank.blob[obj['offset'] + 96 + i]) for i, k in enumerate(FLAGS)}
            # The existing native record supplies the exact eight bindings.
            # Getter 0x3ffbf0 resolves each asset field through virtual +0x88
            # into a per-evaluation-context pointer at field_offset + 8.
            bindings = [(site - gs['offset'], typ) for _, site, external_id, typ, _ in state.external
                        if external_id == ident and gs['offset'] <= site < gs['offset'] + gs['size']]
            if len(bindings) != 1 or bindings[0][1] != 0x7a327d7d:
                raise ValueError('No unique native game-state Boolean binding')
            field, interface = bindings[0]
            rows.append(dict(source_partition=source['partition'], action=f['ActionInput'],
                source_name=f['Name'], target_id=hex(ident), target_name=name,
                target_binding=dict(asset_id='0x30b7d024f81ea1df', getter='0x3ffbf0',
                    asset_field_offset=field, runtime_pointer_offset=field + 8,
                    interface=hex(interface), runtime_record_bytes=0x1580,
                    qualification='native_asset_reference_and_getter_layout_only'),
                source_flags={k: f[k] for k in FLAGS}, target_flags=flags,
                flags_match=all(f[k] == flags[k] for k in FLAGS),
                qualification='candidate_only_input_producer_and_frame_timing_unverified'))
    report = dict(target_bank_sha256=hashlib.sha256(bank.data).hexdigest(),
        target_state_bank_sha256=hashlib.sha256(state.data).hexdigest(),
        layout_evidence='NHL24 1.60 BoolAsset serializer 0x8e8a40',
        adapters_installed=False, candidates=rows)
    (trial / 'input-adapter-audit.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(dict(candidates=len(rows), reset_semantics_match=sum(r['flags_match'] for r in rows),
                          installed=False), indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    run(parser.parse_args().root)
