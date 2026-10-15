//! The PS4 EA-MP3 (SNS codec `0x0b`) **sized-frame grammar** — research R67 §1, as the refusing
//! parser of R71 §3 / contract rev. 2 (R75-corrected):
//!
//! ```text
//! block payload := ea_frame+                    -- sizes tile the payload exactly
//! ea_frame      := u16le hdr | [u32le pcm_samples if hdr & 0x8000] | mpeg_frame | [pcm_samples × s16le]
//! hdr & 0x7fff  == total bytes of the ea_frame
//! ```
//!
//! Contract checks (each refused **by name**, in this order per frame): sizes tile; a valid MPEG-1
//! Layer III header at the frame start; the `0x8000` flag only on the very first frame of the
//! sample, carrying exactly 1 152 PCM samples; 48 000 Hz mono; `main_data_begin` ≤ the bit
//! reservoir accumulated so far (capped at 511); nine frames in every block that declares 10 368
//! samples; and — before any of that — Σ declared block samples = the SNR sample count, the only
//! static check that catches a dropped or duplicated whole block (R71 T5: 16 / 16 applicable).

use thiserror::Error;

/// Bitrate (kbps) by header index, MPEG-1 Layer III.
const BITRATE: [u32; 16] = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0];
const RATE: [u32; 4] = [44100, 48000, 32000, 0];

/// A parsed MPEG-1 Layer III frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpegHeader {
    pub bitrate_kbps: u32,
    pub sample_rate: u32,
    pub padding: bool,
    /// 0 stereo, 1 joint, 2 dual, 3 mono.
    pub mode: u8,
    pub crc: bool,
    pub frame_bytes: usize,
}

impl MpegHeader {
    /// MPEG-1 Layer III header at `b[0..4]`, or `None`: sync, version bits `11` (MPEG-1 only — the
    /// contract's frames; MPEG-2 `10` is rejected), layer `01`, a valid bitrate / rate index — `FF FA/FB xx xx`.
    pub fn parse(b: &[u8]) -> Option<MpegHeader> {
        if b.len() < 4 || b[0] != 0xFF || (b[1] & 0xFE) != 0xFA {
            return None;
        }
        let br = BITRATE[(b[2] >> 4) as usize];
        let sr = RATE[((b[2] >> 2) & 3) as usize];
        if br == 0 || sr == 0 {
            return None;
        }
        let padding = (b[2] >> 1) & 1 == 1;
        Some(MpegHeader {
            bitrate_kbps: br,
            sample_rate: sr,
            padding,
            mode: b[3] >> 6,
            crc: b[1] & 1 == 0,
            frame_bytes: (144 * br * 1000 / sr) as usize + padding as usize,
        })
    }
    /// Any MPEG-1 or MPEG-2 Layer III header (the gen7 24 kHz rebuilds are MPEG-2): `FF FA/FB`
    /// (MPEG-1) or `FF F2/F3` (MPEG-2). `frame_bytes` follows the layer's slot rule; samples per
    /// frame are 1 152 (MPEG-1) or 576 (MPEG-2).
    pub fn parse_any(b: &[u8]) -> Option<(MpegHeader, bool)> {
        if b.len() < 4 || b[0] != 0xFF {
            return None;
        }
        let mpeg1 = match b[1] & 0xFE {
            0xFA => true,
            0xF2 => false,
            _ => return None,
        };
        if mpeg1 {
            return MpegHeader::parse(b).map(|h| (h, true));
        }
        const BR2: [u32; 16] = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0];
        const RATE2: [u32; 4] = [22050, 24000, 16000, 0];
        let br = BR2[(b[2] >> 4) as usize];
        let sr = RATE2[((b[2] >> 2) & 3) as usize];
        if br == 0 || sr == 0 {
            return None;
        }
        let padding = (b[2] >> 1) & 1 == 1;
        Some((
            MpegHeader { bitrate_kbps: br, sample_rate: sr, padding, mode: b[3] >> 6, crc: b[1] & 1 == 0, frame_bytes: (72 * br * 1000 / sr) as usize + padding as usize },
            false,
        ))
    }
    pub fn mode_label(&self) -> &'static str {
        ["stereo", "joint", "dual", "mono"][self.mode as usize]
    }
    /// `main_data_begin` (9 bits) of a single-channel frame's side info.
    pub fn main_data_begin_mono(&self, frame: &[u8]) -> Option<u16> {
        let p = 4 + if self.crc { 2 } else { 0 };
        frame.get(p + 1).map(|_| ((frame[p] as u16) << 1) | (frame[p + 1] as u16 >> 7))
    }
}

/// One EA frame of a block payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EaFrame {
    /// Byte offset inside the block payload.
    pub at: usize,
    pub size: usize,
    pub flag: bool,
    pub pcm_samples: Option<u32>,
    pub header: MpegHeader,
    /// Whether the MPEG frame (+ PCM) exactly fills the declared size.
    pub exact: bool,
    pub mpeg: Vec<u8>,
    /// The embedded s16le PCM, when flagged.
    pub pcm: Vec<u8>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EaMp3Refusal {
    #[error("sum of declared block samples {declared} != SNR num_samples {expected}")]
    BlockSamples { declared: u64, expected: u32 },
    #[error("block {block}: {reason}")]
    Block { block: usize, reason: String },
    #[error("block {block}: no frames")]
    NoFrames { block: usize },
    #[error("block {block}: a frame size does not tile its ea_frame")]
    Tiling { block: usize },
    #[error("block {block} frame {frame}: PCM flag {}", if *.set { "set off the first frame" } else { "missing on the first frame" })]
    Flag { block: usize, frame: usize, set: bool },
    #[error("block {block} frame {frame}: pcm_samples {found:?} (contract: 1152)")]
    PcmSamples { block: usize, frame: usize, found: Option<u32> },
    #[error("block {block} frame {frame}: {rate} Hz {mode} (contract: 48000 mono)")]
    RateMode { block: usize, frame: usize, rate: u32, mode: &'static str },
    #[error("block {block} frame {frame}: side info unreadable")]
    SideInfo { block: usize, frame: usize },
    #[error("block {block} frame {frame}: main_data_begin {mdb} exceeds the reservoir ({reservoir} bytes available)")]
    Reservoir { block: usize, frame: usize, mdb: u16, reservoir: u32 },
    #[error("block {block}: {frames} frames for 10368 declared samples")]
    NineFrames { block: usize, frames: usize },
    #[error("SNR header: codec 0x{codec:02x} ({label}), {rate} Hz, channel config {channels}, type {ty} — not the EA-MP3 contract (0x0b, 48000, mono, stream)")]
    NotEaMp3 { codec: u8, label: &'static str, rate: u32, channels: u8, ty: u8 },
}

/// Walk one block payload as EA frames (R67 `parse_block`). Returns the frames read so far and the
/// first error, if any.
pub fn parse_block(pl: &[u8]) -> (Vec<EaFrame>, Option<String>) {
    let mut p = 0usize;
    let mut frames = Vec::new();
    while p < pl.len() {
        if p + 2 > pl.len() {
            return (frames, Some(format!("trailing {} bytes", pl.len() - p)));
        }
        let hdr = u16::from_le_bytes([pl[p], pl[p + 1]]);
        let size = (hdr & 0x7fff) as usize;
        let flag = hdr & 0x8000 != 0;
        if size < 6 || p + size > pl.len() {
            return (frames, Some(format!("size {size} at {p} exceeds payload {}", pl.len())));
        }
        let mut q = p + 2;
        let mut pcm_n = None;
        if flag {
            if q + 4 > pl.len() {
                return (frames, Some(format!("pcm_samples word at {q} exceeds payload {}", pl.len())));
            }
            pcm_n = Some(u32::from_le_bytes([pl[q], pl[q + 1], pl[q + 2], pl[q + 3]]));
            q += 4;
        }
        let Some(h) = MpegHeader::parse(&pl[q..pl.len().min(q + 4)]) else {
            return (frames, Some(format!("no MPEG header at {q} (ea_frame at {p}, size {size}, flag {flag})")));
        };
        let fb = h.frame_bytes;
        let end_mpeg = q + fb;
        let mpeg = pl.get(q..end_mpeg.min(p + size)).unwrap_or(&[]).to_vec();
        let mut q2 = end_mpeg;
        let mut pcm = Vec::new();
        if let Some(n) = pcm_n {
            let pcm_end = q2.saturating_add(2 * n as usize);
            pcm = pl.get(q2..pcm_end.min(p + size)).unwrap_or(&[]).to_vec();
            q2 = pcm_end;
        }
        frames.push(EaFrame { at: p, size, flag, pcm_samples: pcm_n, header: h, exact: q2 == p + size && mpeg.len() == fb, mpeg, pcm });
        p += size;
    }
    (frames, None)
}

/// The contract-parsed sample: the concatenated MPEG frames of every block (the reservoir crosses
/// blocks — never decode one block alone), the embedded PCM of frame 0, and the counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSample {
    pub frames_per_block: Vec<usize>,
    pub total_frames: usize,
    pub mpeg: Vec<u8>,
    /// The 1 152 embedded samples (s16le decoded to i16).
    pub embedded_pcm: Vec<i16>,
    /// `frames × 1152 − num_samples` (the tail trim declared by the SNR count).
    pub trim: usize,
    pub num_samples: u32,
    pub sample_rate: u32,
}

/// The R71 refusing parser over the data blocks `(declared samples, payload)` of one sample.
pub fn contract_parse(blocks: &[(u32, &[u8])], num_samples: u32) -> Result<ParsedSample, EaMp3Refusal> {
    let declared: u64 = blocks.iter().map(|(n, _)| *n as u64).sum();
    if declared != num_samples as u64 {
        return Err(EaMp3Refusal::BlockSamples { declared, expected: num_samples });
    }
    let mut mpeg = Vec::new();
    let mut per_block = Vec::with_capacity(blocks.len());
    let mut reservoir: u32 = 0;
    let mut embedded = Vec::new();
    let mut rate = 48000;
    for (bi, (n, pl)) in blocks.iter().enumerate() {
        let (frames, err) = parse_block(pl);
        if let Some(reason) = err {
            return Err(EaMp3Refusal::Block { block: bi, reason });
        }
        if frames.is_empty() {
            return Err(EaMp3Refusal::NoFrames { block: bi });
        }
        if frames.iter().any(|f| !f.exact) {
            return Err(EaMp3Refusal::Tiling { block: bi });
        }
        for (j, f) in frames.iter().enumerate() {
            let first = bi == 0 && j == 0;
            if f.flag != first {
                return Err(EaMp3Refusal::Flag { block: bi, frame: j, set: f.flag });
            }
            if f.flag && f.pcm_samples != Some(1152) {
                return Err(EaMp3Refusal::PcmSamples { block: bi, frame: j, found: f.pcm_samples });
            }
            if f.header.sample_rate != 48000 || f.header.mode != 3 {
                return Err(EaMp3Refusal::RateMode { block: bi, frame: j, rate: f.header.sample_rate, mode: f.header.mode_label() });
            }
            rate = f.header.sample_rate;
            let Some(mdb) = f.header.main_data_begin_mono(&f.mpeg) else {
                return Err(EaMp3Refusal::SideInfo { block: bi, frame: j });
            };
            if mdb as u32 > reservoir {
                return Err(EaMp3Refusal::Reservoir { block: bi, frame: j, mdb, reservoir });
            }
            let crc = if f.header.crc { 2 } else { 0 };
            let main = f.header.frame_bytes as u32 - 4 - 17 - crc;
            reservoir = (reservoir - mdb as u32 + main).min(511);
            mpeg.extend_from_slice(&f.mpeg);
            if f.flag {
                embedded = f.pcm.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
            }
        }
        if *n == 10368 && frames.len() != 9 {
            return Err(EaMp3Refusal::NineFrames { block: bi, frames: frames.len() });
        }
        per_block.push(frames.len());
    }
    let total: usize = per_block.iter().sum();
    Ok(ParsedSample {
        frames_per_block: per_block,
        total_frames: total,
        mpeg,
        embedded_pcm: embedded,
        trim: (total * 1152).saturating_sub(num_samples as usize),
        num_samples,
        sample_rate: rate,
    })
}

/// The SNR-level applicability gate of the contract (R67 §4): codec `0x0b`, 48 000 Hz, channel
/// config 0 (mono), stream type.
pub fn check_header(h: &crate::sns::SnrHeader) -> Result<(), EaMp3Refusal> {
    if h.codec != 0x0b || h.sample_rate != 48000 || h.channel_config != 0 || h.stream_type != 1 {
        return Err(EaMp3Refusal::NotEaMp3 { codec: h.codec, label: h.codec_label(), rate: h.sample_rate, channels: h.channel_config, ty: h.stream_type });
    }
    Ok(())
}

/// A synthetic MPEG-1 Layer III mono frame for tests and fixtures: a valid header, CRC off,
/// `main_data_begin` = `mdb`, the rest zero (a decoder renders it as silence).
pub fn synthetic_frame(bitrate_kbps: u32, mdb: u16) -> Vec<u8> {
    let idx = BITRATE.iter().position(|b| *b == bitrate_kbps).expect("a table bitrate") as u8;
    // FF FB: sync, MPEG-1, Layer III, no CRC. byte 2: bitrate idx | rate 48 kHz (01) | no padding.
    // byte 3: mode 3 (mono).
    let hdr = [0xFF, 0xFB, (idx << 4) | (1 << 2), 0xC0];
    let h = MpegHeader::parse(&hdr).unwrap();
    let mut f = vec![0u8; h.frame_bytes];
    f[..4].copy_from_slice(&hdr);
    f[4] = (mdb >> 1) as u8;
    f[5] = ((mdb & 1) as u8) << 7;
    f
}

/// Build one EA frame (test helper): `hdr | [pcm_samples] | mpeg | [pcm]`.
pub fn synthetic_ea_frame(mpeg: &[u8], pcm: Option<&[i16]>) -> Vec<u8> {
    let mut out = Vec::new();
    let size = 2 + pcm.map_or(0, |p| 4 + 2 * p.len()) + mpeg.len();
    let hdr = size as u16 | if pcm.is_some() { 0x8000 } else { 0 };
    out.extend_from_slice(&hdr.to_le_bytes());
    if let Some(p) = pcm {
        out.extend_from_slice(&(p.len() as u32).to_le_bytes());
    }
    out.extend_from_slice(mpeg);
    if let Some(p) = pcm {
        for s in p {
            out.extend_from_slice(&s.to_le_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two blocks: 9 frames (10 368 samples, the first with 1 152 embedded samples) + 3 frames.
    fn sample() -> (Vec<Vec<u8>>, Vec<u32>, u32) {
        let pcm: Vec<i16> = (0..1152).map(|i| (i as i16).wrapping_mul(7)).collect();
        let mut b0 = synthetic_ea_frame(&synthetic_frame(64, 0), Some(&pcm));
        for _ in 0..8 {
            b0.extend(synthetic_ea_frame(&synthetic_frame(96, 100), None));
        }
        let mut b1 = Vec::new();
        for _ in 0..3 {
            b1.extend(synthetic_ea_frame(&synthetic_frame(128, 200), None));
        }
        (vec![b0, b1], vec![10368, 3456 - 500], 10368 + 3456 - 500)
    }

    #[test]
    fn parses_the_grammar_and_concatenates_the_frames() {
        let (blocks, samples, n) = sample();
        let refs: Vec<(u32, &[u8])> = samples.iter().copied().zip(blocks.iter().map(|b| b.as_slice())).collect();
        let p = contract_parse(&refs, n).unwrap();
        assert_eq!(p.frames_per_block, vec![9, 3]);
        assert_eq!(p.total_frames, 12);
        assert_eq!(p.embedded_pcm.len(), 1152);
        assert_eq!(p.embedded_pcm[3], 21);
        assert_eq!(p.trim, 500);
        assert_eq!(p.mpeg.len(), synthetic_frame(64, 0).len() + 8 * synthetic_frame(96, 0).len() + 3 * synthetic_frame(128, 0).len());
        assert!(p.mpeg.starts_with(&[0xFF, 0xFB]));
    }

    #[test]
    fn refuses_every_mutation_class_by_name() {
        let (blocks, samples, n) = sample();
        let run = |blocks: &[Vec<u8>], samples: &[u32], n: u32| {
            let refs: Vec<(u32, &[u8])> = samples.iter().copied().zip(blocks.iter().map(|b| b.as_slice())).collect();
            contract_parse(&refs, n)
        };
        // dropped block
        assert_eq!(run(&blocks[..1], &samples[..1], n), Err(EaMp3Refusal::BlockSamples { declared: 10368, expected: n }));
        // truncated last block
        let mut t = blocks.clone();
        let l = t[1].len();
        t[1].truncate(l - 3);
        assert!(matches!(run(&t, &samples, n), Err(EaMp3Refusal::Block { block: 1, .. })));
        // size word past the payload
        let mut t = blocks.clone();
        t[1][0..2].copy_from_slice(&(0x7fffu16).to_le_bytes());
        assert!(matches!(run(&t, &samples, n), Err(EaMp3Refusal::Block { block: 1, .. })));
        // size word − 1 → the frame no longer tiles (or the next header is missing)
        let mut t = blocks.clone();
        let hdr = u16::from_le_bytes([t[1][0], t[1][1]]) - 1;
        t[1][0..2].copy_from_slice(&hdr.to_le_bytes());
        assert!(matches!(run(&t, &samples, n), Err(EaMp3Refusal::Block { block: 1, .. }) | Err(EaMp3Refusal::Tiling { block: 1 })));
        // flag forced onto block 1 frame 0
        let mut t = blocks.clone();
        let hdr = u16::from_le_bytes([t[1][0], t[1][1]]) | 0x8000;
        t[1][0..2].copy_from_slice(&hdr.to_le_bytes());
        assert!(matches!(run(&t, &samples, n), Err(EaMp3Refusal::Block { block: 1, .. })));
        // flag on frame 0 but the wrong pcm_samples count (the frame no longer tiles)
        let mut t = blocks.clone();
        t[0][2..6].copy_from_slice(&4096u32.to_le_bytes());
        assert!(matches!(run(&t, &samples, n), Err(EaMp3Refusal::Tiling { block: 0 }) | Err(EaMp3Refusal::Block { block: 0, .. })));
        // reservoir: first frame with main_data_begin > 0
        let mut t = blocks.clone();
        t[0][6 + 4] = 0x10; // mdb = 32 on frame 0 (reservoir 0)
        assert_eq!(run(&t, &samples, n), Err(EaMp3Refusal::Reservoir { block: 0, frame: 0, mdb: 32, reservoir: 0 }));
        // stereo header
        let mut t = blocks.clone();
        t[0][6 + 3] = 0x00;
        assert_eq!(run(&t, &samples, n), Err(EaMp3Refusal::RateMode { block: 0, frame: 0, rate: 48000, mode: "stereo" }));
        // eight frames for a 10 368-sample block
        let mut t = blocks.clone();
        let one = synthetic_ea_frame(&synthetic_frame(96, 100), None).len();
        let l = t[0].len();
        t[0].truncate(l - one);
        assert_eq!(run(&t, &samples, n), Err(EaMp3Refusal::NineFrames { block: 0, frames: 8 }));
        // missing flag on the first frame
        let mut t = blocks.clone();
        t[0] = t[0][2 + 4 + synthetic_frame(64, 0).len() + 2304..].to_vec();
        assert!(matches!(run(&t, &samples, n), Err(EaMp3Refusal::Flag { block: 0, frame: 0, set: false })));
        // the header gate
        let h = crate::sns::SnrHeader { version: 1, codec: 0x0c, channel_config: 0, sample_rate: 48000, stream_type: 1, looped: false, num_samples: 1 };
        assert!(matches!(check_header(&h), Err(EaMp3Refusal::NotEaMp3 { codec: 0x0c, label: "ea-opus", .. })));
    }

    #[test]
    fn the_header_parser_rejects_non_layer3_and_reserved_indices() {
        assert!(MpegHeader::parse(&[0xFF, 0xFB, 0x00, 0xC0]).is_none(), "free bitrate");
        assert!(MpegHeader::parse(&[0xFF, 0xFB, 0xF4, 0xC0]).is_none(), "bad bitrate index");
        assert!(MpegHeader::parse(&[0xFF, 0xFB, 0x9C, 0xC0]).is_none(), "reserved rate");
        assert!(MpegHeader::parse(&[0xFF, 0xF3, 0x94, 0xC0]).is_none(), "MPEG-2");
        assert!(MpegHeader::parse(&[0xFF, 0xFD, 0x94, 0xC0]).is_none(), "Layer II");
        let h = MpegHeader::parse(&[0xFF, 0xFB, 0x94, 0xC0]).unwrap();
        assert_eq!((h.bitrate_kbps, h.sample_rate, h.mode_label(), h.frame_bytes, h.crc), (128, 48000, "mono", 384, false));
    }
}
