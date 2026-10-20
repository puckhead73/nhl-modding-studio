"""Original 4a52300 oracle for takeoff phases and cancellation blending."""
import ctypes as C
import math
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from test_panic_state import State
from oracle_fixture import assembly


class Input(C.Structure):
    _fields_=[('feature',C.c_ubyte),('effect',C.c_ubyte)]+[(n,C.c_int) for n in
        ('tick','effect_start','blend_in','hold','blend_out')]+[
        ('cancellation_rate',C.c_float),('maximum',C.c_float*4),('values',C.c_void_p*4)]


class TakeoffTests(unittest.TestCase):
    def test_source_machine_phases_and_float_edges(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        asm=assembly(image,'original_takeoff',0x4a52300,0x4a52840,
            {0x452e0e0:'feature',0x390dd0:'resolve',0x398890:'attributes',0x3c6110:'effect',0x69bb3d0:'original_blend'},
            {0x103e1418:'clock_data',0x104c2f38:'hold_global',0x104c2ee8:'in_global',0x104c2f88:'out_global',
             0x104c3028:'max0',0x104c3078:'max1',0x104c30c8:'max2',0x104c3118:'max3',0x104c2fd8:'rate_global'})
        asm+=assembly(image,'original_blend',0x69bb3d0,0x69bb440,{},{})
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original_takeoff(struct panic_state*);
static const struct panic_takeoff_input *input;
static unsigned char record[0x130],actor[0x80],anim[0xc68],wrapper[0x200],root_wrapper[0x18],root[0x120];
static unsigned char feature_data[0x80],effect_data[0x20],attribute_record[0x130],attribute_asset[0x30];
static unsigned char in_value[0x38],hold_value[0x38],out_value[0x38],rate_value[0x38],max_values[4][0x38];
unsigned char clock_data[0x2d0];
void *in_global=in_value,*hold_global=hold_value,*out_global=out_value,*rate_global=rate_value;
void *max0=max_values[0],*max1=max_values[1],*max2=max_values[2],*max3=max_values[3];
ABI void *feature(void*a){return input->feature_present?feature_data:0;}
ABI void *resolve(void*a,void*b){return record;}
ABI void *attributes(void*a,void*b){return attribute_record;}
ABI void *effect(void*a,unsigned hash,int mode){
 if(hash!=0xce986eb5 || mode!=2)__builtin_trap();return input->effect_present?effect_data:0;
}
__declspec(dllexport) void oracle(struct panic_state*s,const struct panic_takeoff_input*i){
 input=i;panic_address saved=s->actor;s->actor=(panic_address)actor;
 *(void**)(actor+0x60)=anim;*(void**)(anim+0xc60)=wrapper;
 *(void**)(wrapper+0x1f8)=root_wrapper;*(void**)(root_wrapper+0x10)=root;
 *(void**)(root+0x118)=root;*(void**)(wrapper+0x20)=wrapper;
 *(void**)(record+0x128)=attribute_asset;*(void**)(attribute_asset+0x28)=attribute_asset;
 unsigned offsets[4]={0x128,0x108,0x100,0xf8};
 for(unsigned n=0;n<4;n++){
  *(void**)(attribute_record+offsets[n])=i->values[n];*(float*)(max_values[n]+0x34)=i->maximum[n];
 }
 *(int*)(clock_data+0x2cc)=i->tick;*(int*)(effect_data+0x10)=i->effect_start;
 *(int*)(in_value+0x34)=i->blend_in;*(int*)(hold_value+0x34)=i->hold;*(int*)(out_value+0x34)=i->blend_out;
 *(float*)(rate_value+0x34)=i->cancellation_rate;
 original_takeoff(s);s->actor=saved;
}
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-takeoff-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'native.s').write_text(asm);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'native.s'),str(repo/'scripts/nhl27/runtime/panic_state.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                for name in ('oracle','nhl27_panic_takeoff'):
                    getattr(lib,name).argtypes=[C.POINTER(State),C.POINTER(Input)]
                rng=random.Random(27523);edges=[0.,-0.,math.nan,math.inf,-math.inf,1e30,-1e30]
                for index in range(12000):
                    def value():return rng.choice(edges) if index<3000 else rng.uniform(-4,4)
                    inp=Input(rng.randrange(2) if index<1000 else 1,rng.randrange(2),
                        rng.randrange(-50,150),rng.randrange(-50,150),rng.randrange(-5,30),rng.randrange(-5,30),
                        rng.randrange(-5,60),value(),(C.c_float*4)(*(value() for n in range(4))))
                    baseline=[value() for n in range(4)];initial=[value() for n in range(4)]
                    missing=index%6;results=[]
                    for name in ('oracle','nhl27_panic_takeoff'):
                        state=State.from_buffer_copy(bytes([0x55])*96);state.actor=1;state.attributes[:]=baseline
                        values=[C.c_float(v) for v in initial]
                        inp.values[:]=[C.addressof(v) if n!=missing else 0 for n,v in enumerate(values)]
                        getattr(lib,name)(C.byref(state),C.byref(inp))
                        results.append((bytes(state),[bytes(v) for v in values]))
                    self.assertEqual(results[0],results[1],(index,bytes(inp),baseline,initial))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
