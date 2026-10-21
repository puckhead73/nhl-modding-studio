"""Reproducible build of the NHL21 native packer executable used by the guarded export route (H2.4).

python -m src.application.native_build --tdb-rx2 DIR --out FRESH_DIR [--run-root DIR] [--cargo EXE] [--profile release|debug]
    [--expect SOURCE_MANIFEST.json] [--offline] [--snapshot-only] [--blender EXE] [--python EXE]

What it does, in order (nothing is written before step 3; every refusal before it leaves no directory behind):
 1. Capture the demonstrated route's source set - the converter-local files (native crate, `mcd_profile.rs`, the Blender-side packer
    and audit modules and the exporter modules they import) and the ONE local Rust dependency, Studio's `tdb-rx2` crate, given
    explicitly by `--tdb-rx2` (its Cargo.toml and every file under src/; examples/tests are not part of the library build). Each file
    is hashed (sha256); the dependency is recorded by CONTENT hash plus its git commit / dirty state, because a commit alone does not
    identify uncommitted contents. Workspace-inherited manifest fields (edition, rust-version, publish) are resolved from the
    dependency's workspace root, and that root manifest's hash, the inherited-field list and the resolved values are part of the
    source IDENTITY (they determine the generated dependency manifest) - absolute locations are not, so identical relocated inputs match.
 2. If `--expect` names a previously produced source manifest, every identity entry must equal the captured one (refuse otherwise).
    Only `nhl21-build-source-manifest/2` manifests are accepted; v1 manifests (H2.4, before the workspace inputs joined the identity)
    are refused with a message to re-capture - an older manifest is never silently accepted.
 3. Write an isolated snapshot beneath the fresh output - after a full re-capture has confirmed nothing changed since step 1 (files,
    lockfile, workspace manifest, resolved values; `--snapshot-only` included): converter files at their repo-relative paths (so
    `#[path = "../../../../src/adapters/mcd_profile.rs"]` still resolves) and the dependency at `deps/tdb-rx2/`, with generated STANDALONE
    manifests built from bytes re-verified against the capture: the crate's Cargo.toml points at the relative dependency path instead of
    the developer's drive path; the dependency's Cargo.toml carries the captured resolved workspace values and its own empty
    `[workspace]`. Cargo.lock is copied verbatim - the locked graph is the one built.
 4. `cargo build --locked [--offline] --profile P --target-dir <out>/target` on the snapshot. `--locked` refuses any lockfile change;
    a registry crate missing from the local cargo cache fails the build with cargo's own message (no substitution). Toolchain versions
    (`rustc -vV`, `cargo -vV`) are recorded; cargo enforces the crate's rust-version.
 5. After the build every captured input is re-captured and compared as in step 2 (drift, including a workspace change -> the receipt
    says so and the build is not accepted), the lockfile is
    compared to the source lock, the executable is hashed and copied to `<out>/nhl21-candidate-pack.exe`, and `cargo metadata` records
    each resolved crate with version, checksum, license and whether it is linked into the executable or build-time only.
Outputs (all beneath --out): source-manifest.json, dependencies.json, snapshot/, target/, cargo-build.log, build-receipt.json,
nhl21-candidate-pack.exe. Executable byte identity across builds is NOT claimed; the receipt records the hash so a claim can be
checked. Blender / Python / NumPy versions are recorded as the runtime the route needs, not bundled. Licensed game assets are never
part of this package. No installer, signing, CI or SBOM tooling. Exit codes: 0 built (or snapshot written with --snapshot-only),
2 refused before any write, 3 build or post-build check failed (directory kept with the receipt).
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

from src.application.native_paths import fresh_output_beneath

REPO = Path(__file__).resolve().parents[2]
CRATE_DIR = Path('experiments/codex/nhl21_package')
CRATE_NAME = 'nhl21-candidate-pack'
DEPENDENCY = 'tdb-rx2'
SNAPSHOT_DEP = Path('deps') / DEPENDENCY
EXE_NAME = 'nhl21-candidate-pack.exe' if os.name == 'nt' else 'nhl21-candidate-pack'
# The converter-local source set of the demonstrated export route (repo-relative). Rust: the crate and the module it includes by
# path. Python: the Blender-side packer + its imports, the offline audit + its imports, the guarded exporter + its imports.
ROUTE_SOURCES = {
    'native': ['experiments/codex/nhl21_package/Cargo.toml', 'experiments/codex/nhl21_package/Cargo.lock',
               'experiments/codex/nhl21_package/src/main.rs', 'src/adapters/mcd_profile.rs'],
    'packer_python': ['experiments/codex/nhl21_package/build.py', 'experiments/codex/nhl21_package/math_texture.py',
                      'experiments/codex/nhl21_package/shading_proof.py', 'experiments/codex/nhl21_package/layout_fields.py',
                      'experiments/codex/nhl21_package/reference_pack.py'],
    'audit_python': ['experiments/codex/nhl21_package/audit_candidate.py', 'src/conversion/geometry.py'],
    'exporter_python': ['src/application/export.py', 'src/application/lifecycle.py', 'src/application/native_paths.py',
                        'src/application/__init__.py', 'src/conversion/job.py', 'src/conversion/ui_manifest.py', 'src/conversion/__init__.py'],
}
SOURCE_MANIFEST_SCHEMA = 'nhl21-build-source-manifest/2'   # v2 (H2.4a): workspace-derived inputs are part of the identity; v1 manifests are refused
DEPENDENCIES_SCHEMA = 'nhl21-build-dependencies/1'
RECEIPT_SCHEMA = 'nhl21-build-receipt/1'
WORKSPACE_FIELDS = ('edition', 'rust-version', 'publish', 'license')
BLENDER = Path('C:/Program Files/Blender Foundation/Blender 4.5/blender.exe')
EXIT_CODES = {'built': 0, 'refused': 2, 'failed': 3}


class BuildRefused(ValueError):
    """An input problem found before anything was written (actionable message; no directory exists)."""


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def file_entry(path, base):
    path = Path(path)
    return {'path': path.relative_to(base).as_posix(), 'sha256': sha(path), 'bytes': path.stat().st_size}


def _toml_table(text, header):
    """Lines of one top-level TOML table (minimal parser: enough for Cargo manifests written by hand)."""
    lines, inside = [], False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith('['):
            inside = stripped == f'[{header}]'; continue
        if inside: lines.append(line)
    return lines


def _toml_value(lines, key):
    for line in lines:
        stripped = line.split('#', 1)[0].strip()
        if stripped.startswith(key) and stripped[len(key):].lstrip().startswith('='):
            return stripped.split('=', 1)[1].strip()
    return None


def git(cwd, *args):
    try:
        done = subprocess.run(['git', *args], cwd=str(cwd), capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return None, f'{type(exc).__name__}: {exc}'
    return (done.stdout.strip(), None) if done.returncode == 0 else (None, done.stderr.strip() or f'git exit {done.returncode}')


def run_logged(command, cwd, log_path=None, env=None, timeout=1800):
    """Run a tool; return (returncode, stdout, stderr). Output is appended to log_path when given."""
    done = subprocess.run([str(c) for c in command], cwd=str(cwd), capture_output=True, text=True, env=env, timeout=timeout,
                          encoding='utf-8', errors='replace')
    if log_path is not None:
        with Path(log_path).open('a', encoding='utf-8') as log:
            log.write(f'$ {" ".join(str(c) for c in command)}\n{done.stdout}{done.stderr}\n[exit {done.returncode}]\n\n')
    return done.returncode, done.stdout, done.stderr


# ---- capture -----------------------------------------------------------------------------------------------------------------
def capture_converter_sources(repo=REPO):
    files, missing = {}, []
    for group, paths in ROUTE_SOURCES.items():
        files[group] = []
        for rel in paths:
            path = repo / rel
            if not path.is_file(): missing.append(rel); continue
            files[group].append(file_entry(path, repo))
    if missing: raise BuildRefused('Converter source files of the demonstrated route are missing: ' + ', '.join(missing))
    return files


def capture_dependency(tdb_rx2):
    """The tdb-rx2 crate by content: manifest, every file under src/, workspace-inherited fields, git provenance."""
    crate = Path(tdb_rx2).resolve()
    manifest = crate / 'Cargo.toml'
    if not crate.is_dir(): raise BuildRefused(f'--tdb-rx2 must name the {DEPENDENCY} crate directory (Studio crates/{DEPENDENCY}); not a directory: {crate}')
    if not manifest.is_file(): raise BuildRefused(f'--tdb-rx2 directory has no Cargo.toml: {crate}')
    text = manifest.read_text(encoding='utf-8')
    package = _toml_table(text, 'package')
    name = (_toml_value(package, 'name') or '').strip('"')
    if name != DEPENDENCY: raise BuildRefused(f'--tdb-rx2 Cargo.toml declares package {name!r}, expected {DEPENDENCY!r}: {manifest}')
    if not (crate / 'src' / 'lib.rs').is_file(): raise BuildRefused(f'--tdb-rx2 crate has no src/lib.rs: {crate}')
    # Workspace-inherited fields: resolve from the nearest ancestor Cargo.toml with [workspace.package].
    inherited = [f for f in WORKSPACE_FIELDS if _toml_value(package, f'{f}.workspace') == 'true']
    resolved, workspace_root = {}, None
    if inherited:
        for ancestor in crate.parents:
            candidate = ancestor / 'Cargo.toml'
            if candidate.is_file() and _toml_table(candidate.read_text(encoding='utf-8'), 'workspace.package'):
                workspace_root = candidate; break
        if workspace_root is None:
            raise BuildRefused(f'{manifest} inherits {inherited} from a workspace but no ancestor Cargo.toml has [workspace.package]')
        table = _toml_table(workspace_root.read_text(encoding='utf-8'), 'workspace.package')
        for field in inherited:
            value = _toml_value(table, field)
            if value is None: raise BuildRefused(f'{workspace_root} [workspace.package] does not define {field}, inherited by {manifest}')
            resolved[field] = value
    for field in WORKSPACE_FIELDS:
        if field not in resolved and _toml_value(package, field) is not None: resolved[field] = _toml_value(package, field)
    sources = sorted(p for p in (crate / 'src').rglob('*') if p.is_file())
    files = [file_entry(manifest, crate)] + [file_entry(p, crate) for p in sources]
    head, head_error = git(crate, 'rev-parse', 'HEAD')
    status, status_error = git(crate, 'status', '--porcelain', '--untracked-files=all', '--', '.')
    dirty = sorted(line[3:] for line in status.splitlines()) if status is not None else None
    return {'name': DEPENDENCY, 'path': str(crate), 'manifest': str(manifest), 'package_version': (_toml_value(package, 'version') or '').strip('"'),
            'workspace_root': str(workspace_root) if workspace_root else None,
            'workspace_root_sha256': sha(workspace_root) if workspace_root else None,
            'resolved_package_fields': resolved, 'inherited_fields': inherited,
            'files': files, 'content_sha256': hashlib.sha256(''.join(f'{f["path"]}:{f["sha256"]}\n' for f in files).encode()).hexdigest(),
            'git': {'head': head, 'head_error': head_error, 'dirty_paths': dirty, 'status_error': status_error,
                    'note': 'contents are identified by the file hashes above; the commit alone does not identify uncommitted changes'},
            'excluded': ['examples/', 'tests/', 'benches/'], 'license': None,
            'license_note': 'NHL Modding Studio crate, local read-only input; no licence file found beside it - not redistributed by this package'}


def capture(tdb_rx2, repo=REPO):
    converter = capture_converter_sources(repo)
    dependency = capture_dependency(tdb_rx2)
    lock = repo / CRATE_DIR / 'Cargo.lock'
    manifest = {'schema': SOURCE_MANIFEST_SCHEMA, 'crate': CRATE_NAME, 'crate_dir': CRATE_DIR.as_posix(), 'repo': str(repo),
                'captured': time.strftime('%Y-%m-%dT%H:%M:%S'), 'converter_files': converter, 'lockfile_sha256': sha(lock),
                'dependencies': {DEPENDENCY: dependency}}
    manifest['content_sha256'] = manifest_content_hash(manifest)
    return manifest


def identity_index(manifest):
    """Every build-defining input as key -> value, independent of absolute locations: converter file hashes, dependency file hashes,
    the lockfile, and per dependency the workspace-derived inputs that feed its generated manifest - the workspace manifest hash (when
    fields are inherited), which fields are inherited, and the resolved values. Paths, git state and timestamps are provenance, not identity."""
    out = {f'converter:{f["path"]}': f['sha256'] for files in manifest.get('converter_files', {}).values() for f in files}
    out['Cargo.lock'] = manifest.get('lockfile_sha256')
    for name, dep in manifest.get('dependencies', {}).items():
        out.update({f'{name}:{f["path"]}': f['sha256'] for f in dep.get('files', [])})
        out[f'{name}:workspace_root_sha256'] = dep.get('workspace_root_sha256')
        out[f'{name}:inherited_fields'] = ','.join(dep.get('inherited_fields', []))
        for field, value in sorted(dep.get('resolved_package_fields', {}).items()): out[f'{name}:resolved:{field}'] = value
    return out


def manifest_content_hash(manifest):
    """Hash over the identity index (files, lockfile, workspace-derived inputs) - independent of timestamps and locations."""
    return hashlib.sha256('\n'.join(f'{k}:{v}' for k, v in sorted(identity_index(manifest).items())).encode()).hexdigest()


def compare_manifests(expected, actual):
    """Every declared identity entry must equal the captured one; extra captured entries are reported too. Returns the differences.
    An expected manifest of another schema (v1 lacked the workspace-derived entries) is refused outright: re-capture it with this tool."""
    if expected.get('schema') != SOURCE_MANIFEST_SCHEMA:
        return [f'expected manifest schema is {expected.get("schema")!r}, not {SOURCE_MANIFEST_SCHEMA!r} (older manifests lack the workspace-derived identity; re-capture with this tool)']
    diffs = []
    e, a = identity_index(expected), identity_index(actual)
    for key in sorted(set(e) | set(a)):
        short = lambda v: (v[:16] + '…') if isinstance(v, str) and len(v) == 64 else repr(v)
        if key not in a: diffs.append(f'{key}: declared but not captured')
        elif key not in e: diffs.append(f'{key}: captured but not declared')
        elif e[key] != a[key]: diffs.append(f'{key}: declared {short(e[key])} captured {short(a[key])}')
    return diffs


# ---- snapshot ----------------------------------------------------------------------------------------------------------------
def rewrite_crate_manifest(text, dep_relative):
    """The crate's Cargo.toml with the tdb-rx2 path dependency pointed at the snapshot's relative copy."""
    out, replaced = [], False
    for line in text.splitlines():
        stripped = line.split('#', 1)[0].strip()
        if stripped.startswith(DEPENDENCY) and stripped[len(DEPENDENCY):].lstrip().startswith('=') and 'path' in stripped:
            out.append(f'{DEPENDENCY} = {{ path = "{dep_relative}" }}   # snapshot: relative to this manifest (was a developer drive path)'); replaced = True
        else: out.append(line)
    if not replaced: raise BuildRefused(f'{CRATE_DIR / "Cargo.toml"} has no path dependency on {DEPENDENCY} to rewrite')
    return '\n'.join(out) + '\n'


def rewrite_dependency_manifest(text, resolved):
    """The dependency's Cargo.toml made standalone: inherited fields replaced by their resolved values, own [workspace] table."""
    out = []
    for line in text.splitlines():
        stripped = line.split('#', 1)[0].strip()
        field = next((f for f in WORKSPACE_FIELDS if stripped.startswith(f'{f}.workspace')), None)
        if field is not None: out.append(f'{field} = {resolved[field]}   # snapshot: resolved from the workspace root')
        else: out.append(line)
    if not any(l.strip() == '[workspace]' for l in out): out += ['', '[workspace]   # snapshot: standalone package, no enclosing workspace lookup']
    return '\n'.join(out) + '\n'


def verified_bytes(path, expected_sha256, what):
    """The bytes of `path` only if they still hash as captured; otherwise the input drifted since capture."""
    data = Path(path).read_bytes()
    if hashlib.sha256(data).hexdigest() != expected_sha256: raise RuntimeError(f'Input changed between capture and snapshot: {what}')
    return data


def write_snapshot(output, manifest, tdb_rx2, repo=REPO):
    """Copy the captured inputs beneath the output and generate the two standalone manifests - from bytes re-verified against the
    capture, never from a fresh unverified read - after a full re-capture has confirmed that no build-defining input (files, lockfile,
    workspace manifest, inherited fields, resolved values) changed since capture. Every copy is re-hashed afterwards as well."""
    drift = compare_manifests(manifest, capture(tdb_rx2, repo))
    if drift: raise RuntimeError('Inputs changed between capture and snapshot: ' + '; '.join(drift[:20]))
    snapshot = output / 'snapshot'; crate = Path(tdb_rx2).resolve(); dep = manifest['dependencies'][DEPENDENCY]; dep_dest = snapshot / SNAPSHOT_DEP
    generated = (CRATE_DIR / 'Cargo.toml').as_posix()   # the two rewritten manifests are generated from verified bytes, not copied
    for files in manifest['converter_files'].values():
        for entry in files:
            data = verified_bytes(repo / entry['path'], entry['sha256'], entry['path'])
            if entry['path'] == generated: crate_manifest_bytes = data; continue
            dest = snapshot / entry['path']; dest.parent.mkdir(parents=True, exist_ok=True); dest.write_bytes(data)
    for entry in dep['files']:
        data = verified_bytes(crate / entry['path'], entry['sha256'], f'{DEPENDENCY}/{entry["path"]}')
        if entry['path'] == 'Cargo.toml': dep_manifest_bytes = data; continue
        dest = dep_dest / entry['path']; dest.parent.mkdir(parents=True, exist_ok=True); dest.write_bytes(data)
    if dep['workspace_root'] is not None: verified_bytes(dep['workspace_root'], dep['workspace_root_sha256'], f'{DEPENDENCY} workspace root Cargo.toml')
    crate_manifest = snapshot / CRATE_DIR / 'Cargo.toml'
    dep_relative = os.path.relpath(dep_dest, crate_manifest.parent).replace(os.sep, '/')
    crate_manifest.write_text(rewrite_crate_manifest(crate_manifest_bytes.decode('utf-8'), dep_relative), encoding='utf-8')
    (dep_dest / 'Cargo.toml').write_text(rewrite_dependency_manifest(dep_manifest_bytes.decode('utf-8'), dep['resolved_package_fields']), encoding='utf-8')
    copies = [e['path'] for files in manifest['converter_files'].values() for e in files if e['path'] != generated and sha(snapshot / e['path']) != e['sha256']]
    copies += [f'{DEPENDENCY}/{e["path"]}' for e in dep['files'] if e['path'] != 'Cargo.toml' and sha(dep_dest / e['path']) != e['sha256']]
    if copies: raise RuntimeError('Snapshot copies do not hash as captured: ' + ', '.join(copies))
    return {'dir': str(snapshot), 'crate_manifest': str(crate_manifest), 'crate_manifest_sha256': sha(crate_manifest),
            'dependency_manifest': str(dep_dest / 'Cargo.toml'), 'dependency_manifest_sha256': sha(dep_dest / 'Cargo.toml'),
            'dependency_relative_path': dep_relative, 'lockfile_sha256': sha(snapshot / CRATE_DIR / 'Cargo.lock'),
            'generated_from': {'crate_manifest_source_sha256': hashlib.sha256(crate_manifest_bytes).hexdigest(),
                               'dependency_manifest_source_sha256': hashlib.sha256(dep_manifest_bytes).hexdigest(),
                               'workspace_root_sha256': dep['workspace_root_sha256'], 'resolved_package_fields': dep['resolved_package_fields']},
            'drift': []}


# ---- toolchain / runtime versions --------------------------------------------------------------------------------------------
def tool_version(command, cwd=REPO):
    try: code, out, err = run_logged(command, cwd, timeout=120)
    except (OSError, subprocess.TimeoutExpired) as exc: return {'command': [str(c) for c in command], 'error': f'{type(exc).__name__}: {exc}'}
    return {'command': [str(c) for c in command], 'output': (out or err).strip(), 'exit': code}


def runtime_versions(python, blender):
    info = {}
    if python is not None:
        info['python'] = tool_version([python, '-c', 'import sys, json; print(json.dumps({"executable": sys.executable, "version": sys.version}))'])
        info['numpy'] = tool_version([python, '-c', 'import numpy; print(numpy.__version__)'])
    else: info['python'] = info['numpy'] = {'note': 'not queried (--python not given)'}
    if blender is not None and Path(blender).is_file(): info['blender'] = tool_version([blender, '-b', '--version'])
    else: info['blender'] = {'note': f'not queried (missing or not given: {blender})'}
    return info


def rustflags(snapshot_dir, cargo_exe):
    """Strip build-root and cargo-home paths from the binary and ask the MSVC linker for a reproducible image (/Brepro: no
    timestamp). Measured effect: rebuilding at the same snapshot path yields a bit-identical executable; a different build root does
    not (cargo's -C metadata hash includes a path dependency's location, which changes symbol names and link order)."""
    cargo_home = Path(os.environ.get('CARGO_HOME') or Path.home() / '.cargo')
    flags = [f'--remap-path-prefix={snapshot_dir}=/nhl21-build', f'--remap-path-prefix={cargo_home}=/cargo']
    if os.name == 'nt': flags.append('-Clink-arg=/Brepro')
    return ' '.join(flags)


def resolve_cargo(cargo):
    path = shutil.which(str(cargo)) if cargo else shutil.which('cargo')
    if path is None or not Path(path).is_file():
        raise BuildRefused(f'cargo not found ({cargo or "cargo on PATH"}); install the Rust toolchain (rustup) or pass --cargo EXE')
    return Path(path).absolute()   # not resolve(): the rustup proxy `cargo.exe` must stay `cargo`, not become `rustup.exe`


# ---- dependency metadata (after the build, on the locked snapshot) ----------------------------------------------------------
def cargo_metadata(cargo, manifest_path, offline, log):
    command = [cargo, 'metadata', '--format-version', '1', '--locked', '--manifest-path', manifest_path] + (['--offline'] if offline else [])
    code, out, err = run_logged(command, manifest_path.parent, log)
    if code != 0: raise RuntimeError(f'cargo metadata failed (exit {code}): {err.strip()[-800:]}')
    return json.loads(out)


def dependency_report(metadata, lock_text):
    """Every resolved package with version / source / checksum / license and whether it is linked into the executable
    (reached from the crate through normal dependencies without entering a proc-macro crate) or build-time only."""
    packages = {p['id']: p for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    root = next(p['id'] for p in metadata['packages'] if p['name'] == CRATE_NAME)
    proc_macro = {pid for pid, p in packages.items() if any('proc-macro' in t['kind'] for t in p['targets'])}
    linked, stack = set(), [root]
    while stack:
        pid = stack.pop()
        if pid in linked: continue
        linked.add(pid)
        for dep in nodes[pid]['deps']:
            kinds = {k.get('kind') for k in dep['dep_kinds']}   # None = normal; 'build' / 'dev' are not linked
            if None in kinds and dep['pkg'] not in proc_macro and dep['pkg'] not in linked: stack.append(dep['pkg'])
    checksums = {}
    for block in lock_text.split('[[package]]')[1:]:
        fields = {l.split('=', 1)[0].strip(): l.split('=', 1)[1].strip().strip('"') for l in block.splitlines() if '=' in l and not l.startswith(' ')}
        if 'checksum' in fields: checksums[(fields['name'], fields['version'])] = fields['checksum']
    crates = []
    for pid in sorted(nodes, key=lambda i: (packages[i]['name'], packages[i]['version'])):
        p = packages[pid]
        crates.append({'name': p['name'], 'version': p['version'], 'source': p['source'], 'license': p['license'], 'license_file': p.get('license_file'),
                       'checksum': checksums.get((p['name'], p['version'])), 'proc_macro': pid in proc_macro,
                       'role': 'root' if pid == root else 'linked' if pid in linked else 'build-time only',
                       'redistributed': pid in linked and pid != root and p['source'] is not None})
    return crates


# ---- main flow -----------------------------------------------------------------------------------------------------------------
def build(out, tdb_rx2, run_root=None, cargo=None, profile='release', expect=None, offline=False, snapshot_only=False,
          blender=BLENDER, python=None, repo=REPO):
    """Returns the receipt dict; raises BuildRefused before any write, RuntimeError (with `.receipt`) after."""
    run_root = Path(run_root or repo / 'local/runs/claude').resolve()
    output = fresh_output_beneath(run_root, out)
    if profile not in ('release', 'debug'): raise BuildRefused(f'--profile must be release or debug, not {profile!r}')
    cargo_exe = None if snapshot_only else resolve_cargo(cargo)
    manifest = capture(tdb_rx2, repo)
    if expect is not None:
        expect = Path(expect)
        if not expect.is_file(): raise BuildRefused(f'--expect source manifest missing: {expect}')
        try: declared = json.loads(expect.read_text(encoding='utf-8'))
        except (OSError, ValueError) as exc: raise BuildRefused(f'--expect {expect} is not readable JSON: {exc}') from None
        diffs = compare_manifests(declared, manifest)
        if diffs: raise BuildRefused(f'Captured sources differ from the declared manifest {expect} ({len(diffs)}): ' + '; '.join(diffs[:20]))
        manifest['expected_manifest'] = {'path': str(expect), 'sha256': sha(expect), 'content_sha256': declared.get('content_sha256'), 'matched': True}
    # ---- from here on the output directory exists ----
    output.mkdir(parents=True, exist_ok=False)
    log = output / 'cargo-build.log'
    receipt = {'schema': RECEIPT_SCHEMA, 'status': 'running', 'crate': CRATE_NAME, 'profile': profile, 'output': str(output), 'run_root': str(run_root),
               'tool': {'path': str(Path(__file__).resolve()), 'sha256': sha(__file__)},
               'started': manifest['captured'], 'source_manifest': {'path': str(output / 'source-manifest.json'), 'content_sha256': manifest['content_sha256']},
               'inputs': {'tdb_rx2': manifest['dependencies'][DEPENDENCY]['path'], 'cargo': str(cargo_exe) if cargo_exe else None, 'expect': str(expect) if expect else None,
                          'offline': offline, 'snapshot_only': snapshot_only}, 'scope': 'fresh native packer build from captured sources; no installation, no game or Studio write'}
    def save(status=None):
        if status: receipt['status'] = status
        (output / 'build-receipt.json').write_text(json.dumps(receipt, indent=2), encoding='utf-8')
    try:
        snapshot = write_snapshot(output, manifest, tdb_rx2, repo); receipt['snapshot'] = snapshot
        manifest['snapshot'] = {k: v for k, v in snapshot.items() if k != 'drift'}
        (output / 'source-manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
        receipt['source_manifest']['sha256'] = sha(output / 'source-manifest.json')
        deps = {'schema': DEPENDENCIES_SCHEMA, 'local': {DEPENDENCY: {k: manifest['dependencies'][DEPENDENCY][k] for k in
                ('path', 'content_sha256', 'package_version', 'resolved_package_fields', 'git', 'license', 'license_note')}},
                'runtime': runtime_versions(python, blender), 'not_bundled': ['Blender', 'Python/NumPy', 'game assets (donor MCD, GNF, archives)', DEPENDENCY + ' source']}
        if snapshot_only:
            deps['toolchain'] = {'note': 'not queried (--snapshot-only)'}; deps['rust_crates'] = {'note': 'not resolved (--snapshot-only)'}
            (output / 'dependencies.json').write_text(json.dumps(deps, indent=2), encoding='utf-8')
            save('snapshot'); return receipt
        manifest_path = Path(snapshot['crate_manifest'])
        deps['toolchain'] = {'cargo': tool_version([cargo_exe, '-vV']), 'rustc': tool_version([cargo_exe.with_name('rustc' + cargo_exe.suffix), '-vV']),
                             'required_rust_version': manifest['dependencies'][DEPENDENCY]['resolved_package_fields'].get('rust-version')}
        command = [cargo_exe, 'build', '--locked', '--profile', 'dev' if profile == 'debug' else profile, '--manifest-path', manifest_path,
                   '--target-dir', output / 'target'] + (['--offline'] if offline else [])
        receipt['build'] = {'command': [str(c) for c in command], 'cwd': str(manifest_path.parent), 'log': str(log)}
        env = {**os.environ, 'CARGO_TERM_COLOR': 'never', 'CARGO_INCREMENTAL': '0', 'RUSTFLAGS': rustflags(snapshot['dir'], cargo_exe)}
        receipt['build']['rustflags'] = env['RUSTFLAGS']
        receipt['build']['determinism'] = ('build paths remapped and the MSVC linker asked for a reproducible image: a rebuild at the SAME snapshot path '
                                           'is bit-identical (measured), a build at another path is not (cargo hashes the path dependency location into '
                                           'symbol metadata); native OUTPUT parity is the reproducibility claim, executable identity is not')
        t = time.time(); code, out, err = run_logged(command, manifest_path.parent, log, env=env)
        receipt['build'].update({'exit': code, 'seconds': round(time.time() - t, 3)})
        if code != 0: raise RuntimeError(f'cargo build failed (exit {code}); see {log}. Last lines: {(err + out).strip()[-1200:]}')
        exe = output / 'target' / profile / EXE_NAME
        if not exe.is_file(): raise RuntimeError(f'cargo build succeeded but no executable at {exe}')
        # ---- post-build checks: drift (files, lockfile, workspace-derived inputs), snapshot lockfile, executable ----
        drift = compare_manifests(manifest, capture(tdb_rx2, repo))
        lock_after = sha(manifest_path.with_name('Cargo.lock'))
        if lock_after != manifest['lockfile_sha256']: drift.append('Cargo.lock: changed during the build despite --locked')
        receipt['drift_check'] = {'rehashed_after_build': True, 'differences': drift}
        if drift: raise RuntimeError('Inputs drifted during the build; the executable is not accepted: ' + '; '.join(drift[:20]))
        copy = output / EXE_NAME; shutil.copyfile(exe, copy)
        receipt['executable'] = {'path': str(copy), 'built_at': str(exe), 'sha256': sha(exe), 'bytes': exe.stat().st_size,
                                 'copy_sha256_matches': sha(copy) == sha(exe), 'byte_identity_across_builds': 'not claimed; compare sha256 of separate builds'}
        lock_text = manifest_path.with_name('Cargo.lock').read_text(encoding='utf-8')
        deps['rust_crates'] = dependency_report(cargo_metadata(cargo_exe, manifest_path, offline, log), lock_text)
        deps['lockfile'] = {'path': str(manifest_path.with_name('Cargo.lock')), 'sha256': lock_after, 'locked': True}
        (output / 'dependencies.json').write_text(json.dumps(deps, indent=2), encoding='utf-8')
        receipt['dependencies'] = {'path': str(output / 'dependencies.json'), 'sha256': sha(output / 'dependencies.json'),
                                   'linked_registry_crates': sorted(f'{c["name"]} {c["version"]} ({c["license"]})' for c in deps['rust_crates'] if c['redistributed'])}
        receipt['finished'] = time.strftime('%Y-%m-%dT%H:%M:%S'); save('built')
        return receipt
    except (RuntimeError, OSError, ValueError, subprocess.TimeoutExpired) as exc:
        receipt['error'] = f'{type(exc).__name__}: {exc}'; save('failed')
        wrapped = RuntimeError(str(exc)); wrapped.receipt = receipt; raise wrapped from exc


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--tdb-rx2', type=Path, required=True, help=f'directory of the {DEPENDENCY} crate (Studio crates/{DEPENDENCY}); read only')
    parser.add_argument('--out', type=Path, required=True, help='fresh build directory (must not exist) beneath --run-root')
    parser.add_argument('--run-root', type=Path, default=REPO / 'local/runs/claude', help='existing directory --out must lie beneath')
    parser.add_argument('--cargo', type=Path, default=None, help='cargo executable (default: cargo on PATH)')
    parser.add_argument('--profile', default='release', choices=('release', 'debug'))
    parser.add_argument('--expect', type=Path, default=None, help='source-manifest.json of an earlier build; every hash must match or the build is refused')
    parser.add_argument('--offline', action='store_true', help='pass --offline to cargo (registry crates must already be in the local cargo cache)')
    parser.add_argument('--snapshot-only', action='store_true', help='capture, verify and snapshot only; no cargo invocation')
    parser.add_argument('--blender', type=Path, default=BLENDER, help='Blender executable whose version is recorded as the route runtime')
    parser.add_argument('--python', type=Path, default=Path(sys.executable), help='interpreter whose version and NumPy are recorded as the audit runtime')
    args = parser.parse_args(argv)
    try:
        receipt = build(args.out, args.tdb_rx2, run_root=args.run_root, cargo=args.cargo, profile=args.profile, expect=args.expect, offline=args.offline,
                        snapshot_only=args.snapshot_only, blender=args.blender, python=args.python)
    except (BuildRefused, ValueError) as exc:
        print(json.dumps({'status': 'refused', 'error': str(exc)})); return EXIT_CODES['refused']
    except RuntimeError as exc:
        receipt = getattr(exc, 'receipt', {})
        print(json.dumps({'status': 'failed', 'error': str(exc), 'receipt': str(Path(receipt.get('output', args.out)) / 'build-receipt.json')})); return EXIT_CODES['failed']
    print(json.dumps({'status': receipt['status'], 'executable': receipt.get('executable', {}).get('sha256'), 'receipt': str(Path(receipt['output']) / 'build-receipt.json')}))
    return EXIT_CODES['built']


if __name__ == '__main__':
    sys.exit(main())
