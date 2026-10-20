import struct
import unittest
from input_pointer_flow import scan


class PointerFlowTests(unittest.TestCase):
    def prefix(self):
        # Proven record in RAX and callee-saved R12, pointer in RBX.
        return (b'\xe8'+struct.pack('<i',0x3ffbf0-0x1005)
                +bytes.fromhex('4989c4488b9890120000'))

    def test_same_pointer_survives_two_branches_and_register_copy(self):
        raw = self.prefix()+bytes.fromhex('85ff74049090eb0290904889d9c60101c3')
        rows = scan(raw,0x1000)
        self.assertEqual(len(rows),1)
        self.assertEqual(rows[0]['action'],219)
        self.assertEqual(rows[0]['pointer_register'],'rcx')
        self.assertIn('r12',rows[0]['record_registers'])

    def test_partial_register_clobber_on_one_path_invalidates_join(self):
        raw = self.prefix()+bytes.fromhex('85ff740431dbeb029090c60301c3')
        self.assertEqual(scan(raw,0x1000),[])

    def test_different_binding_actions_do_not_merge(self):
        raw = self.prefix()+bytes.fromhex('85ff7409488b9888120000eb0090c60301c3')
        self.assertEqual(scan(raw,0x1000),[])

    def test_unknown_records_are_not_treated_as_boolean_bindings(self):
        self.assertEqual(scan(bytes.fromhex('488b9890120000c60301c3'),0x1000),[])

    def test_saved_pointer_survives_call_but_volatile_copy_does_not(self):
        for reg,store,count in [('rbx','c60301',1),('rcx','c60101',0)]:
            raw = self.prefix()+bytes.fromhex('4889d9')
            raw += b'\xe8'+struct.pack('<i',0x2000-(0x1000+len(raw)+5))
            raw += bytes.fromhex(store+'c3')
            rows = scan(raw,0x1000)
            self.assertEqual(len(rows),count)
            if rows:
                self.assertEqual(rows[0]['record_registers'],['r12'])


if __name__ == '__main__':
    unittest.main()
