"""Read archive name tables to trace NHL19 head normal/material resources."""
import json
import struct
from pathlib import Path

root = Path(r'H:\TEMP\nhl19-demitra\normal-test')
root.mkdir(exist_ok=True)
game = Path(r'H:\Emulators\ShadPS4\Games\CUSA11126\databigs')
entries = []
for path in sorted(game.glob('*.big')):
    with path.open('rb') as f:
        h = f.read(48)
        if h[:4] != b'EB\0\3':
            continue
        count, noff = struct.unpack_from('>I', h, 4)[0], struct.unpack_from('>I', h, 12)[0]
        ns, ds, dc = h[20], h[21], struct.unpack_from('>H', h, 22)[0]
        doff = (noff + count * ns + 15) & ~15
        f.seek(0)
        prefix = f.read(doff + dc * ds)
    for i in range(count):
        n = prefix[noff+i*ns:noff+(i+1)*ns]
        di = struct.unpack_from('>H', n)[0]
        directory = prefix[doff+di*ds:doff+(di+1)*ds].split(b'\0')[0].decode()
        name = n[2:].split(b'\0')[0].decode()
        logical = directory + '/' + name
        row = prefix[48+20*i:68+20*i]
        words = struct.unpack('>5I', row)
        entries.append({'archive': str(path), 'index': i, 'path': logical,
                        'offset': words[0]*16, 'size': words[2], 'row': row.hex()})
(root / 'archive_names.json').write_text(json.dumps(entries))
selected = [e for e in entries if ('head' in e['path'] and ('_nm' in e['path'] or 'normal' in e['path']))
            or any(s in e['path'].lower() for s in ('material', 'gaborik', 'skinshader'))]
(root / 'normal_material_names.json').write_text(json.dumps(selected, indent=2))
for e in selected:
    print(Path(e['archive']).name, e['index'], e['path'])
print('TOTAL', len(entries), 'SELECTED', len(selected))
