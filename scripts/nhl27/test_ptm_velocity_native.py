"""Run the native query-context adapter with qualified ABI service fixtures."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from ptm_trajectory_math import generate


class NativeVelocityTests(unittest.TestCase):
    def test_native_context_pose_history_and_future_bindings(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and LLVM required')
        math=generate(checked_image(ElfImage,source,SOURCE_HASH))[0]
        shim='''.intel_syntax noprefix
.text
.globl fixture_base
fixture_base:
.org 0x2c8410
jmp lookup
.org 0x330f80
jmp cache_get
'''
        fixture=r'''
#include "ptm_velocity_query.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
int _fltused=0;extern void fixture_base(void);
extern ABI int nhl24_ptm_velocity_query_guest(A,void*,void*,void*,void*);
extern ABI void nhl27_ptm_velocity_math(float*,const float*,const panic_u8*,float);
static unsigned char schema[0x70],query[0xb710],rig[0x20],rigrecord[0x8a0],metadata[0xa0];
static unsigned char maps[3][0x20],records[3][16],owner[16],history[0x20],entry[0x4600],map[0x28],nodes[3][0x48];
static A schema_vt[14],history_vt[3],property_vt[16],property,properties[2],buckets[2],joint_buckets[2],history_value;
static int root_index,poses[3],third_cache;static unsigned error,mirror_value;
static void zero(void*p,unsigned n){for(unsigned i=0;i<n;i++)((unsigned char*)p)[i]=0;}
static void transform(float*out,int joint,int phase){
 for(unsigned i=0;i<12;i++)out[i]=0;
 out[0]=out[1]=out[2]=out[3]=out[7]=1;
 if(phase>=0){out[8]=(float)(joint*(phase+1)+phase*2);out[9]=(float)(joint*joint+phase);}
}
ABI int lookup(void*r,void*s,void*j){if(r!=rig||s)error|=1;return (int)(A)j;}
ABI void *cache_get(void*m,void*s){if(s||(m!=query+0x78&&m!=query+0x5bc0))error|=2;return m;}
ABI int mode(void*p){if(p!=schema)error|=4;return *(int*)(schema+0x60);}
ABI void *get_history(void*p,void*c){if(p!=&property||c!=owner)error|=8;return &history_value;}
ABI void *cast_history(void*p,unsigned type){if(p!=history||type!=0xacfb5852)error|=16;return p;}
ABI void nhl24_ptm_feature_transform_guest(A base,float*out,void*cache,void*pose,int joint,panic_u8 mirror){
 if(base!=(A)fixture_base||cache!=&third_cache||pose!=poses+2||mirror!=mirror_value)error|=32;
 transform(out,joint,2);
}
ABI int nhl24_ptm_feature_triplet_guest(A base,void*a,void*b,void*ac,void*bc,int j,int r,panic_u8 mirror,float*out,panic_u8*f){
 int ap=a==poses?0:1,bp=b==poses?0:1;
 if(base!=(A)fixture_base||(a!=poses&&a!=poses+1)||(b!=poses&&b!=poses+1)||
    ac!=query+(ap?0x5bc0:0x78)||bc!=query+(bp?0x5bc0:0x78)||mirror!=mirror_value)error|=64;
 transform(out,j,ap);transform(out+12,r,bp);transform(out+24,0,bp);f[0]=f[1]=0;return 1;
}
__declspec(dllexport) unsigned run(unsigned pose_time,unsigned mirrored,unsigned no_history,unsigned no_third,unsigned missing){
 zero(schema,sizeof(schema));zero(query,sizeof(query));zero(rig,sizeof(rig));zero(rigrecord,sizeof(rigrecord));
 zero(metadata,sizeof(metadata));zero(maps,sizeof(maps));zero(records,sizeof(records));zero(owner,sizeof(owner));
 zero(history,sizeof(history));zero(entry,sizeof(entry));zero(map,sizeof(map));zero(nodes,sizeof(nodes));
 error=0;mirror_value=mirrored;
 schema_vt[13]=(A)mode;*(A*)schema=(A)schema_vt;*(A*)(schema+0x10)=1;*(A*)(schema+0x38)=2;*(unsigned*)(schema+0x60)=pose_time;
 for(unsigned i=0;i<3;i++){
  *(A*)(maps[i]+0x10)=(A)records[i];*(A*)(maps[i]+0x18)=(A)(records[i]+16);
  *(A*)(records[i]+8)=i==2?(A)owner:(A)(poses+i);
 }
 if(missing&1)*(int*)maps[0]=-1;if(missing&2)*(int*)maps[1]=-1;
 *(A*)(query+0x60)=(A)maps[0];*(A*)(query+0x68)=(A)maps[1];*(A*)(query+0x30)=(A)maps[2];
 *(A*)(query+0x70)=(A)rig;*(A*)(rig+0x10)=(A)rigrecord;*(A*)(rig+0x18)=(A)(rigrecord+0x8a0);
 *(A*)(rigrecord+0x890)=(A)metadata;*(A*)(metadata+0x98)=(A)&root_index;
 property_vt[15]=(A)get_history;property=(A)property_vt;properties[1]=(A)&property;*(A*)(query+0x40)=(A)properties;
 history_vt[2]=(A)cast_history;*(A*)history=(A)history_vt;history_value=no_history?0:(A)history;
 buckets[0]=(A)entry;buckets[1]=0;*(A*)(history+0x10)=(A)buckets;*(unsigned*)(history+0x18)=1;
 *(A*)entry=(A)owner;*(A*)(entry+8)=(A)map;*(unsigned*)(entry+0x38)=1;*(float*)(entry+0x45d8)=.5f;
 joint_buckets[0]=(A)nodes;joint_buckets[1]=0;*(A*)(map+0x18)=(A)joint_buckets;*(unsigned*)(map+0x20)=1;
 for(unsigned i=0;i<3;i++){*(int*)nodes[i]=(int)i;transform((float*)(nodes[i]+0x10),(int)i,3);*(A*)(nodes[i]+0x40)=i<2?(A)nodes[i+1]:0;}
 float actual[4]={99,99,99,99},expected[4]={0,0,0,99},transforms[72];
 *(A*)(query+0x50)=(A)actual;query[0x38]=(panic_u8)mirrored;
 int handled=nhl24_ptm_velocity_query_guest((A)fixture_base,schema,query,no_third?0:poses+2,&third_cache);
 unsigned jf=pose_time-1u<2u,rf=(pose_time&~2u)==1;
 int absent=((missing&(jf?2:1))!=0)||((missing&(rf?2:1))!=0);
 if(!absent){
  int jp=jf?(no_third?-1:2):(no_history?-1:3),rp=rf?(no_third?-1:2):(no_history?-1:3);
  transform(transforms,1,jp);transform(transforms+12,2,rp);transform(transforms+24,0,rp);
  transform(transforms+36,1,jf?1:0);transform(transforms+48,2,rf?1:0);transform(transforms+60,0,rf?1:0);
  panic_u8 flags[3]={0,0,0};nhl27_ptm_velocity_math(expected,transforms,flags,(rf||no_history)?2:.5f);expected[3]=99;
 }
 if(handled!=1)error|=128;
 for(unsigned i=0;i<4;i++)if(((unsigned*)actual)[i]!=((unsigned*)expected)[i])error|=256;
 *(A*)(schema+0x18)=1;
 if(nhl24_ptm_velocity_query_guest((A)fixture_base,schema,query,0,&third_cache)!=0)error|=512;
 return error;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-native-velocity-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'math.s').write_text(math);(p/'native.s').write_text(shim)
            sources=[root/'scripts/nhl27/runtime'/f'{name}.c' for name in
                ('ptm_velocity_native','ptm_velocity_query','ptm_velocity_output','ptm_previous_pose','ptm_history_pose')]
            r=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),str(p/'fixture.c'),str(p/'math.s'),
                str(p/'native.s'),*map(str,sources),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint]*5
                for pt in range(4):
                    for mirror in range(3):
                        for history in range(2):
                            for third in range(2):
                                for missing in range(4):self.assertEqual(lib.run(pt,mirror,history,third,missing),0,
                                    (pt,mirror,history,third,missing))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
