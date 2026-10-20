"""Read-only NHL24 plaintext SELF inspection; outputs private research evidence.

Usage: python scripts/nhl24/inspect_eboot.py EBOOT OUTPUT_DIRECTORY
No decryption, game writes, or runtime-support claims are made.
"""
import argparse
import hashlib
import json
import re
import sqlite3
import struct
from pathlib import Path


class Image:
    def __init__(self, path):
        self.path = Path(path)
        self.data = self.path.read_bytes()
        b = self.data
        if b[:4] != bytes.fromhex('4f153d1d'):
            raise ValueError('Expected PS4 SELF')
        count = struct.unpack_from('<H', b, 24)[0]
        elf = 32 + count * 32
        h = struct.unpack_from('<16sHHIQQQIHHHHHH', b, elf)
        if h[0][:6] != b'\x7fELF\x02\x01' or h[2] != 62:
            raise ValueError('Expected little-endian x86-64 ELF')
        self.header = h
        self.elf_header = b[elf:elf + h[5] + h[9] * h[10]]
        self.ph = []
        for i in range(h[10]):
            p = struct.unpack_from('<IIQQQQQQ', b, elf + h[5] + i * h[9])
            self.ph.append(dict(zip(('type', 'flags', 'offset', 'va', 'pa', 'filesz', 'memsz', 'align'), p)))
        self.segments = []
        for i in range(count):
            flags, offset, size, usize = struct.unpack_from('<QQQQ', b, 32 + 32 * i)
            if not flags & 0x800:  # Digest records are not program payloads.
                continue
            if flags & 0x2 or flags & 0x8 or size != usize:
                raise ValueError('Encrypted or compressed payload is unsupported')
            index = (flags >> 20) & 0xfff
            p = self.ph[index]
            if offset + size > len(b) or size != p['filesz']:
                raise ValueError('Payload bounds/size mismatch')
            self.segments.append(dict(p, index=index, self_offset=offset, self_flags=flags))

    def mapped(self):
        return [s for s in self.segments if s['type'] in (1, 0x61000010)]

    def offset(self, va):
        for s in self.mapped():
            if s['va'] <= va < s['va'] + s['filesz']:
                return s['self_offset'] + va - s['va']
        raise ValueError(f'VA {va:x} is not file-backed')

    def read(self, va, size):
        offset = self.offset(va)
        if self.offset(va + size - 1) != offset + size - 1:
            raise ValueError('Read spans mappings')
        return self.data[offset:offset + size]

    def disasm(self, va, size=256):
        from capstone import Cs, CS_ARCH_X86, CS_MODE_64
        md = Cs(CS_ARCH_X86, CS_MODE_64)
        return [(hex(i.address), i.mnemonic, i.op_str, i.bytes.hex()) for i in md.disasm(self.read(va, size), va)]


def inspect(path, out):
    out.mkdir(parents=True, exist_ok=True)
    image = Image(path)
    meta = {'path': str(image.path.resolve()), 'sha256': hashlib.sha256(image.data).hexdigest(),
            'size': len(image.data), 'segments': image.segments, 'program_headers': image.ph}
    (out / 'identity.json').write_text(json.dumps(meta, indent=2))
    # Reconstruct only payload-backed ELF ranges. Missing trailing SELF metadata is not fabricated.
    elf = bytearray(max(s['offset'] + s['filesz'] for s in image.segments))
    elf[:len(image.elf_header)] = image.elf_header
    for s in image.segments:
        elf[s['offset']:s['offset'] + s['filesz']] = image.data[s['self_offset']:s['self_offset'] + s['filesz']]
    (out / 'image.elf').write_bytes(elf)
    db = sqlite3.connect(out / 'index.sqlite')
    db.executescript('DROP TABLE IF EXISTS strings; CREATE TABLE strings(va INTEGER PRIMARY KEY, self_offset INTEGER, text TEXT); DROP TABLE IF EXISTS refs; CREATE TABLE refs(src INTEGER, dst INTEGER, kind TEXT);')
    n = 0
    for s in image.mapped():
        raw = image.data[s['self_offset']:s['self_offset'] + s['filesz']]
        rows = [(s['va'] + m.start(), s['self_offset'] + m.start(), m[0][:-1].decode('ascii'))
                for m in re.finditer(rb'[\x20-\x7e]{4,}\x00', raw)]
        db.executemany('INSERT OR IGNORE INTO strings VALUES(?,?,?)', rows)
        n += len(rows)
    db.commit()
    print(json.dumps(dict(meta, string_count=n)), flush=True)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('eboot', type=Path)
    p.add_argument('output', type=Path)
    args = p.parse_args()
    inspect(args.eboot, args.output)
