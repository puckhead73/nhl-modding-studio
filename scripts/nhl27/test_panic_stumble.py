"""Compare stumble requests against all original instructions of 4a53b70."""
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
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
from test_panic_state import Bindings


class Input(C.Structure):
    _fields_=[('actor_team',C.c_int),('possession_team',C.c_int),('attribute_active',C.c_ubyte),
        ('possessor_present',C.c_ubyte)]+[(n,C.c_float) for n in
        ('rating','speed','speed_threshold','rating_threshold','radius')]+[
        ('position',C.c_float*4),('possessor_position',C.c_float*4)]


class StumbleTests(unittest.TestCase):
    def test_original_machine_gates_and_preserved_bytes(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        calls={0x390dd0:'resolve',0x452baa0:'attributes',0x3c6280:'attribute',
               0x452d430:'speed',0x46c8350:'team'}
        globals_={0x103e3050:'possession',0x104c2ad8:'speed_global',
                  0x104c2a88:'rating_global',0x104c2b28:'radius_global'}
        start,end=0x4a53b70,0x4a53ce0
        md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
        instructions=list(md.disasm(image.read(start,end-start),start))
        self.assertEqual(sum(i.size for i in instructions),end-start)
        asm=['.text','.globl original_stumble','original_stumble:'];constants={}
        def emit(raw):
            if raw:asm.append('.byte '+','.join(str(v) for v in raw))
        for ins in instructions:
            fix=None
            if ins.mnemonic=='call':
                self.assertEqual(ins.operands[0].type,X86_OP_IMM)
                fix=(ins.imm_offset,calls[ins.operands[0].imm])
            elif ins.mnemonic.startswith('j'):
                self.assertTrue(start<=ins.operands[0].imm<end)
            for op in ins.operands:
                if op.type==X86_OP_MEM and op.mem.base==X86_REG_RIP:
                    self.assertIsNone(fix);address=ins.address+ins.size+op.mem.disp
                    if address in globals_:name=globals_[address]
                    else:
                        self.assertTrue(0xded0000<=address<0xdee0000)
                        name=f'constant_{address:x}';constants[name]=image.read(address,op.size)
                    fix=(ins.disp_offset,name)
            if fix:
                offset,name=fix;emit(ins.bytes[:offset])
                asm.append(f'.long {name} - . - {ins.size-offset}');emit(ins.bytes[offset+4:])
            else:emit(ins.bytes)
        for name,data in constants.items():asm.extend(['.balign 16',name+':']);emit(data)
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original_stumble(struct panic_state*);
static const struct panic_stumble_input *input;
static unsigned char record[0xe8],actor[0x200],other[0x80],anim[0xc68],wrapper[0x200];
static unsigned char root_wrapper[0x18],root[0x120],pose[0xa0],other_pose[0xa0];
static unsigned char speed_value[0x38],rating_value[0x38],radius_value[0x38];
void *possession,*speed_global=speed_value,*rating_global=rating_value,*radius_global=radius_value;
ABI void *resolve(void*a,void*b){return record;}
ABI void *attributes(void*a){return actor;}
ABI unsigned char attribute(void*a,unsigned hash,unsigned index){
 if(hash!=0xf34b2901 || index)__builtin_trap();return input->attribute_active;
}
ABI float speed(void*a){return input->speed;}
ABI int team(void*a){return input->possession_team;}
__declspec(dllexport) void oracle(struct panic_bindings*b,const struct panic_stumble_input*i){
 input=i;struct panic_state state={0};state.actor=(panic_address)actor;
 *(void**)(actor+0x10)=pose;*(void**)(actor+0x60)=anim;
 *(int*)(actor+0x30)=i->actor_team;*(float*)(actor+0x19c)=i->rating;
 *(void**)(other+0x10)=other_pose;possession=i->possessor_present?other:0;
 for(unsigned n=0;n<4;n++){
  ((float*)(pose+0x30))[n]=i->position[n];
  ((float*)(other_pose+0x30))[n]=i->possessor_position[n];
 }
 *(void**)(anim+0xc60)=wrapper;*(void**)(wrapper+0x1f8)=root_wrapper;
 *(void**)(root_wrapper+0x10)=root;*(void**)(root+0x118)=root;*(void**)(wrapper+0x20)=wrapper;
 *(void**)(record+0xa8)=b->stumble;*(void**)(record+0xb0)=b->requested;
 *(float*)(speed_value+0x34)=i->speed_threshold;*(float*)(rating_value+0x34)=i->rating_threshold;
 *(float*)(radius_value+0x34)=i->radius;
 original_stumble(&state);
}
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-stumble-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'native.s').write_text('\n'.join(asm)+'\n');(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'native.s'),str(repo/'scripts/nhl27/runtime/panic_state.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                for name in ('oracle','nhl27_panic_request_stumble'):
                    getattr(lib,name).argtypes=[C.POINTER(Bindings),C.POINTER(Input)]
                rng=random.Random(2753)
                edge=[0.,-0.,math.inf,-math.inf,math.nan,1e30,-1e30]
                for index in range(12000):
                    def value(a,b):
                        return rng.choice(edge) if index<2500 and rng.randrange(3)==0 else rng.uniform(a,b)
                    inp=Input(rng.choice([-1,0,1,2]),rng.choice([-1,0,1,2]),
                        rng.randrange(4) if index<2000 else 0,rng.randrange(2),
                        value(0,100),value(0,10),value(0,10),value(0,1),value(0,20),
                        (C.c_float*4)(*(value(-1,1) for _ in range(4))),
                        (C.c_float*4)(*(value(-1,1) for _ in range(4))))
                    request=rng.randrange(4) if index<3000 else 1;stumble=rng.randrange(256)
                    missing=index%17;results=[]
                    for name in ('oracle','nhl27_panic_request_stumble'):
                        r=C.c_ubyte(request);s=C.c_ubyte(stumble);binding=Bindings()
                        binding.requested=C.addressof(r) if missing!=0 else 0
                        binding.stumble=C.addressof(s) if missing!=1 else 0
                        getattr(lib,name)(C.byref(binding),C.byref(inp))
                        results.append((bytes(r),bytes(s),bytes(binding)))
                    # Pointers differ between fixtures; only pointed-to bytes are outputs.
                    self.assertEqual(results[0][:2],results[1][:2],(index,bytes(inp),request,stumble))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
