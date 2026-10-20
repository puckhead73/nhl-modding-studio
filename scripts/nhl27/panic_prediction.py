"""Extract the two source prediction loops with per-call settings, no globals.

Only the 60-position human/AI arrays are produced. The later interception
outputs in 49b8460 are outside this bounded region and remain untouched.
"""
import hashlib
from capstone import Cs, CS_ARCH_X86, CS_MODE_64, CS_AC_WRITE
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_REG_RIP
import panic_math
from oracle_fixture import assembly

START, END = 0x49b8460, 0x49b8c2f
NAME = 'nhl27_panic_prediction_arrays'
GLOBALS = {0x1048f010: 0, 0x1048ef70: 8, 0x1048efc0: 16}


def generate(image):
    md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
    raw=image.read(START,END-START)
    instructions=list(md.disasm(raw,START))
    if sum(i.size for i in instructions)!=len(raw):raise ValueError('Incomplete prediction decode')
    starts={i.address for i in instructions};constants={};seen=set()
    calls={0x463da0:'nhl27_panic_prediction_step',0x25da630:'nhl27_panic_prediction_reduce'}
    lines=['.intel_syntax noprefix','.text',f'.globl {NAME}',NAME+':',
           # Preserve stack alignment and pass a pointer bag beyond source locals.
           'push rsi','sub rsp,8']
    def label(a):return f'.Lpanic_prediction_{a:x}'
    def emit(data):
        if data:lines.append('.byte '+','.join(str(x) for x in data))
    for ins in instructions:
        lines.append(label(ins.address)+':')
        if ins.mnemonic=='call' or ins.mnemonic.startswith('j'):
            if ins.operands[0].type!=X86_OP_IMM:raise ValueError('Indirect prediction transfer')
            dest=ins.operands[0].imm
            if ins.mnemonic=='call':
                lines.append('call '+calls[dest])
            elif dest in starts or dest==END:
                lines.append(ins.mnemonic+' '+label(dest))
            else:raise ValueError('Prediction branch escapes loops')
            continue
        refs=[o for o in ins.operands if o.type==X86_OP_MEM and o.mem.base==X86_REG_RIP]
        if not refs:emit(ins.bytes);continue
        if len(refs)!=1:raise ValueError('Multiple prediction RIP operands')
        op=refs[0];address=ins.address+ins.size+op.mem.disp
        if address==0xec2d9f0:
            if ins.bytes.hex()!='488b057555270a':raise ValueError('Prediction guard changed')
            lines.append('lea rax,[rip+.Lpanic_prediction_zero]')
        elif address in GLOBALS:
            if ins.mnemonic!='mov' or ins.op_str.split(',')[0]!='rax':raise ValueError('Prediction settings load changed')
            seen.add(address)
            lines.extend(['mov rax,[rbp+16]',f'mov rax,[rax+{GLOBALS[address]}]'])
        else:
            segment=next((s for s in image.segments if s['va']<=address and address+op.size<=s['va']+s['filesz']),None)
            if not segment or segment['flags']&2 or op.access&CS_AC_WRITE or ins.mnemonic=='lea':
                raise ValueError(f'Unqualified prediction global {address:x}')
            symbol=f'.Lpanic_prediction_constant_{address:x}_{op.size}'
            constants[symbol]=image.read(address,op.size)
            emit(ins.bytes[:ins.disp_offset])
            lines.append(f'.long {symbol} - . - {ins.size-ins.disp_offset}')
            emit(ins.bytes[ins.disp_offset+4:])
    if seen!=set(GLOBALS):raise ValueError('Missing prediction settings')
    lines += [label(END)+':','add rsp,0x308','pop rbx','pop r12','pop r13',
              'pop r14','pop r15','pop rbp','add rsp,16','ret',
              '.balign 16','.Lpanic_prediction_zero:','.quad 0']
    for name,data in constants.items():lines.extend(['.balign 16',name+':']);emit(data)
    step,receipt=panic_math.relocate(image,0x463da0,0x560)
    lines.append(panic_math.assembly(step,'nhl27_panic_prediction_step'))
    lines.append(assembly(image,'nhl27_panic_prediction_reduce',0x25da630,0x25da750,{},
                          {0xdf600d0:'.Lpanic_prediction_reduce_table'}))
    # The reduction exponent selects at most words 4,5,6 of this immutable table.
    lines.extend(['.balign 16','.Lpanic_prediction_reduce_table:'])
    emit(image.read(0xdf600d0,32))
    return '\n'.join(lines)+'\n',dict(source_start=hex(START),source_end=hex(END),
        source_sha256=hashlib.sha256(raw).hexdigest(),step=receipt,
        settings_offsets={hex(a):v for a,v in GLOBALS.items()},
        output_offsets=['0x1700','0x1ac0'],samples=60)
