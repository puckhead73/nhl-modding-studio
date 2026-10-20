"""Compare full-reset trampoline state with the displaced original instruction."""
import ctypes
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import input_reset_hook as hook
from test_input_asset_hook import fixture


class ResetHookTests(unittest.TestCase):
    def test_full_machine_state_and_replayed_loop_initializer(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        code='.intel_syntax noprefix\n.text\nfixture_base:\n'
        code+=fixture('original',False,True)
        code+=fixture('hooked',False,True).replace('jmp fixture_original',f'jmp {hook.NAME}')
        code+=hook.assembly('fixture_callee','fixture_base','fixture_return')
        code+='fixture_original:\n.byte '+','.join(hex(b) for b in hook.EXPECTED)+'\nfixture_return:\nret\n'
        code+='''fixture_callee:
    lea rax, [rip+fixture_base]
    cmp rax, rdi
    jne bad
    mov byte ptr [r13+24], 1
    jmp clobber
bad:
    mov byte ptr [r13+24], 2
clobber:
'''
        code+='\n'.join(f'vpxor ymm{i}, ymm{i}, ymm{i}' for i in range(16))+'\n'
        code+='\n'.join(f'mov {r}, 0xbadcafe' for r in ('rax','rcx','rdx','rsi','rdi','r8','r9','r10','r11'))
        code+='\nstc\nret\n'
        root=Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='reset-hook-',dir=root) as folder:
            p=Path(folder)
            (p/'fixture.s').write_text(code)
            result=subprocess.run([str(compiler),'-shared','-fuse-ld=lld','-nostdlib','-mavx2',
                '-Wl,/noentry','-Wl,/export:original','-Wl,/export:hooked',
                str(p/'fixture.s'),'-o',str(p/'fixture.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=ctypes.CDLL(str(p/'fixture.dll'))
            try:
                original=ctypes.create_string_buffer(672)
                hooked=ctypes.create_string_buffer(672)
                lib.original.argtypes=lib.hooked.argtypes=[ctypes.c_void_p]
                # Use the same buffer: saved argument register values include it.
                lib.original(original)
                expected=bytes(original)
                ctypes.memset(original,0,len(original))
                lib.hooked(original)
                self.assertEqual(bytes(original)[:664],expected[:664])
                self.assertEqual(bytes(original)[664],1)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
