"""Prepare the exact scalp-revision mesh against the restored working package.

No game writes. Usage: python this.py revised-package working-package output-dir
The reference texture is copied only for offline verification, never installation.
"""
import copy
import hashlib
import json
import shutil
import struct
import sys
from pathlib import Path

revised, working, out = map(Path, sys.argv[1:])
load = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
candidate = load(revised/'package.json')
baseline = load(working/'package.json')
receipt = load(working/'installation.json')
assert load(revised/'installation.json')['status'] == 'restored'
assert receipt['status'] == 'installed'
assert not out.exists(), 'Use a fresh output directory; preserve prior receipts'
out.mkdir(parents=True)
manifest = copy.deepcopy(candidate)
manifest['status'] = 'prepared mesh-only isolation test; not installed'
manifest['rollback_returns_to'] = str(working)
manifest['test_variable'] = 'Exact scalp/neck mesh revision; no texture row or payload changes'
manifest['assets']['textures'] = copy.deepcopy(baseline['assets']['textures'])
manifest['reference_texture_only'] = True
for arc in manifest['archives']:
    prior = next(a for a in receipt['archives'] if a['plan']['path'] == arc['path'])
    assert arc['original_sha256'] == prior['installed_sha256']
    assert arc['original_size'] == prior['plan']['new_size']
    arc['changes'] = [c for c in arc['changes'] if c['path'].endswith('/jack_eichel.mcd')]
    assert len(arc['changes']) == 1
    change = arc['changes'][0]
    for field, hash_field in [('stored_file', 'stored_sha256'), ('decoded_file', 'decoded_sha256')]:
        source = revised/change[field]
        assert digest(source) == change[hash_field]
        shutil.copy2(source, out/source.name)
    assert change['new_offset'] == (arc['original_size']+15)&~15
    arc['new_size'] = change['new_offset']+(out/change['stored_file']).stat().st_size
    header = bytearray.fromhex(arc['original_header'])
    struct.pack_into('>Q', header, 24, arc['new_size'])
    arc['new_header'] = header.hex()

for name in ['native_mesh.json', 'mesh_report.json', 'revision_checks.json',
             'front_all_components.png', 'profile_all_components.png', 'profile_eichel.png']:
    shutil.copy2(revised/name, out/name)
for name in ['head_jack_eichel_cm.gnf'] + [f'diffuse_mip_{i}.bc1' for i in range(11)]:
    shutil.copy2(working/name, out/name)
assert digest(out/'head_jack_eichel_cm.gnf') == baseline['assets']['textures']['diffuse_sha256']
shutil.copy2(Path(__file__).with_name('nhl19_test_install.py'), out/'nhl19_test_install.py')
shutil.copy2(working/'Manage-Test.ps1', out/'Manage-Test.ps1')
(out/'package.json').write_text(json.dumps(manifest, indent=2))
(out/'README.txt').write_text('''NHL19 MESH-ONLY ISOLATION TEST

Prepared, not installed. Changes ONLY the three Jack Eichel MCD archive entries.
Uses the EXACT scalp/neck meshes from the failed combined revision. The known
working 1024 diffuse and all other installed textures remain untouched.
The GNF and mip files in this folder are verification references only.

With shadPS4 closed, run in PowerShell:
  .\\Manage-Test.ps1 apply
Then launch NHL19 and inspect Mogilny's helmet, collar, eyes and mouth.
Record whether launch succeeds and whether the corrected mesh displays properly.

Rollback with shadPS4 closed:
  .\\Manage-Test.ps1 restore
This returns to the previous successful Mogilny version. Keep its package folder.
Do not apply the combined scalp-2K package for this isolation test.
''')
print('PREPARED', out, 'three mesh replacements; zero texture replacements')
