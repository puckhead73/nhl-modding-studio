"""Execute ownership patches, including displaced compares and inline clears."""
import ctypes
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from input_owner_hooks import SITES, assembly
from test_input_asset_hook import fixture


class OwnerHookTests(unittest.TestCase):
    def test_actor_arguments_store_order_and_full_machine_state(self):
        self.execute_fixture(7)

    def test_already_unassigned_actor_still_resets_before_index_load(self):
        self.execute_fixture(255)

    def execute_fixture(self, actor_index):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name != 'nt' or not compiler.exists():
            self.skipTest('Windows LLVM unavailable')
        code = '.intel_syntax noprefix\n.text\nfixture_base:\n'
        exports = []
        for address, raw, actor, index, first in SITES:
            for hooked in (False,True):
                name = f'case_{address:x}_{int(hooked)}'
                exports += ['-Wl,/export:'+name]
                text = fixture(name,False,True)
                text = text.replace('mov r15, 127',
                    f'lea {actor}, [r13+128]\nmov dword ptr [{actor}+32], 1\n'
                    f'mov byte ptr [{actor}+572], {actor_index}\nmov byte ptr [r13+25], {255 if "-1" in index else 9}\n'
                    f'mov byte ptr [r13+26], {255 if first else actor_index}')
                if address == 0x2a2e3d9:
                    text = text.replace('cmp rsp, rsp',
                        'lea rbx, [r13+768]\nmov byte ptr [rbx+4], 9\ncmp rsp, rsp')
                jump = f'jmp input_owner_{address:x}' if hooked else '.byte '+','.join(hex(b) for b in bytes.fromhex(raw))+'\nret'
                code += text.replace('jmp fixture_original',jump)
        code += assembly('fixture_callee','fixture_base','fixture_resume')
        code += '''
fixture_resume:
    ret
fixture_callee:
    lea rax, [rip+fixture_base]
    cmp rdi, rax
    jne bad
    lea rax, [r13+128]
    cmp rsi, rax
    jne bad
    cmp dl, byte ptr [r13+25]
    jne bad
    cmp edx, -1
    jne good
    mov al, byte ptr [r13+26]
    cmp byte ptr [rsi+572], al
    jne bad
good:
    mov byte ptr [r13+24], 1
    jmp clobber
bad:
    mov byte ptr [r13+24], 2
clobber:
'''
        code += '\n'.join(f'vpxor ymm{i}, ymm{i}, ymm{i}' for i in range(16))+'\n'
        code += '\n'.join(f'mov {r}, 0xbadcafe' for r in ('rax','rcx','rdx','rsi','rdi','r8','r9','r10','r11'))+'\nstc\nret\n'
        root = Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='owner-hooks-',dir=root) as folder:
            path = Path(folder)
            (path/'fixture.s').write_text(code)
            subprocess.run([str(compiler),'-shared','-fuse-ld=lld','-nostdlib','-mavx2',
                '-Wl,/noentry',*exports,str(path/'fixture.s'),'-o',str(path/'fixture.dll')],
                check=True,capture_output=True)
            lib = ctypes.CDLL(str(path/'fixture.dll'))
            try:
                buffer = ctypes.create_string_buffer(2048)
                for address,*_ in SITES:
                    expected = None
                    for hooked in (False,True):
                        ctypes.memset(buffer,0,len(buffer))
                        fn = getattr(lib,f'case_{address:x}_{int(hooked)}')
                        fn.argtypes = [ctypes.c_void_p]
                        fn(buffer)
                        if not hooked:
                            expected = bytes(buffer)
                        else:
                            self.assertEqual(bytes(buffer)[:664],expected[:664])
                            self.assertEqual(bytes(buffer)[665:],expected[665:])
                            self.assertEqual(buffer[664],b'\x01')
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__ == '__main__':
    unittest.main()
