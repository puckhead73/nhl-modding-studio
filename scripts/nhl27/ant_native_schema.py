"""Recover native ANT registration names by bounded static constant propagation.

Does not execute the game or infer native field offsets from EBX offsets. Unknown
register writes invalidate values. Output is evidence for serializer inspection.
"""
import argparse
import bisect
import hashlib
import json
from pathlib import Path
import sqlite3
import struct
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_OP_REG
from research import SelfImage

REGISTER = 0x8ac030
FRAME = 1 << 60
VOLATILE = ('rax', 'rcx', 'rdx', 'rsi', 'rdi', 'r8', 'r9', 'r10', 'r11')


def canonical(name):
    aliases = {'eax': 'rax', 'ax': 'rax', 'al': 'rax', 'ah': 'rax',
        'ebx': 'rbx', 'bx': 'rbx', 'bl': 'rbx', 'bh': 'rbx',
        'ecx': 'rcx', 'cx': 'rcx', 'cl': 'rcx', 'ch': 'rcx',
        'edx': 'rdx', 'dx': 'rdx', 'dl': 'rdx', 'dh': 'rdx',
        'esi': 'rsi', 'si': 'rsi', 'sil': 'rsi', 'edi': 'rdi', 'di': 'rdi', 'dil': 'rdi',
        'ebp': 'rbp', 'bp': 'rbp', 'esp': 'rsp', 'sp': 'rsp'}
    if name in aliases:
        return aliases[name]
    if name.startswith('r') and name[1:].rstrip('dwb').isdigit():
        return name.rstrip('dwb')
    return name


def scan(image, start, end, strings):
    md = Cs(CS_ARCH_X86, CS_MODE_64)
    md.detail = True
    regs, memory, rows = {'rbp': FRAME}, {}, []
    last_push = None

    def read(at, width):
        if at is None:
            return None
        if any(at + i in memory for i in range(width)):
            values = [memory.get(at + i) for i in range(width)]
            return None if None in values else int.from_bytes(bytes(values), 'little')
        if at >= FRAME - 0x100000:
            return None
        try:
            return int.from_bytes(image.read(at, width), 'little')
        except ValueError:
            return None

    for ins in md.disasm(image.read(start, end - start), start):
        ops = ins.operands

        def address(op):
            mem = op.mem
            base_name = md.reg_name(mem.base)
            base = ins.address + ins.size if base_name == 'rip' else regs.get(canonical(base_name)) if mem.base else 0
            index = regs.get(canonical(md.reg_name(mem.index))) if mem.index else 0
            return None if base is None or index is None else base + index * mem.scale + mem.disp

        def value(op):
            if op.type == X86_OP_IMM:
                return op.imm
            if op.type == X86_OP_REG:
                return regs.get(canonical(md.reg_name(op.reg)))
            if op.type == X86_OP_MEM:
                return read(address(op), op.size)

        if ins.mnemonic == 'call':
            if ops[0].type == X86_OP_IMM and ops[0].imm == REGISTER:
                field_base, count = regs.get('r8'), regs.get('r9')
                name = strings.get(regs.get('rsi'))
                if name and field_base is not None and count is not None and 0 <= count <= 4096:
                    fields = []
                    for i in range(count):
                        at = field_base + 32 * i
                        fields.append(dict(index=i, ordinal_flags=read(at, 8),
                            name=strings.get(read(at + 8, 8)), type_name=strings.get(read(at + 16, 8)),
                            flags=read(at + 24, 4)))
                    descriptor = last_push
                    vt = read(descriptor, 8)
                    rows.append(dict(name=name, type_hash=regs.get('rdx'), base_hash=regs.get('rcx'),
                        registration=hex(ins.address), descriptor=hex(descriptor) if descriptor is not None else None,
                        vtable=hex(vt) if vt else None,
                        serializer=hex(read(vt + 8, 8) or 0) if vt else None, fields=fields))
            for name in VOLATILE:
                regs.pop(name, None)
            last_push = None
            continue
        if ins.mnemonic == 'push':
            last_push = value(ops[0])
            continue
        dest_value = None
        handled = False
        if ins.mnemonic in ('mov', 'movabs', 'movzx', 'movsxd') and len(ops) == 2:
            dest_value = value(ops[1])
            handled = True
        elif ins.mnemonic == 'lea' and len(ops) == 2:
            dest_value = address(ops[1])
            handled = True
        elif ins.mnemonic == 'xor' and len(ops) == 2 and ops[0].type == ops[1].type == X86_OP_REG and ops[0].reg == ops[1].reg:
            dest_value, handled = 0, True
        if handled:
            dst = ops[0]
            if dst.type == X86_OP_REG:
                name = canonical(md.reg_name(dst.reg))
                # The symbolic frame is independent of the unknown caller stack.
                if name == 'rbp':
                    regs[name] = FRAME
                elif dest_value is None or dst.size < 4:
                    regs.pop(name, None)
                else:
                    regs[name] = dest_value & ((1 << (8 * dst.size)) - 1)
            elif dst.type == X86_OP_MEM:
                at = address(dst)
                if at is not None:
                    for i in range(dst.size):
                        memory[at + i] = None if dest_value is None else (dest_value >> (8 * i)) & 255
        else:
            _, writes = ins.regs_access()
            for reg in writes:
                name = canonical(md.reg_name(reg))
                if name != 'rbp':
                    regs.pop(name, None)
            if ops and ops[0].type == X86_OP_MEM and ops[0].access & 2:
                at = address(ops[0])
                if at is not None:
                    for i in range(ops[0].size):
                        memory[at + i] = None
    return rows


def run(root, target):
    image = SelfImage(target)
    sha = hashlib.sha256(image.data).hexdigest()
    if sha != 'fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9':
        raise ValueError('Native registration decoder is bound to NHL24 1.60')
    with sqlite3.connect(f'file:{(root.parent.parent / "nhl24-eboot/update/index.sqlite").as_posix()}?mode=ro', uri=True) as db:
        functions = [x[0] for x in db.execute('SELECT start FROM functions ORDER BY start')]
        strings = dict(db.execute('SELECT va,text FROM strings'))
    candidates = set()
    for segment in image.segments:
        if not segment['flags'] & 1:
            continue
        data = image.read(segment['va'], segment['filesz'])
        at = 0
        while True:
            at = data.find(b'\xe8', at)
            if at < 0 or at + 5 > len(data):
                break
            va = segment['va'] + at
            if va + 5 + struct.unpack_from('<i', data, at + 1)[0] == REGISTER:
                candidates.add(bisect.bisect_right(functions, va) - 1)
            at += 1
    rows = []
    for i in sorted(candidates):
        rows.extend(scan(image, functions[i], functions[i + 1], strings))
    out = root / 'deke-trial/native-registration-schema.json'
    out.write_text(json.dumps(dict(executable_sha256=sha,
        qualification='Static registration names and serializer addresses; native field offsets not inferred',
        types=rows), indent=2))
    print(json.dumps(dict(types=len(rows), fields=sum(len(x['fields']) for x in rows),
        named_fields=sum(f['name'] is not None for x in rows for f in x['fields'])), indent=2))


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('root', type=Path)
    p.add_argument('--target', type=Path, default=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))
    args = p.parse_args()
    run(args.root, args.target)
