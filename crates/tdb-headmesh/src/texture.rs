//! Per-target texture builds for converted heads.
//!
//! RW (Legacy-family) invariants, learned in-game the hard way:
//! * head raster is the donor texlib's `*_cm` head slot: 512×512 DXT1; hair is the
//!   256×256 DXT3 slot;
//! * the finished texlib is **exactly [`LEGACY_TEXLIB_BYTES`] bytes** — same buffers and
//!   descriptor shape as vanilla (`texlib_67.rx2`);
//! * the full mip chain is regenerated from the new texture down to 16×16 (a kept donor
//!   mip tail shows the donor's face at distance);
//! * **never upscale** — a 1024 head texlib (~1.6 MB) crashes the game; the 512 cap is an
//!   engine loader-buffer limit, not a format limit.
//!
//! Codec reuse: DDS decode + BC encode come from `tdb-archive-codec` (texpresso), the
//! texlib container surgery from `tdb_rx2::texlib::replace_with_resize`. Raster ids keep
//! the donor's names (`head_67_cm` …) — the mesh's materials reference rasters by name,
//! so donor-consistent naming is what renders; the prior effort shipped this way
//! in-game.

use tdb_archive_codec::dds::{decode_dds_to_rgba, DdsDecodeError};
use tdb_archive_codec::{compress_bc, DdsFormat};
use tdb_rx2::texlib::{self, LinearMip, PixelFormat, TexlibError};

pub const LEGACY_TEXLIB_BYTES: usize = 331_776;
pub const LEGACY_HEAD_DIMS: (u32, u32) = (512, 512);
pub const LEGACY_HAIR_DIMS: (u32, u32) = (256, 256);
/// Smallest mip level generated (matches the prior effort's chains).
pub const MIN_MIP: u32 = 16;

#[derive(Debug, thiserror::Error)]
pub enum TextureError {
    #[error("texlib: {0}")]
    Texlib(#[from] TexlibError),
    #[error("source texture: {0}")]
    Dds(#[from] DdsDecodeError),
    #[error("source texture is not a decodable PNG/JPEG: {0}")]
    Image(String),
    #[error("donor texlib has no {0} raster (rasters: {1})")]
    MissingSlot(&'static str, String),
    #[error(
        "built texlib is {actual} bytes, expected exactly {LEGACY_TEXLIB_BYTES} — refusing an \
         out-of-shape texlib (oversize texlibs crash the game)"
    )]
    BadTexlibSize { actual: usize },
}

/// Bilinear-resize an RGBA8 image. Downscale-only in practice (NHL24 sources are
/// 1024/2048; the cap is the vanilla slot size), but correct either way.
pub fn resize_rgba(rgba: &[u8], w: u32, h: u32, nw: u32, nh: u32) -> Vec<u8> {
    if w == nw && h == nh {
        return rgba.to_vec();
    }
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        let sy = (y as f32 + 0.5) * h as f32 / nh as f32 - 0.5;
        let y0 = sy.floor().clamp(0.0, (h - 1) as f32) as u32;
        let y1 = (y0 + 1).min(h - 1);
        let fy = (sy - y0 as f32).clamp(0.0, 1.0);
        for x in 0..nw {
            let sx = (x as f32 + 0.5) * w as f32 / nw as f32 - 0.5;
            let x0 = sx.floor().clamp(0.0, (w - 1) as f32) as u32;
            let x1 = (x0 + 1).min(w - 1);
            let fx = (sx - x0 as f32).clamp(0.0, 1.0);
            let px = |xx: u32, yy: u32, c: usize| rgba[((yy * w + xx) * 4) as usize + c] as f32;
            for c in 0..4 {
                let top = px(x0, y0, c) * (1.0 - fx) + px(x1, y0, c) * fx;
                let bot = px(x0, y1, c) * (1.0 - fx) + px(x1, y1, c) * fx;
                out[((y * nw + x) * 4) as usize + c] = (top * (1.0 - fy) + bot * fy).round() as u8;
            }
        }
    }
    out
}

/// Box-downsample by 2 (used mip-to-mip; dims must be even, callers halve from a
/// power-of-two base).
fn half(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (nw, nh) = (w / 2, h / 2);
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        for x in 0..nw {
            for c in 0..4 {
                let mut sum = 0u32;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    sum += rgba[(((y * 2 + dy) * w + x * 2 + dx) * 4) as usize + c] as u32;
                }
                out[((y * nw + x) * 4) as usize + c] = (sum / 4) as u8;
            }
        }
    }
    out
}

/// RGBA base → DXT-encoded mip chain (base included) down to [`MIN_MIP`].
fn encode_mip_chain(rgba: Vec<u8>, w: u32, h: u32, format: DdsFormat) -> Vec<(u32, u32, Vec<u8>)> {
    let mut mips = Vec::new();
    let mut cur = rgba;
    let (mut cw, mut ch) = (w, h);
    loop {
        mips.push((cw, ch, compress_bc(format, cw, ch, &cur)));
        if cw <= MIN_MIP || ch <= MIN_MIP {
            break;
        }
        cur = half(&cur, cw, ch);
        cw /= 2;
        ch /= 2;
    }
    mips
}

fn find_slot(lib: &texlib::Texlib, keyword: &str) -> Option<usize> {
    lib.textures
        .iter()
        .position(|t| t.name.to_ascii_lowercase().contains(keyword))
}

/// Decode a source image (DDS today; the studio-side path can hand PNG-decoded RGBA
/// directly via [`build_legacy_texlib_rgba`]).
pub fn decode_source_dds(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), TextureError> {
    let d = decode_dds_to_rgba(bytes)?;
    Ok((d.rgba, d.width, d.height))
}

/// Decode a source image by sniffing its container: DDS (NHL24/RW sources) or PNG/JPEG
/// (CC5 exports its skin atlas as a 4K PNG). Returns RGBA8 + dimensions, which is what
/// [`build_legacy_texlib_rgba`] consumes — resizing to the slot happens there.
pub fn decode_source_image(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), TextureError> {
    if bytes.starts_with(b"DDS ") {
        return decode_source_dds(bytes);
    }
    let img = image::load_from_memory(bytes)
        .map_err(|e| TextureError::Image(e.to_string()))?
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    Ok((img.into_raw(), w, h))
}

/// One texture replacement job: the source RGBA (with its dimensions), the slot name it
/// targets, the slot's fixed dimensions, and the DDS/pixel formats to encode it as.
type TextureJob<'a> = (
    Option<(&'a [u8], u32, u32)>,
    &'a str,
    (u32, u32),
    DdsFormat,
    PixelFormat,
);

/// Build a Legacy-family texlib from the donor texlib + replacement head/hair RGBA.
/// Either texture may be `None` to keep the donor's raster for that slot.
pub fn build_legacy_texlib_rgba(
    donor_texlib: &[u8],
    head_rgba: Option<(&[u8], u32, u32)>,
    hair_rgba: Option<(&[u8], u32, u32)>,
) -> Result<Vec<u8>, TextureError> {
    let mut bytes = donor_texlib.to_vec();
    let jobs: [TextureJob<'_>; 2] = [
        (
            head_rgba,
            "head",
            LEGACY_HEAD_DIMS,
            DdsFormat::Dxt1,
            PixelFormat::Dxt1,
        ),
        (
            hair_rgba,
            "hair",
            LEGACY_HAIR_DIMS,
            DdsFormat::Dxt3,
            PixelFormat::Dxt3,
        ),
    ];
    for (source, keyword, (tw, th), dds_format, pixel_format) in jobs {
        let Some((rgba, w, h)) = source else { continue };
        let lib = texlib::parse(&bytes)?;
        let idx = find_slot(&lib, keyword).ok_or_else(|| {
            TextureError::MissingSlot(
                if keyword == "head" { "head" } else { "hair" },
                lib.textures
                    .iter()
                    .map(|t| t.name.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })?;
        let resized = resize_rgba(rgba, w, h, tw, th);
        let mips = encode_mip_chain(resized, tw, th, dds_format);
        let linear: Vec<LinearMip<'_>> = mips
            .iter()
            .map(|(mw, mh, data)| LinearMip {
                width: *mw,
                height: *mh,
                linear_dxt: data,
            })
            .collect();
        bytes = lib.replace_with_resize(idx, pixel_format, &linear)?;
    }
    enforce_texlib_shape(&bytes)?;
    Ok(bytes)
}

/// Final shape check every built texlib must pass before install planning.
pub fn enforce_texlib_shape(bytes: &[u8]) -> Result<(), TextureError> {
    if bytes.len() != LEGACY_TEXLIB_BYTES {
        return Err(TextureError::BadTexlibSize {
            actual: bytes.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_gate_refuses_oversize() {
        assert!(enforce_texlib_shape(&vec![0u8; LEGACY_TEXLIB_BYTES]).is_ok());
        assert!(matches!(
            enforce_texlib_shape(&vec![0u8; 1_600_000]),
            Err(TextureError::BadTexlibSize { .. })
        ));
    }

    #[test]
    fn resize_and_mip_chain_shapes() {
        // 8×8 red → resize to 4×4 → chain would stop immediately at MIN_MIP floor;
        // use explicit small chain math instead.
        let rgba = [255u8, 0, 0, 255].repeat(64);
        let small = resize_rgba(&rgba, 8, 8, 4, 4);
        assert_eq!(small.len(), 4 * 4 * 4);
        assert_eq!(&small[0..4], &[255, 0, 0, 255]);
        let mips = encode_mip_chain(rgba, 8, 8, DdsFormat::Dxt1);
        // 8→(stops: 8 ≤ 16 already) — single mip.
        assert_eq!(mips.len(), 1);
        // DXT1: 8×8 = 4 blocks × 8 bytes.
        assert_eq!(mips[0].2.len(), 32);
        let mips512 = encode_mip_chain(vec![0u8; 512 * 512 * 4], 512, 512, DdsFormat::Dxt1);
        // 512,256,128,64,32,16 = 6 levels.
        assert_eq!(mips512.len(), 6);
        assert_eq!(mips512[5].0, 16);
    }
}
