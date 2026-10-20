"""Exercise appended PTM storage and paired resource ownership without a game."""
import ctypes
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class LifecycleTests(unittest.TestCase):
    def test_storage_bounds_ownership_and_repeated_cleanup(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        root=Path(__file__).resolve().parents[2]
        code=r'''
#include "ptm_lifecycle.h"
#define ABI PTM_GAME_ABI
typedef panic_address A;
extern void nhl24_ptm_extension_init(void*,void(ABI *)(void*));
extern void nhl24_ptm_extension_reset(A,void*,void(ABI *)(A,struct ptm_transition**));
extern void nhl24_ptm_extension_destroy(A,void*,void(ABI *)(A,struct ptm_transition**),void(ABI *)(void*));
static unsigned char memory[PTM_EXTENDED_MATCH_SIZE+32];
static unsigned calls,error;static struct ptm_match_extension *ext;
ABI void nhl24_ptm_transition_clear_guest(A b,struct ptm_transition**s){(void)b;(void)s;}
static ABI void construct(void*p){
 if(p!=ext->extra_path||ext->magic)error|=1;
 for(unsigned i=0;i<0x70;i++)((unsigned char*)p)[i]=(unsigned char)(i+1);
 calls=calls*10+1;
}
static ABI void clear(A b,struct ptm_transition**s){
 if(b!=0x12345678||s!=&ext->transition||ext->magic!=PTM_EXTENSION_MAGIC)error|=2;
 *s=0;calls=calls*10+2;
}
static ABI void destroy(void*p){
 if(p!=ext->extra_path||ext->transition)error|=4;
 for(unsigned i=0;i<0x70;i++)if(((unsigned char*)p)[i]!=(unsigned char)(i+1))error|=8;
 calls=calls*10+3;
}
__declspec(dllexport) unsigned exercise(unsigned reset,unsigned valid){
 for(unsigned i=0;i<sizeof(memory);i++)memory[i]=0xa5;
 calls=error=0;ext=(void*)(memory+PTM_NATIVE_MATCH_SIZE);
 nhl24_ptm_extension_init(memory,construct);
 if(ext->transition||ext->enabled||ext->extra_path_valid||ext->trajectory_flag)error|=16;
 for(unsigned i=0;i<sizeof(ext->padding);i++)if(ext->padding[i])error|=32;
 ext->transition=(void*)0x1230;
 if(!valid)ext->magic=0;
 if(reset)nhl24_ptm_extension_reset(0x12345678,memory,clear);
 nhl24_ptm_extension_destroy(0x12345678,memory,clear,destroy);
 nhl24_ptm_extension_destroy(0x12345678,memory,clear,destroy);
 nhl24_ptm_extension_reset(0x12345678,memory,clear);
 if(ext->magic)error|=64;
 for(unsigned i=0;i<PTM_NATIVE_MATCH_SIZE;i++)if(memory[i]!=0xa5)error|=128;
 for(unsigned i=PTM_EXTENDED_MATCH_SIZE;i<sizeof(memory);i++)if(memory[i]!=0xa5)error|=256;
 if(calls!=(valid?(reset?1223u:123u):1u))error|=512;
 return error;
}
'''
        scratch=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='ptm-lifecycle-',dir=scratch) as folder:
            p=Path(folder);(p/'fixture.c').write_text(code)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared',
                '-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.c'),str(root/'scripts/nhl27/runtime/ptm_lifecycle.c'),
                '-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=ctypes.CDLL(str(p/'fixture.dll'))
            try:
                lib.exercise.argtypes=[ctypes.c_uint,ctypes.c_uint]
                for reset in (0,1):
                    for valid in (0,1):
                        self.assertEqual(lib.exercise(reset,valid),0,(reset,valid))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
