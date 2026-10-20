"""Source-instruction oracle for availability ordering and owner lifetime.

This qualifies the tail algorithm, not the unresolved native status/request
service bindings. Source-layout memory is intentional in this fixture.
"""
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


class AvailabilityTailTests(unittest.TestCase):
    def test_tail_against_source_instructions(self):
        compiler = Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source = Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name != 'nt' or not all(p.exists() for p in (compiler, source)):
            self.skipTest('Private source and Windows LLVM required')
        image = checked_image(ElfImage, source, SOURCE_HASH)
        code = assembly(image, 'original', 0x455e7b0, 0x455e960,
            {0x4481d60: 'manager_blocked', 0x4a19ee0: 'special_animation',
             0x453a9b0: 'global_blocked', 0x3e4220: 'state_condition',
             0x453cd50: 'actor_blocked', 0x4a19aa0: 'animation_blocked',
             0x453e350: 'request_allowed'},
            {0x1042c4c0: 'manager_pointer', 0xf223968: 'registry',
             0x103da598: 'request_options'},
            indirect_calls={0x455e8c3: 'ff90b0000000'})
        fixture = r'''
#include "panic_update.h"
#define ABI __attribute__((sysv_abi))
typedef unsigned char byte;
extern ABI byte original(void*);
static byte actor[0x230],animation[0xc68],wrapper[0x28],evaluation[0x80];
static byte role[0x18],owners[2][0xa0],components[2][0x60],manager[0x340];
void *manager_pointer=manager;
byte request_options[0x20];
static void *methods[24],*state_pointer;
void **registry=methods;
static const unsigned *input;
static unsigned *trace;
static unsigned value(unsigned n) {trace[++trace[0]]=n;return input[n];}
ABI unsigned manager_blocked(void *m,void *a) {
    if(m!=manager || a!=actor)__builtin_trap();return value(0);
}
ABI unsigned special_animation(void *a) {
    if(a!=animation)__builtin_trap();
    if(input[19]&1)*(unsigned*)(actor+0xc0)=14;
    return value(7);
}
ABI unsigned global_blocked(void *a) {
    if(a!=actor)__builtin_trap();return value(8);
}
ABI void **state_query(void *r,void *e) {
    if(r!=&registry || e!=evaluation+0x68)__builtin_trap();
    trace[++trace[0]]=30;return &state_pointer;
}
ABI unsigned state_condition(void *s) {
    if(s!=(input[20]?0:role))__builtin_trap();
    /* Switch the actor owner, then mutate the captured owner's component. */
    if(input[19]&2)*(void**)(actor+0x228)=owners[1];
    if(input[19]&4)*(void**)(owners[0]+0x98)=components[1];
    return value(12);
}
ABI unsigned actor_blocked(void *a) {
    if(a!=actor)__builtin_trap();return value(13);
}
ABI unsigned animation_blocked(void *a) {
    if(a!=animation)__builtin_trap();return value(14);
}
ABI unsigned request_allowed(void *a) {
    if(a!=actor)__builtin_trap();return value(16);
}
static panic_address query(void *c,enum panic_availability_query q,void *a) {
    if(a!=actor)__builtin_trap();
    switch(q) {
    case PANIC_AVAIL_MANAGER_BLOCKED:return manager_blocked(manager,a);
    case PANIC_AVAIL_FLAG_B0:return *(unsigned*)(actor+0xb0);
    case PANIC_AVAIL_FLAG_B8:return *(unsigned*)(actor+0xb8);
    case PANIC_AVAIL_ACTOR_BYTE_40:return actor[0x40];
    case PANIC_AVAIL_CONTROL:return *(unsigned*)(actor+0x1a4);
    case PANIC_AVAIL_CONTROL_MODE:return *(unsigned*)(actor+0x1c0);
    case PANIC_AVAIL_SOURCE_STATUS:return *(unsigned*)(actor+0xc0);
    case PANIC_AVAIL_SPECIAL_ANIMATION:return special_animation(animation);
    case PANIC_AVAIL_GLOBAL_BLOCKED:return global_blocked(a);
    case PANIC_AVAIL_ROLE_KIND:return *(unsigned*)(role+0x10);
    case PANIC_AVAIL_FLAG_B4:return *(unsigned*)(actor+0xb4);
    case PANIC_AVAIL_OWNER:return *(panic_address*)(actor+0x228);
    case PANIC_AVAIL_STATE_CONDITION:
        if(input[20])return state_condition(0);
        return state_condition(*state_query(&registry,evaluation+0x68));
    case PANIC_AVAIL_ACTOR_STATE_BLOCKED:return actor_blocked(a);
    case PANIC_AVAIL_ANIMATION_BLOCKED:return animation_blocked(animation);
    case PANIC_AVAIL_REQUEST_OPTION:return request_options[0x1b];
    case PANIC_AVAIL_REQUEST_ALLOWED:return request_allowed(a);
    }
    __builtin_trap();
}
__declspec(dllexport) void run(unsigned source,const unsigned *v,unsigned *out) {
    input=v;trace=out;out[0]=0;
    *(void**)(actor+0x60)=animation;actor[0x40]=v[3];
    *(unsigned*)(actor+0xb0)=v[1];*(unsigned*)(actor+0xb8)=v[2];
    *(unsigned*)(actor+0x1a4)=v[4];*(unsigned*)(actor+0x1c0)=v[5];
    *(unsigned*)(actor+0xc0)=v[6];*(unsigned*)(actor+0xb4)=v[10];
    *(void**)(actor+0x110)=role;*(unsigned*)(role+0x10)=v[9];
    *(void**)(actor+0x228)=v[11]?0:owners[0];
    *(void**)(owners[0]+0x98)=v[21]?0:components[0];
    *(void**)(owners[1]+0x98)=components[1];
    *(unsigned*)(components[0]+0x10)=v[17];components[0][0x28]=v[18];
    *(unsigned*)(components[1]+0x10)=0x4000;
    *(void**)(animation+0xc60)=v[20]==1?0:wrapper;
    *(void**)(wrapper+0x20)=v[20]==2?0:evaluation;
    *(void**)(manager+0x338)=manager;
    methods[0xb0/8]=state_query;state_pointer=role;request_options[0x1b]=v[15];
    const struct panic_availability_services services={0,query};
    out[31]=source?original(actor):nhl27_panic_availability_tail(actor,&services);
}
int _fltused=0;
'''
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-availability-tail-', dir=
                root / 'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out = Path(folder)
            (out / 'oracle.s').write_text(code)
            (out / 'fixture.c').write_text(fixture)
            result = subprocess.run([str(compiler), '-O2', '-ffreestanding',
                '-fno-builtin', '-shared', '-nostdlib', '-fuse-ld=lld', '-Wl,/noentry',
                '-I', str(root / 'scripts/nhl27/runtime'), str(out / 'fixture.c'),
                str(out / 'oracle.s'), str(root / 'scripts/nhl27/runtime/panic_update.c'),
                '-o', str(out / 'test.dll')], capture_output=True, text=True)
            if result.returncode:
                raise RuntimeError(result.stdout + result.stderr)
            lib = C.CDLL(str(out / 'test.dll'))
            try:
                lib.run.argtypes = [C.c_uint, C.POINTER(C.c_uint), C.POINTER(C.c_uint)]
                rng = random.Random(0x455e7b0)
                default = [0] * 22
                default[16] = 1
                cases = [default.copy()]
                # Individually exercise every gate, including high-byte returns.
                for index in range(22):
                    for value in (0, 1, 2, 3, 7, 14, 17, 18, 255, 256, 0x2000, 0x4000, 0xffffffff):
                        values = default.copy()
                        values[index] = value if index != 20 else value % 3
                        cases.append(values)
                for _ in range(20000):
                    values = default.copy()
                    for _ in range(rng.randrange(1, 7)):
                        index = rng.randrange(22)
                        values[index] = rng.choice((0, 1, 2, 3, 14, 17, 255, 256, 0x2000, 0x4000))
                    values[20] %= 3
                    cases.append(values)
                for n, values in enumerate(cases):
                    inp = (C.c_uint * 22)(*values)
                    a, b = (C.c_uint * 32)(), (C.c_uint * 32)()
                    lib.run(0, inp, a)
                    lib.run(1, inp, b)
                    self.assertEqual(list(a), list(b), (n, values))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__ == '__main__':
    unittest.main()
