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
//! A single-chunk `chunkzip` *encoder* ([`compress_chunkzip`]) exists for codec substitution
//! in staged archive edits; it is unqualified for native loader acceptance.
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
//!   `flag` word, then `zsize` bytes of chunk data. The flag selects the chunk's
//!   coding: `1` = raw deflate, `4` = stored (the `zsize` bytes *are* the plain
//!   bytes). Measured on every chunk of the PS3 NHL Legacy `patch.big` (1 817
//!   chunks, all flag `1`) and the PS4 NHL 21 (`CUSA18182`) `boot.big` / `overlays_2x.big`
//!   (6 380 chunks: all flag `1` except two final chunks with flag `4`, whose
//!   stored bytes match the externally extracted files byte for byte). Other
//!   flag values are unobserved; they are still tried as deflate and reported
//!   with the flag if that fails.
//! - **`dummy > 2`** (older header): `dummy` IS `fullsize`, `chunk_size` at +12,
//!   `n_chunks` derived as `ceil(fullsize / chunk_size)`, data starts at offset
//!   16. No per-chunk padding/gap.
//!
//! Each deflate chunk is tried as raw deflate first (negative window bits),
//! falling back to zlib-wrapped on failure. A chunk may never inflate past
//! `fullsize`, and the chunks together must produce exactly `fullsize` bytes:
//! a payload whose chunks stop short is reported as [`CodecError::ShortOutput`]
//! rather than returned truncated.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod dds;
#[cfg(test)]
pub(crate) mod fixtures;
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
    #[error("chunkzip chunk {chunk_index} (flag {flag}): neither raw deflate nor zlib-wrapped decode succeeded ({raw}; {zlib})")]
    DecompressFailed {
        chunk_index: usize,
        /// The chunk's flag word (`1` on every observed deflate chunk; the legacy
        /// header has no flag and the walker supplies `FLAG_DEFLATE`).
        flag: u32,
        raw: std::io::Error,
        zlib: std::io::Error,
    },
    #[error("chunkzip stored chunk {chunk_index} carries {zsize} bytes but only {remaining} of the declared {declared} remain")]
    StoredChunkTooLong {
        chunk_index: usize,
        zsize: usize,
        remaining: usize,
        declared: usize,
    },
    #[error("chunkzip declares {declared} decompressed bytes but its {n_chunks} chunks produced only {produced}")]
    ShortOutput {
        declared: usize,
        produced: usize,
        n_chunks: usize,
    },
    #[error("chunkzip declares {declared} decompressed bytes, exceeding the limit of {limit}")]
    SizeLimitExceeded { declared: usize, limit: usize },
    #[error("chunklzx (LZX) compression is not implemented")]
    UnsupportedLzx,
    #[error("{0}")]
    Malformed(String),
    #[error("chunklzx LZX decode failed: {0}")]
    LzxDecode(#[from] lzx::LzxError),
}

/// Returns the decompressed bytes for a `chunkzip`- or `chunklzx`-wrapped payload, or the input
/// unchanged when it doesn't carry a known container magic. (`chunklzx` decoding was promoted on
/// 21 Sep 2026 against the research vector manifest — 50 members of six titles byte-exact; the
/// codec has **no encoder**, so [`CodecError::UnsupportedLzx`] remains the answer of every write
/// path that would have to produce one.)
pub fn decompress_if_wrapped(data: &[u8]) -> Result<Vec<u8>, CodecError> {
    if data.len() < 8 {
        return Err(CodecError::TruncatedMagic);
    }
    let magic: &[u8; 8] = data[..8].try_into().expect("len checked above");
    match magic {
        m if m == MAGIC_CHUNKZIP => decompress_chunkzip(data),
        m if m == MAGIC_CHUNKLZX => decompress_chunklzx(data),
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

/// The LZX window of every EA `chunklzx` stream: 2^17 (34 position slots). The header's `window`
/// word (0x40000) is the chunk size, not the LZX window — never derived from it (research
/// 2026-09-18 handoff 40 §1; 2^18 does not decode).
pub const CHUNKLZX_WINDOW_BITS: u32 = 17;

/// Decompress a `chunklzx`-wrapped payload (Xbox 360 NHL 11–Legacy EB archive members) — the
/// framing of research 2026-09-18 handoff 40 (`codec-chunklzx-framing`, validated byte-for-byte
/// on 50 members of six titles by the research probe; this decoder is checked against the same
/// vector manifest by `tdb-archive-big`'s `chunklzx_vectors` test):
///
/// * header: `chunklzx` @0, `dummy` @8, `fullsize` @12, `window` (chunk size) @16, `n_chunks` @20,
///   data from @40;
/// * chunk `i` header at `H = ((cursor - 8 + 15) & !15) + 8` (first `H = 40`): `zsize` u32 BE,
///   `flag` u32 BE (3 = LZX sub-frames; 4 = stored, observed in NHL12;
///   0 = stored, defensive compatibility), then `zsize` bytes;
/// * the payload is a walk of sub-frames — `[u16 csize][csize bytes]` = one full 32 768-byte
///   frame, `[0xFF][u16 usize][u16 csize][csize bytes]` = the last frame — then 5 trailer bytes
///   that must be exactly what is left of `zsize`;
/// * the concatenated sub-frame bytes are **one LZX stream per chunk** (E8 header bit, window
///   bits 17, 32 KiB frames realigned to 16 bits) decoding to exactly `full_frames × 32 768 +
///   usize` bytes;
/// * the output equals `fullsize`; it is never zero-padded or silently truncated.
pub fn decompress_chunklzx(data: &[u8]) -> Result<Vec<u8>, CodecError> {
    if data.len() < 40 {
        return Err(CodecError::TruncatedHeader {
            needed: 40,
            have: data.len(),
        });
    }
    let fullsize = read_u32_be(data, 12)? as usize;
    validate_fullsize(fullsize)?;
    let n_chunks = read_u32_be(data, 20)? as usize;
    if n_chunks == 0 && fullsize > 0 {
        return Err(CodecError::Malformed("chunklzx: n_chunks = 0 with a non-zero fullsize".into()));
    }
    const FRAME: usize = lzx::FRAME_SIZE;

    let mut out = Vec::with_capacity(fullsize);
    let mut cursor = 40usize;
    for chunk_index in 0..n_chunks {
        // Chunk header: the first 16-byte line position ≡ 8 (mod 16) at or after the cursor.
        let off = if cursor == 40 { 40 } else { ((cursor - 8 + 15) & !15) + 8 };
        if off + 8 > data.len() {
            return Err(CodecError::TruncatedHeader {
                needed: off + 8,
                have: data.len(),
            });
        }
        let zsize = read_u32_be(data, off)? as usize;
        let flag = read_u32_be(data, off + 4)?;
        let start = off + 8;
        let end = start.checked_add(zsize).filter(|e| *e <= data.len()).ok_or(CodecError::TruncatedChunk {
            chunk_index,
            offset: start,
            zsize,
            available: data.len().saturating_sub(start),
        })?;
        let payload = &data[start..end];
        let remaining = fullsize - out.len();
        match flag {
            0 | 4 => {
                // NHL12 crowd_29.cwd ends with a flag-4 stored chunk, verified
                // byte-for-byte against the independent extracted plaintext.
                if zsize > remaining {
                    return Err(CodecError::Malformed(format!("chunklzx chunk {chunk_index}: stored chunk of {zsize} bytes exceeds the {remaining} bytes left of fullsize")));
                }
                out.extend_from_slice(payload);
            }
            3 => {
                // Sub-frame walk.
                let mut stream: Vec<u8> = Vec::with_capacity(zsize);
                let mut produced_expected = 0usize;
                let mut p = 0usize;
                let mut last_seen = false;
                while !last_seen {
                    if p + 2 > payload.len() {
                        return Err(CodecError::Malformed(format!("chunklzx chunk {chunk_index}: sub-frame prefix runs past the chunk at {p}")));
                    }
                    if payload[p] == 0xFF {
                        if p + 5 > payload.len() {
                            return Err(CodecError::Malformed(format!("chunklzx chunk {chunk_index}: last-frame prefix runs past the chunk at {p}")));
                        }
                        let usize_ = u16::from_be_bytes([payload[p + 1], payload[p + 2]]) as usize;
                        let csize = u16::from_be_bytes([payload[p + 3], payload[p + 4]]) as usize;
                        if usize_ > FRAME {
                            return Err(CodecError::Malformed(format!("chunklzx chunk {chunk_index}: last-frame size {usize_} exceeds the 32 768-byte frame")));
                        }
                        p += 5;
                        if p + csize > payload.len() {
                            return Err(CodecError::Malformed(format!("chunklzx chunk {chunk_index}: last frame ({csize} bytes) runs past the chunk at {p}")));
                        }
                        stream.extend_from_slice(&payload[p..p + csize]);
                        p += csize;
                        produced_expected += usize_;
                        last_seen = true;
                    } else {
                        let csize = u16::from_be_bytes([payload[p], payload[p + 1]]) as usize;
                        p += 2;
                        if p + csize > payload.len() {
                            return Err(CodecError::Malformed(format!("chunklzx chunk {chunk_index}: full frame ({csize} bytes) runs past the chunk at {p}")));
                        }
                        stream.extend_from_slice(&payload[p..p + csize]);
                        p += csize;
                        produced_expected += FRAME;
                    }
                    if produced_expected > remaining {
                        return Err(CodecError::Malformed(format!("chunklzx chunk {chunk_index}: the sub-frames declare {produced_expected} bytes, more than the {remaining} bytes left of fullsize")));
                    }
                }
                if payload.len() - p != 5 {
                    return Err(CodecError::Malformed(format!("chunklzx chunk {chunk_index}: {} bytes follow the last frame, expected the 5-byte trailer", payload.len() - p)));
                }
                let decoded = lzx::decompress_lzx(&stream, produced_expected, CHUNKLZX_WINDOW_BITS)?;
                if decoded.len() != produced_expected {
                    return Err(CodecError::ShortOutput {
                        declared: produced_expected,
                        produced: decoded.len(),
                        n_chunks,
                    });
                }
                out.extend_from_slice(&decoded);
            }
            other => {
                return Err(CodecError::Malformed(format!("chunklzx chunk {chunk_index}: unknown chunk flag {other} (3 = LZX, 0/4 = stored)")));
            }
        }
        cursor = end;
    }
    if out.len() != fullsize {
        return Err(CodecError::ShortOutput {
            declared: fullsize,
            produced: out.len(),
            n_chunks,
        });
    }
    Ok(out)
}

/// Compress `data` into a **single-chunk padded `chunkzip`** payload (`dummy = 2` header, the same
/// header shape the game processes for these entries) with the chunk flag `1` every observed
/// deflate chunk carries. Round-trips through [`decompress_chunkzip`]; output written before the
/// flag correction (flag `0`) still decodes, since the reader treats any non-stored flag as deflate.
/// Not qualified: native loader acceptance, `chunk_size = fullsize` sizing for large inputs, or
/// whether the game requires the 128 KiB chunking it ships with.
///
/// Used by the write path to substitute a payload's codec (`chunklzx` → `chunkzip`) without
/// implementing LZX compression. The chunk is raw DEFLATE (the decoder tries raw deflate first).
pub fn compress_chunkzip(data: &[u8]) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
    enc.write_all(data)
        .expect("in-memory deflate write cannot fail");
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
                                       // Single chunk: `zsize` prefix must land at offset 0x28 (== 40, and 40 % 16 == 8, as the
                                       // decoder requires), then the chunk flag word, then the deflate bytes at 0x30.
    out.extend_from_slice(&(deflated.len() as u32).to_be_bytes()); // 0x28: zsize
    out.extend_from_slice(&FLAG_DEFLATE.to_be_bytes()); //            0x2C: flag = 1 (deflate)
    out.extend_from_slice(&deflated); //                             0x30: raw deflate
    out
}

/// The chunk window every stock `chunkzip` member uses (research handoffs 01 §1 and 02 §3.2:
/// 240 + 53 + 2 members on PS3 12–Legacy and PS4 15–21, all `0x20000`).
pub const CHUNKZIP_NATIVE_WINDOW: usize = 0x20000;

/// The backend and version [`compress_chunkzip_native`] deflates with, for provenance notes.
/// `flate2` is built with the `zlib` feature here, i.e. `libz-sys` and its bundled zlib, statically
/// linked; the bundled version is what the crate ships (`zlib.h` `ZLIB_VERSION`), recorded at the
/// time of writing. This is what lets the encoder reproduce EA's stock bytes; the framing and the
/// decode check are what the write path relies on.
pub const CHUNKZIP_NATIVE_BACKEND: &str = "flate2 1.1 / libz-sys 1.1.29 (bundled zlib 1.3.2), raw deflate, level 9, default strategy";

/// Compress `data` into a **native-framed** `chunkzip` payload — the framing every measured stock
/// member carries (research handoffs 01 §1, 02 §3.2):
///
/// * 40-byte padded header: `"chunkzip"`, `2`, `fullsize`, window `0x20000`, `n_chunks`, `0x10`,
///   12 zero bytes;
/// * `n_chunks = ceil(fullsize / 0x20000)` chunks of up to 128 KiB of plain input; each chunk's
///   `[zsize][flag]` prefix sits at the next offset `≡ 8 (mod 16)` (zero padding between chunks,
///   never counted in `zsize`); **no trailing bytes** after the last chunk;
/// * a chunk is raw DEFLATE (flag `1`) unless the deflate output is not shorter than the plain
///   chunk, in which case the chunk is **stored** (flag `4`, `zsize = plain length`) — the rule
///   the two stored stock chunks follow (`len(deflate9(chunk)) >= len(chunk)`).
///
/// With the real-zlib backend ([`CHUNKZIP_NATIVE_BACKEND`]) this reproduces the measured stock
/// members byte-for-byte (the reproduction target: 53/53 NHL 16–21 CSV members, 240/240 PS3 + PS4
/// samples in research; the repository's own regression covers the 65 private NHL 16–21 members).
/// Byte identity is evidence of consistency with EA's tooling, **not** a loader rule: what the
/// write path requires is native framing plus an independent decode equal to the input, and the
/// caller keeps the existing qualification disclosures ("native loader acceptance") on any edited
/// member. Output always round-trips through [`decompress_chunkzip`]. Unchanged members must keep
/// their original stored bytes; this is for edited content only.
pub fn compress_chunkzip_native(data: &[u8]) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    let fullsize = data.len();
    let n_chunks = fullsize.div_ceil(CHUNKZIP_NATIVE_WINDOW);
    let mut out = Vec::with_capacity(40 + fullsize / 2);
    out.extend_from_slice(MAGIC_CHUNKZIP); //                              0x00
    out.extend_from_slice(&2u32.to_be_bytes()); //                         0x08 dummy = 2
    out.extend_from_slice(&(fullsize as u32).to_be_bytes()); //            0x0C fullsize
    out.extend_from_slice(&(CHUNKZIP_NATIVE_WINDOW as u32).to_be_bytes()); // 0x10 window
    out.extend_from_slice(&(n_chunks as u32).to_be_bytes()); //            0x14 n_chunks
    out.extend_from_slice(&0x10u32.to_be_bytes()); //                      0x18
    out.extend_from_slice(&[0u8; 12]); //                                  0x1C..0x28
    for chunk in data.chunks(CHUNKZIP_NATIVE_WINDOW) {
        // The prefix lands at the next offset ≡ 8 (mod 16); pad with zeros.
        while out.len() % 16 != 8 {
            out.push(0);
        }
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::new(9));
        enc.write_all(chunk).expect("in-memory deflate write cannot fail");
        let deflated = enc.finish().expect("in-memory deflate finish cannot fail");
        if deflated.len() >= chunk.len() {
            out.extend_from_slice(&(chunk.len() as u32).to_be_bytes());
            out.extend_from_slice(&FLAG_STORED.to_be_bytes());
            out.extend_from_slice(chunk);
        } else {
            out.extend_from_slice(&(deflated.len() as u32).to_be_bytes());
            out.extend_from_slice(&FLAG_DEFLATE.to_be_bytes());
            out.extend_from_slice(&deflated);
        }
    }
    out
}

/// True iff `stored` is framed exactly as [`compress_chunkzip_native`] frames a payload (header
/// words, chunk count, chunk prefix alignment, no trailing bytes) — the structural check the write
/// path applies to an edited member before the independent decode; it does not compare deflate bits.
pub fn is_native_chunkzip_framing(stored: &[u8]) -> Result<(), String> {
    if stored.len() < 40 || &stored[..8] != MAGIC_CHUNKZIP {
        return Err("not a chunkzip payload".into());
    }
    let be = |o: usize| u32::from_be_bytes([stored[o], stored[o + 1], stored[o + 2], stored[o + 3]]);
    if be(8) != 2 {
        return Err(format!("header word 0x08 is {} (native: 2)", be(8)));
    }
    let fullsize = be(12) as usize;
    if be(16) as usize != CHUNKZIP_NATIVE_WINDOW {
        return Err(format!("window {:#x} (native: 0x20000)", be(16)));
    }
    let n_chunks = be(20) as usize;
    if n_chunks != fullsize.div_ceil(CHUNKZIP_NATIVE_WINDOW).max(if fullsize == 0 { 0 } else { 1 }) {
        return Err(format!("{n_chunks} chunks for {fullsize} bytes (native: ceil(fullsize / 0x20000))"));
    }
    if be(24) != 0x10 || stored[28..40].iter().any(|b| *b != 0) {
        return Err("header words 0x18..0x28 are not 0x10 + zeros".into());
    }
    let mut off = 40usize;
    for i in 0..n_chunks {
        while off % 16 != 8 {
            if off >= stored.len() || stored[off] != 0 {
                return Err(format!("chunk {i}: non-zero or missing padding before the prefix"));
            }
            off += 1;
        }
        if off + 8 > stored.len() {
            return Err(format!("chunk {i}: truncated prefix"));
        }
        let zsize = be(off) as usize;
        let flag = be(off + 4);
        if flag != FLAG_DEFLATE && flag != FLAG_STORED {
            return Err(format!("chunk {i}: flag {flag} (native: 1 or 4)"));
        }
        off += 8 + zsize;
        if off > stored.len() {
            return Err(format!("chunk {i}: zsize {zsize} runs past the end"));
        }
    }
    if off != stored.len() {
        return Err(format!("{} trailing byte(s) after the last chunk (native: none)", stored.len() - off));
    }
    Ok(())
}

/// Round `n` up to the next multiple of 16. Saturating, so a corrupt header can
/// never wrap the offset arithmetic; the caller's bounds check rejects the
/// saturated value.
fn align_up_16(n: usize) -> usize {
    n.saturating_add(15) & !15
}

/// Newer "padded" header: `fullsize/chunk_size/n_chunks` at +12/+16/+20, data
/// starts at +40, and each chunk's `zsize` prefix is padded forward to the next
/// `offset % 16 == 8`, followed by the chunk's flag word and then its bytes. The padding
/// sits *between* chunks — it is not counted in any chunk's declared `zsize` —
/// so a chunk's compressed length is arbitrary and every residue class is
/// reachable (see the alignment note inside the loop).
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
        // Each chunk's `[zsize:u32][flag:u32]` header is placed at the next offset
        // congruent to 8 (mod 16) — i.e. the payload that follows it starts 16-byte
        // aligned. Advance to that offset directly.
        //
        // An earlier reading of this rule stepped `off += 8` until `off % 16 == 8`,
        // assuming a chunk's compressed size is always a multiple of 8. That holds
        // for the NHL Legacy fixtures it was derived from but **not** on real PS4
        // NHL 19 `game_*.big` payloads, where `zsize` is arbitrary (e.g. 19030) and
        // the stepping loop can never reach the target residue — every multi-chunk
        // PS4 asset failed with a spurious `TruncatedHeader`. Originally verified
        // against the NHL 19 `jack_eichel.mcd` entries in `game_90-008.big`,
        // `patch_game_91.big` and `patch_game_93.big` (a separately attributed copy,
        // not rerun since): all 8 chunks of each land exactly on this alignment and
        // consume the stored payload to its final byte. S1 (2026-09-16) re-verified
        // the same arithmetic on the PS3 NHL Legacy `patch.big` (1 817 chunks) and
        // the PS4 NHL 21 `CUSA18182` `boot.big` / `overlays_2x.big` (6 380 chunks).
        off = align_up_16(off.saturating_sub(8)).saturating_add(8);
        if off >= data.len() {
            return Err(CodecError::TruncatedHeader {
                needed: off + 4,
                have: data.len(),
            });
        }
        let zsize = read_u32_be(data, off)? as usize;
        off += 4;
        // The chunk's coding flag: `1` deflate, `4` stored (see the module docs).
        let flag = read_u32_be(data, off)?;
        off += 4;
        consume_chunk(data, &mut off, zsize, flag, chunk_index, &mut out, fullsize)?;
    }
    finish_output(out, fullsize, n_chunks)
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
        fullsize.div_ceil(chunk_size)
    };

    let mut out = Vec::with_capacity(fullsize);
    let mut off = 16usize;
    for chunk_index in 0..n_chunks {
        let zsize = read_u32_be(data, off)? as usize;
        off += 4;
        // No flag word in this header; every chunk is deflate.
        consume_chunk(
            data,
            &mut off,
            zsize,
            FLAG_DEFLATE,
            chunk_index,
            &mut out,
            fullsize,
        )?;
    }
    finish_output(out, fullsize, n_chunks)
}

/// Chunk flag word of the padded header: raw deflate (every observed chunk but two).
const FLAG_DEFLATE: u32 = 1;
/// Chunk flag word of the padded header: stored — `zsize` plain bytes, no coding.
const FLAG_STORED: u32 = 4;

/// The chunks must account for exactly `fullsize` bytes. A payload whose chunk
/// walk ends early (fewer chunks than the size needs, or a last chunk that
/// inflates short) is malformed; returning the partial bytes would hand a
/// silently truncated asset to the caller.
fn finish_output(out: Vec<u8>, fullsize: usize, n_chunks: usize) -> Result<Vec<u8>, CodecError> {
    if out.len() != fullsize {
        return Err(CodecError::ShortOutput {
            declared: fullsize,
            produced: out.len(),
            n_chunks,
        });
    }
    Ok(out)
}

fn consume_chunk(
    data: &[u8],
    off: &mut usize,
    zsize: usize,
    flag: u32,
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

    let remaining = fullsize.saturating_sub(out.len());
    if flag == FLAG_STORED {
        // Stored chunk: the bytes are the plain data. Bound it by what the header
        // still owes so a corrupt `zsize` cannot grow the output past `fullsize`.
        if zsize > remaining {
            return Err(CodecError::StoredChunkTooLong {
                chunk_index,
                zsize,
                remaining,
                declared: fullsize,
            });
        }
        out.extend_from_slice(chunk);
        return Ok(());
    }

    // Try raw deflate first (matches Heck's `zlib.decompress(chunk, -15)`).
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
        flag,
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
            vec![0x42u8; 300_000], // large + highly compressible
            (0..5000u32).map(|i| (i * 7) as u8).collect(), // pseudo-random-ish
        ] {
            let packed = compress_chunkzip(&case);
            assert_eq!(&packed[..8], MAGIC_CHUNKZIP);
            let back = decompress_chunkzip(&packed).expect("decodes");
            assert_eq!(
                back,
                case,
                "round-trip mismatch for {}-byte input",
                case.len()
            );
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
            payload.len().div_ceil(chunk_size)
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
            out.extend_from_slice(&[0u8; 4]); // flag word 0: historical encoder output, read as deflate
            out.extend_from_slice(&comp);
        }
        out
    }

    /// Build a padded `chunkzip` payload the way the **real PS4 archives** do, which
    /// differs from [`build_padded_chunkzip`] in the one respect that matters here:
    /// each chunk declares its **exact** compressed length, and the inter-chunk
    /// padding that carries the next header to `offset % 16 == 8` lies *outside*
    /// every declared payload.
    ///
    /// `build_padded_chunkzip` instead rounds each `comp` up to a multiple of 8 and
    /// folds that padding into the declared `zsize`. That keeps every chunk's length
    /// 8-aligned, which is exactly the property the old stepping alignment assumed —
    /// so it masked the defect this fixture exposes. Returns the payload and the
    /// declared `zsize` of each chunk so tests can assert the residues they cover.
    fn build_padded_chunkzip_native(payload: &[u8], chunk_size: usize) -> (Vec<u8>, Vec<usize>) {
        assert!(chunk_size > 0);
        let n_chunks = if payload.is_empty() {
            0
        } else {
            payload.len().div_ceil(chunk_size)
        };
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC_CHUNKZIP);
        out.extend_from_slice(&2u32.to_be_bytes()); // dummy = 2 → padded variant
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes()); // fullsize
        out.extend_from_slice(&(chunk_size as u32).to_be_bytes());
        out.extend_from_slice(&(n_chunks as u32).to_be_bytes());
        out.extend_from_slice(&16u32.to_be_bytes());
        out.extend_from_slice(&[0u8; 12]); // reserved — data starts at 40

        let mut zsizes = Vec::with_capacity(n_chunks);
        for chunk in payload.chunks(chunk_size) {
            let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
            enc.write_all(chunk).unwrap();
            let comp = enc.finish().unwrap();
            // Pad *between* chunks, never inside the declared length.
            while out.len() % 16 != 8 {
                out.push(0);
            }
            out.extend_from_slice(&(comp.len() as u32).to_be_bytes());
            out.extend_from_slice(&1u32.to_be_bytes()); // flag word, as observed on PS4
            out.extend_from_slice(&comp);
            zsizes.push(comp.len());
        }
        (out, zsizes)
    }

    /// Pseudo-random, effectively incompressible bytes, so each chunk's deflate
    /// length tracks its input length and the fixture reaches varied residues.
    fn noise(len: usize, seed: u64) -> Vec<u8> {
        let mut state = seed | 1;
        (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (state >> 33) as u8
            })
            .collect()
    }

    /// The regression for the PS4 alignment defect: a multi-chunk padded payload
    /// whose declared compressed lengths are **not** multiples of 8, covering several
    /// residues mod 8 — the shape every real PS4 `game_*.big` asset has (NHL 19 and NHL 21 alike), and the
    /// shape [`build_padded_chunkzip`] cannot produce.
    #[test]
    fn padded_chunkzip_decodes_chunks_with_unaligned_compressed_lengths() {
        let payload = noise(5000, 0x1929_0910);
        let (wrapped, zsizes) = build_padded_chunkzip_native(&payload, 617);

        assert!(zsizes.len() >= 4, "want a genuinely multi-chunk fixture");
        let residues: std::collections::BTreeSet<usize> = zsizes.iter().map(|z| z % 8).collect();
        assert!(
            residues.iter().any(|&r| r != 0),
            "fixture must contain a chunk whose length is not 8-aligned: {zsizes:?}"
        );
        assert!(
            residues.len() >= 2,
            "fixture must cover multiple alignment residues, saw {residues:?} from {zsizes:?}"
        );

        let decoded = decompress_chunkzip(&wrapped).unwrap();
        assert_eq!(decoded, payload, "byte-exact round trip");
    }

    /// Pins *why* the fixture above is the regression: the prior algorithm advanced
    /// to the next chunk header by stepping `off += 8` until `off % 16 == 8`, capped
    /// at a couple of steps. Replayed over the same bytes it cannot reach a single
    /// one of the real header offsets, which is precisely how every multi-chunk PS4
    /// payload came back as a spurious `TruncatedHeader`.
    #[test]
    fn prior_stepping_alignment_cannot_reach_unaligned_chunk_headers() {
        let payload = noise(5000, 0x1929_0910);
        let (wrapped, _) = build_padded_chunkzip_native(&payload, 617);

        // Replay of the removed loop, over the first chunk boundary only.
        let n_chunks = u32::from_be_bytes(wrapped[20..24].try_into().unwrap()) as usize;
        assert!(n_chunks > 1);
        let mut off = 40usize;
        let zsize = u32::from_be_bytes(wrapped[off..off + 4].try_into().unwrap()) as usize;
        off += 8 + zsize; // header + gap + payload → start of the inter-chunk padding

        let mut steps = 0;
        let mut reached = true;
        while off % 16 != 8 {
            if steps > 2 || off >= wrapped.len() {
                reached = false;
                break;
            }
            off += 8;
            steps += 1;
        }
        assert!(
            !reached,
            "the old stepping loop should be unable to align here; \
             if this ever passes the fixture stopped reproducing the defect"
        );

        // The corrected arithmetic lands on the real header in one step.
        let end = 40 + 8 + zsize;
        let corrected = align_up_16(end.saturating_sub(8)).saturating_add(8);
        assert_eq!(corrected % 16, 8);
        assert!(corrected >= end && corrected < end + 16);
    }

    /// Truncation and boundary handling around the corrected alignment: a payload cut
    /// inside the inter-chunk padding, inside a chunk header, and inside a chunk's
    /// deflate bytes must each be reported, never panic and never silently truncate.
    #[test]
    fn padded_chunkzip_rejects_truncation_around_chunk_alignment() {
        let payload = noise(5000, 0x1929_0910);
        let (wrapped, _) = build_padded_chunkzip_native(&payload, 617);
        let zsize = u32::from_be_bytes(wrapped[40..44].try_into().unwrap()) as usize;
        let first_end = 48 + zsize; // first byte of the inter-chunk padding
        let second_header = align_up_16(first_end.saturating_sub(8)).saturating_add(8);

        for (label, cut) in [
            ("inside inter-chunk padding", first_end + 1),
            ("inside the second chunk header", second_header + 2),
            ("inside the second chunk payload", second_header + 12),
        ] {
            assert!(
                cut < wrapped.len(),
                "{label}: cut must be inside the fixture"
            );
            let err = decompress_chunkzip(&wrapped[..cut]);
            assert!(err.is_err(), "{label}: truncation must be reported, got Ok");
        }
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
    fn decompress_if_wrapped_decodes_a_stored_chunklzx_chunk_and_refuses_malformed_ones() {
        // A stored (flag 0) chunk is the one shape a test can build without an LZX encoder: it
        // exercises the header, the chunk-header alignment rule and the fullsize accounting.
        fn stored(fullsize: u32, chunks: &[&[u8]], gap_to_next: bool) -> Vec<u8> {
            let mut d = Vec::new();
            d.extend_from_slice(MAGIC_CHUNKLZX);
            d.extend_from_slice(&2u32.to_be_bytes());
            d.extend_from_slice(&fullsize.to_be_bytes());
            d.extend_from_slice(&0x40000u32.to_be_bytes());
            d.extend_from_slice(&(chunks.len() as u32).to_be_bytes());
            d.extend_from_slice(&16u32.to_be_bytes());
            d.extend_from_slice(&[0u8; 12]);
            assert_eq!(d.len(), 40);
            for (i, c) in chunks.iter().enumerate() {
                if i > 0 {
                    // Next header at the first offset ≡ 8 (mod 16) at/after the cursor.
                    let h = ((d.len() - 8 + 15) & !15) + 8;
                    d.resize(h, 0);
                    let _ = gap_to_next;
                }
                d.extend_from_slice(&(c.len() as u32).to_be_bytes());
                d.extend_from_slice(&0u32.to_be_bytes());
                d.extend_from_slice(c);
            }
            d
        }
        let a: Vec<u8> = (0..1000u32).map(|i| (i * 7) as u8).collect();
        let b: Vec<u8> = (0..333u32).map(|i| (i * 13) as u8).collect();
        let d = stored(1333, &[&a, &b], true);
        assert_eq!(decompress_if_wrapped(&d).unwrap(), [a.clone(), b.clone()].concat());
        let mut flag4 = stored(1000, &[&a], true);
        flag4[44..48].copy_from_slice(&4u32.to_be_bytes());
        assert_eq!(decompress_if_wrapped(&flag4).unwrap(), a);
        flag4[12..16].copy_from_slice(&999u32.to_be_bytes());
        assert!(matches!(decompress_chunklzx(&flag4), Err(CodecError::Malformed(_))));
        // Chunk sizes ≡ 2 (mod 4) and odd sizes reach the next header by the rule, never by
        // stepping in eights.
        let c1: Vec<u8> = vec![1u8; 30];
        let c2: Vec<u8> = vec![2u8; 7];
        let c3: Vec<u8> = vec![3u8; 5];
        let d = stored(42, &[&c1, &c2, &c3], true);
        assert_eq!(decompress_if_wrapped(&d).unwrap(), [c1, c2, c3].concat());
        // Malformed inputs refuse (never panic, never zero-pad):
        // header shorter than 40 bytes;
        assert!(matches!(decompress_chunklzx(&d[..39]), Err(CodecError::TruncatedHeader { .. })));
        // a chunk whose zsize runs past the payload;
        let mut t = stored(1000, &[&a], true);
        t.truncate(t.len() - 10);
        assert!(matches!(decompress_chunklzx(&t), Err(CodecError::TruncatedChunk { .. })));
        // n_chunks larger than the data holds;
        let mut t = stored(1000, &[&a], true);
        t[20..24].copy_from_slice(&5u32.to_be_bytes());
        assert!(matches!(decompress_chunklzx(&t), Err(CodecError::TruncatedHeader { .. })));
        // a stored chunk longer than what is left of fullsize (overrun);
        let t = stored(999, &[&a], true);
        assert!(matches!(decompress_chunklzx(&t), Err(CodecError::Malformed(_))));
        // chunks that stop short of fullsize;
        let t = stored(1001, &[&a], true);
        assert!(matches!(decompress_chunklzx(&t), Err(CodecError::ShortOutput { declared: 1001, produced: 1000, .. })));
        // an unknown flag;
        let mut t = stored(1000, &[&a], true);
        t[44..48].copy_from_slice(&7u32.to_be_bytes());
        assert!(matches!(decompress_chunklzx(&t), Err(CodecError::Malformed(_))));
        // n_chunks = 0 with a non-zero fullsize;
        let mut t = stored(1000, &[&a], true);
        t[20..24].copy_from_slice(&0u32.to_be_bytes());
        assert!(matches!(decompress_chunklzx(&t), Err(CodecError::Malformed(_))));
        // an LZX chunk (flag 3) whose sub-frame prefix / last frame runs past the chunk, whose
        // last-frame size exceeds a frame, or whose sub-frames declare more than fullsize.
        let lzx_chunk = |payload: &[u8], fullsize: u32| {
            let mut t = stored(fullsize, &[payload], true);
            t[44..48].copy_from_slice(&3u32.to_be_bytes());
            t
        };
        assert!(matches!(decompress_chunklzx(&lzx_chunk(&[0x00, 0x10, 1, 2, 3], 100)), Err(CodecError::Malformed(_))), "full frame past the chunk");
        assert!(matches!(decompress_chunklzx(&lzx_chunk(&[0xFF, 0x00, 0x10, 0x00, 0x09, 1, 2], 100)), Err(CodecError::Malformed(_))), "last frame past the chunk");
        assert!(matches!(decompress_chunklzx(&lzx_chunk(&[0xFF, 0x80, 0x01, 0x00, 0x01, 1, 0, 0, 0, 0, 0], 100)), Err(CodecError::Malformed(_))), "usize > 32768");
        assert!(matches!(decompress_chunklzx(&lzx_chunk(&[0x00, 0x01, 1, 0xFF, 0x00, 0x08, 0x00, 0x01, 1, 0, 0, 0, 0, 0], 100)), Err(CodecError::Malformed(_))), "sub-frames declare more than fullsize");
        assert!(matches!(decompress_chunklzx(&lzx_chunk(&[0xFF, 0x00, 0x08, 0x00, 0x01, 1, 0, 0, 0, 0, 0, 0], 8)), Err(CodecError::Malformed(_))), "trailer not exactly 5 bytes");
        // A well-framed but garbage LZX stream fails in the LZX core, not by panic.
        assert!(matches!(decompress_chunklzx(&lzx_chunk(&[0xFF, 0x00, 0x08, 0x00, 0x04, 0xDE, 0xAD, 0xBE, 0xEF, 0, 0, 0, 0, 0], 8)), Err(CodecError::LzxDecode(_) | CodecError::ShortOutput { .. } | CodecError::Malformed(_))));
        // A bare magic with a 16-byte tail is a truncated header, reported as such.
        let mut data = Vec::from(*MAGIC_CHUNKLZX);
        data.extend_from_slice(&[0u8; 16]);
        assert!(matches!(decompress_if_wrapped(&data), Err(CodecError::TruncatedHeader { .. })));
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

    // ----- S1: per-chunk flag word, stored chunks, short output, malformed framing -----

    /// One chunk of a hand-built padded payload: the flag word to write and the
    /// bytes to store after it (already deflated for `FLAG_DEFLATE`, plain for
    /// `FLAG_STORED`, or anything at all for a malformed case).
    struct ChunkSpec {
        flag: u32,
        stored: Vec<u8>,
    }

    fn deflate_raw(plain: &[u8], level: Compression) -> Vec<u8> {
        let mut enc = DeflateEncoder::new(Vec::new(), level);
        enc.write_all(plain).unwrap();
        enc.finish().unwrap()
    }

    /// Build a padded (`dummy = 2`) payload from explicit chunks, exactly as the
    /// PS3 `patch.big` and PS4 `boot.big` / `overlays_2x.big` members lay them out:
    /// header words `fullsize / chunk_size / n_chunks / 16` then twelve zero bytes,
    /// each chunk's `[zsize][flag]` at the next offset congruent to 8 (mod 16)
    /// with zero padding in between, the stored bytes right after, and nothing
    /// after the last chunk. `fullsize` and `n_chunks` are written verbatim so a
    /// test can declare a lie. Returns the payload and each chunk's header offset.
    fn build_padded_chunkzip_flagged(
        dummy: u32,
        fullsize: u32,
        chunk_size: u32,
        n_chunks: u32,
        chunks: &[ChunkSpec],
    ) -> (Vec<u8>, Vec<usize>) {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC_CHUNKZIP);
        out.extend_from_slice(&dummy.to_be_bytes());
        out.extend_from_slice(&fullsize.to_be_bytes());
        out.extend_from_slice(&chunk_size.to_be_bytes());
        out.extend_from_slice(&n_chunks.to_be_bytes());
        out.extend_from_slice(&16u32.to_be_bytes());
        out.extend_from_slice(&[0u8; 12]);
        let mut headers = Vec::with_capacity(chunks.len());
        for c in chunks {
            while out.len() % 16 != 8 {
                out.push(0);
            }
            headers.push(out.len());
            out.extend_from_slice(&(c.stored.len() as u32).to_be_bytes());
            out.extend_from_slice(&c.flag.to_be_bytes());
            out.extend_from_slice(&c.stored);
        }
        (out, headers)
    }

    /// Every inter-chunk gap width 0..=15 occurs on the real archives (the PS3
    /// `patch.big` shows all sixteen across its 1 817 chunks, so do the PS4
    /// `boot.big` and overlays). Stored deflate blocks make a chunk's `zsize`
    /// track its input length, so seventeen chunks of lengths 1..=17 (sixteen
    /// gaps) reach every residue and therefore every gap width; the test asserts
    /// that coverage rather than assuming it, and decodes the lot byte for byte.
    #[test]
    fn padded_chunkzip_covers_every_inter_chunk_gap_width() {
        let plain = noise(17 * 18 / 2, 0x5151);
        let mut chunks = Vec::new();
        let mut at = 0;
        for len in 1..=17usize {
            chunks.push(ChunkSpec {
                flag: FLAG_DEFLATE,
                stored: deflate_raw(&plain[at..at + len], Compression::none()),
            });
            at += len;
        }
        assert_eq!(at, plain.len());
        let (wrapped, headers) =
            build_padded_chunkzip_flagged(2, plain.len() as u32, 17, 17, &chunks);

        let gaps: std::collections::BTreeSet<usize> = headers
            .windows(2)
            .zip(&chunks)
            .map(|(pair, c)| pair[1] - (pair[0] + 8 + c.stored.len()))
            .collect();
        assert_eq!(
            gaps,
            (0..16).collect(),
            "fixture must reach every gap width; chunk sizes were {:?}",
            chunks.iter().map(|c| c.stored.len()).collect::<Vec<_>>()
        );
        assert!(headers.iter().all(|h| h % 16 == 8));

        assert_eq!(decompress_chunkzip(&wrapped).unwrap(), plain);
    }

    /// Flag `4` marks a stored chunk: its `zsize` bytes are the plain bytes. The
    /// observed shape (PS4 NHL 21 `CUSA18182` `overlays_21.big`, two `.gfx` members, verified
    /// against the extracted files) is a deflate chunk followed by a stored final
    /// chunk; stored chunks anywhere, and a payload of only stored chunks, decode
    /// the same way.
    #[test]
    fn padded_chunkzip_decodes_stored_chunks() {
        let plain = noise(131_072 + 6_839, 0x0421);
        let (head, tail) = plain.split_at(131_072);

        // The observed shape.
        let chunks = [
            ChunkSpec {
                flag: FLAG_DEFLATE,
                stored: deflate_raw(head, Compression::default()),
            },
            ChunkSpec {
                flag: FLAG_STORED,
                stored: tail.to_vec(),
            },
        ];
        let (wrapped, _) =
            build_padded_chunkzip_flagged(2, plain.len() as u32, 131_072, 2, &chunks);
        assert_eq!(decompress_chunkzip(&wrapped).unwrap(), plain);

        // Stored first, then deflate; and stored only.
        let small = noise(700, 0x0422);
        let chunks = [
            ChunkSpec {
                flag: FLAG_STORED,
                stored: small[..300].to_vec(),
            },
            ChunkSpec {
                flag: FLAG_DEFLATE,
                stored: deflate_raw(&small[300..600], Compression::best()),
            },
            ChunkSpec {
                flag: FLAG_STORED,
                stored: small[600..].to_vec(),
            },
        ];
        let (wrapped, _) = build_padded_chunkzip_flagged(2, 700, 300, 3, &chunks);
        assert_eq!(decompress_chunkzip(&wrapped).unwrap(), small);

        let chunks = [
            ChunkSpec {
                flag: FLAG_STORED,
                stored: small[..350].to_vec(),
            },
            ChunkSpec {
                flag: FLAG_STORED,
                stored: small[350..].to_vec(),
            },
        ];
        let (wrapped, _) = build_padded_chunkzip_flagged(2, 700, 350, 2, &chunks);
        assert_eq!(decompress_chunkzip(&wrapped).unwrap(), small);
    }

    /// A stored chunk may not carry more bytes than the header still owes.
    #[test]
    fn stored_chunk_longer_than_the_declared_remainder_is_rejected() {
        let plain = noise(500, 0x0423);
        let chunks = [
            ChunkSpec {
                flag: FLAG_DEFLATE,
                stored: deflate_raw(&plain[..400], Compression::default()),
            },
            ChunkSpec {
                flag: FLAG_STORED,
                stored: plain[400..].to_vec(), // 100 bytes, but only 50 declared
            },
        ];
        let (wrapped, _) = build_padded_chunkzip_flagged(2, 450, 400, 2, &chunks);
        let err = decompress_chunkzip(&wrapped).unwrap_err();
        assert!(
            matches!(
                err,
                CodecError::StoredChunkTooLong {
                    chunk_index: 1,
                    zsize: 100,
                    remaining: 50,
                    declared: 450
                }
            ),
            "{err}"
        );
    }

    /// Chunks that stop short of `fullsize` are malformed, not "a shorter file".
    /// Both header variants report it; before this check the walker returned the
    /// partial bytes as a success.
    #[test]
    fn short_output_is_rejected_not_truncated() {
        // Padded: fewer chunks than the size needs.
        let plain = noise(600, 0x0424);
        let chunks = [ChunkSpec {
            flag: FLAG_DEFLATE,
            stored: deflate_raw(&plain[..300], Compression::default()),
        }];
        let (wrapped, _) = build_padded_chunkzip_flagged(2, 600, 300, 1, &chunks);
        let err = decompress_chunkzip(&wrapped).unwrap_err();
        assert!(
            matches!(
                err,
                CodecError::ShortOutput {
                    declared: 600,
                    produced: 300,
                    n_chunks: 1
                }
            ),
            "{err}"
        );

        // Padded: the right number of chunks, but the last one inflates short.
        let chunks = [
            ChunkSpec {
                flag: FLAG_DEFLATE,
                stored: deflate_raw(&plain[..300], Compression::default()),
            },
            ChunkSpec {
                flag: FLAG_DEFLATE,
                stored: deflate_raw(&plain[300..500], Compression::default()),
            },
        ];
        let (wrapped, _) = build_padded_chunkzip_flagged(2, 600, 300, 2, &chunks);
        let err = decompress_chunkzip(&wrapped).unwrap_err();
        assert!(
            matches!(
                err,
                CodecError::ShortOutput {
                    declared: 600,
                    produced: 500,
                    n_chunks: 2
                }
            ),
            "{err}"
        );

        // Legacy header: `fullsize` bumped by one without changing the derived
        // chunk count (1000 -> 1001 at 64-byte chunks is still 16 chunks).
        let plain = noise(1000, 0x0425);
        let mut wrapped = build_legacy_chunkzip(&plain, 64);
        wrapped[8..12].copy_from_slice(&1001u32.to_be_bytes());
        let err = decompress_chunkzip(&wrapped).unwrap_err();
        assert!(
            matches!(
                err,
                CodecError::ShortOutput {
                    declared: 1001,
                    produced: 1000,
                    n_chunks: 16
                }
            ),
            "{err}"
        );
    }

    /// A chunk that inflates past `fullsize` is rejected at that chunk, and the
    /// failure names the chunk's flag word.
    #[test]
    fn chunk_inflating_past_fullsize_is_rejected() {
        let plain = noise(400, 0x0426);
        let chunks = [ChunkSpec {
            flag: FLAG_DEFLATE,
            stored: deflate_raw(&plain, Compression::default()),
        }];
        let (wrapped, _) = build_padded_chunkzip_flagged(2, 399, 400, 1, &chunks);
        let err = decompress_chunkzip(&wrapped).unwrap_err();
        assert!(
            matches!(
                err,
                CodecError::DecompressFailed {
                    chunk_index: 0,
                    flag: 1,
                    ..
                }
            ),
            "{err}"
        );
        assert!(err.to_string().contains("(flag 1)"), "{err}");
    }

    /// Only `1` and `4` are observed. An unknown flag is still tried as deflate
    /// (so nothing that decoded before stops decoding), and if that fails the
    /// error names the flag so the variant can be recorded.
    #[test]
    fn unknown_flag_is_tried_as_deflate_and_named_on_failure() {
        let plain = noise(256, 0x0427);
        let chunks = [ChunkSpec {
            flag: 7,
            stored: deflate_raw(&plain, Compression::default()),
        }];
        let (wrapped, _) = build_padded_chunkzip_flagged(2, 256, 256, 1, &chunks);
        assert_eq!(decompress_chunkzip(&wrapped).unwrap(), plain);

        let chunks = [ChunkSpec {
            flag: 7,
            stored: vec![0xff; 64], // not a deflate stream in either framing
        }];
        let (wrapped, _) = build_padded_chunkzip_flagged(2, 256, 256, 1, &chunks);
        let err = decompress_chunkzip(&wrapped).unwrap_err();
        assert!(
            matches!(err, CodecError::DecompressFailed { flag: 7, .. }),
            "{err}"
        );
        assert!(err.to_string().contains("(flag 7)"), "{err}");
    }

    /// Corrupt counts and sizes fail at the bounds check, quickly and without
    /// allocating for the lie: a chunk count that cannot fit, a `zsize` that
    /// would wrap, zero chunks for a non-zero size, and a header cut inside the
    /// flag word.
    #[test]
    fn malformed_padded_framing_fails_fast() {
        let plain = noise(128, 0x0428);
        let one = || ChunkSpec {
            flag: FLAG_DEFLATE,
            stored: deflate_raw(&plain, Compression::default()),
        };

        let (wrapped, _) = build_padded_chunkzip_flagged(2, 128, 128, u32::MAX, &[one()]);
        let err = decompress_chunkzip(&wrapped).unwrap_err();
        assert!(matches!(err, CodecError::TruncatedHeader { .. }), "{err}");

        let (mut wrapped, headers) = build_padded_chunkzip_flagged(2, 128, 128, 1, &[one()]);
        wrapped[headers[0]..headers[0] + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        let err = decompress_chunkzip(&wrapped).unwrap_err();
        assert!(
            matches!(
                err,
                CodecError::TruncatedChunk {
                    chunk_index: 0,
                    zsize: 4_294_967_295,
                    ..
                }
            ),
            "{err}"
        );

        let (wrapped, _) = build_padded_chunkzip_flagged(2, 128, 128, 0, &[]);
        let err = decompress_chunkzip(&wrapped).unwrap_err();
        assert!(
            matches!(
                err,
                CodecError::ShortOutput {
                    declared: 128,
                    produced: 0,
                    n_chunks: 0
                }
            ),
            "{err}"
        );
        let (wrapped, _) = build_padded_chunkzip_flagged(2, 0, 128, 0, &[]);
        assert_eq!(decompress_chunkzip(&wrapped).unwrap(), Vec::<u8>::new());

        let (wrapped, headers) = build_padded_chunkzip_flagged(2, 128, 128, 1, &[one()]);
        let err = decompress_chunkzip(&wrapped[..headers[0] + 6]).unwrap_err();
        assert!(
            matches!(
                err,
                CodecError::TruncatedHeader {
                    needed: 48,
                    have: 46
                }
            ),
            "{err}"
        );
    }

    /// `dummy` 0, 1 and 2 all select the padded walker; the same chunks decode
    /// identically under each. (Every measured PS3 and PS4 member carries 2.)
    #[test]
    fn padded_header_dummy_0_1_2_share_the_walker() {
        let plain = noise(3000, 0x0429);
        let chunks: Vec<ChunkSpec> = plain
            .chunks(1000)
            .map(|c| ChunkSpec {
                flag: FLAG_DEFLATE,
                stored: deflate_raw(c, Compression::default()),
            })
            .collect();
        for dummy in 0..=2u32 {
            let (wrapped, _) = build_padded_chunkzip_flagged(dummy, 3000, 1000, 3, &chunks);
            assert_eq!(
                decompress_chunkzip(&wrapped).unwrap(),
                plain,
                "dummy={dummy}"
            );
        }
    }

    // ----- S1 closure: encoder flag correction (Q-S1.1) -----

    /// The encoder's header, byte by byte: every field at its offset, the chunk
    /// flag `1` at 0x2C, and the deflate stream at 0x30 inflating **through
    /// `flate2` directly** (not through `decompress_chunkzip`) to the input.
    #[test]
    fn compress_chunkzip_header_bytes_and_independent_decode() {
        for case in [
            Vec::new(),
            b"flag one".to_vec(),
            noise(70_000, 0x0501),
            vec![0x11u8; 200_000],
        ] {
            let packed = compress_chunkzip(&case);
            let be = |o: usize| u32::from_be_bytes(packed[o..o + 4].try_into().unwrap());
            assert_eq!(&packed[0..8], MAGIC_CHUNKZIP);
            assert_eq!(be(0x08), 2, "dummy");
            assert_eq!(be(0x0C) as usize, case.len(), "fullsize");
            assert_eq!(be(0x10) as usize, case.len(), "chunk_size");
            assert_eq!(be(0x14), 1, "n_chunks");
            assert_eq!(be(0x18), 16, "word 0x18");
            assert_eq!(&packed[0x1C..0x28], &[0u8; 12], "reserved");
            let zsize = be(0x28) as usize;
            assert_eq!(be(0x2C), FLAG_DEFLATE, "chunk flag must be 1 (deflate)");
            assert_eq!(packed.len(), 0x30 + zsize, "single chunk ends the payload");

            let mut inflated = Vec::new();
            DeflateDecoder::new(&packed[0x30..])
                .read_to_end(&mut inflated)
                .expect("raw deflate stream");
            assert_eq!(
                inflated,
                case,
                "independent inflate of {} bytes",
                case.len()
            );
            assert_eq!(decompress_chunkzip(&packed).unwrap(), case);
        }
    }

    /// Payloads written by the encoder before the flag correction carried `0` in
    /// the flag word. They must keep decoding: rewrite a fresh encoding's flag to
    /// `0` and to the historical builder's shape, and decode both.
    #[test]
    fn historical_flag_zero_encoder_output_still_decodes() {
        let plain = noise(9_000, 0x0502);
        let mut packed = compress_chunkzip(&plain);
        assert_eq!(&packed[0x2C..0x30], &1u32.to_be_bytes());
        packed[0x2C..0x30].copy_from_slice(&0u32.to_be_bytes());
        assert_eq!(decompress_chunkzip(&packed).unwrap(), plain);

        let chunks = [ChunkSpec {
            flag: 0,
            stored: deflate_raw(&plain, Compression::best()),
        }];
        let (wrapped, _) =
            build_padded_chunkzip_flagged(2, plain.len() as u32, plain.len() as u32, 1, &chunks);
        assert_eq!(decompress_chunkzip(&wrapped).unwrap(), plain);
    }
}

/// The `chunklzx` **store** route (research 2026-09-21 handoff 06, `research-r30-chunklzx-store`):
/// any plaintext written as a `chunklzx` member whose LZX streams consist only of uncompressed
/// (type 3) blocks, one per 32 KiB frame, inside the pinned framing of handoff 40. No compression
/// is performed — the member grows to the plaintext + 74–182 bytes — and **loader acceptance is
/// not established** (card R30-LZX-1): this is a syntactically valid stream any LZX decoder
/// implementing block type 3 reads; whether the console's decoder accepts it in these members is
/// a runtime question. Refuses an empty input (the framing has no zero-length member).
///
/// Layout (every field as observed on the 50 shipped vectors): header `chunklzx | u32 2 |
/// fullsize | 0x40000 | n_chunks | 16-byte tail 00000010 00…`; chunk headers at
/// `((cursor − 8 + 15) & !15) + 8` with `zsize | 3`; sub-frames `[u16 csize]` (full frames) and
/// `[FF][u16 usize][u16 csize]` (last frame) then 5 zero bytes; per frame the LZX bits
/// `[E8 bit on the first] 3 bits type 3 | 24 bits length`, mspack's drop-to-bytes (partial word
/// discarded; a whole extra word when on a boundary), `R0 R1 R2 = 1 1 1` (LE u32), the raw bytes,
/// one pad byte after an odd block when another block follows.
pub fn compress_chunklzx_store(plain: &[u8]) -> Result<Vec<u8>, CodecError> {
    const FRAME: usize = 32_768;
    const CHUNK: usize = 0x40000;
    const HEADER_TAIL: [u8; 16] = [0, 0, 0, 0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    if plain.is_empty() {
        return Err(CodecError::Malformed("chunklzx store: an empty member cannot be framed (fullsize 0 is refused by the decoder contract)".into()));
    }
    let fullsize = u32::try_from(plain.len()).map_err(|_| CodecError::Malformed("chunklzx store: member longer than u32".into()))?;
    struct BitWriter {
        out: Vec<u8>,
        buf: u32,
        nbits: u32,
    }
    impl BitWriter {
        fn write(&mut self, value: u32, n: u32) {
            for i in (0..n).rev() {
                self.buf = (self.buf << 1) | ((value >> i) & 1);
                self.nbits += 1;
                if self.nbits == 16 {
                    self.out.extend_from_slice(&(self.buf as u16).to_le_bytes());
                    self.buf = 0;
                    self.nbits = 0;
                }
            }
        }
        fn drop_to_bytes(&mut self) {
            if self.nbits != 0 {
                let w = (self.buf << (16 - self.nbits)) as u16;
                self.out.extend_from_slice(&w.to_le_bytes());
                self.buf = 0;
                self.nbits = 0;
            } else {
                self.out.extend_from_slice(&[0, 0]);
            }
        }
    }
    let n_chunks = plain.len().div_ceil(CHUNK);
    let mut out = Vec::with_capacity(plain.len() + 64 * n_chunks + 40);
    out.extend_from_slice(b"chunklzx");
    out.extend_from_slice(&2u32.to_be_bytes());
    out.extend_from_slice(&fullsize.to_be_bytes());
    out.extend_from_slice(&(CHUNK as u32).to_be_bytes());
    out.extend_from_slice(&(n_chunks as u32).to_be_bytes());
    out.extend_from_slice(&HEADER_TAIL);
    debug_assert_eq!(out.len(), 40);
    for piece in plain.chunks(CHUNK) {
        let nf = piece.len().div_ceil(FRAME);
        let mut payload = Vec::with_capacity(piece.len() + 40 * nf);
        for (k, frame) in piece.chunks(FRAME).enumerate() {
            let mut w = BitWriter { out: Vec::with_capacity(8), buf: 0, nbits: 0 };
            if k == 0 {
                w.write(0, 1); // E8 translation: none
            }
            w.write(3, 3);
            w.write(frame.len() as u32, 24);
            w.drop_to_bytes();
            let mut body = w.out;
            for r in [1u32, 1, 1] {
                body.extend_from_slice(&r.to_le_bytes());
            }
            body.extend_from_slice(frame);
            if frame.len() & 1 == 1 && k + 1 < nf {
                body.push(0);
            }
            let csize = u16::try_from(body.len()).map_err(|_| CodecError::Malformed("chunklzx store: sub-frame longer than u16".into()))?;
            if k + 1 < nf {
                payload.extend_from_slice(&csize.to_be_bytes());
            } else {
                payload.push(0xFF);
                payload.extend_from_slice(&(frame.len() as u16).to_be_bytes());
                payload.extend_from_slice(&csize.to_be_bytes());
            }
            payload.extend_from_slice(&body);
        }
        payload.extend_from_slice(&[0; 5]);
        let h = ((out.len() - 8 + 15) & !15) + 8;
        out.resize(h, 0);
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        out.extend_from_slice(&3u32.to_be_bytes());
        out.extend_from_slice(&payload);
    }
    Ok(out)
}

#[cfg(test)]
mod chunklzx_store_tests {
    use super::*;

    /// Research handoff 06 §2: the synthetic lengths (odd lengths, exact frame / chunk multiples,
    /// > 2 chunks) round-trip through the promoted decoder with the research encoder's exact
    /// sizes (`r30-chunklzx-store-roundtrip.json` `synthetic`); the empty input is refused.
    #[test]
    fn store_route_round_trips_the_research_synthetic_lengths() {
        for (len, encoded) in SYNTHETIC {
            let plain: Vec<u8> = (0..len).map(|i| (i * 7 + i / 251) as u8).collect();
            let e = compress_chunklzx_store(&plain).unwrap();
            assert!(e.starts_with(b"chunklzx"));
            assert_eq!(u32::from_be_bytes(e[12..16].try_into().unwrap()) as usize, len);
            assert_eq!(u32::from_be_bytes(e[20..24].try_into().unwrap()) as usize, len.div_ceil(0x40000));
            assert_eq!(e.len(), encoded, "len {len}: size as the research encoder");
            let back = decompress_chunklzx(&e).unwrap_or_else(|err| panic!("len {len}: {err}"));
            assert_eq!(back, plain, "len {len}");
            assert!(decompress_if_wrapped(&e).unwrap() == plain);
        }
        assert!(compress_chunklzx_store(&[]).is_err());
    }

    const SYNTHETIC: [(usize, usize); 16] = [(1, 75), (2, 76), (3, 77), (15, 89), (16, 90), (17, 91), (32767, 32841), (32768, 32842), (32769, 32861), (65535, 65627), (65536, 65628), (262143, 262343), (262144, 262344), (262145, 262379), (294913, 295165), (786437, 786991)];
}
