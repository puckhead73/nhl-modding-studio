"""Coupled crease term (src/conversion/crease.py): two faces can each satisfy an independent orientation hinge while
counter-rotating into a shared-edge fold - the dihedral term must catch it; inherited creases at their reference angle
must stay free; the sign must separate a valley from a ridge; degenerate edges must not produce NaN; the analytic
gradient must match finite differences at smooth points, active hinges and tiny triangles.
Run: python -m unittest tests.conversion.test_crease
"""
import sys, unittest
from pathlib import Path
import numpy as np
REPO = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(REPO))
from src.conversion import crease as CR, face_quality as FQ


def book(angle_deg, sign=+1):
    """Two triangles sharing the edge (0,1) along x; face 0 = (0,1,2) in +y, face 1 = (1,0,3) in -y, both rotated about the
    edge by angle/2 in opposite directions so the dihedral is `angle` (sign chooses ridge or valley)."""
    a = np.radians(angle_deg / 2); X = np.array([[0, 0, 0], [1, 0, 0], [0.5, np.cos(a), sign * np.sin(a)], [0.5, -np.cos(a), sign * np.sin(a)]], float)
    T = np.array([[0, 1, 2], [1, 0, 3]]); return X, T


def fd_grad(X, T, ES, lo, hi, h=1e-6):
    g = np.zeros_like(X)
    for i in range(len(X)):
        for k in range(3):
            e = np.zeros_like(X); e[i, k] = h; g[i, k] = (CR.crease_energy(X + e, T, ES, lo, hi)[0] - CR.crease_energy(X - e, T, ES, lo, hi)[0]) / (2 * h)
    return g


class TestCrease(unittest.TestCase):
    def test_signed_dihedral_and_structure(self):
        X, T = book(0); ES = CR.edge_structure(T, [0, 1]); self.assertEqual(ES['n'], 1); self.assertEqual((int(ES['a'][0]), int(ES['b'][0])), (0, 1)); self.assertEqual(int(ES['c'][0]), 2); self.assertEqual(int(ES['d'][0]), 3)
        self.assertAlmostEqual(CR.signed_dihedrals(X, T, ES)[0], 0.0, places=12)
        th_up = CR.signed_dihedrals(*book(80, +1)[:1], T, ES)[0]; th_dn = CR.signed_dihedrals(*book(80, -1)[:1], T, ES)[0]
        self.assertAlmostEqual(abs(np.degrees(th_up)), 80.0, places=9); self.assertAlmostEqual(th_up, -th_dn, places=12)     # ridge and valley have opposite signs
        # surround: a third face adjacent to face 1 only - its shared edge with face 1 is included because face 1 is in the support
        X3 = np.vstack([X, [[1.5, -0.5, 0.0]]]); T3 = np.vstack([T, [[1, 3, 4]]]); ES3 = CR.edge_structure(T3, [0]); self.assertEqual(ES3['n'], 1)    # only edge (0,1): face 2 is not adjacent to face 0
        ES3b = CR.edge_structure(T3, [1]); self.assertEqual(ES3b['n'], 2)                                                                            # support face 1: its two interior edges, including the one to the surround face

    def test_hinges_miss_the_fold_but_the_crease_term_catches_it(self):
        X0, T = book(0); ES = CR.edge_structure(T, [0, 1]); lo, hi = CR.envelope([CR.signed_dihedrals(X0, T, ES)], np.radians(20))
        X, _ = book(80)                                                     # each face rotated 40 deg: dot to its flat reference = cos 40 = 0.77 >= 0.5 -> the orientation hinge passes
        n0 = FQ.face_normals(X0, T); n1 = FQ.face_normals(X, T); self.assertTrue(((n0 * n1).sum(1) >= 0.5).all())
        E, g, th = CR.crease_energy(X, T, ES, lo, hi); self.assertGreater(E, 0); self.assertAlmostEqual(np.sqrt(E), np.radians(80 - 20), places=9)   # residual = excess over the envelope
        self.assertLess(np.abs(g - fd_grad(X, T, ES, lo, hi)).max(), 1e-6)
        Ein, _, _ = CR.crease_energy(book(15)[0], T, ES, lo, hi); self.assertEqual(Ein, 0.0)                                                          # inside the margin: free

    def test_inherited_crease_is_permitted_and_sign_flip_is_not(self):
        Xr, T = book(80, +1); ES = CR.edge_structure(T, [0, 1]); th_r = CR.signed_dihedrals(Xr, T, ES); lo, hi = CR.envelope([th_r, CR.signed_dihedrals(book(60, +1)[0], T, ES)], np.radians(10))
        self.assertEqual(CR.crease_energy(Xr, T, ES, lo, hi)[0], 0.0)                     # the donor's own 80 deg crease costs nothing
        self.assertEqual(CR.crease_energy(book(70, +1)[0], T, ES, lo, hi)[0], 0.0)         # anywhere between the two references
        self.assertGreater(CR.crease_energy(book(100, +1)[0], T, ES, lo, hi)[0], 0.0)      # sharpened beyond hi
        Ef, gf, _ = CR.crease_energy(book(80, -1)[0], T, ES, lo, hi); self.assertAlmostEqual(np.sqrt(Ef), np.radians(80 + 60 - 10), places=9)   # the same magnitude with the opposite sign = a fold-over, far outside
        self.assertLess(np.abs(gf - fd_grad(book(80, -1)[0], T, ES, lo, hi)).max(), 1e-6)
        cls = CR.classify_edges(ES, th_r, np.zeros(2, bool), np.ones(2, bool)); self.assertEqual(cls[0], 'inherited_crease')
        self.assertEqual(CR.classify_edges(ES, np.zeros(1), np.array([True, False]), np.ones(2, bool))[0], 'fan'); self.assertEqual(CR.classify_edges(ES, np.zeros(1), np.zeros(2, bool), np.array([True, False]))[0], 'interior')

    def test_donor_signed_envelope_keeps_sign_and_allows_smooth_adaptation(self):
        # single anatomical reference policy (cap-radius study): donor +- margin; on crease edges the sign is kept; smooth edges may cross flat
        Xr, T = book(60, +1); ES = CR.edge_structure(T, [0, 1]); th_d = CR.signed_dihedrals(Xr, T, ES)
        lo, hi, is_cr = CR.envelope_donor_signed(th_d, np.radians(20), np.radians(15)); self.assertTrue(is_cr[0]); self.assertAlmostEqual(abs(np.degrees(th_d[0])), 60, places=6)
        self.assertAlmostEqual(np.degrees(hi[0] - lo[0]), 40, places=6); self.assertAlmostEqual(np.degrees(min(abs(lo[0]), abs(hi[0]))), 40, places=6)   # [donor - 20, donor + 20] on the donor's side of flat
        self.assertEqual(CR.crease_energy(book(70, +1)[0], T, ES, lo, hi)[0], 0.0); self.assertGreater(CR.crease_energy(book(90, +1)[0], T, ES, lo, hi)[0], 0.0)
        self.assertGreater(CR.crease_energy(book(60, -1)[0], T, ES, lo, hi)[0], 0.0)                              # the same crease with the opposite sign: outside (no fit-input union can admit it)
        # a crease edge whose donor is only 18 deg from flat: the margin would cross 0, but the sign clip stops at flat (for either sign)
        for sgn in (+1, -1):
            Xs, _ = book(18, sgn); th_s = CR.signed_dihedrals(Xs, T, ES); lo2, hi2, cr2 = CR.envelope_donor_signed(th_s, np.radians(20), np.radians(15)); self.assertTrue(cr2[0])
            self.assertAlmostEqual((lo2[0] if th_s[0] > 0 else hi2[0]), 0.0, places=12); self.assertAlmostEqual(np.degrees(abs(hi2[0] if th_s[0] > 0 else lo2[0])), 38, places=6)
        # a smooth edge (|donor| < sign_deg) is not clipped: it may cross flat within the margin
        Xf, _ = book(5, +1); th_f = CR.signed_dihedrals(Xf, T, ES); lo4, hi4, cr4 = CR.envelope_donor_signed(th_f, np.radians(20), np.radians(15)); self.assertFalse(cr4[0]); self.assertLess(lo4[0], 0.0)
        # free_rad: smooth edges may adapt up to the crease threshold (no new crease beyond max(|donor| + margin, free)); crease edges are unaffected
        lo5, hi5, cr5 = CR.envelope_donor_signed(np.array([th_f[0], th_d[0]]), np.radians(20), np.radians(60), np.radians(60)); self.assertFalse(cr5[0]); self.assertTrue(cr5[1])
        self.assertAlmostEqual(np.degrees(hi5[0]), 60, places=6); self.assertAlmostEqual(np.degrees(lo5[0]), -60, places=6); self.assertAlmostEqual(np.degrees(hi5[1] - lo5[1]), 40, places=6)
        # sign reversals: independent of the envelope
        rev = CR.sign_reversals(CR.signed_dihedrals(book(60, -1)[0], T, ES), th_d, is_cr); self.assertTrue(rev[0])
        self.assertFalse(CR.sign_reversals(CR.signed_dihedrals(book(30, +1)[0], T, ES), th_d, is_cr)[0])
        self.assertFalse(CR.sign_reversals(CR.signed_dihedrals(book(5, -1)[0], T, ES), th_f, cr4)[0])                # a smooth edge never counts as a reversal

    def test_wrap_around_the_branch_cut(self):
        Xr, T = book(175, +1); ES = CR.edge_structure(T, [0, 1]); th_r = CR.signed_dihedrals(Xr, T, ES); sg = np.sign(th_r[0]); lo, hi = CR.envelope([th_r, CR.signed_dihedrals(book(170, +1)[0], T, ES)], np.radians(10))
        E, g, th = CR.crease_energy(book(178, +1)[0], T, ES, lo, hi); self.assertEqual(E, 0.0)                     # 178 deg on the reference side of the cut: inside [160, 185]
        Xw = book(179, -1)[0]; th_w = CR.signed_dihedrals(Xw, T, ES)[0]; self.assertLess(th_w * sg, 0)               # the same ridge continued past 180 deg: raw atan2 reports it on the other branch
        Ew, gw, thw = CR.crease_energy(Xw, T, ES, lo, hi); self.assertEqual(Ew, 0.0); self.assertGreater(thw[0] * sg, 0)   # unwrapped to 181 deg on the reference branch: inside, no 2 pi jump
        self.assertLess(np.abs(gw - fd_grad(Xw, T, ES, lo, hi)).max(), 1e-6)
        lo2, hi2 = CR.envelope([CR.signed_dihedrals(book(179, +1)[0], T, ES), CR.signed_dihedrals(book(179, -1)[0], T, ES)], np.radians(5))
        self.assertLess(float(hi2[0] - lo2[0]), np.radians(15))                                                      # two references straddling the cut give a narrow envelope, not a 358 deg one

    def test_gradient_on_a_patch_with_active_edges_and_tiny_faces(self):
        rng = np.random.default_rng(3); n = 5; xs, ys = np.meshgrid(np.arange(n), np.arange(n)); X = np.c_[xs.ravel(), ys.ravel(), rng.normal(0, 0.15, n * n)].astype(float)
        T = []
        for i in range(n - 1):
            for j in range(n - 1):
                v = i * n + j; T += [[v, v + 1, v + n + 1], [v, v + n + 1, v + n]]
        T = np.array(T); ES = CR.edge_structure(T, np.arange(len(T))); self.assertGreater(ES['n'], 20)
        lo, hi = CR.envelope([CR.signed_dihedrals(X, T, ES)], np.radians(5)); X2 = X + rng.normal(0, 0.12, X.shape); X2[12, 2] += 0.6    # bend the middle: several edges active
        E, g, th = CR.crease_energy(X2, T, ES, lo, hi); self.assertGreater(E, 0); self.assertGreater(int(((th > hi) | (th < lo)).sum()), 3)
        self.assertLess(np.abs(g - fd_grad(X2, T, ES, lo, hi)).max(), 1e-5)
        X3 = X2.copy(); X3[7] = X3[8] + np.array([1e-9, 0, 0])                                                                  # a degenerate (near-zero) edge and two sliver faces
        E3, g3, th3 = CR.crease_energy(X3, T, ES, lo, hi); self.assertTrue(np.isfinite(E3)); self.assertTrue(np.isfinite(g3).all()); self.assertTrue(np.isfinite(th3).all())
        self.assertLess(np.abs(g - fd_grad(X2, T, ES, lo, hi)).max(), 1e-5)


if __name__ == '__main__': unittest.main()
