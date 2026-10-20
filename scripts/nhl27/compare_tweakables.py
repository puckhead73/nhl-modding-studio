"""Conservative NHL24 base -> NHL27 scalar registration comparison.

Only exact labels plus identical normalized instruction sequences qualify.
RIP-relative loads are resolved to data; displacement bytes are never interpreted
as float values. Output contains research candidates, not portable patches.
"""
import argparse
import collections
import hashlib
import json
import sqlite3
from pathlib import Path
from research import ElfImage, SelfImage
from port_catalog import anchors, normalized, MD
from capstone.x86 import X86_OP_MEM, X86_REG_RIP


def run(base, root24, root27):
    bi = SelfImage(base)
    if hashlib.sha256(bi.data).hexdigest() != '0118720c7e0338e20769063437be5656d1c16262c6b81a688adb659582544e17':
        raise ValueError('The supplied catalogue is bound to NHL24 base')
    ni = ElfImage(json.loads((root27/'identity.json').read_text())['path'])
    source = json.loads((root24/'catalog.json').read_text())['items']
    bc = sqlite3.connect(f'file:{(root24/"base/index.sqlite").as_posix()}?mode=ro',uri=True)
    nc = sqlite3.connect(f'file:{(root27/"index.sqlite").as_posix()}?mode=ro',uri=True)
    ba, na = anchors(bc), anchors(nc)
    rows=[]
    for ordinal,t in enumerate(source):
        r={'ordinal':ordinal,'name':t['name'],'how':t['how'],'status':'unresolved'};rows.append(r)
        if t['how']=='array': r['status']='curve_requires_separate_mapping';continue
        if t.get('at') is None: r['status']='missing_source_site';continue
        at=t['at'];orig=bytes.fromhex(t['orig'])
        if bi.read(at,len(orig))!=orig: r['status']='source_bytes_mismatch';continue
        if not na.get(t['name']): r['status']='label_no_candidate_reference';continue
        matches={}
        for anchor in ba.get(t['name'],[]):
            if not 0<=at-anchor<220:continue
            ins=list(MD.disasm(bi.read(anchor,300),anchor))
            n=next((i for i,x in enumerate(ins) if x.address<=at<x.address+x.size),None)
            if n is None:continue
            span=ins[:min(n+5,len(ins))];sig=b''.join(normalized(x) for x in span)
            size=sum(x.size for x in span)
            for cand in na[t['name']]:
                other=list(MD.disasm(ni.read(cand,size),cand))
                if b''.join(normalized(x) for x in other)!=sig:continue
                op=other[n];oldop=ins[n]
                hit={'source_va':hex(at),'target_va':hex(cand+at-anchor),
                     'instruction_va':hex(op.address),'mnemonic':op.mnemonic,'operands':op.op_str,
                     'normalized_bytes':size,'signature_sha256':hashlib.sha256(sig).hexdigest()}
                if t['how'].startswith('load'):
                    width=int(t['how'][4:])
                    mem=next((o for o in op.operands if o.type==X86_OP_MEM and o.mem.base==X86_REG_RIP),None)
                    oldmem=next((o for o in oldop.operands if o.type==X86_OP_MEM and o.mem.base==X86_REG_RIP),None)
                    if mem and oldmem:
                        target=op.address+op.size+mem.mem.disp
                        oldtarget=oldop.address+oldop.size+oldmem.mem.disp
                        hit.update(source_value_bytes=bi.read(oldtarget,width).hex(),
                                   target_value_bytes=ni.read(target,width).hex(),target_value_va=hex(target))
                else:
                    hit.update(source_value_bytes=orig.hex(),target_value_bytes=ni.read(cand+at-anchor,len(orig)).hex())
                hit['same_value_bytes']=hit.get('source_value_bytes')==hit.get('target_value_bytes') if 'source_value_bytes' in hit else None
                matches[hit['target_va']]=hit
        r['matches']=list(matches.values())
        r['status']='unique_static_match' if len(matches)==1 else 'ambiguous' if matches else 'no_normalized_match'
    summary=dict(collections.Counter(r['status'] for r in rows))
    result={'source_sha256':hashlib.sha256(bi.data).hexdigest(),'target_sha256':hashlib.sha256(ni.data).hexdigest(),
            'counts':summary,'items':rows,'qualification':'registration correspondence only; effects not established'}
    (root27/'tweakable-comparison.json').write_text(json.dumps(result,indent=2))
    print(json.dumps(summary,indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('base',type=Path);p.add_argument('root24',type=Path);p.add_argument('root27',type=Path)
    a=p.parse_args();run(a.base,a.root24,a.root27)
