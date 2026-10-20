"""Recover literal target registration names without executing target instructions.

Conservative basic-block constant propagation: unknown writes/calls/branches
discard register facts. Only unambiguous RIP-relative stores whose name pointer
and following hash match the recovered target descriptors become candidates.
Usage: python scripts/nhl27/target_field_names.py COMPATIBILITY_ROOT
"""
import collections
import json
import sqlite3
import struct
import sys
from pathlib import Path
from capstone import Cs,CS_ARCH_X86,CS_MODE_64,CS_GRP_JUMP,CS_GRP_CALL,CS_GRP_RET
from capstone.x86 import X86_OP_IMM,X86_OP_MEM,X86_OP_REG,X86_REG_RIP
from research import SelfImage


def propagate(image,start,size):
    md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True;regs={};stores=collections.defaultdict(set)
    aliases={}
    for root,parts in {'rax':['eax','ax','al'],'rbx':['ebx','bx','bl'],'rcx':['ecx','cx','cl'],'rdx':['edx','dx','dl'],'rsi':['esi','si','sil'],'rdi':['edi','di','dil'],'rbp':['ebp','bp','bpl'],'rsp':['esp','sp','spl']}.items():
        for p in parts:aliases[p]=root
    for i in range(8,16):
        for suffix in ['d','w','b']:aliases[f'r{i}{suffix}']=f'r{i}'
    def key(reg):
        n=md.reg_name(reg);return aliases.get(n,n)
    for ins in md.disasm(image.read(start,size),start):
        ops=ins.operands;handled=False
        if ins.mnemonic in ('mov','movabs','lea','vmovups','vmovaps','movups','movaps','movdqa','vmovdqa') and len(ops)==2:
            dst,src=ops;value=None
            if src.type==X86_OP_IMM:value=src.imm&((1<<(dst.size*8))-1)
            elif src.type==X86_OP_REG:value=regs.get(key(src.reg))
            elif src.type==X86_OP_MEM and src.mem.base==X86_REG_RIP:
                address=ins.address+ins.size+src.mem.disp
                if ins.mnemonic=='lea':value=address
                else:
                    try:value=int.from_bytes(image.read(address,src.size),'little')
                    except ValueError:pass
            if dst.type==X86_OP_REG:
                regs.pop(key(dst.reg),None)
                if value is not None and dst.size>=4:regs[key(dst.reg)]=value&((1<<(dst.size*8))-1)
            elif dst.type==X86_OP_MEM and dst.mem.base==X86_REG_RIP and value is not None:
                at=ins.address+ins.size+dst.mem.disp
                raw=(value&((1<<(dst.size*8))-1)).to_bytes(dst.size,'little')
                for j,v in enumerate(raw):stores[at+j].add(v)
            handled=True
        if ins.group(CS_GRP_JUMP) or ins.group(CS_GRP_RET):regs.clear()
        elif ins.group(CS_GRP_CALL):
            for k in ['rax','rcx','rdx','rsi','rdi','r8','r9','r10','r11']+[f'xmm{i}' for i in range(16)]:regs.pop(k,None)
        elif not handled:
            for reg in ins.regs_access()[1]:regs.pop(key(reg),None)
    return {a:next(iter(v)) for a,v in stores.items() if len(v)==1}


def run(root):
    schemas=json.loads((root/'target-types-base.json').read_text())+json.loads((root/'target-types-patch.json').read_text())
    wanted={int(x['name_hash'],16) for x in schemas}|{int(f['hash'],16) for x in schemas for f in x['fields']}
    paths={'nhl24_base':('base',r'F:\Games\NHL 24\game\eboot.bin',[0x639edb0,0xd56aa0]),
           'nhl24_160':('update',r'F:\Games\NHL 24\game-UPDATE\eboot.bin',[0x63cd310,0xd441b0])}
    result=[]
    for label,(directory,path,starts) in paths.items():
        im=SelfImage(path);db=sqlite3.connect(f'file:local/nhl24-eboot/{directory}/index.sqlite?mode=ro',uri=True);strings=dict(db.execute('SELECT va,text FROM strings'))
        for start in starts:
            end=db.execute('SELECT min(start) FROM functions WHERE start>?',(start,)).fetchone()[0]
            memory=propagate(im,start,end-start)
            for addr in sorted(memory):
                if addr%8:continue
                if not all(addr+j in memory for j in range(12)):continue
                raw=bytes(memory[addr+j] for j in range(12));name=strings.get(int.from_bytes(raw[:8],'little'));h=int.from_bytes(raw[8:],'little')
                if name is not None and h in wanted:result.append(dict(build=label,function=hex(start),slot=hex(addr),hash=hex(h),name=name))
    (root/'target-field-name-candidates.json').write_text(json.dumps(result,indent=2))
    print('literal registration candidates',len(result),'unique hashes',len({x['hash'] for x in result}))


if __name__=='__main__':run(Path(sys.argv[1]))
