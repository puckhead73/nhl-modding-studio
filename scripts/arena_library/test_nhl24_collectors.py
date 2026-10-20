import struct
import unittest
from nhl24_collectors import directory, toc_chunks, collector_manifest_guid


class CollectorTests(unittest.TestCase):
    def manifest(self, name=b'folder/file.db\0'):
        b = bytearray(120 + len(name))
        struct.pack_into('<I', b, 12, 1)
        struct.pack_into('<Q', b, 16, 64)
        struct.pack_into('<5Q', b, 64, 120, 0, 20, 8, 12)
        b[104:120] = bytes(range(16))
        b[120:] = name
        return b

    def test_manifest_batch_offsets_and_guid(self):
        a = directory(self.manifest())[0]
        self.assertEqual((a['offset'], a['size'], a['compressed_end']), (8, 12, 20))
        self.assertEqual(a['chunk_guid'], bytes(range(16)).hex())

    def test_manifest_rejects_truncation_and_unsafe_names(self):
        for b in (self.manifest()[:119], self.manifest(b'../game.db\0'), self.manifest(b'/game.db\0'), self.manifest(b'no-terminator')):
            with self.assertRaises(ValueError):
                directory(b)

    def test_manifest_rejects_bad_pointer_and_range(self):
        for offset, value in [(64, 8), (72, 21), (96, 2**32)]:
            b = self.manifest()
            struct.pack_into('<Q', b, offset, value)
            with self.assertRaises(ValueError):
                directory(b)

    def test_toc_guid_byte_order_and_layer(self):
        b = bytearray(92)
        w = [0] * 15
        w[0], w[4], w[5], w[9], w[10] = 60, 60, 1, 80, 3
        struct.pack_into('>15I', b, 0, *w)
        b[60:76] = bytes(range(16))
        struct.pack_into('>4I', b, 76, 0x01000000, 0x010203, 200, 50)
        a = toc_chunks(b)[bytes(range(16))[::-1].hex()]
        self.assertEqual(a, dict(layer=1, catalog=2, cas=3, offset=200, size=50))
        struct.pack_into('>I', b, 76, 0x01000001)
        with self.assertRaises(ValueError):
            toc_chunks(b)

    def test_collector_rejects_unknown_schema(self):
        with self.assertRaises(ValueError):
            collector_manifest_guid(b'not RIFF')


if __name__ == '__main__':
    unittest.main()
