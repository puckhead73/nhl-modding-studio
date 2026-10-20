"""Compare native-layout actor eligibility with the source gate's call order."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class AllowedTests(unittest.TestCase):
    def test_native_state_registry_and_live_condition(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('LLVM required')
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
typedef __UINTPTR_TYPE__ addr;
static byte memory[0xba0ef00],actor[0x68],animation[0xc30],wrapper[0x28],evaluation[0x70];
static byte object[0x18],asset[0x40],condition[8];
static addr registry_methods[16],condition_methods[4];
static void *slot;
static unsigned result_value,calls;
extern ABI void *nhl24_panic_state_object_guest(addr,void*);
extern ABI int nhl24_panic_state_condition(void*,byte*);
ABI void **query(void*p,void*c){
 if(p!=memory+0xba0eee8 || c!=evaluation+0x60)__builtin_trap();
 calls=calls*16+1;return &slot;
}
ABI byte read_condition(void*p,void*c){
 if(p!=condition || c!=evaluation+0x60)__builtin_trap();
 calls=calls*16+2;return result_value;
}
__declspec(dllexport) void run(unsigned value,unsigned missing,unsigned*out){
 calls=0;result_value=value;
 *(void**)(actor+0x60)=missing==1?0:animation;
 *(void**)(animation+0xc28)=missing==2?0:wrapper;
 *(void**)(wrapper+0x20)=missing==3?0:evaluation;
 *(void**)(memory+0xba0eee8)=missing==4?0:registry_methods;
 registry_methods[15]=(addr)query;slot=missing==5?0:object;
 *(void**)(object+8)=missing==6?0:asset;
 *(void**)(object+0x10)=missing==7?0:evaluation;
 *(void**)(asset+0x38)=missing==8?0:condition;
 *(void**)condition=missing==9?0:condition_methods;
 condition_methods[3]=(addr)read_condition;
 void *state=nhl24_panic_state_object_guest((addr)memory,actor);
 byte v=0xa5;out[0]=nhl24_panic_state_condition(state,&v);out[1]=v;out[2]=calls;
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-state-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(repo/'scripts/nhl27/runtime/panic_services.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint)]
                for value in range(256):
                    for missing in range(10):
                        actual=(C.c_uint*3)();lib.run(value,missing,actual)
                        expected=[1,value,0x12] if missing==0 else [0,0xa5,0 if missing<5 else 1]
                        self.assertEqual(list(actual),expected,(value,missing))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)

    def test_source_gate_with_native_actor_fields(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        path=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not path.exists():self.skipTest('Private source and LLVM required')
        image=checked_image(ElfImage,path,SOURCE_HASH)
        code=assembly(image,'original',0x439bdc0,0x439be44,
            {0x4a19ee0:'special',0x452dc70:'state_object',0x3e4220:'state_condition',
             0x4533ae0:'original_component'},{})
        code+=assembly(image,'original_component',0x4533ae0,0x4533b03,{},{})
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
static byte actor[0x230],role[0x18],animation[0xb0],owner[0xa0],component[0x18];
static unsigned special_value,condition_value,trace,serial;
extern ABI byte original(void*,unsigned);
extern byte nhl24_panic_actor_allowed(void*,unsigned,const struct panic_allowed_services*);
ABI byte special(void *p){if(p!=animation)__builtin_trap();trace=trace*16+1;serial++;return special_value;}
ABI void *state_object(void*p){if(p!=actor)__builtin_trap();return role;}
ABI byte state_condition(void*p){if(p!=role)__builtin_trap();trace=trace*16+2;serial++;return condition_value;}
static byte special_adapter(void*c,void*p){(void)c;return special(p);}
static int condition_adapter(void*c,void*p,byte*out){(void)c;*out=state_condition(state_object(p));return 1;}
__declspec(dllexport) void run(unsigned source,unsigned trial,unsigned*out){
 trace=serial=0;
 *(void**)(actor+0x100)=role;*(void**)(actor+0x110)=role;*(void**)(actor+0x60)=animation;
 *(void**)(actor+0x208)=owner;*(void**)(actor+0x228)=owner;*(void**)(owner+0x98)=component;
 unsigned flag=trial&1,kind=(trial>>1)&3,status=(trial>>3)&1,control=(trial>>4)&1;
 unsigned control_kind=(trial>>5)&3;
 *(unsigned*)(actor+0xb0)=flag;actor[0xa8]=(byte)(flag*4);
 *(unsigned*)(role+0x10)=kind;
 *(unsigned*)(actor+0xc0)=status;*(unsigned*)(actor+0xac)=status;
 *(unsigned*)(actor+0x1a4)=control;*(unsigned*)(actor+0x18c)=control;
 *(unsigned*)(actor+0x1c0)=control_kind;*(unsigned*)(actor+0x1a8)=control_kind;
 special_value=((trial>>7)&1)?(trial>>9)|1:0;
 condition_value=((trial>>8)&1)?(trial>>10)|1:0;
 *(unsigned*)(component+0x10)=((trial>>9)&1)?0x20:0x21;
 unsigned mode=(trial>>10)&3;
 struct panic_allowed_services s={0,special_adapter,condition_adapter};
 out[0]=source?original(actor,mode):nhl24_panic_actor_allowed(actor,mode,&s);
 out[1]=trace;out[2]=serial;
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-allowed-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'fixture.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.s'),str(out/'fixture.c'),str(repo/'scripts/nhl27/runtime/panic_services.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint)]
                for trial in range(8192):
                    a=(C.c_uint*3)();b=(C.c_uint*3)()
                    lib.run(1,trial,a);lib.run(0,trial,b)
                    self.assertEqual(list(a),list(b),trial)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
