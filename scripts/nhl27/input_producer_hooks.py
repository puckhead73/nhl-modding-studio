"""Generate register-transparent hooks for qualified native producer loads.

Only a direct getter-proven, nullable constant byte store is accepted. This
module does not install hooks or claim that producer scheduling is complete.
"""
REGISTERS = ('rax', 'rcx', 'rdx', 'rbx', 'rbp', 'rsi', 'rdi',
             'r8', 'r9', 'r10', 'r11', 'r12', 'r13', 'r14', 'r15')


def selected_sites(rows):
    result = []
    for row in rows:
        if row.get('load_hook_qualified'):
            result.append(dict(row, hook_address=row['load'], hook_bytes=row['load_bytes']))
        elif row.get('guarded_hook_qualified'):
            result.append(dict(row, hook_address=row['load'], hook_bytes=row['load_bytes'],
                               runtime_class_guard=True))
        elif row.get('predicate_hook_qualified'):
            result.append(dict(row, hook_address=row['load'], hook_bytes=row['load_bytes'],
                               runtime_class_guard=True, producer_mode='predicate_observation'))
        elif row.get('store_hook_qualified'):
            result.append(dict(row, hook_address=row['store'], hook_bytes=row['store_hook_bytes'],
                               producer_mode='predicated_store'))
    ordered = sorted(result, key=lambda s: s['hook_address'])
    for first, second in zip(ordered, ordered[1:]):
        if first['hook_address'] + len(bytes.fromhex(first['hook_bytes'])) > second['hook_address']:
            raise ValueError('Overlapping producer hook spans')
    return result


def trampoline(site, name=None, resume=None, base_symbol=None):
    store_hook = site.get('producer_mode') == 'predicated_store' and site.get('store_hook_qualified')
    guarded = site.get('runtime_class_guard') and (site.get('guarded_hook_qualified') or site.get('predicate_hook_qualified'))
    if not (store_hook or guarded) and (not site.get('load_hook_qualified') or site.get('hook_rejections')):
        raise ValueError('Producer hook is not qualified')
    record = site['record_register']
    if record not in REGISTERS or site['value'] not in (0, 1):
        raise ValueError('Unsupported producer register or value')
    displaced = bytes.fromhex(site.get('hook_bytes', site['load_bytes']))
    if not 5 <= len(displaced) <= 15:
        raise ValueError('Invalid displaced load length')
    address = site.get('hook_address', site['load'])
    name = name or f"producer_{address:x}"
    resume = resume or f"0x{site.get('resume_address', address + len(displaced)):x}"
    # The original function may use the SysV red zone. Preserve it before any
    # push, preserve flags before alignment, and dynamically align the call.
    lines = ['.intel_syntax noprefix', '.text', f'.globl {name}', f'{name}:',
             'lea rsp, [rsp-128]', 'pushfq']
    if site.get('predicate_kind') == 'entry_zf_clear':
        # MOV does not change flags. Observe only the path selected by the
        # original JE immediately following that MOV, without moving its store.
        lines.insert(4, f'je {name}_original')
    lines += [f'push {r}' for r in REGISTERS]
    lines += ['mov r11, rsp', 'and rsp, -16', 'sub rsp, 272',
              'mov [rsp+256], r11']
    lines += [f'movdqu [rsp+{16*i}], xmm{i}' for i in range(16)]
    record_offset = (len(REGISTERS) - 1 - REGISTERS.index(record)) * 8
    mode = site.get('producer_mode', 'nullable_constant')
    if mode not in ('nullable_constant', 'changed_constant', 'predicated_store', 'predicate_observation'):
        raise ValueError('Unsupported producer store predicate')
    helper = ('nhl24_input_observe_change_guest' if mode == 'changed_constant'
              else 'nhl24_input_produce_guest')
    lines += [f'mov rdi, [r11+{record_offset}]', f"mov esi, {site['action']}",
              f"mov edx, {site['value']}"]
    if guarded:
        helper = 'nhl24_input_guarded_produce_guest'
        if base_symbol:
            lines += [f'lea rcx, [rip+{base_symbol}]']
        else:
            lines += [f'.set {name}_site, {address}', '.byte 0x48,0x8d,0x0d',
                      f'.long {name}_site - . - 4', f'sub rcx, {address}']
        lines += [f'mov r8d, {int(mode == "changed_constant")}']
        kind = site.get('predicate_kind')
        if kind in ('clear_byte_human_actor', 'byte_or_stack_flag'):
            helper = 'nhl24_input_observe_predicate_guest'
            frame_offset = (len(REGISTERS)-1-REGISTERS.index('rbp'))*8
            lines += [f'mov r8, [r11+{frame_offset}]']
            lines += (['mov r8, [r8-136]', 'mov r9d, 1'] if kind == 'clear_byte_human_actor'
                      else ['mov r8d, [r8-160]', 'mov r9d, 2'])
    lines += ['cld', f'call {helper}']
    lines += [f'movdqu xmm{i}, [rsp+{16*i}]' for i in range(16)]
    lines += ['mov rsp, [rsp+256]']
    lines += [f'pop {r}' for r in reversed(REGISTERS)]
    replay = bytes.fromhex(site['replay_bytes']) if 'replay_bytes' in site else displaced
    lines += ['popfq', 'lea rsp, [rsp+128]', f'{name}_original:',
              '.byte ' + ','.join(hex(b) for b in replay)]
    if 'null_branch_address' in site:
        # The qualified shared-target site displaces TEST RAX,RAX; JE. The short
        # JE cannot be copied verbatim after relocating the block.
        target = site['null_branch_address']
        if isinstance(target, int):
            lines += [f'.set {name}_null, {target}', '.byte 0x0f,0x84',
                      f'.long {name}_null - . - 4']
        else:
            lines += [f'je {target}']
    # A symbol with an absolute value forces a PC-relative link relocation;
    # the relocated image and trampoline therefore share the same slide.
    if resume.startswith('0x'):
        lines += [f'.set {name}_resume, {resume}', '.byte 0xe9',
                  f'.long {name}_resume - . - 4']
    else:
        lines += [f'jmp {resume}']
    return '\n'.join(lines) + '\n'


def assembly(sites):
    qualified = selected_sites(sites)
    if len({s['hook_address'] for s in qualified}) != len(qualified):
        raise ValueError('Duplicate producer hook address')
    return '\n'.join(trampoline(s) for s in qualified)
