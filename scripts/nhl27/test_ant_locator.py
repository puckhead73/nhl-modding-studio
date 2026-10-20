"""Validate locator parameters against an original native asset record."""
from pathlib import Path
import struct
import unittest
from ant_tags import compile_tree
from ant_interfaces import adjustment
from native_ant import Bank


def tree(flags):
    return dict(partition='locator-test',instances=[dict(offset=16,
        type=['ScenarioLocatorParamsAsset'],fields=dict(Name='fixture',LocatorFlags=flags))])


class LocatorAssetTests(unittest.TestCase):
    def test_original_record_and_world_locator_flag(self):
        path=Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/ant/nhl24_160/launch.bin'
        if not path.exists():self.skipTest('Private native bank required')
        bank=Bank(path.read_bytes());obj=bank.by_id[0x511d852738276b51]
        self.assertEqual((obj['type_hash'],obj['size']),(0x1b1c4c1f,24))
        raw=bank.blob[obj['offset']:obj['offset']+24]
        result=compile_tree(tree(struct.unpack_from('<I',raw,16)[0]),dict(internal_offset=16),set())
        self.assertEqual(result['data'],raw)
        self.assertEqual(adjustment(0x1b1c4c1f,0xee8d4bf0),8)
        result=compile_tree(tree(256),dict(internal_offset=16),set())
        self.assertEqual(result['data'],bytes(16)+struct.pack('<II',256,0))


if __name__=='__main__':unittest.main()
