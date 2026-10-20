"""Relocate the source's self-contained panic direction predicate and constants.

Keep the original AVX polynomial, angle wrapping, comparisons and NaN handling.
This accepts no calls, indirect branches, writable globals or outside targets.
The private extracted bytes are generated from the user's checked source image.
"""
import hashlib
import struct
from capstone import Cs, CS_ARCH_X86, CS_MODE_64, CS_AC_WRITE
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_REG_RIP

START, SIZE = 0x4a355e0, 0x480
NAME = 'nhl27_panic_direction_test'


def relocate(image, start=START, size=SIZE, initialized_constants=None):
    # Optional constants must be proven by the caller's initializer audit.
    initialized_constants=initialized_constants or {}
    original=image.read(start,size)
    code=bytearray(original)
    decoder=Cs(CS_ARCH_X86,CS_MODE_64)
    decoder.detail=True
    instructions=list(decoder.disasm(original,start))
    if sum(i.size for i in instructions)!=size:
        raise ValueError('Incomplete source direction decoding')
    starts={i.address for i in instructions}
    refs=[]
    constants={}
    for ins in instructions:
        if ins.mnemonic.startswith('call'):
            raise ValueError('Unexpected source direction dependency')
        if ins.mnemonic.startswith('j'):
            if len(ins.operands)!=1 or ins.operands[0].type!=X86_OP_IMM or ins.operands[0].imm not in starts:
                raise ValueError('Unbounded source direction control flow')
        for op in ins.operands:
            if op.type!=X86_OP_MEM or op.mem.base!=X86_REG_RIP:
                continue
            address=ins.address+ins.size+op.mem.disp
            if ins.mnemonic=='lea' or op.access&CS_AC_WRITE or ins.disp_size!=4:
                raise ValueError('Source direction references mutable/address data')
            segment=next((s for s in image.segments if s['va']<=address and
                          address+op.size<=s['va']+s['filesz']),None)
            override=initialized_constants.get(address)
            if override is None and (segment is None or segment['flags']&2):
                raise ValueError('Source direction constant is not read-only')
            key=(address,op.size)
            if key not in constants:
                code.extend(bytes((-len(code))%16))
                constants[key]=len(code)
                if override is not None and len(override)<op.size:
                    raise ValueError('Initializer constant is shorter than source read')
                code.extend(override[:op.size] if override is not None else image.read(address,op.size))
            at=ins.address-start+ins.disp_offset
            struct.pack_into('<i',code,at,constants[key]-(ins.address-start+ins.size))
            refs.append(dict(site=at,original_address=hex(address),size=op.size,
                             destination=constants[key],instruction_end=ins.address-start+ins.size))
    if not refs and start==START:
        raise ValueError('Expected source direction constants absent')
    restored=bytearray(code[:size])
    for ref in refs:
        at=ref['site']
        restored[at:at+4]=original[at:at+4]
    if restored!=original:
        raise ValueError('Unexpected instruction change in source direction port')
    return bytes(code),dict(source_address=hex(start),source_size=size,
        original_sha256=hashlib.sha256(original).hexdigest(),
        relocated_sha256=hashlib.sha256(code).hexdigest(),constant_relocations=refs)


def assembly(data, name=NAME):
    lines=['.text',f'.globl {name}',f'{name}:']
    lines.extend('.byte '+','.join(hex(b) for b in data[i:i+24]) for i in range(0,len(data),24))
    return '\n'.join(lines)+'\n'
