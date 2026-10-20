"""Execute all original request generator instructions with controlled engine-service inputs."""
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
    _fields_=[('role',C.c_int),('duration',C.c_int)]+[(n,C.c_ubyte) for n in
        ('special','session','trajectory_present','opposing_team','direction')]+[
        ('facing',C.c_float),('position',C.c_float*4),('trajectory_sample',C.c_float*4),
        ('angle_threshold',C.c_float),('distance_threshold',C.c_float)]


class RequestTests(unittest.TestCase):
    def test_source_machine_oracle_angles_gates_timing_and_cancellation(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        calls={0x390dd0:'resolve',0x453df90:'special',0x465c3f0:'session',0x453cc30:'component',
               0x4a355e0:'direction',0x21ce80:'trajectory',0x46c8350:'team',0x4663c30:'mode_actor',0xbfabea0:'fail'}
        globals_={0xec2d9f0:'cookie_global',0x103bad38:'session_global',0x104c2858:'direction_global',
            0x104331f0:'puck_global',0x103e3050:'possession',0x103e3408:'mode_byte',0xec2e090:'mode_global',
            0x104c26c8:'angle_global',0x104c2718:'distance_global',0x104c2768:'duration_global'}
        start,end=0x4a53570,0x4a53b70;md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
        instructions=list(md.disasm(image.read(start,end-start),start));self.assertEqual(sum(i.size for i in instructions),end-start)
        asm=['.text','.globl original_request','original_request:'];constants={}
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
extern ABI void original_request(struct panic_state*);
static const struct panic_request_input *input;
static unsigned char record[0xe8],actor[0x80],anim[0xc68],wrapper[0x200],root_wrapper[0x18],root[0x120],pose[0xa0],trajectory_object[0x20],actor_component[0x18];
static unsigned char angle_value[0x38],distance_value[0x38],direction_value[0x38],duration_value[0x38];
static unsigned long long cookie=0x12345678;
unsigned char possession[0x400],mode_byte;
void *puck_global=actor;
void *cookie_global=&cookie,*session_global,*angle_global=angle_value,*distance_global=distance_value,*direction_global=direction_value,*duration_global=duration_value,*mode_global;
ABI void *resolve(void*unused,void*context){return record;}
ABI unsigned char special(void*unused){return input->special_actor;}
ABI unsigned char session(void*unused){return input->session_result;}
ABI void *component(void*unused){return actor_component;}
ABI unsigned char direction(void*a,void*b,void*c,void*d,float threshold){return input->direction_result;}
ABI void *trajectory(void*unused){return trajectory_object;}
ABI int team(void*unused){return input->opposing_team?1:0;}
ABI void *mode_actor(void*unused){__builtin_trap();}
ABI void fail(void){__builtin_trap();}
__declspec(dllexport) void oracle(struct panic_state*s,struct panic_bindings*b,const struct panic_request_input*i){
 *(void**)(actor_component+0x10)=actor;
 input=i;panic_address saved=s->actor;s->actor=(panic_address)actor;
 *(void**)(actor+0x10)=pose;*(void**)(actor+0x60)=anim;*(int*)(actor+0x48)=i->role;*(int*)(actor+0x30)=0;
 *(float*)(pose+0x84)=i->facing;for(unsigned n=0;n<4;n++)((float*)(pose+0x30))[n]=i->position[n];
 *(void**)(anim+0xc60)=wrapper;*(void**)(wrapper+0x1f8)=root_wrapper;*(void**)(root_wrapper+0x10)=root;
 *(void**)(root+0x118)=root;*(void**)(wrapper+0x20)=wrapper;
 *(void**)(record+0x10)=b->ptm_active;*(void**)(record+0x18)=b->path_lock;*(void**)(record+0xa8)=b->stumble;
 *(void**)(record+0xb0)=b->requested;*(void**)(record+0xc0)=b->between_legs;*(void**)(record+0xc8)=b->type;
 *(const void**)(trajectory_object+0x10)=i->trajectory_sample;
 *(const void**)(trajectory_object+0x18)=i->trajectory_sample+(i->trajectory_present?4:0);
 *(float*)(angle_value+0x34)=i->angle_threshold;*(float*)(distance_value+0x34)=i->distance_threshold;*(int*)(duration_value+0x34)=i->duration;
 original_request(s);s->actor=saved;
}
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-request-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'native.s').write_text('\n'.join(asm)+'\n');(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'native.s'),
                str(repo/'scripts/nhl27/runtime/panic_state.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                for name in ('oracle','nhl27_panic_generate_request'):getattr(lib,name).argtypes=[C.POINTER(State),C.POINTER(Bindings),C.POINTER(Input)]
                rng=random.Random(275)
                angles=[-0.0,0,math.pi,-math.pi,math.tau,-math.tau,1e30,-1e30,float('nan'),float('inf'),-float('inf')]
                for index in range(8000):
                    sample=[rng.uniform(-3,3),rng.uniform(-3,3),rng.uniform(-3,3),rng.choice(angles) if index<1000 else rng.uniform(-20,20)]
                    inp=Input(rng.choice([-1,0,0,0,5,9,10]),rng.choice([-1,0,1,20]),rng.randrange(2) if index<1000 else 0,
                        rng.randrange(3),rng.randrange(2) if index<1000 else 1,rng.randrange(2),rng.randrange(4),
                        rng.uniform(-20,20),(C.c_float*4)(1,2,3,1),(C.c_float*4)(*sample),rng.uniform(-1,181),rng.uniform(0,5))
                    cached_facing=rng.choice(angles) if index<1000 else rng.uniform(-20,20)
                    path_lock=rng.choice([-1,0,1]) if index<1000 else 0
                    results=[]
                    for name in ('oracle','nhl27_panic_generate_request'):
                        state=State.from_buffer_copy(bytes([0x55])*96);state.actor=1;state.facing=cached_facing;state.position[:]=[0,0,0,1]
                        values=[C.c_ubyte(1),C.c_int(path_lock),C.c_ubyte(1),C.c_ubyte(2),C.c_float(99),C.c_ubyte(1),C.c_int(2),C.c_ubyte(0),C.c_ubyte(1)]
                        binding=Bindings(*(C.addressof(v) for v in values))
                        getattr(lib,name)(C.byref(state),C.byref(binding),C.byref(inp));results.append((bytes(state),[bytes(v) for v in values]))
                    self.assertEqual(results[0],results[1],(index,bytes(inp),cached_facing,path_lock))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
