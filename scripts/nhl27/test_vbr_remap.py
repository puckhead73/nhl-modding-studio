import struct
import unittest
from vbr_remap import remap


class RemapTests(unittest.TestCase):
    def fixture(self):
        fields=dict(QuaternionCount=1,ConstQuaternionCount=1,Vector3Count=0,ConstVector3Count=0,
            FloatCount=0,ConstFloatCount=0,KeyTimeSize=1,ConstPaletteSize=4,ConstChanMapSize=2,
            VectorOffsetSize=0,FloatOffsetSize=0,NumKeys=1,TrajMin=0,TrajMax=1,Vec3Min=0,Vec3Max=1)
        stream=b'\1'+bytes(range(4))+b'\1\1'+bytes(16)+b'\1\2\3'+bytes(8)
        data=struct.pack('<12I',4,48,0,1,64,0,len(stream),66,0,0,0,0)
        data+=struct.pack('<4fH',0,0,0,1,3)+stream
        data+=bytes((-len(data))%16)+struct.pack('<3I',4,16,28)
        return fields,data

    def test_constant_dynamic_permutation_and_inverse(self):
        f,data=self.fixture();g,out,order=remap(f,data,[10,20],[20,10])
        self.assertEqual(order,[1,0]);self.assertEqual(g['ConstChanMapSize'],3)
        restored,raw,_=remap(g,out,[20,10],[10,20])
        self.assertEqual(restored,f);self.assertEqual(raw,data)

    def test_ambiguous_or_missing_dofs(self):
        f,data=self.fixture()
        with self.assertRaises(ValueError):remap(f,data,[10,10],[10,10])
        with self.assertRaises(ValueError):remap(f,data,[10,20],[10,30])
