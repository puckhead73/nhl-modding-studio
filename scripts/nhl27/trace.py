"""Inspect bounded functions, annotate decompilation, or find immediate operands.

All addresses are guest image VAs. Byte-scan hits are candidates until confirmed
by decoding from an unwind function start. No installed file is modified.
"""
import argparse
import json
import re
import sqlite3
import struct
from pathlib import Path
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_REG_RIP
from research import ElfImage, SelfImage


def direct_call_candidates(raw, base, target):
    """Find rel32 destinations, including overlapping candidates; not a CFG."""
    for match in re.finditer(rb'(?=\xe8[\s\S]{4})', raw):
        offset = match.start()
        address = base + offset
        if address + 5 + struct.unpack_from('<i', raw, offset + 1)[0] == target:
            yield address


def operand_needles(value):
    """Include narrow operands; decoded operands remain the final filter."""
    result = set()
    for width in (1, 2, 4, 8):
        if -(1 << (width * 8 - 1)) <= value < (1 << (width * 8)):
            result.add((value & ((1 << (width * 8)) - 1)).to_bytes(width, 'little'))
    return sorted(result, key=len)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('root', type=Path)
    p.add_argument('operation', choices=['function', 'immediate', 'displacement', 'callers', 'annotate'])
    p.add_argument('value', nargs='?')
    a = p.parse_args()
    db = sqlite3.connect(f'file:{(a.root / "index.sqlite").as_posix()}?mode=ro', uri=True)
    strings = dict(db.execute('SELECT va,text FROM strings'))
    if a.operation == 'annotate':
        dest = a.root / 'annotated'; dest.mkdir(exist_ok=True)
        for f in (a.root / 'decompiled').glob('*.c'):
            def sub(m):
                name = strings.get(int(m[1], 16))
                return m[0] + (' /* ' + name.replace('*/', '* /').replace('\n', ' ') + ' */' if name else '')
            (dest / f.name).write_text(re.sub(r'(?:UNK|DAT|PTR|s)_(0[0-9a-f]{7,15})', sub, f.read_text()))
        return
    image_path = Path(json.loads((a.root / 'identity.json').read_text())['path'])
    with image_path.open('rb') as stream:
        is_elf = stream.read(4) == b'\x7fELF'
    im = (ElfImage if is_elf else SelfImage)(image_path)
    md = Cs(CS_ARCH_X86, CS_MODE_64); md.detail = True
    value = int(a.value, 0)
    def start_for(addr):
        r = db.execute('SELECT start FROM functions WHERE start<=? ORDER BY start DESC LIMIT 1', (addr,)).fetchone()
        return r[0] if r else None
    if a.operation == 'callers':
        for s in im.mapped():
            if not s['flags'] & 1: continue
            raw = im.read(s['va'], s['filesz'])
            # Lookahead includes E8 bytes inside a preceding candidate's operand.
            # A non-overlapping regex silently skips some real call instructions.
            for va in direct_call_candidates(raw, s['va'], value):
                start = start_for(va)
                if start is None:
                    print('unknown', hex(va), 'candidate direct call', hex(value))
                    continue
                # Validate the boundary from the unwind start, not from E8.
                decoded = next((ins for ins in md.disasm(im.read(start, va + 5 - start), start)
                                if ins.address == va), None)
                if decoded is not None and decoded.mnemonic == 'call':
                    print(hex(start), hex(va), 'linear-boundary direct call', hex(value))
    elif a.operation == 'function':
        start = start_for(value)
        end = db.execute('SELECT start FROM functions WHERE start>? ORDER BY start LIMIT 1', (start,)).fetchone()[0]
        print(f'Unwind interval {start:#x}..{end:#x}; linear decoding is not full CFG recovery')
        for ins in md.disasm(im.read(start, min(end-start, 100000)), start):
            notes = []
            for op in ins.operands:
                if op.type == X86_OP_MEM and op.mem.base == X86_REG_RIP:
                    dst = ins.address + ins.size + op.mem.disp
                    notes.append(hex(dst) + ' ' + strings.get(dst, ''))
            print(f'{ins.address:09x} {ins.bytes.hex():24} {ins.mnemonic:8} {ins.op_str:45} ' + '; '.join(notes))
    else:
        # Search the immediate bytes, then validate against decoded instruction
        # boundaries starting at the enclosing unwind start.
        needles = operand_needles(value)
        starts = set()
        for s in im.mapped():
            if not s['flags'] & 1: continue
            raw = im.read(s['va'], s['filesz'])
            # Lookahead preserves candidates overlapping another encoding.
            pattern = b'(?=' + b'(?:' + b'|'.join(re.escape(n) for n in needles) + b'))'
            for m in re.finditer(pattern, raw):
                start = start_for(s['va'] + m.start())
                if start is not None: starts.add(start)
        for start in sorted(starts):
            nxt = db.execute('SELECT start FROM functions WHERE start>? ORDER BY start LIMIT 1', (start,)).fetchone()
            if not nxt or nxt[0] - start > 1000000: continue
            for ins in md.disasm(im.read(start, nxt[0]-start), start):
                if any((a.operation == 'immediate' and op.type == X86_OP_IMM and op.imm == value)
                       or (a.operation == 'displacement' and op.type == X86_OP_MEM and op.mem.disp == value)
                       for op in ins.operands):
                    print(hex(start), hex(ins.address), ins.mnemonic, ins.op_str)


if __name__ == '__main__': main()
