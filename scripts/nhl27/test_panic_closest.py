"""Original nearest-opponent update, including query order and float edge cases."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class ClosestTests(unittest.TestCase):
    def test_original_order_missing_candidates_and_nan_operand_order(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x4a52200,0x4a522f6,
            {0x390dd0:'resolve',0x4a1ab20:'motion',0x453bf00:'first',
             0x453be00:'second',0x452bfd0:'cached',0x453b4a0:'third'}, {},
            {0x4a5229c:'ff5030',0x4a522b2:'ff5030'})
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original(struct panic_state*);
static unsigned char actor[0x68],anim[0xc68],wrapper[0x200],root_wrapper[0x18],root[0x120],record[0xc0];
static panic_address vtable[7],one[1],two[1];
static unsigned cache[2],value;
static struct panic_bindings bindings;
static const unsigned *input;
static unsigned *trace;
static void log(unsigned n){trace[++trace[0]]=n;}
static float number(unsigned n){union{unsigned u;float f;}v={input[n]};return v.f;}
ABI void *resolve(void*a,void*b){(void)a;(void)b;log(1);return record;}
ABI panic_u8 motion(void*a){(void)a;log(2);return input[0]&255;}
ABI void *first(void*a,int mode){(void)a;if(mode)__builtin_trap();log(3);return input[1]&1?one:0;}
ABI void *second(void*a){(void)a;log(4);return input[1]&2?two:0;}
ABI void *cached(void*a){(void)a;log(5);return cache;}
ABI float distance(void*p,void*a){(void)a;log(p==one?6:7);return number(p==one?3:4);}
ABI float third(void*a){(void)a;log(8);
    /* Source reads the destination pointer after the final distance call. */
    if(input[1]&8){bindings.closest_distance=0;*(void**)(record+0xb8)=0;}
    return number(5);
}
static struct panic_bindings *get_bindings(void*c){(void)c;resolve(0,0);return &bindings;}
static panic_u8 get_motion(void*c,panic_address a){(void)c;return motion((void*)a);}
static void *get_first(void*c,panic_address a){(void)c;return first((void*)a,0);}
static void *get_second(void*c,panic_address a){(void)c;return second((void*)a);}
static float get_cached(void*c,panic_address a){(void)c;unsigned *p=cached((void*)a);union{unsigned u;float f;}v={p[1]};return v.f;}
static float get_distance(void*c,void*p,panic_address a){(void)c;return distance(p,(void*)a);}
static float get_third(void*c,panic_address a){(void)c;return third((void*)a);}
__declspec(dllexport) void run(unsigned source,const unsigned*i,unsigned*out){
    input=i;trace=out;trace[0]=0;value=i[6];cache[1]=i[2];
    vtable[6]=(panic_address)distance;one[0]=two[0]=(panic_address)vtable;
    *(void**)(actor+0x60)=anim;*(void**)(anim+0xc60)=wrapper;
    *(void**)(wrapper+0x20)=wrapper;*(void**)(wrapper+0x1f8)=root_wrapper;
    *(void**)(root_wrapper+0x10)=root;*(void**)(root+0x118)=root;
    bindings.closest_distance=i[1]&4?(float*)&value:0;
    *(void**)(record+0xb8)=bindings.closest_distance;
    struct panic_state state={0};state.actor=(panic_address)actor;
    struct panic_closest_services services={0,get_bindings,get_motion,get_first,get_second,
        get_cached,get_distance,get_third};
    if(source)original(&state);else nhl27_panic_update_closest(&state,&services);
    out[15]=value;
}
int _fltused=0;
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-closest-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/panic_update.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_uint),C.POINTER(C.c_uint)]
                rng=random.Random(27522)
                values=[0,0x80000000,0x3f800000,0xbf800000,0x7f7fffff,0x7fc00001,0xffc01000,0x7f800000,0xff800000]
                for n in range(12000):
                    inp=(C.c_uint*7)(rng.choice([0,1,2,255]),rng.randrange(16),
                                    *(rng.choice(values) for _ in range(5)))
                    expected=(C.c_uint*16)();actual=(C.c_uint*16)()
                    lib.run(1,inp,expected);lib.run(0,inp,actual)
                    self.assertEqual(list(actual),list(expected),(n,list(inp)))
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
