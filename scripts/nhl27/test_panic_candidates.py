"""Run source/native nearest-player queries on corresponding actor graphs."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH,TARGET_HASH
from research import ElfImage,SelfImage
from oracle_fixture import assembly


class CandidateTests(unittest.TestCase):
    def test_original_queries_and_native_bindings(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists() or not target.exists():
            self.skipTest('Private inputs and Windows LLVM required')
        src=checked_image(ElfImage,source,SOURCE_HASH);dst=checked_image(SelfImage,target,TARGET_HASH)
        code=assembly(src,'source_first',0x453b4f0,0x453bbd9,
            {0x48baa80:'source_position_distance',0x470c7b0:'source_next',0xbfabea0:'fail'},
            {0xec2d9f0:'cookie_global',0x103da598:'source_special',0x1032dbd0:'config_global'})
        code+=assembly(src,'source_second',0x453be00,0x453befd,
            {0x488c470:'source_actor_distance',0x470c7b0:'source_next'}, {})
        code+=assembly(src,'source_next',0x470c7b0,0x470c808,{}, {})
        code+=assembly(src,'source_actor_distance',0x488c470,0x488c4a9,{}, {})
        code+=assembly(src,'source_position_distance',0x48baa80,0x48baaa4,{}, {})
        code+=assembly(src,'source_euclidean',0x44c6350,0x44c637c,{}, {})
        code+=assembly(dst,'native_first',0x27bff20,0x27c067c,
            {0x2a91540:'native_next'},{0xbae0a85:'special',0xb8803e0:'config_global'})
        code+=assembly(dst,'native_second',0x2842b10,0x2842c36,{0x2a91540:'native_next'}, {})
        code+=assembly(dst,'native_next',0x2a91540,0x2a915be,{}, {})
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
typedef float vec4 __attribute__((vector_size(16)));
extern ABI void *source_first(void*,unsigned,const float*),*source_second(void*);
extern ABI void *native_first(void*,unsigned,vec4),*native_second(void*);
extern ABI void *nhl24_panic_first_candidate_guest(panic_address,void*);
extern ABI void *nhl24_panic_second_candidate_guest(panic_address,void*);
extern float nhl27_panic_actor_distance(void*,void*);
extern ABI float source_euclidean(void*,void*);
ABI panic_u8 nhl27_segment_overlap(const float*a,const float*b,const float*c,const float*d){__builtin_trap();}
ABI panic_u8 nhl27_segment_intersection(const float*a,const float*b,const float*c,const float*d,float*e,float*f,float*g){__builtin_trap();}
ABI int nhl24_panic_foot_probe_guest(panic_address base,void*a,unsigned bone,float*out){__builtin_trap();}
static unsigned char actors[13][0xb00],teams[2][0x500],config[0x30];
static unsigned poses[13][16];
static panic_address cookie=0x7b210a;
void *cookie_global=&cookie,*config_global=config;
unsigned char special,source_special[23];
ABI void fail(void){__builtin_trap();}
__declspec(dllexport) unsigned run(unsigned source,unsigned second,const unsigned *input){
    unsigned adapter=source==2;source=source==1;
    unsigned own=source?0xa88:0x918,other=source?0xa90:0x920;
    unsigned head=source?0x28:0x70,role=source?0x44:0x8c;
    unsigned next=source?0x320:0x310,chainrole=source?0x33c:0x32c;
    unsigned slots=source?0x250:0x400;
    special=input[0]&1;source_special[22]=special;config[0x29]=(input[0]>>1)&1;
    for(unsigned i=0;i<13;i++){
        for(unsigned j=0;j<0xb00;j++)actors[i][j]=0;
        *(void**)(actors[i]+0x10)=poses[i];
        *(unsigned*)(actors[i]+0x20)=i<12?((input[1]>>i)&1):0;
        *(unsigned*)(actors[i]+0x3c)=i<12&&((input[2]>>i)&1)?12:i;
        for(unsigned j=0;j<4;j++)poses[i][12+j]=input[3+i*4+j];
        *(void**)(actors[i]+own)=teams[0];*(void**)(actors[i]+other)=teams[1];
        *(unsigned*)(actors[i]+chainrole)=1;
        *(void**)(actors[i]+next)=i<11&&i!=5?actors[i+1]:0;
    }
    actors[12][source?0x25c:0x23c]=(input[0]&4)?0xff:0;
    for(unsigned team=0;team<2;team++){
        *(void**)(teams[team]+head)=(input[0]&(8<<team))?0:actors[team*6];
        *(unsigned*)(teams[team]+role)=1;
        for(unsigned j=0;j<6;j++)*(void**)(teams[team]+slots+j*8)=
            (input[2]&(1<<(16+j)))?0:actors[team*6+j];
    }
    void *answer;
    if(adapter)answer=second?
        nhl24_panic_second_candidate_guest((panic_address)native_second-0x2842b10,actors[12]):
        nhl24_panic_first_candidate_guest((panic_address)native_first-0x27bff20,actors[12]);
    else if(second)answer=source?source_second(actors[12]):native_second(actors[12]);
    else if(source)answer=source_first(actors[12],0,(float*)(poses[12]+12));
    else answer=native_first(actors[12],0,*(vec4*)(poses[12]+12));
    if(!answer)return 0xffffffffu;
    for(unsigned i=0;i<13;i++)if(answer==actors[i])return i;
    __builtin_trap();
}
__declspec(dllexport) unsigned distance(unsigned source,const unsigned *input){
    *(void**)(actors[0]+0x10)=poses[0];*(void**)(actors[1]+0x10)=poses[1];
    for(unsigned i=0;i<4;i++){poses[0][12+i]=input[i];poses[1][12+i]=input[4+i];}
    union {float f;unsigned u;} result;
    result.f=source?source_euclidean(actors[0],actors[1]):nhl27_panic_actor_distance(actors[0],actors[1]);
    return result.u;
}
int _fltused=0;
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-candidates-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding',
                '-fno-builtin','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',
                '-I',str(root/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'oracle.s'),
                str(root/'scripts/nhl27/runtime/panic_geometry.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint)];lib.run.restype=C.c_uint
                lib.distance.argtypes=[C.c_uint,C.POINTER(C.c_uint)];lib.distance.restype=C.c_uint
                rng=random.Random(45300)
                values=[0,0x80000000,0x3f800000,0xbf800000,0x7f7fffff,
                        0x7fc00001,0xffc01000,0x7f800000,0xff800000,0x41200000]
                for n in range(8192):
                    inp=(C.c_uint*55)(rng.randrange(32),rng.getrandbits(12),rng.getrandbits(22),
                        *(rng.choice(values) if n<4096 else rng.getrandbits(32) for _ in range(52)))
                    for second in (0,1):
                        self.assertEqual(lib.run(0,second,inp),lib.run(1,second,inp),(n,second,list(inp)))
                        self.assertEqual(lib.run(2,second,inp),lib.run(1,second,inp),(n,second,list(inp)))
                    coords=(C.c_uint*8)(*list(inp)[3:11])
                    self.assertEqual(lib.distance(0,coords),lib.distance(1,coords),(n,list(coords)))
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
