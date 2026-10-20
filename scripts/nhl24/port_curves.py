"""Resolve array registration storage, without assuming curve interpolation semantics."""
import argparse
import collections
import json
import sqlite3
import struct
from pathlib import Path
from capstone.x86 import X86_OP_MEM, X86_OP_IMM, X86_OP_REG, X86_REG_RIP
from inspect_eboot import Image
from port_catalog import MD, normalized, anchors


def target(ins, operand):
    return ins.address+ins.size+operand.mem.disp


def describe(im, anchor):
    inst=list(MD.disasm(im.read(anchor,240),anchor))
    node=None;pointer=None;count=None;pointer_lea=None
    for n,i in enumerate(inst):
        if i.mnemonic=='mov' and len(i.operands)==2:
            a,b=i.operands
            if a.type==X86_OP_MEM and a.mem.base==X86_REG_RIP:
                dest=target(i,a)
                if node is None:node=dest
                if dest==node+0x38 and b.type==X86_OP_IMM:count=b.imm
                if dest==node+0x40 and b.type==X86_OP_REG:
                    # Track the most recent assignment of this register, requiring RIP LEA.
                    for previous in reversed(inst[:n]):
                        if previous.operands and previous.operands[0].type==X86_OP_REG and previous.operands[0].reg==b.reg:
                            if previous.mnemonic=='lea' and previous.operands[1].type==X86_OP_MEM and previous.operands[1].mem.base==X86_REG_RIP:
                                pointer=target(previous,previous.operands[1]);pointer_lea=previous.address
                            break
                if dest==node+0x50 and b.type==X86_OP_IMM and b.imm==0:
                    span=inst[:n+2]
                    return {'node':node,'count':count,'pointer':pointer,'pointer_lea':pointer_lea,
                            'signature':b''.join(normalized(x) for x in span),'size':sum(x.size for x in span)}
    return None


def run(work,base,update):
    bi,ui=Image(base),Image(update)
    bc,uc=(sqlite3.connect(work/v/'index.sqlite') for v in ['base','update'])
    ba,ua=anchors(bc),anchors(uc)
    source=json.loads((work/'catalog.json').read_text());rows=[]
    for t in source['items']:
        if t['how']!='array':continue
        row={'source':t,'status':'unresolved','matches':[]};rows.append(row)
        for a in ba.get(t['name'],[]):
            b=describe(bi,a)
            if not b or b['node']!=int(t['id'],16) or b['pointer']!=t['at'] or b['count']!=len(t['value']):continue
            for a2 in ua.get(t['name'],[]):
                u=describe(ui,a2)
                if not u or u['signature']!=b['signature']:continue
                try:
                    data=ui.read(u['pointer'],4*u['count']); backing='file'
                    values=list(struct.unpack('<'+'f'*u['count'],data))
                except ValueError:
                    backing='not_file_backed';data=None;values=None
                refs=[hex(v) for v, in uc.execute('select src from refs where dst=?',(u['node']+0x50,))]
                row['matches'].append({'node':hex(u['node']),'count':u['count'],'array':hex(u['pointer']),
                  'backing':backing,'values':values,'original':data.hex() if data else None,
                  'pointer_initializer':hex(u['pointer_lea']),'runtime_pointer_slot':hex(u['node']+0x50),
                  'runtime_slot_references':refs})
        row['status']='unique_storage_match' if len(row['matches'])==1 else ('ambiguous' if row['matches'] else 'unresolved')
    result={'counts':dict(collections.Counter(r['status'] for r in rows)),'items':rows}
    (work/'ported-curves.json').write_text(json.dumps(result,indent=2));print(result['counts'])


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('work',type=Path);p.add_argument('base',type=Path);p.add_argument('update',type=Path)
    a=p.parse_args();run(a.work,a.base,a.update)
