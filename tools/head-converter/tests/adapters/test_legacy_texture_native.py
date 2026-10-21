"""Optional licensed-fixture integration: actual encoder stream, native reparse, protection."""
import unittest,tempfile,subprocess,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
R=ROOT/'local/runs/codex/legacy-texture-shading-2026-09-13-02'
D=ROOT/'local/assets/codex/legacy-native-diagnostic-2026-09-13-02/texlib_67.rx2'
EXE=ROOT/'local/runs/codex/native-probe-build/debug/legacy_texture.exe'
@unittest.skipUnless(EXE.exists() and D.exists() and (R/'head-base.bc1').exists(),'licensed Legacy fixture or built probe absent')
class NativeTextureTests(unittest.TestCase):
    def run_probe(self,donor,bc,out):
        return subprocess.run([str(EXE),str(donor),str(bc),str(R/'player_67_SHADING_ONLY.rx2'),str(out)],capture_output=True)
    def test_extract_and_protected_tail(self):
        with tempfile.TemporaryDirectory(dir=R) as td:
            out=Path(td)/'native';r=self.run_probe(D,R/'head-base.bc1',out);self.assertEqual(r.returncode,0,r.stderr)
            self.assertEqual((out/'base-extracted.bc1').read_bytes(),(R/'head-base.bc1').read_bytes())
            old=D.read_bytes();new=(out/'texlib_67_BASE_ONLY.rx2').read_bytes()
            self.assertEqual(old[:135168],new[:135168]);self.assertEqual(old[266240:],new[266240:]);self.assertEqual(len(old),len(new))
            receipt=json.loads((out/'receipt.json').read_text());self.assertFalse(receipt['export_ready'])
    def test_modified_protected_source_refused_without_output(self):
        with tempfile.TemporaryDirectory(dir=R) as td:
            p=Path(td);raw=bytearray(D.read_bytes());raw[100]^=1;(p/'bad.rx2').write_bytes(raw)
            r=self.run_probe(p/'bad.rx2',R/'head-base.bc1',p/'out');self.assertNotEqual(r.returncode,0);self.assertFalse((p/'out').exists())
    def test_wrong_base_size_refused_without_output(self):
        with tempfile.TemporaryDirectory(dir=R) as td:
            p=Path(td);(p/'bad.bc1').write_bytes(b'x')
            r=self.run_probe(D,p/'bad.bc1',p/'out');self.assertNotEqual(r.returncode,0);self.assertFalse((p/'out').exists())
if __name__=='__main__':unittest.main()
