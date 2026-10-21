"""Alpha-aware ray cast (eye_surface_diagnostic.raycast) and the alpha preview controls' pixel predicates: a transparent
texel must let the surface behind it register; an opaque one must not; the magenta test must accept the Workbench and
EEVEE renderings of the sentinel and reject skin / cap greys.
Run: python -m unittest tests.conversion.test_legacy_eye_preview
"""
import sys, unittest
from pathlib import Path
import numpy as np
REPO = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(REPO))
from experiments.claude.legacy import eye_surface_diagnostic as ESD
from experiments.claude.legacy.alpha_preview_controls import is_magenta


class TestAlphaAwareRaycast(unittest.TestCase):
    def scene(self):
        # camera looks along -y from the focus; a large opaque backdrop (class 6) 10 mm behind the focus and a 20 x 20 mm card (class 9) 5 mm in front
        back = np.array([[-0.1, 0.01, -0.1], [0.1, 0.01, -0.1], [0.1, 0.01, 0.1], [-0.1, 0.01, 0.1]]); bt = np.array([[0, 1, 2], [0, 2, 3]])
        card = np.array([[-0.01, -0.005, -0.01], [0.01, -0.005, -0.01], [0.01, -0.005, 0.01], [-0.01, -0.005, 0.01]]); ct = bt.copy(); uv = np.array([[0, 1], [1, 1], [1, 0], [0, 0]], float)   # v top-down
        alpha = np.zeros((8, 8), np.uint8); alpha[:, 4:] = 255                                     # left half (u < 0.5) transparent, right half opaque
        return back, bt, card, ct, uv, alpha

    def test_transparent_texels_pass_the_ray(self):
        back, bt, card, ct, uv, alpha = self.scene(); f = np.zeros(3); d = (0, -1, 0)
        opaque = ESD.raycast([(np.full(2, 6), back, bt), (np.full(2, 9), card, ct)], d, f)[0]
        aware = ESD.raycast([(np.full(2, 6), back, bt), (np.full(2, 9), card, ct, (uv, alpha, 128))], d, f)[0]
        n_op = int((opaque == 9).sum()); n_aw = int((aware == 9).sum()); self.assertGreater(n_op, 1000); self.assertLess(abs(n_aw - n_op / 2), n_op * 0.05)   # exactly the opaque half remains
        H, W = opaque.shape; centre_left = aware[H // 2, W // 2 - 60]; centre_right = aware[H // 2, W // 2 + 60]
        self.assertEqual(int(centre_left), 6); self.assertEqual(int(centre_right), 9)             # behind the transparent half the backdrop shows; the opaque half is the card
        self.assertEqual(int((aware == 0).sum()), int((opaque == 0).sum()))                        # no pixel became background: transparency reveals what is behind, never nothing
        full = ESD.raycast([(np.full(2, 6), back, bt), (np.full(2, 9), card, ct, (uv, np.full((8, 8), 255, np.uint8), 128))], d, f)[0]; self.assertTrue(np.array_equal(full, opaque))   # all-opaque alpha == the historical opaque cast

    def test_magenta_predicate(self):
        px = np.array([[[255, 0, 255], [255, 128, 255], [200, 150, 130], [204, 204, 204], [255, 255, 255], [60, 40, 30]]])
        self.assertEqual(is_magenta(px).tolist(), [[True, True, False, False, False, False]])


if __name__ == '__main__': unittest.main()
