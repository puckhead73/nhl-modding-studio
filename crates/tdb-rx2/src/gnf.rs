//! PS4 **GNF** texture container (the "Kettle" `.gnf` files, NHL 15 → 21).
//!
//! A GNF wraps a single GNM texture: a `"GNF "` header, a GNM T# (Tsharp)
//! descriptor giving format/size/tiling, then the pixel bytes **tiled** for the
//! GCN (Liverpool) GPU. To get usable BCn blocks we reverse the tiling (see
//! [`detile_micro`]), then decode the BCn to RGBA.
//!
//! ## Header (verified on real NHL 15 files)
//!
//! ```text
//! 0x00  "GNF "                 magic (47 4E 46 20)
//! 0x04  u32   contentsSize     = 0xF8
//! 0x08  u8    version          = 2
//! 0x09  u8    alignment
//! ...
//! 0x10  GNM T# descriptor      8 × u32 LE  (see below)
//! 0x100 tiled BCn pixel data   (= 0xF8 header + 8)
//! ```
//!
//! ## T# (Tsharp) fields — parsed per the shadPS4 `Image` bit layout
//!
//! Two 64-bit words. `data_format` = `(dword1 >> 20) & 0x3F`;
//! `width-1` = `dword2 & 0x3FFF`, `height-1` = `(dword2 >> 14) & 0x3FFF`;
//! `tiling_index` = `(dword3 >> 20) & 0x1F`, `last_level` = `(dword3 >> 16) & 0xF`.
//! NHL 15 heads are uniformly `tiling_index = 13`, whose on-disk layout is 1D
//! micro tiling (verified by decoding a real head; see [`detile_micro`]).
//!
//! Data formats seen: `0x23`=BC1, `0x25`=BC3, `0x27`=BC5 (plus `0x24`=BC2,
//! `0x26`=BC4, `0x29`=BC7 by the GCN IMG_DATA_FORMAT table).

use thiserror::Error;

/// 4-byte magic at offset 0 of a GNF file.
pub const MAGIC_GNF: &[u8; 4] = b"GNF ";

/// BCn block-compression formats a GNF may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BcFormat {
    Bc1,
    Bc2,
    Bc3,
    Bc4,
    Bc5,
    Bc7,
}

impl BcFormat {
    /// Bytes per 4×4 block.
    fn block_bytes(self) -> usize {
        match self {
            BcFormat::Bc1 | BcFormat::Bc4 => 8,
            BcFormat::Bc2 | BcFormat::Bc3 | BcFormat::Bc5 | BcFormat::Bc7 => 16,
        }
    }

    fn from_gnm(dfmt: u32) -> Option<BcFormat> {
        Some(match dfmt {
            0x23 => BcFormat::Bc1,
            0x24 => BcFormat::Bc2,
            0x25 => BcFormat::Bc3,
            0x26 => BcFormat::Bc4,
            0x27 => BcFormat::Bc5,
            0x29 => BcFormat::Bc7,
            _ => return None,
        })
    }

    /// GCN `IMG_DATA_FORMAT` code — inverse of [`from_gnm`], used when writing a
    /// GNF T# descriptor.
    fn to_gnm(self) -> u32 {
        match self {
            BcFormat::Bc1 => 0x23,
            BcFormat::Bc2 => 0x24,
            BcFormat::Bc3 => 0x25,
            BcFormat::Bc4 => 0x26,
            BcFormat::Bc5 => 0x27,
            BcFormat::Bc7 => 0x29,
        }
    }
}

#[derive(Debug, Error)]
pub enum GnfError {
    #[error("input is shorter than the GNF header ({0} bytes)")]
    TooShort(usize),
    #[error("magic mismatch — expected `GNF `")]
    BadMagic,
    #[error("unsupported GNM data format 0x{0:02x}")]
    UnsupportedFormat(u32),
    #[error("pixel data slot runs past EOF (need {need} at {off}, have {have})")]
    TruncatedData { off: usize, need: usize, have: usize },
    #[error("unsupported tiling_index {0}")]
    UnsupportedTileMode(u32),
    #[error("new base mip is {got} bytes, expected {expected} for this GNF's dims/format")]
    BaseMipSize { expected: usize, got: usize },
}

/// A decoded GNF texture: the de-tiled base-mip BCn ready to decode to RGBA.
#[derive(Debug, Clone)]
pub struct GnfTexture {
    pub width: u32,
    pub height: u32,
    pub format: BcFormat,
    pub tiling_index: u32,
    /// The base mip, **de-tiled** to a linear BCn block layout.
    pub base_mip_linear: Vec<u8>,
    /// The base mip exactly as stored (tiled) — kept so the de-tile can be
    /// re-tried / compared without re-reading the file.
    pub base_mip_tiled: Vec<u8>,
}

fn read_u32_le(data: &[u8], off: usize) -> Option<u32> {
    data.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Parse a GNF, de-tiling its base mip.
pub fn parse(data: &[u8]) -> Result<GnfTexture, GnfError> {
    if data.len() < 0x100 {
        return Err(GnfError::TooShort(data.len()));
    }
    if &data[0..4] != MAGIC_GNF {
        return Err(GnfError::BadMagic);
    }
    // T# at 0x10.
    let d1 = read_u32_le(data, 0x14).unwrap();
    let d2 = read_u32_le(data, 0x18).unwrap();
    let d3 = read_u32_le(data, 0x1C).unwrap();

    let dfmt = (d1 >> 20) & 0x3F;
    let format = BcFormat::from_gnm(dfmt).ok_or(GnfError::UnsupportedFormat(dfmt))?;
    let width = (d2 & 0x3FFF) + 1;
    let height = ((d2 >> 14) & 0x3FFF) + 1;
    let tiling_index = (d3 >> 20) & 0x1F;

    // Pixel data begins at contentsSize(0xF8) + 8 = 0x100.
    let data_off = 0x100usize;
    let bw = format.block_bytes();
    let blocks_x = width.div_ceil(4) as usize;
    let blocks_y = height.div_ceil(4) as usize;
    // The base mip is tiled and padded to whole 8×8-block micro tiles.
    let base_bytes = tiled_base_bytes(blocks_x, blocks_y, bw);
    let tiled = data
        .get(data_off..data_off + base_bytes)
        .ok_or(GnfError::TruncatedData {
            off: data_off,
            need: base_bytes,
            have: data.len().saturating_sub(data_off),
        })?
        .to_vec();

    let base_mip_linear = detile_micro(&tiled, blocks_x, blocks_y, bw);

    Ok(GnfTexture {
        width,
        height,
        format,
        tiling_index,
        base_mip_linear,
        base_mip_tiled: tiled,
    })
}

// ----------------------------------------------------------- GNM de-tiling

/// Micro-tile side, in BCn blocks (an 8×8-block = 32×32-texel tile).
const MICRO_TILE: usize = 8;

/// GCN "thin" micro-tile element index within an 8×8-block tile — the standard
/// non-displayable Z-order: `x0,y0,x1,y1,x2,y2`. This is what makes tile
/// boundaries seamless (a row-major within-tile order leaves a visible
/// discontinuity at every 32-texel boundary that reads as washboard ripples on
/// the lit 3D model — a per-boundary gradient metric ranks Z-order best by ~2×).
fn micro_z_order(lx: usize, ly: usize) -> usize {
    (lx & 1)
        | ((ly & 1) << 1)
        | (((lx >> 1) & 1) << 2)
        | (((ly >> 1) & 1) << 3)
        | (((lx >> 2) & 1) << 4)
        | (((ly >> 2) & 1) << 5)
}

/// De-tile a `tiling_index = 13` GNF base mip into a linear row-major block
/// layout.
///
/// **Verified by decoding a real head to a seamless image.** Despite the T#
/// reporting a "2D" tile mode, NHL 15's head textures are laid out as simple
/// **1D micro tiling**: the surface is split into 8×8-block micro tiles stored
/// in row-major order, and within each tile the 64 blocks follow the GCN
/// non-displayable Z-order ([`micro_z_order`]). No pipe/bank/macro swizzle.
fn detile_micro(tiled: &[u8], blocks_x: usize, blocks_y: usize, elem_bytes: usize) -> Vec<u8> {
    let tiles_x = blocks_x.div_ceil(MICRO_TILE);
    let tile_elems = MICRO_TILE * MICRO_TILE; // 64
    let mut out = vec![0u8; blocks_x * blocks_y * elem_bytes];
    for y in 0..blocks_y {
        for x in 0..blocks_x {
            let tile = (y / MICRO_TILE) * tiles_x + (x / MICRO_TILE);
            let within = micro_z_order(x % MICRO_TILE, y % MICRO_TILE);
            let src = (tile * tile_elems + within) * elem_bytes;
            let dst = (y * blocks_x + x) * elem_bytes;
            if src + elem_bytes <= tiled.len() {
                out[dst..dst + elem_bytes].copy_from_slice(&tiled[src..src + elem_bytes]);
            }
        }
    }
    out
}

/// Tile a linear row-major block layout back into the `tiling_index = 13`
/// GNM micro-tile order — the exact inverse of [`detile_micro`].
///
/// `tiled` is the destination buffer (already sized to whole 8×8-block micro
/// tiles); this overwrites only the real block positions (`blocks_x × blocks_y`)
/// and leaves any padding blocks untouched, so a caller that seeds `tiled` from
/// the original bytes gets a byte-identical result when the pixels are unchanged.
/// [`micro_z_order`] is a Morton interleave (a bijection on the 8×8 grid), so
/// `tile_micro_into` followed by [`detile_micro`] is the identity on real blocks.
fn tile_micro_into(tiled: &mut [u8], linear: &[u8], blocks_x: usize, blocks_y: usize, elem: usize) {
    let tiles_x = blocks_x.div_ceil(MICRO_TILE);
    let tile_elems = MICRO_TILE * MICRO_TILE; // 64
    for y in 0..blocks_y {
        for x in 0..blocks_x {
            let tile = (y / MICRO_TILE) * tiles_x + (x / MICRO_TILE);
            let within = micro_z_order(x % MICRO_TILE, y % MICRO_TILE);
            let src = (y * blocks_x + x) * elem;
            let dst = (tile * tile_elems + within) * elem;
            if dst + elem <= tiled.len() && src + elem <= linear.len() {
                tiled[dst..dst + elem].copy_from_slice(&linear[src..src + elem]);
            }
        }
    }
}

/// Compute the tiled base-mip byte length for a texture of `blocks_x × blocks_y`
/// BCn blocks of `elem` bytes each (padded to whole 8×8-block micro tiles).
fn tiled_base_bytes(blocks_x: usize, blocks_y: usize, elem: usize) -> usize {
    let tiles_x = blocks_x.div_ceil(MICRO_TILE);
    let tiles_y = blocks_y.div_ceil(MICRO_TILE);
    tiles_x * tiles_y * MICRO_TILE * MICRO_TILE * elem
}

/// Write an edited **base mip** back into a GNF, preserving the original file's
/// authentic 256-byte header (magic + T# descriptor) and any trailing mip data
/// **byte-for-byte** — only the tiled base-mip pixel region is replaced.
///
/// `new_base_linear` is the edited base mip as a linear, row-major BCn block
/// stream (`blocks_x × blocks_y × block_bytes`) in the GNF's existing format —
/// i.e. a **same-format, same-dimensions** replacement (the analogue of
/// `Xpr2Texture::replace_base_mip_linear`). Resizing or reformatting would need a
/// regenerated T# descriptor and is intentionally out of scope: keeping the real
/// header verbatim is what makes this write verifiable rather than guessed.
///
/// The padding blocks of edge micro tiles are carried over from the original
/// tiled bytes, so replacing with the *unchanged* base mip reproduces the input
/// file exactly.
pub fn replace_base_mip(original: &[u8], new_base_linear: &[u8]) -> Result<Vec<u8>, GnfError> {
    let tex = parse(original)?;
    let elem = tex.format.block_bytes();
    let blocks_x = tex.width.div_ceil(4) as usize;
    let blocks_y = tex.height.div_ceil(4) as usize;
    let expected = blocks_x * blocks_y * elem;
    if new_base_linear.len() != expected {
        return Err(GnfError::BaseMipSize {
            expected,
            got: new_base_linear.len(),
        });
    }

    // Seed from the original tiled bytes (preserves edge-tile padding), then
    // overwrite the real block positions with the edited pixels.
    let mut tiled = tex.base_mip_tiled;
    tile_micro_into(&mut tiled, new_base_linear, blocks_x, blocks_y, elem);

    let data_off = 0x100usize;
    let base_bytes = tiled.len();
    let mut out = Vec::with_capacity(original.len());
    out.extend_from_slice(&original[..data_off]);
    out.extend_from_slice(&tiled);
    out.extend_from_slice(&original[data_off + base_bytes..]);
    Ok(out)
}

/// Build a **minimal, structurally-valid** GNF from a linear base-mip BCn block
/// stream: `"GNF "` header + a `tiling_index = 13` T# descriptor + the tiled
/// pixel data. Round-trips through [`parse`] (same format/dims/pixels).
///
/// This is a synthetic container builder — for tests and for constructing
/// fixtures. It is **not** a from-scratch encoder for shipping into the game:
/// real GNF headers carry alignment/reserved fields we deliberately do not
/// synthesise. To edit a real GNF, use [`replace_base_mip`], which preserves the
/// authentic header. `base_linear` must be `blocks_x × blocks_y × block_bytes`.
pub fn encode_gnf(format: BcFormat, width: u32, height: u32, base_linear: &[u8]) -> Vec<u8> {
    let elem = format.block_bytes();
    let blocks_x = width.div_ceil(4) as usize;
    let blocks_y = height.div_ceil(4) as usize;

    let mut hdr = vec![0u8; 0x100];
    hdr[0..4].copy_from_slice(MAGIC_GNF);
    hdr[4..8].copy_from_slice(&0xF8u32.to_le_bytes()); // contentsSize
    hdr[8] = 2; // version
                // T# at 0x10: only the fields `parse` reads.
    let d1 = (format.to_gnm() & 0x3F) << 20;
    let d2 = ((width - 1) & 0x3FFF) | (((height - 1) & 0x3FFF) << 14);
    let d3 = (13u32 & 0x1F) << 20; // tiling_index = 13
    hdr[0x14..0x18].copy_from_slice(&d1.to_le_bytes());
    hdr[0x18..0x1C].copy_from_slice(&d2.to_le_bytes());
    hdr[0x1C..0x20].copy_from_slice(&d3.to_le_bytes());

    let mut tiled = vec![0u8; tiled_base_bytes(blocks_x, blocks_y, elem)];
    tile_micro_into(&mut tiled, base_linear, blocks_x, blocks_y, elem);

    let mut out = hdr;
    out.extend_from_slice(&tiled);
    out
}

// ----------------------------------------------------------- BC4/BC5 → RGBA
//
// BC1/BC2/BC3 are decoded by the caller through the existing DDS block decoder
// (`tdb-archive-codec`, which `tdb-rx2` does not depend on). BC4/BC5 aren't in
// that decoder, so they're handled here as pure functions over de-tiled blocks.

/// Decode one BC4 (8-byte) alpha-style block's 16 values.
fn bc4_block(block: &[u8]) -> [u8; 16] {
    let r0 = block[0];
    let r1 = block[1];
    let mut r = [0u16; 8];
    r[0] = r0 as u16;
    r[1] = r1 as u16;
    if r0 > r1 {
        for i in 2..8 {
            r[i] = (((8 - i as u16) * r0 as u16 + (i as u16 - 1) * r1 as u16) / 7) as u16;
        }
    } else {
        for i in 2..6 {
            r[i] = (((6 - i as u16) * r0 as u16 + (i as u16 - 1) * r1 as u16) / 5) as u16;
        }
        r[6] = 0;
        r[7] = 255;
    }
    let bits = u64::from_le_bytes([
        block[2], block[3], block[4], block[5], block[6], block[7], 0, 0,
    ]);
    let mut out = [0u8; 16];
    for (i, o) in out.iter_mut().enumerate() {
        let idx = ((bits >> (3 * i)) & 0x7) as usize;
        *o = r[idx] as u8;
    }
    out
}

/// Decode a de-tiled BC4 base mip to RGBA8 (single channel → grayscale).
pub fn decode_bc4(data: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut rgba = vec![0u8; w * h * 4];
    let bx = w.div_ceil(4);
    for (bi, block) in data.chunks_exact(8).enumerate() {
        let vals = bc4_block(block);
        let (tx, ty) = ((bi % bx) * 4, (bi / bx) * 4);
        for py in 0..4 {
            for px in 0..4 {
                let (x, y) = (tx + px, ty + py);
                if x < w && y < h {
                    let v = vals[py * 4 + px];
                    let o = (y * w + x) * 4;
                    rgba[o] = v;
                    rgba[o + 1] = v;
                    rgba[o + 2] = v;
                    rgba[o + 3] = 255;
                }
            }
        }
    }
    rgba
}

/// Decode a de-tiled BC5 base mip to RGBA8 (RG + reconstructed Z, for normals).
pub fn decode_bc5(data: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut rgba = vec![0u8; w * h * 4];
    let bx = w.div_ceil(4);
    for (bi, block) in data.chunks_exact(16).enumerate() {
        let red = bc4_block(&block[0..8]);
        let green = bc4_block(&block[8..16]);
        let (tx, ty) = ((bi % bx) * 4, (bi / bx) * 4);
        for py in 0..4 {
            for px in 0..4 {
                let (x, y) = (tx + px, ty + py);
                if x < w && y < h {
                    let o = (y * w + x) * 4;
                    let r = red[py * 4 + px];
                    let g = green[py * 4 + px];
                    // Reconstruct Z for a normal map (unit vector).
                    let nx = (r as f32 / 127.5) - 1.0;
                    let ny = (g as f32 / 127.5) - 1.0;
                    let nz = (1.0 - (nx * nx + ny * ny)).max(0.0).sqrt();
                    rgba[o] = r;
                    rgba[o + 1] = g;
                    rgba[o + 2] = ((nz * 0.5 + 0.5) * 255.0) as u8;
                    rgba[o + 3] = 255;
                }
            }
        }
    }
    rgba
}

/// Decode a de-tiled BC7 base mip to RGBA8. BC7 has eight block modes and is
/// impractical to hand-roll correctly; we defer to the pure-Rust
/// `texture2ddecoder` (which returns BGRA `u32`s in row-major order).
pub fn decode_bc7(data: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (wu, hu) = (w as usize, h as usize);
    let mut argb = vec![0u32; wu * hu];
    // Ignore a decode error (returns whatever it filled) — callers validate the
    // image, and a partial decode is still more useful than a hard failure.
    let _ = texture2ddecoder::decode_bc7(data, wu, hu, &mut argb);
    let mut rgba = vec![0u8; wu * hu * 4];
    for (i, &px) in argb.iter().enumerate() {
        // texture2ddecoder packs 0xAARRGGBB.
        rgba[i * 4] = ((px >> 16) & 0xFF) as u8; // R
        rgba[i * 4 + 1] = ((px >> 8) & 0xFF) as u8; // G
        rgba[i * 4 + 2] = (px & 0xFF) as u8; // B
        rgba[i * 4 + 3] = ((px >> 24) & 0xFF) as u8; // A
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The operator's extracted NHL 16 (PS4) tree. Skipped when absent.
    const NHL16_ROOT: &str = r"H:\Emulators\ShadPS4\Games\CUSA01909\databigs\Extracted";

    /// NHL 16 player portraits are tiled BC7 (`0x29`) GNFs, 512×340. Verify the
    /// de-tile + BC7 decode produces a plausible image (opaque skin-ish pixels in
    /// the centre, a transparent border).
    #[test]
    fn nhl16_portrait_bc7_decodes() {
        let p = format!(r"{NHL16_ROOT}\nhl\ui\kettle_assets\playerportraits\p10000.gnf");
        let Ok(data) = std::fs::read(&p) else {
            eprintln!("skipping: {p} not present");
            return;
        };
        let tex = parse(&data).expect("portrait GNF parses");
        assert_eq!(tex.format, BcFormat::Bc7);
        assert_eq!((tex.width, tex.height), (512, 340));
        let rgba = decode_bc7(&tex.base_mip_linear, tex.width, tex.height);
        assert_eq!(rgba.len(), 512 * 340 * 4);

        // Centre pixel is an opaque face; the top-left corner is the transparent
        // background — proves both the alpha channel and a coherent (non-drifted)
        // layout.
        let centre = ((170 * 512) + 256) * 4;
        assert!(rgba[centre + 3] > 200, "face centre opaque");
        assert!(
            rgba[centre] > rgba[centre + 2],
            "skin tone (R>B) at centre: {:?}",
            &rgba[centre..centre + 3]
        );
        assert_eq!(rgba[3], 0, "top-left corner transparent");
    }

    #[test]
    fn format_map() {
        assert_eq!(BcFormat::from_gnm(0x23), Some(BcFormat::Bc1));
        assert_eq!(BcFormat::from_gnm(0x25), Some(BcFormat::Bc3));
        assert_eq!(BcFormat::from_gnm(0x27), Some(BcFormat::Bc5));
        assert_eq!(BcFormat::from_gnm(0x99), None);
    }

    #[test]
    fn format_gnm_code_round_trips() {
        for f in [
            BcFormat::Bc1,
            BcFormat::Bc2,
            BcFormat::Bc3,
            BcFormat::Bc4,
            BcFormat::Bc5,
            BcFormat::Bc7,
        ] {
            assert_eq!(BcFormat::from_gnm(f.to_gnm()), Some(f));
        }
    }

    /// A distinctive linear BCn block stream where every block byte is a function
    /// of its block index, so a misplaced block is detectable.
    fn synth_blocks(blocks_x: usize, blocks_y: usize, elem: usize) -> Vec<u8> {
        let mut v = vec![0u8; blocks_x * blocks_y * elem];
        for (i, b) in v.iter_mut().enumerate() {
            *b = ((i / elem) * 7 + i % elem) as u8;
        }
        v
    }

    /// Tiling then de-tiling must be the identity on the real (unpadded) blocks —
    /// including a non-square, non-tile-aligned surface (10×6 blocks → 2×1 tiles
    /// with edge padding).
    #[test]
    fn tile_then_detile_is_identity() {
        let (bx, by, elem) = (10usize, 6usize, 8usize);
        let linear = synth_blocks(bx, by, elem);
        let mut tiled = vec![0u8; tiled_base_bytes(bx, by, elem)];
        tile_micro_into(&mut tiled, &linear, bx, by, elem);
        let back = detile_micro(&tiled, bx, by, elem);
        assert_eq!(back, linear, "tile→detile must reproduce the linear blocks");
    }

    /// `encode_gnf` → `parse` recovers the same format, dims, and base mip.
    #[test]
    fn encode_then_parse_round_trips() {
        let (w, h) = (40u32, 24u32); // 10×6 BC1 blocks
        let (bx, by, elem) = (10usize, 6usize, 8usize);
        let linear = synth_blocks(bx, by, elem);
        let gnf = encode_gnf(BcFormat::Bc1, w, h, &linear);
        let tex = parse(&gnf).expect("synthetic GNF parses");
        assert_eq!((tex.width, tex.height), (w, h));
        assert_eq!(tex.format, BcFormat::Bc1);
        assert_eq!(tex.tiling_index, 13);
        assert_eq!(tex.base_mip_linear, linear);
    }

    /// Replacing the base mip with the **unchanged** blocks reproduces the input
    /// file byte-for-byte (header + tiled payload + tail all preserved).
    #[test]
    fn replace_base_mip_unchanged_is_byte_identical() {
        let (w, h) = (64u32, 48u32);
        let (bx, by, elem) = (16usize, 12usize, 16usize); // BC3
        let linear = synth_blocks(bx, by, elem);
        let mut gnf = encode_gnf(BcFormat::Bc3, w, h, &linear);
        // Append a trailing byte run to stand in for a mip tail; it must survive.
        gnf.extend_from_slice(&[0xABu8; 32]);
        let tex = parse(&gnf).unwrap();
        let rebuilt = replace_base_mip(&gnf, &tex.base_mip_linear).unwrap();
        assert_eq!(rebuilt, gnf, "unchanged replace must be byte-identical");
    }

    /// A real edit lands: replace with different blocks, re-parse, get them back;
    /// the 256-byte header is preserved verbatim.
    #[test]
    fn replace_base_mip_applies_edit_and_keeps_header() {
        let (w, h) = (64u32, 64u32);
        let (bx, by, elem) = (16usize, 16usize, 8usize); // BC1
        let original = synth_blocks(bx, by, elem);
        let gnf = encode_gnf(BcFormat::Bc1, w, h, &original);

        let mut edited = original.clone();
        for (i, b) in edited.iter_mut().enumerate() {
            *b = (255 - (i % 256)) as u8;
        }
        assert_ne!(edited, original);

        let rebuilt = replace_base_mip(&gnf, &edited).unwrap();
        assert_eq!(&rebuilt[..0x100], &gnf[..0x100], "header preserved verbatim");
        let tex = parse(&rebuilt).unwrap();
        assert_eq!(tex.base_mip_linear, edited, "edit round-trips through the tiler");
    }

    #[test]
    fn replace_base_mip_rejects_wrong_size() {
        let gnf = encode_gnf(BcFormat::Bc1, 64, 64, &synth_blocks(16, 16, 8));
        let err = replace_base_mip(&gnf, &[0u8; 10]).unwrap_err();
        assert!(matches!(err, GnfError::BaseMipSize { .. }));
    }

    #[test]
    fn rejects_non_gnf() {
        let mut buf = vec![0u8; 0x100];
        buf[0..4].copy_from_slice(b"RSF\0");
        assert!(matches!(parse(&buf), Err(GnfError::BadMagic)));
    }

    #[test]
    fn parses_real_head_cm_dimensions() {
        let path = r"H:\Emulators\ShadPS4\Games\CUSA00561\databigs\Extracted\nhl\data\kettle\rendering\characters\player\head_alexander_ovechkin_cm.gnf";
        let Ok(data) = std::fs::read(path) else {
            eprintln!("skipping: {path} not present");
            return;
        };
        let tex = parse(&data).expect("head_cm parses");
        assert_eq!((tex.width, tex.height), (1024, 1024));
        assert_eq!(tex.format, BcFormat::Bc1);
        assert_eq!(tex.tiling_index, 13);
        // De-tile produced a full base mip (256×256 BC1 blocks × 8 bytes).
        assert_eq!(tex.base_mip_linear.len(), 256 * 256 * 8);
    }
}
