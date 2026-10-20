"""Original instruction oracles for offset transforms and fixed-vector growth."""
import ctypes as C
import os
from pathlib import Path
import random
import struct
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly
from ptm_trajectory_math import generate


def wrapped_slice(image,name,start,end,prologue,epilogue):
    code=assembly(image,name,start,end,{}, {})
    code=code.replace(name+':\n',name+':\n'+prologue+'\n',1)
    if '.balign 16' in code:code=code.replace('.balign 16',epilogue+'\n.balign 16',1)
    else:code+=epilogue+'\n'
    return code


class OffsetTests(unittest.TestCase):
    def test_offset_math_and_vector_storage(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH);code,_=generate(image)
        code+=wrapped_slice(image,'world_original',0x280cac,0x280d09,
            'push rbp\nmov rbp,rsp\nsub rsp,0xa0\nmov rax,rsi\nvmovups xmm0,[rdx+0x20]\nvmovups [rbp-0x90],xmm0',
            'vmovups [rdi],xmm0\nleave\nret')
        code+=wrapped_slice(image,'split_original',0x2808ce,0x28090a,
            'push rbp\nmov rbp,rsp\nsub rsp,0xe0\nmov rax,rdi\nmov rcx,rsi\nvmovups xmm0,[rdx]\nvmovaps [rbp-0xe0],xmm0',
            'leave\nret')
        code+=assembly(image,'resize_original',0x1f46f0,0x1f4865,
            {0x8c20c0:'original_allocate',0x8c2120:'original_free'}, {})
        code+=assembly(image,'sincos_original',0x17130,0x173bb,
            {0x25da630:'ptm_sine_reduce',0xbfabea0:'ptm_math_fail'}, {0xec2d9f0:'guard_pointer'})
        fixture=r'''
#include "ptm_native.h"
#define ABI __attribute__((sysv_abi))
typedef panic_address A;
typedef unsigned char B;
extern ABI void world_original(float*,const float*,const float*);
extern ABI void split_original(float*,float*,const float*);
extern ABI void sincos_original(float,float*,float*);
extern ABI void nhl27_ptm_sincos(float,float*,float*);
extern void nhl27_ptm_offset_world(float*,const float*,const float*);
extern void nhl27_ptm_offset_root(float*,float*,const float*,const float*);
extern ABI void resize_original(void*,unsigned,const float*);
typedef void*(ABI *alloc_fn)(void*,A,const char*,unsigned,unsigned,unsigned);
typedef void(ABI *free_fn)(void*,void*,A);
extern void nhl24_ptm_offset_resize(struct ptm_transition*,unsigned,void*,alloc_fn,free_fn,const char*);
struct scope {void*arena;A handle;void*evaluation[3];B flag;B pad[7];};
extern void nhl24_ptm_offset_scope_begin(struct scope*,void*,void*,A);
extern void nhl24_ptm_offset_scope_end(struct scope*,void*,void(ABI*)(void*));
static struct ptm_transition state;
static _Alignas(16) unsigned old[1024],fresh[1024];
static unsigned bad,allocations,frees,capacity,requested,external;
A guard=1,*guard_pointer=&guard;
int _fltused=0;
ABI void *allocate(void*a,A size,const char*label,unsigned flags,unsigned alignment,unsigned extra){
 if(a!=(void*)123||size!=requested*16||label!=(const char*)456||flags!=1||alignment!=16||extra)bad++;
 allocations++;return fresh;
}
ABI void release(void*a,void*p,A size){
 if(a!=(void*)123||p!=old||size!=capacity*16||state.begin!=(A)old||state.end!=state.begin)bad++;
 for(unsigned n=0;n<requested*4;n++)if(fresh[n])bad++;
 frees++;
}
ABI void *original_allocate(void*a,A size,unsigned alignment,unsigned flags,unsigned extra){
 if(a!=&state.allocator||flags||extra)bad++;
 return allocate((void*)123,size,(const char*)456,1,alignment,0);
}
ABI void original_free(void*a,void*p,A size){if(a!=&state.allocator)bad++;release((void*)123,p,size);}
__declspec(dllexport) void vector(unsigned which,unsigned count,unsigned cap,unsigned ext,unsigned*out){
 bad=allocations=frees=0;capacity=cap;requested=count;external=ext;
 for(unsigned n=0;n<1024;n++)old[n]=fresh[n]=0xcccccccc;
 state.begin=(A)old;state.end=state.begin+8;state.capacity=state.begin+cap*16;
 state.inline_storage=ext?(A)state.inline_vectors:(A)old;
 if(which)nhl24_ptm_offset_resize(&state,count,(void*)123,allocate,release,(const char*)456);
 else {float zero[4]={0,0,0,0};state.end=state.begin;resize_original(&state.begin,count,zero);}
 out[0]=bad;out[1]=allocations;out[2]=frees;out[3]=(unsigned)(state.end-state.begin);
 out[4]=(unsigned)(state.capacity-state.begin);out[5]=state.begin==(A)fresh;
 for(unsigned n=0;n<1024;n++)out[6+n]=old[n];
 for(unsigned n=0;n<1024;n++)out[1030+n]=fresh[n];
}
__declspec(dllexport) void world(unsigned which,const float*root,const float*joint,float*out){
 if(which)nhl27_ptm_offset_world(out,root,joint);else world_original(out,root,joint);
}
__declspec(dllexport) void split(unsigned which,const float*p,float*out){
 float rotation[4]={1,2,3,4};
 for(unsigned n=0;n<24;n++)out[n]=-123;
 if(which)nhl27_ptm_offset_root(out,out+12,p,rotation);
 else {split_original(out,out+12,p);for(unsigned n=0;n<4;n++){out[n]=1;out[n+4]=rotation[n];}}
}
__declspec(dllexport) void sincos(unsigned which,float x,float*out){
 if(which)nhl27_ptm_sincos(x,out,out+1);else sincos_original(x,out,out+1);
}
static B tls[0x6290];static unsigned resets,mask;
ABI void reset(void*p){
 unsigned index=(unsigned)(((B*)p-tls-0x13c0)/0x13b0);
 if(index>=4||!tls[0x6280+index])bad++;resets++;mask|=1u<<index;
}
__declspec(dllexport) unsigned scopes(unsigned occupied){
 struct scope s;bad=resets=mask=0;
 for(unsigned n=0;n<4;n++)tls[0x6280+n]=(occupied>>n)&1;
 nhl24_ptm_offset_scope_begin(&s,tls,(void*)456,777);
 unsigned first=0;while(first<4&&(occupied&(1u<<first)))first++;
 if(s.arena!=(first==4?0:tls+0x13c0+first*0x13b0)||s.handle!=777||s.evaluation[0]!=s.arena||s.evaluation[1]!=(void*)456||s.evaluation[2]||s.flag)bad++;
 nhl24_ptm_offset_scope_end(&s,tls,reset);
 if(s.arena||resets!=(first<4)||mask!=(first<4?(1u<<first):0))bad++;
 for(unsigned n=0;n<4;n++)if(tls[0x6280+n]!=((occupied>>n)&1))bad++;
 return bad;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-offsets-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),*[str(root/'scripts/nhl27/runtime'/n) for n in
                    ['ptm_offsets.c','ptm_native.c','ptm_transition.c']],'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'));F=C.POINTER(C.c_float);U=C.POINTER(C.c_uint)
            lib.world.argtypes=[C.c_uint,F,F,F];lib.split.argtypes=[C.c_uint,F,F]
            lib.sincos.argtypes=[C.c_uint,C.c_float,F];lib.vector.argtypes=[C.c_uint]*4+[U]
            lib.scopes.argtypes=[C.c_uint];lib.scopes.restype=C.c_uint
            rng=random.Random(0x2805a0)
            for n in range(6000):
                a=(C.c_float*12)(*[rng.uniform(-100,100) for _ in range(12)])
                b=(C.c_float*12)(*[rng.uniform(-100,100) for _ in range(12)])
                x=(C.c_float*24)();y=(C.c_float*24)()
                if n%37==0:a[n%12]=[float('nan'),float('inf'),-float('inf'),-0.0][n%4]
                lib.world(0,a,b,x);lib.world(1,a,b,y);self.assertEqual(bytes(x),bytes(y),f'world {n}')
                lib.split(0,a,x);lib.split(1,a,y);self.assertEqual(bytes(x),bytes(y),f'split {n}')
                lib.sincos(0,a[0],x);lib.sincos(1,a[0],y);self.assertEqual(bytes(x),bytes(y),f'sincos {n}')
            for cap in [0,1,16,64,128]:
                for count in [0,1,15,16,17,63,64,65,127,128,129,255]:
                    for ext in [0,1]:
                        x=(C.c_uint*2054)();y=(C.c_uint*2054)()
                        lib.vector(0,count,cap,ext,x);lib.vector(1,count,cap,ext,y)
                        self.assertEqual(bytes(x),bytes(y),(count,cap,ext));self.assertEqual(y[0],0)
            for mask in range(16):self.assertEqual(lib.scopes(mask),0)
            import _ctypes
            _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
