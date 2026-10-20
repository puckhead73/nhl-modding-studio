"""Check map selection and dependency provenance without proprietary inputs."""
import unittest
from deke_new_moves import compatible_map, incoming, native_clip_fragment
from test_dct_reorder import two_rotations


def fields():
    return dict(instances=[dict(type=['ClipControllerAsset'], fields=dict(
        TagCollectionSetAsset=None, RigRef=None, DofCodecAnim=None, TimeScale=1.0,
        **{'0x314b9906': 30.0}, Distance=0.0, TickOffset=0.0, DeltaTrajectory=False,
        DeltaTrajectoryRotationMask=False, OnlyActivateModifiedDofs=False,
        IsLooping=False, PlaybackMode=0, DofMode=0)),
        dict(type=['DctAnimationAsset'], fields=dict(CodecType=0x44435420, AnimId=1,
            TrimOffset=0.0, TrimmedDuration=15.0, EndFrame=15, BaseFlags=0,
            NumVec3=0, NumFloat=0, NumCompressedQuat=2, NumCompressedVec3=0,
            NumCompressedVecFloat=0, Cycle=False))])


class NewMoveTests(unittest.TestCase):
    def test_exact_order_preferred_over_lower_id(self):
        ident, order, count = compatible_map([11, 12, 21, 31], (2, 1, 1),
            {1: [12, 11, 21, 31], 9: [11, 12, 21, 31]})
        self.assertEqual((ident, order, count), (9, [0, 1, 2, 3], 2))

    def test_missing_dof_never_silently_dropped(self):
        with self.assertRaises(ValueError):
            compatible_map([11, 12, 21, 31], (2, 1, 1), {1: [11, 12, 21]})

    def test_scalar_lane_swap_rejected(self):
        with self.assertRaises(ValueError):
            compatible_map([11, 21, 31, 32], (1, 1, 1), {1: [11, 21, 32, 31]})

    def test_incoming_preserves_distinct_layer_variants(self):
        nodes = [dict(name='same', asset=dict(sha1=sha, span=dict(layer=layer)),
            fixup=dict(imports=[dict(partition='clip'), dict(partition='clip')]))
            for sha, layer in [('base', 0), ('patch', 1)]]
        self.assertEqual(set(incoming(nodes)['clip']), {'base', 'patch'})

    def test_native_fragment_preserves_required_tag_dependency(self):
        f = fields()
        f['instances'][0]['fields']['TagCollectionSetAsset'] = dict(internal_offset=840)
        _, links = native_clip_fragment(two_rotations(), f, 42, 100, 200)
        self.assertEqual(links['unresolved_dependencies'][0]['source_reference'],
                         dict(internal_offset=840))
        self.assertEqual(links['unresolved_dependencies'][0]['site'], 16)
        self.assertIsNone(links['objects'][1]['id'])

    def test_native_fragment_rejects_mismatched_compression_metadata(self):
        f = fields()
        f['instances'][1]['fields']['NumCompressedQuat'] = 3
        with self.assertRaises(ValueError):
            native_clip_fragment(two_rotations(), f, 42, 100, 200)


if __name__ == '__main__':
    unittest.main()
