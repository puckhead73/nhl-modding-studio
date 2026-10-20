"""Find RIP references omitted by the legacy SSE-only index; qualify boundaries."""
import argparse
import sqlite3
import numpy as np
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM,X86_REG_RIP
from research import ElfImage,SelfImage

def scan(image,db,lower,upper):
    md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
    functions=set()
    for seg in image.mapped():
        if not seg['flags']&1:continue
        raw=image.read(seg['va'],seg['filesz'])
        for offset in range(0,len(raw)-3,1048576):
            count=min(1048576,len(raw)-3-offset)
            values=np.ndarray((count,),dtype='<i4',buffer=raw,offset=offset,strides=(1,))
            ends=values.astype(np.int64)+np.arange(count,dtype=np.int64)+seg['va']+offset+4
            for j in np.flatnonzero((ends>=lower-4)&(ends<upper)):
                site=seg['va']+offset+int(j)
                row=db.execute('select start from functions where start<=? order by start desc limit 1',(site,)).fetchone()
                if row:functions.add(row[0])
    for start in sorted(functions):
        row=db.execute('select start from functions where start>? order by start limit 1',(start,)).fetchone()
        if not row or row[0]-start>1000000:continue
        for ins in md.disasm(image.read(start,row[0]-start),start):
            for op in ins.operands:
                if op.type==X86_OP_MEM and op.mem.base==X86_REG_RIP:
                    dest=ins.address+ins.size+op.mem.disp
                    if lower<=dest<upper:
                        yield start,ins.address,ins.mnemonic,ins.op_str,dest
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('image');p.add_argument('db');p.add_argument('lower',type=lambda s:int(s,0));p.add_argument('upper',type=lambda s:int(s,0));a=p.parse_args()
    with open(a.image,'rb') as f:elf=f.read(4)==b'\x7fELF'
    for start,site,op,args,dest in scan((ElfImage if elf else SelfImage)(a.image),sqlite3.connect(a.db),a.lower,a.upper):
        print(f'{start:x} {site:x} {op} {args} -> {dest:x}')
