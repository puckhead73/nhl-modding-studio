from pathlib import Path
import hashlib,json,sys
root=Path('E:/'); out=Path('docs/validation/overnight-20261002')
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''): h.update(b)
 return h.hexdigest()
entries={}
for parent in sorted(list(root.glob('nhlms-release-candidate-*-r*'))+list(root.glob('nhlms-review-build-r*'))):
 for p in sorted(parent.rglob('*')):
  if p.is_file(): entries[str(p)]=[p.stat().st_size,sha(p)]
(out/('preservation-'+sys.argv[1]+'.json')).write_text(json.dumps(entries,indent=2),encoding='utf-8')
print(len(entries),'files recorded')
if sys.argv[1]=='after':
 assert entries==json.loads((out/'preservation-before.json').read_text(encoding='utf-8')), 'Prior candidate changed'
 print('All previous candidate archives and extracted trees unchanged')
