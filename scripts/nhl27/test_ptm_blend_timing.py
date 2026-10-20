"""Compare blend clock arithmetic with the original NHL27 instructions."""
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


def wrap(code,name,prologue,epilogue):
    code=code.replace(name+':\n',name+':\n'+prologue+'\n',1)
    if '.balign 16' not in code:return code+'\n'+epilogue+'\n'
    return code.replace('.balign 16',epilogue+'\n.balign 16',1)


class BlendTimingTests(unittest.TestCase):
    def test_original_clock_math(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code='.intel_syntax noprefix\n'+wrap(assembly(image,'progress_original',0x28441e,0x28446e,{0x284e0c:'fail'},{}),
            'progress_original','push r15\nlea r15,[rdi-0x2c0]','vmovaps xmm0,xmm2\npop r15\nret')
        code+=assembly(image,'rescale_original',0x69fcb0,0x69fcd3,{}, {})
        code+=wrap(assembly(image,'advance_original',0x284e0c,0x284e32,{}, {}),
            'advance_original','push r15\nlea r15,[rdi-0x2c0]','pop r15\nret')
        code+=wrap(assembly(image,'speed_original',0x285554,0x285582,{}, {}),
            'speed_original','push rbp\npush r15\nlea rbp,[rsi+0x3964]\nlea r15,[rdi-0x310]',
            'pop r15\npop rbp\nret')
        fixture=r'''
#include "ptm_blend_timing.h"
#define ABI PTM_GAME_ABI
extern ABI float progress_original(float*);
extern ABI void rescale_original(float*,float*,float,float);
extern ABI void advance_original(float*,float);
extern ABI float speed_original(float*,float*,float);
ABI void fail(void){__builtin_trap();}int _fltused=0;
__declspec(dllexport) void run(unsigned w,const float*v,float*out){
 struct ptm_blend_clock s={v[0],v[1],v[2],v[3],v[4],0,0};
 out[0]=w?nhl27_ptm_blend_progress(s.duration,s.elapsed):progress_original(&s.duration);
 if(w)nhl27_ptm_blend_rescale(&s.duration,&s.elapsed,v[3],v[2]);
 else rescale_original(&s.duration,&s.elapsed,v[3],v[2]);
 out[1]=s.duration;out[2]=s.elapsed;
 if(w)nhl27_ptm_blend_advance(&s,v[4]);else advance_original(&s.duration,v[4]);
 out[3]=s.elapsed;
 float feature[2],remaining=v[2];
 out[4]=w?nhl27_ptm_blend_feature_speed(v[0],remaining):speed_original(feature+1,&remaining,v[0]);
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-blend-clock-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'oracle.s').write_text(code)
            r=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.c'),str(p/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_blend_timing.c'),
                '-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_float),C.POINTER(C.c_float)]
                rng=random.Random(0x2843c0)
                edges=[0.0,-0.0,2**-16,2**-23,float('nan'),float('inf'),-float('inf'),1.0,-1.0]
                for n in range(6000):
                    vals=[rng.uniform(-4,4) for _ in range(5)]
                    if n%3==0:vals[n%5]=edges[n%len(edges)]
                    v=(C.c_float*5)(*vals);a=(C.c_float*5)();b=(C.c_float*5)()
                    lib.run(0,v,a);lib.run(1,v,b)
                    self.assertEqual(bytes(a),bytes(b),f'case {n}: {vals}')
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
