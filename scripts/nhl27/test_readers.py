"""Synthetic fixtures: no proprietary game bytes are required."""
import struct
import tempfile
import unittest
from pathlib import Path
from research import ElfImage
from bundle_inventory import bundles


class ReaderTests(unittest.TestCase):
    def setUp(self):
        self.scratch=(Path(__file__).resolve().parents[2]/'local/nhl27-eboot/test-fixtures').resolve()
        self.scratch.mkdir(parents=True,exist_ok=True)
        self.tmp=tempfile.TemporaryDirectory(dir=self.scratch);self.root=Path(self.tmp.name).resolve()
    def tearDown(self):
        # Verify cleanup stays inside the owned fixture directory on Windows.
        self.assertTrue(self.root.is_relative_to(self.scratch))
        self.assertNotEqual(self.root,self.scratch)
        self.tmp.cleanup()
    def file(self, data):
        p=self.root/'fixture.bin';p.write_bytes(data);return p
    def elf(self):
        b=bytearray(1024)
        struct.pack_into('<16sHHIQQQIHHHHHH',b,0,b'\x7fELF\x02\x01\x01'+bytes(9),3,62,1,0x1000,64,0xffff,0,64,56,2,64,54,0)
        struct.pack_into('<IIQQQQQQ',b,64,1,5,512,0x1000,0,16,32,16)
        struct.pack_into('<IIQQQQQQ',b,120,1,4,768,0x2000,0,16,16,16)
        b[512:528]=bytes(range(16));b[768:784]=bytes(range(16,32))
        return b
    def test_segment_mapping_ignores_stale_sections(self):
        im=ElfImage(self.file(self.elf()))
        self.assertEqual(im.read(0x1004,4),bytes([4,5,6,7]))
        self.assertEqual(im.read(0x2004,4),bytes([20,21,22,23]))
        self.assertEqual(im.offset(0x2004),772)
    def test_bss_is_not_file_backed(self):
        im=ElfImage(self.file(self.elf()))
        with self.assertRaises(ValueError): im.read(0x1010,1)
        with self.assertRaises(ValueError): im.read(0x100f,2)
    def test_bad_architecture(self):
        b=self.elf();struct.pack_into('<H',b,18,40)
        with self.assertRaises(ValueError): ElfImage(self.file(b))
    def test_segment_exceeds_file(self):
        b=self.elf();struct.pack_into('<Q',b,64+32,2000)
        with self.assertRaises(ValueError): ElfImage(self.file(b))
    def test_segment_file_exceeds_memory(self):
        b=self.elf();struct.pack_into('<Q',b,64+40,8)
        with self.assertRaises(ValueError): ElfImage(self.file(b))
    def toc(self, compressed=False):
        b=bytearray(96);w=[0]*15;w[0]=60;w[1]=60;w[2]=1;w[8]=76
        if compressed:
            w[8]=84;w[11]=4;w[12]=1;w[13]=2;w[14]=76
            struct.pack_into('>iiI',b,76,~0,~65,1)
        else: b[76:78]=b'A\0'
        struct.pack_into('>15I',b,0,*w);struct.pack_into('>IIQ',b,60,0,0,0)
        return b
    def test_uncompressed_name(self): self.assertEqual(bundles(self.file(self.toc()))[0][0]['name'],'A')
    def test_compressed_name(self): self.assertEqual(bundles(self.file(self.toc(True)))[0][0]['name'],'A')
    def test_compressed_tree_cycle(self):
        b=self.toc(True);struct.pack_into('>iI',b,80,0,1)
        with self.assertRaises(ValueError): bundles(self.file(b))
    def test_empty_compressed_tree(self):
        b=bytearray(60);struct.pack_into('>I',b,0,60);struct.pack_into('>I',b,44,4)
        self.assertEqual(bundles(self.file(b))[0],[])
    def test_name_bitstream_overflow(self):
        b=self.toc(True);struct.pack_into('>I',b,60,32)
        with self.assertRaises(ValueError): bundles(self.file(b))


if __name__=='__main__': unittest.main()
