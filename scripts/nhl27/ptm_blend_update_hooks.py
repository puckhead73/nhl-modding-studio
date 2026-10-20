"""Guarded retained-snapshot setup and scope-owned query in native PTMBlend.

Behavior is still disabled until the remainder of the blend update is adapted.
"""
import struct
import input_frame_hook
from input_producer_hooks import REGISTERS
from ptm_evaluate_hooks import jump

BEGIN=0x369555
QUERY=0x3699ef
EXPECTED_BEGIN=bytes.fromhex('c5fa108fb4020000')
EXPECTED_QUERY=bytes.fromhex('ff90d8000000')
STAGES=(
    (0x36a019,'c5fa100483','speed'),
    (0x369a09,'4d8bb7c0020000','pre_child'),
    (0x369d2c,'4885db7414','finish'),
    (0x369d7f,'c5fa1085acfcffff','late'),
)
SETTERS=((0x36a470,'c5fa1187b0020000',0x310,'duration'),
         (0x36a4c0,'c5fa1187b4020000',0x314,'elapsed'))

def saved(destination,source):
    return f'mov {destination}, [r11+{(len(REGISTERS)-1-REGISTERS.index(source))*8}]'


def stages(base_symbol=None):
    code=''
    for address,raw,stage in STAGES:
        name='ptm_blend_'+stage
        code+=f'''.globl {name}_dispatch
{name}_dispatch:
push rax
movabs rax,0x37444e454c425450
cmp [r15+0x2e0],rax
pop rax
jne {name}_fallback
cmp byte ptr [r15+0x32d],1
jne {name}_fallback
'''
        if stage=='late':
            code+=jump(name+'_skip',0x369d2c)
        else:
            if stage=='speed':code+='vmovss xmm0,[rbx+rax*4]\n'
            args=(saved('rsi','r15'),)
            if stage!='finish':args+=(saved('rdx','rbp'),)
            helper='nhl24_ptm_blend_'+stage+('_guest' if stage=='pre_child' else '_frame_guest')
            code+=input_frame_hook.assembly(helper,name+'_call',base_symbol=base_symbol,
                original=name+'_done',address=address,arguments=args)
            code+=name+'_done:\n'
            if stage=='pre_child':
                # Native feature extraction borrows R13; reload the canary
                # pointer that the bypassed 369a18 instruction would restore.
                if base_symbol:code+=f'lea r13,[rip+{base_symbol}]\nmov r13,[r13+0xa87c1f8]\n'
                else:code+='.set ptm_blend_guard,0xa87c1f8\n.byte 0x4c,0x8b,0x2d\n.long ptm_blend_guard - . - 4\n'
                code+='vmovss xmm0,[rbp-0x350]\n'+jump(name+'_resume',0x369cd0)
            elif stage=='speed':code+=jump(name+'_resume',0x369a09)
            else:code+='jmp '+name+'_fallback\n'
        code+=name+'_fallback:\n'
        if stage=='finish':
            code+='test rbx,rbx\njnz ptm_blend_finish_release\n'+jump(name+'_empty',0x369d45)
            code+='ptm_blend_finish_release:\n'+jump(name+'_nonempty',0x369d31)
        else:
            code+='.byte '+','.join(str(x) for x in bytes.fromhex(raw))+'\n'
            code+=jump(name+'_original',address+len(bytes.fromhex(raw)))
    return code


def setters():
    code=''
    for address,raw,offset,field in SETTERS:
        name='ptm_blend_set_'+field
        code+=f'''.globl {name}
{name}:
push rax
movabs rax,0x37444e454c425450
cmp [rdi+0x2e0],rax
pop rax
jne {name}_native
cmp byte ptr [rdi+0x32d],1
jne {name}_native
vmovss [rdi+{offset}],xmm0
{name}_native:
'''
        code+='.byte '+','.join(str(x) for x in bytes.fromhex(raw))+'\n'
        code+=jump(name+'_original',address+len(bytes.fromhex(raw)))
    return code

def assembly(base_symbol=None):
    code='''.intel_syntax noprefix
.text
.globl ptm_blend_begin_dispatch
ptm_blend_begin_dispatch:
push rax
movabs rax,0x37444e454c425450
cmp [rdi+0x2e0],rax
pop rax
jne ptm_blend_begin_fallback
cmp byte ptr [rdi+0x32d],1
jne ptm_blend_begin_fallback
jmp ptm_blend_begin_call
'''
    code+=input_frame_hook.assembly('nhl24_ptm_blend_begin_frame_guest','ptm_blend_begin_call',
        base_symbol=base_symbol,original='ptm_blend_begin_done',address=BEGIN,
        arguments=(saved('rsi','rdi'),saved('rdx','rsi'),saved('rcx','rbp')))
    code+='''ptm_blend_begin_done:
mov r15,rdi
mov r14,rsi
mov r12,[rbp-0x360]
'''+jump('ptm_blend_begin_resume',0x369768)
    code+='ptm_blend_begin_fallback:\n.byte '+','.join(str(x) for x in EXPECTED_BEGIN)+'\n'
    code+=jump('ptm_blend_begin_original',BEGIN+len(EXPECTED_BEGIN))
    code+='''.globl ptm_blend_query_dispatch
ptm_blend_query_dispatch:
push r10
movabs r10,0x37444e454c425450
cmp [r15+0x2e0],r10
pop r10
jne ptm_blend_query_original
cmp byte ptr [r15+0x32d],1
jne ptm_blend_query_original
sub rsp,0x58
mov [rsp],rsi
mov [rsp+8],rdx
mov [rsp+0x10],rcx
mov [rsp+0x18],r8
mov [rsp+0x20],r9
mov [rsp+0x40],rdi
mov [rsp+0x48],rax
mov r10d,[rsp+0x60]
mov [rsp+0x28],r10d
mov r10d,[rsp+0x68]
mov [rsp+0x2c],r10d
mov r10d,[rsp+0x70]
mov [rsp+0x30],r10d
vmovss [rsp+0x34],xmm0
vmovss [rsp+0x38],xmm1
vmovss [rsp+0x3c],xmm2
mov rsi,r15
mov rdx,rsp
'''
    if base_symbol:code+=f'lea rdi,[rip+{base_symbol}]\n'
    else:code+=f'.set ptm_blend_query_site,{QUERY}\n.byte 0x48,0x8d,0x3d\n.long ptm_blend_query_site - . - 4\nsub rdi,{QUERY}\n'
    code+='''call nhl24_ptm_blend_query_guest
test eax,eax
jz ptm_blend_query_restore
add rsp,0x58
ret
ptm_blend_query_restore:
mov rsi,[rsp]
mov rdx,[rsp+8]
mov rcx,[rsp+0x10]
mov r8,[rsp+0x18]
mov r9,[rsp+0x20]
mov rdi,[rsp+0x40]
mov rax,[rsp+0x48]
vmovss xmm0,[rsp+0x34]
vmovss xmm1,[rsp+0x38]
vmovss xmm2,[rsp+0x3c]
add rsp,0x58
ptm_blend_query_original:
jmp qword ptr [rax+0xd8]
'''
    return code+stages(base_symbol)+setters()

def patches(image,symbols):
    rows=[]
    sites=[(BEGIN,EXPECTED_BEGIN,'ptm_blend_begin_dispatch',b'\xe9'),
            (QUERY,EXPECTED_QUERY,'ptm_blend_query_dispatch',b'\xe8')]
    sites.extend((a,bytes.fromhex(raw),'ptm_blend_'+s+'_dispatch',b'\xe9') for a,raw,s in STAGES)
    sites.extend((a,bytes.fromhex(raw),'ptm_blend_set_'+s,b'\xe9') for a,raw,offset,s in SETTERS)
    for address,before,name,opcode in sites:
        if image.read(address,len(before))!=before:raise ValueError(f'PTMBlend update changed: {address:x}')
        rows.append((address,before,opcode+struct.pack('<i',symbols[name]-address-5)+b'\x90'*(len(before)-5)))
    return rows
