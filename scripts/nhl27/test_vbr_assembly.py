import unittest
from vbr_assembly import key_times


class KeyTimeTests(unittest.TestCase):
    def test_sparse_keys_and_two_byte_runs(self):
        self.assertEqual(key_times([1,1,1,1,1,2,2,1,1,1,1,1,1,2,1],9,17),[0,2,4,7,8,10,12,14,17])
        self.assertEqual(key_times([1,0x81,0,1],2,257),[0,257])

    def test_rejects_incomplete_and_wrong_coverage(self):
        with self.assertRaises(ValueError):key_times([128],1,0)
        with self.assertRaises(ValueError):key_times([2],3,1)
        with self.assertRaises(ValueError):key_times([2],2,2)
