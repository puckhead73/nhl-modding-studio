"""Compare the full original between-legs call flow to the adapted geometry."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest
from capstone import Cs,CS_ARCH_X86,CS_MODE_64
from capstone.x86 import X86_OP_MEM,X86_OP_IMM,X86_REG_RIP
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
import panic_geometry

class GeometryTests(unittest.TestCase):
 def test_original_between_legs_geometry(self):
  compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe');path=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
  if os.name!='nt' or not compiler.exists() or not path.exists():self.skipTest('Private source and Windows LLVM required')
  image=checked_image(ElfImage,path,SOURCE_HASH);md=Cs(CS_ARCH_X86,CS_MODE_64);md.detail=True
  start,end=0x4a3c260,0x4a3c450
  calls={0x44c6850:'predict',0x453fa00:'current',0x4543f10:'previous',0x69bc2c0:'nhl27_segment_overlap',0x24f74a0:'nhl27_segment_intersection',0xbfabea0:'fail'}
  globals_={0xec2d9f0:'cookie_global',0x1042c4c0:'manager_global',0x104331f0:'puck_global'}
  asm=['.text','.globl original_between','original_between:']
  def emit(raw):
   if raw:asm.append('.byte '+','.join(map(str,raw)))
  for ins in md.disasm(image.read(start,end-start),start):
   fix=None
   if ins.mnemonic=='call':fix=(ins.imm_offset,calls[ins.operands[0].imm])
   elif ins.mnemonic.startswith('j'):self.assertTrue(start<=ins.operands[0].imm<end)
   for op in ins.operands:
    if op.type==X86_OP_MEM and op.mem.base==X86_REG_RIP:fix=(ins.disp_offset,globals_[ins.address+ins.size+op.mem.disp])
   if fix:
    off,name=fix;emit(ins.bytes[:off]);asm.append(f'.long {name} - . - {ins.size-off}');emit(ins.bytes[off+4:])
   else:emit(ins.bytes)
  geometry,_=panic_geometry.geometry(image);asm.append(geometry)
  fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
extern ABI unsigned char original_between(void*,unsigned char,int);
extern unsigned char nhl27_panic_between_feet(const float*,const float*,const float*,const float*,const float*,const float*);
static float points[6][4];
static unsigned char puck[0x18],pose[0x60],component[0x18];
static unsigned long long cookie=0x12345678;
void *cookie_global=&cookie,*manager_global=component,*puck_global=puck;
ABI void fail(void){__builtin_trap();}
ABI void predict(void*a,void*b,void*c,int mode,float ticks){__builtin_trap();}
ABI void current(float*out,void*actor,unsigned bone){for(unsigned n=0;n<4;n++)out[n]=points[bone==0x27?2:3][n];}
ABI void previous(float*out,void*actor,unsigned bone){for(unsigned n=0;n<4;n++)out[n]=points[bone==0x27?4:5][n];}
ABI int nhl24_panic_foot_probe_guest(panic_address base,void*actor,unsigned bone,float*out){__builtin_trap();}
__declspec(dllexport) unsigned char oracle(float*input){
 for(unsigned i=0;i<6;i++)for(unsigned j=0;j<4;j++)points[i][j]=input[i*4+j];
 *(void**)(puck+0x10)=pose;
 for(unsigned j=0;j<4;j++){((float*)(pose+0x30))[j]=points[0][j];((float*)(pose+0x40))[j]=points[1][j];}
 return original_between(component,0,2);
}
__declspec(dllexport) unsigned char adapted(float*p){return nhl27_panic_between_feet(p,p+4,p+8,p+12,p+16,p+20);}
int _fltused=0;
'''
  repo=Path(__file__).resolve().parents[2]
  with tempfile.TemporaryDirectory(prefix='panic-geometry-',dir=repo/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
   out=Path(folder);(out/'native.s').write_text('\n'.join(asm));(out/'fixture.c').write_text(fixture)
   result=subprocess.run([str(compiler),'-O2','-ffp-contract=off','-ffreestanding','-fno-builtin','-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(repo/'scripts/nhl27/runtime'),str(out/'fixture.c'),str(out/'native.s'),str(repo/'scripts/nhl27/runtime/panic_geometry.c'),'-o',str(out/'test.dll')],capture_output=True,text=True)
   if result.returncode:raise RuntimeError(result.stdout+result.stderr)
   lib=C.CDLL(str(out/'test.dll'))
   try:
    for name in ('oracle','adapted'):getattr(lib,name).argtypes=[C.POINTER(C.c_float)];getattr(lib,name).restype=C.c_ubyte
    rng=random.Random(2724)
    for n in range(12000):
     values=[rng.uniform(-3,3) for _ in range(24)]
     if n<1000:values=[rng.choice([0,-0.0,1,-1,1e-6,float('nan'),float('inf')]) for _ in range(24)]
     points=(C.c_float*24)(*values)
     self.assertEqual(lib.oracle(points),lib.adapted(points),(n,values))
   finally:
    import _ctypes
    _ctypes.FreeLibrary(lib._handle)

if __name__=='__main__':unittest.main()
