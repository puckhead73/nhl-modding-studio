"""Test harness run INSIDE Blender (`blender -b --python this.py -- <spec.json> -- <build.py args>`): executes the real
experiments/codex/nhl21_package/build.py unchanged, but after the FIRST read of each path listed in the spec's `tamper` list the
disposable source file is overwritten with different bytes (first byte flipped, same length). Any later read of an original source
therefore sees changed bytes; a build that consumes its captured/verified snapshot still completes with the same native output."""
import json, pathlib, runpy, sys

spec_path = sys.argv[sys.argv.index('--') + 1]
spec = json.loads(pathlib.Path(spec_path).read_text(encoding='utf-8'))
pending = {str(pathlib.Path(p).resolve()) for p in spec['tamper']}
tampered = []
_read_bytes = pathlib.Path.read_bytes


def read_bytes(self):
    data = _read_bytes(self)
    key = str(self.resolve())
    if key in pending:
        pending.discard(key)
        with open(self, 'wb') as f: f.write(bytes([data[0] ^ 0xFF]) + data[1:])
        tampered.append(key)
    return data


pathlib.Path.read_bytes = read_bytes
build = spec['build']
sys.argv = [build, '--', *sys.argv[sys.argv.index('--', sys.argv.index('--') + 1) + 1:]]
try:
    runpy.run_path(build, run_name='__main__')
finally:
    pathlib.Path(spec['log']).write_text(json.dumps({'tampered': tampered, 'never_read': sorted(pending)}, indent=2), encoding='utf-8')
