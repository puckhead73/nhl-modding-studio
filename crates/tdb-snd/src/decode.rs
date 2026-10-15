//! **Diagnostic** MPEG-1 Layer III decode of the concatenated EA-MP3 frames, and the two
//! renderings kept apart (research R71 §4 contract rev. 2):
//!
//! * [`Rendering::MpegOnly`] — the decoder's output, frames × 1 152 samples;
//! * [`Rendering::CandidateA`] — the embedded 1 152 PCM samples **replace decoded frame 0**
//!   (strong static support on 20 / 24 vectors, four inconclusive; **not** listening-verified);
//!
//! both trimmed to the SNR sample count. Candidate B (PCM as a pre-roll before the whole stream)
//! is not offered: R71 §1–2 reject it on this set (the decoded stream's first 161–880 samples are
//! below 1 LSB).
//!
//! Decoder: `symphonia-bundle-mp3` 0.6 (pure Rust, MPL-2.0), one packet per MPEG frame with
//! gapless trimming **off**, so the output is exactly frames × 1 152 samples and a decoder error on
//! any frame is a refusal (never a silent gap). Sample rate and channel count are taken from the
//! first frame's header; MPEG-1 mono / stereo both decode, though the EA-MP3 contract only admits
//! 48 000 Hz mono upstream.

use symphonia_bundle_mp3::MpaDecoder;
use symphonia_core::audio::{Channels, Position};
use symphonia_core::codecs::audio::well_known::CODEC_ID_MP3;
use symphonia_core::codecs::audio::{AudioCodecParameters, AudioDecoder, AudioDecoderOptions};
use symphonia_core::packet::PacketRef;
use symphonia_core::units::{Duration, Timestamp};
use thiserror::Error;

use crate::eamp3::MpegHeader;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("no MPEG frame at byte {0}")]
    NoHeader(usize),
    #[error("frame {index} at byte {at}: truncated ({have} of {need} bytes)")]
    Truncated { index: usize, at: usize, have: usize, need: usize },
    #[error("frame {index}: {rate} Hz {mode} differs from the first frame ({rate0} Hz {mode0})")]
    Mixed { index: usize, rate: u32, mode: &'static str, rate0: u32, mode0: &'static str },
    #[error("decoder could not be created: {0}")]
    Create(String),
    #[error("frame {index}: decoder error: {reason}")]
    Frame { index: usize, reason: String },
    #[error("frame {index}: decoder returned {got} samples per channel, expected {expected}")]
    FrameLength { index: usize, got: usize, expected: usize },
    #[error("the embedded PCM has {0} samples, not 1152")]
    EmbeddedLength(usize),
}

/// Decoded interleaved 16-bit PCM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub sample_rate: u32,
    pub channels: u32,
    pub frames: usize,
    /// 1 152 (MPEG-1) or 576 (MPEG-2).
    pub samples_per_frame: usize,
    /// Interleaved i16, `frames × samples_per_frame × channels` samples.
    pub pcm: Vec<i16>,
}

/// Decode a concatenated Layer III stream (MPEG-1 or MPEG-2, one class per stream) frame by frame.
/// Every frame must decode; a frame the decoder refuses is a refusal of the whole sample.
pub fn decode_layer3(mpeg: &[u8]) -> Result<Decoded, DecodeError> {
    let (first, mpeg1) = MpegHeader::parse_any(mpeg).ok_or(DecodeError::NoHeader(0))?;
    let channels = if first.mode == 3 { 1 } else { 2 };
    let per_frame = if mpeg1 { 1152 } else { 576 };
    let mut params = AudioCodecParameters::new();
    params.for_codec(CODEC_ID_MP3).with_sample_rate(first.sample_rate).with_channels(if channels == 1 {
        Channels::Positioned(Position::FRONT_CENTER)
    } else {
        Channels::Positioned(Position::FRONT_LEFT | Position::FRONT_RIGHT)
    });
    let opts = AudioDecoderOptions::default().gapless(false);
    let mut dec = MpaDecoder::try_new(&params, &opts).map_err(|e| DecodeError::Create(e.to_string()))?;

    let mut pcm: Vec<i16> = Vec::new();
    let mut scratch: Vec<i16> = Vec::new();
    let mut at = 0usize;
    let mut index = 0usize;
    while at < mpeg.len() {
        let (h, m1) = MpegHeader::parse_any(&mpeg[at..]).ok_or(DecodeError::NoHeader(at))?;
        if h.sample_rate != first.sample_rate || h.mode != first.mode || m1 != mpeg1 {
            return Err(DecodeError::Mixed { index, rate: h.sample_rate, mode: h.mode_label(), rate0: first.sample_rate, mode0: first.mode_label() });
        }
        let end = at + h.frame_bytes;
        if end > mpeg.len() {
            return Err(DecodeError::Truncated { index, at, have: mpeg.len() - at, need: h.frame_bytes });
        }
        let packet = PacketRef::new(0, Timestamp::new((index * per_frame) as i64), Duration::new(per_frame as u64), &mpeg[at..end]);
        let buf = dec.decode_ref(&packet).map_err(|e| DecodeError::Frame { index, reason: e.to_string() })?;
        if buf.frames() != per_frame {
            return Err(DecodeError::FrameLength { index, got: buf.frames(), expected: per_frame });
        }
        scratch.clear();
        buf.copy_to_vec_interleaved::<i16>(&mut scratch);
        pcm.extend_from_slice(&scratch);
        at = end;
        index += 1;
    }
    Ok(Decoded { sample_rate: first.sample_rate, channels, frames: index, samples_per_frame: per_frame, pcm })
}

/// Which rendering of the sample's start is produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rendering {
    /// The decoder's own output (frame 0 included, degraded by the codec delay).
    MpegOnly,
    /// The embedded 1 152 samples replace decoded frame 0 (R71 candidate A; not listening-verified).
    CandidateA,
}

impl Rendering {
    /// The label a preview must carry (contract rev. 2).
    pub fn label(self) -> &'static str {
        match self {
            Rendering::MpegOnly => "diagnostic preview (MPEG frames only; not game-verified)",
            Rendering::CandidateA => "diagnostic preview (start rendered by candidate A; not game-verified)",
        }
    }
}

/// Assemble a mono rendering and trim it to `num_samples`.
pub fn render_candidate_a(decoded: &Decoded, embedded: &[i16], rendering: Rendering, num_samples: u32) -> Result<Vec<i16>, DecodeError> {
    let mut out = match rendering {
        Rendering::MpegOnly => decoded.pcm.clone(),
        Rendering::CandidateA => {
            if embedded.len() != 1152 {
                return Err(DecodeError::EmbeddedLength(embedded.len()));
            }
            let mut v = embedded.to_vec();
            v.extend_from_slice(decoded.pcm.get(1152 * decoded.channels as usize..).unwrap_or(&[]));
            v
        }
    };
    out.truncate(num_samples as usize * decoded.channels as usize);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eamp3::synthetic_frame;

    #[test]
    fn decodes_synthetic_silent_frames_to_exactly_1152_samples_each() {
        let mut s = Vec::new();
        for _ in 0..5 {
            s.extend(synthetic_frame(96, 0));
        }
        let d = decode_layer3(&s).unwrap();
        assert_eq!((d.sample_rate, d.channels, d.frames, d.samples_per_frame, d.pcm.len()), (48000, 1, 5, 1152, 5 * 1152));
        assert!(d.pcm.iter().all(|&x| x.abs() <= 1), "all-zero frames decode to silence");
        let emb: Vec<i16> = (0..1152).map(|i| i as i16).collect();
        let a = render_candidate_a(&d, &emb, Rendering::CandidateA, 5 * 1152 - 100).unwrap();
        assert_eq!(a.len(), 5 * 1152 - 100);
        assert_eq!(&a[..1152], &emb[..]);
        let m = render_candidate_a(&d, &emb, Rendering::MpegOnly, 5 * 1152 - 100).unwrap();
        assert_eq!(m.len(), 5 * 1152 - 100);
        assert!(matches!(render_candidate_a(&d, &emb[..10], Rendering::CandidateA, 100), Err(DecodeError::EmbeddedLength(10))));
        assert!(Rendering::CandidateA.label().contains("not game-verified"));
    }

    #[test]
    fn refuses_truncation_and_mixed_streams() {
        let mut s = synthetic_frame(96, 0);
        s.truncate(s.len() - 1);
        assert!(matches!(decode_layer3(&s), Err(DecodeError::Truncated { index: 0, .. })));
        assert!(matches!(decode_layer3(&[0, 1, 2, 3]), Err(DecodeError::NoHeader(0))));
        let mut m = synthetic_frame(96, 0);
        let mut stereo = synthetic_frame(96, 0);
        stereo[3] = 0x00;
        m.extend(stereo);
        assert!(matches!(decode_layer3(&m), Err(DecodeError::Mixed { index: 1, .. })));
    }
}
