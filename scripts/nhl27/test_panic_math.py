"""Execute the source direction predicate with its relocated constant pool."""
import ctypes as C
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from panic_math import relocate,assembly


class PanicMathTests(unittest.TestCase):
    def test_original_predicate_cardinal_boundaries_and_relocation(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        data,receipt=relocate(image)
        for ref in receipt['constant_relocations']:
            destination=ref['instruction_end']+struct.unpack_from('<i',data,ref['site'])[0]
            self.assertEqual(destination,ref['destination'])
            self.assertEqual(data[destination:destination+ref['size']],
                             image.read(int(ref['original_address'],16),ref['size']))
        root=Path(__file__).resolve().parents[2]/'local/nhl27-eboot/compatibility/deke-trial/runtime-build'
        with tempfile.TemporaryDirectory(prefix='panic-math-',dir=root) as folder:
            out=Path(folder)
            (out/'math.s').write_text(assembly(data))
            (out/'fixture.c').write_text('''
int _fltused=0;
extern __attribute__((sysv_abi)) unsigned char nhl27_panic_direction_test(
    void *,const float *,const float *,const float *,float);
__declspec(dllexport) unsigned char predicate(const float *a,const float *b,float facing,float threshold) {
    return nhl27_panic_direction_test((void *)0,a,b,&facing,threshold);
}
''')
            result=subprocess.run([str(compiler),'-O2','-shared','-fuse-ld=lld','-nostdlib',
                '-Wl,/noentry',str(out/'fixture.c'),str(out/'math.s'),'-o',str(out/'test.dll')],
                capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:
                lib.predicate.argtypes=[C.POINTER(C.c_float),C.POINTER(C.c_float),C.c_float,C.c_float]
                lib.predicate.restype=C.c_ubyte
                origin=(C.c_float*4)(0,0,0,0)
                # Heading zero faces +Z; the predicate measures angle from the
                # opposite direction, so -Z is the zero-degree case.
                for xyz,threshold,expected in (((0,0,-1),0,1),((0,0,1),179,0),
                        ((0,0,1),180,1),((1,0,0),89,0),((1,0,0),91,1),
                        ((-1,0,0),89,0),((-1,0,0),91,1)):
                    position=(C.c_float*4)(*xyz,0)
                    self.assertEqual(lib.predicate(position,origin,0,threshold),expected)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
