"""Installed-baseline checks use synthetic workspace assets only."""
import hashlib
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest

REPO=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(REPO/'experiments/codex/nhl21_package'))
from installed_baseline import verify_installed, verify_original_table


class BaselineTests(unittest.TestCase):
    def setUp(self):
        runs=REPO/'local/runs/codex/nhl21-baseline-tests'
        runs.mkdir(parents=True,exist_ok=True)
        self.root=Path(tempfile.mkdtemp(dir=runs))
        self.packet=self.root/'packet';self.packet.mkdir()
        self.archive=self.root/'CUSA18182/databigs/game_90.big'
        self.archive.parent.mkdir(parents=True)
        data=bytearray(160);data[:4]=b'EB\0\3';struct.pack_into('>Q',data,24,len(data))
        row=struct.pack('>5I',8,17,32,19,20);data[48:68]=row;data[128:160]=b'x'*32
        self.archive.write_bytes(data)
        self.arc={'path':str(self.archive),'new_size':160,'new_header':data[:48].hex(),
            'changes':[{'path':'head.mcd','original_decoded_sha256':'donor','row_offset':48,
                'new_offset':128,'new_row':row.hex(),'stored_sha256':hashlib.sha256(b'x'*32).hexdigest()}]}
        (self.packet/'package.json').write_text(json.dumps({'archives':[self.arc]}))
        (self.packet/'installation.json').write_text(json.dumps({'status':'installed','archives':[{'plan':self.arc}]}))
        self.pre={'targets':[{'asset_path':'head.mcd','archive':str(self.archive),'decoded_sha256':'donor','row_offset':48}]}

    def test_matching_installed_entry(self):
        self.assertEqual(len(verify_installed(self.packet,self.pre)['targets']),1)

    def test_modified_payload_rejected(self):
        with self.archive.open('r+b') as f:f.seek(130);f.write(b'y')
        with self.assertRaisesRegex(ValueError,'payload changed'):verify_installed(self.packet,self.pre)

    def test_wrong_donor_rejected(self):
        self.pre['targets'][0]['decoded_sha256']='other'
        with self.assertRaisesRegex(ValueError,'qualified donor'):verify_installed(self.packet,self.pre)

    def test_manifest_receipt_mismatch_rejected(self):
        self.arc['new_size']=161
        (self.packet/'package.json').write_text(json.dumps({'archives':[self.arc]}))
        with self.assertRaisesRegex(ValueError,'differs from receipt'):verify_installed(self.packet,self.pre)

    def test_recorded_table_edits_only(self):
        original=bytearray(self.archive.read_bytes()[:128])
        struct.pack_into('>Q',original,24,128)
        original[48:68]=struct.pack('>5I',6,17,16,19,20)
        arc={**self.arc,'original_header':original[:48].hex(),
             'changes':[{**self.arc['changes'][0],'row':original[48:68].hex()}]}
        digest=hashlib.sha256(original).hexdigest()
        table=self.archive.read_bytes()[:128]
        verify_original_table(table,arc,digest)
        altered=bytearray(table);altered[80]^=1
        with self.assertRaisesRegex(ValueError,'unrecorded'):verify_original_table(altered,arc,digest)


if __name__=='__main__':unittest.main()
