//! Xbox 360 `XPR2` / `TX2D` texture container.
//!
//! NHL Legacy's Xbox 360 front-end stores each player portrait as a
//! Scaleform GFx/APT bundle inside an EA `BIGF` archive. The bundle's
//! largest entry is a RefPack-compressed `XPR2` package whose single
//! `TX2D` resource is an Xbox-360-tiled DXT texture. This is the Xbox
//! analogue of the PS3 front-end, which stores a plain little-endian
//! `DDS` in the same archive slot (see `tdb-gui-api::portrait_replace`).
//!
//! The two encodings round-trip identically — same `BIGF`, same GFx
//! structure, same RefPack — so the *only* platform difference is this
//! inner texture wrapper. This module is the missing piece: it decodes
//! the tiled pixels for preview/export and rewrites them in place for a
//! same-format / same-resolution portrait import.
//!
//! ## Layout (big-endian, all offsets confirmed against 3025 real
//! `playerheadssmall` portraits — every one `256×128 DXT5`)
//!
//! ```text
//!  0x00  "XPR2"
//!  0x04  info_size   (u32)  — size of the info block AFTER the 12-byte
//!                             prefix; pixel data starts at 12+info_size
//!  0x08  data_size   (u32)  — tiled pixel-payload byte count
//!  0x0C  resource_count (u32, = 1)
//!  0x10  "TX2D"             — resource type tag
//!  …     GFx resource directory + name ("strName")
//!  …     a real Xbox `GPUTEXTURE_FETCH_CONSTANT`, whose dimension /
//!        format DWORDs match `texlib::make_descriptor`'s DW7–DW10:
//!          DW8  (format|base_addr) at  pixel_start − 0x14
//!          DW9  (2D size)          at  pixel_start − 0x10
//!          DW10 (filter/mip word)  at  pixel_start − 0x0C
//!  pixel_start = 12 + info_size
//!
//! DW10 varies by texture (single-mip portraits = 0x00000D10, mipmapped
//! team logos = 0x00000C14), so the descriptor is located structurally
//! and validated by the format byte + the `data_size` cross-check rather
//! than by a fixed DW10 anchor.
//!  pixel_start  … +data_size       — tiled + 16-bit-pair-swapped DXT
//! ```
//!
//! The pixel region is byte-for-byte the same tiled, `swap16`'d DXT the
//! `.rx2` texlib path uses, so [`crate::texlib::tile_dxt`] /
//! [`crate::texlib::untile_dxt`] / [`crate::texlib::swap16_in_place`]
//! apply unchanged.

use crate::texlib::{
    base_mip_byte_size, swap16_in_place, tile, tile_dxt, tiled_buffer_size,
    tiled_buffer_size_texels, untile, untile_dxt, PixelFormat,
};

/// `XPR2` magic at offset 0.
pub const XPR2_MAGIC: &[u8; 4] = b"XPR2";
/// Resource-type tag at offset 0x10 for a 2-D texture package.
pub const TX2D_TAG: &[u8; 4] = b"TX2D";

/// Bytes per uncompressed `8_8_8_8` texel.
const ARGB8888_TEXEL: usize = 4;

/// Pixel format of an `XPR2`/`TX2D` resource. The low 6 bits of the
/// descriptor's DW8 format byte are the Xenos `GPUTEXTUREFORMAT`:
/// `0x12/0x13/0x14` are the DXT family (stored as `0x52/0x53/0x54`
/// once the `0x40` 8in16-endian bits are OR'd in), while `0x06` is the
/// uncompressed `8_8_8_8` used by FE team / league / division logos
/// (stored as `0x86` with the `0x80` 8in32-endian bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XprFormat {
    /// A DXT-compressed texture (player portraits).
    Dxt(PixelFormat),
    /// Uncompressed 32-bit `GPUTEXTUREFORMAT_8_8_8_8` (FE logos).
    Argb8888,
}

impl XprFormat {
    /// The DXT pixel format when this is a compressed texture, else
    /// `None` (the uncompressed `8_8_8_8` case).
    pub fn as_dxt(self) -> Option<PixelFormat> {
        match self {
            XprFormat::Dxt(f) => Some(f),
            XprFormat::Argb8888 => None,
        }
    }

    /// Map the descriptor's DW8 low byte to a format. Returns `None`
    /// for any `GPUTEXTUREFORMAT` we don't decode.
    fn from_dw8_byte(byte: u8) -> Option<Self> {
        match byte & 0x3F {
            0x12 => Some(XprFormat::Dxt(PixelFormat::Dxt1)),
            0x13 => Some(XprFormat::Dxt(PixelFormat::Dxt3)),
            0x14 => Some(XprFormat::Dxt(PixelFormat::Dxt5)),
            0x06 => Some(XprFormat::Argb8888),
            _ => None,
        }
    }
}

/// Minimum header span we touch (12-byte prefix + the 0x54 info block
/// every observed FE portrait carries). Anything shorter cannot hold a
/// `TX2D` descriptor.
const MIN_HEADER: usize = 0x60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XprError {
    /// Buffer is shorter than the minimum `XPR2` + descriptor header.
    TooShort { len: usize },
    /// Magic at offset 0 was not `XPR2`.
    BadMagic([u8; 4]),
    /// Resource tag at offset 0x10 was not `TX2D` (only 2-D textures
    /// are handled — the FE portrait case).
    NotTx2d([u8; 4]),
    /// `pixel_start + data_size` runs past the buffer.
    PixelRegionOutOfBounds {
        pixel_start: usize,
        data_size: usize,
        total: usize,
    },
    /// Format byte in DW8 is not a DXT variant we encode.
    UnknownFormat(u8),
    /// `data_size` disagrees with the tiled size implied by the parsed
    /// dimensions + format — guards against a mis-located descriptor.
    SizeCrossCheckFailed {
        width: u32,
        height: u32,
        format: XprFormat,
        data_size: usize,
        expected_tiled: usize,
    },
    /// Replacement linear DXT didn't match the slot's base-mip byte size.
    ReplacementSizeMismatch { expected: usize, have: usize },
    /// A write path (`replace_*`) was called on an uncompressed
    /// `8_8_8_8` package — only DXT replace is implemented (FE logos
    /// are decode-only for now).
    ReplaceUnsupportedFormat { format: XprFormat },
}

impl std::fmt::Display for XprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XprError::TooShort { len } => {
                write!(f, "XPR2 buffer too short ({len} bytes, need ≥ {MIN_HEADER})")
            }
            XprError::BadMagic(m) => write!(f, "bad XPR2 magic {m:02x?} (expected \"XPR2\")"),
            XprError::NotTx2d(t) => {
                write!(f, "resource tag {t:02x?} is not \"TX2D\" — unsupported XPR2 resource")
            }
            XprError::PixelRegionOutOfBounds {
                pixel_start,
                data_size,
                total,
            } => write!(
                f,
                "pixel region {pixel_start}..{} runs past XPR2 buffer ({total} bytes)",
                pixel_start + data_size
            ),
            XprError::UnknownFormat(b) => {
                write!(f, "unsupported XPR2 texture format byte 0x{b:02x}")
            }
            XprError::SizeCrossCheckFailed {
                width,
                height,
                format,
                data_size,
                expected_tiled,
            } => write!(
                f,
                "XPR2 size cross-check failed: {width}×{height} {format:?} implies {expected_tiled} \
                 tiled bytes but data_size = {data_size}"
            ),
            XprError::ReplacementSizeMismatch { expected, have } => write!(
                f,
                "replacement base-mip is {have} bytes, slot expects {expected}"
            ),
            XprError::ReplaceUnsupportedFormat { format } => write!(
                f,
                "XPR2 replace is only implemented for DXT formats, not {format:?}"
            ),
        }
    }
}

impl std::error::Error for XprError {}

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// A parsed `XPR2`/`TX2D` package. Holds the original bytes verbatim so
/// a replace can preserve the header + GPU descriptor byte-for-byte and
/// rewrite only the tiled pixel region.
#[derive(Debug, Clone)]
pub struct Xpr2Texture {
    pub width: u32,
    pub height: u32,
    pub format: XprFormat,
    /// Byte offset where the tiled pixel payload begins (`12 + info_size`).
    pixel_start: usize,
    /// Tiled pixel-payload byte count (`data_size`).
    data_size: usize,
    /// Original package bytes (header + descriptor + tiled pixels).
    raw: Vec<u8>,
}

impl Xpr2Texture {
    /// Parse a decompressed `XPR2` package (RefPack already stripped).
    pub fn parse(raw: Vec<u8>) -> Result<Self, XprError> {
        if raw.len() < MIN_HEADER {
            return Err(XprError::TooShort { len: raw.len() });
        }
        if &raw[0..4] != XPR2_MAGIC {
            let mut m = [0u8; 4];
            m.copy_from_slice(&raw[0..4]);
            return Err(XprError::BadMagic(m));
        }
        if &raw[16..20] != TX2D_TAG {
            let mut t = [0u8; 4];
            t.copy_from_slice(&raw[16..20]);
            return Err(XprError::NotTx2d(t));
        }

        let info_size = be32(&raw, 4) as usize;
        let data_size = be32(&raw, 8) as usize;
        let pixel_start = 12usize.saturating_add(info_size);

        // The descriptor sits in the 0x14 bytes immediately before the
        // pixels; guard the back-reads and the pixel region.
        if pixel_start < 0x14
            || pixel_start
                .checked_add(data_size)
                .map_or(true, |end| end > raw.len())
        {
            return Err(XprError::PixelRegionOutOfBounds {
                pixel_start,
                data_size,
                total: raw.len(),
            });
        }

        // DW10 (at pixel_start − 0x0C) is the Xenos fetch-constant
        // filter / mip-clamp word, which legitimately varies by texture:
        // single-mip FE portraits carry 0x00000D10 while mipmapped team
        // logos carry 0x00000C14. It is NOT a fixed anchor, so we don't
        // gate on it. The descriptor is instead validated structurally
        // by the format-byte check plus the `data_size` cross-check
        // below — if `pixel_start` were mislocated, the parsed geometry
        // wouldn't reproduce `data_size` and parse would fail there.
        let format_byte = (be32(&raw, pixel_start - 0x14) & 0xFF) as u8;
        let format =
            XprFormat::from_dw8_byte(format_byte).ok_or(XprError::UnknownFormat(format_byte))?;

        // DW9 packs `(height/8 − 1)` in byte 0x05 and a width word in
        // the low 13 bits — inverse of `texlib::make_descriptor`.
        let dw9 = be32(&raw, pixel_start - 0x10);
        let height = (((dw9 >> 16) & 0xFF) + 1) * 8;
        let width = (dw9 & 0x1FFF) + 1;

        // Cross-check the parsed geometry against the declared payload
        // size. If the descriptor were misread, this catches it before
        // any write touches real pixels. DXT tiles by 4×4 block; the
        // uncompressed `8_8_8_8` path tiles per 4-byte texel.
        let expected_tiled = match format {
            XprFormat::Dxt(f) => tiled_buffer_size(width, height, f.bytes_per_block()),
            XprFormat::Argb8888 => tiled_buffer_size_texels(width, height, ARGB8888_TEXEL),
        };
        if expected_tiled != data_size {
            return Err(XprError::SizeCrossCheckFailed {
                width,
                height,
                format,
                data_size,
                expected_tiled,
            });
        }

        Ok(Xpr2Texture {
            width,
            height,
            format,
            pixel_start,
            data_size,
            raw,
        })
    }

    /// The tiled, `swap16`'d pixel payload exactly as stored.
    pub fn tiled_pixels(&self) -> &[u8] {
        &self.raw[self.pixel_start..self.pixel_start + self.data_size]
    }

    /// Decode the base mip to a **linear** (untiled) stream.
    ///
    /// For DXT this is the little-endian-block stream a standard `DDS`
    /// carries (consumed by the `tdb-archive-rpsgl` DXT decoders). For
    /// the uncompressed `8_8_8_8` case it's a row-major `[A, R, G, B]`
    /// byte stream — exactly what the `tdb-archive-rpsgl` A8R8G8B8
    /// decoder expects.
    pub fn decode_base_mip_linear(&self) -> Vec<u8> {
        let mut t = self.tiled_pixels().to_vec();
        match self.format {
            XprFormat::Dxt(f) => {
                // Stored order is tiled → swap16; reverse is swap16 → untile.
                swap16_in_place(&mut t);
                untile_dxt(&t, self.width, self.height, f.bytes_per_block())
            }
            XprFormat::Argb8888 => {
                // Stored as tiled 4-byte texels already in big-endian
                // `[A, R, G, B]` byte order (verified against the PS3 DDS
                // ground truth: an on-disk texel `[255, 232, 203, 30]`
                // decodes to RGBA `[232, 203, 30, 255]`). No endian swap
                // is needed — just untile; the result feeds the
                // `tdb-archive-rpsgl` A8R8G8B8 decoder directly.
                untile(&t, self.width, self.height, ARGB8888_TEXEL)
            }
        }
    }

    /// Rewrite the texture at **new dimensions** (same format),
    /// regenerating the geometry fields of the embedded GPU descriptor
    /// and the package `data_size` in place. Use this to e.g. upgrade a
    /// 512×256 menu slot to 512×512 so a square source portrait drops
    /// in cleanly.
    ///
    /// Only the dimension-derived fields change — `data_size`, the DW7
    /// pitch byte, and the DW9 height-byte / width-word — which is the
    /// exact inverse of [`crate::texlib::make_descriptor`]'s encoding.
    /// Comparing a real EA 512×256 portrait against a real 512×512 one
    /// confirms these are the *only* header bytes that differ, so the
    /// patched header is byte-identical to EA's own (see the
    /// `resize_*` tests). The info-block size is dimension-independent,
    /// so `pixel_start` is unchanged; only the trailing pixel payload
    /// grows or shrinks.
    pub fn replace_resized(
        &self,
        new_width: u32,
        new_height: u32,
        linear: &[u8],
    ) -> Result<Vec<u8>, XprError> {
        // The descriptor geometry fields sit at fixed negative offsets
        // from pixel_start; DW7 is the deepest at −0x18.
        if self.pixel_start < 0x18 {
            return Err(XprError::TooShort {
                len: self.pixel_start,
            });
        }
        let dxt = self
            .format
            .as_dxt()
            .ok_or(XprError::ReplaceUnsupportedFormat {
                format: self.format,
            })?;
        let bpb = dxt.bytes_per_block();
        let expected = base_mip_byte_size(dxt, new_width, new_height);
        if linear.len() != expected {
            return Err(XprError::ReplacementSizeMismatch {
                expected,
                have: linear.len(),
            });
        }
        let new_data_size = tiled_buffer_size(new_width, new_height, bpb);

        // Start from the existing header (identical info layout) and
        // repatch only the geometry-derived fields.
        let mut header = self.raw[..self.pixel_start].to_vec();
        // data_size @ 0x08 (BE u32) — the tiled pixel-payload length.
        header[8..12].copy_from_slice(&(new_data_size as u32).to_be_bytes());
        // DW7 byte @ pixel_start−0x18 = 0x80 | (width / 128).
        header[self.pixel_start - 0x18] = 0x80 | ((new_width / 128).min(0x7F) as u8);
        // DW9 @ pixel_start−0x10: byte[+1] = height/8 − 1; u16[+2] = width word.
        let dw9 = self.pixel_start - 0x10;
        header[dw9] = 0x00;
        header[dw9 + 1] = ((new_height / 8).saturating_sub(1)).min(0xFF) as u8;
        let width_word: u16 = 0xE000 | ((new_width.saturating_sub(1)) as u16 & 0x1FFF);
        header[dw9 + 2..dw9 + 4].copy_from_slice(&width_word.to_be_bytes());

        let mut tiled = tile_dxt(linear, new_width, new_height, bpb);
        swap16_in_place(&mut tiled);
        if tiled.len() != new_data_size {
            return Err(XprError::ReplacementSizeMismatch {
                expected: new_data_size,
                have: tiled.len(),
            });
        }
        header.extend_from_slice(&tiled);
        Ok(header)
    }

    /// Rewrite the base mip from a **linear** DXT stream of the slot's
    /// own format/size, returning the full new `XPR2` package. The
    /// header + GPU descriptor are preserved verbatim, so the result is
    /// a drop-in same-format / same-resolution replacement.
    pub fn replace_base_mip_linear(&self, linear: &[u8]) -> Result<Vec<u8>, XprError> {
        // Produce the slot's tiled pixel payload from the operator's
        // linear base mip. DXT: tile by 4×4 block + 16-bit-pair swap.
        // Uncompressed `8_8_8_8` (FE logos): tile per 4-byte texel with
        // no endian swap (stored big-endian `[A,R,G,B]`, the same order
        // the linear input carries).
        let tiled = match self.format {
            XprFormat::Dxt(dxt) => {
                let expected = base_mip_byte_size(dxt, self.width, self.height);
                if linear.len() != expected {
                    return Err(XprError::ReplacementSizeMismatch {
                        expected,
                        have: linear.len(),
                    });
                }
                let mut t = tile_dxt(linear, self.width, self.height, dxt.bytes_per_block());
                swap16_in_place(&mut t);
                t
            }
            XprFormat::Argb8888 => {
                let expected = (self.width as usize) * (self.height as usize) * ARGB8888_TEXEL;
                if linear.len() != expected {
                    return Err(XprError::ReplacementSizeMismatch {
                        expected,
                        have: linear.len(),
                    });
                }
                tile(linear, self.width, self.height, ARGB8888_TEXEL)
            }
        };
        // For the FE portrait/logo dims the tiled size equals the slot
        // size, but assert it rather than trust it — a mismatch would
        // corrupt the package.
        if tiled.len() != self.data_size {
            return Err(XprError::ReplacementSizeMismatch {
                expected: self.data_size,
                have: tiled.len(),
            });
        }
        let mut out = self.raw.clone();
        out[self.pixel_start..self.pixel_start + self.data_size].copy_from_slice(&tiled);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Load the bundled real portrait XPR2 (a decompressed
    /// `playerheadssmall/p1512.big` entry). Skips gracefully when the
    /// fixture isn't checked out.
    fn load_fixture() -> Option<Vec<u8>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("Working Assets")
            .join("Template Texture")
            .join("NHL Legacy")
            .join("playerheadssmall_p1512.xpr2");
        p.is_file().then(|| std::fs::read(&p).ok()).flatten()
    }

    #[test]
    fn parses_real_portrait_geometry() {
        let Some(bytes) = load_fixture() else {
            eprintln!("skipping: playerheadssmall_p1512.xpr2 fixture not present");
            return;
        };
        let tex = Xpr2Texture::parse(bytes).expect("real FE portrait must parse");
        assert_eq!(tex.width, 256);
        assert_eq!(tex.height, 128);
        assert_eq!(tex.format, XprFormat::Dxt(PixelFormat::Dxt5));
        // 256×128 DXT5 base mip = 64×32 blocks × 16 B = 32768.
        assert_eq!(tex.tiled_pixels().len(), 32768);
    }

    /// The load-bearing guarantee for the importer: decode → re-encode
    /// the *same* linear bytes must reproduce the original package byte
    /// for byte. Proves tile∘untile and swap16∘swap16 are exact inverses
    /// at these dims AND that the header/descriptor are preserved.
    #[test]
    fn decode_then_replace_is_identity() {
        let Some(bytes) = load_fixture() else {
            return;
        };
        let original = bytes.clone();
        let tex = Xpr2Texture::parse(bytes).unwrap();
        let linear = tex.decode_base_mip_linear();
        // 256×128 DXT5 linear base mip == 32768 (no tiling padding here).
        assert_eq!(linear.len(), 32768);
        let rebuilt = tex.replace_base_mip_linear(&linear).unwrap();
        assert_eq!(rebuilt, original, "round-trip must be byte-identical");
    }

    /// A different base mip must actually change the pixels but leave
    /// the header/descriptor (everything before `pixel_start`) intact.
    #[test]
    fn replace_changes_only_pixels() {
        let Some(bytes) = load_fixture() else {
            return;
        };
        let tex = Xpr2Texture::parse(bytes).unwrap();
        let mut linear = tex.decode_base_mip_linear();
        linear[0] ^= 0xFF; // perturb one block
        let rebuilt = tex.replace_base_mip_linear(&linear).unwrap();
        assert_eq!(
            &rebuilt[..tex.pixel_start],
            &tex.raw[..tex.pixel_start],
            "header + descriptor must be untouched"
        );
        assert_ne!(
            &rebuilt[tex.pixel_start..],
            tex.tiled_pixels(),
            "pixel region must reflect the change"
        );
        // And it must re-parse to the same geometry.
        let re = Xpr2Texture::parse(rebuilt).unwrap();
        assert_eq!(
            (re.width, re.height, re.format),
            (256, 128, XprFormat::Dxt(PixelFormat::Dxt5))
        );
    }

    /// Resizing the bundled 256×128 portrait to 256×256 must patch the
    /// geometry fields so the result re-parses at the new dims, and the
    /// `data_size` + DW9 height byte must reflect 256×256.
    #[test]
    fn resize_repatches_geometry_and_reparses() {
        let Some(bytes) = load_fixture() else {
            return;
        };
        let tex = Xpr2Texture::parse(bytes).unwrap();
        assert_eq!((tex.width, tex.height), (256, 128));
        // 256×256 DXT5 base mip = 64×64 blocks × 16 B = 65536.
        let dxt = tex.format.as_dxt().unwrap();
        let linear = vec![0u8; base_mip_byte_size(dxt, 256, 256)];
        let resized = tex.replace_resized(256, 256, &linear).unwrap();

        // data_size @ 0x08 must now be the 256×256 tiled size (0x10000).
        let data_size = u32::from_be_bytes([resized[8], resized[9], resized[10], resized[11]]);
        assert_eq!(
            data_size as usize,
            tiled_buffer_size(256, 256, dxt.bytes_per_block())
        );
        // DW9 height byte @ pixel_start−0x0F = 256/8 − 1 = 31.
        assert_eq!(resized[tex.pixel_start - 0x0F], 31);

        let re = Xpr2Texture::parse(resized).unwrap();
        assert_eq!(
            (re.width, re.height, re.format),
            (256, 256, XprFormat::Dxt(PixelFormat::Dxt5))
        );
    }

    /// A no-op resize (same dims) must reproduce the original header
    /// bytes exactly — proves the geometry patch writes back the same
    /// values the parser read.
    #[test]
    fn resize_to_same_dims_preserves_header() {
        let Some(bytes) = load_fixture() else {
            return;
        };
        let original = bytes.clone();
        let tex = Xpr2Texture::parse(bytes).unwrap();
        let linear = tex.decode_base_mip_linear();
        let out = tex.replace_resized(tex.width, tex.height, &linear).unwrap();
        assert_eq!(out, original, "same-dims resize must be byte-identical");
    }

    #[test]
    fn rejects_bad_magic() {
        let mut b = vec![0u8; MIN_HEADER + 16];
        b[0..4].copy_from_slice(b"NOPE");
        assert!(matches!(Xpr2Texture::parse(b), Err(XprError::BadMagic(_))));
    }

    #[test]
    fn rejects_too_short() {
        assert!(matches!(
            Xpr2Texture::parse(vec![0u8; 8]),
            Err(XprError::TooShort { .. })
        ));
    }

    #[test]
    fn dw8_byte_maps_dxt_and_argb_formats() {
        // DXT family (stored with the 0x40 bits) → DXT variants.
        assert_eq!(
            XprFormat::from_dw8_byte(0x52),
            Some(XprFormat::Dxt(PixelFormat::Dxt1))
        );
        assert_eq!(
            XprFormat::from_dw8_byte(0x53),
            Some(XprFormat::Dxt(PixelFormat::Dxt3))
        );
        assert_eq!(
            XprFormat::from_dw8_byte(0x54),
            Some(XprFormat::Dxt(PixelFormat::Dxt5))
        );
        // Uncompressed 8_8_8_8 (stored as 0x86 — FE team logos).
        assert_eq!(XprFormat::from_dw8_byte(0x86), Some(XprFormat::Argb8888));
        // Unknown GPU format.
        assert_eq!(XprFormat::from_dw8_byte(0x99), None);
    }

    /// Build a minimal `XPR2`/`TX2D` package for an uncompressed 8×8
    /// `8_8_8_8` texture, then confirm the parser detects the format,
    /// recovers the geometry, and decodes the right number of `[A,R,G,B]`
    /// bytes. Exercises the team-logo path without a binary fixture.
    #[test]
    fn parses_synthetic_argb8888_package() {
        let info_size: u32 = 0x54;
        let pixel_start = 12 + info_size as usize; // 0x60
        let (w, h) = (8u32, 8u32);
        let data_size = tiled_buffer_size_texels(w, h, ARGB8888_TEXEL);
        let mut raw = vec![0u8; pixel_start + data_size];
        raw[0..4].copy_from_slice(XPR2_MAGIC);
        raw[4..8].copy_from_slice(&info_size.to_be_bytes());
        raw[8..12].copy_from_slice(&(data_size as u32).to_be_bytes());
        raw[16..20].copy_from_slice(TX2D_TAG);
        // DW8 @ pixel_start − 0x14: low byte = format 0x86.
        raw[pixel_start - 0x14 + 3] = 0x86;
        // DW9 @ pixel_start − 0x10: height byte = h/8 − 1 = 0; width word
        // low 13 bits = w − 1 = 7.
        let dw9: u32 = ((h / 8 - 1) << 16) | (w - 1);
        raw[pixel_start - 0x10..pixel_start - 0x0C].copy_from_slice(&dw9.to_be_bytes());

        let tex = Xpr2Texture::parse(raw).expect("synthetic 8888 package must parse");
        assert_eq!((tex.width, tex.height), (8, 8));
        assert_eq!(tex.format, XprFormat::Argb8888);
        // Decode yields w×h 4-byte texels, no endian swap applied.
        let linear = tex.decode_base_mip_linear();
        assert_eq!(linear.len(), (w * h * 4) as usize);
    }

    /// 32-bit `8_8_8_8` write round-trip: build a package whose pixel
    /// region carries a known `[A,R,G,B]` texel pattern, parse it,
    /// decode → linear, then `replace_base_mip_linear` with that same
    /// linear must reproduce the original bytes exactly (proves
    /// tile∘untile cancel and no spurious endian swap is applied). A
    /// modified mip must change the pixels and re-parse at the same
    /// geometry.
    #[test]
    fn argb8888_decode_replace_round_trip() {
        use crate::texlib::tile;
        let info_size: u32 = 0x54;
        let pixel_start = 12 + info_size as usize;
        let (w, h) = (8u32, 8u32);
        let data_size = tiled_buffer_size_texels(w, h, ARGB8888_TEXEL);
        // A distinct [A,R,G,B] per texel so any channel/position bug shows.
        let mut linear = vec![0u8; (w * h * 4) as usize];
        for (i, px) in linear.chunks_exact_mut(4).enumerate() {
            px[0] = 0xFF; // A
            px[1] = i as u8; // R
            px[2] = (i * 3) as u8; // G
            px[3] = (i * 7) as u8; // B
        }
        let mut raw = vec![0u8; pixel_start + data_size];
        raw[0..4].copy_from_slice(XPR2_MAGIC);
        raw[4..8].copy_from_slice(&info_size.to_be_bytes());
        raw[8..12].copy_from_slice(&(data_size as u32).to_be_bytes());
        raw[16..20].copy_from_slice(TX2D_TAG);
        raw[pixel_start - 0x14 + 3] = 0x86;
        let dw9: u32 = ((h / 8 - 1) << 16) | (w - 1);
        raw[pixel_start - 0x10..pixel_start - 0x0C].copy_from_slice(&dw9.to_be_bytes());
        // Seed the pixel region with the tiled form of `linear`.
        let tiled = tile(&linear, w, h, ARGB8888_TEXEL);
        raw[pixel_start..pixel_start + data_size].copy_from_slice(&tiled);

        let tex = Xpr2Texture::parse(raw.clone()).unwrap();
        let decoded = tex.decode_base_mip_linear();
        assert_eq!(
            decoded, linear,
            "decode must recover the linear [A,R,G,B] texels"
        );
        let rebuilt = tex.replace_base_mip_linear(&decoded).unwrap();
        assert_eq!(rebuilt, raw, "decode→replace identity must be byte-exact");

        // A changed mip must alter the pixels and re-parse identically.
        let mut linear2 = linear.clone();
        linear2[5] ^= 0xFF;
        let changed = tex.replace_base_mip_linear(&linear2).unwrap();
        assert_ne!(changed, raw, "modified mip must change the bytes");
        let re = Xpr2Texture::parse(changed).unwrap();
        assert_eq!(
            (re.width, re.height, re.format),
            (8, 8, XprFormat::Argb8888)
        );
        assert_eq!(re.decode_base_mip_linear(), linear2);
    }
}
