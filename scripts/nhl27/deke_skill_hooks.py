"""Narrow hooks after native classification and at recognized move selection."""
import struct
from input_producer_hooks import REGISTERS

HIGH_SITES = (0x2b605de, 0x2b635e2, 0x2ef7a89)
TOE_SITES = (0x2b60f93, 0x2b60fbf)
CAPTURE_SITES = (0x2b64346,)
BANK_SITE = 0x2b643bb
BANK_BYTES = bytes.fromhex('4180beb300000000')
RELEASE_SITE = 0x2b6ca8b
RELEASE_BYTES = bytes.fromhex('498b4660488b80280c0000')
TOE_CLEAR_SITE = 0x2b702d2
TOE_CLEAR_BYTES = bytes.fromhex('c70000000000')
PICKUP_SITE = 0x2c0ff6a
PICKUP_BYTES = bytes.fromhex('8b008d48fd')
PICKUP_WAIT_SITE = 0x2c101e0
PICKUP_WAIT_BYTES = bytes.fromhex('c4c15a2acf')


def save(helper, arguments, base_symbol=None):
    lines=['lea rsp, [rsp-128]', 'pushfq']
    lines += [f'push {r}' for r in REGISTERS]
    lines += ['mov r11, rsp', 'and rsp, -32', 'sub rsp, 544', 'mov [rsp+512], r11']
    lines += [f'vmovdqu [rsp+{32*i}], ymm{i}' for i in range(16)]
    lines += arguments
    if base_symbol:
        lines += [f'lea rdi, [rip+{base_symbol}]']
    else:
        lines += ['.byte 0x48,0x8d,0x3d', '.long deke_image_origin - . - 4']
    lines += ['cld', f'call {helper}']
    lines += [f'vmovdqu ymm{i}, [rsp+{32*i}]' for i in range(16)]
    lines += ['mov rsp, [rsp+512]']
    lines += [f'pop {r}' for r in reversed(REGISTERS)]
    return lines+['popfq','lea rsp, [rsp+128]']


def assembly(base_symbol=None, high_original=None, toe_original=None, bank_resume=None):
    lines=['.intel_syntax noprefix','.text','.set deke_image_origin, 0']
    for name, helper, original, native, toe in [
        ('deke_high_hook','deke_high_guest',high_original,0x2b611c0,False),
        ('deke_toe_hook','deke_toe_guest',toe_original,0x2b62ba0,True),
        ('deke_bank_capture_hook','deke_bank_capture_guest',toe_original,0x2b62ba0,False)]:
        lines += [f'.globl {name}',f'{name}:','sub rsp, 40',
                  'mov [rsp], rdi','mov [rsp+8], rsi','mov [rsp+16], rdx']
        if original: lines += [f'call {original}']
        else: lines += [f'.set {name}_native, {native}', '.byte 0xe8',f'.long {name}_native - . - 4']
        args=['mov rsi, [r11+256]']
        if toe: args += ['mov rdx, [r11+264]','mov rcx, [r11+272]']
        lines += save(helper,args,base_symbol)+['add rsp, 40','ret']
    lines += ['.globl deke_bank_hook','deke_bank_hook:']
    r14=(len(REGISTERS)-1-REGISTERS.index('r14'))*8
    lines += save('deke_bank_guest',[f'mov rsi, [r11+{r14}]'],base_symbol)
    lines += ['.byte '+','.join(hex(b) for b in BANK_BYTES)]
    if bank_resume: lines += [f'jmp {bank_resume}']
    else: lines += [f'.set deke_bank_resume, {BANK_SITE+len(BANK_BYTES)}',
                    '.byte 0xe9','.long deke_bank_resume - . - 4']
    lines += ['.globl deke_bank_release_hook','deke_bank_release_hook:']
    saved=lambda r:(len(REGISTERS)-1-REGISTERS.index(r))*8
    lines += save('deke_bank_release_guest',[
        f'mov rsi, [r11+{saved("r12")}]',
        f'mov rdx, [r11+{saved("rbp")}]','sub rdx, 0x120'],base_symbol)
    lines += ['.byte '+','.join(hex(b) for b in RELEASE_BYTES)]
    if bank_resume: lines += [f'jmp {bank_resume}']
    else: lines += [f'.set deke_release_resume, {RELEASE_SITE+len(RELEASE_BYTES)}',
                   '.byte 0xe9','.long deke_release_resume - . - 4']
    lines += ['.globl deke_toe_clear_hook','deke_toe_clear_hook:',
              '.byte '+','.join(hex(b) for b in TOE_CLEAR_BYTES)]
    lines += save('deke_toe_clear_guest',[f'mov rsi, [r11+{saved("r15")}]'],base_symbol)
    if bank_resume:lines += [f'jmp {bank_resume}']
    else:lines += [f'.set deke_toe_clear_resume, {TOE_CLEAR_SITE+len(TOE_CLEAR_BYTES)}',
                  '.byte 0xe9','.long deke_toe_clear_resume - . - 4']
    lines += ['.globl deke_bank_pickup_hook','deke_bank_pickup_hook:',
              'mov eax, [rax]']
    lines += save('deke_bank_pickup_guest',[
        f'mov rsi, [r11+{saved("rbx")}]',
        f'lea rdx, [r11+{saved("rax")}]'],base_symbol)
    lines += ['lea ecx, [rax-3]']
    if bank_resume:lines += [f'jmp {bank_resume}']
    else:lines += [f'.set deke_pickup_resume, {PICKUP_SITE+len(PICKUP_BYTES)}',
                  '.byte 0xe9','.long deke_pickup_resume - . - 4']
    lines += ['.globl deke_pickup_wait_hook','deke_pickup_wait_hook:']
    lines += save('deke_pickup_wait_guest',[
        f'mov rsi, [r11+{saved("rbp")}]','mov rsi, [rsi-0x128]',
        f'lea rdx, [r11+{saved("r15")}]'],base_symbol)
    lines += ['.byte '+','.join(hex(b) for b in PICKUP_WAIT_BYTES)]
    if bank_resume:lines += [f'jmp {bank_resume}']
    else:lines += [f'.set deke_pickup_wait_resume, {PICKUP_WAIT_SITE+len(PICKUP_WAIT_BYTES)}',
                  '.byte 0xe9','.long deke_pickup_wait_resume - . - 4']
    return '\n'.join(lines)+'\n'


def patches(image,symbols):
    rows=[]
    if image.read(PICKUP_WAIT_SITE,len(PICKUP_WAIT_BYTES))!=PICKUP_WAIT_BYTES:
        raise ValueError('Native pickup wait comparison changed')
    rows.append((PICKUP_WAIT_SITE,PICKUP_WAIT_BYTES,b'\xe9'+struct.pack('<i',
        symbols['deke_pickup_wait_hook']-PICKUP_WAIT_SITE-5)))
    if image.read(PICKUP_SITE,len(PICKUP_BYTES))!=PICKUP_BYTES:
        raise ValueError('Native bank pickup query changed')
    rows.append((PICKUP_SITE,PICKUP_BYTES,b'\xe9'+struct.pack('<i',
        symbols['deke_bank_pickup_hook']-PICKUP_SITE-5)))
    for sites,original,symbol in [(HIGH_SITES,0x2b611c0,'deke_high_hook'),
                                  (TOE_SITES,0x2b62ba0,'deke_toe_hook'),
                                  (CAPTURE_SITES,0x2b62ba0,'deke_bank_capture_hook')]:
        for va in sites:
            before=b'\xe8'+struct.pack('<i',original-va-5)
            if image.read(va,5)!=before: raise ValueError('Native move call changed')
            rows.append((va,before,b'\xe8'+struct.pack('<i',symbols[symbol]-va-5)))
    if image.read(BANK_SITE,len(BANK_BYTES))!=BANK_BYTES: raise ValueError('Native bank join changed')
    rows.append((BANK_SITE,BANK_BYTES,b'\xe9'+struct.pack('<i',symbols['deke_bank_hook']-BANK_SITE-5)+b'\x90'*3))
    if image.read(RELEASE_SITE,len(RELEASE_BYTES))!=RELEASE_BYTES:
        raise ValueError('Native bank release join changed')
    rows.append((RELEASE_SITE,RELEASE_BYTES,b'\xe9'+struct.pack('<i',
        symbols['deke_bank_release_hook']-RELEASE_SITE-5)+b'\x90'*(len(RELEASE_BYTES)-5)))
    # This curve belongs specifically to toe-drag playback. NHL27 lowers the
    # fast endpoint from 1.10 to 1.05; retaining it changes move/skater timing.
    site=0xaa52b9c;before=struct.pack('<f',1.1);after=struct.pack('<f',1.05)
    if image.read(site,4)!=before:raise ValueError('Native toe playback endpoint changed')
    rows.append((site,before,after))
    if image.read(TOE_CLEAR_SITE,len(TOE_CLEAR_BYTES))!=TOE_CLEAR_BYTES:
        raise ValueError('Native toe request clear changed')
    rows.append((TOE_CLEAR_SITE,TOE_CLEAR_BYTES,b'\xe9'+struct.pack('<i',
        symbols['deke_toe_clear_hook']-TOE_CLEAR_SITE-5)+b'\x90'))
    return rows
