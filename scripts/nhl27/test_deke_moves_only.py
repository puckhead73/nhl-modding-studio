"""Integration invariants for the actual moves-only native banks."""
import json
from pathlib import Path
import struct
import unittest

from deke_moves_only import NODE, CHOOSER, POLICY, CONTEXT_IDS, allocated_id
from native_ant import Bank


class MovesOnlyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility'
        # This integration suite intentionally fails if its build is absent.
        cls.native = Bank((cls.root/'ant/nhl24_160/launch.bin').read_bytes())
        cls.bank = Bank((cls.root/'deke-trial/moves-only/launch.bank').read_bytes())
        cls.receipt = json.loads((cls.root/'deke-trial/moves-only/integration-receipt.json').read_text())

    def test_native_gameplay_records_unchanged(self):
        replaced = {int(v,16) for v in self.receipt['native_replacements']}
        for ident,obj in self.native.by_id.items():
            if obj['type_hash'] in (0x184a86dd,0x2196d161,0x92de1d8f,0x2fb5a845,0x33e57bd6):
                if ident in replaced:
                    self.assertEqual(ident,NODE)
                    continue
                new = self.bank.by_id[ident]
                self.assertEqual(self.native.blob[obj['offset']:obj['offset']+obj['size']],
                                 self.bank.blob[new['offset']:new['offset']+new['size']])

    def test_every_registered_object_has_a_loader_safe_name(self):
        # Regression: libc strlen(NULL) at 0x809ad30d during startup.
        # A serialized null-name sentinel passes Bank() but is not accepted by
        # NHL24's live registration path.
        self.assertTrue(all(o['name'] is not None for o in self.bank.by_id.values()))

    def test_native_glide_lifetime_and_transition_pointers_preserved(self):
        before = self.native.by_id[NODE]['offset']; after = self.bank.by_id[NODE]['offset']
        self.assertEqual(self.native.blob[before:before+208],self.bank.blob[after:after+208])
        for site,destination in self.native.reloc.items():
            if before<=site<before+208:
                actual = self.bank.pointer(after+site-before)
                if site-before in (136,144):
                    self.assertEqual(actual,self.bank.by_id[CHOOSER]['offset'])
                else:
                    self.assertEqual(actual,destination)

    def test_source_databases_receive_private_context_values(self):
        for ident,count in [(0xe5770b31bcbc233e,22),(0xdd51a2e91e64027c,10)]:
            at=self.bank.by_id[ident]['offset']
            db=self.bank.pointer(at+32)
            self.assertEqual(struct.unpack_from('<Q',self.bank.blob,db+24)[0],count)
            infos=self.bank.pointer(at+64)
            n=struct.unpack_from('<Q',self.bank.blob,at+72)[0]
            values={self.bank.pointer(infos+24*j) for j in range(n)}
            old={self.bank.by_id[i]['offset'] for i in CONTEXT_IDS}
            self.assertFalse(values & old)
            required=[0x2f0fb778b6452f99,0x386e433016486095] if count==22 else [0x346e8e12411acf19]
            self.assertTrue({self.bank.by_id[CONTEXT_IDS[i]]['offset'] for i in required} <= values)

    def test_private_properties_do_not_replace_native_state(self):
        for original,clone in CONTEXT_IDS.items():
            self.assertIn(clone,self.bank.by_id)
            self.assertNotIn(clone,self.native.by_id)
            self.assertEqual(self.bank.by_id[clone]['type_hash'],0x2196d161)
            self.assertNotEqual(self.bank.by_id[clone]['offset'],self.bank.by_id[original]['offset'])
        self.assertEqual(self.receipt['runtime_required'],'deke_skill_stick')

    def test_reverse_turns_use_source_samples_without_changing_general_skating(self):
        root=self.root/'deke-trial/moves-only/turn-audit'
        for original,private in self.receipt['private_turn_clips'].items():
            old,new=int(original,16),int(private,16)
            self.assertEqual(self.bank.clip(old)[2],self.native.clip(old)[2])
            self.assertEqual(self.bank.clip(new)[2],(root/(original+'.resource')).read_bytes())
        for ident in self.receipt['velocity_properties']:
            self.assertIn(int(ident,16),self.bank.by_id)
            self.assertNotIn(int(ident,16),self.native.by_id)

    def test_board_and_net_situations_share_the_complete_same_move(self):
        db=self.bank.pointer(self.bank.by_id[0xdd51a2e91e64027c]['offset']+32)
        entries=self.bank.pointer(db+160)
        # First column contains ten one-byte enum values; source row order is
        # 1,2,3,4,5,6,8,7,9,10, not an assumed ascending enumeration.
        values=list(self.bank.blob[entries:entries+10])
        self.assertEqual(values,[1,2,3,4,5,6,8,7,9,10])
        assets=self.bank.pointer(db+16)
        rows={value:self.bank.pointer(assets+8*i) for i,value in enumerate(values)}
        routes=self.receipt['stationary_pivot']['routes']
        for value,source in [(8,0x75122e5b19cc2e62),(7,0xe1b48b0674842d66),
                             (9,0xcfbf592baad94152),(10,0xdc7b10363575bca2)]:
            self.assertEqual(rows[value],self.bank.by_id[routes[str(source)]]['offset'])
        self.assertEqual(rows[5],self.bank.by_id[0x75122e5b19cc2e62]['offset'])
        self.assertEqual(rows[6],self.bank.by_id[0xe1b48b0674842d66]['offset'])
        self.assertEqual(rows[8],rows[5])
        self.assertEqual(rows[7],rows[6])
        self.assertNotEqual(rows[5],rows[6])
        self.assertEqual(rows[9],rows[5])
        self.assertEqual(rows[10],rows[6])
        self.assertEqual(len({rows[v] for v in range(5,11)}),2)

    def test_shared_net_sequences_retain_all_source_timing_and_events(self):
        from deke_output_audit import compare
        groups=json.loads((self.root/'deke-trial/controller-fragments/report.json').read_text())['partitions']
        remap={int(k,16):int(v,16) for k,v in self.receipt['private_turn_clips'].items()}
        roots={0x75122e5b19cc2e62,0xe1b48b0674842d66}
        checked=set()
        for row in groups:
            if 'fragment' not in row:continue
            f=row['fragment']
            matched={o['id'] for o in f['objects']} & roots
            if matched:
                self.assertFalse(compare(f,self.bank,remap))
                checked.update(matched)
        self.assertEqual(checked,roots)
        self.assertFalse(self.receipt['stationary_pivot']['animation_edits'])
        self.assertFalse(self.receipt['stationary_pivot']['pose_velocity_override'])

    def test_glide_uses_source_context_enum_not_raw_stick(self):
        at=self.bank.by_id[POLICY]['offset']
        self.assertEqual(self.bank.pointer(at+16),self.bank.by_id[CONTEXT_IDS[0xaaa6c8084b72422e]]['offset'])
        self.assertEqual(self.bank.pointer(at+8),self.bank.by_id[allocated_id('context/enum/'+str(0xae0ad0ebd4489066))]['offset'])
        self.assertNotIn('database_mapping',self.receipt)


if __name__=='__main__':
    unittest.main()
