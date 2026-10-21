"""Targeted tests for the native half-float storage model, the lip-crossing classifier and the quantisation-aware
lip-block fit options. Run: python -m unittest tests.conversion.test_legacy_quant
"""
import struct, sys, unittest
from pathlib import Path
import numpy as np
REPO = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(REPO))
from src.conversion import legacy_exchange as LX, legacy_fit as LF, legacy_quant as LQt, geometry as G
from tests.conversion.test_legacy_exchange import synthetic_entry
from tests.conversion.test_legacy_fit import synthetic_cc5


class TestHalfModel(unittest.TestCase):
    def test_every_finite_half_round_trips(self):
        bits = np.arange(0, 0x10000, dtype=np.uint16); finite = ((bits >> 10) & 0x1F) != 0x1F; bits = bits[finite]
        v = LQt.f32_from_half_bits(bits); back = LQt.half_bits_from_f32(v.astype(np.float32))
        neg_zero = bits == 0x8000; self.assertTrue(np.array_equal(back[~neg_zero], bits[~neg_zero])); self.assertEqual(int(back[neg_zero][0]), 0x8000)

    def test_writer_rule_matches_struct_off_ties_and_differs_on_exact_ties(self):
        rng = np.random.default_rng(0); x = (rng.random(20000) * 4 - 2).astype(np.float32)          # head-scale coordinates in [-2, 2)
        x = x[(x.view(np.uint32) & 0x1FFF) != 0x1000]                                                        # drop the exact ties (dropped bits == 0x1000), where the rules differ by design
        ref = np.array([struct.unpack('>e', struct.pack('>e', float(v)))[0] for v in x]); mine = LQt.f32_from_half_bits(LQt.half_bits_from_f32(x))
        self.assertTrue(np.array_equal(ref, mine))                                                           # random float32 values are never exact ties
        ulp = 2.0 ** -10; tie = np.float32(1.0 + 0.5 * ulp); above = np.float32(1.0 + 1.5 * ulp)
        self.assertEqual(float(LQt.native_round(tie)), 1.0 + ulp)                                             # writer: round half up
        self.assertEqual(float(LQt.python_round(tie)), 1.0)                                                   # struct: round half to even
        self.assertEqual(float(LQt.native_round(above)), 1.0 + 2 * ulp); self.assertEqual(float(LQt.python_round(above)), 1.0 + 2 * ulp)
        self.assertEqual(float(LQt.native_round(np.float32(-tie))), -(1.0 + ulp))                            # sign-symmetric (magnitude rounds up)
        carry = np.float32(1.0 + 1023.5 * ulp); self.assertEqual(float(LQt.native_round(carry)), 2.0)         # mantissa overflow carries into the exponent
        self.assertTrue(np.isinf(LQt.native_round(np.float32(70000.0))))                                     # overflow -> inf, as the writer

    def test_ulp_representability_and_storage_error(self):
        self.assertAlmostEqual(float(LQt.half_ulp(1.75)) * 1000, 0.9765625); self.assertAlmostEqual(float(LQt.half_ulp(0.1)), 2.0 ** -14); self.assertAlmostEqual(float(LQt.half_ulp(0.03)), 2.0 ** -16)
        v = LQt.f32_from_half_bits(np.array([0x3F00, 0x2C00, 0x5000], np.uint16)); self.assertTrue(LQt.is_representable(v).all())
        self.assertFalse(LQt.is_representable(v + 0.3 * LQt.half_ulp(v)).any()); self.assertTrue((LQt.storage_error((v + 0.3 * LQt.half_ulp(v))[:, None]) <= 0.5 * LQt.half_ulp(v) + 1e-12).all())

    def test_grid_translation(self):
        rng = np.random.default_rng(3); base = LQt.f32_from_half_bits(rng.integers(0x3C00, 0x4000, size=(50, 3)).astype(np.uint16))   # 50 records in [1, 2): ulp 2^-10 on every axis
        t = np.array([-0.0015873, -0.0019485, 0.0003303]); tp, rep = LQt.grid_translation(base, t)
        self.assertTrue(rep['ok']); self.assertTrue(LQt.is_representable(base + tp).all()); self.assertTrue(all(a['all_representable'] for a in rep['axes']))
        self.assertTrue(np.allclose(tp / 2.0 ** -10, np.round(tp / 2.0 ** -10))); self.assertLess(rep['adjust_mm'], 0.5 * 2.0 ** -10 * 1000 * np.sqrt(3) + 1e-9)
        straddle = np.c_[LQt.f32_from_half_bits(np.array([0x0003, 0x1400, 0x2C00], np.uint16)) * np.array([1, -1, 1]), base[:3, 1:]]            # x: a 3*2^-24 midline record is not a multiple of the coarse ulp the translation needs
        tp2, rep2 = LQt.grid_translation(straddle, t); self.assertFalse(rep2['axes'][0]['all_representable']); self.assertFalse(rep2['ok']); self.assertTrue(rep2['axes'][1]['all_representable'])

    def test_quantize_exchange_and_model_verification(self):
        ex = LX.from_native_structure(synthetic_entry()); H = np.asarray(LX.submesh(ex, 'head')['positions']) + 1e-4   # off-grid head
        ex2 = LX.with_positions(ex, 'head', G.native_to_blender(H)); q = LQt.quantize_exchange(ex2)
        self.assertEqual(LX.submesh(q, 'haircap')['positions'], LX.submesh(ex2, 'haircap')['positions'])
        Q = np.asarray(LX.submesh(q, 'head')['positions']); self.assertTrue((np.abs(Q - H) <= 0.5 * LQt.half_ulp(H) + 1e-12).all()); self.assertTrue(LQt.is_representable(Q).all())
        rep = LQt.verify_model_against_reparsed(ex2, q); self.assertTrue(rep['all_exact']); self.assertEqual(rep['head']['coords_exact'], rep['head']['coords_total'])
        bad = LX.with_positions(q, 'head', G.native_to_blender(Q + 1e-3)); self.assertFalse(LQt.verify_model_against_reparsed(ex2, bad)['all_exact'])


def small_mk(T, n, outer=None):
    b = {k: np.ones(n, bool) for k in ('upper_eyelid_r', 'lower_eyelid_r', 'upper_eyelid_l', 'lower_eyelid_l', 'upper_lip', 'lower_lip', 'mouth_corner_l', 'mouth_corner_r')}
    return {'T': np.asarray(T), 'dup': [], 'bucket': b, 'outer_faces': np.ones(len(T), bool) if outer is None else np.asarray(outer), 'mouth_loop': np.array([], int)}


class TestClassifier(unittest.TestCase):
    def test_known_crossing_non_crossing_adjacent_and_coplanar(self):
        P = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0],            # A: z = 0 plane
                      [0.2, 0.2, -0.5], [0.5, 0.2, 0.5], [0.2, 0.5, 0.5],   # B: pierces A -> crossing
                      [0, 0, 1], [1, 0, 1], [0, 1, 1],            # C: parallel to A, 1 above -> no crossing
                      [0.5, 0.5, -0.5], [1.5, 0.5, 0.5],          # D shares vertex 1 with A and would pierce it -> adjacency exclusion
                      [0.1, 0.1, 0], [0.9, 0.1, 0], [0.1, 0.9, 0]], float)   # E: coplanar with A, overlapping -> unknown, never a crossing
        T = np.array([[0, 1, 2], [3, 4, 5], [6, 7, 8], [1, 9, 10], [11, 12, 13]]); mk = small_mk(T, len(P))
        self.assertEqual(LQt.lip_pairs(P, T, mk), [(0, 1), (1, 4)])                                                # B pierces A and the coplanar E; nothing else
        self.assertEqual(LQt.coplanar_pairs(P, T, mk), [(0, 4)])
        self.assertGreaterEqual(LQt.adjacency_exclusions(P, T, mk), 1)
        P2 = P.copy(); P2[3:6, 2] += 2.0                                                                        # lift B clear of A: no crossing
        self.assertEqual(LQt.lip_pairs(P2, T, mk), [])
        rows = LQt.classify_pairs(P2, P, T, mk, LQt.lip_pairs(P2, T, mk), LQt.lip_pairs(P, T, mk), block_ids=[0, 1, 2], blend_ids=[3, 4, 5])
        self.assertEqual([r['status'] for r in rows], ['introduced', 'introduced']); self.assertEqual(rows[0]['membership'], {'block': 3, 'blend': 3, 'mouth_loop': 0, 'other': 0}); self.assertEqual(rows[0]['region'], 'visible_lip_both_outer')
        rows2 = LQt.classify_pairs(P, P2, T, mk, LQt.lip_pairs(P, T, mk), LQt.lip_pairs(P2, T, mk)); self.assertEqual(rows2[0]['status'], 'removed')
        mk_in = small_mk(T, len(P), outer=[True, False, True, True, True]); rows3 = LQt.classify_pairs(P, P, T, mk_in, [(0, 1)], [(0, 1)]); self.assertEqual(rows3[0]['status'], 'inherited'); self.assertEqual(rows3[0]['region'], 'interior_roll_one_face_not_outer')

    def test_independent_predicate_agrees_on_known_cases(self):
        P = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0.2, 0.2, -0.5], [0.5, 0.2, 0.5], [0.2, 0.5, 0.5], [0, 0, 1], [1, 0, 1], [0, 1, 1], [0.1, 0.1, 0], [0.9, 0.1, 0], [0.1, 0.9, 0]], float)
        self.assertTrue(LQt.independent_tri_tri(*P[[0, 1, 2]], *P[[3, 4, 5]])); self.assertFalse(LQt.independent_tri_tri(*P[[0, 1, 2]], *P[[6, 7, 8]])); self.assertFalse(LQt.independent_tri_tri(*P[[0, 1, 2]], *P[[9, 10, 11]]))   # crossing / parallel / coplanar
        T = np.array([[0, 1, 2], [3, 4, 5], [6, 7, 8], [9, 10, 11]]); mk = small_mk(T, len(P)); self.assertEqual(LQt.independent_pairs(P, T, np.arange(4)), [(0, 1), (1, 3)]); self.assertEqual(LQt.independent_pairs(P, T, np.arange(4)), LQt.lip_pairs(P, T, mk))

    def test_repair_block_crossings_no_targets_is_a_no_op(self):
        P = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1], [1, 0, 1], [0, 1, 1]], float); T = np.array([[0, 1, 2], [3, 4, 5]]); mk = small_mk(T, 6)
        f, rep = LQt.repair_block_crossings(P + 1.5, P + 1.5, T, mk, [0, 1, 2], [3, 4, 5], lambda x: x, lambda x: x)
        self.assertTrue(rep['resolved']); self.assertEqual(rep['records_pushed'], 0); self.assertTrue(np.array_equal(f, P + 1.5))


class TestFitQuantOptions(unittest.TestCase):
    def setUp(self):
        self.ex = LX.from_native_structure(synthetic_entry()); self.src = synthetic_cc5(dz=0.02)

    def test_native_half_grid_keeps_block_rigid_and_representable_on_uniform_axes(self):
        f, m, r, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='preserve_donor_lips', lip_block_rings=2, lip_block_grid='native_half'); P = mk['P']
        lb = r['lip_block']; self.assertEqual(lb['grid'], 'native_half'); block = np.flatnonzero(m == 'lip_block')
        rel = (f[block] - f[block].mean(0)) - (P[block] - P[block].mean(0)); self.assertLess(np.abs(rel).max(), 1e-9)
        t_native = G.blender_to_native(np.array(lb['translation_m'])[None])[0]
        for ax in lb['grid_translation']['axes']:
            self.assertAlmostEqual(t_native[ax['axis']] / ax['ulp_m'], round(t_native[ax['axis']] / ax['ulp_m']), places=9)                      # every axis is an integer multiple of the block's coarsest ulp
            if ax['all_representable']: self.assertTrue(LQt.is_representable(G.blender_to_native(f[block])[:, ax['axis']]).all())
        self.assertLess(lb['grid_translation']['adjust_mm'], 0.5 * 0.9765625 * np.sqrt(3) + 1e-6)
        self.assertEqual(r['attachment_max_error_m'], 0.0); self.assertEqual(r['seam_max_gap_m'], 0.0)
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='preserve_donor_lips', lip_block_grid='nonsense')

    def test_graded_blend_weights_and_bounds(self):
        f, m, r, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='preserve_donor_lips', lip_block_rings=2, lip_block_blend_mode='graded', lip_block_blend_rings=3)
        lb = r['lip_block']; self.assertEqual(lb['blend_mode'], 'graded'); self.assertEqual(lb['graded_weights_by_ring'], {'1': 0.75, '2': 0.5, '3': 0.25}); self.assertGreater(r['modes']['lip_block_blend'], 0)
        self.assertEqual(r['attachment_max_error_m'], 0.0); self.assertEqual(r['seam_max_gap_m'], 0.0)
        f0, m0, r0, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='preserve_donor_lips', lip_block_rings=2)
        block = np.flatnonzero(m == 'lip_block'); self.assertTrue(np.array_equal(np.flatnonzero(m0 == 'lip_block'), block))                                     # the block itself is unchanged by the blend mode
        blend = np.flatnonzero(m == 'lip_block_blend'); self.assertGreater(np.abs(f[blend] - f0[blend]).max(), 0)                                                   # only the blend ring differs
        outside = np.flatnonzero((m != 'lip_block') & (m != 'lip_block_blend')); self.assertTrue(np.array_equal(f[outside], f0[outside]))
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='preserve_donor_lips', lip_block_blend_mode='nonsense')
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='preserve_donor_lips', lip_blend_separation='nonsense')


if __name__ == '__main__':
    unittest.main()
