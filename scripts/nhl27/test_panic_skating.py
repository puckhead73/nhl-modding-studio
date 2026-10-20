"""Compare type-1 reactions with the complete original NHL27 machine function."""
import ctypes as C
import itertools
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_REG_RIP
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
from test_panic_state import State, Bindings


class Input(C.Structure):
    _fields_=[(n,C.c_int) for n in ('role','actor_team','possessor_team','own_rank','opponent_rank')]+[
        (n,C.c_ubyte) for n in ('special','possessor_present','own_present','opponent_present','panic','session')]+[
        ('facing',C.c_float),('position',C.c_float*4)]


class SkatingReactionTests(unittest.TestCase):
    def test_original_function_all_decision_branches_and_state_preservation(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        calls={0x390dd0:'resolve',0x453df90:'special',0x465c3f0:'session',
               0x49fc260:'opponent',0x49fc240:'own',0x4a1ab90:'panic'}
        globals_={0x103bad38:'session_global',0x1042c4c0:'manager_global',0x103e3050:'possessor_global'}
        start,end=0x4a53360,0x4a53570
        md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
        assembly=['.text','.globl original_reaction','original_reaction:']
        instructions=list(md.disasm(image.read(start,end-start),start))
        self.assertEqual(sum(i.size for i in instructions),end-start)
        def emit(raw):
            if raw:assembly.append('.byte '+','.join(str(v) for v in raw))
        for i in instructions:
            fix=None
            if i.mnemonic=='call':
                self.assertEqual(i.operands[0].type,X86_OP_IMM)
                fix=(i.imm_offset,calls[i.operands[0].imm])
            elif i.mnemonic.startswith('j') and i.operands[0].type==X86_OP_IMM:
                self.assertTrue(start<=i.operands[0].imm<end)
            for o in i.operands:
                if o.type==X86_OP_MEM and o.mem.base==X86_REG_RIP:
                    self.assertIsNone(fix)
                    fix=(i.disp_offset,globals_[i.address+i.size+o.mem.disp])
            if fix:
                off,name=fix;emit(i.bytes[:off])
                assembly.append(f'.long {name} - . - {i.size-off}')
                emit(i.bytes[off+4:])
            else:emit(i.bytes)
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original_reaction(struct panic_state*);
static const struct panic_skating_input *input;
static unsigned char record[0xe8],actor[0x80],anim[0xc68],wrapper[0x200];
static unsigned char root_wrapper[0x18],root[0x120],pose[0xa0];
static unsigned char manager[0x3d0],cache[0x48],possessor_actor[0x40],own_entry[0x34],opponent_entry[0x34];
void *session_global,*manager_global,*possessor_global;
ABI void *resolve(void *unused,void *context){return record;}
ABI unsigned char special(void *unused){return input->special_actor;}
ABI unsigned char session(void *unused){return input->session_result;}
ABI void *opponent(void *unused,void *actor){return input->opponent_rank_present?opponent_entry:0;}
ABI void *own(void *unused,void *actor){return input->own_rank_present?own_entry:0;}
ABI unsigned char panic(void *unused){return input->animation_is_panic;}
__declspec(dllexport) void oracle(struct panic_state*s,struct panic_bindings*b,const struct panic_skating_input*i){
    input=i;panic_address saved=s->actor;s->actor=(panic_address)actor;
    *(void**)(actor+0x10)=pose;*(void**)(actor+0x60)=anim;
    *(int*)(actor+0x48)=i->role;*(int*)(actor+0x30)=i->actor_team;
    *(float*)(pose+0x84)=i->facing;for(unsigned n=0;n<4;n++)((float*)(pose+0x30))[n]=i->position[n];
    *(void**)(anim+0xc60)=wrapper;*(void**)(wrapper+0x1f8)=root_wrapper;
    *(void**)(root_wrapper+0x10)=root;*(void**)(root+0x118)=root;
    *(void**)(wrapper+0x20)=wrapper;
    *(void**)(record+0x10)=b->ptm_active;*(void**)(record+0xa8)=b->stumble;
    *(void**)(record+0xb0)=b->requested;*(void**)(record+0xc0)=b->between_legs;*(void**)(record+0xc8)=b->type;
    manager_global=manager;*(void**)(manager+0x3c8)=cache;*(void**)(cache+0x40)=cache;
    possessor_global=i->possessor_present?possessor_actor:0;
    *(int*)(possessor_actor+0x30)=i->possessor_team;
    *(int*)(own_entry+0x30)=i->own_rank;*(int*)(opponent_entry+0x30)=i->opponent_rank;
    original_reaction(s);s->actor=saved;
}
'''
        root_path=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-skating-',dir=root_path/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'original.s').write_text('\n'.join(assembly)+'\n');(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared',
                '-fuse-ld=lld','-nostdlib','-Wl,/noentry','-I',str(root_path/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'original.s'),str(root_path/'scripts/nhl27/runtime/panic_state.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                for name in ('oracle','nhl27_panic_skating_reaction'):
                    getattr(lib,name).argtypes=[C.POINTER(State),C.POINTER(Bindings),C.POINTER(Input)]
                scenarios=list(itertools.product((0,1),(0,1),(0,1),(0,1),(0,1,2),(0,1),(0,1)))
                scenarios += [(1,1,1,0,1,1,1)]*6
                for index,(poss,own,opp,team,active,panic,special) in enumerate(scenarios):
                    inp=Input(0,0,team,index%3-1,0,special,poss,own,opp,panic,1,1.25,(C.c_float*4)(1,2,3,4))
                    if index>=len(scenarios)-6:inp.role=(-1,4,5,9,10,-2147483648)[index-len(scenarios)+6];inp.special=0
                    results=[]
                    for name in ('oracle','nhl27_panic_skating_reaction'):
                        state=State.from_buffer_copy(bytes([0x55])*96);state.actor=1
                        values=[C.c_ubyte(active),C.c_int(11),C.c_ubyte(1),C.c_ubyte(2),C.c_float(99),C.c_ubyte(1),C.c_int(1),C.c_ubyte(0),C.c_ubyte(1)]
                        bindings=Bindings(*(C.addressof(v) for v in values))
                        getattr(lib,name)(C.byref(state),C.byref(bindings),C.byref(inp))
                        results.append((bytes(state),[bytes(v) for v in values]))
                    self.assertEqual(results[0],results[1],(index,bytes(inp)))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
