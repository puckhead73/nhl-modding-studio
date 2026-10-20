"""Post-sample controller boundaries: native loop has six unrolled slots/team."""
import struct
from input_producer_hooks import REGISTERS

ORIGINAL = 0x5003d60
SITES = ((0x5000bf4, 0), (0x5000c98, 1), (0x5000d3e, 2),
         (0x5000de4, 3), (0x5000e8a, 4), (0x5000b23, 5))


def expected(address):
    return b'\xe8' + struct.pack('<i', ORIGINAL-address-5)


def assembly(helper='nhl24_input_human_frame_guest', base_symbol=None, original=None):
    lines = ['.intel_syntax noprefix', '.text']
    for address, slot in SITES:
        name = f'input_human_sample_{address:x}'
        lines += [f'.globl {name}', f'{name}:', 'push rdi']
        # Keep the original raw-controller argument across the original call.
        # Entry RSP is 8 mod 16; this push provides the required call alignment.
        if original:
            lines += [f'call {original}']
        else:
            lines += [f'.set {name}_original, {ORIGINAL}', '.byte 0xe8',
                      f'.long {name}_original - . - 4']
        lines += ['lea rsp, [rsp-128]', 'pushfq']
        lines += [f'push {r}' for r in REGISTERS]
        lines += ['mov r11, rsp', 'and rsp, -32', 'sub rsp, 544', 'mov [rsp+512], r11']
        lines += [f'vmovdqu [rsp+{32*i}], ymm{i}' for i in range(16)]
        # [rbp-0x70].bit0 is first-team loop flag, preserved even when the last
        # unrolled arm reuses [rbp-0x60] to spill the raw-controller pointer.
        lines += ['movzx esi, byte ptr [rbp-0x70]', 'and esi, 1', 'xor esi, 1',
                  'imul esi, esi, 6', f'add esi, {slot}', 'mov rdx, [r11+256]']
        if base_symbol:
            lines += [f'lea rdi, [rip+{base_symbol}]']
        else:
            lines += [f'.set {name}_site, {address}', '.byte 0x48,0x8d,0x3d',
                      f'.long {name}_site - . - 4', f'sub rdi, {address}']
        lines += ['cld', f'call {helper}']
        lines += [f'vmovdqu ymm{i}, [rsp+{32*i}]' for i in range(16)]
        lines += ['mov rsp, [rsp+512]']
        lines += [f'pop {r}' for r in reversed(REGISTERS)]
        lines += ['popfq', 'lea rsp, [rsp+136]', 'ret']
    return '\n'.join(lines) + '\n'
