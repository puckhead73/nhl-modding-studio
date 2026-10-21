"""Tests for the texlib walker, the packed-mip layout and the compressed-block mip writer. Synthetic tests always run;
the licensed-fixture tests (walker vs Codex's texlib_67 receipt, donor round trip, full-chain output re-extraction)
skip visibly when the fixture copy or receipts are absent. Run: python -m unittest tests.adapters.test_legacy_texlib
"""
import hashlib, json, struct, sys, unittest
from pathlib import Path
import numpy as np
ROOT = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(ROOT))
from src.adapters import legacy_native as LN, legacy_texlib as TL, legacy_mip_writer as MW
FIX = ROOT / 'local/assets/claude/legacy-2026-09-13-repaired-native-01/texlib_67.rx2'; CODEX = ROOT / 'local/runs/codex/legacy-native-diagnostic-2026-09-13-02/receipt.json'
FULL = ROOT / 'local/runs/claude/legacy-2026-09-14-packedmip-fullchain-03'


class TestLayout(unittest.TestCase):
    def test_packed_offsets_follow_reference_rule_for_square_512(self):
        # texture_util.cc GetPackedMipOffset, square: 16 -> x 16, 8 -> x 8, 4 -> x 4 (texels), 2 -> y 8, 1 -> y 4; in blocks /4
        self.assertIsNone(TL.packed_mip_offset(512, 512, 4)); self.assertEqual(TL.packed_mip_offset(512, 512, 5), (4, 0)); self.assertEqual(TL.packed_mip_offset(512, 512, 6), (2, 0)); self.assertEqual(TL.packed_mip_offset(512, 512, 7), (1, 0))
        self.assertEqual(TL.packed_mip_offset(512, 512, 8), (0, 2)); self.assertEqual(TL.packed_mip_offset(512, 512, 9), (0, 1))
        self.assertEqual(TL.packed_mip_offset(256, 256, 4), (4, 0))                                            # 256 -> first packed level is 4 (16x16)
        self.assertEqual(TL.packed_mip_offset(512, 256, 4), (0, 4))                                            # wider than tall -> laid out vertically

    def test_slot_layout_totals_and_addresses(self):
        lay = TL.slot_layout(512, 512, 8, 10); self.assertEqual(lay['total_bytes'], 196608); self.assertEqual([l['offset'] for l in lay['levels']], [0, 131072, 163840, 172032, 180224] + [188416] * 5)
        cov = MW.coverage_report(lay, 8, 196608); self.assertEqual(cov['bytes_covered_by_levels'], 174776); self.assertEqual(cov['overlapping_level_pairs'], []); self.assertTrue(cov['within_slot'])
        self.assertEqual(cov['blocks_per_level'], {0: 16384, 1: 4096, 2: 1024, 3: 256, 4: 64, 5: 16, 6: 4, 7: 1, 8: 1, 9: 1})
        lay256 = TL.slot_layout(256, 256, 16, 9); self.assertEqual(lay256['total_bytes'], 65536 + 16384 + 16384 + 16384 + 16384)   # DXT3: 256 base, 128/64/32 own tiles, 16..1 packed


class TestWriter(unittest.TestCase):
    def test_round_trip_and_injection_on_synthetic_slot(self):
        lay = TL.slot_layout(512, 512, 8, 10); rng = np.random.default_rng(3); slot = bytes(rng.integers(0, 256, 196608, dtype=np.uint8).tobytes())   # every byte nonzero-ish: padding will not be reproducible
        rt = MW.round_trip(slot, lay, 8); self.assertTrue(rt['copy_round_trip_exact']); self.assertFalse(rt['zero_round_trip_exact']); self.assertGreater(rt['bytes_not_reproducible_from_levels'], 0); self.assertIsNotNone(rt['cause'])
        syn = MW.synthetic_injection(lay, 8, 196608); self.assertTrue(syn['every_level_reads_back_identically']); self.assertTrue(syn['no_bytes_written_outside_levels']); self.assertEqual(syn['nonzero_bytes_written'], 174776)
        blocks = MW.extract_levels(slot, lay, 8); blocks[5] = bytes(128)                                       # rewrite one level only
        new = MW.write_levels(slot, lay, 8, blocks); a = np.frombuffer(slot, np.uint8); b = np.frombuffer(new, np.uint8); changed = set(np.flatnonzero(a != b).tolist())
        allowed = set(o for off, n in TL.level_block_addresses(lay['levels'][5], 8) for o in range(off, off + n)); self.assertTrue(changed <= allowed); self.assertEqual(MW.extract_levels(new, lay, 8)[5], bytes(128))
        with self.assertRaises(ValueError): MW.write_levels(slot, lay, 8, blocks[:9])
        with self.assertRaises(ValueError): TL.write_level_blocks(bytearray(slot), lay['levels'][5], 8, bytes(64))

    def test_patch_texlib_whitelist_on_synthetic_container(self):
        lay = TL.slot_layout(512, 512, 8, 10); raw = bytes(np.random.default_rng(5).integers(0, 256, 4096 + 196608 + 512, dtype=np.uint8).tobytes())
        tex = {'slot_start': 4096, 'slot_bytes': 196608, 'bytes_per_block': 8}; blocks = MW.extract_levels(raw[4096:4096 + 196608], lay, 8); blocks[9] = b'\x11' * 8
        out, audit = MW.patch_texlib(raw, tex, lay, blocks); self.assertEqual(audit['changed_outside_slot'], 0); self.assertEqual(audit['changed_outside_level_addresses'], 0); self.assertLessEqual(audit['changed_bytes'], 8); self.assertTrue(audit['same_length'])
        self.assertEqual(out[:4096], raw[:4096]); self.assertEqual(out[-512:], raw[-512:])


@unittest.skipUnless(FIX.exists() and CODEX.exists(), 'licensed Legacy texlib copy or Codex receipt absent')
class TestWalkerFixture(unittest.TestCase):
    def test_walker_matches_codex_receipt_and_descriptor_declares_packed_ten_levels(self):
        raw = FIX.read_bytes(); tex = TL.parse(raw); rec = json.loads(CODEX.read_text(encoding='utf-8'))
        for t, c in zip(tex, rec['textures']):
            self.assertEqual((t['name'], t['width'], t['format'], t['descriptor_offset'], t['data_relative_offset'], t['slot_bytes']), (c['name'], c['width'], c['format'], c['descriptor_offset'], c['data_relative_offset'], c['capacity']))
            self.assertEqual(hashlib.sha256(TL.base_linear(raw, t)).hexdigest(), c['base_linear_sha256'])
        head = tex[1]; self.assertEqual(head['fetch']['mip_max_level'], 9); self.assertTrue(head['fetch']['packed_mips']); self.assertTrue(head['fetch']['tiled']); self.assertEqual(head['fetch']['width'], 512); self.assertEqual(head['fetch']['mip_address'], 131072)
        lay = TL.slot_layout(512, 512, 8, 10); slot = raw[head['slot_start']:head['slot_start'] + head['slot_bytes']]; rt = MW.round_trip(slot, lay, 8)
        self.assertTrue(rt['copy_round_trip_exact']); self.assertTrue(rt['zero_round_trip_exact'])          # donor padding outside levels is all zero
        with self.assertRaises(ValueError): TL.parse(b'\x00' * 200)


@unittest.skipUnless(FULL.exists() and FIX.exists(), 'full-chain diagnostic run absent')
class TestFullChainOutput(unittest.TestCase):
    def test_output_levels_reextract_and_whitelist(self):
        rec = json.loads((FULL / 'receipt.json').read_text(encoding='utf-8')); out = (FULL / 'texlib_67_FULL_CHAIN.rx2').read_bytes(); self.assertEqual(hashlib.sha256(out).hexdigest(), rec['full_chain']['output']['sha256'])
        raw = FIX.read_bytes(); tex = next(t for t in TL.parse(out) if t['name'] == 'head_67_cm.Raster'); lay = TL.slot_layout(512, 512, 8, 10)
        back = MW.extract_levels(out[tex['slot_start']:tex['slot_start'] + tex['slot_bytes']], lay, 8)
        for k, b in enumerate(back): self.assertEqual(hashlib.sha256(b).hexdigest(), rec['full_chain']['levels'][k]['sha256'])
        self.assertEqual(out[:tex['slot_start']], raw[:tex['slot_start']]); self.assertEqual(out[tex['slot_start'] + tex['slot_bytes']:], raw[tex['slot_start'] + tex['slot_bytes']:]); self.assertFalse(rec['export_ready'])


if __name__ == '__main__':
    unittest.main()
