"""Index candidate RIP-relative instructions and unwind function starts.

References are instruction-decoded byte-scan candidates, not a full CFG proof.
"""
import argparse
import json
import re
import sqlite3
import struct
from pathlib import Path
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_REG_RIP
from inspect_eboot import Image


def run(path, output):
    im = Image(path)
    db = sqlite3.connect(output / 'index.sqlite')
    db.execute('DELETE FROM refs')
    md = Cs(CS_ARCH_X86, CS_MODE_64)
    md.detail = True
    # All common RIP loads/stores/LEAs, including scalar SSE prefixes.
    pattern = re.compile(rb'(?:[\xf2\xf3\x66]?[\x40-\x4f]?(?:[\x8d\x8b\x89\x8a\x88]|\x0f[\x10\x11\x28\x29\x2e\x2f\xb6\xb7\xbe\xbf])|[\x80\x81\x83\xc6\xc7\xff])[\x05\x0d\x15\x1d\x25\x2d\x35\x3d]')
    rows = []
    for s in im.mapped():
        if not s['flags'] & 1:
            continue
        b = im.data[s['self_offset']:s['self_offset'] + s['filesz']]
        for m in pattern.finditer(b):
            va = s['va'] + m.start()
            ins = next(md.disasm(b[m.start():m.start()+15], va, count=1), None)
            if ins:
                for op in ins.operands:
                    if op.type == X86_OP_MEM and op.mem.base == X86_REG_RIP:
                        dst = ins.address + ins.size + op.mem.disp
                        if any(t['va'] <= dst < t['va'] + t['memsz'] for t in im.mapped()):
                            rows.append((va, dst, ins.mnemonic))
        print('decoded candidates', len(rows), flush=True)
    db.executemany('INSERT INTO refs VALUES(?,?,?)', rows)
    db.executescript('CREATE INDEX IF NOT EXISTS refs_dst ON refs(dst); CREATE INDEX IF NOT EXISTS strings_text ON strings(text); DROP TABLE IF EXISTS functions; CREATE TABLE functions(start INTEGER PRIMARY KEY,fde INTEGER);')
    eh = next(p for p in im.ph if p['type'] == 0x6474e550)
    raw = im.read(eh['va'], eh['filesz'])
    if raw[:4] != bytes.fromhex('011b033b'):
        raise ValueError('Unsupported unwind encoding: ' + raw[:4].hex())
    count = struct.unpack_from('<I', raw, 8)[0]
    if 12 + count*8 > len(raw):
        raise ValueError('Unwind index exceeds segment')
    funcs = [(eh['va'] + a, eh['va'] + b) for a,b in struct.iter_unpack('<ii',raw[12:12+count*8])]
    db.executemany('INSERT OR IGNORE INTO functions VALUES(?,?)', funcs)
    db.commit()
    print(json.dumps({'refs':len(rows), 'unwind_entries':len(funcs)}))


if __name__ == '__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('eboot', type=Path)
    p.add_argument('output', type=Path)
    a=p.parse_args()
    run(a.eboot,a.output)
