"""Execute complete source request gates, including optional service lookup."""
import ctypes as C
import os
from pathlib import Path
import random
import subprocess
import tempfile
import unittest

from deke_runtime_evidence import checked_image, SOURCE_HASH
from oracle_fixture import assembly
from research import ElfImage


class RequestServiceTests(unittest.TestCase):
    def test_source_request_gates(self):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source = Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name != 'nt' or not all(p.exists() for p in (compiler, source)):
            self.skipTest('Private source and Windows LLVM required')
        im = checked_image(ElfImage, source, SOURCE_HASH)
        code = assembly(im, 'original_available', 0x453e350, 0x453e38f,
            {0x4a61a40: 'manager', 0x4a61ba0: 'record_service', 0x4a72120: 'scan_available'}, {})
        code += assembly(im, 'scan_available', 0x4a72120, 0x4a722de,
            {0x3461480: 'kind_blocked'}, {})
        code += assembly(im, 'kind_blocked', 0x3461480, 0x3461492, {}, {})
        code += assembly(im, 'original_blocked', 0x453e080, 0x453e0c0,
            {0x4a61a40: 'manager', 0x4a61ba0: 'record_service', 0x4a66730: 'scan_blocked_actor'}, {})
        code += assembly(im, 'scan_blocked_actor', 0x4a66730, 0x4a66738,
            {0x4a70a20: 'scan_blocked'}, {})
        code += assembly(im, 'scan_blocked', 0x4a70a20, 0x4a70bf6, {}, {})
        fixture = r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
extern ABI byte original_available(void*),original_blocked(void*,unsigned);
static byte actor[0x40],storage[0x70+16*0x2c],managers[2];
static unsigned calls,trace,mode,missing,mutate;
ABI void *manager(void) {
    trace=trace*7+1;calls++;
    if(!mode)return 0;
    return managers+(mode==2 && calls==2);
}
ABI void *record_service(void *m) {
    trace=trace*7+(m==managers?2:3);
    if(m!=managers+(mode==2))__builtin_trap();
    if(mutate)*(unsigned*)(actor+0x3c)=17;
    return missing?0:storage;
}
static void *get_manager(void *c) {return manager();}
static const struct panic_arcade_request *records(void *c,void *m) {
    byte *p=record_service(m);
    return p?(const struct panic_arcade_request*)(p+0x70):0;
}
__declspec(dllexport) unsigned run(unsigned source,unsigned category,unsigned flags,
                                  unsigned id,unsigned kind,const unsigned *input) {
    calls=trace=0;mode=flags&3;missing=flags&4;mutate=flags&8;
    *(unsigned*)(actor+0x3c)=id;
    for(unsigned i=0;i<16;i++) {
        unsigned *r=(unsigned*)(storage+0x70+i*0x2c);
        r[0]=input[i*3];r[1]=input[i*3+1];r[2]=input[i*3+2];
    }
    const struct panic_request_services s={0,get_manager,records};
    byte result=source?(category?original_blocked(actor,kind):original_available(actor)):
        (category?nhl27_panic_request_blocked(actor,kind,&s):nhl27_panic_request_available(actor,&s));
    return (trace<<8)|result;
}
int _fltused=0;
'''
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-request-services-', dir=
                root / 'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out = Path(folder)
            (out / 'oracle.s').write_text(code)
            (out / 'fixture.c').write_text(fixture)
            result = subprocess.run([str(compiler), '-O2', '-ffreestanding', '-fno-builtin',
                '-shared', '-nostdlib', '-fuse-ld=lld', '-Wl,/noentry', '-I',
                str(root / 'scripts/nhl27/runtime'), str(out / 'fixture.c'),
                str(out / 'oracle.s'), str(root / 'scripts/nhl27/runtime/panic_update.c'),
                '-o', str(out / 'test.dll')], capture_output=True, text=True)
            if result.returncode:
                raise RuntimeError(result.stdout + result.stderr)
            lib = C.CDLL(str(out / 'test.dll'))
            try:
                lib.run.argtypes = [C.c_uint] * 5 + [C.POINTER(C.c_uint)]
                lib.run.restype = C.c_uint
                rng = random.Random(0x4a72120)
                cases = []
                # Every record position, every service-presence case, and the
                # exact request-state comparison (not just a nonzero check).
                for slot in range(16):
                    for flags in (0, 1, 2, 4, 5, 6, 8, 9, 10, 13, 14):
                        for state in (0, 1, 2, 3, 255, 256, 0xffffffff):
                            for kind in (0, 10, 11, 12, 13, 14, 15, 16, 267, 0xffffffff):
                                rows = [0xffffffff, 0, 0] * 16
                                rows[slot * 3:slot * 3 + 3] = [17, state, kind]
                                cases.append((flags, 17, kind, rows))
                for _ in range(4000):
                    rows = [rng.choice((0, 1, 2, 3, 11, 13, 15, 17, 255, 0xffffffff)) for _ in range(48)]
                    cases.append((rng.choice((1, 2, 9, 10)), rng.choice((0, 17, 0xffffffff)),
                                  rng.choice((0, 10, 11, 13, 15, 0xffffffff)), rows))
                for n, (flags, ident, kind, rows) in enumerate(cases):
                    data = (C.c_uint * 48)(*rows)
                    for category in (0, 1):
                        args = (category, flags, ident, kind, data)
                        self.assertEqual(lib.run(0, *args), lib.run(1, *args),
                                         (n, category, flags, ident, kind, rows))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__ == '__main__':
    unittest.main()
