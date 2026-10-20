"""Schedule lane analysis before native 2dffe60; reset with analysis records."""
import struct
import input_frame_hook
from input_producer_hooks import REGISTERS

UPDATE=0x2e0a7fc
UPDATE_BEFORE=bytes.fromhex('e85f56ffff')
RESET=0x2e09950
RESET_BEFORE=bytes.fromhex('554889e54157')


def assembly(base_symbol=None,original=None,reset_resume=None):
    saved_rdi=(len(REGISTERS)-1-REGISTERS.index('rdi'))*8
    arguments=(f'mov rsi,[r11+{saved_rdi}]',)
    text=input_frame_hook.assembly('nhl24_panic_lanes_guest','panic_lanes_update',
        base_symbol,original,UPDATE,0x2dffe60,arguments)
    text+=input_frame_hook.assembly('nhl24_panic_lanes_reset_guest','panic_lanes_reset',
        base_symbol,'panic_lanes_reset_replay',RESET,arguments=arguments)
    text+='panic_lanes_reset_replay:\n.byte '+','.join(str(b) for b in RESET_BEFORE)+'\n'
    if reset_resume:text+='jmp '+reset_resume+'\n'
    else:text+='.set panic_lanes_reset_end,0x2e09956\n.byte 0xe9\n.long panic_lanes_reset_end - . - 4\n'
    return text


def patches(image,symbols):
    if image.read(UPDATE-3,8)!=bytes.fromhex('4c89ffe85f56ffff'):
        raise ValueError('Lane scheduler no longer passes analysis owner')
    result=[]
    for address,before,name,opcode in ((UPDATE,UPDATE_BEFORE,'panic_lanes_update',0xe8),
                                      (RESET,RESET_BEFORE,'panic_lanes_reset',0xe9)):
        if image.read(address,len(before))!=before:raise ValueError('Lane hook bytes changed')
        replacement=bytes([opcode])+struct.pack('<i',symbols[name]-address-5)+bytes([0x90])*(len(before)-5)
        result.append((address,before,replacement))
    return result
