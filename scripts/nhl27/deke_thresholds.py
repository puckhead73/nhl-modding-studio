"""Compare seven deke registration defaults in the supplied NHL27/NHL24 1.60.

These compiler-specific patterns identify defaults, not behavioral equivalence.
Packed target double loads contain two float32 values; only lane zero is default.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import struct
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_OP_REG, X86_REG_RIP, X86_REG_XMM0
from research import ElfImage, SelfImage

HASHES = {
    'nhl27': '10c7bee4c27e5260b417f9574e65cee8338019d08635b3ab9dddab30ba524126',
    'nhl24_160': 'fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9',
}


def extract(image, database, source):
    decoder = Cs(CS_ARCH_X86, CS_MODE_64)
    decoder.detail = True
    result = {}
    with sqlite3.connect(f'file:{database.as_posix()}?mode=ro', uri=True) as db:
        for va, name in db.execute("SELECT va,text FROM strings WHERE text LIKE 'gDekeController%'"):
            refs = db.execute('SELECT DISTINCT src FROM refs WHERE dst=?', (va,)).fetchall()
            if len(refs) != 1:
                raise ValueError(f'Ambiguous registration reference: {name}')
            anchor = refs[0][0]
            start = db.execute('SELECT max(start) FROM functions WHERE start<=?', (anchor,)).fetchone()[0]
            instructions = list(decoder.disasm(image.read(start, anchor - start + 128), start))
            candidates = []
            for ins in instructions:
                if not ((anchor - 40 <= ins.address < anchor) if source
                        else (anchor < ins.address < anchor + 100)):
                    continue
                if ins.mnemonic not in ('vmovss', 'vmovsd') or len(ins.operands) != 2:
                    continue
                dst, src = ins.operands
                if dst.type != X86_OP_REG or dst.reg != X86_REG_XMM0:
                    continue
                if src.type != X86_OP_MEM or src.mem.base != X86_REG_RIP:
                    continue
                candidates.append((ins, ins.address + ins.size + src.mem.disp))
            if len(candidates) != 1:
                raise ValueError(f'Default load not uniquely identified: {name}')
            ins, address = candidates[0]
            raw = image.read(address, 4)
            result[name] = dict(default=struct.unpack('<f', raw)[0], value_bytes=raw.hex(),
                value_va=hex(address), label_reference_va=hex(anchor),
                registration_function=hex(start), load_va=hex(ins.address),
                load_bytes=bytes(ins.bytes).hex(), load=f'{ins.mnemonic} {ins.op_str}')
    if len(result) != 7:
        raise ValueError('Expected exactly seven named deke-controller thresholds')
    return result


def run(root, source, target):
    images = {'nhl27': ElfImage(source), 'nhl24_160': SelfImage(target)}
    for name, image in images.items():
        if hashlib.sha256(image.data).hexdigest() != HASHES[name]:
            raise ValueError(f'Unsupported executable: {name}')
    values = {
        'nhl27': extract(images['nhl27'], root.parent / 'index.sqlite', True),
        'nhl24_160': extract(images['nhl24_160'], root.parent.parent / 'nhl24-eboot/update/index.sqlite', False),
    }
    rows = [dict(name=name, source=values['nhl27'][name], target=values['nhl24_160'][name],
        default_identical=values['nhl27'][name]['value_bytes'] == values['nhl24_160'][name]['value_bytes'])
        for name in sorted(values['nhl27'])]
    result = dict(executable_sha256=HASHES, thresholds=rows,
        qualification='Static registration defaults only; consumer logic and runtime overrides not equivalent by implication',
        executable_patches_emitted=0)
    path = root / 'deke-trial/deke-thresholds.json'
    path.write_text(json.dumps(result, indent=2))
    print(json.dumps(dict(thresholds=len(rows), identical=sum(r['default_identical'] for r in rows)), indent=2))


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('root', type=Path)
    p.add_argument('--source', type=Path, default=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'))
    p.add_argument('--target', type=Path, default=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))
    args = p.parse_args()
    run(args.root, args.source, args.target)
