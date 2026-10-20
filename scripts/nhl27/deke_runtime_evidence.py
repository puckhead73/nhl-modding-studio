"""Reproduce executable evidence for the unfinished deke runtime bridge.

Reads the two exact research binaries; never patches or launches a game.
Writer sites are local instruction evidence, not control-flow equivalence.
Run from the repository root: python scripts/nhl27/deke_runtime_evidence.py
"""
import argparse
import bisect
import hashlib
import json
from pathlib import Path
import sqlite3

from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_IMM, X86_REG_ESI, X86_REG_RSI, X86_REG_SI, X86_REG_SIL
from research import ElfImage, SelfImage
from trace import direct_call_candidates


SOURCE_HASH = '10c7bee4c27e5260b417f9574e65cee8338019d08635b3ab9dddab30ba524126'
TARGET_HASH = 'fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9'
ACTIONS = {224, 223, 221, 172, 144, 219, 218, 139}


def checked_image(reader, path, expected):
    image = reader(path)
    if hashlib.sha256(image.data).hexdigest() != expected:
        raise ValueError(f'Unqualified executable: {path}')
    return image


def snapshot(image, address, length):
    raw = image.read(address, length)
    decoder = Cs(CS_ARCH_X86, CS_MODE_64)
    return dict(address=hex(address), bytes=raw.hex(), instructions=[
        dict(address=hex(i.address), mnemonic=i.mnemonic, operands=i.op_str)
        for i in decoder.disasm(raw, address)])


def writer_sites(image, database):
    starts = [r[0] for r in database.execute('SELECT start FROM functions ORDER BY start')]
    parents = set()
    for segment in image.segments:
        if segment['flags'] & 1:
            for address in direct_call_candidates(image.read(segment['va'], segment['filesz']),
                                                  segment['va'], 0x3c9ae0):
                index = bisect.bisect_right(starts, address) - 1
                if index >= 0:
                    parents.add(starts[index])
    decoder = Cs(CS_ARCH_X86, CS_MODE_64)
    decoder.detail = True
    rows = []
    for start in sorted(parents):
        index = bisect.bisect_right(starts, start)
        if index == len(starts):
            continue
        action = None
        for instruction in decoder.disasm(image.read(start, starts[index] - start), start):
            # Track only a constant ESI definition within a linear basic block.
            # Calls and jumps invalidate it; this is deliberately not dataflow
            # recovery across control-flow joins or register copies.
            _, writes = instruction.regs_access()
            if set(writes) & {X86_REG_ESI, X86_REG_RSI, X86_REG_SI, X86_REG_SIL}:
                action = None
                if (X86_REG_ESI in writes and instruction.mnemonic == 'mov' and len(instruction.operands) == 2
                        and instruction.operands[1].type == X86_OP_IMM):
                    action = instruction.operands[1].imm
            if instruction.mnemonic == 'call':
                if instruction.op_str == '0x3c9ae0' and action in ACTIONS:
                    rows.append(dict(function=hex(start), call=hex(instruction.address),
                                     action=action, qualification='local_constant_argument_only'))
                action = None
            elif instruction.mnemonic.startswith('j') or instruction.mnemonic.startswith('ret'):
                action = None
    return rows


def run(root, source_path, target_path):
    source = checked_image(ElfImage, source_path, SOURCE_HASH)
    target = checked_image(SelfImage, target_path, TARGET_HASH)
    database_path = root / 'index.sqlite'
    with sqlite3.connect(f'file:{database_path.resolve().as_posix()}?mode=ro', uri=True) as db:
        slot = db.execute('SELECT addend FROM relocations WHERE site=?', (0xe4022e0,)).fetchone()
        if slot != (0x3b0490,):
            raise ValueError('Context-property vtable evidence changed')
        writers = writer_sites(source, db)
    report = dict(
        status='research_evidence_runtime_bridge_not_installed',
        source_sha256=SOURCE_HASH, target_sha256=TARGET_HASH,
        action_storage=dict(property_address='0xf2244d0', property_initializer='0x3b01c0',
            property_vtable='0xe402230', context_lookup_slot=176, context_lookup='0x3b0490',
            context_lookup_arguments=['property_RDI', 'evaluation_context_RSI'],
            fallback_pointer_offset=64, pool_allocator='0x3d8060', pool_bytes=37056,
            pool_entries=12, entry_stride=3088, boolean_entries=244, boolean_stride=8,
            analog_region_offset=1952, analog_entry_stride=28, analog_entries=40,
            analog_value_getter='0x3c96b0', current_tick_offset=3072,
            previous_boolean_offset=1, changed_tick_offset=4,
            frame_snapshot='0x3d72c0', pressed_getter='0x3d8030', released_getter='0x3d8040',
            reset='0x3d72b0', reset_body='0x3d7570', constructor='0x3d7080',
            frame_snapshot_calls=['0x48a850', '0x470de4c', '0x470debf', '0x4b485e2'],
            frame_snapshot_semantics='Set tick; copy 244 current Boolean bytes to slot+1 and 40 current analog words to analog+4 before producers.',
            pooled_input_tick_global='0x103e16e4', pooled_input_update='0x4b48580',
            inline_actor_frame_update='0x470dcf0', animation_frame_update='0x48a790',
            context_pool_index_offset=3112, inline_storage_offset=24,
            caution='The region after 244 Boolean entries is not Boolean-slot storage.'),
        panic_lifecycle=dict(source_owner_create='0x480d210', source_allocation_call='0x480d45a',
            source_component_bytes=96, source_constructor='0x4a54380',
            source_owner_component_offset=232, source_owner_destroy='0x480cea0',
            source_component_destroy='0x4a544d0', source_update_call='0x480e8bf',
            source_component_update='0x4a517d0', target_owner_assignment='0x2bef040',
            target_owner_reset='0x2bbe800', target_owner_actor_offset=144,
            target_owner_pool='0xba19b28', target_owner_pool_entries=18,
            target_owner_update='0x2bf10a0', target_owner_dispatch='0x2f0dde0',
            target_owner_update_call='0x2f0def0', target_actor_owner_offset=520,
            source_actor_owner_offset=552,
            target_candidate_insertion='0x2bf1810', target_owner_register='r15',
            target_preceding_interaction_update='0x2c45860',
            caution='NHL24 owner +0xe8 contains flags, not the NHL27 component pointer.',
            target_storage_implemented=False, target_update_hook_qualified=False),
        target_boolean_service=dict(root_wrapper_getter='0x2ec80c0',
            wrapper_root_offset=16, root_boolean_asset_offset=104,
            per_actor_getter='0x3ffbf0', evaluation_context_offset=96,
            actor_animation_offset=96, animation_binding_offset=3112,
            binding_context_offset=32,
            qualification='lookup_path_recovered_no_adapter_installed'),
        enter_deke_consumer=dict(source_action=218, source_function='0x3a13c0',
            source_read_call='0x3a1783', target_function='0x3fb1c0',
            target_binding_getter='0x3fd5f0', target_asset_reference_offset=40,
            target_binding_pointer_offset=48,
            qualification='corresponding_consumer_only_production_and_timing_unverified'),
        writer_sites=writers,
        source_snapshots=[snapshot(source, a, n) for a, n in [
            (0x3b0490, 54), (0x3d8450, 13), (0x3c9ae0, 21),
            (0x480d441, 50), (0x480d0fb, 48), (0x3a1752, 54),
            (0x3d72c0, 0x2a5), (0x3d8030, 13), (0x3d8040, 13),
            (0x3d72b0, 15), (0x3d7295, 26)]],
        target_snapshots=[snapshot(target, 0x3fd5f0, 64), snapshot(target, 0x3ffbf0, 0xaf),
                          snapshot(target, 0x4040fc, 0x3f),
                          snapshot(target, 0x2bf17ff, 0x2c),
                          snapshot(target, 0x2f0dee8, 13),
                          snapshot(target, 0x2bb544a, 0x2f)],
        decompilation_caution='Define helper functions 0x3c9ae0 and 0x3cd090 before decompiling callers. Without those boundaries Ghidra incorrectly treated some calls as PIC branches and omitted reachable writes; corrected caller outputs were regenerated.')
    output = root / 'compatibility/deke-trial/runtime-binding-evidence.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2))
    print(json.dumps(dict(output=str(output), local_writer_sites=len(writers),
                          actions_with_sites=sorted({r['action'] for r in writers}), installed=False)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('local/nhl27-eboot'))
    parser.add_argument('--source', type=Path, default=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'))
    parser.add_argument('--target', type=Path, default=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))
    args = parser.parse_args()
    run(args.root, args.source, args.target)
