"""Reversible append-and-retarget NHL19 test installation; standard library only.
Usage: python nhl19_test_install.py package_directory verify|apply|restore
Archives are fingerprinted in full; original payloads remain in place for restoration.
"""
import sys,json,os,hashlib,subprocess,csv,io,time
from pathlib import Path
root=Path(sys.argv[1]).resolve();mode=sys.argv[2]
assert mode in ('verify','apply','restore')
manifest=json.loads((root/'package.json').read_text())
receipt_path=root/'installation.json'
def digest(path):
 h=hashlib.sha256()
 with open(path,'rb') as f:
  for b in iter(lambda:f.read(4*1024*1024),b''):h.update(b)
 return h.hexdigest()
def active_game():
 result=subprocess.run(['tasklist','/FO','CSV','/NH'],capture_output=True,text=True,check=True)
 return any(row and row[0].lower()=='shadps4.exe' for row in csv.reader(io.StringIO(result.stdout)))
def save_receipt(receipt):
 temp=root/'installation.pending.json';temp.write_text(json.dumps(receipt,indent=2));os.replace(temp,receipt_path)
def restore_arc(arc):
 with open(arc['path'],'r+b') as f:
  for c in arc['changes']:f.seek(c['row_offset']);f.write(bytes.fromhex(c['row']))
  f.seek(0);f.write(bytes.fromhex(arc['original_header']));f.truncate(arc['original_size']);f.flush();os.fsync(f.fileno())
 assert digest(arc['path'])==arc['original_sha256'],'restore hash mismatch'
if mode in ('apply','restore') and active_game():raise RuntimeError('Close shadPS4 before changing archives')
for arc in manifest['archives']:
 target=Path(arc['path']).resolve();base=(Path(manifest['game_root'])/'databigs').resolve()
 assert target.parent==base and target.name in ('game_90-008.big','patch_game_91.big','patch_game_93.big')
 for c in arc['changes']:
  payload=(root/c['stored_file']).resolve();assert payload.parent==root
  assert digest(payload)==c['stored_sha256'],'package payload changed'
if mode=='restore':
 receipt=json.loads(receipt_path.read_text());assert receipt['status']=='installed'
 for arc,saved in zip(manifest['archives'],receipt['archives']):
  assert arc==saved['plan'],'package plan changed since installation'
  assert digest(arc['path'])==saved['installed_sha256'],'installed archive changed; refusing destructive restore'
 for arc in reversed(manifest['archives']):restore_arc(arc);print('RESTORED',arc['path'],flush=True)
 receipt['status']='restored';save_receipt(receipt)
else:
 if receipt_path.exists() and json.loads(receipt_path.read_text())['status']!='restored':raise RuntimeError('Existing installation receipt requires inspection or restore')
 for arc in manifest['archives']:
  assert Path(arc['path']).stat().st_size==arc['original_size'],'archive size changed'
  assert digest(arc['path'])==arc['original_sha256'],'archive fingerprint changed'
  print('VERIFIED',arc['path'],flush=True)
 if mode=='apply':
  receipt={'status':'applying','time':time.time(),'archives':[]};save_receipt(receipt);touched=[]
  try:
   for arc in manifest['archives']:
    # Record restoration metadata before the first mutation of each archive.
    receipt['archives'].append({'plan':arc});save_receipt(receipt);touched.append(arc)
    with open(arc['path'],'r+b') as f:
     assert os.fstat(f.fileno()).st_size==arc['original_size']
     for c in arc['changes']:
      f.seek(c['row_offset']);assert f.read(20)==bytes.fromhex(c['row'])
      f.seek(0,2);gap=c['new_offset']-f.tell();assert 0<=gap<16
      f.write(b'\0'*gap);f.write((root/c['stored_file']).read_bytes())
     f.flush();os.fsync(f.fileno())
     for c in arc['changes']:f.seek(c['row_offset']);f.write(bytes.fromhex(c['new_row']))
     f.seek(0);f.write(bytes.fromhex(arc['new_header']));f.flush();os.fsync(f.fileno())
    assert Path(arc['path']).stat().st_size==arc['new_size']
    receipt['archives'][-1]['installed_sha256']=digest(arc['path']);save_receipt(receipt)
    print('INSTALLED',arc['path'],flush=True)
   receipt['status']='installed';save_receipt(receipt)
  except BaseException:
   for arc in reversed(touched):restore_arc(arc)
   receipt['status']='restored';save_receipt(receipt);raise
print('COMPLETE',mode)
