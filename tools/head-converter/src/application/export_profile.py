"""Versioned application export descriptor (`nhl-head-export-profile/1`) and receipted native-build selection for the guarded
NHL21 exporter (H2.5). stdlib only; every function validates and writes nothing.

Descriptor. One JSON file describing ONE demonstrated route (NHL21 PS4 CUSA18182, Ekblad donor, CC5 head-only FBX source, donor_fit
strategy). It references - never embeds - the immutable native-adapter inputs the packer consumes (`native_inputs`: exchange mesh,
donor MCD, `nhl-mcd-profile/1` profile, diffuse GNF) and the measured import preconditions, each by relative path + sha256. It is an
application-facing descriptor over the existing narrow native profile, not an implementation of the proposed canonical
`nhl-head-profile/1`; the mapping and the gaps are documented in research/claude/contract-proposals.md (section J).

Reference resolution. Relative paths (POSIX separators) resolve against the descriptor's own directory, or against the explicit
`inputs_root` binding when the caller gives one; absolute paths are refused so a descriptor never carries a developer drive layout.
The caller's working directory is never used. Identity comes from the descriptor's content (`profile_id`, `version`, the native
profile's own `id`), never from a file name.

Consistency rules enforced by `load_descriptor` (all before any export write): exact schema; the four native inputs and the
preconditions exist and hash as declared; the referenced native profile is `nhl-mcd-profile/1`, its `id` equals the declared native
adapter id, its donor / diffuse hashes equal the referenced donor / texture, its title / platform / title ID / build attribution equal
the descriptor's; the descriptor's `capabilities` equal the native profile's (a descriptor cannot claim more than the adapter
records); the exchange mesh names the same profile id and donor hash; declared output names are plain file names.

Build selection. `select_build_receipt` accepts exactly one `nhl21-build-receipt/1` written by src/application/native_build.py:
status `built`, an empty drift report, the sibling `source-manifest.json` present with the receipt's recorded sha256, schema
`nhl21-build-source-manifest/2` and a content hash that recomputes from its identity index, and the sibling executable present with
the receipt's recorded sha256 and size. Files are resolved as siblings of the receipt (by the recorded file names), so a relocated
build directory still resolves and nothing is scanned or guessed; an invalid selection refuses - it never falls back to another
executable. A validated receipt is provenance (what was built, from which captured sources): it is not a signature and not an
independent review endorsement; whether the executable hash appears in a descriptor's `evidence.native_build.reviewed_executables`
list is reported separately as `independent_review`.
"""
import hashlib
import json
from pathlib import Path, PurePosixPath
import re

from src.application import native_build
from src.application.native_paths import existing_file

DESCRIPTOR_SCHEMA = 'nhl-head-export-profile/1'
NATIVE_PROFILE_SCHEMA = 'nhl-mcd-profile/1'
WRITER_EXCHANGE_SCHEMA = 'nhl21-phase3-handoff/1'
MESH_EXCHANGE_SCHEMA = 'native-fit-exchange/1'
NATIVE_INPUT_KEYS = ('exchange', 'native_donor', 'profile', 'texture')
SUPPORTED_ROUTES = (('cc5_headshot_head_only_fbx', 'donor_fit'),)
PROFILE_ID_RE = re.compile(r'^[a-z0-9][a-z0-9-]{2,79}$')
SHA_RE = re.compile(r'^[0-9a-f]{64}$')
REQUIRED_TOP = ('schema', 'profile_id', 'version', 'status', 'title', 'platform', 'title_id', 'build', 'source', 'native_adapter',
                'native_inputs', 'import_preconditions', 'outputs', 'capabilities', 'evidence', 'limitations', 'unknowns')
BUILD_ATTRIBUTION = ('app_ver', 'sfo_version')
PROVENANCE_NOTE = ('locally validated build receipt: the executable, its source manifest and the recorded hashes agree; this is provenance '
                   'of what was built from which captured sources, not a signature and not an independent review endorsement')


class ProfileRefused(ValueError):
    """A descriptor or build-selection problem found before anything was written."""


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def _need(obj, key, kind, where):
    if not isinstance(obj, dict) or key not in obj: raise ProfileRefused(f'{where}: missing {key}')
    value = obj[key]
    if kind is not None and not isinstance(value, kind) or isinstance(value, bool) and kind is int:
        raise ProfileRefused(f'{where}.{key}: expected {getattr(kind, "__name__", kind)}, got {type(value).__name__}')
    return value


def _plain_name(name, where):
    if not isinstance(name, str) or not name or name in ('.', '..') or PurePosixPath(name).name != name or '\\' in name or Path(name).is_absolute():
        raise ProfileRefused(f'{where}: not a plain file name: {name!r}')
    return name


def resolve_reference(ref, base, what):
    """A {path, sha256} descriptor reference: relative POSIX path resolved against `base`, file present, bytes hash as declared."""
    if not isinstance(ref, dict): raise ProfileRefused(f'{what}: reference must be an object with path and sha256')
    rel, digest = ref.get('path'), ref.get('sha256')
    if not isinstance(rel, str) or not rel: raise ProfileRefused(f'{what}: reference needs a non-empty string path')
    if not isinstance(digest, str) or not SHA_RE.match(digest): raise ProfileRefused(f'{what}: reference needs a lowercase hex sha256')
    if '\\' in rel: raise ProfileRefused(f'{what}: reference path must use POSIX separators: {rel!r}')
    if PurePosixPath(rel).is_absolute() or Path(rel).is_absolute() or re.match(r'^[A-Za-z]:', rel):
        raise ProfileRefused(f'{what}: reference path must be relative to the descriptor location (or the explicit inputs root), not absolute: {rel!r}')
    path = (Path(base) / Path(*PurePosixPath(rel).parts)).resolve()
    if not path.is_file(): raise ProfileRefused(f'{what}: referenced file missing: {path} (declared {rel!r} relative to {base})')
    if sha(path) != digest: raise ProfileRefused(f'{what}: referenced file {path} does not match its declared sha256')
    return {'path': str(path), 'sha256': digest, 'declared': rel}


def _read_json(path, what):
    try: return json.loads(Path(path).read_text(encoding='utf-8'))
    except (OSError, ValueError) as exc: raise ProfileRefused(f'{what} {path} is not readable JSON: {exc}') from None


def load_descriptor(path, inputs_root=None):
    """Validate an `nhl-head-export-profile/1` descriptor and resolve every reference. Returns a dict with the descriptor bytes' hash,
    the resolved native inputs / preconditions, the loaded native profile and the identity fields the exporter records."""
    path = existing_file(path, 'Export profile descriptor', '.json')
    raw = path.read_bytes()
    try: d = json.loads(raw.decode('utf-8'))
    except (UnicodeDecodeError, ValueError) as exc: raise ProfileRefused(f'Export profile descriptor {path} is not readable JSON: {exc}') from None
    if not isinstance(d, dict): raise ProfileRefused(f'Export profile descriptor {path} must be a JSON object')
    if d.get('schema') != DESCRIPTOR_SCHEMA:
        raise ProfileRefused(f'Export profile descriptor {path} schema is {d.get("schema")!r}, not {DESCRIPTOR_SCHEMA!r}')
    missing = [k for k in REQUIRED_TOP if k not in d]
    if missing: raise ProfileRefused(f'Export profile descriptor {path} lacks required fields: {", ".join(missing)}')
    where = 'descriptor'
    profile_id = _need(d, 'profile_id', str, where)
    if not PROFILE_ID_RE.match(profile_id): raise ProfileRefused(f'{where}.profile_id {profile_id!r} is not a stable lowercase identifier')
    version = _need(d, 'version', int, where)
    if version < 1: raise ProfileRefused(f'{where}.version must be a positive integer, not {version!r}')
    for key in ('status', 'title', 'platform', 'title_id'):
        if not _need(d, key, str, where): raise ProfileRefused(f'{where}.{key} must not be empty')
    build = _need(d, 'build', dict, where)
    for key in BUILD_ATTRIBUTION: _need(build, key, str, f'{where}.build')
    if 'update_identifier' not in build: raise ProfileRefused(f'{where}.build.update_identifier must be present (null when unknown)')
    source = _need(d, 'source', dict, where)
    route = (_need(source, 'kind', str, f'{where}.source'), _need(source, 'strategy', str, f'{where}.source'))
    if route not in SUPPORTED_ROUTES: raise ProfileRefused(f'{where}.source {route} is not a supported route; supported: {SUPPORTED_ROUTES}')
    adapter = _need(d, 'native_adapter', dict, where)
    if _need(adapter, 'schema', str, f'{where}.native_adapter') != NATIVE_PROFILE_SCHEMA:
        raise ProfileRefused(f'{where}.native_adapter.schema must be {NATIVE_PROFILE_SCHEMA!r}')
    adapter_id = _need(adapter, 'profile_id', str, f'{where}.native_adapter')
    if _need(adapter, 'writer_exchange_schema', str, f'{where}.native_adapter') != WRITER_EXCHANGE_SCHEMA:
        raise ProfileRefused(f'{where}.native_adapter.writer_exchange_schema must be {WRITER_EXCHANGE_SCHEMA!r}')
    packer = _need(adapter, 'packer', dict, f'{where}.native_adapter')
    for key, expected in (('crate', native_build.CRATE_NAME), ('build_receipt_schema', native_build.RECEIPT_SCHEMA),
                          ('source_manifest_schema', native_build.SOURCE_MANIFEST_SCHEMA)):
        if _need(packer, key, str, f'{where}.native_adapter.packer') != expected:
            raise ProfileRefused(f'{where}.native_adapter.packer.{key} must be {expected!r}')
    refs = _need(d, 'native_inputs', dict, where)
    if set(refs) != set(NATIVE_INPUT_KEYS):
        raise ProfileRefused(f'{where}.native_inputs must have exactly the keys {list(NATIVE_INPUT_KEYS)}, got {sorted(refs)}')
    base = Path(inputs_root).resolve() if inputs_root is not None else path.parent
    if inputs_root is not None and not base.is_dir(): raise ProfileRefused(f'Export profile inputs root is not a directory: {base}')
    native = {key: resolve_reference(refs[key], base, f'{where}.native_inputs.{key}') for key in NATIVE_INPUT_KEYS}
    preconditions = resolve_reference(_need(d, 'import_preconditions', dict, where), base, f'{where}.import_preconditions')
    if not preconditions['path'].lower().endswith('.json'): raise ProfileRefused(f'{where}.import_preconditions must reference a .json file')
    # The native adapter identity is the referenced profile's own content, cross-checked against every declared identity field.
    profile = _read_json(native['profile']['path'], 'Native profile')
    pw = f'{where}.native_inputs.profile ({native["profile"]["declared"]})'
    if not isinstance(profile, dict) or profile.get('schema') != NATIVE_PROFILE_SCHEMA: raise ProfileRefused(f'{pw}: not a {NATIVE_PROFILE_SCHEMA} profile')
    if profile.get('id') != adapter_id: raise ProfileRefused(f'{pw}: native profile id {profile.get("id")!r} != declared native_adapter.profile_id {adapter_id!r}')
    try:
        donor_sha = profile['donor']['sha256']; diffuse = profile['diffuse']['provenance']; diffuse_sha = diffuse['decoded_sha256']
        donor_entry = profile['archive_provenance']['asset_path']; diffuse_entry = diffuse['asset_path']
        profile_build = profile['build']; profile_caps = profile['capabilities']
    except (KeyError, TypeError) as exc: raise ProfileRefused(f'{pw}: native profile lacks {exc}') from None
    if donor_sha != native['native_donor']['sha256']: raise ProfileRefused(f'{pw}: native profile donor sha256 != native_inputs.native_donor')
    if diffuse_sha != native['texture']['sha256']: raise ProfileRefused(f'{pw}: native profile diffuse decoded sha256 != native_inputs.texture')
    for key in ('title', 'platform', 'title_id'):
        if profile.get(key) != d[key]: raise ProfileRefused(f'{where}.{key} {d[key]!r} != native profile {key} {profile.get(key)!r}')
    for key in BUILD_ATTRIBUTION:
        if not isinstance(profile_build, dict) or profile_build.get(key) != build[key]:
            raise ProfileRefused(f'{where}.build.{key} {build[key]!r} != native profile build.{key}')
    caps = _need(d, 'capabilities', dict, where)
    if caps != profile_caps:
        raise ProfileRefused(f'{where}.capabilities must equal the native profile capabilities exactly (no aggregate or extra claims); '
                             f'differences: {sorted(k for k in set(caps) | set(profile_caps) if caps.get(k) != profile_caps.get(k))}')
    mesh = _read_json(native['exchange']['path'], 'Exchange mesh')
    if not isinstance(mesh, dict) or mesh.get('schema') != MESH_EXCHANGE_SCHEMA:
        raise ProfileRefused(f'{where}.native_inputs.exchange: not a {MESH_EXCHANGE_SCHEMA} mesh')
    if mesh.get('profile_id') != adapter_id or mesh.get('donor_sha256') != donor_sha:
        raise ProfileRefused(f'{where}.native_inputs.exchange: mesh profile_id / donor_sha256 disagree with the native profile')
    outputs = _need(d, 'outputs', dict, where)
    _plain_name(_need(outputs, 'native_directory', str, f'{where}.outputs'), f'{where}.outputs.native_directory')
    files = _need(outputs, 'native_files', list, f'{where}.outputs')
    if not files: raise ProfileRefused(f'{where}.outputs.native_files must not be empty')
    names = [_plain_name(_need(f, 'file', str, f'{where}.outputs.native_files[{i}]'), f'{where}.outputs.native_files[{i}].file') for i, f in enumerate(files)]
    if len(set(names)) != len(names): raise ProfileRefused(f'{where}.outputs.native_files names are not distinct')
    for i, name in enumerate(_need(outputs, 'receipts', list, f'{where}.outputs')): _plain_name(name, f'{where}.outputs.receipts[{i}]')
    evidence = _need(d, 'evidence', dict, where)
    _need(evidence, 'fixtures', list, f'{where}.evidence')
    for key in ('limitations', 'unknowns'):
        if not all(isinstance(x, str) and x for x in _need(d, key, list, where)): raise ProfileRefused(f'{where}.{key} must be a list of non-empty strings')
    reviewed = evidence.get('native_build', {}).get('reviewed_executables', []) if isinstance(evidence.get('native_build'), dict) else []
    if not isinstance(reviewed, list) or not all(isinstance(r, dict) and SHA_RE.match(str(r.get('sha256', ''))) for r in reviewed):
        raise ProfileRefused(f'{where}.evidence.native_build.reviewed_executables must be a list of objects with a sha256')
    return {'path': str(path), 'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw), 'descriptor': d,
            'profile_id': profile_id, 'version': version, 'inputs_root': str(base), 'source': {'kind': route[0], 'strategy': route[1]},
            'native_adapter': {'schema': NATIVE_PROFILE_SCHEMA, 'profile_id': adapter_id, 'profile_sha256': native['profile']['sha256']},
            'native_inputs': {k: {'path': v['path'], 'sha256': v['sha256']} for k, v in native.items()},
            'preconditions': {'path': preconditions['path'], 'sha256': preconditions['sha256']},
            'native_profile': profile, 'donor_entry': donor_entry, 'diffuse_entry': diffuse_entry,
            'outputs': {'native_directory': outputs['native_directory'], 'native_files': names, 'receipts': list(outputs['receipts'])},
            'reviewed_executables': {r['sha256']: r for r in reviewed}}


def compatibility_exchange(loaded):
    """The native writer's existing `nhl21-phase3-handoff/1` input shape, generated from a validated descriptor: the four hash-verified
    references and their provenance. The historical research handoff is not read; build.py consumes only these four entries."""
    return {'schema': WRITER_EXCHANGE_SCHEMA,
            'status': f'generated by src.application.export from export profile {loaded["profile_id"]} v{loaded["version"]}; native packer input only',
            'generated_from': {'export_profile': {'path': loaded['path'], 'sha256': loaded['sha256'], 'profile_id': loaded['profile_id'], 'version': loaded['version']}},
            **{key: dict(loaded['native_inputs'][key]) for key in NATIVE_INPUT_KEYS}}


def converter_sources_match(manifest, repo=native_build.REPO):
    """Provenance only: do the converter route sources present NOW still hash as the build's captured manifest? A mismatch is not a
    refusal (the exporter may legitimately have changed since the packer was built); it is recorded so a reader can see it."""
    declared = {k: v for k, v in native_build.identity_index(manifest).items() if k.startswith('converter:')}
    try: now = {k: v for k, v in native_build.identity_index({'converter_files': native_build.capture_converter_sources(repo)}).items() if k.startswith('converter:')}
    except native_build.BuildRefused as exc: return {'matched': False, 'differences': [str(exc)]}
    diffs = [k for k in sorted(set(declared) | set(now)) if declared.get(k) != now.get(k)]
    return {'matched': not diffs, 'differences': diffs}


def select_build_receipt(path, reviewed=None, repo=native_build.REPO):
    """Validate an explicitly selected `nhl21-build-receipt/1` and return the executable to use plus the provenance record."""
    path = existing_file(path, 'Build receipt', '.json')
    receipt = _read_json(path, 'Build receipt')
    what = f'Build receipt {path}'
    if not isinstance(receipt, dict) or receipt.get('schema') != native_build.RECEIPT_SCHEMA:
        raise ProfileRefused(f'{what}: schema is {getattr(receipt, "get", lambda k: None)("schema")!r}, not {native_build.RECEIPT_SCHEMA!r}')
    if receipt.get('status') != 'built': raise ProfileRefused(f'{what}: status is {receipt.get("status")!r}, not a successful built receipt')
    if receipt.get('crate') != native_build.CRATE_NAME: raise ProfileRefused(f'{what}: crate {receipt.get("crate")!r} is not {native_build.CRATE_NAME!r}')
    drift = receipt.get('drift_check')
    if not isinstance(drift, dict) or drift.get('rehashed_after_build') is not True or drift.get('differences') != []:
        raise ProfileRefused(f'{what}: post-build drift check is missing or not empty; the executable was not accepted by its own build')
    sm, exe = receipt.get('source_manifest'), receipt.get('executable')
    for name, block, keys in (('source_manifest', sm, ('path', 'sha256', 'content_sha256')), ('executable', exe, ('path', 'sha256', 'bytes'))):
        if not isinstance(block, dict) or any(k not in block for k in keys): raise ProfileRefused(f'{what}: {name} block lacks {keys}')
    # Deterministic sibling resolution by the recorded file names: a relocated build directory still resolves; nothing is scanned.
    manifest_path = path.with_name(Path(str(sm['path'])).name)
    exe_path = path.with_name(Path(str(exe['path'])).name)
    if not manifest_path.is_file(): raise ProfileRefused(f'{what}: source manifest missing beside the receipt: {manifest_path}')
    if sha(manifest_path) != sm['sha256']: raise ProfileRefused(f'{what}: source manifest {manifest_path} does not match the receipt sha256')
    manifest = _read_json(manifest_path, 'Build source manifest')
    if not isinstance(manifest, dict) or manifest.get('schema') != native_build.SOURCE_MANIFEST_SCHEMA:
        raise ProfileRefused(f'{what}: source manifest schema is {getattr(manifest, "get", lambda k: None)("schema")!r}, not {native_build.SOURCE_MANIFEST_SCHEMA!r} (v1 receipts are not accepted; rebuild)')
    identity = native_build.manifest_content_hash(manifest)
    if identity != sm['content_sha256'] or manifest.get('content_sha256') != identity:
        raise ProfileRefused(f'{what}: source manifest identity does not recompute (receipt {str(sm["content_sha256"])[:16]}…, manifest {str(manifest.get("content_sha256"))[:16]}…, recomputed {identity[:16]}…)')
    if not exe_path.is_file(): raise ProfileRefused(f'{what}: executable missing beside the receipt: {exe_path}')
    if not isinstance(exe['bytes'], int) or exe_path.stat().st_size != exe['bytes'] or sha(exe_path) != exe['sha256']:
        raise ProfileRefused(f'{what}: executable {exe_path} does not match the receipt sha256 / size')
    review = (reviewed or {}).get(exe['sha256'])
    return {'selection': 'explicit build receipt',
            'receipt': {'path': str(path), 'sha256': sha(path), 'schema': receipt['schema'], 'status': receipt['status'], 'profile': receipt.get('profile'),
                        'tool': receipt.get('tool'), 'finished': receipt.get('finished'), 'expected_manifest': manifest.get('expected_manifest')},
            'source_manifest': {'path': str(manifest_path), 'sha256': sm['sha256'], 'schema': manifest['schema'], 'content_sha256': identity,
                                'dependencies': {name: {'content_sha256': dep.get('content_sha256'), 'git_head': (dep.get('git') or {}).get('head'),
                                                        'dirty_paths': len((dep.get('git') or {}).get('dirty_paths') or [])}
                                                 for name, dep in (manifest.get('dependencies') or {}).items()}},
            'executable': {'path': str(exe_path), 'sha256': exe['sha256'], 'bytes': exe['bytes']},
            'converter_sources_now': converter_sources_match(manifest, repo),
            'provenance': PROVENANCE_NOTE,
            'independent_review': ({'matched': True, **{k: v for k, v in review.items() if k != 'sha256'}} if review
                                   else {'matched': False, 'note': 'executable hash not listed among the export profile\'s reviewed executables'
                                         if reviewed is not None else 'no export profile review list to compare against'})}
