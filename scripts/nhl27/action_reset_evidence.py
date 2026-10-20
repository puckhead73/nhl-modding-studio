"""Interpret the constant-only source action reset stores for an independent oracle.

Supports only the six observed instructions in source 0x3d7570. It is deliberately
not a general x86 emulator and rejects other code or memory access patterns.
"""
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_IMM, X86_OP_REG, X86_OP_MEM, X86_REG_RIP, X86_REG_RDI


def reset_template(image):
    decoder = Cs(CS_ARCH_X86, CS_MODE_64)
    decoder.detail = True
    registers = {}
    output, coverage = bytearray(0xc00), bytearray(0xc00)
    end = 0x3d7570
    for instruction in decoder.disasm(image.read(0x3d7570, 0x98e), 0x3d7570):
        if instruction.address != end:
            raise ValueError('Instruction decoding gap')
        end += instruction.size
        if instruction.mnemonic == 'ret':
            if end != 0x3d7efe or not all(coverage):
                raise ValueError('Incomplete reset or uncovered storage')
            return bytes(output)
        if instruction.mnemonic not in ('vbroadcastsd', 'vmovups', 'vmovsd', 'movabs', 'mov'):
            raise ValueError('Unsupported reset instruction')
        destination, source = instruction.operands
        if source.type == X86_OP_IMM:
            data = (source.imm & ((1 << (8 * destination.size)) - 1)).to_bytes(destination.size, 'little')
        elif source.type == X86_OP_REG:
            data = registers[source.reg][:destination.size]
        elif source.type == X86_OP_MEM and source.mem.base == X86_REG_RIP and not source.mem.index:
            data = image.read(end + source.mem.disp, source.size)
        else:
            raise ValueError('Reset reads nonconstant memory')
        if instruction.mnemonic == 'vbroadcastsd':
            if len(data) != 8 or destination.size != 32:
                raise ValueError('Unexpected broadcast width')
            data *= 4
        if destination.type == X86_OP_REG:
            registers[destination.reg] = data + bytes(max(0, destination.size - len(data)))
        elif destination.type == X86_OP_MEM and destination.mem.base == X86_REG_RDI and not destination.mem.index:
            offset = destination.mem.disp
            if offset < 0 or offset + destination.size > len(output) or len(data) != destination.size:
                raise ValueError('Reset store outside action storage')
            output[offset:offset + destination.size] = data
            coverage[offset:offset + destination.size] = bytes([1]) * destination.size
        else:
            raise ValueError('Reset writes unsupported destination')
    raise ValueError('Reset routine did not terminate')
