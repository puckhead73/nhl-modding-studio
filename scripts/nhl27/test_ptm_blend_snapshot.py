"""Compare complete source snapshot initialization against the port adapter."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class SnapshotTests(unittest.TestCase):
    def test_source_snapshot_branches(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and LLVM required')
        im=checked_image(ElfImage,source,SOURCE_HASH)
        indirect={a:'ff5058' for a in (0x283ee8,0x283f00,0x283f72,0x284001,0x284023,0x2840ad)}
        indirect.update({a:'ff90d8000000' for a in (0x283f9a,0x28414a,0x284176)})
        indirect[0x284217]='ff5010'
        code=assembly(im,'original',0x283e60,0x28424a,{
            0x73fa20:'scope_begin',0x73fa70:'scope_end',0x73faa0:'scope_flush',
            0xa3f6870:'pose_clone',0x17ffc60:'pose_retain',0x17fdc10:'pose_release',
            0xa3f9b60:'view_begin',0xa3f9e90:'view_end',0x6c2d50:'pose_import',
            0x6cdd20:'combine',0xbfabea0:'fail'}, {0xec2d9f0:'guard'},indirect_calls=indirect)
        fixture=r'''
#include "ptm_blend_snapshot.h"
#define ABI PTM_GAME_ABI
typedef panic_address A;
int _fltused=0;static A cookie;A *guard=&cookie;
static unsigned char object[0x320],master[0x30],asset[0x80],poses[3][0x100];
static A vt[12],seekvt[3],metavt[28],meta[1];
static unsigned logs[256],used;static int refs[3];
static unsigned id(void *p){return ((unsigned char*)p-poses[0])/sizeof(poses[0]);}
static void event(unsigned n,A a){logs[used++]=n;logs[used++]=(unsigned)a;}
ABI void fail(void){__builtin_trap();}
ABI void pose_retain(void*p){refs[id(p)]++;}
ABI void pose_release(void*p){refs[id(p)]--;}
void nhl24_ptm_pose_retain(void*p){pose_retain(p);}
void nhl24_ptm_pose_release(void*p){pose_release(p);}
ABI void *pose_clone(void*p,int unused){(void)unused;event(3,id(p));return poses[1];}
ABI void scope_begin(A*s){s[0]=123;s[1]=0;event(1,0);}
ABI void scope_end(A*s){(void)s;event(9,0);}
static void flush(A handle,void*p){
 event(4,handle);event(40,id(p));
 /* Include signed zero and NaN lanes in root vectors. */
 for(unsigned i=0;i<24;i++){
  unsigned bits=(i%7==0)?0x80000000u:(i%11==0)?0x7fc12345u:0x3f000000u+((unsigned)handle+i)*97u;
  *(unsigned*)((unsigned char*)p+0x80+i*4)=bits;
 }
}
ABI void scope_flush(A*s,void*p){flush(s[1],p);}
ABI A eval(void*p,void*ctx,float dt){
 (void)p;(void)ctx;union{float f;unsigned u;}v={dt};event(2,v.u);
 *(float*)(master+0x1c)+=dt;return (A)(used+100);
}
ABI void seek(void*p,float t){(void)p;union{float f;unsigned u;}v={t};event(8,v.u);*(float*)(master+0x1c)=t;}
ABI void root(A*out,void*p){(void)p;out[0]=0;out[1]=0;}
ABI void view_begin(void*out,void*p,unsigned x,unsigned y){(void)out;(void)p;(void)x;(void)y;}
ABI void view_end(void*p){(void)p;}
ABI A pose_import(A arena,void**p){(void)arena;event(5,id(*p));return 77;}
ABI A combine(A arena,void*properties,A a,A b){(void)arena;(void)properties;event(6,a);event(60,b);return a*3+b;}
static void *cl(void*c,void*p){(void)c;return pose_clone(p,0);}
static void retain(void*c,void*p){(void)c;pose_retain(p);}
static void release(void*c,void*p){(void)c;pose_release(p);}
static void begin(void*c){(void)c;A s[2];scope_begin(s);}
static void end(void*c){(void)c;scope_end(0);}
static A evaluate(void*c,float dt){(void)c;return eval(master,0,dt);}
static float time(void*c){(void)c;return *(float*)(master+0x1c);}
static void settime(void*c,float t){(void)c;seek(master+0x10,t);}
static void fl(void*c,A h,void*p){(void)c;flush(h,p);}
static A imp(void*c,void*p){(void)c;return pose_import(123,&p);}
static A comp(void*c,A a,A b){(void)c;return combine(123,object+0x38,a,b);}
static float *rt(void*c,void*p){(void)c;return (float*)((unsigned char*)p+0x80);}
extern ABI void original(void*);
__declspec(dllexport) void run(unsigned which,unsigned mode,unsigned update,unsigned force,unsigned nominal,unsigned initial,unsigned*out){
 for(unsigned i=0;i<sizeof(object);i++)object[i]=0;
 for(unsigned i=0;i<sizeof(master);i++)master[i]=0;
 for(unsigned i=0;i<sizeof(asset);i++)asset[i]=0;
 for(unsigned i=0;i<sizeof(poses);i++)((unsigned char*)poses)[i]=0;
 for(unsigned i=0;i<256;i++)logs[i]=0;
 used=0;for(unsigned i=0;i<3;i++)refs[i]=0;
 vt[11]=(A)eval;seekvt[2]=(A)seek;metavt[27]=(A)root;meta[0]=(A)metavt;
 *(A*)master=(A)vt;*(A*)(master+0x10)=(A)seekvt;*(float*)(master+0x1c)=5.0f;
 for(unsigned i=0;i<3;i++){*(unsigned short*)(poses[i]+0x1c)=0x80;*(A*)(poses[i]+0x20)=(A)meta;}
 *(A*)(object+0x30)=(A)poses[0];*(A*)(object+0x2b8)=(A)asset;
 *(A*)(object+0x2c8)=(A)master;*(A*)(object+0x2d8)=(A)poses[2];refs[2]=1;
 *(A*)(object+0x2e0)=initial?(A)poses[1]:0;refs[1]=initial!=0;
 *(unsigned*)(object+0x300)=nominal;object[0x315]=(unsigned char)force;
 *(int*)(asset+0x74)=(int)mode;asset[0x78]=(unsigned char)update;
 struct ptm_blend_extension e={0};e.magic=PTM_BLEND_MAGIC;e.pose_b=poses[2];
 e.pose_a=initial?poses[1]:0;*(unsigned*)&e.clock.nominal=nominal;
 struct ptm_blend_snapshot_services s={{0,cl,retain,release},begin,end,evaluate,time,settime,fl,imp,comp,rt};
 if(which)nhl27_ptm_blend_snapshots(&e,poses[0],(int)mode,(panic_u8)update,(panic_u8)force,&s);
 else original(object);
 unsigned *target=which?(unsigned*)e.target:(unsigned*)(object+0x2f0);
 for(unsigned i=0;i<4;i++)out[i]=target[i];
 for(unsigned p=1;p<3;p++)for(unsigned i=0;i<24;i++)out[4+(p-1)*24+i]=*(unsigned*)(poses[p]+0x80+i*4);
 out[52]=*(unsigned*)(master+0x1c);out[53]=which?e.initialized:object[0x314];
 out[54]=(unsigned)refs[1];out[55]=(unsigned)refs[2];out[56]=used;
 for(unsigned i=0;i<256;i++)out[57+i]=logs[i];
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-snapshot-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'original.s').write_text(code);(p/'fixture.c').write_text(fixture)
            r=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.c'),str(p/'original.s'),str(root/'scripts/nhl27/runtime/ptm_blend_snapshot.c'),
                str(root/'scripts/nhl27/runtime/ptm_blend_storage.c'),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint]*6+[C.POINTER(C.c_uint)]
                values=[0,0x80000000,0x3f800000,0x40000000,0x42200000,0xbf800000,0x7fc12345,0x7f800000,0xff800000]
                for mode in range(3):
                    for update in range(3):
                        for force in range(3):
                            for nominal in values:
                                for initial in range(2):
                                    a=(C.c_uint*313)();b=(C.c_uint*313)()
                                    args=(mode,update,force,nominal,initial)
                                    lib.run(0,*args,a);lib.run(1,*args,b)
                                    self.assertEqual(list(a),list(b),str(args))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
