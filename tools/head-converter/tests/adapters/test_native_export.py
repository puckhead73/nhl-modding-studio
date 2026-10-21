"""Guarded export regressions: explicit input validation (run root, packer script vs native packer executable, import
preconditions), the shared path guards, the route on stand-in tools (no Blender), the audit tool's own entry-point guard, and
byte parity of the two accepted NHL21 fixtures (Turgeon-02, Fedorov-02) against their demonstrated phase-11 exports under a fresh
run root outside local/runs/codex when the real Blender, packer and fixtures are present (otherwise skipped: coverage unknown)."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from src.application import export as exporter
from src.application import native_paths

FAKE = Path(__file__).resolve().with_name('fake_native_tools.py')
FIXTURES = {'turgeon': ('app-2026-09-12-job-turgeon-02', 'nhl21-phase11-export-turgeon-01'),
            'fedorov': ('app-2026-09-12-job-fedorov-02', 'nhl21-phase11-export-fedorov-01')}
DONOR_ENTRY, TEXTURE_ENTRY = 'game/player/fake_head.mcd', 'game/player/fake_head.gnf'


def sha(b): return hashlib.sha256(b).hexdigest()


def write_json(path, data): Path(path).write_text(json.dumps(data), encoding='utf-8'); return Path(path)


def build_route(root):
    """A minimal manifest/job/exchange/profile/preconditions set whose pinned files exist and hash correctly (manifest
    content checks are patched out; this exercises the exporter's own validation, not ui_manifest.check)."""
    root = Path(root); native = root / 'native'; native.mkdir()
    pins = {}
    for key in ('exchange', 'native_donor', 'texture'):
        p = native / key; p.write_bytes(key.encode() * 3); pins[key] = {'path': str(p), 'sha256': sha(p.read_bytes())}
    profile = write_json(native / 'profile.json', {
        'donor': {'sha256': pins['native_donor']['sha256']}, 'archive_provenance': {'asset_path': DONOR_ENTRY},
        'diffuse': {'provenance': {'asset_path': TEXTURE_ENTRY, 'decoded_sha256': pins['texture']['sha256']}}})
    pins['profile'] = {'path': str(profile), 'sha256': sha(profile.read_bytes())}
    exchange = write_json(root / 'exchange.json', {'schema': exporter.EXCHANGE_SCHEMA, **pins})
    preconditions = write_json(root / 'preconditions.json', {'targets': [
        {'asset_path': DONOR_ENTRY, 'decoded_sha256': pins['native_donor']['sha256']},
        {'asset_path': TEXTURE_ENTRY, 'decoded_sha256': pins['texture']['sha256']}]})
    packer_exe = root / 'fake-pack.exe'; packer_exe.write_bytes(b'not an executable, just a pinned file')
    run = root / 'run'; (run / 'stages' / '00_fit').mkdir(parents=True)
    files = {}
    for name in ('status.json', 'handoff.json', 'loss_report.json', 'native_mesh.json', 'stages/00_fit/receipt.json'):
        files[name] = write_json(run / name, {'name': name})
    (run / 'atlas.png').write_bytes(b'not really a png'); files['atlas.png'] = run / 'atlas.png'
    job = write_json(run / 'job.json', {'strategy': 'donor_fit', 'source': {'kind': 'cc5_headshot_head_only_fbx'},
                                        'destination': {'profile_id': 'test-profile-v1', **{job_key: pins[native_key] for job_key, native_key in exporter.JOB_PINS}}})
    ref = lambda p: {'path': str(p)}
    manifest = write_json(run / 'ui_manifest.json', {
        'export_ready': True, 'files': {'job_json': ref(job), 'status_json': ref(files['status.json'])},
        'artifacts': {k: ref(files[n]) for k, n in (('handoff_json', 'handoff.json'), ('native_mesh_json', 'native_mesh.json'),
                                                     ('atlas_png', 'atlas.png'), ('loss_report_json', 'loss_report.json'))},
        'stages': [{'receipt': ref(files['stages/00_fit/receipt.json'])}]})
    tools = dict(blender=FAKE, packer_script=FAKE, packer_exe=packer_exe, preconditions=preconditions, audit=FAKE, run_root=root, exchange=exchange)
    return {'manifest': manifest, 'exchange': exchange, 'job': job, 'pins': pins, 'files': files, 'run_root': root,
            'preconditions': preconditions, 'packer_exe': packer_exe, 'tools': tools}


class RouteCase(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=exporter.RUN_ROOT)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.route = build_route(self.root)
        self.tools = self.route['tools']


class PathGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup); self.root = Path(self.temp.name).resolve()

    def test_contained_and_fresh_output(self):
        self.assertEqual(native_paths.contained(self.root, self.root / 'a' / 'b'), self.root / 'a' / 'b')
        for bad, msg in ((self.root, 'run root itself'), (self.root.parent / 'sibling', 'beneath the run root'), (self.root / '..' / 'x', 'beneath the run root')):
            with self.assertRaisesRegex(ValueError, msg): native_paths.contained(self.root, bad)
        with self.assertRaisesRegex(ValueError, 'not a directory'): native_paths.contained(self.root / 'absent', self.root / 'absent' / 'x')
        (self.root / 'taken').mkdir()
        with self.assertRaisesRegex(ValueError, 'fresh directory'): native_paths.fresh_output_beneath(self.root, self.root / 'taken')
        self.assertEqual(native_paths.fresh_output_beneath(self.root, self.root / 'new'), self.root / 'new')

    def test_plan_artifact_names(self):
        native = self.root / 'native'; native.mkdir(); (native / 'a.mcd').write_bytes(b'a'); (native / 'sub').mkdir()
        (self.root / 'outside.gnf').write_bytes(b'o')
        plan = lambda *names: {'replacements': [{'file': n} for n in names]}
        self.assertEqual(native_paths.plan_artifact_names(plan('a.mcd'), native), ['a.mcd'])
        for names, msg in ((('a.mcd', 'a.mcd'), 'more than once'), (('../outside.gnf',), 'plain file name'), ((str(self.root / 'outside.gnf'),), 'plain file name'),
                           (('sub/x',), 'plain file name'), (('sub',), 'regular file'), (('missing.gnf',), 'regular file'), (('',), 'plain file name'), (('..',), 'plain file name')):
            with self.assertRaisesRegex(ValueError, msg, msg=names): native_paths.plan_artifact_names(plan(*names), native)
        for bad in ({}, {'replacements': []}, {'replacements': [{'entry': 'x'}]}, {'replacements': ['a.mcd']}):
            with self.assertRaises(ValueError): native_paths.plan_artifact_names(bad, native)

    def test_preconditions_structure_and_target_agreement(self):
        p = self.root / 'pre.json'
        for content in ('{bad', '[]', '{"targets": []}', '{"targets": [{"asset_path": 1, "decoded_sha256": "x"}]}', '{"targets": [{"asset_path": "a"}]}'):
            p.write_text(content, encoding='utf-8')
            with self.assertRaises(ValueError, msg=content): native_paths.load_preconditions(p)
        with self.assertRaisesRegex(ValueError, 'missing'): native_paths.load_preconditions(self.root / 'absent.json')
        (self.root / 'pre.txt').write_text('{}', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'must be a .json file'): native_paths.load_preconditions(self.root / 'pre.txt')
        good = write_json(self.root / 'good.json', {'targets': [{'asset_path': 'a', 'decoded_sha256': '1'}]})
        self.assertEqual(native_paths.parse_preconditions(good.read_bytes(), good), native_paths.load_preconditions(good))
        with self.assertRaisesRegex(ValueError, 'not readable JSON'): native_paths.parse_preconditions(b'\xff\xfe', 'x')
        data = {'targets': [{'asset_path': 'a', 'decoded_sha256': '1'}, {'asset_path': 'b', 'decoded_sha256': '2'}, {'asset_path': 'b', 'decoded_sha256': '2'}]}
        self.assertEqual(native_paths.preconditions_target(data, 'a', '1')['asset_path'], 'a')
        with self.assertRaisesRegex(ValueError, 'does not match'): native_paths.preconditions_target(data, 'a', '9')
        with self.assertRaisesRegex(ValueError, 'exactly once \\(found 2\\)'): native_paths.preconditions_target(data, 'b', '2')
        with self.assertRaisesRegex(ValueError, 'exactly once \\(found 0\\)'): native_paths.preconditions_target(data, 'c', '3')


class InputValidationTests(RouteCase):
    def resolve(self, output=None, **overrides):
        return exporter.resolve_inputs(self.route['manifest'], output or self.root / 'out', **{**self.tools, **overrides})

    def test_escape_and_existing_output_refused_before_manifest_read(self):
        for output in (exporter.REPO.parent / 'outside-export', self.root):
            with self.assertRaises(ValueError):
                exporter.preflight(self.root / 'missing.json', output, exporter.BLENDER)
        with self.assertRaisesRegex(ValueError, 'beneath the run root'):
            self.resolve(exporter.RUN_ROOT / 'sibling-export')
        (self.root / 'taken').mkdir()
        with self.assertRaisesRegex(ValueError, 'fresh directory'):
            self.resolve(self.root / 'taken')

    def test_missing_tools_run_root_exchange_and_preconditions_refused(self):
        for key, bad in (('run_root', self.root / 'absent'), ('blender', self.root / 'blender.exe'), ('python', self.root / 'py.exe'),
                         ('packer_script', self.route['files']['status.json']), ('packer_exe', self.root / 'absent-pack.exe'),
                         ('preconditions', self.root / 'absent.json'), ('preconditions', self.route['packer_exe']),
                         ('audit', self.root / 'audit.py'), ('exchange', self.root / 'no.json')):
            with self.assertRaises(ValueError, msg=key):
                self.resolve(**{key: bad})
        inputs = self.resolve()
        self.assertEqual((inputs['exchange'], inputs['packer_exe'], inputs['preconditions']),
                         (self.route['exchange'].resolve(), self.route['packer_exe'].resolve(), self.route['preconditions'].resolve()))
        self.assertFalse((self.root / 'out').exists())

    def test_exchange_schema_pins_and_json_validated(self):
        self.assertEqual(set(exporter.load_exchange(self.route['exchange'])), set(exporter.EXCHANGE_PINS))
        bad = self.root / 'bad.json'
        bad.write_text('{not json', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'not readable JSON'): exporter.load_exchange(bad)
        write_json(bad, {'schema': 'other/1', **self.route['pins']})
        with self.assertRaisesRegex(ValueError, 'is not nhl21-phase3-handoff/1'): exporter.load_exchange(bad)
        pins = dict(self.route['pins']); pins['profile'] = {**pins['profile'], 'sha256': '0' * 64}
        write_json(bad, {'schema': exporter.EXCHANGE_SCHEMA, **pins})
        with self.assertRaisesRegex(ValueError, 'exchange.profile.*does not match'): exporter.load_exchange(bad)
        pins['profile'] = {'path': str(self.root / 'gone.json'), 'sha256': '0' * 64}
        write_json(bad, {'schema': exporter.EXCHANGE_SCHEMA, **pins})
        with self.assertRaisesRegex(ValueError, 'exchange.profile: missing file'): exporter.load_exchange(bad)
        del pins['profile']; write_json(bad, {'schema': exporter.EXCHANGE_SCHEMA, **pins})
        with self.assertRaisesRegex(ValueError, 'exchange.profile: reference'): exporter.load_exchange(bad)

    def test_job_pins_must_equal_the_given_exchange(self):
        other = self.root / 'other'; other.mkdir()
        foreign = build_route(other)   # same layout, other location, identical hashes -> accepted
        with patch.object(exporter, 'check', return_value=(True, [])):
            manifest, inputs, pins = exporter.preflight(self.route['manifest'], self.root / 'out', **{**self.tools, 'exchange': foreign['exchange']})
        self.assertEqual(pins['exchange']['sha256'], sha(foreign['exchange'].read_bytes()))
        self.assertEqual(pins['destination']['profile_id'], 'test-profile-v1')
        self.assertEqual(set(pins['tools']), {'blender', 'packer_script', 'packer_exe', 'audit'})
        self.assertEqual(pins['preconditions']['targets']['donor'], {'asset_path': DONOR_ENTRY, 'decoded_sha256': self.route['pins']['native_donor']['sha256']})
        # A job pinned to a different donor than the exchange is refused before any write.
        (other / 'native' / 'native_donor').write_bytes(b'different donor')
        pins2 = dict(foreign['pins']); pins2['native_donor'] = {'path': str(other / 'native' / 'native_donor'), 'sha256': sha(b'different donor')}
        write_json(foreign['exchange'], {'schema': exporter.EXCHANGE_SCHEMA, **pins2})
        with patch.object(exporter, 'check', return_value=(True, [])):
            with self.assertRaisesRegex(ValueError, 'changed destination donor_mcd'):
                exporter.preflight(self.route['manifest'], self.root / 'out', **{**self.tools, 'exchange': foreign['exchange']})
        self.assertFalse((self.root / 'out').exists())

    def test_preconditions_must_agree_with_the_pinned_profile(self):
        bad = self.root / 'pre-bad.json'
        donor, texture = self.route['pins']['native_donor']['sha256'], self.route['pins']['texture']['sha256']
        cases = [({'targets': [{'asset_path': DONOR_ENTRY, 'decoded_sha256': '0' * 64}, {'asset_path': TEXTURE_ENTRY, 'decoded_sha256': texture}]}, 'does not match'),
                 ({'targets': [{'asset_path': DONOR_ENTRY, 'decoded_sha256': donor}]}, 'exactly once \\(found 0\\)'),
                 ({'targets': [{'asset_path': DONOR_ENTRY, 'decoded_sha256': donor}] * 2 + [{'asset_path': TEXTURE_ENTRY, 'decoded_sha256': texture}]}, 'found 2'),
                 ({'targets': [{'asset_path': DONOR_ENTRY}]}, 'needs string asset_path')]
        for data, msg in cases:
            write_json(bad, data)
            with patch.object(exporter, 'check', return_value=(True, [])):
                with self.assertRaisesRegex(ValueError, msg):
                    exporter.preflight(self.route['manifest'], self.root / 'out', **{**self.tools, 'preconditions': bad})
        self.assertFalse((self.root / 'out').exists())

    def test_blocked_manifest_refused_without_output(self):
        output = self.root / 'blocked'
        with patch.object(exporter, 'load', return_value={'export_ready': False}), patch.object(exporter, 'check', return_value=(True, [])):
            with self.assertRaisesRegex(ValueError, 'not export-ready'):
                exporter.export(self.route['manifest'], output, **self.tools)
        self.assertFalse(output.exists())

    def test_invalid_manifest_refused_without_output(self):
        output = self.root / 'invalid'
        with patch.object(exporter, 'load', return_value={}), patch.object(exporter, 'check', return_value=(False, ['hash mismatch'])):
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                exporter.export(self.route['manifest'], output, **self.tools)
        self.assertFalse(output.exists())


class StandInRouteTests(RouteCase):
    """The full export sequence on the stand-in packer/audit: receipt inputs, derived artifact names, failure receipts."""
    def setUp(self):
        super().setUp()
        self.check = patch.object(exporter, 'check', return_value=(True, [])); self.check.start(); self.addCleanup(self.check.stop)

    def run_export(self, mode, output, **overrides):
        with patch.dict(os.environ, {'FAKE_NATIVE_MODE': mode}):
            return exporter.export(self.route['manifest'], output, **{**self.tools, **overrides})

    def failed_receipt(self, output):
        receipt = json.loads((output / 'export.json').read_text())
        self.assertEqual((receipt['status'], receipt['artifacts']), ('failed', []))
        return receipt

    def test_completed_receipt_records_inputs_and_packer_named_artifacts(self):
        output = self.root / 'ok'
        receipt = self.run_export('ok', output)
        self.assertEqual(receipt['status'], 'completed')
        self.assertEqual(json.loads((output / 'export.json').read_text()), receipt)
        self.assertFalse((output / 'export.pending.json').exists())
        self.assertEqual(receipt['inputs']['exchange']['sha256'], sha(self.route['exchange'].read_bytes()))
        self.assertEqual(receipt['inputs']['exchange']['pins'], self.route['pins'])
        self.assertEqual(receipt['inputs']['tools']['packer_script'], {'path': str(FAKE), 'sha256': sha(FAKE.read_bytes())})
        self.assertEqual(receipt['inputs']['tools']['packer_exe'], {'path': str(self.route['packer_exe'].resolve()), 'sha256': sha(self.route['packer_exe'].read_bytes())})
        self.assertEqual(receipt['inputs']['preconditions']['sha256'], sha(self.route['preconditions'].read_bytes()))
        self.assertEqual(receipt['inputs']['run_root'], str(self.root.resolve()))
        self.assertIn('test-profile-v1', receipt['scope'])
        self.assertEqual([a['path'] for a in receipt['artifacts']],
                         [str(Path('nhl21-phase-native') / n) for n in ('fake_head.mcd', 'fake_head.gnf', 'validation.json', 'final-audit.json')])
        expected_mcd = sha(sha(self.route['files']['native_mesh.json'].read_bytes()).encode())
        self.assertEqual(receipt['artifacts'][0]['sha256'], expected_mcd)
        for a in receipt['artifacts'] + receipt['evidence']:
            self.assertEqual(sha((output / a['path']).read_bytes()), a['sha256'])
        self.assertEqual([e['path'] for e in receipt['evidence']][-1], str(Path('evidence/stages/00/receipt.json')))
        pack, audit = receipt['commands']
        for flag, value in (('--run-root', self.root.resolve()), ('--packer-exe', self.route['packer_exe'].resolve()), ('--preconditions', self.route['preconditions'].resolve())):
            self.assertEqual(pack[pack.index(flag) + 1], str(value))
        self.assertEqual(audit[audit.index('--run-root') + 1], str(self.root.resolve()))

    def test_failed_packer_preserves_failure_receipt(self):
        output = self.root / 'failed'
        with self.assertRaisesRegex(RuntimeError, 'pack.log: process exit 3'):
            self.run_export('pack_fail', output)
        receipt = self.failed_receipt(output)
        self.assertEqual(receipt['evidence'], [])
        self.assertTrue((output / 'pack.log').exists())
        self.assertFalse((output / 'nhl21-phase-native').exists())

    def test_failed_audit_preserves_failure_receipt(self):
        output = self.root / 'audit-failed'
        with self.assertRaisesRegex(RuntimeError, 'audit.log: process exit 5'):
            self.run_export('audit_fail', output)
        self.failed_receipt(output)
        self.assertTrue((output / 'nhl21-phase-native' / 'fake_head.mcd').exists())

    def test_plan_escape_and_missing_outputs_fail_after_packing_without_advertising_artifacts(self):
        for mode, msg in (('plan_escape', 'plain file name'), ('plan_missing', 'regular file inside the native output')):
            output = self.root / mode
            with self.assertRaisesRegex(ValueError, msg):
                self.run_export(mode, output)
            receipt = self.failed_receipt(output)
            self.assertTrue((output / 'nhl21-phase-native' / 'final-audit.json').exists(), 'packing and audit ran before the plan was rejected')
            self.assertTrue(receipt['evidence'])

    def test_manifest_change_during_export_fails(self):
        output = self.root / 'changed'
        original = exporter.sha
        def racing(path):
            if Path(path) == self.route['manifest'].resolve() and (output / 'audit.log').exists():
                self.route['manifest'].write_text('{"export_ready": true, "tampered": 1}', encoding='utf-8')
            return original(path)
        with patch.object(exporter, 'sha', racing):
            with self.assertRaisesRegex(ValueError, 'Manifest changed during export'):
                self.run_export('ok', output)
        self.failed_receipt(output)

    def test_pinned_input_change_during_export_fails(self):
        output = self.root / 'pins-changed'
        original = exporter.pinned_inputs_stable
        def tamper(pins):
            self.route['preconditions'].write_text(self.route['preconditions'].read_text() + ' ', encoding='utf-8')
            self.route['packer_exe'].write_bytes(b'rebuilt')
            return original(pins)
        with patch.object(exporter, 'pinned_inputs_stable', tamper):
            with self.assertRaisesRegex(ValueError, 'Pinned inputs changed during export: preconditions, tools.packer_exe'):
                self.run_export('ok', output)
        self.failed_receipt(output)


class AuditEntryPointTests(unittest.TestCase):
    """audit_candidate.py applies the run-root containment guard itself, before reading any candidate file."""
    def setUp(self):
        try:
            import numpy  # noqa: F401
        except ImportError: self.skipTest('numpy missing for the audit script')
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup); self.root = Path(self.temp.name).resolve()

    def audit(self, candidate, *extra):
        return subprocess.run([sys.executable, '-B', str(exporter.AUDIT), str(candidate), '--offline', *extra], capture_output=True, text=True, cwd=exporter.REPO)

    def test_candidate_outside_or_missing_run_root_refused(self):
        outside = self.root / 'candidate'; outside.mkdir(); (outside / 'native_mesh.json').write_text('{"submeshes": []}')
        for extra in (('--run-root', str(self.root / 'absent')), ('--run-root', str(self.root / 'candidate')), ()):
            result = self.audit(outside, *extra)
            self.assertNotEqual(result.returncode, 0, extra)
            self.assertIn('ValueError', result.stderr, extra)
        self.assertFalse((outside / 'final-audit.json').exists())
        result = self.audit(self.root / 'nothing-here', '--run-root', str(self.root))
        self.assertNotEqual(result.returncode, 0); self.assertIn('NotADirectoryError', result.stderr)


class BuildEntryPointTests(unittest.TestCase):
    """build.py itself (through the real Blender entry point, real Turgeon handoff) refuses import preconditions whose required
    donor/texture targets disagree with the pinned profile BEFORE any output directory exists and before the native packer runs."""
    def setUp(self):
        self.run, _ = FIXTURES['turgeon']
        self.manifest_dir = exporter.REPO / 'local/runs/claude' / self.run
        needed = [exporter.BLENDER, exporter.PACKER_SCRIPT, exporter.PACKER_EXE, exporter.PRECONDITIONS, exporter.EXCHANGE, self.manifest_dir / 'handoff.json']
        missing = [str(p) for p in needed if not p.exists()]
        if missing: self.skipTest('direct build.py regression not runnable here; missing: ' + ', '.join(missing))
        self.temp = tempfile.TemporaryDirectory(dir=exporter.REPO / 'local/runs/claude', prefix='build-entry-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.handoff = json.loads((self.manifest_dir / 'handoff.json').read_text(encoding='utf-8'))
        self.preconditions = json.loads(exporter.PRECONDITIONS.read_text(encoding='utf-8'))
        pins = exporter.load_exchange(exporter.EXCHANGE); profile = exporter.load(pins['profile']['path'])
        self.donor_entry, self.texture_entry = profile['archive_provenance']['asset_path'], profile['diffuse']['provenance']['asset_path']

    def build(self, name, preconditions):
        pre = write_json(self.root / f'{name}-preconditions.json', preconditions)
        out = self.root / name
        cmd = [str(exporter.BLENDER), '-b', '--factory-startup', '--python-exit-code', '1', '--python', str(exporter.PACKER_SCRIPT), '--',
               '--exchange', str(exporter.EXCHANGE), '--candidate-handoff', str(self.manifest_dir / 'handoff.json'),
               '--input', self.handoff['artifacts']['native_mesh_json'], '--atlas', self.handoff['artifacts']['atlas_png'], '--out', str(out),
               '--run-root', str(self.root), '--packer-exe', str(exporter.PACKER_EXE), '--preconditions', str(pre)]
        return out, subprocess.run(cmd, capture_output=True, text=True, cwd=exporter.REPO)

    def test_wrong_or_missing_required_targets_refused_before_output(self):
        targets = self.preconditions['targets']
        by_entry = {t['asset_path']: t for t in targets}
        cases = {'wrong_donor_hash': [{**t, 'decoded_sha256': '0' * 64} if t['asset_path'] == self.donor_entry else t for t in targets],
                 'wrong_texture_hash': [{**t, 'decoded_sha256': '0' * 64} if t['asset_path'] == self.texture_entry else t for t in targets],
                 'missing_texture_target': [t for t in targets if t['asset_path'] != self.texture_entry],
                 'duplicate_donor_target': targets + [by_entry[self.donor_entry]]}
        for name, bad_targets in cases.items():
            with self.subTest(case=name):
                out, result = self.build(name, {**self.preconditions, 'targets': bad_targets})
                self.assertEqual(result.returncode, 1, result.stdout[-2000:])
                self.assertIn('Import preconditions', result.stdout + result.stderr)
                self.assertFalse(out.exists(), f'{name}: output directory was created')
                self.assertFalse(list(self.root.glob(f'{name}/**/positions.mcd')), f'{name}: native packer ran')


class SnapshotConsumptionTests(unittest.TestCase):
    """build.py must consume the exchange pins and preconditions it verified (captured bytes / verified copies), never the original
    sources again. Real Blender + real Turgeon inputs; the four pins and the preconditions are DISPOSABLE COPIES which the harness
    overwrites right after build.py's first read of each. The native output must still equal the phase-11 reference."""
    HARNESS = Path(__file__).resolve().with_name('blender_tamper_harness.py')

    def setUp(self):
        self.run, self.ref = FIXTURES['turgeon']
        self.manifest_dir = exporter.REPO / 'local/runs/claude' / self.run
        needed = [exporter.BLENDER, exporter.PACKER_SCRIPT, exporter.PACKER_EXE, exporter.PRECONDITIONS, exporter.EXCHANGE,
                  self.manifest_dir / 'handoff.json', exporter.RUN_ROOT / self.ref / 'export.json']
        missing = [str(p) for p in needed if not p.exists()]
        if missing: self.skipTest('snapshot-consumption regression not runnable here; missing: ' + ', '.join(missing))
        self.temp = tempfile.TemporaryDirectory(dir=exporter.REPO / 'local/runs/claude', prefix='build-snapshot-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()

    def test_tampered_sources_after_capture_do_not_change_native_output(self):
        handoff = json.loads((self.manifest_dir / 'handoff.json').read_text(encoding='utf-8'))
        original = json.loads(exporter.EXCHANGE.read_text(encoding='utf-8'))
        copies = self.root / 'disposable'; copies.mkdir()
        pins, expected_hashes = {}, {}
        for key in exporter.EXCHANGE_PINS:
            src = Path(original[key]['path']); dst = copies / (key + src.suffix); dst.write_bytes(src.read_bytes())
            pins[key] = {'path': str(dst), 'sha256': original[key]['sha256']}; expected_hashes[key] = original[key]['sha256']
            self.assertEqual(sha(dst.read_bytes()), original[key]['sha256'])
        exchange = write_json(copies / 'exchange-handoff.json', {**original, **pins})
        preconditions = copies / 'preconditions.json'; preconditions.write_bytes(exporter.PRECONDITIONS.read_bytes())
        expected_hashes['preconditions'] = sha(preconditions.read_bytes())
        out, log = self.root / 'snapshot-export', self.root / 'tamper-log.json'
        tamper = [p['path'] for p in pins.values()] + [str(preconditions)]
        spec = write_json(self.root / 'spec.json', {'build': str(exporter.PACKER_SCRIPT), 'log': str(log), 'tamper': tamper})
        cmd = [str(exporter.BLENDER), '-b', '--factory-startup', '--python-exit-code', '1', '--python', str(self.HARNESS), '--', str(spec), '--',
               '--exchange', str(exchange), '--candidate-handoff', str(self.manifest_dir / 'handoff.json'),
               '--input', handoff['artifacts']['native_mesh_json'], '--atlas', handoff['artifacts']['atlas_png'], '--out', str(out),
               '--run-root', str(self.root), '--packer-exe', str(exporter.PACKER_EXE), '--preconditions', str(preconditions)]
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=exporter.REPO)
        self.assertEqual(result.returncode, 0, result.stdout[-3000:] + result.stderr[-3000:])
        tampered = json.loads(log.read_text(encoding='utf-8'))
        self.assertEqual(sorted(tampered['tampered']), sorted(str(Path(p).resolve()) for p in tamper), 'every source was read once then tampered')
        self.assertEqual(tampered['never_read'], [])
        for key in exporter.EXCHANGE_PINS:   # the disposable originals really are different now
            self.assertNotEqual(sha(Path(pins[key]['path']).read_bytes()), expected_hashes[key])
        self.assertNotEqual(sha(preconditions.read_bytes()), expected_hashes['preconditions'])
        # Native output equals the accepted reference -> packing consumed the verified snapshot, not the tampered sources.
        reference = json.loads((exporter.RUN_ROOT / self.ref / 'export.json').read_text(encoding='utf-8'))
        expected = {Path(a['path']).name: a['sha256'] for a in reference['artifacts']}
        for name in ('aaron_ekblad.mcd', 'head_aaron_ekblad_cm.gnf'):
            self.assertEqual(sha((out / name).read_bytes()), expected[name], name)
        validation = json.loads((out / 'validation.json').read_text(encoding='utf-8'))
        for key in exporter.EXCHANGE_PINS:   # copies carry the verified bytes; provenance still names the (now tampered) source path
            self.assertEqual(validation['sources'][key]['sha256'], expected_hashes[key])
            self.assertEqual(sha(Path(validation['sources'][key]['copy']).read_bytes()), expected_hashes[key])
            self.assertEqual(Path(validation['sources'][key]['source']).resolve(), Path(pins[key]['path']).resolve())
        native_inputs = validation['native']['native_pack_verify_inputs']
        self.assertEqual(Path(native_inputs['donor']), out / 'native_donor.mcd'); self.assertEqual(Path(native_inputs['profile']), out / 'profile.json')
        # Preconditions: the validated bytes are the ones hashed in the receipt/plan and copied into the output.
        self.assertEqual(validation['route_inputs']['preconditions']['sha256'], expected_hashes['preconditions'])
        self.assertEqual(sha((out / 'import-preconditions.json').read_bytes()), expected_hashes['preconditions'])
        plan = json.loads((out / 'installation-plan.json').read_text(encoding='utf-8'))
        self.assertEqual(plan['current_archive_preconditions']['sha256'], expected_hashes['preconditions'])
        # The offline audit (frozen candidate inputs, native byte protections) still passes on this output.
        audit = subprocess.run([sys.executable, '-B', str(exporter.AUDIT), str(out), '--offline', '--run-root', str(self.root)], capture_output=True, text=True, cwd=exporter.REPO)
        self.assertEqual(audit.returncode, 0, audit.stdout[-2000:] + audit.stderr[-2000:])
        self.assertEqual(sha((out / 'final-audit.json').read_bytes()), expected['final-audit.json'])


class AcceptedFixtureParityTests(unittest.TestCase):
    """Re-export the accepted NHL21 fixtures through the explicit-input route under a fresh run root outside local/runs/codex
    (inside the repository) with non-`nhl21-phase` output names, and require byte-identical MCD/GNF and native audit against
    the demonstrated phase-11 receipts. Needs the real Blender, Codex packer and the worker run directories."""
    def setUp(self):
        needed = [exporter.BLENDER, exporter.PACKER_SCRIPT, exporter.PACKER_EXE, exporter.PRECONDITIONS, exporter.AUDIT, exporter.EXCHANGE]
        needed += [exporter.REPO / 'local/runs/claude' / run / 'ui_manifest.json' for run, _ in FIXTURES.values()]
        needed += [exporter.RUN_ROOT / ref / 'export.json' for _, ref in FIXTURES.values()]
        missing = [str(p) for p in needed if not p.exists()]
        if missing: self.skipTest('accepted-fixture parity not runnable here; missing: ' + ', '.join(missing))
        try:
            import numpy  # noqa: F401  (the audit script needs it under the interpreter running these tests)
        except ImportError: self.skipTest('numpy missing for the audit interpreter')
        self.temp = tempfile.TemporaryDirectory(dir=exporter.REPO / 'local/runs/claude', prefix='export-root-parity-')
        self.addCleanup(self.temp.cleanup)

    def test_turgeon_and_fedorov_reexports_match_demonstrated_receipts(self):
        root = Path(self.temp.name)
        for name, (run, ref) in FIXTURES.items():
            with self.subTest(fixture=name):
                reference = json.loads((exporter.RUN_ROOT / ref / 'export.json').read_text(encoding='utf-8'))
                expected = {Path(a['path']).name: a['sha256'] for a in reference['artifacts']}
                manifest = exporter.REPO / 'local/runs/claude' / run / 'ui_manifest.json'
                self.assertEqual(reference['manifest']['sha256'], sha(manifest.read_bytes()), 'fixture manifest changed since phase 11')
                receipt = exporter.export(manifest, root / f'{name}-export', exchange=exporter.EXCHANGE, run_root=root, packer_script=exporter.PACKER_SCRIPT,
                                          packer_exe=exporter.PACKER_EXE, preconditions=exporter.PRECONDITIONS, audit=exporter.AUDIT)
                self.assertEqual(receipt['status'], 'completed')
                got = {Path(a['path']).name: a['sha256'] for a in receipt['artifacts']}
                for file in ('aaron_ekblad.mcd', 'head_aaron_ekblad_cm.gnf', 'final-audit.json'):
                    self.assertEqual(got[file], expected[file], f'{name}: {file} differs from the demonstrated export')
                self.assertEqual(receipt['inputs']['destination']['profile_id'], 'nhl21-ps4-cusa18182-game90-aaron-ekblad-v1')
                self.assertEqual(receipt['inputs']['run_root'], str(root.resolve()))


if __name__ == '__main__': unittest.main()
