"""Local actor frame boundary, separate from the source human input callback."""
from input_producer_hooks import REGISTERS

ADDRESS = 0x2a949a7
EXPECTED = bytes.fromhex('e8a4901200')
ORIGINAL = 0x2bbda50
NAME = 'input_local_frame_boundary'


def assembly(helper='nhl24_input_local_frame_guest', name=NAME, base_symbol=None, original=None,
             address=ADDRESS, original_address=ORIGINAL, arguments=()):
    lines = ['.intel_syntax noprefix', '.text', f'.globl {name}', f'{name}:',
             'lea rsp, [rsp-128]', 'pushfq']
    lines += [f'push {r}' for r in REGISTERS]
    lines += ['mov r11, rsp', 'and rsp, -32', 'sub rsp, 544', 'mov [rsp+512], r11']
    lines += [f'vmovdqu [rsp+{32*i}], ymm{i}' for i in range(16)]
    lines += list(arguments)
    if base_symbol:
        lines += [f'lea rdi, [rip+{base_symbol}]']
    else:
        lines += [f'.set {name}_site, {address}', '.byte 0x48,0x8d,0x3d',
                  f'.long {name}_site - . - 4', f'sub rdi, {address}']
    lines += ['cld', f'call {helper}']
    lines += [f'vmovdqu ymm{i}, [rsp+{32*i}]' for i in range(16)]
    lines += ['mov rsp, [rsp+512]']
    lines += [f'pop {r}' for r in reversed(REGISTERS)]
    lines += ['popfq', 'lea rsp, [rsp+128]']
    # Tail dispatch preserves the ORIGINAL return address of the patched CALL.
    if original:
        lines += [f'jmp {original}']
    else:
        lines += [f'.set {name}_original, {original_address}', '.byte 0xe9',
                  f'.long {name}_original - . - 4']
    return '\n'.join(lines) + '\n'
