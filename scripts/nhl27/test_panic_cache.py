"""Compare the exact source and patched target PTM cache-producer instructions."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH,TARGET_HASH
from research import ElfImage,SelfImage
import panic_hooks

class CacheTests(unittest.TestCase):
 def test_all_raw_boolean_values(self):
  compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin');target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
  if os.name!='nt' or not compiler.exists() or not source.exists() or not target.exists():self.skipTest('Private inputs and Windows LLVM required')
  src=checked_image(ElfImage,source,SOURCE_HASH);dst=checked_image(SelfImage,target,TARGET_HASH)
  patch=panic_hooks.patches(dst)[0];raw=bytearray(dst.read(0x521aa8,0x15));at=patch[0]-0x521aa8;raw[at:at+3]=patch[2]
  asm=['.intel_syntax noprefix','.text']
  for name,code in [('source_cache',src.read(0x527d96,0xc)),('target_cache',bytes(raw))]:
   asm += [f'.globl {name}',name+':','push rbp','push rbx','push r12','sub rsp, 192','lea rbp,[rsp+192]',
           'mov rax,rdi','mov rbx,rsi','mov r12,rsi','.byte '+','.join(map(str,code)),
           'add rsp,192','pop r12','pop rbx','pop rbp','ret']
  fixture=r'''
#define ABI __attribute__((sysv_abi))
extern ABI void source_cache(void*,void*),target_cache(void*,void*);
__declspec(dllexport) void run(unsigned char value,unsigned char*out,unsigned target){
 if(target)target_cache(&value,out);else source_cache(&value,out);
}
'''
  repo=Path(__file__).resolve().parents[2]
  with tempfile.TemporaryDirectory(prefix='panic-cache-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
   out=Path(folder);(out/'native.s').write_text('\n'.join(asm));(out/'fixture.c').write_text(fixture)
   result=subprocess.run([str(compiler),'-O2','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',str(out/'fixture.c'),str(out/'native.s'),'-o',str(out/'test.dll')],capture_output=True,text=True)
   if result.returncode:raise RuntimeError(result.stdout+result.stderr)
   lib=C.CDLL(str(out/'test.dll'))
   try:
    lib.run.argtypes=[C.c_ubyte,C.c_void_p,C.c_uint]
    for value in range(256):
     a=C.create_string_buffer(bytes([0x55])*0x100);b=C.create_string_buffer(bytes([0x55])*0x100)
     lib.run(value,a,0);lib.run(value,b,1)
     self.assertEqual(a.raw[0xe6],value);self.assertEqual(b.raw[0xce],value)
     self.assertEqual(a.raw[:0xe6]+a.raw[0xe7:],b.raw[:0xce]+b.raw[0xcf:])
   finally:
    import _ctypes
    _ctypes.FreeLibrary(lib._handle)

if __name__=='__main__':unittest.main()
