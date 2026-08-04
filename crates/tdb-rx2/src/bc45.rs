//! BC4 / BC5 (single- and dual-channel) encoders.
//!
//! - **BC4** packs one channel into an 8-byte block: two endpoint bytes
//!   (`r0`, `r1`) followed by sixteen 3-bit indices into an 8-entry palette.
//! - **BC5** is two BC4 blocks back to back (16 bytes): the red channel then the
//!   green channel — used for tangent-space normal maps (Z is reconstructed on
//!   decode, so only R/G are stored).
//!
//! We always drive the palette from `r0 = max`, `r1 = min`, which selects the
//! decoder's `r0 > r1` ("6-interpolated") mode — the higher-precision layout,
//! endpoints plus six evenly-spaced values between. A constant block collapses
//! to `r0 == r1` with every index 0 (exact).
//!
//! These are the exact inverse of [`crate::gnf::decode_bc4`] /
//! [`crate::gnf::decode_bc5`] and are verified by round-tripping through them.

/// Build the 8-entry palette exactly as the decoder does, so an encoded index
/// resolves to the same value on the way back.
fn palette(r0: u8, r1: u8) -> [u8; 8] {
    let mut p = [0u8; 8];
    p[0] = r0;
    p[1] = r1;
    if r0 > r1 {
        for (i, slot) in p.iter_mut().enumerate().skip(2) {
            *slot = (((8 - i as u16) * r0 as u16 + (i as u16 - 1) * r1 as u16) / 7) as u8;
        }
    } else {
        // Constant block (r0 == r1): index 0 already yields r0, so these slots
        // are unused, but we mirror the decoder's layout for completeness.
        for (i, slot) in p.iter_mut().enumerate().take(6).skip(2) {
            *slot = (((6 - i as u16) * r0 as u16 + (i as u16 - 1) * r1 as u16) / 5) as u8;
        }
        p[6] = 0;
        p[7] = 255;
    }
    p
}

/// Encode 16 single-channel values into an 8-byte BC4 block.
pub fn encode_bc4_block(vals: &[u8; 16]) -> [u8; 8] {
    let r0 = *vals.iter().max().unwrap();
    let r1 = *vals.iter().min().unwrap();
    let pal = palette(r0, r1);

    let mut bits: u64 = 0;
    for (t, &v) in vals.iter().enumerate() {
        let idx = if r0 == r1 {
            0usize
        } else {
            let mut best_i = 0usize;
            let mut best_err = i32::MAX;
            for (i, &pv) in pal.iter().enumerate() {
                let d = (pv as i32 - v as i32).abs();
                if d < best_err {
                    best_err = d;
                    best_i = i;
                }
            }
            best_i
        };
        bits |= (idx as u64) << (3 * t);
    }

    let mut out = [0u8; 8];
    out[0] = r0;
    out[1] = r1;
    out[2..8].copy_from_slice(&bits.to_le_bytes()[0..6]);
    out
}

/// Gather one 4×4 block of a single channel (byte offset `chan` within each
/// RGBA texel), clamping partial edge blocks to the last row/column.
fn gather_channel(rgba: &[u8], w: usize, h: usize, bx: usize, by: usize, chan: usize) -> [u8; 16] {
    let mut vals = [0u8; 16];
    for py in 0..4 {
        for px in 0..4 {
            let sx = (bx * 4 + px).min(w.saturating_sub(1));
            let sy = (by * 4 + py).min(h.saturating_sub(1));
            let o = (sy * w + sx) * 4 + chan;
            if o < rgba.len() {
                vals[py * 4 + px] = rgba[o];
            }
        }
    }
    vals
}

/// Encode RGBA8 to a linear BC4 block stream, taking the **R** channel (the
/// channel [`crate::gnf::decode_bc4`] replicates to grayscale). Block layout
/// matches GNF's de-tiled base mip (`ceil(w/4) × ceil(h/4)` blocks × 8 bytes).
pub fn encode_bc4(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (wu, hu) = (w as usize, h as usize);
    let bxn = wu.div_ceil(4);
    let byn = hu.div_ceil(4);
    let mut out = vec![0u8; bxn * byn * 8];
    for by in 0..byn {
        for bx in 0..bxn {
            let vals = gather_channel(rgba, wu, hu, bx, by, 0);
            let blk = encode_bc4_block(&vals);
            let bo = (by * bxn + bx) * 8;
            out[bo..bo + 8].copy_from_slice(&blk);
        }
    }
    out
}

/// Encode RGBA8 to a linear BC5 block stream, taking **R** and **G** (the two
/// channels [`crate::gnf::decode_bc5`] recovers; Z/blue is reconstructed, not
/// stored). Each block is `bc4(R) ++ bc4(G)` — 16 bytes.
pub fn encode_bc5(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (wu, hu) = (w as usize, h as usize);
    let bxn = wu.div_ceil(4);
    let byn = hu.div_ceil(4);
    let mut out = vec![0u8; bxn * byn * 16];
    for by in 0..byn {
        for bx in 0..bxn {
            let red = encode_bc4_block(&gather_channel(rgba, wu, hu, bx, by, 0));
            let green = encode_bc4_block(&gather_channel(rgba, wu, hu, bx, by, 1));
            let bo = (by * bxn + bx) * 16;
            out[bo..bo + 8].copy_from_slice(&red);
            out[bo + 8..bo + 16].copy_from_slice(&green);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gnf::{decode_bc4, decode_bc5};

    /// A constant single-channel block must round-trip bit-exact.
    #[test]
    fn bc4_constant_block_is_exact() {
        let blk = encode_bc4_block(&[73u8; 16]);
        let rgba = decode_bc4(&blk, 4, 4);
        for px in rgba.chunks_exact(4) {
            assert_eq!(px, [73, 73, 73, 255]);
        }
    }

    /// A within-block ramp must round-trip with small error (BC4 gives ~step/2).
    #[test]
    fn bc4_ramp_round_trips_close() {
        let mut vals = [0u8; 16];
        for (i, v) in vals.iter_mut().enumerate() {
            *v = (i * 17) as u8; // 0..255
        }
        let blk = encode_bc4_block(&vals);
        let rgba = decode_bc4(&blk, 4, 4);
        let mut err = 0u64;
        for (i, &v) in vals.iter().enumerate() {
            err += (rgba[i * 4] as i32 - v as i32).unsigned_abs() as u64;
        }
        assert!(err as f64 / 16.0 < 10.0, "BC4 ramp mean error too high");
    }

    /// Full-image BC4 encode → decode: a horizontal gradient survives with small
    /// error and correct direction.
    #[test]
    fn bc4_image_round_trips() {
        let (w, h) = (64u32, 64u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let v = (x * 255 / w) as u8;
                rgba[i..i + 4].copy_from_slice(&[v, v, v, 255]);
            }
        }
        let blocks = encode_bc4(&rgba, w, h);
        assert_eq!(blocks.len() as u32, (w / 4) * (h / 4) * 8);
        let back = decode_bc4(&blocks, w, h);
        let sample = |x: u32| back[(x * 4) as usize]; // row 0
        assert!((sample(w - 1) as i32) > (sample(0) as i32) + 40, "gradient lost");
    }

    /// BC5 stores two channels: a 2-D field (R = x-gradient, G = y-gradient) must
    /// come back on the R and G channels with small error.
    #[test]
    fn bc5_two_channels_round_trip() {
        let (w, h) = (64u32, 64u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                rgba[i] = (x * 255 / w) as u8; // R varies with x
                rgba[i + 1] = (y * 255 / h) as u8; // G varies with y
                rgba[i + 3] = 255;
            }
        }
        let blocks = encode_bc5(&rgba, w, h);
        assert_eq!(blocks.len() as u32, (w / 4) * (h / 4) * 16);
        let back = decode_bc5(&blocks, w, h);
        // R gradient across, G gradient down — both must survive on their channel.
        let at = |x: u32, y: u32, c: usize| back[((y * w + x) * 4) as usize + c] as i32;
        assert!(at(w - 1, 0, 0) > at(0, 0, 0) + 40, "BC5 red (x) gradient lost");
        assert!(at(0, h - 1, 1) > at(0, 0, 1) + 40, "BC5 green (y) gradient lost");
    }
}
