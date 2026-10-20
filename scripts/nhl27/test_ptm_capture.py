"""Differential test of transition pose capture and callback publication order."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class CaptureTests(unittest.TestCase):
    def test_source_pose_capture(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x2804d0,0x280592,
            {0x280f20:'sample',0x6cdd20:'combine',0xa3f6540:'create_pose',
             0x17ffc60:'retain_pose',0x17fdc10:'release_pose',0x6cd9c0:'write_pose'},
             {},{0x28050a:'ff5058'})
        fixture=r'''
#include "ptm_transition.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original(struct ptm_transition*,void*);
static struct ptm_transition state;
static unsigned char controller[0x40],skeleton[0x28];
static panic_address vt[12],evaluation[2];
static unsigned mode,count,bad,events[32];
static void event(unsigned e){events[count++]=e;}
int _fltused=0;
ABI panic_address sample(struct ptm_transition*s,void*eval,float time){
 union{float f;unsigned u;} bits={time};event(1);event(bits.u);
 if(s!=&state || eval!=evaluation)bad++;
 return 101;
}
ABI panic_address evaluate(void*c,void*eval,float time){
 if(c!=controller || eval!=evaluation || time!=0)bad++;event(2);return 202;
}
ABI panic_address combine(void*arena,void*properties,panic_address prior,panic_address clip){
 if(arena!=(void*)evaluation[0] || properties!=controller+0x38 || prior!=202 || clip!=101)bad++;
 event(3);return 303;
}
ABI void *create_pose(void*skel,unsigned first,unsigned second){
 if(skel!=(void*)9 || first!=1 || second)bad++;event(4);return (void*)(panic_address)(mode%3);
}
ABI void retain_pose(void*p){event(10+(unsigned)(panic_address)p);if(mode&16)state.pose=(void*)7;}
ABI void release_pose(void*p){event(20+(unsigned)(panic_address)p);if(mode&32)state.pose=(void*)8;}
ABI panic_address write_pose(void*arena,panic_address handle,void*p){
 if(arena!=(void*)evaluation[0] || handle!=303)bad++;event(30+(unsigned)(panic_address)p);return 404;
}
static panic_address sample_adapter(void*c,struct ptm_transition*s,void*e,float t){return sample(s,e,t);}
static panic_address evaluate_adapter(void*c,void*controller0,void*e){return evaluate(controller0,e,0);}
static panic_address combine_adapter(void*c,void*e,void*controller0,panic_address prior,panic_address clip){
 return combine((void*)*(panic_address*)e,(unsigned char*)controller0+0x38,prior,clip);
}
static void *create_adapter(void*c,void*controller0){return create_pose(*(void**)(*(unsigned char**)((unsigned char*)controller0+0x30)+0x20),1,0);}
static void retain_adapter(void*c,void*p){retain_pose(p);}
static void release_adapter(void*c,void*p){release_pose(p);}
static panic_address write_adapter(void*c,void*e,panic_address handle,void*p){return write_pose((void*)*(panic_address*)e,handle,p);}
static const struct ptm_capture_services services={0,sample_adapter,evaluate_adapter,combine_adapter,create_adapter,retain_adapter,release_adapter,write_adapter};
__declspec(dllexport) void run(unsigned m,unsigned which,unsigned*out,unsigned time_bits,unsigned duration_bits){
 mode=m;count=bad=0;
 for(unsigned n=0;n<sizeof(state);n++)((unsigned char*)&state)[n]=0x55;
 vt[11]=(panic_address)evaluate;*(void**)controller=vt;*(void**)(controller+0x30)=skeleton;*(void**)(skeleton+0x20)=(void*)9;
 evaluation[0]=123;evaluation[1]=456;
 state.controller=controller;state.pose=(void*)(panic_address)((m>>2)%3);
 *(unsigned*)&state.clip_time=time_bits;*(unsigned*)&state.duration=duration_bits;
 if(which)nhl27_ptm_transition_capture(&state,evaluation,&services);else original(&state,evaluation);
 for(unsigned n=0;n<32;n++)out[n]=n<count?events[n]:0;
 out[32]=count;out[33]=bad;out[34]=(unsigned)(panic_address)state.pose;out[35]=(unsigned)state.pose_handle;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-capture-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_transition.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint),C.c_uint,C.c_uint]
                values=[0,0x80000000,0x3f800000,0xbf800000,0x7f800000,0xff800000,0x7fc01234,0x00000001]
                for mode in range(64):
                    for time in values:
                        for duration in values:
                            original=(C.c_uint*36)();ported=(C.c_uint*36)()
                            lib.run(mode,0,original,time,duration);lib.run(mode,1,ported,time,duration)
                            self.assertEqual(bytes(original),bytes(ported),(mode,time,duration))
                            self.assertEqual(original[33],0)
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
