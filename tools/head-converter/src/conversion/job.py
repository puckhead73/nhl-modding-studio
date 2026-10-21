"""Conversion job runner core (numpy-free; stdlib only): job validation, run-root containment, ordered named stages with
machine-readable receipts (inputs/outputs hashes, params, elapsed), progress file, JSON-lines process events,
structured failure receipts, cancellation (including of a running Blender subprocess), and the application manifest.
Aligned with `nhl-head-job/1` (docs/CANONICAL_CONTRACT.md); deltas are listed in research/claude/contract-proposals.md.

Rules enforced here: output directory must be new and inside the designated run root; every referenced input file
must exist and match its declared sha256 (when declared); a cancelled or failed job is never 'ready'; completed
receipts survive a failure or cancellation; nothing is written outside the output directory.

Events (`nhl-head-worker-event/1`, one JSON object per line in <out>/events.jsonl and optionally on stdout):
  job_started, stage_started, stage_progress, stage_finished, job_completed, job_failed, job_cancelled
  (job_invalid is emitted by the entry point when no output directory exists yet). Fields: schema, seq, ts, job_id,
  run_id, event, and for stage events stage {index, name, id}; stage_finished adds status/elapsed_s/error; job_*
  add status, ready_for_packing, elapsed_s, output_dir, manifest. Diagnostic text stays in log.txt.
"""
import hashlib, json, os, platform, subprocess, sys, time, traceback
from pathlib import Path
from . import ui_manifest as UM

JOB_SCHEMA = 'nhl-head-job/1'
EVENT_SCHEMA = 'nhl-head-worker-event/1'
SUPPORTED_STRATEGIES = ('donor_fit',)
CANCEL_FILE = 'CANCEL'
REPO_ROOT = Path(__file__).resolve().parents[2]


class JobError(Exception):
    pass


class Cancelled(Exception):
    pass


def sha256_file(path, chunk=1 << 20):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for b in iter(lambda: f.read(chunk), b''): h.update(b)
    return h.hexdigest()


def now():
    return time.strftime('%Y-%m-%dT%H:%M:%S')


def resolve_ref(ref, base=None):
    """A file reference is {'path': ..., 'sha256': optional}. Returns (Path, declared sha)."""
    if isinstance(ref, str): ref = {'path': ref}
    p = Path(ref['path'])
    if not p.is_absolute() and base is not None: p = Path(base) / p
    return p, ref.get('sha256')


def verify_ref(name, ref, base=None):
    p, declared = resolve_ref(ref, base)
    if not p.exists(): raise JobError(f'input {name}: missing file {p}')
    digest = sha256_file(p)
    if declared and declared.lower() != digest: raise JobError(f'input {name}: sha256 mismatch {p}: declared {declared[:16]}... measured {digest[:16]}...')
    return {'path': str(p), 'sha256': digest, 'bytes': p.stat().st_size, 'declared_sha256': declared}


def validate_job(job):
    problems = []
    if not isinstance(job, dict): return [f'job must be a JSON object, got {type(job).__name__}']
    if job.get('schema') != JOB_SCHEMA: problems.append(f"schema must be {JOB_SCHEMA}, got {job.get('schema')}")
    for k in ('job_id', 'strategy', 'source', 'destination', 'output', 'tools', 'policies'):
        if k not in job: problems.append(f'missing key {k}')
    for k in ('source', 'destination', 'output', 'tools', 'policies'):
        if k in job and not isinstance(job[k], dict): problems.append(f'{k} must be an object')
    if not isinstance(job.get('job_id', ''), str) or not job.get('job_id', 'x'): problems.append('job_id must be a non-empty string')
    if job.get('strategy') not in SUPPORTED_STRATEGIES: problems.append(f"strategy {job.get('strategy')} unsupported (supported: {SUPPORTED_STRATEGIES})")
    out = job.get('output') if isinstance(job.get('output'), dict) else {}
    if 'run_root' not in out or 'dir' not in out or not isinstance(out.get('run_root'), str) or not isinstance(out.get('dir'), str): problems.append('output.run_root and output.dir (strings) required')
    else:
        root = Path(out['run_root']).resolve(); d = (root / out['dir']).resolve()
        if not root.exists(): problems.append(f'run_root does not exist: {root}')
        if root not in d.parents: problems.append(f'output dir {d} is not inside run_root {root}')
        if Path(out['dir']).is_absolute() or '..' in Path(out['dir']).parts or not out['dir'].strip(): problems.append('output.dir must be a relative child name without ..')
        if d.exists(): problems.append(f'output dir already exists: {d} (refusing collision)')
    return problems


def blender_command(exe, args):
    """Command line for a Blender invocation. Test seam: a '.py' executable is run through this interpreter as a
    stand-in for Blender (used by tests/conversion to exercise subprocess failure and cancellation without Blender)."""
    exe = str(exe)
    if exe.lower().endswith('.py'): return [sys.executable, exe, *[str(a) for a in args]]
    return [exe, *[str(a) for a in args]]


def tool_identity(job):
    info = {'python': {'executable': sys.executable, 'version': platform.python_version()}, 'platform': platform.platform()}
    try:
        import numpy; info['numpy'] = numpy.__version__
    except Exception: info['numpy'] = None
    b = job.get('tools', {}).get('blender')
    if b:
        exe = Path(b['path'])
        if not exe.exists(): raise JobError(f'blender executable missing: {exe}')
        try:
            r = subprocess.run(blender_command(exe, ['--version']), capture_output=True, text=True, timeout=120); line = (r.stdout.strip().splitlines() or [''])[0]
        except Exception as e: raise JobError(f'blender --version failed: {e}')
        info['blender'] = {'path': str(exe), 'version_line': line, 'expected_version': b.get('expected_version'), 'sha256': sha256_file(exe)}
        if b.get('expected_version') and b['expected_version'] not in line: raise JobError(f"blender version '{line}' does not contain expected '{b['expected_version']}'")
    return info


class Runner:
    def __init__(self, job, job_path=None, log_to_stdout=True, events_to_stdout=False, planned_stages=None):
        self.job = job; self.problems = validate_job(job)
        if self.problems: raise JobError('invalid job: ' + '; '.join(self.problems))
        self.root = Path(job['output']['run_root']).resolve(); self.out = (self.root / job['output']['dir']).resolve()
        self.out.mkdir(parents=False)
        (self.out / 'stages').mkdir(); self.receipts = []; self.index = 0; self.t0 = time.time(); self.log_to_stdout = log_to_stdout and not events_to_stdout; self.events_to_stdout = events_to_stdout
        self.seq = 0; self.current_stage = None; self.meta = {}; self.active_proc = None
        self.logf = open(self.out / 'log.txt', 'a', encoding='utf-8'); self.evf = open(self.out / 'events.jsonl', 'a', encoding='utf-8')
        (self.out / 'job.json').write_text(json.dumps(job, indent=1, ensure_ascii=False), encoding='utf-8')
        self.status = {'schema': 'nhl-head-job-status/1', 'job_id': job['job_id'], 'run_id': self.out.name, 'job_path': str(job_path) if job_path else None, 'job_sha256': sha256_file(job_path) if job_path else None, 'status': 'running', 'started': now(), 'output_dir': str(self.out), 'stages': [], 'ready_for_packing': False,
                       'events_jsonl': str(self.out / 'events.jsonl'), 'cancel_file': str(self.out / CANCEL_FILE), 'pid': os.getpid()}
        self.write_status(); self.emit('job_started', status='running', output_dir=str(self.out), cancel_file=str(self.out / CANCEL_FILE), pid=os.getpid(), planned_stages=planned_stages)

    # ---- diagnostics and events -------------------------------------------------------------------------------
    def log(self, *a):
        line = f'[{now()}] ' + ' '.join(str(x) for x in a)
        self.logf.write(line + '\n'); self.logf.flush()
        if self.log_to_stdout: print(line, flush=True)
        if self.current_stage: self.emit('stage_progress', message=' '.join(str(x) for x in a))

    def emit(self, event, **fields):
        self.seq += 1
        ev = {'schema': EVENT_SCHEMA, 'seq': self.seq, 'ts': now(), 'job_id': self.job['job_id'], 'run_id': self.out.name, 'event': event}
        if self.current_stage and event.startswith('stage_'): ev['stage'] = dict(self.current_stage)
        ev.update(fields); ev['elapsed_s'] = round(time.time() - self.t0, 1)
        line = json.dumps(ev, ensure_ascii=False, default=str)
        self.evf.write(line + '\n'); self.evf.flush()
        if self.events_to_stdout: print(line, flush=True)
        return ev

    def write_status(self):
        self.status['elapsed_s'] = round(time.time() - self.t0, 1); self.status['updated'] = now()
        (self.out / 'progress.json').write_text(json.dumps(self.status, indent=1, ensure_ascii=False), encoding='utf-8')

    def check_cancel(self):
        if (self.out / CANCEL_FILE).exists(): raise Cancelled(f'{CANCEL_FILE} file present in output directory')

    def stage_dir(self, name):
        d = self.out / 'stages' / f'{self.index:02d}_{name}'; d.mkdir(); return d

    def hash_map(self, items):
        return {k: ({'path': str(v), 'sha256': sha256_file(v), 'bytes': Path(v).stat().st_size} if v is not None and Path(v).exists() else {'path': str(v), 'missing': True}) for k, v in items.items()}

    # ---- stages ----------------------------------------------------------------------------------------------
    def run_stage(self, name, fn, inputs=None, params=None, required=True):
        """fn(stage_dir, log) -> dict with keys 'outputs' {name: path}, 'summary' (json-able), optional 'losses' list.
        Receipt written on success, failure and cancellation. A failed required stage aborts the job."""
        self.check_cancel(); self.index += 1; d = self.stage_dir(name); t = time.time()
        self.current_stage = {'index': self.index, 'name': name, 'id': f'{self.index:02d}_{name}'}
        rec = {'schema': 'nhl-head-stage-receipt/1', 'index': self.index, 'name': name, 'status': 'running', 'started': now(), 'dir': str(d), 'inputs': self.hash_map(inputs or {}), 'params': params or {}, 'required': required}
        (d / 'receipt.json').write_text(json.dumps(rec, indent=1, ensure_ascii=False), encoding='utf-8'); self.status['stages'].append({'index': self.index, 'name': name, 'id': self.current_stage['id'], 'status': 'running'}); self.write_status()
        self.emit('stage_started', required=required); self.log(f'stage {self.index} {name} start')
        try:
            res = fn(d, self.log) or {}
            rec.update({'status': 'ok', 'outputs': self.hash_map(res.get('outputs', {})), 'summary': res.get('summary'), 'losses': res.get('losses', []), 'elapsed_s': round(time.time() - t, 1), 'finished': now()})
        except Cancelled as e:
            rec.update({'status': 'cancelled', 'error': str(e), 'elapsed_s': round(time.time() - t, 1), 'finished': now()}); self._close_stage(d, rec); raise
        except KeyboardInterrupt:
            self._kill_active(); rec.update({'status': 'cancelled', 'error': 'KeyboardInterrupt', 'elapsed_s': round(time.time() - t, 1), 'finished': now()}); self._close_stage(d, rec); raise Cancelled('KeyboardInterrupt')
        except Exception as e:
            rec.update({'status': 'failed', 'error': f'{type(e).__name__}: {e}', 'traceback': traceback.format_exc(), 'elapsed_s': round(time.time() - t, 1), 'finished': now()}); self._close_stage(d, rec)
            if required: raise JobError(f'stage {name} failed: {type(e).__name__}: {e}') from e
            self.log(f'stage {name} failed (optional): {e}'); return rec
        self._close_stage(d, rec); return rec

    def _close_stage(self, d, rec):
        (d / 'receipt.json').write_text(json.dumps(rec, indent=1, ensure_ascii=False), encoding='utf-8'); self.receipts.append(rec)
        self.status['stages'][-1].update({'status': rec['status'], 'elapsed_s': rec.get('elapsed_s')}); self.write_status()
        self.emit('stage_finished', status=rec['status'], stage_elapsed_s=rec.get('elapsed_s'), error=rec.get('error'), receipt=str(d / 'receipt.json'), required=rec.get('required', True))
        self.log(f"stage {rec['index']} {rec['name']} {rec['status']} {rec.get('elapsed_s')} s"); self.current_stage = None

    # ---- subprocesses ----------------------------------------------------------------------------------------
    def _kill_active(self):
        p = self.active_proc
        if p is not None and p.poll() is None:
            try: p.kill()
            except Exception: pass
            try: p.wait(timeout=30)
            except Exception: pass
        self.active_proc = None

    def blender(self, script, args, stage_dir, timeout=3600, poll_s=0.5):
        """Run a Blender python stage script as a background subprocess; stdout/stderr captured next to the receipt.
        The CANCEL file is polled while it runs: on cancellation the process is killed and the stage is recorded
        as cancelled (its partial outputs stay on disk; earlier receipts are untouched)."""
        b = self.job['tools']['blender']['path']
        cmd = blender_command(b, ['--background', '--factory-startup', '--python-exit-code', '1', '--python', str(script), '--', *[str(a) for a in args]])
        self.log('blender:', ' '.join(cmd[-(len(args) + 3):]))
        so = open(stage_dir / 'blender_stdout.txt', 'w', encoding='utf-8', errors='replace'); se = open(stage_dir / 'blender_stderr.txt', 'w', encoding='utf-8', errors='replace')
        t = time.time(); self.check_cancel()
        p = subprocess.Popen(cmd, stdout=so, stderr=se, cwd=str(REPO_ROOT))
        self.active_proc = p; self.emit('stage_progress', message='subprocess started', subprocess={'pid': p.pid, 'tool': 'blender', 'script': str(script)})
        try:
            while True:
                rc = p.poll()
                if rc is not None: break
                if (self.out / CANCEL_FILE).exists():
                    self._kill_active(); so.close(); se.close()
                    self.emit('stage_progress', message='subprocess killed on cancellation', subprocess={'pid': p.pid, 'tool': 'blender', 'killed': True})
                    raise Cancelled(f'{CANCEL_FILE} file present in output directory (blender pid {p.pid} killed)')
                if time.time() - t > timeout:
                    self._kill_active(); so.close(); se.close(); raise RuntimeError(f'blender timed out after {timeout} s (pid {p.pid} killed)')
                time.sleep(poll_s)
        except KeyboardInterrupt:
            self._kill_active(); so.close(); se.close(); raise
        finally:
            self.active_proc = None
        so.close(); se.close()
        self.emit('stage_progress', message='subprocess finished', subprocess={'pid': p.pid, 'tool': 'blender', 'returncode': rc, 'seconds': round(time.time() - t, 1)})
        out = (stage_dir / 'blender_stdout.txt').read_text(encoding='utf-8', errors='replace')
        if rc != 0:
            err = (stage_dir / 'blender_stderr.txt').read_text(encoding='utf-8', errors='replace')
            raise RuntimeError(f'blender exited {rc}; see {stage_dir / "blender_stderr.txt"}: {err[-800:]}')
        return out

    # ---- completion ------------------------------------------------------------------------------------------
    def finish(self, status, handoff=None, note=None):
        self._kill_active()
        self.status['status'] = status; self.status['finished'] = now(); self.status['ready_for_packing'] = bool(status == 'completed' and handoff and handoff.get('status') == 'ready_for_experimental_packing')
        if note: self.status['note'] = note
        if handoff is not None: self.status['handoff'] = str(self.out / 'handoff.json'); self.status['candidate_status'] = handoff.get('status')
        self.write_status(); (self.out / 'status.json').write_text(json.dumps(self.status, indent=1, ensure_ascii=False), encoding='utf-8')
        man = UM.build(self.out, self.status, self.job, self.receipts, handoff=handoff, preview=self.meta.get('preview'), events_path=self.out / 'events.jsonl', note=note)
        (self.out / 'ui_manifest.json').write_text(json.dumps(man, indent=1, ensure_ascii=False), encoding='utf-8'); self.status['manifest'] = str(self.out / 'ui_manifest.json')
        self.emit({'completed': 'job_completed', 'failed': 'job_failed', 'cancelled': 'job_cancelled'}.get(status, 'job_finished'), status=status, ready_for_packing=self.status['ready_for_packing'], candidate_status=self.status.get('candidate_status'), output_dir=str(self.out), manifest=str(self.out / 'ui_manifest.json'), note=note, export_ready=man['export_ready'])
        self.logf.close(); self.evf.close()
        return self.status
