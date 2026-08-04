//! EA archive container codecs.
//!
//! Provides the compression containers wrapping EA NHL assets:
//!
//! - `chunkzip` (chunked zlib, two header variants) and `chunklzx` (chunked LZX
//!   — detection only) — the read half the `.big` extractor needs.
//! - [`refpack`] — EA's LZ77 variant. Wraps the FE-tree portrait/logo payloads
//!   inside `fe/ion/artassets/**/*.big`.
//! - [`dds`] — DDS encode/decode, the interchange format the texture tools emit.
//!
//! The chunkzip *write* path remains unimplemented (nothing needs it yet).
//!
//! ## Format summary
//!
//! Every chunk-compressed payload starts with an 8-byte ASCII magic:
//!
//! | Magic       | Meaning                                  |
//! | ----------- | ---------------------------------------- |
//! | `chunkzip`  | Multi-chunk zlib/deflate. Two variants.  |
//! | `chunklzx`  | Multi-chunk LZX. Not implemented.        |
//! | _other_     | Uncompressed; pass through.              |
//!
//! ### `chunkzip` variants
//!
//! Distinguished by the first u32 BE after the magic, called `dummy`:
//!
//! - **`dummy <= 2`** (newer header): `fullsize` at +12, `chunk_size` at +16,
//!   `n_chunks` at +20, data starts at offset 40. Each chunk is padded so that
//!   its 4-byte `zsize` prefix lands at `offset % 16 == 8`, followed by a 4-byte
//!   gap, then the deflate payload.
//! - **`dummy > 2`** (older header): `dummy` IS `fullsize`, `chunk_size` at +12,
//!   `n_chunks` derived as `ceil(fullsize / chunk_size)`, data starts at offset
//!   16. No per-chunk padding/gap.
//!
//! Each chunk's deflate payload is tried as raw deflate first (negative window
//! bits), falling back to zlib-wrapped on failure. Output is truncated to
//! `fullsize`.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod dds;
pub mod lzx;
pub mod refpack;

pub use dds::{
    dds_base_mip, decode_dds_to_rgba, encode_dds, is_dds, DdsDecodeError, DdsFormat, DecodedDds,
};
pub use refpack::{
    is_refpack, parse_refpack_header, refpack_compress, refpack_decompress, RefpackError,
    RefpackHeader,
};

use std::io::Read;

use flate2::read::{DeflateDecoder, ZlibDecoder};
use thiserror::Error;

/// 8-byte ASCII magics that wrap EA archive payloads.
pub const MAGIC_CHUNKZIP: &[u8; 8] = b"chunkzip";
pub const MAGIC_CHUNKLZX: &[u8; 8] = b"chunklzx";
pub const MAX_DECOMPRESSED_SIZE: usize = 512 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("payload is shorter than the 8-byte container magic")]
    TruncatedMagic,
    #[error("chunkzip header is truncated: needed {needed} bytes, have {have}")]
    TruncatedHeader { needed: usize, have: usize },
    #[error(
        "chunkzip chunk is truncated: chunk {chunk_index} wants {zsize} bytes at offset {offset}, only {available} available"
    )]
    TruncatedChunk {
        chunk_index: usize,
        offset: usize,
        zsize: usize,
        available: usize,
    },
    #[error("chunkzip chunk {chunk_index}: neither raw deflate nor zlib-wrapped decode succeeded ({raw}; {zlib})")]
    DecompressFailed {
        chunk_index: usize,
        raw: std::io::Error,
        zlib: std::io::Error,
    },
    #[error("chunkzip declares {declared} decompressed bytes, exceeding the limit of {limit}")]
    SizeLimitExceeded { declared: usize, limit: usize },
    #[error("chunklzx (LZX) compression is not implemented")]
    UnsupportedLzx,
    #[error("chunklzx LZX decode failed: {0}")]
    LzxDecode(#[from] lzx::LzxError),
}

/// Returns the decompressed bytes for a `chunkzip`-wrapped payload, or the input
/// unchanged when it doesn't carry a known container magic. Returns
/// [`CodecError::UnsupportedLzx`] for `chunklzx` payloads.
pub fn decompress_if_wrapped(data: &[u8]) -> Result<Vec<u8>, CodecError> {
    if data.len() < 8 {
        return Err(CodecError::TruncatedMagic);
    }
    let magic: &[u8; 8] = data[..8].try_into().expect("len checked above");
    match magic {
        m if m == MAGIC_CHUNKZIP => decompress_chunkzip(data),
        // `decompress_chunklzx` exists but the LZX bitstream framing isn't pinned yet, so the public
        // pass-through still reports chunklzx as unsupported rather than returning wrong bytes.
        m if m == MAGIC_CHUNKLZX => Err(CodecError::UnsupportedLzx),
        _ => Ok(data.to_vec()),
    }
}

/// Decompress a `chunkzip`-wrapped payload. The caller is expected to have
/// already verified the magic (this entry point is exposed for situations where
/// the caller already knows the format).
pub fn decompress_chunkzip(data: &[u8]) -> Result<Vec<u8>, CodecError> {
    if data.len() < 16 {
        return Err(CodecError::TruncatedHeader {
            needed: 16,
            have: data.len(),
        });
    }
    let dummy = read_u32_be(data, 8)?;
    if dummy <= 2 {
        decompress_chunkzip_padded(data)
    } else {
        decompress_chunkzip_legacy(data, dummy)
    }
}

/// Compress tightly-packed RGBA8 into a **linear** DXT/BC block stream (top mip only) for the given
/// format. This is the RGBA→blocks half of the texture write path (the mirror of the `texpresso`
/// decode in [`dds`]); the caller then tiles/wraps it (e.g. `xpr::Xpr2Texture::replace_base_mip_linear`
/// or [`encode_dds`]). `Rgba32` is returned unchanged (uncompressed slots store RGBA directly).
pub fn compress_bc(format: DdsFormat, width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let fmt = match format {
        DdsFormat::Dxt1 => texpresso::Format::Bc1,
        DdsFormat::Dxt3 => texpresso::Format::Bc2,
        DdsFormat::Dxt5 => texpresso::Format::Bc3,
        DdsFormat::Rgba32 => return rgba.to_vec(),
    };
    let (w, h) = (width as usize, height as usize);
    let mut out = vec![0u8; fmt.compressed_size(w, h)];
    fmt.compress(rgba, w, h, texpresso::Params::default(), &mut out);
    out
}

/// Decompress a `chunklzx`-wrapped payload. Same padded chunk framing as `chunkzip` (dummy=2 header:
/// `fullsize`@12, `window`@16, `n_chunks`@20, data@40, each chunk's `zsize` prefix aligned to
/// `off % 16 == 8` with a 4-byte gap), but each chunk is an independent LZX stream of up to `window`
/// decompressed bytes. See [`lzx`].
pub fn decompress_chunklzx(data: &[u8]) -> Result<Vec<u8>, CodecError> {
    if data.len() < 40 {
        return Err(CodecError::TruncatedHeader { needed: 40, have: data.len() });
    }
    let fullsize = read_u32_be(data, 12)? as usize;
    validate_fullsize(fullsize)?;
    let window = read_u32_be(data, 16)?;
    let n_chunks = read_u32_be(data, 20)? as usize;
    let window_bits = window.trailing_zeros(); // window is a power of two (0x40000 → 18)

    let mut out = Vec::with_capacity(fullsize);
    let mut off = 40usize;
    for chunk_index in 0..n_chunks {
        let mut steps = 0;
        while off % 16 != 8 {
            if steps > 2 || off >= data.len() {
                return Err(CodecError::TruncatedHeader { needed: off + 4, have: data.len() });
            }
            off += 8;
            steps += 1;
        }
        let zsize = read_u32_be(data, off)? as usize;
        off += 4; // size prefix
        off += 4; // 4-byte gap
        let end = off.checked_add(zsize).ok_or(CodecError::TruncatedChunk {
            chunk_index,
            offset: off,
            zsize,
            available: data.len().saturating_sub(off),
        })?;
        if end > data.len() {
            return Err(CodecError::TruncatedChunk {
                chunk_index,
                offset: off,
                zsize,
                available: data.len().saturating_sub(off),
            });
        }
        let this = (window as usize).min(fullsize - out.len());
        out.extend_from_slice(&lzx::decompress_lzx(&data[off..end], this, window_bits)?);
        off = end;
    }
    out.truncate(fullsize);
    Ok(out)
}

/// Compress `data` into a **single-chunk padded `chunkzip`** payload (`dummy = 2` header, the same
/// header shape the game processes for these entries). Round-trips through [`decompress_chunkzip`].
///
/// Used by the write path to substitute a payload's codec (`chunklzx` → `chunkzip`) without
/// implementing LZX compression. The chunk is raw DEFLATE (the decoder tries raw deflate first).
pub fn compress_chunkzip(data: &[u8]) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
    enc.write_all(data).expect("in-memory deflate write cannot fail");
    let deflated = enc.finish().expect("in-memory deflate finish cannot fail");

    let fullsize = data.len() as u32;
    let mut out = Vec::with_capacity(48 + deflated.len());
    out.extend_from_slice(MAGIC_CHUNKZIP); //          0x00: "chunkzip"
    out.extend_from_slice(&2u32.to_be_bytes()); //     0x08: dummy = 2  (padded-header variant)
    out.extend_from_slice(&fullsize.to_be_bytes()); // 0x0C: fullsize (decompressed length)
    out.extend_from_slice(&fullsize.to_be_bytes()); // 0x10: chunk_size (>= fullsize -> single chunk)
    out.extend_from_slice(&1u32.to_be_bytes()); //     0x14: n_chunks = 1
    out.extend_from_slice(&16u32.to_be_bytes()); //    0x18: mirrors the observed `chunklzx` (=0x10)
    out.extend_from_slice(&[0u8; 12]); //              0x1C..0x28: reserved (decoder ignores)
    // Single chunk: `zsize` prefix must land at offset 0x28 (== 40, and 40 % 16 == 8, as the decoder
    // requires), a 4-byte gap, then the deflate bytes at 0x30.
    out.extend_from_slice(&(deflated.len() as u32).to_be_bytes()); // 0x28: zsize
    out.extend_from_slice(&[0u8; 4]); //                              0x2C: 4-byte gap
    out.extend_from_slice(&deflated); //                             0x30: raw deflate
    out
}

/// Newer "padded" header: `fullsize/chunk_size/n_chunks` at +12/+16/+20, data
/// starts at +40, chunks padded so each `zsize` prefix lands at
/// `offset % 16 == 8` with a 4-byte gap before the deflate payload.
fn decompress_chunkzip_padded(data: &[u8]) -> Result<Vec<u8>, CodecError> {
    if data.len() < 40 {
        return Err(CodecError::TruncatedHeader {
            needed: 40,
            have: data.len(),
        });
    }
    let fullsize = read_u32_be(data, 12)? as usize;
    validate_fullsize(fullsize)?;
    let _chunk_size = read_u32_be(data, 16)? as usize;
    let n_chunks = read_u32_be(data, 20)? as usize;

    let mut out = Vec::with_capacity(fullsize);
    let mut off = 40usize;
    for chunk_index in 0..n_chunks {
        // Align off so that off % 16 == 8 — Heck's
        // `while off % 16 != 8: off += 8`. The format guarantees we land on this
        // alignment within at most one step (chunks are padded to a multiple of
        // 8 bytes, so `off % 8` is invariant across chunks and `{0, 8}` are the
        // only reachable values). The cap on iterations is a safety belt for
        // malformed input — without it, a corrupt header can drive off through a
        // misaligned sequence forever and OOM the host.
        let mut align_steps = 0;
        while off % 16 != 8 {
            if align_steps > 2 || off >= data.len() {
                return Err(CodecError::TruncatedHeader {
                    needed: off + 4,
                    have: data.len(),
                });
            }
            off += 8;
            align_steps += 1;
        }
        let zsize = read_u32_be(data, off)? as usize;
        off += 4;
        // 4-byte gap between the size prefix and the deflate payload.
        off += 4;
        consume_chunk(data, &mut off, zsize, chunk_index, &mut out, fullsize)?;
    }
    out.truncate(fullsize);
    Ok(out)
}

/// Older "legacy" header: `dummy` IS `fullsize`, `chunk_size` at +12, `n_chunks`
/// derived, data starts at +16, no per-chunk padding.
fn decompress_chunkzip_legacy(data: &[u8], dummy: u32) -> Result<Vec<u8>, CodecError> {
    let fullsize = dummy as usize;
    validate_fullsize(fullsize)?;
    let chunk_size = read_u32_be(data, 12)? as usize;
    let n_chunks = if chunk_size == 0 {
        0
    } else {
        (fullsize + chunk_size - 1) / chunk_size
    };

    let mut out = Vec::with_capacity(fullsize);
    let mut off = 16usize;
    for chunk_index in 0..n_chunks {
        let zsize = read_u32_be(data, off)? as usize;
        off += 4;
        consume_chunk(data, &mut off, zsize, chunk_index, &mut out, fullsize)?;
    }
    out.truncate(fullsize);
    Ok(out)
}

fn consume_chunk(
    data: &[u8],
    off: &mut usize,
    zsize: usize,
    chunk_index: usize,
    out: &mut Vec<u8>,
    fullsize: usize,
) -> Result<(), CodecError> {
    let end = off.checked_add(zsize).ok_or(CodecError::TruncatedChunk {
        chunk_index,
        offset: *off,
        zsize,
        available: data.len().saturating_sub(*off),
    })?;
    if end > data.len() {
        return Err(CodecError::TruncatedChunk {
            chunk_index,
            offset: *off,
            zsize,
            available: data.len().saturating_sub(*off),
        });
    }
    let chunk = &data[*off..end];
    *off = end;

    // Try raw deflate first (matches Heck's `zlib.decompress(chunk, -15)`).
    let remaining = fullsize.saturating_sub(out.len());
    let raw_err = match decode_deflate(chunk, out, remaining) {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    // Fallback: zlib-wrapped.
    let zlib_err = match decode_zlib(chunk, out, remaining) {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    Err(CodecError::DecompressFailed {
        chunk_index,
        raw: raw_err,
        zlib: zlib_err,
    })
}

fn validate_fullsize(fullsize: usize) -> Result<(), CodecError> {
    if fullsize > MAX_DECOMPRESSED_SIZE {
        return Err(CodecError::SizeLimitExceeded {
            declared: fullsize,
            limit: MAX_DECOMPRESSED_SIZE,
        });
    }
    Ok(())
}

fn decode_deflate(chunk: &[u8], out: &mut Vec<u8>, remaining: usize) -> std::io::Result<()> {
    let mut decoder = DeflateDecoder::new(chunk);
    let mut tmp = Vec::new();
    decoder
        .by_ref()
        .take(remaining.saturating_add(1) as u64)
        .read_to_end(&mut tmp)?;
    if tmp.len() > remaining {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "decompressed output exceeds declared size",
        ));
    }
    out.extend_from_slice(&tmp);
    Ok(())
}

fn decode_zlib(chunk: &[u8], out: &mut Vec<u8>, remaining: usize) -> std::io::Result<()> {
    let mut decoder = ZlibDecoder::new(chunk);
    let mut tmp = Vec::new();
    decoder
        .by_ref()
        .take(remaining.saturating_add(1) as u64)
        .read_to_end(&mut tmp)?;
    if tmp.len() > remaining {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "decompressed output exceeds declared size",
        ));
    }
    out.extend_from_slice(&tmp);
    Ok(())
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32, CodecError> {
    if offset + 4 > data.len() {
        return Err(CodecError::TruncatedHeader {
            needed: offset + 4,
            have: data.len(),
        });
    }
    let bytes: [u8; 4] = data[offset..offset + 4].try_into().expect("len checked");
    Ok(u32::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn compress_chunkzip_round_trips() {
        for case in [
            vec![],
            b"hello chunkzip".to_vec(),
            vec![0x42u8; 300_000],                       // large + highly compressible
            (0..5000u32).map(|i| (i * 7) as u8).collect(), // pseudo-random-ish
        ] {
            let packed = compress_chunkzip(&case);
            assert_eq!(&packed[..8], MAGIC_CHUNKZIP);
            let back = decompress_chunkzip(&packed).expect("decodes");
            assert_eq!(back, case, "round-trip mismatch for {}-byte input", case.len());
        }
    }

    #[test]
    fn compress_chunkzip_tolerates_trailing_padding() {
        // The write path pads the payload out to a fixed slot size; the decoder must
        // ignore trailing bytes after the single chunk.
        let data = b"payload that gets padded".to_vec();
        let mut packed = compress_chunkzip(&data);
        packed.resize(packed.len() + 4096, 0);
        assert_eq!(decompress_chunkzip(&packed).unwrap(), data);
    }

    /// Build a synthetic `chunkzip` payload in the legacy (`dummy > 2`) format.
    fn build_legacy_chunkzip(payload: &[u8], chunk_size: usize) -> Vec<u8> {
        assert!(chunk_size > 0);
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC_CHUNKZIP); // magic
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes()); // dummy = fullsize
        out.extend_from_slice(&(chunk_size as u32).to_be_bytes()); // chunk_size
        for chunk in payload.chunks(chunk_size) {
            let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
            enc.write_all(chunk).unwrap();
            let comp = enc.finish().unwrap();
            out.extend_from_slice(&(comp.len() as u32).to_be_bytes());
            out.extend_from_slice(&comp);
        }
        out
    }

    /// Build a synthetic `chunkzip` payload in the newer padded (`dummy <= 2`)
    /// format, mirroring the on-disk layout the decoder expects.
    fn build_padded_chunkzip(payload: &[u8], chunk_size: usize) -> Vec<u8> {
        assert!(chunk_size > 0);
        let n_chunks = if payload.is_empty() {
            0
        } else {
            (payload.len() + chunk_size - 1) / chunk_size
        };
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC_CHUNKZIP);
        out.extend_from_slice(&0u32.to_be_bytes()); // dummy = 0 → padded variant
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes()); // fullsize
        out.extend_from_slice(&(chunk_size as u32).to_be_bytes());
        out.extend_from_slice(&(n_chunks as u32).to_be_bytes());
        out.extend_from_slice(&[0u8; 16]); // reserved
        for chunk in payload.chunks(chunk_size) {
            let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
            enc.write_all(chunk).unwrap();
            let mut comp = enc.finish().unwrap();
            let pad = (8 - comp.len() % 8) % 8;
            comp.resize(comp.len() + pad, 0);
            while out.len() % 16 != 8 {
                out.push(0);
            }
            out.extend_from_slice(&(comp.len() as u32).to_be_bytes());
            out.extend_from_slice(&[0u8; 4]); // 4-byte gap
            out.extend_from_slice(&comp);
        }
        out
    }

    #[test]
    fn legacy_chunkzip_round_trip_small_payload() {
        let payload = b"hello world, this is a test payload for chunkzip decode";
        let wrapped = build_legacy_chunkzip(payload, 16);
        let decoded = decompress_chunkzip(&wrapped).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn legacy_chunkzip_round_trip_multi_chunk() {
        let payload: Vec<u8> = (0..1024u32).flat_map(|n| n.to_be_bytes()).collect();
        let wrapped = build_legacy_chunkzip(&payload, 64);
        let decoded = decompress_chunkzip(&wrapped).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn padded_chunkzip_round_trip_multi_chunk() {
        let payload: Vec<u8> = (0..2048u32).flat_map(|n| n.to_be_bytes()).collect();
        let wrapped = build_padded_chunkzip(&payload, 256);
        let decoded = decompress_chunkzip(&wrapped).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decompress_if_wrapped_passes_through_uncompressed_data() {
        let raw = b"this payload starts with neither chunkzip nor chunklzx";
        let out = decompress_if_wrapped(raw).unwrap();
        assert_eq!(out, raw);
    }

    #[test]
    fn decompress_if_wrapped_rejects_chunklzx_explicitly() {
        let mut data = Vec::from(*MAGIC_CHUNKLZX);
        data.extend_from_slice(&[0u8; 16]);
        assert!(matches!(
            decompress_if_wrapped(&data),
            Err(CodecError::UnsupportedLzx)
        ));
    }

    #[test]
    fn truncated_magic_is_an_error() {
        assert!(matches!(
            decompress_if_wrapped(b"shorty"),
            Err(CodecError::TruncatedMagic)
        ));
    }

    #[test]
    fn rejects_unreasonable_declared_size() {
        let mut data = vec![0u8; 40];
        data[..8].copy_from_slice(MAGIC_CHUNKZIP);
        data[8..12].copy_from_slice(&1u32.to_be_bytes());
        data[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
        let err = decompress_chunkzip(&data).unwrap_err();
        assert!(matches!(err, CodecError::SizeLimitExceeded { .. }));
    }
}
