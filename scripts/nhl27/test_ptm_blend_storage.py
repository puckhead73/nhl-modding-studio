"""Exercise retained snapshot aliasing, publication order and cleanup."""
import ctypes
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class BlendStorageTests(unittest.TestCase):
    def test_pose_ownership(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        root=Path(__file__).resolve().parents[2]
        code=r'''
#include "ptm_blend_storage.h"
int _fltused=0;
void nhl24_ptm_pose_retain(void *p){(void)p;}
void nhl24_ptm_pose_release(void *p){(void)p;}
static struct {unsigned long long before[2];struct ptm_blend_extension e;
 unsigned long long after[2];} storage;
static struct ptm_blend_pose_services services;
static int references[4],errors,reenter,clones;
static void *next;
static void *clone(void *ctx,void *pose){
 if(ctx!=&storage||pose!=&services)errors|=1;
 clones++;return next;
}
static void retain(void *ctx,void *p){
 if(ctx!=&storage)errors|=2;
 references[(panic_address)p]++;
}
static void release(void *ctx,void *p){
 if(ctx!=&storage||--references[(panic_address)p]<0)errors|=4;
 if(storage.e.magic && storage.e.pose_a==p)errors|=8;
 if(reenter){reenter=0;nhl24_ptm_blend_storage_destroy(&storage.e,&services);}
}
__declspec(dllexport) int exercise(unsigned b,unsigned a,unsigned replacement,unsigned recursive){
 for(unsigned i=0;i<sizeof(storage);i++)((unsigned char*)&storage)[i]=0xa5;
 for(unsigned i=0;i<4;i++)references[i]=0;
 errors=clones=reenter=0;services=(struct ptm_blend_pose_services){&storage,clone,retain,release};
 next=(void*)(panic_address)b;
 nhl24_ptm_blend_storage_init(&storage.e,&services,40,6,10,&services);
 if(storage.e.pose_a||storage.e.initialized||storage.e.enabled||storage.e.retime!=1||
    storage.e.clock.duration!=0x1.342618p+116f||storage.e.clock.elapsed||
    storage.e.clock.nominal!=40||storage.e.clock.blend_in!=6||storage.e.clock.blend_out!=10||
    storage.e.clock.speed!=1||storage.e.clock.feature_ticks)errors|=16;
 next=(void*)(panic_address)a;
 nhl24_ptm_blend_storage_capture_a(&storage.e,&services,&services);
 next=(void*)(panic_address)replacement;
 nhl24_ptm_blend_storage_capture_a(&storage.e,&services,&services);
 if(references[b]!=(b?1:0)+(b&&b==replacement?1:0))errors|=32;
 reenter=recursive;
 nhl24_ptm_blend_storage_destroy(&storage.e,&services);
 nhl24_ptm_blend_storage_destroy(&storage.e,&services);
 nhl24_ptm_blend_storage_capture_a(&storage.e,&services,&services);
 if(clones!=3||storage.e.magic||storage.e.pose_a||storage.e.pose_b)errors|=64;
 for(unsigned i=0;i<4;i++)if(references[i])errors|=128;
 for(unsigned i=0;i<2;i++)if(storage.before[i]!=0xa5a5a5a5a5a5a5a5ull||
    storage.after[i]!=0xa5a5a5a5a5a5a5a5ull)errors|=256;
 return errors;
}
'''
        scratch=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='ptm-blend-storage-',dir=scratch) as folder:
            p=Path(folder);(p/'fixture.c').write_text(code)
            result=subprocess.run([str(compiler),'-O2','-Wall','-Wextra','-Werror',
                '-ffreestanding','-fno-builtin','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',
                '-I',str(root/'scripts/nhl27/runtime'),str(p/'fixture.c'),
                str(root/'scripts/nhl27/runtime/ptm_blend_storage.c'),'-o',str(p/'fixture.dll')],
                capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=ctypes.CDLL(str(p/'fixture.dll'))
            try:
                lib.exercise.argtypes=[ctypes.c_uint]*4
                for b in range(4):
                    for a in range(4):
                        for replacement in range(4):
                            for recursive in range(2):
                                self.assertEqual(lib.exercise(b,a,replacement,recursive),0,
                                    (b,a,replacement,recursive))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
