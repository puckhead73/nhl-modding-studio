"""Original transition blend, trajectory and pose-reimport scheduling oracle."""
import ctypes as C
import os
from pathlib import Path
import random
import struct
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class EvaluateTests(unittest.TestCase):
    def test_original_transition_evaluation(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x27fdb0,0x27ffa1,
             {0x17ffc60:'retain_pose',0xa3f9b60:'view_create',0x6c2d50:'import_pose',
              0xa3f9e90:'view_release',0x17fdc10:'release_pose',0x69fb60:'curve_original',
              0x6d12e0:'partial',0x6c2af0:'complete',0x19fa70:'trajectory',0x2805a0:'prepare',0xbfabea0:'fail'},
             {0xec2d9f0:'guard_pointer',0xf298910:'invalid'})
        code+=assembly(image,'curve_original',0x69fb60,0x69fbde,{}, {0xdf1f1a0:'curve_table'})
        code+='\n.balign 16\ncurve_table:\n.byte '+','.join(str(b) for b in image.read(0xdf1f1a0,252))+'\n'
        fixture=r'''
#include "ptm_transition.h"
#define ABI __attribute__((sysv_abi))
typedef panic_address A;
extern ABI A original(struct ptm_transition*,A,void*,void*,float);
extern ABI float curve_original(unsigned,float);
static struct ptm_transition state;
static unsigned char match[0x48],selection[0x18],db[0xd0],clip[0x40],frames[0x10],pose[0x28];
static void *clips[1];static A evaluation[2];
static unsigned mode,bad,count,events[40];
A guard=1,*guard_pointer=&guard,invalid;
int _fltused=0;
static void event(unsigned e){events[count++]=e;}
static unsigned bits(float f){union{float f;unsigned u;} v={f};return v.u;}
ABI void fail(void){__builtin_trap();}
ABI void retain_pose(void*p){(*(int*)((unsigned char*)p+0x18))++;}
ABI void release_pose(void*p){(*(int*)((unsigned char*)p+0x18))--;}
ABI void view_create(void*out,void*in,unsigned x,unsigned y){*(void**)out=in;if(x||y)bad++;}
ABI void view_release(void*p){(void)p;}
ABI A import_pose(void*arena,void*view){if(arena!=(void*)evaluation[0] || *(void**)view!=pose)bad++;event(1);return 100;}
static void check_blend(void*arena,void*properties,A input,A captured){
 if(arena!=(void*)evaluation[0] || properties!=match+0x38 || input!=123 || captured!=state.pose_handle)bad++;
 if(mode&16)state.feature_flag=1;if(mode&32)state.feature_flag=2;
 invalid=999;
}
ABI A partial(void*arena,void*properties,A input,A captured,float weight){check_blend(arena,properties,input,captured);event(2);event(bits(weight));return 200;}
ABI A complete(void*arena,void*properties,A input,A captured){check_blend(arena,properties,input,captured);event(3);return 300;}
ABI A trajectory(void*arena,void*properties,A handle,const float*move,const float*rotation,unsigned flags,float weight,float other){
 if(arena!=(void*)evaluation[0] || properties!=match+0x38 || (handle!=200&&handle!=300) || flags || other || rotation[0] || rotation[1] || rotation[2] || rotation[3]!=1)bad++;
 event(4);event(bits(weight));for(unsigned n=0;n<4;n++)event(bits(move[n]));return 400;
}
ABI void prepare(struct ptm_transition*s,void*eval){if(s!=&state || eval!=evaluation || s->pose_handle!=invalid)bad++;event(5);}
static A invalid_handle(void*c){return invalid;}
static A import_adapter(void*c,void*eval,void*p){
 void *view[2]={p,*(void**)((unsigned char*)p+0x20)};retain_pose(p);
 A result=import_pose(*(void**)eval,view);release_pose(p);return result;
}
static float position(void*c,const void*sel){return *(float*)(frames+4)+*(const float*)((const unsigned char*)sel+0x10);}
static float curve(void*c,float value,unsigned id){return curve_original(id,value);}
static A complete_adapter(void*c,void*eval,void*m,A input,A capture){return complete(*(void**)eval,(unsigned char*)m+0x38,input,capture);}
static A partial_adapter(void*c,void*eval,void*m,A input,A capture,float w){return partial(*(void**)eval,(unsigned char*)m+0x38,input,capture,w);}
static A trajectory_adapter(void*c,void*eval,void*m,A handle,const float*move,float w){const float rotation[4]={0,0,0,1};return trajectory(*(void**)eval,(unsigned char*)m+0x38,handle,move,rotation,0,w,0);}
static void prepare_adapter(void*c,struct ptm_transition*s,void*eval){prepare(s,eval);}
static const struct ptm_evaluate_services services={0,invalid_handle,import_adapter,position,curve,complete_adapter,partial_adapter,trajectory_adapter,prepare_adapter};
__declspec(dllexport) void run(unsigned m,unsigned which,const unsigned*values,unsigned*out){
 mode=m;bad=count=0;invalid=777;
 state.match=match;state.pose=(m&1)?pose:0;state.pose_handle=(m&2)?invalid:100;
 state.feature_flag=(m>>2)%3;state.curve=values[5];
 *(unsigned*)&state.duration=values[0];*(unsigned*)&state.clip_time=values[1];
 *(unsigned*)(frames+4)=values[2];*(unsigned*)(selection+0x10)=values[3];
 *(unsigned*)selection=0;*(void**)(selection+8)=db;*(void**)(db+0xc8)=clips;clips[0]=clip;*(void**)(clip+0x38)=frames;
 *(int*)(pose+0x18)=3;*(void**)(pose+0x20)=(void*)456;
 for(unsigned n=0;n<4;n++)state.target[n]=(float)n-1.25f;
 evaluation[0]=44;evaluation[1]=55;union{unsigned u;float f;} delta={values[4]};
 A result=which?nhl27_ptm_transition_evaluate(&state,evaluation,selection,123,delta.f,&services):original(&state,123,selection,evaluation,delta.f);
 for(unsigned n=0;n<40;n++)out[n]=n<count?events[n]:0;
 out[40]=bad;out[41]=(unsigned)state.pose_handle;out[42]=(unsigned)result;out[43]=*(unsigned*)(pose+0x18);
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-evaluate-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_transition.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint),C.POINTER(C.c_uint)]
                rng=random.Random(270513)
                for case in range(6000):
                    mode=rng.randrange(64)
                    floats=[rng.uniform(.01,5),rng.uniform(-2,2),rng.uniform(-2,2),rng.uniform(-2,2),rng.uniform(-.2,.2)]
                    values=[struct.unpack('<I',struct.pack('<f',v))[0] for v in floats]
                    curve=rng.randrange(4)
                    if case<1000:
                        values[case%5]=(0,0x80000000,0x7fc01234,0x7f800000,0xff800000)[case//5%5];curve=3
                    inp=(C.c_uint*6)(*values,curve);original=(C.c_uint*44)();ported=(C.c_uint*44)()
                    lib.run(mode,0,inp,original);lib.run(mode,1,inp,ported)
                    self.assertEqual(bytes(original),bytes(ported),(case,mode,values,curve))
                    self.assertEqual(original[40],0)
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
