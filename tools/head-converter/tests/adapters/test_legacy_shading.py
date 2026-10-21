import unittest,struct,copy
import numpy as np
from src.adapters.legacy_shading import pack,unpack,regenerate

class ShadingTests(unittest.TestCase):
    def fixture(self):
        raw=bytearray([85]*112)
        for i in range(3):
            struct.pack_into('>I',raw,8+i*32+8,pack([0,0,1],0x80000000))
            struct.pack_into('>I',raw,8+i*32+12,pack([1,0,0],0xc0000000))
        layout=dict(start=8,count=3,stride=32,position_offset=0,attributes=[(8,'002a218700030003'),(12,'002a218700060015')])
        original=[[0,0,0],[1,0,0],[0,1,0]]
        sub=dict(positions=[[0,0,0],[1,0,1],[0,1,0]],uv=[[0,0],[1,0],[0,1]],triangles=[[0,1,2]])
        return bytes(raw),layout,sub,original
    def test_signed_and_opaque_bits(self):
        for top in range(4):
            w=pack([-.7,.2,.6],top<<30)
            self.assertEqual(w>>30,top);np.testing.assert_allclose(unpack(w),[-.7,.2,.6],atol=1/1022)
        with self.assertRaises(ValueError):pack([float('nan'),0,0],0)
    def test_both_native_tags_and_only_shading(self):
        raw,layout,sub,original=self.fixture();out,_=regenerate(raw,layout,sub,original)
        allowed={8+i*32+j for i in range(3) for j in range(8,16)}
        self.assertTrue(all(a==b for i,(a,b) in enumerate(zip(raw,out)) if i not in allowed))
        self.assertNotEqual(out[20:24],raw[20:24])
    def test_unknown_or_overlapping_declaration_refused(self):
        for attrs in [[(8,'002a218700030003')],[(0,'002a218700030003'),(12,'002a218700060015')]]:
            raw,layout,sub,original=self.fixture();layout['attributes']=attrs
            with self.assertRaises(ValueError):regenerate(raw,layout,sub,original)
    def test_unchanged_geometry_noop(self):
        raw,layout,sub,original=self.fixture();sub['positions']=original
        out,r=regenerate(raw,layout,sub,original);self.assertEqual(out,raw);self.assertEqual(r['active_records'],0)
    def test_invalid_record_count(self):
        raw,layout,sub,original=self.fixture();layout['count']=4
        with self.assertRaises(ValueError):regenerate(raw,layout,sub,original)

if __name__=='__main__':unittest.main()
