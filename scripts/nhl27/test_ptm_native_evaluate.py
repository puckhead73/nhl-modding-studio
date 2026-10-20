"""Native clip-position traversal and byte-exact source/native blend curves."""
import ctypes as C
import os
from pathlib import Path
import random
import struct
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH,TARGET_HASH
from research import ElfImage,SelfImage
from oracle_fixture import assembly


class NativeEvaluateTests(unittest.TestCase):
    def test_native_selection_and_curves(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists() or not target.exists():
            self.skipTest('Private source/target and Windows LLVM required')
        src=checked_image(ElfImage,source,SOURCE_HASH);dst=checked_image(SelfImage,target,TARGET_HASH)
        self.assertEqual(src.read(0xdf1f1a0,252),dst.read(0x95eab70,252))
        code=assembly(src,'source_curve',0x69fb60,0x69fbde,{}, {0xdf1f1a0:'curve_table'})
        code+=assembly(dst,'native_curve',0x753c20,0x753c9e,{}, {0x95eab70:'curve_table'})
        code+='\n.balign 16\ncurve_table:\n.byte '+','.join(str(b) for b in src.read(0xdf1f1a0,252))+'\n'
        # Original native evaluator's selected-frame address calculation.
        code+='\n.text\n.globl original_position\noriginal_position:\npush %rbx\nsub $0x280,%rsp\nmov %rsi,%rbx\nmovdqu (%rdi),%xmm0\nmovdqu %xmm0,0x260(%rsp)\nmovdqu 16(%rdi),%xmm0\nmovdqu %xmm0,0x270(%rsp)\n'
        code+=assembly(dst,'position_body',0x31187e,0x3118bb,{}, {})
        code+='add $0x280,%rsp\npop %rbx\nret\n'
        fixture=r'''
#include "ptm_native.h"
#define ABI __attribute__((sysv_abi))
extern ABI float source_curve(unsigned,float);
extern ABI float native_curve(unsigned,float);
extern ABI float original_position(const void*,void*);
extern float nhl24_ptm_clip_position(const void*);
static unsigned char selection[0x20],database[0xb8],clip0[0x20],clip1[0x20],frames[2][19][16];
static void *clips[2],*dummy;
int _fltused=0;
__declspec(dllexport) unsigned curves(unsigned id,unsigned value){
 union {unsigned u;float f;} input={value},a={0},b={0};
 a.f=source_curve(id,input.f);b.f=native_curve(id,input.f);return a.u==b.u;
}
__declspec(dllexport) unsigned position(unsigned index,unsigned frame,unsigned time,unsigned offset){
 *(unsigned*)selection=index;*(unsigned*)(selection+4)=frame;*(void**)(selection+0x10)=database;*(unsigned*)(selection+0x1c)=offset;
 *(void**)(database+0xb0)=clips;clips[0]=clip0;clips[1]=clip1;
 *(void**)(clip0+0x18)=frames[0];*(void**)(clip1+0x18)=frames[1];
 *(unsigned*)(frames[index][frame]+4)=time;
 union{unsigned u;float f;} a,b;
 a.f=original_position(selection,&dummy);b.f=nhl24_ptm_clip_position(selection);return a.u==b.u;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-native-eval-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_native.c'),
                str(root/'scripts/nhl27/runtime/ptm_transition.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                rng=random.Random(75320)
                for case in range(12000):
                    value=rng.uniform(0,1.5);bits=struct.unpack('<I',struct.pack('<f',value))[0]
                    self.assertEqual(lib.curves(case%4,bits),1,(case,value))
                    self.assertEqual(lib.position(case%2,case%19,rng.getrandbits(32),rng.getrandbits(32)),1,case)
                for bits in (0,0x80000000,0x7f800000,0xff800000,0x7fc01234):
                    self.assertEqual(lib.curves(3,bits),1)
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
