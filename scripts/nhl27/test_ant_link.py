import unittest
import struct
import hashlib
from ant_link import link
from native_ant import Bank
from test_native_ant import sample


def fragment(ident=1 << 63):
    return dict(data=bytes(72), objects=[dict(id=ident, offset=0, size=72,
        type_hash=1556467357, name='inserted', initialize=True)],
        symbolic_relocations=[dict(site=32, target_id=456)])


class LinkTests(unittest.TestCase):
    def test_inherited_fixup_preserves_native_destination_and_retargets_replacement(self):
        old = Bank(sample())
        f = fragment()
        f['symbolic_relocations'] = []
        f['template_bank_sha256'] = hashlib.sha256(old.data).hexdigest()
        f['inherited_relocations'] = [dict(site=32, source_site=old.by_id[123]['offset']+32)]
        target = old.by_id[456]
        replacement = dict(data=bytes(target['size']), objects=[dict(id=456, offset=0,
            size=target['size'], type_hash=target['type_hash'])])
        new = Bank(link(old, [f, replacement], replace_ids=[456]))
        self.assertEqual(new.pointer(new.by_id[1 << 63]['offset']+32), new.by_id[456]['offset'])
        f['template_bank_sha256'] = '0'*64
        with self.assertRaisesRegex(ValueError, 'template bank mismatch'):
            link(old, [f])
        f['template_bank_sha256'] = hashlib.sha256(old.data).hexdigest()
        f['inherited_relocations'][0]['source_site'] = len(old.blob)+8
        with self.assertRaisesRegex(ValueError, 'unambiguous native fixup'):
            link(old, [f])

    def test_signed_sort_and_pointer_resolution(self):
        old = Bank(sample())
        new = Bank(link(old, [fragment()]))
        ident = 1 << 63
        self.assertEqual(new.objects[0][1], ident)
        self.assertEqual(new.by_id[ident]['name'], 'inserted')
        self.assertEqual(list(new.values('indices', '<I')), [0])
        self.assertEqual(new.pointer(new.by_id[ident]['offset'] + 32), old.by_id[456]['offset'])
        self.assertEqual(new.clip(123)[2], old.clip(123)[2])

    def test_missing_target_and_tags_fail_closed(self):
        f = fragment()
        f['symbolic_relocations'][0]['target_id'] = 999
        with self.assertRaises(ValueError): link(Bank(sample()), [f])
        f = fragment()
        f['unresolved_dependencies'] = [{'field': 'TagCollectionSetAsset'}]
        with self.assertRaises(ValueError): link(Bank(sample()), [f])

    def test_duplicate_and_unknown_type_rejected(self):
        with self.assertRaises(ValueError): link(Bank(sample()), [fragment(123)])
        f = fragment()
        f['objects'][0]['size'] = 64
        with self.assertRaises(ValueError): link(Bank(sample()), [f])

    def test_empty_link_is_byte_identical(self):
        self.assertEqual(link(Bank(sample()), []), sample())

    def test_registry_extension_preserves_existing_type_indices(self):
        old = Bank(sample())
        f = dict(data=bytes(16), objects=[dict(id=789, offset=0, size=16, type_hash=0x302ef52f)])
        with self.assertRaisesRegex(ValueError, 'absent from target registry'):
            link(old, [f])
        new = Bank(link(old, [f], registered_types=[(0x302ef52f, 16)]))
        self.assertEqual(new.sizes[:-1], old.sizes)
        self.assertEqual(new.hashes[:-1], old.hashes)
        self.assertEqual(new.by_id[789]['type_index'], len(old.sizes))
        for ident in old.by_id:
            self.assertEqual(new.by_id[ident]['type_index'], old.by_id[ident]['type_index'])
        self.assertEqual(new.clip(123)[2], old.clip(123)[2])
        with self.assertRaisesRegex(ValueError, 'Conflicting sizes'):
            link(old, [f], registered_types=[(0x302ef52f, 16), (0x302ef52f, 24)])

    def test_cross_bank_interface_fixup_keeps_identity_and_requested_type(self):
        f = fragment()
        f['symbolic_relocations'] = [dict(site=32, target_id=999, interface=0xbcc2c469)]
        old = Bank(sample())
        new = Bank(link(old, [f], external_types={999: 0x33e57bd6}))
        site = new.by_id[1 << 63]['offset'] + 32
        self.assertEqual(new.external[-1], (-1, site, 999, 0xbcc2c469, 0))
        self.assertNotIn(site, new.reloc)
        self.assertEqual(new.blob[site:site + 8], bytes(8))
        f['symbolic_relocations'].append(dict(site=32, target_id=456))
        with self.assertRaisesRegex(ValueError, 'Multiple fixups'):
            link(old, [f], external_types={999: 0x33e57bd6})

    def test_replacement_retargets_incoming_references(self):
        old = Bank(sample())
        target = old.by_id[456]
        f = dict(data=bytes(target['size']), objects=[dict(id=456, offset=0,
            size=target['size'], type_hash=target['type_hash'], name='replacement')])
        new = Bank(link(old, [f], replace_ids=[456]))
        self.assertEqual(len(new.objects), len(old.objects))
        self.assertEqual(new.pointer(new.by_id[123]['offset'] + 32), new.by_id[456]['offset'])
        self.assertNotEqual(new.by_id[456]['offset'], old.by_id[456]['offset'])
        self.assertEqual(new.blob[:len(old.blob)], old.blob)

    def test_replacement_requires_explicit_complete_same_layout_set(self):
        old = Bank(sample())
        with self.assertRaisesRegex(ValueError, 'Not every requested replacement'):
            link(old, [fragment()], replace_ids=[456])
        with self.assertRaisesRegex(ValueError, 'preserve native type'):
            link(old, [fragment(456)], replace_ids=[456])

    def test_replacement_preserves_interior_pointer_adjustment(self):
        raw = bytearray(sample())
        initial = Bank(raw)
        table, _, stride = initial.sections['relocations']
        for index, (site, dst) in enumerate(initial.relocations):
            if site == initial.by_id[123]['offset'] + 32:
                struct.pack_into('<Q', raw, table + index * stride + 8, dst + 16)
        old = Bank(raw)
        target = old.by_id[456]
        f = dict(data=bytes(target['size']), objects=[dict(id=456, offset=0,
            size=target['size'], type_hash=target['type_hash'])])
        new = Bank(link(old, [f], replace_ids=[456]))
        self.assertEqual(new.pointer(new.by_id[123]['offset'] + 32), new.by_id[456]['offset'] + 16)


if __name__ == '__main__':
    unittest.main()
