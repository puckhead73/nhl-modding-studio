"""Execute the native interface thunk and verify scoped reader lifetime."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time
import unittest
import ptm_velocity_hooks as hook


class QueryScopeTests(unittest.TestCase):
    def test_dispatch_nesting_and_concurrent_close(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        code=hook.assembly().replace(f'.set {hook.NAME}_site, {hook.ADDRESS}',
            f'.set {hook.NAME}_site, fixture_base+{hook.ADDRESS}')
        code=code.replace(f'.set {hook.NAME}_original, 0x331a50','')
        code=code.replace(f'.long {hook.NAME}_original - . - 4','.long native_original - . - 4')
        code+='\n.globl fixture_base\nfixture_base:\nret\n'
        fixture=r'''
#include "ptm_query_scope.h"
#include <xmmintrin.h>
typedef panic_address A;
#define ABI PTM_GAME_ABI
extern void fixture_base(void);
extern ABI int ptm_velocity_dispatch(void*,void*);
static struct ptm_query_scope scopes[3];
static unsigned block,entered,closed;static char schema[32];
ABI int native_original(void*p,void*q){(void)q;return p==schema?0x55:0xee;}
ABI int nhl24_ptm_velocity_query_guest(A base,void*p,void*q,void*pose,void*cache){
 (void)q;if(base!=(A)fixture_base||p!=schema||(A)cache!=(A)pose+0x100)return 0xef;
 __atomic_store_n(&entered,1,__ATOMIC_RELEASE);
 while(__atomic_load_n(&block,__ATOMIC_ACQUIRE))_mm_pause();
 return (int)(A)pose;
}
__declspec(dllexport) int open_scope(unsigned i,A current,A future,A pose){
 return nhl24_ptm_query_scope_open(scopes+i,(void*)current,(void*)future,(void*)pose,(void*)(pose+0x100));
}
__declspec(dllexport) void close_scope(unsigned i){
 nhl24_ptm_query_scope_close(scopes+i);__atomic_store_n(&closed,1,__ATOMIC_RELEASE);
}
__declspec(dllexport) int run(A current,A future){
 A query[16]={0};query[12]=current;query[13]=future;
 return ptm_velocity_dispatch(schema+8,query);
}
__declspec(dllexport) void blocking(unsigned value){
 __atomic_store_n(&block,value,__ATOMIC_RELEASE);
 if(value){__atomic_store_n(&entered,0,__ATOMIC_RELEASE);__atomic_store_n(&closed,0,__ATOMIC_RELEASE);}
}
__declspec(dllexport) unsigned state(unsigned which){
 return which?__atomic_load_n(&closed,__ATOMIC_ACQUIRE):__atomic_load_n(&entered,__ATOMIC_ACQUIRE);
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-query-scope-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'hook.s').write_text(code)
            r=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),str(p/'fixture.c'),
                str(p/'hook.s'),str(root/'scripts/nhl27/runtime/ptm_query_scope.c'),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'));workers=[]
            try:
                lib.open_scope.argtypes=[C.c_uint,C.c_size_t,C.c_size_t,C.c_size_t]
                lib.close_scope.argtypes=[C.c_uint];lib.run.argtypes=[C.c_size_t,C.c_size_t]
                lib.blocking.argtypes=[C.c_uint];lib.state.argtypes=[C.c_uint]
                self.assertEqual(lib.run(1,2),0x55)
                self.assertEqual(lib.open_scope(0,0,2,1),0)
                self.assertEqual(lib.open_scope(0,1,2,1),1)
                self.assertEqual(lib.open_scope(0,1,2,1),0)
                self.assertEqual(lib.run(1,2),1)
                self.assertEqual(lib.run(1,3),0x55)
                self.assertEqual(lib.open_scope(1,1,2,2),1)
                self.assertEqual(lib.open_scope(2,3,4,3),1)
                self.assertEqual(lib.run(1,2),2);self.assertEqual(lib.run(3,4),3)
                lib.close_scope(1);self.assertEqual(lib.run(1,2),1)
                lib.close_scope(2);self.assertEqual(lib.run(3,4),0x55)
                lib.blocking(1);result=[]
                reader=threading.Thread(target=lambda:result.append(lib.run(1,2)));workers.append(reader);reader.start()
                deadline=time.monotonic()+2
                while not lib.state(0) and time.monotonic()<deadline:time.sleep(.001)
                self.assertEqual(lib.state(0),1)
                closer=threading.Thread(target=lambda:lib.close_scope(0));workers.append(closer);closer.start()
                # Wait for unpublication without entering another blocked reader.
                time.sleep(.02)
                self.assertEqual(lib.state(1),0)
                lib.blocking(0);reader.join(2);closer.join(2)
                self.assertFalse(reader.is_alive());self.assertFalse(closer.is_alive())
                self.assertEqual(result,[1]);self.assertEqual(lib.state(1),1)
                self.assertEqual(lib.run(1,2),0x55)
                lib.close_scope(0)
            finally:
                lib.blocking(0)
                for worker in workers:worker.join(2)
                if not any(worker.is_alive() for worker in workers):
                    import _ctypes
                    _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
