"""Compare source history lookups with equivalent native cache layouts."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class HistoryPoseTests(unittest.TestCase):
    def test_source_lookup_and_missing_history_defaults(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and LLVM required')
        im=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(im,'original',0x2024f0,0x2028e0,
            {0x202250:'fail',0x1d1970:'fail',0x1d1cc0:'fail',0xbfabea0:'fail'},
            {0xec2d9f0:'guard',0xdf5fca0:'missing_scale',0xdf5fc60:'missing_rotation',0xdf5fc90:'missing_translation'},indirect_calls={0x2025ce:'ff90a0000000'})
        code+='\n.balign 16\nmissing_scale: .float 1,1,1,0\nmissing_rotation: .float 0,0,0,1\nmissing_translation: .float 0,0,0,0\n'
        fixture=r'''
#include "ptm_native.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
int _fltused=0;static A cookie;A *guard=&cookie;
ABI void fail(void){__builtin_trap();}
extern ABI void original(float*,void*,void*,int,int,void*,float*);
extern int nhl24_ptm_history_pose(void*,void*,int,float*,float*);
static unsigned char context[0x80],selectors[0x40],record[0x30],owner[16],history[0x30];
static unsigned char source_entry[0x200],native_entry[0x4600],map[0x16e0],node[0x50],other[0x50];
static A buckets[2],nodes[2],ring[2];
static void zero(void*p,unsigned n){for(unsigned i=0;i<n;i++)((unsigned char*)p)[i]=0;}
__declspec(dllexport) void run(unsigned which,unsigned mode,int joint,unsigned interval,float*out){
 zero(context,sizeof(context));zero(selectors,sizeof(selectors));zero(record,sizeof(record));
 zero(owner,sizeof(owner));zero(history,sizeof(history));zero(source_entry,sizeof(source_entry));
 zero(native_entry,sizeof(native_entry));zero(map,sizeof(map));zero(node,sizeof(node));zero(other,sizeof(other));
 zero(buckets,sizeof(buckets));zero(nodes,sizeof(nodes));zero(ring,sizeof(ring));
 *(A*)(context+0x40)=(A)selectors;
 *(int*)selectors=(mode&2)?-1:0;*(A*)(selectors+0x18)=(A)record;*(A*)(selectors+0x20)=(A)(record+0x30);
 *(A*)(record+8)=(A)owner;
 *(A*)(history+0x10)=(A)buckets;*(unsigned*)(history+0x18)=1;
 buckets[0]=(mode&4)?0:(A)(which?native_entry:source_entry);
 *(A*)source_entry=(A)owner;*(A*)native_entry=(A)owner;
 *(A*)(source_entry+8)=(mode&8)?(A)(source_entry+8):(A)ring;
 *(A*)(source_entry+0x10)=(A)ring;ring[0]=(A)(source_entry+8);ring[1]=(A)map;
 *(A*)(native_entry+8)=(A)map;*(unsigned*)(native_entry+0x38)=(mode&8)?0:1;
 *(unsigned*)(native_entry+0x45d8)=interval;*(unsigned*)(map+0x16d0)=interval;
 *(A*)(map+0x18)=(A)nodes;*(unsigned*)(map+0x20)=1;
 nodes[0]=(mode&16)?0:(A)node;*(int*)node=joint;
 if(mode&32){nodes[0]=(A)other;*(int*)other=joint^1;*(A*)(other+0x40)=(A)node;}
 for(unsigned i=0;i<12;i++)*(unsigned*)(node+0x10+4*i)=0x3f800000u+i*97u;
 void *h=(mode&1)?0:history;
 if(which)nhl24_ptm_history_pose(h,(mode&2)?0:owner,joint,out,out+12);
 else original(out,context,0,joint,0,h,out+12);
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-history-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'oracle.s').write_text(code)
            r=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.c'),str(p/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_history_pose.c'),
                '-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.c_int,C.c_uint,C.POINTER(C.c_float)]
                for mode in range(64):
                    for joint in (-1,0,1,7):
                        for dt in (0,0x80000000,0x40000000,0x7fc12345):
                            a=(C.c_float*13)();b=(C.c_float*13)()
                            lib.run(0,mode,joint,dt,a);lib.run(1,mode,joint,dt,b)
                            self.assertEqual(bytes(a),bytes(b),(mode,joint,dt))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
