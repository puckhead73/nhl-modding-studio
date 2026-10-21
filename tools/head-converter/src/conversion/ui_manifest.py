"""Application-facing manifest (`nhl-head-ui-manifest/1`) written at the end of every worker job (completed, failed
or cancelled): where the preview, atlas, diagnostics, receipts and handoff are, what the validation found, what was
lost, and whether the candidate may be packed. stdlib only. `check()` re-verifies every listed file and hash so an
application can trust the manifest before showing anything.

Rules: `export_ready` is true only for a completed job whose handoff status is `ready_for_experimental_packing`;
a blocked candidate keeps its diagnostic preview/renders but is never export-ready; a failed or cancelled job has no
candidate artifacts. Nothing here is a native package or a game claim.
"""
import hashlib, json
from pathlib import Path

MANIFEST_SCHEMA = 'nhl-head-ui-manifest/1'


def _sha(p):
    h = hashlib.sha256()
    with open(p, 'rb') as f:
        for b in iter(lambda: f.read(1 << 20), b''): h.update(b)
    return h.hexdigest()


def _file(out_dir, p, kind=None, hash=True):
    if p is None: return None
    p = Path(p)
    try: rel = str(p.resolve().relative_to(Path(out_dir).resolve())).replace('\\', '/')
    except ValueError: rel = None
    d = {'path': str(p), 'relative': rel, 'exists': p.exists()}
    if kind: d['kind'] = kind
    if p.exists() and p.is_file():
        d['bytes'] = p.stat().st_size
        if hash: d['sha256'] = _sha(p)
        else: d['live'] = True
    return d


def build(out_dir, status, job, receipts, handoff=None, preview=None, events_path=None, note=None):
    """status: the runner's nhl-head-job-status/1 dict; receipts: list of stage receipt dicts; handoff: handoff dict
    or None; preview: {'path','info'|'error','status'} or None."""
    out_dir = Path(out_dir); by_name = {r['name']: r for r in receipts}
    job_status = status.get('status'); hand_status = handoff.get('status') if handoff else None
    export_ready = bool(job_status == 'completed' and hand_status == 'ready_for_experimental_packing')
    cls = None
    for k in ('classify_after_repair', 'classify_after_neck'):
        if k in by_name and by_name[k].get('summary'): cls = {'stage': k, **by_name[k]['summary']}; break
    freeze = by_name.get('freeze_candidate', {}).get('summary') or {}
    val_path = handoff.get('validation_json') if handoff else None
    validation = {'candidate_status': hand_status, 'blocking_problems': list(handoff.get('blocking_problems', [])) if handoff else [], 'checks_ok': freeze.get('checks_ok'), 'checks_total': freeze.get('checks_total'),
                  'parity_vs_references': freeze.get('parity_vs_references'), 'classification': cls, 'repair': (by_name.get('collision_repair', {}).get('summary') or None), 'eyes': (by_name.get('eye_place', {}).get('summary') or None),
                  'measurements': (by_name.get('measure', {}).get('summary') or None), 'coverage_limits': (cls or {}).get('coverage_limits'), 'gates': handoff.get('gates') if handoff else None, 'validation_json': _file(out_dir, val_path) if val_path else None}
    artifacts = {}
    if handoff:
        a = handoff.get('artifacts', {})
        artifacts = {'native_mesh_json': _file(out_dir, a.get('native_mesh_json'), 'exchange'), 'atlas_png': _file(out_dir, a.get('atlas_png'), 'png'), 'handoff_json': _file(out_dir, out_dir / 'handoff.json', 'handoff'), 'loss_report_json': _file(out_dir, out_dir / 'loss_report.json', 'loss_report')}
        if artifacts['native_mesh_json'] and artifacts['native_mesh_json'].get('sha256') != a.get('native_mesh_sha256'): artifacts['native_mesh_json']['hash_matches_handoff'] = False
        if artifacts['atlas_png'] and artifacts['atlas_png'].get('sha256') != a.get('atlas_sha256'): artifacts['atlas_png']['hash_matches_handoff'] = False
    bake = by_name.get('bake', {}); bake_dir = Path(bake['dir']) / 'bake' if bake.get('dir') else None
    diagnostics = {}
    if bake_dir and bake_dir.exists():
        diagnostics['atlas_diagnostics'] = {p.stem: _file(out_dir, p, 'png') for p in sorted(bake_dir.glob('diag_*.png'))}
        diagnostics['bake_variants'] = {p.stem: _file(out_dir, p, 'png') for p in sorted(bake_dir.glob('bake_*.png'))}
    rend = by_name.get('render', {})
    if rend.get('dir') and Path(rend['dir']).exists():
        rd = Path(rend['dir']); diagnostics['renders'] = {'textured': {p.stem: _file(out_dir, p, 'png') for p in sorted((rd / 'textured').glob('*.png'))}, 'views': {p.stem: _file(out_dir, p, 'png') for p in sorted((rd / 'views').glob('*.png'))}, 'note': (rend.get('summary') or {}).get('note')}
    for k in ('flip_classes', 'flagged_faces', 'collision_triggers'):
        if cls and cls['stage'] in by_name and k in (by_name[cls['stage']].get('outputs') or {}): diagnostics[k] = _file(out_dir, by_name[cls['stage']]['outputs'][k]['path'], 'json')
    prev = {'status': 'absent', 'path': None}
    if preview:
        prev = {'status': preview.get('status', 'absent'), 'file': _file(out_dir, preview.get('path'), 'glb') if preview.get('path') else None, 'info': preview.get('info'), 'error': preview.get('error'), 'diagnostic_only': not export_ready}
        if prev['file'] and not prev['file']['exists'] and prev['status'] == 'ok': prev['status'] = 'missing'
    losses = (handoff or {}).get('loss_report') or {'schema': 'nhl-head-loss/1', 'entries': [l for r in receipts for l in (r.get('losses') or [])]}
    stages = [{'index': r['index'], 'name': r['name'], 'id': f"{r['index']:02d}_{r['name']}", 'status': r['status'], 'elapsed_s': r.get('elapsed_s'), 'receipt': _file(out_dir, Path(r['dir']) / 'receipt.json', 'receipt'), 'error': r.get('error')} for r in receipts]
    packing = {'ready': export_ready, 'handoff_status': hand_status, 'handoff_json': str(out_dir / 'handoff.json') if handoff else None, 'blocking_problems': validation['blocking_problems'], 'packer': 'Codex experiments/codex/nhl21_package/build.py (refuses status != ready_for_experimental_packing)',
               'runtime_acceptance': 'none inherited: a new run is never game-verified by construction'}
    man = {'schema': MANIFEST_SCHEMA, 'job_id': job.get('job_id'), 'run_id': out_dir.name, 'output_dir': str(out_dir), 'route': job.get('route'), 'source_label': (job.get('source') or {}).get('label'), 'destination_label': (job.get('destination') or {}).get('label'),
           'job_status': job_status, 'candidate_status': hand_status, 'export_ready': export_ready, 'note': note or status.get('note'), 'started': status.get('started'), 'finished': status.get('finished'), 'elapsed_s': status.get('elapsed_s'),
           'files': {'job_json': _file(out_dir, out_dir / 'job.json', 'job'), 'status_json': _file(out_dir, out_dir / 'status.json', 'status'), 'progress_json': _file(out_dir, out_dir / 'progress.json', 'status'), 'events_jsonl': _file(out_dir, events_path, 'events', hash=False) if events_path else None, 'log_txt': _file(out_dir, out_dir / 'log.txt', 'log', hash=False), 'cancel_file': str(out_dir / 'CANCEL')},
           'preview': prev, 'artifacts': artifacts, 'diagnostics': diagnostics, 'validation': validation, 'losses': losses, 'packing': packing, 'stages': stages}
    return man


def _walk_files(obj, trail='manifest'):
    if isinstance(obj, dict):
        if 'path' in obj and 'exists' in obj: yield trail, obj
        else:
            for k, v in obj.items(): yield from _walk_files(v, f'{trail}.{k}')
    elif isinstance(obj, list):
        for i, v in enumerate(obj): yield from _walk_files(v, f'{trail}[{i}]')


READY = 'ready_for_experimental_packing'
JOB_STATES = ('completed', 'failed', 'cancelled')
_HEX64 = set('0123456789abcdef')


def _is_sha(v):
    return isinstance(v, str) and len(v) == 64 and set(v) <= _HEX64


def _load_json(path, label, problems):
    try:
        obj = json.loads(Path(path).read_text(encoding='utf-8'))
    except (OSError, ValueError, UnicodeDecodeError) as e:
        problems.append(f'{label}: malformed JSON ({type(e).__name__}: {str(e)[:80]})'); return None
    if not isinstance(obj, dict): problems.append(f'{label}: JSON root must be an object'); return None
    return obj


def _same_path(a, b):
    try: return Path(a).resolve() == Path(b).resolve()
    except (OSError, TypeError, ValueError): return False


def _required_receipt(stage):
    try: return json.loads(Path(stage['receipt']['path']).read_text(encoding='utf-8')).get('required', True)
    except Exception: return True


def check(manifest, verify_hashes=True):
    """Consistency and content-integrity check of a manifest. Returns (ok, problems).

    Readiness is NOT taken from the manifest's own fields: it is derived from the referenced, hash-verified
    `status.json` (`files.status_json`) and `handoff.json` (`artifacts.handoff_json`), and every duplicated readiness
    field in the manifest (`export_ready`, `candidate_status`, `packing.ready`, `packing.handoff_status`,
    `packing.blocking_problems`, `validation.candidate_status`, `validation.blocking_problems`,
    `preview.diagnostic_only`) must agree with it. Required immutable file references must carry a sha256 that
    matches the file on disk; the candidate artifacts must be the ones the handoff names (same path and hash); stage
    receipts must agree with the stage list. Files marked `live: true` (events, log) are checked for existence only.
    Missing fields, wrong types and malformed referenced JSON are reported as problems, never raised.
    This establishes consistency between the manifest and the run directory it points at, not authenticity: a run
    directory rewritten as a whole would pass; provenance beyond that is the packer's and the user's evidence."""
    problems = []
    if not isinstance(manifest, dict): return False, ['manifest must be a JSON object']
    if manifest.get('schema') != MANIFEST_SCHEMA: problems.append(f"schema {manifest.get('schema')!r} != {MANIFEST_SCHEMA}")
    for k, t in (('job_id', str), ('run_id', str), ('output_dir', str), ('job_status', str), ('export_ready', bool), ('files', dict), ('preview', dict), ('artifacts', dict), ('validation', dict), ('packing', dict), ('stages', list)):
        if k not in manifest: problems.append(f'missing field {k}')
        elif not isinstance(manifest[k], t): problems.append(f'field {k} must be {t.__name__}, got {type(manifest[k]).__name__}')
    if problems: return False, problems
    if manifest['job_status'] not in JOB_STATES: problems.append(f"job_status {manifest['job_status']!r} not in {JOB_STATES}")
    if not isinstance(manifest['packing'].get('ready'), bool): problems.append('packing.ready must be bool')
    # 1. every file reference: shape, existence, hash (live files: existence only)
    verified = {}
    for trail, f in _walk_files(manifest):
        if not isinstance(f.get('path'), str) or not isinstance(f.get('exists'), bool): problems.append(f'{trail}: file reference needs string path and bool exists'); continue
        p = Path(f['path'])
        if not p.exists(): problems.append(f'missing file: {p} ({trail})'); continue
        if not p.is_file(): problems.append(f'not a file: {p} ({trail})'); continue
        if f.get('live') is True: continue
        if not _is_sha(f.get('sha256')): problems.append(f'{trail}: sha256 missing or malformed for {p}'); continue
        if verify_hashes:
            h = _sha(p); verified[str(p)] = h
            if h != f['sha256']: problems.append(f'sha256 mismatch: {p} ({trail})')
        else: verified[str(p)] = f['sha256']

    def ref_ok(ref):
        return isinstance(ref, dict) and isinstance(ref.get('path'), str) and _is_sha(ref.get('sha256')) and verified.get(str(Path(ref['path']))) == ref['sha256']
    # 2. status.json (required, hashed) -> job status
    status = None; sref = manifest['files'].get('status_json')
    if not isinstance(sref, dict) or not _is_sha(sref.get('sha256')): problems.append('files.status_json must be a hashed file reference')
    elif ref_ok(sref): status = _load_json(sref['path'], 'status.json', problems)
    if status is not None:
        if status.get('schema') != 'nhl-head-job-status/1': problems.append(f"status.json schema {status.get('schema')!r} unexpected")
        if status.get('status') != manifest['job_status']: problems.append(f"job_status {manifest['job_status']!r} contradicts status.json {status.get('status')!r}")
        if status.get('job_id') != manifest['job_id']: problems.append('job_id contradicts status.json')
        if not isinstance(status.get('ready_for_packing'), bool): problems.append('status.json ready_for_packing must be bool')
        if status.get('status') not in JOB_STATES: problems.append(f"status.json status {status.get('status')!r} not terminal")
    # 3. handoff (required for completed jobs, hashed) -> candidate status and artifacts
    hand = None; href = manifest['artifacts'].get('handoff_json')
    if manifest['job_status'] == 'completed':
        if not isinstance(href, dict) or not _is_sha(href.get('sha256')): problems.append('completed job requires a hashed artifacts.handoff_json reference')
        elif ref_ok(href): hand = _load_json(href['path'], 'handoff.json', problems)
        if status is not None and status.get('handoff') and isinstance(href, dict) and not _same_path(status['handoff'], href.get('path', '')): problems.append('status.json handoff path differs from artifacts.handoff_json')
    else:
        if manifest['artifacts']: problems.append(f"{manifest['job_status']} job must not list candidate artifacts")
        if manifest.get('candidate_status') is not None: problems.append(f"{manifest['job_status']} job must not carry a candidate_status")
    hand_status = None; blockers = None
    if hand is not None:
        hand_status = hand.get('status'); blockers = hand.get('blocking_problems')
        if not isinstance(hand_status, str): problems.append('handoff status missing or not a string')
        if not isinstance(blockers, list): problems.append('handoff blocking_problems missing or not a list'); blockers = None
        ha = hand.get('artifacts')
        if not isinstance(ha, dict): problems.append('handoff artifacts missing'); ha = {}
        for key, skey in (('native_mesh_json', 'native_mesh_sha256'), ('atlas_png', 'atlas_sha256')):
            mref = manifest['artifacts'].get(key)
            if not isinstance(mref, dict) or not _is_sha(mref.get('sha256')): problems.append(f'artifacts.{key} must be a hashed file reference'); continue
            if not isinstance(ha.get(key), str) or not _is_sha(ha.get(skey)): problems.append(f'handoff artifacts.{key}/{skey} missing or malformed'); continue
            if not _same_path(mref['path'], ha[key]): problems.append(f'artifacts.{key} path differs from handoff ({mref["path"]} != {ha[key]})')
            if mref['sha256'] != ha[skey]: problems.append(f'artifacts.{key} sha256 differs from handoff')
            if verify_hashes and Path(ha[key]).is_file() and _sha(ha[key]) != ha[skey]: problems.append(f'handoff artifact {key} on disk differs from handoff {skey}')
            if mref.get('hash_matches_handoff') is False: problems.append(f'artifact {key} self-reports hash_matches_handoff false')
    # 4. derived readiness vs every duplicated field
    derived = bool(status is not None and status.get('status') == 'completed' and hand is not None and hand_status == READY and not blockers and status.get('ready_for_packing') is True)
    if status is not None and hand is not None and (status.get('ready_for_packing') is True) != (hand_status == READY and not blockers): problems.append('status.json ready_for_packing contradicts handoff status/blockers')
    if manifest['export_ready'] != derived: problems.append(f"export_ready {manifest['export_ready']} contradicts derived readiness {derived} (status.json + handoff)")
    if manifest['packing'].get('ready') != derived: problems.append(f"packing.ready {manifest['packing'].get('ready')} contradicts derived readiness {derived}")
    if manifest.get('candidate_status') != hand_status: problems.append(f"candidate_status {manifest.get('candidate_status')!r} contradicts handoff status {hand_status!r}")
    if manifest['packing'].get('handoff_status') != hand_status: problems.append(f"packing.handoff_status {manifest['packing'].get('handoff_status')!r} contradicts handoff status {hand_status!r}")
    if manifest['validation'].get('candidate_status') != hand_status: problems.append(f"validation.candidate_status {manifest['validation'].get('candidate_status')!r} contradicts handoff status {hand_status!r}")
    exp_block = list(blockers) if blockers is not None else []
    if manifest['packing'].get('blocking_problems') != exp_block: problems.append('packing.blocking_problems contradict the handoff')
    if manifest['validation'].get('blocking_problems') != exp_block: problems.append('validation.blocking_problems contradict the handoff')
    if manifest['packing'].get('handoff_json') is not None and isinstance(href, dict) and not _same_path(manifest['packing']['handoff_json'], href.get('path', '')): problems.append('packing.handoff_json differs from artifacts.handoff_json')
    if derived:
        for k in ('native_mesh_json', 'atlas_png', 'handoff_json'):
            if not manifest['artifacts'].get(k): problems.append(f'export_ready without artifact {k}')
    # 5. preview and stages
    pv = manifest['preview']
    if pv.get('status') == 'ok':
        if not (isinstance(pv.get('file'), dict) and Path(pv['file'].get('path', '')).is_file()): problems.append('preview marked ok but file missing')
        if pv.get('diagnostic_only') != (not derived): problems.append(f"preview.diagnostic_only {pv.get('diagnostic_only')} contradicts derived readiness {derived}")
    for i, st in enumerate(manifest['stages']):
        if not isinstance(st, dict) or not isinstance(st.get('receipt'), dict): problems.append(f'stages[{i}] needs a receipt file reference'); continue
        if not ref_ok(st['receipt']): continue   # existence/hash problem already reported
        rec = _load_json(st['receipt']['path'], f'stages[{i}] receipt', problems)
        if rec is None: continue
        if rec.get('status') != st.get('status') or rec.get('name') != st.get('name') or rec.get('index') != st.get('index'): problems.append(f"stages[{i}] {st.get('id')} contradicts its receipt ({rec.get('index')} {rec.get('name')} {rec.get('status')})")
    if derived and any(s.get('status') == 'cancelled' or (s.get('status') == 'failed' and _required_receipt(s)) for s in manifest['stages'] if isinstance(s, dict)): problems.append('export_ready with a cancelled or failed required stage')
    return (not problems), problems


def load(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))
