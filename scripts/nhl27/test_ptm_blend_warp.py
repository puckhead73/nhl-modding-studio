"""Compare the complete source target update with the native layout adapter."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage,SelfImage
from oracle_fixture import assembly
from ptm_trajectory_math import generate


class BlendWarpTests(unittest.TestCase):
    def test_source_target_update(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        source=checked_image(ElfImage,Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'),SOURCE_HASH)
        native=SelfImage(Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))
        code,_=generate(source)
        code+=assembly(source,'original',0x23a8d0,0x23ab81,
            {0x250f50:'target_original',0xbfabea0:'fail'},{0xec2d9f0:'guard'},
            indirect_calls={0x23a918:'ff5040',0x23a929:'ff5120'})
        code+=assembly(source,'target_original',0x250f50,0x25104d,{0xbfabf50:'remainder_native'}, {},
            indirect_calls={0x250f77:'ff5040',0x250f88:'ff5120',0x250fc6:'ff5040',0x250fd7:'ff5120'})
        self.assertEqual(native.read(0x90802f8,8),bytes.fromhex('0000000000000080'))
        code+=assembly(native,'remainder_native',0x7519d80,0x7519f20,{0x75189d0:'math_error'}, {0x90802f8:'signed_zero'})
        code+='\n.balign 8\nsigned_zero:\n.long 0,0x80000000\n'
        fixture=r'''
#include "ptm_blend_storage.h"
typedef panic_address A;typedef unsigned char B;
#define ABI PTM_GAME_ABI
int _fltused=0;static A cookie;A*guard=&cookie;
static B sm[0x8800],nm[0x17d0],sp[0x120],np[0xb0],sw[0x120],nw[0xe0];
static B ss[0x40],ns[0x20],sr[0x4d0*5],nr[0x80*5];
static float samples[16*4],values[4];static unsigned events,enabled;
static A bv[9],pv[9],fv[9],bo,po,fo;
struct v3 {float x,y,z;};
extern ABI void original(void*,void*,float);
extern ABI float remainder_native(float,float);
extern void nhl24_ptm_blend_warp(void*,void*,void*,float,float(ABI*)(float,float));
ABI void fail(void){__builtin_trap();}
ABI void math_error(unsigned n){(void)n;}
ABI void*cast(void*p,unsigned id){(void)id;return p;}
ABI B boolean(void*p,void*c){if(p!=&bo||c!=(void*)42)fail();events=events*10+1;return (B)enabled;}
ABI struct v3 position(void*p,void*c){if(p!=&po||c!=(void*)42)fail();events=events*10+2;return (struct v3){values[0],values[1],values[2]};}
ABI float facing(void*p,void*c){if(p!=&fo||c!=(void*)42)fail();events=events*10+3;return values[3];}
__declspec(dllexport) void run(unsigned which,unsigned flags,int category,unsigned count,const float*v,float*out){
 for(unsigned i=0;i<sizeof(sm);i++)sm[i]=0;
 for(unsigned i=0;i<sizeof(nm);i++)nm[i]=0;
 for(unsigned i=0;i<sizeof(sw);i++)sw[i]=0;
 for(unsigned i=0;i<sizeof(nw);i++)nw[i]=0;
 for(unsigned i=0;i<64;i++)samples[i]=v[i];
 for(unsigned i=0;i<4;i++)values[i]=v[64+i];
 bv[8]=pv[8]=fv[8]=(A)cast;bv[4]=bv[3]=(A)boolean;
 pv[4]=pv[3]=(A)position;fv[4]=fv[3]=(A)facing;
 bo=(A)bv;po=(A)pv;fo=(A)fv;events=0;enabled=(flags>>4)%3;
 B *m=which?nm:sm,*p=which?np:sp,*w=which?nw:sw;
 *(void**)(m+(which?0x11f0:0x86b0))=p;*(void**)(p+(which?0xa8:0xd0))=w;
 *(void**)(w+(which?0x18:0x38))=(flags&1)?&bo:0;
 *(void**)(w+(which?8:0x50))=(flags&2)?&po:0;
 *(void**)(w+(which?0x10:0x58))=(flags&4)?&fo:0;
 w[which?0xd0:0x114]=(flags&8)?1:0;
 *(void**)(m+(which?0x11a8:0x8668))=which?ns:ss;
 *(int*)((which?ns:ss)+(which?0x18:0x38))=category;
 *(void**)(m+(which?0x330:0x31c0))=which?nr:sr;
 for(unsigned i=0;i<(which?4u:5u);i++)((int*)(m+(which?0x320:0x31a0)))[i]=(int)i;
 for(unsigned i=0;i<6;i++){
  B *path=i==5?m+(which?0x13c0:0x8720):(which?nr:sr)+i*(which?0x80:0x4d0)+0x10;
  *(void**)(path+0x10)=samples;*(void**)(path+0x18)=samples+count*4;
  *(float*)(path+(which?0x50:0x480))=v[68];*(float*)(path+(which?0x54:0x484))=v[69];
 }
 *(float*)(m+0x88)=v[70];
 if(which)nhl24_ptm_blend_warp(m,(void*)42,nr+4*0x80+0x10,v[71],remainder_native);
 else original(m,(void*)42,v[71]);
 B *result=m+(which?0x1170:0x8630);
 for(unsigned i=0;i<11;i++)((unsigned*)out)[i]=((unsigned*)result)[i];
 ((unsigned*)out)[11]=events;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-blend-warp-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'oracle.s').write_text(code)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.c'),str(p/'oracle.s'),*[str(root/'scripts/nhl27/runtime'/n) for n in
                ['ptm_blend_warp.c','ptm_transition.c','ptm_native.c','ptm_offsets.c','ptm_trajectory.c','ptm_integration.c']],
                '-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(p/'fixture.dll'));F=C.POINTER(C.c_float)
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.c_int,C.c_uint,F,F]
                rng=random.Random(0x23a8d0)
                for n in range(4096):
                    vals=[rng.uniform(-100,100) for _ in range(72)]
                    vals[68]=[.1,.5,1,2][n%4]
                    if n%13==0:vals[67]=[0.,-0.,float('inf'),float('nan'),-float('inf')][n%5]
                    v=(C.c_float*72)(*vals);a=(C.c_float*12)();b=(C.c_float*12)()
                    args=(n%48,n%9-2,n%16,v)
                    lib.run(0,*args,a);lib.run(1,*args,b)
                    self.assertEqual(bytes(a),bytes(b),(n,list(a),list(b)))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
