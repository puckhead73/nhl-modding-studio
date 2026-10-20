"""Original machine oracle for reaction dispatch order and late state sampling."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_REG_RIP
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
from test_panic_state import State, Bindings


class UpdateTests(unittest.TestCase):
    def test_original_sequence_with_mid_frame_mutations(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        calls={0x390dd0:'resolve',0x4a1ab90:'motion',0x452baa0:'attributes',
               0x3c6940:'attribute',0x488990:'events',0x48a3d0:'event',
               0x4a53570:'request',0x4a53360:'skating',0x4a52840:'deke',
               0x4a52cd0:'poke',0x4a53b70:'stumble',0x4a53ce0:'emit',0x4a53e10:'cancel'}
        globals_={0x104c27b8:'cooldown_global',0x104c28a8:'lookback_global'}
        start,end=0x4a51a80,0x4a51d20
        md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
        instructions=list(md.disasm(image.read(start,end-start),start))
        self.assertEqual(sum(i.size for i in instructions),end-start)
        asm=['.text','.globl original_update','original_update:']
        def emit(raw):
            if raw:asm.append('.byte '+','.join(str(v) for v in raw))
        for ins in instructions:
            fix=None
            if ins.mnemonic=='call' or (ins.mnemonic=='jmp' and not start<=ins.operands[0].imm<end):
                self.assertEqual(ins.operands[0].type,X86_OP_IMM)
                fix=(ins.imm_offset,calls[ins.operands[0].imm])
            elif ins.mnemonic.startswith('j'):
                self.assertTrue(start<=ins.operands[0].imm<end)
            for op in ins.operands:
                if op.type==X86_OP_MEM and op.mem.base==X86_REG_RIP:
                    self.assertIsNone(fix)
                    fix=(ins.disp_offset,globals_[ins.address+ins.size+op.mem.disp])
            if fix:
                offset,name=fix;emit(ins.bytes[:offset])
                asm.append(f'.long {name} - . - {ins.size-offset}');emit(ins.bytes[offset+4:])
            else:emit(ins.bytes)
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
int _fltused=0;
extern ABI void original_update(struct panic_state*);
static struct panic_state *state;
static struct panic_bindings *bindings;
static const int *input;
static unsigned *trace;
static unsigned char record[0xe8],actor[0x80],other[0x80],anim[0xc68],wrapper[0x200];
static unsigned char root_wrapper[0x18],root[0x120],cooldown_value[0x38],lookback_value[0x38];
void *cooldown_global=cooldown_value,*lookback_global=lookback_value;
static void log(unsigned id){if(trace[0]>=63)__builtin_trap();trace[++trace[0]]=id;}
static void mutate(unsigned id){
 log(id);
 if(input[2]==(int)id){
  state->cooldown=input[3];
  if(bindings->ptm_active)*bindings->ptm_active=(panic_u8)input[4];
  if(bindings->type)*bindings->type=input[5];
  if(bindings->requested)*bindings->requested=(panic_u8)input[6];
 }
}
ABI void *resolve(void*a,void*b){log(1);return record;}
ABI unsigned char motion(void*a){log(2);return (panic_u8)input[0];}
ABI void *attributes(void*a){log(3);return actor;}
ABI unsigned char attribute(void*a,unsigned hash,unsigned long long value){
 if(hash!=0xf34b2901 || value!=(0x100000000ULL|(unsigned)input[7]))__builtin_trap();
 mutate(4);return (panic_u8)input[8];
}
ABI void *events(void){log(5);return actor;}
ABI void event(void*a,int id,int opponent,int c,int d){
 if(id!=0x1c || opponent!=17 || c || d)__builtin_trap();mutate(6);
}
ABI void request(struct panic_state*s){mutate(7);}
ABI void skating(struct panic_state*s){mutate(8);}
ABI void deke(struct panic_state*s){mutate(9);}
ABI void poke(struct panic_state*s){mutate(10);}
ABI void stumble(struct panic_state*s){mutate(11);}
ABI void emit(struct panic_state*s){mutate(12);}
ABI void cancel(struct panic_state*s){mutate(13);}
static struct panic_bindings *get_bindings(void*c){resolve(0,0);return bindings;}
static panic_u8 get_motion(void*c){return motion(0);}
static int get_cooldown(void*c){return input[1];}
static void active_stumble(void*c,struct panic_state*s){
 void *a=attributes((void*)s->actor);
 if(attribute(a,0xf34b2901,0x100000000ULL|(unsigned)input[7]) && s->opponent)
  event(events(),0x1c,*(int*)(s->opponent+0x3c),0,0);
}
#define WRAP(name) static void wrap_##name(void*c,struct panic_state*s){name(s);}
WRAP(request) WRAP(skating) WRAP(deke) WRAP(poke) WRAP(stumble) WRAP(emit) WRAP(cancel)
__declspec(dllexport) void run(int original,struct panic_state*s,struct panic_bindings*b,
                              const int*i,unsigned*t){
 state=s;bindings=b;input=i;trace=t;trace[0]=0;
 panic_address saved=s->actor,opponent=s->opponent;s->actor=(panic_address)actor;
 if(opponent)s->opponent=(panic_address)other;
 *(int*)(other+0x3c)=17;
 *(void**)(actor+0x60)=anim;*(void**)(anim+0xc60)=wrapper;
 *(void**)(wrapper+0x1f8)=root_wrapper;*(void**)(root_wrapper+0x10)=root;
 *(void**)(root+0x118)=root;*(void**)(wrapper+0x20)=wrapper;
 *(void**)(record+0x10)=b->ptm_active;*(void**)(record+0xa8)=b->stumble;
 *(void**)(record+0xb0)=b->requested;*(void**)(record+0xc8)=b->type;
 *(void**)(record+0xd8)=b->interrupt_exit;
 *(int*)(cooldown_value+0x34)=i[1];*(int*)(lookback_value+0x34)=i[7];
 struct panic_update_services services={0,get_bindings,get_motion,get_cooldown,
  active_stumble,wrap_request,wrap_skating,wrap_deke,wrap_poke,wrap_stumble,wrap_emit,wrap_cancel};
 if(original)original_update(s);else nhl27_panic_update_reactions(s,&services);
 s->actor=saved;s->opponent=opponent;
}
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-update-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'native.s').write_text('\n'.join(asm)+'\n');(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'native.s'),str(repo/'scripts/nhl27/runtime/panic_update.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_int,C.POINTER(State),C.POINTER(Bindings),C.POINTER(C.c_int),C.POINTER(C.c_uint)]
                rng=random.Random(2751)
                for index in range(10000):
                    inp=(C.c_int*9)(rng.randrange(3),rng.choice([-1,0,1,2,30]),rng.randrange(4,14),
                        rng.choice([-1,0,1,2,30]),rng.randrange(3),rng.randrange(-1,5),rng.randrange(3),
                        rng.choice([-1,0,60,2147483647]),rng.randrange(3))
                    initial=State.from_buffer_copy(bytes([0x55])*96);initial.actor=1;initial.opponent=rng.randrange(2)
                    initial.duration=rng.choice([-2147483648,-1,0,1,2,20])
                    initial.active_ticks=rng.choice([-1,0,1,2147483647]);initial.cooldown=rng.choice([-1,0,1,2,30])
                    values=[rng.randrange(3),0,rng.randrange(3),2,99,1,rng.randrange(-1,5),1,0]
                    missing=index%11;results=[]
                    for original in (0,1):
                        state=State.from_buffer_copy(bytes(initial))
                        v=[(C.c_int if n in (1,6) else C.c_float if n==4 else C.c_ubyte)(x) for n,x in enumerate(values)]
                        binding=Bindings(*(C.addressof(x) if n!=missing else 0 for n,x in enumerate(v)))
                        trace=(C.c_uint*64)()
                        lib.run(original,C.byref(state),C.byref(binding),inp,trace)
                        results.append((bytes(state),[bytes(x) for x in v],list(trace)[:trace[0]+1]))
                    self.assertEqual(results[0],results[1],(index,list(inp),bytes(initial),values,missing))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
