"""Run NHL24's original node entry/exit code against mock native interfaces."""
import ctypes as C
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_REG_RIP
from deke_runtime_evidence import checked_image, TARGET_HASH
from research import SelfImage


class NativeNodeLifecycleTests(unittest.TestCase):
    def test_original_activation_deactivation_and_entry_gates(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        target=Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not target.exists():
            self.skipTest('Private native target and Windows LLVM required')
        image=checked_image(SelfImage,target,TARGET_HASH)
        start,end=0xa71be0,0xa71f30
        raw=bytearray(image.read(start,end-start))
        decoder=Cs(CS_ARCH_X86,CS_MODE_64);decoder.detail=True
        instructions=list(decoder.disasm(raw,start))
        self.assertEqual(sum(i.size for i in instructions),len(raw))
        for i in instructions:
            self.assertFalse(any(o.type==X86_OP_MEM and o.mem.base==X86_REG_RIP for o in i.operands))
            if (i.mnemonic.startswith('j') or i.mnemonic=='call') and i.operands[0].type==X86_OP_IMM:
                dest=i.operands[0].imm
                if not start<=dest<end:
                    # This is the unrelated NodeType==3 helper. PTMSkating is
                    # type 0. Trap if a fixture accidentally reaches that path.
                    self.assertEqual((i.address,dest),(0xa71f19,0xa71f30))
                    struct.pack_into('<i',raw,i.address-start+1,len(raw)-(i.address-start+5))
        raw+=b'\x0f\x0b'
        assembly='.text\n.globl original_node_lifecycle\noriginal_node_lifecycle:\n'+ '\n'.join(
            '.byte '+','.join(str(b) for b in raw[i:i+24]) for i in range(0,len(raw),24))+'\n'
        fixture=r'''
typedef unsigned long long U;
extern __attribute__((sysv_abi)) void original_node_lifecycle(void*,void*,void*);
struct Signal { void **vt; int value,calls; };
static __attribute__((sysv_abi)) void set(struct Signal*s,void*ctx,int value){
    s->value=value;s->calls++;(*(int*)ctx)++;
}
static __attribute__((sysv_abi)) void *cast(void*self,unsigned kind){return (void*)0;}
__declspec(dllexport) void exercise(int permitted,int repeat,int *out){
    static unsigned char controller[0xf0],asset[208],owner[0x1000];
    for(unsigned i=0;i<sizeof(controller);++i)((volatile unsigned char*)controller)[i]=0;
    for(unsigned i=0;i<sizeof(asset);++i)((volatile unsigned char*)asset)[i]=0;
    for(unsigned i=0;i<sizeof(owner);++i)((volatile unsigned char*)owner)[i]=0;
    void *signal_vt[5]={0,0,0,0,(void*)set},*subject_vt[3]={0,0,(void*)cast};
    void **subject=subject_vt;
    struct Signal a={signal_vt,0,0},b={signal_vt,0,0};
    void *signals[3]={&a,0,&b}; int calls=0;
    *(void**)(asset+40)=signals;*(U*)(asset+80)=3; // Consumer uses deactivation count.
    *(void**)(controller+0x68)=asset;*(void**)(controller+0x70)=&subject;
    owner[0xa6]=(unsigned char)permitted;
    original_node_lifecycle(controller,owner,&calls);
    if(repeat)original_node_lifecycle(controller,owner,&calls);
    out[0]=a.value;out[1]=b.value;out[2]=calls;out[3]=controller[0xb4];
    original_node_lifecycle(controller,(void*)0,&calls);
    original_node_lifecycle(controller,(void*)0,&calls);
    out[4]=a.value;out[5]=b.value;out[6]=calls;out[7]=controller[0xb4];
    out[8]=a.calls;out[9]=b.calls;out[10]=*(void**)(controller+0x78)==0;
}
'''
        root=Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='panic-node-',dir=root) as folder:
            out=Path(folder);(out/'native.s').write_text(assembly);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-shared','-fuse-ld=lld','-nostdlib',
                '-Wl,/noentry',str(out/'fixture.c'),str(out/'native.s'),'-o',str(out/'test.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.exercise.argtypes=[C.c_int,C.c_int,C.POINTER(C.c_int)]
                for permitted in (0,1,2):
                    for repeat in (0,1):
                        values=(C.c_int*11)();lib.exercise(permitted,repeat,values)
                        self.assertEqual(list(values),[1,1,2,1,0,0,4,0,2,2,1] if permitted else [0,0,0,0,0,0,0,0,0,0,1])
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
