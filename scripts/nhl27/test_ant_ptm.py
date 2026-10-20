"""Private-fixture checks of packed search data and initialization ownership."""
import copy
import json
from pathlib import Path
import struct
import unittest

from animation_fields import Fields
from ant_tags import compile_tree
from ant_link import link
from ant_ptm import VEC
from native_ant import Bank
from test_native_ant import sample


class PTMTests(unittest.TestCase):
    def test_parameter_layouts_match_audited_native_factory_sizes(self):
        # Independent executable evidence prevents a source-sized record from
        # accidentally passing a serializer-only round trip.
        from capstone import Cs, CS_ARCH_X86, CS_MODE_64
        from ant_ptm_parameters import LAYOUTS
        from research import SelfImage
        root = Path(__file__).resolve().parents[2]
        catalog = root / 'local/nhl27-eboot/compatibility/deke-trial/controller-native-types.json'
        executable = Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if not catalog.exists() or not executable.exists():
            self.skipTest('Private executable fixtures unavailable')
        import hashlib
        image = SelfImage(executable)
        self.assertEqual(hashlib.sha256(image.data).hexdigest(),
            'fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9')
        types = json.loads(catalog.read_text())
        decoder = Cs(CS_ARCH_X86, CS_MODE_64)
        for name, (type_hash, size) in LAYOUTS.items():
            row = types[name]
            self.assertEqual(type_hash, row['type_hash'])
            factory = struct.unpack('<Q', image.read(int(row['vtable'], 16), 8))[0]
            allocations = [int(op.op_str.split(', ')[1], 0)
                for op in decoder.disasm(image.read(factory, 180), factory)
                if op.mnemonic == 'mov' and op.op_str.startswith('esi,')]
            self.assertEqual(allocations, [size], name)

    def test_source_database_payloads_and_owned_initialization(self):
        root = Path(__file__).resolve().parents[2] / 'local/nhl27-eboot/compatibility'
        graph_file = root / 'deke-trial/integration-closure.json'
        if not graph_file.exists():
            self.skipTest('Private source fixtures unavailable')
        graph = json.loads(graph_file.read_text())
        types = json.loads((root / 'source-type-descriptors.json').read_text())
        masters = databases = trees = 0
        for node in graph['nodes']:
            fields = Fields((root / 'deke-trial/integration-fields' /
                (node['asset']['sha1'] + '.ebx')).read_bytes(), types).export()
            first = fields['instances'][0]
            if first['type'] != ['PTMMasterDatabaseAsset']:
                continue
            result = compile_tree(fields, {'internal_offset': first['offset']}, set())
            masters += 1
            self.assertTrue(result['objects'][0]['initialize'])
            # Owned databases are initialized by the master's vtable callback,
            # not constructed independently by the native object table.
            self.assertFalse(any(o['type_hash'] == 0xa7129111 for o in result['objects']))
            data = result['data']
            rel = {r['site']: r['destination'] for r in result['internal_relocations']}
            by_offset = {i['offset']: i for i in fields['instances']}
            for j, ref in enumerate(first['fields']['Databases']):
                source = by_offset[ref['internal_offset']]['fields']
                db = rel[rel[8] + j * 8]
                self.assertEqual(rel[db + 168], 0)
                self.assertEqual(struct.unpack_from('<Q', data, db + 248)[0], source['DatabaseId'])
                pca = source['PCAData']
                for offset, name in ((192, 'MeanVector'), (208, 'ProjMatrix')):
                    raw = struct.pack('<' + 'f' * len(pca[name]), *pca[name])
                    self.assertEqual(data[rel[db + offset]:rel[db + offset] + len(raw)], raw)
                for k, kd in enumerate(source['PTMKDImpls']):
                    native_kd = rel[db + 344] + k * 56
                    self.assertEqual(struct.unpack_from('<I', data, native_kd + 48)[0], kd['KDTreeDataNumBytes'])
                    if not kd['KDTreeData']:
                        self.assertNotIn(native_kd + 32, rel)
                        continue
                    raw = b''.join(struct.pack('<4I', *(v[n] & 0xffffffff for n in VEC))
                                   for v in kd['KDTreeData'])
                    start = rel[native_kd + 32]
                    self.assertEqual(start % 16, 0)
                    self.assertEqual(data[start:start + len(raw)], raw)
                    trees += 1
                databases += 1
            self.assertTrue(result['compatibility_requirements'])
            with self.assertRaisesRegex(ValueError, 'unqualified source/target'):
                link(Bank(sample()), [result])
            bad = copy.deepcopy(fields)
            db = next(i for i in bad['instances'] if i['type'] == ['PTMDatabaseAsset'])
            db['fields']['PTMKDImpls'][0]['KDTreeDataNumBytes'] += 1
            with self.assertRaisesRegex(ValueError, 'KD tree byte count'):
                compile_tree(bad, {'internal_offset': first['offset']}, set())
        self.assertEqual((masters, databases), (8, 11))
        self.assertGreater(trees, 0)


if __name__ == '__main__':
    unittest.main()
