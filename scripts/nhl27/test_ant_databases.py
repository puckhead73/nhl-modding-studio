"""Private-fixture checks against source bytes and original native query tables."""
import json
from pathlib import Path
import struct
import unittest

from animation_fields import Fields
from ant_tags import compile_tree
from deke_native_audit import native_id
from native_ant import Bank


class DatabaseTests(unittest.TestCase):
    def test_source_payload_bits_and_existing_comparison_slots(self):
        root = Path(__file__).resolve().parents[2] / 'local/nhl27-eboot/compatibility'
        if not (root / 'deke-trial/integration-closure.json').exists():
            self.skipTest('Private source fixtures are unavailable')
        types = json.loads((root / 'source-type-descriptors.json').read_text())
        graph = json.loads((root / 'deke-trial/integration-closure.json').read_text())
        bank = Bank((root / 'ant/nhl24_160/launch.bin').read_bytes())
        count = query_checks = 0
        for node in graph['nodes']:
            reader = Fields((root / 'deke-trial/integration-fields' / (node['asset']['sha1'] + '.ebx')).read_bytes(), types)
            fields = reader.export()
            for instance in fields['instances']:
                kind = instance['type'][0]
                if kind not in ('ContextDatabase', 'PriorityDatabase'):
                    continue
                names = ['EntriesV'] + (['PrioritiesV'] if kind == 'PriorityDatabase' else [])
                raw = {name: reader.raw_array(instance['offset'], name) for name in names}
                instance['fields']['_native_raw_arrays'] = raw
                # Isolate the owned database payload from its parent asset;
                # parent identity/category wiring is exercised by graph staging.
                instance['fields']['CDBAsset'] = None
                result = compile_tree(fields, {'internal_offset': instance['offset']}, set())
                reloc = {r['site']: r['destination'] for r in result['internal_relocations']}
                for slot, name in ((160, 'EntriesV'), (264, 'PrioritiesV')):
                    if name not in raw or not raw[name]['count']:
                        continue
                    start = reloc[slot]
                    self.assertEqual(result['data'][start:start + len(raw[name]['data'])], raw[name]['data'])
                self.assertEqual(struct.unpack_from('<3I', result['data'], 232),
                                 (instance['fields']['NumEntries'], 0, instance['fields']['NumEntries']))
                count += 1
                if node['name'].endswith(('pcdb_psk_lpd_bank', 'pcdb_toedrag')):
                    obj = bank.by_id[native_id(fields['partition'])]
                    original = bank.pointer(bank.pointer(obj['offset'] + 32) + 144)
                    # First 24 comparison ends are meaningful; the serializer's
                    # 25th word can contain padding from its input descriptor.
                    self.assertEqual(result['data'][reloc[144]:reloc[144] + 104],
                                     bank.blob[original:original + 104])
                    query_checks += 1
        self.assertEqual(count, 13)
        self.assertEqual(query_checks, 2)


if __name__ == '__main__':
    unittest.main()
