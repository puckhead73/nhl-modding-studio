"""Differential execution of the complete source panic eligibility gate."""
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


class FeatureTests(unittest.TestCase):
    def test_ordered_predicates_and_raw_boolean_results(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        names=['initialized','special_session','session_actor','same_actor','allowed','available',
               'reaction','state_blocked','game_blocked','session_object','session_blocked',
               'animation_blocked','global_blocked','actor_blocked','request_blocked','component_blocked']
        addresses=[0x3fddb0,0x4846d50,0x4715010,0x44c7fb0,0x439bdc0,0x455ec20,
                   0x4a19c80,0x453cd50,0x4795920,0x452dc70,0x3f0560,
                   0x4a19aa0,0x453a9b0,0x453aa00,0x453e080,0x4533ae0]
        code=assembly(image,'original',0x4a52090,0x4a521f9,dict(zip(addresses,names)),{})
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original(struct panic_state*);
static unsigned char actor[0x118],anim[0x58],flags[0x692],feature[0x20],role[0x14],other[0x80],other_kind[0x44],session[0x680];
static const unsigned *input;
static unsigned *trace;
static panic_address value(unsigned i){trace[++trace[0]]=i;return input[i];}
ABI panic_address initialized(void*p){if(p!=feature)__builtin_trap();return value(0);}
ABI panic_address special_session(void){return value(1);}
ABI void *session_actor(void){value(2);return input[2]?other:0;}
ABI panic_address same_actor(void*a,void*b){if(a!=actor || b!=(input[2]?other:0))__builtin_trap();return value(3);}
ABI panic_address allowed(void*a,unsigned mode){if(a!=actor || mode!=1)__builtin_trap();return value(4);}
#define ACTOR_FN(name,i) ABI panic_address name(void*a){if(a!=actor)__builtin_trap();return value(i);}
ACTOR_FN(available,5)
ABI panic_address reaction(void*a){if(a!=anim)__builtin_trap();return value(6);}
ACTOR_FN(state_blocked,7)
ABI panic_address game_blocked(unsigned mode){if(mode!=1)__builtin_trap();return value(8);}
ABI void *session_object(void){value(9);return session;}
ABI panic_address session_blocked(void*a){if(a!=session+0x670)__builtin_trap();return value(10);}
ABI panic_address animation_blocked(void*a){if(a!=anim)__builtin_trap();return value(11);}
ABI panic_address global_blocked(void){return value(12);}
ABI panic_address actor_blocked(void*a,unsigned mode){if(a!=actor || mode!=1)__builtin_trap();return value(13);}
ABI panic_address request_blocked(void*a,unsigned mode){if(a!=actor || mode!=10)__builtin_trap();return value(14);}
ACTOR_FN(component_blocked,15)
static void *get_feature(void*c,panic_address a){if(a!=(panic_address)actor)__builtin_trap();return feature;}
static panic_address query(void*c,enum panic_feature_query op,panic_address a,panic_address b){
 (void)c;
 switch(op){
 case PANIC_FEATURE_INITIALIZED:return initialized((void*)a);
 case PANIC_SPECIAL_SESSION:return special_session();
 case PANIC_SESSION_ACTOR:return (panic_address)session_actor();
 case PANIC_SAME_ACTOR:return same_actor((void*)a,(void*)b);
 case PANIC_ACTOR_ALLOWED:return allowed((void*)a,b);
 case PANIC_ACTOR_AVAILABLE:return available((void*)a);
 case PANIC_ANIMATION_REACTION:return reaction(*(void**)(a+0x60));
 case PANIC_ACTOR_STATE_BLOCKED:return state_blocked((void*)a);
 case PANIC_GAME_BLOCKED:return game_blocked(a);
 case PANIC_SESSION_OBJECT:return (panic_address)session_object();
 case PANIC_SESSION_BLOCKED:return session_blocked((void*)a);
 case PANIC_ANIMATION_BLOCKED:return animation_blocked(*(void**)(a+0x60));
 case PANIC_GLOBAL_BLOCKED:return global_blocked();
 case PANIC_ACTOR_BLOCKED:return actor_blocked((void*)a,b);
 case PANIC_REQUEST_BLOCKED:return request_blocked((void*)a,b);
 case PANIC_COMPONENT_BLOCKED:return component_blocked((void*)a);
 case PANIC_ANIMATION_FLAG_691:return flags[0x691];
 case PANIC_ACTOR_ROLE_KIND:return *(unsigned*)(role+0x10);
 case PANIC_SESSION_ACTOR_TYPE:if(a!=(panic_address)other)__builtin_trap();return *(unsigned*)(other_kind+0x40);
 }
 __builtin_trap();
}
__declspec(dllexport) void run(unsigned source,const unsigned*i,unsigned*out){
 input=i;trace=out;out[0]=0;feature[0x18]=0xa5;
 *(void**)(actor+0x60)=anim;*(void**)anim=flags;*(void**)(anim+0x50)=feature;
 *(void**)(actor+0x110)=role;*(unsigned*)(role+0x10)=i[17];*(unsigned*)(actor+0x48)=i[18];
 flags[0x691]=i[16];*(void**)(other+0x78)=other_kind;*(unsigned*)(other_kind+0x40)=i[19];
 struct panic_state state={0};state.actor=(panic_address)actor;
 const struct panic_feature_services services={0,get_feature,query};
 if(source)original(&state);else nhl27_panic_update_feature(&state,&services);
 out[31]=feature[0x18];
}
int _fltused=0;
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-feature-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared',
                '-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/panic_update.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_uint),C.POINTER(C.c_uint)]
                rng=random.Random(52090);default=[0]*20
                for i in (0,4,5):default[i]=1
                for n in range(12000):
                    values=default.copy()
                    for _ in range(1+n%5):values[rng.randrange(20)]=rng.choice([0,1,2,3,5,9,10,0x36,255,256,0xffffffff])
                    inp=(C.c_uint*20)(*values);a=(C.c_uint*32)();b=(C.c_uint*32)()
                    lib.run(0,inp,a);lib.run(1,inp,b)
                    self.assertEqual(list(a),list(b),(n,values))
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
