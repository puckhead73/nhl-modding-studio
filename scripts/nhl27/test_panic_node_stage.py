"""Private-bank structural qualification; does not waive runtime requirements."""
import copy
import json
import struct
import unittest
from pathlib import Path
from ant_link import link
from native_ant import Bank
from panic_node_stage import compile_fragment, NODE, NODE_ACTIVE, ACTIVE

ROOT = Path('local/nhl27-eboot/compatibility')


@unittest.skipUnless((ROOT/'ant/nhl24_160/launch.bin').exists(), 'Private native bank unavailable')
class PanicNodeTests(unittest.TestCase):
    def test_node_extension_preserves_native_transitions_and_incoming_interfaces(self):
        old = Bank((ROOT/'ant/nhl24_160/launch.bin').read_bytes())
        source = json.loads((ROOT/'deke-trial/panic-bindings/c827ee683e2173877b33d00157f71f666875ef05.json').read_text())
        fragment = compile_fragment(old, source)
        with self.assertRaisesRegex(ValueError, 'unqualified'):
            link(old, [fragment], replace_ids=[NODE])
        # This copy exercises serialization in isolation, never an install path.
        fixture = copy.deepcopy(fragment)
        fixture.pop('compatibility_requirements')
        new = Bank(link(old, [fixture], replace_ids=[NODE], external_types={NODE_ACTIVE: 0x33e57bd6}))
        at, original = new.by_id[NODE]['offset'], old.by_id[NODE]['offset']
        self.assertEqual(new.blob[:len(old.blob)], old.blob)
        for ref in fixture['inherited_relocations']:
            self.assertEqual(new.pointer(at+ref['site']), old.pointer(ref['source_site']))
        external = {row[1]: row for row in new.external}
        for field, interface, delta in ((40, 0xbcc2c469, 64), (72, 0x28185be6, 72)):
            array = new.pointer(at+field)
            self.assertEqual(struct.unpack_from('<Q',new.blob,at+field+8)[0], 2)
            self.assertEqual(new.pointer(array), new.by_id[ACTIVE]['offset']+delta)
            self.assertEqual(external[array+8][2:4], (NODE_ACTIVE,interface))
        floats = new.pointer(at+56)
        self.assertEqual(new.blob[floats:floats+16],bytes(16))
        self.assertNotIn(floats,new.reloc)
        for site,dest in old.relocations:
            if original <= dest < original+208:
                self.assertEqual(new.pointer(site),at+dest-original)
        changed = copy.deepcopy(source)
        changed['instances'][0]['fields']['ActiveBoolAssets'].reverse()
        with self.assertRaisesRegex(ValueError,'activation list changed'):
            compile_fragment(old, changed)


if __name__ == '__main__':
    unittest.main()
