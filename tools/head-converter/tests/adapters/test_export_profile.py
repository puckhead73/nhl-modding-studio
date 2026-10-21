"""H2.5 regressions: the versioned `nhl-head-export-profile/1` descriptor (resolution relative to the descriptor location or an explicit
inputs root, never the working directory; unknown schema / version / profile, malformed fields, missing or mismatched hashes, job /
profile disagreement), explicit `nhl21-build-receipt/1` selection (failed / tampered receipt, manifest or executable; relocated build
directory; review-list distinction), the exporter's conflict rules, refusal without output, the stand-in route through descriptor +
receipt (generated compatibility exchange, receipt provenance, evidence copies, stability pins), a controlled read guard showing the
historical phase-3 handoff is never opened, a relocated disposable bundle exported from another working directory, and - when the
real Blender, fixtures and the release-04 build are present - Turgeon / Fedorov parity with phase 11 through the checked-in descriptor."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from src.application import export as exporter
from src.application import export_profile as ep
from src.application import native_build

FAKE = Path(__file__).resolve().with_name('fake_native_tools.py')
TRACE = Path(__file__).resolve().with_name('read_trace.py')
DESCRIPTOR = exporter.REPO / 'profiles/nhl21-ps4-cusa18182-ekblad-export-v1.json'
SCHEMA_DOC = exporter.REPO / 'profiles/schemas/nhl-head-export-profile-1.schema.json'
RELEASE_04 = exporter.REPO / 'local/runs/claude/native-build-2026-09-17/release-04/build-receipt.json'
FIXTURES = {'turgeon': ('app-2026-09-12-job-turgeon-02', 'nhl21-phase11-export-turgeon-01'),
            'fedorov': ('app-2026-09-12-job-fedorov-02', 'nhl21-phase11-export-fedorov-01')}
DONOR_ENTRY, TEXTURE_ENTRY = 'game/player/fake_head.mcd', 'game/player/fake_head.gnf'
NATIVE_ID = 'test-native-adapter-v1'
CAPS = {'unchanged_roundtrip': 'measured', 'position_edit': 'qualification receipt required', 'changed_topology': False, 'game_verified': False}


def sha(b): return hashlib.sha256(b).hexdigest()


_TRACE = {'on': False, 'opened': [], 'installed': False}


def _audit(event, args):
    if _TRACE['on'] and event == 'open' and args and isinstance(args[0], (str, os.PathLike)): _TRACE['opened'].append(os.fspath(args[0]))


def trace_reads(stop=False):
    """In-process controlled read guard: start (returns the live list) / stop (returns the opens recorded) collecting every path this
    process opened through CPython's `open` audit event. The hook is installed once and gated, since audit hooks cannot be removed."""
    if not _TRACE['installed']: sys.addaudithook(_audit); _TRACE['installed'] = True
    if stop: _TRACE['on'] = False; return list(_TRACE['opened'])
    _TRACE['opened'].clear(); _TRACE['on'] = True; return _TRACE['opened']


def write_json(path, data): Path(path).write_text(json.dumps(data, indent=1), encoding='utf-8'); return Path(path)


def ref(path, base):
    """A descriptor reference: POSIX path relative to `base` + the file's sha256."""
    return {'path': os.path.relpath(path, base).replace(os.sep, '/'), 'sha256': sha(Path(path).read_bytes())}


def build_bundle(root):
    """A complete synthetic route: native inputs + native profile + preconditions, a descriptor beside the profile referencing them
    relatively, a synthetic built receipt with a v2 source manifest and executable, and a worker run whose job pins the same inputs."""
    root = Path(root).resolve()
    assets, profiles, build = root / 'assets', root / 'profiles', root / 'build'
    for d in (assets, profiles, build): d.mkdir()
    donor = assets / 'donor.mcd'; donor.write_bytes(b'donor bytes ' * 5)
    texture = assets / 'diffuse.gnf'; texture.write_bytes(b'texture bytes ' * 5)
    profile = write_json(profiles / 'native-profile.json', {
        'schema': ep.NATIVE_PROFILE_SCHEMA, 'id': NATIVE_ID, 'title': 'NHL21', 'platform': 'PS4', 'title_id': 'CUSA18182',
        'build': {'app_ver': '01.00', 'sfo_version': '01.01'}, 'donor': {'sha256': sha(donor.read_bytes())},
        'archive_provenance': {'asset_path': DONOR_ENTRY, 'archive': 'game_90.big'},
        'diffuse': {'provenance': {'asset_path': TEXTURE_ENTRY, 'decoded_sha256': sha(texture.read_bytes()), 'archive': 'game_90.big'}}, 'capabilities': CAPS})
    mesh = write_json(assets / 'native_mesh.json', {'schema': ep.MESH_EXCHANGE_SCHEMA, 'profile_id': NATIVE_ID, 'donor_sha256': sha(donor.read_bytes()), 'submeshes': []})
    preconditions = write_json(root / 'preconditions.json', {'targets': [
        {'asset_path': DONOR_ENTRY, 'decoded_sha256': sha(donor.read_bytes())}, {'asset_path': TEXTURE_ENTRY, 'decoded_sha256': sha(texture.read_bytes())}]})
    descriptor = {
        'schema': ep.DESCRIPTOR_SCHEMA, 'profile_id': 'test-export-profile', 'version': 1, 'status': 'synthetic test descriptor',
        'title': 'NHL21', 'platform': 'PS4', 'title_id': 'CUSA18182', 'build': {'app_ver': '01.00', 'sfo_version': '01.01', 'update_identifier': None},
        'source': {'kind': 'cc5_headshot_head_only_fbx', 'strategy': 'donor_fit'},
        'native_adapter': {'schema': ep.NATIVE_PROFILE_SCHEMA, 'profile_id': NATIVE_ID, 'writer_exchange_schema': ep.WRITER_EXCHANGE_SCHEMA,
                           'packer': {'crate': native_build.CRATE_NAME, 'build_receipt_schema': native_build.RECEIPT_SCHEMA,
                                      'source_manifest_schema': native_build.SOURCE_MANIFEST_SCHEMA}},
        'native_inputs': {'exchange': ref(mesh, profiles), 'native_donor': ref(donor, profiles), 'profile': ref(profile, profiles), 'texture': ref(texture, profiles)},
        'import_preconditions': ref(preconditions, profiles),
        'outputs': {'native_directory': 'nhl21-phase-native', 'native_files': [{'file': 'fake_head.mcd'}, {'file': 'fake_head.gnf'}], 'receipts': ['validation.json', 'final-audit.json']},
        'capabilities': dict(CAPS), 'evidence': {'fixtures': [], 'native_build': {'reviewed_executables': []}},
        'limitations': ['synthetic'], 'unknowns': ['synthetic']}
    # Synthetic built receipt: executable + v2 source manifest whose identity recomputes.
    exe = build / native_build.EXE_NAME; exe.write_bytes(b'fake native packer executable bytes')
    manifest = {'schema': native_build.SOURCE_MANIFEST_SCHEMA, 'crate': native_build.CRATE_NAME,
                'converter_files': {'native': [{'path': 'experiments/codex/nhl21_package/src/main.rs', 'sha256': '1' * 64, 'bytes': 1}]},
                'lockfile_sha256': '2' * 64, 'dependencies': {'tdb-rx2': {'files': [{'path': 'Cargo.toml', 'sha256': '3' * 64, 'bytes': 1}],
                'workspace_root_sha256': '4' * 64, 'inherited_fields': ['edition'], 'resolved_package_fields': {'edition': '"2021"'},
                'content_sha256': '5' * 64, 'git': {'head': 'abc', 'dirty_paths': ['src/x.rs']}}}}
    manifest['content_sha256'] = native_build.manifest_content_hash(manifest)
    manifest_path = write_json(build / 'source-manifest.json', manifest)
    descriptor['evidence']['native_build']['reviewed_executables'] = [{'sha256': sha(exe.read_bytes()), 'review': 'synthetic review record'}]
    descriptor_path = write_json(profiles / 'export-v1.json', descriptor)
    receipt = write_json(build / 'build-receipt.json', {
        'schema': native_build.RECEIPT_SCHEMA, 'status': 'built', 'crate': native_build.CRATE_NAME, 'profile': 'release', 'output': str(build),
        'tool': {'path': 'native_build.py', 'sha256': '6' * 64}, 'finished': '2026-09-17T00:00:00',
        'source_manifest': {'path': str(manifest_path), 'sha256': sha(manifest_path.read_bytes()), 'content_sha256': manifest['content_sha256']},
        'drift_check': {'rehashed_after_build': True, 'differences': []},
        'executable': {'path': str(exe), 'sha256': sha(exe.read_bytes()), 'bytes': exe.stat().st_size}})
    # Worker run pinned to the same native inputs.
    run = root / 'run'; (run / 'stages' / '00_fit').mkdir(parents=True)
    files = {n: write_json(run / n, {'name': n}) for n in ('status.json', 'handoff.json', 'loss_report.json', 'native_mesh.json', 'stages/00_fit/receipt.json')}
    (run / 'atlas.png').write_bytes(b'not really a png'); files['atlas.png'] = run / 'atlas.png'
    pin = lambda p: {'path': str(p), 'sha256': sha(p.read_bytes())}
    job = write_json(run / 'job.json', {'strategy': 'donor_fit', 'source': {'kind': 'cc5_headshot_head_only_fbx'},
                                        'destination': {'profile_id': NATIVE_ID, 'profile': pin(profile), 'exchange': pin(mesh), 'donor_mcd': pin(donor), 'native_diffuse_gnf': pin(texture)}})
    r = lambda p: {'path': str(p)}
    manifest_ui = write_json(run / 'ui_manifest.json', {
        'export_ready': True, 'files': {'job_json': r(job), 'status_json': r(files['status.json'])},
        'artifacts': {k: r(files[n]) for k, n in (('handoff_json', 'handoff.json'), ('native_mesh_json', 'native_mesh.json'), ('atlas_png', 'atlas.png'), ('loss_report_json', 'loss_report.json'))},
        'stages': [{'receipt': r(files['stages/00_fit/receipt.json'])}]})
    tools = dict(blender=FAKE, packer_script=FAKE, audit=FAKE, run_root=root, export_profile=descriptor_path, build_receipt=receipt)
    return {'root': root, 'descriptor': descriptor_path, 'descriptor_dict': descriptor, 'profiles': profiles, 'assets': assets, 'build': build,
            'receipt': receipt, 'manifest_path': manifest_path, 'exe': exe, 'profile': profile, 'mesh': mesh, 'donor': donor, 'texture': texture,
            'preconditions': preconditions, 'manifest': manifest_ui, 'job': job, 'files': files, 'tools': tools}


def variant(bundle, name, mutate):
    """A sibling descriptor (same directory, so relative references still resolve) with one mutation applied."""
    d = json.loads(bundle['descriptor'].read_text(encoding='utf-8'))
    mutate(d)
    return write_json(bundle['profiles'] / f'{name}.json', d)


class BundleCase(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='export-profile-'); self.addCleanup(self.temp.cleanup)
        self.bundle = build_bundle(self.temp.name); self.root = self.bundle['root']


class DescriptorTests(BundleCase):
    def test_resolves_relative_to_descriptor_or_explicit_root_never_cwd(self):
        elsewhere = self.root / 'elsewhere'; elsewhere.mkdir()
        cwd = os.getcwd(); os.chdir(elsewhere); self.addCleanup(os.chdir, cwd)
        loaded = ep.load_descriptor(self.bundle['descriptor'])
        self.assertEqual(loaded['profile_id'], 'test-export-profile'); self.assertEqual(loaded['version'], 1)
        self.assertEqual(loaded['native_inputs']['native_donor'], {'path': str(self.bundle['donor']), 'sha256': sha(self.bundle['donor'].read_bytes())})
        self.assertEqual(loaded['native_adapter'], {'schema': ep.NATIVE_PROFILE_SCHEMA, 'profile_id': NATIVE_ID, 'profile_sha256': sha(self.bundle['profile'].read_bytes())})
        self.assertEqual(loaded['preconditions']['path'], str(self.bundle['preconditions']))
        self.assertEqual(loaded['sha256'], sha(self.bundle['descriptor'].read_bytes()))
        # The same descriptor copied elsewhere (references dangling there) resolves through the explicit inputs root, not the cwd.
        moved = elsewhere / 'moved.json'; shutil.copyfile(self.bundle['descriptor'], moved)
        with self.assertRaisesRegex(ValueError, 'referenced file missing'): ep.load_descriptor(moved)
        self.assertEqual(ep.load_descriptor(moved, inputs_root=self.bundle['profiles'])['native_inputs'], loaded['native_inputs'])
        with self.assertRaisesRegex(ValueError, 'inputs root is not a directory'): ep.load_descriptor(moved, inputs_root=self.root / 'absent')
        self.assertEqual(ep.load_descriptor(moved, inputs_root=self.bundle['profiles'])['inputs_root'], str(self.bundle['profiles']))

    def test_unknown_schema_version_profile_and_route_refused(self):
        cases = {
            'schema': (lambda d: d.update(schema='nhl-head-export-profile/2'), 'schema is'),
            'version_zero': (lambda d: d.update(version=0), 'positive integer'),
            'version_str': (lambda d: d.update(version='1'), 'expected int'),
            'version_bool': (lambda d: d.update(version=True), 'expected int'),
            'profile_id': (lambda d: d.update(profile_id='Not Stable!'), 'stable lowercase identifier'),
            'route': (lambda d: d['source'].update(strategy='source_topology'), 'not a supported route'),
            'adapter_schema': (lambda d: d['native_adapter'].update(schema='nhl-head-profile/1'), 'native_adapter.schema'),
            'adapter_id': (lambda d: d['native_adapter'].update(profile_id='other-native-v9'), 'native profile id'),
            'writer_schema': (lambda d: d['native_adapter'].update(writer_exchange_schema='nhl21-phase4-handoff/1'), 'writer_exchange_schema'),
            'crate': (lambda d: d['native_adapter']['packer'].update(crate='other-crate'), 'packer.crate'),
            'manifest_schema': (lambda d: d['native_adapter']['packer'].update(source_manifest_schema='nhl21-build-source-manifest/1'), 'source_manifest_schema'),
        }
        for name, (mutate, msg) in cases.items():
            with self.subTest(case=name), self.assertRaisesRegex(ValueError, msg): ep.load_descriptor(variant(self.bundle, name, mutate))

    def test_malformed_fields_refused(self):
        cases = {
            'missing_top': (lambda d: d.pop('limitations'), 'lacks required fields: limitations'),
            'empty_title': (lambda d: d.update(title=''), 'must not be empty'),
            'no_update_identifier': (lambda d: d['build'].pop('update_identifier'), 'update_identifier must be present'),
            'absolute_ref': (lambda d: d['native_inputs']['native_donor'].update(path=str(self.bundle['donor']).replace(os.sep, '/')), 'not absolute'),
            'posix_abs_ref': (lambda d: d['native_inputs']['native_donor'].update(path='/assets/donor.mcd'), 'not absolute'),
            'backslash_ref': (lambda d: d['native_inputs']['native_donor'].update(path='..\\assets\\donor.mcd'), 'POSIX separators'),
            'bad_sha': (lambda d: d['native_inputs']['texture'].update(sha256='XYZ'), 'lowercase hex sha256'),
            'ref_not_object': (lambda d: d['native_inputs'].update(texture='../assets/diffuse.gnf'), 'must be an object'),
            'extra_input': (lambda d: d['native_inputs'].update(extra={'path': 'x', 'sha256': '0' * 64}), 'exactly the keys'),
            'missing_input': (lambda d: d['native_inputs'].pop('texture'), 'exactly the keys'),
            'precond_not_json': (lambda d: d['import_preconditions'].update(ref(self.bundle['donor'], self.bundle['profiles'])), 'reference a .json file'),
            'output_dir_not_plain': (lambda d: d['outputs'].update(native_directory='a/b'), 'not a plain file name'),
            'output_traversal': (lambda d: d['outputs']['native_files'].append({'file': '../x.mcd'}), 'not a plain file name'),
            'output_duplicate': (lambda d: d['outputs']['native_files'].append({'file': 'fake_head.mcd'}), 'not distinct'),
            'output_empty': (lambda d: d['outputs'].update(native_files=[]), 'must not be empty'),
            'caps_overclaim': (lambda d: d['capabilities'].update(game_verified=True), 'capabilities must equal'),
            'caps_aggregate': (lambda d: d['capabilities'].update(supported=True), 'capabilities must equal'),
            'caps_missing': (lambda d: d['capabilities'].pop('game_verified'), 'capabilities must equal'),
            'title_mismatch': (lambda d: d.update(title='NHL19'), "'NHL19' != native profile title"),
            'title_id_mismatch': (lambda d: d.update(title_id='CUSA00000'), 'native profile title_id'),
            'build_mismatch': (lambda d: d['build'].update(app_ver='01.02'), 'native profile build.app_ver'),
            'limitations_type': (lambda d: d.update(limitations=[1]), 'list of non-empty strings'),
            'reviewed_malformed': (lambda d: d['evidence']['native_build'].update(reviewed_executables=[{'sha256': 'nope'}]), 'reviewed_executables'),
            'fixtures_type': (lambda d: d['evidence'].update(fixtures='none'), 'evidence.fixtures: expected list'),
        }
        for name, (mutate, msg) in cases.items():
            with self.subTest(case=name), self.assertRaisesRegex(ValueError, msg): ep.load_descriptor(variant(self.bundle, name, mutate))
        for text, msg in (('{not json', 'not readable JSON'), ('[]', 'must be a JSON object')):
            bad = self.bundle['profiles'] / 'bad.json'; bad.write_text(text, encoding='utf-8')
            with self.assertRaisesRegex(ValueError, msg): ep.load_descriptor(bad)
        with self.assertRaisesRegex(ValueError, 'missing'): ep.load_descriptor(self.bundle['profiles'] / 'absent.json')
        with self.assertRaisesRegex(ValueError, 'must be a .json file'): ep.load_descriptor(self.bundle['donor'])

    def test_missing_or_mismatched_hashes_and_identity_refused(self):
        with self.subTest(case='declared hash differs'):
            with self.assertRaisesRegex(ValueError, 'native_inputs.profile: referenced file .* does not match'):
                ep.load_descriptor(variant(self.bundle, 'hash', lambda d: d['native_inputs']['profile'].update(sha256='0' * 64)))
        with self.subTest(case='referenced file missing'):
            with self.assertRaisesRegex(ValueError, 'import_preconditions: referenced file missing'):
                ep.load_descriptor(variant(self.bundle, 'gone', lambda d: d['import_preconditions'].update(path='../gone.json')))
        with self.subTest(case='native profile donor hash != referenced donor'):
            other = self.bundle['assets'] / 'other.mcd'; other.write_bytes(b'another donor')
            with self.assertRaisesRegex(ValueError, 'donor sha256 != native_inputs.native_donor'):
                ep.load_descriptor(variant(self.bundle, 'donor', lambda d: d['native_inputs'].update(native_donor=ref(other, self.bundle['profiles']))))
        with self.subTest(case='mesh names another profile'):
            mesh = json.loads(self.bundle['mesh'].read_text()); mesh['profile_id'] = 'other'; m2 = write_json(self.bundle['assets'] / 'mesh2.json', mesh)
            with self.assertRaisesRegex(ValueError, 'mesh profile_id / donor_sha256 disagree'):
                ep.load_descriptor(variant(self.bundle, 'mesh', lambda d: d['native_inputs'].update(exchange=ref(m2, self.bundle['profiles']))))
        with self.subTest(case='native profile file is not nhl-mcd-profile/1'):
            prof = json.loads(self.bundle['profile'].read_text()); prof['schema'] = 'x'; p2 = write_json(self.bundle['profiles'] / 'p2.json', prof)
            with self.assertRaisesRegex(ValueError, 'not a nhl-mcd-profile/1 profile'):
                ep.load_descriptor(variant(self.bundle, 'pschema', lambda d: d['native_inputs'].update(profile=ref(p2, self.bundle['profiles']))))
        with self.subTest(case='tampered file after the descriptor was written'):
            self.bundle['texture'].write_bytes(b'tampered texture')
            with self.assertRaisesRegex(ValueError, 'native_inputs.texture: referenced file .* does not match'): ep.load_descriptor(self.bundle['descriptor'])

    def test_checked_in_descriptor_matches_the_schema_document(self):
        schema = json.loads(SCHEMA_DOC.read_text(encoding='utf-8'))
        self.assertEqual(schema['$id'], ep.DESCRIPTOR_SCHEMA)
        self.assertEqual(sorted(schema['required']), sorted(ep.REQUIRED_TOP))
        self.assertEqual(sorted(schema['properties']['native_inputs']['required']), sorted(ep.NATIVE_INPUT_KEYS))
        d = json.loads(DESCRIPTOR.read_text(encoding='utf-8'))
        self.assertEqual(sorted(set(d) - set(schema['properties'])), [], 'descriptor keys not described by the schema')
        for key in schema['required']: self.assertIn(key, d)
        self.assertEqual(schema['properties']['schema']['const'], d['schema'])
        self.assertEqual(sorted(d['native_inputs']), sorted(ep.NATIVE_INPUT_KEYS))
        if all(Path(exporter.REPO / 'profiles' / Path(*r['path'].split('/'))).is_file() for r in d['native_inputs'].values()):
            loaded = ep.load_descriptor(DESCRIPTOR)   # the real references exist here: the checked-in descriptor must load and agree with the native profile
            self.assertEqual(loaded['native_adapter']['profile_id'], 'nhl21-ps4-cusa18182-game90-aaron-ekblad-v1')
            self.assertEqual(loaded['native_inputs']['profile']['sha256'], '369c1bb898790ad3021dc2d897575fad4b868804a011292b2d5e02ae30ee892b')


class BuildReceiptTests(BundleCase):
    def receipt_variant(self, name, mutate):
        r = json.loads(self.bundle['receipt'].read_text()); mutate(r); return write_json(self.bundle['build'] / f'{name}.json', r)

    def test_valid_receipt_resolves_siblings_and_records_provenance(self):
        loaded = ep.load_descriptor(self.bundle['descriptor'])
        build = ep.select_build_receipt(self.bundle['receipt'], loaded['reviewed_executables'])
        self.assertEqual(build['executable'], {'path': str(self.bundle['exe']), 'sha256': sha(self.bundle['exe'].read_bytes()), 'bytes': self.bundle['exe'].stat().st_size})
        self.assertEqual(build['source_manifest']['content_sha256'], json.loads(self.bundle['manifest_path'].read_text())['content_sha256'])
        self.assertEqual(build['source_manifest']['dependencies'], {'tdb-rx2': {'content_sha256': '5' * 64, 'git_head': 'abc', 'dirty_paths': 1}})
        self.assertEqual(build['independent_review'], {'matched': True, 'review': 'synthetic review record'})
        self.assertIn('not a signature', build['provenance'])
        self.assertFalse(build['converter_sources_now']['matched'], 'synthetic manifest hashes cannot match the real converter sources')
        # Without a descriptor review list, or with one that lacks the hash, the executable is validated but not endorsed.
        self.assertEqual(ep.select_build_receipt(self.bundle['receipt'])['independent_review']['matched'], False)
        self.assertIn('not listed', ep.select_build_receipt(self.bundle['receipt'], {})['independent_review']['note'])
        # Relocated build directory: the recorded absolute paths are stale, the siblings resolve.
        moved = self.root / 'relocated-build'; shutil.copytree(self.bundle['build'], moved)
        relocated = ep.select_build_receipt(moved / 'build-receipt.json')
        self.assertEqual(relocated['executable']['path'], str(moved / native_build.EXE_NAME))
        self.assertEqual(relocated['executable']['sha256'], build['executable']['sha256'])

    def test_failed_tampered_or_incomplete_receipts_refused(self):
        cases = {
            'status_failed': (lambda r: r.update(status='failed'), 'not a successful built receipt'),
            'status_snapshot': (lambda r: r.update(status='snapshot'), 'not a successful built receipt'),
            'schema': (lambda r: r.update(schema='nhl21-build-receipt/2'), 'schema is'),
            'crate': (lambda r: r.update(crate='other'), 'crate'),
            'drift': (lambda r: r['drift_check'].update(differences=['converter:x: declared a captured b']), 'drift check'),
            'drift_not_rehashed': (lambda r: r['drift_check'].update(rehashed_after_build=False), 'drift check'),
            'no_drift': (lambda r: r.pop('drift_check'), 'drift check'),
            'exe_hash': (lambda r: r['executable'].update(sha256='0' * 64), 'does not match the receipt sha256 / size'),
            'exe_size': (lambda r: r['executable'].update(bytes=1), 'does not match the receipt sha256 / size'),
            'exe_missing': (lambda r: r['executable'].update(path='C:/somewhere/absent.exe'), 'executable missing beside the receipt'),
            'manifest_hash': (lambda r: r['source_manifest'].update(sha256='0' * 64), 'source manifest .* does not match'),
            'manifest_missing': (lambda r: r['source_manifest'].update(path='absent-manifest.json'), 'source manifest missing'),
            'identity': (lambda r: r['source_manifest'].update(content_sha256='0' * 64), 'identity does not recompute'),
            'no_exe_block': (lambda r: r.pop('executable'), 'executable block lacks'),
        }
        for name, (mutate, msg) in cases.items():
            with self.subTest(case=name), self.assertRaisesRegex(ValueError, msg): ep.select_build_receipt(self.receipt_variant(name, mutate))
        with self.subTest(case='v1 source manifest'):
            m = json.loads(self.bundle['manifest_path'].read_text()); m['schema'] = 'nhl21-build-source-manifest/1'
            v1 = write_json(self.bundle['build'] / 'v1-manifest.json', m)
            r = self.receipt_variant('v1', lambda r: r['source_manifest'].update(path=str(v1), sha256=sha(v1.read_bytes())))
            with self.assertRaisesRegex(ValueError, 'v1 receipts are not accepted'): ep.select_build_receipt(r)
        with self.subTest(case='manifest identity edited after the build'):
            m = json.loads(self.bundle['manifest_path'].read_text()); m['lockfile_sha256'] = '9' * 64
            edited = write_json(self.bundle['build'] / 'edited-manifest.json', m)
            r = self.receipt_variant('edited', lambda r: r['source_manifest'].update(path=str(edited), sha256=sha(edited.read_bytes())))
            with self.assertRaisesRegex(ValueError, 'identity does not recompute'): ep.select_build_receipt(r)
        with self.subTest(case='executable replaced on disk'):
            self.bundle['exe'].write_bytes(b'a different executable')
            with self.assertRaisesRegex(ValueError, 'does not match the receipt sha256 / size'): ep.select_build_receipt(self.bundle['receipt'])
        for text in ('{bad', '[]'):
            bad = self.bundle['build'] / 'bad.json'; bad.write_text(text, encoding='utf-8')
            with self.assertRaises(ValueError): ep.select_build_receipt(bad)


class ExporterIntegrationTests(BundleCase):
    def setUp(self):
        super().setUp()
        self.check = patch.object(exporter, 'check', return_value=(True, [])); self.check.start(); self.addCleanup(self.check.stop)
        self.tools = self.bundle['tools']

    def preflight(self, **overrides):
        return exporter.preflight(self.bundle['manifest'], self.root / 'out', **{**self.tools, **overrides})

    def test_conflicting_options_refused_without_output(self):
        handoff = write_json(self.root / 'handoff.json', {'schema': exporter.EXCHANGE_SCHEMA})
        cases = [({'exchange': handoff}, '--export-profile conflicts with --exchange'),
                 ({'preconditions': self.bundle['preconditions']}, '--export-profile conflicts with --preconditions'),
                 ({'exchange': handoff, 'preconditions': self.bundle['preconditions']}, 'conflicts with --exchange, --preconditions'),
                 ({'packer_exe': self.bundle['exe']}, '--build-receipt conflicts with --packer-exe'),
                 ({'export_profile': None, 'profile_inputs_root': self.bundle['profiles']}, '--profile-inputs-root requires --export-profile')]
        for overrides, msg in cases:
            with self.subTest(overrides=sorted(overrides)), self.assertRaisesRegex(ValueError, msg):
                exporter.export(self.bundle['manifest'], self.root / 'out', **{**self.tools, **overrides})
        self.assertFalse((self.root / 'out').exists())
        # CLI: refusal exit code 2, nothing created, the conflict named.
        cmd = [sys.executable, '-B', '-m', 'src.application.export', str(self.bundle['manifest']), '--out', str(self.root / 'cli-out'), '--run-root', str(self.root),
               '--blender', str(FAKE), '--packer-script', str(FAKE), '--audit', str(FAKE), '--export-profile', str(self.bundle['descriptor']),
               '--build-receipt', str(self.bundle['receipt']), '--packer-exe', str(self.bundle['exe'])]
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=exporter.REPO)
        self.assertEqual(result.returncode, exporter.EXIT_CODES['refused'], result.stdout + result.stderr)
        self.assertIn('--build-receipt conflicts with --packer-exe', json.loads(result.stdout.strip().splitlines()[-1])['error'])
        self.assertFalse((self.root / 'cli-out').exists())

    def test_job_and_profile_disagreement_refused_without_output(self):
        job = json.loads(self.bundle['job'].read_text())
        def with_job(mutate):
            j = json.loads(json.dumps(job)); mutate(j); write_json(self.bundle['job'], j)
        cases = [(lambda j: j['destination'].update(profile_id='other-native-v2'), 'disagrees with export profile'),
                 (lambda j: j.update(strategy='source_topology'), 'not the export profile route'),
                 (lambda j: j['source'].update(kind='cc5_full_body_fbx'), 'not the export profile route'),
                 (lambda j: j['destination']['donor_mcd'].update(sha256='0' * 64), 'changed destination donor_mcd')]
        for mutate, msg in cases:
            with_job(mutate)
            with self.subTest(msg=msg), self.assertRaisesRegex(ValueError, msg): self.preflight()
        write_json(self.bundle['job'], job)
        # A job pinned to a different (existing, valid) donor than the descriptor references is refused before any write.
        other = self.bundle['assets'] / 'other.mcd'; other.write_bytes(b'other donor')
        with_job(lambda j: j['destination'].update(donor_mcd={'path': str(other), 'sha256': sha(other.read_bytes())}))
        with self.assertRaisesRegex(ValueError, 'changed destination donor_mcd'): self.preflight()
        # An invalid build selection refuses too - it never falls back to the default executable.
        write_json(self.bundle['job'], job)
        broken = write_json(self.bundle['build'] / 'broken.json', {**json.loads(self.bundle['receipt'].read_text()), 'status': 'failed'})
        with self.assertRaisesRegex(ValueError, 'not a successful built receipt'): self.preflight(build_receipt=broken)
        with self.assertRaisesRegex(ValueError, 'Build receipt missing'): self.preflight(build_receipt=self.root / 'absent.json')
        self.assertFalse((self.root / 'out').exists())
        manifest, inputs, pins = self.preflight()
        self.assertEqual(inputs['packer_exe'], self.bundle['exe'].resolve()); self.assertIsNone(inputs['exchange'])
        self.assertEqual(pins['export_profile']['profile_id'], 'test-export-profile'); self.assertEqual(pins['build']['receipt']['path'], str(self.bundle['receipt']))
        self.assertFalse((self.root / 'out').exists())

    def test_stand_in_export_through_descriptor_and_receipt(self):
        output = self.root / 'ok'
        with patch.dict(os.environ, {'FAKE_NATIVE_MODE': 'ok'}):
            receipt = exporter.export(self.bundle['manifest'], output, **self.tools)
        self.assertEqual(receipt['status'], 'completed')
        self.assertEqual(json.loads((output / 'export.json').read_text()), receipt)
        inputs = receipt['inputs']
        self.assertEqual(inputs['export_profile']['sha256'], sha(self.bundle['descriptor'].read_bytes()))
        self.assertEqual((inputs['export_profile']['profile_id'], inputs['export_profile']['version'], inputs['export_profile']['schema']), ('test-export-profile', 1, ep.DESCRIPTOR_SCHEMA))
        self.assertEqual(inputs['export_profile']['native_adapter']['profile_id'], NATIVE_ID)
        self.assertEqual(inputs['build']['receipt']['sha256'], sha(self.bundle['receipt'].read_bytes()))
        self.assertEqual(inputs['build']['executable']['sha256'], sha(self.bundle['exe'].read_bytes()))
        self.assertEqual(inputs['tools']['packer_exe'], {'path': str(self.bundle['exe']), 'sha256': sha(self.bundle['exe'].read_bytes())})
        self.assertEqual(inputs['build']['source_manifest']['content_sha256'], json.loads(self.bundle['manifest_path'].read_text())['content_sha256'])
        self.assertTrue(inputs['build']['independent_review']['matched'])
        # The packer consumed the generated compatibility exchange beneath the export directory, not a research handoff.
        generated = output / exporter.GENERATED_EXCHANGE
        self.assertEqual(inputs['exchange'], {'generated_from': 'export_profile', 'schema': exporter.EXCHANGE_SCHEMA, 'path': str(generated), 'sha256': sha(generated.read_bytes()),
                                              'pins': {k: {'path': str(self.bundle[n]), 'sha256': sha(self.bundle[n].read_bytes())}
                                                       for k, n in (('exchange', 'mesh'), ('native_donor', 'donor'), ('profile', 'profile'), ('texture', 'texture'))}})
        exchange = json.loads(generated.read_text())
        self.assertEqual(exchange['schema'], exporter.EXCHANGE_SCHEMA); self.assertEqual(exchange['generated_from']['export_profile']['sha256'], inputs['export_profile']['sha256'])
        self.assertEqual({k: exchange[k] for k in ep.NATIVE_INPUT_KEYS}, inputs['exchange']['pins'])
        pack = receipt['commands'][0]
        self.assertEqual(pack[pack.index('--exchange') + 1], str(generated))
        self.assertEqual(pack[pack.index('--packer-exe') + 1], str(self.bundle['exe']))
        self.assertEqual(pack[pack.index('--preconditions') + 1], str(self.bundle['preconditions']))
        self.assertEqual(json.loads((output / 'nhl21-phase-native' / 'validation.json').read_text())['exchange'], str(generated))
        self.assertIn('via export profile test-export-profile v1', receipt['scope'])
        evidence = {e['path']: e['sha256'] for e in receipt['evidence']}
        for name, source in (('evidence/export-profile.json', self.bundle['descriptor']), ('evidence/build-receipt.json', self.bundle['receipt']),
                             ('evidence/build-source-manifest.json', self.bundle['manifest_path'])):
            self.assertEqual(evidence[str(Path(name))], sha(source.read_bytes()))
        self.assertEqual([a['path'] for a in receipt['artifacts']], [str(Path('nhl21-phase-native') / n) for n in ('fake_head.mcd', 'fake_head.gnf', 'validation.json', 'final-audit.json')])
        started = json.loads((output / 'events.jsonl').read_text().splitlines()[0])
        self.assertEqual(started['export_profile'], {'profile_id': 'test-export-profile', 'version': 1, 'sha256': inputs['export_profile']['sha256']})
        self.assertEqual(started['build_receipt'], inputs['build']['receipt']['sha256'])
        # Compatibility route unchanged: no descriptor, no receipt -> exchange path pin, build provenance says so.
        handoff = write_json(self.root / 'handoff.json', {'schema': exporter.EXCHANGE_SCHEMA, **inputs['exchange']['pins']})
        with patch.dict(os.environ, {'FAKE_NATIVE_MODE': 'ok'}):
            legacy = exporter.export(self.bundle['manifest'], self.root / 'legacy', **{**self.tools, 'export_profile': None, 'build_receipt': None,
                                                                                    'exchange': handoff, 'preconditions': self.bundle['preconditions'], 'packer_exe': self.bundle['exe']})
        self.assertEqual(legacy['inputs']['exchange']['path'], str(handoff)); self.assertIsNone(legacy['inputs']['export_profile'])
        self.assertIsNone(legacy['inputs']['build']['receipt']); self.assertIn('no build receipt selected', legacy['inputs']['build']['provenance'])
        self.assertEqual([a['sha256'] for a in legacy['artifacts']][:2], [a['sha256'] for a in receipt['artifacts']][:2], 'same native bytes either way')

    def test_descriptor_or_receipt_change_during_export_fails(self):
        output = self.root / 'changed'
        original = exporter.pinned_inputs_stable
        def tamper(pins):
            self.bundle['descriptor'].write_text(self.bundle['descriptor'].read_text() + ' ', encoding='utf-8')
            self.bundle['receipt'].write_text(self.bundle['receipt'].read_text() + ' ', encoding='utf-8')
            return original(pins)
        with patch.object(exporter, 'pinned_inputs_stable', tamper), patch.dict(os.environ, {'FAKE_NATIVE_MODE': 'ok'}):
            with self.assertRaisesRegex(ValueError, 'Pinned inputs changed during export: export_profile, build.receipt'):
                exporter.export(self.bundle['manifest'], output, **self.tools)
        self.assertEqual(json.loads((output / 'export.json').read_text())['status'], 'failed')

    def test_historical_handoff_never_opened_and_relocated_bundle_exports_from_another_cwd(self):
        # Disposable relocated copy of the whole bundle; the original stays untouched (hashed before / after).
        before = {p: sha(p.read_bytes()) for p in self.root.rglob('*') if p.is_file()}
        moved = Path(self.temp.name).parent / f'{Path(self.temp.name).name}-relocated'
        shutil.copytree(self.root, moved); self.addCleanup(shutil.rmtree, moved, True)
        elsewhere = Path(self.temp.name).parent / f'{Path(self.temp.name).name}-cwd'; elsewhere.mkdir(); self.addCleanup(shutil.rmtree, elsewhere, True)
        cwd = os.getcwd(); os.chdir(elsewhere); self.addCleanup(os.chdir, cwd)
        # The relocated worker manifest / job still name the ORIGINAL bundle's pins by absolute path (worker artifacts are not the
        # subject of relocation); the descriptor's references relocate with it and must resolve there. Reads of this process are
        # traced through the controlled audit hook (the packer stand-in only sees the command line the receipt records).
        opened = trace_reads()
        with patch.dict(os.environ, {'FAKE_NATIVE_MODE': 'ok'}):
            receipt = exporter.export(self.bundle['manifest'], moved / 'relocated-export', **{**self.tools, 'run_root': moved,
                                                                                            'export_profile': moved / 'profiles' / 'export-v1.json',
                                                                                            'build_receipt': moved / 'build' / 'build-receipt.json'})
        opened = {str(Path(p).resolve()) for p in trace_reads(stop=True) if Path(p).is_absolute()}
        self.assertEqual(receipt['status'], 'completed')
        self.assertEqual(receipt['inputs']['export_profile']['path'], str(moved / 'profiles' / 'export-v1.json'))
        self.assertEqual(receipt['inputs']['export_profile']['inputs_root'], str(moved / 'profiles'))
        self.assertEqual(receipt['inputs']['exchange']['pins']['native_donor']['path'], str(moved / 'assets' / 'donor.mcd'))
        self.assertEqual(receipt['inputs']['build']['executable']['path'], str(moved / 'build' / native_build.EXE_NAME))
        self.assertNotIn(str(exporter.EXCHANGE.resolve()), opened, 'the historical phase-3 handoff was opened')
        self.assertFalse(any('nhl21-phase3-handoff' in p for p in opened), 'the historical phase-3 handoff was opened')
        for must in (moved / 'profiles' / 'export-v1.json', moved / 'assets' / 'donor.mcd', moved / 'build' / 'build-receipt.json', moved / 'relocated-export' / exporter.GENERATED_EXCHANGE):
            self.assertIn(str(must.resolve()), opened, f'{must} was not read by the exporter')
        pack = receipt['commands'][0]
        self.assertEqual(pack[pack.index('--exchange') + 1], str(moved / 'relocated-export' / exporter.GENERATED_EXCHANGE))
        after = {p: sha(p.read_bytes()) for p in self.root.rglob('*') if p.is_file()}
        self.assertEqual(before, after, 'the original bundle changed')


class AcceptedFixtureProfileParityTests(unittest.TestCase):
    """Turgeon / Fedorov through the checked-in descriptor + the reviewed release-04 build receipt under a fresh run root, from a working
    directory outside the repository, with the exporter process read-traced: MCD / GNF / final-audit must equal phase 11 and the
    historical phase-3 handoff must never be opened. Needs the real Blender, fixtures and build."""
    def setUp(self):
        needed = [exporter.BLENDER, exporter.PACKER_SCRIPT, exporter.AUDIT, DESCRIPTOR, RELEASE_04]
        needed += [exporter.REPO / 'local/runs/claude' / run / 'ui_manifest.json' for run, _ in FIXTURES.values()]
        needed += [exporter.RUN_ROOT / ref / 'export.json' for _, ref in FIXTURES.values()]
        missing = [str(p) for p in needed if not p.exists()]
        if missing: self.skipTest('profile parity not runnable here; missing: ' + ', '.join(missing))
        try:
            import numpy  # noqa: F401
        except ImportError: self.skipTest('numpy missing for the audit interpreter')
        self.temp = tempfile.TemporaryDirectory(dir=exporter.REPO / 'local/runs/claude', prefix='export-profile-parity-'); self.addCleanup(self.temp.cleanup)
        self.cwd = tempfile.TemporaryDirectory(prefix='export-profile-cwd-'); self.addCleanup(self.cwd.cleanup)

    def test_turgeon_and_fedorov_match_phase_11_through_descriptor_and_release_04(self):
        root = Path(self.temp.name).resolve()
        for name, (run, ref) in FIXTURES.items():
            with self.subTest(fixture=name):
                reference = json.loads((exporter.RUN_ROOT / ref / 'export.json').read_text(encoding='utf-8'))
                expected = {Path(a['path']).name: a['sha256'] for a in reference['artifacts']}
                manifest = exporter.REPO / 'local/runs/claude' / run / 'ui_manifest.json'
                log = root / f'{name}-reads.json'
                cmd = [sys.executable, '-B', str(TRACE), str(manifest), '--out', str(root / f'{name}-export'), '--run-root', str(root),
                       '--export-profile', str(DESCRIPTOR), '--build-receipt', str(RELEASE_04)]
                result = subprocess.run(cmd, capture_output=True, text=True, cwd=self.cwd.name, env={**os.environ, 'READ_TRACE_LOG': str(log), 'PYTHONPATH': str(exporter.REPO)})
                self.assertEqual(result.returncode, 0, result.stdout[-3000:] + result.stderr[-3000:])
                receipt = json.loads((root / f'{name}-export' / 'export.json').read_text(encoding='utf-8'))
                self.assertEqual(receipt['status'], 'completed')
                got = {Path(a['path']).name: a['sha256'] for a in receipt['artifacts']}
                for file in ('aaron_ekblad.mcd', 'head_aaron_ekblad_cm.gnf', 'final-audit.json'):
                    self.assertEqual(got[file], expected[file], f'{name}: {file} differs from the demonstrated export')
                self.assertEqual(receipt['inputs']['export_profile']['profile_id'], 'nhl21-ps4-cusa18182-ekblad-export')
                self.assertEqual(receipt['inputs']['build']['executable']['sha256'], '55b1da01b7c350c9c1ece3176f6fc735860a263c90d4119e78e6ec8bf6a8fe1e')
                self.assertTrue(receipt['inputs']['build']['independent_review']['matched'])
                opened = [p for p in json.loads(log.read_text()) if 'nhl21-phase3-handoff' in p]
                self.assertEqual(opened, [], 'the historical phase-3 handoff was opened')


if __name__ == '__main__': unittest.main()
