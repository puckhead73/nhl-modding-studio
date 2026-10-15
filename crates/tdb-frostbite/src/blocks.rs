//! CAS framing is independent of decoding: unknown codecs remain inspectable.
use crate::{be, invalid, slice, Result};
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct Block {
    pub offset: usize,
    pub decoded_size: u32,
    pub stored_size: u32,
    pub codec: u8,
}
pub fn inspect(bytes: &[u8]) -> Result<Vec<Block>> {
    let mut p = 0;
    let mut blocks = Vec::new();
    while p < bytes.len() {
        let h = slice(bytes, p, 8)?;
        let d = be(h, 0)?;
        if h[5] & 0xf0 != 0x70 {
            return Err(invalid(format!("unsupported CAS flags at {p:#x}")));
        }
        let n = ((h[5] as u32 & 15) << 16) | u16::from_be_bytes([h[6], h[7]]) as u32;
        if d == 0 || d > 0x100000 || n == 0 {
            return Err(invalid("invalid CAS block size"));
        }
        slice(bytes, p + 8, n as usize)?;
        if h[4] == 0 && n != d {
            return Err(invalid("stored block size mismatch"));
        }
        blocks.push(Block {
            offset: p,
            decoded_size: d,
            stored_size: n,
            codec: h[4],
        });
        p += 8 + n as usize;
    }
    Ok(blocks)
}
/// Decode stored blocks; callers may supply a separately installed decoder for
/// compressed codecs. Exact output sizes are always checked by this layer.
pub fn decode_with(
    bytes: &[u8],
    limit: usize,
    mut decoder: impl FnMut(u8, &[u8], usize) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    let blocks = inspect(bytes)?;
    let mut out = Vec::new();
    for block in blocks {
        let n = block.decoded_size as usize;
        if out.len().checked_add(n).filter(|v| *v <= limit).is_none() {
            return Err(invalid("decoded asset exceeds limit"));
        }
        let src = slice(bytes, block.offset + 8, block.stored_size as usize)?;
        if block.codec == 0 {
            out.extend_from_slice(src);
        } else {
            let data = decoder(block.codec, src, n)?;
            if data.len() != n {
                return Err(invalid("decoder output size mismatch"));
            }
            out.extend(data);
        }
    }
    Ok(out)
}
pub fn decode_stored(bytes: &[u8], limit: usize) -> Result<Vec<u8>> {
    decode_with(bytes, limit, |codec, _, _| {
        Err(invalid(format!(
            "codec {codec:#04x} requires a decoder; raw export remains available"
        )))
    })
}
