"""Synthetic tests for the Legacy static-qualification measurements and the new declared fit options: ray-cast
facing/exposure, cap contact classification (artifact vs contact vs intersection), haircap covered/poking/open,
UV template evidence (edge + seam agreement), conservative readiness, lid clearance, untouched eye placement and
eye-line levelling. Run: python -m unittest tests.conversion.test_legacy_qualify
"""
import json, sys, unittest
from pathlib import Path
import numpy as np
REPO = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(REPO))
from src.conversion import legacy_exchange as LX, legacy_fit as LF, legacy_qualify as LQ, geometry as G
from tests.conversion.test_legacy_exchange import synthetic_entry, cap
from tests.conversion.test_legacy_fit import synthetic_cc5


def square(z=1.75, y=0.0, s=0.05, flip=False):
    P = np.array([[-s, y, z - s], [s, y, z - s], [s, y, z + s], [-s, y, z + s]]); T = np.array([[0, 1, 2], [0, 2, 3]])
    return P, (T[:, ::-1] if flip else T)


class TestRaycast(unittest.TestCase):
    def test_front_back_and_occlusion(self):
        P, T = square(); face, depth, front = LQ.ortho_raycast(P, T, LQ.VIEWS['front'], res=(60, 60), scale=0.2)   # camera at -y looking +y; normal of (0,1,2) = -y -> towards camera
        self.assertTrue(front.all()); self.assertGreater((face >= 0).sum(), 300); self.assertLess((face >= 0).sum(), 1200)
        P2, T2 = square(flip=True); _, _, front2 = LQ.ortho_raycast(P2, T2, LQ.VIEWS['front'], res=(60, 60), scale=0.2); self.assertFalse(front2.any())
        near, nT = square(y=-0.1, s=0.02); comp = {'far': (P, T), 'near': (near, nT)}
        cls, counts = LQ.visibility(comp, 'front', res=(60, 60), scale=0.2); self.assertIn('near:front', counts); self.assertIn('far:front', counts); self.assertGreater(counts['far:front'], counts['near:front'])
        cls, counts = LQ.visibility({'shell': (P2, T2)}, 'front', res=(60, 60), scale=0.2); self.assertIn('shell:back', counts); self.assertNotIn('shell:front', counts)
        cls, counts = LQ.visibility({'shell': (P, T)}, 'rear', res=(60, 60), scale=0.2); self.assertIn('shell:back', counts)      # same square seen from behind
        self.assertEqual(LQ.ortho_raycast(P, T, LQ.VIEWS['front'], res=(60, 60), scale=0.2, focus=(10, 0, 0))[0].max(), -1)     # off screen: background only

    def test_class_image_written(self):
        P, T = square(); cls, _ = LQ.visibility({'head': (P, T)}, 'front', res=(20, 20), scale=0.2)
        import tempfile
        with tempfile.TemporaryDirectory() as d:
            LQ.class_image(cls, Path(d) / 'm.png'); self.assertTrue((Path(d) / 'm.png').exists())


def dense_cap(centre=(0.0, 0.0, 1.75), radius=0.016, half_deg=70.0, rings=14, segs=40):
    """Structured spherical cap facing -y (Blender forward): apex + rings, proper triangulation (chord sag << 0.1 mm)."""
    c = np.asarray(centre, float); pts = [c + radius * np.array([0.0, -1.0, 0.0])]; T = []
    for k in range(1, rings + 1):
        th = np.radians(half_deg) * k / rings
        for j in range(segs):
            ph = 2 * np.pi * j / segs; pts.append(c + radius * np.array([np.sin(th) * np.cos(ph), -np.cos(th), np.sin(th) * np.sin(ph)]))
    for j in range(segs): T.append([0, 1 + j, 1 + (j + 1) % segs])
    for k in range(1, rings):
        a = 1 + (k - 1) * segs; b = 1 + k * segs
        for j in range(segs): T += [[a + j, b + j, b + (j + 1) % segs], [a + j, b + (j + 1) % segs, a + (j + 1) % segs]]
    return np.array(pts), np.array(T)


class TestCapContact(unittest.TestCase):
    def setUp(self):
        self.cP, self.cT = dense_cap()

    def lid(self, r, ang_deg):
        c = np.array([0.0, 0.0, 1.75]); ax = np.array([0.0, -1.0, 0.0]); a = np.radians(ang_deg)
        d = np.array([np.sin(a), -np.cos(a), 0.0]); return c + r * d

    def test_artifact_vs_contact_vs_intersection(self):
        half = LQ.cap_contact(np.zeros((1, 3)), np.zeros((0, 3), int), self.cP, self.cT, np.array([], int))['sphere']['half_angle_deg']
        # one record inside the sphere but beyond the cap extent (no cap geometry there) = artifact; one within extent below the surface = contact
        self.assertAlmostEqual(half, 70.0, places=3); art = self.lid(0.0150, 95.0); con = self.lid(0.0155, 10.0); free = self.lid(0.020, 10.0)
        head_P = np.array([art, con, free, free + [0.01, 0, 0], free + [0, 0, 0.01]]); head_T = np.array([[2, 3, 4]])
        r = LQ.cap_contact(head_P, head_T, self.cP, self.cT, np.array([0]))
        self.assertEqual(r['inside_sphere'], 1); self.assertEqual(r['inside_sphere_outside_cap_extent'], 1); self.assertEqual(r['inside_actual_cap_surface'], 0)
        r2 = LQ.cap_contact(head_P, head_T, self.cP, self.cT, np.array([1])); self.assertEqual(r2['inside_sphere_within_cap_extent'], 1); self.assertEqual(r2['inside_actual_cap_surface'], 1); self.assertLess(r2['min_signed_to_cap_surface_within_extent_mm'], 0)
        donor = LQ.cap_contact(head_P, head_T, self.cP, self.cT, np.array([2])); self.assertEqual(donor['inside_sphere'], 0)
        self.assertEqual(LQ.contact_verdict(donor, r)['kind'], 'approximation_artifact'); self.assertEqual(LQ.contact_verdict(donor, r2)['kind'], 'surface_contact'); self.assertEqual(LQ.contact_verdict(donor, donor)['kind'], 'no_penetration')
        # a head triangle slicing through the cap = intersection
        tri = np.array([[-0.03, -0.015, 1.75], [0.03, -0.015, 1.75], [0.0, -0.015, 1.80]]); r3 = LQ.cap_contact(tri, np.array([[0, 1, 2]]), self.cP, self.cT, np.array([0]))
        self.assertGreater(r3['head_cap_triangle_intersections'], 0); v3 = LQ.contact_verdict(donor, r3); self.assertGreater(v3['candidate_intersections'], v3['donor_intersections'])   # intersections are read relative to the donor, never as the kind
        # radius-policy reading: the same record outside a smaller sphere
        src = LQ.cap_contact(head_P, head_T, self.cP, self.cT, np.array([1]), head_faces=np.array([], int), sphere=(np.array([0, 0, 1.75]), 0.0149)); v = LQ.contact_verdict(donor, r2, src)
        self.assertGreaterEqual(v['same_records_vs_cc5_radius_sphere_min_mm'], 0); self.assertIn('radius surplus', v['radius_policy_component'])


class TestHaircapRelation(unittest.TestCase):
    def test_covered_poking_open(self):
        # flat head patch at z=1.80 facing +z; cap square above it (covered), cap below one record (poking), no cap over the third (open)
        head_P = np.array([[0, 0, 1.80], [0.02, 0, 1.80], [0, 0.02, 1.80], [0.2, 0, 1.80], [0.22, 0, 1.80], [0.2, 0.02, 1.80], [0.4, 0, 1.80], [0.42, 0, 1.80], [0.4, 0.02, 1.80]], float)
        head_T = np.array([[0, 1, 2], [3, 4, 5], [6, 7, 8]])
        cap_P = np.array([[-0.05, -0.05, 1.81], [0.06, -0.05, 1.81], [0.06, 0.06, 1.81], [-0.05, 0.06, 1.81], [0.15, -0.05, 1.798], [0.26, -0.05, 1.798], [0.26, 0.06, 1.798], [0.15, 0.06, 1.798]], float)   # second square 2 mm below its patch
        cap_T = np.array([[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]])
        r = LQ.haircap_relation(head_P, head_T, cap_P, cap_T, np.array([0, 3, 6]))
        self.assertEqual((r['covered'], r['poking'], r['open']), (1, 1, 1)); self.assertEqual(r['poking_ids'], [3]); self.assertEqual(r['open_ids'], [6]); self.assertEqual(r['head_cap_triangle_intersections'], 0)
        far = LQ.haircap_relation(head_P, head_T, cap_P + [0, 0, -0.02], cap_T, np.array([0, 3]))   # cap 2 cm below: a far hit from behind is 'open', not poking
        self.assertEqual(far['poking'], 0); self.assertEqual(far['open'], 2)
        self.assertEqual(LQ.cross_intersections(head_P, head_T, cap_P, cap_T), [])
        slicer = np.array([[0.0, -0.1, 1.79], [0.0, 0.1, 1.79], [0.0, 0.0, 1.83]]); self.assertEqual(len(LQ.cross_intersections(slicer, np.array([[0, 1, 2]]), cap_P, cap_T)), 2)


class TestUVTemplateEvidence(unittest.TestCase):
    def test_identical_and_decimated_layouts(self):
        rng = np.random.default_rng(2); Ub = rng.random((40, 2)); Tb = np.array([[i, (i + 1) % 40, (i + 7) % 40] for i in range(40)])
        ev = LQ.uv_template_evidence(Ub, Tb, Ub, Tb, seam_groups_a=[[0, 1]], seam_groups_b=[[0, 1], [5, 6]])
        self.assertEqual(ev['uv_edge_agreement'], 1.0); self.assertEqual(ev['twin_residual_px']['max'], 0.0); self.assertEqual(ev['seams_a_mapping_into_one_b_seam'], 1); self.assertEqual(ev['distinct_twins'], 40)
        keep = np.arange(0, 40, 2); Ua = Ub[keep] + 1e-5; Ta = np.array([[i, (i + 1) % 20, (i + 3) % 20] for i in range(20)])     # A = every other B record with new (coarser) connectivity
        ev2 = LQ.uv_template_evidence(Ua, Ta, Ub, Tb, regions_a={'all': np.ones(20, bool)}, seam_groups_a=[[0, 2]], seam_groups_b=[[0, 1], [4, 5]])
        self.assertLess(ev2['twin_residual_px']['max'], 0.05); self.assertLess(ev2['uv_edge_agreement'], 1.0); self.assertGreaterEqual(ev2['uv_edge_agreement_within_2_hops'], ev2['uv_edge_agreement']); self.assertEqual(ev2['seams_a_split_across_b_seams'], 1); self.assertIn('all', ev2['by_region_px'])


class TestReadiness(unittest.TestCase):
    def test_report_stays_conservative(self):
        rep = LQ.readiness_report({'id': 'run04'}, [{'id': 'x', 'path': 'p', 'sha256': 'h', 'differs_from_baseline': '-', 'screening_gates': {'a': True}, 'gate_waivers': []}], [{'id': 'texture', 'state': 'blocked'}], ['s'], ['u'], ['f'], {'candidate_id': 'x'}, ['codex'])
        self.assertFalse(rep['export_ready']); self.assertFalse(rep['ready_for_experimental_packing']); self.assertTrue(rep['candidates'][0]['all_screening_gates_pass']); self.assertEqual(rep['candidates'][0]['status'], 'experimental_not_for_packing')
        self.assertIn('no_lid_record_inside_placed_cap_within_extent', rep['screening_gates']); json.dumps(rep)


class TestFitOptions(unittest.TestCase):
    def setUp(self):
        self.ex = LX.from_native_structure(synthetic_entry()); self.src = synthetic_cc5(dz=0.02)

    def test_level_rotation_and_rotate_source(self):
        SE = np.array([[-0.03, -0.05, 1.83], [0.03, -0.05, 1.836]]); DE = np.array([[-0.03, -0.05, 1.83], [0.03, -0.05, 1.83]])
        R, ang = LF.level_rotation(SE, DE); self.assertAlmostEqual(ang, np.degrees(np.arctan2(0.006, 0.06)), places=6); self.assertAlmostEqual(abs(np.linalg.det(R)), 1.0, places=9)
        a = (SE[1] - SE[0]) @ R.T; self.assertLess(abs(a[2]), 1e-12)
        src2 = LF.rotate_source(self.src, R, SE.mean(0)); self.assertEqual(len(src2), len(self.src)); self.assertEqual(src2[0]['triangles'], self.src[0]['triangles'])
        self.assertEqual(LF.level_rotation(DE, DE)[1], 0.0)

    def test_untouched_eye_placement_and_lid_clearance(self):
        fitp, mode, rep, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}); shift = np.array(rep['shift_m'])
        cand, moves = LF.assemble(self.ex, fitp, self.src, shift, mk, eye_placement='untouched')
        self.assertEqual(LX.submesh(cand, 'eye_r')['positions'], LX.submesh(self.ex, 'eye_r')['positions']); self.assertEqual(moves['eye_r']['translation_mm'], 0.0); self.assertGreater(moves['eye_r']['cc5_centre_offset_mm'], 0)
        ok, problems, rigid = LX.untouched_fields_equal(cand, self.ex); self.assertTrue(ok, problems); self.assertEqual(rigid['eye_r'], [0.0, 0.0, 0.0])
        with self.assertRaises(ValueError): LF.assemble(self.ex, fitp, self.src, shift, mk, eye_placement='nonsense')
        # lid clearance: with a huge clearance every eyelid-bucket record inside the cap extent must end outside r + clearance
        f2, m2, r2, mk2 = LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=3.0, lid_clearance_placement='untouched')
        self.assertIsNotNone(r2['lid_clearance']); self.assertEqual(r2['lid_clearance']['placement'], 'untouched'); self.assertEqual(r2['attachment_max_error_m'], 0.0)
        self.assertEqual(rep['lid_clearance'], None)
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, lid_clearance_mm=1.0, lid_clearance_placement='nonsense')

    def test_preserve_donor_lips_and_scalp_policies(self):
        f, m, r, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}, mouth_gap_policy='preserve_donor_lips', lip_block_rings=2); P = mk['P']
        block = np.flatnonzero(m == 'lip_block'); self.assertGreater(len(block), len(mk['mouth_loop']))
        rel = (f[block] - f[block].mean(0)) - (P[block] - P[block].mean(0)); self.assertLess(np.abs(rel).max(), 1e-9)      # the whole lip block is a rigid translation of the donor's
        self.assertGreater(r['modes']['lip_block_blend'], 0); self.assertEqual(r['attachment_max_error_m'], 0.0); self.assertEqual(r['seam_max_gap_m'], 0.0)
        f2, m2, r2, mk2 = LF.fit(self.ex, self.src, {'kind': 'surface'}, scalp_policy='preserve_under_haircap', scalp_blend_rings=2)
        self.assertIsNotNone(r2['scalp']); frozen_scalp = np.flatnonzero(m2 == 'scalp_frozen')
        if len(frozen_scalp): self.assertTrue(np.array_equal(f2[frozen_scalp], mk2['P'][frozen_scalp]))
        f3, m3, r3, mk3 = LF.fit(self.ex, self.src, {'kind': 'surface'}, scalp_policy='preserve_haircap_rim', scalp_rim_tol_mm=20.0, scalp_blend_rings=1)
        self.assertEqual(r3['scalp']['policy'], 'preserve_haircap_rim'); rim = np.flatnonzero(m3 == 'scalp_frozen'); self.assertGreater(len(rim), 0); self.assertTrue(np.array_equal(f3[rim], mk3['P'][rim]))
        rm, dist = LF.haircap_rim_mask(self.ex, mk3, 0.02); self.assertEqual(int(rm.sum()) >= len(rim), True); self.assertEqual(LF.haircap_rim_mask(self.ex, mk3, 1e-9)[0].sum(), 0)
        with self.assertRaises(ValueError): LF.fit(self.ex, self.src, {'kind': 'surface'}, scalp_policy='nonsense')


if __name__ == '__main__':
    unittest.main()


class TestPostQuantization(unittest.TestCase):
    """Pinned-input verification, quantisation deltas, tolerant protection rule and the screen set on fp16-rounded positions."""

    def setUp(self):
        self.ex = LX.from_native_structure(synthetic_entry()); self.src = synthetic_cc5(dz=0.02)

    def test_verify_pins_inputs_vs_provenance(self):
        import hashlib, tempfile
        with tempfile.TemporaryDirectory() as d:
            a = Path(d) / 'a.json'; a.write_text('{"x":1}'); b = Path(d) / 'b.py'; b.write_text('print(1)')
            good = hashlib.sha256(a.read_bytes()).hexdigest(); h = {'schema': 's', 'input_a': {'path': str(a), 'sha256': good}, 'implementation_provenance': [{'path': str(b), 'sha256': 'deadbeef'}]}
            ok, rows = LQ.verify_pins(h); self.assertTrue(ok); self.assertEqual([r['kind'] for r in rows], ['input', 'provenance']); self.assertFalse(rows[1]['ok'])
            h['input_a']['sha256'] = 'deadbeef'; ok2, _ = LQ.verify_pins(h); self.assertFalse(ok2)
            h['input_a']['path'] = str(Path(d) / 'missing.json'); ok3, rows3 = LQ.verify_pins(h); self.assertFalse(ok3); self.assertIsNone(rows3[0]['sha256_actual'])

    def test_quantization_delta_and_tolerant_untouched(self):
        q = json.loads(json.dumps(self.ex))
        for m in q['submeshes']: m['positions'] = np.asarray(m['positions'], np.float16).astype(float).tolist()     # storage rounding like the native writer
        d = LQ.quantization_delta(self.ex, q); self.assertGreater(d['head']['max_mm'], 0); self.assertLess(d['head']['max_mm'], 1.0); self.assertEqual(d['haircap']['records'], 60)
        ok0, p0, _ = LX.untouched_fields_equal(q, self.ex); self.assertFalse(ok0)                                        # exact rule fails on storage noise
        ok1, p1, mv = LX.untouched_fields_equal(q, self.ex, tol_m=1e-3); self.assertTrue(ok1, p1); self.assertIn('eye_r', mv)
        q2 = json.loads(json.dumps(q)); LX.submesh(q2, 'eye_r')['positions'][0][0] += 0.01; ok2, p2, _ = LX.untouched_fields_equal(q2, self.ex, tol_m=1e-3); self.assertFalse(ok2)

    def test_screen_set_on_fit_and_rounded_fit(self):
        fitp, mode, rep, mk = LF.fit(self.ex, self.src, {'kind': 'surface'}); shift = np.array(rep['shift_m'])
        cand, moves = LF.assemble(self.ex, fitp, self.src, shift, mk); s = LQ.screen_set(self.ex, cand, mk, res=(60, 60), lid_faces_self=False)
        for k in ('gates', 'eyes', 'lips', 'haircap', 'attachment_max_error_mm', 'seam_max_gap_mm'): self.assertIn(k, s)
        self.assertEqual(s['attachment_max_error_mm'], 0.0); self.assertTrue(s['gates']['attachment_exact']); self.assertIsNone(s['lid_lip_self_intersections_outer_faces'])
        q = json.loads(json.dumps(cand))
        for m in q['submeshes']: m['positions'] = np.asarray(m['positions'], np.float16).astype(float).tolist()
        s2 = LQ.screen_set(self.ex, q, mk, res=(60, 60), lid_faces_self=False)
        self.assertGreater(s2['attachment_max_error_mm'], 0.0); self.assertFalse(s2['gates']['attachment_exact'])                  # rounding moves the frozen loop: a real screen change, reported not waived
        self.assertEqual(s2['lips']['mouth_loop_height_mm']['donor'], s['lips']['mouth_loop_height_mm']['donor'])
        d = LQ.screen_set(self.ex, self.ex, mk, res=(60, 60), lid_faces_self=False); self.assertTrue(d['gates']['attachment_exact']); self.assertEqual(d['inverted_excluding_lash']['fit'], d['inverted_excluding_lash']['donor'])
