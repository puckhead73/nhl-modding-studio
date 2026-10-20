"""Compare native registry publication with the original panic reset allocation."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class LocatorTests(unittest.TestCase):
    def test_original_reference_publication_and_reentrant_slot_changes(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        src=checked_image(ElfImage,source,SOURCE_HASH)
        asm=assembly(src,'original_reset',0x4a544f0,0x4a5461a,
            {0x452e130:'actor_context',0x756bd0:'remove_locator',
             0x8bbec0:'allocator_get',0x756620:'construct'},
            {0xf28b0f8:'property',0xf20da00:'feature_property',0xde6c2a7:'label'},
            {0x4a54525:'ff90b0000000',0x4a5454d:'ff93b0000000',
             0x4a545a4:'41ff5218',0x4a545c7:'ff90c0000000',
             0x4a545f0:'ff5028',0x4a54609:'ff5028'})
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
typedef panic_address A;
typedef void (ABI *ctor)(void*);
extern void *nhl24_panic_locator_ensure(void*,void*,void*,ctor,const char*);
extern ABI void original_reset(void*);
A methods[25],feature_methods[25],object_methods[6],allocator_methods[4];
A property[4],feature_property[4],allocator[2];
const char label[]="LocatorRegistry";
static unsigned char fresh[0x770],old[0x770],actor[0x68],animation[0xc68],context_owner[0x28],context[0x100];
static void *slot,*empty;
static unsigned mode,log_entries[16],log_count,bad;
static void record(unsigned n){log_entries[log_count++]=n;}
ABI void destroy(void*p){record(p==old?5:6);}
ABI void **read_slot(void*p,void*c){(void)p;if(c!=context+0x68)bad++;record(1);return &slot;}
ABI void **write_slot(void*p,void*c){(void)p;if(c!=context+0x68)bad++;record(4);return &slot;}
ABI void **feature_read(void*p,void*c){(void)p;(void)c;return &empty;}
ABI void *actor_context(void*p){(void)p;return context;}
ABI void *allocator_get(void){return allocator;}
ABI void remove_locator(void*p,void*l){(void)p;(void)l;bad++;}
ABI void *allocate(void*p,A size,const char*name,unsigned flag,unsigned alignment,unsigned zero){
    if(p!=allocator || size!=0x770 || name!=label || flag!=1 || alignment!=8 || zero)bad++;
    record(2);return fresh;
}
ABI void construct(void*p){
    record(3);*(A**)p=object_methods;*(int*)((char*)p+8)=0;
    if(mode==2)slot=old;
    if(mode==3){slot=fresh;*(int*)(fresh+8)=1;}
}
__declspec(dllexport) void run(unsigned source,unsigned scenario,unsigned*out){
    mode=scenario;log_count=bad=0;empty=0;
    methods[0x78/8]=methods[0xb0/8]=(A)read_slot;
    methods[0x88/8]=methods[0xc0/8]=(A)write_slot;
    feature_methods[0xb0/8]=(A)feature_read;
    object_methods[4]=object_methods[5]=(A)destroy;
    allocator_methods[3]=(A)allocate;
    property[0]=(A)methods;feature_property[0]=(A)feature_methods;allocator[0]=(A)allocator_methods;
    *(A**)old=object_methods;*(int*)(old+8)=1;
    *(A**)fresh=object_methods;*(int*)(fresh+8)=0;
    slot=mode==1?old:0;
    if(source){
        *(void**)(actor+0x60)=animation;*(void**)(animation+0xc60)=context_owner;
        *(void**)(context_owner+0x20)=context;void *state=actor;original_reset(&state);
    }else nhl24_panic_locator_ensure(property,context+0x68,allocator,construct,label);
    out[0]=bad;out[1]=slot==old?1:slot==fresh?2:0;
    out[2]=*(unsigned*)(fresh+8);out[3]=*(unsigned*)(old+8);out[4]=log_count;
    for(unsigned n=0;n<log_count;n++)out[5+n]=log_entries[n];
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        scratch=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='panic-locator-',dir=scratch) as folder:
            out=Path(folder);(out/'oracle.s').write_text(asm);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared',
                '-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(repo/'scripts/nhl27/runtime/panic_services.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint)]
                for scenario in range(4):
                    original=(C.c_uint*32)();ported=(C.c_uint*32)()
                    lib.run(1,scenario,original);lib.run(0,scenario,ported)
                    self.assertEqual(list(ported),list(original),scenario)
                    self.assertEqual(ported[0],0)
                    self.assertEqual(ported[1],1 if scenario==1 else 2)
                    self.assertEqual(ported[2],0 if scenario==1 else 1)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
