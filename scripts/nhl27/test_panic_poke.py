"""Execute all original type-3 instructions with controlled engine-service inputs."""
import ctypes as C
import math
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_REG_RIP
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from test_panic_state import State,Bindings


class Candidate(C.Structure):
    pass
Candidate._fields_=[('next',C.POINTER(Candidate)),('object_type',C.c_int),('facing',C.c_float),('position',C.c_float*4)]
class Input(C.Structure):
    _fields_=[('role',C.c_int),('special',C.c_ubyte),('session',C.c_ubyte),('panic',C.c_ubyte),
        ('facing',C.c_float),('position',C.c_float*4),('radius',C.c_float),('facing_threshold',C.c_float),
        ('heading_threshold',C.c_float),('candidates',C.POINTER(Candidate))]


class PokeReactionTests(unittest.TestCase):
    def test_source_machine_oracle_angles_gates_timing_and_cancellation(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        calls={0x390dd0:'resolve',0x453df90:'special',0x465c3f0:'session',0x4a1ab90:'panic'}
        globals_={0x103bad38:'session_global',0x104c2e98:'radius_global',
                  0x104c2df8:'facing_global',0x104c2e48:'heading_global'}
        start,end=0x4a52cd0,0x4a53360;md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
        instructions=list(md.disasm(image.read(start,end-start),start));self.assertEqual(sum(i.size for i in instructions),end-start)
        asm=['.text','.globl original_poke','original_poke:'];constants={}
        def emit(raw):
            if raw:asm.append('.byte '+','.join(str(v) for v in raw))
        for ins in instructions:
            fix=None
            if ins.mnemonic=='call':
                self.assertEqual(ins.operands[0].type,X86_OP_IMM);fix=(ins.imm_offset,calls[ins.operands[0].imm])
            elif ins.mnemonic.startswith('j') and ins.operands[0].type==X86_OP_IMM:self.assertTrue(start<=ins.operands[0].imm<end)
            for op in ins.operands:
                if op.type==X86_OP_MEM and op.mem.base==X86_REG_RIP:
                    self.assertIsNone(fix);address=ins.address+ins.size+op.mem.disp
                    if address in globals_:name=globals_[address]
                    else:
                        self.assertTrue(0xded0000<=address<0xdee0000)
                        name=f'constant_{address:x}';constants[name]=image.read(address,op.size)
                    fix=(ins.disp_offset,name)
            if fix:
                offset,name=fix;emit(ins.bytes[:offset]);asm.append(f'.long {name} - . - {ins.size-offset}');emit(ins.bytes[offset+4:])
            else:emit(ins.bytes)
        for name,data in constants.items():asm.extend(['.balign 16',name+':']);emit(data)
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original_poke(struct panic_state*);
static const struct panic_poke_input *input;
static unsigned char record[0xe8],actor[0xa98],anim[0xc68],wrapper[0x200];
static unsigned char root_wrapper[0x18],root[0x120],pose[0xa0],nearby[0x30];
static unsigned char objects[8][0x328],poses[8][0xa0];
static unsigned char radius_value[0x38],facing_value[0x38],heading_value[0x38];
void *session_global,*radius_global=radius_value,*facing_global=facing_value,*heading_global=heading_value;
ABI void *resolve(void*unused,void*context){return record;}
ABI unsigned char special(void*unused){return input->special_actor;}
ABI unsigned char session(void*unused){return input->session_result;}
ABI unsigned char panic(void*unused){return input->animation_is_panic;}
__declspec(dllexport) void oracle(struct panic_state*s,struct panic_bindings*b,const struct panic_poke_input*i){
    input=i;panic_address saved=s->actor;s->actor=(panic_address)actor;
    *(void**)(actor+0x10)=pose;*(void**)(actor+0x60)=anim;*(int*)(actor+0x48)=i->role;
    *(void**)(actor+0xa90)=nearby;*(void**)(nearby+0x28)=i->candidates?objects[0]:0;
    unsigned k=0;
    for(const struct panic_poke_candidate*c=i->candidates;c;c=c->next,k++){
        if(k>=8)__builtin_trap();
        *(void**)(objects[k]+0x10)=poses[k];*(int*)(objects[k]+0x20)=c->object_type;
        *(void**)(objects[k]+0x320)=c->next?objects[k+1]:0;
        *(float*)(poses[k]+0x84)=c->facing;
        for(unsigned n=0;n<4;n++)((float*)(poses[k]+0x30))[n]=c->position[n];
    }
    *(float*)(pose+0x84)=i->facing;
    for(unsigned n=0;n<4;n++)((float*)(pose+0x30))[n]=i->position[n];
    *(void**)(anim+0xc60)=wrapper;*(void**)(wrapper+0x1f8)=root_wrapper;*(void**)(root_wrapper+0x10)=root;
    *(void**)(root+0x118)=root;*(void**)(wrapper+0x20)=wrapper;
    *(void**)(record+0x10)=b->ptm_active;*(void**)(record+0xa8)=b->stumble;
    *(void**)(record+0xb0)=b->requested;*(void**)(record+0xc0)=b->between_legs;*(void**)(record+0xc8)=b->type;
    *(float*)(radius_value+0x34)=i->radius;*(float*)(facing_value+0x34)=i->facing_threshold;
    *(float*)(heading_value+0x34)=i->heading_threshold;
    original_poke(s);s->actor=saved;
}
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-poke-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'native.s').write_text('\n'.join(asm)+'\n');(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'native.s'),
                str(repo/'scripts/nhl27/runtime/panic_state.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                for name in ('oracle','nhl27_panic_poke_reaction'):getattr(lib,name).argtypes=[C.POINTER(State),C.POINTER(Bindings),C.POINTER(Input)]
                rng=random.Random(273)
                angles=[-0.0,0,math.pi,-math.pi,math.tau,-math.tau,1e30,-1e30,float('nan'),float('inf'),-float('inf')]
                for index in range(12000):
                    candidates=(Candidate*4)()
                    for k,c in enumerate(candidates):
                        c.next=C.pointer(candidates[k+1]) if k<3 else C.POINTER(Candidate)()
                        c.object_type=rng.choice([0,0,1,2])
                        c.facing=rng.choice(angles) if index<1000 else rng.uniform(-30,30)
                        c.position[:]=[rng.uniform(-2,2),rng.uniform(-2,2),rng.uniform(-2,2),1]
                        if index<1000:c.position[:]=[rng.choice([0,-0.0,1e-6,1,-1,float('nan')]) for _ in range(4)]
                    inp=Input(rng.choice([0,0,0,5,9,10,-1]),rng.randrange(2) if index<1000 else 0,
                        rng.randrange(3),rng.randrange(3),rng.choice(angles) if index<1000 else rng.uniform(-30,30),
                        (C.c_float*4)(0,0,0,1),rng.choice([0,1,10,100,float('nan')]),
                        rng.uniform(-1,181),rng.uniform(-1,181),C.pointer(candidates[0]) if index%17 else C.POINTER(Candidate)())
                    initial_request=rng.randrange(3);active=rng.randrange(3);poke=0 if index%19==0 else 1
                    results=[]
                    for name in ('oracle','nhl27_panic_poke_reaction'):
                        state=State.from_buffer_copy(bytes([0x55])*96);state.actor=1;state.poke_opponent=poke
                        values=[C.c_ubyte(active),C.c_int(11),C.c_ubyte(1),C.c_ubyte(initial_request),C.c_float(99),C.c_ubyte(0),C.c_int(3),C.c_ubyte(0),C.c_ubyte(1)]
                        binding=Bindings(*(C.addressof(v) for v in values))
                        if index%23==0:binding.requested=None
                        getattr(lib,name)(C.byref(state),C.byref(binding),C.byref(inp));results.append((bytes(state),[bytes(v) for v in values]))
                    self.assertEqual(results[0],results[1],(index,bytes(inp),bytes(candidates)))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
