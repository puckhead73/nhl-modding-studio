"""Execute all original type-2 instructions with controlled engine-service inputs."""
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


class Input(C.Structure):
    _fields_=[(n,C.c_int) for n in ('role','tick','delay')]+[(n,C.c_ubyte) for n in
        ('special','between','blocked','direction','session')]+[(n,C.c_float) for n in
        ('facing','opponent_facing','speed','angle_threshold','speed_threshold','distance_threshold')]+[
        ('position',C.c_float*4),('opponent_position',C.c_float*4)]


class DekeReactionTests(unittest.TestCase):
    def test_source_machine_oracle_angles_gates_timing_and_cancellation(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        calls={0x390dd0:'resolve',0x453df90:'special',0x465c3f0:'session',0x453cc30:'component',
               0x4a355e0:'direction',0x4a3c260:'between',0x4533ae0:'blocked',0xbfabea0:'fail'}
        globals_={0xec2d9f0:'cookie_global',0x103bad38:'session_global',0x104c2c18:'delay_global',
            0x103e1418:'tick_manager',0x104c2d58:'direction_global',0x104331f0:'puck_global',
            0x104c2cb8:'angle_global',0x104c2c68:'speed_global',0x104c2d08:'distance_global'}
        start,end=0x4a52840,0x4a52cd0;md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
        instructions=list(md.disasm(image.read(start,end-start),start));self.assertEqual(sum(i.size for i in instructions),end-start)
        asm=['.text','.globl original_deke','original_deke:'];constants={}
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
extern ABI void original_deke(struct panic_state*);
static const struct panic_deke_input *input;
static unsigned char record[0xe8],actor[0x80],opponent_actor[0x80],anim[0xc68],wrapper[0x200];
static unsigned char root_wrapper[0x18],root[0x120],pose[0xa0],opponent_pose[0xa0],puck_pose[0xa0],puck_actor[0x80],actor_component[0x18];
static unsigned char delay_value[0x38],angle_value[0x38],speed_value[0x38],distance_value[0x38],direction_value[0x38];
unsigned char tick_manager[0x2d0];
static unsigned long long cookie=0x12345678;
void *cookie_global=&cookie,*session_global,*delay_global=delay_value,*angle_global=angle_value;
void *speed_global=speed_value,*distance_global=distance_value,*direction_global=direction_value,*puck_global=puck_actor;
ABI void *resolve(void*unused,void*context){return record;}
ABI unsigned char special(void*unused){return input->special_actor;}
ABI unsigned char session(void*unused){return input->session_result;}
ABI void *component(void*unused){return actor_component;}
ABI unsigned char direction(void*a,void*b,void*c,void*d,float threshold){return input->direction_result;}
ABI unsigned char between(void*unused,int a,int b){return input->between_legs;}
ABI unsigned char blocked(void*unused){return input->component_blocked;}
ABI void fail(void){__builtin_trap();}
__declspec(dllexport) void oracle(struct panic_state*s,struct panic_bindings*b,const struct panic_deke_input*i){
    input=i;panic_address saved_actor=s->actor,saved_opponent=s->opponent;s->actor=(panic_address)actor;
    if(s->opponent)s->opponent=(panic_address)opponent_actor;
    *(void**)(actor+0x10)=pose;*(void**)(actor+0x60)=anim;*(int*)(actor+0x48)=i->role;
    *(void**)(actor_component+0x10)=actor;
    *(void**)(opponent_actor+0x10)=opponent_pose;*(void**)(puck_actor+0x10)=puck_pose;
    *(float*)(pose+0x84)=i->facing;*(float*)(pose+0x90)=i->speed;*(float*)(opponent_pose+0x84)=i->opponent_facing;
    for(unsigned n=0;n<4;n++){((float*)(pose+0x30))[n]=i->position[n];((float*)(opponent_pose+0x30))[n]=i->opponent_position[n];}
    *(void**)(anim+0xc60)=wrapper;*(void**)(wrapper+0x1f8)=root_wrapper;*(void**)(root_wrapper+0x10)=root;
    *(void**)(root+0x118)=root;*(void**)(wrapper+0x20)=wrapper;
    *(void**)(record+0x10)=b->ptm_active;*(void**)(record+0xa8)=b->stumble;
    *(void**)(record+0xb0)=b->requested;*(void**)(record+0xc0)=b->between_legs;*(void**)(record+0xc8)=b->type;
    *(int*)(delay_value+0x34)=i->delay;*(int*)(tick_manager+0x2cc)=i->tick;
    *(float*)(angle_value+0x34)=i->angle_threshold;*(float*)(speed_value+0x34)=i->speed_threshold;
    *(float*)(distance_value+0x34)=i->distance_threshold;
    original_deke(s);s->actor=saved_actor;if(s->opponent)s->opponent=saved_opponent;
}
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-deke-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'native.s').write_text('\n'.join(asm)+'\n');(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'native.s'),
                str(repo/'scripts/nhl27/runtime/panic_state.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                for name in ('oracle','nhl27_panic_deke_reaction'):getattr(lib,name).argtypes=[C.POINTER(State),C.POINTER(Bindings),C.POINTER(Input)]
                rng=random.Random(27)
                angles=[-0.0,0,math.pi,-math.pi,math.tau,-math.tau,1e30,-1e30,float('nan'),float('inf'),-float('inf')]
                cases=[]
                for a in angles:
                    for threshold in (-1,0,89.999,90,179.999,180,float('nan')):
                        cases.append((Input(0,10,0,0,0,0,1,1,a,0,5,threshold,0,0,(C.c_float*4)(0,0,0,1),(C.c_float*4)(1,2,3,1)),0,1))
                for _ in range(2000):
                    inp=Input(rng.choice([-2147483648,-1,0,4,5,9,10]),rng.choice([-2147483648,-1,0,1,2147483647]),
                        rng.choice([-1,0,1,2147483647]),rng.randrange(2),rng.randrange(3),rng.randrange(2),rng.randrange(3),rng.randrange(3),
                        rng.uniform(-30,30),rng.uniform(-30,30),rng.choice([0,1,5,float('nan')]),rng.uniform(-1,181),rng.choice([0,1,5]),
                        rng.choice([0,1,14,15,float('nan')]),(C.c_float*4)(0,0,0,1),(C.c_float*4)(1,2,3,1))
                    cases.append((inp,rng.choice([-2147483648,-1,0,1,2147483647]),rng.randrange(2)))
                for index,(inp,stamp,opponent_present) in enumerate(cases):
                    results=[]
                    for name in ('oracle','nhl27_panic_deke_reaction'):
                        state=State.from_buffer_copy(bytes([0x55])*96);state.actor=1;state.opponent=opponent_present;state.stamp=stamp
                        values=[C.c_ubyte(1),C.c_int(11),C.c_ubyte(1),C.c_ubyte(2),C.c_float(99),C.c_ubyte(0),C.c_int(2),C.c_ubyte(0),C.c_ubyte(1)]
                        binding=Bindings(*(C.addressof(v) for v in values))
                        getattr(lib,name)(C.byref(state),C.byref(binding),C.byref(inp));results.append((bytes(state),[bytes(v) for v in values]))
                    self.assertEqual(results[0],results[1],(index,bytes(inp),stamp,opponent_present))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
