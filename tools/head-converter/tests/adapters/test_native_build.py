"""H2.4 build/input contract of `src.application.native_build`: refusals before any write (missing / wrong dependency, missing cargo,
existing output, declared-hash mismatch), the isolated snapshot (layout, rewritten standalone manifests, verbatim lockfile, content
hashes), drift detection, workspace-derived identity (H2.4a: `--expect` refusals on workspace-only changes, mutation between capture and
snapshot and during a stubbed build, relocated identical inputs, v1 manifests refused), exit codes - all without cargo - plus one real `cargo build` when the toolchain and the Studio crate are
present (executable runs, receipt/dependency report complete, declared manifest accepted). Accepted outputs are never touched."""
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from src.application import native_build as nb

STUDIO_TDB_RX2 = Path(os.environ.get('NHL21_TDB_RX2', 'E:/Repositories/nhl-modding-studio/crates/tdb-rx2'))


def fake_dependency(root, name='tdb-rx2', inherit=True, lib=True):
    """A stand-in Studio layout: workspace root with [workspace.package] and the crate inheriting from it."""
    studio = root / 'studio'; crate = studio / 'crates' / name
    (crate / 'src').mkdir(parents=True); (crate / 'examples').mkdir()
    (studio / 'Cargo.toml').write_text('[workspace]\nmembers = ["crates/tdb-rx2"]\n\n[workspace.package]\nedition = "2021"\nrust-version = "1.96"\npublish = false\n', encoding='utf-8')
    fields = 'edition.workspace = true\nrust-version.workspace = true\npublish.workspace = true\n' if inherit else 'edition = "2021"\n'
    (crate / 'Cargo.toml').write_text(f'[package]\nname = "{name}"\nversion = "0.0.0"\n{fields}\n[dependencies]\nserde_json = "1"\n', encoding='utf-8')
    if lib: (crate / 'src' / 'lib.rs').write_text('pub mod gnf;\n', encoding='utf-8')
    (crate / 'src' / 'gnf.rs').write_text('pub fn parse() {}\n', encoding='utf-8')
    (crate / 'examples' / 'probe.rs').write_text('fn main() {}\n', encoding='utf-8')
    return crate


class BuildContractTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=nb.REPO / 'local/runs/claude', prefix='native-build-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.dep = fake_dependency(self.root)

    def build(self, name, **kw):
        return nb.build(self.root / name, kw.pop('tdb_rx2', self.dep), run_root=self.root, snapshot_only=True, python=None, blender=None, **kw)

    def assert_refused(self, name, regex, **kw):
        with self.assertRaisesRegex(ValueError, regex):
            self.build(name, **kw)
        self.assertFalse((self.root / name).exists(), 'a refusal writes nothing')

    # ---- refusals before any write ---------------------------------------------------------------------------------
    def test_missing_or_wrong_dependency_is_refused_without_writing(self):
        self.assert_refused('missing', '--tdb-rx2 must name the tdb-rx2 crate', tdb_rx2=self.root / 'absent')
        (self.root / 'empty-dep').mkdir(); self.assert_refused('out-empty', 'has no Cargo.toml', tdb_rx2=self.root / 'empty-dep')
        other = fake_dependency(self.root / 'other', name='tdb-core'); self.assert_refused('out-wrong-name', "declares package 'tdb-core'", tdb_rx2=other)
        nolib = fake_dependency(self.root / 'nolib', lib=False); self.assert_refused('out-no-lib', 'has no src/lib.rs', tdb_rx2=nolib)

    def test_workspace_inheritance_without_a_workspace_root_is_refused(self):
        orphan = self.root / 'orphan' / 'tdb-rx2'; shutil.copytree(self.dep, orphan)   # no ancestor Cargo.toml with [workspace.package]
        self.assert_refused('out-orphan', r'inherits .* but no ancestor Cargo.toml', tdb_rx2=orphan)

    def test_existing_output_and_foreign_run_root_are_refused(self):
        (self.root / 'taken').mkdir(); marker = self.root / 'taken' / 'keep.txt'; marker.write_text('accepted output stays', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'requires a fresh directory'):
            self.build('taken')
        self.assertEqual(sorted(p.name for p in (self.root / 'taken').iterdir()), ['keep.txt'], 'the existing directory is untouched')
        with self.assertRaisesRegex(ValueError, 'must lie beneath the run root'):
            nb.build(nb.REPO / 'local/runs/claude/elsewhere', self.dep, run_root=self.root, snapshot_only=True)

    def test_missing_cargo_is_refused_before_any_write(self):
        with self.assertRaisesRegex(ValueError, 'cargo not found'):
            nb.build(self.root / 'no-cargo', self.dep, run_root=self.root, cargo=self.root / 'no-such-cargo.exe')
        self.assertFalse((self.root / 'no-cargo').exists())

    def test_declared_hash_mismatch_is_refused_before_any_write(self):
        first = self.build('declared'); manifest = json.loads((self.root / 'declared' / 'source-manifest.json').read_text(encoding='utf-8'))
        self.assertEqual(first['status'], 'snapshot')
        tampered = json.loads(json.dumps(manifest)); tampered['converter_files']['native'][2]['sha256'] = '0' * 64   # src/main.rs
        (self.root / 'tampered.json').write_text(json.dumps(tampered), encoding='utf-8')
        self.assert_refused('mismatch', r'differ from the declared manifest .*main\.rs: declared 0000', expect=self.root / 'tampered.json')
        tampered = json.loads(json.dumps(manifest)); tampered['dependencies']['tdb-rx2']['files'][1]['sha256'] = 'f' * 64
        (self.root / 'tampered-dep.json').write_text(json.dumps(tampered), encoding='utf-8')
        self.assert_refused('mismatch-dep', r'tdb-rx2:src/gnf\.rs: declared ffff', expect=self.root / 'tampered-dep.json')
        tampered = json.loads(json.dumps(manifest)); tampered['converter_files']['native'].append({'path': 'src/adapters/never.rs', 'sha256': 'a' * 64, 'bytes': 1})
        (self.root / 'tampered-extra.json').write_text(json.dumps(tampered), encoding='utf-8')
        self.assert_refused('mismatch-extra', 'never.rs: declared but not captured', expect=self.root / 'tampered-extra.json')
        self.assert_refused('no-expect', 'source manifest missing', expect=self.root / 'absent.json')
        (self.root / 'schema.json').write_text('{"schema": "other/1"}', encoding='utf-8'); self.assert_refused('schema', 'schema is', expect=self.root / 'schema.json')
        # the unmodified manifest is accepted and recorded
        ok = self.build('declared-ok', expect=self.root / 'declared' / 'source-manifest.json')
        again = json.loads((self.root / 'declared-ok' / 'source-manifest.json').read_text(encoding='utf-8'))
        self.assertEqual((again['expected_manifest']['matched'], again['content_sha256']), (True, manifest['content_sha256']))

    # ---- snapshot ---------------------------------------------------------------------------------------------------
    def test_snapshot_is_isolated_with_standalone_manifests_and_matching_hashes(self):
        receipt = self.build('snap'); out = self.root / 'snap'; snapshot = out / 'snapshot'
        self.assertEqual(receipt['status'], 'snapshot'); self.assertEqual(receipt['snapshot']['drift'], [])
        manifest = json.loads((out / 'source-manifest.json').read_text(encoding='utf-8'))
        self.assertEqual(manifest['schema'], nb.SOURCE_MANIFEST_SCHEMA); self.assertEqual(receipt['snapshot']['generated_from']['resolved_package_fields']['edition'], '"2021"')
        # every converter file of the route is captured at its repo-relative path with the hash of the real file, and copied
        for group, rels in nb.ROUTE_SOURCES.items():
            self.assertEqual([f['path'] for f in manifest['converter_files'][group]], rels)
            for entry in manifest['converter_files'][group]:
                self.assertEqual(entry['sha256'], nb.sha(nb.REPO / entry['path']), entry['path'])
                if entry['path'] != 'experiments/codex/nhl21_package/Cargo.toml': self.assertEqual(nb.sha(snapshot / entry['path']), entry['sha256'])
        # the dependency: manifest + src/** only (examples excluded), content hash, resolved workspace fields, git provenance recorded
        dep = manifest['dependencies']['tdb-rx2']
        self.assertEqual([f['path'] for f in dep['files']], ['Cargo.toml', 'src/gnf.rs', 'src/lib.rs'])
        self.assertEqual(dep['resolved_package_fields'], {'edition': '"2021"', 'rust-version': '"1.96"', 'publish': 'false'})
        self.assertEqual(dep['inherited_fields'], ['edition', 'rust-version', 'publish']); self.assertEqual(Path(dep['workspace_root']), self.root / 'studio' / 'Cargo.toml')
        self.assertEqual(set(dep['git']), {'head', 'head_error', 'dirty_paths', 'status_error', 'note'}); self.assertIsNone(dep['license'])
        self.assertFalse((snapshot / 'deps/tdb-rx2/examples').exists())
        self.assertEqual(nb.sha(snapshot / 'deps/tdb-rx2/src/gnf.rs'), dep['files'][1]['sha256'])
        # generated standalone manifests: relative dependency path, resolved inherited fields, own [workspace]
        crate_toml = (snapshot / 'experiments/codex/nhl21_package/Cargo.toml').read_text(encoding='utf-8')
        self.assertIn('tdb-rx2 = { path = "../../../deps/tdb-rx2" }', crate_toml); self.assertNotIn('E:/Repositories', crate_toml)
        self.assertEqual((snapshot / 'experiments/codex/nhl21_package/Cargo.toml').parent.joinpath('../../../deps/tdb-rx2').resolve(), (snapshot / 'deps/tdb-rx2').resolve())
        dep_toml = (snapshot / 'deps/tdb-rx2/Cargo.toml').read_text(encoding='utf-8')
        self.assertNotIn('.workspace = true', dep_toml)
        for line in ('edition = "2021"', 'rust-version = "1.96"', 'publish = false', '[workspace]'): self.assertIn(line, dep_toml)
        # lockfile verbatim; receipts consistent
        self.assertEqual(nb.sha(snapshot / 'experiments/codex/nhl21_package/Cargo.lock'), nb.sha(nb.REPO / 'experiments/codex/nhl21_package/Cargo.lock'))
        self.assertEqual(manifest['lockfile_sha256'], receipt['snapshot']['lockfile_sha256'])
        self.assertEqual(manifest['content_sha256'], nb.manifest_content_hash(manifest))
        deps = json.loads((out / 'dependencies.json').read_text(encoding='utf-8'))
        self.assertEqual(deps['local']['tdb-rx2']['content_sha256'], dep['content_sha256']); self.assertIn('Blender', deps['not_bundled'])
        self.assertIsNone(receipt.get('executable')); self.assertFalse((out / 'target').exists())
        self.assertEqual(receipt['tool']['sha256'], nb.sha(nb.__file__))

    def test_non_inherited_dependency_manifest_is_copied_with_only_a_workspace_table_added(self):
        plain = fake_dependency(self.root / 'plain-dep', inherit=False)
        self.build('plain', tdb_rx2=plain)
        manifest = json.loads((self.root / 'plain' / 'source-manifest.json').read_text(encoding='utf-8'))['dependencies']['tdb-rx2']
        self.assertEqual((manifest['inherited_fields'], manifest['workspace_root']), ([], None))
        text = (self.root / 'plain/snapshot/deps/tdb-rx2/Cargo.toml').read_text(encoding='utf-8')
        self.assertIn('edition = "2021"\n', text); self.assertTrue(text.rstrip().endswith('[workspace]   # snapshot: standalone package, no enclosing workspace lookup'))

    def mutate_workspace(self, field='edition', value='"2024"'):
        root = self.dep.parents[1] / 'Cargo.toml'; text = root.read_text(encoding='utf-8')   # the current fixture's workspace root
        root.write_text(re.sub(rf'^{field} = .*$', f'{field} = {value}', text, flags=re.M), encoding='utf-8')

    def test_source_change_between_capture_and_snapshot_fails_with_receipt(self):
        """A disposable dependency file (a) or its workspace manifest (b) changes after capture: the snapshot must not be written from
        the changed inputs; the receipt says failed and nothing claims matched/built."""
        for name, mutate, regex in (('drift-file', lambda: (self.dep / 'src' / 'gnf.rs').write_text('pub fn parse() { changed() }\n', encoding='utf-8'),
                                     r'Inputs changed between capture and snapshot: tdb-rx2:src/gnf\.rs: declared'),
                                    ('drift-workspace', lambda: self.mutate_workspace(), r'tdb-rx2:resolved:edition: declared \'"2021"\' captured \'"2024"\'')):
            original = nb.capture; calls = []
            def once(tdb_rx2, repo=nb.REPO):
                manifest = original(tdb_rx2, repo)
                if not calls: calls.append(1); mutate()   # the world changes right after the first capture
                return manifest
            with patch.object(nb, 'capture', once):
                with self.assertRaisesRegex(RuntimeError, regex) as ctx:
                    self.build(name)
            receipt = json.loads((self.root / name / 'build-receipt.json').read_text(encoding='utf-8'))
            self.assertEqual((receipt['status'], ctx.exception.receipt['status']), ('failed', 'failed'))
            self.assertFalse((self.root / name / 'snapshot').exists(), 'nothing snapshotted from changed inputs')
            self.assertFalse((self.root / name / 'source-manifest.json').exists()); self.assertNotIn('executable', receipt)
            self.mutate_workspace('edition', '"2021"'); self.dep = fake_dependency(self.root / f'fresh-{name}')   # clean fixture for the next case

    # ---- H2.4a: workspace-derived inputs are part of the identity -----------------------------------------------------
    def test_workspace_only_changes_are_refused_by_expect_without_output(self):
        self.build('declared'); declared = self.root / 'declared' / 'source-manifest.json'
        manifest = json.loads(declared.read_text(encoding='utf-8'))
        self.assertEqual(manifest['schema'], 'nhl21-build-source-manifest/2')
        index = nb.identity_index(manifest)
        self.assertEqual((index['tdb-rx2:resolved:edition'], index['tdb-rx2:inherited_fields'], index['tdb-rx2:workspace_root_sha256']),
                         ('"2021"', 'edition,rust-version,publish', nb.sha(self.root / 'studio' / 'Cargo.toml')))
        self.mutate_workspace('edition', '"2024"')
        self.assert_refused('edition', r'tdb-rx2:resolved:edition: declared \'"2021"\' captured \'"2024"\'', expect=declared)
        self.mutate_workspace('edition', '"2021"'); self.mutate_workspace('rust-version', '"1.97"')
        self.assert_refused('rust-version', r'tdb-rx2:resolved:rust-version: declared \'"1.96"\' captured \'"1.97"\'', expect=declared)
        self.mutate_workspace('rust-version', '"1.96"')
        # a comment-only change to the workspace manifest is still a different workspace input (its bytes are part of the identity)
        (self.root / 'studio' / 'Cargo.toml').write_text((self.root / 'studio' / 'Cargo.toml').read_text(encoding='utf-8') + '# touched\n', encoding='utf-8')
        self.assert_refused('touched', 'tdb-rx2:workspace_root_sha256: declared', expect=declared)
        # the crate declaring the field itself instead of inheriting it (same value) is a different build input too
        crate_toml = self.dep / 'Cargo.toml'; crate_toml.write_text(crate_toml.read_text(encoding='utf-8').replace('edition.workspace = true', 'edition = "2021"'), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, r'tdb-rx2:inherited_fields: declared \'edition,rust-version,publish\' captured \'rust-version,publish\''):
            self.build('declares', expect=declared)
        self.assertFalse((self.root / 'declares').exists())

    def test_relocated_identical_inputs_keep_their_identity(self):
        self.build('declared'); declared = self.root / 'declared' / 'source-manifest.json'
        moved = self.root / 'elsewhere' / 'deeper'; shutil.copytree(self.root / 'studio', moved / 'studio')
        receipt = self.build('relocated', tdb_rx2=moved / 'studio' / 'crates' / 'tdb-rx2', expect=declared)
        again = json.loads((self.root / 'relocated' / 'source-manifest.json').read_text(encoding='utf-8')); before = json.loads(declared.read_text(encoding='utf-8'))
        self.assertEqual(receipt['status'], 'snapshot'); self.assertTrue(again['expected_manifest']['matched'])
        self.assertEqual(again['content_sha256'], before['content_sha256'])
        self.assertNotEqual(again['dependencies']['tdb-rx2']['path'], before['dependencies']['tdb-rx2']['path'])
        self.assertEqual(again['snapshot']['dependency_manifest_sha256'], before['snapshot']['dependency_manifest_sha256'], 'same generated manifest')

    def test_v1_expected_manifest_is_refused_not_silently_accepted(self):
        self.build('declared'); manifest = json.loads((self.root / 'declared' / 'source-manifest.json').read_text(encoding='utf-8'))
        v1 = json.loads(json.dumps(manifest)); v1['schema'] = 'nhl21-build-source-manifest/1'
        for dep in v1['dependencies'].values():
            for key in ('workspace_root_sha256', 'inherited_fields', 'resolved_package_fields'): dep.pop(key, None)
        (self.root / 'v1.json').write_text(json.dumps(v1), encoding='utf-8')
        self.assert_refused('v1', r"schema is 'nhl21-build-source-manifest/1', not 'nhl21-build-source-manifest/2' \(older manifests lack", expect=self.root / 'v1.json')

    def test_workspace_mutation_during_stubbed_build_is_not_reported_built(self):
        """The cargo step is stubbed: it mutates the workspace manifest and drops a fake executable. The post-build drift check must
        refuse to accept the executable, whatever cargo produced."""
        fake_cargo = self.root / 'bin' / 'cargo.exe'; fake_cargo.parent.mkdir(); fake_cargo.write_bytes(b'stub')
        original = nb.run_logged
        def stub(command, cwd, log_path=None, env=None, timeout=1800):
            if [str(c) for c in command][1:2] == ['build']:
                self.mutate_workspace('edition', '"2024"')
                exe = Path(command[command.index('--target-dir') + 1]) / 'release' / nb.EXE_NAME; exe.parent.mkdir(parents=True); exe.write_bytes(b'MZ stub')
                return 0, 'stub build ok', ''
            return original(command, cwd, log_path, env, timeout)
        with patch.object(nb, 'run_logged', stub), patch.object(nb, 'tool_version', return_value={'output': 'stub'}), \
             patch.object(nb, 'cargo_metadata', side_effect=AssertionError('metadata must not run after drift')):
            with self.assertRaisesRegex(RuntimeError, r'Inputs drifted during the build; the executable is not accepted: tdb-rx2:resolved:edition'):
                nb.build(self.root / 'stubbed', self.dep, run_root=self.root, cargo=fake_cargo, python=None, blender=None)
        receipt = json.loads((self.root / 'stubbed' / 'build-receipt.json').read_text(encoding='utf-8'))
        self.assertEqual((receipt['status'], receipt['build']['exit']), ('failed', 0)); self.assertNotIn('executable', receipt)
        self.assertIn('tdb-rx2:workspace_root_sha256', receipt['error']); self.assertFalse((self.root / 'stubbed' / nb.EXE_NAME).exists())
        self.assertFalse((self.root / 'stubbed' / 'dependencies.json').exists())

    # ---- CLI --------------------------------------------------------------------------------------------------------
    def test_cli_exit_codes(self):
        self.assertEqual(nb.EXIT_CODES, {'built': 0, 'refused': 2, 'failed': 3})
        argv = ['--tdb-rx2', str(self.dep), '--run-root', str(self.root), '--snapshot-only']
        with patch('builtins.print') as out:
            self.assertEqual(nb.main(argv + ['--out', str(self.root / 'cli')]), 0)
            self.assertEqual(json.loads(out.call_args[0][0])['status'], 'snapshot')
            self.assertEqual(nb.main(argv + ['--out', str(self.root / 'cli')]), 2)   # exists now
            self.assertEqual(json.loads(out.call_args[0][0])['status'], 'refused')
            self.assertEqual(nb.main(['--tdb-rx2', str(self.root / 'absent'), '--run-root', str(self.root), '--out', str(self.root / 'cli2'), '--snapshot-only']), 2)
        self.assertFalse((self.root / 'cli2').exists())

    def test_dependency_report_classifies_linked_and_build_time_crates(self):
        meta = {'packages': [
            {'id': 'root', 'name': nb.CRATE_NAME, 'version': '0.1.0', 'source': None, 'license': None, 'targets': [{'kind': ['bin']}]},
            {'id': 'lib', 'name': 'serde', 'version': '1.0.0', 'source': 'registry+x', 'license': 'MIT OR Apache-2.0', 'targets': [{'kind': ['lib']}]},
            {'id': 'pm', 'name': 'serde_derive', 'version': '1.0.0', 'source': 'registry+x', 'license': 'MIT', 'targets': [{'kind': ['proc-macro']}]},
            {'id': 'syn', 'name': 'syn', 'version': '2.0.0', 'source': 'registry+x', 'license': 'MIT', 'targets': [{'kind': ['lib']}]},
            {'id': 'vc', 'name': 'version_check', 'version': '0.9.0', 'source': 'registry+x', 'license': 'MIT', 'targets': [{'kind': ['lib']}]}],
            'resolve': {'nodes': [
                {'id': 'root', 'deps': [{'pkg': 'lib', 'dep_kinds': [{'kind': None}]}]},
                {'id': 'lib', 'deps': [{'pkg': 'pm', 'dep_kinds': [{'kind': None}]}, {'pkg': 'vc', 'dep_kinds': [{'kind': 'build'}]}]},
                {'id': 'pm', 'deps': [{'pkg': 'syn', 'dep_kinds': [{'kind': None}]}]}, {'id': 'syn', 'deps': []}, {'id': 'vc', 'deps': []}]}}
        lock = '[[package]]\nname = "serde"\nversion = "1.0.0"\nchecksum = "abc"\n\n[[package]]\nname = "syn"\nversion = "2.0.0"\nchecksum = "def"\n'
        report = {c['name']: c for c in nb.dependency_report(meta, lock)}
        self.assertEqual({n: (c['role'], c['redistributed']) for n, c in report.items()},
                         {nb.CRATE_NAME: ('root', False), 'serde': ('linked', True), 'serde_derive': ('build-time only', False),
                          'syn': ('build-time only', False), 'version_check': ('build-time only', False)})
        self.assertEqual((report['serde']['checksum'], report['syn']['checksum'], report['serde_derive']['proc_macro']), ('abc', 'def', True))


@unittest.skipUnless(shutil.which('cargo') and STUDIO_TDB_RX2.is_dir(), 'real build needs cargo on PATH and the Studio tdb-rx2 crate (NHL21_TDB_RX2)')
class RealBuildTests(unittest.TestCase):
    """One real locked build from captured sources into a fresh directory (~20 s): the executable exists, runs, and the receipts
    are complete; the produced source manifest is then accepted as `--expect` for a snapshot-only capture."""
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=nb.REPO / 'local/runs/claude', prefix='native-build-real-')
        self.addCleanup(self.temp.cleanup); self.root = Path(self.temp.name).resolve()

    def test_locked_build_from_snapshot_produces_a_running_executable_and_complete_receipts(self):
        out = self.root / 'build'
        receipt = nb.build(out, STUDIO_TDB_RX2, run_root=self.root, python=None, blender=None)
        self.assertEqual(receipt['status'], 'built', receipt.get('error'))
        exe = Path(receipt['executable']['path']); self.assertTrue(exe.is_file()); self.assertEqual(nb.sha(exe), receipt['executable']['sha256'])
        self.assertEqual(receipt['drift_check']['differences'], []); self.assertEqual(receipt['build']['exit'], 0)
        self.assertIn('--locked', receipt['build']['command']); self.assertIn('/Brepro', receipt['build']['rustflags'])
        usage = subprocess.run([str(exe)], capture_output=True, text=True)
        self.assertNotEqual(usage.returncode, 0); self.assertIn('export donor profile output | pack donor profile input output', usage.stderr + usage.stdout)
        deps = json.loads((out / 'dependencies.json').read_text(encoding='utf-8'))
        self.assertEqual(deps['lockfile']['sha256'], nb.sha(nb.REPO / 'experiments/codex/nhl21_package/Cargo.lock'))
        crates = {c['name']: c for c in deps['rust_crates']}
        self.assertEqual((crates['tdb-rx2']['role'], crates['tdb-rx2']['source']), ('linked', None))
        self.assertTrue(all(c['license'] and c['checksum'] for c in crates.values() if c['redistributed']), 'every redistributed crate has a licence and lock checksum')
        self.assertEqual(crates['serde_derive']['role'], 'build-time only'); self.assertIn('rustc', deps['toolchain']['rustc']['output'])
        self.assertTrue(receipt['dependencies']['linked_registry_crates'])
        again = nb.build(self.root / 'again', STUDIO_TDB_RX2, run_root=self.root, expect=out / 'source-manifest.json', snapshot_only=True, python=None, blender=None)
        self.assertEqual(again['status'], 'snapshot')
        self.assertTrue(json.loads((self.root / 'again' / 'source-manifest.json').read_text(encoding='utf-8'))['expected_manifest']['matched'])


if __name__ == '__main__': unittest.main()
