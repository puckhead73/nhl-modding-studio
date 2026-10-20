"""Action-service readers for primary BoolAsset and its +64 Boolean interface."""
from input_producer_hooks import REGISTERS

SITES = ((0x8a7e40,0),(0x8a7e90,64))
EXPECTED = bytes.fromhex('554889e553')


def assembly(helper='nhl24_input_read_asset_guest', base_symbol=None, fallback=None):
    lines = ['.intel_syntax noprefix','.text']
    def saved(reg):
        return (len(REGISTERS)-1-REGISTERS.index(reg))*8
    def restore():
        return ([f'vmovdqu ymm{i}, [rsp+{32*i}]' for i in range(16)]
                + ['mov rsp, [rsp+512]']+[f'pop {r}' for r in reversed(REGISTERS)]
                + ['popfq','lea rsp, [rsp+128]'])
    for address, adjustment in SITES:
        name = f'input_asset_read_{address:x}'
        lines += [f'.globl {name}',f'{name}:','lea rsp, [rsp-128]','pushfq']
        lines += [f'push {r}' for r in REGISTERS]
        lines += ['mov r11, rsp','and rsp, -32','sub rsp, 544','mov [rsp+512], r11']
        lines += [f'vmovdqu [rsp+{32*i}], ymm{i}' for i in range(16)]
        lines += [f'mov rdx, [r11+{saved("rsi")}]',f'mov rsi, [r11+{saved("rdi")}]']
        if adjustment:
            lines += [f'sub rsi, {adjustment}']
        if base_symbol:
            lines += [f'lea rdi, [rip+{base_symbol}]']
        else:
            lines += [f'.set {name}_site, {address}', '.byte 0x48,0x8d,0x3d',
                      f'.long {name}_site - . - 4', f'sub rdi, {address}']
        lines += ['cld',f'call {helper}','cmp eax, 255',f'ja {name}_native',
                  'mov r11, [rsp+512]',f'mov [r11+{saved("rax")}], rax']
        lines += restore()+['ret',f'{name}_native:']+restore()
        if fallback:
            lines += [f'jmp {fallback}']
        else:
            lines += ['.byte '+','.join(hex(b) for b in EXPECTED),
                      f'.set {name}_resume, {address+len(EXPECTED)}','.byte 0xe9',
                      f'.long {name}_resume - . - 4']
    return '\n'.join(lines)+'\n'
