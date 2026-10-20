"""Execute generated guest hooks and compare their machine state with native code."""
import ctypes
import json
import itertools
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest

from input_producer_hooks import REGISTERS, trampoline, selected_sites
from input_join_hooks import qualified
from research import SelfImage


def wrapper(site, name, hook, valid_class=True):
    # Preserve Win64 nonvolatile registers and XMM6..15 around a guest fixture.
    saved = ('rbx', 'rbp', 'rsi', 'rdi', 'r12', 'r13', 'r14', 'r15')
    kind = site.get('predicate_kind')
    frame = 408 if kind else 200
    lines = ['.text', f'.globl {name}', f'{name}:']
    lines += [f'push {r}' for r in saved]
    lines += [f'sub rsp, {frame}', 'mov [rsp], rcx', 'mov [rsp+8], rdx', 'mov [rsp+16], r8']
    lines += [f'movdqu [rsp+{32+16*i}], xmm{i+6}' for i in range(10)]
    lines += [f'mov {r}, {0x123400+i}' for i, r in enumerate(REGISTERS)]
    lines += [f'mov {site["record_register"]}, [rsp]']
    if kind:
        lines += ['lea rbp, [rsp+384]', 'mov rax, [rsp+16]']
        if kind == 'clear_byte_human_actor':
            lines += ['mov [rbp-136], rax']
        elif kind == 'byte_or_stack_flag':
            lines += ['mov [rbp-160], eax', 'mov qword ptr [rbp-120], 0x12340b']
        lines += [f'mov rax, {0x123400}', f'mov {site["record_register"]}, [rsp]']
    if site.get('runtime_class_guard'):
        lines += ['lea rax, [rip+fixture_base]',
                  f'add rax, {0xa426580 if valid_class else 0xa426581}', 'mov rcx, [rsp]', 'mov [rcx], rax',
                  f'mov rax, {0x123400}', f'mov rcx, {0x123401}',
                  f'mov {site["record_register"]}, [rsp]']
    if site.get('producer_mode') == 'predicated_store':
        lines += [f'mov {site["pointer_register"]}, [{site["record_register"]}+{site["binding_offset"]}]']
    for i in range(16):
        lines += [f'pcmpeqd xmm{i}, xmm{i}', f'pslld xmm{i}, {i}']
    lines += ['cmp dword ptr [rsp+16], 0' if kind == 'entry_zf_clear' else 'cmp rsp, rsp',
              f'call {name}_entry', 'pushfq']
    lines += [f'push {r}' for r in REGISTERS]
    lines += ['mov rdx, [rsp+136]']
    for i in range(16):
        lines += [f'mov rax, [rsp+{i*8}]', f'mov [rdx+{i*8}], rax']
    lines += [f'movdqu [rdx+{128+16*i}], xmm{i}' for i in range(16)]
    lines += ['add rsp, 128']
    lines += [f'movdqu xmm{i+6}, [rsp+{32+16*i}]' for i in range(10)]
    lines += [f'add rsp, {frame}']
    lines += [f'pop {r}' for r in reversed(saved)]
    lines += ['ret']
    if hook:
        adjusted = dict(site)
        if 'null_branch_address' in adjusted:
            adjusted['null_branch_address'] = name+'_done'
        lines += [trampoline(adjusted, name + '_entry', name + '_resume', base_symbol='fixture_base')]
    else:
        lines += [name + '_entry:', '.byte ' + ','.join(hex(b) for b in bytes.fromhex(site.get('replay_bytes',site['hook_bytes'])))]
        if 'null_branch_address' in site:
            lines += [f'je {name}_done']
    if site.get('producer_mode') == 'predicated_store':
        lines += [name+'_resume:']
        if 'null_branch_address' in site:
            lines += [f'mov byte ptr [{site["pointer_register"]}], {site["value"]}']
        return '\n'.join(lines + [name + '_done:', 'ret']) + '\n'
    p = site['pointer_register']
    lines += [name + '_resume:']
    if kind == 'entry_zf_clear':
        lines += [f'je {name}_done']
    lines += [f'test {p}, {p}', f'je {name}_done']
    if kind == 'clear_byte_human_actor':
        lines += [f'cmp byte ptr [{p}], 0', f'jne {name}_done', 'mov rcx, [rbp-136]',
                  'cmp byte ptr [rcx+0x23c], 0', f'js {name}_done']
    elif kind == 'byte_or_stack_flag':
        lines += ['mov ecx, [rbp-160]', f'or cl, byte ptr [{p}]', 'mov r13, [rbp-120]', f'je {name}_done']
    if site.get('producer_mode') == 'changed_constant':
        lines += [f'cmp byte ptr [{p}], {site["value"]}', f'je {name}_done']
    lines += [f'mov byte ptr [{p}], {site["value"]}', name + '_done:', 'ret']
    return '\n'.join(lines) + '\n'


class ProducerHookTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name != 'nt' or not compiler.exists():
            raise unittest.SkipTest('Windows Clang fixture toolchain unavailable')
        root = Path(__file__).resolve().parents[2]
        report = root / 'local/nhl27-eboot/compatibility/deke-trial/target-input-producer-sites.json'
        cls.sites = selected_sites(json.loads(report.read_text())['sites']
            + qualified(SelfImage(Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))))
        scratch = report.parent / 'runtime-build'
        scratch.mkdir(parents=True, exist_ok=True)
        cls.directory = tempfile.TemporaryDirectory(prefix='producer-hooks-', dir=scratch)
        path = Path(cls.directory.name)
        code = '.intel_syntax noprefix\n.text\nfixture_base:\n'
        exports = []
        for i, site in enumerate(cls.sites):
            for kind in ('original', 'hook'):
                name = f'{kind}_{i}'
                code += wrapper(site, name, kind == 'hook')
                exports.append('/export:' + name)
            if site.get('runtime_class_guard'):
                name = f'wrong_{i}'
                code += wrapper(site, name, True, valid_class=False)
                exports.append('/export:' + name)
        (path / 'hooks.s').write_text(code)
        command = [str(compiler), '-shared', '-fuse-ld=lld', '-nostdlib', '-O2',
                   '-ffreestanding', '-fno-stack-protector', '-Wl,/noentry',
                   str(path / 'hooks.s'), str(root / 'scripts/nhl27/runtime/input_bridge.c'),
                   '-o', str(path / 'fixture.dll')] + ['-Wl,' + e for e in exports]
        compiled = subprocess.run(command, capture_output=True, text=True)
        if compiled.returncode:
            raise RuntimeError(compiled.stderr[-4000:])
        cls.library = ctypes.CDLL(str(path / 'fixture.dll'))
        cls.library.nhl24_input_initialize_extension.argtypes = [ctypes.c_void_p]

    @classmethod
    def tearDownClass(cls):
        import _ctypes
        _ctypes.FreeLibrary(cls.library._handle)
        cls.directory.cleanup()

    def test_native_register_vector_flag_and_store_equivalence(self):
        self.assertTrue(self.sites)
        for i, site in enumerate(self.sites):
            for missing, initially_equal, predicate_pass in itertools.product((False, True), (False, True),
                    (False, True) if site.get('predicate_kind') else (True,)):
                if missing and site.get('producer_mode') == 'predicated_store' and 'null_branch_address' not in site:
                    continue  # Original store-site entry requires a non-null pointer.
                with self.subTest(load=hex(site['load']), missing=missing, initially_equal=initially_equal):
                    record = ctypes.create_string_buffer(0x21a0)
                    initial = site['value'] if initially_equal else 1 - site['value']
                    native = ctypes.c_ubyte(initial)
                    actor = ctypes.create_string_buffer(0x240)
                    actor[0x23c] = b'\0' if predicate_pass else b'\xff'
                    predicate = ctypes.addressof(actor) if site.get('predicate_kind') == 'clear_byte_human_actor' else int(predicate_pass)
                    struct.pack_into('<Q', record, site['binding_offset'],
                                     0 if missing else ctypes.addressof(native))
                    self.library.nhl24_input_initialize_extension(record)
                    struct.pack_into('<I', record, 0x1580 + 0xc00, 73)
                    slot = 0x1580 + site['action'] * 8
                    initial_shadow = 1-site['value']
                    record[slot] = bytes([initial_shadow])
                    selected = True
                    if site.get('predicate_kind') == 'entry_zf_clear':
                        selected = predicate_pass
                    elif site.get('predicate_kind') == 'clear_byte_human_actor':
                        selected = initial == 0 and predicate_pass
                    elif site.get('predicate_kind') == 'byte_or_stack_flag':
                        selected = bool(initial or predicate_pass)
                    before = bytes(record)
                    captures = []
                    for kind in ('original', 'hook'):
                        native.value = initial
                        result = ctypes.create_string_buffer(384)
                        fn = getattr(self.library, f'{kind}_{i}')
                        fn.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t]
                        fn(record, result, predicate)
                        if kind == 'original' and site.get('runtime_class_guard'):
                            before = bytes(record)
                        captures.append(bytes(result))
                        self.assertEqual(native.value, initial if missing or not selected else site['value'])
                    self.assertEqual(captures[0], captures[1])
                    self.assertEqual(bytes(record)[:0x1580], before[:0x1580])
                    slot = 0x1580 + site['action'] * 8
                    wrote = not missing and selected and not (initially_equal and
                            site.get('producer_mode') == 'changed_constant')
                    self.assertEqual(record[slot], bytes([site['value'] if wrote else initial_shadow]))
                    self.assertEqual(struct.unpack_from('<I', record, slot + 4)[0],
                                     73 if wrote and site['value'] != initial_shadow else 0xffffffff)

    def test_wrong_class_does_not_touch_extension_or_suppress_original_store(self):
        guarded = [(i, s) for i, s in enumerate(self.sites) if s.get('runtime_class_guard')]
        self.assertTrue(guarded)
        for i, site in guarded:
            record = ctypes.create_string_buffer(0x21a0)
            ctypes.memset(ctypes.addressof(record)+0x1580, 0x6b, 0xc10)
            native = ctypes.c_ubyte(1-site['value'])
            struct.pack_into('<Q', record, site['binding_offset'], ctypes.addressof(native))
            captured = ctypes.create_string_buffer(384)
            function = getattr(self.library, f'wrong_{i}')
            actor = ctypes.create_string_buffer(0x240)
            predicate = ctypes.addressof(actor) if site.get('predicate_kind') == 'clear_byte_human_actor' else 1
            function.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t]
            function(record, captured, predicate)
            self.assertEqual(native.value, site['value'])
            self.assertEqual(bytes(record)[0x1580:0x2190], bytes([0x6b])*0xc10)


if __name__ == '__main__':
    unittest.main()
