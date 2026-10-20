"""Qualified controller assignment and explicit player-unassignment boundaries."""
from input_producer_hooks import REGISTERS

# (address, exact displaced bytes, actor register, index expression, replay first)
SITES = (
    (0x2a2e3d9, '418b472083f801', 'r15', 'movsx edx, byte ptr [rbx+4]', False),
    (0x2a2cc6e, 'c6833c020000ff', 'rbx', 'mov edx, -1', True),
    # Team-capacity release loops: source 46d8ad0 resets at 46d8c00/46d8cfe;
    # source 46d8fa0 resets at 46d909b/46d918d. Reset before the native signed
    # controller-index gate, including actors whose index is already negative.
    # Replay the index load after the helper; do not alter native ownership here.
    (0x2a2b997, '410fb684243c020000', 'r12', 'mov edx, -1', False),
    (0x2a2bd47, '410fb684243c020000', 'r12', 'mov edx, -1', False),
    (0x2a2c424, '410fb684243c020000', 'r12', 'mov edx, -1', False),
    (0x2a2c7a3, '410fb684243c020000', 'r12', 'mov edx, -1', False),
)


def assembly(helper='nhl24_input_assign_actor_guest', base_symbol=None, fixture_resume=None):
    lines = ['.intel_syntax noprefix', '.text']
    for address, raw, actor, index, first in SITES:
        name = f'input_owner_{address:x}'
        displaced = '.byte '+','.join(hex(b) for b in bytes.fromhex(raw))
        lines += [f'.globl {name}', f'{name}:']
        if first:
            lines += [displaced]
        lines += ['lea rsp, [rsp-128]', 'pushfq']
        lines += [f'push {r}' for r in REGISTERS]
        lines += ['mov r11, rsp', 'and rsp, -32', 'sub rsp, 544', 'mov [rsp+512], r11']
        lines += [f'vmovdqu [rsp+{32*i}], ymm{i}' for i in range(16)]
        lines += [f'mov rsi, {actor}', index]
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
        if not first:
            lines += [displaced]
        if fixture_resume:
            lines += [f'jmp {fixture_resume}']
        else:
            lines += [f'.set {name}_resume, {address+len(bytes.fromhex(raw))}', '.byte 0xe9',
                      f'.long {name}_resume - . - 4']
    return '\n'.join(lines)+'\n'
