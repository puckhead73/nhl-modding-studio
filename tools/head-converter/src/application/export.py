"""Guarded offline NHL21 native export from a completed worker UI manifest.

python -m src.application.export MANIFEST --out FRESH_DIRECTORY [--exchange JSON] [--run-root DIR] [--blender EXE]
    [--packer-script BUILD_PY] [--packer-exe NATIVE_PACK_EXE] [--preconditions JSON] [--audit AUDIT_PY] [--python EXE]
    [--export-profile DESCRIPTOR.json [--profile-inputs-root DIR]] [--build-receipt build-receipt.json]
No game writes. Every route input is explicit, validated before any write and hashed into the receipt; the defaults
reproduce the demonstrated Ekblad route (Codex phase-3 exchange, local/runs/codex run root, pinned Codex packer script,
native packer executable, import preconditions and audit script). A given path that is missing refuses; nothing falls back.

Two ways to name the native inputs (src/application/export_profile.py):
  compatibility route  `--exchange` (nhl21-phase3-handoff/1, default = the historical Codex handoff) + `--preconditions`;
  profile route        `--export-profile` = a versioned `nhl-head-export-profile/1` descriptor whose relative references (resolved
                       against the descriptor's directory, or `--profile-inputs-root`, never the working directory) name the four
                       native inputs and the import preconditions by hash. The historical handoff is not read; after preflight a
                       compatibility exchange (`<out>/native-exchange.json`, same four pins) is generated for the unchanged packer.
Conflict rules (refused, nothing written): `--export-profile` with `--exchange` or `--preconditions`; `--profile-inputs-root`
without `--export-profile`; `--build-receipt` with `--packer-exe`. `--build-receipt` selects the packer executable through a
validated H2.4 `nhl21-build-receipt/1` (built, empty drift, v2 source manifest, executable hash matches) and records receipt /
source identity / executable hash in `inputs.build`; an invalid selection refuses instead of falling back to the default executable.
The job's destination profile id, strategy and source kind must equal the descriptor's; descriptor bytes, build receipt and source
manifest are pinned like every other input (stability re-checked before completion; copies under evidence/).

Lifecycle: stages pack -> audit -> validate -> publish, each reported in <out>/events.jsonl (`nhl-head-export-event/1`, see
src/application/lifecycle.py; `--events` mirrors them to stdout). Cancellation = a `CANCEL` file inside the export directory or
KeyboardInterrupt; it is checked before every subprocess launch, polled while one runs, and checked again before publication.
Subprocess trees are owned before they run (Windows Job Object, child launched suspended, assigned, then resumed; kill-on-close)
and terminated on cancellation; descendants must be confirmed gone before a stage advances. Ownership is finalised unconditionally
(`OwnedProcess.release`): whatever interrupts the poll loop, the descendant drain, a job-accounting query or the subprocess-exit event
(Cancelled, KeyboardInterrupt, OSError), the owned tree is terminated unless already measured empty and every handle is closed; the
receipt records the MEASURED cleanup (`cancellation.cleanup` / `cleanup`, `cleanup_failure` when emptiness was not confirmed) for
cancelled and failed runs alike - an exited immediate child is never taken as proof that nothing was running.
Terminal states: completed (artifacts published) / failed / cancelled (artifacts always []). Commit point = the atomic replacement
of export.json with the completed receipt (`commit_receipt`): before it any cancellation or failure wins; after it nothing rewrites
the receipt or emits a contradictory terminal - an interruption or event/stdout delivery error after commitment is recorded in
`export.post-commit.json` and the completed result stands (exit 0). The terminal event follows the persisted receipt. A receipt left
at `running` means the exporter died abruptly (owned children are then killed by the closing job handle); it never counts as
completed and no terminal receipt is promised in that case. Exit codes: 0 completed, 2 refused (no directory created), 3 failed,
4 cancelled.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import time

from src.application.export_profile import compatibility_exchange, load_descriptor, select_build_receipt
from src.application.lifecycle import CANCEL_FILE, CLEANUP_TIMEOUT_S, Cancelled, EventLog, OwnedProcess, cancel_requested, stdout_event
from src.application.native_paths import existing_file, fresh_output_beneath, load_preconditions, plan_artifact_names, preconditions_target
from src.conversion.job import blender_command
from src.conversion.ui_manifest import check, load

REPO = Path(__file__).resolve().parents[2]
RUN_ROOT = REPO / 'local/runs/codex'
EXCHANGE = REPO / 'research/codex/nhl21-phase3-handoff.json'
EXCHANGE_SCHEMA = 'nhl21-phase3-handoff/1'
EXCHANGE_PINS = ('exchange', 'native_donor', 'profile', 'texture')
# Job destination reference -> exchange pin it must equal (same hash) for the native writer to accept the route.
JOB_PINS = (('profile', 'profile'), ('exchange', 'exchange'), ('donor_mcd', 'native_donor'), ('native_diffuse_gnf', 'texture'))
BLENDER = Path('C:/Program Files/Blender Foundation/Blender 4.5/blender.exe')
PACKER_SCRIPT = REPO / 'experiments/codex/nhl21_package/build.py'
PACKER_EXE = RUN_ROOT / 'nhl21-candidate-build/debug/nhl21-candidate-pack.exe'
PRECONDITIONS = REPO / 'research/codex/nhl21-phase3-import-preconditions.json'
AUDIT = REPO / 'experiments/codex/nhl21_package/audit_candidate.py'
RECEIPT_ARTIFACTS = ('validation.json', 'final-audit.json')
STAGES = ('pack', 'audit', 'validate', 'publish')
POLL_S = 0.2
EXIT_CODES = {'completed': 0, 'refused': 2, 'failed': 3, 'cancelled': 4}
POST_COMMIT_NOTE = 'export.post-commit.json'
GENERATED_EXCHANGE = 'native-exchange.json'   # profile route: compatibility exchange written for the packer beneath the export directory


def commit_receipt(receipt, output):
    """THE completion commit point: atomically replace export.json with the completed receipt. Returns after the replace."""
    pending = Path(output) / 'export.pending.json'
    pending.write_text(json.dumps(receipt, indent=2), encoding='utf-8')
    pending.replace(Path(output) / 'export.json')


def committed(output):
    """Truth about the commit point is the persisted file, not an in-memory flag: an interrupt landing between the atomic
    replace and any bookkeeping must still see a committed export."""
    try: return json.loads((Path(output) / 'export.json').read_text(encoding='utf-8')).get('status') == 'completed'
    except (OSError, ValueError): return False


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def _pin(ref, what):
    """A {path, sha256} reference whose file exists and hashes as declared."""
    if not isinstance(ref, dict) or not isinstance(ref.get('path'), str) or not isinstance(ref.get('sha256'), str):
        raise ValueError(f'{what}: reference needs string path and sha256')
    path = Path(ref['path'])
    if not path.is_file(): raise ValueError(f'{what}: missing file {path}')
    if sha(path) != ref['sha256']: raise ValueError(f'{what}: file {path} does not match its declared sha256')
    return {'path': str(path), 'sha256': ref['sha256']}


def resolve_inputs(manifest_path, output, exchange=None, run_root=None, blender=None, packer_script=None, packer_exe=None,
                   preconditions=None, audit=None, python=None, export_profile=None, profile_inputs_root=None, build_receipt=None):
    """Resolve every route input (defaults = the demonstrated route) and refuse unsafe destinations, competing overrides, invalid
    descriptors / build receipts and missing tools before anything is read from the manifest. Returns resolved paths plus, under
    'export_profile' and 'build', the validated descriptor / build selection (None when not used); nothing is written."""
    if export_profile is not None:
        competing = [flag for flag, value in (('--exchange', exchange), ('--preconditions', preconditions)) if value is not None]
        if competing: raise ValueError(f'--export-profile conflicts with {", ".join(competing)}: the descriptor names the native inputs and import preconditions')
    elif profile_inputs_root is not None: raise ValueError('--profile-inputs-root requires --export-profile')
    if build_receipt is not None and packer_exe is not None:
        raise ValueError('--build-receipt conflicts with --packer-exe: the receipt names the executable')
    descriptor = load_descriptor(export_profile, profile_inputs_root) if export_profile is not None else None
    build = select_build_receipt(build_receipt, descriptor['reviewed_executables'] if descriptor else None) if build_receipt is not None else None
    inputs = {'manifest': Path(manifest_path), 'output': Path(output),
              'exchange': None if descriptor else Path(exchange or EXCHANGE),
              'run_root': Path(run_root or RUN_ROOT), 'blender': Path(blender or BLENDER), 'packer_script': Path(packer_script or PACKER_SCRIPT),
              'packer_exe': Path(build['executable']['path']) if build else Path(packer_exe or PACKER_EXE),
              'preconditions': Path(descriptor['preconditions']['path']) if descriptor else Path(preconditions or PRECONDITIONS),
              'audit': Path(audit or AUDIT), 'python': Path(python or sys.executable)}
    inputs = {k: (v.resolve() if v is not None else None) for k, v in inputs.items()}
    inputs['output'] = fresh_output_beneath(inputs['run_root'], inputs['output'])
    for key, what, suffix in (('blender', 'blender executable', None), ('python', 'python executable', None), ('packer_exe', 'packer_exe executable', None),
                              ('packer_script', 'packer_script', '.py'), ('audit', 'audit script', '.py'), ('preconditions', 'preconditions', '.json'),
                              ('exchange', 'exchange', '.json')):
        if inputs[key] is not None: existing_file(inputs[key], what, suffix)
    inputs['export_profile'] = descriptor
    inputs['build'] = build
    return inputs


def load_exchange(path):
    """The native writer's exchange: schema-checked, every pinned native input present and hash-verified."""
    try: exchange = load(path)
    except (OSError, ValueError) as exc: raise ValueError(f'Exchange {path} is not readable JSON: {exc}') from None
    if not isinstance(exchange, dict) or exchange.get('schema') != EXCHANGE_SCHEMA:
        raise ValueError(f'Exchange {path} is not {EXCHANGE_SCHEMA}')
    return {key: _pin(exchange.get(key), f'exchange.{key}') for key in EXCHANGE_PINS}


def preflight(manifest_path, output, blender=None, **overrides):
    """Refuse incomplete/blocked jobs, foreign profiles and unsafe destinations before writing.
    Returns (manifest, inputs, pins) where pins are the hash-verified route inputs recorded in the receipt."""
    inputs = resolve_inputs(manifest_path, output, blender=blender, **overrides)
    manifest = load(inputs['manifest'])
    ok, problems = check(manifest)
    if not ok: raise ValueError('Invalid worker manifest: ' + '; '.join(problems))
    if manifest['export_ready'] is not True: raise ValueError('Worker candidate is not export-ready')
    job = load(manifest['files']['job_json']['path'])
    descriptor = inputs['export_profile']
    if descriptor is not None:
        # Profile route: the descriptor's hash-verified references are the exchange pins; the job must name the same route.
        exchange = descriptor['native_inputs']
        if job['destination'].get('profile_id') != descriptor['native_adapter']['profile_id']:
            raise ValueError(f"Job destination profile_id {job['destination'].get('profile_id')!r} disagrees with export profile "
                             f"{descriptor['profile_id']} v{descriptor['version']} (native adapter {descriptor['native_adapter']['profile_id']!r})")
        if (job['source']['kind'], job['strategy']) != (descriptor['source']['kind'], descriptor['source']['strategy']):
            raise ValueError(f"Job route ({job['source']['kind']}, {job['strategy']}) is not the export profile route {descriptor['source']}")
    else: exchange = load_exchange(inputs['exchange'])
    # Pin the route to the same donor, exchange and profile accepted by the native writer.
    destination = {}
    for key, native_key in JOB_PINS:
        ref = job['destination'][key]
        if ref['sha256'] != exchange[native_key]['sha256'] or sha(ref['path']) != ref['sha256']:
            raise ValueError(f'Unsupported or changed destination {key}')
        destination[key] = {'path': ref['path'], 'sha256': ref['sha256']}
    if job['strategy'] != 'donor_fit' or job['source']['kind'] != 'cc5_headshot_head_only_fbx':
        raise ValueError('Unsupported native export route')
    # The import preconditions must name exactly the qualified donor and texture archive targets of the pinned profile.
    profile = descriptor['native_profile'] if descriptor is not None else load(exchange['profile']['path'])
    preconditions = load_preconditions(inputs['preconditions'])
    targets = {}
    for key, asset_path, decoded in (('donor', profile['archive_provenance']['asset_path'], profile['donor']['sha256']),
                                     ('texture', profile['diffuse']['provenance']['asset_path'], profile['diffuse']['provenance']['decoded_sha256'])):
        targets[key] = {'asset_path': asset_path, 'decoded_sha256': preconditions_target(preconditions, asset_path, decoded)['decoded_sha256']}
    if descriptor is not None:   # the generated exchange file's path / sha256 are filled in by export() once the output directory exists
        exchange_pin = {'generated_from': 'export_profile', 'schema': EXCHANGE_SCHEMA, 'pins': exchange}
        export_profile = {'path': descriptor['path'], 'sha256': descriptor['sha256'], 'bytes': descriptor['bytes'], 'schema': descriptor['descriptor']['schema'],
                          'profile_id': descriptor['profile_id'], 'version': descriptor['version'], 'inputs_root': descriptor['inputs_root'],
                          'native_adapter': descriptor['native_adapter'], 'source': descriptor['source'], 'outputs': descriptor['outputs']}
    else:
        exchange_pin = {'path': str(inputs['exchange']), 'sha256': sha(inputs['exchange']), 'pins': exchange}
        export_profile = None
    build = inputs['build'] or {'selection': 'explicit --packer-exe or the accepted default executable', 'receipt': None,
                               'provenance': 'no build receipt selected; the executable is identified by its hash under tools.packer_exe only'}
    pins = {'exchange': exchange_pin, 'export_profile': export_profile, 'build': build,
            'destination': {'profile_id': job['destination'].get('profile_id'), **destination},
            'preconditions': {'path': str(inputs['preconditions']), 'sha256': sha(inputs['preconditions']), 'targets': targets},
            'tools': {key: {'path': str(inputs[key]), 'sha256': sha(inputs[key])} for key in ('blender', 'packer_script', 'packer_exe', 'audit')},
            'python': str(inputs['python']), 'run_root': str(inputs['run_root'])}
    return manifest, inputs, pins


def pinned_inputs_stable(pins):
    """Every hashed route input (exchange file and its pins, descriptor, build receipt + source manifest, destination pins, tools,
    preconditions) still hashes as at preflight."""
    refs = [('exchange', pins['exchange']), ('preconditions', pins['preconditions'])]
    refs += [(f'exchange.{k}', v) for k, v in pins['exchange']['pins'].items()]
    if pins.get('export_profile'): refs.append(('export_profile', pins['export_profile']))
    if pins.get('build', {}).get('receipt'): refs += [('build.receipt', pins['build']['receipt']), ('build.source_manifest', pins['build']['source_manifest'])]
    refs += [(f'destination.{k}', v) for k, v in pins['destination'].items() if isinstance(v, dict)]
    refs += [(f'tools.{k}', v) for k, v in pins['tools'].items()]
    changed = [name for name, ref in refs if not isinstance(ref.get('path'), str) or not Path(ref['path']).is_file() or sha(ref['path']) != ref['sha256']]
    if changed: raise ValueError('Pinned inputs changed during export: ' + ', '.join(changed))


def export(manifest_path, output, blender=None, events_to_stdout=False, poll_s=POLL_S, **overrides):
    """Run the guarded export. Returns the completed receipt; raises Cancelled / the failure exception after persisting the
    terminal receipt (available as `exc.export_receipt`). Preflight refusals raise before any directory exists."""
    manifest, inputs, pins = preflight(manifest_path, output, blender, **overrides)
    manifest_path, output = inputs['manifest'], inputs['output']
    manifest_sha = sha(manifest_path)
    output.mkdir(parents=True, exist_ok=False)
    exchange_path = inputs['exchange']
    if inputs['export_profile'] is not None:
        # Profile route: the packer keeps its existing input shape; it reads this generated file, never the historical handoff.
        exchange_path = output / GENERATED_EXCHANGE
        exchange_path.write_text(json.dumps(compatibility_exchange(inputs['export_profile']), indent=2), encoding='utf-8')
        pins['exchange'].update({'path': str(exchange_path), 'sha256': sha(exchange_path)})
    export_id = f'{manifest["run_id"]}->{output.name}' if isinstance(manifest.get('run_id'), str) else output.name
    events = EventLog(output / 'events.jsonl', export_id, output.name, to_stdout=events_to_stdout)
    via = (f" via export profile {pins['export_profile']['profile_id']} v{pins['export_profile']['version']}" if pins['export_profile'] else '') + \
          (f" with build receipt executable {pins['build']['executable']['sha256'][:16]}" if pins['build'].get('receipt') else '')
    receipt = {'schema': 'nhl-head-native-export/1', 'status': 'running', 'export_id': export_id, 'pid': os.getpid(),
               'manifest': {'path': str(manifest_path), 'sha256': manifest_sha}, 'inputs': pins,
               'scope': f"Offline NHL21 native export for profile {pins['destination']['profile_id']}{via}; no installation or new runtime acceptance",
               'events_jsonl': {'path': str(output / 'events.jsonl'), 'live': True}, 'cancel_file': str(output / CANCEL_FILE),
               'stages': [], 'commands': [], 'artifacts': [], 'evidence': []}
    state = {'stage': None, 'proc': None, 'cancel_source': None, 'tree': None, 'cleanup': None, 'committed': False}

    def save():
        pending = output / 'export.pending.json'
        pending.write_text(json.dumps(receipt, indent=2), encoding='utf-8')
        pending.replace(output / 'export.json')

    def check_cancel(where):
        if cancel_requested(output):
            state['cancel_source'] = f'{CANCEL_FILE} file'; raise Cancelled(f'{CANCEL_FILE} file present in export directory ({where})')

    def begin(name):
        check_cancel(f'before stage {name}')
        state['stage'] = {'name': name, 'index': STAGES.index(name) + 1}
        receipt['stages'].append({**state['stage'], 'status': 'running', 'started': time.strftime('%Y-%m-%dT%H:%M:%S')}); save()
        events.emit('stage_started', stage=state['stage'])

    def end(status, **fields):
        if state['stage'] is None: return
        receipt['stages'][-1].update({'status': status, **fields}); save()
        events.emit('stage_finished', stage=state['stage'], status=status, **fields); state['stage'] = None

    def run(command, log, tool):
        """One owned subprocess: cancellation is checked before launch and polled while it runs. Ownership is released in a
        `finally` that runs whatever raised - in the poll loop, the descendant drain, an accounting query or the exit event - so a
        job can never stay open, and `state['cleanup']` then holds the measured result for the terminal receipt."""
        check_cancel(f'before launching {tool}')
        receipt['commands'].append([str(v) for v in command]); save()
        with (output / log).open('w', encoding='utf-8') as stream:
            proc = OwnedProcess(command, stream, REPO); state['proc'] = proc
            settled = reported = False   # settled: tree confirmed empty and its exit event delivered; reported: exit event attempted
            try:
                events.emit('subprocess_started', stage=state['stage'], subprocess={'pid': proc.pid, 'tool': tool, 'log': log, 'ownership': proc.ownership})
                while proc.poll() is None:
                    if cancel_requested(output):
                        state['cancel_source'] = f'{CANCEL_FILE} file'; raise Cancelled(f'{CANCEL_FILE} file present in export directory (while {tool} ran, pid {proc.pid})')
                    time.sleep(poll_s)
                # The child exited; the stage may only advance once every owned descendant is confirmed gone.
                tree = proc.wait_tree(); state['tree'] = tree
                if not tree.get('confirmed'):
                    raise RuntimeError(f'{log}: owned descendants of pid {proc.pid} not confirmed terminated within {CLEANUP_TIMEOUT_S} s: {tree}')
                reported = True
                events.emit('subprocess_exited', stage=state['stage'], subprocess={'pid': proc.pid, 'tool': tool, 'returncode': proc.poll(),
                                                                                   'seconds': round(time.time() - proc.started, 3), 'tree': tree})
                settled = True
            finally:
                if not settled: state['cleanup'] = {'pid': proc.pid, 'confirmed': False, 'note': 'cleanup interrupted before it could report'}
                try: cleanup = proc.release()   # terminates unless measured empty; always closes the job handle
                finally: state['proc'] = None
                if not settled:
                    state['cleanup'] = cleanup
                    if not reported:
                        try: events.emit('subprocess_exited', stage=state['stage'], subprocess={'pid': proc.pid, 'tool': tool, 'returncode': proc.poll(),
                                                                                                'killed': cleanup['terminated'], 'cleanup': cleanup})
                        except Exception: pass   # the cancellation/failure outcome is recorded by the caller regardless of event delivery
        if proc.poll(): raise RuntimeError(f'{log}: process exit {proc.poll()}')

    save()
    events.emit('export_started', status='running', output_dir=str(output), cancel_file=receipt['cancel_file'], pid=os.getpid(), planned_stages=list(STAGES),
                manifest=receipt['manifest'], profile_id=pins['destination']['profile_id'],
                export_profile=({k: pins['export_profile'][k] for k in ('profile_id', 'version', 'sha256')} if pins['export_profile'] else None),
                build_receipt=(pins['build']['receipt']['sha256'] if pins['build'].get('receipt') else None))
    native = output / 'nhl21-phase-native'
    try:
        artifacts = manifest['artifacts']
        begin('pack')
        run(blender_command(inputs['blender'], ['-b', '--factory-startup', '--python-exit-code', '1', '--python', inputs['packer_script'], '--',
                                                '--exchange', exchange_path, '--candidate-handoff', artifacts['handoff_json']['path'],
                                                '--input', artifacts['native_mesh_json']['path'], '--atlas', artifacts['atlas_png']['path'],
                                                '--out', native, '--run-root', inputs['run_root'], '--packer-exe', inputs['packer_exe'],
                                                '--preconditions', inputs['preconditions']]), 'pack.log', 'blender')
        end('ok')
        begin('audit')
        run([str(inputs['python']), '-B', str(inputs['audit']), str(native), '--offline', '--run-root', str(inputs['run_root'])], 'audit.log', 'audit')
        end('ok')
        begin('validate')
        # Check for concurrent source changes before promoting the export to completed.
        if sha(manifest_path) != manifest_sha: raise ValueError('Manifest changed during export')
        ok, problems = check(manifest)
        if not ok: raise ValueError('Worker artifacts changed during export: ' + '; '.join(problems))
        pinned_inputs_stable(pins)
        # Evidence snapshots are kept even if the native outputs fail validation below.
        evidence = output / 'evidence'; evidence.mkdir()
        sources = [('ui_manifest.json', manifest_path), ('job.json', Path(manifest['files']['job_json']['path'])),
                   ('status.json', Path(manifest['files']['status_json']['path'])),
                   ('handoff.json', Path(artifacts['handoff_json']['path'])),
                   ('loss_report.json', Path(artifacts['loss_report_json']['path']))]
        sources += [(f'stages/{i:02d}/receipt.json', Path(s['receipt']['path'])) for i, s in enumerate(manifest['stages'])]
        if pins['export_profile']: sources.append(('export-profile.json', Path(pins['export_profile']['path'])))
        if pins['build'].get('receipt'): sources += [('build-receipt.json', Path(pins['build']['receipt']['path'])), ('build-source-manifest.json', Path(pins['build']['source_manifest']['path']))]
        for name, source in sources:
            dest = evidence / name; dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, dest)
            receipt['evidence'].append({'path': str(dest.relative_to(output)), 'sha256': sha(dest)})
        # The native file names come from the packer's own installation plan, not from this module.
        names = plan_artifact_names(load(native / 'installation-plan.json'), native)
        produced = []
        for name in (*names, *RECEIPT_ARTIFACTS):
            path = existing_file(native / name, f'native output {name}')
            produced.append({'path': str(path.relative_to(output)), 'sha256': sha(path), 'bytes': path.stat().st_size})
        end('ok', checked={'manifest': True, 'worker_artifacts': True, 'pinned_inputs': True, 'native_outputs': len(produced)})
        begin('publish')
        # Completion boundary: a cancellation observed here still wins; the commit below is the last point where anything can.
        check_cancel('before publication')
        receipt['artifacts'] = produced
        receipt['status'] = 'completed'
        receipt['stages'][-1].update({'status': 'ok', 'published': len(produced)})
        commit_receipt(receipt, output); state['committed'] = True
        # ---- committed: from here on the receipt is immutable and only delivery of the two remaining events can fail ----
        events.emit('stage_finished', stage=state['stage'], status='ok', published=len(produced)); state['stage'] = None
        events.emit('export_completed', status='completed', artifacts=len(produced), receipt=str(output / 'export.json'))
    except BaseException as exc:
        if state['committed'] or committed(output):
            # Interrupt or event/stdout delivery failure after the commit point: the completed receipt stands; report the limitation.
            note = {'schema': 'nhl-head-export-post-commit/1', 'committed_status': 'completed', 'receipt': str(output / 'export.json'),
                    'interruption': f'{type(exc).__name__}: {exc}', 'terminal_event_written': events.terminal_written,
                    'terminal_event_claimed': events.terminal, 'terminal_delivery': events.terminal_delivery,
                    'events_written': events.seq, 'note': 'the committed receipt is authoritative; no second terminal outcome was emitted'}
            try: (output / POST_COMMIT_NOTE).write_text(json.dumps(note, indent=2), encoding='utf-8')
            except OSError: pass
            receipt['post_commit'] = note   # in-memory only; the persisted receipt is not rewritten
            events.close()
            return receipt
        cancelled = isinstance(exc, (Cancelled, KeyboardInterrupt))
        if isinstance(exc, KeyboardInterrupt): state['cancel_source'] = 'KeyboardInterrupt'
        if state['proc'] is not None:   # only reachable if an interrupt landed between ownership and `run`'s own finaliser
            try: state['cleanup'] = state['proc'].release()
            finally: state['proc'] = None
        cleanup = state['cleanup']   # the measured finalisation of the subprocess live at the failure, or None if none was
        receipt['status'] = 'cancelled' if cancelled else 'failed'
        receipt['artifacts'] = []
        receipt['error'] = f'{type(exc).__name__}: {exc}'
        if cancelled:
            receipt['cancellation'] = {'requested_by': state['cancel_source'], 'observed_stage': state['stage']['name'] if state['stage'] else 'between stages',
                                       'cleanup': cleanup if cleanup is not None else {'confirmed': True, 'note': 'no subprocess was running'}}
        elif cleanup is not None: receipt['cleanup'] = cleanup
        if cleanup is not None and not cleanup.get('confirmed'): receipt['cleanup_failure'] = 'owned process tree not confirmed terminated within the cleanup bound'
        if state['stage'] is not None: receipt['stages'][-1].update({'status': receipt['status'], 'error': receipt['error']})
        save()   # authoritative terminal receipt first ...
        try:      # ... then the events; a delivery failure here must not hide the persisted outcome
            if state['stage'] is not None: events.emit('stage_finished', stage=state['stage'], status=receipt['status'], error=receipt['error']); state['stage'] = None
            events.emit({'cancelled': 'export_cancelled', 'failed': 'export_failed'}[receipt['status']], status=receipt['status'], error=receipt['error'],
                        cancellation=receipt.get('cancellation'), cleanup=receipt.get('cleanup'), cleanup_failure=receipt.get('cleanup_failure'),
                        receipt=str(output / 'export.json'))
        except Exception as delivery:
            try: (output / POST_COMMIT_NOTE).write_text(json.dumps({'schema': 'nhl-head-export-post-commit/1', 'committed_status': receipt['status'],
                                                                    'delivery_failure': f'{type(delivery).__name__}: {delivery}', 'terminal_event_claimed': events.terminal,
                                                                    'terminal_delivery': events.terminal_delivery, 'terminal_event_written': events.terminal_written},
                                                                   indent=2), encoding='utf-8')
            except OSError: pass
        events.close()
        exc.export_receipt = receipt
        if isinstance(exc, KeyboardInterrupt):
            wrapped = Cancelled('KeyboardInterrupt'); wrapped.export_receipt = receipt; raise wrapped from exc
        raise
    events.close()
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('manifest', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    # Route-defining options default to None so that an explicitly given competing override can be told from the default.
    parser.add_argument('--exchange', type=Path, default=None, help=f'native writer exchange (nhl21-phase3-handoff/1); default {EXCHANGE}; conflicts with --export-profile')
    parser.add_argument('--export-profile', type=Path, default=None, help='versioned nhl-head-export-profile/1 descriptor naming the native inputs + preconditions by hash')
    parser.add_argument('--profile-inputs-root', type=Path, default=None, help="explicit binding: directory the descriptor's relative references resolve against (default: the descriptor's directory)")
    parser.add_argument('--build-receipt', type=Path, default=None, help='H2.4 nhl21-build-receipt/1 selecting the packer executable (validated); conflicts with --packer-exe')
    parser.add_argument('--run-root', type=Path, default=RUN_ROOT, help='existing directory the output must lie beneath')
    parser.add_argument('--blender', type=Path, default=BLENDER)
    parser.add_argument('--packer-script', type=Path, default=PACKER_SCRIPT, help='Blender-side native packer script (Python)')
    parser.add_argument('--packer-exe', type=Path, default=None, help=f'native packer executable (pack/verify); default {PACKER_EXE}')
    parser.add_argument('--preconditions', type=Path, default=None, help=f'import-preconditions JSON for the qualified archive targets; default {PRECONDITIONS}; conflicts with --export-profile')
    parser.add_argument('--audit', type=Path, default=AUDIT, help='offline native audit script')
    parser.add_argument('--python', type=Path, default=Path(sys.executable), help='interpreter for the audit script (needs numpy)')
    parser.add_argument('--events', action='store_true', help='mirror nhl-head-export-event/1 lines to stdout (refusals emit export_refused)')
    args = parser.parse_args()
    try:
        receipt = export(args.manifest, args.out, args.blender, events_to_stdout=args.events, exchange=args.exchange, run_root=args.run_root,
                         packer_script=args.packer_script, packer_exe=args.packer_exe, preconditions=args.preconditions, audit=args.audit, python=args.python,
                         export_profile=args.export_profile, profile_inputs_root=args.profile_inputs_root, build_receipt=args.build_receipt)
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, Cancelled, KeyboardInterrupt) as exc:
        receipt = getattr(exc, 'export_receipt', None) or getattr(exc.__cause__, 'export_receipt', None)
        if receipt is None:
            if args.events: stdout_event('export_refused', status='refused', error=str(exc))
            print(json.dumps({'status': 'refused', 'error': str(exc)})); return EXIT_CODES['refused']
        print(json.dumps({'status': receipt['status'], 'error': receipt.get('error'), 'receipt': str(args.out / 'export.json')})); return EXIT_CODES[receipt['status']]
    print(json.dumps({'status': receipt['status'], 'receipt': str(args.out / 'export.json')})); return EXIT_CODES['completed']


if __name__ == '__main__':
    sys.exit(main())
