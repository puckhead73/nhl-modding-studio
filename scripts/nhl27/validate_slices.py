"""Execute two bounded NHL 27 code slices with owned synthetic inputs.

Windows x64, AVX/BMI1-capable CPU required. This does not launch a game, qualify
a backport, or test complete rules/modes. Input hash and exact code are checked.
Usage: python scripts/nhl27/validate_slices.py EBOOT OUTPUT_JSON
"""
import argparse
import ctypes
import hashlib
import json
import platform
import struct
from pathlib import Path
from research import ElfImage

DIGEST = '10c7bee4c27e5260b417f9574e65cee8338019d08635b3ab9dddab30ba524126'


def run(source, output):
    if platform.system() != 'Windows' or ctypes.sizeof(ctypes.c_void_p) != 8:
        raise RuntimeError('Windows x64 is required')
    im = ElfImage(source)
    if hashlib.sha256(im.data).hexdigest() != DIGEST: raise ValueError('Wrong build')
    predicate = im.read(0x470a1f9, 0x470a233-0x470a1f9)
    expected = bytes.fromhex('48638850130000488bb058130000488b9038130000b0014889cf48f7df480faffe833c3a1b7413b8010000004829c8480faff0833c321b0f94c0')
    if predicate != expected: raise ValueError('Predicate code mismatch')
    clamp = im.read(0x3e483ce, 16)
    if clamp != bytes.fromhex('c1f81fc4c278f2c73dff0000000f4cd0'):
        raise ValueError('Clamp code mismatch')
    # Preserve Windows nonvolatile registers used by the original SysV code.
    predicate = bytes.fromhex('56574889c8') + predicate + bytes.fromhex('5f5ec3')
    clamp = bytes.fromhex('41574189cf89c8baff000000') + clamp + bytes.fromhex('89d0415fc3')
    k = ctypes.WinDLL('kernel32', use_last_error=True)
    k.VirtualAlloc.restype = ctypes.c_void_p
    k.VirtualAlloc.argtypes = [ctypes.c_void_p,ctypes.c_size_t,ctypes.c_uint32,ctypes.c_uint32]
    k.VirtualProtect.argtypes = [ctypes.c_void_p,ctypes.c_size_t,ctypes.c_uint32,ctypes.POINTER(ctypes.c_uint32)]
    k.VirtualFree.argtypes = [ctypes.c_void_p,ctypes.c_size_t,ctypes.c_uint32]
    k.GetCurrentProcess.restype = ctypes.c_void_p
    k.FlushInstructionCache.argtypes = [ctypes.c_void_p,ctypes.c_void_p,ctypes.c_size_t]
    mem = k.VirtualAlloc(None,4096,0x3000,4)
    if not mem: raise ctypes.WinError(ctypes.get_last_error())
    checks = [0,0]
    try:
        ctypes.memmove(mem,predicate,len(predicate));ctypes.memmove(mem+512,clamp,len(clamp))
        old = ctypes.c_uint32()
        if not k.VirtualProtect(mem,4096,0x20,ctypes.byref(old)): raise ctypes.WinError(ctypes.get_last_error())
        if not k.FlushInstructionCache(k.GetCurrentProcess(),mem,4096): raise ctypes.WinError(ctypes.get_last_error())
        f = ctypes.CFUNCTYPE(ctypes.c_ubyte,ctypes.c_void_p)(mem)
        g = ctypes.CFUNCTYPE(ctypes.c_int,ctypes.c_int)(mem+512)
        obj = ctypes.create_string_buffer(0x1360)
        data = ctypes.create_string_buffer(256)
        for stride in [4,8,16]:
            for base in [-2,-1,0,1,2]:
                pointer = ctypes.addressof(data)+64+base*stride
                struct.pack_into('<Q',obj,0x1338,pointer)
                struct.pack_into('<i',obj,0x1350,base)
                struct.pack_into('<Q',obj,0x1358,stride)
                for home in [-1,0,1,26,27,28,89]:
                    for away in [-1,0,1,26,27,28,89]:
                        struct.pack_into('<i',data,64,home);struct.pack_into('<i',data,64+stride,away)
                        actual = f(ctypes.addressof(obj))
                        if actual != int(home==27 or away==27): raise AssertionError((home,away,base,stride,actual))
                        checks[0]+=1
        for value in list(range(-1024,2049))+[-2147483648,2147483647]:
            if g(value) != min(255,max(0,value)): raise AssertionError(value)
            checks[1]+=1
    finally:
        if not k.VirtualFree(mem,0,0x8000): raise ctypes.WinError(ctypes.get_last_error())
    result={'source_sha256':DIGEST,'scope':'isolated machine code; no game/runtime qualification',
            'predicate':{'va':'0x470a1f9','cases':checks[0],'result':'either row 0 or row 1 equals 0x1b'},
            'table_length_clamp':{'va':'0x3e483ce','cases':checks[1],'result':'signed input clamped to 0..255'},
            'passed':True}
    output.write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('source',type=Path);p.add_argument('output',type=Path)
    a=p.parse_args();run(a.source,a.output)
