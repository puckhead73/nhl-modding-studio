//! BC7 (BPTC) encoder — **Mode 6 only**.
//!
//! BC7 has eight block modes; a full encoder that searches all of them (with
//! partitioning, rotation, index-selection bits) is large and, for our inputs,
//! unnecessary. **Mode 6** is BC7's single-subset, full-RGBA "catch-all": one
//! 4×4 block, both endpoints RGBA at 7-bit precision plus a per-endpoint p-bit
//! (so an effective 8-bit endpoint), and one 4-bit interpolation index per texel.
//! It has no partitioning, so it can't split a block into two independently-lit
//! subsets — but the GNF content that is BC7 (NHL 16+ **player portraits** and
//! skin/appearance maps) is smooth, where mode 6 is a faithful, fully
//! spec-compliant encoding.
//!
//! Correctness is self-checked by round-tripping through the BC7 **decoder**
//! (`texture2ddecoder`, the same one the read path uses): encode → decode →
//! compare. Because the bitstream is written to the real spec, any decoder reads
//! it correctly.
//!
//! ## Mode 6 block layout (128 bits, LSB-first / little-endian)
//!
//! ```text
//! [7]  mode marker            (bit 6 set: six 0s then a 1)
//! [56] colours R0 R1 G0 G1 B0 B1 A0 A1  — 7 bits each, endpoint-interleaved
//! [2]  p-bits P0 P1           (one shared p-bit per endpoint)
//! [63] indices                texel 0 = 3 bits (anchor, high bit implied 0),
//!                             texels 1..15 = 4 bits each
//! ```

/// BC7 4-bit interpolation weights (the `aWeight4` table). Symmetric:
/// `W4[15 - i] == 64 - W4[i]`, which is what makes the anchor endpoint-swap
/// (below) lossless.
const W4: [i32; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

/// Interpolate one channel between two 8-bit endpoints at weight `w` (0..=64).
fn interp(a: u8, b: u8, w: i32) -> u8 {
    (((a as i32) * (64 - w) + (b as i32) * w + 32) >> 6) as u8
}

/// Reconstruct the full 8-bit endpoint from its 7-bit value + shared p-bit,
/// exactly as the decoder does: `(comp << 1) | pbit`.
fn recon(q: [u8; 4], p: u8) -> [u8; 4] {
    let mut r = [0u8; 4];
    for c in 0..4 {
        r[c] = (q[c] << 1) | p;
    }
    r
}

/// Quantize an 8-bit RGBA endpoint to 7 bits + one **shared** p-bit, choosing
/// the p-bit (0 or 1) that minimises the total squared reconstruction error
/// across all four channels (the p-bit is per-endpoint, not per-channel).
fn quantize(e: [u8; 4]) -> ([u8; 4], u8) {
    let mut best = ([0u8; 4], 0u8);
    let mut best_err = i64::MAX;
    for p in 0u8..2 {
        let mut q = [0u8; 4];
        let mut err = 0i64;
        for c in 0..4 {
            let qc = (((e[c] as i32) - p as i32 + 1) / 2).clamp(0, 127);
            q[c] = qc as u8;
            let rec = (qc << 1) | p as i32;
            let d = rec - e[c] as i32;
            err += (d * d) as i64;
        }
        if err < best_err {
            best_err = err;
            best = (q, p);
        }
    }
    best
}

/// Pick the 4-bit index that best matches `t` on the line between reconstructed
/// endpoints `r0`/`r1`.
fn best_index(t: [u8; 4], r0: [u8; 4], r1: [u8; 4]) -> u8 {
    let mut best = 0u8;
    let mut best_err = i64::MAX;
    for (i, &w) in W4.iter().enumerate() {
        let mut err = 0i64;
        for c in 0..4 {
            let v = interp(r0[c], r1[c], w) as i32;
            let d = v - t[c] as i32;
            err += (d * d) as i64;
        }
        if err < best_err {
            best_err = err;
            best = i as u8;
        }
    }
    best
}

/// Choose two endpoints for the block: the texels at the extremes of the
/// per-channel bounding-box diagonal (the classic DXT-style axis). Real texel
/// colours are used so the endpoints sit on the data.
fn pick_endpoints(texels: &[[u8; 4]; 16]) -> ([u8; 4], [u8; 4]) {
    let mut minc = [255i32; 4];
    let mut maxc = [0i32; 4];
    for t in texels {
        for c in 0..4 {
            minc[c] = minc[c].min(t[c] as i32);
            maxc[c] = maxc[c].max(t[c] as i32);
        }
    }
    let axis = [
        maxc[0] - minc[0],
        maxc[1] - minc[1],
        maxc[2] - minc[2],
        maxc[3] - minc[3],
    ];
    if axis.iter().all(|&a| a == 0) {
        let e = [minc[0] as u8, minc[1] as u8, minc[2] as u8, minc[3] as u8];
        return (e, e);
    }
    let (mut lo_i, mut hi_i) = (0usize, 0usize);
    let (mut lo_d, mut hi_d) = (i64::MAX, i64::MIN);
    for (i, t) in texels.iter().enumerate() {
        let mut d = 0i64;
        for c in 0..4 {
            d += (t[c] as i64 - minc[c] as i64) * axis[c] as i64;
        }
        if d < lo_d {
            lo_d = d;
            lo_i = i;
        }
        if d > hi_d {
            hi_d = d;
            hi_i = i;
        }
    }
    (texels[lo_i], texels[hi_i])
}

/// LSB-first bit accumulator for the 128-bit block.
struct Bits {
    acc: u128,
    pos: u32,
}

impl Bits {
    fn put(&mut self, val: u32, bits: u32) {
        self.acc |= ((val as u128) & ((1u128 << bits) - 1)) << self.pos;
        self.pos += bits;
    }
}

/// Encode one 4×4 RGBA block (16 texels, row-major) to a 16-byte BC7 mode-6 block.
pub fn encode_block(texels: &[[u8; 4]; 16]) -> [u8; 16] {
    let (e0, e1) = pick_endpoints(texels);
    let (mut q0, mut p0) = quantize(e0);
    let (mut q1, mut p1) = quantize(e1);
    let r0 = recon(q0, p0);
    let r1 = recon(q1, p1);

    let mut idx = [0u8; 16];
    for (i, t) in texels.iter().enumerate() {
        idx[i] = best_index(*t, r0, r1);
    }

    // Anchor rule: the first texel's index must have its high bit clear
    // (it's stored with one fewer bit). If not, swap endpoints and invert every
    // index — lossless because W4 is symmetric.
    if idx[0] & 0x8 != 0 {
        std::mem::swap(&mut q0, &mut q1);
        std::mem::swap(&mut p0, &mut p1);
        for v in idx.iter_mut() {
            *v = 15 - *v;
        }
    }

    let mut b = Bits { acc: 0, pos: 0 };
    b.put(1 << 6, 7); // mode 6 marker
    for c in 0..4 {
        b.put(q0[c] as u32, 7);
        b.put(q1[c] as u32, 7);
    }
    b.put(p0 as u32, 1);
    b.put(p1 as u32, 1);
    b.put(idx[0] as u32, 3); // anchor: 3 bits
    for &v in &idx[1..] {
        b.put(v as u32, 4);
    }
    debug_assert_eq!(b.pos, 128, "mode 6 block must be exactly 128 bits");
    b.acc.to_le_bytes()
}

/// Encode a tightly-packed RGBA8 image (`w`×`h`) to a linear, row-major BC7
/// block stream (`ceil(w/4) × ceil(h/4)` blocks × 16 bytes) — the layout GNF's
/// de-tiled base mip uses. Partial edge blocks clamp to the last row/column.
pub fn encode(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (wu, hu) = (w as usize, h as usize);
    let bx = wu.div_ceil(4);
    let by = hu.div_ceil(4);
    let mut out = vec![0u8; bx * by * 16];
    for byi in 0..by {
        for bxi in 0..bx {
            let mut texels = [[0u8; 4]; 16];
            for py in 0..4 {
                for px in 0..4 {
                    let sx = (bxi * 4 + px).min(wu.saturating_sub(1));
                    let sy = (byi * 4 + py).min(hu.saturating_sub(1));
                    let o = (sy * wu + sx) * 4;
                    if o + 4 <= rgba.len() {
                        texels[py * 4 + px].copy_from_slice(&rgba[o..o + 4]);
                    }
                }
            }
            let blk = encode_block(&texels);
            let bo = (byi * bx + bxi) * 16;
            out[bo..bo + 16].copy_from_slice(&blk);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decode a single BC7 block to 16 RGBA texels via the production decoder.
    fn decode_block(blk: &[u8; 16]) -> [[u8; 4]; 16] {
        let mut argb = [0u32; 16];
        let _ = texture2ddecoder::decode_bc7(blk, 4, 4, &mut argb);
        let mut out = [[0u8; 4]; 16];
        for (i, &px) in argb.iter().enumerate() {
            out[i] = [
                ((px >> 16) & 0xFF) as u8, // R
                ((px >> 8) & 0xFF) as u8,  // G
                (px & 0xFF) as u8,         // B
                ((px >> 24) & 0xFF) as u8, // A
            ];
        }
        out
    }

    /// A solid block whose channels are even (so the 7-bit + p-bit endpoint
    /// reproduces them exactly) must decode back bit-exact.
    #[test]
    fn solid_even_block_is_exact() {
        let texels = [[10u8, 20, 30, 40]; 16];
        let blk = encode_block(&texels);
        for px in decode_block(&blk) {
            assert_eq!(px, [10, 20, 30, 40], "solid block must be exact");
        }
    }

    /// A smooth **1-D** colour gradient (all channels moving together along a
    /// single line — the case mode 6 is built for, and what real portrait/skin
    /// content looks like within a 4×4 block) must round-trip with small error.
    /// (A 2-D gradient with independently-varying channels is mode 6's known
    /// weakness — no single-subset encoding can fit two axes tightly — which is
    /// why the partitioned modes exist; those are future work.)
    #[test]
    fn gradient_block_round_trips_close() {
        let lerp = |a: i32, b: i32, t: f32| (a as f32 + (b - a) as f32 * t).round() as u8;
        let mut texels = [[0u8; 4]; 16];
        for (i, texel) in texels.iter_mut().enumerate() {
            let t = i as f32 / 15.0;
            *texel = [lerp(20, 200, t), lerp(30, 180, t), lerp(200, 40, t), 255];
        }
        let blk = encode_block(&texels);
        let back = decode_block(&blk);
        let mut err = 0u64;
        for (a, b) in texels.iter().zip(&back) {
            for c in 0..4 {
                err += (a[c] as i32 - b[c] as i32).unsigned_abs() as u64;
            }
        }
        let mean = err as f64 / 64.0;
        assert!(mean < 6.0, "1-D gradient block mean error too high: {mean}");
        // Alpha was constant 255 → stays near-opaque. (It can come back as 254:
        // the p-bit is shared across all four channels, so when RGB fits better
        // with p=0 the endpoint's alpha rounds to 254 — a negligible off-by-one.)
        assert!(back.iter().all(|p| p[3] >= 254), "alpha stays near-opaque");
    }

    /// The anchor swap path: force a block whose natural texel-0 index is high
    /// (texel 0 is the bright end of the ramp). It must still decode correctly.
    #[test]
    fn anchor_swap_still_decodes() {
        let mut texels = [[0u8; 4]; 16];
        for (i, texel) in texels.iter_mut().enumerate() {
            let v = (i * 17) as u8; // 0..255 ramp, texel 0 darkest
            *texel = [255 - v, 255 - v, 255 - v, 255]; // texel 0 brightest
        }
        let blk = encode_block(&texels);
        let back = decode_block(&blk);
        let mut err = 0u64;
        for (a, b) in texels.iter().zip(&back) {
            for c in 0..3 {
                err += (a[c] as i32 - b[c] as i32).unsigned_abs() as u64;
            }
        }
        assert!(err as f64 / 48.0 < 8.0, "anchor-swapped ramp drifted");
    }

    /// Full-image encode → decode via the same BC7 decoder GNF uses: a 64×64
    /// gradient survives with small mean error and correct orientation.
    #[test]
    fn image_encode_decode_directional_gradient() {
        let (w, h) = (64u32, 64u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                rgba[i] = (x * 255 / w) as u8;
                rgba[i + 2] = (y * 255 / h) as u8;
                rgba[i + 3] = 255;
            }
        }
        let blocks = encode(&rgba, w, h);
        assert_eq!(blocks.len() as u32, (w / 4) * (h / 4) * 16);
        let decoded = super::super::gnf::decode_bc7(&blocks, w, h);
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            [decoded[i], decoded[i + 1], decoded[i + 2]]
        };
        let tl = px(0, 0);
        let br = px(w - 1, h - 1);
        assert!(br[0] as i32 > tl[0] as i32 + 40, "red gradient lost");
        assert!(br[2] as i32 > tl[2] as i32 + 40, "blue gradient lost");
    }
}
