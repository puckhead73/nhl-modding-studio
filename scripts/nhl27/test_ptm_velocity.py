"""Validate the velocity adapter against the complete source feature query."""
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
from ptm_trajectory_math import generate


class VelocityTests(unittest.TestCase):
    def test_source_feature_reference_frames(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=generate(image)[0]
        code+=assembly(image,'query_original',0x24b1d0,0x24b746,
            {0x248c40:'override_query',0x1f9ab0:'previous_query',0x1f9cf0:'current_query',
             0x2028e0:'nhl27_ptm_transform_inverse',0x202420:'nhl27_ptm_transform_compose',
             0xbfabea0:'fixture_fail'}, {0xec2d9f0:'fixture_guard'},
            indirect_calls={0x24b21f:'ff90b0000000',0x24b23c:'ff5010',0x24b25e:'ff5010'})
        fixture=r'''
#include "ptm_velocity_query.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
extern ABI void query_original(void*,void*);
extern ABI void nhl27_ptm_velocity_math(float*,const float*,const unsigned char*,float);
A guard_value=0;A *fixture_guard=&guard_value;int _fltused=0;
static unsigned skip_query,lookups;
extern void nhl27_ptm_velocity_output(float*,const float*,const unsigned char*,const float*,int);
static const float*transforms;static const unsigned char*flags;static float timestep;
ABI void fixture_fail(void){__builtin_trap();}
ABI unsigned char override_query(void*a,void*b,unsigned c){(void)a;(void)b;(void)c;return skip_query&1;}
ABI unsigned lookup(void*a,void*b,void*c){(void)a;(void)b;(void)c;return (skip_query&(2u<<lookups++))?-1:1;}
ABI int mode_query(void*p){(void)p;return !(skip_query&64);}
static void copy(float*d,const float*s){for(unsigned i=0;i<12;++i)d[i]=s[i];}
ABI unsigned char previous_query(void*a,void*b,int c,unsigned char d,void*e,int f,
 unsigned char g,float*dt,float*joint,float*reference,float*root,unsigned char*jroot,unsigned char*rroot){
 (void)a;(void)b;(void)c;(void)e;(void)f;
 if(d!=((skip_query>>8)-1u<2u)||g!=(((skip_query>>8)&~2u)==1u))__builtin_trap();
 *dt=timestep;copy(joint,transforms);copy(reference,transforms+12);copy(root,transforms+24);
 *jroot=0;*rroot=flags[0];return !(skip_query&8);
}
ABI unsigned char current_query(void*a,void*b,int c,unsigned char d,void*e,int f,
 unsigned char g,float*joint,float*reference,float*root,unsigned char*jroot,unsigned char*rroot){
 (void)a;(void)b;(void)c;(void)e;(void)f;
 if(d!=((skip_query>>8)-1u<2u)||g!=(((skip_query>>8)&~2u)==1u))__builtin_trap();
 copy(joint,transforms+36);copy(reference,transforms+48);copy(root,transforms+60);
 *jroot=flags[1];*rroot=flags[2];return !(skip_query&16);
}
__declspec(dllexport) void run(unsigned which,const float*t,const unsigned char*f,float dt,float*out){
 skip_query=lookups=0;
 if(which){nhl27_ptm_velocity_math(out,t,f,dt);return;}
 A schema[20]={0},context[20]={0},vtable[3]={0,0,(A)lookup},mapping=(A)vtable;
 unsigned char stripped[3]={0,0,0};
 schema[9]=(A)stripped;schema[10]=1;schema[12]=2;
 context[0]=(A)out;context[15]=(A)&mapping;
 transforms=t;flags=f;timestep=dt;
 query_original(schema,context);
}
static int override_adapter(void*c,struct ptm_velocity_schema*s){return override_query(s,c,0);}
static int mode_adapter(void*c,struct ptm_velocity_schema*s){(void)c;return mode_query(s);}
static int lookup_adapter(void*c,void*s,void*j){return (int)lookup(c,s,j);}
static int previous_adapter(void*c,void*s,int j,panic_u8 jf,void*r,int ri,panic_u8 rf,float*out,float*dt,panic_u8*f){
 return previous_query(c,s,j,jf,r,ri,rf,dt,out,out+12,out+24,f,f+1);
}
static int current_adapter(void*c,void*s,int j,panic_u8 jf,void*r,int ri,panic_u8 rf,float*out,panic_u8*f){
 return current_query(c,s,j,jf,r,ri,rf,out,out+12,out+24,f,f+1);
}
__declspec(dllexport) void run_output(unsigned which,const float*t,const unsigned char*f,float dt,
 const unsigned char*mask,const float*transform,unsigned skip,float*out){
 transforms=t;flags=f;timestep=dt;skip_query=skip;lookups=0;
 if(which){
  struct ptm_velocity_schema schema={0,(void*)1,0,(void*)(A)((skip&128)?1:2),skip>>8,{mask[0],mask[1],mask[2]}};
  struct ptm_velocity_query q={0,out,transform,(panic_u8)((skip>>5)&1),override_adapter,mode_adapter,
      lookup_adapter,previous_adapter,current_adapter};
  nhl27_ptm_velocity_query(&schema,&q);return;
 }
 A schema[20]={0},context[20]={0},vtable[3]={0,0,(A)lookup},mapping=(A)vtable,svt[23]={0};
 schema[18]=skip>>8;svt[22]=(A)mode_query;schema[0]=(A)svt;schema[9]=(A)mask;schema[10]=1;schema[12]=(skip&128)?1:2;
 context[0]=(A)out;context[11]=(A)transform;context[15]=(A)&mapping;((unsigned char*)context)[0x49]=(skip>>5)&1;
 query_original(schema,context);
}

'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-velocity-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'oracle.s').write_text(code)
            r=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.c'),str(p/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_velocity_output.c'),str(root/'scripts/nhl27/runtime/ptm_velocity_query.c'),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_float),C.POINTER(C.c_ubyte),C.c_float,C.POINTER(C.c_float)]
                rng=random.Random(0x24b1d0)
                edges=[0.,-0.,float('nan'),float('inf'),-float('inf'),1.,-1.]
                for n in range(8192):
                    vals=[rng.uniform(-4,4) for _ in range(72)]
                    if n%3==0:vals[n%72]=edges[n%len(edges)]
                    t=(C.c_float*72)(*vals)
                    # Noncanonical flag bytes matter: the source mixes ==1 and !=0.
                    f=(C.c_ubyte*3)(n%3,(n//3)%3,(n//9)%3)
                    dt=edges[n%7] if n%5==0 else rng.uniform(.01,4)
                    a=(C.c_float*4)();b=(C.c_float*4)()
                    lib.run(0,t,f,dt,a);lib.run(1,t,f,dt,b)
                    self.assertEqual(bytes(a)[:12],bytes(b)[:12],f'case {n}: flags={list(f)} dt={dt}')
                lib.run_output.argtypes=[C.c_uint,C.POINTER(C.c_float),C.POINTER(C.c_ubyte),C.c_float,
                    C.POINTER(C.c_ubyte),C.POINTER(C.c_float),C.c_uint,C.POINTER(C.c_float)]
                for n in range(8192):
                    t=(C.c_float*72)(*(rng.uniform(-4,4) for _ in range(72)))
                    f=(C.c_ubyte*3)(n%3,(n//3)%3,(n//9)%3)
                    mask=(C.c_ubyte*3)(n&1,(n>>1)&1,(n>>2)&1)
                    tf=(C.c_float*8)(*(rng.uniform(-2,2) for _ in range(8)))
                    if n%3==0:tf[n%8]=edges[n%7]
                    transform=tf if n%5 else None
                    a=(C.c_float*4)(1.25,-2.5,3.75,99);b=(C.c_float*4)(1.25,-2.5,3.75,99)
                    lib.run_output(0,t,f,0.5,mask,transform,(n>>3)&1023,a)
                    lib.run_output(1,t,f,0.5,mask,transform,(n>>3)&1023,b)
                    self.assertEqual(bytes(a),bytes(b),f'output case {n}')
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
