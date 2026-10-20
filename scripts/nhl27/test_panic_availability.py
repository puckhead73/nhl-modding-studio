"""Availability prefix ordering, byte semantics, and native field adapters."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from oracle_fixture import assembly
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage


class AvailabilityTests(unittest.TestCase):
    def test_prefix_against_source(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        src=checked_image(ElfImage,source,SOURCE_HASH)
        asm=assembly(src,'source_available',0x455ec20,0x455ecc5,
            {0x382630:'action_query',0x455e7b0:'tail_query'},
            {0xf226d60:'registry'},indirect_calls={0x455ec59:'ff90b0000000'})
        fixture=r'''
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
typedef void *(ABI *root_fn)(void*);
typedef void *(ABI *action_fn)(void*,void*);
extern ABI byte source_available(void*);
extern int nhl24_panic_availability_prefix(void*,void*,root_fn,action_fn,byte*);
static byte actor[0x70],animation[0xc68],wrappers[2][0x200],evaluations[2][0x80];
static byte root_wrapper[0x18],root[0x90],actions[0x1530],state[0x38],disabled;
static void *methods[32];void **registry=methods;static void *state_pointer;
static unsigned trace,which,mode,source_mode;
static ABI void **state_query(void *self,void *evaluation) {
    trace=trace*11+1;
    if(evaluation!=evaluations[0]+(source_mode?0x68:0x60))trace+=100000;
    if(mode&8) {
        *(void**)(animation+0xc28)=wrappers[1];
        *(void**)(animation+0xc60)=wrappers[1];which=1;
    }
    return &state_pointer;
}
ABI void *action_query(void *property,void *evaluation) {
    trace=trace*11+2;
    if(property!=root || evaluation!=evaluations[which]+(source_mode?0x68:0x60))trace+=100000;
    return actions;
}
static ABI void *root_query(void *evaluation) {
    if(evaluation!=evaluations[which])trace+=100000;
    return root_wrapper;
}
ABI byte tail_query(void *unused) { trace=trace*11+3;return 1; }
__declspec(dllexport) unsigned run(unsigned source,unsigned flags,unsigned a,unsigned b,unsigned d) {
    source_mode=source;mode=flags;which=0;trace=0;
    *(void**)(actor+0x60)=animation;
    *(void**)(animation+0xc28)=*(void**)(animation+0xc60)=wrappers[0];
    for(unsigned i=0;i<2;i++) {
        *(void**)(wrappers[i]+0x20)=evaluations[i];
        *(void**)(wrappers[i]+0x1f8)=root_wrapper;
    }
    *(void**)(root_wrapper+0x10)=root;
    *(void**)(root+0x68)=*(void**)(root+0x88)=root;
    methods[0x78/8]=methods[0xb0/8]=state_query;
    state_pointer=flags&1?0:state;state[0x31]=a;state[0x33]=b;disabled=d;
    *(void**)(actions+0x1060)=*(void**)(actions+0x1528)=flags&2?0:&disabled;
    byte result=0;
    if(source)result=source_available(actor);
    else {
        if(!nhl24_panic_availability_prefix(actor,&registry,root_query,action_query,&result))return 0xffffffff;
        if(result)result=tail_query(actor);
    }
    return (trace<<8)|result;
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-availability-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'fixture.s').write_text(asm);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.s'),str(out/'fixture.c'),str(repo/'scripts/nhl27/runtime/panic_services.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint]*5;lib.run.restype=C.c_uint
                for flags in (0,1,2,3,8,9,10,11):
                    for a in (0,1,2,127,128,255):
                        for b in (0,1,2,127,128,255):
                            for d in (0,1,2,127,128,255):
                                args=(flags,a,b,d)
                                self.assertEqual(lib.run(0,*args),lib.run(1,*args),args)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
