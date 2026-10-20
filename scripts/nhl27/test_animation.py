"""Synthetic motion signals and malformed-stream tests, without game assets."""
import math
import struct
import unittest
from animation_dct import Bits,decode,envelope
from animation_stage import bank_ids


def resource(dynamic=False):
    # One quaternion, eight keys, unit W; optional X frequency coefficient.
    body=struct.pack('<5H2B',8,1,0,0,1000,0,15)+bytes([0x20 if dynamic else 0,0])
    body+=struct.pack('<4h',0,0,0,4000)
    if dynamic:
        body+=struct.pack('<2H',0,0x8000)+bytes([20])+bytes(31)
    else:body+=bytes(16)
    out=struct.pack('<5I',0,32,0,len(body),32)+bytes(12)+body
    return out+bytes((-len(out))%16)+struct.pack('<2I',4,16)


class AnimationTests(unittest.TestCase):
    def test_constant_unit_rotation(self):
        meta,frames,checks=decode(resource())
        self.assertEqual(meta['keys'],8)
        self.assertEqual(frames,[[[0,0,0,1]]]*8)
        self.assertEqual(checks['consumed_bits'],0)

    def test_known_frequency_signal(self):
        _,frames,checks=decode(resource(True))
        for i,frame in enumerate(frames):
            self.assertAlmostEqual(frame[0][0],.01*math.cos(math.pi*(2*i+1)/16))
        self.assertEqual(checks['consumed_bits'],8)

    def test_lsb_signed_and_bounds(self):
        bits=Bits(bytes([0b11010101]))
        self.assertEqual(bits.signed(3),-3)
        self.assertEqual(bits.signed(5),-6)
        with self.assertRaises(ValueError):bits.signed(1)

    def test_bank_identifier_crosscheck(self):
        b=struct.pack('>9I',84,3,2,2,0,0,0,0,0)
        b+=struct.pack('<4Q',123,0,456,0)+struct.pack('<2Q',123,456)
        self.assertEqual(bank_ids(b),{123,456})
        with self.assertRaises(ValueError):bank_ids(b[:-1]+b'\x01')

    def test_non_rotation_channels(self):
        b=bytearray(resource());struct.pack_into('<HH',b,34,0,1)
        _,_,checks=decode(bytes(b))
        self.assertIsNone(checks['min_quaternion_norm'])

    def test_corruption_rejected(self):
        r=resource()
        with self.assertRaises(ValueError):envelope(r[:-1])
        bad=bytearray(r);bad[32+14+6:32+14+8]=struct.pack('<h',8000)
        with self.assertRaises(ValueError):decode(bytes(bad))
        bad=bytearray(r);bad[32+12]=0xf0
        with self.assertRaises(ValueError):decode(bytes(bad))


if __name__=='__main__':unittest.main()
