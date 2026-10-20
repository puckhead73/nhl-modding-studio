"""Execute every post-sample hook with both native teams and ABI clobbers."""
import ctypes
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from input_human_hook import SITES, assembly
from test_input_asset_hook import fixture


class HumanSampleHookTests(unittest.TestCase):
    def test_all_twelve_indices_run_after_sample_and_preserve_machine_state(self):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name != 'nt' or not compiler.exists():
            self.skipTest('Windows LLVM fixture unavailable')
        code = '.intel_syntax noprefix\n.text\nfixture_base:\n'
        exports = []
        for team in range(2):
            for address, slot in SITES:
                for hooked in (False, True):
                    name = f'case_{team}_{slot}_{int(hooked)}'
                    exports += ['-Wl,/export:'+name]
                    wrapper = fixture(name, False, True)
                    wrapper = wrapper.replace('sub rsp, 248', 'sub rsp, 520').replace('add rsp, 248', 'add rsp, 520')
                    wrapper = wrapper.replace('lea rbp, [rsp+240]', 'lea rbp, [rsp+512]')
                    wrapper = wrapper.replace('mov r15, 127', f'mov r15, 127\nmov rdi, r13\nmov byte ptr [rbp-112], {1-team}')
                    if hooked:
                        wrapper = wrapper.replace('jmp fixture_original', f'jmp input_human_sample_{address:x}')
                    code += wrapper
        code += assembly('fixture_callee', 'fixture_base', 'fixture_original')
        code += '\nfixture_original:\nmov byte ptr [rdi], 127\nmov rdi, 0x7654\nstc\nret\n'
        code += '''
fixture_callee:
    lea rax, [rip+fixture_base]
    cmp rax, rdi
    jne bad
    cmp rdx, r13
    jne bad
    cmp byte ptr [rdx], 127
    jne bad
    mov byte ptr [r13+24], 1
    mov byte ptr [r13+25], sil
    jmp clobber
bad:
    mov byte ptr [r13+24], 2
clobber:
'''
        code += '\n'.join(f'vpxor ymm{i}, ymm{i}, ymm{i}' for i in range(16))+'\n'
        code += '\n'.join(f'mov {r}, 0xbadcafe' for r in ('rax','rcx','rdx','rsi','rdi','r8','r9','r10','r11'))
        code += '\nclc\nret\n'
        root = Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='human-sample-', dir=root) as folder:
            path = Path(folder)
            (path/'fixture.s').write_text(code)
            subprocess.run([str(compiler), '-shared','-fuse-ld=lld','-nostdlib','-mavx2',
                '-Wl,/noentry', *exports, str(path/'fixture.s'), '-o',str(path/'fixture.dll')],
                check=True, capture_output=True)
            library = ctypes.CDLL(str(path/'fixture.dll'))
            try:
                buffer = ctypes.create_string_buffer(672)
                for team in range(2):
                    for _, slot in SITES:
                        expected = None
                        for hooked in (False, True):
                            ctypes.memset(buffer,0,len(buffer))
                            fn = getattr(library,f'case_{team}_{slot}_{int(hooked)}')
                            fn.argtypes = [ctypes.c_void_p]
                            fn(buffer)
                            if not hooked:
                                expected = bytes(buffer)[:664]
                            else:
                                self.assertEqual(bytes(buffer)[:664],expected)
                                self.assertEqual(buffer[664],b'\x01')
                                self.assertEqual(buffer[665],bytes([team*6+slot]))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(library._handle)


if __name__ == '__main__':
    unittest.main()
