"""Execute the move selectors and native-ABI trampolines on fixture inputs."""
import ctypes
import math
from pathlib import Path
import subprocess
import tempfile
import unittest
import struct

import deke_skill_hooks

COMPILER=Path(r'C:\Program Files\LLVM\bin\clang.exe')
ROOT=Path(__file__).resolve().parents[2]
SCRATCH=ROOT/'local/nhl27-eboot/compatibility/deke-trial/moves-only'


class SkillSelectors(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp=tempfile.TemporaryDirectory(dir=SCRATCH)
        path=Path(cls.tmp.name)/'selectors.dll'
        support=path.with_suffix('.c');support.write_text('int _fltused=0;\n')
        command=[str(COMPILER),'-shared','-fuse-ld=lld','-nostdlib','-Wl,/noentry',
            '-O2','-ffreestanding','-fno-stack-protector','-ffp-contract=off','-fno-math-errno','-mavx',str(support),
            str(ROOT/'scripts/nhl27/runtime/deke_skill_stick.c'),
            str(ROOT/'scripts/nhl27/runtime/deke_context_binding.c'),'-o',str(path)]
        result=subprocess.run(command,capture_output=True,text=True)
        if result.returncode:raise RuntimeError(result.stderr)
        cls.lib=ctypes.CDLL(str(path))
        cls.lib.deke_high_level.argtypes=[ctypes.c_uint]+[ctypes.c_float]*4+[ctypes.c_uint]
        cls.lib.deke_toe_type.argtypes=[ctypes.c_uint,ctypes.c_uint]
        cls.lib.deke_toe_secondary.argtypes=[ctypes.c_uint,ctypes.c_uint,ctypes.c_float]
        cls.lib.deke_bank_type.argtypes=[ctypes.c_uint,ctypes.c_float]
        cls.lib.deke_bank_gameplay_type.argtypes=[ctypes.c_uint,ctypes.c_uint]
        cls.lib.deke_bank_pickup_type.argtypes=[ctypes.c_uint,ctypes.c_uint]
        cls.lib.deke_pickup_wait.argtypes=[ctypes.c_uint]*3
        cls.lib.deke_pickup_wait.restype=ctypes.c_uint
        cls.lib.deke_reverse_board_velocity.argtypes=[ctypes.POINTER(ctypes.c_float)]+[ctypes.c_float]*5
        cls.lib.deke_reverse_midice_velocity.argtypes=[ctypes.POINTER(ctypes.c_float)]*5
        cls.lib.deke_soft_drop_velocity.argtypes=[ctypes.POINTER(ctypes.c_float),
            ctypes.POINTER(ctypes.c_float),ctypes.c_float,ctypes.POINTER(ctypes.c_float)]

    def test_board_situations_use_supported_native_net_gameplay_types(self):
        for native in range(32):
            for variant in range(32):
                expected={(1,7):6,(2,8):5,(3,9):5,(4,10):6}.get((native,variant),native)
                self.assertEqual(self.lib.deke_bank_gameplay_type(native,variant),expected)
        # Repeated evaluation of native 5/6 is idempotent and cannot swap hands.
        self.assertEqual(self.lib.deke_bank_gameplay_type(6,7),6)
        self.assertEqual(self.lib.deke_bank_gameplay_type(5,8),5)

    def test_soft_drop_is_short_grounded_and_rotation_invariant(self):
        vector=ctypes.c_float*4
        for heading in range(0,360,15):
            a=math.radians(heading);forward=(math.sin(a),math.cos(a))
            position=vector(12,0,-7,0)
            for lead in (0,.2,.75,2,12):
                puck=vector(12+lead*forward[0],.1,-7+lead*forward[1],0)
                out=vector()
                self.lib.deke_soft_drop_velocity(out,position,a,puck)
                speed=math.hypot(out[0],out[2])
                self.assertLessEqual(speed,.165001)
                self.assertGreaterEqual(speed,.112499)
                self.assertAlmostEqual(speed,1.5*max(.075,min((lead+.90)/12,.11)),places=6)
                self.assertLess(out[0]*forward[0]+out[2]*forward[1],0)
                self.assertEqual((out[1],out[3]),(0,0))

    def test_pickup_wait_cap_only_applies_to_active_reversals(self):
        for native in range(12):
            for variant in range(12):
                for ticks in (0,1,5,6,7,12,24,25,60,120,0xffffffff):
                    expected=ticks
                    if (native,variant) in {(6,7),(5,8),(5,9),(6,10)} and 6<ticks<0x80000000:
                        expected=6
                    self.assertEqual(self.lib.deke_pickup_wait(native,variant,ticks),expected)

    def test_board_pickup_query_restores_handed_filter_only(self):
        for native in range(32):
            for variant in range(32):
                expected={(6,7):1,(5,8):2}.get((native,variant),native)
                self.assertEqual(self.lib.deke_bank_pickup_type(native,variant),expected)

    def test_reverse_midice_release_uses_initial_velocity_and_skate_midpoint(self):
        vector=ctypes.c_float*4
        for speed in (0,.01,.2,1,3):
            for heading in range(0,360,30):
                a=math.radians(heading)
                initial=vector(speed*math.sin(a),0,speed*math.cos(a),0)
                for offset in (-2,-.1,0,.1,2):
                    left=vector(-.2,0,0,0);right=vector(.2,0,0,0)
                    puck=vector(offset,.1,.3,0);out=vector()
                    self.lib.deke_reverse_midice_velocity(out,initial,left,right,puck)
                    norm=math.sqrt(sum(puck[i]**2 for i in range(3)))
                    expected=[-puck[i]/norm*speed*.3*.6-initial[i]*.3*.4 for i in range(3)]
                    expected[1]=0
                    length=math.hypot(expected[0],expected[2])
                    if 0<length<.12:expected=[v/length*.12 for v in expected]
                    for actual,want in zip(out,expected):self.assertAlmostEqual(actual,want,places=6)

    @classmethod
    def tearDownClass(cls):
        import _ctypes
        _ctypes.FreeLibrary(cls.lib._handle);cls.tmp.cleanup()

    def test_lateral_context_uses_geometry_and_handedness(self):
        fn=self.lib.deke_high_level
        for native in (1,3):
            for mirrored in (0,1):
                self.assertEqual(fn(native,70,10,70,1.2,mirrored),4+mirrored)
                self.assertEqual(fn(native,70,10,70,5.0,mirrored),5-mirrored)
        for native in (0,2):self.assertEqual(fn(native,70,10,70,1.2,0),native)
        for facing,velocity in [(45,10),(95,10),(70,20.01)]:
            self.assertEqual(fn(1,facing,velocity,70,1.2,0),1)
        self.assertEqual(fn(1,70,20,70,1.2,0),4)
        self.assertEqual(fn(1,0,0,130,3,0),3)

    def test_all_source_toe_types_and_secondary_variants(self):
        for high,expected in [(1,5),(2,5),(3,25),(4,26),(5,27)]:
            self.assertEqual(self.lib.deke_toe_type(high,5),expected)
        self.assertEqual(self.lib.deke_toe_type(2,15),15)
        for high in (1,2,3):
            self.assertEqual(self.lib.deke_toe_secondary(high,1,0.1),5)
            self.assertEqual(self.lib.deke_toe_secondary(high,3,-0.1),6)
            self.assertEqual(self.lib.deke_toe_secondary(high,3,0.1),3)
            self.assertEqual(self.lib.deke_toe_secondary(high,1,0),1)
            for middle in (2,4):self.assertEqual(self.lib.deke_toe_secondary(high,middle,1),middle)
        for high in (4,5):
            for secondary in range(1,5):self.assertEqual(self.lib.deke_toe_secondary(high,secondary,1),secondary)
        for type in (0,4,6,13,24):self.assertEqual(self.lib.deke_toe_type(4,type),type)

    def test_reverse_bank_threshold_and_behind_net_preserved(self):
        threshold=ctypes.c_float(3*math.pi/4).value
        for native in range(7):
            self.assertEqual(self.lib.deke_bank_type(native,threshold),native)
            self.assertEqual(self.lib.deke_bank_type(native,threshold+.001),
                native+6 if 1<=native<=4 else native)

    def test_reverse_board_release_points_back_and_into_boards(self):
        # Exercise either board, either skating direction, corners, low/high
        # speed and distance clamping. Reference uses independent trig math.
        fn=self.lib.deke_reverse_board_velocity
        count=0
        for degrees in range(0,360,15):
            normal=math.radians(degrees)
            nx,nz=math.sin(normal),math.cos(normal)
            for direction in (-1,1):
                facing=(normal+direction*math.pi/2)%(2*math.pi)
                for speed in (-.1,0,.1,.2,.4,.8):
                    for distance in (-1,0,.5,2,4,8):
                        out=(ctypes.c_float*4)(9,9,9,9)
                        fn(out,nx,nz,facing,speed,distance)
                        offset=.75+(.65*max(0,min(.4,speed))/.4)
                        # Tangential component reverses; normal points inward
                        # from boards, so a bank must have negative normal dot.
                        magnitude=.3+.1*max(0,min(4,distance))/4
                        tx,tz=math.sin(facing),math.cos(facing)
                        self.assertAlmostEqual(out[0]*tx+out[2]*tz,-magnitude*math.cos(offset),delta=1e-5)
                        self.assertAlmostEqual(out[0]*nx+out[2]*nz,-magnitude*math.sin(offset),delta=1e-5)
                        self.assertEqual((out[1],out[3]),(0,0))
                        count+=1
        self.assertEqual(count,1728)

    def test_reverse_board_source_curve_defaults(self):
        from research import ElfImage
        source=ElfImage(Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'))
        self.assertEqual(source.read(0xef20e40,32),
            struct.pack('<8f',0,4,.3,.4,0,.4,.75,1.4))
        self.assertEqual(source.read(0xef6a4e4,4),struct.pack('<f',math.pi))
        for va,value in [(0xded9b5c,.12),(0xdedb810,.3),(0xded9520,.6),
                         (0xded8d4c,2**-32),(0xded88d8,2**-16)]:
            self.assertEqual(source.read(va,4),struct.pack('<f',value))
        self.assertEqual(source.read(0xef20eb0,16),struct.pack('<4f',.6,1.1,.7,1.05))


class SkillHooks(unittest.TestCase):
    def test_bank_capture_keeps_command_time_velocity_per_actor(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH) as tmp:
            p=Path(tmp);support=p/'capture.c'
            support.write_text('''
int _fltused=0;
__attribute__((sysv_abi)) int deke_ant_bind_guest(unsigned long long base,
 void *context,unsigned long long id,void **result) {
 unsigned index=id==0xd0de771f4db32fe6ULL ? 0 : id==0x18f7fe05f9f5c2bbULL ? 1 : 2;
 if(id==0x68f19d1c486bd2f0ULL)index=3;
 *result=(unsigned*)context+index;return 0;
}
__attribute__((sysv_abi)) void deke_bank_capture_guest(unsigned long long,void*);
__declspec(dllexport) void capture(void *owner) {deke_bank_capture_guest(1,owner);}
__attribute__((sysv_abi)) void deke_toe_clear_guest(unsigned long long,void*);
__declspec(dllexport) void clear_toe(void *owner) {deke_toe_clear_guest(1,owner);}
''')
            result=subprocess.run([str(COMPILER),'-shared','-fuse-ld=lld','-nostdlib',
                '-Wl,/noentry','-O2','-ffreestanding','-fno-stack-protector',
                '-ffp-contract=off','-fno-math-errno','-mavx',str(support),
                str(ROOT/'scripts/nhl27/runtime/deke_skill_stick.c'),'-o',str(p/'capture.dll')],
                capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            lib=ctypes.CDLL(str(p/'capture.dll'));lib.capture.argtypes=[ctypes.c_void_p]
            lib.clear_toe.argtypes=[ctypes.c_void_p]
            actors=[]
            try:
                for velocity in [(1.25,-.3,-2.5),(-3,0,7)]:
                    owner=ctypes.create_string_buffer(0x100);actor=ctypes.create_string_buffer(0x240)
                    pose=ctypes.create_string_buffer(0xa0);animation=ctypes.create_string_buffer(0xc30)
                    animation_owner=ctypes.create_string_buffer(0x28);context=ctypes.create_string_buffer(0x80)
                    for buf,offset,target in [(owner,0x58,actor),(actor,0x10,pose),
                        (actor,0x60,animation),(animation,0xc28,animation_owner),(animation_owner,0x20,context)]:
                        struct.pack_into('<Q',buf,offset,ctypes.addressof(target))
                    struct.pack_into('<3f',pose,0x50,*velocity);lib.capture(owner)
                    expected=pose.raw[0x50:0x5c]
                    struct.pack_into('<I',context,0x6c,27);lib.clear_toe(owner)
                    self.assertEqual(struct.unpack_from('<I',context,0x6c)[0],0)
                    struct.pack_into('<3f',pose,0x50,9,9,9)
                    actors.append((owner,actor,pose,animation,animation_owner,context,expected))
                for *_,context,expected in actors:self.assertEqual(context.raw[0x60:0x6c],expected)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)

    def test_release_guest_changes_only_reverse_board_variants(self):
        # Execute the guest adapter against a fake actor/context with an already
        # current native board cache. Other move variants must retain all bits.
        with tempfile.TemporaryDirectory(dir=SCRATCH) as tmp:
            p=Path(tmp);support=p/'fixture.c'
            support.write_text('''
int _fltused=0;
static unsigned selected;
__attribute__((sysv_abi)) int deke_ant_bind_guest(unsigned long long base,
 void *context,unsigned long long id,void **result) {
 if(id!=0xabcc521ace57de1cULL)return 2;
 *result=&selected;return 0;
}
__attribute__((sysv_abi)) void deke_bank_release_guest(unsigned long long,void*,float*);
__declspec(dllexport) void release(unsigned long long base,void *owner,float *out,unsigned variant) {
 selected=variant;deke_bank_release_guest(base,owner,out);
}
static unsigned calls;
static void *expected_cache,*expected_position;
static unsigned mismatch;
__attribute__((sysv_abi)) unsigned char query(float radius,void *cache,void *position) {
 ++calls;
 mismatch += radius!=0.0f || cache!=expected_cache || position!=expected_position;
 return 0;
}
__attribute__((sysv_abi)) void deke_board_cache_guest(unsigned long long,void*,void*,unsigned);
__declspec(dllexport) unsigned check_cache(void *actor,void *pose) {
 expected_cache=(char*)actor+0x1d0;expected_position=(char*)pose+0x30;
 calls=0;mismatch=0;
 unsigned long long base=(unsigned long long)&query-0x2a8dbd0;
 deke_board_cache_guest(base,actor,pose,123);
 deke_board_cache_guest(base,actor,pose,123);
 deke_board_cache_guest(base,actor,pose,124);
 return mismatch + (calls!=2) + (*(unsigned*)((char*)actor+0x238)!=124);
}
''')
            result=subprocess.run([str(COMPILER),'-shared','-fuse-ld=lld','-nostdlib',
                '-Wl,/noentry','-O2','-ffreestanding','-fno-stack-protector',
                '-ffp-contract=off','-fno-math-errno','-mavx',str(support),
                str(ROOT/'scripts/nhl27/runtime/deke_skill_stick.c'),'-o',str(p/'fixture.dll')],
                capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stderr)
            lib=ctypes.CDLL(str(p/'fixture.dll'))
            try:
                owner=ctypes.create_string_buffer(0x100);actor=ctypes.create_string_buffer(0x240)
                pose=ctypes.create_string_buffer(0xa0);animation=ctypes.create_string_buffer(0xc30)
                animation_owner=ctypes.create_string_buffer(0x28);context=ctypes.create_string_buffer(0x80)
                for buf,offset,target in [(owner,0x58,actor),(actor,0x10,pose),
                    (actor,0x60,animation),(animation,0xc28,animation_owner),
                    (animation_owner,0x20,context)]:
                    struct.pack_into('<Q',buf,offset,ctypes.addressof(target))
                struct.pack_into('<f',pose,0x30,2);struct.pack_into('<f',pose,0x90,.2)
                struct.pack_into('<f',actor,0x1d4,2)
                # Include the puck global too: open-ice variants now read it
                # directly rather than first fetching captured velocity words.
                generation=ctypes.create_string_buffer(0xbae0ea8-0xba45a04+8)
                base=ctypes.addressof(generation)-0xba45a04
                lib.release.argtypes=[ctypes.c_uint64,ctypes.c_void_p,ctypes.c_void_p,ctypes.c_uint]
                for variant in range(11):
                    out=(ctypes.c_float*4)(1,2,3,4)
                    lib.release(base,owner,out,variant)
                    if variant in (7,8):
                        self.assertLess(out[0],0);self.assertLess(out[2],0)
                        self.assertEqual(out[1],0)
                    else:self.assertEqual(list(out),[1,2,3,4])
                # A stale cache exercises the native SysV call, including its
                # float argument in XMM0 and both pointer argument registers.
                lib.check_cache.argtypes=[ctypes.c_void_p,ctypes.c_void_p]
                self.assertEqual(lib.check_cache(actor,pose),0)
                puck=ctypes.create_string_buffer(0x20)
                puck_pose=ctypes.create_string_buffer(0x50)
                struct.pack_into('<Q',puck,0x10,ctypes.addressof(puck_pose))
                struct.pack_into('<Q',generation,0xbae0ea8-0xba45a04,ctypes.addressof(puck))
                struct.pack_into('<3f',puck_pose,0x30,2,.1,.25)
                for variant in (9,10):
                    out=(ctypes.c_float*4)(1,2,3,4)
                    lib.release(base,owner,out,variant)
                    self.assertAlmostEqual(out[0],0,places=6)
                    self.assertAlmostEqual(out[2],-1.5*1.15/12,places=6)
                    self.assertEqual((out[1],out[3]),(0,0))
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)

    def test_native_left_stick_decoder_matches_source_all_packed_angles(self):
        from panic_math import relocate, assembly
        from research import ElfImage, SelfImage
        source=ElfImage(Path(r'W:\Downloads\NHL 27\PPSA34063-app0\eboot.bin'))
        native=SelfImage(Path(r'F:\Games\NHL 24\game-UPDATE\eboot.bin'))
        # Source 69a9627/963f computes 1 / 255; native 500387f stores
        # the same rounded IEEE word. No live globals are guessed here.
        self.assertEqual(source.read(0xef69d8c,4),struct.pack('<f',255))
        self.assertEqual(native.read(0x500387f,10)[-4:],bytes.fromhex('8180803b'))
        reciprocal=bytes.fromhex('8180803b')
        src,_=relocate(source,0x69a8ac0,0x160,{0xef69d8c:struct.pack('<f',255),0x105508bc:reciprocal})
        dst,_=relocate(native,0x5003fb0,0x130,{0xbaa82a4:reciprocal})
        with tempfile.TemporaryDirectory(dir=SCRATCH) as tmp:
            p=Path(tmp);(p/'code.s').write_text(assembly(src,'source_decode')+assembly(dst,'native_decode'))
            (p/'fixture.c').write_text('''
int _fltused=0;
extern __attribute__((sysv_abi)) void source_decode(float*,void*,int,int);
extern __attribute__((sysv_abi)) void native_decode(float*,void*,int);
__declspec(dllexport) unsigned compare(void) {
    unsigned char packet[128]={0}; unsigned differences=0;
    for(unsigned a=0;a<256;++a) for(unsigned b=0;b<256;++b) {
        packet[0x2d]=a; packet[0x2f]=b;
        union {float f;unsigned u;} x,y;
        source_decode(&x.f,packet,1,2); native_decode(&y.f,packet,1);
        differences+=x.u!=y.u;
    }
    return differences;
}
''')
            result=subprocess.run([str(COMPILER),'-shared','-fuse-ld=lld','-nostdlib',
                '-ffreestanding','-O2','-mavx','-Wl,/noentry',str(p/'code.s'),str(p/'fixture.c'),
                '-o',str(p/'decoder.dll')],capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stderr)
            lib=ctypes.CDLL(str(p/'decoder.dll'))
            try:self.assertEqual(lib.compare(),0)
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)

    def test_original_call_arguments_return_state_and_stack_survive(self):
        # Call from Windows ABI into the actual SysV trampolines. The original
        # writes recognizable outputs; the adapter must observe the original
        # arguments after a callee that destroys volatile registers/YMM state.
        asm=deke_skill_hooks.assembly('fixture_base','native_high','native_toe','bank_done')
        asm+='''
.text
fixture_base:
.globl fixture
fixture:
 push rdi
 push rsi
 push rbx
 sub rsp, 160
'''
        asm+='\n'.join(f'vmovdqu [rsp+{16*i}], xmm{i+6}' for i in range(10))+'\n'
        asm+='''mov rbx, rcx
 mov rdi, rbx
 mov esi, 15
 mov edx, 3
 call deke_toe_hook
 mov [rbx+40], rax
 vmovd [rbx+48], xmm0
 mov rdi, rbx
 mov esi, 0x456
 call deke_high_hook
 mov [rbx+56], rax
'''
        asm+='\n'.join(f'vmovdqu xmm{i+6}, [rsp+{16*i}]' for i in range(10))+'\n'
        asm+='''add rsp, 160
 pop rbx
 pop rsi
 pop rdi
 ret
native_toe:
 mov [rdi], rsi
 mov [rdi+8], rdx
 mov rax, 0x12345678
 vmovd xmm0, eax
 xor esi, esi
 xor edx, edx
 ret
native_high:
 mov [rdi+16], rsi
 mov rax, 0x87654321
 xor edi, edi
 ret
deke_toe_guest:
 lea rax, [rip+fixture_base]
 cmp rax, rdi
 jne fail
 cmp edx, 15
 jne fail
 cmp ecx, 3
 jne fail
 mov dword ptr [rsi+24], 1
 jmp clobber
deke_high_guest:
 lea rax, [rip+fixture_base]
 cmp rax, rdi
 jne fail
 mov dword ptr [rsi+28], 1
clobber:
 mov eax, 0xdeadbeef
 vpxor ymm0, ymm0, ymm0
 xor ecx, ecx
 xor edx, edx
 xor r8d, r8d
 xor r9d, r9d
 xor r10d, r10d
 xor r11d, r11d
 ret
fail:
 mov dword ptr [rsi+32], 1
 ret
deke_bank_guest:
deke_bank_capture_guest:
deke_toe_clear_guest:
 ret
deke_bank_pickup_guest:
 cmp dword ptr [rdx], 6
 jne fail
 mov dword ptr [rdx], 1
 jmp clobber
deke_pickup_wait_guest:
 cmp dword ptr [rdx], 90
 jne fail
 mov dword ptr [rdx], 24
 jmp clobber
deke_bank_release_guest:
 mov dword ptr [rsi], 0x1234
 mov dword ptr [rdx], 0x5678
 vpxor ymm0, ymm0, ymm0
 xor edx, edx
 xor r11d, r11d
 ret
bank_done:
 ret
'''
        asm+='''
.globl bank_fixture
bank_fixture:
 push rbx
 push rbp
 push r12
 push r14
 push r15
 sub rsp, 160
'''
        asm+='\n'.join(f'vmovdqu [rsp+{16*i}], xmm{i+6}' for i in range(10))+'\n'
        asm+='''mov rbx, rcx
 mov r12, rcx
 mov r14, rcx
 lea rbp, [rcx+0x140]
 lea rax, [rcx+0x80]
 mov [rcx+0x60], rax
 mov qword ptr [rcx+0xca8], 0xabcdef
 call deke_bank_release_hook
 mov [rbx+8], rax
 lea rax, [rbx+48]
 mov dword ptr [rax], 6
 call deke_bank_pickup_hook
 mov [rbx+56], eax
 mov [rbx+60], ecx
 mov [rbp-0x128], rbx
 mov r15d, 90
 mov eax, 0x40400000
 vmovd xmm0, eax
 call deke_pickup_wait_hook
 mov [rbx+64], r15d
 vmovss [rbx+68], xmm1
 vmovss [rbx+72], xmm0
'''
        asm+='\n'.join(f'vmovdqu xmm{i+6}, [rsp+{16*i}]' for i in range(10))+'\n'
        asm+='''add rsp, 160
 pop r15
 pop r14
 pop r12
 pop rbp
 pop rbx
 ret
'''
        with tempfile.TemporaryDirectory(dir=SCRATCH) as tmp:
            p=Path(tmp);(p/'fixture.s').write_text(asm)
            result=subprocess.run([str(COMPILER),'-shared','-fuse-ld=lld','-nostdlib',
                '-mavx','-Wl,/noentry','-Wl,/export:fixture','-Wl,/export:bank_fixture',str(p/'fixture.s'),'-o',str(p/'fixture.dll')],
                capture_output=True,text=True)
            if result.returncode:raise RuntimeError(result.stderr)
            lib=ctypes.CDLL(str(p/'fixture.dll'))
            try:
                data=(ctypes.c_uint64*8)();lib.fixture.argtypes=[ctypes.c_void_p];lib.fixture(data)
                self.assertEqual(list(data),[15,3,0x456,0x100000001,0,0x12345678,0x12345678,0x87654321])
                bank=(ctypes.c_uint64*420)();lib.bank_fixture.argtypes=[ctypes.c_void_p]
                lib.bank_fixture(bank)
                self.assertEqual((bank[0],bank[1],bank[4]),(0x1234,0xabcdef,0x5678))
                self.assertEqual(bank[6],6) # The native gameplay word is untouched.
                self.assertEqual(bank[7],0xfffffffe00000001) # EAX=1, ECX=1-3.
                self.assertEqual(bank[8],0x41c0000000000018) # R15=24, XMM1=24.0.
                self.assertEqual(bank[9],0x40400000) # The frame value survives.
            finally:
                import _ctypes
                _ctypes.FreeLibrary(lib._handle)


if __name__=='__main__':unittest.main()
