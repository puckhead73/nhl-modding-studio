"""Licensed-fixture integration: the established native writer repacks the repaired candidate into a temporary
output, the stored positions read back from the bytes equal the pinned prediction exactly, protected bytes are
unchanged, and refusals leave no output. Skips visibly when the fixture, the built probe or the pinned candidate is
absent. Run: python -m unittest tests.adapters.test_legacy_repaired_native
"""
import hashlib, json, subprocess, sys, tempfile, unittest
from pathlib import Path
import numpy as np
ROOT = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(ROOT))
from src.adapters import legacy_native as LN
from experiments.codex.legacy_diagnostic import layouts
A = ROOT / 'local/assets/claude/legacy-2026-09-13-repaired-native-01'; EXE = ROOT / 'local/runs/codex/native-probe-build/debug/legacy_diagnostic.exe'
CAND = ROOT / 'local/runs/claude/legacy-2026-09-13-liprepair-06/grid3_sep_storage/candidate_native_mesh.json'; PRED = ROOT / 'local/runs/claude/legacy-2026-09-13-liprepair-06/grid3_sep_storage/predicted_native_mesh.json'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()


@unittest.skipUnless(EXE.exists() and (A / 'player_67.rx2').exists() and CAND.exists() and PRED.exists(), 'licensed Legacy fixture copy, built probe or pinned repair candidate absent')
class RepairedNativeTests(unittest.TestCase):
    def run_writer(self, donor, cand, out):
        return subprocess.run([str(EXE), str(donor), str(A / 'texlib_67.rx2'), str(cand), str(out)], capture_output=True, text=True)

    def test_repack_parity_and_protected_bytes(self):
        self.assertEqual(sha(CAND), '0174b2aa5bcb022fc2efc4c7260a0ace0d3b110ee65e8769ebe9de9790582daf'); self.assertEqual(sha(PRED), '0884eaf52eb489324e91722d6e51ec984354cf60d2a024409524a55b3757f423')
        with tempfile.TemporaryDirectory(dir=ROOT / 'local/runs/claude') as td:
            out = Path(td) / 'native'; r = self.run_writer(A / 'player_67.rx2', CAND, out); self.assertEqual(r.returncode, 0, r.stderr)
            before = (A / 'player_67.rx2').read_bytes(); after = (out / 'player_67_diagnostic.rx2').read_bytes(); lay = layouts(before)
            cand = json.loads(CAND.read_text(encoding='utf-8')); pred = json.loads(PRED.read_text(encoding='utf-8')); roles = [s['role'] for s in cand['submeshes']]
            for L, s, p in zip(lay, cand['submeshes'], pred['submeshes']):
                v, bits, _ = LN.read_positions(after, L); self.assertEqual(LN.compare_positions_exact(v, bits, np.array(p['positions']), np.array(s['positions']), s['name']), [])
            audit = LN.byte_change_audit(before, after, LN.position_byte_mask(len(before), lay, roles, ('head', 'eye_r', 'eye_l', 'mouthbag'))); self.assertEqual(audit['changed_outside_whitelist'], 0); self.assertEqual(audit['changed_bytes'], 11482)
            self.assertEqual(sha(out / 'player_67_diagnostic.rx2'), json.loads((ROOT / 'local/runs/claude/legacy-2026-09-13-repaired-native-01/receipt.json').read_text(encoding='utf-8'))['output']['diagnostic_rx2']['sha256'])   # deterministic writer

    def test_modified_donor_refused_without_output(self):
        with tempfile.TemporaryDirectory(dir=ROOT / 'local/runs/claude') as td:
            p = Path(td); raw = bytearray((A / 'player_67.rx2').read_bytes()); raw[200] ^= 1; (p / 'bad.rx2').write_bytes(raw)
            r = self.run_writer(p / 'bad.rx2', CAND, p / 'out'); self.assertNotEqual(r.returncode, 0); self.assertFalse((p / 'out').exists())

    def test_existing_output_refused(self):
        with tempfile.TemporaryDirectory(dir=ROOT / 'local/runs/claude') as td:
            out = Path(td) / 'taken'; out.mkdir(); r = self.run_writer(A / 'player_67.rx2', CAND, out); self.assertNotEqual(r.returncode, 0); self.assertEqual(list(out.iterdir()), [])


if __name__ == '__main__':
    unittest.main()
