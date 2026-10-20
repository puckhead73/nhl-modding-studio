"""Compare the entire outer panic schedule and alignment guard with NHL27."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class PanicFrameTests(unittest.TestCase):
    def test_original_schedule_and_late_alignment_guard(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        asm=assembly(image,'original',0x4a517d0,0x4a51924,
            {0x4a164a0:'cached',0x4a51a80:'reactions',0x4a51d20:'preconditions',
             0x4a52090:'feature',0x4a52200:'closest',0x4a52300:'takeoff',
             0x4a54620:'alignment',0x390dd0:'resolve',0x21ce80:'trajectory',
             0x453cc30:'query_path_tracker',0x4a51930:'reset',0x4a54190:'reset_alignment'},
            {0x103e1418:'clock_data',0x104c31b8:'tolerance_global'})
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original(struct panic_state*);
static unsigned char actor[0x68],anim[0xc68],wrapper[0x200],root_wrapper[0x18],root[0x120];
static unsigned char record[0x20],trajectory_record[0x20],tracker[0x18],pose_wrapper[0x18],pose[0x40];
static unsigned char tolerance_data[0x38];
unsigned char clock_data[0x2d0];
void *tolerance_global=tolerance_data;
static float samples[8];
static struct panic_state state;
static struct panic_bindings bindings;
static int lock;
static panic_u8 active;
static const int *input;
static unsigned *trace;
static void log(unsigned id){
    trace[++trace[0]]=id;
    if(input[6]==(int)id){
        lock=input[7];active=(panic_u8)input[8];
        *(panic_address*)(trajectory_record+0x18)=(panic_address)samples+(unsigned)input[9];
    }
}
ABI panic_u8 cached(void*a){(void)a;log(1);return (panic_u8)input[0];}
#define STAGE(name,id) ABI void name(struct panic_state*s){(void)s;log(id);}
STAGE(reactions,2) STAGE(preconditions,3) STAGE(feature,4)
STAGE(closest,5) STAGE(takeoff,6) STAGE(alignment,7)
STAGE(reset,11) STAGE(reset_alignment,12)
ABI void *resolve(void*a,void*b){(void)a;(void)b;log(8);return record;}
ABI void *trajectory(void*a){(void)a;log(9);return trajectory_record;}
ABI void *query_path_tracker(void*a){(void)a;log(10);return tracker;}
static panic_u8 cached_port(void*c,panic_address a){(void)c;return cached((void*)a);}
#define WRAP(name) static void wrap_##name(void*c,struct panic_state*s){(void)c;name(s);}
WRAP(reactions) WRAP(preconditions) WRAP(feature) WRAP(closest) WRAP(takeoff)
WRAP(alignment) WRAP(reset) WRAP(reset_alignment)
static struct panic_bindings *get_bindings(void*c){(void)c;resolve(0,0);return &bindings;}
static int tick(void*c){(void)c;return input[4];}
static const struct panic_trajectory_view *get_trajectory(void*c,panic_address a){
    (void)c;return (const struct panic_trajectory_view*)((char*)trajectory((void*)a)+0x10);
}
static void *get_tracker(void*c,panic_address a){(void)c;return query_path_tracker((void*)a);}
static const float *position(void*c,void*p){(void)c;(void)p;return (const float*)(pose+0x30);}
static float tolerance(void*c){(void)c;return *(float*)(tolerance_data+0x34);}
__declspec(dllexport) void run(unsigned source,const int*i,const float*f,unsigned*out){
    input=i;trace=out;trace[0]=0;state.actor=(panic_address)actor;state.align_deadline=f[0];
    lock=i[1];active=(panic_u8)i[3];
    bindings.path_lock=(i[2]&1)?&lock:0;bindings.ptm_active=(i[2]&2)?&active:0;
    *(void**)(actor+0x60)=anim;*(void**)(anim+0xc60)=wrapper;
    *(void**)(wrapper+0x20)=wrapper;*(void**)(wrapper+0x1f8)=root_wrapper;
    *(void**)(root_wrapper+0x10)=root;*(void**)(root+0x118)=root;
    *(void**)(record+0x18)=bindings.path_lock;*(void**)(record+0x10)=bindings.ptm_active;
    *(int*)(clock_data+0x2cc)=i[4];*(float*)(tolerance_data+0x34)=f[1];
    *(panic_address*)(trajectory_record+0x10)=(panic_address)samples;
    *(panic_address*)(trajectory_record+0x18)=(panic_address)samples+(unsigned)i[5];
    *(void**)(tracker+0x10)=pose_wrapper;*(void**)(pose_wrapper+0x10)=pose;
    for(unsigned n=0;n<4;n++){((float*)(pose+0x30))[n]=f[2+n];samples[4+n]=f[6+n];}
    struct panic_frame_services services={0,cached_port,wrap_reactions,wrap_preconditions,
        wrap_feature,wrap_closest,wrap_takeoff,wrap_alignment,get_bindings,tick,
        get_trajectory,get_tracker,position,tolerance,wrap_reset,wrap_reset_alignment};
    if(source)original(&state);else nhl27_panic_update_frame(&state,&services);
    out[30]=(unsigned)lock;out[31]=active;
}
int _fltused=0;
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-frame-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(asm);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-ffp-contract=off','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',
                '-I',str(root/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'oracle.s'),
                str(root/'scripts/nhl27/runtime/panic_update.c'),'-o',str(out/'test.dll')],
                capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_int),C.POINTER(C.c_float),C.POINTER(C.c_uint)]
                rng=random.Random(27517)
                values=[0.,-0.,1.,-1.,10.,100.,float('nan'),float('inf'),float('-inf')]
                for n in range(12000):
                    ints=(C.c_int*10)(rng.choice([0,1,2,255]),rng.choice([0,11,12]),
                        rng.choice([0,1,2,3,3,3]),rng.choice([0,1,1,2,255]),
                        rng.choice([-2147483648,-1,0,1,10,2147483647]),
                        rng.choice([0,16,32,48,64]),rng.randrange(12),
                        rng.choice([0,11,12]),rng.choice([0,1,1,2,255]),rng.choice([0,16,32,64]))
                    floats=(C.c_float*10)(*(rng.choice(values) for _ in range(10)))
                    original=(C.c_uint*32)();ported=(C.c_uint*32)()
                    lib.run(1,ints,floats,original);lib.run(0,ints,floats,ported)
                    self.assertEqual(list(ported),list(original),(n,list(ints),list(floats)))
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
