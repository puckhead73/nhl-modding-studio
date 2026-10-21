"""Run the adapted installer against synthetic workspace archives only."""
import contextlib
import hashlib
import io
import json
from pathlib import Path
import runpy
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'experiments/codex/nhl21_package'))
from runtime_packet import installer_text


class RuntimeInstallerTests(unittest.TestCase):
    def setUp(self):
        runs = REPO / 'local/runs/codex/nhl21-installer-tests'
        runs.mkdir(parents=True, exist_ok=True)
        self.root = Path(tempfile.mkdtemp(prefix='synthetic-', dir=runs))
        self.game = self.root / 'CUSA18182'
        (self.game / 'databigs').mkdir(parents=True)
        self.archive = self.game / 'databigs/game_90.big'
        self.package = self.root / 'packet'
        self.package.mkdir()
        self.script = self.package / 'nhl21_test_install.py'
        self.script.write_text(installer_text())
        self.sha = lambda b: hashlib.sha256(b).hexdigest()
        raw = bytearray(128)
        raw[:4] = b'EB\0\3'
        struct.pack_into('>Q', raw, 24, 128)
        row = struct.pack('>5I', 6, 7, 16, 8, 9)
        raw[48:68] = row
        self.original = bytes(raw)
        self.archive.write_bytes(raw)
        payload = b'test replacement payload'
        (self.package / 'replacement.bin').write_bytes(payload)
        new_header = bytearray(raw[:48])
        struct.pack_into('>Q', new_header, 24, 128 + len(payload))
        self.manifest = {'game_root': str(self.game), 'archives': [{
            'path': str(self.archive), 'original_size': 128, 'original_sha256': self.sha(raw),
            'original_header': raw[:48].hex(), 'new_header': new_header.hex(),
            'new_size': 128 + len(payload), 'changes': [{'row_offset': 48, 'row': row.hex(),
                'new_row': struct.pack('>5I', 8, 7, len(payload), 8, 9).hex(),
                'new_offset': 128, 'stored_file': 'replacement.bin', 'stored_sha256': self.sha(payload)}]}]}
        (self.package / 'package.json').write_text(json.dumps(self.manifest))

    def run_installer(self, mode, active=False):
        # Every file target is under this test's isolated workspace directory.
        self.archive.resolve().relative_to(self.root.resolve())
        with patch.object(sys, 'argv', [str(self.script), str(self.package), mode]), \
             patch('subprocess.run') as process, contextlib.redirect_stdout(io.StringIO()):
            process.return_value.stdout = '"shadps4.exe","123"\n' if active else ''
            runpy.run_path(str(self.script), run_name='__main__')

    def test_apply_and_exact_restore(self):
        self.run_installer('verify')
        self.assertEqual(self.archive.read_bytes(), self.original)
        self.run_installer('apply')
        self.assertNotEqual(self.archive.read_bytes(), self.original)
        self.run_installer('restore')
        self.assertEqual(self.archive.read_bytes(), self.original)

    def test_modified_payload_refused_before_write(self):
        (self.package / 'replacement.bin').write_bytes(b'changed')
        with self.assertRaises(AssertionError):
            self.run_installer('apply')
        self.assertEqual(self.archive.read_bytes(), self.original)

    def test_game_running_refused(self):
        with self.assertRaises(RuntimeError):
            self.run_installer('apply', active=True)
        self.assertEqual(self.archive.read_bytes(), self.original)

    def test_modified_installed_archive_refuses_restore(self):
        self.run_installer('apply')
        changed = bytearray(self.archive.read_bytes())
        changed[-1] ^= 1
        self.archive.write_bytes(changed)
        with self.assertRaises(AssertionError):
            self.run_installer('restore')
        self.assertEqual(self.archive.read_bytes(), changed)


if __name__ == '__main__':
    unittest.main()
