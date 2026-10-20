"""Scope-gated velocity interface thunk; unregistered queries retain native dispatch."""
import struct
ADDRESS=0x332170
EXPECTED=bytes.fromhex('4883c7f8e9d7f8ffff')
NAME='ptm_velocity_dispatch'

def assembly():
    return f'''.intel_syntax noprefix
.text
.globl {NAME}
{NAME}:
push rdi
push rsi
sub rsp,8
mov rdx,rsi
lea rsi,[rdi-8]
.set {NAME}_site, {ADDRESS}
.byte 0x48,0x8d,0x3d
.long {NAME}_site - . - 4
sub rdi,{ADDRESS}
call nhl24_ptm_velocity_dispatch_guest
test eax,eax
lea rsp,[rsp+8]
pop rsi
pop rdi
jz {NAME}_native
ret
{NAME}_native:
add rdi,-8
.set {NAME}_original, 0x331a50
.byte 0xe9
.long {NAME}_original - . - 4
'''

def patches(image,symbols):
    if image.read(ADDRESS,len(EXPECTED))!=EXPECTED:raise ValueError('Velocity interface thunk changed')
    return [(ADDRESS,EXPECTED,b'\xe9'+struct.pack('<i',symbols[NAME]-ADDRESS-5)+b'\x90'*4)]
