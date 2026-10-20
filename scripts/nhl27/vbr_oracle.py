"""Build-bound, isolated native oracle for NHL27/NHL24 VBR codec helpers.

Copies only allowlisted helper code and its RIP-relative constants into a private
address space. No game entry point, imports, filesystem or game state are executed.
Run in a disposable Python subprocess: invalid code/data can terminate that process.
Requires Windows x64 with AVX2; accepts only the two previously audited binaries.
"""
import ctypes
import hashlib
import json
import sqlite3
import struct
import sys
from pathlib import Path
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_MEM, X86_OP_IMM, X86_REG_RIP
from research import ElfImage

SOURCE_SHA256='10c7bee4c27e5260b417f9574e65cee8338019d08635b3ab9dddab30ba524126'
HELPERS=(0x64a6c0,0x64ac20)
GUARD_SLOT=0xec2d9f0
STACK_FAILURE=0xbfabea0


def aligned(size):
    raw=ctypes.create_string_buffer(size+64)
    return raw,(ctypes.addressof(raw)+31)&~31


class Oracle:
    def __init__(self,root,target=False):
        self.target=target
        if target:
            sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'nhl24'))
            from inspect_eboot import Image
            im=Image(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
            expected='fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9'
            self.helpers=(0x851160,0x8516e0);guard_slot=0xa87c1f8;failure=0x8ec6790
        else:
            identity=json.loads((root/'identity.json').read_text());im=ElfImage(identity['path'])
            expected=SOURCE_SHA256
            self.helpers=HELPERS;guard_slot=GUARD_SLOT;failure=STACK_FAILURE
        if hashlib.sha256(im.data).hexdigest()!=expected:raise ValueError('Unrecognized oracle binary')
        db=sqlite3.connect(f'file:{(root/"index.sqlite").as_posix()}?mode=ro',uri=True)
        md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True;code={};constants={}
        for start in self.helpers:
            end=db.execute('select min(start) from functions where start>?',(start,)).fetchone()[0]
            if not start<end<start+0x10000:raise ValueError('Unexpected codec helper boundary')
            data=im.read(start,end-start);code[start]=data
            for ins in md.disasm(data,start):
                if ins.mnemonic.startswith('call'):
                    if len(ins.operands)!=1 or ins.operands[0].type!=X86_OP_IMM or ins.operands[0].imm!=failure:
                        raise ValueError('Codec helper contains an unapproved call')
                for op in ins.operands:
                    if op.type==X86_OP_MEM and op.mem.base==X86_REG_RIP:
                        at=ins.address+ins.size+op.mem.disp
                        constants[at]=im.read(at,max(op.size,32))
        db.close()
        self.k=ctypes.WinDLL('kernel32',use_last_error=True)
        self.k.VirtualAlloc.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_uint32,ctypes.c_uint32]
        self.k.VirtualAlloc.restype=ctypes.c_void_p
        self.k.VirtualProtect.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_uint32,ctypes.POINTER(ctypes.c_uint32)]
        self.k.VirtualFree.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_uint32]
        self.guard=0xed00000
        self.base=self.k.VirtualAlloc(None,self.guard+0x10000,0x2000,1)
        if not self.base:raise ctypes.WinError(ctypes.get_last_error())
        self.pages=set()
        def place(at,data):
            for page in range(at&~4095,(at+len(data)+4095)&~4095,4096):
                if page not in self.pages:
                    if not self.k.VirtualAlloc(self.base+page,4096,0x1000,4):raise ctypes.WinError(ctypes.get_last_error())
                    self.pages.add(page)
            ctypes.memmove(self.base+at,bytes(data),len(data))
        for at,data in constants.items():place(at,data)
        for at,data in code.items():place(at,data)
        place(self.guard,bytes(8));place(guard_slot,struct.pack('<Q',self.base+self.guard))
        place(failure,b'\x0f\x0b') # Unexpected guard failure stops the subprocess.
        # Windows -> SysV bridge; preserve Windows nonvolatile SIMD registers.
        shim=bytearray.fromhex('57564881eca80000004989cb')
        for reg in range(6,16):
            shim+=b'\xf3'+(b'\x44' if reg>=8 else b'')+b'\x0f\x7f'+bytes([0x84|((reg&7)<<3),0x24])+struct.pack('<I',(reg-6)*16)
        shim+=bytes.fromhex('498b03498b7b08498b7310498b5318498b4b204d8b43284d8b4b30')
        shim+=bytes.fromhex('f3410f104338f3410f104b3cf3410f105340ffd0')
        for reg in range(6,16):
            shim+=b'\xf3'+(b'\x44' if reg>=8 else b'')+b'\x0f\x6f'+bytes([0x84|((reg&7)<<3),0x24])+struct.pack('<I',(reg-6)*16)
        shim+=bytes.fromhex('4881c4a80000005e5fc3')
        place(self.guard+0x1000,shim)
        executable={self.guard+0x1000,failure&~4095}
        for at,data in code.items():executable.update(range(at&~4095,(at+len(data)+4095)&~4095,4096))
        for page in self.pages:
            old=ctypes.c_uint32()
            if not self.k.VirtualProtect(self.base+page,4096,0x20 if page in executable else 2,ctypes.byref(old)):
                raise ctypes.WinError(ctypes.get_last_error())
        self.invoke=ctypes.CFUNCTYPE(None,ctypes.c_void_p)(self.base+self.guard+0x1000)

    def call(self,helper,args,floats=(0.,0.,0.)):
        if helper not in self.helpers or len(args)>6:raise ValueError('Unapproved oracle invocation')
        block=struct.pack('<7Q3f',self.base+helper,*(list(args)+[0]*(6-len(args))),*floats)
        buf=ctypes.create_string_buffer(block);self.invoke(ctypes.addressof(buf))

    def unpack(self,q,v,widths,block):
        count=len(widths)
        if not 0<=q<=1024 or not 0<=v<=1024 or not 0<count<=4096 or count<q*4+v*3:
            raise ValueError('Invalid VBR oracle dimensions')
        if len(block)<3 or len(block)>1024*1024:raise ValueError('Invalid VBR block bounds')
        # Bound worst-case reads independent of the stream's declared block size.
        scalar_count=4*(q+v)+((count-4*q-3*v+3)&~3);size=scalar_count*8*4
        out,op=aligned(size+64);wb=ctypes.create_string_buffer(struct.pack('<'+'I'*count,*widths))
        bb=ctypes.create_string_buffer(block+bytes(count*20+64))
        ctypes.memset(op+size,0xa5,64)
        self.call(self.helpers[0],[q,v,op,ctypes.addressof(bb),ctypes.addressof(wb),count])
        if ctypes.string_at(op+size,64)!=bytes([0xa5])*64:raise ValueError('Oracle output guard overwritten')
        return ctypes.string_at(op,size),scalar_count

    def inverse(self,asset,coefficients,scalar_count,subquantizers):
        if len(asset)!=(184 if self.target else 168) or len(coefficients)!=scalar_count*32:raise ValueError('Invalid inverse input size')
        q,v,nf=struct.unpack_from('<3H',asset,108 if self.target else 140)
        if scalar_count!=4*(q+v)+((nf+3)&~3):raise ValueError('Inverse channel dimensions mismatch')
        if len(subquantizers)!=3 or any(not 0<=x<=255 for x in subquantizers):raise ValueError('Invalid quantizer')
        out,op=aligned(len(coefficients)+64);src,sp=aligned(len(coefficients));ctypes.memmove(sp,coefficients,len(coefficients))
        ab=ctypes.create_string_buffer(bytes(asset));ctypes.memset(op+len(coefficients),0xa5,64)
        self.call(self.helpers[1],[op,ctypes.addressof(ab),sp,scalar_count],tuple(float(x) for x in subquantizers))
        if ctypes.string_at(op+len(coefficients),64)!=bytes([0xa5])*64:raise ValueError('Inverse output guard overwritten')
        return ctypes.string_at(op,len(coefficients))

    def close(self):
        if self.base:self.k.VirtualFree(self.base,0,0x8000);self.base=None
