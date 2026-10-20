"""Check recovered binding and lifecycle facts against private executable bytes."""
import hashlib
from pathlib import Path
import unittest

from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from deke_runtime_evidence import SOURCE_HASH, TARGET_HASH
from research import ElfImage, SelfImage


class RuntimeEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        source = Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        target = Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if not source.exists() or not target.exists():
            raise unittest.SkipTest('Private executable fixtures unavailable')
        cls.source = ElfImage(source)
        cls.target = SelfImage(target)
        if (hashlib.sha256(cls.source.data).hexdigest() != SOURCE_HASH or
                hashlib.sha256(cls.target.data).hexdigest() != TARGET_HASH):
            raise AssertionError('Executable fixture identity changed')

    def test_panic_allocation_is_96_bytes_and_owner_slot_is_not_portable(self):
        self.assertEqual(self.source.read(0x480d450, 5), b'\xbe\x60\x00\x00\x00')
        self.assertEqual(self.source.read(0x480d46c, 7), b'\x4c\x89\xbb\xe8\x00\x00\x00')
        decoder = Cs(CS_ARCH_X86, CS_MODE_64)
        # Target reset really writes a word of flags into that same offset.
        writes = {(i.mnemonic, i.op_str) for i in decoder.disasm(
            self.target.read(0x2bbe800, 0x380), 0x2bbe800)}
        self.assertTrue(any(m == 'mov' and o.endswith('+ 0xe8], 0')
                            and o.startswith('word ptr') for m, o in writes))
        self.assertEqual(self.target.read(0x2bf1810, 7), bytes.fromhex('498b8790000000'))
        self.assertEqual(self.target.read(0x2f0dee8, 8), bytes.fromhex('498bbc2408020000'))

    def test_eight_native_binding_fields_are_read_and_runtime_pointers_written(self):
        decoder = Cs(CS_ARCH_X86, CS_MODE_64)
        instructions = {(i.mnemonic, i.op_str) for i in decoder.disasm(
            self.target.read(0x3ffbf0, 0x407290 - 0x3ffbf0), 0x3ffbf0)}
        for field in [0x12e8, 0x1280, 0x12e0, 0x1420, 0x1568, 0x1288, 0x718, 0x12c0]:
            self.assertIn(('mov', f'rdi, qword ptr [rbx + {field:#x}]'), instructions)
            self.assertIn(('mov', f'qword ptr [rcx + {field + 8:#x}], rax'), instructions)
        self.assertEqual(self.target.read(0x3ffc58, 5), b'\xbe\x80\x15\x00\x00')

    def test_action_setter_preserves_change_timestamp_on_equal_value(self):
        # Decode-independent signature includes conditional skip over both writes.
        self.assertEqual(self.source.read(0x3c9ae0, 21), bytes.fromhex(
            '89f03814c7740d8814c78b8f000c0000894cc704c3'))
        self.assertEqual(self.source.read(0x3d8450, 13), bytes.fromhex(
            '89f04869c0100c00004801f8c3'))


if __name__ == '__main__':
    unittest.main()
