"""Full-reset oracle: retained state, optional bindings, pose and feature flags."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly
from test_panic_state import State


class ResetTests(unittest.TestCase):
    def test_original_full_reset_and_retained_state(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        asm=assembly(image,'original',0x4a51930,0x4a51a7c,
            {0x390dd0:'resolve',0x465c3f0:'session',0x4a54190:'align_reset',0x3fddb0:'feature'},
            {0x103bad38:'session_global'})
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original(struct panic_state*);
void *session_global;
static unsigned char actor[0x68],anim[0xc68],wrapper[0x200],root_wrapper[0x18],root[0x120];
static unsigned char record[0xd0],pose[0x88],flags[0x6e8],feature_data[0x20];
static struct panic_bindings bindings;
static unsigned values[5],*output,scenario,resolves;
static void log(unsigned n){output[++output[0]]=n;}
ABI void *resolve(void*a,void*b){(void)a;(void)b;log(1);resolves++;
    if(resolves==2 && (*(unsigned short*)(flags+0x6e3)!=0))__builtin_trap();
    return record;
}
ABI panic_u8 session(void*a){(void)a;log(2);return scenario&255;}
ABI void align_reset(struct panic_state*s){log(3);s->align_deadline=0;}
ABI panic_u8 feature(void*a){(void)a;log(4);return (scenario>>8)&255;}
static struct panic_bindings *get_bindings(void*c){(void)c;resolve(0,0);return &bindings;}
static panic_u8 get_session(void*c){(void)c;return session(0);}
static void get_pose(void*c,panic_address a,float*f,float*p){(void)c;(void)a;
    *f=*(float*)(pose+0x84);for(unsigned n=0;n<4;n++)p[n]=((float*)(pose+0x30))[n];
}
static void clear(void*c,panic_address a){(void)c;(void)a;*(unsigned short*)(flags+0x6e3)=0;}
static void reset_align(void*c,struct panic_state*s){(void)c;align_reset(s);}
static panic_u8 *get_feature(void*c,panic_address a){(void)c;(void)a;
    return feature(feature_data)?feature_data+0x18:0;
}
__declspec(dllexport) void run(unsigned source,unsigned mode,struct panic_state*s,unsigned*out){
    scenario=mode;output=out;output[0]=0;resolves=0;
    for(unsigned n=0;n<5;n++)values[n]=0xaabbccdd+n;
    bindings.requested=(mode&(1u<<16))?(panic_u8*)(values+0):0;
    bindings.stumble=(mode&(1u<<17))?(panic_u8*)(values+1):0;
    bindings.between_legs=(mode&(1u<<18))?(panic_u8*)(values+2):0;
    bindings.type=(mode&(1u<<19))?(int*)(values+3):0;
    bindings.closest_distance=(mode&(1u<<20))?(float*)(values+4):0;
    *(void**)(record+0xb0)=bindings.requested;*(void**)(record+0xa8)=bindings.stumble;
    *(void**)(record+0xc0)=bindings.between_legs;*(void**)(record+0xc8)=bindings.type;
    *(void**)(record+0xb8)=bindings.closest_distance;
    *(void**)(actor+0x10)=pose;*(void**)(actor+0x60)=anim;*(void**)anim=flags;
    *(void**)(anim+0x50)=feature_data;*(void**)(anim+0xc60)=wrapper;
    *(void**)(wrapper+0x20)=wrapper;*(void**)(wrapper+0x1f8)=root_wrapper;
    *(void**)(root_wrapper+0x10)=root;*(void**)(root+0x118)=root;
    *(unsigned short*)(flags+0x6e3)=0xffff;feature_data[0x18]=255;
    *(float*)(pose+0x84)=s->attributes[0];
    for(unsigned n=0;n<4;n++)((float*)(pose+0x30))[n]=s->attributes[n];
    s->actor=(panic_address)actor;
    struct panic_reset_services services={0,get_bindings,get_session,get_pose,clear,reset_align,get_feature};
    if(source)original(s);else nhl27_panic_reset_full(s,&services);
    s->actor=1;for(unsigned n=0;n<5;n++)out[16+n]=values[n];
    out[21]=feature_data[0x18];out[22]=*(unsigned short*)(flags+0x6e3);
}
int _fltused=0;
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-reset-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(asm);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/panic_update.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(State),C.POINTER(C.c_uint)]
                rng=random.Random(27519)
                for n in range(4000):
                    mode=rng.choice([0,1,2,255])|(rng.choice([0,1,2,255])<<8)|(rng.randrange(32)<<16)
                    raw=bytes(rng.randrange(256) for _ in range(96))
                    a=State.from_buffer_copy(raw);b=State.from_buffer_copy(raw)
                    expected=(C.c_uint*24)();actual=(C.c_uint*24)()
                    lib.run(1,mode,C.byref(a),expected);lib.run(0,mode,C.byref(b),actual)
                    self.assertEqual(bytes(b),bytes(a),(n,mode))
                    self.assertEqual(list(actual),list(expected),(n,mode))
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
