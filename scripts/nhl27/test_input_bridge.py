"""Execute the compiled bridges against isolated pointer fixtures, not the game."""
import ctypes
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest

from build_input_bridge import build


class InputBridgeTests(unittest.TestCase):
    def test_full_reset_preserves_owners_and_clears_former_controller(self):
        lib=self.library
        reset=lib.nhl24_input_reset_feature
        reset.argtypes=[ctypes.c_void_p]
        record=ctypes.create_string_buffer(0x21a0)
        local=lib.nhl24_input_initialize_extension(record)
        lib.nhl24_input_assign_controller(record,3,0)
        pool=lib.nhl27_controller_table(3)
        for address,owner in ((local,0x11112222),(pool,0x33334444)):
            ctypes.c_uint64.from_address(address+0xc08).value=owner
            ctypes.c_uint32.from_address(address+0xc04).value=0x55667788
            lib.nhl27_action_begin_frame(address,77)
            lib.nhl27_action_write(address,219,1)
        self.assertEqual(reset(record),0)
        self.assertEqual(lib.nhl24_input_action_storage(record),local)
        for address,owner in ((local,0x11112222),(pool,0x33334444)):
            self.assertEqual(ctypes.string_at(address+219*8,8),b'\0\0\0\0\xff\xff\xff\xff')
            self.assertEqual(ctypes.c_uint64.from_address(address+0xc08).value,owner)
            self.assertEqual(ctypes.c_uint32.from_address(address+0xc04).value,0x55667788)
        for i in range(12):
            address=lib.nhl27_controller_table(i)
            ctypes.c_uint64.from_address(address+0xc08).value=0x100+i
            lib.nhl27_action_begin_frame(address,55)
            lib.nhl27_action_write(address,224,1)
        lib.nhl27_controller_reset_all()
        for i in range(12):
            address=lib.nhl27_controller_table(i)
            self.assertEqual(ctypes.c_uint64.from_address(address+0xc08).value,0x100+i)
            self.assertEqual(ctypes.c_uint32.from_address(address+0xc00).value,0)
            self.assertEqual(ctypes.c_ubyte.from_address(address+224*8).value,0)

    def test_local_actor_snapshot_excludes_human_controller_pool(self):
        begin = self.library.nhl24_input_begin_local_actor_frame
        begin.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_uint32]
        record = ctypes.create_string_buffer(0x21a0)
        storage = self.library.nhl24_input_initialize_extension(record)
        self.library.nhl27_action_begin_frame(storage, 41)
        self.library.nhl27_action_write(storage, 219, 1)
        original = bytes(record)
        for controller in range(12):
            self.assertEqual(begin(record, controller, 42), 0)
            self.assertEqual(bytes(record), original)
        self.assertEqual(begin(record, -1, 42), 0)
        self.assertEqual(struct.unpack_from('<BB2xI',record,0x1580+219*8), (1,1,41))
        self.assertEqual(struct.unpack_from('<I',record,0x1580+0xc00)[0],42)

    def test_generic_asset_writer_matches_identity_and_context_destination(self):
        action_for = self.library.nhl24_input_action_for_asset
        action_for.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
        action_for.restype = ctypes.c_uint32
        observe = self.library.nhl24_input_observe_asset_write
        observe.argtypes = [ctypes.c_void_p] * 4 + [ctypes.c_ubyte]
        gs_asset = ctypes.create_string_buffer(0x1580)
        record = ctypes.create_string_buffer(0x21a0)
        assets = ctypes.create_string_buffer(8 * 104)
        slots = (ctypes.c_ubyte * 8)(*([0xa5] * 8))
        self.library.nhl24_input_initialize_extension(record)
        storage = ctypes.addressof(record) + 0x1580
        self.library.nhl27_action_begin_frame(storage, 47)
        cases = [(224, 0x12f0), (223, 0x12e8), (221, 0x12c8), (172, 0x1570),
                 (144, 0x1428), (219, 0x1290), (218, 0x1288), (139, 0x720)]
        for i, (action, offset) in enumerate(cases):
            asset, slot = ctypes.addressof(assets) + 104*i, ctypes.addressof(slots) + i
            struct.pack_into('<Q', gs_asset, offset-8, asset)
            struct.pack_into('<Q', record, offset, slot)
            self.assertEqual(action_for(gs_asset, asset), action)
            self.assertEqual(observe(record, gs_asset, asset, slot+1, 1), 2)
            self.assertEqual(observe(record, gs_asset, asset, slot, 1), 0)
            self.assertEqual(struct.unpack_from('<BB2xI', record, 0x1580+8*action), (1, 0, 47))
        self.assertEqual(bytes(slots), bytes([0xa5]*8))
        self.assertEqual(action_for(gs_asset, ctypes.addressof(assets)+1), 0xffffffff)
        self.assertEqual(action_for(None, None), 0xffffffff)

    @classmethod
    def setUpClass(cls):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name != 'nt' or not compiler.exists():
            raise unittest.SkipTest('Windows x64 Clang fixture toolchain unavailable')
        scratch = Path(__file__).resolve().parents[2] / 'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        scratch.mkdir(parents=True, exist_ok=True)
        cls.directory = tempfile.TemporaryDirectory(prefix='fixture-', dir=scratch)
        build(Path(cls.directory.name), compiler)
        cls.library = ctypes.CDLL(str(Path(cls.directory.name) / 'input_bridge.dll'))
        cls.library.nhl24_input_read.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.POINTER(ctypes.c_ubyte)]
        cls.library.nhl24_input_write.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_ubyte]
        cls.library.nhl24_input_produce.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_ubyte]
        cls.library.nhl27_action_write.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_ubyte]
        cls.library.nhl27_action_read.argtypes = [ctypes.c_void_p, ctypes.c_uint32,
            ctypes.POINTER(ctypes.c_ubyte), ctypes.POINTER(ctypes.c_uint32)]
        cls.library.nhl27_action_begin_frame.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
        cls.library.nhl27_action_reset.argtypes = [ctypes.c_void_p]
        cls.library.nhl27_action_initialize.argtypes = [ctypes.c_void_p]
        cls.library.nhl24_input_initialize_extension.argtypes = [ctypes.c_void_p]
        cls.library.nhl24_input_initialize_extension.restype = ctypes.c_void_p
        cls.library.nhl27_controller_table.argtypes = [ctypes.c_uint32]
        cls.library.nhl27_controller_table.restype = ctypes.c_void_p
        cls.library.nhl24_input_action_storage.argtypes = [ctypes.c_void_p]
        cls.library.nhl24_input_action_storage.restype = ctypes.c_void_p
        cls.library.nhl24_input_assign_controller.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_ubyte]
        cls.library.nhl27_controller_begin_frame.argtypes = [ctypes.c_uint32,ctypes.c_int,ctypes.c_uint32,ctypes.c_void_p]
        cls.library.nhl24_input_read_asset_value.argtypes = [ctypes.c_void_p]*3
        cls.library.nhl27_action_edges.argtypes = [ctypes.c_void_p, ctypes.c_uint32,
            ctypes.POINTER(ctypes.c_ubyte), ctypes.POINTER(ctypes.c_ubyte)]

    @classmethod
    def tearDownClass(cls):
        # Unload before removing the DLL; Windows holds loaded files open.
        import _ctypes
        _ctypes.FreeLibrary(cls.library._handle)
        cls.directory.cleanup()

    def test_controller_history_survives_player_switch_and_local_reset(self):
        lib = self.library
        first, second = ctypes.create_string_buffer(0x21a0), ctypes.create_string_buffer(0x21a0)
        pool = lib.nhl27_controller_table(3)
        lib.nhl27_action_initialize(pool)
        self.assertIsNone(lib.nhl27_controller_table(12))
        self.assertEqual(lib.nhl27_controller_begin_frame(3, 3, 90, 0x123456), 0)
        self.assertEqual(lib.nhl24_input_assign_controller(first, 3, 0), 0)
        self.assertEqual(lib.nhl24_input_action_storage(first), pool)
        value = ctypes.c_ubyte()
        struct.pack_into('<Q', first, 0x12f0, ctypes.addressof(value))
        self.assertEqual(lib.nhl24_input_produce(first, 224, 1), 0)
        self.assertEqual(ctypes.string_at(pool+224*8,8), struct.pack('<BB2xI',1,0,90))
        self.assertEqual(lib.nhl24_input_assign_controller(first,-1,0),0)
        self.assertEqual(lib.nhl24_input_action_storage(first),ctypes.addressof(first)+0x1580)
        self.assertEqual(lib.nhl24_input_assign_controller(second,3,2),0)
        self.assertEqual(lib.nhl24_input_action_storage(second),pool)
        self.assertEqual(ctypes.string_at(pool+224*8,8),struct.pack('<BB2xI',1,0,90))
        before = ctypes.string_at(pool,0xc10)
        self.assertEqual(lib.nhl27_controller_begin_frame(3,-1,91,0xabcdef),0)
        self.assertEqual(ctypes.string_at(pool,0xc10),before)
        self.assertEqual(lib.nhl27_controller_begin_frame(3,3,92,0xabcdef),0)
        self.assertEqual(ctypes.string_at(pool+224*8,8),struct.pack('<BB2xI',1,1,90))
        self.assertEqual(lib.nhl24_input_assign_controller(second,3,1),0)
        self.assertEqual(ctypes.string_at(pool+224*8,8),struct.pack('<BB2xI',0,0,0xffffffff))
        self.assertEqual(ctypes.string_at(pool+0xc08,8),struct.pack('<Q',0xabcdef))
        before = bytes(second)
        self.assertEqual(lib.nhl24_input_assign_controller(second,12,1),3)
        self.assertEqual(bytes(second),before)

    def test_action_asset_reader_uses_selected_history_and_assignment_syncs_native_byte(self):
        lib = self.library
        record = ctypes.create_string_buffer(0x21a0)
        gs = ctypes.create_string_buffer(0x1580)
        asset = ctypes.create_string_buffer(104)
        native = ctypes.c_ubyte(99)
        struct.pack_into('<Q', gs, 0x12e8, ctypes.addressof(asset))
        struct.pack_into('<Q', record, 0x12f0, ctypes.addressof(native))
        pool = lib.nhl27_controller_table(7)
        lib.nhl27_action_reset(pool)
        lib.nhl27_action_begin_frame(pool,123)
        lib.nhl27_action_write(pool,224,0x7f)
        lib.nhl24_input_assign_controller(record,7,0)
        self.assertEqual(native.value,0x7f)
        self.assertEqual(lib.nhl24_input_read_asset_value(record,gs,asset),0x7f)
        self.assertEqual(lib.nhl24_input_read_asset_value(None,gs,asset),0)
        self.assertEqual(lib.nhl24_input_read_asset_value(record,gs,None),0x100)
        lib.nhl24_input_assign_controller(record,-1,0)
        self.assertEqual(native.value,0)
        self.assertEqual(lib.nhl24_input_read_asset_value(record,gs,asset),0)

    def test_named_adapters_touch_only_their_native_byte(self):
        cases = [(224, 'spin_cw', 0x12f0), (223, 'spin_ccw', 0x12e8),
                 (221, 'shot_release', 0x12c8), (172, 'fake_shot', 0x1570),
                 (144, 'lacrosse_prepare', 0x1428), (219, 'glide_deke', 0x1290),
                 (218, 'enter_deke', 0x1288), (139, 'between_legs', 0x720)]
        record = ctypes.create_string_buffer(0x1580)
        backing = (ctypes.c_ubyte * 24)(*([0xa5] * 24))
        for index, (_, _, offset) in enumerate(cases):
            struct.pack_into('<Q', record, offset, ctypes.addressof(backing) + index * 3 + 1)
        record_before = bytes(record)
        for index, (action, name, _) in enumerate(cases):
            with self.subTest(action=action):
                write = getattr(self.library, f'nhl24_{name}_write')
                write.argtypes = [ctypes.c_void_p, ctypes.c_ubyte]
                read = getattr(self.library, f'nhl24_{name}_read')
                read.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ubyte)]
                for value in (0, 1, 0x7f):
                    before = bytes(backing)
                    self.assertEqual(write(record, value), 0)
                    expected = bytearray(before)
                    expected[index * 3 + 1] = value
                    self.assertEqual(bytes(backing), bytes(expected))
                    result = ctypes.c_ubyte(0xff)
                    self.assertEqual(read(record, ctypes.byref(result)), 0)
                    self.assertEqual(result.value, value)
                    self.assertEqual(self.library.nhl24_input_read(record, action, ctypes.byref(result)), 0)
                    self.assertEqual(result.value, value)
        self.assertEqual(bytes(record), record_before)

    def test_absent_and_unknown_bindings_leave_output_unchanged(self):
        record = ctypes.create_string_buffer(0x1580)
        result = ctypes.c_ubyte(0xa5)
        for record_arg, action, status in [(None, 224, 2), (record, 224, 2), (record, 999, 1)]:
            self.assertEqual(self.library.nhl24_input_read(record_arg, action, ctypes.byref(result)), status)
            self.assertEqual(self.library.nhl24_input_write(record_arg, action, 1), status)
            self.assertEqual(result.value, 0xa5)
        self.assertEqual(self.library.nhl24_input_read(record, 224, None), 3)

    def test_action_timestamps_and_opaque_regions(self):
        # Independently construct expected whole records. This catches padding,
        # analog-region writes, repeated-write timestamps and tick wrap handling.
        record = ctypes.create_string_buffer(bytes([0xa5]) * 0xc10, 0xc10)
        for action in range(244):
            for tick, value in [(17, 0), (18, 0), (0xffffffff, 1), (0, 1), (0, 0x7f)]:
                struct.pack_into('<I', record, 0xc00, tick)
                expected = bytearray(record.raw)
                if expected[action * 8] != value:
                    expected[action * 8] = value
                    struct.pack_into('<I', expected, action * 8 + 4, tick)
                self.assertEqual(self.library.nhl27_action_write(record, action, value), 0)
                self.assertEqual(record.raw, expected)
                out, stamp = ctypes.c_ubyte(), ctypes.c_uint32()
                self.assertEqual(self.library.nhl27_action_read(record, action,
                    ctypes.byref(out), ctypes.byref(stamp)), 0)
                self.assertEqual(out.value, value)
                self.assertEqual(stamp.value, struct.unpack_from('<I', expected, action * 8 + 4)[0])

    def test_action_table_rejects_analog_indices_and_nulls(self):
        record = ctypes.create_string_buffer(bytes([0xa5]) * 0xc10, 0xc10)
        original = record.raw
        out, stamp = ctypes.c_ubyte(99), ctypes.c_uint32(1234)
        for pointer, action, result in [(record, 244, 1), (record, 0xffffffff, 1), (None, 0, 2)]:
            self.assertEqual(self.library.nhl27_action_write(pointer, action, 1), result)
            self.assertEqual(self.library.nhl27_action_read(pointer, action,
                ctypes.byref(out), ctypes.byref(stamp)), result)
            self.assertEqual((out.value, stamp.value), (99, 1234))
        self.assertEqual(self.library.nhl27_action_read(record, 0, None, ctypes.byref(stamp)), 3)
        self.assertEqual(self.library.nhl27_action_read(record, 0, ctypes.byref(out), None), 3)
        self.assertEqual(record.raw, original)

    def test_frame_boundary_preserves_timestamps_and_analog_bits(self):
        record = ctypes.create_string_buffer(bytes((i * 31 + 7) % 256 for i in range(0xc10)), 0xc10)
        expected = bytearray(record.raw)
        for i in range(244):
            expected[i * 8 + 1] = expected[i * 8]
        for i in range(40):
            offset = 0x7a0 + i * 28
            expected[offset + 4:offset + 8] = expected[offset:offset + 4]
        struct.pack_into('<I', expected, 0xc00, 0xdeadbeef)
        self.assertEqual(self.library.nhl27_action_begin_frame(record, 0xdeadbeef), 0)
        self.assertEqual(record.raw, expected)
        self.assertEqual(self.library.nhl27_action_begin_frame(None, 0), 2)

    def test_extended_allocation_preserves_native_binding_and_guards(self):
        record = ctypes.create_string_buffer(bytes([0xa5]) * (0x2190 + 16), 0x2190 + 16)
        original = record.raw
        expected = ctypes.create_string_buffer(bytes([0xa5]) * 0xc10, 0xc10)
        self.library.nhl27_action_initialize(expected)
        self.assertEqual(self.library.nhl24_input_initialize_extension(record),
                         ctypes.addressof(record) + 0x1580)
        self.assertEqual(record.raw[:0x1580], original[:0x1580])
        self.assertEqual(record.raw[0x1580:0x2190], expected.raw)
        self.assertEqual(record.raw[0x2190:0x2194], bytes(4))
        self.assertEqual(record.raw[0x2194:], original[0x2194:])
        self.assertIsNone(self.library.nhl24_input_initialize_extension(None))

    def test_all_eight_producers_update_native_value_and_change_tick(self):
        cases = [(224, 'spin_cw', 0x12f0), (223, 'spin_ccw', 0x12e8),
                 (221, 'shot_release', 0x12c8), (172, 'fake_shot', 0x1570),
                 (144, 'lacrosse_prepare', 0x1428), (219, 'glide_deke', 0x1290),
                 (218, 'enter_deke', 0x1288), (139, 'between_legs', 0x720)]
        record = ctypes.create_string_buffer(0x21a0)
        table = self.library.nhl24_input_initialize_extension(record)
        backing = (ctypes.c_ubyte * 24)(*([0xa5] * 24))
        for index, (action, name, offset) in enumerate(cases):
            struct.pack_into('<Q', record, offset, ctypes.addressof(backing) + index * 3 + 1)
        prefix = record.raw[:0x1580]
        for index, (action, name, _) in enumerate(cases):
            function = getattr(self.library, f'nhl24_{name}_produce')
            function.argtypes = [ctypes.c_void_p, ctypes.c_ubyte]
            for tick, value in [(1, 1), (2, 1), (2, 0), (2, 1), (3, 0x7f)]:
                self.library.nhl27_action_begin_frame(table, tick)
                expected = bytearray(record.raw)
                slot = 0x1580 + action * 8
                if expected[slot] != value:
                    expected[slot] = value
                    struct.pack_into('<I', expected, slot + 4, tick)
                native_expected = bytearray(backing)
                native_expected[index * 3 + 1] = value
                self.assertEqual(function(record, value), 0)
                self.assertEqual(record.raw, expected)
                self.assertEqual(bytes(backing), native_expected)
        self.assertEqual(record.raw[:0x1580], prefix)
        before = record.raw
        self.assertEqual(self.library.nhl24_input_produce(record, 999, 1), 1)
        self.assertEqual(self.library.nhl24_input_produce(None, 172, 1), 2)
        self.assertEqual(record.raw, before)
        struct.pack_into('<Q', record, 0x1570, 0)
        before = record.raw
        self.assertEqual(self.library.nhl24_input_produce(record, 172, 1), 2)
        self.assertEqual(record.raw, before)

    def test_guest_publication_hook_with_original_target_publisher(self):
        from deke_runtime_evidence import checked_image, TARGET_HASH
        from research import SelfImage
        target_path = Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if not target_path.exists():
            self.skipTest('Private target binary unavailable')
        target = checked_image(SelfImage, target_path, TARGET_HASH)
        # The original publisher has only internal relative branches and one
        # indirect virtual destruction call. Fixtures retain an old reference
        # so no game virtual function is invoked. Preserve the exact return-
        # address/publisher distance used by the guest hook's image-base lookup.
        publisher = target.read(0x407290, 0x3d)
        directory = Path(self.directory.name)
        assembly = directory / 'publication-oracle.s'
        assembly.write_text('.text\n.p2align 4\n.globl fixture_guest_call\n'
            'fixture_guest_call:\nsubq $8, %rsp\ncall nhl24_input_publish_extended\n'
            'addq $8, %rsp\nretq\n.space 0x75ed-5, 0xcc\n.byte ' +
            ','.join(f'0x{b:02x}' for b in publisher) + '\n')
        wrapper = directory / 'publication-oracle.c'
        wrapper.write_text('extern __attribute__((sysv_abi)) void *fixture_guest_call(void *, void *);\n'
            '__declspec(dllexport) void *fixture_publish(void *cache, void *record) '
            '{return fixture_guest_call(cache, record);}\n')
        output = directory / 'publication-oracle.dll'
        bridge = Path(__file__).parent / 'runtime/input_bridge.c'
        command = [r'C:\Program Files\LLVM\bin\clang.exe', '-O2', '-ffreestanding', '-fno-builtin',
            '-fno-stack-protector', '-fuse-ld=lld', '-shared', '-nostdlib', '-Wl,/noentry',
            str(wrapper), str(assembly), str(bridge), '-o', str(output)]
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        fixture = ctypes.CDLL(str(output))
        try:
            fixture.fixture_publish.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
            fixture.fixture_publish.restype = ctypes.c_void_p
            for has_old in (False, True):
                old = ctypes.create_string_buffer(16)
                struct.pack_into('<I', old, 8, 2)
                cache = ctypes.c_void_p(ctypes.addressof(old) if has_old else None)
                record = ctypes.create_string_buffer(bytes([0xa5]) * (0x2190 + 16), 0x2190 + 16)
                struct.pack_into('<I', record, 8, 0)
                expected = bytearray(record.raw)
                expected_table = ctypes.create_string_buffer(bytes([0xa5]) * 0xc10, 0xc10)
                self.library.nhl27_action_initialize(expected_table)
                expected[0x1580:0x2190] = expected_table.raw
                struct.pack_into('<I', expected, 0x2190, 0)
                struct.pack_into('<I', expected, 8, 1)
                self.assertEqual(fixture.fixture_publish(ctypes.byref(cache), record), ctypes.addressof(cache))
                self.assertEqual(cache.value, ctypes.addressof(record))
                self.assertEqual(record.raw, expected)
                self.assertEqual(struct.unpack_from('<I', old, 8)[0], 1 if has_old else 2)
        finally:
            import _ctypes
            _ctypes.FreeLibrary(fixture._handle)

    def test_press_hold_release_and_same_frame_reversal(self):
        record = ctypes.create_string_buffer(0xc10)
        pressed, released = ctypes.c_ubyte(), ctypes.c_ubyte()
        def edges():
            self.assertEqual(self.library.nhl27_action_edges(record, 172,
                ctypes.byref(pressed), ctypes.byref(released)), 0)
            return pressed.value, released.value
        self.library.nhl27_action_begin_frame(record, 10)
        self.library.nhl27_action_write(record, 172, 1)
        self.assertEqual(edges(), (1, 0))
        self.library.nhl27_action_begin_frame(record, 11)
        self.assertEqual(edges(), (0, 0))
        self.library.nhl27_action_write(record, 172, 0)
        self.assertEqual(edges(), (0, 1))
        self.library.nhl27_action_write(record, 172, 1)
        self.assertEqual(edges(), (0, 0))
        self.assertEqual(struct.unpack_from('<I', record, 172 * 8 + 4)[0], 11)
        # Native instructions intentionally do not normalize these bytes.
        record[172 * 8], record[172 * 8 + 1] = 0x7f, 0x81
        self.assertEqual(edges(), ((0x81 ^ 1) & 0x7f, (0x7f ^ 1) & 0x81))

    def test_action_primitives_against_original_machine_code(self):
        # Execute only these four relocation-free leaf routines on isolated
        # buffers. The original binary is hash-checked and never modified.
        from deke_runtime_evidence import checked_image, SOURCE_HASH
        from research import ElfImage
        source_path = Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if not source_path.exists():
            self.skipTest('Private source binary unavailable')
        image = checked_image(ElfImage, source_path, SOURCE_HASH)
        # Independently interpret all source reset stores and compare the entire
        # native record, including the distinction between reset and construction.
        from action_reset_evidence import reset_template
        template = reset_template(image)
        reset_record = ctypes.create_string_buffer(bytes([0xa5]) * 0xc10, 0xc10)
        self.assertEqual(self.library.nhl27_action_reset(reset_record), 0)
        self.assertEqual(reset_record.raw, template + bytes(4) + bytes([0xa5]) * 12)
        self.assertEqual(self.library.nhl27_action_initialize(reset_record), 0)
        self.assertEqual(reset_record.raw, template + bytes(4) + bytes([0xa5]) * 4 + bytes(8))
        self.assertEqual(self.library.nhl27_action_reset(None), 2)
        self.assertEqual(self.library.nhl27_action_initialize(None), 2)
        definitions = []
        routines = [
            ('write', 'void', 'void *p, unsigned a, unsigned char v', 'p,a,v', 0x3c9ae0, 21),
            ('frame', 'void', 'void *p, unsigned t', 'p,t', 0x3d72c0, 0x2a5),
            ('pressed', 'unsigned char', 'void *p, unsigned a', 'p,a', 0x3d8030, 13),
            ('released', 'unsigned char', 'void *p, unsigned a', 'p,a', 0x3d8040, 13),
        ]
        for name, result, parameters, arguments, address, length in routines:
            code = image.read(address, length)
            self.assertEqual(code[-1], 0xc3)
            asm = ','.join(f'0x{byte:02x}' for byte in code)
            definitions.append(f'__attribute__((naked,sysv_abi)) {result} raw_{name}({parameters})'
                '{ __asm__(".byte ' + asm + '"); }\n'
                f'__declspec(dllexport) {result} oracle_{name}({parameters})'
                '{ ' + ('return ' if result != 'void' else '') + f'raw_{name}({arguments}); }}')
        directory = Path(self.directory.name)
        fixture = directory / 'source-oracle.c'
        fixture.write_text('\n'.join(definitions))
        output = directory / 'source-oracle.dll'
        command = [r'C:\Program Files\LLVM\bin\clang.exe', '-O2', '-fuse-ld=lld', '-shared',
            '-nostdlib', '-Wl,/noentry', str(fixture), '-o', str(output)]
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        oracle = ctypes.CDLL(str(output))
        try:
            oracle.oracle_write.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_ubyte]
            oracle.oracle_frame.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
            for name in ('pressed', 'released'):
                function = getattr(oracle, 'oracle_' + name)
                function.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
                function.restype = ctypes.c_ubyte
            initial = bytes((i * 73 + 11) % 256 for i in range(0xc10))
            source = ctypes.create_string_buffer(initial, 0xc10)
            port = ctypes.create_string_buffer(initial, 0xc10)
            pressed, released = ctypes.c_ubyte(), ctypes.c_ubyte()
            for tick in (0, 1, 1, 0xfffffffe, 0xffffffff, 0):
                oracle.oracle_frame(source, tick)
                self.library.nhl27_action_begin_frame(port, tick)
                self.assertEqual(source.raw, port.raw)
                for action in range(244):
                    for value in (0, 1, 1, 0x7f, 0xff, 0):
                        oracle.oracle_write(source, action, value)
                        self.library.nhl27_action_write(port, action, value)
                        self.assertEqual(source.raw, port.raw)
                        self.library.nhl27_action_edges(port, action,
                            ctypes.byref(pressed), ctypes.byref(released))
                        self.assertEqual((pressed.value, released.value),
                            (oracle.oracle_pressed(source, action), oracle.oracle_released(source, action)))
        finally:
            import _ctypes
            _ctypes.FreeLibrary(oracle._handle)


if __name__ == '__main__':
    unittest.main()
