"""Path and input guards shared by the guarded exporter and the Codex NHL21 native tools. stdlib only, so the Blender-side
packer can import it. Every function raises ValueError with a plain message and writes nothing."""
import json
from pathlib import Path


def contained(run_root, path, what='path'):
    """`path` resolved, strictly beneath the existing directory `run_root`."""
    root, target = Path(run_root).resolve(), Path(path).resolve()
    if not root.is_dir(): raise ValueError(f'Run root is not a directory: {root}')
    try: target.relative_to(root)
    except ValueError: raise ValueError(f'{what} must lie beneath the run root {root}: {target}') from None
    if target == root: raise ValueError(f'{what} must not be the run root itself: {root}')
    return target


def fresh_output_beneath(run_root, output):
    """A not-yet-existing output directory strictly beneath the run root (the caller creates it)."""
    target = contained(run_root, output, 'Output')
    if target.exists(): raise ValueError(f'Output requires a fresh directory: {target}')
    return target


def existing_file(path, what, suffix=None):
    path = Path(path).resolve()
    if not path.is_file(): raise ValueError(f'{what} missing: {path}')
    if suffix and path.suffix.lower() != suffix: raise ValueError(f'{what} must be a {suffix} file: {path}')
    return path


def parse_preconditions(raw, source='<bytes>'):
    """Import-preconditions JSON from already-captured bytes: an object whose `targets` list names archive entries by
    asset_path + decoded_sha256. Callers that copy/hash the bytes should validate these same bytes."""
    path = source
    try: data = json.loads(raw.decode('utf-8'))
    except (UnicodeDecodeError, ValueError) as exc: raise ValueError(f'Import preconditions {path} is not readable JSON: {exc}') from None
    if not isinstance(data, dict) or not isinstance(data.get('targets'), list) or not data['targets']:
        raise ValueError(f'Import preconditions {path} needs a non-empty targets list')
    for i, target in enumerate(data['targets']):
        if not isinstance(target, dict) or not isinstance(target.get('asset_path'), str) or not isinstance(target.get('decoded_sha256'), str):
            raise ValueError(f'Import preconditions {path}: targets[{i}] needs string asset_path and decoded_sha256')
    return data


def load_preconditions(path):
    """Path-based convenience over parse_preconditions (existence + .json suffix checked first)."""
    path = existing_file(path, 'Import preconditions', '.json')
    try: raw = path.read_bytes()
    except OSError as exc: raise ValueError(f'Import preconditions {path} is not readable JSON: {exc}') from None
    return parse_preconditions(raw, path)


def preconditions_target(preconditions, asset_path, decoded_sha256):
    """The single target for `asset_path`; its decoded hash must equal the qualified donor/texture hash."""
    matches = [t for t in preconditions['targets'] if t['asset_path'] == asset_path]
    if len(matches) != 1: raise ValueError(f'Import preconditions must name {asset_path} exactly once (found {len(matches)})')
    if matches[0]['decoded_sha256'] != decoded_sha256:
        raise ValueError(f'Import preconditions target {asset_path} does not match the qualified decoded sha256')
    return matches[0]


def plan_artifact_names(plan, native_dir):
    """Native file names from an installation plan: plain file names, distinct, each a regular file inside native_dir."""
    native_dir = Path(native_dir).resolve()
    if not isinstance(plan, dict) or not isinstance(plan.get('replacements'), list) or not plan['replacements']:
        raise ValueError('Installation plan needs a non-empty replacements list')
    names = []
    for r in plan['replacements']:
        name = r.get('file') if isinstance(r, dict) else None
        if not isinstance(name, str) or not name or name in ('.', '..') or Path(name).name != name or Path(name).is_absolute():
            raise ValueError(f'Installation plan file reference is not a plain file name: {name!r}')
        path = native_dir / name
        if path.resolve().parent != native_dir or path.is_symlink() or not path.is_file():
            raise ValueError(f'Installation plan file is not a regular file inside the native output: {name}')
        if name in names: raise ValueError(f'Installation plan names {name} more than once')
        names.append(name)
    return names
