"""Compare the style-output route with NHL24's original initializer instructions."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image, TARGET_HASH
from research import SelfImage
from oracle_fixture import assembly


class StyleOutputTests(unittest.TestCase):
    def test_native_route_and_nullable_live_slots(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not target.exists():
            self.skipTest('Private target and Windows LLVM required')
        image=checked_image(SelfImage,target,TARGET_HASH)
        # Enter after the native root query, with its original RAX/R15 inputs.
        code='.text\n.globl original\noriginal:\npush %r15\nmov %rsi,%r15\nmov %rdi,%rax\n'
        code+=assembly(image,'original_route',0x2ec377b,0x2ec37b1,
                       {0x40c5a0:'skating_query',0x411510:'style_query'}, {})
        code+='pop %r15\nret\n'
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
extern ABI void *original(void*,void*);
extern void *nhl24_panic_style_outputs(void*,void*(ABI*)(void*),void*(ABI*)(void*,void*),void*(ABI*)(void*,void*));
extern int nhl24_panic_style_slots(void*,float**);
static unsigned char actor[0x68],anim[0xc30],wrapper[0x28],context[0x70];
static unsigned char root_wrapper[0x18],root[0x100],skating[0xb0],style[0x10],record[0x130];
static unsigned char asset[8],style_asset[8];
static float values[4];
static unsigned calls,bad;
int _fltused=0;
ABI void *root_query(void *c){if(c!=context)bad++;return root_wrapper;}
ABI void *skating_query(void*a,void*c){if(a!=asset || c!=context+0x60 || calls++!=0)bad++;return skating;}
ABI void *style_query(void*a,void*c){if(a!=style_asset || c!=context+0x60 || calls++!=1)bad++;return record;}
__declspec(dllexport) unsigned check(void){
 *(void**)(actor+0x60)=anim;*(void**)(anim+0xc28)=wrapper;*(void**)(wrapper+0x20)=context;
 *(void**)(root_wrapper+0x10)=root;*(void**)(root+0xf8)=asset;
 *(void**)(skating+0xa8)=style;*(void**)(style+8)=style_asset;
 void *w=wrapper;calls=bad=0;
 if(original(root_wrapper,&w)!=record || calls!=2 || bad)return 1;
 calls=bad=0;
 if(nhl24_panic_style_outputs(actor,root_query,skating_query,style_query)!=record || calls!=2 || bad)return 2;
 unsigned offsets[4]={0x128,0x108,0x100,0xf8};
 for(unsigned mask=0;mask<16;mask++){
  float *slots[4];
  for(unsigned n=0;n<4;n++)*(void**)(record+offsets[n])=(mask&(1u<<n))?values+n:0;
  if(!nhl24_panic_style_slots(record,slots))return 3;
  for(unsigned n=0;n<4;n++)if(slots[n]!=((mask&(1u<<n))?values+n:0))return 4;
 }
 void **links[8]={(void**)(actor+0x60),(void**)(anim+0xc28),(void**)(wrapper+0x20),
  (void**)(root_wrapper+0x10),(void**)(root+0xf8),(void**)(skating+0xa8),(void**)(style+8),0};
 for(unsigned n=0;n<7;n++){
  void *saved=*links[n];*links[n]=0;calls=bad=0;
  if(nhl24_panic_style_outputs(actor,root_query,skating_query,style_query))return 5;
  *links[n]=saved;
 }
 if(nhl24_panic_style_outputs(0,root_query,skating_query,style_query))return 6;
 return 0;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-style-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/panic_services.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:self.assertEqual(lib.check(),0)
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
