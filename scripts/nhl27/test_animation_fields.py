import struct
import unittest
from animation_fields import Fields


class PointerTests(unittest.TestCase):
    def reader(self,value):
        f=Fields.__new__(Fields);f.data=bytes(64)+struct.pack('<Q',value)+bytes(32)
        f.end=len(f.data);f.nodes={48:{},80:{}};f.fixup={'imports':[{'partition':'p','instance':'i'}]}
        return f

    def test_zero_extended_negative_relative_pointer(self):
        self.assertEqual(self.reader(0xfffffff0).pointer(64),{'internal_offset':16})

    def test_positive_relative_and_external(self):
        self.assertEqual(self.reader(16).pointer(64),{'internal_offset':48})
        self.assertEqual(self.reader(1).pointer(64),{'external':{'partition':'p','instance':'i'}})

    def test_invalid_reference_rejected(self):
        for v in [18,3,0xffffffff00000000]:
            with self.assertRaises(ValueError):self.reader(v).pointer(64)


if __name__=='__main__':unittest.main()
