import pathlib
import struct
import sys
import unittest
import json
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[2]/'src/adapters'))
from frostbite_sections import decode_indices

class Indices(unittest.TestCase):
    def test_alignment_and_element_offsets(self):
        sections=[dict(vertexCount=3,vertex_bytes=18,startVertex=0,primitiveCount=1,startIndex=0),
                  dict(vertexCount=4,vertex_bytes=24,startVertex=18,primitiveCount=1,startIndex=3)]
        blob=b'\xff'*42+b'\0'*6+struct.pack('<6H',0,1,2,1,2,3)
        self.assertEqual(decode_indices(blob,sections),(48,[[[0,1,2]],[[1,2,3]]]))

    def test_bad_extent_and_index_refused(self):
        s=[dict(vertexCount=3,vertex_bytes=16,startVertex=0,primitiveCount=1,startIndex=0)]
        for b in (b'\0'*16,b'\0'*16+struct.pack('<3H',0,1,3)):
            with self.assertRaises(ValueError): decode_indices(b,s)

    def test_noncontiguous_layout_refused(self):
        with self.assertRaises(ValueError):
            decode_indices(bytes(64),[dict(vertexCount=3,vertex_bytes=16,startVertex=8,primitiveCount=1,startIndex=0)])

    def test_local_ovechkin_fixture_when_available(self):
        root=pathlib.Path(__file__).resolve().parents[2]
        geo=root/'local/assets/codex/nhl26-native-03/entry-133.bin'
        receipt=root/'local/runs/codex/nhl26-native-03/sections.json'
        if not geo.exists() or not receipt.exists(): self.skipTest('licensed local fixture absent; coverage unknown')
        sections=json.loads(receipt.read_text())
        base,triangles=decode_indices(geo.read_bytes(),[s['section'] for s in sections])
        self.assertEqual(base,259856)
        self.assertEqual(triangles,[s['triangles'] for s in sections])
