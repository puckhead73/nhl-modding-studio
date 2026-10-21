"""Application-facing worker interface tests: preview GLB, UI manifest, JSON-lines events, cancellation of a running
subprocess, subprocess failure, missing Blender, malformed jobs, paths with spaces and non-ASCII characters.
No Blender or licensed fixture needed (a '.py' stand-in executable is used); the licensed-fixture checks at the end
skip visibly when the local demo runs are absent.
Run: python -m unittest tests.conversion.test_app_interface
"""
import json, os, shutil, struct, subprocess, sys, tempfile, threading, time, unittest, zlib
from pathlib import Path
import numpy as np
REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO))
from src.conversion import exchange as X, job as J, preview as PV, ui_manifest as UM
from tests.conversion.test_worker import synthetic_exchange

FAKE = REPO / 'tests/conversion/fake_blender.py'
RUN_JOB = REPO / 'experiments/claude/worker/run_job.py'


def tiny_png(path, w=4, h=4, rgb=(200, 150, 120)):
    raw = b''.join(b'\x00' + bytes(rgb + (255,)) * w for _ in range(h))

    def chunk(t, d): return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    Path(path).write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b''))


def pid_alive(pid):
    if sys.platform == 'win32':
        r = subprocess.run(['tasklist', '/FI', f'PID eq {pid}', '/NH'], capture_output=True, text=True); return str(pid) in r.stdout
    try: os.kill(pid, 0); return True
    except OSError: return False


def events_of(out):
    return [json.loads(l) for l in (Path(out) / 'events.jsonl').read_text(encoding='utf-8').splitlines() if l.strip()]


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix='nhl-app-')); self.runs = self.tmp / 'runs'; self.runs.mkdir()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def job(self, d='job-01', root=None, blender=None):
        j = {'schema': 'nhl-head-job/1', 'job_id': 'synthetic', 'strategy': 'donor_fit', 'source': {'label': 'synthetic source'}, 'destination': {'label': 'synthetic donor'}, 'calibration': {}, 'policies': {}, 'tools': {}, 'output': {'run_root': str(root or self.runs), 'dir': d}}
        if blender: j['tools']['blender'] = {'path': str(blender), 'expected_version': '4.5.5'}
        return j

    def fake_handoff(self, R, status='ready_for_experimental_packing', problems=()):
        d = R.out / 'stages' / 'artifacts'; d.mkdir(exist_ok=True); ex = synthetic_exchange(); X.save(ex, d / 'native_mesh.json'); tiny_png(d / 'atlas_1024.png')
        hand = {'schema': 'claude-phase3-nhl21-candidate/1', 'status': status, 'blocking_problems': list(problems), 'artifacts': {'native_mesh_json': str(d / 'native_mesh.json'), 'native_mesh_sha256': J.sha256_file(d / 'native_mesh.json'), 'atlas_png': str(d / 'atlas_1024.png'), 'atlas_sha256': J.sha256_file(d / 'atlas_1024.png')},
                'validation_json': None, 'gates': {'G5': 'open'}, 'loss_report': {'schema': 'nhl-head-loss/1', 'entries': [{'stage': 'x', 'action': 'drop'}]}}
        (R.out / 'handoff.json').write_text(json.dumps(hand), encoding='utf-8'); (R.out / 'loss_report.json').write_text(json.dumps(hand['loss_report']), encoding='utf-8'); return hand, ex


class TestPreviewGLB(Base):
    def test_glb_round_trip_preserves_geometry_uv_and_atlas(self):
        ex = synthetic_exchange(); X.head(ex)['positions'][3][0] += 0.001   # a fitted head
        png = self.tmp / 'atlas.png'; tiny_png(png); out = self.tmp / 'preview.glb'
        info = PV.write_glb(ex, png, out, 'jobA', 'ready_for_experimental_packing')
        ok, problems, _ = PV.verify_glb(out, ex, png); self.assertTrue(ok, problems)
        g, nodes, img = PV.read_glb(out)
        self.assertEqual(img, png.read_bytes()); self.assertEqual(set(nodes), {'geo_eyesShape', 'geo_headShape'})
        h = nodes['geo_headShape']; hm = X.head(ex)
        self.assertTrue(np.array_equal(h['positions'], np.asarray(hm['positions'], np.float32)))          # native frame == glTF frame, unchanged
        self.assertTrue(np.array_equal(h['uv'], np.asarray(hm['uv'], np.float32)))                        # per-record UV unchanged
        self.assertTrue(np.array_equal(h['indices'].astype(int), np.asarray(hm['triangles'])))
        self.assertEqual(h['material']['pbrMetallicRoughness']['baseColorTexture']['index'], 0); self.assertIn('preview_approximation', h['material']['extras'])
        self.assertEqual(h['extras']['role'], 'head'); self.assertTrue(h['extras']['positions_changed_vs_donor']); self.assertTrue(h['extras']['default_visible'])
        e = nodes['geo_eyesShape']; self.assertEqual(e['extras']['role'], 'eyes'); self.assertFalse(e['extras']['positions_changed_vs_donor']); self.assertIn('baseColorFactor', e['material']['pbrMetallicRoughness'])
        self.assertFalse(g['asset']['extras']['diagnostic_only']); self.assertEqual(g['asset']['extras']['schema'], 'nhl-head-preview/1')
        self.assertTrue(any('not the game shader' in a or 'game shader' in a for a in info['approximations']))
        # deterministic bytes
        out2 = self.tmp / 'preview2.glb'; PV.write_glb(ex, png, out2, 'jobA', 'ready_for_experimental_packing'); self.assertEqual(out.read_bytes(), out2.read_bytes())
        with self.assertRaises(FileExistsError): PV.write_glb(ex, png, out, 'jobA', 'ready_for_experimental_packing')

    def test_blocked_preview_is_marked_diagnostic_and_tampering_is_detected(self):
        ex = synthetic_exchange(); png = self.tmp / 'atlas.png'; tiny_png(png); out = self.tmp / 'blocked.glb'
        PV.write_glb(ex, png, out, 'jobB', 'blocked'); g, _, _ = PV.read_glb(out); self.assertTrue(g['asset']['extras']['diagnostic_only'])
        ex2 = json.loads(json.dumps(ex)); X.head(ex2)['uv'][0][0] += 0.01
        ok, problems, _ = PV.verify_glb(out, ex2, png); self.assertFalse(ok); self.assertTrue(any('uv differ' in p for p in problems))
        png2 = self.tmp / 'other.png'; tiny_png(png2, rgb=(1, 2, 3)); ok, problems, _ = PV.verify_glb(out, ex, png2); self.assertTrue(any('atlas differs' in p for p in problems))
        b = bytearray(out.read_bytes()); b[-1] ^= 0xFF; (self.tmp / 't.glb').write_bytes(b[:-4])
        with self.assertRaises(ValueError): PV.read_glb(self.tmp / 't.glb')


class TestManifestAndEvents(Base):
    def test_completed_job_manifest_events_and_check(self):
        R = J.Runner(self.job(), log_to_stdout=False, planned_stages=['a', 'preview'])
        R.run_stage('a', lambda d, log: (log('working'), (d / 'o.txt').write_text('x'), {'outputs': {'o': d / 'o.txt'}, 'summary': {'n': 1}})[-1], params={'p': 1})
        hand, ex = self.fake_handoff(R)
        glb = R.out / 'preview.glb'; info = PV.write_glb(ex, hand['artifacts']['atlas_png'], glb, 'synthetic', hand['status'])
        R.meta['preview'] = {'status': 'ok', 'path': str(glb), 'info': {'nodes': info['nodes'], 'approximations': info['approximations']}}
        st = R.finish('completed', handoff=hand); self.assertTrue(st['ready_for_packing'])
        man = UM.load(R.out / 'ui_manifest.json'); ok, problems = UM.check(man); self.assertTrue(ok, problems)
        self.assertEqual(man['schema'], 'nhl-head-ui-manifest/1'); self.assertTrue(man['export_ready']); self.assertEqual(man['job_status'], 'completed'); self.assertEqual(man['run_id'], 'job-01')
        self.assertEqual(man['preview']['status'], 'ok'); self.assertEqual(man['preview']['file']['sha256'], J.sha256_file(glb)); self.assertFalse(man['preview']['diagnostic_only'])
        self.assertEqual(man['artifacts']['atlas_png']['sha256'], hand['artifacts']['atlas_sha256']); self.assertEqual(man['artifacts']['native_mesh_json']['relative'], 'stages/artifacts/native_mesh.json')
        self.assertTrue(man['packing']['ready']); self.assertEqual(man['losses']['entries'][0]['action'], 'drop'); self.assertEqual(man['stages'][0]['id'], '01_a'); self.assertEqual(man['files']['events_jsonl']['relative'], 'events.jsonl')
        ev = events_of(R.out); kinds = [e['event'] for e in ev]
        self.assertEqual(kinds[0], 'job_started'); self.assertEqual(kinds[-1], 'job_completed'); self.assertIn('stage_started', kinds); self.assertIn('stage_finished', kinds); self.assertIn('stage_progress', kinds)
        self.assertEqual([e['seq'] for e in ev], list(range(1, len(ev) + 1)))
        for e in ev: self.assertEqual(e['schema'], 'nhl-head-worker-event/1'); self.assertEqual(e['job_id'], 'synthetic'); self.assertEqual(e['run_id'], 'job-01'); self.assertIn('ts', e); self.assertIn('elapsed_s', e)
        sf = [e for e in ev if e['event'] == 'stage_finished'][0]; self.assertEqual(sf['stage'], {'index': 1, 'name': 'a', 'id': '01_a'}); self.assertEqual(sf['status'], 'ok'); self.assertTrue(Path(sf['receipt']).exists())
        self.assertEqual(ev[0]['planned_stages'], ['a', 'preview']); self.assertTrue(ev[0]['cancel_file'].endswith('CANCEL')); self.assertTrue(ev[-1]['export_ready']); self.assertTrue(Path(ev[-1]['manifest']).exists())
        # diagnostic log is separate from the event stream
        self.assertIn('working', (R.out / 'log.txt').read_text(encoding='utf-8')); self.assertTrue(all(l.startswith('{') for l in (R.out / 'events.jsonl').read_text(encoding='utf-8').splitlines()))

    def test_blocked_candidate_keeps_diagnostic_preview_but_is_never_export_ready(self):
        R = J.Runner(self.job('blocked'), log_to_stdout=False); hand, ex = self.fake_handoff(R, status='blocked', problems=['collision_triggers_remaining: [3]'])
        glb = R.out / 'preview.glb'; PV.write_glb(ex, hand['artifacts']['atlas_png'], glb, 'synthetic', 'blocked'); R.meta['preview'] = {'status': 'ok', 'path': str(glb), 'info': {}}
        st = R.finish('completed', handoff=hand); self.assertFalse(st['ready_for_packing'])
        man = UM.load(R.out / 'ui_manifest.json'); ok, problems = UM.check(man); self.assertTrue(ok, problems)
        self.assertFalse(man['export_ready']); self.assertEqual(man['candidate_status'], 'blocked'); self.assertTrue(man['preview']['diagnostic_only']); self.assertEqual(man['preview']['status'], 'ok'); self.assertEqual(man['packing']['blocking_problems'], ['collision_triggers_remaining: [3]'])
        man['export_ready'] = True; ok, problems = UM.check(man); self.assertFalse(ok); self.assertTrue(any('derived readiness' in p for p in problems))

    def test_incomplete_preview_is_reported_and_missing_files_fail_check(self):
        R = J.Runner(self.job('nopreview'), log_to_stdout=False); hand, ex = self.fake_handoff(R)
        R.meta['preview'] = {'status': 'failed', 'path': str(R.out / 'preview.glb'), 'error': 'preview stage did not complete'}
        R.finish('completed', handoff=hand); man = UM.load(R.out / 'ui_manifest.json')
        self.assertEqual(man['preview']['status'], 'failed'); self.assertFalse(man['preview']['file']['exists']); self.assertIn('did not complete', man['preview']['error']); self.assertTrue(man['export_ready'])   # conversion artifacts are complete; preview is not
        ok, problems = UM.check(man); self.assertFalse(ok); self.assertTrue(any('preview.glb' in p for p in problems))
        # a manifest whose preview claims ok but whose file vanished, and a tampered atlas
        R2 = J.Runner(self.job('vanish'), log_to_stdout=False); hand2, ex2 = self.fake_handoff(R2); glb = R2.out / 'preview.glb'; PV.write_glb(ex2, hand2['artifacts']['atlas_png'], glb, 's', 'ready_for_experimental_packing')
        R2.meta['preview'] = {'status': 'ok', 'path': str(glb), 'info': {}}; R2.finish('completed', handoff=hand2); man2 = UM.load(R2.out / 'ui_manifest.json'); glb.unlink()
        ok, problems = UM.check(man2); self.assertTrue(any('preview marked ok but file missing' in p for p in problems))
        Path(hand2['artifacts']['atlas_png']).write_bytes(b'garbage'); ok, problems = UM.check(man2); self.assertTrue(any('sha256 mismatch' in p and 'atlas' in p for p in problems))

    def test_failed_and_cancelled_jobs_write_manifest_without_artifacts(self):
        R = J.Runner(self.job('f'), log_to_stdout=False)
        with self.assertRaises(J.JobError): R.run_stage('bad', lambda d, log: 1 / 0)
        R.finish('failed', note='stage bad failed'); man = UM.load(R.out / 'ui_manifest.json'); ok, problems = UM.check(man); self.assertTrue(ok, problems)
        self.assertFalse(man['export_ready']); self.assertEqual(man['job_status'], 'failed'); self.assertEqual(man['artifacts'], {}); self.assertIsNone(man['candidate_status']); self.assertEqual(man['stages'][0]['status'], 'failed'); self.assertIn('ZeroDivisionError', man['stages'][0]['error'])
        self.assertEqual(events_of(R.out)[-1]['event'], 'job_failed')
        R2 = J.Runner(self.job('c'), log_to_stdout=False); (R2.out / 'CANCEL').write_text('')
        with self.assertRaises(J.Cancelled): R2.run_stage('x', lambda d, log: {})
        R2.finish('cancelled', note='user'); man2 = UM.load(R2.out / 'ui_manifest.json'); self.assertFalse(man2['export_ready']); self.assertEqual(man2['job_status'], 'cancelled'); self.assertEqual(events_of(R2.out)[-1]['event'], 'job_cancelled')


class TestReadinessHardening(Base):
    """Regressions for the Codex phase-10 finding: readiness must come from the hash-verified handoff/status, not
    from the manifest's duplicated fields (docs/PHASE10_WORK.md)."""

    def blocked_run(self, name='blocked'):
        R = J.Runner(self.job(name), log_to_stdout=False); R.run_stage('freeze_candidate', lambda d, log: {'outputs': {}, 'summary': {'status': 'blocked'}}); hand, ex = self.fake_handoff(R, status='blocked', problems=['collision_triggers_remaining: [3]'])
        glb = R.out / 'preview.glb'; PV.write_glb(ex, hand['artifacts']['atlas_png'], glb, 'synthetic', 'blocked'); R.meta['preview'] = {'status': 'ok', 'path': str(glb), 'info': {}}
        R.finish('completed', handoff=hand); return R, UM.load(R.out / 'ui_manifest.json')

    def ready_run(self, name='ready'):
        R = J.Runner(self.job(name), log_to_stdout=False); R.run_stage('freeze_candidate', lambda d, log: {'outputs': {}, 'summary': {'status': 'ready'}}); hand, ex = self.fake_handoff(R)
        glb = R.out / 'preview.glb'; PV.write_glb(ex, hand['artifacts']['atlas_png'], glb, 'synthetic', hand['status']); R.meta['preview'] = {'status': 'ok', 'path': str(glb), 'info': {}}
        R.finish('completed', handoff=hand); return R, UM.load(R.out / 'ui_manifest.json')

    def test_in_memory_promotion_of_blocked_manifest_is_rejected(self):
        R, man = self.blocked_run(); ok, problems = UM.check(man); self.assertTrue(ok, problems)
        forged = json.loads(json.dumps(man))                                  # exactly Codex's probe: duplicated fields only, hashed references untouched
        forged['export_ready'] = True; forged['candidate_status'] = 'ready_for_experimental_packing'; forged['packing']['ready'] = True; forged['packing']['handoff_status'] = 'ready_for_experimental_packing'
        forged['packing']['blocking_problems'] = []; forged['validation']['candidate_status'] = 'ready_for_experimental_packing'; forged['validation']['blocking_problems'] = []; forged['preview']['diagnostic_only'] = False
        ok, problems = UM.check(forged); self.assertFalse(ok)
        for needle in ('export_ready True contradicts derived readiness False', 'packing.ready True contradicts', "candidate_status 'ready_for_experimental_packing' contradicts handoff status 'blocked'", 'packing.blocking_problems contradict', 'validation.blocking_problems contradict', 'preview.diagnostic_only False contradicts'):
            self.assertTrue(any(needle in p for p in problems), (needle, problems))
        self.assertEqual(json.loads(Path(man['artifacts']['handoff_json']['path']).read_text(encoding='utf-8'))['status'], 'blocked')   # evidence untouched
        # partial promotions are rejected too
        for field, value in (('export_ready', True), ('candidate_status', 'ready_for_experimental_packing')):
            f2 = json.loads(json.dumps(man)); f2[field] = value; self.assertFalse(UM.check(f2)[0], field)

    def test_missing_or_malformed_hashes_are_rejected(self):
        R, man = self.ready_run()
        for key in ('native_mesh_json', 'atlas_png', 'handoff_json'):
            f = json.loads(json.dumps(man)); del f['artifacts'][key]['sha256']; ok, problems = UM.check(f); self.assertFalse(ok); self.assertTrue(any('sha256 missing' in p and key in p for p in problems), problems)
        f = json.loads(json.dumps(man)); f['files']['status_json']['sha256'] = 'abc'; ok, problems = UM.check(f); self.assertFalse(ok); self.assertTrue(any('status_json' in p for p in problems))
        f = json.loads(json.dumps(man)); f['stages'][0]['receipt'].pop('sha256'); ok, problems = UM.check(f); self.assertFalse(ok); self.assertTrue(any('receipt' in p and 'sha256 missing' in p for p in problems))
        f = json.loads(json.dumps(man)); f['artifacts']['native_mesh_json']['sha256'] = '0' * 64; ok, problems = UM.check(f); self.assertFalse(ok); self.assertTrue(any('sha256 mismatch' in p for p in problems))
        self.assertTrue(man['files']['events_jsonl']['live'] and 'sha256' not in man['files']['events_jsonl'])   # live files stay unhashed and accepted

    def test_packing_flag_contradictions_are_rejected(self):
        R, man = self.ready_run(); self.assertTrue(UM.check(man)[0])
        for mutate, needle in ((lambda f: f['packing'].__setitem__('ready', False), 'packing.ready False contradicts derived readiness True'), (lambda f: f.__setitem__('export_ready', False), 'export_ready False contradicts derived readiness True'),
                               (lambda f: f['packing'].__setitem__('handoff_status', 'blocked'), 'packing.handoff_status'), (lambda f: f['packing'].__setitem__('blocking_problems', ['x']), 'packing.blocking_problems contradict'),
                               (lambda f: f['preview'].__setitem__('diagnostic_only', True), 'preview.diagnostic_only True contradicts'), (lambda f: f['packing'].__setitem__('ready', 'yes'), 'packing.ready must be bool'),
                               (lambda f: f.__setitem__('job_status', 'failed'), "job_status 'failed' contradicts status.json 'completed'")):
            f = json.loads(json.dumps(man)); mutate(f); ok, problems = UM.check(f); self.assertFalse(ok, needle); self.assertTrue(any(needle in p for p in problems), (needle, problems))
        R2, blocked = self.blocked_run('blocked2'); f = json.loads(json.dumps(blocked)); f['packing']['ready'] = True; ok, problems = UM.check(f); self.assertFalse(ok); self.assertTrue(any('packing.ready True contradicts derived readiness False' in p for p in problems))

    def test_artifact_substitution_is_rejected(self):
        R, man = self.ready_run(); other = R.out / 'stages' / 'artifacts' / 'other_mesh.json'
        ex = synthetic_exchange(); X.head(ex)['positions'][0][0] += 0.01; X.save(ex, other)
        f = json.loads(json.dumps(man)); f['artifacts']['native_mesh_json'] = {'path': str(other), 'relative': 'stages/artifacts/other_mesh.json', 'exists': True, 'kind': 'exchange', 'bytes': other.stat().st_size, 'sha256': J.sha256_file(other)}   # real file, correct hash, wrong artifact
        ok, problems = UM.check(f); self.assertFalse(ok); self.assertTrue(any('native_mesh_json path differs from handoff' in p for p in problems), problems); self.assertTrue(any('native_mesh_json sha256 differs from handoff' in p for p in problems))
        hand_path = Path(man['artifacts']['handoff_json']['path']); hand = json.loads(hand_path.read_text(encoding='utf-8')); hand['artifacts']['native_mesh_json'] = str(other); hand_path.write_text(json.dumps(hand), encoding='utf-8')
        f = json.loads(json.dumps(man)); f['artifacts']['handoff_json']['sha256'] = J.sha256_file(hand_path)                     # handoff re-pointed (and re-hashed): its declared sha no longer matches the file it names
        ok, problems = UM.check(f); self.assertFalse(ok); self.assertTrue(any('on disk differs from handoff native_mesh_sha256' in p for p in problems), problems)

    def test_malformed_handoff_status_and_missing_fields(self):
        R, man = self.ready_run()
        hand_path = Path(man['artifacts']['handoff_json']['path']); hand_path.write_text('{not json', encoding='utf-8')
        f = json.loads(json.dumps(man)); f['artifacts']['handoff_json']['sha256'] = J.sha256_file(hand_path)
        ok, problems = UM.check(f); self.assertFalse(ok); self.assertTrue(any('handoff.json: malformed JSON' in p for p in problems), problems); self.assertTrue(any('export_ready True contradicts derived readiness False' in p for p in problems))
        hand_path.write_text('[1,2]', encoding='utf-8'); f['artifacts']['handoff_json']['sha256'] = J.sha256_file(hand_path); ok, problems = UM.check(f); self.assertTrue(any('JSON root must be an object' in p for p in problems))
        R2, man2 = self.ready_run('ready2'); sp = Path(man2['files']['status_json']['path']); sp.write_text('garbage', encoding='utf-8')
        f = json.loads(json.dumps(man2)); f['files']['status_json']['sha256'] = J.sha256_file(sp); ok, problems = UM.check(f); self.assertFalse(ok); self.assertTrue(any('status.json: malformed JSON' in p for p in problems), problems)
        R3, man3 = self.ready_run('ready3')
        for missing in ('packing', 'artifacts', 'files', 'export_ready', 'stages'):
            f = json.loads(json.dumps(man3)); del f[missing]; ok, problems = UM.check(f); self.assertFalse(ok); self.assertIn(f'missing field {missing}', problems)
        f = json.loads(json.dumps(man3)); f['export_ready'] = 'yes'; ok, problems = UM.check(f); self.assertTrue(any('export_ready must be bool' in p for p in problems))
        f = json.loads(json.dumps(man3)); f['files']['status_json'] = None; ok, problems = UM.check(f); self.assertTrue(any('files.status_json must be a hashed file reference' in p for p in problems))
        self.assertEqual(UM.check([])[0], False); self.assertEqual(UM.check({'schema': 'x'})[0], False)
        f = json.loads(json.dumps(man3)); f['stages'][0]['status'] = 'failed'; ok, problems = UM.check(f); self.assertTrue(any('contradicts its receipt' in p for p in problems))
        self.assertTrue(UM.check(man3)[0])


class TestPathsAndSubprocess(Base):
    def test_paths_with_spaces_and_non_ascii(self):
        root = self.tmp / 'runs ünïcode ä 漢字'; root.mkdir(); name = 'job ø №1 (test)'
        R = J.Runner(self.job(name, root=root, blender=FAKE), log_to_stdout=False)
        env = dict(os.environ, FAKE_BLENDER_MODE='version')
        tools = J.tool_identity(R.job); self.assertIn('4.5.5', tools['blender']['version_line'])

        def stage(d, log):
            R.blender(REPO / 'tests/conversion/fake_blender.py', ['in put ä', d / 'out dir é'], d)
            return {'outputs': {'done': d / 'out dir é' / 'done.txt'}, 'summary': {'ok': True}}
        os.environ['FAKE_BLENDER_MODE'] = 'version'
        try: r = R.run_stage('extract', stage, inputs={'job': R.out / 'job.json'})
        finally: os.environ.pop('FAKE_BLENDER_MODE', None)
        self.assertEqual(r['status'], 'ok'); self.assertEqual(r['outputs']['done']['sha256'], J.sha256_file(R.out / 'stages' / '01_extract' / 'out dir é' / 'done.txt'))
        self.assertIn("in put ä", (R.out / 'stages' / '01_extract' / 'blender_stdout.txt').read_text(encoding='utf-8'))
        hand, ex = self.fake_handoff(R); st = R.finish('completed', handoff=hand); self.assertTrue(st['ready_for_packing'])
        man = UM.load(R.out / 'ui_manifest.json'); ok, problems = UM.check(man); self.assertTrue(ok, problems); self.assertEqual(man['run_id'], name); self.assertIn('ünïcode', man['output_dir'])
        self.assertEqual(json.loads((R.out / 'status.json').read_text(encoding='utf-8'))['run_id'], name); self.assertEqual(events_of(R.out)[0]['run_id'], name)

    def test_missing_blender_fails_with_receipt(self):
        j = self.job('nob', blender=self.tmp / 'nowhere' / 'blender.exe'); R = J.Runner(j, log_to_stdout=False)
        with self.assertRaises(J.JobError): R.run_stage('validate', lambda d, log: J.tool_identity(j))
        st = R.finish('failed', note='blender missing'); self.assertEqual(st['status'], 'failed')
        rec = json.loads((R.out / 'stages' / '01_validate' / 'receipt.json').read_text(encoding='utf-8')); self.assertEqual(rec['status'], 'failed'); self.assertIn('blender executable missing', rec['error'])
        j2 = self.job('nob2', blender=FAKE); j2['tools']['blender']['expected_version'] = '9.9.9'
        with self.assertRaises(J.JobError) as cm: J.tool_identity(j2)
        self.assertIn('9.9.9', str(cm.exception))

    def test_subprocess_failure_is_recorded(self):
        R = J.Runner(self.job('subfail', blender=FAKE), log_to_stdout=False); os.environ['FAKE_BLENDER_MODE'] = 'fail'
        try:
            with self.assertRaises(J.JobError): R.run_stage('bake', lambda d, log: R.blender(FAKE, [d / 'x'], d))
        finally: os.environ.pop('FAKE_BLENDER_MODE', None)
        rec = json.loads((R.out / 'stages' / '01_bake' / 'receipt.json').read_text(encoding='utf-8')); self.assertEqual(rec['status'], 'failed'); self.assertIn('blender exited 1', rec['error']); self.assertIn('simulated failure', rec['error'])
        self.assertIn('simulated failure', (R.out / 'stages' / '01_bake' / 'blender_stderr.txt').read_text(encoding='utf-8'))
        st = R.finish('failed', note=rec['error']); self.assertFalse(st['ready_for_packing'])
        ev = events_of(R.out); sub = [e for e in ev if e['event'] == 'stage_progress' and e.get('subprocess')]; self.assertEqual(sub[-1]['subprocess']['returncode'], 1); self.assertEqual(ev[-1]['event'], 'job_failed')

    def test_cancellation_during_blender_stage_kills_process_and_keeps_evidence(self):
        R = J.Runner(self.job('cancel', blender=FAKE), log_to_stdout=False)
        R.run_stage('validate', lambda d, log: ((d / 'v.txt').write_text('ok'), {'outputs': {'v': d / 'v.txt'}, 'summary': {}})[-1])
        os.environ['FAKE_BLENDER_MODE'] = 'sleep'; os.environ['FAKE_BLENDER_SLEEP'] = '60'
        threading.Timer(1.5, lambda: (R.out / 'CANCEL').write_text('cancel from app')).start(); t = time.time()
        try:
            with self.assertRaises(J.Cancelled): R.run_stage('bake', lambda d, log: R.blender(FAKE, [d / 'x'], d), required=True)
        finally: os.environ.pop('FAKE_BLENDER_MODE', None); os.environ.pop('FAKE_BLENDER_SLEEP', None)
        self.assertLess(time.time() - t, 20, 'cancellation must not wait for the 60 s subprocess')
        st = R.finish('cancelled', note='CANCEL'); self.assertEqual(st['status'], 'cancelled'); self.assertFalse(st['ready_for_packing'])
        ev = events_of(R.out); started = [e for e in ev if e['event'] == 'stage_progress' and (e.get('subprocess') or {}).get('pid') and 'started' in e['message']][0]
        pid = started['subprocess']['pid']; time.sleep(0.5); self.assertFalse(pid_alive(pid), f'blender stand-in pid {pid} still alive after cancellation')
        self.assertTrue(any(e['event'] == 'stage_progress' and (e.get('subprocess') or {}).get('killed') for e in ev)); self.assertEqual(ev[-1]['event'], 'job_cancelled')
        rec = json.loads((R.out / 'stages' / '02_bake' / 'receipt.json').read_text(encoding='utf-8')); self.assertEqual(rec['status'], 'cancelled'); self.assertIn('killed', rec['error'])
        self.assertEqual(json.loads((R.out / 'stages' / '01_validate' / 'receipt.json').read_text(encoding='utf-8'))['status'], 'ok')   # completed evidence preserved
        man = UM.load(R.out / 'ui_manifest.json'); self.assertEqual([s['status'] for s in man['stages']], ['ok', 'cancelled']); self.assertFalse(man['export_ready'])


class TestEntryPointCLI(Base):
    def run_cli(self, *a, timeout=120):
        r = subprocess.run([sys.executable, str(RUN_JOB), *[str(x) for x in a]], capture_output=True, text=True, encoding='utf-8', timeout=timeout, cwd=str(REPO)); return r.returncode, r.stdout, r.stderr

    def test_malformed_jobs_exit_4_with_job_invalid_event(self):
        bad = self.tmp / 'bad.json'; bad.write_text('{not json', encoding='utf-8')
        rc, out, err = self.run_cli(bad, '--events'); self.assertEqual(rc, 4); ev = json.loads(out.strip().splitlines()[-1]); self.assertEqual(ev['event'], 'job_invalid'); self.assertTrue(any('not valid JSON' in p for p in ev['problems']))
        rc, out, _ = self.run_cli(bad); self.assertEqual(rc, 4); self.assertIn('invalid job', out)                                   # classic output unchanged
        lst = self.tmp / 'list.json'; lst.write_text('[1,2]', encoding='utf-8'); rc, out, _ = self.run_cli(lst, '--events'); self.assertEqual(rc, 4); self.assertIn('JSON object', out)
        rc, out, _ = self.run_cli(self.tmp / 'missing.json', '--events'); self.assertEqual(rc, 4); self.assertIn('not found', out)
        j = self.job(); del j['policies']; j['strategy'] = 'source_topology'; j['tools'] = 'x'; p = self.tmp / 'j.json'; p.write_text(json.dumps(j), encoding='utf-8')
        rc, out, _ = self.run_cli(p, '--events'); self.assertEqual(rc, 4); ev = json.loads(out.strip().splitlines()[-1]); self.assertEqual(ev['job_id'], 'synthetic')
        self.assertTrue(any('missing key policies' in q for q in ev['problems'])); self.assertTrue(any('unsupported' in q for q in ev['problems'])); self.assertTrue(any('tools must be an object' in q for q in ev['problems']))
        rc, out, _ = self.run_cli(p, '--dry-run'); self.assertEqual(rc, 4); self.assertFalse(json.loads(out)['valid'])
        self.assertFalse(list(self.runs.iterdir()), 'an invalid job must not create an output directory')

    def test_valid_job_with_missing_blender_fails_at_validate_with_events(self):
        j = self.job('cli-nob', blender=self.tmp / 'no' / 'blender.exe'); j['destination'] = {'exchange': {'path': str(self.tmp / 'no_exchange.json')}, 'label': 'x'}
        p = self.tmp / 'j.json'; p.write_text(json.dumps(j), encoding='utf-8'); rc, out, err = self.run_cli(p, '--events')
        self.assertEqual(rc, 2, err); ev = [json.loads(l) for l in out.strip().splitlines()]; self.assertEqual(ev[0]['event'], 'job_started'); self.assertEqual(ev[-1]['event'], 'job_failed'); self.assertFalse(ev[-1]['export_ready'])
        self.assertTrue(all(l.startswith('{') for l in out.strip().splitlines()), 'events mode prints only JSON lines')
        man = UM.load(Path(ev[-1]['manifest'])); self.assertEqual(man['job_status'], 'failed'); self.assertEqual(man['stages'][0]['name'], 'validate'); self.assertEqual(man['stages'][0]['status'], 'failed'); ok, problems = UM.check(man); self.assertTrue(ok, problems)
        rc, out, _ = self.run_cli(p); self.assertEqual(rc, 4); self.assertIn('already exists', out)                                    # collision refused, classic mode


class TestLocalDemoRuns(unittest.TestCase):
    """Licensed-fixture demonstrations (skipped visibly when absent): worker outputs carry preview + manifest and the
    converted mesh/atlas hashes equal the accepted worker outputs."""
    ACCEPTED = {'turgeon': ('7fb34573c38541dd63ad8b6bbe7bb124a4b7af83a1c1ad966a2c8ac7b01a2299', 'df56d53251f41ee280a1bee3f416f780a203c73f66214c2ed6b2300aafdef8fe'),
                'fedorov': ('a6a79260', '59d570e1')}

    def test_demo_runs_manifest_and_hashes(self):
        base = REPO / 'local/runs/claude'; found = 0
        for name, (mesh, atlas) in self.ACCEPTED.items():
            runs = sorted(base.glob(f'app-2026-09-12-job-{name}-[0-9][0-9]'))   # completed demo runs only (cancel demos excluded)
            if not runs: continue
            found += 1
            for run in runs:
                man = UM.load(run / 'ui_manifest.json'); ok, problems = UM.check(man); self.assertTrue(ok, (run, problems))
                self.assertTrue(man['export_ready']); self.assertEqual(man['preview']['status'], 'ok'); self.assertTrue(man['artifacts']['native_mesh_json']['sha256'].startswith(mesh)); self.assertTrue(man['artifacts']['atlas_png']['sha256'].startswith(atlas))
                ok, problems, _ = PV.verify_glb(run / 'preview.glb', X.load(man['artifacts']['native_mesh_json']['path']), man['artifacts']['atlas_png']['path']); self.assertTrue(ok, problems)
        if not found: self.skipTest('local demo runs absent (app-2026-09-12-job-*): licensed fixtures; coverage unknown')


if __name__ == '__main__':
    unittest.main()
