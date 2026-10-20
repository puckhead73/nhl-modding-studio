"""Index ELF RELA entries without applying them to the installed image.

Usage: python scripts/nhl27/relocations.py local/nhl27-eboot [VA COUNT]
Only R_X86_64_RELATIVE (8) resolves to an image-relative target here. External
symbol relocations remain unresolved; zero on disk is not assumed to mean null.
"""
import collections
import json
import sqlite3
import struct
import sys
from pathlib import Path
from research import ElfImage

root = Path(sys.argv[1])
im = ElfImage(json.loads((root / 'identity.json').read_text())['path'])
db = sqlite3.connect(root / 'index.sqlite')
if len(sys.argv) == 2:
    p = next(p for p in im.ph if p['type'] == 2)
    tags = dict(struct.iter_unpack('<qQ', im.data[p['offset']:p['offset']+p['filesz']]))
    if tags[9] != 24 or tags[8] % 24: raise ValueError('Unsupported RELA entries')
    db.executescript('DROP TABLE IF EXISTS relocations; CREATE TABLE relocations(site INTEGER PRIMARY KEY,type INTEGER,symbol INTEGER,addend INTEGER);')
    entries = [(site, info & 0xffffffff, info >> 32, addend)
               for site, info, addend in struct.iter_unpack('<QQq', im.read(tags[7], tags[8]))]
    db.executemany('INSERT INTO relocations VALUES(?,?,?,?)', entries)
    db.execute('CREATE INDEX relocations_target ON relocations(addend)')
    db.commit()
    summary = {'entries': len(entries), 'types': dict(collections.Counter(r[1] for r in entries)),
               'rela_va': hex(tags[7]), 'rela_bytes': tags[8], 'plt_rela_va': hex(tags[23]), 'plt_rela_bytes': tags[2]}
    (root / 'relocations.json').write_text(json.dumps(summary, indent=2))
    print(json.dumps(summary))
else:
    start = int(sys.argv[2], 0)
    for site in range(start, start + int(sys.argv[3]) * 8, 8):
        r = db.execute('SELECT type,symbol,addend FROM relocations WHERE site=?', (site,)).fetchone()
        raw = struct.unpack('<Q', im.read(site, 8))[0]
        target = r[2] if r and r[0] == 8 else raw if not r else None
        name = db.execute('SELECT text FROM strings WHERE va=?', (target,)).fetchone()
        print(hex(site), 'raw='+hex(raw), 'relocation='+str(r),
              'target='+ (hex(target) if target is not None else 'unresolved'), name[0] if name else '')
