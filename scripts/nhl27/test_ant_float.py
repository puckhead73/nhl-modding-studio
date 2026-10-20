"""Compare emitted FloatAsset records to the original target memory image."""
import copy
from pathlib import Path
import struct
import unittest
from ant_tags import compile_tree
from ant_interfaces import adjustment
from native_ant import Bank


def tree(default):
    return dict(partition='float-test',instances=[dict(offset=16,type=['FloatAsset'],fields={
        'Name':'fixture','0xb1e00a80':True,
        '0x6516a1e3':dict(LayoutHash=10,ElementSize=4,Offset=0,Name=1,Count=1,Flags=0,
                         ElementAlign=4,LayoutPtr=0,**{'0xaacc1dc':1}),
        '0x3593b156':default})])


class FloatAssetTests(unittest.TestCase):
    def test_original_record_bytes_and_distinct_interface(self):
        path=Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/ant/nhl24_160/launch.bin'
        if not path.exists():
            self.skipTest('Private original native bank unavailable')
        bank=Bank(path.read_bytes())
        obj=bank.by_id[0x7913cf0fc3f631c4]
        self.assertEqual((obj['type_hash'],obj['size']),(0x028883b0,88))
        raw=bank.blob[obj['offset']:obj['offset']+obj['size']]
        default=struct.unpack_from('<f',raw,80)[0]
        result=compile_tree(tree(default),dict(internal_offset=16),set())
        self.assertEqual(result['data'],raw)
        self.assertEqual(adjustment(0x028883b0,0x67745308),64)
        self.assertEqual(adjustment(0x028883b0,0x7a327d7d),0)

    def test_wrong_entry_layout_is_rejected(self):
        bad=copy.deepcopy(tree(0.0))
        bad['instances'][0]['fields']['0x6516a1e3']['LayoutHash']=6
        with self.assertRaisesRegex(ValueError,'entry layout'):
            compile_tree(bad,dict(internal_offset=16),set())


if __name__ == '__main__':
    unittest.main()
