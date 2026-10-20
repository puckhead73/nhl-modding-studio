"""Rank cross-build function candidates; rankings are evidence leads, never bindings."""
import argparse
from collections import Counter
from difflib import SequenceMatcher
import json
import math
from pathlib import Path
import sqlite3
from capstone import Cs,CS_ARCH_X86,CS_MODE_64
from capstone.x86 import X86_OP_MEM,X86_OP_IMM,X86_REG_RIP
from research import ElfImage,SelfImage


def image_at(root):
    path=Path(json.loads((root/'identity.json').read_text())['path'])
    with path.open('rb') as stream:kind=stream.read(4)
    return (ElfImage if kind==b'\x7fELF' else SelfImage)(path)


def starts(root):
    with sqlite3.connect(f'file:{(root/"index.sqlite").as_posix()}?mode=ro',uri=True) as db:
        return [row[0] for row in db.execute('select start from functions order by start')]


def fingerprint(decoder,image,start,end):
    instructions=list(decoder.disasm(image.read(start,end-start),start))
    sequence=[];features=Counter()
    for ins in instructions:
        if ins.mnemonic in ('nop','int3','endbr64'):continue
        operands=tuple(op.type for op in ins.operands)
        token=(ins.mnemonic,operands);sequence.append(token);features[token]+=1
        for op in ins.operands:
            if op.type==X86_OP_MEM and op.mem.base!=X86_REG_RIP and 0<=op.mem.disp<0x10000:
                features[('memory',op.size,op.mem.disp)]+=.3
            elif op.type==X86_OP_IMM and not (ins.mnemonic.startswith('j') or ins.mnemonic=='call'):
                features[('immediate',op.imm)]+=.2
    return sequence,features


def run(source_root,target_root,address,lower,upper,limit):
    source=image_at(source_root);target=image_at(target_root)
    source_starts=starts(source_root);target_starts=starts(target_root)
    index=source_starts.index(address);end=source_starts[index+1];size=end-address
    decoder=Cs(CS_ARCH_X86,CS_MODE_64);decoder.detail=True
    wanted,features=fingerprint(decoder,source,address,end)
    norm=math.sqrt(sum(v*v for v in features.values()));ranked=[]
    for start,end in zip(target_starts,target_starts[1:]):
        if not lower<=start<upper or not size*.3<=end-start<=size*3:continue
        try:sequence,counter=fingerprint(decoder,target,start,end)
        except (ValueError,IndexError):continue
        denominator=norm*math.sqrt(sum(v*v for v in counter.values()))
        score=sum(v*counter[k] for k,v in features.items())/denominator if denominator else 0
        if score>.70:ranked.append((score,start,end,sequence))
    candidates=[]
    for cosine,start,end,sequence in sorted(ranked,reverse=True)[:100]:
        ordered=SequenceMatcher(None,wanted,sequence,autojunk=False).ratio()
        candidates.append(dict(start=hex(start),end=hex(end),cosine=cosine,ordered=ordered,
                               score=.45*cosine+.55*ordered))
    return sorted(candidates,key=lambda row:row['score'],reverse=True)[:limit]


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path);parser.add_argument('target',type=Path)
    parser.add_argument('address',type=lambda x:int(x,0))
    parser.add_argument('--lower',type=lambda x:int(x,0),default=0)
    parser.add_argument('--upper',type=lambda x:int(x,0),default=0x1000000)
    parser.add_argument('--limit',type=int,default=20)
    args=parser.parse_args();print(json.dumps(run(args.source,args.target,args.address,args.lower,args.upper,args.limit),indent=2))
