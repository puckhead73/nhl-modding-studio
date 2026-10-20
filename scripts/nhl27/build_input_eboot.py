"""Build a private input-allocation EBOOT candidate; never install or launch it.

This installs only the paired allocation/publication changes into a derived ELF.
It does not enable the deke graph, input producers, panic logic, or PTM changes.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

from build_input_bridge import build
from deke_runtime_evidence import checked_image, TARGET_HASH
from research import SelfImage, ElfImage
from input_producer_hooks import assembly, selected_sites
import input_asset_hook
import input_frame_hook
import input_human_hook
import input_owner_hooks
import input_reader_hooks
import input_join_hooks
import input_release_hooks
import input_reset_hook
import panic_hooks
import panic_lane_hooks
import input_glide_hooks
import ptm_lifecycle_hooks
import ptm_blend_lifecycle_hooks
import ptm_velocity_hooks
import ptm_blend_update_hooks
import ptm_evaluate_hooks


def align(value, alignment=0x4000):
    return (value + alignment - 1) & -alignment


def linked_payload(path):
    """Require read-only executable payload plus one zero-filled RW BSS area."""
    data = path.read_bytes()
    header = struct.unpack_from('<16sHHIQQQIHHHHHH', data)
    if header[0][:6] != b'\x7fELF\x02\x01' or header[11] != 64:
        raise ValueError('Unexpected linker output')
    sections = [struct.unpack_from('<IIQQQQIIQQ', data, header[6] + 64 * i)
                for i in range(header[12])]
    allocated = [s for s in sections if s[2] & 2 and s[5]]
    # LLD may retain SHF_MERGE/SHF_STRINGS from read-only constant pools.
    # Neither permits writable payload data; the only RW region is BSS.
    code = [s for s in allocated if s[1] == 1 and s[2] in (6, 22, 54)]
    bss = [s for s in allocated if s[1] == 8 and s[2] == 3]
    if len(code) != 1 or len(bss) != 1 or len(allocated) != 2:
        raise ValueError('Payload has unexpected allocated sections or writable data')
    symbols = {}
    for section in sections:
        if section[1] != 2:
            continue
        strings = sections[section[6]]
        names = data[strings[4]:strings[4] + strings[5]]
        if section[9] != 24:
            raise ValueError('Unexpected symbol table layout')
        for offset in range(section[4], section[4] + section[5], 24):
            name, info, other, index, value, size = struct.unpack_from('<IBBHQQ', data, offset)
            text = names[name:names.index(0, name)].decode()
            if text and index == 0:
                raise ValueError(f'Unresolved payload symbol: {text}')
            if text:
                symbols[text] = value
    section = code[0]
    return section[3], data[section[4]:section[4] + section[5]], symbols, (bss[0][3], bss[0][5])


def reconstruct(image):
    """Preserve all supplied payloads, plus the exact adjacent terminal metadata.

    This exact target SELF has a terminal metadata range without a payload
    descriptor. Its size and physical adjacency are checked, not zero-filled.
    """
    output = bytearray(max(p['offset'] + p['filesz'] for p in image.ph))
    output[:len(image.elf_header)] = image.elf_header
    ranges = []
    for segment in image.segments:
        start, size = segment['offset'], segment['filesz']
        output[start:start + size] = image.data[segment['self_offset']:segment['self_offset'] + size]
        ranges.append((start, start + size))
    tail = image.ph[10]
    previous = next(s for s in image.segments if s['index'] == 9)
    physical = previous['self_offset'] + previous['filesz']
    if (tail['type'] != 0x6fffff01 or tail['offset'] != previous['offset'] + previous['filesz']
            or physical + tail['filesz'] != len(image.data)):
        raise ValueError('Unqualified terminal SELF metadata layout')
    output[tail['offset']:tail['offset'] + tail['filesz']] = image.data[physical:]
    ranges.append((tail['offset'], tail['offset'] + tail['filesz']))
    for program in image.ph:
        start, size = program['offset'], program['filesz']
        if size and not any(a <= start and start + size <= b for a, b in ranges):
            raise ValueError('Unbacked program header range')
    return output


def run(target, output, compiler, producer_sites=None):
    image = checked_image(SelfImage, target, TARGET_HASH)
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    build(output, compiler)
    sites = []
    objects = [str(output / name) for name in ('input_bridge.o', 'asset_binding.o', 'panic_state.o', 'panic_services.o', 'panic_geometry.o', 'panic_closest_native.o', 'panic_math.o', 'panic_update.o', 'panic_lanes.o', 'panic_prediction_native.o', 'panic_lifecycle.o', 'ptm_transition.o', 'ptm_native.o', 'ptm_trajectory.o', 'ptm_offsets.o', 'ptm_integration.o', 'ptm_lifecycle.o', 'ptm_blend_timing.o', 'ptm_feature_pose.o', 'ptm_blend_snapshot.o', 'ptm_blend_snapshot_native.o', 'ptm_velocity_output.o', 'ptm_history_pose.o', 'ptm_previous_pose.o', 'ptm_velocity_query.o', 'ptm_velocity_native.o', 'ptm_query_scope.o', 'ptm_blend_query.o', 'ptm_blend_frame.o', 'ptm_blend_warp.o', 'ptm_blend_storage.o')]
    if producer_sites is not None:
        inventory = json.loads(producer_sites.read_text())
        if inventory['target_sha256'] != TARGET_HASH:
            raise ValueError('Producer inventory targets a different executable')
        sites = selected_sites(inventory['sites'] + input_join_hooks.qualified(image))
        if not sites:
            raise ValueError('No qualified producer hooks')
        source = output / 'input-producer-hooks.s'
        source.write_text(assembly(sites) + input_asset_hook.assembly() + input_frame_hook.assembly()
                          + input_human_hook.assembly() + input_owner_hooks.assembly()
                          + input_reader_hooks.assembly() + input_release_hooks.assembly()
                          + input_reset_hook.assembly() + input_glide_hooks.assembly() + ptm_lifecycle_hooks.assembly() + ptm_blend_lifecycle_hooks.assembly()
                          + ptm_evaluate_hooks.assembly() + ptm_velocity_hooks.assembly() + ptm_blend_update_hooks.assembly()
                          + panic_lane_hooks.assembly())
        obj = output / 'input-producer-hooks.o'
        subprocess.run([str(compiler), '--target=x86_64-unknown-freebsd', '-c',
                        str(source), '-o', str(obj)], check=True, capture_output=True)
        objects.append(str(obj))
    # Place code after the full mapped BSS/RELRO extents, using loader alignment.
    address = max(p['va'] + align(p['memsz'], p['align'] or 1)
                  for p in image.ph if p['type'] in (1, 0x61000010))
    address = align(address)
    # Extend the terminal native RW mapping with BSS; do not make code writable.
    rw = image.ph[2]
    if rw['type'] != 1 or rw['flags'] != 6 or align(rw['va']+rw['memsz']) != address:
        raise ValueError('Unqualified terminal writable segment')
    bss_address = address
    bss_capacity = 0x10000
    address += bss_capacity
    script = output / 'input-payload.ld'
    script.write_text('SECTIONS { .port_bss 0x%x (NOLOAD) : { *(.bss*) } '
        '.port 0x%x : { *(.text*) *(.rodata*) } '
        '/DISCARD/ : { *(.eh_frame*) *(.comment) *(.note*) } }\n' % (bss_address, address))
    linked = output / 'input-payload.elf'
    command = [str(compiler.parent / 'ld.lld.exe'), '-m', 'elf_x86_64', '--no-undefined',
               '-T', str(script), '-o', str(linked)] + objects
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)
    payload_address, payload, symbols, bss = linked_payload(linked)
    if payload_address != address:
        raise ValueError('Unexpected payload address')
    if bss[0] != bss_address or not 0 < bss[1] <= bss_capacity:
        raise ValueError('Writable payload exceeds reserved BSS')
    hook = symbols['nhl24_input_publish_extended']
    if not address <= hook < address + len(payload):
        raise ValueError('Hook outside executable payload')
    original = reconstruct(image)
    candidate = bytearray(original)
    patches = [
        (0x3ffc58, bytes.fromhex('be80150000'), b'\xbe' + struct.pack('<I', 0x21a0)),
        (0x3ffc9e, bytes.fromhex('e8ed750000'), b'\xe8' + struct.pack('<i', hook - 0x3ffca3)),
    ]
    if sites:
        va = input_asset_hook.ADDRESS
        expected = input_asset_hook.EXPECTED
        destination = symbols[input_asset_hook.NAME]
        patches.append((va, expected, b'\xe9' + struct.pack('<i', destination-va-5)
                        + b'\x90' * (len(expected)-5)))
        va = input_frame_hook.ADDRESS
        patches.append((va, input_frame_hook.EXPECTED,
                        b'\xe8' + struct.pack('<i', symbols[input_frame_hook.NAME]-va-5)))
        for va, slot in input_human_hook.SITES:
            patches.append((va, input_human_hook.expected(va),
                b'\xe8' + struct.pack('<i', symbols[f'input_human_sample_{va:x}']-va-5)))
        for va, raw, actor, index, first in input_owner_hooks.SITES:
            expected = bytes.fromhex(raw)
            patches.append((va, expected, b'\xe9' + struct.pack('<i', symbols[f'input_owner_{va:x}']-va-5)
                            + b'\x90'*(len(expected)-5)))
        for va, adjustment in input_reader_hooks.SITES:
            patches.append((va, input_reader_hooks.EXPECTED,
                b'\xe9'+struct.pack('<i',symbols[f'input_asset_read_{va:x}']-va-5)))
        for va in input_release_hooks.SITES:
            patches.append((va, input_release_hooks.expected(va),
                b'\xe8'+struct.pack('<i',symbols[f'input_release_{va:x}']-va-5)))
        va=input_reset_hook.ADDRESS
        patches.append((va,input_reset_hook.EXPECTED,
            b'\xe9'+struct.pack('<i',symbols[input_reset_hook.NAME]-va-5)
            + b'\x90'*(len(input_reset_hook.EXPECTED)-5)))
    for site in sites:
        va = site['hook_address']
        expected = bytes.fromhex(site['hook_bytes'])
        destination = symbols[f'producer_{va:x}']
        patches.append((va, expected, b'\xe9' + struct.pack('<i', destination - va - 5)
                        + b'\x90' * (len(expected) - 5)))
    if sites:
        patches.extend(panic_hooks.patches(image))
        patches.extend(panic_lane_hooks.patches(image,symbols))
        patches.extend(input_glide_hooks.patches(image, symbols))
        patches.extend(ptm_lifecycle_hooks.patches(image, symbols))
        patches.extend(ptm_blend_lifecycle_hooks.patches(image, symbols))
        patches.extend(ptm_velocity_hooks.patches(image, symbols))
        patches.extend(ptm_blend_update_hooks.patches(image, symbols))
        patches.extend(ptm_evaluate_hooks.patches(image, symbols))
    receipt_patches = []
    for va, expected, replacement in patches:
        if image.read(va, len(expected)) != expected:
            raise ValueError('Unqualified target hook bytes')
        segment = next(s for s in image.segments if s['va'] <= va < s['va'] + s['filesz'])
        offset = segment['offset'] + va - segment['va']
        if candidate[offset:offset + len(expected)] != expected:
            raise ValueError('Reconstruction differs at hook site')
        candidate[offset:offset + len(expected)] = replacement
        receipt_patches.append(dict(va=hex(va), file_offset=offset,
                                    before=expected.hex(), after=replacement.hex()))
    # Reuse only the verified empty PT_NOTE; no original metadata is discarded.
    empty = image.ph[11]
    if empty['type'] != 4 or empty['filesz'] or empty['memsz'] or empty['offset'] or empty['va']:
        raise ValueError('No qualified empty program header')
    offset = align(len(candidate))
    candidate += bytes(offset - len(candidate)) + payload
    struct.pack_into('<IIQQQQQQ', candidate, image.header[5] + 11 * 56,
                     1, 5, offset, address, address, len(payload), len(payload), 0x4000)
    struct.pack_into('<Q', candidate, image.header[5] + 2*56 + 40,
                     bss_address + bss[1] - rw['va'])
    stem = 'input-producer' if sites else 'input-allocation'
    path = output / f'eboot-{stem}-candidate.elf'
    path.write_bytes(candidate)
    reread = ElfImage(path)
    if reread.read(address, len(payload)) != payload:
        raise ValueError('Payload round trip failed')
    # Verify every original payload byte, permitting only the two declared patches.
    restored = bytearray(candidate[:len(original)])
    restored[image.header[5] + 11 * 56:image.header[5] + 12 * 56] = original[image.header[5] + 11 * 56:image.header[5] + 12 * 56]
    restored[image.header[5] + 2*56:image.header[5] + 3*56] = original[image.header[5] + 2*56:image.header[5] + 3*56]
    for patch in receipt_patches:
        at = patch['file_offset']
        expected = bytes.fromhex(patch['before'])
        restored[at:at + len(expected)] = expected
    if restored != original:
        raise ValueError('Undeclared modification to original image')
    report = dict(status=stem.replace('-', '_') + '_candidate_only_not_full_port', installed=False,
        source_sha256=TARGET_HASH, candidate_sha256=hashlib.sha256(candidate).hexdigest(),
        path=str(path), patches=receipt_patches,
        payload=dict(va=hex(address), size=len(payload), symbols={k: hex(v) for k, v in symbols.items()}),
        allocation=dict(before=0x1580, after=0x21a0, action_table_offset=0x1580, controller_selector_offset=0x2190),
        writable_bss=dict(va=hex(bss_address), size=bss[1], program_header=2,
            original_memsz=rw['memsz'], memsz=bss_address+bss[1]-rw['va'],
            controller_ownership_hooks_included=bool(sites), ownership_coverage_complete=False),
        producer_hooks=dict(count=len(sites), actions=sorted({s['action'] for s in sites}),
                            complete=False, frame_scheduling_integrated=False),
        asset_writer_hook=dict(included=bool(sites), actions=[139, 144, 172, 218, 219, 221, 223, 224]
                               if sites else [], site=hex(input_asset_hook.ADDRESS)),
        glide_packed_decoder=dict(included=bool(sites),
            sites=[hex(va) for va in input_glide_hooks.SITES] if sites else [],
            source_function='0x45e8c20', exhaustive_packed_values=65536,
            context_selection_parity=False),
        local_actor_snapshot_hook=dict(included=bool(sites), site=hex(input_frame_hook.ADDRESS),
            human_pool_scheduling_included=bool(sites), source_pool_ownership_parity=False),
        human_sample_hooks=dict(included=bool(sites), count=len(input_human_hook.SITES) if sites else 0,
            raw_action_producers_included=False, actor_assignment_hooks_included=bool(sites)),
        action_asset_readers=dict(included=bool(sites), primary=True, boolean_interface_offset=64,
            unrelated_asset_fallback=True, missing_feature_default=0),
        controller_release_hooks=dict(included=bool(sites), count=len(input_release_hooks.SITES) if sites else 0,
            source_functions=['0x46d66c0','0x46d1060','0x46d4b30'],
            target_functions=['0x2a33e60','0x2a2a1b0','0x2a33660'], all_release_paths_complete=False),
        full_input_reset_hook=dict(included=bool(sites),site=hex(input_reset_hook.ADDRESS),
            source_function='0x46d4c10',target_function='0x2a33660',
            pool_reset_before_actor_resets=True),
        validation='original payload round trip and declared-patch-only comparison passed',
        unresolved=['input producer/snapshot hooks', 'action asset integration',
                    'panic runtime', 'PTM semantics', 'full graph package', 'runtime loading validation'])
    receipt = 'input-producer-eboot-build.json' if sites else 'input-eboot-build.json'
    (output / receipt).write_text(json.dumps(report, indent=2))
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', type=Path, default=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))
    parser.add_argument('--output', type=Path, default=Path('local/nhl27-eboot/compatibility/deke-trial/runtime-build'))
    parser.add_argument('--compiler', type=Path, default=Path(r'C:\Program Files\LLVM\bin\clang.exe'))
    parser.add_argument('--producer-sites', type=Path,
                        help='Include qualified producer trampolines in a private candidate')
    args = parser.parse_args()
    result = run(args.target, args.output, args.compiler, args.producer_sites)
    print(json.dumps({k: result[k] for k in ('status', 'installed', 'path', 'candidate_sha256')}, indent=2))
