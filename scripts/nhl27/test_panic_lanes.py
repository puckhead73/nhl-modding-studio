"""Execute original lane-history writer and compare updates and service order."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from oracle_fixture import assembly
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
import panic_geometry


class LaneTests(unittest.TestCase):
    def test_source_history_writer(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private image and LLVM required')
        src=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(src,'original',0x49b6020,0x49b6800,
            {0x46f0960:'project',0x46f2ce0:'distance',0xbfabea0:'fail'},
            {0xec2d9f0:'canary',0x104331f0:'puck',0x1048eec8:'prediction',
             0x1048ee28:'shot',0x1048ee78:'passing',0x103e1418:'ticks'})
        code += panic_geometry.geometry(src)[0]
        fixture=r'''
#include "panic_lanes.h"
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
void *memset(void*p,int v,unsigned long long n){volatile byte *b=p;for(unsigned long long i=0;i<n;i++)b[i]=(byte)v;return p;}
static byte history[0xac300],actor[0x18],pose[0x40],pred[0x38],sh[0x38],pa[0x38];
static byte native_history[0x70000];
static struct panic_lane_history extra[2][7];
byte ticks[0x2d0];void *puck=actor,*prediction=pred,*shot=sh,*passing=pa;
static unsigned long long cookie=0x3456789;void *canary=&cookie;
extern ABI void original(void*);
static unsigned seed,serial,trace[512],count,geometry_mode;
extern ABI unsigned char nhl27_lane_project(const float*,const float*,const float*,float*);
extern ABI float nhl27_lane_distance(const float*,const float*,const float*,const float*,float*,float*);
static unsigned next(void){seed=seed*1664525u+1013904223u;return seed;}
static float number(void){return (float)(int)(next()%400)-200.0f;}
ABI void fail(void){__builtin_trap();}
static unsigned id(const float *p){return (unsigned)((const byte*)p-history);}
ABI void project(const float *p,const float *target,const float *defender,float *out){
 if(p!=(float*)(pose+0x30))fail();
 trace[count++]=id(target);trace[count++]=id(defender);
 ++serial;
 if(geometry_mode){nhl27_lane_project(p,target,defender,out);return;}
 for(unsigned k=0;k<4;k++)out[k]=defender[k]+(float)((serial+k)%3);
}
ABI float distance(const float *p,const float *target,const float *defender,
                   const float *predicted,float *one,float *two){
 if(p!=(float*)(pose+0x30))fail();
 trace[count++]=id(target)^0x80000000u;trace[count++]=id(defender);
 /* Verify the prediction selected by the writer, including its fourth lane. */
 for(unsigned k=0;k<4;k++){union{float f;unsigned u;}v={predicted[k]};trace[count++]=v.u;}
 if(geometry_mode)return nhl27_lane_distance(p,target,defender,predicted,one,two);
 one[0]=(float)serial;two[1]=(float)serial;
 return (float)(serial%7)-1.0f;
}
static void project_bridge(void*c,const float*p,const float*t,const float*d,float*out){
 (void)c;project(p,t,d,out);
}
static float distance_bridge(void*c,const float*p,const float*t,const float*d,
 const float*pr,float*a,float*b){(void)c;return distance(p,t,d,pr,a,b);}
__declspec(dllexport) void run(unsigned source,unsigned trial,unsigned *out){
 seed=trial+1;serial=count=0;geometry_mode=source>>1;
 for(unsigned i=0;i<sizeof(history);i++)history[i]=0;
 unsigned bank=trial%2,team=(trial/2)%2,role=(trial/4)%9;
 int possession=(trial%17==0)?-1:(int)team;
 *(int*)(history+0x10)=possession;*(unsigned*)(history+0x18)=role;
 *(unsigned*)(history+0x1bf50)=bank;
 *(void**)(actor+0x10)=pose;
 unsigned horizon=trial%4;*(unsigned*)(pred+0x34)=horizon;
 *(float*)(sh+0x34)=(float)(trial%6);*(float*)(pa+0x34)=(float)((trial/6)%6);
 int tick=(int)(trial*9876543u);*(int*)(ticks+0x2cc)=tick;
 struct panic_lane_frame frame={0};
 frame.possession_team=possession;frame.possession_role=role;frame.tick=tick;
 frame.prediction_ticks=horizon;frame.shot_radius=*(float*)(sh+0x34);
 frame.pass_radius=*(float*)(pa+0x34);frame.puck_position=(float*)(pose+0x30);
 for(unsigned t=0;t<2;t++){
  byte *base=history+0x1bf60+bank*0x48080+t*0x24040;
  frame.team_target[t]=(float*)(base+0x10);
  for(unsigned r=0;r<7;r++){
   byte *record=base+0x80+r*0x2920;
   *(unsigned short*)(record+0x3b4)=(trial+r)%2?8:0;
   record[0x2918]=(next()%7<5)?1:(next()%2?2:0);
   *(float*)(record+0x364)=number()/100.0f;
   *(int*)(record+0x430)=source>=5?0:123;*(int*)(record+0xb38)=source>=5?0:456;
   for(unsigned k=0;k<4;k++){
    ((float*)record)[k]=number();float previous=number();((float*)(record+0xb40))[k]=source>=5?0:previous;
    ((float*)(record+0x1700+horizon*16))[k]=number();
    ((float*)(record+0x1ac0+horizon*16))[k]=number();
   }
   struct panic_lane_record *v=&frame.records[t][r];
   v->position=(float*)record;v->active=record[0x2918];v->speed=*(float*)(record+0x364);
   v->predicted_position=(float*)(record+((*(unsigned short*)(record+0x3b4)&8)?0x1700:0x1ac0)+horizon*16);
   v->shot_tick=(int*)(record+0x430);v->pass_tick=(int*)(record+0xb38);
   v->receiver_position=(float*)(record+0xb40);
  }
 }
 if(source==6){
  *(unsigned*)(native_history+0x1bfb0)=bank;
  *(int*)(native_history+0x7c)=possession;*(unsigned*)(native_history+0x88)=role;
  for(unsigned t=0;t<2;t++)for(unsigned r=0;r<7;r++){
   struct panic_lane_record *v=&frame.records[t][r];
   byte *nr=native_history+0x1c030+bank*0x222e0+t*0x11170+r*0x2700;
   unsigned flags=((trial+r)%2?8:0)|(v->active==1);
   *(unsigned short*)(nr+0x31c)=(unsigned short)flags;*(float*)(nr+0x2d4)=v->speed;
   for(unsigned k=0;k<4;k++){
    ((float*)nr)[k]=v->position[k];
    ((float*)(nr+((flags&8)?0x1520:0x18e0)+horizon*16))[k]=v->predicted_position[k];
   }
   /* Targets are zero in this fixture, as in the source buffer initialization. */
   for(unsigned k=0;k<4;k++)((float*)(native_history+0x1bfd0+bank*0x222e0+t*0x11170))[k]=0;
  }
  struct panic_lane_settings settings={horizon,frame.shot_radius,frame.pass_radius};
  if(!nhl24_panic_lane_frame(native_history,(float*)(pose+0x30),tick,&settings,extra,&frame))fail();
  nhl27_panic_update_lanes_geometry(&frame);
 }
 else if(source&1)original(history);
 else if(source==4)nhl27_panic_update_lanes_geometry(&frame);
 else {struct panic_lane_services s={0,project_bridge,distance_bridge};nhl27_panic_update_lanes(&frame,&s);}
 unsigned n=0;
 for(unsigned t=0;t<2;t++)for(unsigned r=0;r<7;r++){
  struct panic_lane_record *v=&frame.records[t][r];
  out[n++]=(unsigned)*v->shot_tick;out[n++]=(unsigned)*v->pass_tick;
  for(unsigned k=0;k<4;k++){union{float f;unsigned u;}v1={v->receiver_position[k]};out[n++]=v1.u;}
 }
 out[n++]=count;
 for(unsigned i=0;i<count;i++)out[n++]=trace[i];
 for(;n<512;n++)out[n]=0;
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-lanes-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding',
                '-fno-builtin','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',
                '-I',str(repo/'scripts/nhl27/runtime'),str(out/'oracle.s'),str(out/'fixture.c'),
                str(repo/'scripts/nhl27/runtime/panic_lanes.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint)]
                for trial in range(4096):
                    a=(C.c_uint*512)();b=(C.c_uint*512)()
                    lib.run(1,trial,a);lib.run(0,trial,b)
                    self.assertEqual(list(a),list(b),f'trial {trial}')
                for trial in range(4096):
                    a=(C.c_uint*512)();b=(C.c_uint*512)();c=(C.c_uint*512)()
                    lib.run(3,trial,a);lib.run(2,trial,b);lib.run(4,trial,c)
                    self.assertEqual(list(a),list(b),f'geometry trial {trial}')
                    self.assertEqual(list(a)[:84],list(c)[:84],f'production geometry trial {trial}')
                for trial in range(4096):
                    a=(C.c_uint*512)();b=(C.c_uint*512)()
                    lib.run(5,trial,a);lib.run(6,trial,b)
                    self.assertEqual(list(a)[:84],list(b)[:84],f'native records trial {trial}')
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
