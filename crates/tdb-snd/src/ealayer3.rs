//! Gen7 **EALayer3 v1** (SNS codec `0x05`, PS3 / Xbox 360 NHL 09–Legacy speech and music) — the
//! granule grammar of research R77 (2026-09-22 pm handoff 09, R79-narrowed) as extended by the R80
//! survey (handoff 12), and the **diagnostic** rebuild into standard MPEG frames:
//!
//! ```text
//! block payload := ea_frame+                       -- tiles exactly
//! ea_frame      := u8 pcm_flag (00 | EE)
//!                  u8 header  version(2) sr_index(2) channel_mode(2) mode_ext(2)
//!                  bit granule_index(1)
//!                  [ MPEG-1 && granule_index == 1: scfsi(4) per channel ]
//!                  per channel: part2_3_length(12) side_info(47 MPEG-1 | 51 MPEG-2)
//!                  main_data(Σ part2_3_length bits)  pad-to-byte
//!                  [ pcm_flag == EE: u16 offset_samples  u16 pcm_samples  u32 zero  pcm_samples × nch × s16 ]
//! ```
//!
//! The first block carries two extra granules (1 152 samples) and exactly one `EE` frame — the
//! **second** EA-frame of block 0 — with 47 samples (R77 §1, R79 correction). The rebuild (R77 §2)
//! packs two granules (MPEG-1) or one (MPEG-2) into a standard frame at the maximum table bitrate,
//! placing every frame's main data in a continuous stream through the standard **bit reservoir**
//! (`main_data_begin` ≤ 511 / 255); a conforming Layer III decoder then returns `granules × 576`
//! samples. The audible start (two warm-up granules + the 47 PCM samples) is **unresolved**
//! (Q-R77-1): the output is labelled *"diagnostic preview — start handling unresolved; not
//! game-verified"*. The PS4 EA-MP3 grammar does not apply here and is not used.
//!
//! Refusals by name (R80 §2, 3 / 3 classes): a truncated block ("EOF inside granule"), an unknown
//! flag byte, a PCM count past the payload; plus the header-class, granule-count and PCM-frame
//! position rules of R77 §1. Coverage is a *measured-tuple* question decided by the caller
//! ([`Tuple`]): this module parses what it is given and states what it saw.

use thiserror::Error;

const RATES_MPEG1: [u32; 3] = [44100, 48000, 32000];
const RATES_MPEG2: [u32; 3] = [22050, 24000, 16000];

/// MSB-first bit reader (the research `Bits` class).
struct Bits<'a> {
    b: &'a [u8],
    pos: usize,
}
impl<'a> Bits<'a> {
    fn new(b: &'a [u8], pos_bits: usize) -> Self {
        Bits { b, pos: pos_bits }
    }
    fn get(&mut self, k: usize) -> Option<u32> {
        if self.pos + k > self.b.len() * 8 {
            return None;
        }
        let mut v = 0u32;
        for _ in 0..k {
            v = (v << 1) | ((self.b[self.pos >> 3] >> (7 - (self.pos & 7))) & 1) as u32;
            self.pos += 1;
        }
        Some(v)
    }
    fn align(&mut self) {
        self.pos = (self.pos + 7) / 8 * 8;
    }
}

/// MSB-first bit writer.
#[derive(Default)]
struct BitW {
    bits: Vec<u8>,
}
impl BitW {
    fn put(&mut self, v: u32, k: usize) {
        for i in (0..k).rev() {
            self.bits.push(((v >> i) & 1) as u8);
        }
    }
    fn put_bits(&mut self, seq: &[u8]) {
        self.bits.extend_from_slice(seq);
    }
    fn bytes(mut self) -> Vec<u8> {
        while self.bits.len() % 8 != 0 {
            self.bits.push(0);
        }
        self.bits.chunks_exact(8).map(|c| c.iter().fold(0u8, |a, b| (a << 1) | b)).collect()
    }
}

/// One channel's side info of a granule: `part2_3_length` and the remaining side bits as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GranuleChannel {
    pub part2_3_length: u32,
    pub side_bits: Vec<u8>,
}

/// The PCM frame carried by the flagged granule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmFrame {
    pub offset_samples: u16,
    pub pcm_samples: u16,
    pub unknown_u32: u32,
    /// Interleaved s16 (big-endian in the stream) as i16.
    pub samples: Vec<i16>,
}

/// One EA granule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Granule {
    pub at: usize,
    pub flagged: bool,
    pub version: u8,
    pub sr_index: u8,
    pub channel_mode: u8,
    pub mode_ext: u8,
    pub granule_index: u8,
    pub scfsi: Vec<u8>,
    pub channels: Vec<GranuleChannel>,
    /// Main data bits, MSB-first, channel 0 then channel 1 (exactly Σ part2_3_length bits).
    pub main_bits: Vec<u8>,
    pub pcm: Option<PcmFrame>,
    pub end: usize,
}

impl Granule {
    pub fn mpeg1(&self) -> bool {
        self.version == 3
    }
    pub fn nch(&self) -> usize {
        if self.channel_mode == 3 { 1 } else { 2 }
    }
    pub fn sample_rate(&self) -> Option<u32> {
        let t = if self.mpeg1() { &RATES_MPEG1 } else { &RATES_MPEG2 };
        t.get(self.sr_index as usize).copied()
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum EaLayer3Refusal {
    #[error("block {block}: trailing {bytes} bytes")]
    Trailing { block: usize, bytes: usize },
    #[error("block {block}: pcm_flag {flag:02x} at {at}")]
    Flag { block: usize, flag: u8, at: usize },
    #[error("block {block}: EOF inside granule at {at}")]
    Eof { block: usize, at: usize },
    #[error("block {block}: main data {bits} bits at {at} exceeds payload")]
    MainData { block: usize, bits: usize, at: usize },
    #[error("block {block}: PCM block {samples} samples at {at} exceeds payload")]
    Pcm { block: usize, samples: u16, at: usize },
    #[error("block {block}: version {version} / sample-rate index {sr_index} is reserved")]
    Header { block: usize, version: u8, sr_index: u8 },
    #[error("block {block} granule {granule}: header class {found:02x} (version / rate / channel mode) differs from the sample's {expected:02x}")]
    HeaderClass { block: usize, granule: usize, found: u8, expected: u8 },
    #[error("block {block}: {granules} granules for {declared} declared samples (expected {expected})")]
    GranuleCount { block: usize, granules: usize, declared: u32, expected: usize },
    #[error("PCM frame: {found} flagged frame(s); the contract has exactly one, the second EA-frame of block 0 with 47 samples (found at block {block:?} frame {frame:?}, {samples:?} samples)")]
    PcmFrame { found: usize, block: Option<usize>, frame: Option<usize>, samples: Option<u16> },
    #[error("MPEG-1 granule indices do not alternate 0, 1 in block {block}")]
    Alternation { block: usize },
    #[error("no granules")]
    Empty,
    #[error("rebuild: frame {frame}: main data does not fit the reservoir ({over} bytes beyond the frame)")]
    Reservoir { frame: usize, over: usize },
    #[error("SNR header: codec 0x{codec:02x} ({label}), {rate} Hz, channel config {channels} — not EALayer3 v1 (0x05)")]
    NotEaLayer3 { codec: u8, label: &'static str, rate: u32, channels: u8 },
}

/// Walk one block payload as EA frames (R77 `parse_block`).
pub fn parse_block(pl: &[u8], block: usize) -> Result<Vec<Granule>, EaLayer3Refusal> {
    let mut p = 0usize;
    let mut out = Vec::new();
    while p < pl.len() {
        if p + 2 > pl.len() {
            return Err(EaLayer3Refusal::Trailing { block, bytes: pl.len() - p });
        }
        let flag = pl[p];
        let common = pl[p + 1];
        if flag != 0x00 && flag != 0xEE {
            return Err(EaLayer3Refusal::Flag { block, flag, at: p });
        }
        let version = common >> 6;
        let sr_index = (common >> 4) & 3;
        let channel_mode = (common >> 2) & 3;
        let mode_ext = common & 3;
        if (version != 3 && version != 2) || sr_index == 3 {
            return Err(EaLayer3Refusal::Header { block, version, sr_index });
        }
        let mpeg1 = version == 3;
        let nch = if channel_mode == 3 { 1 } else { 2 };
        let mut bs = Bits::new(pl, (p + 2) * 8);
        let eof = EaLayer3Refusal::Eof { block, at: p };
        let granule_index = bs.get(1).ok_or(eof.clone())? as u8;
        let mut scfsi = Vec::with_capacity(nch);
        for _ in 0..nch {
            scfsi.push(if mpeg1 && granule_index == 1 { bs.get(4).ok_or(eof.clone())? as u8 } else { 0 });
        }
        let mut channels = Vec::with_capacity(nch);
        for _ in 0..nch {
            let size = bs.get(12).ok_or(eof.clone())?;
            let n = if mpeg1 { 47 } else { 51 };
            let mut side = Vec::with_capacity(n);
            for _ in 0..n {
                side.push(bs.get(1).ok_or(eof.clone())? as u8);
            }
            channels.push(GranuleChannel { part2_3_length: size, side_bits: side });
        }
        let start = bs.pos;
        let total: usize = channels.iter().map(|c| c.part2_3_length as usize).sum();
        if start + total > pl.len() * 8 {
            return Err(EaLayer3Refusal::MainData { block, bits: total, at: start });
        }
        let main_bits: Vec<u8> = (0..total).map(|i| (pl[(start + i) >> 3] >> (7 - ((start + i) & 7))) & 1).collect();
        bs.pos = start + total;
        bs.align();
        let mut pcm = None;
        if flag == 0xEE {
            let offset_samples = bs.get(16).ok_or(eof.clone())? as u16;
            let pcm_samples = bs.get(16).ok_or(eof.clone())? as u16;
            let unknown_u32 = bs.get(32).ok_or(eof.clone())?;
            let data_at = bs.pos / 8;
            let bytes = pcm_samples as usize * nch * 2;
            if data_at + bytes > pl.len() {
                return Err(EaLayer3Refusal::Pcm { block, samples: pcm_samples, at: data_at });
            }
            let samples = pl[data_at..data_at + bytes].chunks_exact(2).map(|c| i16::from_be_bytes([c[0], c[1]])).collect();
            bs.pos += bytes * 8;
            pcm = Some(PcmFrame { offset_samples, pcm_samples, unknown_u32, samples });
        }
        let end = bs.pos / 8;
        out.push(Granule { at: p, flagged: flag == 0xEE, version, sr_index, channel_mode, mode_ext, granule_index, scfsi, channels, main_bits, pcm, end });
        p = end;
    }
    Ok(out)
}

/// The contract-parsed sample: every granule of every block in order, the PCM frame, and counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEaLayer3 {
    pub granules: Vec<Granule>,
    pub granules_per_block: Vec<usize>,
    pub mpeg1: bool,
    pub sample_rate: u32,
    pub channels: usize,
    pub pcm: PcmFrame,
    /// `granules × 576 + 47 − num_samples` (R77: 1 152 + the last block's trim).
    pub surplus_samples: i64,
    pub num_samples: u32,
}

/// The R77 §1 / R80 §2 contract over the data blocks `(declared samples, payload)` of one sample.
pub fn contract_parse(blocks: &[(u32, &[u8])], num_samples: u32) -> Result<ParsedEaLayer3, EaLayer3Refusal> {
    let mut all: Vec<Granule> = Vec::new();
    let mut per_block = Vec::with_capacity(blocks.len());
    let mut class: Option<(u8, bool, usize, u32)> = None; // (header byte, mpeg1, nch, rate)
    let mut pcm_frames: Vec<(usize, usize, u16)> = Vec::new();
    let mut pcm: Option<PcmFrame> = None;
    for (bi, (declared, pl)) in blocks.iter().enumerate() {
        let gr = parse_block(pl, bi)?;
        if gr.is_empty() {
            return Err(EaLayer3Refusal::Empty);
        }
        if gr.last().map(|g| g.end) != Some(pl.len()) {
            return Err(EaLayer3Refusal::Trailing { block: bi, bytes: pl.len() - gr.last().unwrap().end });
        }
        for (j, g) in gr.iter().enumerate() {
            // The rate class is version + rate index + channel mode; `mode_extension` varies per
            // granule on joint-stereo music (R80: carried as read), so it is not part of the class.
            let hdr = (g.version << 6) | (g.sr_index << 4) | (g.channel_mode << 2);
            let rate = g.sample_rate().ok_or(EaLayer3Refusal::Header { block: bi, version: g.version, sr_index: g.sr_index })?;
            match class {
                None => class = Some((hdr, g.mpeg1(), g.nch(), rate)),
                Some((h0, ..)) if h0 != hdr => return Err(EaLayer3Refusal::HeaderClass { block: bi, granule: j, found: hdr, expected: h0 }),
                _ => {}
            }
            if let Some(p) = &g.pcm {
                pcm_frames.push((bi, j, p.pcm_samples));
                if pcm.is_none() {
                    pcm = Some(p.clone());
                }
            }
        }
        let (_, mpeg1, ..) = class.unwrap();
        if mpeg1 && gr.iter().enumerate().any(|(i, g)| g.granule_index as usize != i % 2) {
            return Err(EaLayer3Refusal::Alternation { block: bi });
        }
        let pcm_in_block: u32 = gr.iter().filter_map(|g| g.pcm.as_ref()).map(|p| p.pcm_samples as u32).sum();
        let body = declared.saturating_sub(pcm_in_block);
        if bi + 1 < blocks.len() {
            // Interior block: granules = declared / 576 (+ 2 warm-up granules on block 0).
            let mut expected = (body / 576) as usize;
            if body % 576 != 0 {
                return Err(EaLayer3Refusal::GranuleCount { block: bi, granules: gr.len(), declared: *declared, expected });
            }
            if bi == 0 {
                expected += 2;
            }
            if gr.len() != expected {
                return Err(EaLayer3Refusal::GranuleCount { block: bi, granules: gr.len(), declared: *declared, expected });
            }
        } else {
            // Last block: granules × 576 ≥ declared (the declared count trims the tail).
            let need = body.div_ceil(576) as usize + if bi == 0 { 2 } else { 0 };
            if gr.len() * 576 < body as usize {
                return Err(EaLayer3Refusal::GranuleCount { block: bi, granules: gr.len(), declared: *declared, expected: need });
            }
        }
        per_block.push(gr.len());
        all.extend(gr);
    }
    let Some((_, mpeg1, nch, rate)) = class else { return Err(EaLayer3Refusal::Empty) };
    match pcm_frames.as_slice() {
        [(0, 1, 47)] => {}
        [] => return Err(EaLayer3Refusal::PcmFrame { found: 0, block: None, frame: None, samples: None }),
        [(b, f, n), ..] => return Err(EaLayer3Refusal::PcmFrame { found: pcm_frames.len(), block: Some(*b), frame: Some(*f), samples: Some(*n) }),
    }
    let pcm = pcm.unwrap();
    let surplus = all.len() as i64 * 576 + 47 - num_samples as i64;
    Ok(ParsedEaLayer3 { granules: all, granules_per_block: per_block, mpeg1, sample_rate: rate, channels: nch, pcm, surplus_samples: surplus, num_samples })
}

/// The SNR gate: codec `0x05`.
pub fn check_header(h: &crate::sns::SnrHeader) -> Result<(), EaLayer3Refusal> {
    if h.codec != 0x05 {
        return Err(EaLayer3Refusal::NotEaLayer3 { codec: h.codec, label: h.codec_label(), rate: h.sample_rate, channels: h.channel_config });
    }
    Ok(())
}

/// A rebuilt standard Layer III stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rebuilt {
    pub mpeg: Vec<u8>,
    pub frames: usize,
    pub max_main_data_begin: u32,
    /// The last frame was padded with a silent granule (odd MPEG-1 granule count).
    pub padded: bool,
    pub frame_bytes: usize,
}

/// R77 §2: standard frames at bitrate index 14 (320 / 160 kbps), padding 0, main data placed
/// through the standard bit reservoir. Two granules per MPEG-1 frame, one per MPEG-2 frame.
pub fn rebuild(granules: &[Granule]) -> Result<Rebuilt, EaLayer3Refusal> {
    let Some(g0) = granules.first() else { return Err(EaLayer3Refusal::Empty) };
    let mpeg1 = g0.mpeg1();
    let rate = g0.sample_rate().ok_or(EaLayer3Refusal::Header { block: 0, version: g0.version, sr_index: g0.sr_index })?;
    let nch = g0.nch();
    let per = if mpeg1 { 2 } else { 1 };
    let frame_bytes = if mpeg1 { (144 * 320_000 / rate) as usize } else { (72 * 160_000 / rate) as usize };
    let side_bytes = match (mpeg1, nch) {
        (true, 1) => 17,
        (true, _) => 32,
        (false, 1) => 9,
        (false, _) => 17,
    };
    let area = frame_bytes - 4 - side_bytes;
    let maxmdb = if mpeg1 { 511usize } else { 255 };
    let mut groups: Vec<Vec<Option<&Granule>>> = granules.chunks(per).map(|c| c.iter().map(Some).collect()).collect();
    let mut padded = false;
    if let Some(last) = groups.last_mut() {
        if last.len() < per {
            last.push(None);
            padded = true;
        }
    }
    // Main-data stream with the reservoir placement.
    let n_frames = groups.len();
    let mut stream = vec![0u8; n_frames * area];
    let mut pos_bits = 0usize;
    let mut placements = Vec::with_capacity(n_frames);
    for (f, grp) in groups.iter().enumerate() {
        let a = f * area;
        let start = ((pos_bits + 7) / 8).max(a.saturating_sub(maxmdb));
        if start > a {
            return Err(EaLayer3Refusal::Reservoir { frame: f, over: start - a });
        }
        let mut bits: Vec<u8> = Vec::new();
        for g in grp.iter().flatten() {
            bits.extend_from_slice(&g.main_bits);
        }
        let need_bytes = (start * 8 + bits.len()).div_ceil(8);
        if need_bytes > stream.len() {
            stream.resize(need_bytes, 0);
        }
        for (i, bit) in bits.iter().enumerate() {
            if *bit == 1 {
                let k = start * 8 + i;
                stream[k >> 3] |= 0x80 >> (k & 7);
            }
        }
        placements.push(start);
        pos_bits = start * 8 + bits.len();
    }
    let sr_idx = if mpeg1 { RATES_MPEG1 } else { RATES_MPEG2 }.iter().position(|r| *r == rate).unwrap() as u8;
    let mut out = Vec::with_capacity(n_frames * frame_bytes);
    let mut max_mdb = 0u32;
    for (f, grp) in groups.iter().enumerate() {
        let a = f * area;
        let mdb = (a - placements[f]) as u32;
        max_mdb = max_mdb.max(mdb);
        let first = grp.iter().flatten().next().unwrap();
        let hdr = [
            0xFF,
            0xF0 | if mpeg1 { 0x08 } else { 0x00 } | 0x02 | 0x01,
            (14 << 4) | (sr_idx << 2),
            (first.channel_mode << 6) | (first.mode_ext << 4),
        ];
        let mut w = BitW::default();
        let g1 = grp.iter().flatten().find(|g| g.granule_index == 1);
        if mpeg1 {
            w.put(mdb, 9);
            w.put(0, if nch == 1 { 5 } else { 3 });
            for ch in 0..nch {
                w.put(g1.map(|g| g.scfsi[ch] as u32).unwrap_or(0), 4);
            }
        } else {
            w.put(mdb, 8);
            w.put(0, if nch == 1 { 1 } else { 2 });
        }
        for g in grp {
            for ch in 0..nch {
                match g {
                    None => {
                        w.put(0, 12);
                        w.put_bits(&vec![0u8; if mpeg1 { 47 } else { 51 }]);
                    }
                    Some(g) => {
                        let c = &g.channels[ch];
                        w.put(c.part2_3_length, 12);
                        w.put_bits(&c.side_bits);
                    }
                }
            }
        }
        let side = w.bytes();
        debug_assert_eq!(side.len(), side_bytes);
        out.extend_from_slice(&hdr);
        out.extend_from_slice(&side);
        let end = (a + area).min(stream.len());
        out.extend_from_slice(&stream[a..end]);
        out.resize(out.len() + (a + area - end), 0);
    }
    Ok(Rebuilt { mpeg: out, frames: n_frames, max_main_data_begin: max_mdb, padded, frame_bytes })
}

/// The label every gen7 preview must carry (R77 §4 / R80 §3).
pub const LABEL: &str = "diagnostic preview — start handling unresolved (two warm-up granules + 47 PCM samples; not game-verified)";

/// A measured tuple of the R77 / R80 surveys: the caller decides coverage from these; anything
/// outside is refused, never guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tuple {
    pub platform: &'static str,
    pub game: &'static str,
    pub bank: &'static str,
    pub sample_rate: u32,
    pub channels: usize,
    pub source: &'static str,
}

/// R77 §1 (six pbp sets) + R80 §2 (`ann_eng` 24 kHz on PS3 09–Legacy; `pbp_eng` 24 kHz on PS3
/// 09 / 10 / 12; `music_samples` 44.1 kHz stereo on X360 13–Legacy and PS3 11 / 14 / 15 / Legacy) + R82
/// (`music_samples` on the archive-backed PS3 09 / 10 / 12 / 13).
/// PS3 NHL 14 `pbp_eng` is codec `0x09` (not this codec); X360 announcer / pbp 11–14 are EA-XMA.
/// `.abk` raw streams (S10A entries) are **not** listed: their container is not read here.
pub const MEASURED: &[Tuple] = &[
    Tuple { platform: "xbox", game: "nhl15", bank: "pbp_eng", sample_rate: 44100, channels: 1, source: "R77" },
    Tuple { platform: "xbox", game: "nhllegacy", bank: "pbp_eng", sample_rate: 44100, channels: 1, source: "R77" },
    Tuple { platform: "ps3", game: "nhl15", bank: "pbp_eng", sample_rate: 44100, channels: 1, source: "R77" },
    Tuple { platform: "ps3", game: "nhllegacy", bank: "pbp_eng", sample_rate: 44100, channels: 1, source: "R77" },
    Tuple { platform: "ps3", game: "nhl11", bank: "pbp_eng", sample_rate: 24000, channels: 1, source: "R77" },
    Tuple { platform: "ps3", game: "nhl13", bank: "pbp_eng", sample_rate: 24000, channels: 1, source: "R77" },
    Tuple { platform: "ps3", game: "nhl09", bank: "pbp_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhl10", bank: "pbp_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhl12", bank: "pbp_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhl09", bank: "ann_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhl10", bank: "ann_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhl11", bank: "ann_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhl12", bank: "ann_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhl13", bank: "ann_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhl14", bank: "ann_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhl15", bank: "ann_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "ps3", game: "nhllegacy", bank: "ann_eng", sample_rate: 24000, channels: 1, source: "R80" },
    Tuple { platform: "xbox", game: "nhl13", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R80" },
    Tuple { platform: "xbox", game: "nhl14", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R80" },
    Tuple { platform: "xbox", game: "nhl15", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R80" },
    Tuple { platform: "xbox", game: "nhllegacy", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R80" },
    Tuple { platform: "ps3", game: "nhl11", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R80" },
    Tuple { platform: "ps3", game: "nhl14", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R80" },
    Tuple { platform: "ps3", game: "nhl15", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R80" },
    Tuple { platform: "ps3", game: "nhllegacy", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R80" },
    // R82 (2026-09-24 handoff 01 §3; research handoff 19 A1): the archive-backed PS3 NHL 09 / 10 / 12 / 13
    // music banks — `.sbr` in `audioboot.big` (RefPack on 09 / 10, chunkzip on 12 / 13), `.sbs` in
    // `data0.big` (09), loose (10), `nocache.big` (12 / 13); three vectors each (receipt 36).
    Tuple { platform: "ps3", game: "nhl09", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R82" },
    Tuple { platform: "ps3", game: "nhl10", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R82" },
    Tuple { platform: "ps3", game: "nhl12", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R82" },
    Tuple { platform: "ps3", game: "nhl13", bank: "music_samples", sample_rate: 44100, channels: 2, source: "R82" },
];

/// The measured tuple for `(platform, game, bank stem)`, if any.
pub fn measured(platform: &str, game: &str, bank: &str) -> Option<&'static Tuple> {
    MEASURED.iter().find(|t| t.platform == platform && t.game == game && t.bank == bank)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic granule: header byte `cc` (MPEG-1 44.1k mono) / `9c` (MPEG-2 24k mono), given
    /// granule index, `size` main-data bits of a pattern, optional 47-sample PCM frame.
    fn granule_bytes(mpeg1: bool, gi: u8, size: u32, pcm: Option<&[i16]>) -> Vec<u8> {
        let mut w = BitW::default();
        w.put(gi as u32, 1);
        if mpeg1 && gi == 1 {
            w.put(0b1010, 4);
        }
        w.put(size, 12);
        for i in 0..(if mpeg1 { 47 } else { 51 }) {
            w.put((i % 3 == 0) as u32, 1);
        }
        for i in 0..size {
            w.put((i % 5 == 0) as u32, 1);
        }
        let mut out = vec![if pcm.is_some() { 0xEE } else { 0x00 }, if mpeg1 { 0xcc } else { 0x9c }];
        out.extend(w.bytes());
        if let Some(p) = pcm {
            out.extend_from_slice(&0u16.to_be_bytes());
            out.extend_from_slice(&(p.len() as u16).to_be_bytes());
            out.extend_from_slice(&0u32.to_be_bytes());
            for s in p {
                out.extend_from_slice(&s.to_be_bytes());
            }
        }
        out
    }

    /// Block 0 with 2 extra granules + the PCM frame as the second EA-frame, then an interior block,
    /// then a short last block.
    fn sample(mpeg1: bool) -> (Vec<Vec<u8>>, Vec<u32>, u32) {
        let pcm: Vec<i16> = (0..47).map(|i| i as i16 * 10).collect();
        let g = |i: usize, size: u32, p: Option<&[i16]>| granule_bytes(mpeg1, if mpeg1 { (i % 2) as u8 } else { 0 }, size, p);
        // block 0: declares 4 × 576 + 47 = 2351 → 4 + 2 = 6 granules
        let mut b0 = Vec::new();
        for i in 0..6 {
            b0.extend(g(i, 200 + i as u32 * 7, if i == 1 { Some(&pcm) } else { None }));
        }
        // interior: 4 granules = 2304
        let mut b1 = Vec::new();
        for i in 0..4 {
            b1.extend(g(i, 300, None));
        }
        // last: 2 granules for 1000 declared (2 × 576 ≥ 1000)
        let mut b2 = Vec::new();
        for i in 0..2 {
            b2.extend(g(i, 100, None));
        }
        (vec![b0, b1, b2], vec![2351, 2304, 1000], 2351 + 2304 + 1000)
    }

    #[test]
    fn parses_both_rate_classes_and_rebuilds_standard_frames() {
        for mpeg1 in [true, false] {
            let (blocks, samples, n) = sample(mpeg1);
            let refs: Vec<(u32, &[u8])> = samples.iter().copied().zip(blocks.iter().map(|b| b.as_slice())).collect();
            let p = contract_parse(&refs, n).unwrap();
            assert_eq!(p.granules_per_block, vec![6, 4, 2]);
            assert_eq!((p.mpeg1, p.sample_rate, p.channels), (mpeg1, if mpeg1 { 44100 } else { 24000 }, 1));
            assert_eq!(p.pcm.pcm_samples, 47);
            assert_eq!(p.pcm.samples[3], 30);
            assert_eq!(p.surplus_samples, 12 * 576 + 47 - n as i64);
            let r = rebuild(&p.granules).unwrap();
            let per = if mpeg1 { 2 } else { 1 };
            assert_eq!(r.frames, 12 / per);
            assert_eq!(r.mpeg.len(), r.frames * r.frame_bytes);
            assert!(!r.padded);
            // Every frame starts with a valid standard header of the right class.
            for f in 0..r.frames {
                let h = &r.mpeg[f * r.frame_bytes..f * r.frame_bytes + 4];
                assert_eq!(h[0], 0xFF);
                assert_eq!(h[1], if mpeg1 { 0xFB } else { 0xF3 });
                assert_eq!(h[2] >> 4, 14);
            }
            // Frame 0's main_data_begin is 0; the side info's first part2_3_length is granule 0's.
            let side = &r.mpeg[4..];
            let mut bs = Bits::new(side, 0);
            assert_eq!(bs.get(if mpeg1 { 9 } else { 8 }).unwrap(), 0);
            bs.get(if mpeg1 { 5 } else { 1 }).unwrap();
            if mpeg1 {
                assert_eq!(bs.get(4).unwrap(), 0b1010, "scfsi of granule 1 carried");
            }
            assert_eq!(bs.get(12).unwrap(), 200, "granule 0 part2_3_length");
            // An odd MPEG-1 granule count pads the last frame.
            if mpeg1 {
                let r2 = rebuild(&p.granules[..11]).unwrap();
                assert!(r2.padded && r2.frames == 6);
            }
        }
    }

    #[test]
    fn refuses_the_r80_mutation_classes_and_the_r77_rules_by_name() {
        let (blocks, samples, n) = sample(true);
        let run = |blocks: &[Vec<u8>], samples: &[u32], n: u32| {
            let refs: Vec<(u32, &[u8])> = samples.iter().copied().zip(blocks.iter().map(|b| b.as_slice())).collect();
            contract_parse(&refs, n)
        };
        // truncated last block by 3 bytes → EOF inside granule / main data exceeds
        let mut t = blocks.clone();
        let l = t[2].len();
        t[2].truncate(l - 3);
        assert!(matches!(run(&t, &samples, n), Err(EaLayer3Refusal::MainData { block: 2, .. }) | Err(EaLayer3Refusal::Eof { block: 2, .. }) | Err(EaLayer3Refusal::Trailing { block: 2, .. })), "{:?}", run(&t, &samples, n));
        // flag byte 0x01
        let mut t = blocks.clone();
        t[1][0] = 0x01;
        assert_eq!(run(&t, &samples, n), Err(EaLayer3Refusal::Flag { block: 1, flag: 1, at: 0 }));
        // PCM count overwritten with 4096: the PCM block exceeds the payload
        let mut t = blocks.clone();
        let g0 = granule_bytes(true, 0, 200, None).len();
        let g1_hdr_bits = 2 * 8 + 1 + 4 + 12 + 47 + 207;
        let pcm_fields_at = g0 + (g1_hdr_bits + 7) / 8;
        t[0][pcm_fields_at + 2..pcm_fields_at + 4].copy_from_slice(&4096u16.to_be_bytes());
        assert!(matches!(run(&t, &samples, n), Err(EaLayer3Refusal::Pcm { block: 0, samples: 4096, .. })), "{:?}", run(&t, &samples, n));
        // header class differs mid-sample
        let mut t = blocks.clone();
        t[1][1] = 0x9c;
        // (a class change also changes the side-info width, so the walk may fail on tiling first —
        // either way block 1 is refused by name)
        let e = run(&t, &samples, n).unwrap_err();
        assert!(e.to_string().starts_with("block 1"), "{e}");
        // granule count: interior block with a wrong declared count
        let mut s = samples.clone();
        s[1] = 2304 - 576;
        assert!(matches!(run(&blocks, &s, n - 576), Err(EaLayer3Refusal::GranuleCount { block: 1, granules: 4, expected: 3, .. })));
        // alternation broken
        let mut t = blocks.clone();
        t[1] = {
            let mut b = Vec::new();
            for _ in 0..4 {
                b.extend(granule_bytes(true, 0, 300, None));
            }
            b
        };
        assert_eq!(run(&t, &samples, n), Err(EaLayer3Refusal::Alternation { block: 1 }));
        // PCM frame missing (block 0 without the EE frame → also a granule-count change, so compare the PCM rule on a full sample with the flag moved)
        let mut t = blocks.clone();
        t[0] = {
            let pcm: Vec<i16> = vec![0; 47];
            let mut b = Vec::new();
            for i in 0..6 {
                b.extend(granule_bytes(true, (i % 2) as u8, 200 + i as u32 * 7, if i == 2 { Some(&pcm) } else { None }));
            }
            b
        };
        assert!(matches!(run(&t, &samples, n), Err(EaLayer3Refusal::PcmFrame { found: 1, block: Some(0), frame: Some(2), samples: Some(47) })));
        // header gate
        let h = crate::sns::SnrHeader { version: 1, codec: 0x09, channel_config: 0, sample_rate: 32000, stream_type: 1, looped: false, num_samples: 1 };
        assert!(matches!(check_header(&h), Err(EaLayer3Refusal::NotEaLayer3 { codec: 0x09, .. })));
        assert!(measured("ps3", "nhl14", "pbp_eng").is_none(), "PS3 14 pbp is codec 0x09");
        assert_eq!(measured("xbox", "nhl15", "pbp_eng").unwrap().sample_rate, 44100);
        assert_eq!(measured("ps3", "nhl09", "ann_eng").unwrap().sample_rate, 24000);
    }
}
