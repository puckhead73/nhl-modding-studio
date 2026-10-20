"""Paired match allocation, initialization, reset and destruction hooks.

Both direct constructor callers allocate exactly 1740 bytes. No non-executable
pointer to 326f10 exists in the checked target. Append 90 bytes; never reuse the
native 1730 transition slot for a source-layout transition.
"""
import struct
from input_producer_hooks import REGISTERS
import input_frame_hook

ALLOCATIONS=(0x326ec2,0x3277e2)
CONSTRUCTORS=(0x326ee7,0x32780b)
CONSTRUCTOR=0x326f10
SITES=(
    (0x30d2ec,'4c8bb730170000','rdi','nhl24_ptm_match_destroy_guest'),
    # Source 23bb59 clears after the rig reset and before publishing pose data.
    (0x30d903,'80ba1812000000','rdx','nhl24_ptm_match_reset_guest'),
)

def _argument(register):
    offset=(len(REGISTERS)-1-REGISTERS.index(register))*8
    return (f'mov rsi, [r11+{offset}]',)

def assembly():
    chunks=[]
    for address in CONSTRUCTORS:
        chunks.append(input_frame_hook.assembly('nhl24_ptm_match_initialize_guest',
            f'ptm_construct_{address:x}',address=address,original_address=CONSTRUCTOR,arguments=_argument('rdi')))
    for address,raw,register,helper in SITES:
        name=f'ptm_lifecycle_{address:x}'
        chunks.append(input_frame_hook.assembly(helper,name,original=name+'_replay',address=address,
                                                arguments=_argument(register)))
        chunks.append(f'{name}_replay:\n.byte '+','.join(str(b) for b in bytes.fromhex(raw))+
            f'\n.set {name}_resume, {address+len(bytes.fromhex(raw))}\n.byte 0xe9\n.long {name}_resume - . - 4\n')
    return '\n'.join(chunks)

def patches(image,symbols):
    rows=[]
    for address in ALLOCATIONS:
        rows.append((address,bytes.fromhex('be40170000'),bytes.fromhex('bed0170000')))
    for address in CONSTRUCTORS:
        rows.append((address,b'\xe8'+struct.pack('<i',CONSTRUCTOR-address-5),
                     b'\xe8'+struct.pack('<i',symbols[f'ptm_construct_{address:x}']-address-5)))
    for address,raw,_,_ in SITES:
        before=bytes.fromhex(raw)
        rows.append((address,before,b'\xe9'+struct.pack('<i',symbols[f'ptm_lifecycle_{address:x}']-address-5)
                     +b'\x90'*(len(before)-5)))
    for address,before,_ in rows:
        if image.read(address,len(before))!=before:raise ValueError(f'PTM lifecycle site changed: {address:x}')
    return rows
