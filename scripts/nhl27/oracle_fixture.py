"""Relocate bounded source routines into private differential-test fixtures."""
from capstone import Cs, CS_ARCH_X86, CS_MODE_64, CS_AC_WRITE
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_REG_RIP


def assembly(image,name,start,end,externals,globals_,indirect_calls=None,indirect_tail_jumps=None,labels=None):
    decoder=Cs(CS_ARCH_X86,CS_MODE_64);decoder.detail=True
    instructions=list(decoder.disasm(image.read(start,end-start),start))
    if sum(i.size for i in instructions)!=end-start:raise ValueError('Incomplete oracle decode')
    starts={i.address for i in instructions}
    labels=labels or {}
    if not set(labels)<=starts:raise ValueError('Oracle label is not an instruction boundary')
    lines=['.text',f'.globl {name}',name+':'];constants={}
    indirect_calls=indirect_calls or {};seen_indirect=set()
    indirect_tail_jumps=indirect_tail_jumps or {};seen_tail_jumps=set()
    def emit(raw):
        if raw:lines.append('.byte '+','.join(str(v) for v in raw))
    for ins in instructions:
        if ins.address in labels:lines.append(labels[ins.address]+':')
        fix=None
        if ins.mnemonic=='call' or ins.mnemonic.startswith('j'):
            if ins.operands[0].type!=X86_OP_IMM:
                if ins.mnemonic=='jmp' and indirect_tail_jumps.get(ins.address)==ins.bytes.hex():
                    seen_tail_jumps.add(ins.address)
                elif ins.mnemonic=='call' and indirect_calls.get(ins.address)==ins.bytes.hex():
                    seen_indirect.add(ins.address)
                else:
                    raise ValueError('Unqualified indirect oracle branch')
            else:
                address=ins.operands[0].imm
                if address not in starts:
                    if ins.imm_size!=4:raise ValueError('Short external branch')
                    fix=(ins.imm_offset,externals[address])
        for op in ins.operands:
            if op.type==X86_OP_MEM and op.mem.base==X86_REG_RIP:
                if fix:raise ValueError('Overlapping oracle relocation')
                address=ins.address+ins.size+op.mem.disp
                if address in globals_:symbol=globals_[address]
                else:
                    segment=next((s for s in image.segments if s['va']<=address and
                        address+op.size<=s['va']+s['filesz']),None)
                    if not segment or segment['flags']&2 or op.access&CS_AC_WRITE or ins.mnemonic=='lea':
                        raise ValueError(f'Unqualified oracle global {address:x}')
                    symbol=f'{name}_constant_{address:x}_{op.size}'
                    constants[symbol]=image.read(address,op.size)
                fix=(ins.disp_offset,symbol)
        if fix:
            offset,symbol=fix;emit(ins.bytes[:offset])
            lines.append(f'.long {symbol} - . - {ins.size-offset}');emit(ins.bytes[offset+4:])
        else:emit(ins.bytes)
    if seen_indirect!=set(indirect_calls):raise ValueError('Unused indirect-call qualification')
    if seen_tail_jumps!=set(indirect_tail_jumps):raise ValueError('Unused indirect-tail-jump qualification')
    for symbol,data in constants.items():lines.extend(['.balign 16',symbol+':']);emit(data)
    return '\n'.join(lines)+'\n'
