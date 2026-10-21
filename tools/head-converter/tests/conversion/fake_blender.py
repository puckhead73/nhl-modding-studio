"""Stand-in for the Blender executable in tests (run through the current interpreter via job.blender_command when
tools.blender.path ends with '.py'). Behaviour is chosen by the FAKE_BLENDER_MODE environment variable:
  version (default)  `--version` prints a Blender-like line; a stage call writes <last arg>/done.txt and exits 0
  fail               stage call prints to stderr and exits 1
  sleep              stage call sleeps FAKE_BLENDER_SLEEP seconds (default 60) then exits 0 (for cancellation tests)
"""
import os, sys, time
from pathlib import Path
for st in (sys.stdout, sys.stderr): st.reconfigure(encoding='utf-8', errors='replace')   # Blender itself writes UTF-8

mode = os.environ.get('FAKE_BLENDER_MODE', 'version')
if '--version' in sys.argv:
    print('Blender 4.5.5 LTS (fake)'); print('\tbuild date: test'); sys.exit(0)
args = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
if mode == 'fail':
    print('fake blender: simulated failure', file=sys.stderr); sys.exit(1)
if mode == 'sleep':
    t = float(os.environ.get('FAKE_BLENDER_SLEEP', '60')); print(f'fake blender sleeping {t}s', flush=True); time.sleep(t); sys.exit(0)
out = Path(args[-1]) if args else None
if out is not None:
    out.mkdir(parents=True, exist_ok=True); (out / 'done.txt').write_text('ok', encoding='utf-8')
print('fake blender ok', args)
