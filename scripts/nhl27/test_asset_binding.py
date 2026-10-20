"""Use the original target bank lookup against isolated signed-ID/alias tables."""
import ctypes
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image, TARGET_HASH
from research import SelfImage


class AssetBindingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        target = Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name != 'nt' or not compiler.exists() or not target.exists():
            raise unittest.SkipTest('Private target and Windows LLVM required')
        root = Path(__file__).resolve().parents[2]
        cls.scratch = tempfile.TemporaryDirectory(prefix='asset-binding-',
            dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build')
        p = Path(cls.scratch.name)
        # The successful lookup block lies after RET and jumps back to it.
        # Preserve the entire unwind body, not just the first return path.
        image = checked_image(SelfImage,target,TARGET_HASH)
        raw = image.read(0xadf550,0x10e)
        assert raw[-2:] == bytes.fromhex('ebe8')
        fixture = '''
typedef __UINTPTR_TYPE__ address;
__attribute__((naked,sysv_abi)) void *query(void *bank, const unsigned long long *id) {
    __asm__(".byte BYTES");
}
__attribute__((sysv_abi)) void *cast(void *asset, unsigned hash) {
    return hash == 0x7a327d7dU ? (char *)asset+64 : (void *)0;
}
__attribute__((sysv_abi)) void *storage(void *property, void *context) {
    return (char *)property == (char *)context+64 ? (char *)context+200 : (void *)0;
}
__attribute__((sysv_abi)) void *no_cast(void *asset, unsigned hash) {
    (void)asset; (void)hash; return (void *)0;
}
__attribute__((naked,sysv_abi)) void *enum_cast(void *asset, unsigned hash) { __asm__(".byte ENUM_BYTES"); }
__attribute__((naked,sysv_abi)) void *bool_cast(void *asset, unsigned hash) { __asm__(".byte CAST_BYTES"); }
__attribute__((naked,sysv_abi)) void *native_read(void *asset, void *context) { __asm__(".byte READ_BYTES"); }
__attribute__((naked,sysv_abi)) void *native_default(void *asset, void *context) { __asm__(".byte DEFAULT_BYTES"); }
__attribute__((sysv_abi)) void *lookup(void *context, unsigned key) {
    return key == 0x1000000U ? (char *)context+201 : (void *)0;
}
__declspec(dllexport) address callback(unsigned which) {
    switch(which) {
        case 0: return (address)query;
        case 1: return (address)cast;
        case 2: return (address)storage;
        case 4: return (address)enum_cast;
        case 5: return (address)lookup;
        case 6: return (address)bool_cast;
        case 7: return (address)native_read;
        case 8: return (address)native_default;
        default: return (address)no_cast;
    }
}
'''
        for token,address,size in [('ENUM_BYTES',0x7bada0,0x50),('CAST_BYTES',0x8e1510,0xc0),
                                   ('READ_BYTES',0x7a1a90,0x90),('DEFAULT_BYTES',0x8a87a0,0x10)]:
            fixture=fixture.replace(token,','.join(hex(x) for x in image.read(address,size)))
        fixture=fixture.replace('BYTES',','.join(hex(x) for x in raw))
        (p/'fixture.c').write_text(fixture)
        subprocess.run([str(compiler),'-O2','-Wall','-Wextra','-Werror','-ffreestanding',
            '-fno-builtin','-fuse-ld=lld','-shared','-nostdlib','-Wl,/noentry',
            str(root/'scripts/nhl27/runtime/asset_binding.c'),str(p/'fixture.c'),
            '-o',str(p/'test.dll')],check=True,capture_output=True)
        cls.lib = ctypes.CDLL(str(p/'test.dll'))
        cls.lib.callback.argtypes=[ctypes.c_uint]
        cls.lib.callback.restype=ctypes.c_void_p
        cls.lib.nhl24_ant_find_anchored_asset.argtypes=[ctypes.c_void_p,ctypes.c_uint,
            ctypes.c_uint64,ctypes.c_void_p,ctypes.c_uint64,ctypes.c_void_p]
        cls.lib.nhl24_ant_bind_value.argtypes=[ctypes.c_void_p]*3
        cls.lib.nhl24_ant_find_registered_asset.argtypes=cls.lib.nhl24_ant_find_anchored_asset.argtypes
        cls.lib.nhl24_panic_motion_registered.argtypes=[ctypes.c_void_p,ctypes.c_uint,
            ctypes.c_void_p,ctypes.c_void_p,ctypes.c_ubyte,ctypes.c_uint,ctypes.c_void_p,ctypes.c_void_p]

    @classmethod
    def tearDownClass(cls):
        import _ctypes
        _ctypes.FreeLibrary(cls.lib._handle)
        cls.scratch.cleanup()

    def bank(self, values, aliases=()):
        signed = lambda p: p[0] if p[0] < 1<<63 else p[0]-(1<<64)
        values,aliases = sorted(values,key=signed),sorted(aliases,key=signed)
        vt = (ctypes.c_void_p*7)()
        vt[6]=self.lib.callback(0)
        table=(ctypes.c_uint64*(2*len(values)))(*(v for p in values for v in p))
        alias=(ctypes.c_uint64*(2*len(aliases)))(*(v for p in aliases for v in p))
        bank=ctypes.create_string_buffer(0x160)
        struct.pack_into('<Q',bank,0,ctypes.addressof(vt))
        struct.pack_into('<I',bank,0x98,len(values))
        struct.pack_into('<Q',bank,0xa0,ctypes.addressof(table))
        struct.pack_into('<Q',bank,0xd8,ctypes.addressof(alias))
        struct.pack_into('<I',bank,0xe0,len(aliases))
        return bank,vt,table,alias

    def test_cross_bank_signal_and_conflicting_generations(self):
        state=self.bank([(7,0x111)])
        launch=self.bank([(8,0x222)])
        banks=(ctypes.c_void_p*2)(ctypes.addressof(state[0]),ctypes.addressof(launch[0]))
        out=ctypes.c_void_p(0x555)
        find=self.lib.nhl24_ant_find_registered_asset
        self.assertEqual(find(banks,2,7,0x111,8,ctypes.byref(out)),0)
        self.assertEqual(out.value,0x222)
        other=self.bank([(7,0x111),(8,0x333)])
        banks[0]=ctypes.addressof(other[0])
        out.value=0x555
        self.assertEqual(find(banks,2,7,0x111,8,ctypes.byref(out)),3)
        self.assertEqual(out.value,0x555)

    def test_native_signed_lookup_alias_and_anchor_select_correct_bank(self):
        # High-bit panic IDs sort before low IDs in the game's signed table.
        wanted=0xde0e918abd457be9
        first=self.bank([(7,0x111),(wanted,0xabc)])
        second=self.bank([(7,0x222),(wanted,0xdef)],[(8,wanted)])
        banks=(ctypes.c_void_p*2)(ctypes.addressof(first[0]),ctypes.addressof(second[0]))
        out=ctypes.c_void_p(0x555)
        find=self.lib.nhl24_ant_find_anchored_asset
        for id in (wanted,8):
            self.assertEqual(find(banks,2,7,0x222,id,ctypes.byref(out)),0)
            self.assertEqual(out.value,0xdef)
        out.value=0x555
        self.assertEqual(find(banks,2,7,0x333,wanted,ctypes.byref(out)),2)
        self.assertEqual(find(banks,2,7,0x222,999,ctypes.byref(out)),2)
        self.assertEqual(out.value,0x555)
        duplicate=self.bank([(7,0x222),(wanted,0xdef)])
        banks[0]=ctypes.addressof(duplicate[0])
        self.assertEqual(find(banks,2,7,0x222,wanted,ctypes.byref(out)),3)
        self.assertEqual(out.value,0x555)

    def test_property_cast_adjustment_and_context_are_required(self):
        asset=ctypes.create_string_buffer(256)
        vt=(ctypes.c_void_p*18)();property_vt=(ctypes.c_void_p*18)()
        vt[2]=self.lib.callback(1);property_vt[17]=self.lib.callback(2)
        struct.pack_into('<Q',asset,0,ctypes.addressof(vt))
        struct.pack_into('<Q',asset,64,ctypes.addressof(property_vt))
        out=ctypes.c_void_p(0x555);bind=self.lib.nhl24_ant_bind_value
        self.assertEqual(bind(asset,asset,ctypes.byref(out)),0)
        self.assertEqual(out.value,ctypes.addressof(asset)+200)
        out.value=0x555
        self.assertEqual(bind(asset,ctypes.addressof(asset)+8,ctypes.byref(out)),2)
        vt[2]=self.lib.callback(3)
        self.assertEqual(bind(asset,asset,ctypes.byref(out)),4)
        self.assertEqual(out.value,0x555)
        self.assertEqual(bind(None,asset,ctypes.byref(out)),1)

    def test_motion_registry_cross_bank_read_only_and_generation_checks(self):
        node=ctypes.create_string_buffer(256);enum=ctypes.create_string_buffer(40)
        vt=(ctypes.c_void_p*18)();property_vt=(ctypes.c_void_p*18)();enum_vt=(ctypes.c_void_p*3)()
        vt[2]=self.lib.callback(1);property_vt[16]=self.lib.callback(2);enum_vt[2]=self.lib.callback(4)
        struct.pack_into('<Q',node,0,ctypes.addressof(vt))
        struct.pack_into('<Q',node,64,ctypes.addressof(property_vt))
        struct.pack_into('<Q',enum,0,ctypes.addressof(enum_vt));struct.pack_into('<I',enum,8,0x1000000)
        enum[32]=bytes([2]);node[200]=bytes([1]);node[201]=bytes([8]);node[202]=bytes([2])
        state=self.bank([(0x30b7d024f81ea1df,0x111),(0xc9f1cc065e44d353,ctypes.addressof(enum))])
        launch=self.bank([(0x9f2ed2b822171119,ctypes.addressof(node))])
        banks=(ctypes.c_void_p*2)(ctypes.addressof(state[0]),ctypes.addressof(launch[0]))
        fn=self.lib.nhl24_panic_motion_registered;out=ctypes.c_ubyte(99)
        def read(active=1,bit=9):
            return fn(banks,2,0x111,node,active,bit,self.lib.callback(5),ctypes.byref(out))
        self.assertEqual(read(),0);self.assertEqual(out.value,1)
        self.assertEqual(read(bit=3),0);self.assertEqual(out.value,1)
        node[200]=bytes([2]);self.assertEqual(read(),0);self.assertEqual(out.value,0)
        self.assertEqual(read(bit=3),0);self.assertEqual(out.value,1)
        node[200]=bytes([1]);enum[32]=bytes([1])
        self.assertEqual(read(),0);self.assertEqual(out.value,0)
        enum[32]=bytes([2]);struct.pack_into('<I',enum,8,0x1001000)
        self.assertEqual(read(),0);self.assertEqual(out.value,0)
        struct.pack_into('<I',enum,8,0x1000000)
        other=self.bank([(0x9f2ed2b822171119,ctypes.addressof(node)),(0xc9f1cc065e44d353,0x333)])
        banks[1]=ctypes.addressof(other[0]);out.value=99
        self.assertEqual(read(),3);self.assertEqual(out.value,99)
        self.assertEqual(read(active=2),0);self.assertEqual(out.value,0)

    def test_motion_uses_native_context_value_not_asset_default(self):
        node=ctypes.create_string_buffer(104);enum=ctypes.create_string_buffer(40)
        context=ctypes.create_string_buffer(256);block=ctypes.create_string_buffer(0x900)
        vt=(ctypes.c_void_p*18)();enum_vt=(ctypes.c_void_p*3)()
        vt[2]=self.lib.callback(6);vt[15]=self.lib.callback(8);vt[16]=self.lib.callback(7)
        enum_vt[2]=self.lib.callback(4)
        struct.pack_into('<QI',node,0,ctypes.addressof(vt),0x1000000)
        struct.pack_into('<QI',enum,0,ctypes.addressof(enum_vt),0x1000000);enum[32]=bytes([2])
        struct.pack_into('<Q',context,0x18,ctypes.addressof(block))
        struct.pack_into('<I',block,0x71c,1);block[0]=bytes([1])
        context[202]=bytes([2]);node[88]=bytes([0])
        bank=self.bank([(0x30b7d024f81ea1df,0x111),(0x9f2ed2b822171119,ctypes.addressof(node)),
                        (0xc9f1cc065e44d353,ctypes.addressof(enum))])
        banks=(ctypes.c_void_p*1)(ctypes.addressof(bank[0]));out=ctypes.c_ubyte(99)
        fn=self.lib.nhl24_panic_motion_registered
        self.assertEqual(fn(banks,1,0x111,context,1,9,self.lib.callback(5),ctypes.byref(out)),0)
        self.assertEqual(out.value,1)
        # With no initialized value, the native getter falls back to default 0.
        struct.pack_into('<I',block,0x71c,0)
        self.assertEqual(fn(banks,1,0x111,context,1,9,self.lib.callback(5),ctypes.byref(out)),0)
        self.assertEqual(out.value,0)
        self.assertEqual(struct.unpack_from('<I',block,0x71c)[0],0)


if __name__ == '__main__':
    unittest.main()
