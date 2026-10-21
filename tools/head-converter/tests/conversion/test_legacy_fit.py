"""Synthetic tests for the bounded Legacy fit core: alignment, TPS solve/warp round trip, surface correspondence,
Legacy-specific registration, protected attachment/seams, mouth-gap policy, rigid companion assembly.
Run: python -m unittest tests.conversion.test_legacy_fit
"""
import json, sys, unittest
from pathlib import Path
import numpy as np
REPO = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(REPO))
from src.conversion import legacy_exchange as LX, legacy_fit as LF, uv_fit as F, geometry as G
from tests.conversion.test_legacy_exchange import synthetic_entry, open_head, cap


def synthetic_cc5(scale=1.06, dz=0.0):
    """CC5-like source in the Blender frame (Z up, -Y forward): a scaled copy of the synthetic Legacy head plus an eye
    mesh with the four CC5 materials (two spheres), matching cc5_source's access functions."""
    P, T, U, J, W = open_head(nz=14, na=32, y0=1.50, y1=1.95, scalp_rows=0, mouth=False)
    Pb = G.native_to_blender(P) * np.array([scale, scale, 1.0]) + np.array([0, 0, dz])
    head = {'name': '0', 'positions': Pb.tolist(), 'uv': np.c_[U[:, 0], 1 - U[:, 1]].tolist(), 'triangles': T.tolist(), 'triangle_materials': [0] * len(T)}
    rng = np.random.default_rng(5); u = rng.normal(size=(30, 3)); u /= np.linalg.norm(u, axis=1)[:, None]
    def sphere(c, r): return np.asarray(c) + r * u
    cR = G.native_to_blender(np.array([[-0.03, 1.83, 0.08]]))[0] + [0, 0, dz]; cL = G.native_to_blender(np.array([[0.03, 1.83, 0.08]]))[0] + [0, 0, dz]
    E = np.vstack([sphere(cR, 0.015), sphere(cR, 0.0151), sphere(cL, 0.015), sphere(cL, 0.0151)])
    ET = np.array([[i, (i + 1) % 30, (i + 2) % 30] for i in range(0, 28, 2)]); ET = np.vstack([ET, ET + 30, ET + 60, ET + 90]); mats = [0] * 14 + [1] * 14 + [2] * 14 + [3] * 14
    return [head, {'name': 'cc_base_eye', 'positions': E.tolist(), 'uv': np.zeros((120, 2)).tolist(), 'triangles': ET.tolist(), 'triangle_materials': mats}]


class TestLegacyFit(unittest.TestCase):
    def setUp(self):
        self.ex = LX.from_native_structure(synthetic_entry()); self.src = synthetic_cc5(dz=0.02)

    def test_alignment_and_tps_round_trip(self):
        SE, r = LF.eye_centres_cc5(self.src); DE, rl = LF.eye_centres_legacy(self.ex); shift = LF.align_shift(SE, DE)
        self.assertAlmostEqual(shift[2], -0.02, places=4); self.assertAlmostEqual(r, 0.015, places=3)
        rng = np.random.default_rng(1); a = rng.random((20, 2)); t = a * 0.9 + 0.05 + 0.01 * np.sin(a * 6)
        coef = LF.tps_from_pairs(a, t); self.assertLess(np.abs(F.tps_warp(a, coef, a) - t).max(), 1e-6)   # interpolates its anchors
        sub = LF.farthest_point_subset(a, 5); self.assertEqual(len(set(sub.tolist())), 5)

    def test_surface_registration_recovers_uv_map(self):
        P, T, U = LX.head_arrays(self.ex); SE, _ = LF.eye_centres_cc5(self.src); DE, _ = LF.eye_centres_legacy(self.ex); shift = LF.align_shift(SE, DE)
        from src.conversion import cc5_source as C
        S0, ST, SU = C.head_arrays(self.src); SU_td = np.c_[SU[:, 0], 1 - SU[:, 1]]; S = S0 + shift
        Q, dist, K, uv = LF.surface_correspondence(P, S, ST, SU_td); self.assertLess(np.median(dist), 0.02)
        eligible = np.ones(len(P), bool); anchors, coef, rep = LF.registration_from_surface(U, uv, dist, eligible, n_anchors=24, max_dist=0.05)
        self.assertEqual(rep['anchors'], 24); self.assertLess(rep['held_out_residual_px_at_4096']['median'], 120)      # same cylinder parametrisation: small residual

    def test_fit_protects_attachment_and_seams_and_reports(self):
        for reg in ({'kind': 'surface'},):
            fitp, mode, rep, mk = LF.fit(self.ex, self.src, reg, mouth_gap_policy='follow_source')
            self.assertEqual(rep['attachment_max_error_m'], 0.0); self.assertEqual(rep['seam_max_gap_m'], 0.0)
            self.assertTrue(np.array_equal(fitp[mk['frozen']], mk['P'][mk['frozen']])); self.assertGreater(rep['max_move_vs_donor_mm'], 0)
            for k in ('modes', 'triangle_quality_vs_donor', 'fit_metrics_excluding_lash_faces', 'inverted_by_region', 'registration'): self.assertIn(k, rep)
            m = LF.measure(self.ex, fitp, mode, mk, self.src, np.array(rep['shift_m']))
            for k in ('likeness', 'lips', 'eyelids', 'jaw_neck', 'coverage', 'rear_opening_vs_haircap'): self.assertIn(k, m)
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='nonsense')

    def test_mouth_gap_policy_preserves_loop_shape(self):
        f1, m1, r1, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='preserve_donor_gap'); ml = mk['mouth_loop']; P = mk['P']
        rel_fit = f1[ml] - f1[ml].mean(0); rel_don = P[ml] - P[ml].mean(0); self.assertLess(np.abs(rel_fit - rel_don).max(), 1e-9)   # rigid loop
        self.assertEqual(r1['modes']['mouth_rigid'], len(ml))
        f2, m2, r2, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='preserve_donor_gap', lip_relax_rings=2, lip_relax_passes=2)
        self.assertGreater(r2['modes']['lip_relaxed'], 0); self.assertEqual(r2['attachment_max_error_m'], 0.0)

    def test_assemble_moves_companions_rigidly_only(self):
        fitp, mode, rep, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}); shift = np.array(rep['shift_m'])
        cand, moves = LF.assemble(self.ex, fitp, self.src, shift, mk)
        ok, problems, rigid = LX.untouched_fields_equal(cand, self.ex); self.assertTrue(ok, problems)
        self.assertIn('eye_r', rigid); self.assertIn('mouthbag', rigid); self.assertEqual(moves['haircap']['policy'], 'untouched native geometry'); self.assertEqual(moves['eye_r']['scale_applied'], 1.0)
        SE, r = LF.eye_centres_cc5(self.src); SE = SE + shift
        from src.conversion.eye_fit import sphere_fit
        c, rr, _ = sphere_fit(G.native_to_blender(LX.submesh(cand, 'eye_r')['positions'])); self.assertLess(np.linalg.norm(c - SE[0]), 1e-6); self.assertAlmostEqual(rr, 0.016, places=4)
        cand2, moves2 = LF.assemble(self.ex, fitp, self.src, shift, mk, scale_eyes=True); ok2, problems2, _ = LX.untouched_fields_equal(cand2, self.ex); self.assertFalse(ok2)   # scaled eyes are not a rigid translation: flagged
        okv, pv, _ = LX.validate(cand); self.assertTrue(okv, pv)


def socket_fixture(n_rim=6):
    """Synthetic entry whose head carries an eye-socket rim per side: the `n_rim` head records nearest each donor cap centre
    are placed ON that cap sphere (donor signed distance 0), one more is sunk 10 mm inside (the socket-fan apex), and all
    of them get atlas lash-band UVs (u in the side's band, v inside the band). Returns (exchange, {'r': ids, 'l': ids})."""
    from tests.conversion.test_legacy_exchange import synthetic_entry as se
    from src.conversion.masks import ATLAS_POLICY_NHL_HEAD_A as POL
    entry = se(); head = next(m for m in entry['submeshes'] if m['name'] == 'head_.Mesh'); P = np.array(head['positions']); U = np.array(head['uv']); ids = {}
    for name, side, ub in (('eyer_.Mesh', 'r', POL['lash_band']['u'][0]), ('eyel_.Mesh', 'l', POL['lash_band']['u'][1])):
        cap = np.array(next(m for m in entry['submeshes'] if m['name'] == name)['positions']); c, rad, _ = __import__('src.conversion.eye_fit', fromlist=['sphere_fit']).sphere_fit(cap)   # the donor cap's own sphere
        rng = np.random.default_rng(7 if side == 'r' else 8); u = rng.normal(size=(n_rim + 1, 3)); u /= np.linalg.norm(u, axis=1)[:, None]; u[:, 2] = np.abs(u[:, 2]) + 0.4; u /= np.linalg.norm(u, axis=1)[:, None]
        pool = np.flatnonzero((P[:, 0] < 0) if side == 'r' else (P[:, 0] > 0)); near = pool[np.argsort(np.linalg.norm(P[pool] - c, axis=1))[:n_rim + 1]]     # that side's records only
        P[near[:n_rim]] = c + rad * u[:n_rim]; P[near[n_rim]] = c + (rad - 0.010) * u[n_rim]      # rim on the sphere; apex 10 mm inside
        U[near, 0] = np.linspace(ub[0] + 0.005, ub[1] - 0.005, n_rim + 1); U[near, 1] = 0.328; ids[side] = sorted(int(i) for i in near)
    head['positions'] = P.tolist(); head['uv'] = U.tolist(); return entry, ids


class TestSocketRim(unittest.TestCase):
    def setUp(self):
        entry, self.ids = socket_fixture(); self.ex = LX.from_native_structure(entry); self.src = synthetic_cc5(dz=0.02); self.mk = LF.legacy_masks(self.ex)

    def test_socket_rim_sets_classify_by_geometry_and_side(self):
        sets = LF.socket_rim_sets(self.ex, self.mk, tol_mm=1.0)
        for side in ('r', 'l'):
            s = sets['sides'][side]; self.assertEqual(sorted(s['ids']), self.ids[side]); self.assertEqual(len(s['on_cap']), 6); self.assertEqual(len(s['interior']), 1)
            self.assertLess(max(abs(s['donor_signed_mm'][i]) for i in s['on_cap']), 1e-6); self.assertAlmostEqual(s['donor_signed_mm'][s['interior'][0]], -10.0, places=6)
        self.assertIn('character right (native x < 0)', sets['sides']['r']['character_side']); self.assertTrue(all(self.mk['P'][i, 0] < 0 for i in sets['sides']['r']['ids'])); self.assertTrue(all(self.mk['P'][i, 0] > 0 for i in sets['sides']['l']['ids']))
        self.assertIn('unbanded_on_cap', sets)

    def test_preserve_donor_radius_restores_both_sides_and_touches_nothing_else(self):
        base, m0, r0, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}); fitp, mode, rep, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, socket_rim_policy='preserve_donor_radius')
        self.assertIsNone(r0['socket_rim']); self.assertEqual(rep['socket_rim']['policy'], 'preserve_donor_radius'); shift = np.array(rep['shift_m']); SE, _ = LF.eye_centres_cc5(self.src)
        all_rim = self.ids['r'] + self.ids['l']; others = np.setdiff1d(np.arange(len(base)), all_rim)
        self.assertTrue(np.array_equal(fitp[others], base[others]))                       # only rim records move
        for side, cidx, role in (('r', 0, 'eye_r'), ('l', 1, 'eye_l')):
            from src.conversion.eye_fit import sphere_fit
            c0, rr, _ = sphere_fit(G.native_to_blender(LX.submesh(self.ex, role)['positions'])); c = SE[cidx] + shift
            for i in self.ids[side]:
                self.assertAlmostEqual(np.linalg.norm(fitp[i] - c), np.linalg.norm(mk['P'][i] - c0), places=9)   # donor radius relation exactly
                self.assertEqual(mode[i], 'socket_rim')
            s = rep['socket_rim']['sides'][side]; self.assertEqual(s['records'], 7); self.assertEqual(s['on_cap'], 6); self.assertEqual(s['interior'], 1)
        self.assertEqual(rep['modes']['socket_rim'], 14); self.assertEqual(rep['attachment_max_error_m'], 0.0)
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, socket_rim_policy='nonsense')

    def test_socket_rings_scope_clears_rows_and_reports(self):
        f1, m1, r1, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=1.0, lid_clearance_scope='socket_rings', socket_rings=2, socket_rim_policy='preserve_donor_radius')
        lid = r1['lid_clearance']; self.assertEqual(lid['scope'], 'socket_rings'); self.assertIn('socket_ring_records', lid); self.assertGreater(lid['socket_ring_records']['eye_r'], 0)
        from src.conversion import legacy_qualify as LQ
        cand, _ = LF.assemble(self.ex, f1, self.src, np.array(r1['shift_m']), mk); soc = LQ.socket_contact(self.ex, cand, mk, rings=2, tol_mm=1.0)
        for side in ('r', 'l'):
            v = soc['sides'][side]; self.assertLess(v['rim_donor_relation_deviation_mm']['max'], 1e-6); self.assertEqual(v['ring']['inside_actual_cap_surface'], 0); self.assertEqual(v['on_cap_rim_inside_placed_sphere_beyond_tol'], 0)
        self.assertEqual(set(soc['summary']), {'rim_records_inside_actual_cap', 'ring_records_inside_actual_cap', 'on_cap_rim_inside_beyond_tol', 'rim_donor_relation_max_dev_mm'})
        # the frozen recipe (buckets scope, no rim policy) leaves the rim inside the smaller CC5-placed sphere: the screen must say so
        f0, m0, r0, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=1.0); cand0, _ = LF.assemble(self.ex, f0, self.src, np.array(r0['shift_m']), mk); soc0 = LQ.socket_contact(self.ex, cand0, mk)
        self.assertGreater(max(soc0['summary']['rim_donor_relation_max_dev_mm'].values()), 0.5)
        sc = LQ.screen_set(self.ex, cand, mk); self.assertIn('socket', sc); self.assertEqual(sc['socket'], soc['summary'])
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=1.0, lid_clearance_scope='nonsense')


def card_fixture():
    """socket_fixture plus two eyelash cards on the haircap: separate 6-record strips 2 mm outside each donor cap, weighted on
    the side's eye_control palette (joints = 3 * palette), hair UVs in the card region."""
    entry, ids = socket_fixture(); hc = next(m for m in entry['submeshes'] if m['name'] == 'haircap_.Mesh')
    HP = np.array(hc['positions']); HT = np.vstack([np.array(hc['triangles']), [[len(HP) - 3, len(HP) - 2, len(HP) - 1]]]); n0 = len(HP); cards = {}   # the synthetic shell's last record is otherwise isolated
    for name, side in (('eyer_.Mesh', 'r'), ('eyel_.Mesh', 'l')):
        cap = np.array(next(m for m in entry['submeshes'] if m['name'] == name)['positions']); c, rad, _ = __import__('src.conversion.eye_fit', fromlist=['sphere_fit']).sphere_fit(cap)
        xs = np.linspace(-0.008, 0.008, 3); pts = np.array([[c[0] + x, c[1] + 0.006 + dy, c[2] + rad + 0.002] for dy in (0.0, 0.003) for x in xs]); k = len(HP); HP = np.vstack([HP, pts])
        HT = np.vstack([HT, [[k, k + 1, k + 4], [k, k + 4, k + 3], [k + 1, k + 2, k + 5], [k + 1, k + 5, k + 4]]]); cards[side] = list(range(k, k + 6))
    pal = 24 if True else 23; J = np.array(hc['joints']); W = np.array(hc['weights']); J = np.vstack([J, np.full((12, 4), 0)]); W = np.vstack([W, np.c_[np.ones(12), np.zeros((12, 3))]])
    J[n0:n0 + 6, 0] = 24 * 3; J[n0 + 6:n0 + 12, 0] = 23 * 3          # eye_control_r / eye_control_l palettes (joint = 3 * palette)
    card_uv = [[u, v] for v in (0.07, 0.2) for u in (0.902, 0.99, 0.902)] * 2      # 3 columns x 2 rows per card like the real strips: row = v, column = u alternating, root row v = 0.07
    hc.update(positions=HP.tolist(), triangles=HT.tolist(), joints=J.tolist(), weights=W.tolist(), uv=np.vstack([np.array(hc['uv']), np.array(card_uv)]).tolist()); return entry, ids, cards


class TestLashCards(unittest.TestCase):
    def setUp(self):
        entry, self.ids, self.cards = card_fixture(); self.ex = LX.from_native_structure(entry); self.src = synthetic_cc5(dz=0.02); self.mk = LF.legacy_masks(self.ex)

    def test_card_classification_and_refusals(self):
        c = LF.haircap_lash_components(self.ex, self.mk); self.assertEqual(c['r'], self.cards['r']); self.assertEqual(c['l'], self.cards['l']); self.assertEqual(c['components'], 3)
        # a small component without eye palettes is refused, not silently treated as a card
        entry, _, cards = card_fixture(); hc = next(m for m in entry['submeshes'] if m['name'] == 'haircap_.Mesh')
        for i in cards['r']: hc['joints'][i] = [0, 0, 0, 0]
        with self.assertRaises(ValueError): LF.haircap_lash_components(LX.from_native_structure(entry), self.mk)
        # a card far from both caps is refused
        entry, _, cards = card_fixture(); hc = next(m for m in entry['submeshes'] if m['name'] == 'haircap_.Mesh')
        for i in cards['l']: hc['positions'][i][1] += 0.2
        with self.assertRaises(ValueError): LF.haircap_lash_components(LX.from_native_structure(entry), self.mk)
        with self.assertRaises(ValueError): LF.move_lash_cards(self.ex, self.mk['P'], self.mk, 'nonsense')

    def test_cards_follow_the_lid_and_the_shell_stays(self):
        fitp, mode, rep, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}); shift = np.array(rep['shift_m']); HP = G.native_to_blender(LX.submesh(self.ex, 'haircap')['positions'])
        shell = np.setdiff1d(np.arange(len(HP)), self.cards['r'] + self.cards['l'])
        for policy in ('follow_lid_record', 'follow_lid_rigid', 'follow_cap_rigid'):
            cand, moves = LF.assemble(self.ex, fitp, self.src, shift, mk, haircap_lash_policy=policy); HPn = G.native_to_blender(LX.submesh(cand, 'haircap')['positions'])
            self.assertTrue(np.array_equal(HPn[shell], HP[shell]), policy)                                   # hair shell untouched
            if policy != 'follow_cap_rigid':                                                                                              # (the synthetic caps already sit on the CC5 centres: zero cap translation)
                for s in ('r', 'l'): self.assertGreater(np.linalg.norm(HPn[self.cards[s]] - HP[self.cards[s]], axis=1).max(), 1e-4, policy)   # both cards moved
            self.assertIn(policy, moves['haircap']['policy']); self.assertIn('sides', moves['haircap'])
            ok, problems, _ = LX.untouched_fields_equal(cand, self.ex, movable_roles=('head', 'haircap')); self.assertTrue(ok, problems)
        cand_cap, moves = LF.assemble(self.ex, fitp, self.src, shift, mk, haircap_lash_policy='follow_cap_rigid'); HPc = G.native_to_blender(LX.submesh(cand_cap, 'haircap')['positions'])
        t = np.array(moves['eye_r']['translation_m']); self.assertLess(np.abs((HPc[self.cards['r']] - HP[self.cards['r']]) - t).max(), 1e-9)   # exactly the cap translation
        cand_rig, moves = LF.assemble(self.ex, fitp, self.src, shift, mk, haircap_lash_policy='follow_lid_rigid'); HPr = G.native_to_blender(LX.submesh(cand_rig, 'haircap')['positions'])
        d0 = np.linalg.norm(HP[self.cards['l']][:, None] - HP[self.cards['l']][None], axis=2); d1 = np.linalg.norm(HPr[self.cards['l']][:, None] - HPr[self.cards['l']][None], axis=2); self.assertLess(np.abs(d0 - d1).max(), 1e-9)   # rigid: card shape preserved
        cand_u, moves = LF.assemble(self.ex, fitp, self.src, shift, mk); self.assertEqual(LX.submesh(cand_u, 'haircap')['positions'], LX.submesh(self.ex, 'haircap')['positions'])   # default untouched

    def test_card_structure_expectations_and_column_rigid_policy(self):
        hc = LX.submesh(self.ex, 'haircap'); HP = G.native_to_blender(hc['positions']); HT = np.array(hc['triangles']); HU = np.array(hc['uv'])
        st = LF.lash_card_structure(HP, HT, HU, self.cards['r']); self.assertEqual(len(st['rows_v']), 2); self.assertEqual(len(st['columns']), 3); self.assertEqual(sorted(st['roots']), self.cards['r'][:3]); self.assertEqual(sorted(st['tips']), self.cards['r'][3:])
        expect = {'profile': 'synthetic', 'haircap_components': 3, 'cards': {'r': {'records': 6, 'faces': 4, 'native_x_sign': -1}, 'l': {'records': 6, 'faces': 4, 'native_x_sign': +1}}, 'rows': 2, 'columns': 3}
        c = LF.haircap_lash_components(self.ex, self.mk, expect=expect); self.assertEqual(c['expectations'], expect)
        for bad in ({**expect, 'haircap_components': 2}, {**expect, 'rows': 4}, {**expect, 'columns': 8}, {**expect, 'cards': {**expect['cards'], 'r': {'records': 32, 'faces': 42, 'native_x_sign': -1}}}, {**expect, 'cards': {**expect['cards'], 'r': {'records': 6, 'faces': 4, 'native_x_sign': +1}}}):
            with self.assertRaises(ValueError): LF.haircap_lash_components(self.ex, self.mk, expect=bad)
        with self.assertRaises(ValueError): LF.haircap_lash_components(self.ex, self.mk, expect=LF.LASH_CARD_EXPECTATIONS)      # the player_67 pins refuse the synthetic donor
        # a broken strip (a tip record's u no longer matches its root) is refused by the structure rule
        HU2 = HU.copy(); HU2[self.cards['r'][3], 0] = 0.5
        with self.assertRaises(ValueError): LF.lash_card_structure(HP, HT, HU2, self.cards['r'])
        fitp, mode, rep, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}); shift = np.array(rep['shift_m']); shell = np.setdiff1d(np.arange(len(HP)), self.cards['r'] + self.cards['l'])
        cand, moves = LF.assemble(self.ex, fitp, self.src, shift, mk, haircap_lash_policy='follow_lid_column_rigid', cards=c); HPn = G.native_to_blender(LX.submesh(cand, 'haircap')['positions'])
        self.assertTrue(np.array_equal(HPn[shell], HP[shell]))
        cand_r, _ = LF.assemble(self.ex, fitp, self.src, shift, mk, haircap_lash_policy='follow_lid_record', cards=c); HPr = G.native_to_blender(LX.submesh(cand_r, 'haircap')['positions'])
        for col in st['columns']:
            self.assertLess(np.linalg.norm(HPn[col[0]] - HPr[col[0]]), 1e-9)                                                                            # roots at the per-record lid targets
            self.assertAlmostEqual(np.linalg.norm(HPn[col[1]] - HPn[col[0]]), np.linalg.norm(HP[col[1]] - HP[col[0]]), places=12)                        # each column rigid (length kept)
        self.assertEqual(moves['haircap']['sides']['r']['columns'], 3)
        # column_fit: roots at the per-record targets, column shape rigid (lengths kept), rotation fitted to the tips' targets
        cand_f, mv_f = LF.assemble(self.ex, fitp, self.src, shift, mk, haircap_lash_policy='follow_lid_column_fit', cards=c); HPf = G.native_to_blender(LX.submesh(cand_f, 'haircap')['positions'])
        self.assertTrue(np.array_equal(HPf[shell], HP[shell]))
        for col in st['columns']:
            self.assertLess(np.linalg.norm(HPf[col[0]] - HPr[col[0]]), 1e-9); self.assertAlmostEqual(np.linalg.norm(HPf[col[1]] - HPf[col[0]]), np.linalg.norm(HP[col[1]] - HP[col[0]]), places=12)
            self.assertLessEqual(np.linalg.norm(HPf[col[1]] - HPr[col[1]]), np.linalg.norm(HPn[col[1]] - HPr[col[1]]) + 1e-9)     # never further from the per-record target than the face-frame column-rigid tip
        # record_bounded: roots at targets; every tip within the declared length ratio of the donor column length
        cand_b, mv_b = LF.assemble(self.ex, fitp, self.src, shift, mk, haircap_lash_policy='follow_lid_record_bounded', cards=c); HPb = G.native_to_blender(LX.submesh(cand_b, 'haircap')['positions'])
        self.assertTrue(np.array_equal(HPb[shell], HP[shell])); self.assertIn('records_clamped', mv_b['haircap']['sides']['r'])
        for col in st['columns']:
            self.assertLess(np.linalg.norm(HPb[col[0]] - HPr[col[0]]), 1e-9); ratio = np.linalg.norm(HPb[col[1]] - HPb[col[0]]) / np.linalg.norm(HP[col[1]] - HP[col[0]])
            self.assertTrue(1 - LF.LASH_BOUNDS['length_ratio'] - 1e-9 <= ratio <= 1 + LF.LASH_BOUNDS['length_ratio'] + 1e-9, ratio)
        m = LF.lash_card_metrics(self.ex, self.ex, mk, c, placed_caps=None)
        for s_ in ('r', 'l'): self.assertEqual(m[s_]['column_length_ratio_vs_donor'], {'min': 1.0, 'max': 1.0}); self.assertEqual(m[s_]['column_angle_to_skin_normal_deg']['max_abs_change'], 0.0); self.assertEqual(len(m[s_]['root_embedding_mm']['donor']), 3)
        m2 = LF.lash_card_metrics(self.ex, cand, mk, c, placed_caps={s_: (np.zeros(3), 0.01) for s_ in ('r', 'l')})
        for s_ in ('r', 'l'): self.assertIn('min_signed_mm', m2[s_]['cap_clearance']); self.assertGreater(m2[s_]['move_mm']['max'], 0)

    def test_scale_eye_regions_and_cap_override(self):
        SE, r = LF.eye_centres_cc5(self.src); k = 1.1; out, rep = LF.scale_eye_regions(self.src, SE, k, r0_m=0.017, R_m=0.025)
        self.assertEqual(rep['k'], 1.1); self.assertGreater(rep['vertices_moved'], 0); self.assertEqual(len(out), len(self.src))
        for m0, m1 in zip(self.src, out):
            X0 = np.array(m0['positions']); X1 = np.array(m1['positions']); d = np.linalg.norm(X0[:, None] - SE[None], axis=2).min(1)
            self.assertTrue(np.array_equal(X0[d > 0.025], X1[d > 0.025]))                                                                                 # untouched beyond R
            inner = d < 0.017
            if inner.any():
                c = SE[np.linalg.norm(X0[inner][:, None] - SE[None], axis=2).argmin(1)]; self.assertLess(np.abs(np.linalg.norm(X1[inner] - c, axis=1) / np.linalg.norm(X0[inner] - c, axis=1) - k).max(), 1e-9)   # exact k in the core
        SE2, r2 = LF.eye_centres_cc5(out); self.assertLess(np.abs(SE2 - SE).max(), 1e-6); self.assertAlmostEqual(r2 / r, k, places=6)                  # centres kept, eyeball radius scaled
        DE, _ = LF.eye_centres_legacy(self.ex); shift = LF.align_shift(SE, DE); override = (SE + shift + np.array([[0, 0, 0.002], [0, 0, -0.002]])).tolist()
        fitp, mode, rep, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}, cap_centre_override=override, lid_clearance_mm=1.0); cand, moves = LF.assemble(self.ex, fitp, self.src, shift, mk, cap_centre_override=override)
        from src.conversion.eye_fit import sphere_fit
        for role, cidx in (('eye_r', 0), ('eye_l', 1)):
            c, _, _ = sphere_fit(G.native_to_blender(LX.submesh(cand, role)['positions'])); self.assertLess(np.abs(c - np.array(override[cidx])).max(), 1e-6)   # caps land on the declared centres
        self.assertEqual(rep['lid_clearance']['placement'], 'cc5_centre')

    def test_constrained_socket_policy(self):
        f, m, r, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=None, socket_rim_policy='constrained', socket_con_gradcheck=True, socket_con_iters=150)
        self.assertEqual(r['socket_rim']['policy'], 'constrained'); self.assertEqual(r['attachment_max_error_m'], 0.0); self.assertEqual(r['seam_max_gap_m'], 0.0); self.assertGreater(r['modes']['socket_ring'], 0)
        for s in ('r', 'l'):
            sd = r['socket_rim']['sides'][s]; self.assertLess(sd['gradcheck']['max_rel_error'], 1e-4, sd['gradcheck'])                                     # analytic gradient == finite differences
            self.assertIn(sd['stopping_reason'], ('relative energy decrease below tol 1e-07 on 3 consecutive full steps', 'iteration budget', 'gradient below 1e-9', 'line search failed along steepest descent (energy stationary at the tolerance of the search)')); self.assertIn('memory_resets', sd['line_search'])
            self.assertLess(sd['energy_end'], sd['energy_start']); self.assertLess(sd['residuals']['rim_radius_max_mm'], 0.5); self.assertEqual(sd['residuals']['floor_violations'], 0)
            self.assertGreaterEqual(sd['residuals']['orient_min_dot_reached'], 0.5 - 0.05); self.assertGreaterEqual(sd['residuals']['area_min_ratio_reached'], 0.5 - 0.05); self.assertGreater(sd['support']['K'], 0); self.assertEqual(sd['rim_records'], 7)
        # crease term on: every support/surround edge stays inside its donor / fit-input envelope; categorised gradient checks recorded; warm start accepted
        f3, m3, r3, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=None, socket_rim_policy='constrained', socket_con_w_crease=100.0, socket_con_crease_margin_deg=10.0, socket_con_gradcheck=True, socket_con_iters=150)
        for s in ('r', 'l'):
            sd = r3['socket_rim']['sides'][s]; s0 = r['socket_rim']['sides'][s]; self.assertGreater(sd['crease']['edges'], 0); self.assertEqual(sd['init'], 'fit input')
            self.assertLessEqual(sd['crease']['max_excess_deg_at_end'], max(s0['crease']['max_excess_deg_at_end'], 0.5)); self.assertLessEqual(sd['crease']['outside_envelope_at_end'], s0['crease']['outside_envelope_at_end'])   # the term never leaves more crease excess than the uncoupled solve
            for pt in ('start', 'perturbed'):
                for cat in ('random', 'active_hinges', 'small_area_faces'): v = sd['gradcheck']['points'][pt][cat]; self.assertTrue(v['max_rel_error'] < 1e-4 or v['max_abs_error'] < 1e-4, (pt, cat, v))
        f4, m4, r4, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=None, socket_rim_policy='constrained', socket_con_w_crease=100.0, socket_con_init=f3.tolist(), socket_con_iters=20)
        self.assertEqual(r4['socket_rim']['sides']['r']['init'], 'declared warm start'); self.assertLessEqual(r4['socket_rim']['sides']['l']['energy_start'], r3['socket_rim']['sides']['l']['energy_end'] + 1e-6)   # the side solved last restarts at exactly its own end energy (the synthetic head's two regions overlap, so the first side is re-moved by the second: reported)
        self.assertIn('region_overlap_with_other_side', r4['socket_rim']['sides']['l'])
        # storage-lattice stage: region records end exactly representable in native half storage, and the lattice energy never exceeds the plain snap
        from src.conversion import legacy_quant as LQt
        f5, m5, r5, mk5 = LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=None, socket_rim_policy='constrained', socket_con_w_crease=100.0, socket_con_iters=100, socket_con_lattice=True, socket_con_lattice_passes=4)
        reg_ids = np.flatnonzero(np.isin(m5, ('socket_rim', 'socket_ring'))); self.assertTrue(LQt.is_representable(G.blender_to_native(f5[reg_ids]), atol=1e-12).all())
        for s in ('r', 'l'):
            lat = r5['socket_rim']['sides'][s]['lattice']; self.assertLessEqual(lat['energy_lattice'], lat['energy_snapped'] + 1e-9); self.assertGreaterEqual(lat['passes'], 1); self.assertEqual(len(lat['energy_by_pass']), lat['passes'] + 1)
            self.assertTrue(all(b <= a + 1e-9 for a, b in zip(lat['energy_by_pass'], lat['energy_by_pass'][1:])))     # monotone coordinate descent
        self.assertIsNone(r3['socket_rim']['sides']['r']['lattice'])
        f2, m2, r2, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=None, socket_rim_policy='constrained', socket_con_rings=2, socket_con_iters=50)
        for s in ('r', 'l'): self.assertEqual(r2['socket_rim']['sides'][s]['support']['K'], 2); self.assertEqual(r2['socket_rim']['sides'][s]['support']['rule'], 'declared K')
        self.assertIn('constrained', LF.SOCKET_RIM_POLICIES)

    def test_cap_radius_scale_declared_isotropic_about_the_cap_centre(self):
        from src.conversion.eye_fit import sphere_fit
        base, m0, r0, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}); shift = np.array(r0['shift_m']); SE, _ = LF.eye_centres_cc5(self.src)
        cand, moves = LF.assemble(self.ex, base, self.src, shift, mk, cap_radius_scale=[0.9, 0.95])
        for role, cidx, sc in (('eye_r', 0, 0.9), ('eye_l', 1, 0.95)):
            Pe = G.native_to_blender(LX.submesh(self.ex, role)['positions']); Pn = G.native_to_blender(LX.submesh(cand, role)['positions']); c0, rr0, _ = sphere_fit(Pe); c1, rr1, _ = sphere_fit(Pn)
            self.assertAlmostEqual(rr1 / rr0, sc, places=9); self.assertLess(np.linalg.norm(c1 - (SE[cidx] + shift)), 1e-6)                                   # radius scaled, centre on the placed CC5 centre
            self.assertLess(np.abs((Pn - c1) - (Pe - c0) * sc).max(), 1e-9); self.assertEqual(moves[role]['scale_applied'], sc); self.assertIn('DECLARED', moves[role]['policy'])   # exactly the similarity: angular shape kept
        ok, problems, mv = LX.untouched_fields_equal(cand, self.ex, declared_scales={'eye_r': 0.9, 'eye_l': 0.95}); self.assertTrue(ok, problems); self.assertEqual(mv['eye_r']['kind'], 'declared_similarity')
        ok0, _, _ = LX.untouched_fields_equal(cand, self.ex); self.assertFalse(ok0)                                                                                   # undeclared: still refused
        okv, pv, _ = LX.validate(cand); self.assertTrue(okv, pv)                                                                                                     # UV / indices / weights / topology hashes untouched
        for m_ in (LX.submesh(cand, 'eye_r'), LX.submesh(cand, 'eye_l')): self.assertEqual(m_['uv'], LX.submesh(self.ex, m_['role'])['uv']); self.assertEqual(m_['joints'], LX.submesh(self.ex, m_['role'])['joints'])
        c1, _ = LF.assemble(self.ex, base, self.src, shift, mk, cap_radius_scale=[1.0, 1.0]); c2, _ = LF.assemble(self.ex, base, self.src, shift, mk)
        self.assertEqual(LX.submesh(c1, 'eye_r')['positions'], LX.submesh(c2, 'eye_r')['positions'])                                                                  # scale 1 == the historical translation exactly
        with self.assertRaises(ValueError): LF.assemble(self.ex, base, self.src, shift, mk, cap_radius_scale=[0.9])
        with self.assertRaises(ValueError): LF.assemble(self.ex, base, self.src, shift, mk, cap_radius_scale=[0.9, 0.9], scale_eyes=True)
        # preserve_donor_radius against the scaled cap: rim records keep their DONOR SIGNED DISTANCE to the scaled cap
        fitp, mode, rep, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, socket_rim_policy='preserve_donor_radius', cap_radius_scale=[0.9, 0.95])
        for side, cidx, role, sc in (('r', 0, 'eye_r', 0.9), ('l', 1, 'eye_l', 0.95)):
            c0, rr0, _ = sphere_fit(G.native_to_blender(LX.submesh(self.ex, role)['positions'])); c = SE[cidx] + shift
            for i in self.ids[side]: self.assertAlmostEqual(np.linalg.norm(fitp[i] - c) - rr0 * sc, np.linalg.norm(mk['P'][i] - c0) - rr0, places=9)
            self.assertAlmostEqual(rep['socket_rim']['sides'][side]['cap_radius_scale'], sc)
        # constrained solver with the donor-signed reference against the scaled cap: rim residual vs the shifted target, reversal counts reported
        f, m, r, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=None, socket_rim_policy='constrained', socket_con_w_crease=100.0, socket_con_crease_reference='donor_signed', socket_con_crease_margin_deg=20.0, socket_con_iters=120, socket_con_gradcheck=True, cap_radius_scale=[0.9, 0.95])
        for side, cidx, role, sc in (('r', 0, 'eye_r', 0.9), ('l', 1, 'eye_l', 0.95)):
            sd = r['socket_rim']['sides'][side]; self.assertAlmostEqual(sd['cap_radius_scale'], sc); self.assertEqual(sd['crease']['reference_policy'], 'donor_signed'); self.assertIn('sign_reversals_vs_donor', sd['crease']); self.assertIn('step_size_study', sd['gradcheck'])
            c0, rr0, _ = sphere_fit(G.native_to_blender(LX.submesh(self.ex, role)['positions'])); c = SE[cidx] + shift; on = [i for i in self.ids[side] if abs(np.linalg.norm(mk['P'][i] - c0) - rr0) < 1e-6]
            self.assertLess(max(abs(np.linalg.norm(f[i] - c) - rr0 * sc) for i in on), 0.5e-3)                                                                       # rim on the SCALED cap within the storage bound
            ss = sd['gradcheck']['step_size_study']['directional_along_gradient']; self.assertTrue(all('abs_error' in v for v in ss.values()))
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, socket_rim_policy='constrained', socket_con_crease_reference='nonsense', socket_con_iters=5)
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, socket_rim_policy='joint_smooth', cap_radius_scale=[0.9, 0.9])
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, cap_radius_scale=[0.9, 0.0])

    def test_joint_smooth_and_donor_block_policies(self):
        f, m, r, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}, socket_rim_policy='joint_smooth', socket_joint_rings=2, socket_joint_iters=40)
        self.assertEqual(r['socket_rim']['policy'], 'joint_smooth')
        for s in ('r', 'l'): self.assertLess(r['socket_rim']['sides'][s]['rim_radius_residual_mm'], 1e-6); self.assertEqual(r['socket_rim']['sides'][s]['rim_records'], 7)
        self.assertEqual(r['attachment_max_error_m'], 0.0); self.assertGreater(r['modes']['socket_ring'], 0)
        f2, m2, r2, _ = LF.fit(self.ex, self.src, {'kind': 'surface'}, socket_rim_policy='donor_block', socket_block_rings=1, socket_block_blend_rings=2); SE, _ = LF.eye_centres_cc5(self.src); shift = np.array(r2['shift_m'])
        for s, cidx, role in (('r', 0, 'eye_r'), ('l', 1, 'eye_l')):
            from src.conversion.eye_fit import sphere_fit
            c0, _, _ = sphere_fit(G.native_to_blender(LX.submesh(self.ex, role)['positions'])); t = SE[cidx] + shift - c0
            self.assertLess(np.abs(f2[self.ids[s]] - (mk['P'][self.ids[s]] + t)).max(), 1e-12)          # rim = donor translated by the cap translation
        self.assertGreater(r2['modes']['socket_blend'], 0); self.assertEqual(r2['socket_rim']['policy'], 'donor_block')


if __name__ == '__main__':
    unittest.main()
