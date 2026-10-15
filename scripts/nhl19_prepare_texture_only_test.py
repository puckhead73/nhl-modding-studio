"""Stage the supplied 2K diffuse over the successful scalp mesh; no game writes.
Usage: python this.py combined-package mesh-only-package new-output-directory
"""
import copy
import hashlib
import json
import shutil
import struct
import sys
from pathlib import Path

combined, mesh, out = map(Path, sys.argv[1:])
load = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = load(combined/'package.json')
receipt = load(mesh/'installation.json')
assert receipt['status'] == 'installed'
assert not out.exists(), 'Preserve previous packages; use a fresh directory'
base = next(a for a in receipt['archives'] if Path(a['plan']['path']).name == 'game_90-008.big')
old = next(a for a in source['archives'] if a['path'] == base['plan']['path'])
change = copy.deepcopy(next(c for c in old['changes'] if c['path'].endswith('/head_jack_eichel_cm.gnf')))
arc = {'path': base['plan']['path'], 'original_size': base['plan']['new_size'],
       'original_sha256': base['installed_sha256'], 'original_header': base['plan']['new_header']}
with open(arc['path'], 'rb') as f:
    assert f.read(48).hex() == arc['original_header']
    f.seek(change['row_offset'])
    assert f.read(20).hex() == change['row'], 'Working texture row changed'
assert Path(arc['path']).stat().st_size == arc['original_size']
out.mkdir(parents=True)
for field, hash_field in [('stored_file','stored_sha256'), ('decoded_file','decoded_sha256')]:
    path = combined/change[field]
    assert digest(path) == change[hash_field]
    shutil.copy2(path, out/path.name)
change['new_offset'] = (arc['original_size']+15)&~15
row = bytearray.fromhex(change['row'])
struct.pack_into('>I', row, 0, change['new_offset']//16)
struct.pack_into('>I', row, 8, (out/change['stored_file']).stat().st_size)
change['new_row'] = row.hex()
arc['changes'] = [change]
arc['new_size'] = change['new_offset']+(out/change['stored_file']).stat().st_size
header = bytearray.fromhex(arc['original_header'])
struct.pack_into('>Q', header, 24, arc['new_size'])
arc['new_header'] = header.hex()
manifest = {'game_root':source['game_root'], 'status':'prepared texture-only test; not installed',
            'archives':[arc], 'assets':{'textures':source['assets']['textures']},
            'rollback_returns_to':str(mesh), 'test_variable':'2K diffuse only; successful meshes untouched'}
for name in ['native_mesh.json']+[f'diffuse_mip_{i}.bc1' for i in range(12)]:
    shutil.copy2(combined/name, out/name)
shutil.copy2(Path(__file__).with_name('nhl19_test_install.py'), out/'nhl19_test_install.py')
shutil.copy2(mesh/'Manage-Test.ps1', out/'Manage-Test.ps1')
(out/'package.json').write_text(json.dumps(manifest,indent=2))
(out/'README.txt').write_text('''NHL19 TEXTURE-ONLY TEST
One diffuse GNF replacement in the base archive; no mesh or patch archive changes.
Supplied Mogilny 2048x2048 diffuse, BC1 with all 12 mip levels.
Close shadPS4 before applying. Wait for COMPLETE apply before launching NHL19.
Manage-Test.ps1 apply installs; Manage-Test.ps1 restore returns to the successful
scalp mesh with its working 1024 texture. Keep all previous package folders.
''')
print('PREPARED',out,'one texture replacement; zero mesh replacements')
