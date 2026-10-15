//! EA-Speex (SNR codec `0x09`) — the refusing block grammar and a **diagnostic** decode through
//! pinned libspeex (`tdb-speex`). Research R83 (24 Sep 2026) as corrected by R88 §F4 and R93.
//!
//! ```text
//! SNR     : codec 0x9 | channel_config 0 (mono) | 32 000 Hz
//! block   : payload = { u8 length (1..=255) | Speex UWB frame } × k, consuming the payload exactly
//! counts  : first block  samples + 509 = k × 640          (the reference decoder's 509-sample delay)
//!           middle block samples = k × 640
//!           last block   0 ≤ k × 640 − samples < 640      (tail trim)
//!           single block 0 ≤ k × 640 − 509 − samples < 640
//!           Σ block samples = SNR num_samples
//! decode  : one frame per packet, Speex ultra-wideband, enhancer on (R83's reference path);
//!           drop the first 509 samples and the tail trim → exactly num_samples
//! ```
//!
//! Every rule refuses by name before any decoding. The output is a **diagnostic preview — not
//! game-verified**: 509 is a count relationship in the data and the reference decoder's rule, with
//! no game trace (R88 §F4); nothing here was listened to. Nothing is encoded or written.

use crate::sns::{Block, SnrHeader};

/// The EA-Speex codec nibble.
pub const CODEC: u8 = 0x09;
/// Ultra-wideband sample rate.
pub const SAMPLE_RATE: u32 = 32_000;
/// Samples per Speex UWB frame.
pub const FRAME_SAMPLES: u32 = 640;
/// The encoder delay the first block declares (R83 T2; the reference decoder's constant).
pub const DELAY: u32 = 509;

/// The admitted gen7 EA-Speex tuples (research R83 families, each covered by the receipt-29
/// real-vector regression `tests/ea_speex_real_vectors.rs`: SNS identity, grammar, counts and a
/// sample-by-sample comparison within 1 LSB of the libspeex oracle). PS4 `rink_ann_samples[_inxp]`
/// banks are admitted by title in `studio-domain::ps4_speech_bank`. Legacy Recompiled has no R83
/// vector and is not admitted. X360 NHL 14 `ann_eng` was held back in receipt 29 (the SBKR reader
/// followed eight all-zero id-table slots); receipt 31 made the reader skip them and admitted it.
pub static MEASURED: &[crate::ealayer3::Tuple] = &[
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl11", bank: "ann_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl12", bank: "ann_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl13", bank: "ann_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl14", bank: "ann_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl15", bank: "ann_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhllegacy", bank: "ann_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl11", bank: "pbp_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl12", bank: "pbp_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl13", bank: "pbp_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl14", bank: "pbp_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
    crate::ealayer3::Tuple { platform: "ps3", game: "nhl14", bank: "pbp_eng", sample_rate: SAMPLE_RATE, channels: 1, source: "R83" },
];

/// The admitted EA-Speex tuple for `(platform, game, bank stem)`, if any.
pub fn measured(platform: &str, game: &str, bank: &str) -> Option<&'static crate::ealayer3::Tuple> {
    MEASURED.iter().find(|t| t.platform == platform && t.game == game && t.bank == bank)
}

/// The linked libspeex version string ("speex-1.2.1").
pub fn decoder_version() -> String {
    tdb_speex::version()
}

/// The fixed preview label.
pub const LABEL: &str = "diagnostic preview — not game-verified (EA-Speex through libspeex 1.2.1: the first 509 samples and the tail trim are dropped by the count rule, not a heard result)";

/// A named refusal of the grammar or the decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeexRefusal {
    NotEaSpeex { codec: u8, sample_rate: u32, channels: u32 },
    NoBlocks,
    ZeroLength { block: usize, at: usize },
    FrameOverrun { block: usize, at: usize, length: usize, remaining: usize },
    EmptyBlock { block: usize },
    FirstBlockCount { frames: u32, samples: u32 },
    MiddleBlockCount { block: usize, frames: u32, samples: u32 },
    LastBlockCount { block: usize, frames: u32, samples: u32 },
    SingleBlockCount { frames: u32, samples: u32 },
    TotalSamples { blocks: u64, header: u32 },
    Decoder { frame: usize, error: String },
}

impl std::fmt::Display for SpeexRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SpeexRefusal::*;
        match self {
            NotEaSpeex { codec, sample_rate, channels } => write!(f, "not an EA-Speex stream (codec 0x{codec:02x}, {sample_rate} Hz, {channels} channel(s); EA-Speex is 0x09, 32 000 Hz mono)"),
            NoBlocks => write!(f, "the stream has no data blocks"),
            ZeroLength { block, at } => write!(f, "block {block}: a frame length of 0 at payload byte {at}"),
            FrameOverrun { block, at, length, remaining } => write!(f, "block {block}: the frame at payload byte {at} claims {length} bytes but {remaining} remain"),
            EmptyBlock { block } => write!(f, "block {block}: no frames"),
            FirstBlockCount { frames, samples } => write!(f, "first block: {frames} frames × 640 − 509 ≠ {samples} declared samples"),
            MiddleBlockCount { block, frames, samples } => write!(f, "block {block}: {frames} frames × 640 ≠ {samples} declared samples"),
            LastBlockCount { block, frames, samples } => write!(f, "last block {block}: {frames} frames × 640 − {samples} declared samples is outside the tail range 0..640"),
            SingleBlockCount { frames, samples } => write!(f, "single block: {frames} frames × 640 − 509 − {samples} declared samples is outside 0..640"),
            TotalSamples { blocks, header } => write!(f, "the blocks declare {blocks} samples, the header {header}"),
            Decoder { frame, error } => write!(f, "frame {frame}: {error}"),
        }
    }
}

impl std::error::Error for SpeexRefusal {}

/// One parsed stream: frames per block, in order, and the trims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EaSpeexStream {
    pub frames: Vec<Vec<u8>>,
    pub frames_per_block: Vec<u32>,
    pub num_samples: u32,
    /// `frames × 640 − 509 − num_samples`, in `0..640`.
    pub tail_trim: u32,
}

/// Split one block payload into its frames (`u8 length | frame`, consuming the payload exactly).
pub fn split_block(block: usize, payload: &[u8]) -> Result<Vec<Vec<u8>>, SpeexRefusal> {
    let mut frames = Vec::new();
    let mut p = 0usize;
    while p < payload.len() {
        let len = payload[p] as usize;
        if len == 0 {
            return Err(SpeexRefusal::ZeroLength { block, at: p });
        }
        let remaining = payload.len() - p - 1;
        if len > remaining {
            return Err(SpeexRefusal::FrameOverrun { block, at: p, length: len, remaining });
        }
        frames.push(payload[p + 1..p + 1 + len].to_vec());
        p += 1 + len;
    }
    if frames.is_empty() {
        return Err(SpeexRefusal::EmptyBlock { block });
    }
    Ok(frames)
}

/// Check the header and every count rule; returns the frames in order. Nothing is decoded.
pub fn parse(header: &SnrHeader, blocks: &[Block]) -> Result<EaSpeexStream, SpeexRefusal> {
    if header.codec != CODEC || header.sample_rate != SAMPLE_RATE || header.channels() != 1 {
        return Err(SpeexRefusal::NotEaSpeex { codec: header.codec, sample_rate: header.sample_rate, channels: header.channels() });
    }
    if blocks.is_empty() {
        return Err(SpeexRefusal::NoBlocks);
    }
    let mut frames = Vec::new();
    let mut per_block = Vec::new();
    let last = blocks.len() - 1;
    let mut total: u64 = 0;
    for (i, b) in blocks.iter().enumerate() {
        let f = split_block(i, &b.payload)?;
        let k = f.len() as u32;
        let decoded = k as i64 * FRAME_SAMPLES as i64;
        let s = b.samples as i64;
        if blocks.len() == 1 {
            let tail = decoded - DELAY as i64 - s;
            if !(0..FRAME_SAMPLES as i64).contains(&tail) {
                return Err(SpeexRefusal::SingleBlockCount { frames: k, samples: b.samples });
            }
        } else if i == 0 {
            if s + DELAY as i64 != decoded {
                return Err(SpeexRefusal::FirstBlockCount { frames: k, samples: b.samples });
            }
        } else if i == last {
            let tail = decoded - s;
            if !(0..FRAME_SAMPLES as i64).contains(&tail) {
                return Err(SpeexRefusal::LastBlockCount { block: i, frames: k, samples: b.samples });
            }
        } else if s != decoded {
            return Err(SpeexRefusal::MiddleBlockCount { block: i, frames: k, samples: b.samples });
        }
        total += b.samples as u64;
        per_block.push(k);
        frames.extend(f);
    }
    if total != header.num_samples as u64 {
        return Err(SpeexRefusal::TotalSamples { blocks: total, header: header.num_samples });
    }
    let tail = frames.len() as u64 * FRAME_SAMPLES as u64 - DELAY as u64 - header.num_samples as u64;
    Ok(EaSpeexStream { frames, frames_per_block: per_block, num_samples: header.num_samples, tail_trim: tail as u32 })
}

/// The diagnostic decode: every frame through one libspeex UWB decoder (enhancer on), then the
/// 509-sample start and the tail trim dropped. Returns `(pcm, decoded_before_trim)`.
pub fn decode(stream: &EaSpeexStream) -> Result<(Vec<i16>, usize), SpeexRefusal> {
    let mut dec = tdb_speex::UwbDecoder::new(true).map_err(|e| SpeexRefusal::Decoder { frame: 0, error: e.to_string() })?;
    let fs = dec.frame_size();
    let mut out = Vec::with_capacity(stream.frames.len() * fs);
    let mut buf = vec![0i16; fs];
    for (i, f) in stream.frames.iter().enumerate() {
        let n = dec.decode_into(f, &mut buf).map_err(|e| SpeexRefusal::Decoder { frame: i, error: e.to_string() })?;
        if n != FRAME_SAMPLES as usize {
            return Err(SpeexRefusal::Decoder { frame: i, error: format!("{n} samples, expected 640") });
        }
        out.extend_from_slice(&buf[..n]);
    }
    let decoded = out.len();
    let start = DELAY as usize;
    let end = start + stream.num_samples as usize;
    if end > decoded {
        return Err(SpeexRefusal::Decoder { frame: stream.frames.len(), error: format!("decoded {decoded} samples, fewer than 509 + {}", stream.num_samples) });
    }
    Ok((out[start..end].to_vec(), decoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(num_samples: u32) -> SnrHeader {
        SnrHeader { version: 1, codec: CODEC, channel_config: 0, sample_rate: SAMPLE_RATE, stream_type: 0, looped: false, num_samples }
    }

    fn block(frames: usize, samples: u32) -> Block {
        let mut payload = Vec::new();
        for i in 0..frames {
            payload.push(3);
            payload.extend_from_slice(&[i as u8, 0x55, 0xaa]);
        }
        Block { offset: 0, samples, payload }
    }

    #[test]
    fn counts_accept_the_measured_shape() {
        // 11 frames first (11 × 640 − 509 = 6 531), a middle 10-frame block, a last block with trim 216.
        let blocks = vec![block(11, 6531), block(10, 6400), block(3, 640 * 3 - 216)];
        let s = parse(&hdr(6531 + 6400 + 1704), &blocks).unwrap();
        assert_eq!((s.frames.len(), s.frames_per_block.clone(), s.tail_trim), (24, vec![11, 10, 3], 216));
        // Single block: 12 frames, 7 680 − 509 − samples ∈ 0..640 (R83: NHL 21 base, trim 252).
        let one = parse(&hdr(7680 - 509 - 252), &[block(12, 7680 - 509 - 252)]).unwrap();
        assert_eq!(one.tail_trim, 252);
    }

    #[test]
    fn every_rule_refuses_by_name() {
        let good = || vec![block(11, 6531), block(10, 6400), block(3, 1704)];
        let n = 6531 + 6400 + 1704;
        let mut h = hdr(n);
        h.codec = 0x0b;
        assert!(matches!(parse(&h, &good()), Err(SpeexRefusal::NotEaSpeex { .. })));
        let mut b = good();
        b[1].payload[4] = 0; // second frame's length byte
        assert!(matches!(parse(&hdr(n), &b), Err(SpeexRefusal::ZeroLength { block: 1, .. })));
        let mut b = good();
        let l = b[2].payload.len();
        b[2].payload.truncate(l - 3); // last block truncated by 3 bytes
        assert!(matches!(parse(&hdr(n), &b), Err(SpeexRefusal::FrameOverrun { block: 2, .. })));
        let mut b = good();
        b[0].samples += 1;
        assert!(matches!(parse(&hdr(n + 1), &b), Err(SpeexRefusal::FirstBlockCount { .. })));
        let mut b = good();
        b[1].samples += 1;
        assert!(matches!(parse(&hdr(n + 1), &b), Err(SpeexRefusal::MiddleBlockCount { block: 1, .. })));
        let mut b = good();
        b[2].samples = 640 * 3 + 1;
        assert!(matches!(parse(&hdr(6531 + 6400 + 1921), &b), Err(SpeexRefusal::LastBlockCount { .. })));
        assert!(matches!(parse(&hdr(n + 1), &good()), Err(SpeexRefusal::TotalSamples { .. })));
        assert!(matches!(parse(&hdr(0), &[]), Err(SpeexRefusal::NoBlocks)));
        assert!(matches!(parse(&hdr(200), &[block(1, 200)]), Err(SpeexRefusal::SingleBlockCount { .. })), "640 − 509 − 200 < 0");
        assert!(parse(&hdr(100), &[block(1, 100)]).is_ok(), "640 − 509 − 100 = 31 is a valid tail");
    }

    /// Synthetic frames are not real Speex, so the decoder either decodes them to 640 samples each
    /// or refuses by name — never a panic or a process exit.
    #[test]
    fn decode_of_synthetic_frames_is_bounded() {
        let s = parse(&hdr(6531 + 6400 + 1704), &[block(11, 6531), block(10, 6400), block(3, 1704)]).unwrap();
        match decode(&s) {
            Ok((pcm, decoded)) => assert_eq!((pcm.len(), decoded), (6531 + 6400 + 1704, 24 * 640)),
            Err(SpeexRefusal::Decoder { .. }) => {}
            Err(e) => panic!("{e}"),
        }
    }
}
