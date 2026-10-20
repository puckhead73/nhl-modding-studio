"""Compare the history adapter with both games' original lookup instructions."""
import ctypes as C
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from oracle_fixture import assembly
from deke_runtime_evidence import checked_image, SOURCE_HASH, TARGET_HASH
from research import ElfImage, SelfImage


class PanicHistoryTests(unittest.TestCase):
    def test_original_lookups_and_adapter(self):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source = Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        target = Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name != 'nt' or not all(p.exists() for p in (compiler, source, target)):
            self.skipTest('Private images and Windows LLVM required')
        src = checked_image(ElfImage, source, SOURCE_HASH)
        dst = checked_image(SelfImage, target, TARGET_HASH)
        asm = assembly(src, 'source_lookup', 0x452cf70, 0x452cffe, {},
                       {0x1042c4c0: 'manager'})
        asm += assembly(dst, 'native_lookup', 0x2815160, 0x28151ca, {},
                        {0xbadc558: 'manager'})
        fixture = r'''
#define ABI __attribute__((sysv_abi))
void *manager;
extern ABI void *source_lookup(void*),*native_lookup(void*);
extern ABI void *nhl24_panic_history(void*,void*);
__declspec(dllexport) void *lookup(unsigned which,void *m,void *actor) {
    manager=m;
    if(which==1)return source_lookup(actor);
    if(which==2)return native_lookup(actor);
    return nhl24_panic_history(m,actor);
}
int _fltused=0;
'''
        repo = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-history-', dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out = Path(folder)
            (out/'oracle.s').write_text(asm)
            (out/'fixture.c').write_text(fixture)
            subprocess.run([str(compiler), '-O2', '-ffreestanding', '-fno-builtin',
                '-shared', '-nostdlib', '-fuse-ld=lld', '-Wl,/noentry',
                '-I', str(repo/'scripts/nhl27/runtime'), str(out/'oracle.s'),
                str(out/'fixture.c'), str(repo/'scripts/nhl27/runtime/panic_services.c'),
                '-o', str(out/'test.dll')], check=True, capture_output=True)
            lib = C.CDLL(str(out/'test.dll'))
            try:
                lib.lookup.argtypes = [C.c_uint, C.c_void_p, C.c_void_p]
                lib.lookup.restype = C.c_void_p
                manager = C.create_string_buffer(0x3d8)
                service = C.create_string_buffer(0x50)
                history = C.create_string_buffer(0xc0000)
                actor = C.create_string_buffer(0x1e8)
                role = C.create_string_buffer(0x48)
                for offset in (0x3c8, 0x3d0):
                    struct.pack_into('<Q', manager, offset, C.addressof(service))
                struct.pack_into('<Q', service, 0x48, C.addressof(history))
                for offset in (0x100, 0x110):
                    struct.pack_into('<Q', actor, offset, C.addressof(role))
                for bank in range(2):
                    for offset in (0x1bf50, 0x1bfb0):
                        struct.pack_into('<I', history, offset, bank)
                    for team in range(2):
                        struct.pack_into('<I', actor, 0x30, team)
                        for index in range(7):
                            struct.pack_into('<I', role, 0x40, index)
                            for disabled in (0, 1, 2, 255):
                                history[8] = disabled
                                a = lib.lookup(0, manager, actor)
                                b = lib.lookup(2, manager, actor)
                                self.assertEqual(a, b)
                                s = lib.lookup(1, manager, actor)
                                if disabled:
                                    self.assertIsNone(s)
                                else:
                                    self.assertEqual(a-C.addressof(history),
                                        0x1c030+team*0x11170+bank*0x222e0+index*0x2700)
                                    self.assertEqual(s-C.addressof(history),
                                        0x1bfe0+team*0x24040+bank*0x48080+index*0x2920)
                history[8] = 0
                # Invalid native roles must not alias the next team/bank.
                for index in (7, 13, 14, 0xffffffff):
                    struct.pack_into('<I', role, 0x40, index)
                    self.assertIsNone(lib.lookup(0, manager, actor))
                struct.pack_into('<I', role, 0x40, 0)
                for offset, buffer in ((0x30, actor), (0x1bfb0, history)):
                    old = struct.unpack_from('<I', buffer, offset)[0]
                    struct.pack_into('<I', buffer, offset, 2)
                    self.assertIsNone(lib.lookup(0, manager, actor))
                    struct.pack_into('<I', buffer, offset, old)
                self.assertIsNone(lib.lookup(0, None, actor))
                self.assertIsNone(lib.lookup(0, manager, None))
                for buffer, offset in ((manager, 0x3d0), (service, 0x48), (actor, 0x100)):
                    old = struct.unpack_from('<Q', buffer, offset)[0]
                    struct.pack_into('<Q', buffer, offset, 0)
                    self.assertIsNone(lib.lookup(0, manager, actor))
                    struct.pack_into('<Q', buffer, offset, old)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__ == '__main__':
    unittest.main()
