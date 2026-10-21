"""Controlled read guard for the exporter: run `python read_trace.py <export CLI args>` and every path this process opens (CPython
`open` audit events - io.open, Path.read_bytes/read_text, json loads through them) is written as a JSON list to the file named by
READ_TRACE_LOG. The exporter runs unchanged in this process (runpy, `__main__`); its exit code is propagated. Subprocesses (the packer
stand-in / Blender, the audit) are not traced here - their inputs are the command lines the export receipt records."""
import json
import os
import runpy
import sys

log = os.environ['READ_TRACE_LOG']
opened = []


def hook(event, args):
    if event == 'open' and args and isinstance(args[0], (str, bytes, os.PathLike)):
        target = args[0]
        opened.append(target.decode(errors='replace') if isinstance(target, bytes) else os.fspath(target))


sys.addaudithook(hook)
sys.argv = ['export', *sys.argv[1:]]
code = 0
try:
    runpy.run_module('src.application.export', run_name='__main__', alter_sys=True)
except SystemExit as exc:
    code = exc.code if isinstance(exc.code, int) else 1
finally:
    with open(log, 'w', encoding='utf-8') as f: json.dump(opened, f, indent=1)
sys.exit(code)
