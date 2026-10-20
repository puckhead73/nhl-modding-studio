"""Qualify three CFG-joined producers against the exact target and typed flow."""
from input_pointer_flow import scan
from input_producer_hooks import trampoline
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_IMM


def reject_interior_entries(instructions, start, size):
    """A multi-instruction detour must not erase another branch's entry."""
    for ins in instructions:
        if (ins.mnemonic.startswith('j') or ins.mnemonic == 'call') and ins.operands:
            operand = ins.operands[0]
            if operand.type == X86_OP_IMM and start < operand.imm < start + size:
                raise ValueError(f'Interior branch entry {operand.imm:#x} from {ins.address:#x}')


def qualified(image):
    def read_only(va,size):
        for p in image.mapped():
            if not p['flags'] & 2 and p['va'] <= va and va+size <= p['va']+p['filesz']:
                return image.read(va,size)
        return None
    rows = {}
    instructions = []
    decoder = Cs(CS_ARCH_X86, CS_MODE_64)
    decoder.detail = True
    for start,end in ((0x2f0d990,0x2f0dde0),(0x314eb70,0x3156330)):
        raw = image.read(start,end-start)
        instructions.extend(decoder.disasm(raw,start))
        rows.update({r['store']:r for r in scan(raw,start,read_only)})
    specifications = [
        dict(store=0x2f0ddcb,load=0x2f0dd08,load_bytes='488b80c8120000',
             record_register='rax',pointer_register='rax',action=221,value=0,
             load_hook_qualified=True,hook_rejections=[],
             sequence=(0x2f0dd08,'488b80c81200004885c00f85b3000000e956feffff')),
        dict(store=0x3154a30,load=0x3154a30,load_bytes='c60001e90c060000',
             store_hook_bytes='c60001e90c060000',store_hook_qualified=True,
             record_register='r14',pointer_register='rax',action=139,value=1,
             replay_bytes='c60001',resume_address=0x3155044,
             sequence=(0x3154a27,'4885c00f8414060000c60001e90c060000')),
        dict(store=0x315503c,flow_store=0x3155041,load=0x315503c,load_bytes='4885c07403',
             store_hook_bytes='4885c07403',store_hook_qualified=True,
             record_register='r14',pointer_register='rax',action=139,value=0,
             replay_bytes='4885c0',null_branch_address=0x3155044,
             sequence=(0x315503c,'4885c07403c600004084f67459')),
    ]
    result = []
    for spec in specifications:
        proof = rows.get(spec.get('flow_store',spec['store']))
        if not proof or proof['action'] != spec['action'] or proof['value'] != spec['value'] or proof['unresolved_indirect_jumps']:
            raise ValueError('Joined producer lacks typed CFG evidence')
        if spec['record_register'] not in proof['record_registers'] and spec['load'] not in proof['loads']:
            raise ValueError('Joined producer lost its binding record')
        va,raw = spec.pop('sequence')
        if image.read(va,len(bytes.fromhex(raw))).hex() != raw:
            raise ValueError('Joined producer predicate bytes changed')
        reject_interior_entries(instructions, spec['load'], len(bytes.fromhex(spec['load_bytes'])))
        spec['binding_offset'] = 0x12c8 if spec['action'] == 221 else 0x720
        result.append(spec)
    return result


def assembly(sites):
    from input_producer_hooks import selected_sites
    return '\n'.join(trampoline(s) for s in selected_sites(sites))
