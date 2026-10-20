import copy
import json
from pathlib import Path
import struct
import unittest

from deke_gesture_audit import source_gesture
from deke_gesture_stage import compile_fragment, CONFIG, NEW_MAP, OLD_MAP, R3_MATCHER, FAKE_SHOT
from native_ant import Bank


class GestureReferenceTests(unittest.TestCase):
    def test_exported_matcher_selection_does_not_evaluate_partition_root(self):
        matcher = dict(Name='direct', Button=15, OutputTo=None)
        document = dict(selected_offset=168, instances=[
            dict(offset=16, type=['GestureAsset'], fields=dict(Steps=[], BufferSize=99,
                                                              CancelOnNotMatched=True)),
            dict(offset=168, type=['GamepadButtonsMatchAsset'], fields=matcher)])
        self.assertEqual(source_gesture(document), dict(Button=15, OutputTo=None))

    def test_native_fragment_uses_existing_matcher_and_source_mapping_order(self):
        root = Path(__file__).resolve().parents[2] / 'local/nhl27-eboot/compatibility'
        path = root / 'deke-trial/fake-shot-producer-audit.json'
        if not path.exists():
            self.skipTest('Private source audit unavailable')
        audit = json.loads(path.read_text())
        state = Bank((root / 'ant/nhl24_160/state.bin').read_bytes())
        launch = Bank((root / 'ant/nhl24_160/launch.bin').read_bytes())
        fragment = compile_fragment(state, launch, audit)
        self.assertEqual([o['id'] for o in fragment['objects']], [CONFIG, NEW_MAP])
        internal = {r['site']: r['destination'] for r in fragment['internal_relocations']}
        symbolic = {r['site']: r['target_id'] for r in fragment['symbolic_relocations']}
        self.assertEqual(struct.unpack_from('<Q', fragment['data'], 32)[0], 2)
        self.assertEqual([symbolic[internal[24]+8*i] for i in range(2)], [NEW_MAP, OLD_MAP])
        self.assertEqual(symbolic[internal[120]], R3_MATCHER)
        self.assertEqual(symbolic[88], FAKE_SHOT)
        self.assertEqual(symbolic[136], CONFIG)
        self.assertTrue(fragment['compatibility_requirements'])
        changed = copy.deepcopy(audit)
        next(m for m in changed['source_mappings'] if m['action'] == 171)['gestures'][0]['Button'] = 14
        with self.assertRaisesRegex(ValueError, 'matcher differs'):
            compile_fragment(state, launch, changed)


if __name__ == '__main__':
    unittest.main()
