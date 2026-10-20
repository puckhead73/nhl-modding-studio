"""Execute the source rink-end distance query against the portable calculation."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class EndDistanceTests(unittest.TestCase):
    def test_original_float_operations(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x453b4a0,0x453b4ea,{},
                      {0xef1f9cc:'end_z'})
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
extern ABI float original(void*);
extern float nhl27_panic_end_distance(const float*,float);
extern ABI float nhl24_panic_end_distance_guest(panic_address,void*);
unsigned end_z;
ABI panic_u8 nhl27_segment_overlap(const float*a,const float*b,const float*c,const float*d){__builtin_trap();}
ABI panic_u8 nhl27_segment_intersection(const float*a,const float*b,const float*c,const float*d,float*e,float*f,float*g){__builtin_trap();}
ABI int nhl24_panic_foot_probe_guest(panic_address base,void*actor,unsigned bone,float*out){__builtin_trap();}
__declspec(dllexport) unsigned run(unsigned mode,const unsigned *input){
    unsigned actor[6]={0},pose[16]={0};
    *(void**)((char*)actor+0x10)=pose;
    pose[12]=input[0];pose[14]=input[1];end_z=input[2];
    union {float f;unsigned u;} result,extent={.u=end_z};
    if(mode==2)result.f=nhl24_panic_end_distance_guest((panic_address)&end_z-0xaa5d4a4,actor);
    else result.f=mode?original(actor):nhl27_panic_end_distance((float*)(pose+12),extent.f);
    return result.u;
}
int _fltused=0;
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-end-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder)
            (out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding',
                '-fno-builtin','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',
                '-I',str(root/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'oracle.s'),
                str(root/'scripts/nhl27/runtime/panic_geometry.c'),'-o',str(out/'test.dll')],
                capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_uint)];lib.run.restype=C.c_uint
                rng=random.Random(4534)
                values=[0,0x80000000,0x3f800000,0xbf800000,0x7f7fffff,
                        0x7fc00001,0xffc01000,0x7f800000,0xff800000]
                for n in range(8192):
                    inp=(C.c_uint*3)(*(rng.choice(values) if n<4096 else rng.getrandbits(32) for _ in range(3)))
                    self.assertEqual(lib.run(0,inp),lib.run(1,inp),(n,list(inp)))
                    self.assertEqual(lib.run(2,inp),lib.run(1,inp),(n,list(inp)))
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
