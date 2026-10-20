"""Original PTM allocation/publication/capture ordering, including reentrancy."""
import ctypes as C
import os
from pathlib import Path
import random
import struct
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class CreateTests(unittest.TestCase):
    def test_original_constructor(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private image and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x27ffb0,0x2803be,
            {0x23a640:'unexpected',0x8bbec0:'get_allocator',0x8c1f30:'allocator_init',
             0x8c1fb0:'allocator_copy',0x8c1f70:'allocator_name',0x2804d0:'capture_original',0xbfabea0:'fail'},
            {0xec2d9f0:'guard_pointer',0xec60090:'zero_vector',0xf298910:'invalid',
             0xdc8af0b:'label',0xd9970b6:'label',0xddc9a1e:'label'},
            indirect_calls={0x280209:'41ff5218',0x280385:'ff5028'})
        fixture=r'''
#include "ptm_transition.h"
#define ABI __attribute__((sysv_abi))
typedef panic_address A;typedef unsigned char B;
extern ABI void original(void*,void*,void*,void*,const float*,float);
static B match[0xbbc8],config[0x120],controller[0x20],selection[0x18],db[0xd0],clip[0x40],frames[0x10];
static struct ptm_transition state,alternate;static A *slot;
static A clips[1],controller_vt[6],allocator_vt[4],allocator[1];
static unsigned bad,events[16],used,mode;static float target[4];
A guard=1,*guard_pointer=&guard,invalid=777;float zero_vector[4];char label[4];int _fltused=0;
ABI void fail(void){__builtin_trap();}
ABI void *unexpected(void*p){(void)p;__builtin_trap();}
static void event(unsigned e){events[used++]=e;}
ABI void *alloc_original(void*a,A bytes,const char*name,unsigned flags,unsigned align,unsigned extra){
 if(a!=allocator||bytes!=0x4a0||name!=label||flags!=1||align!=16||extra)bad++;
 if(*slot)bad++;event(1);
 if(mode&2){*(unsigned*)(config+0x10c)=3;config[0x111]=2;target[0]=-9;}
 return &state;
}
ABI void *get_allocator(void){return allocator;}
ABI void allocator_init(void*out,void*name){if(name!=label)bad++;*(A*)out=456;}
ABI void allocator_copy(void*out,void*in){*(A*)out=*(A*)in;}
ABI void allocator_name(void*out,void*name){if(name!=label)bad++;*(A*)out=456;}
ABI void destroy_original(void*p){if(p!=controller||*slot!=(A)&state)bad++;event(2);*slot=(A)&alternate;}
ABI void capture_original(struct ptm_transition*s,void*e){
 if(e!=(void*)999||s!=(void*)*slot)bad++;event(3);
 if(s==&alternate)event(4);else event((unsigned)*(int*)(controller+8));
}
static struct ptm_transition *allocate(void*c,A bytes){return alloc_original(allocator,bytes,label,1,16,0);}
static void destroy(void*c,void*p){destroy_original(p);}
static void capture(void*c,struct ptm_transition*s,void*e){capture_original(s,e);}
static const struct ptm_creation_services services={0,allocate,destroy,capture};
__declspec(dllexport) void run(unsigned which,unsigned m,const float*values,unsigned*out){
 mode=m;bad=used=0;for(unsigned n=0;n<16;n++)events[n]=0;
 for(unsigned n=0;n<sizeof(state);n++)((B*)&state)[n]=0xcc;
 slot=(A*)(match+0xbbc0);*slot=0;
 *(void**)(match+0x86b0)=config;
 *(unsigned*)(config+0x10c)=m%4;config[0x111]=(B)(m%3);
 for(unsigned n=0;n<4;n++){((float*)(match+0x9200))[n]=1;target[n]=n+0.25f;}
 *(float*)(match+0x88)=values[0];*(float*)(frames+4)=values[1];*(float*)(selection+0x10)=values[2];
 *(unsigned*)selection=m&1;*(void**)(selection+8)=db;*(void**)(db+0xc8)=clips;
 clips[0]=(A)clip;*(void**)(clip+0x38)=frames;
 *(void**)controller=controller_vt;controller_vt[5]=(A)destroy_original;
 *(int*)(controller+8)=(m&4)?-1:5;
 allocator[0]=(A)allocator_vt;allocator_vt[3]=(A)alloc_original;
 if(which){
  struct ptm_creation args={match,controller,clip,match+0x8c20,match+0x96f0,invalid,456,
   (values[1]+values[2])+0.0f,values[3]*values[0],(unsigned*)(config+0x10c),config+0x111,(B)(m&1),target};
  nhl27_ptm_transition_create((struct ptm_transition**)slot,&args,(void*)999,&services);
 }else original(match,(void*)999,selection,controller,target,values[3]);
 out[0]=bad;out[1]=used;for(unsigned n=0;n<16;n++)out[2+n]=events[n];
 out[18]=*(unsigned*)(controller+8);out[19]=*slot==(A)&alternate;
 /* Normalize only instance-relative pointers before comparing every byte. */
 state.match=(void*)1;state.controller=(void*)2;state.clip=(void*)3;
 state.trajectory=(void*)4;state.match_state=(void*)5;
 state.begin-= (A)state.inline_vectors;state.end-=(A)state.inline_vectors;
 state.capacity-=(A)state.inline_vectors;state.inline_storage-=(A)state.inline_vectors;
 for(unsigned n=0;n<sizeof(state)/4;n++)out[20+n]=((unsigned*)&state)[n];
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-create-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            r=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'oracle.s'),
                str(root/'scripts/nhl27/runtime/ptm_transition.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(out/'test.dll'));lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_float),C.POINTER(C.c_uint)]
            rng=random.Random(0x27ffb0)
            for n in range(2048):
                vals=[rng.uniform(-2,2) for _ in range(4)]
                if n%31==0:vals[n%4]=[float('nan'),float('inf'),-float('inf'),-0.0][n%4]
                a=(C.c_float*4)(*vals);x=(C.c_uint*316)();y=(C.c_uint*316)()
                lib.run(0,n%24,a,x);lib.run(1,n%24,a,y)
                self.assertEqual(bytes(x),bytes(y),f'case {n}');self.assertEqual(y[0],0)
            import _ctypes
            _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
