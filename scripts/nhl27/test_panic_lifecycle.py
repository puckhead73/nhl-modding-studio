"""Constructor/destructor oracle plus native owner slot reuse guards."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from test_panic_state import State
from oracle_fixture import assembly


class LifecycleTests(unittest.TestCase):
    def test_original_construction_padding_and_owner_reuse(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        asm=assembly(image,'original_construct',0x4a54380,0x4a544d0,
            {0x4a51930:'source_reset',0x4b254d0:'source_ratings',0x4a05fb0:'original_normalize'},{})
        asm+=assembly(image,'original_destruct',0x4a544d0,0x4a544f0,{0x4a51930:'source_reset'},{})
        asm+=assembly(image,'original_normalize',0x4a05fb0,0x4a05fe0,{},{})
        fixture=r'''
#include "panic_lifecycle.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original_construct(struct panic_state*,panic_address),original_destruct(struct panic_state*);
extern void nhl27_panic_construct(struct panic_state*,panic_address,const struct panic_lifecycle_services*);
extern void nhl27_panic_destruct(struct panic_state*,const struct panic_lifecycle_services*);
extern struct panic_state *nhl24_panic_owner_create(unsigned,panic_address,panic_address,const struct panic_lifecycle_services*);
extern struct panic_state *nhl24_panic_owner_get(unsigned,panic_address,panic_address);
extern int nhl24_panic_owner_destroy(unsigned,panic_address,const struct panic_lifecycle_services*);
static unsigned char actor[0x80],database[0x3670];
static int ratings[4],reset_count;
ABI void source_reset(struct panic_state*s){
 ++reset_count;s->duration=s->active_ticks=0;s->opponent=0;s->facing=0.5f;
 for(unsigned n=0;n<4;n++)s->position[n]=(float)n;
 s->flag=0;s->align_deadline=0;
}
ABI void *source_ratings(void){return database;}
static void reset(void*c,struct panic_state*s){source_reset(s);}
static int rating(void*c,panic_address a,unsigned id){
 switch(id){case 0x11d:return ratings[0];case 0x11f:return ratings[1];
 case 0x121:return ratings[2];case 0x120:return ratings[3];default:__builtin_trap();}
}
static const struct panic_lifecycle_services services={0,reset,rating};
__declspec(dllexport) int run(int original,int destroy,struct panic_state*s,const int*r){
 reset_count=0;
 unsigned offsets[4]={0x3588,0x35e8,0x3648,0x3618};
 for(unsigned n=0;n<4;n++){
  ratings[n]=r[n];*(void**)(database+offsets[n])=ratings+n;
  *(int*)(database+offsets[n]+0x18)=0;*(panic_address*)(database+offsets[n]+0x20)=0;
 }
 if(destroy){
  if(original)original_destruct(s);else nhl27_panic_destruct(s,&services);
 }else{
  if(original)original_construct(s,(panic_address)actor);
  else nhl27_panic_construct(s,(panic_address)actor,&services);
  s->actor=1;
 }
 return reset_count;
}
__declspec(dllexport) int slots(void){
 for(unsigned n=0;n<18;n++){
  struct panic_state *s=nhl24_panic_owner_create(n,100+n,200+n,&services);
  if(!s || s!=nhl24_panic_owner_get(n,100+n,200+n))return 1;
  if(nhl24_panic_owner_create(n,100+n,200+n,&services))return 2;
  if(nhl24_panic_owner_get(n,100+n,999) || nhl24_panic_owner_get(n,999,200+n))return 3;
  if(nhl24_panic_owner_destroy(n,999,&services))return 4;
 }
 if(nhl24_panic_owner_create(18,1000,2000,&services))return 5;
 for(unsigned n=0;n<18;n++){
  if(!nhl24_panic_owner_destroy(n,100+n,&services))return 6;
  if(nhl24_panic_owner_get(n,100+n,200+n))return 7;
  if(nhl24_panic_owner_destroy(n,100+n,&services))return 8;
  struct panic_state *s=nhl24_panic_owner_create(n,300+n,400+n,&services);
  if(!s || s->actor!=400+n)return 9;
  if(!nhl24_panic_owner_destroy(n,300+n,&services))return 10;
 }
 return 0;
}
'''
        repo=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-lifecycle-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'native.s').write_text(asm);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'native.s'),str(repo/'scripts/nhl27/runtime/panic_lifecycle.c'),
                str(repo/'scripts/nhl27/runtime/panic_state.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_int,C.c_int,C.POINTER(State),C.POINTER(C.c_int)]
                rng=random.Random(27543)
                for index in range(2000):
                    ratings=(C.c_int*4)(*(rng.choice([-2147483648,-100,0,49,50,51,99,100,2147483647]) for n in range(4)))
                    initial=bytes(rng.randrange(256) for n in range(96));results=[]
                    for original in (0,1):
                        state=State.from_buffer_copy(initial)
                        self.assertEqual(lib.run(original,index%2,C.byref(state),ratings),1)
                        results.append(bytes(state))
                    self.assertEqual(results[0],results[1],(index,list(ratings)))
                self.assertEqual(lib.slots(),0)
                self.assertEqual(lib.slots(),0)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
