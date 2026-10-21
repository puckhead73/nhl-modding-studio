"""Screen-calibration measurements (src/conversion/eye_screen_calibration.py): tri-tri intersection segments, contact dispositions
on a synthetic cap (contact within the grid vs intrusion; vertex-clear-but-crossing; coplanar limit), face classes, the donor-frame
Jacobian orientation sign (a mirrored triangle is a local inversion), the edge sensitivity / allowance (larger for smaller faces,
capped) and the numerical-error probe. Positive AND negative regression: a healthy contact must read as contact; a seeded fold must
stay rejected whatever the allowance.
Run: python -m unittest tests.conversion.test_eye_screen_calibration
"""
import sys, unittest
from pathlib import Path
import numpy as np
REPO = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(REPO))
from src.conversion import eye_screen_calibration as ESC, crease as CR, geometry as G


def cap_mesh(c, r, n_az=24, rings=(20, 40, 60)):
    """Open spherical cap (triangles) about c with radius r, axis -Y (Blender), half angle 60 deg."""
    pts = [c + r * np.array([0, -1, 0])]; tris = []
    for k, ang in enumerate(rings):
        a = np.radians(ang)
        for j in range(n_az):
            t = 2 * np.pi * j / n_az; pts.append(c + r * np.array([np.sin(a) * np.cos(t), -np.cos(a), np.sin(a) * np.sin(t)]))
    for j in range(n_az): tris.append([0, 1 + j, 1 + (j + 1) % n_az])
    for k in range(1, len(rings)):
        b0 = 1 + (k - 1) * n_az; b1 = 1 + k * n_az
        for j in range(n_az): jn = (j + 1) % n_az; tris += [[b0 + j, b1 + j, b1 + jn], [b0 + j, b1 + jn, b0 + jn]]
    return np.array(pts), np.array(tris)


class TestEyeScreenCalibration(unittest.TestCase):
    def setUp(self):
        self.c = np.array([0.0, 0.0, 1.8]); self.r = 0.016; self.cp, self.ct = cap_mesh(self.c, self.r); u = (self.cp - self.c) / np.linalg.norm(self.cp - self.c, axis=1)[:, None]; self.ax = u.mean(0) / np.linalg.norm(u.mean(0)); self.half = float(np.degrees(np.arccos(np.clip(u @ self.ax, -1, 1))).max())

    def head_face(self, depth_m, size=0.003, x=0.0, z=None):
        """One head triangle in the plane y = -(r - depth) ahead of the cap apex (depth > 0 = inside the cap), plus a far-away partner face."""
        y = self.c[1] - (self.r - depth_m); z = self.c[2] if z is None else z
        P = np.array([[x - size, y, z - size], [x + size, y, z - size], [x, y, z + size], [0.2, 0.2, 0.2], [0.21, 0.2, 0.2], [0.2, 0.21, 0.2]]); T = np.array([[0, 1, 2], [3, 4, 5]]); return P, T

    def test_tri_tri_segment_and_contact_dispositions(self):
        A = np.array([[-1, 0, 0], [1, 0, 0], [0, 0, 1.0]]); B = np.array([[0, -1, 0.5], [0, 1, 0.5], [0, 0, -0.5]])
        L, p, q = ESC.tri_tri_segment(A, B); self.assertAlmostEqual(L, 0.5, places=9)                                   # the crossing segment inside A along x = 0
        self.assertEqual(ESC.tri_tri_segment(A, A + np.array([0, 0, 5.0]))[0], 0.0)                                       # disjoint
        lash = np.zeros(6, bool); outer = np.array([True, True])
        # a head face resting 0.2 mm inside the cap surface: contact within the grid, not an intrusion
        P, T = self.head_face(0.0002); rows = ESC.contact_evidence(P, T, self.cp, self.ct, [0], lash, outer, self.c, self.ax, self.half)
        self.assertTrue(rows); self.assertTrue(all(r['disposition'] == 'contact_within_grid' for r in rows)); self.assertGreater(min(r['deepest_mm'] for r in rows), -0.49); self.assertEqual(rows[0]['class'], 'outer_skin')
        # a head face whose two base vertices sit 1.5 mm inside the cap and whose apex is 1 mm outside (it crosses the cap surface): intrusion with a positive inside area
        P, T = self.head_face(0.0015); P[2, 1] = self.c[1] - (self.r + 0.001); rows = ESC.contact_evidence(P, T, self.cp, self.ct, [0], lash, outer, self.c, self.ax, self.half)
        self.assertTrue(rows); self.assertTrue(all(r['disposition'] == 'intrusion' for r in rows)); self.assertLess(min(r['deepest_mm'] for r in rows), -0.8); self.assertGreater(max(r['inside_area_mm2'] for r in rows), 0)
        s = ESC.summarise_contact(rows, {0: {'front': 500}}); self.assertEqual(s['by_class_and_reading'].get('outer_skin|visible_intrusion'), len(rows)); self.assertEqual(s['hidden_intrusion_pairs'], 0)
        s2 = ESC.summarise_contact(ESC.contact_evidence(P, T, self.cp, self.ct, [0], lash, outer, self.c, self.ax, self.half), {}); self.assertEqual(s2['hidden_intrusion_pairs'], s2['pairs'])   # invisible -> hidden intrusion
        # vertex-clear-but-crossing: a large face whose vertices are all outside the cap sphere but whose interior passes through the cap rim region
        P, T = self.head_face(-0.001); P[1] = self.c + [0.03, 0.0, 0.0]; P[2] = self.c + [-0.03, 0.0, 0.0]; rows = ESC.contact_evidence(P, T, self.cp, self.ct, [0], lash, outer, self.c, self.ax, self.half)   # apex 1 mm in front of the cap, base 30 mm to the sides at the centre plane: every vertex outside the sphere, the plane cuts the cap
        self.assertTrue(rows and all(r['vertex_clear_but_crossing'] for r in rows), rows)
        # classes
        cls = ESC.head_face_classes(np.array([[0, 1, 2], [0, 1, 3], [0, 1, 2]]), np.array([True, True, True, False]), np.array([True, True, False])); self.assertEqual(list(cls), ['hidden_fan', 'rim_margin', 'hidden_fan'])
        cls = ESC.head_face_classes(np.array([[0, 1, 2], [0, 1, 2]]), np.zeros(3, bool), np.array([True, False])); self.assertEqual(list(cls), ['outer_skin', 'fold_underside'])

    def test_jacobian_orientation_and_allowance(self):
        Pd = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0.0]]); nd = np.array([0, 0, 1.0])
        sv, sgn = ESC.face_jacobian(Pd, Pd * np.array([2, 1, 1]), nd); self.assertEqual(sgn, 1); self.assertAlmostEqual(max(sv), 2.0, places=9); self.assertAlmostEqual(min(sv), 1.0, places=9)
        sv, sgn = ESC.face_jacobian(Pd, Pd[[0, 2, 1]], nd); self.assertEqual(sgn, -1)                                    # mirrored winding = local inversion
        # allowance: sensitivity grows as faces shrink; the cap bounds it; a seeded fold is rejected regardless
        def patch(scale_mm):
            X = np.array([[0, 0, 0], [1, 0, 0], [0.5, 0.8, 0.6], [0.5, -0.8, 0.6]]) * scale_mm / 1000 + np.array([-0.03, 1.833, 0.095]); return X, np.array([[0, 1, 2], [1, 0, 3]])
        X4, T4 = patch(4.0); X1, T1 = patch(1.0); ES = CR.edge_structure(T4, [0, 1])
        s4, c4 = ESC.edge_sensitivity(X4, T4, ES); s1, c1 = ESC.edge_sensitivity(X1, T1, ES); self.assertGreater(s1[0], s4[0]); self.assertGreater(c1[0], c4[0]); self.assertGreater(s4[0], 0)
        al = ESC.proposed_allowance(np.array([5.0, 500.0]), 30.0); self.assertEqual(list(al), [5.0, 30.0])
        th = CR.signed_dihedrals(G.native_to_blender(X4) * 1000, T4, ES); Xf = X4.copy(); Xf[[2, 3], 2] = 2 * X4[[0, 1], 2].mean() - X4[[2, 3], 2]; thf = CR.signed_dihedrals(G.native_to_blender(Xf) * 1000, T4, ES)
        lo, hi, _ = CR.envelope_donor_signed(th, np.radians(20), np.radians(15)); ex = abs(np.degrees(max(0, thf[0] - hi[0]) + min(0, thf[0] - lo[0]))); self.assertGreater(ex, s4[0])   # the mirrored fold exceeds any storage allowance of this edge
        num = ESC.numerical_error(G.native_to_blender(X4) * 1000, T4, ES); self.assertLess(num['float32_roundtrip_max_deg'], 1e-3); self.assertEqual(num['reproducibility_max_deg'], 0.0)


    def test_v3_contact_controls(self):
        """Review controls: (a) one head face crossing several cap triangles is measured ONCE (face rows deduplicated, pairs listed);
        (b) a narrow intrusion between coarse samples is still reported by the segment and refined sampling; (c) contact at the cap
        boundary is flagged unresolved, not passed; (d) a sample at the cap centre is 'undefined' (no NaN -> outside); (e) only the
        non-intruding portion visible -> patch visibility reports occluded while whole-face pixels would say visible."""
        lash = np.zeros(6, bool); outer = np.array([True, True])
        # (a) a wide flat face 0.2 mm inside the apex crosses many cap triangles
        P, T = self.head_face(0.0002, size=0.006); pairs, faces = ESC.contact_evidence_v3(P, T, self.cp, self.ct, [0], lash, outer, self.c)
        self.assertGreater(len(pairs), 1); self.assertEqual(len(faces), 1); self.assertEqual(sorted(faces[0]['cap_faces']), sorted(p_['cap_face'] for p_ in pairs)); self.assertEqual(faces[0]['disposition'], 'sub_allowance_contact')
        self.assertTrue(all('deepest_mm' not in p_ for p_ in pairs))                                                          # pair rows carry no face-level depth
        self.assertIsNotNone(faces[0]['inside_area_level_change_mm2']); self.assertGreater(pairs[0]['relative_quantization_sensitivity_mm'], 0)
        # (b) a narrow sliver intrusion: a thin face whose tip dips 1.2 mm inside on a strip narrower than the coarse sample pitch
        y_in = self.c[1] - (self.r - 0.0012); y_out = self.c[1] - (self.r + 0.0015)
        P = np.array([[0, y_in, self.c[2] + 0.0003], [0, y_in, self.c[2] - 0.0003], [0.012, y_out, self.c[2]], [0.2, 0.2, 0.2], [0.21, 0.2, 0.2], [0.2, 0.21, 0.2]]); T = np.array([[0, 1, 2], [3, 4, 5]])
        pairs, faces = ESC.contact_evidence_v3(P, T, self.cp, self.ct, [0], lash, outer, self.c); self.assertTrue(pairs); f0 = faces[0]
        self.assertEqual(f0['disposition'], 'intrusion'); self.assertLess(f0['deepest_estimate_mm'], -0.8); self.assertGreaterEqual(f0['levels'][-1]['inside_fraction'], f0['levels'][0]['inside_fraction'] * 0.5)   # refinement does not lose the sliver
        # (c) contact at the open cap boundary: a face that dips inside the sphere beyond the cap's 60-degree rim (outside the footprint)
        a = np.radians(66); inward = self.c + (self.r - 0.0012) * np.array([np.sin(a), -np.cos(a), 0])
        P = np.array([inward + [0, 0, 0.002], inward - [0, 0, 0.002], self.c + (self.r + 0.003) * np.array([np.sin(np.radians(50)), -np.cos(np.radians(50)), 0]), [0.2, 0.2, 0.2], [0.21, 0.2, 0.2], [0.2, 0.21, 0.2]]); T = np.array([[0, 1, 2], [3, 4, 5]])
        pairs, faces = ESC.contact_evidence_v3(P, T, self.cp, self.ct, [0], lash, outer, self.c)
        if faces: self.assertTrue(faces[0]['boundary_unresolved'])                                                              # inside the sphere but outside the cap footprint -> unresolved
        self.assertTrue(all(h is False for h in ESC.footprint_hits(np.array([inward]), G.TriangleSoup(self.cp, self.ct), self.c)))
        # (d) the cap centre itself: undefined, never outside
        fh = ESC.footprint_hits(np.array([self.c, self.c + [0, -0.001, 0]]), G.TriangleSoup(self.cp, self.ct), self.c); self.assertIsNone(fh[0]); self.assertTrue(fh[1])
        sd = ESC.signed_to_cap_safe(np.array([self.c]), G.TriangleSoup(self.cp, self.ct), self.c); self.assertTrue(np.isnan(sd[0]))
        # (e) patch visibility: occlusion of the intruding samples is tested per sample; back-facing and unsampled states are explicit
        P, T = self.head_face(0.0015); P[2, 1] = self.c[1] - (self.r + 0.001); soup = G.TriangleSoup(self.cp, self.ct); pts = ESC._bary_grid(20) @ P[T[0]]; sd = ESC.signed_to_cap_safe(pts, soup, self.c); fp = ESC.footprint_hits(pts, soup, self.c)
        inside = pts[[bool(s_ < 0 and f_ is True) for s_, f_ in zip(sd, fp)]]; self.assertGreater(len(inside), 0); n = np.array([0, -1.0, 0])
        plate = np.array([[-0.05, self.c[1] - self.r - 0.02, self.c[2] - 0.05], [0.05, self.c[1] - self.r - 0.02, self.c[2] - 0.05], [0.0, self.c[1] - self.r - 0.02, self.c[2] + 0.05]]); occl = G.TriangleSoup(plate, np.array([[0, 1, 2]]))   # a plate in front of the whole face
        vis = ESC.patch_visibility(inside, n, {'front': (0, -1, 0), 'behind': (0, 1, 0)}, [occl]); self.assertEqual(vis['front']['state'], 'occluded_by_opaque'); self.assertEqual(vis['behind']['state'], 'back_facing')
        self.assertEqual(ESC.patch_visibility(inside, n, {'front': (0, -1, 0)}, [])['front']['state'], 'visible_unoccluded')
        vis2 = ESC.patch_visibility(inside, n, {'front': (0, -1, 0)}, [], haircap_soup=occl); self.assertEqual(vis2['front']['state'], 'behind_haircap_alpha_not_evaluated'); self.assertEqual(ESC.patch_visibility(np.zeros((0, 3)), n, {'front': (0, -1, 0)}, [])['front']['state'], 'unsampled')

    def test_v3_orientation_rotation_counterexample_and_fold(self):
        """Codex's counterexample: a PROPER 180-degree rotation about an in-plane axis flips the donor-facing sign with unit stretch and must
        NOT be called structural; a genuinely folded neighbour (fold pair + local inversion) must; a healthy bent patch must not."""
        from src.conversion import face_quality as FQ
        Pd = np.array([[0, 0, 0], [2, 0, 0], [1, 1.5, 0.5], [1, -1.5, 0.5]], float) * 1e-3 + np.array([-0.03, -0.09, 1.833]); T_ = np.array([[0, 1, 2], [1, 0, 3]])
        ES = CR.edge_structure(T_, [0, 1]); th_d = CR.signed_dihedrals(Pd * 1000, T_, ES); lo, hi, cr = CR.envelope_donor_signed(th_d, np.radians(20), np.radians(60), np.radians(60)); c = np.array([-0.03, -0.10, 1.833])
        def run(X, folds):
            inv = set(np.flatnonzero(FQ.face_metrics(X, T_, [])['inverted_local']).tolist()); return ESC.orientation_evidence_v3(0, Pd, X, None, T_, {0: [1], 1: [0]}, c, folds, set(), inv, ES, th_d, cr, lo, hi, 'none')
        Rm = np.array([[1, 0, 0], [0, -1, 0], [0, 0, -1.0]]); self.assertAlmostEqual(np.linalg.det(Rm), 1.0); c_ = Pd.mean(0); r = run((Pd - c_) @ Rm.T + c_, set())
        self.assertEqual(r['donor_facing_sign'], -1); self.assertTrue(all(abs(x - 1) < 1e-9 for x in r['stretch_singular_values_vs_donor'])); self.assertNotEqual(r['disposition'], 'confirmed_structural'); self.assertEqual(r['storage_attribution'], 'unknown_(no earlier geometry)')
        Xb = Pd.copy(); Xb[2, 2] += 0.4e-3; rb = run(Xb, set()); self.assertNotEqual(rb['disposition'], 'confirmed_structural'); self.assertFalse(rb['inverted_local_(vs adjacent faces)'])
        Xf = Pd.copy(); Xf[3, 2] = 2 * Pd[[0, 1], 2].mean() - Pd[3, 2] - 0.3e-3; Xf[3, 1] = Pd[2, 1] * 0.8; rf = run(Xf, {0, 1}); self.assertEqual(rf['disposition'], 'confirmed_structural'); self.assertTrue(rf['inverted_local_(vs adjacent faces)'])
        # phase semantics: a post-lattice 'earlier' geometry never yields a storage attribution
        r_post = ESC.orientation_evidence_v3(0, Pd, Xb, Xb, T_, {0: [1], 1: [0]}, c, set(), set(), set(), ES, th_d, cr, lo, hi, 'post_lattice_candidate'); self.assertTrue(r_post['storage_attribution'].startswith('unknown'))
        r_pre = ESC.orientation_evidence_v3(0, Pd, Xb, Pd, T_, {0: [1], 1: [0]}, c, set(), set(), set(), ES, th_d, cr, lo, hi, 'pre_quantization_continuous'); self.assertIn(r_pre['storage_attribution'], ('storage_created', 'present_before_storage'))
        rt = ESC.coordinate_roundtrip_repeat(Pd * 1000, T_, ES); self.assertIn('repeat_execution_max_deg', rt); self.assertNotIn('numerical_error', rt)


if __name__ == '__main__': unittest.main()
