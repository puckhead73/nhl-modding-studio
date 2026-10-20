"""Verify the engine-facing closest-distance adapter's call order and gates."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class ClosestNativeTests(unittest.TestCase):
    def test_native_context_failure_paths_and_virtual_dispatch(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
extern ABI int nhl24_panic_update_closest_guest(panic_address,struct panic_state*);
static unsigned char actor[0x68],anim[0xc30],wrapper[0x28],evaluation[0x80];
static panic_address tables[2][7],candidates[2];
static unsigned *trace,flags;
static float result;
static void log(unsigned n){trace[++trace[0]]=n;}
static void check(panic_address base,void*a){if(base!=0x10000000 || a!=actor)__builtin_trap();}
ABI int nhl24_panic_bind_guest(panic_address base,void*ctx,struct panic_bindings*out){
 if(base!=0x10000000 || ctx!=evaluation+0x60)__builtin_trap();log(1);
 out->closest_distance=&result;return flags&1?2:0;
}
ABI unsigned char nhl24_panic_cached_active(void*a){if(a!=actor)__builtin_trap();log(2);return 1;}
ABI int nhl24_panic_motion_context_guest(panic_address base,void*ctx,unsigned char cached,unsigned bit,unsigned char*out){
 if(base!=0x10000000 || ctx!=evaluation+0x60 || cached!=1 || bit!=3)__builtin_trap();
 log(3);*out=!(flags&4);return flags&2?3:0;
}
ABI void *nhl24_panic_first_candidate_guest(panic_address base,void*a){check(base,a);log(4);return flags&8?0:&candidates[0];}
ABI void *nhl24_panic_second_candidate_guest(panic_address base,void*a){check(base,a);log(5);return flags&16?0:&candidates[1];}
ABI float nhl24_panic_boundary_distance_guest(panic_address base,void*a){check(base,a);log(6);return 8;}
ABI float nhl24_panic_end_distance_guest(panic_address base,void*a){check(base,a);log(9);return 6;}
float nhl27_panic_actor_distance(void*c,void*a){
 if(a!=actor || c!=&candidates[0])__builtin_trap();log(7);return 2;
}
ABI float virtual_distance(void*c,void*a){if(a!=actor || c!=&candidates[1])__builtin_trap();log(8);return 4;}
__declspec(dllexport) void run(unsigned options,unsigned*out){
 flags=options;trace=out;trace[0]=0;result=10;
 *(void**)(actor+0x60)=anim;*(void**)(anim+0xc28)=wrapper;*(void**)(wrapper+0x20)=evaluation;
 tables[0][6]=0x126323d0;tables[0][4]=0x12749130;
 tables[1][6]=(panic_address)virtual_distance;
 candidates[0]=(panic_address)tables[0];candidates[1]=(panic_address)tables[1];
 struct panic_state state={0};state.actor=(panic_address)actor;
 out[14]=nhl24_panic_update_closest_guest(0x10000000,&state);
 union {float f;unsigned u;} value={result};out[15]=value.u;
}
int _fltused=0;
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-closest-native-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(root/'scripts/nhl27/runtime/panic_update.c'),
                str(root/'scripts/nhl27/runtime/panic_closest_native.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_uint)]
                for flags in range(32):
                    out=(C.c_uint*16)();lib.run(flags,out)
                    trace=[1];status=0;bits=0x41200000
                    if flags&1:status=2
                    else:
                        trace += [2,3]
                        if flags&2:status=3
                        elif not flags&4:
                            trace += [4,5,6]
                            if not flags&8:trace += [7]
                            if not flags&16:trace += [8]
                            trace += [9]
                            bits=0x40000000 if not flags&8 else (0x40800000 if not flags&16 else 0x40c00000)
                    self.assertEqual(list(out)[1:1+out[0]],trace,flags)
                    self.assertEqual((out[14],out[15]),(status,bits),flags)
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
