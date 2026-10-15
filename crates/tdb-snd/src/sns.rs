//! The EA SNS block stream of one sample (research probe `snd10_sbr.py`, R61):
//!
//! ```text
//! block := u8 type | u24be size (whole block incl. these 4 bytes) | body
//! 0x48 header: body = 8-byte SNR header (two big-endian words)
//!     word 1: version[31:28] | codec[27:24] | channel_config[23:18] | sample_rate[17:0]
//!     word 2: type[31:30] | loop[29] | num_samples[28:0]
//! 0x44 data:   body = u32be samples-in-block | payload
//! 0x45 end
//! ```

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SnsError {
    #[error("no SNS header block (0x48) at offset {0}")]
    NoHeader(u64),
    #[error("block at offset {at}: type 0x{ty:02x} is not a header / data / end block")]
    BadType { at: u64, ty: u8 },
    #[error("block at offset {at}: size {size} runs past the buffer ({len} bytes)")]
    Truncated { at: u64, size: usize, len: usize },
    #[error("stream at offset {0} ends without an end block (0x45)")]
    NoEnd(u64),
}

/// The 8-byte SNR header, field split as the community (vgmstream) names it; the bit positions are
/// what the NHL 21 witnesses confirm (`.DUR` = `num_samples` / `sample_rate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnrHeader {
    pub version: u8,
    pub codec: u8,
    pub channel_config: u8,
    pub sample_rate: u32,
    pub stream_type: u8,
    pub looped: bool,
    pub num_samples: u32,
}

impl SnrHeader {
    pub fn parse(h: &[u8]) -> Option<SnrHeader> {
        if h.len() < 8 {
            return None;
        }
        let w1 = u32::from_be_bytes([h[0], h[1], h[2], h[3]]);
        let w2 = u32::from_be_bytes([h[4], h[5], h[6], h[7]]);
        Some(SnrHeader {
            version: (w1 >> 28) as u8,
            codec: ((w1 >> 24) & 0xf) as u8,
            channel_config: ((w1 >> 18) & 0x3f) as u8,
            sample_rate: w1 & 0x3ffff,
            stream_type: (w2 >> 30) as u8,
            looped: (w2 >> 29) & 1 == 1,
            num_samples: w2 & 0x1fff_ffff,
        })
    }
    /// The community label for the codec nibble (research `snd10_sbr.py` CODECS).
    pub fn codec_label(&self) -> &'static str {
        match self.codec {
            0x00 => "none",
            0x02 => "pcm16be",
            0x03 => "ea-xma",
            0x04 => "ea-xas1",
            0x05 => "ealayer3-v1",
            0x06 => "ealayer3-v2-pcm",
            0x07 => "ealayer3-v2-spike",
            0x09 => "ea-speex",
            0x0b => "ea-mp3",
            0x0c => "ea-opus",
            _ => "unidentified",
        }
    }
    /// The number of channels implied by `channel_config` (0 = mono, 1 = stereo … as observed).
    pub fn channels(&self) -> u32 {
        self.channel_config as u32 + 1
    }
}

/// One `0x44` data block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub offset: u64,
    pub samples: u32,
    pub payload: Vec<u8>,
}

/// Walk one sample's blocks from `off` in `buf`: the header, every data block, and the end block.
/// Returns the header, the data blocks and the offset just past the end block.
pub fn walk_sample(buf: &[u8], off: u64) -> Result<(SnrHeader, Vec<Block>, u64), SnsError> {
    let mut p = off as usize;
    let mut header: Option<SnrHeader> = None;
    let mut blocks = Vec::new();
    loop {
        let Some(&ty) = buf.get(p) else { return Err(header.map_or(SnsError::NoHeader(off), |_| SnsError::NoEnd(off))) };
        if p + 4 > buf.len() {
            return Err(SnsError::Truncated { at: p as u64, size: 4, len: buf.len() });
        }
        let size = u32::from_be_bytes([0, buf[p + 1], buf[p + 2], buf[p + 3]]) as usize;
        if size < 4 || p + size > buf.len() {
            return Err(SnsError::Truncated { at: p as u64, size, len: buf.len() });
        }
        match ty {
            0x48 => {
                if header.is_some() {
                    return Err(SnsError::BadType { at: p as u64, ty });
                }
                header = Some(SnrHeader::parse(&buf[p + 4..p + size]).ok_or(SnsError::Truncated { at: p as u64, size, len: buf.len() })?);
            }
            0x44 => {
                if header.is_none() {
                    return Err(SnsError::NoHeader(off));
                }
                if size < 8 {
                    return Err(SnsError::Truncated { at: p as u64, size, len: buf.len() });
                }
                let samples = u32::from_be_bytes([buf[p + 4], buf[p + 5], buf[p + 6], buf[p + 7]]);
                blocks.push(Block { offset: p as u64, samples, payload: buf[p + 8..p + size].to_vec() });
            }
            0x45 => {
                let h = header.ok_or(SnsError::NoHeader(off))?;
                return Ok((h, blocks, (p + size) as u64));
            }
            _ => return Err(SnsError::BadType { at: p as u64, ty }),
        }
        p += size;
    }
}

/// Encode a block stream (test helper and fixture builder — not a game writer).
pub fn encode_sample(header: &SnrHeader, blocks: &[(u32, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let w1 = ((header.version as u32) << 28) | ((header.codec as u32) << 24) | ((header.channel_config as u32) << 18) | (header.sample_rate & 0x3ffff);
    let w2 = ((header.stream_type as u32) << 30) | ((header.looped as u32) << 29) | (header.num_samples & 0x1fff_ffff);
    out.push(0x48);
    out.extend_from_slice(&12u32.to_be_bytes()[1..]);
    out.extend_from_slice(&w1.to_be_bytes());
    out.extend_from_slice(&w2.to_be_bytes());
    for (samples, payload) in blocks {
        let size = 8 + payload.len();
        out.push(0x44);
        out.extend_from_slice(&(size as u32).to_be_bytes()[1..]);
        out.extend_from_slice(&samples.to_be_bytes());
        out.extend_from_slice(payload);
    }
    out.push(0x45);
    out.extend_from_slice(&4u32.to_be_bytes()[1..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr() -> SnrHeader {
        SnrHeader { version: 1, codec: 0x0b, channel_config: 0, sample_rate: 48000, stream_type: 1, looped: false, num_samples: 77428 }
    }

    #[test]
    fn round_trips_a_block_stream_and_reports_the_end() {
        let bytes = encode_sample(&hdr(), &[(10368, &[1, 2, 3]), (5000, &[9])]);
        let mut buf = vec![0xEEu8; 7];
        buf.extend_from_slice(&bytes);
        buf.extend_from_slice(&[0x11; 3]);
        let (h, blocks, end) = walk_sample(&buf, 7).unwrap();
        assert_eq!(h, hdr());
        assert_eq!(h.codec_label(), "ea-mp3");
        assert_eq!(h.channels(), 1);
        assert_eq!(blocks.iter().map(|b| (b.samples, b.payload.clone())).collect::<Vec<_>>(), vec![(10368, vec![1, 2, 3]), (5000, vec![9])]);
        assert_eq!(end, 7 + bytes.len() as u64);
    }

    #[test]
    fn refuses_missing_header_bad_type_truncation_and_missing_end() {
        let bytes = encode_sample(&hdr(), &[(10368, &[1, 2, 3])]);
        assert_eq!(walk_sample(&bytes[12..], 0), Err(SnsError::NoHeader(0)));
        let mut bad = bytes.clone();
        bad[12] = 0x47;
        assert_eq!(walk_sample(&bad, 0), Err(SnsError::BadType { at: 12, ty: 0x47 }));
        assert!(matches!(walk_sample(&bytes[..bytes.len() - 1], 0), Err(SnsError::Truncated { .. })));
        assert_eq!(walk_sample(&bytes[..bytes.len() - 4], 0), Err(SnsError::NoEnd(0)));
        assert!(walk_sample(&[], 0).is_err());
    }
}
