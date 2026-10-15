"""Read-only local executable strings and shared-normal chunk diagnostics."""
import json, re, struct, zlib
from pathlib import Path
root=Path(r'H:\TEMP\nhl19-demitra\normal-test')
data=Path(r'H:\Emulators\ShadPS4\Games\CUSA11126\eboot.bin').read_bytes()
strings=[(m.start(),m.group().decode('ascii')) for m in re.finditer(rb'[\x20-\x7e]{6,}',data)
         if any(k in m.group().lower() for k in (b'head_',b'gaborik',b'normalmap',b'normal_map'))]
(root/'executable_head_strings.json').write_text(json.dumps(strings,indent=2))
print('EXECUTABLE',data[:16].hex(),'matches',len(strings))
for off,s in strings: print(hex(off),s[:240])
entries=json.loads((root/'archive_names.json').read_text())
e=next(e for e in entries if e['path'].endswith('/head_generic_nm.gnf'))
with open(e['archive'],'rb') as f:f.seek(e['offset']);raw=f.read(e['size'])
(root/'head_generic_nm.chunkzip').write_bytes(raw)
print('GENERIC_HEADER',raw[:48].hex())
off=40
for i in range(struct.unpack_from('>I',raw,20)[0]):
    n,flags=struct.unpack_from('>II',raw,off)
    block=raw[off+8:off+8+n]
    try:out=zlib.decompress(block,-15);status=f'deflate {len(out)}'
    except zlib.error:status='not deflate'
    print('CHUNK',i,off,n,hex(flags),block[:16].hex(),status)
    off=((off+8+n-8+15)&~15)+8
