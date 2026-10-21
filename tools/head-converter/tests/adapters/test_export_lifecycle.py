"""Export lifecycle (H2.3): events.jsonl stream, CANCEL-file and KeyboardInterrupt cancellation at every boundary, owned
process-tree cleanup (packer + long-lived descendant), abrupt parent death, CLI exit codes / stdout events, the completion commit
boundary (H2.3a), ownership-before-execution (H2.3a) and exception-safe cleanup inside the drain itself (H2.3b). Stand-in tools only
(tests/adapters/fake_native_tools.py); no Blender or game assets. Synchronisation uses explicit started/ready signal files."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from src.application import export as exporter
from src.application import lifecycle
from tests.adapters.test_native_export import FAKE, build_route, sha

WAIT_S = 20.0


def wait_for(predicate, timeout=WAIT_S, what='condition'):
    t = time.time()
    while time.time() - t < timeout:
        if predicate(): return True
        time.sleep(0.05)
    raise AssertionError(f'timed out waiting for {what}')


def read_events(output):
    lines = (Path(output) / 'events.jsonl').read_text(encoding='utf-8').splitlines()
    return [json.loads(l) for l in lines if l.strip()]


def stopped_writing(path, quiet_s=0.6):
    """True when the heartbeat file has not grown for quiet_s (the writer is gone)."""
    size = Path(path).stat().st_size if Path(path).exists() else -1
    time.sleep(quiet_s)
    return (Path(path).stat().st_size if Path(path).exists() else -1) == size


class LifecycleCase(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=exporter.RUN_ROOT, prefix='lifecycle-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.route = build_route(self.root)
        self.tools = self.route['tools']
        self.signals = self.root / 'signals'; self.signals.mkdir()
        self.check = patch.object(exporter, 'check', return_value=(True, [])); self.check.start(); self.addCleanup(self.check.stop)
        self.env = patch.dict(os.environ, {'FAKE_NATIVE_SIGNAL_DIR': str(self.signals)}); self.env.start(); self.addCleanup(self.env.stop)
        self.addCleanup(self.kill_leftovers)
        self.leftover_pids = []

    def kill_leftovers(self):
        for pid in self.leftover_pids:
            if lifecycle.process_alive(pid):
                subprocess.run(['taskkill', '/PID', str(pid), '/T', '/F'], capture_output=True) if os.name == 'nt' else os.kill(pid, 9)

    def run_export(self, mode, output, **overrides):
        with patch.dict(os.environ, {'FAKE_NATIVE_MODE': mode}):
            return exporter.export(self.route['manifest'], output, poll_s=0.05, **{**self.tools, **overrides})

    def signal(self, name):
        path = self.signals / name
        wait_for(path.exists, what=name)
        return json.loads(path.read_text()) if name.endswith('.json') else path

    # ---- common assertions ----------------------------------------------------------------------------------------
    def assert_consistent(self, output, receipt, status, terminal):
        events = read_events(output)
        self.assertEqual([e['seq'] for e in events], list(range(1, len(events) + 1)), 'monotonic sequence')
        self.assertTrue(all(e['schema'] == lifecycle.EVENT_SCHEMA and e['run_id'] == output.name and e['export_id'] == receipt['export_id'] for e in events))
        terminals = [e for e in events if e['event'] in ('export_completed', 'export_failed', 'export_cancelled')]
        self.assertEqual([e['event'] for e in terminals], [terminal], 'exactly one terminal event')
        self.assertIs(events[-1], terminals[0], 'terminal event is last')
        self.assertEqual(events[0]['event'], 'export_started')
        self.assertEqual((receipt['status'], terminals[0]['status']), (status, status))
        self.assertEqual(json.loads((output / 'export.json').read_text(encoding='utf-8')), receipt, 'persisted receipt equals the reported one')
        self.assertFalse((output / 'export.pending.json').exists())
        if status != 'completed':
            self.assertEqual(receipt['artifacts'], [], 'unsuccessful runs publish nothing')
        starts = [e['stage']['name'] for e in events if e['event'] == 'stage_started']
        self.assertEqual(starts, [s['name'] for s in receipt['stages']])
        return events

    def assert_tree_gone(self, pids, heartbeats):
        wait_for(lambda: not any(lifecycle.process_alive(p) for p in pids), what=f'processes {pids} to exit')
        for hb in heartbeats:
            self.assertTrue(stopped_writing(hb), f'{hb.name} still being written')

    def capture_instances(self):
        made = []
        class Spy(lifecycle.OwnedProcess):
            def __init__(self, *a, **k):
                made.append(self); super().__init__(*a, **k)
        return made, patch.object(exporter, 'OwnedProcess', Spy)


class EventStreamTests(LifecycleCase):
    def test_success_emits_stage_and_subprocess_boundaries(self):
        output = self.root / 'ok'
        receipt = self.run_export('ok', output)
        events = self.assert_consistent(output, receipt, 'completed', 'export_completed')
        names = [(e['event'], e.get('stage', {}).get('name')) for e in events]
        self.assertEqual(names, [('export_started', None), ('stage_started', 'pack'), ('subprocess_started', 'pack'), ('subprocess_exited', 'pack'), ('stage_finished', 'pack'),
                                 ('stage_started', 'audit'), ('subprocess_started', 'audit'), ('subprocess_exited', 'audit'), ('stage_finished', 'audit'),
                                 ('stage_started', 'validate'), ('stage_finished', 'validate'), ('stage_started', 'publish'), ('stage_finished', 'publish'), ('export_completed', None)])
        sub = [e['subprocess'] for e in events if e['event'] == 'subprocess_exited']
        self.assertEqual([s['returncode'] for s in sub], [0, 0]); self.assertTrue(all('pid' in s and 'seconds' in s for s in sub))
        started = [e['subprocess'] for e in events if e['event'] == 'subprocess_started']
        self.assertEqual([s['tool'] for s in started], ['blender', 'audit'])
        if os.name == 'nt': self.assertTrue(all(s['ownership']['mechanism'] == 'windows_job_object' and s['ownership']['tree_owned'] for s in started), started)
        self.assertEqual(events[0]['planned_stages'], list(exporter.STAGES)); self.assertEqual(events[0]['pid'], os.getpid())
        self.assertEqual(receipt['events_jsonl'], {'path': str(output / 'events.jsonl'), 'live': True})
        self.assertEqual([s['status'] for s in receipt['stages']], ['ok'] * 4); self.assertEqual(len(receipt['artifacts']), 4)

    def test_nonzero_pack_and_audit_exits_fail_with_single_terminal(self):
        for mode, stage, code in (('pack_fail', 'pack', 3), ('audit_fail', 'audit', 5)):
            output = self.root / mode
            with self.assertRaisesRegex(RuntimeError, f'process exit {code}') as ctx:
                self.run_export(mode, output)
            receipt = ctx.exception.export_receipt
            events = self.assert_consistent(output, receipt, 'failed', 'export_failed')
            self.assertEqual(receipt['stages'][-1]['name'], stage); self.assertEqual(receipt['stages'][-1]['status'], 'failed')
            self.assertEqual([e['subprocess']['returncode'] for e in events if e['event'] == 'subprocess_exited'][-1], code)
            self.assertTrue((output / f'{stage}.log').exists())

    def test_spawn_failure_is_a_failed_receipt(self):
        output = self.root / 'spawn'
        not_executable = self.root / 'not-blender.exe'; not_executable.write_bytes(b'MZ but not a program')
        with self.assertRaises(OSError) as ctx:
            self.run_export('ok', output, blender=not_executable)
        receipt = ctx.exception.export_receipt
        events = self.assert_consistent(output, receipt, 'failed', 'export_failed')
        self.assertEqual([e['event'] for e in events], ['export_started', 'stage_started', 'stage_finished', 'export_failed'])
        self.assertIn('OSError', receipt['error'])

    def test_terminal_receipt_is_persisted_before_terminal_event(self):
        output = self.root / 'ordering'; seen = {}
        original = lifecycle.EventLog.emit
        def spy(self_, event, **fields):
            if event.startswith('export_') and event != 'export_started':
                seen['on_disk'] = json.loads((output / 'export.json').read_text(encoding='utf-8'))['status']; seen['event'] = fields.get('status')
            return original(self_, event, **fields)
        with patch.object(lifecycle.EventLog, 'emit', spy):
            self.run_export('ok', output)
        self.assertEqual(seen, {'on_disk': 'completed', 'event': 'completed'})


class CancellationTests(LifecycleCase):
    def cancel_when(self, event_name, stage_name=None):
        """Write CANCEL from inside the exporter's own sequence: when the named event (and stage) is emitted."""
        original = lifecycle.EventLog.emit
        def hook(self_, event, **fields):
            ev = original(self_, event, **fields)
            if event == event_name and (stage_name is None or fields.get('stage', {}).get('name') == stage_name):
                (Path(self_.path).parent / lifecycle.CANCEL_FILE).write_text('cancel', encoding='utf-8')
            return ev
        return patch.object(lifecycle.EventLog, 'emit', hook)

    def assert_cancelled(self, output, receipt, observed_stage, requested_by='CANCEL file'):
        events = self.assert_consistent(output, receipt, 'cancelled', 'export_cancelled')
        self.assertEqual(receipt['cancellation']['requested_by'], requested_by)
        self.assertEqual(receipt['cancellation']['observed_stage'], observed_stage)
        self.assertNotIn('cleanup_failure', receipt, receipt.get('cancellation'))
        self.assertTrue(receipt['cancellation']['cleanup']['confirmed'], receipt['cancellation']['cleanup'])
        return events

    def test_cancel_file_during_pack_kills_packer_and_descendant(self):
        output = self.root / 'cancel-pack'; result = {}
        def watcher():
            info = self.signal('pack-started.json'); result['pids'] = [info['pid'], info['child_pid']]; self.leftover_pids.extend(result['pids'])
            self.signal('child-heartbeat.txt'); self.signal('pack-heartbeat.txt')
            for pid in result['pids']: assert lifecycle.process_alive(pid), pid
            (output / lifecycle.CANCEL_FILE).write_text('cancel', encoding='utf-8')
        thread = threading.Thread(target=watcher, daemon=True); thread.start()
        t = time.time()
        with self.assertRaisesRegex(lifecycle.Cancelled, 'CANCEL file present'):
            self.run_export('pack_sleep', output)
        thread.join(WAIT_S); self.assertIn('pids', result)
        elapsed = time.time() - t
        self.assert_tree_gone(result['pids'], [self.signals / 'child-heartbeat.txt', self.signals / 'pack-heartbeat.txt'])
        receipt = json.loads((output / 'export.json').read_text(encoding='utf-8'))
        events = self.assert_cancelled(output, receipt, 'pack')
        cleanup = receipt['cancellation']['cleanup']
        self.assertLess(cleanup['seconds'], lifecycle.CLEANUP_TIMEOUT_S); self.assertTrue(cleanup['child_exited'])
        if os.name == 'nt': self.assertEqual((cleanup['mechanism'], cleanup['job_active_processes']), ('windows_job_object', 0))
        exited = [e for e in events if e['event'] == 'subprocess_exited']
        self.assertEqual(len(exited), 1); self.assertTrue(exited[0]['subprocess']['killed']); self.assertEqual(exited[0]['subprocess']['cleanup'], cleanup)
        self.assertEqual([s['status'] for s in receipt['stages']], ['cancelled']); self.assertTrue((output / 'pack.log').exists())
        self.assertNotIn('audit', [s['name'] for s in receipt['stages']], 'no later stage after cancellation')
        self.assertLess(elapsed, WAIT_S)

    def test_keyboard_interrupt_during_pack_cleans_up_tree(self):
        output = self.root / 'kbd-pack'; result = {}
        real_sleep = time.sleep
        def interrupting_sleep(s):
            if (self.signals / 'pack-started.json').exists() and (self.signals / 'child-heartbeat.txt').exists():
                info = json.loads((self.signals / 'pack-started.json').read_text()); result['pids'] = [info['pid'], info['child_pid']]; self.leftover_pids.extend(result['pids'])
                raise KeyboardInterrupt
            real_sleep(s)
        with patch.object(exporter.time, 'sleep', interrupting_sleep):
            with self.assertRaisesRegex(lifecycle.Cancelled, 'KeyboardInterrupt'):
                self.run_export('pack_sleep', output)
        self.assertIn('pids', result)
        self.assert_tree_gone(result['pids'], [self.signals / 'child-heartbeat.txt', self.signals / 'pack-heartbeat.txt'])
        receipt = json.loads((output / 'export.json').read_text(encoding='utf-8'))
        self.assert_cancelled(output, receipt, 'pack', requested_by='KeyboardInterrupt')
        self.assertEqual(receipt['error'], 'KeyboardInterrupt: ')

    def test_cancel_file_during_audit(self):
        output = self.root / 'cancel-audit'; result = {}
        def watcher():
            info = self.signal('audit-started.json'); result['pid'] = info['pid']; self.leftover_pids.append(info['pid'])
            self.signal('audit-heartbeat.txt'); (output / lifecycle.CANCEL_FILE).write_text('cancel', encoding='utf-8')
        thread = threading.Thread(target=watcher, daemon=True); thread.start()
        with self.assertRaisesRegex(lifecycle.Cancelled, 'while audit ran'):
            self.run_export('audit_sleep', output)
        thread.join(WAIT_S); self.assertIn('pid', result)
        self.assert_tree_gone([result['pid']], [self.signals / 'audit-heartbeat.txt'])
        receipt = json.loads((output / 'export.json').read_text(encoding='utf-8'))
        self.assert_cancelled(output, receipt, 'audit')
        self.assertEqual([s['status'] for s in receipt['stages']], ['ok', 'cancelled'])
        self.assertTrue((output / 'nhl21-phase-native' / 'fake_head.mcd').exists(), 'partial native files are preserved')

    def test_cancel_between_stages_and_before_publication(self):
        for name, (event, stage, observed, expect_stages) in {'between': ('stage_finished', 'pack', 'between stages', ['ok']),
                                                             'before-publish': ('stage_started', 'publish', 'publish', ['ok', 'ok', 'ok', 'cancelled'])}.items():
            output = self.root / name
            with self.cancel_when(event, stage):
                with self.assertRaisesRegex(lifecycle.Cancelled, 'CANCEL file present') as ctx:
                    self.run_export('ok', output)
            receipt = ctx.exception.export_receipt
            events = self.assert_cancelled(output, receipt, observed)
            self.assertEqual([s['status'] for s in receipt['stages']], expect_stages, name)
            self.assertEqual(receipt['cancellation']['cleanup'], {'confirmed': True, 'note': 'no subprocess was running'})
            self.assertFalse([e for e in events if e['event'] == 'subprocess_exited' and e['subprocess'].get('killed')])
            if name == 'before-publish':
                self.assertTrue((output / 'nhl21-phase-native' / 'final-audit.json').exists()); self.assertTrue(receipt['evidence'])

    def test_cancel_after_completion_does_not_rewrite(self):
        output = self.root / 'late-cancel'
        receipt = self.run_export('ok', output)
        (output / lifecycle.CANCEL_FILE).write_text('too late', encoding='utf-8')
        self.assertEqual(json.loads((output / 'export.json').read_text(encoding='utf-8')), receipt)
        self.assertEqual(read_events(output)[-1]['event'], 'export_completed')


class CliAndParentDeathTests(unittest.TestCase):
    """CLI exit codes / stdout events and abrupt exporter death. `ui_manifest.check` cannot be patched across processes, so
    these use the real accepted Turgeon manifest + exchange + preconditions with the STAND-IN tools (skipped when absent)."""
    MANIFEST = exporter.REPO / 'local/runs/claude/app-2026-09-12-job-turgeon-02/ui_manifest.json'

    def setUp(self):
        missing = [str(p) for p in (self.MANIFEST, exporter.EXCHANGE, exporter.PRECONDITIONS) if not p.exists()]
        if missing: self.skipTest('CLI lifecycle tests need the accepted Turgeon fixture; missing: ' + ', '.join(missing))
        self.temp = tempfile.TemporaryDirectory(dir=exporter.REPO / 'local/runs/claude', prefix='lifecycle-cli-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve(); self.signals = self.root / 'signals'; self.signals.mkdir()
        self.packer_exe = self.root / 'fake-pack.exe'; self.packer_exe.write_bytes(b'pinned stand-in')
        self.leftover_pids = []; self.addCleanup(self.kill_leftovers)

    def kill_leftovers(self):
        for pid in self.leftover_pids:
            if lifecycle.process_alive(pid):
                subprocess.run(['taskkill', '/PID', str(pid), '/T', '/F'], capture_output=True) if os.name == 'nt' else os.kill(pid, 9)

    def command(self, output, *extra):
        return [sys.executable, '-B', '-m', 'src.application.export', str(self.MANIFEST), '--out', str(output), '--run-root', str(self.root),
                '--blender', str(FAKE), '--packer-script', str(FAKE), '--packer-exe', str(self.packer_exe), '--audit', str(FAKE), '--python', sys.executable, *extra]

    def env(self, mode):
        return {**os.environ, 'FAKE_NATIVE_MODE': mode, 'FAKE_NATIVE_SIGNAL_DIR': str(self.signals), 'PYTHONPATH': str(exporter.REPO)}

    def cli(self, mode, output, *extra):
        return subprocess.run(self.command(output, *extra), capture_output=True, text=True, cwd=exporter.REPO, env=self.env(mode))

    def signal(self, name):
        path = self.signals / name; wait_for(path.exists, what=name)
        return json.loads(path.read_text()) if name.endswith('.json') else path

    def test_exit_codes_and_stdout_contract(self):
        self.assertEqual(exporter.EXIT_CODES, {'completed': 0, 'refused': 2, 'failed': 3, 'cancelled': 4})
        ok = self.cli('ok', self.root / 'ok')
        self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr); self.assertEqual(len(ok.stdout.strip().splitlines()), 1, 'default stdout = one result line')
        self.assertEqual(json.loads(ok.stdout)['status'], 'completed')
        failed = self.cli('pack_fail', self.root / 'failed', '--events')
        self.assertEqual(failed.returncode, 3, failed.stdout + failed.stderr)
        lines = [json.loads(l) for l in failed.stdout.splitlines() if l.strip()]
        self.assertEqual([l.get('event') for l in lines[:-1]], ['export_started', 'stage_started', 'subprocess_started', 'subprocess_exited', 'stage_finished', 'export_failed'])
        self.assertEqual(lines[-1]['status'], 'failed'); self.assertIn('pack.log: process exit 3', lines[-1]['error'])
        self.assertEqual(read_events(self.root / 'failed'), lines[:-1], 'stdout mirrors events.jsonl')
        self.assertNotIn('simulated failure', failed.stdout, 'subprocess diagnostics stay in the log'); self.assertIn('simulated failure', (self.root / 'failed' / 'pack.log').read_text())
        refused = self.cli('ok', self.root / 'refused', '--preconditions', str(self.root / 'absent.json'), '--events')
        self.assertEqual(refused.returncode, 2); self.assertFalse((self.root / 'refused').exists())
        lines = [json.loads(l) for l in refused.stdout.splitlines() if l.strip()]
        self.assertEqual([l.get('event', l.get('status')) for l in lines], ['export_refused', 'refused'])

    def test_cancel_file_through_cli_exits_4_and_cleans_tree(self):
        output = self.root / 'cancelled'
        proc = subprocess.Popen(self.command(output, '--events'), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, cwd=exporter.REPO, env=self.env('pack_sleep'))
        try:
            info = self.signal('pack-started.json'); pids = [info['pid'], info['child_pid']]; self.leftover_pids.extend(pids)
            self.signal('child-heartbeat.txt'); self.signal('pack-heartbeat.txt')
            (output / lifecycle.CANCEL_FILE).write_text('cancel', encoding='utf-8')
            out, err = proc.communicate(timeout=WAIT_S)
        finally:
            if proc.poll() is None: proc.kill()
        self.assertEqual(proc.returncode, 4, out + err)
        wait_for(lambda: not any(lifecycle.process_alive(p) for p in pids), what='packer tree exit')
        self.assertTrue(stopped_writing(self.signals / 'child-heartbeat.txt'))
        lines = [json.loads(l) for l in out.splitlines() if l.strip()]
        self.assertEqual(lines[-2]['event'], 'export_cancelled'); self.assertEqual(lines[-1]['status'], 'cancelled')
        receipt = json.loads((output / 'export.json').read_text(encoding='utf-8'))
        self.assertEqual((receipt['status'], receipt['artifacts']), ('cancelled', [])); self.assertTrue(receipt['cancellation']['cleanup']['confirmed'])

    @unittest.skipUnless(os.name == 'nt', 'kill-on-close job semantics are Windows-specific; not exercised elsewhere')
    def test_abrupt_exporter_death_kills_owned_tree(self):
        """The exporter process is killed while the packer and its descendant run; the job handle closes with the process and
        KILL_ON_JOB_CLOSE terminates both. The receipt stays at `running` (never completed) and no terminal event exists."""
        output = self.root / 'parent-death'
        parent = subprocess.Popen(self.command(output), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, cwd=exporter.REPO, env=self.env('pack_sleep'))
        try:
            info = self.signal('pack-started.json'); pids = [info['pid'], info['child_pid']]; self.leftover_pids.extend(pids)
            self.signal('child-heartbeat.txt'); self.signal('pack-heartbeat.txt')
            self.assertTrue(all(lifecycle.process_alive(p) for p in pids))
            parent.kill(); parent.communicate(timeout=WAIT_S)
            t = time.time()
            wait_for(lambda: not any(lifecycle.process_alive(p) for p in pids), what='owned tree exit after parent death')
            self.assertLess(time.time() - t, lifecycle.CLEANUP_TIMEOUT_S)
            self.assertTrue(stopped_writing(self.signals / 'child-heartbeat.txt')); self.assertTrue(stopped_writing(self.signals / 'pack-heartbeat.txt'))
        finally:
            if parent.poll() is None: parent.kill()
        receipt = json.loads((output / 'export.json').read_text(encoding='utf-8'))
        self.assertEqual((receipt['status'], receipt['artifacts']), ('running', []), 'abandoned export never counts as completed')
        self.assertFalse([e for e in read_events(output) if e['event'].startswith('export_') and e['event'] != 'export_started'], 'no terminal event after an uncatchable kill')


class CommitBoundaryTests(LifecycleCase):
    """The completion commit point: before `commit_receipt` a cancellation wins; after it nothing rewrites the receipt or emits a
    contradictory terminal, whatever interrupts or fails (KeyboardInterrupt, event delivery, stdout)."""
    def committed_ok(self, output, receipt, terminal_written):
        persisted = json.loads((output / 'export.json').read_text(encoding='utf-8'))
        self.assertEqual((persisted['status'], len(persisted['artifacts'])), ('completed', 4), 'committed receipt never rolls back')
        self.assertEqual({k: v for k, v in receipt.items() if k != 'post_commit'}, persisted)
        self.assertEqual([s['status'] for s in persisted['stages']], ['ok'] * 4)
        events = read_events(output)
        terminals = [e['event'] for e in events if e['event'].startswith('export_') and e['event'] != 'export_started']
        self.assertEqual(terminals, ['export_completed'] if terminal_written else [], 'no contradictory terminal is ever attempted')
        note = json.loads((output / exporter.POST_COMMIT_NOTE).read_text(encoding='utf-8'))
        self.assertEqual((note['committed_status'], note['terminal_event_written']), ('completed', terminal_written))
        self.assertEqual(receipt['post_commit'], note)
        return events, note

    def test_interrupt_before_commit_is_a_cancellation(self):
        output = self.root / 'before-commit'
        def refuse(receipt, out): raise KeyboardInterrupt
        with patch.object(exporter, 'commit_receipt', refuse):
            with self.assertRaisesRegex(lifecycle.Cancelled, 'KeyboardInterrupt') as ctx:
                self.run_export('ok', output)
        receipt = ctx.exception.export_receipt
        self.assert_consistent(output, receipt, 'cancelled', 'export_cancelled')
        self.assertEqual([s['status'] for s in receipt['stages']], ['ok', 'ok', 'ok', 'cancelled'])
        self.assertFalse((output / exporter.POST_COMMIT_NOTE).exists())

    def test_interrupt_right_after_receipt_replacement(self):
        output = self.root / 'after-replace'; original = exporter.commit_receipt
        def interrupt_after(receipt, out): original(receipt, out); raise KeyboardInterrupt
        with patch.object(exporter, 'commit_receipt', interrupt_after):
            receipt = self.run_export('ok', output)
        events, note = self.committed_ok(output, receipt, terminal_written=False)
        self.assertEqual(events[-1]['event'], 'stage_started'); self.assertEqual(events[-1]['stage']['name'], 'publish')
        self.assertIn('KeyboardInterrupt', note['interruption'])

    def test_interrupt_after_publish_stage_finished(self):
        output = self.root / 'after-stage-finished'; original = lifecycle.EventLog.emit
        def hook(self_, event, **fields):
            ev = original(self_, event, **fields)
            if event == 'stage_finished' and fields.get('stage', {}).get('name') == 'publish': raise KeyboardInterrupt
            return ev
        with patch.object(lifecycle.EventLog, 'emit', hook):
            receipt = self.run_export('ok', output)
        events, _ = self.committed_ok(output, receipt, terminal_written=False)
        self.assertEqual((events[-1]['event'], events[-1]['stage']['name'], events[-1]['status']), ('stage_finished', 'publish', 'ok'))

    def test_interrupt_immediately_after_terminal_event(self):
        output = self.root / 'after-terminal'; original = lifecycle.EventLog.emit
        def hook(self_, event, **fields):
            ev = original(self_, event, **fields)
            if event == 'export_completed': raise KeyboardInterrupt
            return ev
        with patch.object(lifecycle.EventLog, 'emit', hook):
            receipt = self.run_export('ok', output)
        events, _ = self.committed_ok(output, receipt, terminal_written=True)
        self.assertEqual(events[-1]['event'], 'export_completed')

    def test_event_and_stdout_delivery_failures_after_commit(self):
        output = self.root / 'delivery'; original = lifecycle.EventLog.emit
        def failing(self_, event, **fields):
            if event == 'export_completed': raise OSError('disk full while writing events')
            return original(self_, event, **fields)
        with patch.object(lifecycle.EventLog, 'emit', failing):
            receipt = self.run_export('ok', output)
        _, note = self.committed_ok(output, receipt, terminal_written=False)
        self.assertIn('disk full', note['interruption'])
        output2 = self.root / 'stdout'; real_print = print
        def broken_print(*a, **k):
            if a and 'export_completed' in str(a[0]): raise OSError('stdout closed')
            return real_print(*a, **k)
        with patch('builtins.print', broken_print):
            receipt = self.run_export('ok', output2, events_to_stdout=True)
        self.committed_ok(output2, receipt, terminal_written=True)   # the line was written to events.jsonl before stdout failed
        self.assertIn('stdout closed', receipt['post_commit']['interruption'])

    def test_terminal_stream_write_failure_is_not_reported_as_delivered(self):
        """The terminal is CLAIMED before its write (so no second terminal can follow); a failed stream write must be reported as
        not delivered, not as a written terminal event."""
        output = self.root / 'terminal-write'; original = lifecycle.EventLog.emit
        class BrokenStream:
            def __init__(self, real): self.real = real
            def write(self, text): raise OSError('events stream write failed')
            def flush(self): self.real.flush()
            def fileno(self): return self.real.fileno()
            def close(self): self.real.close()
        def hook(self_, event, **fields):
            if event == 'export_completed': self_.stream = BrokenStream(self_.stream)
            return original(self_, event, **fields)
        with patch.object(lifecycle.EventLog, 'emit', hook):
            receipt = self.run_export('ok', output)
        events, note = self.committed_ok(output, receipt, terminal_written=False)
        self.assertEqual(events[-1]['event'], 'stage_finished', 'no terminal line reached the log')
        self.assertEqual(note['terminal_event_claimed'], 'export_completed', 'the terminal was claimed (a second one is impossible)')
        self.assertTrue(note['terminal_delivery'].startswith('failed: OSError'), note); self.assertIn('stream write failed', note['interruption'])
        # a pre-commit terminal whose write fails is reported the same way in the delivery-failure sidecar
        output2 = self.root / 'terminal-write-failed'
        def hook2(self_, event, **fields):
            if event == 'export_failed': self_.stream = BrokenStream(self_.stream)
            return original(self_, event, **fields)
        with patch.object(lifecycle.EventLog, 'emit', hook2):
            with self.assertRaisesRegex(RuntimeError, 'process exit 3'):
                self.run_export('pack_fail', output2)
        note2 = json.loads((output2 / exporter.POST_COMMIT_NOTE).read_text(encoding='utf-8'))
        self.assertEqual((note2['committed_status'], note2['terminal_event_claimed'], note2['terminal_event_written']), ('failed', 'export_failed', False))
        self.assertTrue(note2['terminal_delivery'].startswith('failed: OSError'), note2)
        self.assertEqual(read_events(output2)[-1]['event'], 'stage_finished')
        self.assertEqual(json.loads((output2 / 'export.json').read_text(encoding='utf-8'))['status'], 'failed', 'the persisted receipt is unaffected')

    def test_delivery_failure_before_commit_still_persists_the_failed_receipt(self):
        output = self.root / 'pre-commit-delivery'; original = lifecycle.EventLog.emit
        def failing(self_, event, **fields):
            if event in ('export_failed', 'export_cancelled'): raise OSError('events unwritable')
            return original(self_, event, **fields)
        with patch.object(lifecycle.EventLog, 'emit', failing):
            with self.assertRaisesRegex(RuntimeError, 'process exit 3') as ctx:
                self.run_export('pack_fail', output)
        receipt = ctx.exception.export_receipt
        self.assertEqual(json.loads((output / 'export.json').read_text(encoding='utf-8'))['status'], 'failed')
        self.assertEqual(receipt['artifacts'], [])
        note = json.loads((output / exporter.POST_COMMIT_NOTE).read_text(encoding='utf-8'))
        self.assertEqual(note['committed_status'], 'failed'); self.assertIn('events unwritable', note['delivery_failure'])


class CommitBoundaryCliTests(unittest.TestCase):
    """The CLI result and exit code follow the committed receipt even when interrupted after commitment (real Turgeon manifest,
    stand-in tools; the interrupt is injected by a driver that patches `commit_receipt` and then runs `main()`)."""
    MANIFEST = exporter.REPO / 'local/runs/claude/app-2026-09-12-job-turgeon-02/ui_manifest.json'

    def setUp(self):
        missing = [str(p) for p in (self.MANIFEST, exporter.EXCHANGE, exporter.PRECONDITIONS) if not p.exists()]
        if missing: self.skipTest('needs the accepted Turgeon fixture; missing: ' + ', '.join(missing))
        self.temp = tempfile.TemporaryDirectory(dir=exporter.REPO / 'local/runs/claude', prefix='lifecycle-commit-cli-')
        self.addCleanup(self.temp.cleanup); self.root = Path(self.temp.name).resolve()
        self.packer_exe = self.root / 'fake-pack.exe'; self.packer_exe.write_bytes(b'pinned stand-in')

    def test_cli_reports_committed_completion_after_post_commit_interrupt(self):
        output = self.root / 'cli-after-commit'
        driver = ('import sys\nfrom src.application import export as e\noriginal = e.commit_receipt\n'
                  'def hook(r, o):\n    original(r, o); raise KeyboardInterrupt\n'
                  'e.commit_receipt = hook\nsys.argv = ["export", *sys.argv[1:]]\nsys.exit(e.main())\n')
        cmd = [sys.executable, '-B', '-c', driver, str(self.MANIFEST), '--out', str(output), '--run-root', str(self.root), '--blender', str(FAKE),
               '--packer-script', str(FAKE), '--packer-exe', str(self.packer_exe), '--audit', str(FAKE), '--python', sys.executable, '--events']
        env = {**os.environ, 'FAKE_NATIVE_MODE': 'ok', 'PYTHONPATH': str(exporter.REPO)}
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=exporter.REPO, env=env)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        lines = [json.loads(l) for l in result.stdout.splitlines() if l.strip()]
        self.assertEqual(lines[-1]['status'], 'completed')
        self.assertNotIn('export_cancelled', [l.get('event') for l in lines])
        self.assertEqual(json.loads((output / 'export.json').read_text(encoding='utf-8'))['status'], 'completed')
        self.assertTrue((output / exporter.POST_COMMIT_NOTE).exists())


@unittest.skipUnless(os.name == 'nt', 'Job Object ownership is Windows-specific; the POSIX session path is not exercised here')
class OwnershipTests(LifecycleCase):
    """Ownership is established before the child runs; every failure refuses/terminates instead of running unowned."""
    def assert_no_owned_leftovers(self, made, output, expect_error):
        self.assertEqual(len(made), 1); inst = made[0]
        self.assertIsNone(inst.job, 'job handle closed')
        if inst.proc is not None:
            self.assertIsNotNone(inst.proc.poll(), 'never-run child reaped'); self.assertFalse(lifecycle.process_alive(inst.proc.pid))
        self.assertFalse((self.signals / 'pack-started.json').exists(), 'no payload executed')
        receipt = json.loads((output / 'export.json').read_text(encoding='utf-8'))
        self.assertEqual((receipt['status'], receipt['artifacts']), (expect_error, []))
        return inst, receipt

    def test_job_creation_failure_refuses_launch_without_spawning(self):
        output = self.root / 'no-job'; made, spy = self.capture_instances()
        with spy, patch.object(lifecycle, 'create_kill_on_close_job', side_effect=lifecycle.OwnershipError(5, 'CreateJobObjectW denied')), \
             patch.object(lifecycle.subprocess, 'Popen', side_effect=AssertionError('Popen must not be called without a job')):
            with self.assertRaisesRegex(lifecycle.OwnershipError, 'CreateJobObjectW denied'):
                self.run_export('pack_sleep', output)
        inst, receipt = self.assert_no_owned_leftovers(made, output, 'failed')
        self.assertIsNone(inst.proc); self.assertIn('OwnershipError', receipt['error'])
        self.assertEqual([e['event'] for e in read_events(output)], ['export_started', 'stage_started', 'stage_finished', 'export_failed'])

    def test_process_creation_failure_closes_job(self):
        output = self.root / 'no-process'; made, spy = self.capture_instances()
        with spy, patch.object(lifecycle.subprocess, 'Popen', side_effect=OSError(193, 'not a valid Win32 application')):
            with self.assertRaisesRegex(OSError, 'not a valid Win32 application'):
                self.run_export('pack_sleep', output)
        inst, _ = self.assert_no_owned_leftovers(made, output, 'failed'); self.assertIsNone(inst.proc)

    def test_assignment_and_resume_failures_terminate_the_suspended_child(self):
        for name, target in (('assign', 'assign_to_job'), ('resume', 'resume_process_threads')):
            output = self.root / f'fail-{name}'; made, spy = self.capture_instances()
            with spy, patch.object(lifecycle, target, side_effect=lifecycle.OwnershipError(87, f'{target} failed')):
                with self.assertRaisesRegex(lifecycle.OwnershipError, f'{target} failed'):
                    self.run_export('pack_sleep', output)
            inst, _ = self.assert_no_owned_leftovers(made, output, 'failed')
            self.assertIsNotNone(inst.proc, 'the child was created (suspended) and then terminated')
            time.sleep(0.3); self.assertFalse((self.signals / 'pack-started.json').exists(), f'{name}: suspended child never ran')

    def test_keyboard_interrupt_during_construction_terminates_child_and_cancels(self):
        output = self.root / 'kbd-construction'; made, spy = self.capture_instances()
        with spy, patch.object(lifecycle, 'resume_process_threads', side_effect=KeyboardInterrupt):
            with self.assertRaisesRegex(lifecycle.Cancelled, 'KeyboardInterrupt'):
                self.run_export('pack_sleep', output)
        inst, receipt = self.assert_no_owned_leftovers(made, output, 'cancelled')
        self.assertIsNotNone(inst.proc); self.assertEqual(receipt['cancellation']['requested_by'], 'KeyboardInterrupt')

    def test_child_cannot_start_work_before_ownership_and_descendant_is_owned(self):
        """Barrier: hold the suspended child for 1 s before assignment - no payload may run; after resume the child spawns a
        descendant immediately; cancellation then kills both (they are in the job)."""
        output = self.root / 'barrier'; original = lifecycle.assign_to_job; seen = {}
        def delayed_assign(job, handle):
            time.sleep(1.0)
            seen['payload_before_ownership'] = (self.signals / 'pack-started.json').exists()
            seen['active_before_assign'] = lifecycle.job_active_processes(job)
            original(job, handle); seen['active_after_assign'] = lifecycle.job_active_processes(job)
        result = {}
        def watcher():
            info = self.signal('pack-started.json'); result['pids'] = [info['pid'], info['child_pid']]; self.leftover_pids.extend(result['pids'])
            self.signal('child-heartbeat.txt'); (output / lifecycle.CANCEL_FILE).write_text('cancel', encoding='utf-8')
        thread = threading.Thread(target=watcher, daemon=True); thread.start()
        with patch.object(lifecycle, 'assign_to_job', delayed_assign):
            with self.assertRaises(lifecycle.Cancelled):
                self.run_export('pack_sleep', output)
        thread.join(WAIT_S)
        self.assertEqual(seen, {'payload_before_ownership': False, 'active_before_assign': 0, 'active_after_assign': 1}, seen)
        self.assert_tree_gone(result['pids'], [self.signals / 'child-heartbeat.txt'])
        receipt = json.loads((output / 'export.json').read_text(encoding='utf-8'))
        self.assertTrue(receipt['cancellation']['cleanup']['confirmed']); self.assertEqual(receipt['cancellation']['cleanup']['job_active_processes'], 0)
        started = [e for e in read_events(output) if e['event'] == 'subprocess_started'][0]
        self.assertTrue(started['subprocess']['ownership']['launched_suspended'])

    def test_lingering_descendant_after_child_exit_is_terminated_before_advancing(self):
        output = self.root / 'orphan'
        receipt = self.run_export('pack_orphan', output)
        info = json.loads((self.signals / 'orphan-started.json').read_text()); self.leftover_pids.append(info['child_pid'])
        self.assertEqual(receipt['status'], 'completed')
        exited = [e for e in read_events(output) if e['event'] == 'subprocess_exited' and e['subprocess']['tool'] == 'blender'][0]['subprocess']
        tree = exited['tree']
        self.assertGreaterEqual(tree['lingering_descendants'], 1, tree)
        self.assertEqual((tree['lingering_terminated'], tree['job_active_processes'], tree['confirmed']), (True, 0, True), tree)
        self.assertLess(tree['seconds'], lifecycle.CLEANUP_TIMEOUT_S)
        self.assertFalse(lifecycle.process_alive(info['child_pid'])); self.assertTrue(stopped_writing(self.signals / 'orphan-heartbeat.txt'))
        # the audit stage started only after the tree was confirmed empty
        events = read_events(output); i_exit = events.index(next(e for e in events if e['event'] == 'subprocess_exited'))
        self.assertEqual(events[i_exit + 2]['event'], 'stage_started'); self.assertEqual(events[i_exit + 2]['stage']['name'], 'audit')

    def test_unconfirmed_tree_fails_the_stage(self):
        output = self.root / 'unconfirmed'
        with patch.object(lifecycle.OwnedProcess, 'wait_tree', return_value={'confirmed': False, 'job_active_processes': 1, 'seconds': 10.0}):
            with self.assertRaisesRegex(RuntimeError, 'owned descendants .* not confirmed terminated') as ctx:
                self.run_export('ok', output)
        receipt = ctx.exception.export_receipt
        self.assertEqual((receipt['status'], receipt['artifacts'], [s['status'] for s in receipt['stages']]), ('failed', [], ['failed']))


@unittest.skipUnless(os.name == 'nt', 'Job Object ownership is Windows-specific; the POSIX session path is not exercised here')
class ExceptionalCleanupTests(LifecycleCase):
    """H2.3b: an exception INSIDE the cleanup path (interrupt or accounting failure during the descendant drain, event-delivery
    failure after the immediate child exited) must still terminate the owned tree, release every handle, start no later stage,
    publish nothing and report the MEASURED cleanup - never `no subprocess was running` inferred from the exited child alone.
    Uses `pack_orphan`: the packer exits while a heartbeat-writing descendant it spawned keeps running inside the job."""
    def accounting(self, failures):
        """Wrap `job_active_processes`: the first len(failures) calls raise the given exceptions; later calls (or None entries) are real."""
        real = lifecycle.job_active_processes; calls = []
        def wrapped(job):
            calls.append(1)
            exc = failures[len(calls) - 1] if len(calls) <= len(failures) else None
            if exc is not None: raise exc
            return real(job)
        return calls, patch.object(lifecycle, 'job_active_processes', wrapped)

    def orphan(self):
        info = json.loads((self.signals / 'orphan-started.json').read_text()); self.leftover_pids.extend([info['pid'], info['child_pid']])
        return info

    def assert_tree_finalised(self, made, output, status, cleanup):
        """Descendant dead and silent, handles closed, only the pack stage exists, nothing published, receipt == disk."""
        info = self.orphan()
        self.assert_tree_gone([info['pid'], info['child_pid']], [self.signals / 'orphan-heartbeat.txt'])
        self.assertEqual(len(made), 1); self.assertIsNone(made[0].job, 'job handle released'); self.assertTrue(cleanup['handles_closed'])
        receipt = json.loads((output / 'export.json').read_text(encoding='utf-8'))
        self.assertEqual((receipt['status'], receipt['artifacts'], [s['name'] for s in receipt['stages']], [s['status'] for s in receipt['stages']]),
                         (status, [], ['pack'], [status]))
        self.assertFalse((output / 'nhl21-phase-native' / 'final-audit.json').exists(), 'no later stage ran')
        self.assertFalse((output / 'evidence').exists())
        self.assertEqual(cleanup['pid'], info['pid'])
        return receipt, info

    def test_interrupt_during_descendant_drain_terminates_owned_tree(self):
        output = self.root / 'drain-interrupt'; made, spy = self.capture_instances()
        calls, accounting = self.accounting([KeyboardInterrupt()])
        with spy, accounting:
            with self.assertRaisesRegex(lifecycle.Cancelled, 'KeyboardInterrupt') as ctx:
                self.run_export('pack_orphan', output)
        receipt = ctx.exception.export_receipt
        events = self.assert_consistent(output, receipt, 'cancelled', 'export_cancelled')
        cleanup = receipt['cancellation']['cleanup']
        self.assert_tree_finalised(made, output, 'cancelled', cleanup)
        self.assertEqual(receipt['cancellation']['requested_by'], 'KeyboardInterrupt'); self.assertEqual(receipt['cancellation']['observed_stage'], 'pack')
        self.assertNotIn('note', cleanup, 'measured, not inferred'); self.assertNotIn('cleanup_failure', receipt)
        self.assertEqual((cleanup['terminated'], cleanup['confirmed'], cleanup['child_exited'], cleanup['returncode'], cleanup['job_active_processes']), (True, True, True, 0, 0), cleanup)
        self.assertGreater(len(calls), 1, 'the interrupted query was followed by real confirmation queries')
        exited = [e['subprocess'] for e in events if e['event'] == 'subprocess_exited']
        self.assertEqual(len(exited), 1); self.assertEqual((exited[0]['killed'], exited[0]['returncode'], exited[0]['cleanup']), (True, 0, cleanup))
        self.assertEqual(events[-1]['cancellation'], receipt['cancellation'])

    def test_accounting_failure_during_drain_fails_the_run_with_measured_cleanup(self):
        output = self.root / 'drain-oserror'; made, spy = self.capture_instances()
        calls, accounting = self.accounting([OSError(6, 'QueryInformationJobObject failed (injected)')])
        with spy, accounting:
            with self.assertRaisesRegex(OSError, 'injected') as ctx:
                self.run_export('pack_orphan', output)
        receipt = ctx.exception.export_receipt
        events = self.assert_consistent(output, receipt, 'failed', 'export_failed')
        cleanup = receipt['cleanup']
        self.assert_tree_finalised(made, output, 'failed', cleanup)
        self.assertNotIn('cancellation', receipt); self.assertNotIn('cleanup_failure', receipt); self.assertIn('injected', receipt['error'])
        self.assertEqual((cleanup['terminated'], cleanup['confirmed'], cleanup['child_exited'], cleanup['job_active_processes']), (True, True, True, 0), cleanup)
        self.assertLess(cleanup['seconds'], lifecycle.CLEANUP_TIMEOUT_S)
        exited = [e['subprocess'] for e in events if e['event'] == 'subprocess_exited']
        self.assertEqual(len(exited), 1); self.assertTrue(exited[0]['killed']); self.assertEqual(exited[0]['cleanup'], cleanup)
        self.assertEqual(events[-1]['cleanup'], cleanup)

    def test_event_delivery_failure_after_child_exit_releases_the_job(self):
        output = self.root / 'exit-event'; made, spy = self.capture_instances(); original = lifecycle.EventLog.emit
        def failing(self_, event, **fields):
            if event == 'subprocess_exited' and 'tree' in fields.get('subprocess', {}): raise OSError('events unwritable (injected)')
            return original(self_, event, **fields)
        with spy, patch.object(lifecycle.EventLog, 'emit', failing):
            with self.assertRaisesRegex(OSError, 'injected') as ctx:
                self.run_export('pack_orphan', output)
        receipt = ctx.exception.export_receipt
        events = self.assert_consistent(output, receipt, 'failed', 'export_failed')
        cleanup = receipt['cleanup']
        self.assert_tree_finalised(made, output, 'failed', cleanup)
        # the drain itself had confirmed emptiness (lingering descendant terminated), so release only closed the handle
        self.assertEqual((cleanup['terminated'], cleanup['confirmed'], cleanup['returncode']), (False, True, 0), cleanup); self.assertNotIn('cleanup_failure', receipt)
        self.assertEqual([e['event'] for e in events], ['export_started', 'stage_started', 'subprocess_started', 'stage_finished', 'export_failed'],
                         'the failed exit event is not retried with a contradictory report')

    def test_persistent_confirmation_failure_is_reported_not_claimed(self):
        """Every accounting query fails: the tree is still terminated and the handles closed (kill-on-close backs the termination),
        but the receipt must say cleanup was NOT confirmed rather than substituting a confirmation."""
        for name, first, expect_status, regex in (('failed', OSError(6, 'accounting down (injected)'), 'failed', 'injected'),
                                                  ('cancelled', KeyboardInterrupt(), 'cancelled', 'KeyboardInterrupt')):
            output = self.root / f'unconfirmed-{name}'; made, spy = self.capture_instances()
            persistent = OSError(6, 'accounting down (injected)')
            calls, accounting = self.accounting([first] + [persistent] * 10_000)
            with spy, accounting:
                with self.assertRaisesRegex((OSError, lifecycle.Cancelled), regex) as ctx:
                    self.run_export('pack_orphan', output)
            receipt = ctx.exception.export_receipt
            self.assert_consistent(output, receipt, expect_status, f'export_{expect_status}')
            cleanup = receipt['cancellation']['cleanup'] if expect_status == 'cancelled' else receipt['cleanup']
            self.assert_tree_finalised(made, output, expect_status, cleanup)   # measured dead: the job was terminated and closed
            self.assertEqual(receipt['cleanup_failure'], 'owned process tree not confirmed terminated within the cleanup bound', name)
            self.assertEqual((cleanup['terminated'], cleanup['confirmed'], cleanup['child_exited']), (True, False, True), cleanup)
            self.assertIn('accounting down', cleanup['error']); self.assertNotIn('job_active_processes', cleanup, 'no count was measured')
            if expect_status == 'cancelled': self.assertNotEqual(cleanup.get('note'), 'no subprocess was running')
            self.assertEqual(read_events(output)[-1]['cleanup_failure'], receipt['cleanup_failure'])
            for f in self.signals.iterdir(): f.unlink()   # fresh signal files for the second case


if __name__ == '__main__': unittest.main()
