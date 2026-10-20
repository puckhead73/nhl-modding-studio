"""Inventory native byte stores reached from the eight target binding offsets.

Local register provenance is evidence, not proof that the base register holds a
GSBool binding. Branch joins and record origin require further qualification.
"""
import argparse
import bisect
import json
from pathlib import Path
import re
import sqlite3
import struct
from collections import deque

from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_REG, X86_OP_MEM, X86_OP_IMM, X86_REG_RIP, X86_REG_RSP, X86_REG_RBP
from deke_runtime_evidence import checked_image, TARGET_HASH
from research import SelfImage

BINDINGS = {0x12f0: 224, 0x12e8: 223, 0x12c8: 221, 0x1570: 172,
            0x1428: 144, 0x1290: 219, 0x1288: 218, 0x720: 139}

# Exact target-hash-specific predicates that cannot be reduced to a nullable
# constant store. These retain the original branch decisions and byte writes.
PREDICATES = {
    0x29cdeac: ('entry_zf_clear', '488b89c81200000f846f0100004885c97403c60101'),
    0x29cea73: ('clear_byte_human_actor', '488b80c81200004885c00f84da0200008038000f85f2020000488b8d78ffffff80b93c020000000f88cd020000c60001'),
    0x2a4162b: ('byte_or_stack_flag', '488b80c81200004885c00f84f00200008b8d60ffffff0a084c8b6d880f84160c0000c60000'),
}


def restored_frame_tail_calls(instructions):
    """Recognize an indirect ABI transfer after the complete frame epilogue.

    Such a transfer is not a switch edge into the current stack frame. Require
    the canonical frame prologue and its exact inverse, not just a nearby POP.
    """
    if len(instructions) < 4 or instructions[0].mnemonic != 'push' or instructions[0].op_str != 'rbp':
        return set()
    if instructions[1].mnemonic != 'mov' or instructions[1].op_str != 'rbp, rsp':
        return set()
    saved, cursor = ['rbp'], 2
    while cursor < len(instructions) and instructions[cursor].mnemonic == 'push':
        saved.append(instructions[cursor].op_str)
        cursor += 1
    reserve = instructions[cursor]
    if not (reserve.mnemonic == 'sub' and len(reserve.operands) == 2
            and reserve.operands[0].type == X86_OP_REG
            and reserve.operands[0].reg == X86_REG_RSP
            and reserve.operands[1].type == X86_OP_IMM):
        return set()
    result = set()
    for n, jump in enumerate(instructions):
        if jump.mnemonic != 'jmp' or jump.operands[0].type != X86_OP_REG or n < len(saved)+1:
            continue
        pops = instructions[n-len(saved):n]
        release = instructions[n-len(saved)-1]
        if (all(i.mnemonic == 'pop' and i.op_str == r for i, r in zip(pops, reversed(saved)))
                and release.mnemonic == 'add' and len(release.operands) == 2
                and release.operands[0].type == X86_OP_REG
                and release.operands[0].reg == X86_REG_RSP
                and release.operands[1].type == X86_OP_IMM
                and release.operands[1].imm == reserve.operands[1].imm
                and jump.op_str not in saved):
            result.add(jump.address)
    return result


def bounded_switches(instructions, read_only, decoder):
    """Resolve the compiler's guarded, signed-relative jump table idiom.

    Only immutable tables with an adjacent unsigned bounds check are accepted.
    Every case must land on a decoded instruction in this function. An entry
    into the middle of the guard/dispatch sequence invalidates that dispatch.
    Other indirect transfers remain unresolved and continue to gate hooks.
    """
    if read_only is None:
        return {}
    addresses = {i.address for i in instructions}
    entries = {o.imm for i in instructions if i.mnemonic.startswith('j')
               for o in i.operands if o.type == X86_OP_IMM}
    candidates = {}
    for n in range(5, len(instructions)):
        cmp, guard, lea, load, add, jump = instructions[n-5:n+1]
        if [i.mnemonic for i in (cmp, guard, lea, load, add, jump)] != [
                'cmp', 'ja', 'lea', 'movsxd', 'add', 'jmp']:
            continue
        c, g, l, m, a, j = [i.operands for i in (cmp, guard, lea, load, add, jump)]
        zero_extend = None
        if len(c) == 2 and c[0].type == X86_OP_REG and c[0].size == 4 and n >= 6:
            previous = instructions[n-6]
            p = previous.operands
            # A 32-bit bounds comparison only constrains the low half of the
            # address index. Require an adjacent unconditional 32-bit write,
            # which clears the high half, and reject entries past that write.
            if (previous.mnemonic in ('mov', 'movzx', 'movsx', 'lea', 'add', 'sub',
                                      'and', 'or', 'xor', 'inc', 'dec')
                    and p and p[0].type == X86_OP_REG and p[0].size == 4
                    and p[0].reg == c[0].reg):
                zero_extend = previous
        if not (len(c) == 2 and c[0].type == X86_OP_REG
                and (c[0].size == 8 or zero_extend is not None)
                and c[1].type == X86_OP_IMM and 0 <= c[1].imm < 1024
                and len(g) == 1 and g[0].type == X86_OP_IMM
                and g[0].imm in addresses
                and len(l) == 2 and l[0].type == X86_OP_REG and l[0].size == 8
                and register_family(decoder, l[0].reg) != register_family(decoder, c[0].reg)
                and l[1].type == X86_OP_MEM and l[1].mem.base == X86_REG_RIP
                and not l[1].mem.index
                and len(m) == 2 and m[0].type == X86_OP_REG and m[0].size == 8
                and m[1].type == X86_OP_MEM and m[1].size == 4
                and m[1].mem.base == l[0].reg
                and register_family(decoder, m[1].mem.index) == register_family(decoder, c[0].reg)
                and m[1].mem.scale == 4 and m[1].mem.disp == 0
                and m[0].reg != l[0].reg
                and len(a) == 2 and all(o.type == X86_OP_REG and o.size == 8 for o in a)
                and a[0].reg == m[0].reg and a[1].reg == l[0].reg
                and len(j) == 1 and j[0].type == X86_OP_REG and j[0].reg == m[0].reg):
            continue
        table = lea.address + lea.size + l[1].mem.disp
        count = c[1].imm + 1
        raw = read_only(table, count * 4)
        if raw is None or len(raw) != count * 4:
            continue
        targets = [table + v[0] for v in struct.iter_unpack('<i', raw)]
        if not all(t in addresses for t in targets):
            continue
        interior = {i.address for i in (guard, lea, load, add, jump)}
        if zero_extend is not None:
            interior.add(cmp.address)
        candidates[jump.address] = dict(table=table, count=count, targets=targets,
            guard=zero_extend.address if zero_extend is not None else cmp.address,
            interior=interior)
    # Include all candidate case edges when rejecting guard bypasses, even if
    # another candidate is itself rejected later. Conservative by construction.
    entries |= {t for s in candidates.values() for t in s['targets']}
    simple = {address: {k: v for k, v in row.items() if k != 'interior'}
              for address, row in candidates.items() if not entries & row['interior']}
    return extended_switches(instructions, read_only, decoder, simple)


def register_facts(instructions, decoder, switches):
    """Must-analysis for immutable table addresses and 32-bit zero extension."""
    by_address = {i.address: i for i in instructions}
    incoming = {instructions[0].address: {}}
    pending = deque(incoming)
    while pending:
        va = pending.popleft()
        ins, before = by_address[va], incoming[va]
        _, writes = ins.regs_access()
        killed = {register_family(decoder, r) for r in writes}
        after = {r: fact for r, fact in before.items() if r not in killed}
        ops = ins.operands
        if ops and ops[0].type == X86_OP_REG:
            family = register_family(decoder, ops[0].reg)
            if family in killed and ops[0].size == 4:
                after[family] = ('zero_extended', 0)
            if ins.mnemonic == 'lea' and len(ops) == 2 and ops[0].size == 8:
                if ops[1].type == X86_OP_MEM and ops[1].mem.base == X86_REG_RIP and not ops[1].mem.index:
                    after[family] = ('address', va + ins.size + ops[1].mem.disp)
            if ins.mnemonic == 'mov' and len(ops) == 2 and all(o.type == X86_OP_REG and o.size == 8 for o in ops):
                fact = before.get(register_family(decoder, ops[1].reg))
                if fact is not None:
                    after[family] = fact
        if ins.mnemonic == 'call':
            after = {r: fact for r, fact in after.items() if r in ('rbx','rbp','r12','r13','r14','r15')}
        successors = []
        if ins.mnemonic.startswith('j'):
            if ops[0].type == X86_OP_IMM:
                successors.append(ops[0].imm)
            elif va in switches:
                successors.extend(switches[va]['targets'])
            if ins.mnemonic != 'jmp':
                successors.append(va + ins.size)
        elif ins.mnemonic not in ('ret', 'ud2', 'int3'):
            successors.append(va + ins.size)
        for successor in successors:
            if successor not in by_address:
                continue
            prior = incoming.get(successor)
            merged = dict(after) if prior is None else {r: fact for r, fact in after.items() if prior.get(r) == fact}
            if prior != merged:
                incoming[successor] = merged
                pending.append(successor)
    return incoming


def extended_switches(instructions, read_only, decoder, simple):
    """Qualify separated table loads and guards using all candidate case edges.

    Structural discovery supplies a conservative CFG first. Constant table
    provenance and upper-index bits are then checked across its joins/loops.
    """
    addresses = {i.address for i in instructions}
    entries = {o.imm for i in instructions if i.mnemonic.startswith('j')
               for o in i.operands if o.type == X86_OP_IMM}
    definitions = {}
    for ins in instructions:
        o = ins.operands
        if (ins.mnemonic == 'lea' and len(o) == 2 and o[0].type == X86_OP_REG and o[0].size == 8
                and o[1].type == X86_OP_MEM and o[1].mem.base == X86_REG_RIP and not o[1].mem.index):
            definitions.setdefault(register_family(decoder, o[0].reg), set()).add(ins.address+ins.size+o[1].mem.disp)
    candidates = {}
    for n, jump in enumerate(instructions):
        if jump.address in simple or jump.mnemonic != 'jmp' or jump.operands[0].type != X86_OP_REG:
            continue
        target = jump.operands[0].reg
        for k in range(max(0, n-8), n):
            load = instructions[k]
            m = load.operands
            if not (load.mnemonic == 'movsxd' and len(m) == 2 and m[0].type == X86_OP_REG
                    and m[0].reg == target and m[0].size == 8 and m[1].type == X86_OP_MEM
                    and m[1].size == 4 and m[1].mem.scale == 4 and m[1].mem.disp == 0
                    and m[1].mem.base and m[1].mem.index and m[1].mem.base != target):
                continue
            base = register_family(decoder, m[1].mem.base)
            index = register_family(decoder, m[1].mem.index)
            target_family = register_family(decoder, target)
            if base == index:
                continue
            add_count, valid = 0, True
            for step in instructions[k+1:n]:
                ops = step.operands
                if (step.mnemonic == 'add' and len(ops) == 2
                        and all(o.type == X86_OP_REG and o.size == 8 for o in ops)
                        and ops[0].reg == target and ops[1].reg == m[1].mem.base):
                    add_count += 1
                elif (step.mnemonic not in ('mov','neg','imul','lea')
                      or {register_family(decoder, r) for r in step.regs_access()[1]} & {base,target_family}):
                    valid = False
            if not valid or add_count != 1:
                continue
            for g in range(max(0, k-8), k):
                guard = instructions[g]
                if guard.mnemonic != 'ja' or guard.operands[0].type != X86_OP_IMM or guard.operands[0].imm not in addresses:
                    continue
                c = g-1
                while c >= max(0, g-3) and instructions[c].mnemonic in ('mov','lea'):
                    c -= 1
                if c < 0:
                    continue
                compare = instructions[c]
                ops = compare.operands
                if not (compare.mnemonic == 'cmp' and len(ops) == 2 and ops[0].type == X86_OP_REG
                        and ops[0].size in (4,8) and register_family(decoder, ops[0].reg) == index
                        and ops[1].type == X86_OP_IMM and 0 <= ops[1].imm < 1024):
                    continue
                if any(step.mnemonic not in ('mov','movsxd','lea','ja') or
                       index in {register_family(decoder,r) for r in step.regs_access()[1]}
                       for step in instructions[c+1:k]):
                    continue
                count = ops[1].imm+1
                tables = []
                for table in definitions.get(base, ()):
                    raw = read_only(table, count*4)
                    if raw is None or len(raw) != count*4:
                        continue
                    targets = [table+v[0] for v in struct.iter_unpack('<i',raw)]
                    if all(t in addresses for t in targets):
                        tables.append((table,targets))
                if not tables or len(tables) > 32:
                    continue
                # Include every plausible immutable table's edges while proving
                # which address reaches the load. Never select by proximity.
                targets = sorted({t for _, destinations in tables for t in destinations})
                candidates[jump.address] = dict(tables=tables, count=count, targets=targets,
                    guard=compare.address, interior={s.address for s in instructions[c if ops[0].size == 4 else c+1:n+1]},
                    base=base, load=load.address, index=index, narrow=ops[0].size == 4)
    all_edges = dict(simple)
    all_edges.update(candidates)
    entries |= {t for row in all_edges.values() for t in row['targets']}
    # Narrow only when the conservative graph proves a unique table address.
    # Removing impossible case edges can then qualify a loop-carried table base.
    for _ in range(len(candidates)+1):
        facts = register_facts(instructions, decoder, all_edges)
        narrowed = False
        for va, row in candidates.items():
            fact = facts.get(row['load'], {}).get(row['base'])
            match = [targets for table, targets in row['tables'] if fact == ('address',table)]
            if len(match) == 1 and set(all_edges[va]['targets']) != set(match[0]):
                all_edges[va] = dict(all_edges[va], targets=match[0])
                narrowed = True
        if not narrowed:
            break
    result = dict(simple)
    for va, row in candidates.items():
        if entries & row['interior']:
            continue
        fact = facts.get(row['load'], {}).get(row['base'])
        matching = [(table, targets) for table, targets in row['tables'] if fact == ('address',table)]
        if len(matching) != 1:
            continue
        if row['narrow'] and facts.get(row['guard'], {}).get(row['index']) != ('zero_extended',0):
            continue
        table, targets = matching[0]
        result[va] = dict(table=table, targets=targets, count=row['count'], guard=row['guard'])
    return result


def record_provenance(instructions, decoder, switches=None):
    """Must-analysis: retain record types only when every reachable path agrees.

    Direct branches are followed; indirect jumps end a path. This deliberately
    cannot qualify blocks reachable only through an unresolved jump table.
    """
    switches = switches or {}
    if not instructions:
        return {}
    by_address = {i.address: i for i in instructions}
    incoming = {instructions[0].address: {}}
    pending = deque([instructions[0].address])
    while pending:
        address = pending.popleft()
        instruction = by_address[address]
        before = incoming[address]
        _, writes = instruction.regs_access()
        killed = {register_family(decoder, r) for r in writes}
        after = {r: origin for r, origin in before.items() if r not in killed}
        operands = instruction.operands
        if (instruction.mnemonic == 'mov' and len(operands) == 2
                and all(o.type == X86_OP_REG and o.size == 8 for o in operands)):
            origin = before.get(register_family(decoder, operands[1].reg))
            if origin is not None:
                after[register_family(decoder, operands[0].reg)] = origin
        if instruction.mnemonic == 'call':
            after = {r: origin for r, origin in after.items()
                     if r in ('rbx', 'rbp', 'r12', 'r13', 'r14', 'r15')}
            if len(operands) == 1 and operands[0].type == X86_OP_IMM and operands[0].imm == 0x3ffbf0:
                after['rax'] = frozenset([address])
        successors = []
        if instruction.mnemonic.startswith('j'):
            if len(operands) == 1 and operands[0].type == X86_OP_IMM:
                successors.append(operands[0].imm)
            elif address in switches:
                successors.extend(switches[address]['targets'])
            if instruction.mnemonic != 'jmp':
                successors.append(address + instruction.size)
        elif instruction.mnemonic not in ('ret', 'ud2', 'int3'):
            successors.append(address + instruction.size)
        for successor in successors:
            if successor not in by_address:
                continue
            prior = incoming.get(successor)
            merged = dict(after) if prior is None else {
                r: prior[r] | after[r] for r in prior.keys() & after.keys()}
            if prior != merged:
                incoming[successor] = merged
                pending.append(successor)
    return incoming


def register_family(decoder, register):
    name = decoder.reg_name(register)
    for family, names in {
        'rax': ('rax', 'eax', 'ax', 'al', 'ah'), 'rbx': ('rbx', 'ebx', 'bx', 'bl', 'bh'),
        'rcx': ('rcx', 'ecx', 'cx', 'cl', 'ch'), 'rdx': ('rdx', 'edx', 'dx', 'dl', 'dh'),
        'rsi': ('rsi', 'esi', 'si', 'sil'), 'rdi': ('rdi', 'edi', 'di', 'dil'),
        'rbp': ('rbp', 'ebp', 'bp', 'bpl'), 'rsp': ('rsp', 'esp', 'sp', 'spl')}.items():
        if name in names:
            return family
    return re.sub(r'^r(\d+)[dwb]$', r'r\1', name)


def scan_function(raw, address, read_only=None):
    decoder = Cs(CS_ARCH_X86, CS_MODE_64)
    decoder.detail = True
    pointers, rows, records = {}, [], {}
    instructions = list(decoder.disasm(raw, address))
    switches = bounded_switches(instructions, read_only, decoder)
    tail_calls = restored_frame_tail_calls(instructions)
    origins = record_provenance(instructions, decoder, switches)
    indirect_jumps = [i.address for i in instructions if i.mnemonic == 'jmp'
                      and i.operands[0].type != X86_OP_IMM and i.address not in switches
                      and i.address not in tail_calls]
    by_address = {i.address: n for n, i in enumerate(instructions)}
    branch_targets = {op.imm for i in instructions if i.mnemonic.startswith('j')
                      for op in i.operands if op.type == X86_OP_IMM}
    branch_targets |= {t for s in switches.values() for t in s['targets']}
    for instruction in instructions:
        # No local definition may be carried through a control-flow join.
        if instruction.address in branch_targets:
            records.clear()
            pointers.clear()
        operands = instruction.operands
        record_copy = None
        if (instruction.mnemonic == 'mov' and len(operands) == 2
                and all(o.type == X86_OP_REG and o.size == 8 for o in operands)):
            record_copy = records.get(register_family(decoder, operands[1].reg))
        if instruction.mnemonic == 'mov' and len(operands) == 2:
            destination, source = operands
            if (destination.type == X86_OP_MEM and destination.size == 1
                    and not destination.mem.index and destination.mem.disp == 0):
                pointer = pointers.get(register_family(decoder, destination.mem.base))
                if pointer and instruction.address - pointer['load'] <= 128:
                    rows.append(dict(pointer, store=instruction.address, bytes=instruction.bytes.hex(),
                        value=source.imm if source.type == X86_OP_IMM else None,
                        value_register=decoder.reg_name(source.reg) if source.type == X86_OP_REG else None,
                        qualification='local_pointer_load_only_record_origin_and_control_flow_unqualified'))
        _, writes = instruction.regs_access()
        written = {register_family(decoder, r) for r in writes}
        old_records = dict(records)
        records = {r: origin for r, origin in records.items() if r not in written}
        pointers = {r: p for r, p in pointers.items()
                    if r not in written and p['record_register'] not in written}
        if instruction.mnemonic == 'mov' and len(operands) == 2:
            destination, source = operands
            if (destination.type == X86_OP_REG and destination.size == 8
                    and source.type == X86_OP_MEM and source.size == 8
                    and source.mem.disp in BINDINGS and not source.mem.index
                    and source.mem.base not in (0, X86_REG_RIP, X86_REG_RSP)):
                pointer_register = register_family(decoder, destination.reg)
                base_register = register_family(decoder, source.mem.base)
                pointers[pointer_register] = dict(action=BINDINGS[source.mem.disp],
                    binding_offset=source.mem.disp, record_register=base_register,
                    pointer_register=pointer_register, load=instruction.address,
                    record_destroyed_by_load=pointer_register == base_register,
                    getter_call=old_records.get(base_register),
                    cfg_getter_calls=sorted(origins.get(instruction.address, {}).get(base_register, ())),
                    load_bytes=instruction.bytes.hex())
            elif record_copy is not None:
                records[register_family(decoder, destination.reg)] = record_copy
        if instruction.mnemonic == 'call':
            records = {r: origin for r, origin in records.items()
                       if r in ('rbx', 'rbp', 'r12', 'r13', 'r14', 'r15')}
            if len(operands) == 1 and operands[0].type == X86_OP_IMM and operands[0].imm == 0x3ffbf0:
                records['rax'] = instruction.address
        if instruction.mnemonic.startswith('j') or instruction.mnemonic == 'ret':
            records.clear()
        if instruction.mnemonic in ('call', 'jmp', 'ret'):
            pointers.clear()
    for row in rows:
        # Qualify only the exact nullable store idiom. Unknown indirect edges
        # could enter the span or invalidate a previously inferred record type.
        n = by_address[row['load']]
        # The compiler may restore unrelated scalar temporaries between the
        # binding load and its null check. Accept only stack-to-register MOVs:
        # no flags, memory writes, calls, or pointer/record clobbers are allowed.
        # All entries into this extended sequence are checked below as usual.
        cursor = n + 1
        while cursor < min(n + 5, len(instructions)):
            candidate = instructions[cursor]
            ops = candidate.operands
            if not (candidate.mnemonic == 'mov' and len(ops) == 2
                    and ops[0].type == X86_OP_REG
                    and ops[1].type == X86_OP_MEM
                    and ops[1].mem.base in (X86_REG_RSP, X86_REG_RBP, X86_REG_RIP)
                    and not ops[1].mem.index
                    and register_family(decoder, ops[0].reg) not in
                    (row['pointer_register'], row['record_register'], 'rsp')):
                break
            cursor += 1
        conditional = [instructions[n]] + instructions[cursor:cursor + 5]
        changed = (len(conditional) == 6 and conditional[3].mnemonic == 'cmp'
                   and conditional[4].mnemonic == 'je'
                   and conditional[5].address == row['store'])
        span = conditional if changed else [instructions[n]] + instructions[cursor:cursor + 3]
        reasons = []
        if indirect_jumps:
            reasons.append('unresolved_indirect_control_flow')
        if not row['cfg_getter_calls']:
            reasons.append('record_origin_not_proven_on_all_direct_paths')
        if row['value'] not in (0, 1):
            reasons.append('nonconstant_boolean_store')
        exact = False
        if len(span) in (4, 6):
            load, test, jump = span[:3]
            store = span[-1]
            exact = (len(load.bytes) >= 5 and test.mnemonic == 'test'
                     and len(test.operands) == 2
                     and all(o.type == X86_OP_REG and
                             register_family(decoder, o.reg) == row['pointer_register']
                             and o.size == 8 for o in test.operands)
                     and jump.mnemonic == 'je' and jump.operands[0].type == X86_OP_IMM
                     # A null branch may use a shared continuation anywhere in
                     # the function. The helper does not touch action storage
                     # for a null backing pointer, and the non-null fallthrough
                     # still executes this store without another condition.
                     # Entries into the displaced sequence are rejected below.
                     and jump.operands[0].imm in by_address
                     and store.address == row['store'])
            if changed:
                compare, skip = span[3:5]
                ops = compare.operands
                exact = (exact and len(ops) == 2 and ops[0].type == X86_OP_MEM
                         and ops[0].size == 1 and not ops[0].mem.index
                         and ops[0].mem.disp == 0
                         and register_family(decoder, ops[0].mem.base) == row['pointer_register']
                         and ops[1].type == X86_OP_IMM and ops[1].imm == row['value']
                         and skip.operands[0].type == X86_OP_IMM
                         and skip.operands[0].imm in by_address
                         and jump.operands[0].imm == skip.operands[0].imm)
            complete_span = instructions[n + 1:by_address[store.address] + 1]
            if any(i.address in branch_targets for i in complete_span):
                reasons.append('branch_enters_displaced_store_sequence')
        if not exact:
            reasons.append('not_exact_nullable_store_sequence')
        row['hook_rejections'] = reasons
        row['load_hook_qualified'] = not reasons
        # A spilled record need not be inferred through unknown callees. The
        # exact nullable idiom can instead use a runtime class guard. Allocation
        # growth applies to every instance of this exact native binding class.
        row['guarded_hook_qualified'] = reasons == ['record_origin_not_proven_on_all_direct_paths']
        row['producer_mode'] = 'changed_constant' if changed else 'nullable_constant'
        row['unresolved_indirect_jumps'] = indirect_jumps
        row['resolved_switches'] = switches
        row['restored_frame_tail_calls'] = sorted(tail_calls)
        row['qualification'] = ('verified_getter_and_nullable_constant_store'
                                if not reasons else 'candidate_only')
        row['store_hook_qualified'] = False
        row['predicate_hook_qualified'] = False
        profile = PREDICATES.get(row['load'])
        if profile is not None and not indirect_jumps:
            kind, expected = profile
            end_index = by_address[row['store']]
            actual = b''.join(i.bytes for i in instructions[n:end_index+1]).hex()
            if actual == expected and not any(i.address in branch_targets for i in instructions[n+1:end_index+1]):
                row['predicate_hook_qualified'] = True
                row['predicate_kind'] = kind
                row['predicate_bytes'] = expected
                row['qualification'] = 'exact_native_predicate_with_runtime_class_guard'
        if reasons == ['not_exact_nullable_store_sequence'] and not row['record_destroyed_by_load']:
            store_index = by_address[row['store']]
            store = instructions[store_index]
            following = instructions[store_index + 1:store_index + 2]
            between = instructions[n + 1:store_index]
            # Place a hook AFTER all predicates have executed. Displace only
            # the byte store and one ordinary register MOV. Never relocate a
            # branch, RIP-relative access, or an independently entered suffix.
            stable = all(not (i.operands and i.operands[0].type == X86_OP_MEM
                              and i.operands[0].access & 2) for i in between)
            if following:
                move = following[0]
                ops = move.operands
                safe = (move.mnemonic == 'mov' and len(ops) == 2
                        and ops[0].type == X86_OP_REG
                        and ops[1].type in (X86_OP_REG, X86_OP_MEM, X86_OP_IMM)
                        and not (ops[1].type == X86_OP_MEM and ops[1].mem.base == X86_REG_RIP)
                        and move.address not in branch_targets
                        and 5 <= store.size + move.size <= 15)
                origin = origins.get(store.address, {}).get(row['record_register'], ())
                if stable and safe and sorted(origin) == row['cfg_getter_calls']:
                    row['store_hook_qualified'] = True
                    row['store_hook_bytes'] = (store.bytes + move.bytes).hex()
                    row['qualification'] = 'verified_predicated_store_site'
    return rows


def run(target, database, output):
    image = checked_image(SelfImage, target, TARGET_HASH)
    with sqlite3.connect(f'file:{database.resolve().as_posix()}?mode=ro', uri=True) as db:
        starts = [row[0] for row in db.execute('SELECT start FROM functions ORDER BY start')]
    parents = set()
    pattern = re.compile(b'|'.join(re.escape(struct.pack('<I', n)) for n in BINDINGS))
    for segment in image.mapped():
        if not segment['flags'] & 1:
            continue
        for match in pattern.finditer(image.read(segment['va'], segment['filesz'])):
            index = bisect.bisect_right(starts, segment['va'] + match.start()) - 1
            if 0 <= index < len(starts) - 1:
                parents.add(index)
    rows = []
    def read_only(va, size):
        for segment in image.mapped():
            if (not segment['flags'] & 2 and segment['va'] <= va
                    and va + size <= segment['va'] + segment['filesz']):
                return image.read(va, size)
        return None
    for index in sorted(parents):
        start, end = starts[index:index + 2]
        for row in scan_function(image.read(start, end - start), start, read_only):
            rows.append(dict(function=start, **row))
    report = dict(target_sha256=TARGET_HASH, installed=False, sites=rows,
                  caveat='Candidate inventory is not an exhaustive CFG or producer parity result.')
    output.write_text(json.dumps(report, indent=2))
    print(json.dumps(dict(sites=len(rows), actions=sorted({r['action'] for r in rows}), installed=False)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', type=Path, default=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))
    parser.add_argument('--database', type=Path, default=Path('local/nhl24-eboot/update/index.sqlite'))
    parser.add_argument('--output', type=Path,
        default=Path('local/nhl27-eboot/compatibility/deke-trial/target-input-producer-sites.json'))
    args = parser.parse_args()
    run(args.target, args.database, args.output)
