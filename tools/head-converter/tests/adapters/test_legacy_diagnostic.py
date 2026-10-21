"""Independent Legacy byte-layout and DDS wrapper checks."""
import struct
import unittest
from experiments.codex.legacy_diagnostic import layouts, dds


class LegacyDiagnosticTests(unittest.TestCase):
    def fixture(self):
        raw=bytearray(1024)
        raw[:7]=b'\x89RW4xb2'
        for off,val in ((0x44,512),(0x30,128),(0x20,3)):
            struct.pack_into('>I',raw,off,val)
        for i,e in enumerate(((256,0,28,0,0,0x20004),(0,0,32,0,0,0x10031),(320,0,44,0,0,0x20005))):
            struct.pack_into('>6I',raw,128+i*24,*e)
        raw[264]=1;raw[265]=32
        raw[276:284]=bytes.fromhex('001a236000000001')
        struct.pack_into('>H',raw,352,32);struct.pack_into('>I',raw,356,1)
        return raw

    def test_layout_resolves_position_field(self):
        layout=layouts(self.fixture())[0]
        self.assertEqual((layout['start'],layout['count'],layout['stride'],layout['position_offset']),(512,1,32,0))

    def test_wrong_declaration_refused(self):
        raw=self.fixture();raw[276]=255
        with self.assertRaises(AssertionError):layouts(raw)

    def test_short_payload_refused(self):
        raw=self.fixture();struct.pack_into('>I',raw,128+24+8,2)
        with self.assertRaises(AssertionError):layouts(raw)

    def test_dds_header_and_payload(self):
        data=dds(b'x'*8,4,4,b'DXT1')
        self.assertEqual(len(data),136)
        self.assertEqual(data[:4],b'DDS ')
        self.assertEqual(data[84:88],b'DXT1')
        self.assertEqual(struct.unpack_from('<II',data,12),(4,4))
        self.assertEqual(data[128:],b'x'*8)


if __name__=='__main__':unittest.main()
