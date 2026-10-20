"""Execute update dispatch with real guest stack offsets and disabled fallbacks."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import ptm_evaluate_hooks as hook


class EvaluateHookTests(unittest.TestCase):
    def test_frame_arguments_guards_and_offsets(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        code=hook.assembly('fixture_base')
        for name,address,target in [('ptm_evaluate_resume',0x3125ba,'evaluated'),
                ('ptm_evaluate_original',hook.EVALUATE+8,'original'),
                ('ptm_offsets_resume',0x312601,'offsets_done'),
                ('ptm_offsets_original',hook.OFFSETS+8,'offsets_original')]:
            code=code.replace(f'.set {name}, {address}',f'.set {name}, {target}')
        code+='''
.globl dispatch
dispatch:
push rbx
push rsi
push rdi
push r12
sub rsp, 0x348
mov r12, rcx
mov [rsp+0x28], edx
mov qword ptr [rsp+0x30], 0x123456
mov qword ptr [rsp+0x50], 0x789abc
mov qword ptr [rsp+0x260], 0xabc123
mov rax, 0x234567
jmp ptm_evaluate_dispatch
evaluated:
cmp rax, 0x234567
jne failed
mov rax, [rsp+0x50]
jmp finish
original:
mov rax, rbx
jmp finish
failed:
xor eax,eax
finish:
add rsp, 0x348
pop r12
pop rdi
pop rsi
pop rbx
ret
.globl offsets
offsets:
push r12
push rsi
mov r12, rcx
jmp ptm_offsets_dispatch
offsets_done:
mov rax, r11
jmp offsets_return
offsets_original:
mov rax, 0xdead
offsets_return:
pop rsi
pop r12
ret
.globl fixture_base
fixture_base:
ret
'''
        c=r'''
#include "ptm_lifecycle.h"
extern void fixture_base(void);
__attribute__((sysv_abi)) void nhl24_ptm_match_evaluate_frame_guest(panic_address base,void*match,void*frame){
 unsigned char*f=frame;struct ptm_match_extension*e=(void*)((unsigned char*)match+0x1740);
 unsigned valid=base==(panic_address)&fixture_base&&e->magic==PTM_EXTENSION_MAGIC&&
  *(panic_address*)(f+0x30)==0x123456&&*(panic_address*)(f+0x50)==0x789abc&&
  *(panic_address*)(f+0x260)==0xabc123&&*(unsigned*)(f+0x28)==0x3e800000;
 *(panic_address*)(f+0x50)=valid?0xabcdef:0;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='ptm-update-hook-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            p=Path(folder);(p/'fixture.s').write_text(code);(p/'fixture.c').write_text(c)
            r=subprocess.run([str(compiler),'-O2','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',
                '-Wl,/export:dispatch','-Wl,/export:offsets','-I',str(root/'scripts/nhl27/runtime'),
                str(p/'fixture.s'),str(p/'fixture.c'),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if r.returncode:raise RuntimeError(r.stdout+r.stderr)
            lib=C.CDLL(str(p/'fixture.dll'))
            try:
                lib.dispatch.argtypes=[C.c_void_p,C.c_uint];lib.dispatch.restype=C.c_uint64
                lib.offsets.argtypes=[C.c_void_p];lib.offsets.restype=C.c_uint64
                match=C.create_string_buffer(0x17d0);state=C.create_string_buffer(0x4a0);rig=C.create_string_buffer(0x208)
                def q(buf,off,value):C.c_uint64.from_buffer(buf,off).value=value
                q(match,0x1730,0xcafe);q(state,0x30,C.addressof(rig))
                q(state,0x40,0x1000);q(state,0x48,0x1040)
                for magic in (0,0x37544d5054524f50):
                    for enabled in (0,1,2):
                        for present in (0,1):
                            q(match,0x1740,magic);match[0x1751]=enabled
                            q(match,0x1748,C.addressof(state) if present else 0)
                            active=bool(magic and enabled==1 and present)
                            self.assertEqual(lib.dispatch(match,0x3e800000),0xabcdef if magic and enabled==1 else 0xcafe)
                            for count in (0,3,4,5):
                                C.c_uint.from_buffer(rig,0x200).value=count
                                expected=C.addressof(state)+0x40 if count==4 else 0
                                self.assertEqual(lib.offsets(match),expected if active else 0xdead)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
