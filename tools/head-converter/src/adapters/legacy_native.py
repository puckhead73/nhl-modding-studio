"""Legacy (Xbox 360 RX2) native-file readers used to qualify Codex's writer output INDEPENDENTLY of the Rust
library that wrote it: half-float position readback per record with stored bits, byte-change audits against a
whitelist, the XGAddress2DTiledOffset element tiling, DXT1 block decoding, and the packed-mip-tail search. Pure
numpy; nothing here writes a native file.

Layout discovery reuses the file-table walker `experiments.codex.legacy_diagnostic.layouts` (labelled reuse: it is a
Python re-derivation of the descriptor chain, not the Rust parser). Everything else is derived here from the
bytes.
"""
import struct
import numpy as np


# ---- positions --------------------------------------------------------------------------------------------------
def half_bits_to_f64(bits):
    """IEEE half bits (uint16 array) -> exact float64 values (same reader rule as the native parser)."""
    h = np.asarray(bits, np.uint16).astype(np.int64); s = (h >> 15) & 1; e = (h >> 10) & 0x1F; m = h & 0x3FF
    v = np.where(e == 0, (m / 1024.0) * 2.0 ** -14, np.where(e == 31, np.inf, (1.0 + m / 1024.0) * 2.0 ** (e - 15)))
    return np.where(s != 0, -v, v)


def read_positions(raw, layout):
    """Every record's stored XYZ from the bytes: (values float64 [n,3], bits uint16 [n,3], byte offsets [n])."""
    n = layout['count']; st = layout['stride']; base = layout['start'] + layout['position_offset']
    offs = base + np.arange(n) * st; bits = np.zeros((n, 3), np.uint16)
    for k in range(3):
        bits[:, k] = [struct.unpack_from('>H', raw, int(o) + 2 * k)[0] for o in offs]
    return half_bits_to_f64(bits), bits, offs


def compare_positions_exact(actual_values, actual_bits, predicted_values, candidate_values, component):
    """Exact (no epsilon) comparison of stored positions against a prediction. Returns mismatch rows with component,
    record, axis, candidate float, stored bits, predicted and actual values. Empty list = exact parity."""
    A = np.asarray(actual_values, np.float64); Pd = np.asarray(predicted_values, np.float64); C = np.asarray(candidate_values, np.float64); rows = []
    for r, k in zip(*np.nonzero(A != Pd)):
        rows.append({'component': component, 'record': int(r), 'axis': 'xyz'[k], 'candidate_float': float(C[r, k]), 'stored_bits': f'0x{int(actual_bits[r, k]):04x}', 'predicted': float(Pd[r, k]), 'actual': float(A[r, k]), 'difference_mm': float((A[r, k] - Pd[r, k]) * 1000)})
    return rows


def position_byte_mask(raw_len, layouts, roles, allowed_roles):
    """Boolean mask of bytes that a positions-only write is allowed to change (XYZ of the allowed roles only)."""
    m = np.zeros(raw_len, bool)
    for lay, role in zip(layouts, roles):
        if role not in allowed_roles: continue
        for j in range(lay['count']):
            o = lay['start'] + j * lay['stride'] + lay['position_offset']; m[o:o + 6] = True
    return m


def shading_byte_mask(raw_len, layout, offsets=(8, 12)):
    m = np.zeros(raw_len, bool)
    for j in range(layout['count']):
        for off in offsets:
            o = layout['start'] + j * layout['stride'] + off; m[o:o + 4] = True
    return m


def byte_change_audit(before, after, allowed_mask):
    """Changed bytes, changed bytes outside the whitelist (with their offsets), and the whitelist size."""
    a = np.frombuffer(before, np.uint8); b = np.frombuffer(after, np.uint8)
    if len(a) != len(b): return {'same_length': False, 'before': len(a), 'after': len(b)}
    changed = a != b; outside = changed & ~allowed_mask
    return {'same_length': True, 'changed_bytes': int(changed.sum()), 'changed_outside_whitelist': int(outside.sum()), 'outside_offsets_first_16': np.flatnonzero(outside)[:16].tolist(), 'whitelist_bytes': int(allowed_mask.sum())}


def unpack_snorm10(words):
    """Independent signed 10-bit XYZ extraction (x bits 0-9, y 10-19, z 20-29) -> float [n,3] and top-2 bits."""
    w = np.asarray(words, np.int64); out = np.zeros((len(w), 3))
    for k, s in enumerate((0, 10, 20)):
        q = (w >> s) & 1023; out[:, k] = np.where(q & 512, q - 1024, q) / 511.0
    return out, (w >> 30) & 3


def read_words(raw, layout, offset):
    return np.array([struct.unpack_from('>I', raw, layout['start'] + i * layout['stride'] + offset)[0] for i in range(layout['count'])], np.int64)


# ---- tiling -----------------------------------------------------------------------------------------------------
def xg_address_2d_tiled(x, y, width, texel_pitch):
    """Port of Studio texlib.rs::xg_address_2d_tiled (Microsoft XGAddress2DTiledOffset) for element pitch 8/16."""
    aligned_w = (width + 31) & ~31
    log_bpp = (texel_pitch >> 2) + ((texel_pitch >> 1) >> (texel_pitch >> 2))
    macro_ = ((x >> 5) + (y >> 5) * (aligned_w >> 5)) << (log_bpp + 7)
    micro = ((x & 7) + ((y & 6) << 2)) << log_bpp
    offset = macro_ + ((micro & ~0xF) << 1) + (micro & 0xF) + ((y & 8) << (3 + log_bpp)) + ((y & 1) << 4)
    return ((offset & ~0x1FF) << 3) + ((y & 16) << 7) + ((offset & 0x1C0) << 2) + (((((y & 8) >> 2) + (x >> 3)) & 3) << 6) + (offset & 0x3F)


def untile(tiled, grid_w, grid_h, pitch, width_for_addressing=None):
    """Tiled element stream -> linear row-major elements over grid_w x grid_h. `width_for_addressing` lets a
    sub-rectangle be read out of a larger tile (packed mip tail: the addressing width is the tile's, 32 blocks)."""
    gw = max(grid_w, 1); gh = max(grid_h, 1); aw = gw if width_for_addressing is None else width_for_addressing
    out = bytearray(gw * gh * pitch)
    for y in range(gh):
        for x in range(gw):
            src = xg_address_2d_tiled(x, y, aw, pitch); dst = (y * gw + x) * pitch
            if src + pitch <= len(tiled): out[dst:dst + pitch] = tiled[src:src + pitch]
    return bytes(out)


def untile_region(tiled, x0, y0, grid_w, grid_h, pitch, width_for_addressing):
    """Linear elements of the sub-rectangle [x0, x0+grid_w) x [y0, y0+grid_h) of a tiled buffer addressed with the
    given width (used to read a packed level at a candidate block origin)."""
    out = bytearray(grid_w * grid_h * pitch)
    for y in range(grid_h):
        for x in range(grid_w):
            src = xg_address_2d_tiled(x0 + x, y0 + y, width_for_addressing, pitch); dst = (y * grid_w + x) * pitch
            if src + pitch <= len(tiled): out[dst:dst + pitch] = tiled[src:src + pitch]
    return bytes(out)


def swap16(buf):
    b = bytearray(buf); b[0::2], b[1::2] = buf[1::2], buf[0::2]; return bytes(b)


# ---- BC1 --------------------------------------------------------------------------------------------------------
def decode_bc1(linear, width, height):
    """Linear DXT1 block stream (little-endian words, DDS order) -> RGB uint8 [height, width, 3] and alpha mask.
    Independent of the PCA encoder and of Blender's DDS loader."""
    bw = max((width + 3) // 4, 1); bh = max((height + 3) // 4, 1); blocks = np.frombuffer(linear, np.uint8)[:bw * bh * 8].reshape(-1, 8)
    c0 = blocks[:, 0].astype(np.uint32) | (blocks[:, 1].astype(np.uint32) << 8); c1 = blocks[:, 2].astype(np.uint32) | (blocks[:, 3].astype(np.uint32) << 8)
    bits = blocks[:, 4].astype(np.uint32) | (blocks[:, 5].astype(np.uint32) << 8) | (blocks[:, 6].astype(np.uint32) << 16) | (blocks[:, 7].astype(np.uint32) << 24)
    def rgb(c): return np.stack([((c >> 11) & 31) * 255 // 31, ((c >> 5) & 63) * 255 // 63, (c & 31) * 255 // 31], 1).astype(np.int32)
    a = rgb(c0); b = rgb(c1); four = c0 > c1
    p2 = np.where(four[:, None], (2 * a + b) // 3, (a + b) // 2); p3 = np.where(four[:, None], (a + 2 * b) // 3, 0)
    pal = np.stack([a, b, p2, p3], 1); alpha_pal = np.stack([np.ones(len(a)), np.ones(len(a)), np.ones(len(a)), four.astype(float)], 1)
    idx = np.stack([(bits >> (2 * i)) & 3 for i in range(16)], 1)
    tex = pal[np.arange(len(pal))[:, None], idx]; al = alpha_pal[np.arange(len(pal))[:, None], idx]
    img = tex.reshape(bh, bw, 4, 4, 3).transpose(0, 2, 1, 3, 4).reshape(bh * 4, bw * 4, 3)[:height, :width].astype(np.uint8)
    am = al.reshape(bh, bw, 4, 4).transpose(0, 2, 1, 3).reshape(bh * 4, bw * 4)[:height, :width]
    return img, am


def box_downsample(rgb, dim):
    """Mean box filter of a square RGB image to dim x dim (reference for mip-level hypotheses)."""
    h, w, _ = rgb.shape; k = h // dim; return rgb.reshape(dim, k, dim, k, 3).mean((1, 3))


def packed_tail_search(tile_swapped, donor_rgb, dims=(16, 8, 4, 2, 1), tile_blocks=32, pitch=8):
    """Search one tiled DXT1 macro tile (tile_blocks x tile_blocks blocks, already byte-swapped back) for the small
    mip levels of `donor_rgb` (the decoded base). For each level dimension the level occupies ceil(dim/4)^2 blocks;
    every block origin (bx, by) inside the tile is untiled with the TILE's addressing width and decoded, and the mean
    absolute RGB error against the box-downsampled donor is recorded. Returns per-level best/second-best origins
    and errors. Evidence for a layout hypothesis, never a layout proof by itself."""
    results = []
    for d in dims:
        nb = max((d + 3) // 4, 1); exp = box_downsample(donor_rgb.astype(float), d); best = []
        for by in range(0, tile_blocks - nb + 1):
            for bx in range(0, tile_blocks - nb + 1):
                lin = untile_region(tile_swapped, bx, by, nb, nb, pitch, tile_blocks); img, _ = decode_bc1(lin, nb * 4, nb * 4)
                mae = float(np.abs(img[:d, :d].astype(float) - exp).mean()); best.append((mae, bx, by))
        best.sort(); results.append({'dimension': d, 'blocks': nb, 'best': {'mae': best[0][0], 'block_origin': [best[0][1], best[0][2]]}, 'second': {'mae': best[1][0], 'block_origin': [best[1][1], best[1][2]]} if len(best) > 1 else None, 'origins_tested': len(best)})
    return results
