"""Experimental conversion-worker entry point: consume an `nhl-head-job/1` JSON and run the CC5 -> NHL21 (atlas-A
donor) `donor_fit` conversion stages through src/conversion (numerics) and Blender background subprocesses
(source extraction, bake, renders).

python experiments/claude/worker/run_job.py <job.json> [--out-dir NAME] [--events] [--skip-render] [--skip-measure] [--dry-run]

Outputs (fresh directory <run_root>/<dir>, refused if it exists): job.json, progress.json, log.txt (diagnostic text),
events.jsonl (machine-readable `nhl-head-worker-event/1`, one JSON object per line), stages/NN_<name>/receipt.json
(+ stage outputs), status.json, handoff.json (Codex packer layout), loss_report.json, preview.glb, ui_manifest.json.
--events: print the same JSON-lines events on stdout (and nothing else) for an application to consume; without it
the human log lines and a final JSON summary are printed as before.
Cancellation: create <out>/CANCEL (path given in the job_started event / status.json) or Ctrl-C. A running Blender
subprocess is killed; completed stage receipts are preserved; the job ends `cancelled` (exit 3).
Exit code 0 completed, 2 failed, 3 cancelled, 4 invalid job. This is the conversion portion of the worker only: no
native packing, no installation, no game claims.
"""
import json, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from src.conversion import job as J, stages_nhl21 as S

PLANNED = ['validate', 'extract_source', 'register_fit', 'neck_transition', 'classify_after_neck', 'collision_repair', '[classify_after_repair]', 'eye_place', 'bake', '[measure]', '[render]', 'freeze_candidate', 'preview']
args = sys.argv[1:]; events = '--events' in args


def invalid(problems, job_path, job_id=None):
    ev = {'schema': J.EVENT_SCHEMA, 'seq': 0, 'ts': J.now(), 'job_id': job_id, 'run_id': None, 'event': 'job_invalid', 'status': 'invalid', 'problems': problems, 'job_path': str(job_path)}
    print(json.dumps(ev if events else {'error': 'invalid job', 'problems': problems}, indent=None if events else 1, ensure_ascii=False), flush=True); sys.exit(4)


if not args or args[0].startswith('--'): print(__doc__); sys.exit(4)
job_path = Path(args[0])
try:
    job = json.loads(job_path.read_text(encoding='utf-8'))
except FileNotFoundError: invalid([f'job file not found: {job_path}'], job_path)
except (ValueError, UnicodeDecodeError) as e: invalid([f'job file is not valid JSON: {e}'], job_path)
if not isinstance(job, dict): invalid(['job must be a JSON object'], job_path)
if '--out-dir' in args:   # fresh output directory name override (the job file itself is left untouched; the override is recorded in the run's job.json)
    if not isinstance(job.get('output'), dict): invalid(['output block missing; --out-dir cannot apply'], job_path, job.get('job_id'))
    job['output']['dir'] = args[args.index('--out-dir') + 1]; job.setdefault('overrides', {})['output.dir'] = job['output']['dir']
problems = J.validate_job(job)
if '--dry-run' in args: print(json.dumps({'valid': not problems, 'problems': problems}, indent=1)); sys.exit(0 if not problems else 4)
if problems: invalid(problems, job_path, job.get('job_id'))
try:
    R = J.Runner(job, job_path, events_to_stdout=events, planned_stages=PLANNED)
except Exception as e:
    invalid([f'{type(e).__name__}: {e}'], job_path, job.get('job_id'))
prev = {}; receipts = []
try:
    receipts.append(R.run_stage('validate', S.stage_validate(job, R), inputs={'exchange': job['destination']['exchange']['path']}))
    r = R.run_stage('extract_source', S.stage_extract(job, R)); receipts.append(r); prev['source_geometry'] = Path(r['outputs']['source_geometry']['path'])
    r = R.run_stage('register_fit', S.stage_fit(job, R, prev), inputs={'exchange': job['destination']['exchange']['path'], 'source_geometry': prev['source_geometry'], 'tps_ledger': job['calibration']['tps_ledger']['path']}, params=job['policies'].get('fit', {})); receipts.append(r)
    prev.update({'fit_native_mesh': Path(r['outputs']['native_mesh']['path']), 'native_mesh': Path(r['outputs']['native_mesh']['path']), 'ledger': Path(r['outputs']['ledger']['path']), 'masks': Path(r['outputs']['masks']['path'])})
    r = R.run_stage('neck_transition', S.stage_neck(job, R, prev), inputs={'fit_native_mesh': prev['native_mesh'], 'ledger': prev['ledger']}, params=job['policies'].get('neck', {})); receipts.append(r); prev['native_mesh'] = Path(r['outputs']['native_mesh']['path'])
    r = R.run_stage('classify_after_neck', S.stage_classify(job, R, prev), inputs={'fit_native_mesh': prev['fit_native_mesh'], 'native_mesh': prev['native_mesh']}); receipts.append(r)
    prev.update({'mouth_islands': Path(r['outputs']['mouth_islands']['path']), 'collision_triggers': Path(r['outputs']['collision_triggers']['path'])})
    r = R.run_stage('collision_repair', S.stage_repair(job, R, prev), inputs={'native_mesh': prev['native_mesh'], 'collision_triggers': prev['collision_triggers']}, params=job['policies'].get('collision_repair', {})); receipts.append(r)
    if r['summary']['changed_count']:
        prev['native_mesh'] = Path(r['outputs']['native_mesh']['path'])
        r2 = R.run_stage('classify_after_repair', S.stage_classify(job, R, prev, 'classify_after_repair'), inputs={'fit_native_mesh': prev['fit_native_mesh'], 'native_mesh': prev['native_mesh']}); receipts.append(r2)
    r = R.run_stage('eye_place', S.stage_eyes(job, R, prev), inputs={'native_mesh': prev['native_mesh']}, params=job['policies'].get('eyes', {})); receipts.append(r); prev['native_mesh'] = Path(r['outputs']['native_mesh']['path'])
    r = R.run_stage('bake', S.stage_bake(job, R, prev), inputs={'native_mesh': prev['native_mesh'], 'cc5_diffuse': job['source']['diffuse']['path'], 'native_diffuse': job['destination']['native_diffuse_png']['path']}, params=job['policies'].get('bake', {})); receipts.append(r); prev['atlas'] = Path(r['outputs']['atlas']['path'])
    if '--skip-measure' not in args: receipts.append(R.run_stage('measure', S.stage_measure(job, R, prev), inputs={'native_mesh': prev['native_mesh']}, required=False))
    if '--skip-render' not in args and job.get('render', True): receipts.append(R.run_stage('render', S.stage_render(job, R, prev), inputs={'native_mesh': prev['native_mesh'], 'atlas': prev['atlas']}, required=False))
    r = R.run_stage('freeze_candidate', S.stage_freeze(job, R, prev, receipts), inputs={'native_mesh': prev['native_mesh'], 'atlas': prev['atlas']}); receipts.append(r)
    hand = json.loads((R.out / 'handoff.json').read_text(encoding='utf-8'))
    receipts.append(R.run_stage('preview', S.stage_preview(job, R, prev, hand), inputs={'native_mesh': hand['artifacts']['native_mesh_json'], 'atlas': hand['artifacts']['atlas_png']}, required=False))
    st = R.finish('completed', handoff=hand)
    if not events: print(json.dumps({'status': st['status'], 'ready_for_packing': st['ready_for_packing'], 'handoff_status': hand['status'], 'problems': hand['blocking_problems'], 'output_dir': st['output_dir'], 'manifest': st['manifest'], 'elapsed_s': st['elapsed_s']}, indent=1))
    sys.exit(0)
except J.Cancelled as e:
    st = R.finish('cancelled', note=str(e))
    if not events: print(json.dumps({'status': 'cancelled', 'note': str(e), 'output_dir': st['output_dir'], 'manifest': st['manifest']}))
    sys.exit(3)
except J.JobError as e:
    st = R.finish('failed', note=str(e))
    if not events: print(json.dumps({'status': 'failed', 'error': str(e), 'output_dir': st['output_dir'], 'manifest': st['manifest']}))
    sys.exit(2)
except Exception as e:
    import traceback; (R.out / 'traceback.txt').write_text(traceback.format_exc(), encoding='utf-8'); st = R.finish('failed', note=f'{type(e).__name__}: {e}')
    if not events: print(json.dumps({'status': 'failed', 'error': f'{type(e).__name__}: {e}', 'output_dir': st['output_dir'], 'manifest': st['manifest']}))
    sys.exit(2)
