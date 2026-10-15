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
    pub fn block_bytes(self) -> usize {
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
    TruncatedData {
        off: usize,
        need: usize,
        have: usize,
    },
    #[error("unsupported tiling_index {0}")]
    UnsupportedTileMode(u32),
    #[error("new base mip is {got} bytes, expected {expected} for this GNF's dims/format")]
    BaseMipSize { expected: usize, got: usize },
    #[error("resize refused: {0}")]
    ResizeRule(String),
    /// The requested output does not fit a header field (14-bit width / height, 4-bit level
    /// count, u32 sizes) or exceeds the chain the size halves to: refused, never masked.
    #[error("resize refused: {what} {value} cannot be stored in the GNF header (1–{max})")]
    Unrepresentable { what: &'static str, value: u64, max: u64 },
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
    assert!((1..=GNF_MAX_DIM).contains(&width) && (1..=GNF_MAX_DIM).contains(&height), "encode_gnf: {width}x{height} is not a representable T# size");
    let d1 = (format.to_gnm() & 0x3F) << 20;
    let d2 = (width - 1) | ((height - 1) << 14);
    // tiling_index = 13, one level (`last_level` 0), type 9 (2D) — the fields the measured size
    // rule (`check_resize_rule`) reads, so a synthetic file also takes the resize route.
    let d3 = ((13u32 & 0x1F) << 20) | (9 << 28);
    let d4 = (width.div_ceil(32) * 32 - 1) << 13; // pitch, depth 0
    hdr[0x14..0x18].copy_from_slice(&d1.to_le_bytes());
    hdr[0x18..0x1C].copy_from_slice(&d2.to_le_bytes());
    hdr[0x1C..0x20].copy_from_slice(&d3.to_le_bytes());
    hdr[0x20..0x24].copy_from_slice(&d4.to_le_bytes());

    let mut tiled = vec![0u8; tiled_base_bytes(blocks_x, blocks_y, elem)];
    tile_micro_into(&mut tiled, base_linear, blocks_x, blocks_y, elem);
    // `streamSize` (0x0C) = file length; `d7` (0x2C) = data bytes.
    hdr[0x0C..0x10].copy_from_slice(&((0x100 + tiled.len()) as u32).to_le_bytes());
    hdr[0x2C..0x30].copy_from_slice(&(tiled.len() as u32).to_le_bytes());

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
        for (i, v) in r.iter_mut().enumerate().skip(2) {
            *v = ((8 - i as u16) * r0 as u16 + (i as u16 - 1) * r1 as u16) / 7;
        }
    } else {
        for (i, v) in r.iter_mut().enumerate().take(6).skip(2) {
            *v = ((6 - i as u16) * r0 as u16 + (i as u16 - 1) * r1 as u16) / 5;
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
        assert_eq!(
            &rebuilt[..0x100],
            &gnf[..0x100],
            "header preserved verbatim"
        );
        let tex = parse(&rebuilt).unwrap();
        assert_eq!(
            tex.base_mip_linear, edited,
            "edit round-trips through the tiler"
        );
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

// ----------------------------------------------------------- P2-B resize (research handoff 15)

/// The largest width / height a T# can store (`width − 1` / `height − 1` are 14-bit fields).
pub const GNF_MAX_DIM: u32 = 0x4000;
/// The most levels a T# can declare (`last_level` is a 4-bit field).
pub const GNF_MAX_LEVELS: u32 = 16;

/// One linear level for [`replace_resized`]: a row-major BCn block stream of `width × height`.
#[derive(Debug, Clone)]
pub struct LinearLevel<'a> {
    pub width: u32,
    pub height: u32,
    pub linear: &'a [u8],
}

/// Why a shipped GNF header is outside the measured resize rule (research 2026-09-21 handoff 15
/// §1–3: 216 000 files of NHL 16 / 21; the refusals are the census's own exceptions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GnfRuleRefusal {
    NotTiling13 { tiling: u32 },
    NotTwoD { type_field: u32 },
    Volume { depth: u32 },
    /// Format `0x28` (BC6H) — block size unmodelled; and any format the reader does not decode.
    UnmodelledFormat { dfmt: u32 },
    /// The shipped header does not follow the rule (`pitch`, `d7`, `streamSize`, data size): a
    /// wrong field split or a cube map (6 × the chain) — refused rather than rewritten.
    NotReproducible { what: &'static str, expected: u64, actual: u64 },
    /// Two or more `GNF ` streams concatenated in one file: the first stream's `streamSize` is
    /// exact and a second header follows at that offset (research 2026-09-21 pm handoff 03 §2:
    /// NHL 15 ships 1 115, NHL 19 ships 5 342; NHL 16 / 17 / 18 / 20 / 21 ship none). The extra
    /// stream is content, not padding, so a resize that rewrote only the first would silently
    /// drop it — refused with its own reason rather than as a `streamSize` mismatch.
    MultiStream { streams: usize, first_stream_size: u64, file_len: u64 },
}

impl std::fmt::Display for GnfRuleRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotTiling13 { tiling } => write!(f, "tiling mode {tiling} is not the measured 1D micro-tiled mode 13"),
            Self::NotTwoD { type_field } => write!(f, "texture type {type_field} is not a 2D texture (cube / array maps are not resized)"),
            Self::Volume { depth } => write!(f, "depth {depth}: a volume texture is not resized"),
            Self::UnmodelledFormat { dfmt } => write!(f, "data format {dfmt:#x} is not modelled by the size rule (BC6H / unsupported)"),
            Self::NotReproducible { what, expected, actual } => write!(f, "the shipped header is not reproduced by the size rule ({what}: rule {expected}, file {actual}) — a cube map or an unmeasured layout; refused rather than rewritten"),
            Self::MultiStream { streams, first_stream_size, file_len } => write!(f, "multi-stream file: {streams} `GNF ` streams tile the {file_len}-byte file (the first is {first_stream_size} bytes); resizing one stream would drop the others, so this container is refused"),
        }
    }
}

/// How many `GNF ` streams tile `data` end to end, if it is a multi-stream file: walk from 0,
/// following each header's `streamSize`, and require every step to land on a `GNF ` magic and the
/// last to land exactly on the end. `None` when the file is a single stream or the walk does not
/// tile it (a truncated or otherwise irregular file, which stays a `streamSize` mismatch).
/// Research 2026-09-21 pm handoff 03 §2.
fn count_tiled_streams(data: &[u8]) -> Option<usize> {
    let mut off = 0usize;
    let mut streams = 0usize;
    while off < data.len() {
        if data.len() - off < 0x100 || &data[off..off + 4] != MAGIC_GNF {
            return None;
        }
        let size = read_u32_le(data, off + 0x0C)? as usize;
        if size < 0x100 {
            return None;
        }
        off = off.checked_add(size)?;
        streams += 1;
        if streams > 64 {
            return None;
        }
    }
    (off == data.len() && streams > 1).then_some(streams)
}

/// The micro-tiled byte count of one level (whole 8 × 8-block tiles × block bytes).
fn level_tiled_bytes(width: u32, height: u32, elem: usize) -> usize {
    tiled_base_bytes(width.div_ceil(4) as usize, height.div_ceil(4) as usize, elem)
}

/// The size rule of handoff 15 §2 over a mip chain from `width × height` with `levels` levels.
pub fn chain_tiled_bytes(width: u32, height: u32, elem: usize, levels: u32) -> usize {
    (0..levels).map(|l| level_tiled_bytes((width >> l).max(1), (height >> l).max(1), elem)).sum()
}

/// Check a shipped GNF against the measured header rules (handoff 15 §1–2): tiling 13, 2D, depth
/// 0, a modelled format, `pitch = roundup(w, 32) − 1`, `d7` = data bytes, `streamSize` = file
/// size, data bytes = the micro-tiled chain. The "descriptor reproducible" gate a resize passes
/// first; a cube map fails the data-size rule (6 × the chain).
pub fn check_resize_rule(original: &[u8]) -> Result<(u32, u32, BcFormat, u32), GnfRuleRefusal> {
    if original.len() < 0x100 || &original[0..4] != MAGIC_GNF {
        return Err(GnfRuleRefusal::NotReproducible { what: "magic / header length", expected: 0x100, actual: original.len() as u64 });
    }
    let t = |i: usize| read_u32_le(original, 0x10 + i * 4).unwrap();
    let dfmt = (t(1) >> 20) & 0x3F;
    let format = BcFormat::from_gnm(dfmt).ok_or(GnfRuleRefusal::UnmodelledFormat { dfmt })?;
    let width = (t(2) & 0x3FFF) + 1;
    let height = ((t(2) >> 14) & 0x3FFF) + 1;
    let tiling = (t(3) >> 20) & 0x1F;
    if tiling != 13 {
        return Err(GnfRuleRefusal::NotTiling13 { tiling });
    }
    let type_field = (t(3) >> 28) & 0xF;
    if type_field != 9 {
        return Err(GnfRuleRefusal::NotTwoD { type_field });
    }
    let depth = t(4) & 0x1FFF;
    if depth != 0 {
        return Err(GnfRuleRefusal::Volume { depth });
    }
    let levels = ((t(3) >> 16) & 0xF) + 1;
    let pitch = (t(4) >> 13) & 0x3FFF;
    let want_pitch = width.div_ceil(32) * 32 - 1;
    if pitch != want_pitch {
        return Err(GnfRuleRefusal::NotReproducible { what: "pitch", expected: want_pitch as u64, actual: pitch as u64 });
    }
    let data = original.len() as u64 - 0x100;
    let stream = read_u32_le(original, 0x0C).unwrap() as u64;
    if stream != original.len() as u64 {
        if let Some(streams) = count_tiled_streams(original) {
            return Err(GnfRuleRefusal::MultiStream { streams, first_stream_size: stream, file_len: original.len() as u64 });
        }
        return Err(GnfRuleRefusal::NotReproducible { what: "streamSize", expected: original.len() as u64, actual: stream });
    }
    if t(7) as u64 != data {
        return Err(GnfRuleRefusal::NotReproducible { what: "d7 (data bytes)", expected: data, actual: t(7) as u64 });
    }
    let chain = chain_tiled_bytes(width, height, format.block_bytes(), levels) as u64;
    if chain != data {
        return Err(GnfRuleRefusal::NotReproducible { what: "data size (micro-tiled chain)", expected: chain, actual: data });
    }
    Ok((width, height, format, levels))
}

/// Resize a shipped 2D GNF: the header's `width` / `height`, `last_level`, `pitch`, `d7` and
/// `streamSize` recomputed by the rules of handoff 15 §3, every other byte of the 256-byte
/// header kept (`d5` / `d6`, the `USER` tail, the reserved words), and the pixel data re-emitted
/// as the micro-tiled chain of `levels` (level 0 first). The format stays the file's. Refuses
/// anything `check_resize_rule` refuses, a level whose bytes do not match its dimensions, or a
/// chain that does not halve. Runtime acceptance of a changed size is card R31-GNF-1; the PS4
/// registry's size directory rule is the caller's outer gate.
pub fn replace_resized(original: &[u8], levels: &[LinearLevel<'_>]) -> Result<Vec<u8>, GnfError> {
    let (_w, _h, format, _n) = check_resize_rule(original).map_err(|e| GnfError::ResizeRule(e.to_string()))?;
    let elem = format.block_bytes();
    let Some(base) = levels.first() else {
        return Err(GnfError::BaseMipSize { expected: 0, got: 0 });
    };
    // Representability first (coordinator review R1 F1, 21 Sep 2026): the T# stores `width − 1`
    // and `height − 1` in 14 bits, `last_level` in 4 bits, `pitch` in 14 bits (implied by the
    // width bound), `d7` and `streamSize` as u32. A request outside a field is refused here,
    // before any level is validated, tiled or allocated — it is never masked into another size.
    let unrep = |what: &'static str, value: u64, max: u64| GnfError::Unrepresentable { what, value, max };
    if base.width == 0 || base.width > GNF_MAX_DIM {
        return Err(unrep("width", base.width as u64, GNF_MAX_DIM as u64));
    }
    if base.height == 0 || base.height > GNF_MAX_DIM {
        return Err(unrep("height", base.height as u64, GNF_MAX_DIM as u64));
    }
    let n_levels = levels.len() as u64;
    if n_levels > GNF_MAX_LEVELS as u64 {
        return Err(unrep("levels", n_levels, GNF_MAX_LEVELS as u64));
    }
    let halvings = 32 - base.width.max(base.height).leading_zeros(); // levels down to 1 × 1
    if n_levels > halvings as u64 {
        return Err(unrep("levels (for this size)", n_levels, halvings as u64));
    }
    let data_len = chain_tiled_bytes(base.width, base.height, elem, levels.len() as u32);
    let stream_len = data_len.checked_add(0x100).filter(|n| u32::try_from(*n).is_ok()).ok_or_else(|| unrep("streamSize (bytes)", data_len as u64 + 0x100, u32::MAX as u64))?;
    let mut data = Vec::with_capacity(data_len);
    for (i, lv) in levels.iter().enumerate() {
        let (ew, eh) = ((base.width >> i).max(1), (base.height >> i).max(1));
        if (lv.width, lv.height) != (ew, eh) {
            return Err(GnfError::ResizeRule(format!("level {i} is {}x{}, expected {ew}x{eh}", lv.width, lv.height)));
        }
        let (bx, by) = (lv.width.div_ceil(4) as usize, lv.height.div_ceil(4) as usize);
        if lv.linear.len() != bx * by * elem {
            return Err(GnfError::BaseMipSize { expected: bx * by * elem, got: lv.linear.len() });
        }
        let mut tiled = vec![0u8; tiled_base_bytes(bx, by, elem)];
        tile_micro_into(&mut tiled, lv.linear, bx, by, elem);
        data.extend_from_slice(&tiled);
    }
    debug_assert_eq!(data.len(), data_len);
    let mut out = original[..0x100].to_vec();
    let wr = |out: &mut [u8], off: usize, v: u32| out[off..off + 4].copy_from_slice(&v.to_le_bytes());
    let t = |i: usize| read_u32_le(original, 0x10 + i * 4).unwrap();
    wr(&mut out, 0x0C, stream_len as u32);
    wr(&mut out, 0x18, (t(2) & !0x0FFF_FFFF) | (base.width - 1) | ((base.height - 1) << 14));
    wr(&mut out, 0x1C, (t(3) & !(0xF << 16)) | ((levels.len() as u32 - 1) << 16));
    wr(&mut out, 0x20, (t(4) & !(0x3FFF << 13)) | ((base.width.div_ceil(32) * 32 - 1) << 13));
    wr(&mut out, 0x2C, data_len as u32);
    out.extend_from_slice(&data);
    // Read the result back through the same rule the input had to pass: the requested size and
    // level count, and the payload relationships (`pitch`, `d7`, `streamSize`, chain) must be
    // what was asked for — a writer that cannot reproduce its own request refuses.
    match check_resize_rule(&out) {
        Ok((w, h, f, n)) if (w, h, f, n as usize) == (base.width, base.height, format, levels.len()) => Ok(out),
        Ok((w, h, _, n)) => Err(GnfError::ResizeRule(format!("the written header reads back as {w}x{h} / {n} levels, not the requested {}x{} / {}", base.width, base.height, levels.len()))),
        Err(e) => Err(GnfError::ResizeRule(format!("the written header does not satisfy the size rule: {e}"))),
    }
}

/// The level count the T# declares (`last_level + 1`), or `None` for a buffer too short to hold a
/// header or without the `GNF ` magic. Read only; says nothing about whether the chain's layout is
/// the measured one — [`check_resize_rule`] decides that.
pub fn declared_levels(data: &[u8]) -> Option<u32> {
    if data.len() < 0x100 || &data[0..4] != MAGIC_GNF {
        return None;
    }
    Some(((read_u32_le(data, 0x1C)? >> 16) & 0xF) + 1)
}

/// Replace **every level** of a shipped 2D GNF at its existing size and format — the same-size
/// counterpart of [`replace_resized`]. [`replace_base_mip`] alone leaves the lower levels holding
/// the old image, which then shows whenever the game samples a smaller mip (at distance).
///
/// The file must pass [`check_resize_rule`], so the chain's level offsets are the measured
/// micro-tiled rule rather than a guess; `levels` must supply exactly the file's level count at
/// the halving sizes. The 256-byte header is kept verbatim (nothing about the size changes), and
/// each level is seeded from its original tiled bytes before the new blocks are tiled in, so edge-
/// tile padding is preserved and an unchanged chain reproduces the input byte for byte.
pub fn replace_chain_same_size(original: &[u8], levels: &[LinearLevel<'_>]) -> Result<Vec<u8>, GnfError> {
    let (width, height, format, n) = check_resize_rule(original).map_err(|e| GnfError::ResizeRule(e.to_string()))?;
    if levels.len() != n as usize {
        return Err(GnfError::ResizeRule(format!("{} level(s) supplied, the file has {n}", levels.len())));
    }
    let elem = format.block_bytes();
    let mut out = original.to_vec();
    let mut off = 0x100usize;
    for (i, lv) in levels.iter().enumerate() {
        let (ew, eh) = ((width >> i).max(1), (height >> i).max(1));
        if (lv.width, lv.height) != (ew, eh) {
            return Err(GnfError::ResizeRule(format!("level {i} is {}x{}, expected {ew}x{eh}", lv.width, lv.height)));
        }
        let (bx, by) = (ew.div_ceil(4) as usize, eh.div_ceil(4) as usize);
        if lv.linear.len() != bx * by * elem {
            return Err(GnfError::BaseMipSize { expected: bx * by * elem, got: lv.linear.len() });
        }
        let len = tiled_base_bytes(bx, by, elem);
        tile_micro_into(&mut out[off..off + len], lv.linear, bx, by, elem);
        off += len;
    }
    debug_assert_eq!(off, original.len(), "check_resize_rule guarantees the chain tiles the file");
    Ok(out)
}

#[cfg(test)]
mod resize_tests {
    use super::*;

    /// Every level of a rule-conformant chain is rewritten in place: header untouched, length
    /// unchanged, each level reading back as supplied; an unchanged chain is byte-identical; a
    /// wrong level count, a wrong level size and a file the rule refuses are all refused.
    #[test]
    fn same_size_chain_rewrites_every_level_and_keeps_the_header() {
        let src = conformant_bc1(64, 32, 3);
        let elem = BcFormat::Bc1.block_bytes();
        let dims = [(64u32, 32u32), (32, 16), (16, 8)];
        let fresh: Vec<Vec<u8>> = dims.iter().map(|&(w, h)| vec![0x5A; (w.div_ceil(4) * h.div_ceil(4)) as usize * elem]).collect();
        let lv: Vec<LinearLevel<'_>> = dims.iter().zip(&fresh).map(|(&(w, h), d)| LinearLevel { width: w, height: h, linear: d }).collect();
        let out = replace_chain_same_size(&src, &lv).unwrap();
        assert_eq!(&out[..0x100], &src[..0x100], "header kept verbatim");
        assert_eq!(out.len(), src.len());
        let mut off = 0x100;
        for (&(w, h), want) in dims.iter().zip(&fresh) {
            let (bx, by) = (w.div_ceil(4) as usize, h.div_ceil(4) as usize);
            let len = tiled_base_bytes(bx, by, elem);
            assert_eq!(&detile_micro(&out[off..off + len], bx, by, elem), want, "level {w}x{h} rewritten");
            off += len;
        }
        // Unchanged chain → identical bytes.
        let original: Vec<Vec<u8>> = {
            let mut off = 0x100;
            dims.iter().map(|&(w, h)| { let (bx, by) = (w.div_ceil(4) as usize, h.div_ceil(4) as usize); let len = tiled_base_bytes(bx, by, elem); let d = detile_micro(&src[off..off + len], bx, by, elem); off += len; d }).collect()
        };
        let same: Vec<LinearLevel<'_>> = dims.iter().zip(&original).map(|(&(w, h), d)| LinearLevel { width: w, height: h, linear: d }).collect();
        assert_eq!(replace_chain_same_size(&src, &same).unwrap(), src, "unchanged chain is byte-identical");
        // Refusals.
        assert!(replace_chain_same_size(&src, &lv[..2]).is_err(), "too few levels");
        let wrong = [LinearLevel { width: 64, height: 32, linear: &fresh[0] }, LinearLevel { width: 32, height: 32, linear: &fresh[1] }, LinearLevel { width: 16, height: 8, linear: &fresh[2] }];
        assert!(replace_chain_same_size(&src, &wrong).is_err(), "wrong level size");
        let mut irregular = src.clone();
        irregular.extend_from_slice(&[0u8; 16]);
        assert!(replace_chain_same_size(&irregular, &lv).is_err(), "a file the size rule refuses");
    }

    /// A synthetic 64 × 32 BC1 two-level GNF built by the rules → passes the reproducibility gate;
    /// a resize to 128 × 64 (3 levels) reproduces the rule fields and round-trips through `parse`;
    /// a wrong level size, a non-halving chain, and a cube-sized file are refused.
    #[test]
    fn resize_rewrites_the_rule_fields_and_round_trips() {
        let elem = 8usize;
        let level = |w: u32, h: u32, seed: u8| -> Vec<u8> { (0..(w.div_ceil(4) * h.div_ceil(4)) as usize * elem).map(|i| (i as u8).wrapping_mul(seed)).collect() };
        // Build the source by the rules themselves (encode_gnf gives a rule-conformant one-level
        // file; the checked writer extends it to two levels).
        let l0 = level(64, 32, 3);
        let l1 = level(32, 16, 5);
        let src = {
            let one = encode_gnf(BcFormat::Bc1, 64, 32, &l0);
            assert_eq!(check_resize_rule(&one).unwrap(), (64, 32, BcFormat::Bc1, 1), "encode_gnf is rule-conformant");
            replace_resized(&one, &[LinearLevel { width: 64, height: 32, linear: &l0 }, LinearLevel { width: 32, height: 16, linear: &l1 }]).unwrap()
        };
        assert_eq!(check_resize_rule(&src).unwrap(), (64, 32, BcFormat::Bc1, 2));
        let n0 = level(128, 64, 7);
        let n1 = level(64, 32, 9);
        let n2 = level(32, 16, 11);
        let out = replace_resized(&src, &[LinearLevel { width: 128, height: 64, linear: &n0 }, LinearLevel { width: 64, height: 32, linear: &n1 }, LinearLevel { width: 32, height: 16, linear: &n2 }]).unwrap();
        assert_eq!(check_resize_rule(&out).unwrap(), (128, 64, BcFormat::Bc1, 3));
        let p = parse(&out).unwrap();
        assert_eq!((p.width, p.height), (128, 64));
        assert_eq!(p.base_mip_linear, n0);
        assert_eq!(out.len(), 0x100 + chain_tiled_bytes(128, 64, elem, 3));
        assert_eq!(&out[0x30..0x100], &src[0x30..0x100], "the header tail is kept");
        // Refusals.
        assert!(replace_resized(&src, &[LinearLevel { width: 128, height: 64, linear: &n1 }]).is_err(), "wrong level bytes");
        assert!(replace_resized(&src, &[LinearLevel { width: 128, height: 64, linear: &n0 }, LinearLevel { width: 32, height: 16, linear: &n2 }]).is_err(), "non-halving chain");
        let mut cube = src.clone();
        let extra = cube[0x100..].to_vec();
        for _ in 0..5 {
            cube.extend_from_slice(&extra);
        }
        let n = cube.len() as u32;
        cube[0x0C..0x10].copy_from_slice(&n.to_le_bytes());
        cube[0x2C..0x30].copy_from_slice(&(n - 0x100).to_le_bytes());
        assert!(matches!(check_resize_rule(&cube), Err(GnfRuleRefusal::NotReproducible { what: "data size (micro-tiled chain)", .. })), "6 × the chain is refused");
    }

    /// A rule-conformant `width × height` BC1 GNF with the given level count (level bytes seeded).
    fn conformant_bc1(width: u32, height: u32, levels: u32) -> Vec<u8> {
        let elem = BcFormat::Bc1.block_bytes();
        let level = |w: u32, h: u32, seed: u8| -> Vec<u8> { (0..(w.div_ceil(4) * h.div_ceil(4)) as usize * elem).map(|i| (i as u8).wrapping_mul(seed)).collect() };
        let data: Vec<(u32, u32, Vec<u8>)> = (0..levels).map(|l| { let (w, h) = ((width >> l).max(1), (height >> l).max(1)); (w, h, level(w, h, 3 + 2 * l as u8)) }).collect();
        let one = encode_gnf(BcFormat::Bc1, width, height, &data[0].2);
        let lv: Vec<LinearLevel<'_>> = data.iter().map(|(w, h, d)| LinearLevel { width: *w, height: *h, linear: d }).collect();
        replace_resized(&one, &lv).unwrap()
    }

    /// The header's dimension fields hold `width − 1` / `height − 1` in 14 bits each (max 16 384);
    /// `last_level` holds 4 bits (max 16 levels). Coordinator review R1 (F1, 21 Sep 2026): a request
    /// of 16 400 × 4 was masked to width 16 and reported as success. Now: the largest representable
    /// value on each axis succeeds and is read back exactly; the first value past it, an excess of
    /// levels, a level beyond the halvings and a zero dimension are refused — and the refusal names
    /// the field, never a masked value.
    #[test]
    fn resize_refuses_unrepresentable_dimensions_instead_of_masking_them() {
        let src = conformant_bc1(32, 32, 1);
        assert_eq!(check_resize_rule(&src).unwrap(), (32, 32, BcFormat::Bc1, 1));
        let elem = 8usize;
        let level = |w: u32, h: u32| -> Vec<u8> { vec![0xA5; (w.div_ceil(4) * h.div_ceil(4)) as usize * elem] };
        // The reviewer's reproduction: 16 400 × 4 must not become 16 × 4.
        let wide = level(16_400, 4);
        let err = replace_resized(&src, &[LinearLevel { width: 16_400, height: 4, linear: &wide }]).expect_err("16 400 wide is not representable in 14 bits");
        assert!(err.to_string().contains("width") && err.to_string().contains("16400") && err.to_string().contains("16384"), "{err}");
        // Boundaries: 16 384 on either axis is representable and reads back exactly.
        for (w, h) in [(16_384u32, 4u32), (4, 16_384)] {
            let l = level(w, h);
            let out = replace_resized(&src, &[LinearLevel { width: w, height: h, linear: &l }]).unwrap_or_else(|e| panic!("{w}x{h}: {e}"));
            assert_eq!(check_resize_rule(&out).unwrap(), (w, h, BcFormat::Bc1, 1), "{w}x{h}");
            let p = parse(&out).unwrap();
            assert_eq!((p.width, p.height), (w, h));
            assert_eq!(p.base_mip_linear, l);
            assert_eq!(out.len(), 0x100 + chain_tiled_bytes(w, h, elem, 1));
        }
        // First invalid value on each axis, the zero dimension, and the level-count field.
        for (w, h) in [(16_385u32, 4u32), (4, 16_385), (0, 4), (4, 0)] {
            let l = level(w, h);
            let err = replace_resized(&src, &[LinearLevel { width: w, height: h, linear: &l }]).expect_err("must refuse");
            assert!(!err.to_string().contains("expected"), "{w}x{h}: refused for representability, not for a masked size: {err}");
        }
        let chain: Vec<Vec<u8>> = (0..17).map(|l| level((65_536u32 >> l).max(1), 4)).collect();
        let too_many: Vec<LinearLevel<'_>> = chain.iter().enumerate().map(|(l, d)| LinearLevel { width: (65_536u32 >> l).max(1), height: 4, linear: d }).collect();
        assert!(replace_resized(&src, &too_many).is_err(), "17 levels / 65 536 wide");
        // 32 × 32 affords 6 levels (32 → 1); a seventh 1 × 1 level is not a halving.
        let seven: Vec<Vec<u8>> = (0..7).map(|l| level((32u32 >> l).max(1), (32u32 >> l).max(1))).collect();
        let lv: Vec<LinearLevel<'_>> = seven.iter().enumerate().map(|(l, d)| LinearLevel { width: (32u32 >> l).max(1), height: (32u32 >> l).max(1), linear: d }).collect();
        let err = replace_resized(&src, &lv).expect_err("7 levels from 32 × 32");
        assert!(err.to_string().contains("levels"), "{err}");
        let out = replace_resized(&src, &lv[..6]).expect("6 levels from 32 × 32");
        assert_eq!(check_resize_rule(&out).unwrap(), (32, 32, BcFormat::Bc1, 6));
    }
}
