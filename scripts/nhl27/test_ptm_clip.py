"""Source clip-sampling oracle against native clip/controller interface adapters."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class ClipTests(unittest.TestCase):
    def test_original_sampling_and_mirror_scope(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x280f20,0x2810d0,
            {0x74e2c0:'lookup',0x17ffc60:'retain_pose',0xa3f97b0:'properties_copy',
             0x835bf0:'mirror_create',0xa3f9e90:'properties_release',0x17fdc10:'release_pose',0xbfabea0:'fail'},
            {0xec2d9f0:'guard_pointer',0xf28b080:'mirror_key'},
            {0x280fe0:'ff5020',0x280ff4:'ff5010',0x281046:'ff5028',0x281058:'ff5058',0x281070:'ff5028'})
        fixture=r'''
#include "ptm_native.h"
#define ABI __attribute__((sysv_abi))
typedef panic_address A;
extern ABI A original(struct ptm_transition*,void*,float);
static struct ptm_transition state;
static unsigned char match[0x48],clip[0x30],asset[0x30],a[0x20],b[0x20],pose[0x20],properties[0x10];
static A controller_vt[12],seek_vt[3],asset_vt[10],pose_vt[4],properties_vt[3],evaluation[2];
static unsigned word,mode,count,bad,events[32];
A mirror_key=123,guard=456,*guard_pointer=&guard;
int _fltused=0;
static void event(unsigned e){events[count++]=e;}
ABI void fail(void){__builtin_trap();}
ABI unsigned *lookup(void*c,A key,unsigned create){if(c!=(void*)evaluation[1] || key!=123 || create!=1)bad++;return &word;}
ABI void retain_pose(void*p){(*(int*)((unsigned char*)p+0x18))++;}
ABI void release_pose(void*p){(*(int*)((unsigned char*)p+0x18))--;}
ABI void properties_copy(void*out,void*in,unsigned x,unsigned y){
 ((void**)out)[0]=((void**)in)[0];((void**)out)[1]=((void**)in)[1];
 if(x || y)bad++;if(*(void**)out)(*(int*)((unsigned char*)*(void**)out+8))++;
}
ABI void properties_release(void*p){if(*(void**)p)(*(int*)((unsigned char*)*(void**)p+8))--;}
static void verify_snapshot(void*s){
 void **snapshot=s;
 if(snapshot[0]!=((mode&4)?pose:0) || snapshot[1]!=((mode&8)?properties:0) || snapshot[2]!=(void*)999 || snapshot[3] || snapshot[4]!=(void*)evaluation[1])bad++;
}
ABI void create_controller(void**out,void*receiver,void*snapshot){
 if(receiver!=asset && receiver!=asset+0x28)bad++;verify_snapshot(snapshot);
 event(1);*out=a;*(int*)(a+8)=1;
}
ABI void seek(void*controller,float time){
 if(controller!=a+0x10)bad++;union{float f;unsigned u;} bits={time};event(2);event(bits.u);
 *(unsigned short*)&word=(unsigned short)(word+(mode>>4)%3);
 if(mode&128)state.mirrored=1;
 if(mode&256)state.mirrored=2;
}
ABI void destroy_controller(void*c){event(c==a?10:11);}
ABI A evaluate(void*c,void*e,float time){
 if(e!=evaluation || time!=0)bad++;event(c==a?20:21);return 777;
}
ABI void *mirror_create(void*c,void*snapshot,void*unused){
 if(c!=a || unused)bad++;verify_snapshot(snapshot);event(3);
 return (mode&64)?a:b;
}
ABI void begin_scope(struct ptm_mirror_scope*s,void*c,panic_u8 mirrored){
 s->word=lookup(c,mirror_key,1);s->saved=word;s->reset_count=(word&0xffff)==0;
 *(unsigned short*)&word=(unsigned short)(word+1);
 ((unsigned char*)&word)[2]=mirrored;((unsigned char*)&word)[3]=1;
}
static const struct ptm_native_clip_services services={begin_scope,mirror_create};
__declspec(dllexport) void run(unsigned m,unsigned which,unsigned initial,unsigned*out){
 mode=m;count=bad=0;word=initial;
 controller_vt[4]=controller_vt[5]=(A)destroy_controller;controller_vt[11]=(A)evaluate;seek_vt[2]=(A)seek;
 asset_vt[4]=asset_vt[9]=(A)create_controller;
 *(void**)a=*(void**)b=controller_vt;*(void**)(a+0x10)=*(void**)(b+0x10)=seek_vt;
 *(int*)(a+8)=*(int*)(b+8)=0;*(int*)(pose+0x18)=3;*(int*)(properties+8)=4;
 *(void**)pose=pose_vt;*(void**)properties=properties_vt;
 *(void**)asset=asset_vt;*(void**)(asset+0x28)=asset_vt;
 *(void**)(clip+8)=*(void**)(clip+0x28)=asset;
 *(void**)(match+0x30)=(m&4)?pose:0;*(void**)(match+0x38)=(m&8)?properties:0;*(void**)(match+0x40)=(void*)999;
 state.match=match;state.clip=clip;state.mirrored=m%3;
 evaluation[0]=77;evaluation[1]=88;
 A result=which?nhl24_ptm_sample_clip(match,clip,evaluation,1.25f,&state.mirrored,&services):original(&state,evaluation,1.25f);
 for(unsigned n=0;n<32;n++)out[n]=n<count?events[n]:0;
 out[32]=bad;out[33]=word;out[34]=*(unsigned*)(a+8);out[35]=*(unsigned*)(b+8);
 out[36]=*(unsigned*)(pose+0x18);out[37]=*(unsigned*)(properties+8);out[38]=(unsigned)result;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-clip-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_native.c'),
                str(root/'scripts/nhl27/runtime/ptm_transition.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.c_uint,C.POINTER(C.c_uint)]
                for mode in range(512):
                    for word in (0,1,2,0xffff,0x1234fffe,0xaabbffff):
                        original=(C.c_uint*39)();ported=(C.c_uint*39)()
                        lib.run(mode,0,word,original);lib.run(mode,1,word,ported)
                        self.assertEqual(bytes(original),bytes(ported),(mode,word))
                        self.assertEqual(original[32],0)
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
