import unittest
from deke_closure_stage import closure
from deke_native_audit import native_id


class ClosureTests(unittest.TestCase):
    def test_existing_partition_does_not_hide_new_exported_enumerator(self):
        root = '00000001-0000-0000-0000-000000000000'
        part = '00000002-0000-0000-0000-000000000000'
        new = '00000003-0000-0000-0000-000000000000'
        catalog = [dict(asset={'sha1': 'root'}, fixup=dict(partition=root,
            imports=[dict(partition=part, instance=part), dict(partition=part, instance=new)])),
            dict(asset={'sha1': 'enum'}, fixup=dict(partition=part, imports=[]))]
        graph = closure(catalog, [root], {native_id(part)})
        self.assertEqual(len(graph['nodes']), 2)
        self.assertEqual(graph['reused_partitions'], [])
        self.assertFalse(graph['missing'])


if __name__ == '__main__':
    unittest.main()
