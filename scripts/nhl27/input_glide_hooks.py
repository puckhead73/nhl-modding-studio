"""Port the source packed glide decoder at the two native consumer sites.

Source 45e8c20 uses an integer decoder, not IEEE F16C: exponent zero becomes
signed zero and exponent 31 stays finite. Keep this local to the glide path.
Context selection and producer scheduling are qualified separately.
"""
SITES = (0x2ecaef6, 0x2ecafef)
EXPECTED = bytes.fromhex('c4e27913c0')
GATE = 0x2ecae9e
GATE_EXPECTED = bytes.fromhex('4885ff743a')
# Source 4a1d0ca and 4a1d128 require the active byte to equal exactly one.
ACTIVE_PATCHES = (
    (0x2ecab1e, '80bb3805000000', '80bb3805000001'),
    (0x2ecab29, '0f8411030000', '0f8511030000'),
    (0x2ecae8a, '80bb3805000000', '80bb3805000001'),
    (0x2ecae91, '0f8416080000', '0f8516080000'),
)
# Target 2ecb960's third-context override writes action 219 after controller
# movement. Source 4a1b3d0 has no such override or action write; retain the
# native movement result and join its existing common tail. The preceding
# motion-record load is intentionally retained for that tail.
HUMAN_OVERRIDE_PATCH = (0x2ecca7b, '743f', 'eb3f')


def context_gate(puck_symbol=None, eligible=None, skipped=None):
    """Source 4a1d144 gate; omit the target-only third-controller override.

    RDI is the actor, R12 the selected contextual record. Both remain intact.
    RAX/flags are scratch in the replaced native selection block. Native
    2ecaedf still checks the record and its bit 2 before using either value.
    """
    load = (f'mov rax, [rip+{puck_symbol}]' if puck_symbol else
        '.set glide_puck, 0xbae0ea8\n.byte 0x48,0x8b,0x05\n.long glide_puck - . - 4')
    yes = eligible or 'glide_context_resume'
    no = skipped or 'glide_context_skip'
    suffix = '' if eligible else '.set glide_context_resume, 0x2ecaedf\n'
    suffix += '' if skipped else '.set glide_context_skip, 0x2ecb022\n'
    return f'''.intel_syntax noprefix
.text
.globl input_glide_context_gate
input_glide_context_gate:
    {load}
    mov eax, [rax+0xd8]
    cmp eax, [rdi+0x3c]
    .byte 0x0f,0x85
    .long {no} - . - 4
    xor eax, eax
    .byte 0xe9
    .long {yes} - . - 4
{suffix}'''


def decoder(name, resume):
    # Preserve flags and integer registers; leave the caller's red zone intact.
    # The replaced instruction overwrites XMM0 and clears its upper YMM half.
    return f'''.intel_syntax noprefix
.text
.globl {name}
{name}:
    lea rsp, [rsp-128]
    pushfq
    push rax
    push rcx
    vmovd eax, xmm0
    mov ecx, eax
    and ecx, 0x8000
    shl ecx, 16
    test eax, 0x7c00
    jz {name}_zero
    and eax, 0x7fff
    shl eax, 13
    add eax, 0x38000000
    or ecx, eax
{name}_zero:
    vmovd xmm0, ecx
    pop rcx
    pop rax
    popfq
    lea rsp, [rsp+128]
    {resume}
'''


def assembly():
    return context_gate() + ''.join(decoder(f'input_glide_decode_{va:x}',
        f'.set glide_resume_{va:x}, {va+len(EXPECTED)}\n'
        f'.byte 0xe9\n.long glide_resume_{va:x} - . - 4') for va in SITES)


def patches(image, symbols):
    import struct
    rows = []
    for va in SITES:
        if image.read(va, len(EXPECTED)) != EXPECTED:
            raise ValueError('Unqualified glide packed-input consumer')
        rows.append((va, EXPECTED, b'\xe9'+struct.pack('<i',
            symbols[f'input_glide_decode_{va:x}']-va-5)))
    for va, before, after in (*ACTIVE_PATCHES, HUMAN_OVERRIDE_PATCH):
        expected = bytes.fromhex(before)
        if image.read(va, len(expected)) != expected:
            raise ValueError('Unqualified glide context active-byte predicate')
        rows.append((va, expected, bytes.fromhex(after)))
    if image.read(GATE, len(GATE_EXPECTED)) != GATE_EXPECTED:
        raise ValueError('Unqualified glide context-selection gate')
    rows.append((GATE, GATE_EXPECTED, b'\xe9'+struct.pack('<i',
        symbols['input_glide_context_gate']-GATE-5)))
    return rows
