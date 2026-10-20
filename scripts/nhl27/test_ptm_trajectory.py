"""Compare native pose-job adapter with the original NHL27 trajectory job."""
import ctypes as C
import math
import os
from pathlib import Path
import random
import struct
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly
from ptm_trajectory_math import generate


class TrajectoryTests(unittest.TestCase):
    def test_original_job_and_native_packet(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code,_=generate(image)
        code+=assembly(image,'original',0x1bed90,0x1bf123,
            {0x73d300:'get_pose',0x4a890:'original_sine',0xbfabea0:'fail'},
            {0xec2d9f0:'guard_pointer'},indirect_calls={0x1bede1:'ff90d8000000'})
        code+=assembly(image,'original_sine',0x4a890,0x4ad89,
            {0x25da630:'original_reduce',0xbfabea0:'fail'},{0xec2d9f0:'guard_pointer'})
        code+=assembly(image,'original_reduce',0x25da630,0x25da743,{},
            {0xdf600d0:'original_table'})
        code+='\n.balign 16\noriginal_table:\n.byte '+','.join(map(str,image.read(0xdf600d0,28)))+'\n'
        fixture=r'''
#include "ptm_native.h"
#define ABI __attribute__((sysv_abi))
typedef panic_address A;
typedef unsigned char B;
typedef void *(ABI *get_fn)(void*,unsigned);
typedef A (ABI *final_fn)(void*,unsigned,void**);
extern ABI void original(void*);
extern void nhl24_ptm_trajectory_execute(void*,get_fn);
extern A nhl24_ptm_trajectory_enqueue(A,void*,A,const float*,float,A,A,final_fn,A);
static _Alignas(16) B pose[0x100],metadata[0x108],pool[0x100],job[0x40],payload[64],arena[0x1400];
static A vtable[28],result_properties;static unsigned offset,bad,calls;
A guard=1,*guard_pointer=&guard;int _fltused=0;
ABI void fail(void){__builtin_trap();}
ABI void *get_pose(void*p,unsigned i){if(p!=pool||i!=3)bad++;calls++;return pose;}
ABI void metadata_root(void*out,void*m){
 if(m!=metadata)bad++;((unsigned*)out)[0]=0;
 ((unsigned*)out)[1]=0;((unsigned*)out)[2]=*(unsigned*)(metadata+0xf4);
 ((unsigned*)out)[3]=*(unsigned*)(metadata+0xf8);
}
__declspec(dllexport) void run(unsigned which,unsigned missing,const unsigned*values,unsigned*out){
 for(unsigned n=0;n<sizeof(pose);n++)pose[n]=0;
 bad=calls=0;offset=0;
 *(void**)(pose+0x20)=metadata;*(unsigned short*)(pose+0x1c)=0x40;
 *(void**)metadata=vtable;vtable[27]=(A)metadata_root;
 *(unsigned*)(metadata+0xf4)=0;*(unsigned*)(metadata+0xf8)=missing?0x80000000u:0;
 for(unsigned n=0;n<16;n++)((unsigned*)(pose+0x40))[n]=values[n];
 for(unsigned n=0;n<11;n++)((unsigned*)payload)[n]=values[16+n];
 *(void**)(job+0x10)=&offset;*(void**)(job+0x20)=pool;
 *(unsigned*)(job+0x28)=3;*(void**)(job+0x38)=payload;
 *(A*)(pool+0x48+3*16)=123;
 if(which)nhl24_ptm_trajectory_execute(job,get_pose);else original(job);
 for(unsigned n=0;n<16;n++)out[n]=((unsigned*)(pose+0x40))[n];
 out[16]=bad;out[17]=calls;out[18]=(unsigned)*(A*)(pool+0x48+3*16);
}
static unsigned packet_mode;static float movement[4]={1,2,3,4};static A input_word;
ABI A finalize(void*a,unsigned size,void**properties){
 if(a!=arena||size!=8)bad++;
 A *p=(A*)(arena+8);
 if(!(packet_mode&1)){
  if(*p++!=3||*p++!=(A)&input_word)bad++;
  if(*(unsigned*)(arena+0x100c)!=0x7b)bad++;
 }else if(!arena[0x139c])bad++;
 A *header=p;B *data=(B*)(((A)p+31)&~(A)15);
 if(*p!=1||header[1]!=(A)(data+64-(B*)(header+2))/8)bad++;
 for(unsigned n=0;n<4;n++)if(((float*)data)[n]!=movement[n])bad++;
 if(((float*)data)[4]||((float*)data)[5]||((float*)data)[6]||((float*)data)[7]!=1)bad++;
 if(((float*)data)[8]!=0.375f||*(A*)(data+0x28)||*(A*)(data+0x30)!=0x12345678)bad++;
 p=header+2+header[1];
 if(*p++!=5||*p++!=0xffffffffu||*p++!=0x9999)bad++;
 if(*(A**)(arena+0x1000)!=p)bad++;
 *properties=&result_properties;calls++;return 0x5678;
}
__declspec(dllexport) unsigned packet(unsigned mode){
 for(unsigned n=0;n<sizeof(arena);n++)arena[n]=0;
 packet_mode=mode;bad=calls=0;input_word=(A)0x7b<<24;result_properties=0;
 A result=nhl24_ptm_trajectory_enqueue(0x12345678,(mode&2)?0:arena,
  (mode&1)?777:(A)&input_word,movement,0.375f,777,0x9999,finalize,0x8888);
 if(mode&2){if(result!=777||calls)bad++;}
 else if(result!=0x5678||calls!=1||result_properties!=0x8888)bad++;
 return bad;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-trajectory-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/ptm_trajectory.c'),
                '-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'));lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_uint),C.POINTER(C.c_uint)]
            lib.packet.argtypes=[C.c_uint];lib.packet.restype=C.c_uint
            rng=random.Random(0x1bed90)
            def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
            for n in range(8192):
                values=[bits(rng.uniform(-10,10)) for _ in range(27)]
                for start in (0,20):
                    q=[rng.uniform(-1,1) for _ in range(4)];length=math.sqrt(sum(v*v for v in q))
                    values[start:start+4]=[bits(v/length) for v in q]
                values[24]=bits(rng.uniform(-2,2));values[25]=bits(rng.uniform(-2,2));values[26]=n%3
                if n%4==0:values[20:24]=[0,0,0,bits(1)];values[25]=0;values[26]=0
                if n%31==0:values[(n//31)%26]=[0x7fc00001,0x80000000,0x7f800000,0xff800000][n%4]
                a=(C.c_uint*27)(*values);x=(C.c_uint*19)();y=(C.c_uint*19)()
                lib.run(0,n%17==0,a,x);lib.run(1,n%17==0,a,y)
                self.assertEqual(bytes(x),bytes(y),f'case {n}')
                self.assertEqual(list(y)[16:],[0,1,0])
            for mode in range(4):self.assertEqual(lib.packet(mode),0)
            import _ctypes
            _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
