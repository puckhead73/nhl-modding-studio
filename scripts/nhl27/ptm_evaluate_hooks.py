"""Guarded source-transition dispatch and vector view in the native update."""
import struct
import input_frame_hook
from ptm_lifecycle_hooks import _argument

EVALUATE=0x31166f
OFFSETS=0x3125ce
SITES=((EVALUATE,'498b9c2430170000'),(OFFSETS,'498b842430170000'))


def guard(name,fallback,require_transition=True):
    # RAX and stack are restored on both exits; following native instructions
    # overwrite flags before their next use. Do not borrow a live register.
    code=f'''{name}:
push rax
movabs rax, 0x37544d5054524f50
cmp [r12+0x1740], rax
pop rax
jne {fallback}
cmp byte ptr [r12+0x1751], 1
jne {fallback}
'''
    if require_transition:code+=f'cmp qword ptr [r12+0x1748], 0\nje {fallback}\n'
    return code


def jump(name,address):
    return f'.set {name}, {address}\n.byte 0xe9\n.long {name} - . - 4\n'


def assembly(base_symbol=None):
    code='.intel_syntax noprefix\n.text\n.globl ptm_evaluate_dispatch\n'
    code+=guard('ptm_evaluate_dispatch','ptm_evaluate_fallback',False)
    code+='jmp ptm_evaluate_call\n'
    # The frame wrapper saves 128 bytes of red zone, flags and 15 GPRs:
    # original RSP = saved-register pointer R11 + 256.
    code+=input_frame_hook.assembly('nhl24_ptm_match_evaluate_frame_guest',
        'ptm_evaluate_call',base_symbol=base_symbol,original='ptm_evaluate_done',address=EVALUATE,
        arguments=(*_argument('r12'),'lea rdx, [r11+256]'))
    # Both native blend branches and the no-transition path reload this live
    # vector before the subsequent motion-warp operation.
    code+='ptm_evaluate_done:\nvmovaps xmm1, [rsp+0xb0]\n'+jump('ptm_evaluate_resume',0x3125ba)
    code+='ptm_evaluate_fallback:\n.byte '+','.join(str(b) for b in bytes.fromhex(SITES[0][1]))+'\n'
    code+=jump('ptm_evaluate_original',EVALUATE+8)
    code+='.globl ptm_offsets_dispatch\n'+guard('ptm_offsets_dispatch','ptm_offsets_fallback')
    # This block replaces the native offset-view block, using the same scratch
    # registers and preserving XMM0. Count comparison intentionally uses ESI.
    code+='''mov rax, [r12+0x1748]
mov rsi, [rax+0x48]
sub rsi, [rax+0x40]
mov rdx, [rax+0x30]
xor r11d, r11d
lea rcx, [rax+0x40]
shr rsi, 4
cmp [rdx+0x200], esi
cmove r11, rcx
'''
    code+=jump('ptm_offsets_resume',0x312601)
    code+='ptm_offsets_fallback:\n.byte '+','.join(str(b) for b in bytes.fromhex(SITES[1][1]))+'\n'
    code+=jump('ptm_offsets_original',OFFSETS+8)
    return code


def patches(image,symbols):
    rows=[]
    for (address,raw),name in zip(SITES,('ptm_evaluate_dispatch','ptm_offsets_dispatch')):
        before=bytes.fromhex(raw)
        if image.read(address,len(before))!=before:raise ValueError(f'PTM update site changed: {address:x}')
        after=b'\xe9'+struct.pack('<i',symbols[name]-address-5)+b'\x90'*(len(before)-5)
        rows.append((address,before,after))
    return rows
