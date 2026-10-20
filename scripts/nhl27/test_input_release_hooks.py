"""Execute release wrappers with destructive callees and negative actor indices."""
import ctypes
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import input_release_hooks as hooks
from test_input_asset_hook import fixture


class ReleaseHooksTests(unittest.TestCase):
    def test_autoselect_reset_is_the_possession_acquisition_branch(self):
        from deke_runtime_evidence import checked_image,SOURCE_HASH,TARGET_HASH
        from research import ElfImage,SelfImage
        import struct
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if not source.exists() or not target.exists():self.skipTest('Private executables required')
        s=checked_image(ElfImage,source,SOURCE_HASH);t=checked_image(SelfImage,target,TARGET_HASH)
        def call(image,address,destination):
            self.assertEqual(image.read(address,5),b'\xe8'+struct.pack('<i',destination-address-5))
        call(s,0x46db4fe,0x3e7bf0)
        call(s,0x46db506,0x46d6590)
        call(s,0x46db525,0x46d7a90)
        call(t,0x281a1e0,hooks.ORIGINAL)
        call(t,0x281a205,0x2a2e2e0)
        self.assertEqual(t.read(0x281a1eb,26),bytes.fromhex(
            '6641c7442405ffff41c78424840000000000000041c644244001'))
        # Tick-1 after assignment identifies this branch, unlike similar
        # assignments in 2a94690. Native controller last-tick is +dc.
        self.assertEqual(t.read(0x281a20a,16),bytes.fromhex('8b05f4b72209ffc841898424dc000000'))
        self.assertEqual(hooks.AUTOSELECT_RESET_SITES,(0x281a1e0,))

    def test_special_state_reset_reaches_the_hooked_common_loop(self):
        from deke_runtime_evidence import checked_image,SOURCE_HASH,TARGET_HASH
        from research import ElfImage,SelfImage
        import struct
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if not source.exists() or not target.exists():self.skipTest('Private executables required')
        s=checked_image(ElfImage,source,SOURCE_HASH);t=checked_image(SelfImage,target,TARGET_HASH)
        def call(image,address,destination):
            self.assertEqual(image.read(address,5),b'\xe8'+struct.pack('<i',destination-address-5))
        # Both surrounding state machines dispatch modes 3/4 through this reset.
        self.assertEqual(s.read(0x47bbfe2,7),bytes.fromhex('418d46fd83f802'))
        self.assertEqual(t.read(0x2b9b7d1,8),bytes.fromhex('418d4424fd83f802'))
        call(s,0x47bbff2,0x46f88c0)
        call(s,0x46f88e1,0x47ba2d0)
        self.assertEqual(s.read(0x47ba2d0,16),bytes.fromhex('893766c747040100c74708ffffffffc3'))
        self.assertEqual(t.read(0x2b9b7e2,20),bytes.fromhex(
            'c740100200000066c740140100c74018ffffffff'))
        # Native selected-actor equality branches to the shared loop call.
        self.assertEqual(t.read(0x2b9b80c,7),bytes.fromhex('4839d87568eb61'))
        call(t,0x2b9b874,0x2a33e60)
        for site in hooks.RELEASE_SITES:call(t,site,hooks.ORIGINAL)

    def test_reset_precedes_original_with_null_and_unassigned_actors(self):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name != 'nt' or not compiler.exists():
            self.skipTest('Windows LLVM unavailable')
        code = '.intel_syntax noprefix\n.text\nfixture_base:\n'
        exports = []
        for va in hooks.SITES:
            for null in (False, True):
                name = f'case_{va:x}_{int(null)}'
                exports.append('-Wl,/export:'+name)
                body = fixture(name, False, True)
                body = body.replace('mov r15, 127',
                    'mov byte ptr [r13+572], 255\n' +
                    ('xor edi, edi' if null else 'mov rdi, r13') + '\nmov [r13+32], rdi')
                code += body.replace('jmp fixture_original', f'jmp input_release_{va:x}')
        code += hooks.assembly('fixture_callee','fixture_base','fixture_original')
        code += '''
fixture_callee:
    lea rax, [rip+fixture_base]
    cmp rdi, rax
    jne bad
    cmp rsi, [r13+32]
    jne bad
    cmp edx, -1
    jne bad
    mov byte ptr [r13+24], 1
    jmp clobber
bad:
    mov byte ptr [r13+24], 2
clobber:
'''
        code += '\n'.join(f'vpxor ymm{i}, ymm{i}, ymm{i}' for i in range(16))+'\n'
        code += '\n'.join(f'mov {r}, 0xbadcafe' for r in ('rax','rcx','rdx','rsi','rdi','r8','r9','r10','r11'))
        code += '''
    stc
    ret
fixture_original:
    jne original_bad
    cmp rdi, [r13+32]
    jne original_bad
    cmp byte ptr [r13+24], 1
    jne original_bad
    mov byte ptr [r13+25], 1
    ret
original_bad:
    mov byte ptr [r13+25], 2
    ret
'''
        root = Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='release-hooks-', dir=root) as folder:
            p = Path(folder)
            (p/'fixture.s').write_text(code)
            subprocess.run([str(compiler),'-shared','-fuse-ld=lld','-nostdlib','-mavx2',
                '-Wl,/noentry',*exports,str(p/'fixture.s'),'-o',str(p/'fixture.dll')],
                check=True,capture_output=True)
            lib = ctypes.CDLL(str(p/'fixture.dll'))
            try:
                buf = ctypes.create_string_buffer(2048)
                for va in hooks.SITES:
                    for null in (False,True):
                        ctypes.memset(buf,0,len(buf))
                        fn = getattr(lib,f'case_{va:x}_{int(null)}')
                        fn.argtypes = [ctypes.c_void_p]
                        fn(buf)
                        self.assertEqual(bytes(buf)[664:666],b'\1\1')
                        for i in range(16):
                            word = ((0xffffffff << i) & 0xffffffff).to_bytes(4,'little')
                            self.assertEqual(bytes(buf)[128+32*i:160+32*i],word*8)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__ == '__main__':
    unittest.main()
