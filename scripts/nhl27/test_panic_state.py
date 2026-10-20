"""Native attribute oracle and panic state boundary/preservation checks."""
import ctypes as C
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage


class State(C.Structure):
    _fields_ = [('actor', C.c_uint64), ('opponent', C.c_uint64), ('poke_opponent', C.c_uint64),
        ('duration', C.c_int), ('active_ticks', C.c_int), ('cooldown', C.c_int), ('stamp', C.c_int),
        ('flag', C.c_ubyte), ('padding', C.c_ubyte*3), ('facing', C.c_float),
        ('position', C.c_float*4), ('attributes', C.c_float*4), ('align_deadline', C.c_float),
        ('tail', C.c_ubyte*12)]


class Bindings(C.Structure):
    _fields_ = [(name, C.c_void_p) for name in ('active', 'path_lock', 'stumble', 'requested',
        'distance', 'between_legs', 'type', 'interrupt', 'align')]


class PanicStateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source = Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name != 'nt' or not compiler.exists() or not source.exists():
            raise unittest.SkipTest('Private source binary and Windows LLVM required')
        root = Path(__file__).resolve().parents[2]
        cls.scratch = tempfile.TemporaryDirectory(prefix='panic-state-',
            dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build')
        out = Path(cls.scratch.name)
        image = checked_image(ElfImage, source, SOURCE_HASH)
        # Retain all three RET paths; relocate only RIP-relative constant loads.
        raw = bytearray(image.read(0x4a05fb0, 0x30))
        for displacement, end, constant in ((0x15,0x19,0xdedafb8),
                (0x22,0x26,0xded9b54), (0x2b,0x2f,0xded936c)):
            struct.pack_into('<i', raw, displacement, len(raw)-end)
            raw.extend(image.read(constant,4))
        fixture = '''
__attribute__((naked,sysv_abi)) float original(unsigned id, int rating) {
    __asm__(".byte BYTES");
}
__declspec(dllexport) float oracle(unsigned id, int rating) {return original(id,rating);}
'''.replace('BYTES', ','.join(hex(v) for v in raw))
        (out/'fixture.c').write_text(fixture)
        result = subprocess.run([str(compiler), '-O2', '-Wall', '-Wextra', '-Werror',
            '-ffreestanding', '-fno-builtin', '-fuse-ld=lld', '-shared', '-nostdlib',
            '-Wl,/noentry', str(root/'scripts/nhl27/runtime/panic_state.c'),
            str(out/'fixture.c'), '-o', str(out/'fixture.dll')], capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError(result.stdout+result.stderr)
        cls.lib = C.CDLL(str(out/'fixture.dll'))
        for name in ('oracle','nhl27_panic_attribute'):
            f = getattr(cls.lib,name)
            f.argtypes=[C.c_uint,C.c_int]
            f.restype=C.c_float
        cls.lib.nhl27_panic_begin_update.argtypes=[C.POINTER(State), C.POINTER(Bindings), C.c_ubyte,C.c_int]
        cls.lib.nhl27_panic_finish_cooldown.argtypes=[C.POINTER(State), C.POINTER(Bindings), C.c_ubyte]
        cls.lib.nhl27_panic_cancel.argtypes=[C.POINTER(State), C.POINTER(Bindings), C.c_ubyte,
                                           C.c_float,C.POINTER(C.c_float)]

    @classmethod
    def tearDownClass(cls):
        import _ctypes
        _ctypes.FreeLibrary(cls.lib._handle)
        cls.scratch.cleanup()

    def fixture(self):
        state=State()
        values=[C.c_int(0) for _ in range(9)]
        bindings=Bindings(*(C.addressof(v) for v in values))
        return state,bindings,values

    def test_attribute_original_machine_code(self):
        for id in (0,0x11d,0x11f,0x120,0x121,0x123,0xffffffff):
            for rating in (-2147483648,-100,-1,*range(102),255,2147483647):
                self.assertEqual(struct.pack('<f', self.lib.oracle(id,rating)),
                                 struct.pack('<f', self.lib.nhl27_panic_attribute(id,rating)))

    def test_inactive_request_does_not_dispatch(self):
        s,b,v=self.fixture()
        s.active_ticks=9
        for duration,cooldown,expected in ((0,0,1),(-1,-1,1),(0,1,0),(1,0,2),(2,0,2)):
            s.duration,s.cooldown=duration,cooldown
            v[7].value=1
            self.assertEqual(self.lib.nhl27_panic_begin_update(C.byref(s),C.byref(b),1,5),expected)
            self.assertEqual(s.active_ticks,0)
            self.assertEqual(s.duration,duration-1 if duration>0 else duration)
            self.assertEqual(v[7].value,0)

    def test_active_counter_wrap_and_exact_boolean(self):
        s,b,v=self.fixture()
        v[0].value=v[2].value=1
        s.active_ticks=2147483647
        s.duration=-1
        self.assertEqual(self.lib.nhl27_panic_begin_update(C.byref(s),C.byref(b),2,17),6)
        self.assertEqual((s.active_ticks,s.duration,s.cooldown),(-2147483648,0,17))
        v[0].value=2
        self.assertEqual(self.lib.nhl27_panic_begin_update(C.byref(s),C.byref(b),1,99),0)
        self.assertEqual(s.cooldown,17)

    def test_cooldown_resamples_active_and_preserves_request_at_one(self):
        s,b,v=self.fixture()
        for old in (-2147483648,-1,0,1,2,2147483647):
            for active in (0,1,2):
                s.cooldown=old
                v[0].value=active
                v[3].value=1
                self.lib.nhl27_panic_finish_cooldown(C.byref(s),C.byref(b),1)
                self.assertEqual(s.cooldown,max(old,1)-1)
                self.assertEqual(v[3].value,0 if old>1 and active!=1 else 1)

    def test_cancel_changes_only_source_fields(self):
        s,b,v=self.fixture()
        C.memset(C.byref(s),0xa5,C.sizeof(s))
        s.actor=1
        before=bytes(s)
        for value in v: value.value=1
        position=(C.c_float*4)(1,2,3,4)
        self.lib.nhl27_panic_cancel(C.byref(s),C.byref(b),1,5,position)
        after=bytes(s)
        changed=set(range(8,16))|set(range(24,32))|set(range(44,64))
        for i in set(range(96))-changed:
            self.assertEqual(after[i],before[i],f'Unexpected state write at {i}')
        self.assertEqual((s.opponent,s.duration,s.active_ticks), (0,0,0))
        self.assertEqual((s.facing,list(s.position)),(5,[1,2,3,4]))
        self.assertEqual([x.value for x in v],[1,1,0,0,1,0,0,1,1])
        self.lib.nhl27_panic_cancel(C.byref(s),C.byref(b),2,5,position)
        self.assertEqual((s.facing,list(s.position)),(0,[0,0,0,0]))


if __name__ == '__main__':
    unittest.main()
