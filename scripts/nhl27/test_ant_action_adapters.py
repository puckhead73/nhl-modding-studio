import copy
import json
from pathlib import Path
import unittest

from ant_action_adapters import bindings, adapt
from ant_interfaces import BOOL, DEACTIVATE, CONDITION
from ant_link import link
from native_ant import Bank


class ActionAdapterTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        root = Path(__file__).resolve().parents[2] / 'local/nhl27-eboot/compatibility'
        path = root / 'deke-trial/input-adapter-audit.json'
        if not path.exists():
            raise unittest.SkipTest('Private action audit unavailable')
        cls.audit = json.loads(path.read_text())
        cls.state = Bank((root / 'ant/nhl24_160/state.bin').read_bytes())
        cls.launch = Bank((root / 'ant/nhl24_160/launch.bin').read_bytes())
        cls.mapping = bindings(cls.audit, cls.state, cls.launch)

    def test_all_eight_references_retain_interfaces_and_require_runtime_parity(self):
        refs = [dict(site=i*8, target_id=source, interface=interface)
                for i, (source, interface) in enumerate(
                    (source, interface) for source in self.mapping
                    for interface in (BOOL, DEACTIVATE, CONDITION))]
        fragment = dict(data=bytes(len(refs)*8), objects=[], symbolic_relocations=refs)
        before = copy.deepcopy(fragment)
        result = adapt(fragment, self.mapping)
        self.assertEqual(fragment, before)
        self.assertEqual(len(result['compatibility_requirements']), 8)
        for original, rewritten in zip(refs, result['symbolic_relocations']):
            self.assertEqual(rewritten['interface'], original['interface'])
            self.assertEqual(rewritten['target_id'], self.mapping[original['target_id']]['target_id'])
        with self.assertRaisesRegex(ValueError, 'unqualified source/target runtime semantics'):
            link(self.launch, [result])

    def test_mismatched_flags_hashes_or_interfaces_are_rejected(self):
        changed = copy.deepcopy(self.audit)
        changed['candidates'][0]['source_flags']['DeactivateAfterUpdateEnabled'] = True
        with self.assertRaisesRegex(ValueError, 'reset semantics'):
            bindings(changed, self.state, self.launch)
        changed = copy.deepcopy(self.audit)
        changed['target_bank_sha256'] = '0'*64
        with self.assertRaisesRegex(ValueError, 'hash mismatch'):
            bindings(changed, self.state, self.launch)
        with self.assertRaisesRegex(ValueError, 'Unqualified native interface'):
            adapt(dict(symbolic_relocations=[dict(site=0, target_id=next(iter(self.mapping)),
                                                   interface=0xdeadbeef)]), self.mapping)


if __name__ == '__main__':
    unittest.main()
