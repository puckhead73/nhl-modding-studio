import struct
import unittest
from ant_tags import allocate_id, compile_tree


def sample():
    base = dict(Name='', StartLinkOffset=3.5, LinkFlags=32,
                StartLinkUpdates=2, EndLinkOffset=11.25)
    return dict(partition='00000001-0002-0003-0004-000000000005', instances=[
        dict(offset=16, type=['TagCollectionSetAsset'], fields=dict(Name='',
            TagCollections=[dict(internal_offset=40)])),
        dict(offset=40, type=['DefaultTagCollectionAsset'], fields=dict(Name='',
            Tags=[dict(internal_offset=80), dict(internal_offset=144)], HasExitTags=True)),
        dict(offset=80, type=['BoolGameStateTagAsset'], fields=dict(**base,
            TagEnable=None, GameStateValue=dict(external=dict(instance='00000006-0007-0008-0009-00000000000a')),
            ResetOnLeaveMode=1, Mode=2, Value=True)),
        dict(offset=144, type=['PTMKeepCurrentAnimBiasTag'], fields=dict(**base, Bias=50.0))])


class NativeTagTests(unittest.TestCase):
    def test_shared_monitored_tag_is_one_object_with_two_references(self):
        result = compile_tree(sample(), dict(internal_offset=16), set())
        self.assertEqual(len(result['objects']), 4)
        bool_obj = next(o for o in result['objects'] if o['type_hash'] == 0x4a11ba30)
        self.assertEqual(sum(r['destination'] == bool_obj['offset'] for r in result['internal_relocations']), 2)
        coll = next(o for o in result['objects'] if o['type_hash'] == 0x3d4a3aec)
        self.assertEqual(struct.unpack_from('<Q', result['data'], coll['offset'] + 16)[0], 2)
        self.assertEqual(struct.unpack_from('<Q', result['data'], coll['offset'] + 32)[0], 1)

    def test_base_and_derived_tag_fields_preserve_types(self):
        result = compile_tree(sample(), dict(internal_offset=16), set())
        obj = next(o for o in result['objects'] if o['type_hash'] == 0x4a11ba30)
        at, data = obj['offset'], result['data']
        self.assertEqual(struct.unpack_from('<fHB', data, at + 8), (3.5, 32, 2))
        self.assertEqual(struct.unpack_from('<f', data, at + 16)[0], 11.25)
        self.assertEqual(struct.unpack_from('<I', data, at + 40)[0], 1)
        self.assertEqual(struct.unpack_from('<I?', data, at + 48), (2, True))
        self.assertEqual(result['symbolic_relocations'], [dict(site=at + 32, target_id=0x0008000700000006)])

    def test_unknown_tag_is_never_discarded(self):
        source = sample()
        source['instances'][-1]['type'] = ['UnknownFutureTag']
        with self.assertRaisesRegex(ValueError, 'Unsupported native tag'):
            compile_tree(source, dict(internal_offset=16), set())

    def test_identity_collisions_are_resolved_deterministically(self):
        occupied = set()
        first = allocate_id('partition', 80, occupied)
        second = allocate_id('partition', 80, occupied)
        self.assertNotEqual(first, second)
        self.assertEqual(second, allocate_id('partition', 80, {first}))


if __name__ == '__main__':
    unittest.main()
