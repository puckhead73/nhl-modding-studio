"""Worker-slice tests (synthetic fixtures; optional local licensed fixtures are skipped visibly when absent).
Run: python -m unittest tests.conversion.test_worker
"""
import json, os, shutil, sys, tempfile, unittest
from pathlib import Path
import numpy as np
REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO))
from src.conversion import exchange as X, cc5_source as C, job as J, classify as CL, collision_repair as CR, candidate as CA, face_quality as FQ, geometry as G, eye_place as EP, mouth_islands as MI, masks as M
from tests.conversion.test_conversion import cylinder


def grid(nx=5, ny=5, z=0.0):
    P = np.array([[x * 0.01, y * 0.01, z] for y in range(ny) for x in range(nx)]); T = []
    for y in range(ny - 1):
        for x in range(nx - 1):
            a = y * nx + x; T += [[a, a + 1, a + nx], [a + 1, a + nx + 1, a + nx]]
    return P, np.array(T)


def synthetic_exchange():
    """Minimal native-fit-exchange/1 dict: a head (open-bottom cylinder, one seam-duplicated column) and two eye spheres."""
    P, T = cylinder(lambda z: 0.08, 1.55, 1.95, nz=12, na=24); n = len(P)
    col = np.flatnonzero(np.abs(np.arctan2(P[:, 1], P[:, 0])) < 1e-9); P2 = np.vstack([P, P[col]]); remap = {int(c): n + i for i, c in enumerate(col)}
    T2 = T.copy()
    for fi, t in enumerate(T):
        if all(np.arctan2(P[v, 1], P[v, 0]) >= -1e-9 and np.arctan2(P[v, 1], P[v, 0]) < 0.5 for v in t) and any(v in remap for v in t): T2[fi] = [remap.get(int(v), int(v)) for v in t]
    uv = np.c_[(np.arctan2(P2[:, 1], P2[:, 0]) / (2 * np.pi)) % 1.0, (P2[:, 2] - 1.55) / 0.4]
    rng = np.random.default_rng(1); u = rng.normal(size=(60, 3)); u /= np.linalg.norm(u, axis=1)[:, None]
    eyeL = np.array([0.03, -0.07, 1.83]) + 0.012 * u; eyeR = np.array([-0.03, -0.07, 1.83]) + 0.012 * u
    E = np.vstack([eyeL, eyeR]); ET = np.array([[i, (i + 1) % 60, (i + 2) % 60] for i in range(0, 58, 2)]); ET = np.vstack([ET, ET + 60])

    def sub(name, Pb, Tt, uvv, fmt, comps):
        nat = G.blender_to_native(Pb).tolist()
        return {'format': fmt, 'name': name, 'original_positions': nat, 'positions': [list(p) for p in nat], 'streams': [0], 'triangles': Tt.tolist(), 'normals': [[0, 0, 1]] * len(Pb), 'tangents': [[1, 0, 0]] * len(Pb), 'uv': uvv.tolist(), 'native_fields': {'p0': {}}, 'role': name, 'connected_components': comps}
    ex = {'donor_sha256': 'deadbeef', 'layout': {}, 'schema': 'native-fit-exchange/1', 'title': 'synthetic', 'profile_id': 'synthetic-profile-v1', 'coordinate_system': {'native_to_blender': '(x,y,z) -> (x,-z,y)', 'native_uv_to_blender': '(u,v) -> (u,1-v)'}, 'rules': [],
          'submeshes': [sub('geo_eyesShape', E, ET, np.zeros((120, 2)), 1, [{'vertex_ids': list(range(60))}, {'vertex_ids': list(range(60, 120))}]), sub('geo_headShape', P2, T2, uv, 5, [{'vertex_ids': list(range(len(P2)))}])]}
    return ex


class TestInputValidation(unittest.TestCase):
    def test_donor_layout_mismatch_is_refused(self):
        ex = synthetic_exchange(); sig = X.signature(ex)
        ok, problems = X.validate_donor_layout(ex, {'schema': 'native-fit-exchange/1', 'profile_id': 'synthetic-profile-v1', 'donor_sha256': 'deadbeef', 'submeshes': {k: {'records': v['records'], 'triangles': v['triangles'], 'ordered_topology_sha256': v['ordered_topology_sha256']} for k, v in sig['submeshes'].items()}})
        self.assertTrue(ok, problems)
        ok, problems = X.validate_donor_layout(ex, {'profile_id': 'other-profile'}); self.assertFalse(ok); self.assertTrue(any('profile_id' in p for p in problems))
        bad = {'submeshes': {'geo_eyesShape': {'records': 120, 'triangles': 58}, 'geo_headShape': {'records': 999, 'triangles': sig['submeshes']['geo_headShape']['triangles']}}}
        ok, problems = X.validate_donor_layout(ex, bad); self.assertFalse(ok); self.assertTrue(any('geo_headShape.records' in p for p in problems))
        # same counts but different ordered topology (winding reversed) -> different signature -> refused
        ex2 = json.loads(json.dumps(ex)); h = X.head(ex2); h['triangles'] = [t[::-1] for t in h['triangles']]
        ok, problems = X.validate_donor_layout(ex2, {'submeshes': {'geo_eyesShape': {}, 'geo_headShape': {'ordered_topology_sha256': sig['submeshes']['geo_headShape']['ordered_topology_sha256']}}})
        self.assertFalse(ok); self.assertTrue(any('ordered_topology_sha256' in p for p in problems))

    def test_cc5_source_family_mismatch_is_refused(self):
        P, T = grid(); src = [{'name': '0', 'positions': P.tolist(), 'uv': P[:, :2].tolist(), 'triangles': T.tolist(), 'triangle_materials': [0] * len(T)},
                              {'name': 'cc_base_eye', 'positions': P.tolist(), 'uv': P[:, :2].tolist(), 'triangles': T.tolist(), 'triangle_materials': [i % 4 for i in range(len(T))]}]
        ok, problems = C.validate(src); self.assertFalse(ok); self.assertTrue(any('family' in p for p in problems))
        ok, problems = C.validate(src, expected_family=None); self.assertFalse(ok); self.assertTrue(any('extent' in p for p in problems))   # 4 cm grid is not a head
        ok, problems = C.validate([src[0]], expected_family=None); self.assertFalse(ok); self.assertIn('missing mesh cc_base_eye', problems)

    def test_uv_signature_detects_corner_uv_change(self):
        ex = synthetic_exchange(); s1 = X.signature(ex)['submeshes']['geo_headShape']['corner_uv_sha256']
        X.head(ex)['uv'][5][0] += 1e-3; s2 = X.signature(ex)['submeshes']['geo_headShape']['corner_uv_sha256']; self.assertNotEqual(s1, s2)


class TestJobRunner(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp()); (self.tmp / 'runs').mkdir()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def job(self, d='job-01', root=None):
        return {'schema': 'nhl-head-job/1', 'job_id': 'synthetic', 'strategy': 'donor_fit', 'source': {}, 'destination': {}, 'calibration': {}, 'policies': {}, 'tools': {}, 'output': {'run_root': str(root or (self.tmp / 'runs')), 'dir': d}}

    def test_output_path_refusals(self):
        self.assertEqual(J.validate_job(self.job()), [])
        (self.tmp / 'runs' / 'exists').mkdir(); self.assertTrue(any('already exists' in p for p in J.validate_job(self.job('exists'))))
        self.assertTrue(any('not inside run_root' in p or '..' in p for p in J.validate_job(self.job('../escape'))))
        self.assertTrue(any('not inside run_root' in p or 'relative' in p for p in J.validate_job(self.job(str(self.tmp / 'abs')))))
        self.assertTrue(any('does not exist' in p for p in J.validate_job(self.job('x', root=self.tmp / 'nowhere'))))
        j = self.job(); j['strategy'] = 'source_topology'; self.assertTrue(any('unsupported' in p for p in J.validate_job(j)))
        j = self.job(); j['schema'] = 'nhl-head-job/0'; self.assertTrue(any('schema' in p for p in J.validate_job(j)))

    def test_input_hash_mismatch_is_refused(self):
        f = self.tmp / 'in.bin'; f.write_bytes(b'abc'); good = J.sha256_file(f)
        self.assertEqual(J.verify_ref('x', {'path': str(f), 'sha256': good})['sha256'], good)
        with self.assertRaises(J.JobError): J.verify_ref('x', {'path': str(f), 'sha256': '0' * 64})
        with self.assertRaises(J.JobError): J.verify_ref('x', {'path': str(self.tmp / 'missing.bin')})

    def test_failed_stage_preserves_receipts_and_is_not_ready(self):
        R = J.Runner(self.job(), log_to_stdout=False); f = self.tmp / 'in.bin'; f.write_bytes(b'abc')

        def ok_stage(d, log):
            (d / 'out.txt').write_text('x'); return {'outputs': {'out': d / 'out.txt'}, 'summary': {'n': 1}, 'losses': [{'stage': 'a', 'action': 'drop'}]}

        def bad_stage(d, log): raise ValueError('boom')
        r1 = R.run_stage('a', ok_stage, inputs={'in': f}, params={'k': 1})
        with self.assertRaises(J.JobError): R.run_stage('b', bad_stage, inputs={'in': f})
        st = R.finish('failed', note='stage b failed')
        self.assertEqual(st['status'], 'failed'); self.assertFalse(st['ready_for_packing'])
        for k in ('index', 'name', 'status', 'started', 'finished', 'elapsed_s', 'inputs', 'params', 'outputs', 'summary', 'losses'): self.assertIn(k, r1)
        self.assertEqual(r1['inputs']['in']['sha256'], J.sha256_file(f)); self.assertEqual(r1['outputs']['out']['sha256'], J.sha256_file(R.out / 'stages' / '01_a' / 'out.txt')); self.assertEqual(r1['params'], {'k': 1})
        rb = json.loads((R.out / 'stages' / '02_b' / 'receipt.json').read_text()); self.assertEqual(rb['status'], 'failed'); self.assertIn('boom', rb['error']); self.assertIn('traceback', rb)
        ra = json.loads((R.out / 'stages' / '01_a' / 'receipt.json').read_text()); self.assertEqual(ra['status'], 'ok')
        prog = json.loads((R.out / 'progress.json').read_text()); self.assertEqual([s['status'] for s in prog['stages']], ['ok', 'failed'])
        self.assertTrue((R.out / 'job.json').exists() and (R.out / 'status.json').exists())

    def test_cancellation_is_recorded_and_not_ready(self):
        R = J.Runner(self.job('job-02'), log_to_stdout=False)
        R.run_stage('a', lambda d, log: {'outputs': {}, 'summary': None})
        (R.out / 'CANCEL').write_text('')
        with self.assertRaises(J.Cancelled): R.run_stage('b', lambda d, log: {'outputs': {}})
        st = R.finish('cancelled', note='user'); self.assertEqual(st['status'], 'cancelled'); self.assertFalse(st['ready_for_packing'])
        self.assertEqual(json.loads((R.out / 'stages' / '01_a' / 'receipt.json').read_text())['status'], 'ok')

    def test_runner_refuses_existing_output(self):
        J.Runner(self.job('job-03'), log_to_stdout=False).finish('completed')
        with self.assertRaises(J.JobError): J.Runner(self.job('job-03'), log_to_stdout=False)


class TestCollisionRepair(unittest.TestCase):
    def _folded(self):
        donor, T = grid(); fit = donor.copy(); fit[12] = [0.035, 0.02, 0.0]       # centre vertex dragged 2.5 cells right: its fan folds over non-adjacent faces
        fit[11, 2] = 0.002; fit[14, 2] = -0.002                                       # tilt so the folded slivers pierce flat faces (non-coplanar -> exact test detects 6 pairs)
        return donor, fit, T

    def test_triggers_derive_from_classification_not_ids(self):
        donor, fit, T = self._folded(); uv = donor[:, :2] / 0.05
        res = CL.classify(donor, fit, fit, T, uv, [], np.zeros(len(donor), bool), np.zeros(len(donor), bool))
        trig = CL.collision_triggers(res['records'])
        self.assertGreater(len(trig), 0)
        self.assertTrue(all(res['face_shell'][f] == 'skin' for f in trig))
        for r in res['records']:
            if r['face'] in trig: self.assertIn('self_intersect', r['defects']['after']); self.assertEqual(r['defects']['donor'], [])
        self.assertIn('coplanar', res['summary']['coverage_limits'])
        clean = CL.classify(donor, donor, donor, T, uv, [], np.zeros(len(donor), bool), np.zeros(len(donor), bool))
        self.assertEqual(CL.collision_triggers(clean['records']), [])

    def test_noop_without_trigger_and_active_repair_with_trigger(self):
        donor, fit, T = self._folded()
        out, rep = CR.smooth_repair(donor, fit, T, [], [], [], rings=1, passes=1)
        self.assertTrue(np.array_equal(out, fit)); self.assertTrue(rep['result'].startswith('NO-OP')); self.assertEqual(rep['changed_count'], 0); self.assertTrue(rep['gate_accepted'])
        uv = donor[:, :2] / 0.05; trig = CL.collision_triggers(CL.classify(donor, fit, fit, T, uv, [], np.zeros(25, bool), np.zeros(25, bool))['records'])
        out2, rep2 = CR.smooth_repair(donor, fit, T, [], [], trig, rings=1, passes=2, force=True)
        self.assertGreater(rep2['changed_count'], 0); self.assertFalse(np.array_equal(out2, fit))
        self.assertEqual(rep2['triggered_faces'], trig)
        for k in ('new_skin_skin', 'new_skin_interior', 'remaining_skin_skin', 'interior_interior_new_documented'): self.assertIn(k, rep2['gate'])
        self.assertIn('coplanar', rep2['coverage_limits'])
        moved = np.flatnonzero(np.linalg.norm(out2 - fit, axis=1) > 0); self.assertTrue(np.array_equal(out2[moved], out2[moved].astype(np.float32).astype(np.float64)))   # float32-exact where moved
        out3, rep3 = CR.smooth_repair(donor, fit, T, [], [], trig, rings=1, passes=2, force=False)
        if not rep3['gate_accepted']: self.assertTrue(np.array_equal(out3, fit)); self.assertIn('unchanged', rep3['result'])

    def test_seam_groups_stay_coincident_and_oral_untouched(self):
        donor, fit, T = self._folded(); n = len(donor)
        donor2 = np.vstack([donor, donor[[12]]]); fit2 = np.vstack([fit, fit[[12]]]); T2 = T.copy(); T2[0] = [0, 1, 25] if 12 not in T[0] else T2[0]
        for fi, t in enumerate(T2):
            if 12 in t and fi % 2 == 0: T2[fi] = [25 if v == 12 else v for v in t]
        uv = donor2[:, :2] / 0.05; trig = CL.collision_triggers(CL.classify(donor2, fit2, fit2, T2, uv, [7], np.zeros(n + 1, bool), np.zeros(n + 1, bool))['records'])
        out, rep = CR.smooth_repair(donor2, fit2, T2, [7], [], trig, rings=1, passes=1, force=True)
        self.assertEqual(rep['seam_max_gap_m'], 0.0); self.assertTrue(np.array_equal(out[7], fit2[7]))   # oral id never moves


class TestCoplanarCoverage(unittest.TestCase):
    def test_coplanar_overlap_not_detected_is_reported(self):
        R = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0.2, 0.2, 0], [1.2, 0.2, 0], [0.2, 1.2, 0.]]); T = np.array([[0, 1, 2], [3, 4, 5]])
        self.assertEqual(FQ.self_intersections(R, T), [])                       # overlapping coplanar pair is invisible to the exact test
        R2 = R.copy(); R2[3, 2] = -0.1; R2[4, 2] = 0.1; self.assertEqual(FQ.self_intersections(R2, T), [(0, 1)])
        # adjacent (edge-sharing) faces are excluded by construction
        A = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [1, 1, 0.5]]); TA = np.array([[0, 1, 2], [1, 3, 2]]); self.assertEqual(FQ.self_intersections(A, TA), [])


class TestCandidateAndEyes(unittest.TestCase):
    def test_attachment_and_seam_preservation_checks(self):
        ex = synthetic_exchange(); checks, problems, per = CA.validate_candidate(ex, ex); self.assertEqual(problems, [], problems)
        cand = json.loads(json.dumps(ex)); h = X.head(cand); loop = G.bottom_loop(G.native_to_blender(h['original_positions']), np.array(h['triangles']))
        h['positions'][int(loop[0])][0] += 0.001
        checks, problems, per = CA.validate_candidate(cand, ex); self.assertIn('head:attachment_loop_plus_2_rings_exact', problems)
        cand = json.loads(json.dumps(ex)); h = X.head(cand); dup = G.seam_groups(G.native_to_blender(h['original_positions'])); self.assertTrue(dup)
        h['positions'][dup[0][0]][1] += 0.002
        checks, problems, per = CA.validate_candidate(cand, ex); self.assertIn('head:seam_duplicates_coincident', problems)
        cand = json.loads(json.dumps(ex)); X.head(cand)['uv'][0][0] += 0.1
        checks, problems, per = CA.validate_candidate(cand, ex); self.assertIn('geo_headShape:uv_equal', problems)
        cand = json.loads(json.dumps(ex)); X.submesh(cand, 'geo_eyesShape')['positions'][0][0] += 0.001
        checks, problems, per = CA.validate_candidate(cand, ex, eyes_may_move=False); self.assertIn('geo_eyesShape:positions_untouched', problems)
        checks, problems, per = CA.validate_candidate(cand, ex, eyes_may_move=True); self.assertTrue(any('uniform_scale_plus_translation' in p for p in problems))

    def test_eye_similarity_placement_verified(self):
        ex = synthetic_exchange(); e = X.submesh(ex, 'geo_eyesShape'); Pe = G.native_to_blender(e['original_positions']); comps = [c['vertex_ids'] for c in e['connected_components']]
        rng = np.random.default_rng(2); u = rng.normal(size=(80, 3)); u /= np.linalg.norm(u, axis=1)[:, None]
        Q = np.vstack([np.array([-0.032, -0.075, 1.835]) + 0.0148 * u, np.array([-0.032, -0.075, 1.835]) + 0.0148 * u, np.array([0.032, -0.075, 1.835]) + 0.0148 * u, np.array([0.032, -0.075, 1.835]) + 0.0148 * u])
        mats = {'Std_Eye_R': np.arange(0, 80), 'Std_Cornea_R': np.arange(80, 160), 'Std_Eye_L': np.arange(160, 240), 'Std_Cornea_L': np.arange(240, 320)}
        Pn, rep = EP.place_eyes(Pe, comps, Q, mats); ok, ver = EP.verify_similarity(Pe, Pn, comps); self.assertTrue(ok)
        for c in rep['components']: self.assertAlmostEqual(c['uniform_scale'], 0.0148 / 0.012, places=3)
        Pbad = Pn.copy(); Pbad[0] += 0.001; ok, _ = EP.verify_similarity(Pe, Pbad, comps); self.assertFalse(ok)


class TestMasksAndIslands(unittest.TestCase):
    def test_masks_derive_from_fixture_policy(self):
        ex = synthetic_exchange(); P, T, U = X.head_arrays(ex, 'original_positions'); mk = M.derive(P, T, U, [c['vertex_ids'] for c in X.head(ex)['connected_components']])
        self.assertEqual(mk['policy_id'], 'nhl-head-atlas-a'); self.assertEqual(len(mk['oral_ids']), 0)            # no occluded islands in a cylinder
        self.assertGreater(len(mk['frozen']), len(mk['bottom_loop'])); self.assertTrue(mk['is_frozen'][mk['bottom_loop']].all())
        prov = M.provenance(mk); self.assertIn('no other donor', prov['provenance'])


class TestLocalFixtures(unittest.TestCase):
    """Integration checks against local licensed fixtures; skipped (visible) when absent."""
    EX = REPO / 'local/runs/codex/nhl21-phase3-exchange-01/native_mesh.json'

    def test_exchange_layout_matches_profile(self):
        if not self.EX.exists(): self.skipTest(f'local NHL21 exchange fixture absent: {self.EX} (coverage unknown)')
        ex = X.load(self.EX); sig = X.signature(ex)
        self.assertEqual(sig['submeshes']['geo_headShape']['records'], 3157); self.assertEqual(sig['submeshes']['geo_headShape']['triangles'], 6014)
        ok, problems = X.validate_donor_layout(ex, {'profile_id': 'nhl21-ps4-cusa18182-game90-aaron-ekblad-v1'}); self.assertTrue(ok, problems)

    def test_worker_job_examples_validate(self):
        for name in ('turgeon_ekblad.json', 'fedorov_ekblad.json'):
            job = json.loads((REPO / 'experiments/claude/worker/jobs' / name).read_text())
            missing = [v['path'] for grp in ('source', 'destination', 'calibration') for v in job[grp].values() if isinstance(v, dict) and 'path' in v and not Path(v['path']).exists()]
            if missing: self.skipTest(f'{name}: fixtures absent {missing[:2]} (coverage unknown)')
            problems = [p for p in J.validate_job(job) if 'already exists' not in p]; self.assertEqual(problems, [], problems)


if __name__ == '__main__':
    unittest.main()
