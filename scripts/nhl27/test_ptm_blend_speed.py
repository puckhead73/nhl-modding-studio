"""Compare native-layout blend speed clamping against NHL27's setter."""
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


class BlendSpeedTests(unittest.TestCase):
    def test_source_normal_search_setter(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        indirect={0x188c8b:'ff5040',0x188ca0:'ff5120',0x188ccd:'ff5040',
                  0x188ce2:'ff5120',0x188d19:'ff5040',0x188d4a:'ff5040',0x188d5b:'ff5120'}
        code=assembly(image,'speed_original',0x188c50,0x188dc0,{}, {},indirect_calls=indirect)
        fixture=r'''
#include "ptm_blend_timing.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
extern ABI void speed_original(void*,void*,unsigned char,float);
struct query{A *vt;float value;unsigned id;};
static unsigned char state[0x8560],params[2][0x60];
static A vt[9];static struct query queries[4];
static unsigned which,mode,logvalue;static float replacement,observed;
static unsigned pointer_offset,value_offset;
ABI void *cast(void *p,unsigned id){if(id!=0x67745308)__builtin_trap();return p;}
ABI float get(void *p,void *ctx){
 struct query*q=p;if(ctx!=state)__builtin_trap();
 logvalue=logvalue*10+q->id;
 observed=*(float*)(state+value_offset);
 if(q->id==1 && (mode&4))*(void**)(state+pointer_offset)=params[1];
 if(mode&8)*(float*)(state+value_offset)=replacement;
 return q->value;
}
int _fltused=0;
__declspec(dllexport) void run(unsigned w,unsigned flags,const float*v,float*out,unsigned*log){
 which=w;mode=flags;logvalue=0;observed=0;replacement=v[5];
 for(unsigned i=0;i<sizeof(state);++i)state[i]=0;
 for(unsigned j=0;j<2;++j)for(unsigned i=0;i<0x60;++i)params[j][i]=0;
 pointer_offset=w?0x1100:0x8550;value_offset=w?0x88:0x18;
 unsigned maxp=w?0x10:0x30,minp=w?0x18:0x38,maxv=w?0x20:0x50,minv=w?0x24:0x54;
 vt[3]=(A)get;vt[4]=(A)get;vt[8]=(A)cast;
 for(unsigned i=0;i<4;++i){queries[i].vt=vt;queries[i].id=i+1;queries[i].value=v[i+1];}
 for(unsigned i=0;i<2;++i){
  *(void**)(params[i]+maxp)=(flags&1)?queries+2*i:0;
  *(void**)(params[i]+minp)=(flags&2)?queries+2*i+1:0;
  *(float*)(params[i]+maxv)=v[1+2*i];*(float*)(params[i]+minv)=v[2+2*i];
 }
 *(void**)(state+pointer_offset)=(flags&16)?0:params[0];
 if(w)nhl24_ptm_blend_set_speed(state,state,v[0]);else speed_original(state,state,0,v[0]);
 out[0]=*(float*)(state+value_offset);out[1]=observed;
 *log=logvalue;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-speed-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'oracle.s').write_text(code)
            r=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.c'),str(p/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_blend_timing.c'),
                '-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_float),C.POINTER(C.c_float),C.POINTER(C.c_uint)]
                rng=random.Random(0x188c50)
                edges=[0.,-0.,float('nan'),float('inf'),-float('inf'),1.,-1.]
                for n in range(4096):
                    vals=[rng.uniform(-4,4) for _ in range(6)]
                    if n%3==0:vals[n%6]=edges[n%len(edges)]
                    v=(C.c_float*6)(*vals);a=(C.c_float*2)();b=(C.c_float*2)();la=C.c_uint();lb=C.c_uint()
                    lib.run(0,n%32,v,a,C.byref(la));lib.run(1,n%32,v,b,C.byref(lb))
                    self.assertEqual((bytes(a),la.value),(bytes(b),lb.value),f'case {n}: {vals}')
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
