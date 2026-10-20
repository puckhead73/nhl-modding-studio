"""Observe the eight mapped native assets at the generic Boolean writer.

The hook runs after property initialization and before the final byte store.
It preserves all YMM registers because resolving a binding can call game code.
"""
from input_producer_hooks import REGISTERS

ADDRESS = 0x8a7fae
EXPECTED = bytes.fromhex('45887d004883c408')  # mov [r13],r15b; add rsp,8
NAME = 'input_asset_writer'


def assembly(helper='nhl24_input_observe_asset_guest', name=NAME, resume=None, base_symbol=None):
    lines = ['.intel_syntax noprefix', '.text', f'.globl {name}', f'{name}:',
             'lea rsp, [rsp-128]', 'pushfq']
    lines += [f'push {r}' for r in REGISTERS]
    lines += ['mov r11, rsp', 'and rsp, -32', 'sub rsp, 544', 'mov [rsp+512], r11']
    lines += [f'vmovdqu [rsp+{32*i}], ymm{i}' for i in range(16)]

    def saved(register):
        return (len(REGISTERS) - 1 - REGISTERS.index(register)) * 8

    if base_symbol:
        lines += [f'lea rdi, [rip+{base_symbol}]']
    else:
        lines += [f'.set {name}_site, {ADDRESS}', '.byte 0x48,0x8d,0x3d',
                  f'.long {name}_site - . - 4', f'sub rdi, {ADDRESS}']
    lines += [f'mov rsi, [r11+{saved("r12")}]',
              f'mov rdx, [r11+{saved("rbp")}]', 'mov rdx, [rdx-48]',
              f'mov rcx, [r11+{saved("r13")}]', f'mov r8, [r11+{saved("r15")}]',
              'cld', f'call {helper}']
    lines += [f'vmovdqu ymm{i}, [rsp+{32*i}]' for i in range(16)]
    lines += ['mov rsp, [rsp+512]']
    lines += [f'pop {r}' for r in reversed(REGISTERS)]
    lines += ['popfq', 'lea rsp, [rsp+128]', '.byte ' + ','.join(hex(b) for b in EXPECTED)]
    if resume is None:
        lines += [f'.set {name}_resume, {ADDRESS+len(EXPECTED)}', '.byte 0xe9',
                  f'.long {name}_resume - . - 4']
    else:
        lines += [f'jmp {resume}']
    return '\n'.join(lines) + '\n'
