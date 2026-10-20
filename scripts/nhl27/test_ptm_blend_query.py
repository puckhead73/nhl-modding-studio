"""Validate query ABI, cache ownership and scope publication boundaries."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class BlendQueryTests(unittest.TestCase):
    def test_query_arguments_and_resource_lifetime(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        fixture=r'''
#include "ptm_blend_query.h"
#include "ptm_query_scope.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
int _fltused=0;
static unsigned char blend[PTM_BLEND_EXTENDED_SIZE],cache[0x16c0];
static A match,vt[28];static unsigned logvalue,error,refs,fail_alloc;
static struct ptm_blend_query_arguments args;
static int pose;
static void *allocate(void*c){if(c!=blend)error|=1;logvalue=logvalue*10+1;return fail_alloc?0:cache;}
static void construct(void*c,void*p){if(c!=blend||p!=cache)error|=2;logvalue=logvalue*10+2;}
void nhl24_ptm_pose_retain(void*p){if(p!=&pose)error|=4;refs++;logvalue=logvalue*10+3;}
void nhl24_ptm_pose_release(void*p){if(p!=&pose||refs!=2)error|=8;refs--;logvalue=logvalue*10+5;}
ABI int nhl24_ptm_velocity_query_guest(A base,void*s,void*q,void*p,void*c){
 (void)q;if(base!=123||s!=blend||p!=&pose||c!=cache||refs!=2)error|=16;
 logvalue=logvalue*10+8;return 1;
}
static int dispatch(void){A q[16]={0};q[12]=(A)args.current_map;q[13]=(A)args.future_map;
 return nhl24_ptm_velocity_dispatch_guest(123,blend,q);}
static void destroy(void*c,void*p){if(c!=blend||p!=cache||refs!=1||dispatch())error|=32;logvalue=logvalue*10+6;}
static void release(void*c,void*p){if(c!=blend||p!=cache)error|=64;logvalue=logvalue*10+7;}
ABI void query(void*m,void*current,void*future,void*slot,void*contexts,void*input,
 unsigned first,unsigned second,unsigned third,float remaining,float delta,float scale){
 if(m!=&match||current!=args.current_map||future!=args.future_map||slot!=args.slot||contexts!=args.contexts||
 input!=args.input_state||first!=args.first||second!=args.second||third!=args.third||
 remaining!=args.remaining||delta!=args.delta||scale!=args.scale)error|=128;
 logvalue=logvalue*10+4;if(dispatch()!=1)error|=256;
}
__declspec(dllexport) unsigned run(unsigned enabled,unsigned initialized,unsigned failure,unsigned maps){
 for(unsigned i=0;i<sizeof(blend);i++)blend[i]=0;
 error=logvalue=0;refs=1;fail_alloc=failure;
 struct ptm_blend_extension *e=(void*)(blend+PTM_BLEND_EXTENSION_OFFSET);
 e->magic=PTM_BLEND_MAGIC;e->enabled=(panic_u8)enabled;e->initialized=(panic_u8)initialized;e->pose_a=&pose;
 vt[27]=(A)query;match=(A)vt;*(A*)(blend+0x2c0)=(A)&match;
 args=(struct ptm_blend_query_arguments){maps?(void*)1:0,(void*)2,(void*)3,(void*)4,(void*)5,6,7,8,9.25f,.5f,11.75f};
 struct ptm_blend_query_services s={blend,allocate,construct,destroy,release};
 int result=nhl24_ptm_blend_query(blend,&args,&s);
 int active=enabled==1&&initialized!=0;
 unsigned expected=!active?0:failure?1:maps?12348567:123567;
 if(result!=(active&&!failure&&maps)||logvalue!=expected||refs!=1||dispatch())error|=512;
 return error;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-blend-query-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture)
            r=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),str(p/'fixture.c'),
                str(root/'scripts/nhl27/runtime/ptm_blend_query.c'),str(root/'scripts/nhl27/runtime/ptm_query_scope.c'),
                '-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint]*4
                for enabled in range(3):
                    for initialized in range(3):
                        for failure in range(2):
                            for maps in range(2):self.assertEqual(lib.run(enabled,initialized,failure,maps),0,
                                (enabled,initialized,failure,maps))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
