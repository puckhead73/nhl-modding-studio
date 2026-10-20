"""Native skating-style columns versus the original panic constructor reads."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from deke_runtime_evidence import checked_image,SOURCE_HASH
from research import ElfImage
from oracle_fixture import assembly


class PanicRatingTests(unittest.TestCase):
    def test_columns_row_mapping_bias_stride_and_normalization(self):
        compiler=Path(r'C:\Program Files\LLVM\bin\clang.exe')
        source=Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin')
        if os.name!='nt' or not compiler.exists() or not source.exists():
            self.skipTest('Private source and Windows LLVM required')
        image=checked_image(ElfImage,source,SOURCE_HASH)
        code=assembly(image,'original',0x4a54380,0x4a544d0,
            {0x4a51930:'reset',0x4b254d0:'database',0x4a05fb0:'normalize'},{})
        code+=assembly(image,'normalize',0x4a05fb0,0x4a05fe0,{},{})
        fixture=r'''
#include "panic_state.h"
#define ABI __attribute__((sysv_abi))
extern ABI void original(struct panic_state*,void*);
extern int nhl24_panic_style_rating(const void*,int,unsigned,int*);
extern float nhl27_panic_attribute(unsigned,int);
static unsigned char source_table[0x3670],target_table[0x2e00],actor[0x80];
static int source_values[4][384][3],target_values[4][384][5];
ABI void reset(void*p){(void)p;}
ABI void *database(void){return source_table;}
__declspec(dllexport) unsigned check(void){
    const unsigned ids[4]={0x11d,0x11f,0x121,0x120};
    const unsigned old_offsets[4]={0x3588,0x35e8,0x3648,0x3618};
    const unsigned new_offsets[4]={0x2d18,0x2d78,0x2dd8,0x2da8};
    for(unsigned n=0;n<4;n++) {
        /* Different column biases and strides prevent accidental flat reads. */
        for(int row=-128;row<200;row++){
            int value=(row*13+(int)n*37)%201;
            source_values[n][row+128][0]=value;
            target_values[n][row+144][0]=value;
        }
        *(void**)(source_table+old_offsets[n])=source_values[n];
        *(int*)(source_table+old_offsets[n]+0x18)=-128;
        *(panic_address*)(source_table+old_offsets[n]+0x20)=12;
        *(void**)(target_table+new_offsets[n])=target_values[n];
        *(int*)(target_table+new_offsets[n]+0x18)=-144;
        *(panic_address*)(target_table+new_offsets[n]+0x20)=20;
    }
    unsigned count=0;
    for(int actor_id=-41;actor_id<122;actor_id++) {
        *(int*)(actor+0x3c)=actor_id;
        struct panic_state state;original(&state,actor);
        for(unsigned n=0;n<4;n++) {
            int rating;
            if(!nhl24_panic_style_rating(target_table,actor_id,ids[n],&rating))return 1;
            union {float f;unsigned u;} value={nhl27_panic_attribute(ids[n],rating)};
            union {float f;unsigned u;} expected={state.attributes[n]};
            if(value.u!=expected.u)return 2;
            count++;
        }
    }
    int unchanged=123;
    if(nhl24_panic_style_rating(target_table,0,0x123,&unchanged)||unchanged!=123)return 3;
    if(nhl24_panic_style_rating(0,0,0x11d,&unchanged)||unchanged!=123)return 4;
    return count;
}
'''
        root=Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory(prefix='panic-ratings-',dir=root/'local/nhl27-eboot/compatibility/deke-trial/runtime-build') as folder:
            out=Path(folder);(out/'oracle.s').write_text(code);(out/'fixture.c').write_text(fixture)
            result=subprocess.run([str(compiler),'-O2','-ffreestanding','-fno-builtin',
                '-shared','-nostdlib','-fuse-ld=lld','-Wl,/noentry','-I',str(root/'scripts/nhl27/runtime'),
                str(out/'fixture.c'),str(out/'oracle.s'),str(root/'scripts/nhl27/runtime/panic_services.c'),
                str(root/'scripts/nhl27/runtime/panic_state.c'),'-o',str(out/'test.dll')],
                capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stdout+result.stderr)
            lib=C.CDLL(str(out/'test.dll'))
            try:self.assertEqual(lib.check(),652)
            finally:C.windll.kernel32.FreeLibrary(C.c_void_p(lib._handle))


if __name__=='__main__':unittest.main()
