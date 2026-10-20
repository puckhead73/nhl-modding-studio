"""Regression checks for instruction discovery, independent of decompilation."""
import struct
import unittest

from trace import direct_call_candidates, operand_needles


class CallDiscoveryTests(unittest.TestCase):
    def test_narrow_and_negative_operands_are_candidates(self):
        self.assertIn(b'\x60', operand_needles(0x60))
        self.assertIn(b'\x60\x00\x00\x00', operand_needles(0x60))
        self.assertIn(b'\xff', operand_needles(-1))
        self.assertIn(b'\xff' * 4, operand_needles(-1))
        self.assertNotIn(b'\x70', operand_needles(0x670))

    def test_call_after_e8_in_operand_is_not_skipped(self):
        # mov al, 0xe8; call 0x2000. A consuming E8+4 regex misses the call.
        raw = b'\xb0\xe8\xe8' + struct.pack('<i', 0x2000 - (0x1002 + 5))
        self.assertEqual(list(direct_call_candidates(raw, 0x1000, 0x2000)), [0x1002])

    def test_negative_relative_displacement_and_truncated_tail(self):
        raw = b'\xe8' + struct.pack('<i', 0x100 - 0x205) + b'\xe8\x00'
        self.assertEqual(list(direct_call_candidates(raw, 0x200, 0x100)), [0x200])
        self.assertEqual(list(direct_call_candidates(raw, 0x200, 0x101)), [])


if __name__ == '__main__':
    unittest.main()
