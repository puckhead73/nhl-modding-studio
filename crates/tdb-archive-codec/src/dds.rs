//! Minimal DDS writer.
//!
//! Produces a valid DirectDraw-Surface (`DDS `) file header + payload
//! from a known-format pixel stream. Used by the bulk texture-export
//! workflow:
//!
//! - **DXT1 / DXT3 / DXT5** — the caller already has the raw DXT block
//!   stream (e.g. from a parsed RPSGL `Texture::pixel_data` or a PC
//!   `.big` entry that was already DDS-wrapped). We wrap it in a DDS
//!   header verbatim — no recompression, no quality loss.
//! - **Rgba32** — the caller has decoded RGBA bytes (top mip only). We
//!   wrap it in an uncompressed 32 bpp DDS with the byte-order channel
//!   masks for RGBA-in-memory layout. Used for PS3 A8R8G8B8 textures
//!   that we deswizzle through `Texture::decode_top_mip_rgba` and as a
//!   last-resort PNG-fallback alternative.
//!
//! ## Scope
//!
//! - Write only. We never parse DDS — the read path goes through
//!   format-specific decoders (DXT via `texpresso`, RPSGL via
//!   `tdb-archive-rpsgl`).
//! - No DX10 extended header. Every format we emit fits in the classic
//!   124-byte `DDS_HEADER` + 32-byte `DDS_PIXELFORMAT`.
//! - No volume textures, no cubemaps. 2D textures with optional mip
//!   chain.

use std::convert::TryFrom;

/// Pixel format selector for [`encode_dds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdsFormat {
    /// DXT1 / BC1 — 4×4 blocks of 8 bytes (8 bpp).
    Dxt1,
    /// DXT3 / BC2 — 4×4 blocks of 16 bytes (16 bpp).
    Dxt3,
    /// DXT5 / BC3 — 4×4 blocks of 16 bytes (16 bpp).
    Dxt5,
    /// Uncompressed 32 bpp, **bytes in RGBA order** in memory (R is
    /// byte 0, A is byte 3). The header writes channel masks that
    /// match this byte order so any standard DDS reader sees the right
    /// channels.
    Rgba32,
}

impl DdsFormat {
    /// Bytes per 4×4 block (DXT) or per pixel × 16 (uncompressed).
    /// Used to compute the `pitch_or_linear_size` field for the top
    /// mip — the field is informational for compressed formats but
    /// some readers refuse files where the header lies.
    fn linear_size(self, width: u32, height: u32) -> u32 {
        match self {
            Self::Dxt1 => {
                let bw = ((width + 3) / 4).max(1);
                let bh = ((height + 3) / 4).max(1);
                bw * bh * 8
            }
            Self::Dxt3 | Self::Dxt5 => {
                let bw = ((width + 3) / 4).max(1);
                let bh = ((height + 3) / 4).max(1);
                bw * bh * 16
            }
            Self::Rgba32 => width * height * 4,
        }
    }
}

const DDS_MAGIC: &[u8; 4] = b"DDS ";
const HEADER_SIZE: u32 = 124;
const PIXELFORMAT_SIZE: u32 = 32;

// DDSD_* flags.
const DDSD_CAPS: u32 = 0x1;
const DDSD_HEIGHT: u32 = 0x2;
const DDSD_WIDTH: u32 = 0x4;
const DDSD_PITCH: u32 = 0x8;
const DDSD_PIXELFORMAT: u32 = 0x1000;
const DDSD_MIPMAPCOUNT: u32 = 0x20000;
const DDSD_LINEARSIZE: u32 = 0x80000;

// DDPF_* flags.
const DDPF_ALPHAPIXELS: u32 = 0x1;
const DDPF_FOURCC: u32 = 0x4;
const DDPF_RGB: u32 = 0x40;

// DDSCAPS_* flags.
const DDSCAPS_COMPLEX: u32 = 0x8;
const DDSCAPS_TEXTURE: u32 = 0x1000;
const DDSCAPS_MIPMAP: u32 = 0x400000;

/// Encode a DDS file (`DDS ` magic + 124-byte header + payload).
///
/// `payload` must hold the full mip chain in declared format, packed
/// contiguously (top mip first). The header's `mipMapCount` is set
/// from `mip_count` — pass `1` for top-mip-only.
///
/// Returns a `Vec<u8>` containing the complete `.dds` byte stream
/// ready to write to disk. Never fails for in-range inputs; if
/// `width`/`height` are zero we still emit a syntactically valid DDS
/// (some readers reject 0-size, but we leave that judgment to the
/// caller — exports keep going on truncated source data).
pub fn encode_dds(
    format: DdsFormat,
    width: u32,
    height: u32,
    mip_count: u32,
    payload: &[u8],
) -> Vec<u8> {
    let mip_count = mip_count.max(1);
    let mut out = Vec::with_capacity(4 + 124 + payload.len());
    out.extend_from_slice(DDS_MAGIC);

    // DDS_HEADER (little-endian throughout — DDS is a Windows format).
    let is_compressed = matches!(format, DdsFormat::Dxt1 | DdsFormat::Dxt3 | DdsFormat::Dxt5);
    let mut flags = DDSD_CAPS | DDSD_HEIGHT | DDSD_WIDTH | DDSD_PIXELFORMAT;
    if is_compressed {
        flags |= DDSD_LINEARSIZE;
    } else {
        flags |= DDSD_PITCH;
    }
    if mip_count > 1 {
        flags |= DDSD_MIPMAPCOUNT;
    }

    let pitch_or_linear_size = if is_compressed {
        format.linear_size(width, height)
    } else {
        // Pitch = (width * bits_per_pixel + 7) / 8 — 32 bpp here.
        width * 4
    };

    push_u32_le(&mut out, HEADER_SIZE);
    push_u32_le(&mut out, flags);
    push_u32_le(&mut out, height);
    push_u32_le(&mut out, width);
    push_u32_le(&mut out, pitch_or_linear_size);
    push_u32_le(&mut out, 0); // depth
    push_u32_le(&mut out, mip_count);
    // reserved1[11] — 44 bytes of zero.
    out.extend_from_slice(&[0u8; 44]);

    // DDS_PIXELFORMAT (32 bytes).
    push_u32_le(&mut out, PIXELFORMAT_SIZE);
    match format {
        DdsFormat::Dxt1 => {
            push_u32_le(&mut out, DDPF_FOURCC);
            out.extend_from_slice(b"DXT1");
            push_u32_le(&mut out, 0); // rgb_bit_count
            push_u32_le(&mut out, 0); // r_bit_mask
            push_u32_le(&mut out, 0); // g_bit_mask
            push_u32_le(&mut out, 0); // b_bit_mask
            push_u32_le(&mut out, 0); // a_bit_mask
        }
        DdsFormat::Dxt3 => {
            push_u32_le(&mut out, DDPF_FOURCC);
            out.extend_from_slice(b"DXT3");
            push_u32_le(&mut out, 0);
            push_u32_le(&mut out, 0);
            push_u32_le(&mut out, 0);
            push_u32_le(&mut out, 0);
            push_u32_le(&mut out, 0);
        }
        DdsFormat::Dxt5 => {
            push_u32_le(&mut out, DDPF_FOURCC);
            out.extend_from_slice(b"DXT5");
            push_u32_le(&mut out, 0);
            push_u32_le(&mut out, 0);
            push_u32_le(&mut out, 0);
            push_u32_le(&mut out, 0);
            push_u32_le(&mut out, 0);
        }
        DdsFormat::Rgba32 => {
            // 32 bpp uncompressed; bytes in memory ordered R,G,B,A.
            // The DDS DWORD masks below describe how a little-endian
            // DWORD reader would extract each channel — for an RGBA
            // byte stream the masks come out as below.
            push_u32_le(&mut out, DDPF_RGB | DDPF_ALPHAPIXELS);
            push_u32_le(&mut out, 0); // fourcc unused
            push_u32_le(&mut out, 32); // rgb_bit_count
            push_u32_le(&mut out, 0x000000FF); // r_bit_mask
            push_u32_le(&mut out, 0x0000FF00); // g_bit_mask
            push_u32_le(&mut out, 0x00FF0000); // b_bit_mask
            push_u32_le(&mut out, 0xFF000000); // a_bit_mask
        }
    }

    // caps / caps2 / caps3 / caps4 / reserved2 (20 bytes).
    let mut caps = DDSCAPS_TEXTURE;
    if mip_count > 1 {
        caps |= DDSCAPS_COMPLEX | DDSCAPS_MIPMAP;
    }
    push_u32_le(&mut out, caps);
    push_u32_le(&mut out, 0); // caps2
    push_u32_le(&mut out, 0); // caps3
    push_u32_le(&mut out, 0); // caps4
    push_u32_le(&mut out, 0); // reserved2

    // Header is exactly 128 bytes (4 magic + 124 header).
    debug_assert_eq!(out.len(), 128);

    out.extend_from_slice(payload);
    out
}

fn push_u32_le(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Look at the first 4 bytes and decide whether the buffer is already
/// a DDS file. Used by the export walker to decide between
/// "passthrough" and "wrap" paths.
pub fn is_dds(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && <[u8; 4]>::try_from(&bytes[..4]).unwrap() == *DDS_MAGIC
}

/// Decoded top-mip RGBA from a DDS file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedDds {
    pub width: u32,
    pub height: u32,
    /// Tightly packed RGBA8 (`[R,G,B,A]` per pixel), length
    /// `4 * width * height`.
    pub rgba: Vec<u8>,
}

/// Failure modes for [`decode_dds_to_rgba`].
#[derive(Debug, thiserror::Error)]
pub enum DdsDecodeError {
    #[error("DDS is shorter than its 128-byte header ({0} bytes)")]
    TooShort(usize),
    #[error("missing `DDS ` magic")]
    BadMagic,
    #[error("DDS reports zero width or height")]
    ZeroDimensions,
    #[error("unsupported DDS fourcc `{0}` (only DXT1/DXT3/DXT5 are decoded here)")]
    UnsupportedFourcc(String),
    #[error("DDS payload truncated: need {needed} bytes total, have {have}")]
    Truncated { needed: usize, have: usize },
}

/// Decode the **top mip** of a block-compressed DDS (`DXT1` / `DXT3` /
/// `DXT5`) into RGBA8 via `texpresso`.
///
/// Scope mirrors the read side the CLI texture tools need (resize +
/// prebuild): only the classic 124-byte header + a DXT fourcc. Mip
/// chains beyond the base, DX10 extended headers, and uncompressed
/// pixel formats are out of scope — callers needing 32-bit/A8R8G8B8
/// go through the format-aware decoders in `tdb-gui-api::texture_lab`.
pub fn decode_dds_to_rgba(bytes: &[u8]) -> Result<DecodedDds, DdsDecodeError> {
    if bytes.len() < 128 {
        return Err(DdsDecodeError::TooShort(bytes.len()));
    }
    if &bytes[..4] != DDS_MAGIC {
        return Err(DdsDecodeError::BadMagic);
    }
    // Classic DDS_HEADER offsets (little-endian): height@12, width@16,
    // pixelformat fourcc@84.
    let height = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    let width = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
    if width == 0 || height == 0 {
        return Err(DdsDecodeError::ZeroDimensions);
    }
    let fourcc: [u8; 4] = bytes[84..88].try_into().unwrap();
    let fmt = match &fourcc {
        b"DXT1" => texpresso::Format::Bc1,
        b"DXT3" => texpresso::Format::Bc2,
        b"DXT5" => texpresso::Format::Bc3,
        _ => {
            return Err(DdsDecodeError::UnsupportedFourcc(
                String::from_utf8_lossy(&fourcc).into_owned(),
            ));
        }
    };

    let (w, h) = (width as usize, height as usize);
    let block_size = fmt.compressed_size(w, h);
    let needed = 128 + block_size;
    if bytes.len() < needed {
        return Err(DdsDecodeError::Truncated {
            needed,
            have: bytes.len(),
        });
    }
    let mut rgba = vec![0u8; w * h * 4];
    fmt.decompress(&bytes[128..needed], w, h, &mut rgba);
    Ok(DecodedDds {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dxt5_header_layout_matches_spec() {
        // 4×4 texture, 1 mip, dummy block (16 bytes).
        let payload = vec![0xAAu8; 16];
        let dds = encode_dds(DdsFormat::Dxt5, 4, 4, 1, &payload);
        assert_eq!(&dds[..4], b"DDS ");
        // size at offset 4 == 124.
        assert_eq!(u32::from_le_bytes(dds[4..8].try_into().unwrap()), 124);
        // height at 12 == 4; width at 16 == 4.
        assert_eq!(u32::from_le_bytes(dds[12..16].try_into().unwrap()), 4);
        assert_eq!(u32::from_le_bytes(dds[16..20].try_into().unwrap()), 4);
        // pixelformat fourcc at offset 4(magic) + 72(pf offset within
        // header, i.e. after the 8 leading fields + reserved1) +
        // 4(pf.size) + 4(pf.flags) = 84.
        assert_eq!(&dds[84..88], b"DXT5");
        // payload appended verbatim.
        assert_eq!(&dds[128..], payload.as_slice());
    }

    #[test]
    fn dxt1_pitch_is_block_count_times_eight() {
        let dds = encode_dds(DdsFormat::Dxt1, 16, 16, 1, &[0u8; 128]);
        // linear_size at offset 4+16 = 20: 4 blocks * 4 blocks * 8 = 128.
        assert_eq!(u32::from_le_bytes(dds[20..24].try_into().unwrap()), 128);
    }

    #[test]
    fn rgba32_masks_describe_rgba_byte_order() {
        let dds = encode_dds(DdsFormat::Rgba32, 2, 2, 1, &[0u8; 16]);
        // Total file layout: 4 magic + 124 header. DDS_PIXELFORMAT sits
        // at offset 4 + 72 = 76 (after the 8 leading DDS_HEADER fields
        // and reserved1[11]). Within DDS_PIXELFORMAT the masks start at
        // pf+16: size(4) flags(4) fourcc(4) bits(4) rmask(4) gmask(4)
        // bmask(4) amask(4). So r_mask is at file offset 76+16 = 92.
        let r = u32::from_le_bytes(dds[92..96].try_into().unwrap());
        let g = u32::from_le_bytes(dds[96..100].try_into().unwrap());
        let b = u32::from_le_bytes(dds[100..104].try_into().unwrap());
        let a = u32::from_le_bytes(dds[104..108].try_into().unwrap());
        assert_eq!(r, 0x000000FF);
        assert_eq!(g, 0x0000FF00);
        assert_eq!(b, 0x00FF0000);
        assert_eq!(a, 0xFF000000);
    }

    #[test]
    fn mip_count_propagates_to_flags_and_caps() {
        let dds = encode_dds(DdsFormat::Dxt5, 16, 16, 5, &[0u8; 1024]);
        let flags = u32::from_le_bytes(dds[8..12].try_into().unwrap());
        assert!(flags & DDSD_MIPMAPCOUNT != 0);
        let mip_count = u32::from_le_bytes(dds[28..32].try_into().unwrap());
        assert_eq!(mip_count, 5);
        // caps sits at file offset 4 + 104 = 108 — after DDS_PIXELFORMAT
        // (76..108).
        let caps = u32::from_le_bytes(dds[108..112].try_into().unwrap());
        assert!(caps & DDSCAPS_MIPMAP != 0);
        assert!(caps & DDSCAPS_COMPLEX != 0);
    }

    #[test]
    fn is_dds_detects_magic_only() {
        assert!(is_dds(b"DDS \x7C\x00\x00\x00"));
        assert!(!is_dds(b"\x89RW4short"));
        assert!(!is_dds(b"too"));
    }

    #[test]
    fn dxt1_encode_then_decode_round_trips_dims() {
        // Encode a flat 8×8 RGBA block to DXT1, wrap in DDS, decode back.
        let w = 8u32;
        let h = 8u32;
        let rgba_in = vec![0x40u8; (w * h * 4) as usize];
        let fmt = texpresso::Format::Bc1;
        let mut bc = vec![0u8; fmt.compressed_size(w as usize, h as usize)];
        fmt.compress(
            &rgba_in,
            w as usize,
            h as usize,
            texpresso::Params::default(),
            &mut bc,
        );
        let dds = encode_dds(DdsFormat::Dxt1, w, h, 1, &bc);

        let decoded = decode_dds_to_rgba(&dds).expect("decode DXT1 DDS");
        assert_eq!(decoded.width, w);
        assert_eq!(decoded.height, h);
        assert_eq!(decoded.rgba.len(), (w * h * 4) as usize);
    }

    #[test]
    fn decode_rejects_bad_input() {
        assert!(matches!(
            decode_dds_to_rgba(&[0u8; 16]),
            Err(DdsDecodeError::TooShort(16))
        ));
        let mut not_dds = vec![0u8; 200];
        not_dds[..4].copy_from_slice(b"XXXX");
        assert!(matches!(
            decode_dds_to_rgba(&not_dds),
            Err(DdsDecodeError::BadMagic)
        ));
        // Valid header, unsupported fourcc (uncompressed RGBA).
        let rgba_dds = encode_dds(DdsFormat::Rgba32, 4, 4, 1, &[0u8; 64]);
        assert!(matches!(
            decode_dds_to_rgba(&rgba_dds),
            Err(DdsDecodeError::UnsupportedFourcc(_))
        ));
    }
}
