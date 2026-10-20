"""Paired PTMBlend storage hooks, with behavioral activation kept separate.

36bdf0 owns both direct 36bf60 constructor calls (36be8c/36bebc).
Both virtual destructor implementations inline release and need independent hooks.
"""
import struct
from input_producer_hooks import REGISTERS
import input_frame_hook

ALLOCATION=0x36be40
SITES=(
    (0x36c06e,'49c787c002000000000000','nhl24_ptm_blend_initialize_guest',('r15','rbx')),
    (0x36c0c3,'41c787d00200000000803f','nhl24_ptm_blend_finish_clock_guest',('r15',)),
    (0x3693be,'488bbfc0020000','nhl24_ptm_blend_destroy_guest',('rdi',)),
    (0x36942e,'488bbfc0020000','nhl24_ptm_blend_destroy_guest',('rdi',)),
)

def assembly():
    chunks=[]
    for address,raw,helper,registers in SITES:
        name=f'ptm_blend_lifecycle_{address:x}'
        args=tuple(f'mov {dest}, [r11+{(len(REGISTERS)-1-REGISTERS.index(src))*8}]'
                   for dest,src in zip(('rsi','rdx'),registers))
        chunks.append(input_frame_hook.assembly(helper,name,address=address,
            original=name+'_replay',arguments=args))
        chunks.append(f'{name}_replay:\n.byte '+','.join(str(b) for b in bytes.fromhex(raw))+
            f'\n.set {name}_resume, {address+len(bytes.fromhex(raw))}\n.byte 0xe9\n.long {name}_resume - . - 4\n')
    return '\n'.join(chunks)

def patches(image,symbols):
    rows=[(ALLOCATION,bytes.fromhex('bed8020000'),bytes.fromhex('be30030000'))]
    for address,raw,_,_ in SITES:
        before=bytes.fromhex(raw)
        after=b'\xe9'+struct.pack('<i',symbols[f'ptm_blend_lifecycle_{address:x}']-address-5)
        rows.append((address,before,after+b'\x90'*(len(before)-5)))
    for address,before,_ in rows:
        if image.read(address,len(before))!=before:
            raise ValueError(f'PTMBlend lifecycle site changed: {address:x}')
    return rows
