"""Synthetic tests for src/conversion (run: python -m unittest tests.conversion.test_conversion)."""
import sys, unittest
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'src'))
from conversion import geometry as G, eye_fit, atlas, jaw_metrics, face_quality as FQ
from conversion.neck_transition import neck_transition


def cylinder(radius_fn, z0, z1, nz=30, na=32, cap=False):
    zs = np.linspace(z0, z1, nz); th = np.linspace(0, 2 * np.pi, na, endpoint=False)
    P = np.array([[radius_fn(z) * np.cos(t), radius_fn(z) * np.sin(t), z] for z in zs for t in th])
    T = []
    for i in range(nz - 1):
        for j in range(na):
            a = i * na + j; b = i * na + (j + 1) % na; c = a + na; d = b + na
            T += [[a, b, c], [b, d, c]]
    return P, np.array(T)


class TestGeometry(unittest.TestCase):
    def test_bottom_loop_and_rings(self):
        P, T = cylinder(lambda z: 1.0, 0, 1)
        bl = G.bottom_loop(P, T); self.assertEqual(len(bl), 32); self.assertTrue(np.allclose(P[bl, 2], 0))
        adj = G.adjacency(len(P), T); self.assertEqual(len(G.grow(bl, adj, 1)), 64)

    def test_ray_and_nearest(self):
        P, T = cylinder(lambda z: 1.0, 0, 1, na=64); soup = G.TriangleSoup(P, T)
        t = soup.ray_hit(np.array([0, 0, .5]), np.array([1, 0, 0.])); self.assertAlmostEqual(t, 1.0, places=2)
        q, d = soup.nearest(np.array([1.3, 0, .5])); self.assertAlmostEqual(d, 0.3, places=2)


class TestEyeFit(unittest.TestCase):
    def test_sphere_fit_and_cap_placement(self):
        rng = np.random.default_rng(0); u = rng.normal(size=(200, 3)); u /= np.linalg.norm(u, axis=1)[:, None]
        c0 = np.array([0.03, -0.09, 1.83]); P = c0 + 0.0156 * u
        c, r, res = eye_fit.sphere_fit(P); self.assertTrue(np.allclose(c, c0, atol=1e-9)); self.assertAlmostEqual(r, 0.0156, places=9)
        cap = P[u[:, 1] < -0.3]  # front cap only
        placed, info = eye_fit.place_cap(cap, [0, 0, 0], 0.01)
        c2, r2, _ = eye_fit.sphere_fit(placed); self.assertAlmostEqual(r2, 0.01, places=6); self.assertTrue(np.allclose(c2, 0, atol=1e-6))
        pen = eye_fit.lid_penetration(placed, c2, r2, np.array([[0, -0.009, 0.0]]))
        self.assertEqual(pen['penetrating'], 1); self.assertAlmostEqual(pen['max_depth'], 0.001, places=6)


class TestAtlas(unittest.TestCase):
    def test_rasterize_and_holes(self):
        uv = np.array([[0, 0], [1, 0], [1, 1], [0, 1.]]); tris = np.array([[0, 1, 2], [0, 2, 3]])
        tid = atlas.rasterize(uv, tris, 64); self.assertEqual(int((tid >= 0).sum()), 64 * 64)
        # source chart covers only the left half (V-up coords); identity warp -> right half uncovered
        suv = np.array([[0, 0], [.5, 0], [.5, 1], [0, 1.]])
        used, unc = atlas.coverage_holes(uv, tris, suv, tris, lambda q: q, 64)
        self.assertEqual(int(used.sum()), 4096); self.assertAlmostEqual(unc.sum() / 4096, 0.5, delta=0.03)


class TestJawMetrics(unittest.TestCase):
    def test_turning_detects_right_angle(self):
        line = np.array([[0, 0, 0], [0.05, 0, 0], [0.05, 0.05, 0.]]); pts = np.array([line[0] + (line[1] - line[0]) * t for t in np.linspace(0, 1, 20)] + [line[1] + (line[2] - line[1]) * t for t in np.linspace(0.05, 1, 20)])
        r, ang = jaw_metrics.turning(pts); self.assertGreater(ang.max(), 60); self.assertLess(np.median(ang), 5)  # windowed (1 cm) turning at a 90° corner with 4 mm resampling


class TestNeckTransition(unittest.TestCase):
    def test_cylinder_to_wider_source(self):
        # donor: radius 1 from z 1.5..2.0 ; source: radius 1.1 from 1.55..2.0 (source has no chest below 1.55)
        orig, T = cylinder(lambda z: 1.0, 1.5, 2.0, nz=26)
        src, ST = cylinder(lambda z: 1.1, 1.55, 2.0, nz=24, na=48)
        fit = orig.copy(); top = orig[:, 2] >= 1.79; fit[top, :2] *= 1.1  # trusted fit above fit_top
        res, mode, rep = neck_transition(orig, fit, T, src, ST, band_top=1.775, fit_top=1.79, max_delta=0.3, shape_rings=6)
        self.assertEqual(rep['frozen_max_error'], 0.0); self.assertEqual(rep['new_collapsed_triangles'], 0)
        r = np.linalg.norm(res[:, :2], axis=1)
        self.assertTrue(np.allclose(r[orig[:, 2] > 1.85], 1.1, atol=1e-6))       # face kept
        self.assertTrue(np.all(r[(orig[:, 2] > 1.7) & (orig[:, 2] < 1.77)] > 1.08))  # neck reached the source
        self.assertTrue(np.allclose(r[orig[:, 2] < 1.51], 1.0))                      # attachment kept
        # monotone, no overshoot
        self.assertLessEqual(r.max(), 1.1 + 1e-6); self.assertGreaterEqual(r.min(), 1.0 - 1e-6)

    def test_v3_and_v4_agree_on_clean_input(self):
        orig, T = cylinder(lambda z: 1.0, 1.5, 2.0, nz=26); src, ST = cylinder(lambda z: 1.1, 1.55, 2.0, nz=24, na=48)
        fit = orig.copy(); fit[orig[:, 2] >= 1.79, :2] *= 1.1
        a, _, ra = neck_transition(orig, fit, T, src, ST, max_delta=0.3, shape_rings=6, outlier_mm=None, fold_relax_iters=0, seam_consistent_sets=False, interior_follows_fit=False, ear_blend=False, lip_rings=0, snap_mm=None, ray_from='donor', lip_pin_mm=None, relax_keep_best=False)
        b, _, rb = neck_transition(orig, fit, T, src, ST, max_delta=0.3, shape_rings=6)
        self.assertTrue(np.allclose(a, b)); self.assertEqual(rb['demoted_outliers'], 0); self.assertEqual(rb['demoted_by_fold_relaxation'], 0)

    def test_outlier_target_is_demoted_and_fold_relaxed(self):
        # a source with a 3 cm spike at one angle gives one vertex a target far from its neighbours' targets
        def spiky(z): return 1.1
        orig, T = cylinder(lambda z: 1.0, 1.5, 2.0, nz=26); src, ST = cylinder(spiky, 1.55, 2.0, nz=24, na=48)
        k = np.flatnonzero((np.abs(src[:, 2] - 1.72) < 0.02) & (np.abs(np.arctan2(src[:, 1], src[:, 0])) < 0.07)); src[k, :2] *= 1.25
        fit = orig.copy(); fit[orig[:, 2] >= 1.79, :2] *= 1.1
        v3, _, r3 = neck_transition(orig, fit, T, src, ST, max_delta=0.4, shape_rings=6, outlier_mm=None, fold_relax_iters=0, seam_consistent_sets=False, interior_follows_fit=False, ear_blend=False, lip_rings=0, snap_mm=None, ray_from='donor')
        v4, _, r4 = neck_transition(orig, fit, T, src, ST, max_delta=0.4, shape_rings=6, outlier_mm=8.0)
        self.assertGreater(r4['demoted_outliers'], 0)
        rad3 = np.linalg.norm(v3[:, :2], axis=1); rad4 = np.linalg.norm(v4[:, :2], axis=1)
        self.assertLess(rad4.max(), rad3.max())                 # spike attenuated
        self.assertEqual(r4['frozen_max_error'], 0.0); self.assertTrue(np.allclose(v4[orig[:, 2] > 1.85], fit[orig[:, 2] > 1.85]))

    def test_seam_consistent_ear_set(self):
        # duplicate one column of vertices (a UV seam); the ear set names only one copy -> v4 expands it to the twin
        orig, T = cylinder(lambda z: 1.0, 1.5, 2.0, nz=26); src, ST = cylinder(lambda z: 1.1, 1.55, 2.0, nz=24, na=48)
        n = len(orig); col = np.flatnonzero(np.abs(np.arctan2(orig[:, 1], orig[:, 0])) < 1e-9)
        orig2 = np.vstack([orig, orig[col]]); remap = {int(c): n + i for i, c in enumerate(col)}
        T2 = T.copy()
        for fi, t in enumerate(T):   # triangles on one side of the seam use the duplicates
            if all(np.arctan2(orig[v, 1], orig[v, 0]) >= -1e-9 and np.arctan2(orig[v, 1], orig[v, 0]) < 0.5 for v in t) and any(v in remap for v in t):
                T2[fi] = [remap.get(int(v), int(v)) for v in t]
        fit2 = orig2.copy(); fit2[orig2[:, 2] >= 1.79, :2] *= 1.1
        ear = [int(c) for c in col if 1.72 < orig[c, 2] < 1.78]   # original copies only
        res, mode, rep = neck_transition(orig2, fit2, T2, src, ST, ear_ids=ear, max_delta=0.3, shape_rings=6)
        self.assertEqual(rep['seam_max_gap'], 0.0)
        for c in ear: self.assertEqual(mode[remap[c]], 5)     # twin joined the ear set


class TestNeckTransitionPhase3(unittest.TestCase):
    def _setup(self):
        orig, T = cylinder(lambda z: 1.0, 1.5, 2.0, nz=26); src, ST = cylinder(lambda z: 1.1, 1.55, 2.0, nz=24, na=48)
        fit = orig.copy(); fit[orig[:, 2] >= 1.79, :2] *= 1.1
        # the fit already sits on the source surface in a patch of the band (a "lip margin" that agrees with the source)
        patch = (orig[:, 2] > 1.70) & (orig[:, 2] < 1.74) & (np.abs(np.arctan2(orig[:, 1], orig[:, 0])) < 0.5)
        fit[patch, :2] *= 1.1
        return orig, T, src, ST, fit, patch

    def test_lip_pinning_keeps_on_surface_fit(self):
        orig, T, src, ST, fit, patch = self._setup()
        # synthetic scale: donor angles fall between the 48 source facets (chord sag up to 2.4e-3), so pin at 5e-3
        res, mode, rep = neck_transition(orig, fit, T, src, ST, max_delta=0.3, shape_rings=6, lip_pin_mm=5.0, lip_pin_rings=None, oral_ids=[int(np.flatnonzero(orig[:, 2] > 1.99)[0])])
        self.assertGreater(rep['lip_pinned_vertices'], 0)
        self.assertTrue(np.allclose(res[patch], fit[patch]))                      # pinned vertices keep the fit exactly
        self.assertEqual(int((mode[patch] == 3).sum()), int(patch.sum()))
        res0, _, rep0 = neck_transition(orig, fit, T, src, ST, max_delta=0.3, shape_rings=6, lip_pin_mm=None, oral_ids=[int(np.flatnonzero(orig[:, 2] > 1.99)[0])])
        self.assertEqual(rep0['lip_pinned_vertices'], 0)
        self.assertLess(np.abs(np.linalg.norm(res[patch, :2], axis=1) - 1.1).max(), 1e-9)   # on the source surface
        self.assertGreater(np.abs(np.linalg.norm(res0[patch, :2], axis=1) - 1.1).max(), 1e-4)  # without pinning they drift

    def test_relaxation_keeps_best_iteration(self):
        orig, T, src, ST, fit, patch = self._setup()
        _, _, rep = neck_transition(orig, fit, T, src, ST, max_delta=0.3, shape_rings=6, fold_relax_iters=2)
        self.assertTrue(rep['relaxation'][-1].get('final_check') or rep['relaxation'][-1]['demoted_confident_vertices'] == 0)
        counts = [it['introduced_defect_faces'] for it in rep['relaxation']]
        kept = rep['relaxation_kept_iteration']
        if kept is not None: self.assertLess(counts[kept - 1], counts[-1])     # a rollback only happens to a strictly better state
        self.assertEqual(rep['frozen_max_error'], 0.0)


class TestFaceQuality(unittest.TestCase):
    def test_fold_and_intersection_detection(self):
        # flat 5x5 grid: clean, then one interior vertex pushed past its opposite edge (a real fold), then one
        # triangle with reversed winding (inverted relative to its neighbourhood, dihedral 180)
        g = np.array([[x, y, 0.] for y in range(5) for x in range(5)]); T = []
        for y in range(4):
            for x in range(4):
                a = y * 5 + x; T += [[a, a + 1, a + 5], [a + 1, a + 6, a + 5]]
        T = np.array(T); m = FQ.face_metrics(g, T)
        self.assertEqual(int(m['inverted_local'].sum()), 0); self.assertEqual(int(m['folded_edge'].sum()), 0); self.assertEqual(FQ.self_intersections(g, T), [])
        Q = g.copy(); Q[12] = [3.5, 2.0, 0]                       # centre vertex dragged 1.5 cells to the right: its left fan folds over
        m2 = FQ.face_metrics(Q, T); self.assertGreater(int(m2['inverted_local'].sum()), 0); self.assertGreater(m2['max_dihedral_deg'].max(), 150)
        T4 = T.copy(); T4[13] = T4[13][::-1]
        m3 = FQ.face_metrics(g, T4); self.assertTrue(m3['inverted_local'][13]); self.assertAlmostEqual(m3['max_dihedral_deg'][13], 180.0, places=6)
        # a triangle piercing another (non-coplanar) is a self-intersection
        R = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [.3, .3, -.5], [.3, .3, .5], [.6, .1, .5]]); T3 = np.array([[0, 1, 2], [3, 4, 5]])
        self.assertEqual(FQ.self_intersections(R, T3), [(0, 1)])
        self.assertLess(FQ.min_angles_deg(np.array([[0, 0, 0], [1, 0, 0], [2, 0.01, 0.]]), np.array([[0, 1, 2]]))[0], 1.0)


if __name__ == '__main__':
    unittest.main()
