"""Source cache scheduling and native actor-field binding evidence."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image, SOURCE_HASH, TARGET_HASH
from research import ElfImage, SelfImage
from oracle_fixture import assembly


class BoundaryCacheTests(unittest.TestCase):
    def test_stamp_before_query_and_same_tick_reuse(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists() or not target.exists():
            self.skipTest('Private inputs and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        native=checked_image(SelfImage,target,TARGET_HASH)
        self.assertEqual(native.read(0x3116835,0x2e).hex(),
            '8b05c9f1920841398538020000741f41898538020000498dbdd0010000c5f857c0'
            '498b75104883c630e86d7397ff')
        code=assembly(image,'original',0x452bfd0,0x452c01d,
                      {0x4708b90:'query'},{0x103e1418:'tick_storage'})
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
extern ABI void *original(void*);
typedef panic_u8 (ABI *query_fn)(float,void*,const float*);
extern void *nhl24_panic_boundary_cache(void*,unsigned,query_fn);
unsigned tick_storage[180];
static unsigned actor[160],pose[16],mode,calls;
ABI panic_u8 query(float radius,void*out,const float*position){
    if(radius!=0 || out!=(char*)actor+(mode?0x1f0:0x1d0) || position!=(float*)(pose+12))__builtin_trap();
    if(actor[(mode?0x258:0x238)/4]!=tick_storage[0x2cc/4])__builtin_trap();
    calls++;
    for(unsigned i=0;i<12;i++)((unsigned*)out)[i]=pose[12+i%4]^i;
    return 1;
}
ABI panic_u8 nhl27_segment_overlap(const float*a,const float*b,const float*c,const float*d){__builtin_trap();}
ABI panic_u8 nhl27_segment_intersection(const float*a,const float*b,const float*c,const float*d,float*e,float*f,float*g){__builtin_trap();}
ABI int nhl24_panic_foot_probe_guest(panic_address base,void*a,unsigned bone,float*out){__builtin_trap();}
__declspec(dllexport) void run(unsigned source,const unsigned *input,unsigned *output){
    mode=source;calls=0;
    for(unsigned i=0;i<160;i++)actor[i]=0xa5a5a5a5;
    *(void**)((char*)actor+0x10)=pose;
    for(unsigned i=0;i<4;i++)pose[12+i]=input[2+i];
    tick_storage[0x2cc/4]=input[0];actor[(mode?0x258:0x238)/4]=input[1];
    void *result=mode?original(actor):nhl24_panic_boundary_cache(actor,input[0],query);
    if(result!=(char*)actor+(mode?0x1f0:0x1d0))__builtin_trap();
    for(unsigned i=0;i<12;i++)output[i]=((unsigned*)result)[i];
    output[12]=calls;output[13]=actor[(mode?0x258:0x238)/4];
    if(mode)original(actor);else nhl24_panic_boundary_cache(actor,input[0],query);
    output[14]=calls;
}
int _fltused=0;
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-boundary-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding',
                '-fno-builtin','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',
                '-I',str(root/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'oracle.s'),
                str(root/'scripts/nhl27/runtime/panic_geometry.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_uint),C.POINTER(C.c_uint)]
                rng=random.Random(452)
                for n in range(4096):
                    tick=rng.getrandbits(32)
                    inp=(C.c_uint*6)(tick,tick if n%2 else rng.getrandbits(32),*(rng.getrandbits(32) for _ in range(4)))
                    a=(C.c_uint*15)();b=(C.c_uint*15)()
                    lib.run(0,inp,a);lib.run(1,inp,b)
                    self.assertEqual(list(a),list(b),(n,list(inp)))
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
