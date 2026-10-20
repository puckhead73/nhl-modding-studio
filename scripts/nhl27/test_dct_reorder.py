"""Independent synthetic coefficient streams for lossless channel conversion."""
import struct
import unittest
from animation_dct import decode
from dct_reorder import permutation, reorder


def two_rotations():
    # Sixteen samples cross a block boundary; the second rotation is constant.
    body=struct.pack('<5H2B',16,2,0,0,1000,0,15)+bytes([0x20,0])
    body+=struct.pack('<4h2H',0,0,0,4000,0x1000,0x8000)
    body+=struct.pack('<4h',0,0,4000,0)
    # First block X harmonic = 20, second block DC delta = 0, harmonic = -10.
    bits=20 | (246<<9)
    body+=bits.to_bytes(3,'little')+bytes(29)
    data=struct.pack('<5I',0,32,0,len(body),32)+bytes(12)+body
    return data+bytes((-len(data))%16)+struct.pack('<II',4,16)


class ReorderTests(unittest.TestCase):
    def test_identity_is_byte_exact(self):
        data=two_rotations();self.assertEqual(reorder(data,[0,1]),data)

    def test_swap_and_inverse_across_blocks(self):
        data=two_rotations();out=reorder(data,[1,0])
        self.assertEqual(decode(out)[1],[list(reversed(f)) for f in decode(data)[1]])
        self.assertEqual(reorder(out,[1,0]),data)

    def test_bad_permutation(self):
        with self.assertRaises(ValueError):reorder(two_rotations(),[0,0])

    def test_dof_permutation(self):
        self.assertEqual(permutation([11,12,21,31,32],[12,11,21,31,32],2,1,1),[1,0,2,3])

    def test_scalar_repacking_and_category_crossing_rejected(self):
        for target in [[12,11,21,32,31],[21,11,12,31,32],[11,12,21,31,33]]:
            with self.assertRaises(ValueError):permutation([11,12,21,31,32],target,2,1,1)


if __name__=='__main__':unittest.main()
