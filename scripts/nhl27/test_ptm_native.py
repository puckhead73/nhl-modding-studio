"""Native pose allocation and controller/pose reference-count ABI checks."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,TARGET_HASH
from research import SelfImage
from oracle_fixture import assembly


class NativePtmTests(unittest.TestCase):
    def test_native_pose_services(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not target.exists():
            self.skipTest('Private target and Windows LLVM required')
        image=checked_image(SelfImage,target,TARGET_HASH)
        code=assembly(image,'original_create',0x8bb850,0x8bb927,
             {0x8ec6000:'zero_memory',0x8bb3b0:'construct_pose'},
             {0x94eb4f4:'pose_label'},{0x8bb8ae:'ff5018'})
        fixture=r'''
#include "ptm_transition.h"
#define ABI __attribute__((sysv_abi))
typedef panic_address A;
extern ABI void *original_create(void*,void*);
extern void *nhl24_ptm_pose_create(void*,void*,void*(ABI*)(void*,void*));
extern void nhl24_ptm_pose_retain(void*);
extern void nhl24_ptm_pose_release(void*);
extern void nhl24_ptm_controller_destroy(void*);
extern A nhl24_ptm_controller_evaluate(void*,void*);
extern A nhl24_ptm_pose_combine(void*,void*,A,A,A(ABI*)(void*,A,A,unsigned));
extern A nhl24_ptm_pose_import(void*,void*,void(ABI*)(void*,void*,void*),A(ABI*)(void*,void*));
static unsigned char controller[0x48],owner[0x28],skeleton[0xb1],layout[0x10],pose[4096],properties[0xa0],mapping[8];
static A allocator_vt[4],controller_vt[12],pose_vt[4],*allocator=allocator_vt;
static A evaluation[2];
static unsigned bad,allocated,pose_frees,controller_frees,eval_calls,combine_calls;
static unsigned char view_properties[0x10];static A view_vt[3];static unsigned view_frees;
const char pose_label[]="Pose";
int _fltused=0;
ABI void *allocate(void*a,A size,const char*label,unsigned kind,unsigned alignment,unsigned zero){
 if(a!=&allocator || size>sizeof(pose) || label!=pose_label || kind!=1 || alignment!=16 || zero)bad++;
 allocated=(unsigned)size;return pose;
}
ABI void *zero_memory(void*p,unsigned value,A bytes){
 if(p!=pose || bytes!=allocated || value)bad++;
 for(unsigned n=0;n<bytes && n<sizeof(pose);n++)((unsigned char*)p)[n]=(unsigned char)value;
 return p;
}
ABI void pose_destroy(void*p){if(p!=pose)bad++;pose_frees++;}
ABI void controller_destroy(void*p){if(p!=controller)bad++;controller_frees++;}
ABI void construct_pose(void*p){if(p!=pose)bad++;*(void**)p=pose_vt;*(int*)(pose+0x18)=0;}
ABI A evaluate(void*c,void*e,float t){if(c!=controller || e!=evaluation || t!=0)bad++;eval_calls++;return 123;}
ABI A combine(void*a,A prior,A clip,unsigned id){
 if(a!=(void*)evaluation[0] || prior!=123 || clip!=456 || id!=combine_calls*789u)bad++;
 combine_calls++;return 321;
}
ABI void view_destroy(void*p){if(p!=view_properties)bad++;view_frees++;}
ABI void make_view(void*out,void*descriptor,void*options){
 if(descriptor!=(void*)99 || options || *(int*)(pose+0x18)!=5)bad++;
 ((void**)out)[0]=view_properties;((void**)out)[1]=descriptor;
}
ABI A import_view(void*arena,void*view){
 if(arena!=(void*)evaluation[0] || ((void**)view)[0]!=pose || ((void**)view)[1]!=view_properties || ((void**)view)[2]!=(void*)99)bad++;
 return 505;
}
__declspec(dllexport) unsigned check(unsigned size,unsigned alignment,unsigned flag){
 bad=allocated=pose_frees=controller_frees=eval_calls=combine_calls=0;
 allocator_vt[3]=(A)allocate;pose_vt[3]=(A)pose_destroy;
 controller_vt[4]=(A)controller_destroy;controller_vt[11]=(A)evaluate;
 *(void**)controller=controller_vt;*(void**)(controller+0x30)=owner;*(void**)(owner+0x20)=skeleton;
 *(void**)(skeleton+0xa0)=layout;skeleton[0xb0]=(unsigned char)flag;
 *(unsigned*)(layout+8)=size;*(unsigned*)(layout+12)=alignment;
 if(nhl24_ptm_pose_create(controller,&allocator,original_create)!=pose)return 1;
 unsigned offset=(-alignment)&(alignment+0x2f);
 if(allocated!=((size+15+offset)&~15u) || *(unsigned short*)(pose+0x1c)!=(unsigned short)offset || pose[0x1e]!=(unsigned char)flag)return 2;
 nhl24_ptm_pose_retain(pose);nhl24_ptm_pose_retain(pose);
 if(*(int*)(pose+0x18)!=2 || *(unsigned short*)(pose+0x1c)!=(unsigned short)offset)return 3;
 nhl24_ptm_pose_release(pose);if(pose_frees || *(int*)(pose+0x18)!=1)return 4;
 nhl24_ptm_pose_release(pose);if(pose_frees!=1)return 5;
 nhl24_ptm_controller_destroy(controller);if(controller_frees!=1)return 6;
 evaluation[0]=77;evaluation[1]=88;
 if(nhl24_ptm_controller_evaluate(controller,evaluation)!=123 || eval_calls!=1)return 7;
 *(void**)(controller+0x40)=properties;*(void**)(properties+0x98)=0;
 if(nhl24_ptm_pose_combine(evaluation,controller,123,456,combine)!=321)return 8;
 *(void**)(properties+0x98)=mapping;*(unsigned*)(mapping+4)=789;
 if(nhl24_ptm_pose_combine(evaluation,controller,123,456,combine)!=321)return 9;
 nhl24_ptm_pose_retain(0);nhl24_ptm_pose_release(0);
 *(int*)(pose+0x18)=4;*(void**)(pose+0x20)=(void*)99;
 view_frees=0;view_vt[2]=(A)view_destroy;*(void**)view_properties=view_vt;*(int*)(view_properties+8)=1;
 if(nhl24_ptm_pose_import(evaluation,pose,make_view,import_view)!=505 || view_frees!=1 || *(int*)(pose+0x18)!=4)return 11;
 return bad?10:0;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-native-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_native.c'),
                str(root/'scripts/nhl27/runtime/ptm_transition.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                for size in (0,1,16,32,100,1000):
                    for alignment in (1,2,4,8,16,32,64):
                        for flag in (0,1,2,255):
                            self.assertEqual(lib.check(size,alignment,flag),0,(size,alignment,flag))
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
