"""Exhaustively compare packed glide inputs against the source machine code."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image, SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly as oracle
from input_glide_hooks import decoder, context_gate


class GlideDecoderTests(unittest.TestCase):
    def test_glide_facing_table_and_index_are_identical(self):
        from research import SelfImage
        from deke_runtime_evidence import TARGET_HASH
        s=checked_image(ElfImage,Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'),SOURCE_HASH)
        t=checked_image(SelfImage,Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'),TARGET_HASH)
        self.assertEqual(s.read(0xdef03e0,16),t.read(0x905b630,16))
        self.assertEqual(t.read(0x2ecaaa9,13),bytes.fromhex('418b8424d8000000ffc883f803'))

    def test_all_packed_values_against_original_source(self):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source = Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name != 'nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image = checked_image(ElfImage, source, SOURCE_HASH)
        code = oracle(image, 'original', 0x45e8c20, 0x45e8c93, {}, {})
        code += oracle(image,'original_gate',0x4a1d144,0x4a1d172,
                       {0x4a1d2ad:'original_skip'},{0x104331f0:'puck'})
        code += '''
.intel_syntax noprefix
    mov eax, 1
    ret
original_skip:
    xor eax, eax
    ret
.globl original_gate_call
original_gate_call:
    push r13
    push r14
    sub rsp, 8
    mov r14, rdi
    mov r13, rsi
    call original_gate
    add rsp, 8
    pop r14
    pop r13
    ret
.globl ported_gate_call
ported_gate_call:
    push r12
    mov r12, rsi
    call input_glide_context_gate
    pop r12
    ret
eligible:
    test r12, r12
    jz original_skip
    test byte ptr [r12+0x10], 4
    jz original_skip
    mov eax, 1
    ret
'''
        code += context_gate('puck','eligible','original_skip')
        code += decoder('ported', 'jmp ported_end')
        code += '''
.globl original_bits
original_bits:
    sub rsp, 8
    call original
    add rsp, 8
    vmovq rax, xmm0
    ret
.globl ported_bits
ported_bits:
    vmovd xmm0, edi
    jmp ported
ported_end:
    vmovd eax, xmm0
    ret
'''
        fixture = r'''
typedef unsigned U;
typedef unsigned long long Q;
#define ABI __attribute__((sysv_abi))
extern ABI Q original_bits(void*);
extern ABI U ported_bits(U);
extern ABI U original_gate_call(void*,void*);
extern ABI U ported_gate_call(void*,void*);
static U puck_storage[64];
void *puck=puck_storage;
__declspec(dllexport) U check(U *details) {
    U record[3]={0}; void *pointer=record;
    for(U n=0;n<65536;n++) {
        /* Exercise both packed lanes, including opposite sign/exponent. */
        record[1]=n|((65535-n)<<16);
        Q expected=original_bits(&pointer);
        U a=ported_bits(n),b=ported_bits(65535-n);
        if(a!=(U)expected || b!=(U)(expected>>32)) {
            details[0]=n;details[1]=a;details[2]=(U)expected;
            details[3]=b;details[4]=(U)(expected>>32);return 0;
        }
    }
    return 65536;
}
__declspec(dllexport) U gates(void) {
    U actor[64]={0},record[8]={0};
    for(U owner=0;owner<4;owner++)for(U id=0;id<4;id++)
    for(U flags=0;flags<256;flags++)for(U present=0;present<2;present++) {
        puck_storage[0xd8/4]=owner;actor[0x3c/4]=id;record[4]=flags;
        void *p=present?record:0;
        U original=original_gate_call(actor,p),ported=ported_gate_call(actor,p);
        if(original!=ported)return 0;
    }
    return 8192;
}
'''
        root = Path(__file__).resolve().parents[2]
        scratch = root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='glide-decode-', dir=scratch) as folder:
            out=Path(folder)
            (out/'oracle.s').write_text(code)
            (out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry',
                str(out/'fixture.c'),str(out/'oracle.s'),'-o',str(out/'fixture.dll')],
                capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'fixture.dll'))
            try:
                lib.check.argtypes=[C.POINTER(C.c_uint)]
                lib.check.restype=C.c_uint
                details=(C.c_uint*5)()
                self.assertEqual(lib.check(details),65536,list(details))
                self.assertEqual(lib.gates(),8192)
            finally:
                C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__ == '__main__':
    unittest.main()
