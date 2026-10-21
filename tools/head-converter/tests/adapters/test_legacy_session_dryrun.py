"""Tests for the dry-run session planner (`experiments/claude/legacy/session_dryrun.py`). Everything runs on synthetic
temporary trees: a fake install (exe + game/default.xex + ReShade.ini) and fake protected trees. Asserts the root
refusals (unresolved, ambiguous spelling before resolution, drive root, overlap in both directions with install / live
user data / protected trees incl. an alias, not empty), run-name validation, the cache-attempt lifecycle, the wrapper's
process-local environment policy (simulated offline against synthetic inherited variables - no cmd.exe, no game), CRLF bytes, the config-edit binding refusals (key absent, current value differs), the in-memory edit (original bytes
untouched, predicted hash changes, combined hash for two edits), that the planner never creates the proposed root, and
the wrapper text (cwd and env inside the root, no other roots). Run: python -m unittest tests.adapters.test_legacy_session_dryrun
"""
import hashlib, json, sys, tempfile, unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(ROOT))
from experiments.claude.legacy import session_dryrun as SD, dossier_validate as DV

INI = "[ADDON]\r\nAddonPath=.\\\r\n\r\n[GENERAL]\r\nIntermediateCachePath=C:\\Users\\other\\Temp\\ReShade\r\nPresetPath=.\\ReShadePreset.ini\r\n\r\n[SCREENSHOT]\r\nSavePath=.\\\r\n"


def synth_manifest(ids=('P0', 'PG', 'P1', 'P2', 'P3'), controls=None):
    """Minimal manifest with semantic controls: P0 = donor pair (control), PG = repaired geometry + donor texlib, the rest repaired geometry + full chain."""
    arts = {'donor_player_67': {'file': 'donor/p.rx2', 'bytes': 177024, 'sha256': 'a' * 64}, 'donor_texlib_67': {'file': 'donor/t.rx2', 'bytes': 331776, 'sha256': 'b' * 64}, 'positions_only': {'file': 'geometry/pos.rx2', 'bytes': 177024, 'sha256': 'c' * 64}, 'full_chain': {'file': 'texture/full.rx2', 'bytes': 331776, 'sha256': 'd' * 64}}
    def pair(i):
        g, t, c = ('donor_player_67', 'donor_texlib_67', True) if i == 'P0' else ('positions_only', 'donor_texlib_67', True) if i == 'PG' else ('positions_only', 'full_chain', False)
        return {'id': i, 'geometry': g, 'texlib': t, 'control': c, 'geometry_sha256': arts[g]['sha256'], 'texlib_sha256': arts[t]['sha256']}
    m = {'artifacts': arts, 'pairings': [pair(i) for i in ids]}
    if controls is not None: m['required_controls'] = controls
    return m


class DryRunTests(unittest.TestCase):
    def setUp(self):
        self.td = tempfile.TemporaryDirectory(); d = Path(self.td.name)
        self.install = d / 'install'; (self.install / 'game').mkdir(parents=True); (self.install / 'nhllegacy.exe').write_bytes(b'exe'); (self.install / 'game/default.xex').write_bytes(b'xex'); (self.install / 'ReShade.ini').write_bytes(INI.encode('utf-8'))
        self.live = d / 'live'; self.live.mkdir(); self.prot = d / 'vanilla'; self.prot.mkdir(); self.root = d / 'observation'
        self.plan = {**SD.DEFAULT_PLAN, 'install': str(self.install), 'live_user_data': str(self.live), 'protected_trees': [str(self.prot)], 'root': str(self.root),
                     'config_edits': [{'file': 'ReShade.ini', 'section': 'GENERAL', 'key': 'IntermediateCachePath', 'expected_current': 'C:\\Users\\other\\Temp\\ReShade', 'new': '<root>\\external\\reshade-cache', 'why': 't'}, {'file': 'ReShade.ini', 'section': 'SCREENSHOT', 'key': 'SavePath', 'expected_current': '.\\', 'new': '<root>\\session\\captures', 'why': 't'}]}
    def tearDown(self): self.td.cleanup()
    def refused(self, code, **over):
        plan = {**self.plan, **over}
        with self.assertRaises(SD.PlanError) as cm: SD.check_root(plan)
        self.assertEqual(cm.exception.code, code, str(cm.exception))

    def test_valid_root_accepted_and_not_created(self):
        r = SD.check_root(self.plan); self.assertEqual(r, self.root); self.assertFalse(self.root.exists())
    def test_root_unresolved(self):
        self.refused('root_unresolved', root=''); self.refused('root_unresolved', root='relative/dir'); self.refused('root_unresolved', root=None)
    def test_drive_root_refused(self): self.refused('root_overlap', root=str(Path(self.td.name).anchor))
    def test_root_inside_install_refused(self): self.refused('root_overlap', root=str(self.install / 'observation'))
    def test_root_equal_install_refused(self): self.refused('root_overlap', root=str(self.install))
    def test_root_over_install_refused(self): self.refused('root_overlap', root=str(self.install.parent))   # would contain the install
    def test_root_inside_live_user_data_refused(self): self.refused('root_overlap', root=str(self.live / 'x'))
    def test_root_inside_protected_tree_refused(self): self.refused('root_overlap', root=str(self.prot / 'copy'))
    def test_root_case_insensitive_overlap(self): self.refused('root_overlap', root=str(self.install).upper() + '\\sub')
    def test_root_not_empty_refused(self):
        self.root.mkdir(); (self.root / 'x').write_bytes(b'1'); self.refused('root_not_empty')
    def test_root_empty_dir_accepted(self):
        self.root.mkdir(); SD.check_root(self.plan)

    def test_config_edit_predicted_without_writing(self):
        before = (self.install / 'ReShade.ini').read_bytes(); edits = SD.plan_config_edits(self.plan, self.install, self.root)
        self.assertEqual((self.install / 'ReShade.ini').read_bytes(), before)                       # live file untouched
        e0 = [e for e in edits if e.get('key') == 'IntermediateCachePath'][0]
        self.assertEqual(e0['before'], 'C:\\Users\\other\\Temp\\ReShade'); self.assertEqual(e0['after'], str(self.root) + '\\external\\reshade-cache')
        self.assertEqual(e0['original_sha256_preserved'], hashlib.sha256(before).hexdigest()); self.assertNotEqual(e0['predicted_sha256_after_this_edit_alone'], e0['original_sha256_preserved']); self.assertFalse(e0['live_file_written'])
        comb = edits[-1]['combined_predictions']['ReShade.ini']; self.assertEqual(comb['edits'], 2)
        # the combined text equals both edits applied by hand
        text = INI.replace('C:\\Users\\other\\Temp\\ReShade', str(self.root) + '\\external\\reshade-cache').replace('SavePath=.\\', 'SavePath=' + str(self.root) + '\\session\\captures')
        self.assertEqual(comb['predicted_sha256_after_all_edits'], hashlib.sha256(text.encode()).hexdigest()); self.assertFalse(self.root.exists())
    def test_config_edit_key_absent_refused(self):
        plan = {**self.plan, 'config_edits': [{'file': 'ReShade.ini', 'section': 'GENERAL', 'key': 'Nope', 'expected_current': 'x', 'new': 'y', 'why': 't'}]}
        with self.assertRaises(SD.PlanError) as cm: SD.plan_config_edits(plan, self.install, self.root)
        self.assertEqual(cm.exception.code, 'config_edit_unbound')
    def test_config_edit_value_changed_refused(self):
        plan = {**self.plan, 'config_edits': [{'file': 'ReShade.ini', 'section': 'GENERAL', 'key': 'IntermediateCachePath', 'expected_current': 'D:\\somewhere\\else', 'new': 'y', 'why': 't'}]}
        with self.assertRaises(SD.PlanError) as cm: SD.plan_config_edits(plan, self.install, self.root)
        self.assertEqual(cm.exception.code, 'config_edit_unbound')
    def test_config_edit_wrong_section_refused(self):
        plan = {**self.plan, 'config_edits': [{'file': 'ReShade.ini', 'section': 'ADDON', 'key': 'IntermediateCachePath', 'expected_current': 'C:\\Users\\other\\Temp\\ReShade', 'new': 'y', 'why': 't'}]}
        with self.assertRaises(SD.PlanError) as cm: SD.plan_config_edits(plan, self.install, self.root)
        self.assertEqual(cm.exception.code, 'config_edit_unbound')
    def test_config_file_missing_refused(self):
        plan = {**self.plan, 'config_edits': [{'file': 'missing.ini', 'section': 'A', 'key': 'k', 'expected_current': 'x', 'new': 'y', 'why': 't'}]}
        with self.assertRaises(SD.PlanError) as cm: SD.plan_config_edits(plan, self.install, self.root)
        self.assertEqual(cm.exception.code, 'config_edit_unbound')

    def test_walk_excludes_and_counts(self):
        (self.install / 'logs').mkdir(); (self.install / 'logs/a.log').write_bytes(b'12345'); (self.install / 'game/big.bin').write_bytes(b'x' * 10)
        files, total = SD.walk(self.install, exclude=('game', 'logs')); self.assertEqual({f['rel'] for f in files}, {'nhllegacy.exe', 'ReShade.ini'}); self.assertEqual(total, 3 + len(INI))
        gfiles, gtotal = SD.walk(self.install / 'game'); self.assertEqual(gtotal, 13)

    def test_wrapper_text_points_into_root(self):
        for method in ('direct', 'feed-layer'):
            t = SD.wrapper_text(self.plan, self.root, 'PG', method)
            self.assertIn(f'cd /d "{self.root}\\runtime" ||', t); self.assertIn(f'--cache_root "{self.root}\\cache\\PG"', t); self.assertIn(f'--user_data_root "{self.root}\\user"', t); self.assertIn('set "NHL_LOG_ASSET_OPENS=1"', t)
            self.assertNotIn(str(self.install), t); self.assertNotIn(str(self.live), t)
            self.assertEqual(f'set "VK_LAYER_PATH={self.root}\\runtime\\layer-x64"' in t, method == 'feed-layer')
            self.assertIn('set "VK_LAYER_PATH="', t)          # both modes clear the inherited activation first
            self.assertNotIn('start ""', t)                   # foreground run so the exit code is the child's

    # --- wrapper environment policy (simulated offline; no cmd.exe, no game) ---
    def test_wrapper_env_policy_direct_clears_inherited_controls(self):
        inherited = {'NHL_GAME_DUMP_DIR': 'C:\\dumps', 'NHL_GAME_CRASH_TEST_SECONDS': '5', 'NHL_TRACE_TEXLIB_OPEN': '1', 'NHL_REPLAY_XTR': 'x.xtr', 'VK_LAYER_PATH': 'C:\\other\\layers', 'VK_INSTANCE_LAYERS': 'VK_LAYER_feed_vk', 'NHL_VP6_FFMPEG': 'C:\\stale\\ffmpeg.exe', 'PATH': 'C:\\Windows;C:\\Windows\\System32', 'SystemRoot': 'C:\\Windows', 'TEMP': 'C:\\Temp', 'NVIDIA_SOMETHING': 'keep'}
        child, parent = SD.simulate_env(SD.wrapper_text(self.plan, self.root, 'P0', 'direct'), inherited)
        for k in ('NHL_GAME_DUMP_DIR', 'NHL_GAME_CRASH_TEST_SECONDS', 'NHL_TRACE_TEXLIB_OPEN', 'NHL_REPLAY_XTR', 'VK_LAYER_PATH', 'VK_INSTANCE_LAYERS'): self.assertNotIn(k, child, k)
        self.assertEqual(child['NHL_VP6_FFMPEG'], str(self.root) + '\\runtime\\ffmpeg.exe'); self.assertEqual(child['NHL_LOG_ASSET_OPENS'], '1'); self.assertEqual(child['NHL_LOOSE_PROBE'], '1')
        for k in ('PATH', 'SystemRoot', 'TEMP', 'NVIDIA_SOMETHING'): self.assertEqual(child[k], inherited[k], k)   # essential/unrelated variables intact
        self.assertEqual(parent, inherited)                                                                     # parent never mutated
    def test_wrapper_env_policy_feed_layer_uses_only_copied_paths(self):
        inherited = {'VK_LAYER_PATH': 'C:\\other\\layers', 'VK_ADD_LAYER_PATH': 'C:\\more', 'VK_INSTANCE_LAYERS': 'VK_LAYER_other', 'PATH': 'p'}
        child, _ = SD.simulate_env(SD.wrapper_text(self.plan, self.root, 'P0', 'feed-layer'), inherited)
        self.assertEqual(child['VK_LAYER_PATH'], str(self.root) + '\\runtime\\layer-x64'); self.assertEqual(child['VK_INSTANCE_LAYERS'], 'VK_LAYER_feed_vk'); self.assertNotIn('VK_ADD_LAYER_PATH', child); self.assertEqual(child['PATH'], 'p')
    def test_wrapper_clears_every_listed_control_and_is_process_local(self):
        t = SD.wrapper_text(self.plan, self.root, 'P1', 'direct'); lines = t.split('\r\n')
        self.assertEqual(lines[0], '@echo off'); self.assertIn('setlocal', lines); self.assertTrue(lines[-2].startswith('endlocal & exit /b %RC%'))
        for k in SD.CLEARED_ENV: self.assertIn(f'set "{k}="', lines, k)
        self.assertIn('set "RC=%ERRORLEVEL%"', lines); self.assertTrue(any(l.startswith('if not exist') and 'exit /b 3' in l for l in lines)); self.assertTrue(any('"%CD%"' in l and 'exit /b 3' in l for l in lines))
        self.assertLess(lines.index('setlocal'), min(i for i, l in enumerate(lines) if l.startswith('set "')))
    def test_wrapper_bytes_crlf_exactly_once(self):
        d = Path(self.td.name) / 'w.cmd.proposed.txt'; text = SD.wrapper_text(self.plan, self.root, 'P2', 'direct'); SD.write_wrapper(d, text)
        raw = d.read_bytes(); self.assertNotIn(b'\r\r\n', raw); self.assertEqual(raw.count(b'\n'), raw.count(b'\r\n')); self.assertTrue(raw.endswith(b'\r\n')); self.assertEqual(raw, text.encode('utf-8'))
        SD.write_wrapper(d, text.replace('\r\n', '\n')); self.assertEqual(d.read_bytes(), raw)             # LF input canonicalised to the same bytes
    def test_wrapper_unknown_method_refused(self):
        with self.assertRaises(SD.PlanError): SD.wrapper_text(self.plan, self.root, 'P0', 'both')

    # --- path spelling before resolution; run names ---
    def test_ambiguous_spellings_refused_before_resolution(self):
        for bad in ['F:/Games/NHL Legacy /child', 'F:\\Games\\NHL Legacy.', 'E:\\Games\\legacy-observation\\', 'E:\\Games\\\\x', 'E:\\Games\\CON\\x', 'E:\\Games\\nul.txt\\y', 'E:\\Games\\a:b', 'E:\\Games\\a?b', 'E:\\Games\\ lead', '\\\\server\\share\\x', 'E:\\Games\\tab\tx']:
            with self.assertRaises(SD.PlanError, msg=bad) as cm: SD.check_spelling(bad)
            self.assertEqual(cm.exception.code, 'root_ambiguous', bad)
        for ok in ['E:\\Games\\legacy-observation', 'E:/Games/legacy-observation', 'E:\\Games\\x.y\\z_1-2']: SD.check_spelling(ok)
    def test_codex_counterexample_trailing_space_refused_without_creating(self):
        plan = {**self.plan, 'root': 'F:/Games/NHL Legacy /child'}
        with self.assertRaises(SD.PlanError) as cm: SD.check_root(plan)
        self.assertEqual(cm.exception.code, 'root_ambiguous'); self.assertFalse(Path('F:/Games/NHL Legacy /child').exists()); self.assertFalse(Path('F:/Games/NHL Legacy/child').exists())
    def test_resolved_ancestry_still_checked_after_spelling(self):
        # a junction/alias-like case: the spelling is clean but the resolved path is inside the install
        alias = Path(self.td.name) / 'alias'          # NTFS junction (no privilege needed) inside the temp dir only
        import subprocess; r = subprocess.run(['cmd', '/c', 'mklink', '/J', str(alias), str(self.install)], capture_output=True, text=True)
        if r.returncode != 0 or not alias.exists(): self.skipTest('junction creation failed: ' + r.stderr.strip())
        with self.assertRaises(SD.PlanError) as cm: SD.check_root({**self.plan, 'root': str(alias / 'sub')})
        self.assertEqual(cm.exception.code, 'root_overlap')
    def test_run_name_validation(self):
        for bad in ['', '..', '../x', 'a/b', 'a\\b', 'name ', 'name.', 'CON', 'x' * 101, None, 'E:\\abs']:
            with self.assertRaises(SD.PlanError, msg=repr(bad)) as cm: SD.check_run_name(bad)
            self.assertEqual(cm.exception.code, 'run_name_invalid')
        out = SD.check_run_name('legacy-2026-09-14-x_1.0'); self.assertEqual(out.parent, SD.RUNS_DIR.resolve()); self.assertFalse(out.exists())

    # --- cache lifecycle ---
    def test_cache_attempt_never_deletes_and_advances(self):
        used = {str(self.root / 'cache' / 'PG')}
        c, n = SD.next_cache_attempt(self.root, 'PG', exists=lambda p: str(p) in used); self.assertEqual((c.name, n), ('PG-attempt2', 2))
        c, n = SD.next_cache_attempt(self.root, 'P0', exists=lambda p: str(p) in used); self.assertEqual((c.name, n), ('P0', 1))
        used.add(str(self.root / 'cache' / 'PG-attempt2')); c, n = SD.next_cache_attempt(self.root, 'PG', exists=lambda p: str(p) in used); self.assertEqual(n, 3)
        self.assertFalse((self.root / 'cache').exists())
    def test_pairing_plan_stages_parent_only(self):
        m = synth_manifest()
        plan = SD.pairing_plan({**self.plan, 'dossier': str(self.root / 'nope')}, self.root, m)
        for p in plan:
            self.assertEqual(p['cache_attempt'], 1); self.assertIn('never delete', p['cache_lifecycle']); self.assertTrue(any('does not exist' in c and 'never delete' in c for c in p['pre_launch_checks']))
            t = SD.wrapper_text(self.plan, self.root, p['id'], 'direct', p['cache_root']); self.assertIn(f'--cache_root "{p["cache_root"]}"', t)

    # --- config baseline drift is reported, not rebaselined ---
    def test_config_baseline_drift_reported(self):
        edits = SD.plan_config_edits(self.plan, self.install, self.root); pred = edits[-1]['combined_predictions']['ReShade.ini']['predicted_sha256_after_all_edits']
        base = SD.CONFIG_BASELINE['ReShade.ini']; self.assertNotEqual(pred, base['predicted_after_all_edits'])   # synthetic ini != live -> drift must be flagged
        self.assertEqual(len(base['live_sha256']), 64); self.assertEqual(base['root'], 'E:\\Games\\legacy-observation')

    def test_pairing_plan_from_synthetic_manifest(self):
        m = synth_manifest()
        plan = SD.pairing_plan({**self.plan, 'dossier': str(self.root / 'nope')}, self.root, m)
        self.assertEqual([p['id'] for p in plan], SD.PAIRINGS); self.assertEqual(len({p['cache_root'] for p in plan}), 5)
        for p in plan:
            self.assertTrue(p['cache_root'].startswith(str(self.root))); self.assertTrue(any('177024 bytes' in c for c in p['post_launch_log_checks'])); self.assertTrue(any('entry-open diagnostic only' in c for c in p['post_launch_log_checks']))
            self.assertTrue(any('PG indistinguishable' in s for s in p['stop_if'])); self.assertFalse(self.root.exists())

    def test_pairing_ids_from_manifest_semantic_controls(self):
        ids, ctl = SD.pairing_ids(synth_manifest()); self.assertEqual(ids, ['P0', 'PG', 'P1', 'P2', 'P3']); self.assertEqual(ctl, {'donor_baseline': 'P0', 'geometry_only': 'PG'})
        ids2, _ = SD.pairing_ids(synth_manifest(ids=('P1', 'PG', 'P0'))); self.assertEqual(ids2, ['P1', 'PG', 'P0'])      # reordered controls: manifest order kept, controls still found
        m = synth_manifest(ids=('R0', 'R1')); m['pairings'][0].update(id='R0', geometry='donor_player_67', texlib='donor_texlib_67', control=True, geometry_sha256='a' * 64, texlib_sha256='b' * 64); m['pairings'][1].update(id='R1', geometry='positions_only', texlib='donor_texlib_67', geometry_sha256='c' * 64, texlib_sha256='b' * 64); m['required_controls'] = {'donor_baseline': 'R0', 'geometry_only': 'R1'}
        self.assertEqual(SD.pairing_ids(m), (['R0', 'R1'], {'donor_baseline': 'R0', 'geometry_only': 'R1'}))
        plan = SD.pairing_plan({**self.plan, 'dossier': str(self.root / 'nope')}, self.root, m); self.assertEqual([p['id'] for p in plan], ['R0', 'R1']); self.assertTrue(plan[1]['cache_root'].endswith('R1')); self.assertEqual(plan[1]['cache_attempt'], 1)
        c, n = SD.next_cache_attempt(self.root, 'R1', exists=lambda p: p.name == 'R1'); self.assertEqual((c.name, n), ('R1-attempt2', 2))
    def test_pairing_ids_refusals(self):
        def refuse(m, code):
            with self.assertRaises(SD.PlanError) as cm: SD.pairing_ids(m)
            self.assertEqual(cm.exception.code, code, cm.exception)
        refuse({'artifacts': {}, 'pairings': []}, 'pairing_invalid'); refuse({'artifacts': {}}, 'pairing_invalid')
        for bad in ('p0', 'P 0', 'P0/../x', 'P0.txt', '..', 'CACHE\\P0', 'P' * 9, '', 7, None, 'CON', 'PRN', 'AUX', 'NUL', 'COM1', 'LPT1', 'COM0', 'LPT9'):   # Codex counterexample: 'CON' becomes a cache/file component
            m = synth_manifest(); m['pairings'][2]['id'] = bad; refuse(m, 'pairing_invalid')
        m = synth_manifest(); m['pairings'][2]['id'] = 'PG'; refuse(m, 'pairing_invalid')                       # duplicate
        m = synth_manifest(); m['pairings'][1]['geometry_sha256'] = 'e' * 64; refuse(m, 'pairing_invalid')     # hash not the artifact's
        m = synth_manifest(); m['pairings'][1]['geometry'] = 'ghost'; refuse(m, 'pairing_invalid')             # artifact missing
        refuse(synth_manifest(ids=('PG', 'P1')), 'control_missing')                                            # donor baseline absent
        refuse(synth_manifest(ids=('P0', 'P1')), 'control_missing')                                            # geometry-only control absent
        m = synth_manifest(); m['pairings'][0]['control'] = False; refuse(m, 'control_missing')                # baseline not flagged control
        m = synth_manifest(); m['pairings'][1]['texlib'] = 'full_chain'; m['pairings'][1]['texlib_sha256'] = 'd' * 64; refuse(m, 'control_missing')   # PG not donor texlib
        m = synth_manifest(); m['required_controls'] = {'donor_baseline': 'P0'}; refuse(m, 'control_missing')  # incomplete declaration
        m = synth_manifest(); m['required_controls'] = {'donor_baseline': 'P1', 'geometry_only': 'PG'}; refuse(m, 'control_missing')   # names a non-donor pairing
        m = synth_manifest(); del m['artifacts']['donor_texlib_67']; refuse(m, 'pairing_invalid')
        m = synth_manifest(); m['pairings'][2]['id'] = 'CONX'; self.assertEqual(SD.pairing_ids(m)[0][2], 'CONX')                    # only the exact device names are reserved
        self.assertIs(SD.valid_pairing_id, DV.valid_pairing_id)


if __name__ == '__main__': unittest.main()
