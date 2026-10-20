"""Path selection and future-pose sampling before PTM transition creation."""
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
from ptm_trajectory_math import generate


class PathTests(unittest.TestCase):
    def test_original_path_selection_and_sampling(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private image and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH);code,_=generate(image)
        code+=assembly(image,'select_original',0x23a640,0x23a694,{}, {})
        raw=assembly(image,'sample_original',0x280022,0x28018e,{}, {})
        raw=raw.replace('sample_original:\n','''sample_original:
push rbp
mov rbp,rsp
push r14
sub rsp,0x58
lea r14,[rdi-0x9200]
mov rax,rsi
vmovaps xmm11,xmm0
vmovss [rbp-0x48],xmm0
''',1).replace('.balign 16','add rsp,0x58\npop r14\npop rbp\nret\n.balign 16',1)
        code+=raw
        fixture=r'''
#include "ptm_native.h"
#define ABI __attribute__((sysv_abi))
typedef unsigned char B;typedef panic_address A;
extern ABI void *select_original(void*);
extern ABI void sample_original(float*,void*,float);
extern ABI void nhl27_ptm_sample_path(float*,void*,float);
extern void *nhl24_ptm_selected_path(void*,void*);
extern void nhl24_ptm_prepare_target_path(void*,float,void*);
static B source_match[0x8800],native_match[0x1740],source_path[0x488],native_path[0x70];
static B source_records[0x4d0*8],native_records[0x80*8],source_selector[0x40],native_selector[0x20];
static float samples[65*4];int _fltused=0;
__declspec(dllexport) void sample(unsigned which,unsigned count,const float*data,float period,float offset,float time,float*out){
 for(unsigned n=0;n<65*4;n++)samples[n]=data[n];
 B *path=which?native_path:source_path;
 *(void**)(path+0x10)=samples;*(void**)(path+0x18)=samples+count*4;
 *(float*)(path+(which?0x50:0x480))=period;*(float*)(path+(which?0x54:0x484))=offset;
 if(which)nhl27_ptm_sample_path(out,path,time);else sample_original(out,path,time);
}
__declspec(dllexport) unsigned select(unsigned which,int category,unsigned present,unsigned fallback){
 B *match=which?native_match:source_match;B *selector=which?native_selector:source_selector;
 *(void**)(match+(which?0x11a8:0x8668))=present?selector:0;
 *(int*)(selector+(which?0x18:0x38))=category;
 *(void**)(match+(which?0x330:0x31c0))=which?native_records:source_records;
 for(unsigned n=0;n<(which?4u:5u);n++)((int*)(match+(which?0x320:0x31a0)))[n]=(fallback&(1u<<n))?-1:(int)n;
 B *result=which?nhl24_ptm_selected_path(match,(fallback&16)?0:native_records+4*0x80+0x10):select_original(match);
 if(result==match+(which?0x13c0:0x8720))return 99;
 return (unsigned)((result-(which?native_records:source_records)-0x10)/(which?0x80:0x4d0));
}
__declspec(dllexport) void cached(unsigned mode,float*out){
 *(void**)(native_match+0x11a8)=0;*(int*)(native_match+0x320)=0;
 *(void**)(native_match+0x330)=native_records;
 B *path=native_records+0x10;
 *(void**)(path+0x10)=samples;*(void**)(path+0x18)=samples+8;
 for(unsigned n=0;n<12;n++)samples[n]=(float)n;
 *(float*)(path+0x50)=1;*(float*)(path+0x54)=0;
 float *cache=(float*)(native_match+0x1440);
 for(unsigned n=0;n<4;n++)cache[n]=0x1.fffffep127f;
 if(mode){union{unsigned u;float f;} value={mode==1?0:0x7fc00001};cache[mode%4]=value.f;}
 nhl24_ptm_prepare_target_path(native_match,0.5f,0);
 for(unsigned n=0;n<4;n++)out[n]=cache[n];
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-path-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build',ignore_cleanup_errors=True) as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            r=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'oracle.s'),
                *[str(root/'scripts/nhl27/runtime'/n) for n in ['ptm_transition.c','ptm_native.c','ptm_offsets.c','ptm_trajectory.c','ptm_integration.c']],
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(out/'test.dll'));F=C.POINTER(C.c_float)
            lib.sample.argtypes=[C.c_uint,C.c_uint,F,C.c_float,C.c_float,C.c_float,F]
            lib.select.argtypes=[C.c_uint,C.c_int,C.c_uint,C.c_uint];lib.select.restype=C.c_uint
            lib.cached.argtypes=[C.c_uint,F]
            rng=random.Random(0x280022)
            for n in range(6000):
                values=(C.c_float*260)(*[rng.uniform(-100,100) for _ in range(260)])
                count=n%65;period=[0.01,0.1,0.5,1,2][n%5];offset=rng.uniform(-2,2);time=rng.uniform(-2,70)
                if n%23==0:time=[float('nan'),float('inf'),-float('inf'),-0.0][n%4]
                x=(C.c_float*4)();y=(C.c_float*4)()
                lib.sample(0,count,values,period,offset,time,x);lib.sample(1,count,values,period,offset,time,y)
                self.assertEqual(bytes(x),bytes(y),f'sample {n}')
            for cat in range(-8,13):
                for present in range(2):
                    for mask in range(32):self.assertEqual(lib.select(0,cat,present,mask),lib.select(1,cat,present,mask))
            result=(C.c_float*4)();lib.cached(0,result)
            expected=(C.c_float*4)();data=(C.c_float*260)(*range(12))
            lib.sample(0,2,data,1,0,0.5,expected)
            self.assertEqual(bytes(result),bytes(expected))
            lib.cached(1,result);self.assertEqual(result[1],0)
            lib.cached(2,result);self.assertNotEqual(result[2],result[2])
            import _ctypes
            _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
