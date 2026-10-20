"""Original transition cleanup and rig-vector getter, including reentrant releases."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class TransitionTests(unittest.TestCase):
    def test_original_cleanup_and_vector_count(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original_destruct',0x280440,0x2804cb,
            {0x17fdc10:'pose_release',0x8c2120:'vector_release'}, {},
            {0x280479:'ff5028'}, {0x2804c1:'ff6028'})
        code+=assembly(image,'original_clear',0x2803c0,0x280401,
            {0x280440:'original_destruct',0x8bbec0:'get_allocator'}, {},{0x2803ee:'ff5120'})
        code+=assembly(image,'original_offsets',0x280410,0x280440,{}, {})
        fixture=r'''
#include "ptm_transition.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original_clear(void*);
extern ABI void *original_offsets(void*);
struct controller {panic_address *vt;int references;int padding;};
static struct ptm_transition state;
static unsigned char match[0xbbc8],rig[0x2494];
static struct controller a,b;
static panic_address controller_vt[6],allocator_vt[5];
static panic_address *allocator=allocator_vt;
static unsigned mode,log[32],count,bad;
int _fltused=0;
static void event(unsigned n){if(count<32)log[count++]=n;else bad++;}
ABI void controller_destroy(void*c){event(c==&a?20:21);}
ABI void pose_release(void*p){event(p==(void*)1?10:11);}
ABI void vector_release(void*allocator_arg,panic_address begin,panic_address bytes){
 if(allocator_arg!=&state.allocator || begin!=state.begin || bytes!=state.capacity-state.begin)bad++;
 event(30);
 if(mode&32)state.pose=(void*)2;
 if(mode&64)state.controller=&b;
}
ABI void *get_allocator(void){return &allocator;}
ABI void free_state(void*allocator_arg,void*p,unsigned flags){
 if(allocator_arg!=&allocator || p!=&state || flags || *(void**)(match+0xbbc0)!=&state)bad++;
 event(40);
}
static void release_pose(void*c,void*p){pose_release(p);}
static void destroy_controller(void*c,void*p){controller_destroy(p);}
static void release_vector(void*c,void*a0,panic_address b0,panic_address n){vector_release(a0,b0,n);}
static void free_transition(void*c,struct ptm_transition*p){free_state(&allocator,p,0);}
static const struct ptm_transition_services services={0,release_pose,destroy_controller,release_vector,free_transition};
__declspec(dllexport) unsigned run(unsigned m,unsigned which,unsigned*out){
 mode=m;count=bad=0;
 for(unsigned n=0;n<sizeof(state);n++)((unsigned char*)&state)[n]=0x55;
 controller_vt[5]=(panic_address)controller_destroy;allocator_vt[4]=(panic_address)free_state;
 a.vt=b.vt=controller_vt;a.references=1+(m>>7)%3;b.references=1+(m>>9)%3;
 state.controller=(m&1)?&a:0;state.pose=(m&2)?(void*)1:0;
 state.begin=(m&4)?0:0x12340;state.end=0x12380;state.capacity=0x12440;
 state.inline_storage=(m&8)?state.begin:0;
 *(void**)(match+0xbbc0)=(m&16)?0:&state;
 if(which)nhl27_ptm_transition_clear((struct ptm_transition**)(match+0xbbc0),&services);
 else original_clear(match);
 for(unsigned n=0;n<32;n++)out[n]=n<count?log[n]:0;
 out[32]=a.references;out[33]=b.references;out[34]=bad;
 out[35]=state.controller==&a?1:state.controller==&b?2:0;
 out[36]=(unsigned)(panic_address)state.pose;out[37]=*(void**)(match+0xbbc0)!=0;
 return count;
}
__declspec(dllexport) unsigned vector_check(panic_address begin,panic_address end,int joints){
 *(void**)(match+0xbbc0)=&state;state.begin=begin;state.end=end;state.match_state=rig;
 *(int*)(rig+0x2490)=joints;
 return original_offsets(match)==(void*)nhl27_ptm_transition_offsets(&state,joints);
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-transition-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_transition.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint)]
                lib.vector_check.argtypes=[C.c_uint64,C.c_uint64,C.c_int]
                for case in range(2048):
                    original=(C.c_uint*38)();ported=(C.c_uint*38)()
                    self.assertEqual(lib.run(case,0,original),lib.run(case,1,ported),case)
                    self.assertEqual(bytes(original),bytes(ported),case)
                    self.assertEqual(original[34],0,case)
                    begin=(case*0x1000000001)&0xffffffffffffffff
                    for span in (0,16,32,0xfffffffffffffff0,0x1000000000):
                        end=(begin+span)&0xffffffffffffffff
                        self.assertEqual(lib.vector_check(begin,end,case%3),1,(case,span))
                self.assertEqual(lib.vector_check(0,0xfffffffffffffff0,-1),1)
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
