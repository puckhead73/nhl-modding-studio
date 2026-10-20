"""Differential execution of source/native KD queries on the actual port trees."""
import ctypes as C
import json
import os
from pathlib import Path
import random
import struct
import subprocess
import tempfile
import unittest
from capstone import Cs,CS_ARCH_X86,CS_MODE_64
from deke_runtime_evidence import checked_image,SOURCE_HASH,TARGET_HASH
from research import ElfImage,SelfImage
from oracle_fixture import assembly

SOURCE_RANGES=[(0x279e40,0x27a1a0),(0x27cba0,0x27ce00),(0x27ce00,0x27d120),
 (0x27d120,0x27d730),(0x27d730,0x27dbd0),(0x27dbd0,0x27dff0),
 (0x2797f0,0x279880),(0x279880,0x279930),(0x279930,0x279a00),
 (0x279a00,0x279af0),(0x279af0,0x279c00),(0x279c00,0x279d10)]
NATIVE_RANGES=[(0x366450,0x366960),(0x366960,0x366d10),(0x366d10,0x366d60),
 (0x366d60,0x366fc0),(0x366fc0,0x3672e0),(0x3672e0,0x3675e0),(0x3675e0,0x367680),
 (0x365fa0,0x366020),(0x366020,0x3660b0),(0x3660b0,0x366170),
 (0x366170,0x366240),(0x366240,0x366320),(0x366320,0x366410)]

def relocate(image,source):
    prefix='src' if source else 'dst'
    ranges=SOURCE_RANGES if source else NATIVE_RANGES
    names={a:f'{prefix}_{a:x}' for a,b in ranges}
    names.update({0xbfabea0:'guard_fail',0xbfabf10:'fill_bytes'} if source else {0x8ec6790:'guard_fail',0x8ec6000:'fill_bytes'})
    table=0xdf17af8 if source else 0x90805cc
    destinations=[table+v for v in struct.unpack('<5i',image.read(table,20))]
    labels={v:f'{prefix}_case_{v:x}' for v in destinations}
    globals_={0xec2d9f0 if source else 0xa87c1f8:'guard_pointer',table:f'{prefix}_cases'}
    if source:
        if image.read(0xec624e0,32)!=struct.pack('<4Q',0,1,2,3):raise ValueError('Unexpected radix byte order')
        globals_.update({0xec624e0+8*i:f'src_radix_{i}' for i in range(4)})
    allowed_calls={0x27a166:'ff5128',0x27cc8f:'41ff5220',0x27cd7e:'41ff5220'} if source else {
        0x366713:'ff5028',0x366f70:'ff5020',0x366f8d:'ff5020'}
    jump=0x27d19a if source else 0x3671db
    result=''
    for a,b in ranges:
        result+=assembly(image,names[a],a,b,names,globals_,
            {v:raw for v,raw in allowed_calls.items() if a<=v<b},
            {jump:'ffe1'} if a<=jump<b else {},
            {v:name for v,name in labels.items() if a<=v<b})
    result+=f'.balign 4\n{prefix}_cases:\n'+''.join(f'.long {labels[v]} - {prefix}_cases\n' for v in destinations)
    if source:result+=''.join(f'.balign 8\nsrc_radix_{i}:\n.quad {i}\n' for i in range(4))
    return result

class KDTests(unittest.TestCase):
    def test_qualification_rejects_unseen_tree_or_configuration(self):
        from copy import deepcopy
        from ptm_kd_qualification import qualified
        root=Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/deke-trial'
        manifest=root/'kd-database-inputs.json'
        if not manifest.exists():self.skipTest('Private converted databases required')
        for row in json.loads(manifest.read_text()):
            d=json.loads((Path(__file__).resolve().parents[2]/row['path']).read_text())
            f=next(i['fields'] for i in d['instances'] if i['offset']==row['offset'])
            self.assertTrue(qualified(f))
            changed=deepcopy(f);changed['SortKNNResults']=True
            self.assertFalse(qualified(changed))
            changed=deepcopy(f)
            tree=next(t for t in changed['PTMKDImpls'] if t['KDTreeData'])
            tree['KDTreeData'][0]['0x29c0831c']^=1
            self.assertFalse(qualified(changed))

    def test_actual_packed_trees(self):
        repo=Path(__file__).resolve().parents[2];root=repo/'local/nhl27-eboot/compatibility/deke-trial'
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin');target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name!='nt' or not source.exists() or not target.exists() or not compiler.exists():self.skipTest('Private binaries and LLVM required')
        code=relocate(checked_image(ElfImage,source,SOURCE_HASH),True)+relocate(checked_image(SelfImage,target,TARGET_HASH),False)
        fixture=r'''
#define ABI __attribute__((sysv_abi))
typedef __UINTPTR_TYPE__ addr;
static addr guard;addr *guard_pointer=&guard;
ABI void guard_fail(void){__builtin_trap();}
ABI void *fill_bytes(void*p,int value,addr size){for(addr n=0;n<size;n++)((unsigned char*)p)[n]=value;return p;}
static unsigned char arena[16*1024*1024] __attribute__((aligned(32)));
static addr used,methods[6];
ABI void *allocate(void*a,addr size,addr alignment){(void)a;if(alignment>32)__builtin_trap();used=(used+31)&~(addr)31;addr old=used;used+=size;if(used>sizeof(arena))__builtin_trap();return arena+old;}
ABI void release(void*a,void*p){(void)a;(void)p;}
struct head {void *allocator;addr capacity;void *tree;};
extern ABI addr src_279e40(struct head*,float*,void*,addr,struct head**,unsigned,unsigned,unsigned);
extern ABI addr dst_366450(struct head*,float*,void*,addr,struct head**,unsigned,unsigned);
__declspec(dllexport) unsigned run(unsigned source,void**trees,unsigned tree_count,float*query,unsigned count,void*out){
 methods[4]=(addr)allocate;methods[5]=(addr)release;addr *allocator=methods;
 used=0;struct head h[16];struct head *heads[16];
 if(!tree_count || tree_count>16)__builtin_trap();
 for(unsigned n=0;n<tree_count;n++){h[n]=(struct head){&allocator,0,trees[n]};heads[n]=&h[n];}
 return source?src_279e40(h,query,out,count,heads,tree_count,count,(source>>1)&1):dst_366450(h,query,out,count,heads,tree_count,count);
}
int _fltused=0;
'''
        with tempfile.TemporaryDirectory(prefix='ptm-kd-',dir=root/'runtime-build') as folder:
            out=Path(folder);(out/'fixture.s').write_text(code);(out/'fixture.c').write_text(fixture)
            r=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',str(out/'fixture.s'),str(out/'fixture.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_void_p),C.c_uint,C.POINTER(C.c_float),C.c_uint,C.c_void_p];lib.run.restype=C.c_uint
                rng=random.Random(0x279e40);cases=0
                for row in json.loads((root/'kd-database-inputs.json').read_text()):
                    d=json.loads((repo/row['path']).read_text());f=next(i['fields'] for i in d['instances'] if i['offset']==row['offset'])
                    previous=None;previous_dims=None
                    for index,t in enumerate(f['PTMKDImpls']):
                        raw=b''.join(struct.pack('<4I',*(v[k]&0xffffffff for k in ('0x29c0831c','0xa53f7f32','0xeafbb36d','0x3d0f7ed4'))) for v in t['KDTreeData'])
                        if not raw:continue
                        dims=struct.unpack_from('<I',raw,16)[0];self.assertLessEqual(dims,8)
                        tree=C.create_string_buffer(raw);low=struct.unpack_from('<8f',raw,0x30);high=struct.unpack_from('<8f',raw,0x50)
                        for trial in range(20):
                            query=(C.c_float*8)(*(rng.uniform(low[j]-1,high[j]+1) for j in range(8)))
                            count=(1,3,8,32,200)[trial%5];other=previous if trial>=15 and previous_dims==dims else tree
                            forest=(C.c_void_p*2)(C.addressof(tree),C.addressof(other));ntrees=1 if trial<10 else 2
                            a=(C.c_uint*600)();b=(C.c_uint*600)();na=lib.run(1 if trial<5 else 3,forest,ntrees,query,count,a);nb=lib.run(0,forest,ntrees,query,count,b)
                            self.assertEqual(na,nb,(row['name'],index,trial))
                            aa=[tuple(a[3*i:3*i+3]) for i in range(na)];bb=[tuple(b[3*i:3*i+3]) for i in range(nb)]
                            # Both engines decode packed 0xffffffff as an invalid candidate;
                            # source divides index/tree at bit 23, target at bit 24.
                            aa=[(v[0],0xffffffff,0xffffffff) if v==(0x7f800000,0x7fffff,0x1ff) else v for v in aa]
                            bb=[(v[0],0xffffffff,0xffffffff) if v==(0x7f800000,0xffffff,0xff) else v for v in bb]
                            if trial<5:self.assertEqual(aa,bb,(row['name'],index,trial,'unsorted order'))
                            self.assertEqual(sorted(aa),sorted(bb),(row['name'],index,trial));cases+=1
                        previous=tree;previous_dims=dims
                self.assertGreater(cases,0)
                (root/'kd-query-verification.json').write_text(json.dumps({'cases':cases,'status':'passed','source_sorted_and_unsorted':True,'single_duplicate_and_mixed_forests':True},indent=2))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)

if __name__=='__main__':unittest.main()
