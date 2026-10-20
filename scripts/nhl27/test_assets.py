"""Synthetic failure and decoding checks for the research-only asset readers."""
import struct
import unittest
from asset_manifest import spans, manifest
from dbobject import Reader


class AssetTests(unittest.TestCase):
    def test_identifier_and_reuse(self):
        # One full identifier followed by an inherited identifier.
        b=bytearray(62)
        struct.pack_into('>III',b,8,60,2,36)
        struct.pack_into('>QIIII',b,36,0x0000f89501b40001,100,20,200,30)
        b[60:]=bytes([0x84,0])
        a=spans(b)
        self.assertEqual(a[0],dict(layer=0,persistent_index=0xf89501b4,cas=1,offset=100,size=20))
        self.assertEqual(a[1]['offset'],200)
        b[60]=0x80
        self.assertEqual(spans(b),a)
        b[60]=0x81
        with self.assertRaises(ValueError):spans(b)

    def test_manifest_names(self):
        b=struct.pack('<8I',0x9d798ed6,1,1,0,0,60,0,0)+bytes(20)+struct.pack('<II',0,123)+b'test\0'
        data=struct.pack('>I',len(b))+b
        self.assertEqual(manifest(data)[0]['name'],'test')
        with self.assertRaises(ValueError):manifest(data[:-1])
        with self.assertRaises(ValueError):manifest(struct.pack('>I',len(b))+bytes(4)+b[4:])

    def test_dbobject_container_and_bounds(self):
        # Anonymous object containing an integer named x and a terminator.
        body=b'\x08x\0'+struct.pack('<i',-12)+b'\0'
        self.assertEqual(Reader(b'\x82'+bytes([len(body)])+body).entry(),('',{'x':-12}))
        with self.assertRaises(ValueError):Reader(b'\x87\x03a').entry()
        with self.assertRaises(ValueError):Reader(b'\xff'*5).length()
        with self.assertRaises(ValueError):Reader(b'\x86\x02').entry()


if __name__=='__main__':unittest.main()
