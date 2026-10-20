"""Regression checks for condition interface pointers and observed bank casts."""
import bisect
from collections import Counter
from pathlib import Path
import struct
import unittest

from ant_interfaces import BOOL, DEACTIVATE, BUFFERED, CONDITION, adjustment
from ant_tags import compile_tree
from native_ant import Bank


class InterfaceTests(unittest.TestCase):
    def test_transition_retains_distinct_symbolic_interfaces(self):
        ref = {'external': {'instance': '00000001-0002-0003-0004-000000000005'}}
        fields = dict(Name='', From=None, BranchWindowType=None, BranchInWindowType=None,
            DestinationPhaseSignal=None, Blend=None, TagCollectionSetAsset=None,
            DestinationPhase=.25, DestinationTickOffset=2., BranchAtEndOffset=-1,
            PhaseMode=1, Flags=0, TransitionConditions=[dict(BoolAsset=ref,
            DeactivateAsset=ref, BufferedAsset=ref, ConditionAsset=ref, Required=True)],
            **{'0x2b6786f7': None})
        tree = dict(partition='test', instances=[dict(offset=16, type=['TransitionAsset'], fields=fields)])
        result = compile_tree(tree, {'internal_offset': 16}, set())
        self.assertEqual([r['interface'] for r in result['symbolic_relocations']],
                         [BOOL, DEACTIVATE, BUFFERED, CONDITION])
        start = result['internal_relocations'][0]['destination']
        self.assertEqual(struct.unpack_from('<?', result['data'], start + 32), (True,))
        self.assertEqual(struct.unpack_from('<i', result['data'], 88), (-1,))

    def test_unqualified_interfaces_do_not_silently_use_object_base(self):
        with self.assertRaisesRegex(ValueError, 'Unqualified native interface'):
            adjustment(0x12345678, BOOL)
        self.assertIsNone(adjustment(0x33e57bd6, BUFFERED))

    def test_original_banks_match_every_qualified_transition_cast(self):
        root = Path(__file__).resolve().parents[2] / 'local/nhl27-eboot/compatibility/ant/nhl24_160'
        if not all((root / (n + '.bin')).exists() for n in ('state', 'launch')):
            self.skipTest('Private original native-bank fixtures are unavailable')
        observed = Counter()
        for name in ('state', 'launch'):
            bank = Bank((root / (name + '.bin')).read_bytes())
            objects = sorted(bank.by_id.values(), key=lambda o: o['offset'])
            starts = [o['offset'] for o in objects]
            for obj in objects:
                if obj['type_hash'] != 0x8b7dc2e8:
                    continue
                count = struct.unpack_from('<Q', bank.blob, obj['offset'] + 64)[0]
                if not count:
                    continue
                start = bank.pointer(obj['offset'] + 56)
                for i in range(count):
                    for slot, interface in enumerate((BOOL, DEACTIVATE, BUFFERED, CONDITION)):
                        dst = bank.reloc.get(start + i * 40 + slot * 8)
                        if dst is None:
                            continue
                        index = bisect.bisect_right(starts, dst) - 1
                        self.assertGreaterEqual(index, 0)
                        target = objects[index]
                        delta = dst - target['offset']
                        self.assertLess(delta, target['size'])
                        self.assertEqual(adjustment(target['type_hash'], interface), delta)
                        observed[(target['type_hash'], interface)] += 1
        self.assertGreater(observed[(0x33e57bd6, BOOL)], 900)
        self.assertGreater(observed[(0x33e57bd6, DEACTIVATE)], 900)


if __name__ == '__main__':
    unittest.main()
