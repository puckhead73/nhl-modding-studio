"""Recover panic/defensive-skating tuning directly from the source initializer."""
import argparse
import json
from pathlib import Path
import struct
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_REG, X86_OP_IMM, X86_OP_MEM, X86_REG_RIP
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage


def extract(image):
    md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
    regs={};stack=[];rows=[]
    aliases={'edi':'rdi','esi':'rsi','edx':'rdx','ecx':'rcx','r8d':'r8','r9d':'r9'}
    def reg(op):
        name=md.reg_name(op.reg)
        return aliases.get(name,name)
    def string(address):
        return image.read(address,256).split(b'\0',1)[0].decode('ascii')
    start,end=0x4a5e3b1,0x4a5ed09
    instructions=list(md.disasm(image.read(start,end-start),start))
    if sum(i.size for i in instructions)!=end-start:raise ValueError('Initializer decode gap')
    for ins in instructions:
        ops=ins.operands
        if ins.mnemonic=='call':
            target=ops[0].imm
            if target in (0x13139e0,0x1313ae0,0x1313c00):
                kind={0x13139e0:'bool',0x1313ae0:'int',0x1313c00:'float'}[target]
                row=dict(name=string(regs['rdx']),description=string(regs['rcx']),
                    category=string(regs['rsi']),kind=kind,constructor_va=hex(ins.address),
                    object_va=hex(regs['rdi']),handle_va=hex(regs['rdi']+(0x40 if kind=='bool' else 0x48)))
                if kind=='float':
                    for key,r in [('default','xmm0'),('minimum','xmm1'),('maximum','xmm2')]:
                        bits=regs[r];row[key]=struct.unpack('<f',struct.pack('<I',bits))[0]
                        row[key+'_bits']=f'{bits:08x}'
                elif kind=='int':
                    row.update(default=regs['r8'],minimum=regs['r9'],maximum=stack[-1])
                else:row['default']=regs['r8']
                rows.append(row)
            for name in ('rdi','rsi','rdx','rcx','r8','r9','rax','xmm0','xmm1','xmm2'):
                regs.pop(name,None)
        elif ins.mnemonic=='push':
            if ops[0].type!=X86_OP_IMM:raise ValueError('Unknown stack initializer')
            stack.append(ops[0].imm)
        elif ins.mnemonic=='add' and ins.op_str=='rsp, 0x10':stack=stack[:-2]
        elif ins.mnemonic in ('xor','vxorps') and ops[0].type==X86_OP_REG:
            if any(o.type!=X86_OP_REG or o.reg!=ops[0].reg for o in ops):raise ValueError('Unknown xor')
            regs[reg(ops[0])]=0
        elif ins.mnemonic in ('mov','lea','vmovss') and ops[0].type==X86_OP_REG:
            dest=reg(ops[0]);src=ops[1]
            if src.type==X86_OP_REG:
                if reg(src) in regs:regs[dest]=regs[reg(src)]
                else:regs.pop(dest,None)
            elif src.type==X86_OP_IMM:regs[dest]=src.imm
            elif src.type==X86_OP_MEM and src.mem.base==X86_REG_RIP:
                address=ins.address+ins.size+src.mem.disp
                regs[dest]=address if ins.mnemonic=='lea' else struct.unpack('<I',image.read(address,4))[0]
            else:raise ValueError(f'Unknown initializer operand at {ins.address:x}')
        else:raise ValueError(f'Unsupported initializer instruction {ins.address:x}: {ins.mnemonic} {ins.op_str}')
    if len(rows)!=37:raise ValueError(f'Expected 37 tuning registrations, found {len(rows)}')
    return rows


def run(source,output):
    image=checked_image(ElfImage,source,SOURCE_HASH)
    rows=extract(image)
    output.write_text(json.dumps(dict(source_sha256=SOURCE_HASH,initializer='0x4a573a0',
        registration_window=['0x4a5e3b1','0x4a5ed09'],values=rows),indent=2)+'\n')
    for row in rows:print(row['handle_va'],row['name'],row['default'])


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source',type=Path,default=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'))
    p.add_argument('--output',type=Path,default=Path('local/nhl27-eboot/compatibility/deke-trial/panic-tuning.json'))
    a=p.parse_args();run(a.source,a.output)
