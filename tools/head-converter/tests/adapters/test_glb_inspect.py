"""Synthetic format fixtures exercise byte bounds, normalization and chunk preservation."""
import json
from pathlib import Path
import struct
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'src/adapters'))
from glb_inspect import Glb, signature

def fixture(doc, blob=b''):
    j=json.dumps(doc).encode()
    j += b' ' * (-len(j)%4)
    blob += b'\0' * (-len(blob)%4)
    chunks=struct.pack('<I4s',len(j),b'JSON')+j+struct.pack('<I4s',len(blob),b'BIN\0')+blob
    chunks+=struct.pack('<I4s',4,b'TEST')+b'keep'
    return struct.pack('<4sII',b'glTF',2,len(chunks)+12)+chunks

class Checks(unittest.TestCase):
    def test_strided_normalized_and_unknown_chunk_roundtrip(self):
        doc={'buffers':[{'byteLength':8}], 'bufferViews':[{'buffer':0,'byteLength':8,'byteStride':4}],
             'accessors':[{'bufferView':0,'componentType':5121,'type':'VEC2','count':2,'normalized':True}]}
        raw=fixture(doc,bytes([0,255,77,88,255,0,77,88]))
        g=Glb(raw)
        self.assertEqual(g.accessor(0),[(0,1),(1,0)])
        self.assertEqual(g.serialize(),raw)

    def test_access_cannot_spill_view(self):
        doc={'buffers':[{'byteLength':8}], 'bufferViews':[{'byteLength':2}],
             'accessors':[{'bufferView':0,'componentType':5121,'type':'VEC2','count':2}]}
        with self.assertRaises(ValueError):
            Glb(fixture(doc,b'12345678')).accessor(0)

    def test_truncated_chunk_refused(self):
        with self.assertRaises(ValueError):
            Glb(fixture({})[:-1])

    def test_equal_counts_do_not_mean_equal_topology(self):
        self.assertNotEqual(signature([0,1,2,0,2,3]),signature([0,1,3,1,2,3]))

    def test_sparse_refused(self):
        with self.assertRaises(ValueError):
            Glb(fixture({'accessors':[{'sparse':{}}]})).accessor(0)

if __name__=='__main__':
    unittest.main()
