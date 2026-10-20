import unittest
from vbr_coefficients import decode,encode,padded


class VbrCoefficientsTests(unittest.TestCase):
    def test_signed_sparse_cross_byte_coefficients(self):
        widths=[0x43212345,0x11111111,0]
        values=[[17,-4,3,2,-1,2,-3,5],[1,-1,0,1,0,-1,1,0],[0]*8]
        block=encode(widths,values,[60,55,55])
        self.assertEqual(decode(widths,block),values)
        self.assertEqual(encode(widths,decode(widths,block),block[:3]),block)

    def test_truncation_and_overflow(self):
        with self.assertRaises(ValueError):decode([0xffffffff],b'\0\0\0')
        with self.assertRaises(ValueError):encode([1],[[2]+[0]*7],[0]*3)
        with self.assertRaises(ValueError):encode([0],[[1]+[0]*7],[0]*3)

    def test_vector_and_float_padding(self):
        import struct
        channels=[[i]*8 for i in range(1,9)]
        data,stride=padded(channels,1,1)
        self.assertEqual(stride,12)
        values=struct.unpack('<'+'i'*(len(data)//4),data)
        self.assertEqual(values[:4],(1,2,3,4))
        self.assertEqual(values[32:36],(5,6,7,0))
        self.assertEqual(values[64:68],(8,0,0,0))
