"""Differential automatic transition creation and end-of-clip scheduling."""
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


class AutoTests(unittest.TestCase):
    def test_source_constructor_and_scheduling(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():self.skipTest('Private source and LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x27fb10,0x27fda3,
            {0x17ffc60:'pose_retain',0xa3f97b0:'property_copy',0x8bbec0:'get_allocator',
             0x8c1f30:'allocator_init',0x8c1fb0:'allocator_copy',0x8c1f70:'allocator_name',
             0x2804d0:'capture_original',0xa3f9e90:'property_release',0x17fdc10:'pose_release',0xbfabea0:'fail'},
            {0xec2d9f0:'guard_pointer',0xf298910:'invalid',0xdc8af0b:'label',0xd9970b6:'label',0xddc9a1e:'label'},
            indirect_calls={0x27fb8b:'ff9000010000',0x27fbb6:'41ff5218',0x27fd3b:'ff5028',0x27fd63:'ff5028'})
        code+=assembly(image,'due_original',0x27fa30,0x27fb0a,
            {0x27fdb0:'unexpected',0x27fb10:'record_due'}, {})
        fixture=r'''
#include "ptm_native.h"
#define ABI PTM_GAME_ABI
typedef panic_address A;typedef unsigned char B;
extern ABI void original(void*,void*,void*);
extern ABI A due_original(void*,A,void*,void*,float);
static B source_match[0xbbc8],native_match[0x17d0],source_config[0x120],native_config[0x100];
static B source_selection[0x20],native_selection[0x20],source_db[0xd0],native_db[0xd0];
static B source_clip[0xa0],native_clip[0xa0],frames[0x10],pose[0x28],properties[0x20],controller[0x20];
static A asset_vt[33],asset[1],controller_vt[6],allocator_vt[4],allocator[1],clips[1];
static void *evaluation[2];static struct ptm_transition state,*slot;
static unsigned mode,which,bad,due,captured;static B *match,*config,*selection,*clip;
A guard=1,*guard_pointer=&guard,invalid=777;char label[4];int _fltused=0;
ABI void fail(void){__builtin_trap();}
ABI A unexpected(void){__builtin_trap();}
ABI void record_due(void*m,void*e,void*s){due++;}
ABI void pose_retain(void*p){++*(int*)((B*)p+0x18);}
ABI void pose_release(void*p){--*(int*)((B*)p+0x18);}
ABI void property_copy(void*out,void*in,unsigned a,unsigned b){
 void **o=out;*o=*(void**)in;o[1]=((void**)in)[1];++*(int*)((B*)*o+8);
}
ABI void property_release(void*out){--*(int*)((B*)*(void**)out+8);}
ABI void factory(void**out,void*a,void**snapshot){
 if(a!=asset||snapshot[0]!=pose||snapshot[1]!=properties||snapshot[2]!=(void*)456||
    snapshot[3]||snapshot[4]!=(void*)789||*(int*)(pose+0x18)!=6||*(int*)(properties+8)!=8)bad++;
 *out=controller;++*(int*)(controller+8);
}
ABI void *alloc_original(void*a,A bytes,const char*n,unsigned flags,unsigned align,unsigned extra){
 if(a!=allocator||bytes!=0x4a0||n!=label||flags!=1||align!=16||extra)bad++;
 if(mode&2){*(float*)(config+(which?0xd8:0x108))=0.75f;*(unsigned*)(config+(which?0xdc:0x10c))=3;}
 return &state;
}
ABI void *get_allocator(void){return allocator;}
ABI void allocator_init(void*out,void*n){*(A*)out=456;}
ABI void allocator_copy(void*out,void*in){*(A*)out=*(A*)in;}
ABI void allocator_name(void*out,void*n){*(A*)out=456;}
ABI void destroy(void*p){bad++;}
ABI void capture_original(struct ptm_transition*s,void*e){
 if(s!=&state||e!=evaluation||*(int*)(controller+8)!=7)bad++;
 captured++;
}
static struct ptm_transition *allocate(void*c,A n){return alloc_original(allocator,n,label,1,16,0);}
static void destroy_adapter(void*c,void*p){destroy(p);}
static void capture(void*c,struct ptm_transition*s,void*e){capture_original(s,e);}
static const struct ptm_creation_services services={0,allocate,destroy_adapter,capture};
__declspec(dllexport) void run(unsigned w,unsigned m,const float*v,unsigned*out){
 which=w;mode=m;bad=captured=due=0;slot=0;
 match=w?native_match:source_match;config=w?native_config:source_config;
 selection=w?native_selection:source_selection;clip=w?native_clip:source_clip;
 B*db=w?native_db:source_db;
 for(unsigned n=0;n<sizeof(state);n++)((B*)&state)[n]=0xcc;
 *(void**)(match+(w?0x11f0:0x86b0))=config;
 *(void**)(config+(w?0xb8:0xe8))=asset;
 *(float*)(config+(w?0xd8:0x108))=v[0];*(unsigned*)(config+(w?0xdc:0x10c))=m%4;
 *(void**)(match+0x30)=pose;*(void**)(match+0x38)=properties;*(A*)(match+0x40)=456;
 *(int*)(pose+0x18)=5;*(int*)(properties+8)=7;*(int*)(controller+8)=5;
 *(void**)controller=controller_vt;controller_vt[w?4:5]=(A)destroy;
 *(float*)(clip+(w?0x90:0x80))=v[1];*(void**)(clip+(w?0x18:0x38))=frames;
 *(float*)(frames+4)=v[2];*(float*)(selection+(w?0x1c:0x10))=v[3];
 *(unsigned*)selection=w?0:m&1;if(w)selection[0x18]=m&1;
 *(void**)(selection+(w?0x10:8))=db;*(void**)(db+(w?0xb0:0xc8))=clips;clips[0]=(A)clip;
 asset[0]=(A)asset_vt;asset_vt[w?23:32]=(A)factory;
 allocator[0]=(A)allocator_vt;allocator_vt[3]=(A)alloc_original;
 evaluation[0]=(void*)123;evaluation[1]=(void*)789;
 *(A*)(source_match+0xbbc0)=0;
 if(w)due=nhl24_ptm_auto_due(match,selection,v[4]);
 else due_original(match,123,selection,evaluation,v[4]);
 unsigned scheduled=due;
 if(w)nhl24_ptm_auto_create(match,evaluation,selection,&slot,invalid,456,&services);
 else original(match,evaluation,selection);
 out[0]=bad;out[1]=captured;out[2]=*(unsigned*)(pose+0x18);out[3]=*(unsigned*)(properties+8);
 out[4]=*(unsigned*)(controller+8);out[5]=scheduled;
 state.match=(void*)1;state.controller=(void*)2;state.clip=(void*)3;
 state.trajectory=(void*)4;state.match_state=(void*)5;
 state.begin-=(A)state.inline_vectors;state.end-=(A)state.inline_vectors;
 state.capacity-=(A)state.inline_vectors;state.inline_storage-=(A)state.inline_vectors;
 for(unsigned n=0;n<sizeof(state)/4;n++)out[6+n]=((unsigned*)&state)[n];
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-auto-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build',ignore_cleanup_errors=True) as folder:
            p=Path(folder);(p/'oracle.s').write_text(code);(p/'fixture.c').write_text(fixture)
            r=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),str(p/'fixture.c'),str(p/'oracle.s'),
                str(root/'scripts/nhl27/runtime/ptm_native.c'),str(root/'scripts/nhl27/runtime/ptm_transition.c'),
                '-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint,C.c_uint,C.POINTER(C.c_float),C.POINTER(C.c_uint)]
                rng=random.Random(0x27fb10)
                for n in range(4096):
                    vals=[rng.uniform(-3,3) for _ in range(5)]
                    if n%29==0:vals[n%5]=[float('nan'),float('inf'),-float('inf'),-0.0][n%4]
                    v=(C.c_float*5)(*vals);a=(C.c_uint*302)();b=(C.c_uint*302)()
                    lib.run(0,n%8,v,a);lib.run(1,n%8,v,b)
                    self.assertEqual(bytes(a),bytes(b),f'case {n}: {vals}')
                    self.assertEqual(list(b[:5]),[0,1,5,7,6])
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
