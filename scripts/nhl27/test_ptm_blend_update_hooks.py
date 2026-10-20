"""Execute the query-site replacement with native SysV stack arguments."""
import ctypes as C
import os
import re
from pathlib import Path
import subprocess
import tempfile
import unittest
import ptm_blend_update_hooks as hook


class BlendUpdateHookTests(unittest.TestCase):
    def test_stage_guards_and_native_frame_arguments(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        asm=hook.assembly('fixture_base').split('.globl ptm_blend_query_dispatch')[0]+hook.stages('fixture_base')
        # The continuation bodies belong to the game; substitute recording
        # returns while retaining every instruction of the generated hooks.
        continuations={}
        for name,value in re.findall(r'\.set (\w+), (\d+)\n',asm):
            continuations[name]=int(value)
            asm=asm.replace(f'.set {name}, {value}\n','')
        # Substitute only the canary global's address, avoiding a 176 MB test
        # allocation. Its generated instruction is explicitly checked here.
        self.assertIn('lea r13,[rip+fixture_base]\nmov r13,[r13+0xa87c1f8]',asm)
        asm=asm.replace('lea r13,[rip+fixture_base]\nmov r13,[r13+0xa87c1f8]',
                        'mov r13,[rip+fixture_guard]')
        for name,address in continuations.items():
            asm+=f'{name}:\nmov eax,{address}\nret\n'
        asm+='''
.globl invoke_stage
invoke_stage:
push rbp
push r15
push r13
push r14
push r12
push rbx
sub rsp,8
mov r15,rdi
mov rbp,rsi
mov rbx,rdx
mov eax,1
cmp ecx,0
je invoke_speed
cmp ecx,1
je invoke_pre
cmp ecx,2
je invoke_finish
cmp ecx,4
je invoke_begin
call ptm_blend_late_dispatch
jmp invoke_done
invoke_speed:
call ptm_blend_speed_dispatch
jmp invoke_done
invoke_pre:
call ptm_blend_pre_child_dispatch
jmp invoke_done
invoke_finish:
call ptm_blend_finish_dispatch
jmp invoke_done
invoke_begin:
call ptm_blend_begin_dispatch
invoke_done:
add rsp,8
pop rbx
pop r12
pop r14
pop r13
pop r15
pop rbp
ret
'''
        fixture=r'''
#include "ptm_blend_storage.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
int _fltused=0;static A cookie;A *fixture_guard=&cookie;
static unsigned char blend[0x330],frame[0x400];
static float values[2]={0,12.5f};static unsigned calls,error;
void fixture_base(void){}
extern ABI unsigned invoke_stage(void*,void*,void*,unsigned,float);
ABI void nhl24_ptm_blend_begin_frame_guest(A base,void*b,void*evaluation,void*f,float delta){
 if(base!=(A)fixture_base||b!=blend||evaluation!=frame+sizeof(frame)||f!=evaluation||delta!=.5f)error|=16;
 *(void**)((unsigned char*)f-0x360)=values;
 calls=4;
}
ABI void nhl24_ptm_blend_speed_frame_guest(A base,void*b,void*f,float value){
 if(base!=(A)fixture_base||b!=blend||f!=frame+sizeof(frame)||value!=12.5f)error|=1;
 calls=1;
}
ABI void nhl24_ptm_blend_pre_child_guest(A base,void*b,void*f){
 if(base!=(A)fixture_base||b!=blend||f!=frame+sizeof(frame))error|=2;
 calls=2;
}
ABI void nhl24_ptm_blend_finish_frame_guest(A base,void*b){
 if(base!=(A)fixture_base||b!=blend)error|=4;
 calls=3;
}
__declspec(dllexport) unsigned run_stage(unsigned magic,unsigned enabled,unsigned stage,unsigned null){
 for(unsigned i=0;i<sizeof(blend);i++)blend[i]=0;
 struct ptm_blend_extension *e=(void*)(blend+0x2e0);
 e->magic=magic?PTM_BLEND_MAGIC:0;e->enabled=(panic_u8)enabled;
 unsigned active=magic&&enabled==1;calls=error=0;
 unsigned result=invoke_stage(blend,frame+sizeof(frame),null?0:values,stage,.5f);
 unsigned destination=0;
 switch(stage){
 case 0:destination=active?0x369a09:0x36a01e;break;
 case 1:destination=active?0x369cd0:0x369a10;break;
 case 2:destination=null?0x369d45:0x369d31;break;
 case 3:destination=active?0x369d2c:0x369d87;break;
 case 4:destination=active?0x369768:0x36955d;break;
 }
 if(calls!=(active?(stage<3?stage+1:stage==4?4:0):0)||result!=destination)error|=8;
 return error;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-blend-stages-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'hook.s').write_text(asm)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),str(p/'fixture.c'),
                str(p/'hook.s'),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run_stage.argtypes=[C.c_uint]*4
                for magic in range(2):
                    for enabled in range(3):
                        for stage in range(5):
                            for null in range(2 if stage==2 else 1):
                                self.assertEqual(lib.run_stage(magic,enabled,stage,null),0,(magic,enabled,stage,null))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)

    def test_query_dispatch_abi_and_fallback(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        # Exercise the query half independently; the begin half has native-frame
        # continuations and is verified separately from this call-site contract.
        asm='.intel_syntax noprefix\n.text\n'+hook.assembly('fixture_base').split('.globl ptm_blend_query_dispatch')[1].split('.globl ptm_blend_speed_dispatch')[0]
        asm=asm.replace('\nptm_blend_query_dispatch:', '\n.globl ptm_blend_query_dispatch\nptm_blend_query_dispatch:',1)
        asm+='''
.globl invoke
invoke:
push r15
sub rsp,32
mov r15,rdi
mov rax,[rsi]
mov rdi,rsi
mov esi,1
mov edx,2
mov ecx,3
mov r8d,4
mov r9d,5
mov qword ptr [rsp],6
mov qword ptr [rsp+8],7
mov qword ptr [rsp+16],8
call ptm_blend_query_dispatch
add rsp,32
pop r15
ret
'''
        setters=hook.setters()
        for address,raw,offset,field in hook.SETTERS:
            from ptm_evaluate_hooks import jump
            setters=setters.replace(jump('ptm_blend_set_'+field+'_original',address+len(bytes.fromhex(raw))),'ret\n')
        asm+=setters
        fixture=r'''
#include "ptm_blend_query.h"
typedef panic_address A;
#define ABI PTM_GAME_ABI
int _fltused=0;
static unsigned char blend[PTM_BLEND_EXTENDED_SIZE];
static A match,vt[28];static unsigned error,calls,handled;
void fixture_base(void){}
extern ABI void invoke(void*,void*,float,float,float);
extern ABI void ptm_blend_set_duration(void*,float);
extern ABI void ptm_blend_set_elapsed(void*,float);
ABI void original(void*m,void*c,void*f,void*s,void*ctx,void*input,
 unsigned a,unsigned b,unsigned c3,float r,float d,float scale){
 calls|=2;
 if(m!=&match||c!=(void*)1||f!=(void*)2||s!=(void*)3||ctx!=(void*)4||input!=(void*)5||
 a!=6||b!=7||c3!=8||r!=9.25f||d!=.5f||scale!=11.75f)error|=1;
}
ABI int nhl24_ptm_blend_query_guest(A base,void*p,const struct ptm_blend_query_arguments*a){
 calls|=1;
 if(base!=(A)fixture_base||p!=blend||a->current_map!=(void*)1||a->future_map!=(void*)2||
 a->slot!=(void*)3||a->contexts!=(void*)4||a->input_state!=(void*)5||a->first!=6||
 a->second!=7||a->third!=8||a->remaining!=9.25f||a->delta!=.5f||a->scale!=11.75f)error|=4;
 return handled;
}
__declspec(dllexport) unsigned run(unsigned magic,unsigned enabled,unsigned success){
 for(unsigned i=0;i<sizeof(blend);i++)blend[i]=0;
 struct ptm_blend_extension*e=(void*)(blend+PTM_BLEND_EXTENSION_OFFSET);
 e->magic=magic?PTM_BLEND_MAGIC:0;e->enabled=(panic_u8)enabled;
 vt[27]=(A)original;match=(A)vt;error=calls=0;handled=success;
 invoke(blend,&match,9.25f,.5f,11.75f);
 unsigned expected=magic&&enabled==1?(success?1:3):2;
 return error|(calls!=expected?8:0);
}
__declspec(dllexport) unsigned setter(unsigned magic,unsigned enabled,unsigned bits){
 for(unsigned i=0;i<sizeof(blend);i++)blend[i]=0;
 struct ptm_blend_extension*e=(void*)(blend+PTM_BLEND_EXTENSION_OFFSET);
 e->magic=magic?PTM_BLEND_MAGIC:0;e->enabled=(panic_u8)enabled;
 union{unsigned u;float f;}v={bits};
 ptm_blend_set_duration(blend,v.f);ptm_blend_set_elapsed(blend,v.f);
 if(*(unsigned*)(blend+0x2b0)!=bits||*(unsigned*)(blend+0x2b4)!=bits)return 1;
 unsigned expected=magic&&enabled==1?bits:0;
 return *(unsigned*)&e->clock.duration!=expected||*(unsigned*)&e->clock.elapsed!=expected;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-blend-update-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.c').write_text(fixture);(p/'hook.s').write_text(asm)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib',
                '-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),str(p/'fixture.c'),
                str(p/'hook.s'),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.run.argtypes=[C.c_uint]*3
                for magic in range(2):
                    for enabled in range(3):
                        for success in range(2):self.assertEqual(lib.run(magic,enabled,success),0,(magic,enabled,success))
                lib.setter.argtypes=[C.c_uint]*3
                for magic in range(2):
                    for enabled in range(3):
                        for bits in (0,0x80000000,0x7fc12345,0x7f800000,0xff800000,0x3f800000):
                            self.assertEqual(lib.setter(magic,enabled,bits),0,(magic,enabled,bits))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
