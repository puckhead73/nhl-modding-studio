"""Verify the paired input allocation patch in a private, reconstructed image."""
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest

from build_input_eboot import run
from input_producer_hooks import selected_sites
from research import ElfImage, SelfImage
from deke_runtime_evidence import TARGET_HASH
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_REG_RIP
import input_asset_hook
import input_frame_hook
import input_human_hook
import input_owner_hooks
import input_join_hooks
import input_release_hooks
import input_reset_hook
import panic_hooks
import panic_lane_hooks
import input_glide_hooks
import ptm_lifecycle_hooks
import ptm_evaluate_hooks
import ptm_blend_lifecycle_hooks
import ptm_blend_update_hooks
import ptm_velocity_hooks


class InputEbootTests(unittest.TestCase):
    def test_producer_trampolines_link_back_to_each_displaced_instruction(self):
        target = Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        root = Path(__file__).resolve().parents[2]
        inventory = root / 'local/nhl27-eboot/compatibility/deke-trial/target-input-producer-sites.json'
        if not target.exists() or not compiler.exists() or not inventory.exists():
            self.skipTest('Private target/inventory/toolchain unavailable')
        scratch = inventory.parent / 'runtime-build'
        sites = selected_sites(json.loads(inventory.read_text())['sites'] + input_join_hooks.qualified(SelfImage(target)))
        with tempfile.TemporaryDirectory(prefix='producer-eboot-fixture-', dir=scratch) as folder:
            report = run(target, Path(folder), compiler, inventory)
            image = ElfImage(Path(report['path']))
            symbols = {k: int(v, 16) for k, v in report['payload']['symbols'].items()}
            decoder = Cs(CS_ARCH_X86, CS_MODE_64)
            decoder.detail = True
            for site in sites:
                start = symbols[f"producer_{site['hook_address']:x}"]
                patch = next(decoder.disasm(image.read(site['hook_address'], 5), site['hook_address']))
                self.assertEqual(patch.mnemonic, 'jmp')
                self.assertEqual(patch.operands[0].imm, start)
                limit = int(report['payload']['va'], 16) + report['payload']['size']
                instructions = []
                for ins in decoder.disasm(image.read(start, min(1024, limit-start)), start):
                    instructions.append(ins)
                    if ins.mnemonic == 'jmp':
                        break
                self.assertEqual(instructions[-1].mnemonic, 'jmp')
                self.assertEqual(instructions[-1].operands[0].imm,
                                 site.get('resume_address',site['hook_address'] + len(bytes.fromhex(site['hook_bytes']))))
                replay = site.get('replay_bytes',site['hook_bytes'])
                displaced = list(decoder.disasm(bytes.fromhex(replay), site['hook_address']))
                tail = 2 if 'null_branch_address' in site else 1
                self.assertEqual(b''.join(i.bytes for i in instructions[-tail-len(displaced):-tail]).hex(),replay)
                if 'null_branch_address' in site:
                    self.assertEqual(instructions[-2].mnemonic,'je')
                    self.assertEqual(instructions[-2].operands[0].imm,site['null_branch_address'])
                calls = [i for i in instructions if i.mnemonic == 'call']
                helper = ('nhl24_input_observe_change_guest'
                          if site.get('producer_mode') == 'changed_constant'
                          else 'nhl24_input_produce_guest')
                if site.get('runtime_class_guard'):
                    helper = 'nhl24_input_guarded_produce_guest'
                    if site.get('predicate_kind') in ('clear_byte_human_actor','byte_or_stack_flag'):
                        helper = 'nhl24_input_observe_predicate_guest'
                    references = [i.address+i.size+o.mem.disp for i in instructions
                                  if i.mnemonic == 'lea' for o in i.operands
                                  if o.type == X86_OP_MEM and o.mem.base == X86_REG_RIP]
                    self.assertIn(site['hook_address'], references)
                self.assertEqual([i.operands[0].imm for i in calls],
                                 [symbols[helper]])
                self.assertFalse(any(i.mnemonic.startswith('v') for i in instructions))
            self.assertEqual(len(report['patches']), len(sites) + 4 + len(input_human_hook.SITES)
                             + len(input_owner_hooks.SITES) + 4 + len(input_release_hooks.SITES)
                             + len(input_glide_hooks.SITES) + len(input_glide_hooks.ACTIVE_PATCHES) + 2
                             + len(ptm_lifecycle_hooks.ALLOCATIONS) + len(ptm_lifecycle_hooks.CONSTRUCTORS)
                             + len(ptm_lifecycle_hooks.SITES) + len(ptm_evaluate_hooks.SITES)
                             + len(ptm_blend_lifecycle_hooks.patches(SelfImage(target),symbols))
                             + len(ptm_blend_update_hooks.patches(SelfImage(target),symbols))
                             + len(ptm_velocity_hooks.patches(SelfImage(target),symbols))
                             + len(panic_lane_hooks.patches(SelfImage(target),symbols)))
            for module in (ptm_blend_lifecycle_hooks,ptm_blend_update_hooks,ptm_velocity_hooks,panic_lane_hooks):
                for va,before,after in module.patches(SelfImage(target),symbols):
                    self.assertEqual(image.read(va,len(after)),after)
            for va,before,after in ptm_evaluate_hooks.patches(SelfImage(target),symbols):
                self.assertEqual(image.read(va,len(after)),after)
            for va, before, after in ptm_lifecycle_hooks.patches(SelfImage(target),symbols):
                self.assertEqual(image.read(va,len(after)),after)
            for va in ptm_lifecycle_hooks.CONSTRUCTORS:
                start=symbols[f'ptm_construct_{va:x}']
                body=[]
                for ins in decoder.disasm(image.read(start,1024),start):
                    body.append(ins)
                    if ins.mnemonic=='jmp':break
                self.assertEqual(body[-1].operands[0].imm,ptm_lifecycle_hooks.CONSTRUCTOR)
                self.assertEqual([i.operands[0].imm for i in body if i.mnemonic=='call'],
                                 [symbols['nhl24_ptm_match_initialize_guest']])
            for va,raw,register,helper in ptm_lifecycle_hooks.SITES:
                start=symbols[f'ptm_lifecycle_{va:x}']
                body=[]
                for ins in decoder.disasm(image.read(start,1024),start):
                    body.append(ins)
                    if ins.mnemonic=='jmp':break
                replay=body[-1].operands[0].imm
                self.assertEqual(image.read(replay,len(bytes.fromhex(raw))),bytes.fromhex(raw))
                jump=next(decoder.disasm(image.read(replay+len(bytes.fromhex(raw)),5),replay+len(bytes.fromhex(raw))))
                self.assertEqual(jump.operands[0].imm,va+len(bytes.fromhex(raw)))
                self.assertEqual([i.operands[0].imm for i in body if i.mnemonic=='call'],[symbols[helper]])
            va, before, after = input_glide_hooks.HUMAN_OVERRIDE_PATCH
            self.assertEqual(image.read(va,2),bytes.fromhex(after))
            self.assertEqual(next(decoder.disasm(image.read(va,2),va)).operands[0].imm,0x2eccabc)
            for va in input_glide_hooks.SITES:
                jump = next(decoder.disasm(image.read(va,5),va))
                self.assertEqual(jump.mnemonic,'jmp')
                start = symbols[f'input_glide_decode_{va:x}']
                self.assertEqual(jump.operands[0].imm,start)
                body=[]
                for ins in decoder.disasm(image.read(start,128),start):
                    body.append(ins)
                    if ins.mnemonic=='jmp':break
                self.assertEqual(body[-1].operands[0].imm,va+5)
            self.assertEqual(image.read(panic_hooks.CACHE_NORMALIZATION_SITE,3),
                             panic_hooks.CACHE_NORMALIZATION_AFTER)
            for va in input_release_hooks.SITES:
                call = next(decoder.disasm(image.read(va,5),va))
                self.assertEqual(call.mnemonic,'call')
                start = symbols[f'input_release_{va:x}']
                self.assertEqual(call.operands[0].imm,start)
                body = []
                for ins in decoder.disasm(image.read(start,1024),start):
                    body.append(ins)
                    if ins.mnemonic == 'jmp':
                        break
                self.assertEqual(body[-1].operands[0].imm,input_release_hooks.ORIGINAL)
                self.assertEqual([i.operands[0].imm for i in body if i.mnemonic == 'call'],
                                 [symbols['nhl24_input_assign_actor_guest']])
            start = symbols[input_asset_hook.NAME]
            limit = int(report['payload']['va'], 16) + report['payload']['size']
            asset_code = []
            for ins in decoder.disasm(image.read(start, min(1024, limit-start)), start):
                asset_code.append(ins)
                if ins.mnemonic == 'jmp':
                    break
            references = [i.address+i.size+o.mem.disp for i in asset_code
                          if i.mnemonic == 'lea' for o in i.operands
                          if o.type == X86_OP_MEM and o.mem.base == X86_REG_RIP]
            self.assertIn(input_asset_hook.ADDRESS, references)
            self.assertEqual(asset_code[-1].operands[0].imm,
                             input_asset_hook.ADDRESS + len(input_asset_hook.EXPECTED))
            frame_call = next(decoder.disasm(image.read(input_frame_hook.ADDRESS,5), input_frame_hook.ADDRESS))
            self.assertEqual(frame_call.mnemonic, 'call')
            self.assertEqual(frame_call.operands[0].imm, symbols[input_frame_hook.NAME])

    def test_private_image_preserves_payloads_and_links_the_publication_hook(self):
        target = Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if not target.exists() or not compiler.exists():
            self.skipTest('Private target binary/toolchain unavailable')
        scratch = Path(__file__).resolve().parents[2] / 'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='eboot-fixture-', dir=scratch) as folder:
            report = run(target, Path(folder), compiler)
            image = ElfImage(Path(report['path']))
            original = SelfImage(target)
            self.assertFalse(report['installed'])
            self.assertEqual(hashlib.sha256(original.data).hexdigest(), TARGET_HASH)
            self.assertEqual(image.read(0x3ffc58, 5), bytes.fromhex('bea0210000'))
            call = image.read(0x3ffc9e, 5)
            self.assertEqual(call[0], 0xe8)
            resolved = 0x3ffca3 + struct.unpack_from('<i', call, 1)[0]
            self.assertEqual(resolved, int(report['payload']['symbols']['nhl24_input_publish_extended'], 16))
            self.assertEqual(image.ph[11]['flags'], 5)
            rw = image.ph[2]
            self.assertEqual(rw['flags'], 6)
            self.assertEqual(rw['filesz'], original.ph[2]['filesz'])
            self.assertEqual(rw['memsz'], report['writable_bss']['memsz'])
            self.assertGreater(rw['memsz'], original.ph[2]['memsz'])
            self.assertLessEqual(rw['va']+rw['memsz'], image.ph[11]['va'])
            pool = int(report['payload']['symbols']['controller_pool'],16)
            self.assertGreaterEqual(pool,int(report['writable_bss']['va'],16))
            self.assertLess(pool,rw['va']+rw['memsz'])
            self.assertEqual(image.ph[11]['va'], int(report['payload']['va'], 16))
            self.assertGreaterEqual(image.ph[11]['va'], max(p['va'] + p['memsz']
                for p in original.ph if p['type'] in (1, 0x61000010)))
            # Include the PS4 RELRO segment, whose original vtables must survive.
            for segment in original.mapped():
                restored = bytearray(image.read(segment['va'], segment['filesz']))
                for patch in report['patches']:
                    relative = int(patch['va'], 16) - segment['va']
                    if 0 <= relative < len(restored):
                        restored[relative:relative + 5] = bytes.fromhex(patch['before'])
                self.assertEqual(restored, original.read(segment['va'], segment['filesz']))
            saved = json.loads((Path(folder) / 'input-eboot-build.json').read_text())
            self.assertEqual(saved['candidate_sha256'], hashlib.sha256(image.data).hexdigest())


if __name__ == '__main__':
    unittest.main()
