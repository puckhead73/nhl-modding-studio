"""Guest adapter ABI/offset checks and source/target virtual-foot evidence."""
import ctypes as C
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
import math
from oracle_fixture import assembly
from deke_runtime_evidence import checked_image,SOURCE_HASH,TARGET_HASH
from research import ElfImage,SelfImage


class PanicServicesTests(unittest.TestCase):
    def test_foot_definitions_and_service_adapter(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists() or not target.exists():
            self.skipTest('Private source/target and Windows LLVM required')
        src=checked_image(ElfImage,source,SOURCE_HASH);dst=checked_image(SelfImage,target,TARGET_HASH)
        for a,b in ((0xdee6310,0x8fcf3b0),(0xdee54c0,0x8fcf3c0),
                    (0xdeead00,0x8fcf3d0),(0xdeebb70,0x8fcf3e0)):
            self.assertEqual(src.read(a,16),dst.read(b,16))
        # Original 465c3f0: relocate only its two LEAs to fixture globals.
        raw=src.read(0x465c3f0,0x30)
        self.assertEqual(raw[:3],bytes.fromhex('488d05'))
        self.assertEqual(raw[14:17],bytes.fromhex('488d05'))
        asm='.text\n.globl original_session\noriginal_session:\n.byte '+','.join(map(str,raw[:3]))+'\n.long manager - . - 4\n'
        asm+='.byte '+','.join(map(str,raw[7:17]))+'\n.long puck - . - 4\n.byte '+','.join(map(str,raw[21:]))+'\n'
        asm+=assembly(src,'original_speed',0x452d430,0x452d480,{},{})
        asm+=assembly(src,'original_blocked',0x4533ae0,0x4533b03,{},{})
        asm+=assembly(dst,'native_blocked',0x2831140,0x2831184,{},
                      {0xbadc558:'actor_manager'})
        asm+=assembly(src,'source_reaction',0x4a19c80,0x4a19ca7,{},{})
        asm+=assembly(src,'source_animation_blocked',0x4a19aa0,0x4a19b25,{},{})
        asm+=assembly(dst,'native_animation_blocked',0x2effb90,0x2effbd2,{},{})
        asm+=assembly(dst,'native_animation_full',0x2f00040,0x2f0007f,
                      {0x2effb90:'native_animation_blocked'},{})
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
unsigned char manager[0x302],puck[0xf0];
typedef void (ABI *query)(unsigned,unsigned,float*,unsigned);
extern int nhl24_panic_foot_probe(void*,unsigned,query,float*);
extern ABI unsigned char original_session(void*);
extern ABI float original_speed(void*);
extern ABI unsigned char original_blocked(void*),nhl24_panic_component_blocked(void*);
extern ABI void native_blocked(void*,void*);
extern ABI unsigned char source_reaction(void*),source_animation_blocked(void*);
extern ABI unsigned char native_animation_blocked(void*);
extern ABI unsigned char native_animation_full(void*);
extern ABI unsigned char nhl24_panic_animation_reaction(void*),nhl24_panic_animation_blocked(void*);
__declspec(dllexport) unsigned animation_flags(unsigned source,unsigned state,unsigned kind,unsigned missing){
 static unsigned char animation[0xc58],node[0x68],active[0x20],descriptor[0x10];
 *(void**)(animation+0xc18)=node;*(void**)(animation+0xc50)=node;
 *(unsigned*)(node+0x20)=state;
 *(void**)(node+0x60)=missing==1?0:active;
 *(void**)(active+0x18)=missing==2?0:descriptor;
 *(void**)(descriptor+8)=&kind;
 if(source==2)return native_animation_blocked(node);
 if(source==3)return native_animation_full(node);
 if(source)return source_reaction(animation)|((unsigned)source_animation_blocked(animation)<<8);
 return nhl24_panic_animation_reaction(animation)|((unsigned)nhl24_panic_animation_blocked(animation)<<8);
}
static unsigned char actor_storage[0x1e0];
void *actor_manager=actor_storage;
__declspec(dllexport) unsigned blocked(unsigned source,void*actor){
 if(source==1)return original_blocked(actor);
 if(source==0)return nhl24_panic_component_blocked(actor);
 unsigned char result=0;unsigned short id=0;void *output[2]={0,&result};
 *(void**)(actor_storage+0x1d8)=actor;native_blocked(output,&id);return result;
}
__declspec(dllexport) float speed_oracle(void*actor){return original_speed(actor);}
static ABI void capture(unsigned actor,unsigned bone,float*out,unsigned handed){
    out[0]=(float)actor;out[1]=(float)bone;out[2]=(float)handed;out[3]=1;
}
__declspec(dllexport) int probe(void*actor,unsigned bone,float*out){return nhl24_panic_foot_probe(actor,bone,capture,out);}
__declspec(dllexport) unsigned char oracle(unsigned char disabled,int state){
    manager[0x301]=disabled;*(int*)(puck+0xec)=state;return original_session(0);
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-services-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'session.s').write_text(asm);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'session.s'),str(repo/'scripts/nhl27/runtime/panic_services.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.animation_flags.argtypes=[C.c_uint]*4;lib.animation_flags.restype=C.c_uint
                for state in (0,1,2,3,4,5,6,7,8,0xffffffff):
                    for kind in range(0x80):
                        for missing in range(3):
                            args=(state,kind,missing)
                            self.assertEqual(lib.animation_flags(0,*args),lib.animation_flags(1,*args),args)
                            self.assertEqual(lib.animation_flags(0,*args)>>8,lib.animation_flags(3,*args),args)
                            old=lib.animation_flags(2,*args)
                            self.assertEqual(old,int(state in (0,1,5) and kind in (0x21,0x23,0x25) and not missing))
                lib.blocked.argtypes=[C.c_uint,C.c_void_p];lib.blocked.restype=C.c_uint
                blocked_actor=C.create_string_buffer(0x230);owner=C.create_string_buffer(0xa0);component=C.create_string_buffer(0x18)
                for missing in range(3):
                    for at in (0x208,0x228):struct.pack_into('<Q',blocked_actor,at,C.addressof(owner) if missing!=1 else 0)
                    struct.pack_into('<Q',owner,0x98,C.addressof(component) if missing!=2 else 0)
                    for kind in (0,1,0x1f,0x20,0x21,0x2000,0x4000,0xffffffff):
                        struct.pack_into('<I',component,0x10,kind)
                        expected=lib.blocked(1,blocked_actor)
                        self.assertEqual(lib.blocked(0,blocked_actor),expected,(missing,kind))
                        self.assertEqual(lib.blocked(2,blocked_actor),expected,(missing,kind))
                for name in ('oracle','nhl24_panic_session_state'):
                    getattr(lib,name).argtypes=[C.c_ubyte,C.c_int];getattr(lib,name).restype=C.c_ubyte
                for disabled in (0,1,2,255):
                    for state in (-2147483648,-2,-1,0,1,2,3,4,2147483647):
                        self.assertEqual(lib.oracle(disabled,state),lib.nhl24_panic_session_state(disabled,state))
                lib.probe.argtypes=[C.c_void_p,C.c_uint,C.POINTER(C.c_float)]
                actor=C.create_string_buffer(0x180);pose=C.create_string_buffer(0xa0)
                struct.pack_into('<Q',actor,0x10,C.addressof(pose));struct.pack_into('<I',pose,0x94,17)
                struct.pack_into('<I',actor,0x170,7)
                before=bytes(actor),bytes(pose)
                for bone in range(0x25,0x29):
                    point=(C.c_float*4)();self.assertEqual(lib.probe(actor,bone,point),1)
                    self.assertEqual(list(point),[17,bone-5,7,1])
                self.assertEqual((bytes(actor),bytes(pose)),before)
                point=(C.c_float*4)(9,8,7,6)
                for bone in (0,0x24,0x29,0xffffffff):
                    self.assertEqual(lib.probe(actor,bone,point),0);self.assertEqual(list(point),[9,8,7,6])
                self.assertEqual(lib.probe(None,0x25,point),0)
                animation=C.create_string_buffer(0x68);parameters=C.create_string_buffer(0x38)
                for name in ('speed_oracle','nhl24_panic_normalized_speed'):
                    getattr(lib,name).argtypes=[C.c_void_p];getattr(lib,name).restype=C.c_float
                for missing in range(3):
                    struct.pack_into('<Q',actor,0x60,C.addressof(animation) if missing!=1 else 0)
                    struct.pack_into('<Q',animation,8,C.addressof(parameters) if missing!=2 else 0)
                    for scale in [0.,-0.,2**-16,-2**-16,2**-15,-1.,1.,math.nan,math.inf,-math.inf]:
                        for speed in [-0.,0.,1.,-1.,math.nan,math.inf]:
                            struct.pack_into('<f',parameters,0x30,scale);struct.pack_into('<f',pose,0x90,speed)
                            self.assertEqual(struct.pack('<f',lib.speed_oracle(actor)),
                                struct.pack('<f',lib.nhl24_panic_normalized_speed(actor)),(missing,scale,speed))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
