"""Tests for the observation-dossier validator (revision 2) and the v3 diagnostic profile. Synthetic tests build a tiny
dossier in a temporary directory and assert every refusal code, including the three exact in-memory counterexamples of
Codex's review (`experiments/codex/review_legacy_dossier.py`: wrong duplicated pairing hash, all geometry links missing,
all BC1 links missing) and the containment/type/id/control/malformed cases; the profile tests check v3's scope and that
v1/v2 are untouched; the licensed-fixture tests re-validate the frozen revision-2 dossier and replay the counterexamples
against it; StrictEvidenceTests cover the strict evidence mode (required pins, deletion of the whole evidence object,
null/empty/wrong-type pins, absent or changed files) and the documented weaker inspection mode.
Run: python -m unittest tests.adapters.test_legacy_dossier
"""
import copy, hashlib, json, os, sys, tempfile, unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]; sys.path.insert(0, str(ROOT))
from experiments.claude.legacy import dossier_validate as DV, observation_matrix as OM
DOSSIER = ROOT / 'local/runs/claude/legacy-2026-09-14-observation-dossier-02'; PROFILE = ROOT / 'profiles/nhl-legacy-x360-player67-diagnostic-v3.json'
sha = lambda b: hashlib.sha256(b).hexdigest()
H, A, S = 'a' * 64, 'b' * 64, 'c' * 64


def make_dossier(d):
    """Minimal valid manifest + files: donor pair, two mesh variants, texture + control, all controls present, links bound."""
    arts = {}
    for key, role, typ, sub in (('donor_player_67', 'donor_original', 'player_mesh_rx2', 'donor/p.rx2'), ('donor_texlib_67', 'donor_original', 'texlib_rx2', 'donor/t.rx2'), ('positions_only', 'geometry', 'player_mesh_rx2', 'geometry/pos.rx2'), ('shading_only', 'shading', 'player_mesh_rx2', 'geometry/sh.rx2'), ('full_chain', 'texture', 'texlib_rx2', 'texture/full.rx2'), ('base_only', 'texture_control', 'texlib_rx2', 'texture/base.rx2')):
        p = d / sub; p.parent.mkdir(parents=True, exist_ok=True); b = key.encode() * 7; p.write_bytes(b); arts[key] = {'file': sub, 'original_filename': p.name, 'label': key, 'role': role, 'type': typ, 'sha256': sha(b), 'bytes': len(b), 'source_sha256': sha(b)}
    ev = d / 'evidence'; ev.mkdir(); pins = {}
    for k, want in (('prediction', H), ('atlas04', A), ('atlas02', A), ('codex_bc1_stream', S)): pins[k] = {'path': str(ev / f'{k}.bin'), 'sha256': want}
    pairs = [('P0', 'donor_player_67', 'donor_texlib_67', True), ('PG', 'positions_only', 'donor_texlib_67', True), ('P1', 'positions_only', 'base_only', False), ('P2', 'positions_only', 'full_chain', False), ('P3', 'shading_only', 'full_chain', False)]
    return {'schema': DV.SCHEMA, 'status': 'offline dossier; not installation-ready, not game-verified, not production-ready', 'export_ready': False, 'ready_for_experimental_packing': False, 'import_ready_archive': False, 'runtime_approved': False, 'artifacts': arts,
            'pairings': [{'id': i, 'geometry': g, 'texlib': t, 'control': c, 'geometry_sha256': arts[g]['sha256'], 'texlib_sha256': arts[t]['sha256'], 'runtime_approved': False, 'purpose': i} for i, g, t, c in pairs],
            'links': {'prediction_sha256': H, 'bake04_geometry_input_sha256': H, 'render_exchange_sha256': H, 'stored_positions_equal_prediction': True, 'atlas_sha256': A, 'bake02_atlas_sha256': A, 'bc1_stream_sha256': S, 'codex_bc1_stream_sha256': S, 'full_chain_level0_sha256': S, 'base_only_level0_sha256': S, 'bc1_reencode_of_atlas_equals_stream': True},
            'evidence_pins': pins, 'view_sha256': {}, 'documents': {}}


def bind_evidence_files(d, m):
    """Strict-evidence fixture: write the four required pin files (+ one extra receipt) and re-point every link/pin hash
    at their real contents, so strict mode can re-hash them; artifacts' source paths point at the frozen copies."""
    ev = d / 'evidence'; ev.mkdir(exist_ok=True); real = {}
    for k, body in (('prediction', b'prediction-json'), ('atlas04', b'atlas-png'), ('atlas02', b'atlas-png'), ('codex_bc1_stream', b'bc1-stream'), ('codex_receipt', b'receipt')):
        f = ev / f'{k}.bin'; f.write_bytes(body); real[k] = {'path': str(f), 'sha256': sha(body)}
    m['evidence_pins'] = real; L = m['links']
    for k in DV.GEOMETRY_LINKS: L[k] = real['prediction']['sha256']
    for k in DV.ATLAS_LINKS: L[k] = real['atlas04']['sha256']
    for k in DV.BC1_LINKS: L[k] = real['codex_bc1_stream']['sha256']
    for a in m['artifacts'].values(): a['source_path'] = str(d / a['file'])
    return m


class StrictEvidenceTests(unittest.TestCase):
    """Strict evidence mode: the four bound pins are mandatory and every pinned file is re-hashed. Inspection mode keeps
    its documented weaker behaviour (absent pins warn) and the two modes are reported distinctly."""
    def setUp(self): self.td = tempfile.TemporaryDirectory(); self.d = Path(self.td.name); self.m = bind_evidence_files(self.d, make_dossier(self.d))
    def tearDown(self): self.td.cleanup()
    def strict_refused(self, code, mutate, **kw):
        m = copy.deepcopy(self.m); mutate(m)
        with self.assertRaises(DV.DossierError) as cm: DV.validate_manifest(m, self.d, strict_evidence=True, **kw)
        self.assertEqual(cm.exception.code, code, str(cm.exception))

    def test_strict_passes_when_pins_bound_and_files_present(self):
        r = DV.validate_manifest(self.m, self.d, strict_evidence=True)
        self.assertTrue(r['ok']); self.assertEqual(r['evidence_mode'], 'strict'); self.assertTrue(r['evidence_pins_verified']); self.assertEqual(r['warnings'], [])
        self.assertEqual(len([x for x in r['recomputed'] if x.startswith('evidence_pins')]), 5); self.assertFalse(any('evidence_pins' in x for x in r['reused_evidence']))
        self.assertEqual(DV.REQUIRED_EVIDENCE_PINS, ('prediction', 'atlas04', 'atlas02', 'codex_bc1_stream'))

    def test_inspection_mode_is_reported_as_weaker(self):
        r = DV.validate_manifest(self.m, self.d); self.assertEqual(r['evidence_mode'], 'inspection'); self.assertFalse(r['evidence_pins_verified']); self.assertTrue(any('evidence_pins' in x for x in r['reused_evidence']))
        r2 = DV.validate_manifest(self.m, self.d, check_sources=True); self.assertEqual(r2['evidence_mode'], 'inspection'); self.assertTrue(r2['evidence_pins_verified'])

    # --- the whole evidence object ---
    def test_entire_evidence_object_deleted(self):
        self.strict_refused('evidence_unbound', lambda m: m.pop('evidence_pins'))
        self.strict_refused('evidence_unbound', lambda m: m.pop('evidence_pins'), check_sources=True)
        m = copy.deepcopy(self.m); m.pop('evidence_pins'); r = DV.validate_manifest(m, self.d, check_sources=True)   # inspection mode: accepted with four warnings (documented, weaker)
        self.assertTrue(r['ok']); self.assertEqual(len([w for w in r['warnings'] if 'evidence_pins' in w]), 4); self.assertEqual(r['evidence_mode'], 'inspection')
    def test_evidence_object_null(self): self.strict_refused('evidence_unbound', lambda m: m.update(evidence_pins=None))
    def test_evidence_object_empty(self): self.strict_refused('evidence_unbound', lambda m: m.update(evidence_pins={}))
    def test_evidence_object_wrong_type(self):
        self.strict_refused('evidence_unbound', lambda m: m.update(evidence_pins=[])); self.strict_refused('evidence_unbound', lambda m: m.update(evidence_pins='pins')); self.strict_refused('evidence_unbound', lambda m: m.update(evidence_pins=4))

    # --- each required pin ---
    def test_each_required_pin_missing(self):
        for k in DV.REQUIRED_EVIDENCE_PINS: self.strict_refused('evidence_unbound', lambda m, k=k: m['evidence_pins'].pop(k))
    def test_required_pin_null_empty_or_wrong_type(self):
        for bad in (None, {}, [], '', 'sha', 0, ['path', 'sha']): self.strict_refused('evidence_unbound', lambda m, bad=bad: m['evidence_pins'].update(atlas02=bad))
    def test_required_pin_sha_missing_or_malformed(self):
        self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['prediction'].pop('sha256')); self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['prediction'].update(sha256=None))
        self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['prediction'].update(sha256='abc')); self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['prediction'].update(sha256=12345))
        self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['prediction'].update(sha256='A' * 64))
    def test_required_pin_path_missing_or_wrong_type(self):
        self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['codex_bc1_stream'].pop('path')); self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['codex_bc1_stream'].update(path=None))
        self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['codex_bc1_stream'].update(path='')); self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['codex_bc1_stream'].update(path=['x']))
    def test_required_pin_file_absent(self): self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['atlas04'].update(path=str(self.d / 'evidence' / 'nope.bin')))
    def test_required_pin_path_is_directory(self): self.strict_refused('evidence_unbound', lambda m: m['evidence_pins']['atlas04'].update(path=str(self.d / 'evidence')))
    def test_required_pin_disagrees_with_link(self):   # well-formed, existing file, wrong binding -> the hash code, not the structural one
        self.strict_refused('evidence_mismatch', lambda m: m['evidence_pins'].update(prediction=dict(m['evidence_pins']['codex_receipt'])))
    def test_required_pin_file_changed(self):
        Path(self.m['evidence_pins']['atlas02']['path']).write_bytes(b'edited'); self.strict_refused('evidence_mismatch', lambda m: None)
    def test_additional_pin_file_verified_in_strict(self):
        Path(self.m['evidence_pins']['codex_receipt']['path']).write_bytes(b'edited'); self.strict_refused('evidence_mismatch', lambda m: None)
        self.strict_refused('evidence_mismatch', lambda m: m['evidence_pins'].update(codex_receipt={'path': str(self.d / 'evidence' / 'gone.bin'), 'sha256': sha(b'receipt')}))
        self.strict_refused('evidence_mismatch', lambda m: m['evidence_pins'].update(codex_receipt={'path': None, 'sha256': sha(b'receipt')}))
    def test_additional_pin_absent_is_fine_in_strict(self):
        self.m['evidence_pins'].pop('codex_receipt'); r = DV.validate_manifest(self.m, self.d, strict_evidence=True); self.assertTrue(r['ok']); self.assertEqual(r['warnings'], [])
    def test_strict_still_refuses_earlier_codes_first(self):   # strict evidence adds a check; it does not weaken the others
        self.strict_refused('readiness_promoted', lambda m: m.update(runtime_approved=True)); self.strict_refused('pairing_invalid', lambda m: m['pairings'][2].update(geometry_sha256='0' * 64))
        self.strict_refused('stale_geometry_link', lambda m: [m['links'].pop(k) for k in DV.GEOMETRY_LINKS])


class ValidatorTests(unittest.TestCase):
    def setUp(self): self.td = tempfile.TemporaryDirectory(); self.d = Path(self.td.name); self.m = make_dossier(self.d)
    def tearDown(self): self.td.cleanup()
    def refused(self, code, mutate, **kw):
        m = copy.deepcopy(self.m); mutate(m)
        with self.assertRaises(DV.DossierError) as cm: DV.validate_manifest(m, self.d, **kw)
        self.assertEqual(cm.exception.code, code, str(cm.exception))

    def test_valid_manifest_passes_and_reports(self):
        r = DV.validate_manifest(self.m, self.d); self.assertTrue(r['ok']); self.assertEqual(r['warnings'], []); self.assertEqual(len([x for x in r['recomputed'] if x.startswith('artifact')]), 6); self.assertTrue(any('evidence_pins' in x for x in r['reused_evidence']))

    # --- Codex's three exact counterexamples (previously accepted) ---
    def test_codex_1_wrong_duplicated_pairing_hash(self): self.refused('pairing_invalid', lambda m: m['pairings'][2].update(geometry_sha256='0' * 64))
    def test_codex_2_all_geometry_links_missing(self): self.refused('stale_geometry_link', lambda m: [m['links'].pop(k) for k in DV.GEOMETRY_LINKS])
    def test_codex_3_all_bc1_links_missing(self): self.refused('stale_atlas_link', lambda m: [m['links'].pop(k) for k in DV.BC1_LINKS])
    def test_codex_4_absolute_path_inside_root_refused(self): self.refused('path_escape', lambda m: m['artifacts']['positions_only'].update(file=str((self.d / 'geometry/pos.rx2').resolve())))

    # --- hashes: present, well-formed, bound ---
    def test_one_geometry_link_missing(self): self.refused('stale_geometry_link', lambda m: m['links'].pop('render_exchange_sha256'))
    def test_geometry_links_both_none(self): self.refused('stale_geometry_link', lambda m: m['links'].update(prediction_sha256=None, bake04_geometry_input_sha256=None, render_exchange_sha256=None))
    def test_malformed_link_hash(self): self.refused('stale_atlas_link', lambda m: m['links'].update(atlas_sha256='B' * 64, bake02_atlas_sha256='B' * 64))
    def test_atlas_links_missing_together(self): self.refused('stale_atlas_link', lambda m: [m['links'].pop(k) for k in DV.ATLAS_LINKS])
    def test_pairing_texlib_hash_mismatch(self): self.refused('pairing_invalid', lambda m: m['pairings'][3].update(texlib_sha256=m['artifacts']['base_only']['sha256']))
    def test_pairing_hashes_swapped_between_pairings(self):   # P1 carries P2's texlib hash and P2 carries P1's: both well-formed, both real hashes, both wrong
        self.refused('pairing_invalid', lambda m: (m['pairings'][2].update(texlib_sha256=m['artifacts']['full_chain']['sha256']), m['pairings'][3].update(texlib_sha256=m['artifacts']['base_only']['sha256'])))
    def test_pairing_hash_missing(self): self.refused('pairing_invalid', lambda m: m['pairings'][1].pop('geometry_sha256'))
    def test_pairing_hash_malformed(self): self.refused('pairing_invalid', lambda m: m['pairings'][1].update(geometry_sha256='abc'))
    def test_artifact_hash_malformed(self): self.refused('stale_artifact', lambda m: m['artifacts']['full_chain'].update(sha256='xyz'))
    def test_artifact_source_hash_missing(self): self.refused('copy_mismatch', lambda m: m['artifacts']['full_chain'].pop('source_sha256'))
    def test_evidence_pin_disagrees_with_link(self): self.refused('evidence_mismatch', lambda m: m['evidence_pins']['prediction'].update(sha256='d' * 64))
    def test_evidence_pin_absent_is_lint_only(self):
        self.m['evidence_pins'].pop('atlas02'); r = DV.validate_manifest(self.m, self.d); self.assertTrue(any('atlas02' in w for w in r['warnings']))
    def test_evidence_pin_file_checked_with_sources(self):
        for k, want in (('prediction', H), ('atlas04', A), ('atlas02', A), ('codex_bc1_stream', S)): Path(self.m['evidence_pins'][k]['path']).write_bytes(b'x')
        for a in self.m['artifacts'].values(): a['source_path'] = str(self.d / a['file'])
        self.refused('evidence_mismatch', lambda m: None, check_sources=True)
    def test_source_changed_with_sources(self):
        for a in self.m['artifacts'].values(): a['source_path'] = str(self.d / a['file'])
        self.m['evidence_pins'] = {}; src = self.d / 'src.rx2'; src.write_bytes(b'other'); self.m['artifacts']['full_chain']['source_path'] = str(src)
        self.refused('copy_mismatch', lambda m: None, check_sources=True)

    # --- containment ---
    def test_traversal_refused(self):
        outside = self.d.parent / 'outside_full.rx2'; outside.write_bytes(b'full_chain' * 7)
        try: self.refused('path_escape', lambda m: m['artifacts']['full_chain'].update(file='../outside_full.rx2'))
        finally: outside.unlink()
    def test_external_absolute_refused(self):
        with tempfile.TemporaryDirectory() as td2:
            p = Path(td2) / 'full.rx2'; p.write_bytes(b'full_chain' * 7); self.refused('path_escape', lambda m: m['artifacts']['full_chain'].update(file=str(p)))
    def test_drive_relative_refused(self): self.refused('path_escape', lambda m: m['artifacts']['full_chain'].update(file='C:texture/full.rx2'))
    def test_root_slash_refused(self): self.refused('path_escape', lambda m: m['artifacts']['full_chain'].update(file='/texture/full.rx2'))
    def test_view_path_traversal_refused(self): self.refused('path_escape', lambda m: m['view_sha256'].update({'../x.png': 'e' * 64}))
    def test_protected_document_changed(self):
        (self.d / 'READ_ME_FIRST.md').write_text('a'); self.m['documents'] = {'READ_ME_FIRST.md': sha(b'a')}; DV.validate_manifest(self.m, self.d)
        (self.d / 'READ_ME_FIRST.md').write_text('b'); self.refused('stale_artifact', lambda m: None)

    # --- roles / types / ids / controls ---
    def test_donor_mesh_in_texlib_slot_refused(self): self.refused('pairing_invalid', lambda m: m['pairings'][0].update(texlib='donor_player_67', texlib_sha256=m['artifacts']['donor_player_67']['sha256']))
    def test_donor_texlib_in_geometry_slot_refused(self): self.refused('pairing_invalid', lambda m: m['pairings'][0].update(geometry='donor_texlib_67', geometry_sha256=m['artifacts']['donor_texlib_67']['sha256']))
    def test_type_missing_refused(self): self.refused('manifest_malformed', lambda m: m['artifacts']['full_chain'].pop('type'))
    def test_type_unknown_refused(self): self.refused('manifest_malformed', lambda m: m['artifacts']['full_chain'].update(type='rx2'))
    def test_role_type_conflict_refused(self): self.refused('pairing_invalid', lambda m: m['artifacts']['full_chain'].update(type='player_mesh_rx2'))
    def test_duplicate_pairing_id_refused(self): self.refused('pairing_invalid', lambda m: m['pairings'][4].update(id='P1'))
    def test_geometry_control_required(self): self.refused('pairing_control_missing', lambda m: m['pairings'].pop(1))
    def test_donor_control_required(self): self.refused('pairing_control_missing', lambda m: m['pairings'].pop(0))
    def test_texture_control_required(self): self.refused('pairing_control_missing', lambda m: m['pairings'][2].update(texlib='full_chain', texlib_sha256=m['artifacts']['full_chain']['sha256']))
    def test_donor_donor_must_be_marked_control(self): self.refused('pairing_invalid', lambda m: m['pairings'][0].update(control=False))
    def test_unknown_artifact_refused(self): self.refused('pairing_invalid', lambda m: m['pairings'][2].update(texlib='nope'))
    def test_pairing_runtime_approved_refused(self): self.refused('pairing_invalid', lambda m: m['pairings'][2].update(runtime_approved=True))
    def test_pairing_runtime_flag_missing_refused(self): self.refused('pairing_invalid', lambda m: m['pairings'][2].pop('runtime_approved'))

    # --- readiness flags are the authority; prose is lint ---
    # --- pairing ids: token rule + Windows reserved device names (Codex socket-review counterexample: 'CON' accepted) ---
    def test_reserved_device_name_pairing_id_refused(self):
        for bad in ('CON', 'PRN', 'AUX', 'NUL', 'COM1', 'LPT1', 'COM9', 'LPT0', 'con'.upper()):
            self.refused('pairing_invalid', lambda m, b=bad: m['pairings'][3].update(id=b))
        for bad in ('p1', 'P1/x', 'P1.txt', 'P' * 9, ''): self.refused('pairing_invalid', lambda m, b=bad: m['pairings'][3].update(id=b))
        self.assertTrue(DV.valid_pairing_id('CONX')); self.assertTrue(DV.valid_pairing_id('R1')); self.assertFalse(DV.valid_pairing_id('CON')); self.assertFalse(DV.valid_pairing_id(None))
        m = copy.deepcopy(self.m); m['pairings'][3]['id'] = 'CONX'; self.assertTrue(DV.validate_manifest(m, self.d)['ok'])

    # --- purposes: full-chain default unchanged; explicit versioned geometry-only purpose ---
    def geometry_only(self):
        """A geometry-only manifest: donor pair + two geometry variants against the donor texlib, no texture artifacts,
        explicit controls, prediction/render links only, prediction pin only."""
        m = copy.deepcopy(self.m); m['purpose'] = 'geometry_only/1'
        for k in ('full_chain', 'base_only'): m['artifacts'].pop(k)
        m['pairings'] = [{'id': i, 'geometry': g, 'texlib': 'donor_texlib_67', 'control': c, 'geometry_sha256': m['artifacts'][g]['sha256'], 'texlib_sha256': m['artifacts']['donor_texlib_67']['sha256'], 'runtime_approved': False, 'purpose': i}
                         for i, g, c in (('P0', 'donor_player_67', True), ('R0', 'positions_only', True), ('R1', 'shading_only', False))]
        m['required_controls'] = {'donor_baseline': 'P0', 'geometry_only': 'R0'}
        m['links'] = {'prediction_sha256': H, 'render_exchange_sha256': H, 'stored_positions_equal_prediction': True}; m['evidence_pins'] = {'prediction': self.m['evidence_pins']['prediction']}
        return m
    def test_purpose_absent_is_full_chain_and_dossier02_rules_unchanged(self):
        r = DV.validate_manifest(self.m, self.d); self.assertEqual(r['purpose'], 'full_chain/1')
        m = copy.deepcopy(self.m); m['purpose'] = 'full_chain/1'; self.assertEqual(DV.validate_manifest(m, self.d)['purpose'], 'full_chain/1')
        self.refused('pairing_control_missing', lambda m: m['pairings'][2].update(texlib='full_chain', texlib_sha256=m['artifacts']['full_chain']['sha256']))   # base-only control still required by default
        self.refused('purpose_invalid', lambda m: m.update(purpose='geometry_only'))      # unversioned
        self.refused('purpose_invalid', lambda m: m.update(purpose='geometry_only/2')); self.refused('purpose_invalid', lambda m: m.update(purpose=None))
        self.refused('purpose_invalid', lambda m: m.update(purpose='geometry_only/1'))   # the full-chain fixture carries texture artifacts: not a geometry-only dossier
    def test_geometry_only_purpose_accepts_r_sequence_without_texture_control(self):
        m = self.geometry_only(); r = DV.validate_manifest(m, self.d); self.assertTrue(r['ok']); self.assertEqual(r['purpose'], 'geometry_only/1'); self.assertEqual(r['warnings'], [])
        # strict evidence needs only the prediction pin here (bound file present)
        ev = self.d / 'evidence'; ev.mkdir(exist_ok=True); f = ev / 'prediction.bin'; f.write_bytes(b'prediction-json'); h = sha(b'prediction-json')
        m['evidence_pins'] = {'prediction': {'path': str(f), 'sha256': h}}; m['links'].update(prediction_sha256=h, render_exchange_sha256=h)
        self.assertEqual(DV.validate_manifest(m, self.d, strict_evidence=True)['evidence_mode'], 'strict')
        m2 = copy.deepcopy(m); m2['evidence_pins'] = {}
        with self.assertRaises(DV.DossierError) as cm: DV.validate_manifest(m2, self.d, strict_evidence=True)
        self.assertEqual(cm.exception.code, 'evidence_unbound')
    def test_geometry_only_purpose_refusals(self):
        def refused(code, mutate):
            m = self.geometry_only(); mutate(m)
            with self.assertRaises(DV.DossierError) as cm: DV.validate_manifest(m, self.d)
            self.assertEqual(cm.exception.code, code, str(cm.exception))
        refused('purpose_invalid', lambda m: m.pop('required_controls'))                                                  # controls must be explicit
        refused('purpose_invalid', lambda m: m.update(required_controls={'donor_baseline': 'R0', 'geometry_only': 'R1'}))   # R0 (failing-fitted geometry) relabelled as the baseline
        refused('purpose_invalid', lambda m: m.update(required_controls={'donor_baseline': 'P0', 'geometry_only': 'P0'}))
        refused('purpose_invalid', lambda m: m.update(required_controls={'donor_baseline': 'P0', 'geometry_only': 'ZZ'}))
        refused('pairing_invalid', lambda m: m['pairings'][0].update(control=False))                                          # the earlier donor/donor rule fires first
        refused('purpose_invalid', lambda m: m['artifacts'].update(full_chain=self.m['artifacts']['full_chain']))         # a texture artifact present
        refused('pairing_control_missing', lambda m: m['pairings'].pop(0))                                                # donor/donor baseline still required
        refused('stale_geometry_link', lambda m: m['links'].pop('render_exchange_sha256'))
        refused('stale_geometry_link', lambda m: m['links'].update(stored_positions_equal_prediction=False))
        refused('stale_atlas_link', lambda m: m['links'].update(atlas_sha256=A))                                           # a partial atlas group is still checked as a group
        refused('readiness_promoted', lambda m: m.update(runtime_approved=True)); refused('pairing_invalid', lambda m: m['pairings'][1].update(id='NUL'))
        refused('path_escape', lambda m: m['artifacts']['positions_only'].update(file='../pos.rx2'))
        refused('pairing_invalid', lambda m: m['pairings'][1].update(geometry_sha256='0' * 64))
        # the texlib slot of every pairing must be the donor texlib: even a well-formed geometry artifact there is refused by the slot rule
        refused('pairing_invalid', lambda m: m['pairings'][2].update(texlib='positions_only', texlib_sha256=m['artifacts']['positions_only']['sha256']))

    def test_export_ready_promotion_refused(self): self.refused('readiness_promoted', lambda m: m.update(export_ready=True))
    def test_packing_promotion_refused(self): self.refused('readiness_promoted', lambda m: m.update(ready_for_experimental_packing=True))
    def test_import_ready_archive_refused(self): self.refused('readiness_promoted', lambda m: m.update(import_ready_archive=True))
    def test_missing_flag_refused(self): self.refused('readiness_promoted', lambda m: m.pop('runtime_approved'))
    def test_flag_string_false_refused(self): self.refused('readiness_promoted', lambda m: m.update(runtime_approved='false'))
    def test_status_claim_is_lint_not_refusal(self):
        self.m['status'] = 'dossier is game-verified'; r = DV.validate_manifest(self.m, self.d); self.assertTrue(any('game-verified' in w for w in r['warnings']))
    def test_negated_claim_no_warning(self): self.m['status'] = 'not game-verified and not installation-ready'; self.assertEqual(DV.validate_manifest(self.m, self.d)['warnings'], [])

    # --- malformed inputs return deliberate errors ---
    def test_manifest_not_dict(self):
        with self.assertRaises(DV.DossierError) as cm: DV.validate_manifest([], self.d)
        self.assertEqual(cm.exception.code, 'manifest_malformed')
    def test_root_missing(self):
        with self.assertRaises(DV.DossierError) as cm: DV.validate_manifest(self.m, self.d / 'nope')
        self.assertEqual(cm.exception.code, 'manifest_malformed')
    def test_root_is_file(self):
        f = self.d / 'f'; f.write_bytes(b'x')
        with self.assertRaises(DV.DossierError) as cm: DV.validate_manifest(self.m, f)
        self.assertEqual(cm.exception.code, 'manifest_malformed')
    def test_missing_sections(self):
        for key in ('artifacts', 'pairings', 'links', 'status'): self.refused('manifest_malformed', lambda m, k=key: m.pop(k))
    def test_wrong_types(self):
        self.refused('manifest_malformed', lambda m: m.update(pairings={})); self.refused('manifest_malformed', lambda m: m.update(artifacts=[])); self.refused('manifest_malformed', lambda m: m['artifacts']['full_chain'].update(bytes='42'))
        self.refused('manifest_malformed', lambda m: m['artifacts']['full_chain'].update(file=None)); self.refused('manifest_malformed', lambda m: m.update(evidence_pins=[]))
    def test_stale_copy_refused(self):
        (self.d / 'geometry/pos.rx2').write_bytes(b'changed'); self.refused('stale_artifact', lambda m: None)
    def test_missing_copy_refused(self):
        (self.d / 'texture/full.rx2').unlink(); self.refused('stale_artifact', lambda m: None)
    def test_copy_not_the_reviewed_source_refused(self): self.refused('copy_mismatch', lambda m: m['artifacts']['full_chain'].update(source_sha256='e' * 64))

    def test_matrix_sequence_and_rows(self):
        ids = {k['id'] for k in OM.KNOWN_DEFECTS}; pids = [p['id'] for p in OM.PAIRINGS]; self.assertEqual(pids, ['P0', 'PG', 'P1', 'P2', 'P3'])
        by = {p['id']: p for p in OM.PAIRINGS}; self.assertEqual((by['PG']['geometry'], by['PG']['texlib']), ('positions_only', 'donor_texlib_67')); self.assertTrue(by['PG']['control'])
        self.assertEqual(by['P1']['geometry'], by['PG']['geometry']); self.assertEqual(by['P2']['geometry'], by['P1']['geometry']); self.assertEqual(by['P3']['texlib'], by['P2']['texlib'])
        for r in OM.ROWS:
            self.assertTrue(set(r['expected_known_defects']) <= ids, r['id']); self.assertTrue(set(r['pairings']) <= set(pids), r['id'])
            for k in ('pass', 'fail', 'unknown', 'evidence', 'stop_rule'): self.assertTrue(r[k], f'{r["id"]}.{k} empty')
            if r.get('approximation'): self.assertIn(r['approximation'], OM.PREVIEW_APPROXIMATIONS)
            if r['id'] in ('O2', 'O3', 'O5', 'O7', 'O7b', 'O8', 'O10', 'O11'): self.assertEqual(r['pairings'], ['P0', 'PG'], f'{r["id"]} must isolate geometry via PG')
        text = json.dumps(OM.ROWS + OM.KNOWN_DEFECTS).lower()
        for forbidden in ('would mean the base level differs', 'would mean the lower levels are not sampled', 'tangent convention wrong', 'unobservable unless', 'not resolvable at gameplay distance'): self.assertNotIn(forbidden, text, forbidden)
        self.assertTrue(all(k['id'] in {d for r in OM.ROWS for d in r['expected_known_defects']} for k in OM.KNOWN_DEFECTS), 'every known defect is attached to a row')


class ProfileTests(unittest.TestCase):
    def test_v3_valid_and_previous_unchanged(self):
        prev = {p['path']: p['sha256'] for p in json.loads(PROFILE.read_text(encoding='utf-8'))['previous_versions']}
        self.assertEqual(set(prev), {'profiles/nhl-legacy-x360-player67-diagnostic-v1.json', 'profiles/nhl-legacy-x360-player67-diagnostic-v2.json'})
        prof = DV.validate_profile(PROFILE, {str(ROOT / k): v for k, v in prev.items()}); self.assertEqual(prof['version'], 3)
        v2 = json.loads((ROOT / 'profiles/nhl-legacy-x360-player67-diagnostic-v2.json').read_text(encoding='utf-8'))
        for k in ('components', 'attachment', 'coordinate_system', 'palette', 'donor', 'texlib', 'source_provenance', 'component_policy'): self.assertEqual(prof[k], v2[k], k)

    def test_v3_refusals(self):
        prof = json.loads(PROFILE.read_text(encoding='utf-8'))
        def refused(code, mutate, raw=None):
            p = copy.deepcopy(prof); mutate(p)
            with tempfile.TemporaryDirectory() as td:
                f = Path(td) / 'p.json'; f.write_text(raw if raw is not None else json.dumps(p), encoding='utf-8')
                with self.assertRaises(DV.DossierError) as cm: DV.validate_profile(f)
                self.assertEqual(cm.exception.code, code)
        refused('profile_invalid', lambda p: p.update(export_ready=True)); refused('profile_invalid', lambda p: p.pop('export_ready'))
        refused('profile_invalid', lambda p: p['capabilities'].update(runtime_acceptance=True)); refused('profile_invalid', lambda p: p['capabilities'].pop('installation'))
        refused('profile_invalid', lambda p: p['capabilities'].update(production_export='structurally-validated'))
        refused('profile_invalid', lambda p: p['capabilities']['full_mip_chain']['scope'].update(width=256)); refused('profile_invalid', lambda p: p['capabilities']['full_mip_chain']['scope'].update(format='DXT3'))
        refused('profile_invalid', lambda p: p['capabilities']['full_mip_chain'].update(hair=True)); refused('profile_invalid', lambda p: p['capabilities']['full_mip_chain'].update(generalises_to=['1024x1024']))
        refused('profile_invalid', lambda p: p['capabilities']['full_mip_chain'].update(native_sampling='verified')); refused('profile_invalid', lambda p: p['capabilities']['full_mip_chain'].update(original_1x1_content_policy='per-channel max'))
        refused('profile_invalid', lambda p: p['capabilities'].update(normal_tangent_regeneration='native')); refused('profile_invalid', lambda p: p.update(version=2))
        refused('profile_invalid', lambda p: None, raw='not json'); refused('profile_invalid', lambda p: None, raw='[]')
        with tempfile.TemporaryDirectory() as td:
            f = Path(td) / 'v2.json'; f.write_bytes(b'{}')
            with self.assertRaises(DV.DossierError) as cm: DV.validate_profile(PROFILE, {str(f): 'f' * 64})
            self.assertEqual(cm.exception.code, 'profile_invalid')


@unittest.skipUnless((DOSSIER / 'MANIFEST.json').exists(), 'frozen dossier revision 2 absent (licensed local run)')
class FrozenDossierTests(unittest.TestCase):
    def setUp(self): self.m = json.loads((DOSSIER / 'MANIFEST.json').read_text(encoding='utf-8'))

    def test_frozen_dossier_validates(self):
        r = DV.validate_manifest(self.m, DOSSIER); self.assertTrue(r['ok']); self.assertEqual(r['warnings'], [])
        self.assertEqual([p['id'] for p in self.m['pairings']], ['P0', 'PG', 'P1', 'P2', 'P3']); self.assertTrue(all(p['runtime_approved'] is False for p in self.m['pairings']))
        self.assertEqual(self.m['artifacts']['full_chain']['sha256'], 'e1f9e3f6ff8661a2d23dc06b7a5fd51930fc304cd7260671967dab263b76a356'); self.assertEqual(self.m['artifacts']['positions_only']['sha256'], 'aea6a1cb4179a82e873a4abf66c48ce41295b6c610edc332e32826e36d8cdbbf')
        self.assertEqual(self.m['profile']['sha256'], sha(PROFILE.read_bytes())); self.assertTrue(self.m['measured_here']['texlib_full_chain']['level_hashes_equal_codex_review'])
        self.assertEqual(self.m['supersedes']['manifest_sha256'], '98c7f4696f049ce41c6044d66050de2cfb09a151e2f121297bf2241bf4e20d3c'); self.assertTrue(all(self.m['supersedes']['previous_artifact_hashes_equal_sources'].values()))
        self.assertTrue(all(r['coincide_on_these_pixels'] for r in json.loads((DOSSIER / 'filter_wording_correction.json').read_text(encoding='utf-8'))['demonstrated_on_specific_pixels']))

    def test_frozen_dossier_strict_evidence(self):
        r = DV.validate_manifest(self.m, DOSSIER, strict_evidence=True); self.assertTrue(r['ok']); self.assertEqual(r['evidence_mode'], 'strict'); self.assertEqual(r['warnings'], [])
        self.assertEqual({k for k in DV.REQUIRED_EVIDENCE_PINS} - set(self.m['evidence_pins']), set()); self.assertEqual(len([x for x in r['recomputed'] if x.startswith('evidence_pins')]), len(self.m['evidence_pins']))
        for code, alter in (('evidence_unbound', lambda v: v.pop('evidence_pins')), ('evidence_unbound', lambda v: v.update(evidence_pins={})), ('evidence_unbound', lambda v: v['evidence_pins'].pop('codex_bc1_stream')), ('evidence_unbound', lambda v: v['evidence_pins'].update(atlas02=None))):
            v = copy.deepcopy(self.m); alter(v)
            with self.assertRaises(DV.DossierError) as cm: DV.validate_manifest(v, DOSSIER, strict_evidence=True)
            self.assertEqual(cm.exception.code, code)
        v = copy.deepcopy(self.m); v.pop('evidence_pins'); r = DV.validate_manifest(v, DOSSIER)   # inspection mode on the same manifest: accepted, warned, reported as inspection
        self.assertEqual(r['evidence_mode'], 'inspection'); self.assertEqual(len([w for w in r['warnings'] if 'evidence_pins' in w]), 4)

    def test_codex_counterexamples_replayed_on_frozen_manifest(self):
        for code, alter in (('pairing_invalid', lambda v: v['pairings'][2].update(geometry_sha256='0' * 64)), ('stale_geometry_link', lambda v: [v['links'].pop(k) for k in DV.GEOMETRY_LINKS]), ('stale_atlas_link', lambda v: [v['links'].pop(k) for k in DV.BC1_LINKS]),
                            ('path_escape', lambda v: next(iter(v['artifacts'].values())).update(file=str((DOSSIER / next(iter(self.m['artifacts'].values()))['file']).resolve())))):
            v = copy.deepcopy(self.m); alter(v)
            with self.assertRaises(DV.DossierError) as cm: DV.validate_manifest(v, DOSSIER)
            self.assertEqual(cm.exception.code, code)
