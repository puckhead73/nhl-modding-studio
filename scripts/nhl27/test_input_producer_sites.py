import unittest
import struct
from input_producer_sites import scan_function


class ProducerSiteTests(unittest.TestCase):
    def test_separated_switch_uses_proven_table_base_and_index_width(self):
        for clobber_base, zero_extend, bypass in ((False,True,False),(True,True,False),
                                                 (False,False,False),(False,True,True)):
            base, table = 0x1000, 0x2000
            code = bytearray(b'\xe8' + struct.pack('<i', 0x3ffbf0-(base+5)))
            code += bytes.fromhex('4989c4')  # r12 keeps record
            lea = len(code)
            code += bytes.fromhex('488d1d') + struct.pack('<i', table-(base+lea+7))
            code += bytes.fromhex('31db' if clobber_base else '9090')
            code += bytes.fromhex('89f9' if zero_extend else '9090')
            code += bytes.fromhex('83f901')  # cmp ecx,1
            code += bytes.fromhex('89542420')  # flags-preserving spill
            guard = len(code)
            code += bytes.fromhex('0f8700000000')
            load = len(code)
            code += bytes.fromhex('4863048bba0b0000004801d8be03000000ffe0')
            first = base+len(code)
            code += bytes.fromhex('498b8424901200004885c07403c60001')
            end = base+len(code)
            code += b'\xc3'
            struct.pack_into('<i',code,guard+2,end-(base+guard+6))
            if bypass:
                code += b'\xe9'+struct.pack('<i',base+load-(base+len(code)+5))
            raw_table = struct.pack('<ii',first-table,first-table)
            reader = lambda va,size: raw_table if va==table and size==8 else None
            row = scan_function(bytes(code),base,reader)[0]
            self.assertEqual(row['load_hook_qualified'], not clobber_base and zero_extend and not bypass)

    def test_complete_indirect_tail_epilogue_is_not_a_switch(self):
        prefix = bytes.fromhex('554889e54154534883ec38')
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - (0x1000+len(prefix)+5))
        body = bytes.fromhex('488b80881200004885c07403c60001')
        tail = bytes.fromhex('4883c4385b415c5dffe0')
        code = prefix + call + body + tail
        row = scan_function(code, 0x1000)[0]
        self.assertTrue(row['load_hook_qualified'])
        self.assertEqual(len(row['restored_frame_tail_calls']), 1)
        for bad in ('4883c4305b415c5dffe0', '4883c4385b415c5dffe3', '4883c438415c5b5dffe0'):
            row = scan_function(prefix + call + body + bytes.fromhex(bad), 0x1000)[0]
            self.assertFalse(row['load_hook_qualified'])

    def test_predicated_store_hook_runs_after_conditions(self):
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - 0x1005)
        prefix = call + bytes.fromhex('4989c6498b8e88120000')
        # Additional stack predicate means no unconditional load-site hook.
        body = bytes.fromhex('4885c97411807c242000740ac60101498b8620070000c3')
        row = scan_function(prefix + body, 0x1000)[0]
        self.assertFalse(row['load_hook_qualified'])
        self.assertTrue(row['store_hook_qualified'])
        self.assertEqual(row['store_hook_bytes'], 'c60101498b8620070000')
        for following in ('488b0500000000', 'e900000000', '90'):
            raw = prefix + body.replace(bytes.fromhex('498b8620070000'), bytes.fromhex(following))
            self.assertFalse(scan_function(raw, 0x1000)[0]['store_hook_qualified'])

    def test_interleaved_stack_restores_preserve_nullable_store(self):
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - 0x1005)
        prefix = call + bytes.fromhex('4989c6')  # r14 = record
        load = bytes.fromhex('498b86e8120000')
        restores = bytes.fromhex('8b5424208b7424248b4c2428')
        tail = bytes.fromhex('4885c07403c60000c3')
        code = prefix + load + restores + tail
        self.assertTrue(scan_function(code, 0x1000)[0]['load_hook_qualified'])
        for bad in ('8b442420', '448b742420', '89442420', '83c201'):
            rows = scan_function(prefix + load + bytes.fromhex(bad) + tail, 0x1000)
            self.assertFalse(any(r['load_hook_qualified'] for r in rows))
        # An external branch into a restore would bypass the helper.
        destination = 0x1000 + len(prefix) + len(load)
        code += b'\xe9' + struct.pack('<i', destination - (0x1000 + len(code) + 5))
        self.assertFalse(any(r['load_hook_qualified'] for r in scan_function(code, 0x1000)))

    def switch_fixture(self, corrupt_record=False, bypass_guard=False, narrow=False, zero_extend=True):
        base, table = 0x1000, 0x2000
        code = bytearray(b'\xe8' + struct.pack('<i', 0x3ffbf0 - (base + 5)))
        code += bytes.fromhex('4889c3')  # rbx = record
        if narrow:
            code += bytes.fromhex('ffcf' if zero_extend else '9090')  # dec edi clears high half
            code += bytes.fromhex('83ff01')
        else:
            code += bytes.fromhex('4883ff01')
        guard = len(code)
        code += bytes.fromhex('0f8700000000')
        lea = len(code)
        code += bytes.fromhex('488d0d') + struct.pack('<i', table - (base + lea + 7))
        code += bytes.fromhex('486304b94801c8ffe0')
        first = base + len(code)
        code += bytes.fromhex('31db' if corrupt_record else '9090')
        second = base + len(code)
        code += bytes.fromhex('488b83901200004885c07403c60001')
        end = base + len(code)
        code += b'\xc3'
        struct.pack_into('<i', code, guard + 2, end - (base + guard + 6))
        if bypass_guard:
            code += b'\xe9' + struct.pack('<i', base + lea - (base + len(code) + 5))
        data = struct.pack('<ii', first - table, second - table)
        return bytes(code), lambda va, size: data if va == table and size == 8 else None

    def test_guarded_switch_edges_participate_in_must_analysis(self):
        for corrupt in (False, True):
            code, reader = self.switch_fixture(corrupt_record=corrupt)
            row = scan_function(code, 0x1000, reader)[0]
            self.assertEqual(len(row['resolved_switches']), 1)
            self.assertEqual(row['load_hook_qualified'], not corrupt)

    def test_32_bit_switch_requires_dominating_zero_extension(self):
        for zero_extend in (False, True):
            code, reader = self.switch_fixture(narrow=True, zero_extend=zero_extend)
            self.assertEqual(scan_function(code, 0x1000, reader)[0]['load_hook_qualified'], zero_extend)
        code, reader = self.switch_fixture(narrow=True)
        # A direct entry at the CMP skips DEC EDI, so the upper index bits are
        # unconstrained even though the low 32-bit comparison passes.
        code += b'\xe9' + struct.pack('<i', 0x100a - (0x1000 + len(code) + 5))
        self.assertFalse(scan_function(code, 0x1000, reader)[0]['load_hook_qualified'])

    def test_switch_rejects_unreadable_tables_and_guard_bypass(self):
        code, reader = self.switch_fixture()
        for invalid_reader in (None, lambda va, size: None,
                               lambda va, size: struct.pack('<ii', -0x1001, -0x1000)):
            row = scan_function(code, 0x1000, invalid_reader)[0]
            self.assertFalse(row['load_hook_qualified'])
            self.assertTrue(row['unresolved_indirect_jumps'])
        code, reader = self.switch_fixture(bypass_guard=True)
        self.assertFalse(scan_function(code, 0x1000, reader)[0]['load_hook_qualified'])

    def test_switch_rejects_wrong_guard_clobbered_index_and_case_guard_entry(self):
        code, reader = self.switch_fixture()
        wrong_guard = code.replace(bytes.fromhex('0f87'), bytes.fromhex('0f85'), 1)
        clobbered = code.replace(bytes.fromhex('4883ff01'), bytes.fromhex('4883f901'), 1)
        clobbered = clobbered.replace(bytes.fromhex('486304b9'), bytes.fromhex('48630489'), 1)
        # Entry at 0x100c bypasses the CMP and reaches the JA with stale flags.
        guard_entry = lambda va, size: struct.pack('<ii', 0x100c - 0x2000, 0x100c - 0x2000)
        for raw, fetch in ((wrong_guard, reader), (clobbered, reader), (code, guard_entry)):
            row = scan_function(raw, 0x1000, fetch)[0]
            self.assertFalse(row['load_hook_qualified'])
            self.assertTrue(row['unresolved_indirect_jumps'])

    def test_nullable_pointer_store(self):
        # mov rax,[r13+1290]; test rax,rax; je end; mov byte ptr[rax],1
        code = bytes.fromhex('498b85901200004885c07403c60001c3')
        rows = scan_function(code, 0x1000)
        self.assertEqual(len(rows), 1)
        self.assertEqual((rows[0]['action'], rows[0]['record_register'], rows[0]['value']),
                         (219, 'r13', 1))
        self.assertFalse(rows[0]['load_hook_qualified'])
        self.assertTrue(rows[0]['guarded_hook_qualified'])

    def test_conditional_same_value_store_uses_observation_hook(self):
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - 0x1005)
        for compare_value, store_value, expected in ((0, 0, True), (1, 1, True), (0, 1, False)):
            body = bytes.fromhex('488b80901200004885c074088038') + bytes([compare_value])
            body += bytes.fromhex('7403c600') + bytes([store_value]) + b'\xc3'
            row = scan_function(call + body, 0x1000)[0]
            self.assertEqual(row['load_hook_qualified'], expected)
            self.assertEqual(row['producer_mode'], 'changed_constant')

    def test_changed_store_can_use_shared_backward_continuation(self):
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - 0x1005)
        # Both null and unchanged paths return through a shared earlier RET.
        code = call + bytes.fromhex('eb01c3488b80901200004885c074f380380074eec60000c3')
        row = scan_function(code, 0x1000)[0]
        self.assertTrue(row['load_hook_qualified'])
        self.assertEqual(row['producer_mode'], 'changed_constant')

    def test_partial_register_writes_calls_and_destroyed_record_reject_provenance(self):
        for middle in ('b000', '31c0', '4531ed', 'e800000000'):
            code = bytes.fromhex('498b8590120000' + middle + 'c60001c3')
            self.assertEqual(scan_function(code, 0x1000), [])
        rows = scan_function(bytes.fromhex('488b8090120000c60001c3'), 0x1000)
        self.assertEqual(len(rows), 1)
        self.assertTrue(rows[0]['record_destroyed_by_load'])

    def test_record_provenance_requires_all_incoming_paths(self):
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - 0x1005)
        # mov rbx,rax; test edi,edi; je join; [two bytes]; join: load/store
        for middle, expected in [('9090', True), ('31db', False)]:
            code = call + bytes.fromhex('4889c385ff7402' + middle +
                                       '488b83901200004885c07403c60001c3')
            rows = scan_function(code, 0x1000)
            self.assertEqual(len(rows), 1)
            self.assertEqual(bool(rows[0]['cfg_getter_calls']), expected)

    def test_callee_saved_record_survives_call_but_rax_does_not(self):
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - 0x1005)
        code = call + bytes.fromhex('4989c4e800000000498b8424901200004885c07403c60001c3')
        self.assertEqual(scan_function(code, 0x1000)[0]['cfg_getter_calls'], [0x1000])

    def test_qualified_hook_rejects_unknown_edges_and_conditional_store(self):
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - 0x1005)
        body = bytes.fromhex('488b80901200004885c07403c60001c3')
        row = scan_function(call + body, 0x1000)[0]
        self.assertTrue(row['load_hook_qualified'])
        self.assertTrue(row['record_destroyed_by_load'])
        row = scan_function(call + body + bytes.fromhex('ffe0'), 0x1000)[0]
        self.assertIn('unresolved_indirect_control_flow', row['hook_rejections'])
        changed = call + bytes.fromhex('488b80901200004885c07503c60001c3')
        self.assertFalse(scan_function(changed, 0x1000)[0]['load_hook_qualified'])

    def test_null_branch_can_skip_larger_block_but_must_land_on_instruction(self):
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - 0x1005)
        for displacement, expected in ((5, True), (4, False), (127, False), (1, False)):
            # The extra MOV after the store is skipped by a null pointer. A
            # branch into its immediate, outside the function, or into the
            # store is not qualified.
            body = bytes.fromhex('488b80901200004885c074') + bytes([displacement])
            body += bytes.fromhex('c60001b001c3')
            row = scan_function(call + body, 0x1000)[0]
            self.assertEqual(row['load_hook_qualified'], expected)

    def test_null_branch_can_use_backward_continuation(self):
        call = b'\xe8' + struct.pack('<i', 0x3ffbf0 - 0x1005)
        # A null pointer retries through the getter; non-null still stores.
        body = bytes.fromhex('488b80901200004885c074efc60001c3')
        row = scan_function(call + body, 0x1000)[0]
        self.assertTrue(row['load_hook_qualified'])


if __name__ == '__main__':
    unittest.main()
