"""Extract private, hash-qualified NHL27 root-transform math for the port.

Only the pose-independent arithmetic is retained. The native job adapter owns
pose lookup, metadata and invalidation. No source engine pointers are copied.
"""
import hashlib
from oracle_fixture import assembly


def generate(image):
    # The original sine helper's guard load becomes a same-size RIP-relative
    # LEA of a private immutable guard. This avoids an absolute data relocation.
    patches = {0x4a89f: bytes.fromhex('488b1d4a31be0e'),
               0x1714e: bytes.fromhex('4c8b3d9b68c10e')}
    for site, expected in patches.items():
        if image.read(site, 7) != expected:
            raise ValueError('Source sine guard instruction changed')

    class GuardImage:
        segments = image.segments

        def read(self, address, size):
            result = bytearray(image.read(address, size))
            for site in patches:
                if address <= site + 1 < address + size:
                    result[site + 1 - address] = 0x8d
            return bytes(result)

    body = assembly(image, 'nhl27_ptm_root_math', 0x1bee05, 0x1bf0ec,
                    {0x4a890: 'ptm_vector_sine'}, {})
    prologue = '''
.intel_syntax noprefix
.text
.globl nhl27_ptm_root_math
nhl27_ptm_root_math:
push rbp
mov rbp,rsp
push r15
push r14
push r13
push r12
push rbx
sub rsp,0x48
mov r15,rdi
xor r12d,r12d
mov r13,rsi
xor ebx,ebx
'''
    body = body.replace('.text\n.globl nhl27_ptm_root_math\nnhl27_ptm_root_math:\n', prologue, 1)
    epilogue = '''
add rsp,0x48
pop rbx
pop r12
pop r13
pop r14
pop r15
pop rbp
ret
'''
    body = body.replace('.balign 16', epilogue + '.balign 16', 1)
    body += assembly(GuardImage(), 'ptm_vector_sine', 0x4a890, 0x4ad89,
                     {0x25da630: 'ptm_sine_reduce', 0xbfabea0: 'ptm_math_fail'},
                     {0xec2d9f0: 'ptm_math_guard'})
    body += assembly(image, 'ptm_sine_reduce', 0x25da630, 0x25da743, {},
                     {0xdf600d0: 'ptm_sine_reduce_table'})
    body += assembly(GuardImage(), 'nhl27_ptm_sincos', 0x17130, 0x173bb,
                     {0x25da630: 'ptm_sine_reduce', 0xbfabea0: 'ptm_math_fail'},
                     {0xec2d9f0: 'ptm_math_guard'})
    # PrimaryPath's fixed-vector tail moved from +480/+484 to +50/+54.
    path_patches = {0x280038: ('c5a2588884040000', 'c5a2588854000000'),
                    0x280062: ('c5f25e8880040000', 'c5f25e8850000000')}
    class PathImage:
        segments = image.segments
        def read(self,address,size):
            result=bytearray(image.read(address,size))
            for site,(before,after) in path_patches.items():
                if image.read(site,len(bytes.fromhex(before)))!=bytes.fromhex(before):
                    raise ValueError('Source path timing instruction changed')
                if address<=site and site+len(bytes.fromhex(after))<=address+size:
                    result[site-address:site-address+len(bytes.fromhex(after))]=bytes.fromhex(after)
            return bytes(result)
    path=assembly(PathImage(),'nhl27_ptm_sample_path',0x280022,0x28018e,{}, {})
    path=path.replace('nhl27_ptm_sample_path:\n','''nhl27_ptm_sample_path:
push rbp
mov rbp,rsp
push r14
sub rsp,0x58
lea r14,[rdi-0x9200]
mov rax,rsi
vmovaps xmm11,xmm0
vmovss [rbp-0x48],xmm0
''',1)
    path=path.replace('.balign 16','add rsp,0x58\npop r14\npop rbp\nret\n.balign 16',1)
    body+=path
    # Velocity features compare two joint/reference/root transform triplets.
    # Keep source operation order, including its special handling of a root
    # joint and reference changes. Engine pose lookup remains in the adapter.
    body += assembly(image,'nhl27_ptm_transform_inverse',0x2028e0,0x202965,{}, {})
    body += assembly(image,'nhl27_ptm_transform_compose',0x202420,0x2024e7,{}, {})
    velocity = assembly(image,'nhl27_ptm_velocity_math',0x24b36e,0x24b5ae,
        {0x2028e0:'nhl27_ptm_transform_inverse',0x202420:'nhl27_ptm_transform_compose',
         0x24b5ae:'ptm_velocity_done'},{0xec2d9f0:'ptm_math_guard'})
    setup = '''push rbp
mov rbp,rsp
push r15
push r13
push rbx
sub rsp,0x218
mov rbx,rdi
vmovss [rbp-0xc4],xmm0
mov al,[rdx]
mov [rbp-0xc6],al
mov al,[rdx+1]
mov [rbp-0xc7],al
mov al,[rdx+2]
mov [rbp-0xc8],al
'''
    for index,frame in enumerate((0x100,0x130,0x160,0x190,0x1c0,0x1f0)):
        for lane in range(0,48,16):
            setup+=f'vmovups xmm1,[rsi+{index*48+lane}]\nvmovups [rbp-{frame-lane}],xmm1\n'
    velocity=velocity.replace('nhl27_ptm_velocity_math:\n','nhl27_ptm_velocity_math:\n'+setup,1)
    velocity+='''ptm_velocity_done:
vmovups [rbx],xmm0
add rsp,0x218
pop rbx
pop r13
pop r15
pop rbp
ret
'''
    body+=velocity
    output=assembly(image,'nhl27_ptm_velocity_output_transform',0x24b659,0x24b6bd,{}, {})
    output=output.replace('nhl27_ptm_velocity_output_transform:\n',
        'nhl27_ptm_velocity_output_transform:\nvmovups xmm0,[rsi]\nmov rsi,rdx\n',1)
    output=output.replace('.balign 16','vmovups [rdi],xmm0\nret\n.balign 16',1)
    body+=output
    # Float exponent 0..255 gives at most table index 6 in the reduction path.
    table = image.read(0xdf600d0, 28)
    body += '\n.balign 16\nptm_sine_reduce_table:\n.byte ' + ','.join(map(str, table))
    body += '\n.balign 8\nptm_math_guard:\n.quad 0\nptm_math_fail:\nud2\n'
    receipt = {'ranges': [
        {'start': hex(a), 'end': hex(b), 'sha256': hashlib.sha256(image.read(a, b-a)).hexdigest()}
        for a, b in [(0x1bee05, 0x1bf0ec), (0x4a890, 0x4ad89), (0x25da630, 0x25da743),
                     (0x17130, 0x173bb), (0x280022,0x28018e),
                     (0x2028e0,0x202965),(0x202420,0x2024e7),(0x24b36e,0x24b5ae),
                     (0x24b659,0x24b6bd)]],
        'guard_patches': [{'site': hex(site), 'before': expected.hex(), 'opcode': 'mov to lea'}
                          for site, expected in patches.items()],
        'path_timing_patches':{hex(site):{'before':v[0],'after':v[1]} for site,v in path_patches.items()},
        'reduction_table': table.hex()}
    return body, receipt
