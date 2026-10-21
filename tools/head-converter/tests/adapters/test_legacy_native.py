"""Unit tests for the independent Legacy native readers (src/adapters/legacy_native): half readback with bits,
exact parity comparison rows, byte-change audits, the XGAddress2DTiledOffset port, BC1 decoding and the packed-tail
search on synthetic data. Run: python -m unittest tests.adapters.test_legacy_native
"""
import struct, sys, unittest
from pathlib import Path
import numpy as np
ROOT = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(ROOT))
from src.adapters import legacy_native as LN


def synthetic_buffer(values, stride=32, start=64, pos_off=0):
    raw = bytearray(start + len(values) * stride)
    for i, v in enumerate(values): struct.pack_into('>3e', raw, start + i * stride + pos_off, *v)
    return bytes(raw), {'start': start, 'count': len(values), 'stride': stride, 'position_offset': pos_off, 'attributes': [(0, '001a236000000001'), (8, '002a218700030003'), (12, '002a218700060015')]}


class TestPositions(unittest.TestCase):
    def test_readback_matches_struct_and_bits(self):
        vals = [(1.75, -0.03125, 0.1220703125), (0.0, 1.9443359375, -0.1226806640625)]; raw, lay = synthetic_buffer(vals)
        v, bits, offs = LN.read_positions(raw, lay); self.assertTrue(np.array_equal(v, np.array(vals))); self.assertEqual(int(bits[0, 0]), 0x3F00); self.assertEqual(offs.tolist(), [64, 96])
        rows = LN.compare_positions_exact(v, bits, np.array(vals), np.array(vals), 'x'); self.assertEqual(rows, [])
        pred = np.array(vals); pred[1, 2] += 2 ** -13; rows = LN.compare_positions_exact(v, bits, pred, np.array(vals) + 1e-5, 'head')
        self.assertEqual(len(rows), 1); self.assertEqual((rows[0]['record'], rows[0]['axis']), (1, 'z')); self.assertEqual(rows[0]['stored_bits'], f'0x{int(bits[1, 2]):04x}'); self.assertAlmostEqual(rows[0]['difference_mm'], -2 ** -13 * 1000)

    def test_byte_audit_whitelist(self):
        vals = [(1.0, 1.0, 1.0)] * 3; raw, lay = synthetic_buffer(vals); after = bytearray(raw); struct.pack_into('>3e', after, 64 + 32, 1.5, 1.0, 1.0)
        mask = LN.position_byte_mask(len(raw), [lay], ['head'], ('head',)); a = LN.byte_change_audit(raw, bytes(after), mask); self.assertEqual(a['changed_outside_whitelist'], 0); self.assertGreater(a['changed_bytes'], 0)
        after[64 + 8] ^= 1; a = LN.byte_change_audit(raw, bytes(after), mask); self.assertEqual(a['changed_outside_whitelist'], 1); self.assertEqual(a['outside_offsets_first_16'], [72])
        m2 = LN.position_byte_mask(len(raw), [lay], ['haircap'], ('head',)); self.assertEqual(int(m2.sum()), 0)
        self.assertFalse(LN.byte_change_audit(raw, raw + b'x', mask)['same_length'])

    def test_snorm10_unpack(self):
        w = np.array([(511) | (1023 << 10) | (0 << 20) | (2 << 30)], np.int64); v, top = LN.unpack_snorm10(w)
        self.assertAlmostEqual(v[0, 0], 1.0); self.assertAlmostEqual(v[0, 1], -1 / 511); self.assertEqual(v[0, 2], 0.0); self.assertEqual(int(top[0]), 2)


class TestTiling(unittest.TestCase):
    def test_untile_recovers_a_known_placement(self):
        gw = gh = 32; pitch = 8; tiled = bytearray(gw * gh * pitch); expected = bytearray()
        for y in range(gh):
            for x in range(gw):
                blk = struct.pack('>2I', x, y); a = LN.xg_address_2d_tiled(x, y, gw, pitch); tiled[a:a + pitch] = blk; expected += blk
        self.assertEqual(LN.untile(bytes(tiled), gw, gh, pitch), bytes(expected))
        sub = LN.untile_region(bytes(tiled), 4, 0, 4, 4, pitch, gw); self.assertEqual(struct.unpack('>2I', sub[:8]), (4, 0)); self.assertEqual(struct.unpack('>2I', sub[-8:]), (7, 3))
        addrs = [LN.xg_address_2d_tiled(x, y, gw, pitch) for y in range(gh) for x in range(gw)]; self.assertEqual(len(set(addrs)), gw * gh); self.assertEqual(max(addrs) + pitch, gw * gh * pitch)   # a bijection onto one macro tile

    def test_swap16(self):
        self.assertEqual(LN.swap16(b'\x01\x02\x03\x04'), b'\x02\x01\x04\x03')


def bc1_block(c0, c1, indices):
    bits = 0
    for i, k in enumerate(indices): bits |= (k & 3) << (2 * i)
    return struct.pack('<HHI', c0, c1, bits)


class TestBC1(unittest.TestCase):
    def test_four_colour_and_three_colour_blocks(self):
        red = (31 << 11); blue = 31; blk = bc1_block(red, blue, [0] * 4 + [1] * 4 + [2] * 4 + [3] * 4)
        img, alpha = LN.decode_bc1(blk, 4, 4); self.assertEqual(img[0, 0].tolist(), [255, 0, 0]); self.assertEqual(img[1, 0].tolist(), [0, 0, 255]); self.assertEqual(img[2, 0].tolist(), [170, 0, 85]); self.assertTrue(alpha.all())
        blk3 = bc1_block(blue, red, [3] * 16); img3, alpha3 = LN.decode_bc1(blk3, 4, 4); self.assertEqual(img3[0, 0].tolist(), [0, 0, 0]); self.assertFalse(alpha3.any())

    def test_packed_tail_search_finds_planted_levels(self):
        rng = np.random.default_rng(7); donor = rng.integers(0, 256, (64, 64, 3)).astype(np.uint8); pitch = 8; tile = bytearray(32 * 32 * pitch)
        sys.path.insert(0, str(ROOT / 'experiments/codex/nhl21_package')); from reference_pack import bc1
        plants = {16: (4, 0), 8: (2, 0), 4: (1, 0)}
        for d, (bx, by) in plants.items():
            lvl = LN.box_downsample(donor.astype(float), d).astype(np.uint8); nb = d // 4; lin = bc1(lvl)
            for y in range(nb):
                for x in range(nb): a = LN.xg_address_2d_tiled(bx + x, by + y, 32, pitch); tile[a:a + pitch] = lin[(y * nb + x) * pitch:(y * nb + x + 1) * pitch]
        res = LN.packed_tail_search(bytes(tile), donor, dims=(16, 8, 4))
        for r in res: self.assertEqual(r['best']['block_origin'], list(plants[r['dimension']])); self.assertLess(r['best']['mae'], r['second']['mae'])


if __name__ == '__main__':
    unittest.main()
