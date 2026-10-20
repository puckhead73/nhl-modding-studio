"""Execute lane scheduling hooks with destructive callees and original prologues."""
import ctypes
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import panic_lane_hooks as hook
from test_input_asset_hook import fixture


class LaneHookTests(unittest.TestCase):
    def test_update_and_reset_preserve_machine_state(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        if os.name!='nt' or not compiler.exists():self.skipTest('Windows LLVM required')
        root=Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        for reset in (False,True):
            with self.subTest(reset=reset),tempfile.TemporaryDirectory(prefix='lane-hooks-',dir=root) as folder:
                code='.intel_syntax noprefix\n.text\nfixture_base:\n'
                code+=fixture('original',False,True)
                name='panic_lanes_reset' if reset else 'panic_lanes_update'
                code+=fixture('hooked',False,True).replace('jmp fixture_original','jmp '+name)
                code+=hook.assembly('fixture_base','fixture_original','fixture_resume')
                code+='fixture_original:\n'
                if reset:
                    code+='.byte '+','.join(str(b) for b in hook.RESET_BEFORE)+'\n'
                    code+='fixture_resume:\npop r15\npop rbp\n'
                else:
                    code+='fixture_resume:\n'
                code+='mov byte ptr [r13],127\nret\n'
                code+='nhl24_panic_lanes_guest:\nnhl24_panic_lanes_reset_guest:\n'
                code+='lea rax,[rip+fixture_base]\ncmp rdi,rax\njne bad\n'
                # fixture() initializes RDI to this sentinel; helper receives
                # it as its second argument, not the overwritten base address.
                from input_producer_hooks import REGISTERS
                code+=f'cmp rsi,{0x123400+REGISTERS.index("rdi")}\njne bad\n'
                code+='mov byte ptr [r13+24],1\njmp clobber\nbad:\nmov byte ptr [r13+24],2\nclobber:\n'
                code+='\n'.join(f'vpxor ymm{i},ymm{i},ymm{i}' for i in range(16))+'\n'
                code+='\n'.join(f'mov {r},0xbadcafe' for r in ('rax','rcx','rdx','rsi','rdi','r8','r9','r10','r11'))
                code+='\nstc\nret\n'
                path=Path(folder);(path/'fixture.s').write_text(code)
                result=subprocess.run([str(compiler),'-shared','-fuse-ld=lld','-nostdlib','-mavx2',
                    '-Wl,/noentry','-Wl,/export:original','-Wl,/export:hooked',str(path/'fixture.s'),
                    '-o',str(path/'fixture.dll')],capture_output=True,text=True)
                if result.returncode:raise RuntimeError(result.stdout+result.stderr)
                lib=ctypes.CDLL(str(path/'fixture.dll'))
                try:
                    buffer=ctypes.create_string_buffer(672)
                    lib.original.argtypes=lib.hooked.argtypes=[ctypes.c_void_p]
                    lib.original(buffer);expected=bytes(buffer)
                    ctypes.memset(buffer,0,len(buffer));lib.hooked(buffer)
                    self.assertEqual(bytes(buffer)[:664],expected[:664])
                    self.assertEqual(buffer[664],b'\x01')
                finally:
                    import _ctypes
                    _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
