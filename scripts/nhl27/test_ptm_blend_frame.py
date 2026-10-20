"""Verify the blend frame's retained pose, selection timing and final clock."""
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


class BlendFrameTests(unittest.TestCase):
    def test_frame_setup_and_original_finish(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        image=checked_image(ElfImage,Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'),SOURCE_HASH)
        code='.intel_syntax noprefix\n'+assembly(image,'finish_original',0x284dea,0x284e6d,{}, {},
            indirect_calls={0x284e43:'ff90c8000000'})
        code=code.replace('finish_original:\n','''finish_original:
push rbp
push r15
push r14
sub rsp,0x3980
lea rbp,[rsp+0x3980]
mov r15,rdi
vmovss [rbp-0x3974],xmm0
''',1)
        code+='add rsp,0x3980\npop r14\npop r15\npop rbp\nret\n'
        code+=assembly(image,'remaining_original',0x284340,0x284351,{}, {})
        fixture=r'''
#include "ptm_blend_storage.h"
#include "ptm_lifecycle.h"
typedef panic_address A;typedef panic_u8 B;
#define ABI PTM_GAME_ABI
int _fltused=0;
static B blend[0x330],child[0x30],frame[0x400],asset[0x80],match[0x17d0],pose;
static A vtable[26],evaluation[2];static unsigned events,error;
extern ABI void finish_original(void*,float);
extern ABI float remaining_original(void*);
extern ABI void nhl24_ptm_blend_begin_frame_guest(A,void*,void*,void*,float);
extern ABI void nhl24_ptm_blend_finish_frame_guest(A,void*);
extern ABI void nhl24_ptm_blend_speed_frame_guest(A,void*,void*,float);
extern ABI void nhl24_ptm_blend_pre_child_guest(A,void*,void*);
ABI void nhl24_ptm_blend_snapshots_guest(A base,void*b,void*ev,int mode,B force){
 if(base!=123||b!=blend||ev!=evaluation||mode||force)error|=1;
 events=events*10+1;((struct ptm_blend_extension*)(blend+0x2e0))->initialized=1;
}
void nhl24_ptm_pose_retain(void*p){if(p!=&pose)error|=2;events=events*10+2;}
ABI void nhl24_ptm_blend_warp_guest(A base,void*m,void*context,void*extra,float remaining){
 (void)extra;(void)remaining;if(base!=123||m!=match||context!=(void*)456)error|=4;
 events=events*10+3;
}
ABI void nhl24_ptm_start_transition_guest(A base,struct ptm_transition**slot,void*m,void*ev,
 const void*s,void*controller,const float*target,float remaining,const B*flag,void*extra){
 (void)remaining;(void)extra;
 struct ptm_match_extension *me=(void*)(match+0x1740);
 if(base!=123||slot!=&me->transition||m!=match||ev!=evaluation||s!=match+0x618||
 controller!=(void*)789||target!=(float*)(blend+0x300)||flag!=&me->trajectory_flag)error|=8;
 events=events*10+4;*slot=(void*)1;
}
__declspec(dllexport) unsigned setup(unsigned initialized,unsigned transition,float remaining){
 for(unsigned i=0;i<sizeof(blend);i++)blend[i]=0;
 for(unsigned i=0;i<sizeof(match);i++)match[i]=0;
 for(unsigned i=0;i<sizeof(asset);i++)asset[i]=0;
 for(unsigned i=0;i<sizeof(frame);i++)frame[i]=0;
 events=error=0;B*f=frame+sizeof(frame);
 struct ptm_blend_extension *e=(void*)(blend+0x2e0);
 e->clock=(struct ptm_blend_clock){100,0,40,6,10,0,1};e->pose_b=&pose;e->initialized=(B)initialized;
 *(void**)(blend+0x2a8)=asset;*(void**)(blend+0x2c0)=match;*(void**)(blend+0x2b8)=(void*)789;
 evaluation[1]=456;
 nhl24_ptm_blend_begin_frame_guest(123,blend,evaluation,f,.5f);
 if(events!=(initialized?2u:12u)||e->frame_delta!=.5f||e->clock.duration!=40||e->clock.elapsed!=0||
 *(float*)(blend+0x2b0)!=40||*(float*)(blend+0x2c8)!=6||*(float*)(blend+0x2cc)!=10||
 *(float*)(f-0x350)!=.5f||*(float*)(f-0x380)!=0||*(float*)(f-0x354)!=40||
 *(void**)(f-0x340)!=(void*)456||*(void**)(f-0x3b0)!=evaluation||*(void**)(f-0x360)!=&pose)error|=16;
 events=0;*(float*)(f-0x380)=remaining;
 struct ptm_match_extension*me=(void*)(match+0x1740);me->transition=transition?(void*)1:0;
 nhl24_ptm_blend_pre_child_guest(123,blend,f);
 if(events!=((remaining<=10&&!transition)?34u:3u))error|=32;
 return error;
}
__declspec(dllexport) void finish(unsigned which,const float*v,float*out){
 for(unsigned i=0;i<sizeof(blend);i++)blend[i]=0;
 for(unsigned i=0;i<6;i++)((float*)(child+0x18))[i]=v[i+3];
 *(void**)(blend+0x50)=child;
 if(which){
  struct ptm_blend_extension*e=(void*)(blend+0x2e0);
  e->clock.duration=v[0];e->clock.elapsed=v[1];e->frame_delta=v[2];
  nhl24_ptm_blend_finish_frame_guest(123,blend);out[6]=e->clock.elapsed;
 }else{
  vtable[25]=(A)remaining_original;*(A*)blend=(A)vtable;
  *(float*)(blend+0x2c0)=v[0];*(float*)(blend+0x2c4)=v[1];
  finish_original(blend,v[2]);out[6]=*(float*)(blend+0x2c4);
 }
 for(unsigned i=0;i<6;i++)out[i]=((float*)(blend+0x18))[i];
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-blend-frame-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'oracle.s').write_text(code)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.c'),str(p/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_blend_frame.c'),
                str(root/'scripts/nhl27/runtime/ptm_blend_timing.c'),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(p/'fixture.dll'));F=C.POINTER(C.c_float)
            try:
                lib.setup.argtypes=[C.c_uint,C.c_uint,C.c_float]
                for initialized in range(3):
                    for transition in range(2):
                        for remaining in [-1,0,5,10,10.01,float('nan'),float('inf')]:
                            self.assertEqual(lib.setup(initialized,transition,remaining),0)
                lib.finish.argtypes=[C.c_uint,F,F];rng=random.Random(0x284dea)
                edges=[0.,-0.,float('nan'),float('inf'),-float('inf')]
                for n in range(6000):
                    values=[rng.uniform(-100,100) for _ in range(9)]
                    if n%3==0:values[n%9]=edges[n%5]
                    v=(C.c_float*9)(*values);a=(C.c_float*7)();b=(C.c_float*7)()
                    lib.finish(0,v,a);lib.finish(1,v,b)
                    self.assertEqual(bytes(a),bytes(b),(n,values,list(a),list(b)))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
