import io
import struct
import unittest
from census import eb_entries, arena_token


class CensusTests(unittest.TestCase):
    def test_big_inside_iso_uses_relative_offsets(self):
        b=bytearray(128);b[:4]=b'BIG4';struct.pack_into('<I',b,4,len(b));struct.pack_into('>II',b,8,1,40)
        struct.pack_into('>II',b,16,64,8);b[24:30]=b'a.rx2\0'
        row=list(eb_entries(io.BytesIO(b'prefix'+b),6,len(b)))[0]
        self.assertEqual((row['offset'],row['size'],row['name']),(64,8,'a.rx2'))

    def test_big_rejects_out_of_bounds_member(self):
        b=bytearray(128);b[:4]=b'BIG4';struct.pack_into('>II',b,8,1,40)
        struct.pack_into('>II',b,16,120,32);b[24:26]=b'a\0'
        with self.assertRaisesRegex(ValueError,'outside'):list(eb_entries(io.BytesIO(b),0,len(b)))

    def test_eb_uses_stored_not_decoded_length(self):
        b=bytearray(128);b[:4]=b'EB\0\3';struct.pack_into('>III',b,4,1,0x200400,64)
        b[20:22]=bytes([8,8]);struct.pack_into('>H',b,22,1)
        struct.pack_into('>IIII',b,48,6,16,1000,0);b[64:72]=b'\0\0a.bin\0';b[80:88]=b'dir\0\0\0\0\0'
        row=list(eb_entries(io.BytesIO(b),0,len(b)))[0]
        self.assertEqual(row['size'],16);self.assertEqual(row['declared_length_word'],1000)

    def test_venue_tokens_do_not_infer_team_bindings(self):
        self.assertEqual(arena_token('content/worlds/stadiums/boston_2/lighting/authentic'),'boston_2')
        self.assertEqual(arena_token('rendering/stadium/stadium_26_container_0.rx2'),'stadium_26')
        self.assertIsNone(arena_token('rendering/jumbotron/texlib_26.rx2'))
        self.assertIsNone(arena_token('rendering/icesurface/centerlogo_26_cm.rx2'))


if __name__=='__main__':unittest.main()
