"""Original cancellation tail with independent gates and changing session state."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from test_panic_state import State,Bindings
from oracle_fixture import assembly


class CancelTests(unittest.TestCase):
    def test_independent_gates_against_source_machine(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        asm=assembly(image,'original_cancel',0x4a53e10,0x4a54190,
            {0x38fdb0:'general',0x390dd0:'resolve',0x465c3f0:'session',0x48580c0:'collision'},
            {0x103bad38:'session_global'})
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
int _fltused=0;
extern ABI void original_cancel(struct panic_state*);
static struct panic_bindings *bindings;
static const unsigned char *input;
static unsigned *trace,session_index;
static unsigned char record[0xe8],general_record[0x100],actor[0x80],anim[0xc68],wrapper[0x200];
static unsigned char root_wrapper[0x18],root[0x138],pose_data[0xa0];
void *session_global;
static void log(unsigned id){trace[++trace[0]]=id;}
ABI void *resolve(void*a,void*b){log(1);return record;}
ABI void *general(void*a,void*b){log(2);return general_record;}
ABI unsigned char session(void*a){log(3);return input[4+session_index++];}
ABI unsigned char collision(void*a,int b,int c,int d){
 if(b||c||d)__builtin_trap();log(4);return input[2];
}
static struct panic_bindings *get_bindings(void*c){resolve(0,0);return bindings;}
static const panic_u8 *get_interrupt(void*c){general(0,0);return *(panic_u8**)(general_record+0xf0);}
static panic_u8 get_collision(void*c){return collision(actor,0,0,0);}
static panic_u8 get_present(void*c){return session_global!=0;}
static panic_u8 get_session(void*c){return session(session_global);}
static void get_pose(void*c,float*f,float*p){
 *f=*(float*)(pose_data+0x84);for(unsigned n=0;n<4;n++)p[n]=((float*)(pose_data+0x30))[n];
}
__declspec(dllexport) void run(int original,struct panic_state*s,struct panic_bindings*b,
                              const unsigned char*i,unsigned*t){
 bindings=b;input=i;trace=t;trace[0]=0;session_index=0;
 panic_address saved=s->actor;s->actor=(panic_address)actor;
 *(void**)(actor+0x60)=anim;*(void**)(actor+0x10)=pose_data;
 *(void**)(anim+0xc60)=wrapper;*(void**)(wrapper+0x1f8)=root_wrapper;
 *(void**)(root_wrapper+0x10)=root;*(void**)(root+0x118)=root;
 *(void**)(root+0x130)=root;*(void**)(wrapper+0x20)=wrapper;
 *(const void**)(general_record+0xf0)=i[0]?i+1:0;
 *(void**)(record+0xa8)=b->stumble;*(void**)(record+0xb0)=b->requested;
 *(void**)(record+0xc0)=b->between_legs;*(void**)(record+0xc8)=b->type;
 *(void**)(record+0xd8)=b->interrupt_exit;
 *(float*)(pose_data+0x84)=-2.5f;
 for(unsigned n=0;n<4;n++)((float*)(pose_data+0x30))[n]=(float)n+0.5f;
 session_global=i[3]?actor:0;
 struct panic_cancel_services services={0,get_bindings,get_interrupt,get_collision,get_present,get_session,get_pose};
 if(original)original_cancel(s);else nhl27_panic_update_cancel(s,&services);
 s->actor=saved;
}
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-cancel-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'native.s').write_text(asm);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'native.s'),str(repo/'scripts/nhl27/runtime/panic_update.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_int,C.POINTER(State),C.POINTER(Bindings),C.POINTER(C.c_ubyte),C.POINTER(C.c_uint)]
                rng=random.Random(27538)
                for index in range(6000):
                    inp=(C.c_ubyte*12)(rng.randrange(2),*(rng.randrange(4) for n in range(11)))
                    missing=index%11;results=[]
                    for original in (0,1):
                        state=State.from_buffer_copy(bytes([0x55])*96);state.actor=1
                        values=[C.c_ubyte(1),C.c_int(2),C.c_ubyte(2),C.c_ubyte(3),C.c_float(99),C.c_ubyte(1),C.c_int(2),C.c_ubyte(0),C.c_ubyte(1)]
                        binding=Bindings(*(C.addressof(v) if n!=missing else 0 for n,v in enumerate(values)))
                        trace=(C.c_uint*64)();lib.run(original,C.byref(state),C.byref(binding),inp,trace)
                        results.append((bytes(state),[bytes(v) for v in values],list(trace)[:trace[0]+1]))
                    self.assertEqual(results[0],results[1],(index,list(inp),missing))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
