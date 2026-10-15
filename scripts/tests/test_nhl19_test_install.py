"""Exercise archive transaction and refusal paths using synthetic bytes only."""
import hashlib,json,subprocess,sys,tempfile,unittest
from pathlib import Path
INSTALL=Path(__file__).resolve().parents[1]/'nhl19_test_install.py'
hashbytes=lambda b:hashlib.sha256(b).hexdigest()
class InstallTest(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory(prefix='nhl19-install-test-');self.root=Path(self.temp.name)
  folder=self.root/'game'/'databigs';folder.mkdir(parents=True)
  self.path=folder/'patch_game_93.big';self.original=bytes(range(256))*4;self.path.write_bytes(self.original)
  self.payload=b'new test payload';(self.root/'payload.bin').write_bytes(self.payload)
  self.arc={'path':str(self.path),'original_size':1024,'original_sha256':hashbytes(self.original),
   'original_header':self.original[:48].hex(),'new_header':(b'X'*48).hex(),'new_size':1024+len(self.payload),
   'changes':[{'row_offset':48,'row':self.original[48:68].hex(),'new_row':(b'Y'*20).hex(),
    'new_offset':1024,'stored_file':'payload.bin','stored_sha256':hashbytes(self.payload)}]}
  (self.root/'package.json').write_text(json.dumps({'game_root':str(folder.parent),'archives':[self.arc]}))
 def tearDown(self):self.temp.cleanup()
 def run_mode(self,mode,ok=True):
  p=subprocess.run([sys.executable,str(INSTALL),str(self.root),mode],capture_output=True,text=True)
  self.assertEqual(p.returncode==0,ok,p.stdout+p.stderr)
 def test_apply_restore_exact_bytes_and_no_double_apply(self):
  self.run_mode('verify');self.run_mode('apply')
  self.assertEqual(self.path.read_bytes()[68:1024],self.original[68:])
  self.assertEqual(self.path.read_bytes()[1024:],self.payload)
  self.run_mode('apply',False);self.run_mode('restore');self.assertEqual(self.path.read_bytes(),self.original)
 def test_corrupt_payload_refused_before_archive_mutation(self):
  (self.root/'payload.bin').write_bytes(b'corrupt');self.run_mode('apply',False)
  self.assertEqual(self.path.read_bytes(),self.original)
 def test_changed_archive_refused_before_apply_or_restore(self):
  self.path.write_bytes(self.original[:-1]+b'X');self.run_mode('apply',False)
  self.path.write_bytes(self.original);self.run_mode('apply')
  with open(self.path,'r+b') as f:f.seek(100);f.write(b'X')
  changed=self.path.read_bytes();self.run_mode('restore',False);self.assertEqual(self.path.read_bytes(),changed)
if __name__=='__main__':unittest.main()
