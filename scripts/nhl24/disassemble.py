"""Dump a selected unwind-indexed function with string and global annotations."""
import argparse
import bisect
import sqlite3
from pathlib import Path
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_REG_RIP
from inspect_eboot import Image


def dump(im, db, va, limit=100000):
    start = db.execute('select max(start) from functions where start<=?', (va,)).fetchone()[0]
    end = db.execute('select min(start) from functions where start>?', (start,)).fetchone()[0]
    md = Cs(CS_ARCH_X86, CS_MODE_64)
    md.detail = True
    lines = [f'; unwind function {start:#x}, next indexed start {end:#x}; query {va:#x}']
    for ins in md.disasm(im.read(start, min(end-start, limit)), start):
        notes = []
        for op in ins.operands:
            if op.type == X86_OP_MEM and op.mem.base == X86_REG_RIP:
                dst = ins.address + ins.size + op.mem.disp
                row = db.execute('select text from strings where va=?',(dst,)).fetchone()
                notes.append(f'{dst:#x}' + (f' {row[0]}' if row else ''))
        lines.append(f'{ins.address:010x} {ins.bytes.hex():30s} {ins.mnemonic:12s} {ins.op_str:55s} ; ' + ' | '.join(notes))
    return '\n'.join(lines) + '\n'


if __name__ == '__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('eboot',type=Path)
    p.add_argument('index',type=Path)
    p.add_argument('address',type=lambda s:int(s,0))
    p.add_argument('--output',type=Path)
    a=p.parse_args()
    result=dump(Image(a.eboot),sqlite3.connect(a.index),a.address)
    if a.output:a.output.write_text(result,encoding='utf-8')
    else:print(result)
