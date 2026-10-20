"""Find typed Boolean stores across CFG joins that the local scanner misses.

This is inventory only. A proof of pointer type does not qualify displaced code
or establish NHL27 producer semantics. Unknown indirect transfers stay explicit.
"""
import argparse
import bisect
from collections import deque
import json
from pathlib import Path
import re
import sqlite3
import struct
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_REG, X86_OP_MEM, X86_OP_IMM, X86_REG_RIP
from deke_runtime_evidence import checked_image, TARGET_HASH
from research import SelfImage
from input_producer_sites import BINDINGS, bounded_switches, record_provenance, register_family, restored_frame_tail_calls


def pointer_flow(instructions, decoder, switches, records):
    if not instructions:
        return {}
    by_address = {i.address:i for i in instructions}
    incoming = {instructions[0].address:{}}
    pending = deque(incoming)
    while pending:
        address = pending.popleft()
        ins = by_address[address]
        before = incoming[address]
        _, writes = ins.regs_access()
        killed = {register_family(decoder,r) for r in writes}
        after = {r:p for r,p in before.items() if r not in killed}
        ops = ins.operands
        if ins.mnemonic == 'mov' and len(ops) == 2 and ops[0].type == X86_OP_REG and ops[0].size == 8:
            dst,src = ops
            name = register_family(decoder,dst.reg)
            if src.type == X86_OP_REG and src.size == 8:
                pointer = before.get(register_family(decoder,src.reg))
                if pointer is not None:
                    after[name] = pointer
            elif src.type == X86_OP_MEM and src.size == 8 and not src.mem.index and src.mem.disp in BINDINGS:
                origin = records.get(address,{}).get(register_family(decoder,src.mem.base))
                if origin:
                    after[name] = (BINDINGS[src.mem.disp],frozenset([address]),origin)
        if ins.mnemonic == 'call':
            after = {r:p for r,p in after.items() if r in ('rbx','rbp','r12','r13','r14','r15')}
        successors = []
        if ins.mnemonic.startswith('j'):
            if ops[0].type == X86_OP_IMM:
                successors.append(ops[0].imm)
            elif address in switches:
                successors += switches[address]['targets']
            if ins.mnemonic != 'jmp':
                successors.append(address+ins.size)
        elif ins.mnemonic not in ('ret','retf','ud2','int3'):
            successors.append(address+ins.size)
        for successor in successors:
            if successor not in by_address:
                continue
            prior = incoming.get(successor)
            if prior is None:
                merged = dict(after)
            else:
                merged = {}
                for r,p in prior.items():
                    q = after.get(r)
                    if q is not None and p[0] == q[0]:
                        merged[r] = (p[0],p[1]|q[1],p[2]|q[2])
            if prior != merged:
                incoming[successor] = merged
                pending.append(successor)
    return incoming


def scan(raw,address,read_only=None):
    md = Cs(CS_ARCH_X86,CS_MODE_64)
    md.detail = True
    ins = list(md.disasm(raw,address))
    switches = bounded_switches(ins,read_only,md)
    records = record_provenance(ins,md,switches)
    pointers = pointer_flow(ins,md,switches,records)
    tails = restored_frame_tail_calls(ins)
    unresolved = [x.address for x in ins if x.mnemonic == 'jmp' and x.operands[0].type != X86_OP_IMM
                  and x.address not in switches and x.address not in tails]
    rows = []
    for i,x in enumerate(ins):
        if x.mnemonic != 'mov' or len(x.operands) != 2:
            continue
        dst,src = x.operands
        if dst.type != X86_OP_MEM or dst.size != 1 or dst.mem.index or dst.mem.disp:
            continue
        reg = register_family(md,dst.mem.base)
        pointer = pointers.get(x.address,{}).get(reg)
        if pointer is None:
            continue
        matching_records = [r for r,origins in records.get(x.address,{}).items() if origins == pointer[2]]
        rows.append(dict(store=x.address,bytes=x.bytes.hex(),action=pointer[0],loads=sorted(pointer[1]),
            getter_calls=sorted(pointer[2]),pointer_register=reg,record_registers=matching_records,
            value=src.imm if src.type == X86_OP_IMM else None,
            value_register=md.reg_name(src.reg) if src.type == X86_OP_REG else None,
            unresolved_indirect_jumps=unresolved,
            following=[dict(va=v.address,bytes=v.bytes.hex(),mnemonic=v.mnemonic,operands=v.op_str)
                       for v in ins[i+1:i+4]]))
    return rows


def run(target,database,inventory,output):
    image = checked_image(SelfImage,target,TARGET_HASH)
    with sqlite3.connect(database) as db:
        starts = [r[0] for r in db.execute('select start from functions order by start')]
    pattern = re.compile(b'|'.join(re.escape(struct.pack('<I',n)) for n in BINDINGS))
    parents = set()
    for segment in image.mapped():
        if segment['flags'] & 1:
            for match in pattern.finditer(image.read(segment['va'],segment['filesz'])):
                index = bisect.bisect_right(starts,segment['va']+match.start())-1
                if 0 <= index < len(starts)-1:
                    parents.add(index)
    def read_only(va,size):
        for segment in image.mapped():
            if not segment['flags'] & 2 and segment['va'] <= va and va+size <= segment['va']+segment['filesz']:
                return image.read(va,size)
        return None
    known = {r['store'] for r in json.loads(inventory.read_text())['sites']}
    rows = []
    for index in sorted(parents):
        start,end = starts[index:index+2]
        rows += [dict(function=start,new=r['store'] not in known,**r) for r in scan(image.read(start,end-start),start,read_only)]
    report = dict(target_sha256=TARGET_HASH,installed=False,sites=rows,
        caveat='Typed CFG inventory, not hook qualification or exhaustive producer parity.')
    output.write_text(json.dumps(report,indent=2))
    print(json.dumps(dict(typed_stores=len(rows),new_stores=sum(r['new'] for r in rows))))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target',type=Path,default=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))
    parser.add_argument('--database',type=Path,default=Path('local/nhl24-eboot/update/index.sqlite'))
    root = Path('local/nhl27-eboot/compatibility/deke-trial')
    parser.add_argument('--inventory',type=Path,default=root/'target-input-producer-sites.json')
    parser.add_argument('--output',type=Path,default=root/'target-input-pointer-flow.json')
    args = parser.parse_args()
    run(args.target,args.database,args.inventory,args.output)
