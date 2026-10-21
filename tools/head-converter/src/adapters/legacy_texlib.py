"""Read-only Legacy (Xbox 360 RX2) texlib walker and packed-mip layout (numpy only; nothing here writes a file).

Container walk mirrors Studio `tdb-rx2/src/texlib.rs::parse` (file table at 0x30, 24-byte entries, tags 00010031 data /
00020003 descriptor / 00ec0010 names, hdr_size at 0x44, descriptor field block at +33) and is validated against
Codex's `legacy_diagnostic.exe` receipt for texlib_67 by the tests. The 56-byte descriptor wraps a Xenos
`xe_gpu_texture_fetch_t` at +28 (six big-endian dwords); the fields used here follow the reference layout in
`nhl-legacy-recomp/docs/nhl12-decomp-reference/gpu/xenos.h` (sha256 5cab1758…): dword_0 pitch(+22, 9b) tiled(+31);
dword_1 format(+0, 6b) endianness(+6) base_address(+12, 20b, >>12); dword_2 width-1 (13b) height-1 (13b);
dword_4 mip_min_level(+2, 4b) mip_max_level(+6, 4b); dword_5 dimension(+9, 2b) packed_mips(+11) mip_address(+12,
20b, >>12).

Packed-mip layout follows `texture_util.cc::GetPackedMipOffset` (sha256 85db9fae…) of the same reference: levels
whose smaller dimension is <= 16 texels share ONE 32x32-block tile; for a square texture the 16/8/4 levels sit at
texel x = 16 >> packed_mip, y = 0 and the 2/1 levels at x = 0, y = 16 >> (packed_mip - 2). Larger levels are each
tiled in their own 32-block-aligned buffer, sequentially after the base.
"""
import struct
import numpy as np
from . import legacy_native as LN

MAGIC = b'\x89RW4xb2'; TAG_DATA = b'\x00\x01\x00\x31'; TAG_DESC = b'\x00\x02\x00\x03'; TAG_NAMES = b'\x00\xec\x00\x10'
FORMATS = {0x52: ('Dxt1', 8), 0x53: ('Dxt3', 16), 0x54: ('Dxt5', 16), 0x86: ('Argb8888', 4)}


def _u32(raw, o): return struct.unpack_from('>I', raw, o)[0]


def _names(raw, start):
    out = []; i = start; ok = lambda b: 32 < b < 127
    while i < len(raw):
        if not ok(raw[i]): i += 1; continue
        s = i
        while i < len(raw) and ok(raw[i]): i += 1
        tok = raw[s:i]
        if any(tok.endswith(sfx) for sfx in (b'.Raster', b'.Texture', b'.RasterData', b'.RasterTexture')): out.append(tok.decode('ascii'))
    return out


def fetch_constant(raw, desc_off):
    dw = [_u32(raw, desc_off + 28 + 4 * k) for k in range(6)]
    return {'dwords_hex': [f'{d:08x}' for d in dw], 'tiled': bool(dw[0] >> 31), 'pitch_texels': ((dw[0] >> 22) & 0x1FF) << 5, 'format_byte': dw[1] & 0x3F, 'endianness': (dw[1] >> 6) & 3, 'base_address': (dw[1] >> 12) << 12,
            'width': (dw[2] & 0x1FFF) + 1, 'height': ((dw[2] >> 13) & 0x1FFF) + 1, 'mip_min_level': (dw[4] >> 2) & 0xF, 'mip_max_level': (dw[4] >> 6) & 0xF, 'dimension': (dw[5] >> 9) & 3, 'packed_mips': bool((dw[5] >> 11) & 1), 'mip_address': (dw[5] >> 12) << 12}


def parse(raw):
    """Textures of a texlib: name, dims, format, absolute slot range, descriptor offset, decoded fetch constant."""
    if raw[:7] != MAGIC: raise ValueError('not an RW4 xb2 container')
    count = _u32(raw, 0x20); table = _u32(raw, 0x30); hdr = _u32(raw, 0x44); data_size = _u32(raw, 0x54)
    if data_size == 0: hdr = 0
    entries = [(struct.unpack_from('>5I', raw, table + 24 * i), raw[table + 24 * i + 20: table + 24 * i + 24]) for i in range(count)]
    data = [e for e, t in entries if t == TAG_DATA]; desc = [e for e, t in entries if t == TAG_DESC]; names = [e[0] for e, t in entries if t == TAG_NAMES]
    if len(data) != len(desc): raise ValueError(f'orphan data/descriptor entries: {len(data)} vs {len(desc)}')
    nm = _names(raw, names[0]) if names else []; out = []
    for k, (d, s) in enumerate(zip(data, desc)):
        doff = s[0]; fmt = raw[doff + 35]; hb = raw[doff + 37]; ww = struct.unpack_from('>H', raw, doff + 38)[0]
        if fmt not in FORMATS: continue
        w = (ww + 1) & 0x1FFF; h = (hb + 1) * 8; name, bpb = FORMATS[fmt]; start = hdr + d[0]
        base = LN_tiled_size(w, h, bpb) if name != 'Argb8888' else w * h * 4
        out.append({'index': k, 'name': nm[k] if k < len(nm) else f'texture_{k:04}', 'width': w, 'height': h, 'format': name, 'bytes_per_block': bpb, 'descriptor_offset': doff, 'data_relative_offset': d[0], 'slot_start': start, 'slot_bytes': d[2], 'base_tiled_bytes': base, 'hdr_size': hdr, 'fetch': fetch_constant(raw, doff)})
    return out


def LN_tiled_size(w, h, bpb):
    bw = max((w + 3) // 4, 1); bh = max((h + 3) // 4, 1); return ((bw + 31) & ~31) * ((bh + 31) & ~31) * bpb


def base_linear(raw, tex):
    """Linear DXT block stream of the base level (swap16 + untile), as the Rust parser produces."""
    w, h, bpb = tex['width'], tex['height'], tex['bytes_per_block']; bw = max(w // 4, 1); bh = max(h // 4, 1)
    return LN.untile(LN.swap16(raw[tex['slot_start']:tex['slot_start'] + tex['base_tiled_bytes']]), bw, bh, bpb)


# ---- packed layout ------------------------------------------------------------------------------------------------
def level_dims(w, h, level): return max(w >> level, 1), max(h >> level, 1)


def packed_mip_offset(w, h, level):
    """Xenia GetPackedMipOffset for 2D DXT (block units). Returns None when the level is not packed."""
    lw = int(np.log2(w)); lh = int(np.log2(h)); ls = min(lw, lh)
    if ls > 4 + level: return None
    base = ls - 4 if ls > 4 else 0; pm = level - base
    if pm < 3: x, y = (0, 16 >> pm) if lw > lh else (16 >> pm, 0)
    else:
        off = (1 << ((lw if lw > lh else lh) - base)) >> (pm - 2); x, y = (off, 0) if lw > lh else (0, off)
    return x >> 2, y >> 2      # texels -> DXT blocks


def slot_layout(w, h, bpb, levels):
    """Byte layout of a slot holding `levels` mip levels (0 = base): for each level {level, dim, offset, tiled_bytes,
    packed, block_origin, addressing_blocks}. Unpacked levels occupy their own 32-aligned tiled buffer sequentially;
    all packed levels share one 32x32-block tile that starts where the first packed level would."""
    out = []; off = 0; packed_tile = None
    for lv in range(levels):
        lw, lh = level_dims(w, h, lv); po = packed_mip_offset(lw if lv == 0 else w, lh if lv == 0 else h, lv)
        if po is None:
            size = LN_tiled_size(lw, lh, bpb); out.append({'level': lv, 'dim': [lw, lh], 'offset': off, 'tiled_bytes': size, 'packed': False, 'block_origin': [0, 0], 'addressing_blocks': (max(lw // 4, 1) + 31) & ~31}); off += size
        else:
            if packed_tile is None: packed_tile = off; off += 32 * 32 * bpb
            out.append({'level': lv, 'dim': [lw, lh], 'offset': packed_tile, 'tiled_bytes': 32 * 32 * bpb, 'packed': True, 'block_origin': list(po), 'addressing_blocks': 32})
    return {'levels': out, 'total_bytes': off}


def extract_level_blocks(slot, lay_level, bpb):
    """Original compressed blocks of one level (no decode): swap16 the level's tile, read the level's block rectangle at
    its origin with the tile's addressing width. Returns the linear block stream (row-major)."""
    lw, lh = lay_level['dim']; nb_w = max(lw // 4, 1); nb_h = max(lh // 4, 1); o = lay_level['offset']
    tile = LN.swap16(slot[o:o + lay_level['tiled_bytes']]); bx, by = lay_level['block_origin']
    return LN.untile_region(tile, bx, by, nb_w, nb_h, bpb, lay_level['addressing_blocks'])


def level_block_addresses(lay_level, bpb):
    """Absolute byte ranges inside the slot that one level's blocks occupy (for coverage/overlap proofs)."""
    lw, lh = lay_level['dim']; nb_w = max(lw // 4, 1); nb_h = max(lh // 4, 1); bx, by = lay_level['block_origin']; o = lay_level['offset']; aw = lay_level['addressing_blocks']
    return [(o + LN.xg_address_2d_tiled(bx + x, by + y, aw, bpb), bpb) for y in range(nb_h) for x in range(nb_w)]


def write_level_blocks(slot, lay_level, bpb, linear_blocks):
    """Place a level's linear block stream into `slot` (bytearray) at its tiled addresses, byte-swapped as stored. Only
    the level's own block addresses are touched; everything else in the slot is left as it is."""
    lw, lh = lay_level['dim']; nb_w = max(lw // 4, 1); nb_h = max(lh // 4, 1)
    if len(linear_blocks) != nb_w * nb_h * bpb: raise ValueError(f'level {lay_level["level"]}: expected {nb_w * nb_h * bpb} bytes, got {len(linear_blocks)}')
    for k, (addr, n) in enumerate(level_block_addresses(lay_level, bpb)):
        slot[addr:addr + n] = LN.swap16(linear_blocks[k * bpb:(k + 1) * bpb])
    return slot
