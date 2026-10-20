"""Execute both Boolean interfaces and the unrelated-asset native fallback."""
import ctypes
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from input_reader_hooks import SITES, assembly
from test_input_asset_hook import fixture


class ReaderHookTests(unittest.TestCase):
    def test_all_byte_values_interfaces_and_native_fallback(self):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name != 'nt' or not compiler.exists():
            self.skipTest('Windows LLVM unavailable')
        code = '.intel_syntax noprefix\n.text\nfixture_base:\n'
        exports = []
        for address,adjustment in SITES:
            for hooked in (False,True):
                name = f'case_{address:x}_{int(hooked)}'
                exports += ['-Wl,/export:'+name]
                text = fixture(name,False,True)
                text = text.replace('mov r15, 127', f'mov r15, 127\nlea rdi, [r13+{128+adjustment}]\nlea rsi, [r13+256]')
                if hooked:
                    text = text.replace('jmp fixture_original',f'jmp input_asset_read_{address:x}')
                code += text
        code += assembly('fixture_callee','fixture_base','fixture_original')
        code += '''
fixture_original:
    mov byte ptr [r13+28], 1
    movzx eax, byte ptr [r13+25]
    ret
fixture_callee:
    lea rax, [rip+fixture_base]
    cmp rax, rdi
    jne bad
    lea rax, [r13+128]
    cmp rax, rsi
    jne bad
    lea rax, [r13+256]
    cmp rax, rdx
    jne bad
    mov byte ptr [r13+24], 1
    jmp clobber
bad:
    mov byte ptr [r13+24], 2
clobber:
'''
        code += '\n'.join(f'vpxor ymm{i}, ymm{i}, ymm{i}' for i in range(16))+'\n'
        code += '\n'.join(f'mov {r}, 0xbadcafe' for r in ('rcx','rdx','rsi','rdi','r8','r9','r10','r11'))
        code += '\nmovzx eax, byte ptr [r13+25]\ncmp byte ptr [r13+27], 0\nje done\nmov eax, 256\ndone:\nstc\nret\n'
        root = Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='reader-hooks-',dir=root) as folder:
            path = Path(folder)
            (path/'fixture.s').write_text(code)
            subprocess.run([str(compiler),'-shared','-fuse-ld=lld','-nostdlib','-mavx2',
                '-Wl,/noentry',*exports,str(path/'fixture.s'),'-o',str(path/'fixture.dll')],
                check=True,capture_output=True)
            lib = ctypes.CDLL(str(path/'fixture.dll'))
            try:
                buffer = ctypes.create_string_buffer(1024)
                for address,_ in SITES:
                    for value in (0,1,127,255):
                        for fallback in (0,1):
                            expected = None
                            for hooked in (False,True):
                                ctypes.memset(buffer,0,len(buffer))
                                buffer[665] = bytes([value])
                                buffer[667] = bytes([fallback])
                                fn = getattr(lib,f'case_{address:x}_{int(hooked)}')
                                fn.argtypes = [ctypes.c_void_p]
                                fn(buffer)
                                if not hooked:
                                    expected = bytes(buffer)[:664]
                                else:
                                    self.assertEqual(bytes(buffer)[:664],expected)
                                    self.assertEqual(buffer[664],b'\x01')
                                    self.assertEqual(buffer[668],bytes([fallback]))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__ == '__main__':
    unittest.main()
