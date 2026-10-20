import struct
import unittest
from ant_buffer_pool import repack
from native_ant import Bank
from test_native_ant import sample


class PoolTests(unittest.TestCase):
    def test_identity_keeps_payload_and_header(self):
        b=Bank(sample());out,r=repack(b,{})
        actual=Bank(out)
        self.assertEqual(actual.clip(123)[2],b.clip(123)[2])
        self.assertEqual(out[:b.header],b.data[:b.header])
        self.assertEqual(len(actual.blob),len(b.blob))

    def test_new_key_allocation_adds_relocation(self):
        b=Bank(sample());out,r=repack(b,{152:bytes(16),136:b'\x01\0\x02\0'})
        actual=Bank(out)
        self.assertEqual(r['added_relocations'],1)
        self.assertEqual(len(out),len(b.data)+16)
        at=actual.pointer(136)
        self.assertEqual(actual.blob[at:at+4],b'\x01\0\x02\0')

    def test_rejects_growth_beyond_pool(self):
        with self.assertRaises(ValueError):repack(Bank(sample()),{152:bytes(10000)})

    def test_rejects_unowned_site(self):
        with self.assertRaises(ValueError):repack(Bank(sample()),{33:bytes(4)})
