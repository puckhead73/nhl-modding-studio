"""Legacy exchange builder/validator tests on a synthetic decoded structure (no licensed data): open-scalp head with a
neck attachment and a mouth opening, two spherical eye caps, a mouthbag and a haircap, joints as shader registers.
Run: python -m unittest tests.conversion.test_legacy_exchange
"""
import json, sys, unittest
from pathlib import Path
import numpy as np
REPO = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(REPO))
from src.conversion import legacy_exchange as LX


def open_head(nz=10, na=24, r=0.09, y0=1.55, y1=1.93, scalp_rows=2, mouth=True):
    """Cylinder-ish head (native frame, +Y up, +Z forward), open at the bottom (neck) and at the top rows (scalp),
    with a small mouth hole in the front. Records carry uv, joints (shader registers) and weights."""
    ys = np.linspace(y0, y1, nz); th = np.linspace(0, 2 * np.pi, na, endpoint=False)
    P = np.array([[r * np.sin(t), y, r * np.cos(t)] for y in ys for t in th]); T = []
    for i in range(nz - 1 - scalp_rows):
        for j in range(na):
            a = i * na + j; b = i * na + (j + 1) % na; c = a + na; d = b + na
            if mouth and i == 4 and j in (0, 1): continue      # mouth hole: front-centre quad missing
            T += [[a, b, c], [b, d, c]]
    T = np.array(T); used = np.unique(T); remap = {int(v): k for k, v in enumerate(used)}; P = P[used]; T = np.vectorize(remap.get)(T)
    U = np.c_[(np.arctan2(P[:, 0], P[:, 2]) / (2 * np.pi)) % 1.0, 1 - (P[:, 1] - y0) / (y1 - y0)]
    J = np.zeros((len(P), 4), int); W = np.zeros((len(P), 4)); W[:, 0] = 1.0
    J[:, 0] = 24                                                  # Head palette 8
    J[P[:, 1] < y0 + 1e-6, 0] = 12                                # bottom ring: Spine3 (palette 4, body)
    return P, T, U, J, W


def cap(centre, radius=0.016, n=40):
    rng = np.random.default_rng(3); u = rng.normal(size=(n, 3)); u /= np.linalg.norm(u, axis=1)[:, None]; u[:, 2] = np.abs(u[:, 2]) + 0.2; u /= np.linalg.norm(u, axis=1)[:, None]
    P = np.asarray(centre) + radius * u; T = np.array([[i, (i + 1) % n, (i + 2) % n] for i in range(0, n - 2, 2)])
    return P, T


def synthetic_entry():
    P, T, U, J, W = open_head(); subs = []

    def sub(name, P, T, U=None, J=None, W=None, stride=32):
        n = len(P); U = np.c_[np.full(n, 0.5), np.linspace(0.1, 0.9, n)] if U is None else U
        J = np.full((n, 4), 24) if J is None else J; W = np.c_[np.ones(n), np.zeros((n, 3))] if W is None else W
        return {'name': name, 'positions': P.tolist(), 'uv': U.tolist(), 'triangles': T.tolist(), 'joints': J.tolist(), 'weights': W.tolist(), 'stride': stride, 'topology_sha256': 'x' * 64, 'uv_ordered_sha256': 'y' * 64, 'has_blend_indices': True, 'has_blend_weights': True}
    subs.append(sub('eyer_.Mesh', *cap([-0.03, 1.83, 0.08])))
    subs.append(sub('eyel_.Mesh', *cap([0.03, 1.83, 0.08])))
    hc, hcT = cap([0, 1.85, 0], radius=0.1, n=60); subs.append(sub('haircap_.Mesh', hc, hcT, stride=52))
    subs.append(sub('head_.Mesh', P, T, U, J, W))
    mb, mbT = cap([0, 1.765, 0.06], radius=0.02, n=30); subs.append(sub('mouthbag_.Mesh', mb, mbT))
    return {'path': 'X:/fixtures/player_synthetic.rx2', 'sha256': 'a' * 64, 'bytes': 1234, 'decode_mode': 'strict native skin', 'container_roundtrip_equal': True, 'palette_matrices': [np.eye(4).tolist() for _ in range(122)], 'submeshes': subs}


class TestLegacyExchange(unittest.TestCase):
    def test_build_and_validate_synthetic(self):
        ex = LX.from_native_structure(synthetic_entry(), 'X:/structure.json', 'b' * 64)
        ok, problems, summary = LX.validate(ex); self.assertTrue(ok, problems)
        h = LX.submesh(ex, 'head'); st = h['structure']
        self.assertEqual(len(st['open_loops']), 3, st['open_loops'])                          # neck ring, scalp ring, mouth hole (separate loops in this synthetic head)
        self.assertEqual(len(st['attachment_loop']), 24); self.assertTrue(all(h['joints'][i][0] == 12 for i in st['attachment_loop']))
        self.assertEqual(st['rear_opening'], []); self.assertEqual(st['attachment_palettes'], [4]); self.assertEqual(len(st['mouth_opening']), 6); self.assertEqual(len(st['other_loops']), 1)
        self.assertEqual([m['role'] for m in ex['submeshes']], ['eye_r', 'eye_l', 'haircap', 'head', 'mouthbag'])
        self.assertEqual(LX.submesh(ex, 'eye_r')['eye']['kind'], 'spherical_cap'); self.assertAlmostEqual(LX.submesh(ex, 'eye_r')['eye']['radius_m'], 0.016, places=4)
        self.assertEqual(ex['coordinate_system']['native_to_blender'], '(x,y,z) -> (x,-z,y)'); self.assertEqual(ex['donor_sha256'], 'a' * 64); self.assertEqual(ex['decode']['structure_sha256'], 'b' * 64)
        self.assertEqual(h['original_positions'], h['positions']); self.assertEqual(len(ex['palette_matrices']), 122)
        exp = {'donor_sha256': 'a' * 64, 'roles': {'head': {'records': len(h['positions']), 'triangles': len(h['triangles']), 'ordered_topology_sha256': h['ordered_topology_sha256']}}}
        self.assertTrue(LX.validate(ex, exp)[0]); exp['roles']['head']['records'] += 1; self.assertFalse(LX.validate(ex, exp)[0])

    def test_validator_rejects_broken_inputs(self):
        base = LX.from_native_structure(synthetic_entry())

        def broken(fn):
            ex = json.loads(json.dumps(base)); fn(ex); return LX.validate(ex)[1]
        self.assertTrue(any('role mouthbag' in p for p in broken(lambda ex: ex['submeshes'].pop())))
        self.assertTrue(any('index out of range' in p for p in broken(lambda ex: LX.submesh(ex, 'head')['triangles'].__setitem__(0, [0, 1, 99999]))))
        self.assertTrue(any('uv must be per-record' in p for p in broken(lambda ex: LX.submesh(ex, 'head')['uv'].__setitem__(0, [1.5, 0.2]))))
        self.assertTrue(any('sum to 1' in p for p in broken(lambda ex: LX.submesh(ex, 'head')['weights'].__setitem__(0, [0.5, 0.2, 0, 0]))))
        self.assertTrue(any('multiples of 3' in p for p in broken(lambda ex: LX.submesh(ex, 'head')['joints'].__setitem__(0, [25, 0, 0, 0]))))
        self.assertTrue(any('beyond palette' in p for p in broken(lambda ex: LX.submesh(ex, 'head')['joints'].__setitem__(0, [3 * 200, 0, 0, 0]))))
        self.assertTrue(any('ordered_topology_sha256' in p for p in broken(lambda ex: LX.submesh(ex, 'head')['triangles'].__setitem__(0, LX.submesh(ex, 'head')['triangles'][0][::-1]))))
        self.assertTrue(any('corner_uv_sha256' in p for p in broken(lambda ex: LX.submesh(ex, 'head')['uv'].__setitem__(3, [0.31, 0.31]))))
        self.assertTrue(any('no attachment loop' in p for p in broken(lambda ex: [LX.submesh(ex, 'head')['joints'].__setitem__(i, [24, 0, 0, 0]) for i in range(len(LX.submesh(ex, 'head')['joints']))] and LX.submesh(ex, 'head').pop('structure'))))
        self.assertTrue(any('not a sphere' in p for p in broken(lambda ex: (LX.submesh(ex, 'eye_r')['positions'].__setitem__(0, [0, 1.9, 0]), LX.submesh(ex, 'eye_r').pop('eye')))))
        self.assertTrue(any('coordinate_system' in p for p in broken(lambda ex: ex['coordinate_system'].__setitem__('up', '+Z'))))
        self.assertTrue(any('palette_matrices' in p for p in broken(lambda ex: ex.__setitem__('palette_matrices', []))))
        self.assertTrue(any('provenance' in p for p in broken(lambda ex: ex['donor'].pop('sha256'))))
        self.assertFalse(LX.validate([])[0])

    def test_untouched_fields_and_rigid_companions(self):
        ref = LX.from_native_structure(synthetic_entry()); cand = json.loads(json.dumps(ref))
        h = LX.submesh(cand, 'head'); h['positions'][5][0] += 0.003                      # head may move
        e = LX.submesh(cand, 'eye_r'); e['positions'] = (np.array(e['positions']) + [0.001, -0.002, 0.0005]).tolist()   # rigid translation allowed
        ok, problems, moves = LX.untouched_fields_equal(cand, ref); self.assertTrue(ok, problems); self.assertAlmostEqual(moves['eye_r'][1], -0.002)
        bad = json.loads(json.dumps(cand)); LX.submesh(bad, 'eye_l')['positions'][0][0] += 0.001
        ok, problems, _ = LX.untouched_fields_equal(bad, ref); self.assertFalse(ok); self.assertTrue(any('rigid translation' in p for p in problems))
        bad = json.loads(json.dumps(cand)); LX.submesh(bad, 'head')['joints'][0] = [27, 0, 0, 0]
        ok, problems, _ = LX.untouched_fields_equal(bad, ref); self.assertIn('head_.Mesh.joints', problems)
        bad = json.loads(json.dumps(cand)); LX.submesh(bad, 'haircap')['positions'][0][0] += 0.001
        ok, problems, _ = LX.untouched_fields_equal(bad, ref); self.assertIn('haircap_.Mesh.positions_moved', problems)
        bad = json.loads(json.dumps(cand)); bad['palette_matrices'][0][0][0] = 2.0
        ok, problems, _ = LX.untouched_fields_equal(bad, ref); self.assertIn('top_level_palette_matrices', problems)

    def test_declared_cap_scale_is_distinguished_from_a_rigid_translation(self):
        ref = LX.from_native_structure(synthetic_entry()); cand = json.loads(json.dumps(ref)); m = LX.submesh(cand, 'eye_r'); P = np.array(m['positions']); c = P.mean(0)
        m['positions'] = ((P - c) * 0.93 + c + np.array([0.001, 0.0, 0.002])).tolist()
        ok0, p0, _ = LX.untouched_fields_equal(cand, ref); self.assertFalse(ok0); self.assertTrue(any('rigid translation' in q for q in p0))                    # historical rule: an undeclared scale fails
        ok1, p1, mv = LX.untouched_fields_equal(cand, ref, declared_scales={'eye_r': 0.93}); self.assertTrue(ok1, p1); self.assertEqual(mv['eye_r']['kind'], 'declared_similarity'); self.assertEqual(mv['eye_r']['scale'], 0.93)
        self.assertLess(np.abs(np.array(mv['eye_r']['translation']) - [0.001, 0.0, 0.002]).max(), 1e-9); self.assertNotIsInstance(mv['eye_l'], dict)             # the other eye keeps the translation-only report
        ok2, p2, _ = LX.untouched_fields_equal(cand, ref, declared_scales={'eye_r': 0.95}); self.assertFalse(ok2); self.assertTrue(any('declared similarity' in q for q in p2))   # the wrong declared scale fails
        ok3, p3, _ = LX.untouched_fields_equal(cand, ref, declared_scales={'eye_r': 0.93, 'head': 0.9}); self.assertFalse(ok3); self.assertIn('declared_scale_for_non_rigid_role_head', p3)
        ok4, p4, _ = LX.untouched_fields_equal(cand, ref, declared_scales={'eye_r': -1}); self.assertFalse(ok4); self.assertIn('declared_scale_eye_r_invalid', p4)

    def test_boundary_loops_and_components(self):
        P, T, U, J, W = open_head(mouth=False); loops, nm = LX.boundary_loops(T); self.assertEqual(nm, 0); self.assertEqual(sorted(len(l) for l in loops), [24, 24])
        P, T, U, J, W = open_head(mouth=True); loops, nm = LX.boundary_loops(T); self.assertEqual(len(loops), 3)
        comps = LX.connected_components(len(P), T); self.assertEqual(len(comps), 1); self.assertEqual(len(comps[0]['vertex_ids']), len(P))


class TestLocalLegacyFixture(unittest.TestCase):
    STRUCT = REPO / 'local/runs/codex/inventory-01/native-structure-v2.json'

    def test_player67_exchange_validates(self):
        runs = sorted((REPO / 'local/runs/claude').glob('legacy-2026-09-12-exchange-*/native_mesh.json'))
        if not runs: self.skipTest('local Legacy exchange run absent (licensed decode; coverage unknown)')
        ex = LX.load(runs[-1]); ok, problems, summary = LX.validate(ex, {'roles': {'head': {'records': 2623, 'triangles': 5098}, 'eye_r': {'records': 66, 'triangles': 100}, 'mouthbag': {'records': 534, 'triangles': 916}}})
        self.assertTrue(ok, problems); self.assertEqual(summary['head_.Mesh']['attachment_records'], 69); self.assertEqual(summary['head_.Mesh']['rear_opening_records'], 47); self.assertEqual(summary['head_.Mesh']['mouth_opening_records'], 32)


if __name__ == '__main__':
    unittest.main()
