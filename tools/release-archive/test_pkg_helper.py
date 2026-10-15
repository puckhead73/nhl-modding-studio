"""Opt-in checks against the real qualified helper: NHLMS_PKG_HELPER_BUNDLE."""
import json
import os
from pathlib import Path
import tempfile
import unittest
import pkg_helper


@unittest.skipUnless(os.environ.get("NHLMS_PKG_HELPER_BUNDLE"), "real helper bundle required")
class HelperBundleTest(unittest.TestCase):
    def test_runtime_source_notices_and_tampering(self):
        source = Path(os.environ["NHLMS_PKG_HELPER_BUNDLE"])
        with tempfile.TemporaryDirectory(prefix="nhlms-pkg-bundle-") as scratch:
            record = pkg_helper.add(scratch, source)
            dest = Path(scratch) / "tools/pkg-extractor"
            self.assertEqual(record, pkg_helper.validate(dest))
            # Removing a runtime, corresponding source or notice must fail even
            # if the outer release archive's manifest is subsequently regenerated.
            for rel in ("libc++.dll", "source/shadPS4Plus.zip", "source/cryptopp.zip",
                        "licenses/zlib.txt", "licenses/shadPS4Plus.txt"):
                path = dest / rel
                data = path.read_bytes()
                path.unlink()
                with self.assertRaises(ValueError):
                    pkg_helper.validate(dest)
                path.write_bytes(data)
            binary = dest / "pkg_extractor.exe"
            data = binary.read_bytes()
            binary.write_bytes(data + b"changed")
            with self.assertRaises(ValueError):
                pkg_helper.validate(dest)
            manifest = dest / "BUNDLE.json"
            old = manifest.read_text(encoding="utf-8")
            fake = json.loads(old)
            fake["files"]["pkg_extractor.exe"] = "0" * 64
            manifest.write_text(json.dumps(fake), encoding="utf-8")
            with self.assertRaises(ValueError):
                pkg_helper.validate(dest)
            binary.write_bytes(data)
            manifest.write_text(old, encoding="utf-8")
            self.assertEqual(record, pkg_helper.validate(dest))


if __name__ == "__main__":
    unittest.main()
