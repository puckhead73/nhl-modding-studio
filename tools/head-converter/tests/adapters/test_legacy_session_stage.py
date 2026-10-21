"""Tests for the staging/session driver (`experiments/claude/legacy/session_stage.py`) on SYNTHETIC repository-local
fixtures only: a fake install (exe, dll, ReShade.ini, game/default.xex + tiny .big + _compiled/rendering/player with a
modded pair, its .bak originals and siblings), a fake live user-data tree, a synthetic strict-valid dossier (from the
dossier test helpers) and synthetic pins. Nothing here touches F:, the live user data or E:\\Games; no game runs.
Covers: authorisation parsing/refusal, pin drift, config drift, root refusals, a complete stage with hash verification,
occupied destination, interrupted copy leaving a partial marker + refusal to resume, cache attempts advancing without
deletion, attempt placement with hash/sibling checks, live-change refusal, post-attempt log checks (route evidence,
missing asset-open, FAILED line, shadow subtree, PG indistinguishable), rollback, and that the live sources stay untouched.
Run: python -m unittest tests.adapters.test_legacy_session_stage
"""
import hashlib, json, sys, tempfile, unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(ROOT))
from experiments.claude.legacy import session_stage as SS, session_dryrun as SD
from tests.adapters.test_legacy_dossier import make_dossier, bind_evidence_files

INI = "[ADDON]\r\nAddonPath=.\\\r\n\r\n[GENERAL]\r\nIntermediateCachePath=C:\\Users\\other\\Temp\\ReShade\r\n\r\n[SCREENSHOT]\r\nSavePath=.\\\r\n"
sha = lambda b: hashlib.sha256(b).hexdigest()


def hash_all(base):
    return {str(p.relative_to(base)): (sha(p.read_bytes()) if p.is_file() else 'dir') for p in sorted(Path(base).rglob('*'))}


class StageFixture(unittest.TestCase):
    def setUp(self):
        self.td = tempfile.TemporaryDirectory(); d = Path(self.td.name); self._runs = SD.RUNS_DIR; SD.RUNS_DIR = d / 'runs'; SD.RUNS_DIR.mkdir()
        self.install = d / 'install'; player = self.install / 'game/_compiled/rendering/player'; player.mkdir(parents=True); (self.install / 'logs').mkdir()
        (self.install / 'nhllegacy.exe').write_bytes(b'EXE'); (self.install / 'rexruntime.dll').write_bytes(b'DLL'); (self.install / 'ReShade.ini').write_bytes(INI.encode()); (self.install / 'ffmpeg.exe').write_bytes(b'FF')
        (self.install / 'game/default.xex').write_bytes(b'XEX'); (self.install / 'game/cacherender.big').write_bytes(b'BIG' * 100); (self.install / 'logs/nhllegacy_001.log').write_bytes(b'old log')
        # dossier: synthetic strict-valid manifest whose donor files are the .bak originals of the fake install
        self.dossier = d / 'dossier'; self.dossier.mkdir(); m = bind_evidence_files(self.dossier, make_dossier(self.dossier)); (self.dossier / 'MANIFEST.json').write_text(json.dumps(m), encoding='utf-8'); self.m = m
        donor_p = (self.dossier / m['artifacts']['donor_player_67']['file']).read_bytes(); donor_t = (self.dossier / m['artifacts']['donor_texlib_67']['file']).read_bytes()
        (player / 'player_67.rx2.bak').write_bytes(donor_p); (player / 'texlib_67.rx2.bak').write_bytes(donor_t)
        (player / 'player_67.rx2').write_bytes(b'MODDED-P' * 9); (player / 'texlib_67.rx2').write_bytes(b'MODDED-T' * 9)
        for i in range(3): (player / f'player_{i}.rx2').write_bytes(b'sib%d' % i)
        self.live = d / 'live'; (self.live / 'cache/guest').mkdir(parents=True); (self.live / 'cache/guest/809284.ver').write_bytes(b'v')
        self.root = d / 'observation'; self.prot = d / 'vanilla'; self.prot.mkdir()
        self.plan = {**SD.DEFAULT_PLAN, 'install': str(self.install), 'live_user_data': str(self.live), 'protected_trees': [str(self.prot)], 'root': str(self.root), 'dossier': str(self.dossier),
                     'config_edits': [{'file': 'ReShade.ini', 'section': 'GENERAL', 'key': 'IntermediateCachePath', 'expected_current': 'C:\\Users\\other\\Temp\\ReShade', 'new': '<root>\\external\\reshade-cache', 'why': 't'}, {'file': 'ReShade.ini', 'section': 'SCREENSHOT', 'key': 'SavePath', 'expected_current': '.\\', 'new': '<root>\\session\\captures\\reshade', 'why': 't'}]}
        edits = SD.plan_config_edits(self.plan, self.install, self.root); pred = edits[-1]['combined_predictions']['ReShade.ini']['predicted_sha256_after_all_edits']
        self.pins = {'root': str(self.root), 'nhllegacy.exe': sha(b'EXE'), 'rexruntime.dll': sha(b'DLL'), 'modded': {'player_67.rx2': sha(b'MODDED-P' * 9), 'texlib_67.rx2': sha(b'MODDED-T' * 9)}, 'bak': {'player_67.rx2.bak': sha(donor_p), 'texlib_67.rx2.bak': sha(donor_t)}, 'ReShade.ini': {'live_sha256': sha(INI.encode()), 'predicted_after_all_edits': pred}}
        self.auth_text = f'Use {self.root}, launch direct, test P0 -> PG -> P1 -> P2 -> P3, and capture with the copied ReShade facility. Proceed with the disposable copy and observation session; leave F: and live user data unchanged.'
        self.auth = SS.parse_authorization(self.auth_text, self.plan); self.run = SD.RUNS_DIR / 'r1'; self.run.mkdir()
        self.before_install = hash_all(self.install); self.before_live = hash_all(self.live)
    def tearDown(self): SD.RUNS_DIR = self._runs; self.td.cleanup()
    def assert_sources_untouched(self): self.assertEqual(hash_all(self.install), self.before_install); self.assertEqual(hash_all(self.live), self.before_live)


class AuthTests(StageFixture):
    def test_parse_expected_wording(self):
        a = self.auth; self.assertEqual(a['launch_method'], 'direct'); self.assertEqual(a['pairings'], SD.PAIRINGS); self.assertIn('ReShade', a['capture']); self.assertEqual(a['sha256'], sha(self.auth_text.encode()))
        b = SS.parse_authorization(self.auth_text.replace('launch direct', 'launch feed-layer').replace('P1 -> P2 -> P3', ''), self.plan); self.assertEqual(b['launch_method'], 'feed-layer'); self.assertEqual(b['pairings'], ['P0', 'PG'])
        c = SS.parse_authorization(self.auth_text.replace('capture with the copied ReShade facility', 'capture with OBS Studio'), self.plan); self.assertEqual(c['capture'], 'OBS Studio')
    def test_refusals(self):
        for bad in ['', 'please go ahead', self.auth_text.replace(str(self.root), 'D:\\elsewhere'), self.auth_text.replace('launch direct, ', ''), self.auth_text.replace('launch direct', 'launch [direct / feed-layer]'), self.auth_text.replace('test P0 -> PG', 'launch feed-layer, test P0 -> PG'), self.auth_text.replace('P0 -> PG -> P1 -> P2 -> P3', 'all pairings'), self.auth_text.replace('capture with the copied ReShade facility', 'take pictures')]:
            with self.assertRaises(SS.StageError, msg=bad) as cm: SS.parse_authorization(bad, self.plan)
            self.assertEqual(cm.exception.code, 'not_authorized')
    def test_no_auth_file_refuses_every_writing_mode(self):
        for mode in (['stage'], ['prepare-attempt', 'P0'], ['post-attempt', 'P0'], ['rollback']):
            with self.assertRaises(SS.StageError) as cm: SS.main(['r1', *mode, '--plan', str(self._write_plan())])
            self.assertEqual(cm.exception.code, 'not_authorized')
        self.assertFalse(self.root.exists()); self.assert_sources_untouched()
    def _write_plan(self):
        p = Path(self.td.name) / 'plan.json'; p.write_text(json.dumps(self.plan)); return p


class StageTests(StageFixture):
    def test_full_stage_verified(self):
        r = SS.stage(self.run, self.plan, self.pins, self.auth)
        self.assertTrue(r['complete']); self.assertTrue((self.root / 'runtime/nhllegacy.exe').is_file()); self.assertFalse((self.root / 'runtime/logs/nhllegacy_001.log').exists())   # logs excluded
        self.assertEqual(r['runtime_copy']['files'], 4); self.assertEqual(r['game_copy']['files'], 9)
        ini = (self.root / 'runtime/ReShade.ini').read_bytes(); self.assertIn(f'IntermediateCachePath={self.root}\\external\\reshade-cache'.encode(), ini); self.assertEqual(sha(ini), self.pins['ReShade.ini']['predicted_after_all_edits'])
        self.assertEqual(r['config_live_sha256'], self.pins['ReShade.ini']['live_sha256'])
        self.assertEqual(sha((self.root / 'session/backup/modded/player_67.rx2').read_bytes()), self.pins['modded']['player_67.rx2']); self.assertEqual(sha((self.root / 'session/backup/donor/texlib_67.rx2').read_bytes()), self.pins['bak']['texlib_67.rx2.bak'])
        self.assertTrue((self.root / 'cache').is_dir()); self.assertFalse(any((self.root / 'cache' / p).exists() for p in SD.PAIRINGS))
        self.assertEqual(set(r['wrappers']), set(SD.PAIRINGS)); self.assertNotIn(b'\r\r\n', (self.root / 'session/attempts/P0-attempt1.launch.proposed.cmd').read_bytes())
        self.assertTrue((self.run / 'stage_receipt.json').is_file()); self.assertTrue((self.run / 'baseline.json').is_file()); self.assertEqual(json.loads((self.run / 'copy_hashes.json').read_text())['runtime']['nhllegacy.exe']['sha256'], sha(b'EXE'))
        self.assert_sources_untouched(); self.assertFalse(r['runtime_approved'])
    def test_only_selected_method_wrappers(self):
        auth = SS.parse_authorization(self.auth_text.replace('launch direct', 'launch feed-layer'), self.plan); SS.stage(self.run, self.plan, self.pins, auth)
        t = (self.root / 'session/attempts/P0-attempt1.launch.proposed.cmd').read_text(); self.assertIn('VK_INSTANCE_LAYERS=VK_LAYER_feed_vk', t); self.assertEqual(len(list((self.root / 'session/attempts').iterdir())), 5)
    def test_pin_mismatch_refused_before_any_write(self):
        (self.install / 'nhllegacy.exe').write_bytes(b'EXE2'); self.before_install = hash_all(self.install)
        with self.assertRaises(SS.StageError) as cm: SS.stage(self.run, self.plan, self.pins, self.auth)
        self.assertEqual(cm.exception.code, 'pin_mismatch'); self.assertFalse(self.root.exists()); self.assert_sources_untouched()
    def test_stale_reshade_pin_refused(self):
        pins = json.loads(json.dumps(self.pins)); pins['ReShade.ini']['live_sha256'] = 'f' * 64
        with self.assertRaises(SS.StageError) as cm: SS.stage(self.run, self.plan, pins, self.auth)
        self.assertEqual(cm.exception.code, 'pin_mismatch'); self.assertFalse(self.root.exists())
    def test_config_drift_refused(self):
        pins = json.loads(json.dumps(self.pins)); pins['ReShade.ini']['predicted_after_all_edits'] = 'e' * 64
        with self.assertRaises(SS.StageError) as cm: SS.stage(self.run, self.plan, pins, self.auth)
        self.assertEqual(cm.exception.code, 'config_drift'); self.assertFalse(self.root.exists())
    def test_root_mismatch_and_overlap_refused(self):
        with self.assertRaises(SS.StageError) as cm: SS.stage(self.run, {**self.plan, 'root': str(Path(self.td.name) / 'other')}, self.pins, self.auth)
        self.assertEqual(cm.exception.code, 'not_authorized')
        pins = {**self.pins, 'root': str(self.install / 'sub')}; auth = SS.parse_authorization(self.auth_text.replace(str(self.root), str(self.install / 'sub')), {**self.plan, 'root': str(self.install / 'sub')})
        with self.assertRaises(SD.PlanError) as cm: SS.stage(self.run, {**self.plan, 'root': str(self.install / 'sub')}, pins, auth)
        self.assertEqual(cm.exception.code, 'root_overlap'); self.assert_sources_untouched()
    def test_occupied_root_refused(self):
        self.root.mkdir(); (self.root / 'unrelated.txt').write_bytes(b'x')
        with self.assertRaises(SD.PlanError) as cm: SS.stage(self.run, self.plan, self.pins, self.auth)
        self.assertEqual(cm.exception.code, 'root_not_empty'); self.assertEqual((self.root / 'unrelated.txt').read_bytes(), b'x')
    def test_interrupted_copy_leaves_marker_and_refuses_resume(self):
        def hook(i, rel):
            if i == 2: raise OSError('simulated interruption')
        with self.assertRaises(OSError): SS.stage(self.run, self.plan, self.pins, self.auth, hook=hook)
        prog = json.loads((self.run / SS.MARKER).read_text()); self.assertEqual(prog['copied'], 2); self.assertFalse(prog.get('complete')); self.assertTrue((self.root / SS.MARKER).is_file())
        steps = [s['step'] for s in prog['steps']]; self.assertIn('root_created', steps); self.assertNotIn('runtime_copied', steps)
        with self.assertRaises(SS.StageError) as cm: SS.stage(self.run, self.plan, self.pins, self.auth)
        self.assertEqual(cm.exception.code, 'stage_partial_exists'); self.assertTrue((self.root / SS.MARKER).is_file())    # nothing auto-cleaned
        with self.assertRaises(SS.StageError) as cm: SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'P0')
        self.assertEqual(cm.exception.code, 'not_staged'); self.assert_sources_untouched()


class AttemptTests(StageFixture):
    def setUp(self):
        super().setUp(); SS.stage(self.run, self.plan, self.pins, self.auth)
    def expected(self, pid):
        p = {x['id']: x for x in self.m['pairings']}[pid]; return p['geometry_sha256'], p['texlib_sha256']
    def test_prepare_places_and_advances_attempts(self):
        r1 = SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'P0'); g, t = self.expected('P0')
        self.assertEqual(r1['attempt'], 1); self.assertEqual(r1['placed']['player_67.rx2']['sha256'], g); self.assertEqual(r1['placed']['player_67.rx2']['previous_sha256'], self.pins['modded']['player_67.rx2']); self.assertFalse(r1['launched'])
        self.assertTrue(Path(r1['wrapper']).is_file()); self.assertIn(f'--cache_root "{self.root}\\cache\\P0"', Path(r1['wrapper']).read_text())
        (self.root / 'cache/P0/guest').mkdir(parents=True); (self.root / 'cache/P0/guest/809284.ver').write_bytes(b'used')   # simulate a launch having used the root
        r2 = SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'P0'); self.assertEqual(r2['attempt'], 2); self.assertTrue(r2['cache_root'].endswith('P0-attempt2')); self.assertTrue((self.root / 'cache/P0/guest/809284.ver').is_file())
        self.assertIn('P0-attempt2', Path(r2['wrapper']).read_text()); self.assert_sources_untouched()
    def test_prepare_refuses_unauthorised_pairing_and_live_change(self):
        auth = SS.parse_authorization(self.auth_text.replace('P0 -> PG -> P1 -> P2 -> P3', 'P0 -> PG'), self.plan)
        with self.assertRaises(SS.StageError) as cm: SS.prepare_attempt(self.run, self.plan, self.pins, auth, 'P1')
        self.assertEqual(cm.exception.code, 'not_authorized')
        (self.live / 'cache/guest/809284.ver').write_bytes(b'changed')
        with self.assertRaises(SS.StageError) as cm: SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'P0')
        self.assertEqual(cm.exception.code, 'live_changed')
    def test_prepare_refuses_sibling_change_and_loose_tex(self):
        (self.root / 'runtime/game/_compiled/rendering/player/player_1.rx2').write_bytes(b'tampered')
        with self.assertRaises(SS.StageError) as cm: SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'P0')
        self.assertEqual(cm.exception.code, 'copy_verify_failed')
        (self.root / 'runtime/game/_compiled/rendering/player/player_1.rx2').write_bytes(b'sib1'); (self.root / 'runtime/game/_loose_tex').mkdir()
        with self.assertRaises(SS.StageError) as cm: SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'PG')
        self.assertEqual(cm.exception.code, 'destination_occupied')
    def log(self, pid, cache, **kw):
        rt = self.root / 'runtime'; g, t = self.expected(pid)
        lines = [f'[t] [info] [core] [t1]   Game directory: {rt}\\game', f'[t] [info] [core] [t1]   User data:      {self.root}\\user', f'[t] [info] [core] [t1]   Cache root:     {cache}',
                 f'[t] [info] [core] [t1] [loose-overlay] mounted {rt}\\game\\_compiled read-only beneath cache: {cache}\\guest']
        pr = {x['id']: x for x in self.m['pairings']}[pid]; gb = self.m['artifacts'][pr['geometry']]['bytes']; tb = self.m['artifacts'][pr['texlib']]['bytes']
        if kw.get('asset', True): lines += [f'[t] [info] [core] [t2] [asset-open] \\rendering\\player\\player_67.rx2 ({gb} bytes)', f'[t] [info] [core] [t2] [asset-open] \\rendering\\player\\texlib_67.rx2 ({tb} bytes)']
        if kw.get('probe', True): lines.append('[t] [info] [core] [t2] [loose-probe] loose   \\rendering\\player')
        if kw.get('failed'): lines.append("[t] [warning] [krnl] [t3] [NtCreateFile] FAILED: path='cache:\\rendering\\player\\texlib_67.rx2' -> 0xc000000f")
        return '\n'.join(lines)
    def test_post_attempt_route_evidence_and_stops(self):
        r = SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'P0'); cache = Path(r['cache_root']); (cache / 'guest').mkdir(parents=True)
        ok = SS.post_attempt(self.run, self.plan, self.pins, self.auth, 'P0', {'O1': 'pass', 'K1': 'observed'}, log_text=self.log('P0', cache))
        self.assertEqual(ok['stop'], []); self.assertTrue(ok['log']['route_evidence_ok']); self.assertIn('entry-open diagnostic only', ok['log']['asset_open_note']); self.assertEqual(ok['observations']['O1'], 'pass'); self.assertTrue((Path(r['wrapper']).parent / 'result_receipt.json').is_file())
        bad = SS.post_attempt(self.run, self.plan, self.pins, self.auth, 'P0', {}, log_text=self.log('P0', cache, asset=False)); self.assertTrue(any('route evidence' in s for s in bad['stop'])); self.assertEqual(bad['verdict'], 'STOP')
        bad = SS.post_attempt(self.run, self.plan, self.pins, self.auth, 'P0', {}, log_text=self.log('P0', cache, failed=True)); self.assertFalse(bad['log']['no_failed_ok']); self.assertTrue(bad['stop'])
        (cache / 'guest/rendering/player').mkdir(parents=True); bad = SS.post_attempt(self.run, self.plan, self.pins, self.auth, 'P0', {}, log_text=self.log('P0', cache)); self.assertTrue(any('shadow' in s for s in bad['stop']))
    def test_post_attempt_pg_indistinguishable_stops(self):
        SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'P0'); r = SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'PG'); cache = Path(r['cache_root']); (cache / 'guest').mkdir(parents=True)
        res = SS.post_attempt(self.run, self.plan, self.pins, self.auth, 'PG', {'visibly_differs_from_P0': False}, log_text=self.log('PG', cache)); self.assertTrue(any('indistinguishable' in s for s in res['stop']))
        res = SS.post_attempt(self.run, self.plan, self.pins, self.auth, 'PG', {'visibly_differs_from_P0': True, 'O2': 'unknown'}, log_text=self.log('PG', cache)); self.assertEqual(res['stop'], [])
    def test_post_attempt_detects_inplace_change_and_live_change(self):
        r = SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'P1'); cache = Path(r['cache_root']); (cache / 'guest').mkdir(parents=True)
        (self.root / 'runtime/game/_compiled/rendering/player/texlib_67.rx2').write_bytes(b'altered'); (self.live / 'newsave').write_bytes(b's')
        res = SS.post_attempt(self.run, self.plan, self.pins, self.auth, 'P1', {}, log_text=self.log('P1', cache)); self.assertFalse(res['in_place_unchanged']); self.assertFalse(res['live_unchanged']); self.assertEqual(len(res['stop']), 2)
    def test_rollback_restores_modded_pair_only_in_copy(self):
        SS.prepare_attempt(self.run, self.plan, self.pins, self.auth, 'P0'); r = SS.rollback(self.run, self.plan, self.pins, self.auth)
        self.assertEqual(r['restored_modded_pair'], self.pins['modded']); self.assertEqual(sha((self.root / 'runtime/game/_compiled/rendering/player/player_67.rx2').read_bytes()), self.pins['modded']['player_67.rx2'])
        self.assertTrue((self.root / 'session/attempts/P0-attempt1/prepare_receipt.json').is_file()); self.assert_sources_untouched()
        (self.root / 'session/backup/modded/player_67.rx2').write_bytes(b'corrupt')
        with self.assertRaises(SS.StageError) as cm: SS.rollback(self.run, self.plan, self.pins, self.auth)
        self.assertEqual(cm.exception.code, 'pin_mismatch')



class ManifestIdTests(StageFixture):
    def test_pairing_ids_from_manifest_drive_authorisation_and_stage(self):
        # the manifest's ids (not the P0..P3 constant) decide which tokens the instruction may name; a retest-style manifest with R0/R1
        m = json.loads(json.dumps(self.m)); m['pairings'] = [dict(m['pairings'][0], id='R0'), dict(m['pairings'][1], id='R1'), dict(m['pairings'][2], id='R2')]; m['required_controls'] = {'donor_baseline': 'R0', 'geometry_only': 'R1'}   # R2 keeps the dossier validator's base-only texture control
        ids, ctl = SD.pairing_ids(m); self.assertEqual(ids, ['R0', 'R1', 'R2'])
        a = SS.parse_authorization(self.auth_text.replace('P0 -> PG -> P1 -> P2 -> P3', 'R0 -> R1'), self.plan, ids); self.assertEqual(a['pairings'], ['R0', 'R1'])
        with self.assertRaises(SS.StageError): SS.parse_authorization(self.auth_text, self.plan, ids)                 # P0..P3 are not ids of this manifest
        (self.dossier / 'MANIFEST.json').write_text(json.dumps(m), encoding='utf-8'); af = self.run / 'auth.txt'; af.write_text(self.auth_text.replace('P0 -> PG -> P1 -> P2 -> P3', 'R0 -> R1'), encoding='utf-8')
        auth = SS.load_auth(['--auth', str(af)], self.plan, self.run); self.assertEqual(auth['pairings'], ['R0', 'R1'])
        r = SS.stage(self.run, self.plan, self.pins, auth); self.assertTrue(r['complete']); self.assertEqual(sorted(r['wrappers']), ['R0', 'R1'])   # wrappers only for the authorised subset
        p = SS.prepare_attempt(self.run, self.plan, self.pins, auth, 'R1'); self.assertTrue(p['cache_root'].endswith('R1')); self.assertEqual(p['attempt'], 1)
        with self.assertRaises(SS.StageError) as cm: SS.prepare_attempt(self.run, self.plan, self.pins, auth, 'PG')
        self.assertEqual(cm.exception.code, 'not_authorized'); self.assert_sources_untouched()
    def test_manifest_without_semantic_controls_or_with_stale_evidence_refused(self):
        m = json.loads(json.dumps(self.m)); m['pairings'] = [p for p in m['pairings'] if p['id'] != 'P0']; (self.dossier / 'MANIFEST.json').write_text(json.dumps(m), encoding='utf-8')
        af = self.run / 'auth.txt'; af.write_text(self.auth_text, encoding='utf-8')
        with self.assertRaises(SD.PlanError) as cm: SS.load_auth(['--auth', str(af)], self.plan, self.run)
        self.assertEqual(cm.exception.code, 'control_missing'); self.assertFalse(self.root.exists())
        (self.dossier / 'MANIFEST.json').write_text(json.dumps(self.m), encoding='utf-8'); auth = SS.load_auth(['--auth', str(af)], self.plan, self.run)
        ev = self.m['evidence_pins']['prediction']['path']; Path(ev).write_bytes(Path(ev).read_bytes() + b'x')          # stale evidence: pinned file changed on disk
        with self.assertRaises(SS.StageError) as cm: SS.stage(self.run, self.plan, self.pins, auth)
        self.assertEqual(cm.exception.code, 'dossier_invalid'); self.assertIn('evidence', str(cm.exception)); self.assertFalse(self.root.exists()); self.assert_sources_untouched()


if __name__ == '__main__': unittest.main()
