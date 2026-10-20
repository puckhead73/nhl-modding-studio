"""Boundary checks for the compatibility readers and narrowly guarded converter."""
import struct
import unittest
from compatibility import ps4_spans,frames
from container_probe import normalize
from riff_evidence import chunks,fixup
from schema_compare import shared
from target_field_names import propagate
from dependency_probe import exports,resolve


def riff(parts):
    body=b'EBX\0'+b''.join(k+struct.pack('<I',len(v))+v+bytes(len(v)%2) for k,v in parts)
    return b'RIFF'+struct.pack('<I',len(body))+body


class CompatibilityTests(unittest.TestCase):
    def test_dependency_export_bounds_and_exact_instance(self):
        body=bytes(16)+struct.pack('<14I',0,0,1,1,16,0,0,0,0,0,0,0,0,0)
        data=riff([(b'EBXD',bytes(12)+bytes.fromhex('01000000')+bytes(14)),
            (b'EFIX',body),(b'EBXX',bytes(8))])
        f,ids=exports(data)
        self.assertEqual(ids,['00000001-0000-0000-0000-000000000000'])
        node=dict(name='test',sha1='test',exports=ids)
        ref=dict(partition=f['partition'],instance=ids[0])
        self.assertEqual(resolve([ref],{ref['partition']:[node]})[0]['status'],'exact_instance_in_sample')
        ref['instance']='unseen'
        self.assertEqual(resolve([ref],{ref['partition']:[node]})[0]['status'],'partition_only_in_sample')
        self.assertEqual(resolve([ref],{})[0]['status'],'partition_not_in_sample')
        bad=bytearray(body);struct.pack_into('<I',bad,32,1)
        with self.assertRaises(ValueError):exports(riff([(b'EBXD',bytes(30)),(b'EFIX',bytes(bad))]))

    def test_ps4_identifier_inheritance(self):
        r=bytearray(54)
        struct.pack_into('>I',r,12,2)
        struct.pack_into('>IIIII',r,32,0x10203,10,20,30,40)
        r[52:]=bytes([1,0]);s=ps4_spans(r)
        self.assertEqual(s[1],dict(layer=1,catalog=2,cas=3,offset=30,size=40))
        r[52]=0x80
        with self.assertRaises(ValueError):ps4_spans(r)

    def test_frames_and_decoder_failure(self):
        stored=struct.pack('>I',3)+b'\x00\x70\x00\x03abc'
        self.assertEqual(frames(stored)[1],b'abc')
        compressed=struct.pack('>I',3)+b'\x11\x70\x00\x01x'
        self.assertIsNone(frames(compressed)[1])
        self.assertEqual(frames(compressed,lambda b,n:b'abc')[1],b'abc')
        with self.assertRaises(ValueError):frames(compressed,lambda b,n:b'a')
        with self.assertRaises(ValueError):frames(stored[:-1])

    def test_conversion_requires_exact_other_chunks(self):
        old=riff([(b'EBXD',b'data'),(b'EFIX',b'fixup'),(b'EBXX',bytes(8))])
        new=riff([(b'EBXD',b'data'),(b'EFIX',b'fixup'+bytes(4)),(b'EBXX',bytes(8))])
        self.assertEqual(normalize(new,old),old)
        with self.assertRaises(ValueError):normalize(new.replace(b'data',b'xxxx'),old)
        with self.assertRaises(ValueError):normalize(new,old[:-1])

    def test_fixup_old_new_tail(self):
        # Empty table envelope: 16-byte partition, eight count fields,
        # exported count and three offsets. Old format has no reserved word.
        body=bytes(16)+struct.pack('<12I',*([0]*12))
        for reserved in [b'',bytes(4)]:
            data=riff([(b'EBXD',bytes(16)),(b'EFIX',body+reserved),(b'EBXX',bytes(8))])
            self.assertEqual(fixup(data)['instance_count'],0)
        with self.assertRaises(ValueError):chunks(b'RIFF'+bytes(20))

    def test_shared_descriptor_reference_bounds(self):
        signature=bytes(16)+struct.pack('<I',123)
        body=struct.pack('<I',1)+signature+struct.pack('<I',1)+struct.pack('<IIHHHH',1,0,1,0x60,32,8)
        body+=struct.pack('<I',1)+struct.pack('<IIHH',2,0,0,0)+bytes(8)
        payload=b'EBXT'+b'REFL'+struct.pack('<I',len(body))+body
        data=b'RIFF'+struct.pack('<I',len(payload))+payload
        self.assertEqual(shared(data)[0]['size'],32)
        bad=bytearray(data);struct.pack_into('<H',bad,20+4+20+4+16+4+10,1)
        with self.assertRaises(ValueError):shared(bytes(bad))

    def test_literal_store_and_branch_reset(self):
        class Image:
            def __init__(self,data):self.data=data
            def read(self,at,size):
                if at+size>len(self.data):raise ValueError('unmapped')
                return self.data[at:at+size]
        # movabs rax,42; store to RIP+0x100. A branch then invalidates rax.
        code=bytes.fromhex('48b82a0000000000000048890500010000eb0048890500020000c3')
        memory=propagate(Image(code),0,len(code))
        self.assertEqual(bytes(memory[i] for i in range(0x111,0x119)),struct.pack('<Q',42))
        self.assertNotIn(0x21a,memory)


if __name__=='__main__':unittest.main()
