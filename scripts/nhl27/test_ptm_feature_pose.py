"""Source-oracle checks for native pose triplet extraction and callback order."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class FeaturePoseTests(unittest.TestCase):
    def test_source_triplet(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'triplet_original',0x1f9cf0,0x1fa2f5,
            {0x202250:'cache_query',0x1d1970:'mirrored_transform',0x1d1cc0:'plain_transform',
             0xbfabea0:'fixture_fail'},{0xec2d9f0:'fixture_guard'},
            indirect_calls={0x1f9e38:'ff90a0000000',0x1f9fda:'ff90a0000000',
                0x1fa132:'ff5038',0x1fa13f:'ff5138',0x1fa193:'ff90a0000000'})
        fixture=r'''
#include "ptm_feature_pose.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
extern ABI unsigned char triplet_original(void*,void*,int,unsigned char,void*,int,unsigned char,
 float*,float*,float*,unsigned char*,unsigned char*);
A guard_value=0;A*fixture_guard=&guard_value;int _fltused=0;
static unsigned char context[0x80],poses[2][0x30],metas[2][0x110],layout[0x30];
static A vtable[21],maps[2][5],records[2][2];static int mapdata[16];static unsigned logvalue;
ABI void fixture_fail(void){__builtin_trap();}
ABI void*cache_query(void*p,void*selector){(void)selector;return p;}
ABI void*get_layout(void*p){(void)p;return layout;}
ABI void*get_root(void*p){logvalue=logvalue*10+*(unsigned*)((unsigned char*)p+8);return (unsigned char*)p+8;}
static void fill(float*out,void*pose,int id,unsigned mirror){
 unsigned tag=*(unsigned*)((unsigned char*)pose+8);logvalue=logvalue*10+tag;
 for(unsigned n=0;n<12;n++)out[n]=(float)(id*20+(int)n+(int)tag*100+(int)mirror*1000);
}
ABI void plain_transform(float*out,void*cache,void*pose,int id,int stop,void*map,float*initial){
 (void)cache;(void)map;if(stop!=-1 || initial[7]!=1)__builtin_trap();fill(out,pose,id,0);
}
ABI void mirrored_transform(float*out,void*cache,void*pose,int id,int stop,void*map,float*initial){
 (void)cache;(void)map;if(stop!=-1 || initial[7]!=1)__builtin_trap();fill(out,pose,id,1);
}
static void adapted_transform(void*c,float*out,void*cache,void*pose,int id,unsigned char mirror){
 (void)c;(void)cache;fill(out,pose,id,mirror==1);
}
static int adapted_root(void*c,void*pose){
 (void)c;unsigned char*m=*(void**)((unsigned char*)pose+0x20);
 get_root(m);return *(int*)(m+16);
}
__declspec(dllexport) unsigned run(unsigned which,const int*v,float*out,unsigned char*flags,unsigned*log){
 for(unsigned i=0;i<sizeof(context);i++)context[i]=0;
 for(unsigned i=0;i<sizeof(layout);i++)layout[i]=0;
 *(void**)(layout+0x10)=mapdata;
 vtable[7]=(A)get_root;vtable[20]=(A)get_layout;
 for(unsigned n=0;n<2;n++){
  *(void**)metas[n]=vtable;*(unsigned*)(metas[n]+8)=3+n;*(int*)(metas[n]+16)=v[2+n];
  *(void**)(poses[n]+0x20)=metas[n];*(unsigned*)(poses[n]+8)=1+n;
  for(unsigned i=0;i<5;i++)((int*)maps[n])[i]=0;
  maps[n][3]=(A)records[n];records[n][1]=(v[5]&(1<<n))?(A)poses[n]:0;
 }
 ((void**)context)[12]=maps[0];((void**)context)[13]=maps[1];context[0x48]=(unsigned char)v[4];
 for(unsigned i=0;i<36;i++)out[i]=-999;flags[0]=flags[1]=0xcd;logvalue=0;
 unsigned result;
 if(which){
  const struct ptm_feature_pose_services services={0,adapted_transform,adapted_root};
  result=nhl24_ptm_feature_triplet((void*)records[0][1],(void*)records[1][1],0,0,v[0],v[1],
   (unsigned char)v[4],out,flags,&services);
 }else result=triplet_original(context,0,v[0],0,0,v[1],1,out,out+12,out+24,flags,flags+1);
 *log=logvalue;return result;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-feature-pose-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'oracle.s').write_text(code)
            r=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.c'),str(p/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_feature_pose.c'),
                '-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_int),C.POINTER(C.c_float),C.POINTER(C.c_ubyte),C.POINTER(C.c_uint)]
                for n in range(4800):
                    v=(C.c_int*6)(n%5-1,(n//5)%5-1,(n//25)%4,(n//100)%4,(n//400)%3,(n//1200)%4)
                    a=(C.c_float*36)();b=(C.c_float*36)();fa=(C.c_ubyte*2)();fb=(C.c_ubyte*2)();la=C.c_uint();lb=C.c_uint()
                    ra=lib.run(0,v,a,fa,C.byref(la));rb=lib.run(1,v,b,fb,C.byref(lb))
                    self.assertEqual((ra,bytes(a),bytes(fa),la.value),(rb,bytes(b),bytes(fb),lb.value),f'case {n}: {list(v)}')
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
