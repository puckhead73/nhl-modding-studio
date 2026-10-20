"""Run the original child-checking gate against its port, including live mutation."""
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

class PendingTests(unittest.TestCase):
    def test_original_call_order_and_mutable_assets(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x3f0560,0x3f06f8,{}, {0xf223968:'registry'}, indirect_calls={
            0x3f05b8:'ff5040',0x3f05cc:'ff5020',0x3f05ed:'ff90b0000000',
            0x3f0614:'ff9028010000',0x3f064b:'ff9010010000',0x3f0670:'ff5040',
            0x3f0688:'ff5020',0x3f06ae:'ff5040',0x3f06c6:'ff5020'})
        fixture=r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
typedef __UINTPTR_TYPE__ addr;
extern ABI byte original(void*);
static byte parent[0x168],root[3][0x680],assets[3][0x138],contexts[3][0x70];
static void *entries[3],*slots[3];
static addr registry_methods[23],methods[38];
addr registry;
struct property {addr *methods;unsigned child,kind;};
static struct property properties[3][5];
static const unsigned *values;
static unsigned *trace;
static unsigned log_value(unsigned child,unsigned kind){
 trace[++trace[0]]=child*16+kind+1;
 unsigned v=values[child*7+kind];
 if(values[child*7+6]==kind+1)*(void**)(root[child]+0x670)=0;
 return v;
}
ABI void **lookup(void*p,void*c){
 if(p!=&registry)__builtin_trap();
 for(unsigned n=0;n<3;n++)if(c==contexts[n]+0x68){trace[++trace[0]]=n*16;return &slots[n];}
 __builtin_trap();
}
ABI void *cast(void*p,unsigned type){if(type!=0xbcc2c469)__builtin_trap();return p;}
ABI unsigned integer(void*p,void*c){struct property*q=p;if(c!=contexts[q->child]+0x68)__builtin_trap();return log_value(q->child,q->kind);}
ABI byte boolean(void*p,void*c){return integer(p,c);}
ABI byte rally(void*p,void*c,unsigned mode){if(mode!=1)__builtin_trap();return boolean(p,c);}
static void *resolve(void*c,void*entry){(void)c;return *lookup(&registry,(byte*)entry+0x68);}
static int has_asset(void*c,void*r){(void)c;return *(void**)((byte*)r+0x670)!=0;}
static unsigned query(void*c,void*r,enum panic_checking_query kind){
 (void)c;unsigned n=((byte*)r-(byte*)root)/sizeof(root[0]);return log_value(n,kind);
}
__declspec(dllexport) void run(unsigned source,const unsigned*in,unsigned*out){
 values=in;trace=out;out[0]=0;registry=(addr)registry_methods;registry_methods[22]=(addr)lookup;
 methods[8]=(addr)cast;methods[4]=(addr)boolean;methods[0x110/8]=(addr)rally;methods[0x128/8]=(addr)integer;
 unsigned offsets[5]={0x60,0x90,0xa0,0xb8,0x130};
 for(unsigned n=0;n<3;n++){
  entries[n]=contexts[n];slots[n]=root[n];
  *(void**)(root[n]+0x670)=in[n*7+5]?assets[n]:0;*(void**)(root[n]+0x678)=contexts[n];
  for(unsigned k=0;k<5;k++){properties[n][k]=(struct property){methods,n,k};*(void**)(assets[n]+offsets[k])=&properties[n][k];}
 }
 unsigned count=in[21]%4;
 *(void**)(parent+0x158)=entries;*(void**)(parent+0x160)=entries+count;
 struct panic_pending_services s={0,resolve,has_asset,query};
 out[40]=source?original(parent):nhl27_panic_pending_check(entries,entries+count,&s);
}
int _fltused=0;
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-pending-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'fixture.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),str(out/'fixture.s'),
                str(out/'fixture.c'),str(repo/'scripts/nhl27/runtime/panic_update.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.POINTER(C.c_uint),C.POINTER(C.c_uint)]
                rng=random.Random(0x3f0560)
                for trial in range(8192):
                    values=[]
                    for n in range(3):
                        values.extend(rng.choice([0,0,0,1,2,255,256,0xffffffff]) for _ in range(5))
                        values.extend([rng.randrange(2),rng.choice([0,0,1,2,5])])
                    values.append(rng.randrange(4));data=(C.c_uint*22)(*values)
                    # Rally mutation requires a stable asset for its immediate Boolean reads.
                    # The source only guards mutation after hip and before the push query.
                    for n in range(3):
                        if data[n*7+6]==2:data[n*7+6]=0
                    a=(C.c_uint*41)();b=(C.c_uint*41)();lib.run(1,data,a);lib.run(0,data,b)
                    self.assertEqual(list(a),list(b),(trial,values))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)

if __name__=='__main__':unittest.main()
