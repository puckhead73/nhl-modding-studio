"""Compare source previous-pose triplet ordering, interval and root selection."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class PreviousPoseTests(unittest.TestCase):
    def test_source_previous_triplet(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and LLVM required')
        im=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(im,'original',0x1f9ab0,0x1f9ce2,
            {0x2024f0:'get_original',0xbfabea0:'fail'},{0xec2d9f0:'guard'},
            indirect_calls={0x1f9b01:'ff90b0000000',0x1f9b17:'ff10',0x1f9c27:'ff5038'})
        fixture=r'''
#include "ptm_previous_pose.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
int _fltused=0;static A cookie;A *guard=&cookie;
static unsigned char context[0x80],contexts[0x40],record[0x30],properties[0x30],rig[0x30],rigrecord[0x8a0],slots[2][0x40];
static A history,property_vt[23],property,meta_vt[8],meta;static int root_data[3];
static unsigned logs[24],used,calls,interval;
ABI void fail(void){__builtin_trap();}
ABI void *history_get(void*p,void*q){(void)p;(void)q;return &history;}
ABI void *root_get(void*p){(void)p;logs[used++]=99;return root_data;}
ABI void *get_original(float*out,void*ctx,void*slot,int joint,int future,void*h,float*dt){
 (void)ctx;(void)h;logs[used++]=(unsigned)joint;logs[used++]=(unsigned)future;
 logs[used++]=slot==slots[0]?0:1;
 for(unsigned i=0;i<12;i++)out[i]=(float)(joint*3+future*17+(int)i);
 *(unsigned*)dt=calls++==1?interval:0x3f000000u;return out;
}
static void get(void*c,void*slot,int joint,panic_u8 future,float*out,float*dt){get_original(out,c,slot,joint,future,0,dt);}
static int root_id(void*c,void*slot){(void)c;(void)slot;root_get(0);return root_data[2];}
extern ABI int original(void*,void*,int,panic_u8,void*,int,panic_u8,float*,float*,float*,float*,panic_u8*,panic_u8*);
__declspec(dllexport) unsigned run(unsigned which,int j,int r,int root,unsigned jf,unsigned rf,unsigned delta,unsigned*out){
 used=calls=0;interval=delta;for(unsigned i=0;i<24;i++)logs[i]=0;
 property_vt[22]=(A)history_get;property=(A)property_vt;
 *(A*)(properties+0x28)=(A)&property;*(A*)(context+0x50)=(A)properties;
 *(A*)(context+0x40)=(A)contexts;*(A*)(contexts+0x18)=(A)record;
 *(A*)(context+0x78)=(A)rig;*(A*)(rig+0x20)=(A)rigrecord;*(A*)(rig+0x28)=(A)(rigrecord+0x8a0);
 meta_vt[7]=(A)root_get;meta=(A)meta_vt;*(A*)(rigrecord+0x890)=(A)&meta;root_data[2]=root;
 float transforms[36];for(unsigned i=0;i<36;i++)transforms[i]=99.0f;
 float dt=2;panic_u8 flags[2]={0xa5,0xa5};unsigned result;
 if(which){struct ptm_previous_pose_services s={context,get,root_id};
  result=nhl27_ptm_previous_triplet(slots[0],j,(panic_u8)jf,slots[1],r,(panic_u8)rf,transforms,&dt,flags,&s);
 }else result=original(context,slots[0],j,(panic_u8)jf,slots[1],r,(panic_u8)rf,&dt,
    transforms,transforms+12,transforms+24,flags,flags+1);
 for(unsigned i=0;i<36;i++)out[i]=((unsigned*)transforms)[i];
 out[36]=*(unsigned*)&dt;out[37]=flags[0];out[38]=flags[1];out[39]=used;
 for(unsigned i=0;i<24;i++)out[40+i]=logs[i];return result&255;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-previous-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'oracle.s').write_text(code)
            r=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),str(p/'fixture.c'),
                str(p/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_previous_pose.c'),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_int,C.c_int,C.c_int,C.c_uint,C.c_uint,C.c_uint,C.POINTER(C.c_uint)]
                deltas=[0,0x80000000,0x33ffffff,0x34000000,0x40000000,0x7fc12345,0x7f800000]
                for n in range(2268):
                    args=(n%3-1,(n//3)%3-1,(n//9)%3-1,(n//27)%3,(n//81)%3,deltas[(n//243)%7])
                    a=(C.c_uint*64)();b=(C.c_uint*64)()
                    ra=lib.run(0,*args,a);rb=lib.run(1,*args,b)
                    self.assertEqual((ra,list(a)),(rb,list(b)),str(args))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
