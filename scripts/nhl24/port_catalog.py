"""Match scalar initializer sites by exact label plus normalized instructions.

Produces static candidates, never a deployment or runtime qualification.
Usage: python scripts/nhl24/port_catalog.py WORKDIR BASE_EBOOT UPDATE_EBOOT
WORKDIR must contain catalog.json and base/update index.sqlite from the other probes.
"""
import argparse
import collections
import hashlib
import json
import sqlite3
from pathlib import Path
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_OP_IMM, X86_REG_RIP
from inspect_eboot import Image

MD = Cs(CS_ARCH_X86, CS_MODE_64)
MD.detail = True


def normalized(ins):
    b = bytearray(ins.bytes)
    if any(o.type == X86_OP_MEM and o.mem.base == X86_REG_RIP for o in ins.operands):
        b[ins.disp_offset:ins.disp_offset+ins.disp_size] = b'\x00'*ins.disp_size
    if (ins.mnemonic.startswith('j') or ins.mnemonic == 'call') and ins.operands[0].type == X86_OP_IMM:
        b[ins.imm_offset:ins.imm_offset+ins.imm_size] = b'\x00'*ins.imm_size
    return bytes(b)


def anchors(db):
    result = collections.defaultdict(list)
    for name, src in db.execute('select s.text,r.src from strings s join refs r on r.dst=s.va where r.kind="lea"'):
        result[name].append(src)
    return result


def run(work, base, update):
    bi, ui = Image(base), Image(update)
    bc, uc = (sqlite3.connect(work / v / 'index.sqlite') for v in ('base','update'))
    ba, ua = anchors(bc), anchors(uc)
    source = json.loads((work/'catalog.json').read_text())
    rows=[]
    for ordinal,t in enumerate(source['items']):
        r=dict(ordinal=ordinal, source=t, status='unresolved')
        rows.append(r)
        if t['how']=='array':
            r['status']='curve_not_ported';continue
        at=t.get('at')
        if at is None:
            r['status']='missing_site';continue
        orig=bytes.fromhex(t['orig'])
        if bi.read(at,len(orig))!=orig:
            r['status']='base_bytes_mismatch';continue
        matches=[]
        for anchor in ba.get(t['name'],[]):
            if not 0 <= at-anchor < 220:continue
            inst=list(MD.disasm(bi.read(anchor,300),anchor))
            patch=next((i for i in inst if i.address <= at < i.address+i.size),None)
            if patch is None:continue
            # A relocation operand is not a literal float. Record its source separately.
            n=next(j for j,i in enumerate(inst) if i.address==patch.address)
            span=inst[:min(n+5,len(inst))]
            signature=b''.join(normalized(i) for i in span)
            size=sum(i.size for i in span)
            for candidate in ua.get(t['name'],[]):
                other=list(MD.disasm(ui.read(candidate,size),candidate))
                if b''.join(normalized(i) for i in other)!=signature:continue
                pi=other[n]
                new_at=candidate+at-anchor
                item={'anchor':hex(candidate),'base_anchor':hex(anchor),'site':hex(new_at),
                      'xml_address':hex(new_at+0x400000),'self_offset':hex(ui.offset(new_at)),
                      'original':ui.read(new_at,len(orig)).hex(), 'instruction':hex(pi.address),
                      'mnemonic':pi.mnemonic,'operands':pi.op_str,
                      'matched_bytes':size, 'normalized_sha256':hashlib.sha256(signature).hexdigest()}
                mem=next((o for o in pi.operands if o.type==X86_OP_MEM and o.mem.base==X86_REG_RIP),None)
                if mem:
                    dst=pi.address+pi.size+mem.mem.disp
                    item['operand_target']=hex(dst)
                    if t['how'].startswith('load'):
                        length=int(t['how'][4:]);item['loaded_bytes']=ui.read(dst,length).hex()
                        bm=next(o for o in patch.operands if o.type==X86_OP_MEM)
                        bdst=patch.address+patch.size+bm.mem.disp
                        item['base_loaded_bytes']=bi.read(bdst,length).hex()
                matches.append(item)
        unique={m['site']:m for m in matches}
        r['matches']=list(unique.values())
        r['status']='unique_static_match' if len(unique)==1 else ('ambiguous' if unique else 'no_normalized_match')
    report={'base_sha256':hashlib.sha256(bi.data).hexdigest(),'update_sha256':hashlib.sha256(ui.data).hexdigest(),
            'status_counts':dict(collections.Counter(r['status'] for r in rows)),'items':rows}
    (work/'ported-catalog.json').write_text(json.dumps(report,indent=2))
    print(json.dumps(report['status_counts']))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('work',type=Path);p.add_argument('base',type=Path);p.add_argument('update',type=Path)
    a=p.parse_args();run(a.work,a.base,a.update)
