"""Execute the isolated pressure-rating block with synthetic inputs on Windows x64.

This is an offline machine-code regression test, NOT a game/runtime qualification.
Only a bounded, hash-identified instruction block is copied to this process.
Its three RIP-relative data operands are redirected to owned synthetic fixtures.
Usage: python scripts/nhl24/validate_pressure_slice.py BASE_EBOOT UPDATE_EBOOT OUTPUT_JSON
"""
import argparse
import ctypes
import hashlib
import json
import platform
import struct
from pathlib import Path
from inspect_eboot import Image

BUILDS = [
    ('01.00','0118720c7e0338e20769063437be5656d1c16262c6b81a688adb659582544e17',0x282f914),
    ('01.60','fe5dcab197256487dd47906a4b0e0601bfe77fac9f47ad3f4f91b03b9372deb9',0x27b6394),
]


def validate(path, build):
    version, digest, start = build
    im = Image(path)
    if hashlib.sha256(im.data).hexdigest() != digest:
        raise ValueError('Executable identity mismatch')
    body = im.read(start, 0x59)  # End immediately before the original epilogue jump.
    if body[:5] != bytes.fromhex('83f8017422') or body[-3:] != bytes.fromhex('0f4cc1'):
        raise ValueError('Unexpected instruction block')
    # Preserve Windows nonvolatile r12/r13. Inputs: ecx state, edx attribute, r8d rating.
    wrapper = bytes.fromhex('4154415589c84189d44589c5')
    tail = bytes.fromhex('415d415cc3')
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.VirtualAlloc.restype=ctypes.c_void_p
    kernel.VirtualAlloc.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_uint32,ctypes.c_uint32]
    kernel.VirtualFree.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_uint32]
    kernel.VirtualProtect.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_uint32,ctypes.POINTER(ctypes.c_uint32)]
    kernel.GetCurrentProcess.restype=ctypes.c_void_p
    kernel.FlushInstructionCache.argtypes=[ctypes.c_void_p,ctypes.c_void_p,ctypes.c_size_t]
    memory = kernel.VirtualAlloc(None,4096,0x3000,0x04)
    if not memory:raise ctypes.WinError(ctypes.get_last_error())
    count=0
    try:
        # Two independently callable copies; all absolute fixture pointers are local.
        for offset,patched in [(0,False),(512,True)]:
            block=bytearray(wrapper+body+tail)
            if patched:block[len(wrapper):len(wrapper)+5]=bytes.fromhex('31c0909090')
            for displacement_offset,following_offset,target in [(9,13,1024),(21,25,1040),(42,46,1048)]:
                struct.pack_into('<i',block,len(wrapper)+displacement_offset,target-(offset+len(wrapper)+following_offset))
            ctypes.memmove(memory+offset,bytes(block),len(block))
        ctypes.memmove(memory+1024,struct.pack('<f',1.0),4)
        ctypes.memmove(memory+1040,struct.pack('<QQ',memory+1100,memory+1200),16)
        # Values match measured stock defaults, with live object scalar at +0x34.
        ctypes.memmove(memory+1100+0x34,struct.pack('<f',1.05),4)
        ctypes.memmove(memory+1200+0x34,struct.pack('<f',0.8),4)
        old=ctypes.c_uint32()
        if not kernel.VirtualProtect(memory,4096,0x20,ctypes.byref(old)):
            raise ctypes.WinError(ctypes.get_last_error())
        kernel.FlushInstructionCache(kernel.GetCurrentProcess(),memory,4096)
        signature=ctypes.CFUNCTYPE(ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_int)
        stock,patch=signature(memory),signature(memory+512)
        for state in [-1,0,1,2,3]:
            for attribute in range(32):
                for rating in range(-10,121):
                    multiplier=0.8 if state==1 else (1.05 if state==2 and attribute in [8,15] else 1.0)
                    f32=lambda x:struct.unpack('<f',struct.pack('<f',x))[0]
                    expected=max(36,min(99,int(f32(f32(multiplier)*f32(rating)))))
                    actual=stock(state,attribute,rating)
                    neutral=patch(state,attribute,rating)
                    if actual!=expected or neutral!=max(36,min(99,rating)):
                        raise AssertionError((state,attribute,rating,actual,expected,neutral))
                    count+=1
    finally:
        if not kernel.VirtualFree(memory,0,0x8000):raise ctypes.WinError(ctypes.get_last_error())
    return {'version':version,'sha256':digest,'cases':count,'comparisons':count*2,
            'image_address':hex(start),'xml_address':hex(start+0x400000),
            'self_offset':hex(im.offset(start)),'expected':'83f8017422','replacement':'31c0909090',
            'result':'passed isolated machine-code test; game untested'}


if __name__=='__main__':
    if platform.system()!='Windows' or ctypes.sizeof(ctypes.c_void_p)!=8:
        raise SystemExit('Requires Windows x64 and an AVX-capable CPU/OS')
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('base',type=Path);p.add_argument('update',type=Path);p.add_argument('output',type=Path)
    a=p.parse_args()
    results=[validate(a.base,BUILDS[0]),validate(a.update,BUILDS[1])]
    a.output.write_text(json.dumps(results,indent=2));print(json.dumps(results,indent=2))
