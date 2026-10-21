"""Stand-in for the NHL21 native packer (run in place of Blender through job.blender_command) and the offline audit
script, so the export route can be exercised without Blender or game assets. Behaviour: FAKE_NATIVE_MODE env var
  ok (default)   packer writes <--out>/ with two native files named by its installation-plan.json plus validation.json;
                 audit writes <candidate>/final-audit.json
  pack_fail      packer exits 3 before writing anything
  audit_fail     audit exits 5 without writing its receipt
  plan_escape    packer writes a plan whose replacements point outside the output (traversal + absolute path)
  plan_missing   packer writes a plan naming a native file it never wrote
  pack_sleep     packer spawns a long-lived descendant (heartbeat writer), signals FAKE_NATIVE_SIGNAL_DIR/pack-started.json
                 {pid, child_pid}, then sleeps ~120 s writing its own heartbeat (for cancellation / process-tree tests)
  audit_sleep    audit signals FAKE_NATIVE_SIGNAL_DIR/audit-started.json {pid} then sleeps ~120 s
  pack_orphan    packer behaves like `ok` but first spawns a long-lived descendant (signals orphan-started.json {child_pid}) and
                 exits while it is still running (immediate child exits first, descendant lingers)
The packer also refuses (exit 4) when --run-root / --packer-exe / --preconditions are absent or do not exist, mirroring build.py.
Native file bytes are the sha256 of the --input / --atlas files so a test can predict the receipt hashes."""
import hashlib, json, os, sys
from pathlib import Path

import subprocess, time
mode = os.environ.get('FAKE_NATIVE_MODE', 'ok')
signal_dir = Path(os.environ.get('FAKE_NATIVE_SIGNAL_DIR', '.'))
HEARTBEAT = 'import sys,time\np=sys.argv[1]\nfor _ in range(2400):\n    open(p,"a").write("."); time.sleep(0.05)\n'


def sleep_with_heartbeat(path, seconds=120):
    for _ in range(int(seconds / 0.05)):
        open(path, 'a').write('.'); time.sleep(0.05)


argv = sys.argv[1:]
if '--offline' in argv:                                     # audit role: <candidate> --offline
    if mode == 'audit_fail': print('fake audit: simulated failure'); sys.exit(5)
    if mode == 'audit_sleep':
        (signal_dir / 'audit-started.json').write_text(json.dumps({'pid': os.getpid()})); print('fake audit sleeping', flush=True)
        sleep_with_heartbeat(signal_dir / 'audit-heartbeat.txt'); sys.exit(0)
    out = Path(argv[0]); (out / 'final-audit.json').write_text(json.dumps({'status': 'passed', 'fake': True}))
    print('FINAL AUDIT PASSED (fake)'); sys.exit(0)
args = argv[argv.index('--') + 1:]                          # packer role: blender flags -- --exchange .. --out OUT
opts = {args[i]: args[i + 1] for i in range(0, len(args), 2)}
if mode == 'pack_fail': print('fake packer: simulated failure', file=sys.stderr); sys.exit(3)
for key in ('--run-root', '--packer-exe', '--preconditions'):
    if key not in opts or not Path(opts[key]).exists(): print(f'fake packer: {key} missing', file=sys.stderr); sys.exit(4)
if mode == 'pack_sleep':
    child = subprocess.Popen([sys.executable, '-c', HEARTBEAT, str(signal_dir / 'child-heartbeat.txt')])
    (signal_dir / 'pack-started.json').write_text(json.dumps({'pid': os.getpid(), 'child_pid': child.pid})); print('fake packer sleeping', flush=True)
    sleep_with_heartbeat(signal_dir / 'pack-heartbeat.txt'); sys.exit(0)
if mode == 'pack_orphan':
    child = subprocess.Popen([sys.executable, '-c', HEARTBEAT, str(signal_dir / 'orphan-heartbeat.txt')])
    (signal_dir / 'orphan-started.json').write_text(json.dumps({'pid': os.getpid(), 'child_pid': child.pid}))
out = Path(opts['--out']); out.mkdir(parents=False, exist_ok=False)
digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest().encode()
(out / 'fake_head.mcd').write_bytes(digest(opts['--input']))
(out / 'fake_head.gnf').write_bytes(digest(opts['--atlas']))
names = {'ok': ['fake_head.mcd', 'fake_head.gnf'], 'plan_escape': ['../fake_head.mcd', str(out / 'fake_head.gnf')],
         'plan_missing': ['fake_head.mcd', 'never_written.gnf']}.get(mode, ['fake_head.mcd', 'fake_head.gnf'])
(out / 'installation-plan.json').write_text(json.dumps({'status': 'prepared native files only; not installed',
    'replacements': [{'file': n} for n in names]}))
(out / 'validation.json').write_text(json.dumps({'status': 'fake', 'exchange': opts['--exchange']}))
print('NHL21_NATIVE_COMPLETE (fake)', out)
