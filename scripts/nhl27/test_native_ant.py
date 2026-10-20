"""Synthetic memory images exercise native-bank bounds independently of game data."""
import struct
import unittest
from animation_dct import decode, envelope
from native_ant import Bank
from test_animation import resource


def sample():
    _, body = envelope(resource())
    blob = bytearray(176 + len(body) + 5)
    struct.pack_into('<I', blob, 72+16, 0x44435420)
    struct.pack_into('<I', blob, 72+88, len(body))
    blob[176:176+len(body)] = body
    blob[-5:] = b'test\0'
    # Two objects, two types, two relocations, no external references/init entries.
    outer = struct.pack('>9I', 84, 3, 2, 2, 0, 0, 0, 0, 0)
    outer += struct.pack('<4Q', 123, 0, 456, 0) + struct.pack('<2Q', 123, 456)
    header = struct.pack('<12Q', 6, 2, 8, 2, 4, 2, 16, 2, 24, 0, 0, len(blob))
    sizes = struct.pack('<2Q', 72, 104)
    hashes = struct.pack('<2I', 1556467357, 3842202299)
    relocations = struct.pack('<4Q', 32, 72, 72+80, 176)
    objects = struct.pack('<6Q', 0, 123, 0, 72, 456, 1)
    names = struct.pack('<2Q', len(blob)-5, 2**64-1)
    return outer+header+sizes+hashes+relocations+objects+names+blob


class NativeBankTests(unittest.TestCase):
    def test_locate_and_decode_control(self):
        bank = Bank(sample())
        root, anim, data = bank.clip(123)
        self.assertEqual(root['name'], 'test')
        self.assertIsNone(anim['name'])
        self.assertEqual(decode(data)[1], [[[0, 0, 0, 1]]]*8)

    def test_truncation_and_trailing_bytes(self):
        for data in [sample()[:-1], sample()+b'\0']:
            with self.assertRaises(ValueError): Bank(data)

    def test_invalid_type_index(self):
        bank = Bank(sample()); data = bytearray(bank.data)
        struct.pack_into('<Q', data, bank.sections['objects'][0]+16, 2)
        with self.assertRaises(ValueError): Bank(data)

    def test_outside_relocation(self):
        bank = Bank(sample()); data = bytearray(bank.data)
        struct.pack_into('<Q', data, bank.sections['relocations'][0]+8, len(bank.blob)+1)
        with self.assertRaises(ValueError): Bank(data)

    def test_duplicate_objects(self):
        bank = Bank(sample()); data = bytearray(bank.data)
        struct.pack_into('<Q', data, bank.sections['objects'][0]+32, 123)
        with self.assertRaises(ValueError): Bank(data)

    def test_wrong_codec(self):
        bank = Bank(sample()); data = bytearray(bank.data)
        struct.pack_into('<I', data, bank.sections['blob'][0]+88, 0)
        with self.assertRaises(ValueError): Bank(data).clip(123)

    def test_missing_relocation(self):
        with self.assertRaises(ValueError): Bank(sample()).pointer(17)


if __name__ == '__main__': unittest.main()
