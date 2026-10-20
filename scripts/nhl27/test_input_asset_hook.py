"""Execute the generic writer trampoline with a destructive guest-ABI callee."""
import ctypes
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest

from input_asset_hook import assembly, EXPECTED
from input_producer_hooks import REGISTERS
import input_frame_hook


def fixture(name, hooked, frame=False):
    saved = ('rbx', 'rbp', 'rsi', 'rdi', 'r12', 'r13', 'r14', 'r15')
    lines = ['.text', f'.globl {name}', f'{name}:']
    lines += [f'push {r}' for r in saved]
    # Buffer argument contains output state, a byte destination, and arguments.
    lines += ['sub rsp, 248', 'mov [rsp], rcx']
    lines += [f'vmovdqu [rsp+{32+16*i}], xmm{i+6}' for i in range(10)]
    lines += [f'mov {r}, {0x123400+i}' for i, r in enumerate(REGISTERS)]
    lines += ['mov rax, [rsp]', 'lea rbp, [rsp+240]', 'lea r13, [rax+640]',
              'lea r12, [rax+648]', 'mov r15, 127', 'lea rcx, [rax+656]',
              'mov [rbp-48], rcx']
    for i in range(16):
        lines += [f'vpcmpeqd ymm{i}, ymm{i}, ymm{i}', f'vpslld ymm{i}, ymm{i}, {i}']
    lines += ['cmp rsp, rsp', f'call {name}_enter', 'pushfq']
    lines += [f'push {r}' for r in REGISTERS]
    lines += ['mov rdx, [rsp+128]']
    for i in range(16):
        lines += [f'mov rax, [rsp+{8*i}]', f'mov [rdx+{8*i}], rax']
    lines += [f'vmovdqu [rdx+{128+32*i}], ymm{i}' for i in range(16)]
    lines += ['add rsp, 128']
    lines += [f'vmovdqu xmm{i+6}, [rsp+{32+16*i}]' for i in range(10)]
    lines += ['add rsp, 248'] + [f'pop {r}' for r in reversed(saved)] + ['ret']
    lines += [f'{name}_enter:']
    if not frame:
        lines += ['sub rsp, 8']
    if hooked:
        if frame:
            generated = input_frame_hook.assembly('fixture_callee', name+'_hook', 'fixture_base', 'fixture_original')
        else:
            generated = assembly('fixture_callee', name+'_hook', name+'_resume', 'fixture_base')
        lines += [f'jmp {name}_hook', generated]
    elif frame:
        lines += ['jmp fixture_original']
    else:
        lines += ['.byte ' + ','.join(hex(b) for b in EXPECTED)]
    lines += [f'{name}_resume:', 'ret']
    return '\n'.join(lines) + '\n'


class AssetWriterHookTests(unittest.TestCase):
    def test_predicate_arguments_full_ymm_flags_and_stack_survive_guest_call(self):
        self.execute_fixture(False)

    def test_local_frame_hook_preserves_original_call_and_full_machine_state(self):
        self.execute_fixture(True)

    def execute_fixture(self, frame):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name != 'nt' or not compiler.exists():
            self.skipTest('Windows LLVM fixture unavailable')
        scratch = Path(__file__).resolve().parents[2] / 'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        code = '.intel_syntax noprefix\n.text\nfixture_base:\n' + fixture('original', False, frame) + fixture('hooked', True, frame)
        if frame:
            code += '\nfixture_original:\nmov byte ptr [r13], 127\nret\n'
        code += '\n.text\nfixture_callee:\n'
        if frame:
            code += 'mov rcx, r13\n'
        code += 'lea rax, [rip+fixture_base]\ncmp rax, rdi\njne bad\n'
        # The exact slot, asset, evaluation context and value must survive the
        # trampoline's argument remapping. Return a marker through the buffer.
        if not frame:
            code += 'lea rax, [rcx+8]\ncmp rax, rsi\njne bad\n'
            code += 'lea rax, [rcx+16]\ncmp rax, rdx\njne bad\n'
            code += 'cmp r8, 127\njne bad\n'
        code += 'mov byte ptr [rcx+24], 1\njmp clobber\n'
        code += 'bad:\nmov byte ptr [rcx+24], 2\nclobber:\n'
        code += '\n'.join(f'vpxor ymm{i}, ymm{i}, ymm{i}' for i in range(16)) + '\n'
        code += '\n'.join(f'mov {r}, 0xbadcafe' for r in ('rax','rcx','rdx','rsi','rdi','r8','r9','r10','r11'))
        code += '\nstc\nret\n'
        with tempfile.TemporaryDirectory(prefix='asset-hook-', dir=scratch) as folder:
            path = Path(folder)
            (path/'fixture.s').write_text(code)
            subprocess.run([str(compiler), '-shared', '-fuse-ld=lld', '-nostdlib', '-mavx2',
                '-Wl,/noentry', '-Wl,/export:original', '-Wl,/export:hooked',
                str(path/'fixture.s'), '-o', str(path/'fixture.dll')], check=True, capture_output=True)
            library = ctypes.CDLL(str(path/'fixture.dll'))
            try:
                buffer = ctypes.create_string_buffer(672)
                for name in ('original', 'hooked'):
                    function = getattr(library, name)
                    function.argtypes = [ctypes.c_void_p]
                library.original(buffer)
                expected = bytes(buffer)
                ctypes.memset(buffer, 0, len(buffer))
                library.hooked(buffer)
                self.assertEqual(bytes(buffer)[:664], expected[:664])
                self.assertEqual(buffer[664], b'\x01')
                self.assertEqual(buffer[640], b'\x7f')
            finally:
                import _ctypes
                _ctypes.FreeLibrary(library._handle)


if __name__ == '__main__':
    unittest.main()
