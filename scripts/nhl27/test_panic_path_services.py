"""Native path-context traversal and source tracker-owner identity."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class PathServicesTests(unittest.TestCase):
    def test_context_live_vector_and_original_tracker_owner(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'construct',0x4a3da00,0x4a3da73,{},{})
        code+=assembly(image,'tracker',0x453cc30,0x453cc38,{},{})
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
extern ABI void construct(void*,void*);
extern ABI void *tracker(void*);
extern const float *nhl24_panic_path_owner_position(void*);
extern const struct panic_trajectory_view *nhl24_panic_trajectory(void*,void*(ABI*)(void*));
static unsigned char actor[0x800],pose[0x50],anim[0xc30],wrapper[0x28],context[0x80],path[0x70];
static unsigned calls,bad;
ABI void *query(void*c){calls++;if(c!=context+0x60)bad++;return path;}
__declspec(dllexport) unsigned check(void){
    *(void**)(actor+0x10)=pose;construct(actor+0x718,actor);
    unsigned char *source_tracker=tracker(actor);
    unsigned char *owner=*(unsigned char**)(source_tracker+0x10);
    const float *expected=(const float*)(*(unsigned char**)(owner+0x10)+0x30);
    if(nhl24_panic_path_owner_position(actor)!=expected)return 1;
    *(void**)(actor+0x60)=anim;*(void**)(anim+0xc28)=wrapper;
    *(void**)(wrapper+0x20)=context;
    const struct panic_trajectory_view *view=nhl24_panic_trajectory(actor,query);
    if(view!=(const struct panic_trajectory_view*)(path+0x10)||calls!=1||bad)return 2;
    *(panic_address*)(path+0x10)=123;*(panic_address*)(path+0x18)=456;
    if(view->begin!=123||view->end!=456)return 3;
    *(void**)(wrapper+0x20)=0;
    if(nhl24_panic_trajectory(actor,query)||calls!=1)return 4;
    if(nhl24_panic_trajectory(0,query)||nhl24_panic_path_owner_position(0))return 5;
    return 0;
}
int _fltused=0;
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-path-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/panic_services.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:self.assertEqual(lib.check(),0)
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
