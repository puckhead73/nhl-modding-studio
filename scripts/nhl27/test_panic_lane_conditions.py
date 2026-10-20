"""Check lane eligibility and query ordering against the source instructions."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from oracle_fixture import assembly
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
import panic_geometry


class LaneConditionsTests(unittest.TestCase):
    def test_source_conditions_including_tick_wrap_and_noncanonical_flags(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private image and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x4a51d20,0x4a51e1a,
            {0x452cf70:'history_query',0x46f1830:'zone_query',0x4542cc0:'shot_query'},
            {0xec2d9f0:'canary',0x104331f0:'puck',0x103e1418:'ticks',0xfacf5b8:'setting'})
        code+='\n.intel_syntax noprefix\nadd rsp,0x58\npop rbx\npop r12\npop r13\npop r14\npop r15\npop rbp\nret\n'
        code+=panic_geometry.geometry(image)[0]
        code+=assembly(image,'original_pass_zone',0x46f1830,0x46f18c4,{},
                       {0xef1f9d0:'blue_line',0xef1f9c8:'goal_line'})
        code+=assembly(image,'original_shot_zone',0x46f1430,0x46f14ff,{},
                       {0xef1f9d0:'blue_line',0xef1f9c8:'goal_line'})
        fixture=r'''
#include "panic_lanes.h"
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
static byte actor[0xaa0],flags[0x6e8],history[0xb50],opponents[0x70];
static byte puck_actor[0x18],pose[0x40],parameter[0x38];
byte ticks[0x2d0],setting[0x50];void *puck=puck_actor;
static unsigned long long cookie=123;void *canary=&cookie;
static void *flag_pointer=flags;
static unsigned trial,serial,trace[8];
float blue_line,goal_line;
extern ABI unsigned char original_pass_zone(void*,float,float,float);
extern ABI unsigned char original_shot_zone(void*,void*,float,float);
extern unsigned char nhl27_panic_pass_zone(float,float,float);
extern unsigned char nhl27_panic_shot_zone(float,float,float);
__declspec(dllexport) unsigned zones(unsigned source,unsigned direction,unsigned z,unsigned blue){
 float d=*(float*)&direction,p=*(float*)&z;blue_line=*(float*)&blue;
 if(!source)return nhl27_panic_pass_zone(d,p,blue_line)|
                      ((unsigned)nhl27_panic_shot_zone(d,p,blue_line)<<8);
 *(float*)(actor+0xa9c)=d;
 return original_pass_zone(0,d,p,0.0f)|
       ((unsigned)original_shot_zone(actor,0,p,0.0f)<<8);
}
extern ABI void original(void*);
ABI void *history_query(void*a){if(a!=actor)__builtin_trap();return history;}
static unsigned char zone(float direction,float z){
 trace[serial++]=*(unsigned*)&direction;trace[serial++]=*(unsigned*)&z;
 return (byte)(trial>>(serial==2?0:8));
}
ABI unsigned char zone_query(void*out,float direction,float z,float margin){
 if(out || margin!=0)__builtin_trap();return zone(direction,z);
}
static unsigned char shot(void){trace[serial++]=0xabcdef;return (byte)(trial>>16);}
ABI unsigned char shot_query(void*a,float margin){
 if(a!=actor || margin!=0)__builtin_trap();return shot();
}
static unsigned char zone_adapter(void*c,float d,float z){(void)c;return zone(d,z);}
static unsigned char shot_adapter(void*c){(void)c;return shot();}
__declspec(dllexport) void run(unsigned source,unsigned seed,unsigned*out){
 trial=seed*1664525u+1013904223u;serial=0;
 for(unsigned n=0;n<8;n++)trace[n]=0;
 *(void**)(actor+0x60)=&flag_pointer;*(void**)(actor+0xa90)=opponents;
 *(void**)(puck_actor+0x10)=pose;*(void**)(setting+0x48)=parameter;
 int tick=(int)(trial^0x98765432u),pass=(int)(trial*3u),sh=(int)(trial*5u);
 int window=(int)(trial*7u);
 /* Explicit exact boundary and wrap cases mixed with broad signed values. */
 if(seed%4==0){tick=(int)0x80000000u;pass=0x7fffffff;sh=tick;window=seed%3;}
 *(int*)(ticks+0x2cc)=tick;*(int*)(parameter+0x34)=window;
 *(int*)(history+0xb38)=pass;*(int*)(history+0x430)=sh;
 *(unsigned*)(opponents+0x6c)=trial;*(unsigned*)(pose+0x38)=trial*9u;
 *(unsigned*)(history+0xb48)=trial*11u;
 if(source){void *state=actor;original(&state);out[0]=flags[0x6e4];}
 else{
  struct panic_lane_history h;h.pass_tick=pass;h.shot_tick=sh;
  h.receiver_position[2]=*(float*)(history+0xb48);
  struct panic_lane_condition_services s={0,zone_adapter,shot_adapter};
  struct panic_lane_conditions c=nhl27_panic_lane_conditions(&h,tick,window,
       *(float*)(opponents+0x6c),*(float*)(pose+0x38),&s);
  out[0]=c.combined;
 }
 out[1]=serial;for(unsigned n=0;n<8;n++)out[n+2]=trace[n];
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='lane-conditions-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            path=Path(folder);(path/'fixture.s').write_text(code);(path/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared',
                '-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(path/'fixture.s'),str(path/'fixture.c'),str(repo/'scripts/nhl27/runtime/panic_lanes.c'),
                '-o',str(path/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(path/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint)]
                for seed in range(8192):
                    a=(C.c_uint*10)();b=(C.c_uint*10)()
                    lib.run(1,seed,a);lib.run(0,seed,b)
                    self.assertEqual(list(a),list(b),seed)
                lib.zones.argtypes=[C.c_uint]*4;lib.zones.restype=C.c_uint
                import random,struct
                values=[0,0x80000000,0x3f800000,0xbf800000,0x7f800000,
                        0xff800000,0x7fc12345,0xffc12345,1,0x80000001]
                bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
                values += [bits(f) for f in (-30.,-8.,-7.4676,7.4676,8.,30.)]
                for d in values:
                    for z in values:
                        for blue in (bits(7.62),bits(-7.62),0,0x7fc12345):
                            self.assertEqual(lib.zones(0,d,z,blue),lib.zones(1,d,z,blue),(d,z,blue))
                rng=random.Random(27124)
                for _ in range(8192):
                    args=[rng.getrandbits(32) for n in range(3)]
                    self.assertEqual(lib.zones(0,*args),lib.zones(1,*args),args)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
