"""Original panic motion gates and cross-title parent-context lookup oracle."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from capstone import Cs,CS_ARCH_X86,CS_MODE_64
from capstone.x86 import X86_OP_MEM,X86_OP_IMM,X86_REG_RIP
from deke_runtime_evidence import checked_image,SOURCE_HASH,TARGET_HASH
from research import ElfImage,SelfImage

class MotionTests(unittest.TestCase):
 def test_source_predicates_native_context_lookup(self):
  compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin');target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
  if os.name!='nt' or not compiler.exists() or not source.exists() or not target.exists():self.skipTest('Private inputs and Windows LLVM required')
  src=checked_image(ElfImage,source,SOURCE_HASH);dst=checked_image(SelfImage,target,TARGET_HASH)
  md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True;asm=[]
  for image,name,start,size in [(src,'original_panic',0x4a1ab90,0x90),(src,'original_close',0x4a1ab20,0x70),(src,'source_lookup',0x74e4c0,0x70),(dst,'target_lookup',0x8a7780,0x60)]:
   asm+=['.text','.globl '+name,name+':']
   calls={0x3915f0:'get_root',0x390da0:'get_record',0x74e4c0:'source_lookup'}
   for i in md.disasm(image.read(start,size),start):
    self.assertFalse(any(o.type==X86_OP_MEM and o.mem.base==X86_REG_RIP for o in i.operands))
    if i.mnemonic.startswith('j'):self.assertTrue(start<=i.operands[0].imm<start+size)
    if i.mnemonic=='call':asm+=['.byte '+','.join(map(str,i.bytes[:i.imm_offset])),f'.long {calls[i.operands[0].imm]} - . - 4']
    else:asm+=['.byte '+','.join(map(str,i.bytes))]
  fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
typedef void *(ABI *lookup)(void*,unsigned);
extern ABI void *target_lookup(void*,unsigned);
extern unsigned char nhl24_panic_motion_bit(unsigned char,const unsigned char*,const void*,void*,unsigned,lookup);
extern ABI unsigned char original_panic(void*),original_close(void*);
static unsigned char anim[0xc68],config[0xe8],root[0x120],skating_asset[0xf8],property[0x50],native_property[0x28],record[0xe8],wrapper[0x28];
static unsigned char contexts[3][0x80],blocks[3][0x900],node;
ABI void *get_root(void*unused){return root;}
ABI void *get_record(void*a,void*b){return record;}
__declspec(dllexport) unsigned run(unsigned active,unsigned node_value,unsigned count,unsigned key,unsigned present,unsigned bits,unsigned use_native,unsigned bit){
 node=(unsigned char)node_value;config[0xe6]=(unsigned char)active;
 *(void**)(anim+0x48)=config;*(void**)(anim+0xc60)=wrapper;*(void**)(wrapper+0x20)=contexts[0]-0x68;
 *(void**)(root+0x118)=skating_asset;*(void**)(skating_asset+0xf0)=property;
 *(void**)(record+0xe0)=node_value==256?0:&node;
 property[0x48]=(unsigned char)count;*(unsigned*)(property+0x28)=key;
 native_property[32]=(unsigned char)count;*(unsigned*)(native_property+8)=key;
 unsigned slot=key&0xfff,index=(key>>12)&0x7ff;
 if(slot>=8)__builtin_trap();
 for(unsigned n=0;n<3;n++){
   for(unsigned j=0;j<sizeof(contexts[n]);j++)contexts[n][j]=0;
   for(unsigned j=0;j<sizeof(blocks[n]);j++)blocks[n][j]=0;
   *(void**)(contexts[n]+8)=n<2?contexts[n+1]:0;
   *(void**)(contexts[n]+0x18+slot*8)=(present&(1u<<n))?blocks[n]:0;
   *(unsigned*)(blocks[n]+0x71c+(index>>5)*4)=(present&(8u<<n))?1u<<(index&31):0;
   blocks[n][index]=(unsigned char)bits;blocks[n][index+1]=(unsigned char)(bits>>8);
 }
 if(use_native)return nhl24_panic_motion_bit((unsigned char)active,node_value==256?0:&node,native_property,contexts[0],bit,target_lookup);
 return bit==9?original_panic(anim):original_close(anim);
}
int _fltused=0;
'''
  repo=Path(__file__).resolve().parents[2]
  with tempfile.TemporaryDirectory(prefix='panic-motion-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
   out=Path(folder);(out/'native.s').write_text('\n'.join(asm));(out/'fixture.c').write_text(fixture)
   result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'native.s'),str(repo/'scripts/nhl27/runtime/panic_services.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
   if result.returncode:raise RuntimeError(result.stdout+result.stderr)
   lib=C.CDLL(str(out/'test.dll'))
   try:
    lib.run.argtypes=[C.c_uint]*8;lib.run.restype=C.c_uint;rng=random.Random(279)
    for n in range(10000):
     # Keep tested storage bytes outside the initialization bitmap itself.
     key=rng.choice([0,0xffffff,0x1000000,0x1001000,0x101f003,0x1020007,0x16ff005])
     if key==0xffffff:key=0xfff007
     args=[rng.choice([0,1,1,2]),rng.choice([0,1,1,2,256]),rng.choice([0,1,2,255]),key,rng.randrange(64),rng.randrange(65536)]
     bit=rng.choice([3,9]);self.assertEqual(lib.run(*args,0,bit),lib.run(*args,1,bit),(n,args,bit))
   finally:
    import _ctypes
    _ctypes.FreeLibrary(lib._handle)

if __name__=='__main__':unittest.main()
