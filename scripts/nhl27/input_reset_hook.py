"""Full input-service reset before the target's controller initialization loop."""
import input_frame_hook

ADDRESS=0x2a3366e
EXPECTED=bytes.fromhex('48c7c3c0f4ffff')
NAME='input_full_service_reset'


def assembly(helper='nhl24_input_reset_all_guest',base_symbol=None,fixture_resume=None):
    replay=NAME+'_replay'
    text=input_frame_hook.assembly(helper,NAME,base_symbol,replay,ADDRESS)
    text+=replay+':\n.byte '+','.join(hex(b) for b in EXPECTED)+'\n'
    if fixture_resume:
        text+=f'jmp {fixture_resume}\n'
    else:
        text+=f'.set {NAME}_resume, {ADDRESS+len(EXPECTED)}\n.byte 0xe9\n.long {NAME}_resume - . - 4\n'
    return text
