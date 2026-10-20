"""Original source prediction loops versus relocated loops and native inputs."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import struct
import unittest
from oracle_fixture import assembly
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
import panic_prediction
import panic_geometry


class PredictionTests(unittest.TestCase):
    def test_native_input_adapter_and_source_arrays(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code,receipt=panic_prediction.generate(image)
        code+=panic_geometry.geometry(image)[0]
        code+=assembly(image,'original',panic_prediction.START,panic_prediction.END,
            {0x463da0:'nhl27_panic_prediction_step',0x25da630:'nhl27_panic_prediction_reduce',
             panic_prediction.END:'original_finish'},
            {0xec2d9f0:'canary',0x1048f010:'first',0x1048ef70:'second',0x1048efc0:'third'})
        code+='''.intel_syntax noprefix
.text
original_finish:
add rsp,0x308
pop rbx
pop r12
pop r13
pop r14
pop r15
pop rbp
ret
'''
        fixture=r'''
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
static unsigned long long cookie=123;void *canary=&cookie;
struct parameter{byte padding[0x34];float value;};
static struct parameter a,b,c;void *first=&a,*second=&b,*third=&c;
extern ABI void original(void*),nhl27_panic_prediction_arrays(void*,const void*const*);
extern int nhl24_panic_predict_position(const void*,const float*,unsigned,float*);
extern ABI void nhl24_panic_lanes_guest(__UINTPTR_TYPE__,void*);
extern ABI void nhl24_panic_lanes_reset_guest(__UINTPTR_TYPE__,void*);
extern ABI void *nhl24_panic_lane_history_guest(__UINTPTR_TYPE__,void*);
extern ABI int nhl24_panic_lane_conditions_guest(__UINTPTR_TYPE__,void*,void*);
__declspec(dllexport) void update(void*base,void*history){nhl24_panic_lanes_guest((__UINTPTR_TYPE__)base,history);}
__declspec(dllexport) void reset(void*history){nhl24_panic_lanes_reset_guest(0,history);}
__declspec(dllexport) void *lookup(void*base,void*actor){return nhl24_panic_lane_history_guest((__UINTPTR_TYPE__)base,actor);}
__declspec(dllexport) int conditions(void*base,void*actor,void*out){return nhl24_panic_lane_conditions_guest((__UINTPTR_TYPE__)base,actor,out);}
static byte native[0x2700],record[0x1e80];
static unsigned seed;
static unsigned next(void){seed=seed*1664525u+1013904223u;return seed;}
static float number(void){return ((float)(int)(next()%2001)-1000.0f)/100.0f;}
__declspec(dllexport) int run(unsigned mode,unsigned trial,unsigned *out){
 seed=trial+1;
 for(unsigned n=0;n<sizeof(record);n++)record[n]=0;
 for(unsigned n=0;n<sizeof(native);n++)native[n]=0;
 for(unsigned n=0;n<0x60;n+=4)*(float*)(record+n)=number();
 for(unsigned n=0;n<0x108;n+=4)*(float*)(record+0x160+n)=number();
 *(int*)(record+0x260)=trial%2;*(int*)(record+0x264)=(trial/2)%2;
 *(int*)(record+0x25c)=(trial/4)%4;
 *(float*)(record+0x344)=number();*(int*)(record+0x348)=(trial/8)%9;
 *(float*)(record+0x34c)=number();
 /* Exercise both the polynomial and large-angle reduction paths. */
 if(trial%5==0)*(float*)(record+0x34c)=1000000.0f+number();
 a.value=number()/10.0f;b.value=number()/10.0f;c.value=number()/10.0f;
 const void *parameters[3]={&a,&b,&c};
 if(mode==1)original(record);
 else if(mode==0)nhl27_panic_prediction_arrays(record,parameters);
 else{
  for(unsigned n=0;n<0x60;n++)native[n]=record[n];
  for(unsigned n=0;n<0x108;n++)native[0x160+n]=record[0x160+n];
  for(unsigned n=0;n<12;n++)native[0x2b4+n]=record[0x344+n];
  *(unsigned short*)(native+0x31c)=trial%2?8:0;
  float settings[3]={a.value,b.value,c.value};
  return nhl24_panic_predict_position(native,settings,trial%60,(float*)out);
 }
 for(unsigned n=0;n<0x780;n+=4)out[n/4]=*(unsigned*)(record+0x1700+n);
 return 1;
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-prediction-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',
                '-I',str(repo/'scripts/nhl27/runtime'),str(out/'oracle.s'),str(out/'fixture.c'),
                str(repo/'scripts/nhl27/runtime/panic_prediction_native.c'),
                str(repo/'scripts/nhl27/runtime/panic_lanes.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint)]
                for trial in range(2048):
                    a=(C.c_uint*480)();b=(C.c_uint*480)();c=(C.c_uint*4)()
                    self.assertEqual(lib.run(1,trial,a),1)
                    self.assertEqual(lib.run(0,trial,b),1)
                    self.assertEqual(list(a),list(b),f'arrays {trial}')
                    self.assertEqual(lib.run(2,trial,c),1)
                    offset=(0 if trial%2 else 240)+(trial%60)*4
                    self.assertEqual(list(c),list(a)[offset:offset+4],f'native {trial}')
                # Exercise the actual guest entry points through both history
                # banks, owner replacement, disabled analysis and full reset.
                base=C.create_string_buffer(0xbb00000)
                history=C.create_string_buffer(0x60600)
                replacement=C.create_string_buffer(0x60600)
                manager=C.create_string_buffer(0x3d8);analysis=C.create_string_buffer(0x50)
                actor=C.create_string_buffer(0x938);role=C.create_string_buffer(0x48)
                puck=C.create_string_buffer(0x18);pose=C.create_string_buffer(0x40)
                actor_pose=C.create_string_buffer(0x40);opponent=C.create_string_buffer(0xf0)
                def pointer(buffer,offset,target):struct.pack_into('<Q',buffer,offset,C.addressof(target))
                pointer(base,0xbadc558,manager);pointer(manager,0x3d0,analysis)
                pointer(analysis,0x48,history);pointer(actor,0x100,role)
                pointer(base,0xbae0ea8,puck);pointer(puck,0x10,pose)
                pointer(actor,0x10,actor_pose);pointer(actor,0x920,opponent)
                struct.pack_into('<I',role,0x40,1)
                struct.pack_into('<i',base,0xba45a04,1234)
                lib.lookup.argtypes=[C.c_void_p,C.c_void_p];lib.lookup.restype=C.c_void_p
                lib.update.argtypes=[C.c_void_p,C.c_void_p];lib.reset.argtypes=[C.c_void_p]
                lib.conditions.argtypes=[C.c_void_p]*3;lib.conditions.restype=C.c_int
                enabled=C.c_ubyte.in_dll(lib,'nhl27_panic_lanes_enabled')
                self.assertIsNone(lib.lookup(base,actor))
                lib.update(base,history);enabled.value=1
                self.assertIsNone(lib.lookup(base,actor)) # disabled update did nothing
                for bank in (0,1,0):
                    struct.pack_into('<I',history,0x1bfb0,bank)
                    lib.update(base,history)
                    view=lib.lookup(base,actor);self.assertIsNotNone(view)
                    self.assertEqual(C.string_at(view,24),bytes(24))
                    # A later frame must clear prior results even with no actors.
                    C.memset(view,0x7f,24);lib.update(base,history)
                    self.assertEqual(C.string_at(lib.lookup(base,actor),24),bytes(24))
                    history[8]=1;self.assertIsNone(lib.lookup(base,actor));history[8]=0
                lib.reset(replacement);self.assertIsNotNone(lib.lookup(base,actor))
                lib.reset(history);self.assertIsNone(lib.lookup(base,actor))
                lib.update(base,history);lib.update(base,replacement)
                self.assertIsNone(lib.lookup(base,actor)) # old owner invalidated
                pointer(analysis,0x48,replacement);self.assertIsNotNone(lib.lookup(base,actor))
                view=lib.lookup(base,actor)
                for name in ('nhl27_panic_pass_zone','nhl27_panic_shot_zone'):
                    fn=getattr(lib,name);fn.argtypes=[C.c_float]*3;fn.restype=C.c_ubyte
                struct.pack_into('<f',base,0xaa5d4a8,7.62)
                for trial in range(128):
                    tick=1234;pass_tick=tick-trial%8;shot_tick=tick-(trial//8)%8
                    direction=-1.0 if trial&1 else 1.0
                    receiver_z=-10.0 if trial&2 else 10.0
                    puck_z=-10.0 if trial&4 else 10.0
                    actor_z=-10.0 if trial&8 else 10.0
                    struct.pack_into('<f',opponent,0xec,direction)
                    struct.pack_into('<f',actor,0x930,direction)
                    struct.pack_into('<f',pose,0x38,puck_z)
                    struct.pack_into('<f',actor_pose,0x38,actor_z)
                    C.memmove(view,struct.pack('<iiffff',shot_tick,pass_tick,0,0,receiver_z,0),24)
                    result=(C.c_ubyte*3)()
                    self.assertEqual(lib.conditions(base,actor,result),1)
                    p=(lib.nhl27_panic_pass_zone(direction,puck_z,7.62)^lib.nhl27_panic_pass_zone(direction,receiver_z,7.62)^1)&(tick-pass_tick<5)
                    s=lib.nhl27_panic_shot_zone(direction,actor_z,7.62) if tick-shot_tick<5 else 0
                    self.assertEqual(list(result),[p,s,p|s])
                struct.pack_into('<I',replacement,0x1bfb0,2)
                self.assertIsNone(lib.lookup(base,actor))
                result=(C.c_ubyte*3)(255,255,255)
                self.assertEqual(lib.conditions(base,actor,result),0)
                self.assertEqual(list(result),[0,0,0])
                enabled.value=0
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
