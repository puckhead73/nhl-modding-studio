"""Execute recovered owner gates against source and native instructions."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from oracle_fixture import assembly
from deke_runtime_evidence import checked_image, SOURCE_HASH, TARGET_HASH
from research import ElfImage, SelfImage


class OwnerGateTests(unittest.TestCase):
    def test_owner_gate_semantics(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name!='nt' or not all(p.exists() for p in (compiler,source,target)):
            self.skipTest('Private binaries and Windows LLVM required')
        src=checked_image(ElfImage,source,SOURCE_HASH)
        dst=checked_image(SelfImage,target,TARGET_HASH)
        # Qualify the enum difference through both signal producers before
        # testing the narrow native availability view.
        from panic_status_evidence import qualify
        qualify(Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility',src,dst)
        asm=assembly(src,'source_state',0x453cd50,0x453cd8c,{},{})
        asm+=assembly(src,'source_global',0x453a9b0,0x453a9f2,{}, {0x1032db88:'option_pointer'})
        asm+=assembly(src,'source_actor',0x453aa00,0x453aa2a,{},{})
        asm+=assembly(dst,'native_state',0x27c88b0,0x27c88f1,{},{})
        asm+=assembly(dst,'native_global',0x27c63e0,0x27c6427,{}, {0xb880398:'option_pointer'})
        fixture=r'''
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
void *memset(void *destination,int value,unsigned long long size) {
    volatile byte *p=destination;for(unsigned long long i=0;i<size;i++)p[i]=(byte)value;
    return destination;
}
extern ABI byte source_state(void*),source_global(void*),source_actor(void*,unsigned);
extern ABI byte native_state(void*),native_global(void*);
extern ABI byte nhl24_panic_actor_state_blocked(void*);
extern ABI byte nhl24_panic_actor_blocked(void*,unsigned);
extern ABI byte nhl24_panic_global_blocked(void*,byte);
extern ABI unsigned nhl24_panic_availability_status(void*);
byte option[0x30];void *option_pointer=option;
__declspec(dllexport) unsigned run(unsigned implementation,unsigned category,unsigned flags,
                                 unsigned kind,unsigned value,unsigned requested) {
    byte actor[0x230]={0},owner[0xe0]={0},component[0x60]={0};
    *(void**)(actor+0x208)=*(void**)(actor+0x228)=flags&1?0:owner;
    *(void**)(owner+0x98)=flags&2?0:component;
    *(void**)(owner+0xd0)=flags&2?0:&kind;
    *(unsigned*)(actor+0x20)=flags&4;
    *(unsigned*)(actor+0x1cc)=*(unsigned*)(actor+0x1e4)=flags&4;
    *(unsigned*)(component+0x10)=kind;component[0x58]=value;option[0x29]=value;
    *(unsigned*)(actor+0xac)=kind;
    if(category==3)return nhl24_panic_availability_status(actor);
    if(category==0)return implementation==1?source_state(actor):
        implementation==2?native_state(actor):nhl24_panic_actor_state_blocked(actor);
    if(category==1)return implementation==1?source_global(actor):
        implementation==2?native_global(actor):nhl24_panic_global_blocked(actor,value);
    return implementation?source_actor(actor,requested):nhl24_panic_actor_blocked(actor,requested);
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-owner-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'fixture.s').write_text(asm);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.s'),str(out/'fixture.c'),str(repo/'scripts/nhl27/runtime/panic_services.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint]*6;lib.run.restype=C.c_uint
                for category in range(3):
                    for flags in range(8):
                        for kind in (0,1,2,3,4,5,6,0x20,0x8000,0x10000,0xffffffff):
                            for value in (0,1,2,127,128,255):
                                for requested in (0,1,kind,0xffffffff):
                                    args=(category,flags,kind,value,requested)
                                    wanted=lib.run(1,*args)
                                    self.assertEqual(lib.run(0,*args),wanted,args)
                                    if category<2:self.assertEqual(lib.run(2,*args),int(bool(wanted)),args)
                for status in list(range(256))+[0xffffffff]:
                    translated=lib.run(0,3,0,status,0,0)
                    self.assertEqual(translated==17,status==18)
                    self.assertEqual(translated==14,status==14)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
