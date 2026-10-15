//! EA-XMA (SNR codec `0x03`, Xbox 360) — the refusing packet extraction and the RIFF XMA2 wrapper
//! for a **diagnostic** decode through an external XMA2 helper (research R84; the receipt-27 review's
//! decision: an optional, separately invoked minimal FFmpeg helper). No decoder lives here.
//!
//! ```text
//! SNS block payload : { u32 w0 (w0 >> 2 = sub-block length incl. w0) | one XMA2 packet } per stream
//!                     (streams = (channels + 1) / 2); packet < 0x800 bytes → zero-padded to 0x800
//! raw ram segment   : u32 w0 (w0 >> 2 = segment length) | consecutive 0x800 packets, last padded
//! packet header     : u32 BE = frame count (6) | first-frame bit offset (15) | metadata (3) | skip (8)
//! RIFF              : WAVEFORMATEX tag 0x0166 + the documented XMA2WAVEFORMATEX extension
//!                     (cbSize 34: streams, channel mask, samples encoded, bytes per block 0x10000,
//!                     play / loop 0, encoder version 4, block count), then `data`
//! ```
//!
//! Refusals (R84 §B.4): a sub-block / segment whose `w0 >> 2` disagrees with its length, a packet
//! header with frame count 0 or a first-frame offset past the packet, trailing bytes after the last
//! stream of a mono block. The caller refuses a decode whose sample count differs from the declared
//! count. The X360 NHL 09 packet-header variant (constant header words, R98b / R100) is read with
//! [`HeaderRule::Nhl09Variant`] (research R131, receipt 36): the header word is not a frame count
//! there, so it is not checked, a sub-block may hold several 0x800 packets, and the caller's count
//! rule (decoded = declared) is the only refusal — R131 found the NHL 10+ header check would reject
//! 47 NHL 09 records that decode exactly.

use crate::sns::{Block, SnrHeader};

/// The SNS-blocked EA-XMA sample-bank tuples research R84 measured on the Xbox 360 discs (loose
/// `.sbr` / `.sbs`): announcer and play-by-play on NHL 09 / 10 (24 kHz mono) and the music bank on
/// NHL 09–12 (44.1 kHz stereo). Every R84 vector of each family decodes through Studio's RIFF +
/// helper route to exactly its declared count with PCM equal to the research decode
/// (`tests/ea_xma_real_vectors.rs`, receipts 30 / 32). A `static` so a tuple can be told apart by
/// address from the EALayer3 / EA-Speex tables that share the `Tuple` type. X360 NHL 11 → Legacy
/// speech is EA-Speex (R83) and is not listed here; Legacy Recompiled has no measured vector.
pub static MEASURED: &[crate::ealayer3::Tuple] = &[
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl09", bank: "ann_eng", sample_rate: 24_000, channels: 1, source: "R84" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl09", bank: "pbp_eng", sample_rate: 24_000, channels: 1, source: "R84" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl10", bank: "ann_eng", sample_rate: 24_000, channels: 1, source: "R84" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl10", bank: "pbp_eng", sample_rate: 24_000, channels: 1, source: "R84" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl09", bank: "music_samples", sample_rate: 44_100, channels: 2, source: "R84" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl10", bank: "music_samples", sample_rate: 44_100, channels: 2, source: "R84" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl11", bank: "music_samples", sample_rate: 44_100, channels: 2, source: "R84" },
    crate::ealayer3::Tuple { platform: "xbox", game: "nhl12", bank: "music_samples", sample_rate: 44_100, channels: 2, source: "R84" },
];

/// The admitted SNS EA-XMA tuple for `(platform, game, bank stem)`, if any.
pub fn measured(platform: &str, game: &str, bank: &str) -> Option<&'static crate::ealayer3::Tuple> {
    MEASURED.iter().find(|t| t.platform == platform && t.game == game && t.bank == bank)
}

pub const CODEC: u8 = 0x03;
pub const PACKET: usize = 0x800;
pub const FRAME_SAMPLES: u32 = 512;

/// The fixed preview label.
pub const LABEL: &str = "diagnostic preview — not game-verified (EA-XMA through the external XMA2 helper; the decoder's sample count is checked against the declared count, which is not a listening result)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmaRefusal {
    NotEaXma { codec: u8 },
    UnsupportedChannels(u32),
    SubBlockLength { block: usize, stream: usize, w0_len: usize, available: usize },
    TrailingBytes { block: usize, extra: usize },
    PacketHeader { at: String, frame_count: u32, offset_bits: u32, bytes: usize },
    Oversize { block: usize, bytes: usize },
    SegmentLength { w0_len: usize, segment: usize },
    Empty,
}

impl std::fmt::Display for XmaRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use XmaRefusal::*;
        match self {
            NotEaXma { codec } => write!(f, "not an EA-XMA stream (codec 0x{codec:02x}; EA-XMA is 0x03)"),
            UnsupportedChannels(c) => write!(f, "{c} channels: only mono / stereo (one XMA stream) are measured (R84)"),
            SubBlockLength { block, stream, w0_len, available } => write!(f, "block {block}, stream {stream}: w0 >> 2 = {w0_len} but {available} bytes remain"),
            TrailingBytes { block, extra } => write!(f, "block {block}: {extra} bytes after the stream"),
            PacketHeader { at, frame_count, offset_bits, bytes } => write!(f, "{at}: packet header frame count {frame_count}, first-frame offset {offset_bits} bits in a {bytes}-byte packet"),
            Oversize { block, bytes } => write!(f, "block {block}: a {bytes}-byte packet is larger than 0x800"),
            SegmentLength { w0_len, segment } => write!(f, "w0 >> 2 = {w0_len} but the segment is {segment} bytes"),
            Empty => write!(f, "no packets"),
        }
    }
}

impl std::error::Error for XmaRefusal {}

/// The "no frame begins in this packet" header (frame count 0, first-frame offset `0x7FFF`) — accepted only under
/// [`HeaderRule::CheckedNoFrameMarker`] (research 2026-09-28 handoff 05 §4, R139c).
pub const NO_FRAME_OFFSET: u32 = 0x7FFF;

fn check_packet(at: String, pk: &[u8]) -> Result<u32, XmaRefusal> {
    check_packet_rule(at, pk, false)
}

fn check_packet_rule(at: String, pk: &[u8], allow_no_frame: bool) -> Result<u32, XmaRefusal> {
    if pk.len() < 4 {
        return Err(XmaRefusal::PacketHeader { at, frame_count: 0, offset_bits: 0, bytes: pk.len() });
    }
    let w = u32::from_be_bytes([pk[0], pk[1], pk[2], pk[3]]);
    let (fc, off) = (w >> 26, (w >> 11) & 0x7fff);
    if allow_no_frame && fc == 0 && off == NO_FRAME_OFFSET {
        return Ok(0);
    }
    if fc == 0 || off as usize >= pk.len() * 8 {
        return Err(XmaRefusal::PacketHeader { at, frame_count: fc, offset_bits: off, bytes: pk.len() });
    }
    Ok(fc)
}

/// How packet headers are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderRule {
    /// NHL 10 → Legacy (R84 / R91): frame count (top 6 bits) > 0 and first-frame offset inside
    /// the packet, else refused; one packet per sub-block.
    Checked,
    /// X360 NHL 10 → Legacy **ram records** (research 2026-09-28 handoff 05, X1): [`HeaderRule::Checked`], except
    /// that a packet with frame count 0 **and** first-frame offset `0x7FFF` ("no frame begins here") is kept (0
    /// frames). Every other invalid header still refuses — including NHL 10's one (2, 16 384) sub-stream. The
    /// caller's count rule (decoded = declared) stays the admission test.
    CheckedNoFrameMarker,
    /// X360 NHL 09 (R98b / R100 / R131): constant header words, not a frame count — not checked;
    /// a sub-block may hold several packets (split at 0x800). `frame_count_sum` is then 0 (unknown).
    Nhl09Variant,
}

/// Packets extracted for a decoder, with the counts a caller reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmaPackets {
    pub packets: Vec<u8>,
    pub packet_count: usize,
    pub frame_count_sum: u32,
}

/// SNS-blocked EA-XMA (speech / music): strip each block's `w0`, check it, pad the packet to 0x800.
pub fn sns_packets(header: &SnrHeader, blocks: &[Block]) -> Result<XmaPackets, XmaRefusal> {
    if header.codec != CODEC {
        return Err(XmaRefusal::NotEaXma { codec: header.codec });
    }
    let ch = header.channels();
    if !(1..=2).contains(&ch) {
        return Err(XmaRefusal::UnsupportedChannels(ch));
    }
    let payloads: Vec<&[u8]> = blocks.iter().map(|b| b.payload.as_slice()).collect();
    block_packets(&payloads, ch.div_ceil(2) as usize, None, false, HeaderRule::Checked)
}

/// The packets of block payloads that each hold `streams` sub-blocks `{ u32 w0 | one XMA2 packet }`
/// (SNS speech / music, and `.ast` stream records — R91 / R92: one sub-block per XMA stream, 2 for
/// channel config 3, 3 for 5). `pick = None` keeps every stream's packet (a single-stream record);
/// `Some(k)` keeps only stream `k` — one stereo pair of a multichannel record, a **diagnostic** pair
/// selection (the other streams' sub-blocks are still length-checked). Every block must be consumed
/// exactly, except that with `terminal_zero_pad` (Xbox 360 `.ast` streams, R91 §1: the terminal
/// block's size is a multiple of 0x40 and its payload ends in zero padding) the **last** block may
/// end in zero bytes; a nonzero trailing byte, or padding in any other block, is still refused.
pub fn block_packets(payloads: &[&[u8]], streams: usize, pick: Option<usize>, terminal_zero_pad: bool, rule: HeaderRule) -> Result<XmaPackets, XmaRefusal> {
    if streams == 0 || pick.is_some_and(|k| k >= streams) {
        return Err(XmaRefusal::UnsupportedChannels(streams as u32 * 2));
    }
    let mut out = Vec::new();
    let (mut n, mut frames) = (0usize, 0u32);
    for (bi, pl) in payloads.iter().enumerate() {
        let mut p = 0usize;
        for si in 0..streams {
            if p + 4 > pl.len() {
                return Err(XmaRefusal::SubBlockLength { block: bi, stream: si, w0_len: 0, available: pl.len() - p });
            }
            let w0 = u32::from_be_bytes([pl[p], pl[p + 1], pl[p + 2], pl[p + 3]]);
            let size = (w0 >> 2) as usize;
            if size < 8 || p + size > pl.len() {
                return Err(XmaRefusal::SubBlockLength { block: bi, stream: si, w0_len: size, available: pl.len() - p });
            }
            let data = &pl[p + 4..p + size];
            match rule {
                HeaderRule::Checked | HeaderRule::CheckedNoFrameMarker => {
                    if data.len() > PACKET {
                        return Err(XmaRefusal::Oversize { block: bi, bytes: data.len() });
                    }
                    let f = check_packet_rule(format!("block {bi}"), data, rule == HeaderRule::CheckedNoFrameMarker)?;
                    if pick.is_none_or(|k| k == si) {
                        frames += f;
                        out.extend_from_slice(data);
                        out.resize(out.len() + (PACKET - data.len() % PACKET) % PACKET, 0);
                        n += 1;
                    }
                }
                HeaderRule::Nhl09Variant => {
                    if pick.is_none_or(|k| k == si) {
                        for pk in data.chunks(PACKET) {
                            out.extend_from_slice(pk);
                            out.resize(out.len() + (PACKET - pk.len() % PACKET) % PACKET, 0);
                            n += 1;
                        }
                    }
                }
            }
            p += size;
        }
        let padded = terminal_zero_pad && bi + 1 == payloads.len() && pl[p..].iter().all(|&b| b == 0);
        if p != pl.len() && !padded {
            return Err(XmaRefusal::TrailingBytes { block: bi, extra: pl.len() - p });
        }
    }
    if n == 0 {
        return Err(XmaRefusal::Empty);
    }
    Ok(XmaPackets { packets: out, packet_count: n, frame_count_sum: frames })
}

/// A raw `.abk` ram segment (`u32 w0 | 0x800 packets`), checked and padded.
pub fn raw_segment_packets(segment: &[u8]) -> Result<XmaPackets, XmaRefusal> {
    raw_segment_packets_with(segment, HeaderRule::Checked)
}

/// [`raw_segment_packets`] under an explicit [`HeaderRule`] (the length word is checked either way).
pub fn raw_segment_packets_with(segment: &[u8], rule: HeaderRule) -> Result<XmaPackets, XmaRefusal> {
    if segment.len() < 8 {
        return Err(XmaRefusal::SegmentLength { w0_len: 0, segment: segment.len() });
    }
    let w0 = u32::from_be_bytes([segment[0], segment[1], segment[2], segment[3]]);
    if (w0 >> 2) as usize != segment.len() {
        return Err(XmaRefusal::SegmentLength { w0_len: (w0 >> 2) as usize, segment: segment.len() });
    }
    let data = &segment[4..];
    let mut out = Vec::with_capacity(data.len() + PACKET);
    let mut frames = 0;
    for (i, pk) in data.chunks(PACKET).enumerate() {
        match rule {
            HeaderRule::Checked => frames += check_packet(format!("packet {i}"), pk)?,
            HeaderRule::CheckedNoFrameMarker => frames += check_packet_rule(format!("packet {i}"), pk, true)?,
            HeaderRule::Nhl09Variant => {}
        }
        out.extend_from_slice(pk);
        out.resize(out.len() + (PACKET - pk.len() % PACKET) % PACKET, 0);
    }
    Ok(XmaPackets { packet_count: out.len() / PACKET, packets: out, frame_count_sum: frames })
}

/// A four- / six-channel ram segment (research 2026-09-28 handoff 05, X4): a chain of `{u32 w0 (w0 >> 2 = length
/// incl. w0) | XMA packets}` chunks, one stereo sub-stream each, that must tile the segment exactly. Returns the
/// chunks (each starting at its own `w0`, ready for [`raw_segment_packets_with`]).
pub fn segment_chunks(segment: &[u8]) -> Result<Vec<&[u8]>, XmaRefusal> {
    let mut out = Vec::new();
    let mut p = 0usize;
    while p < segment.len() {
        if p + 8 > segment.len() {
            return Err(XmaRefusal::SegmentLength { w0_len: 0, segment: segment.len() - p });
        }
        let w0 = u32::from_be_bytes([segment[p], segment[p + 1], segment[p + 2], segment[p + 3]]);
        let len = (w0 >> 2) as usize;
        if len < 8 || p + len > segment.len() {
            return Err(XmaRefusal::SegmentLength { w0_len: len, segment: segment.len() - p });
        }
        out.push(&segment[p..p + len]);
        p += len;
    }
    if out.is_empty() {
        return Err(XmaRefusal::Empty);
    }
    Ok(out)
}

/// The RIFF XMA2 file a decoder consumes: WAVEFORMATEX (tag 0x0166) with the documented
/// XMA2WAVEFORMATEX extension, then the padded packets.
pub fn riff_xma2(packets: &[u8], samples: u32, channels: u16, sample_rate: u32) -> Vec<u8> {
    let streams = channels.div_ceil(2);
    let mask: u32 = match channels {
        1 => 0x04,
        2 => 0x03,
        _ => 0,
    };
    let mut fmt = Vec::with_capacity(0x34);
    fmt.extend_from_slice(&0x0166u16.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&sample_rate.to_le_bytes());
    fmt.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
    fmt.extend_from_slice(&(channels * 2).to_le_bytes());
    fmt.extend_from_slice(&16u16.to_le_bytes());
    fmt.extend_from_slice(&0x22u16.to_le_bytes());
    fmt.extend_from_slice(&streams.to_le_bytes());
    fmt.extend_from_slice(&mask.to_le_bytes());
    fmt.extend_from_slice(&(samples * channels as u32 * 2).to_le_bytes());
    fmt.extend_from_slice(&0x10000u32.to_le_bytes());
    for _ in 0..4 {
        fmt.extend_from_slice(&0u32.to_le_bytes());
    }
    fmt.push(0);
    fmt.push(4);
    fmt.extend_from_slice(&(packets.len().div_ceil(0x10000) as u16).to_le_bytes());
    debug_assert_eq!(fmt.len(), 0x34);
    let mut out = Vec::with_capacity(0x50 + packets.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((4 + 8 + fmt.len() + 8 + packets.len()) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    out.extend_from_slice(&fmt);
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(packets.len() as u32).to_le_bytes());
    out.extend_from_slice(packets);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkt(frames: u32, len: usize) -> Vec<u8> {
        let mut p = vec![0u8; len];
        p[..4].copy_from_slice(&((frames << 26) | (32 << 11) | (1 << 8)).to_be_bytes());
        p
    }

    fn sub(p: &[u8]) -> Vec<u8> {
        let mut v = (((p.len() + 4) as u32) << 2).to_be_bytes().to_vec();
        v.extend_from_slice(p);
        v
    }

    fn hdr(ch: u8) -> SnrHeader {
        SnrHeader { version: 1, codec: CODEC, channel_config: ch, sample_rate: 24000, stream_type: 1, looped: false, num_samples: 1000 }
    }

    #[test]
    fn sns_blocks_become_padded_packets_and_refuse_by_name() {
        let blocks = vec![Block { offset: 0, samples: 3712, payload: sub(&pkt(8, 1867)) }, Block { offset: 0, samples: 697, payload: sub(&pkt(2, 290)) }];
        let p = sns_packets(&hdr(0), &blocks).unwrap();
        assert_eq!((p.packet_count, p.frame_count_sum, p.packets.len()), (2, 10, 2 * PACKET));
        let mut bad = blocks.clone();
        let l = bad[1].payload.len();
        bad[1].payload.truncate(l - 3);
        assert!(matches!(sns_packets(&hdr(0), &bad), Err(XmaRefusal::SubBlockLength { block: 1, .. })));
        let mut bad = blocks.clone();
        bad[0].payload[4] &= 0x03; // frame count 0
        assert!(matches!(sns_packets(&hdr(0), &bad), Err(XmaRefusal::PacketHeader { .. })));
        let mut bad = blocks.clone();
        bad[0].payload.extend_from_slice(&[0, 0]);
        assert!(matches!(sns_packets(&hdr(0), &bad), Err(XmaRefusal::TrailingBytes { block: 0, extra: 2 })));
        let mut h = hdr(0);
        h.codec = 0x09;
        assert!(matches!(sns_packets(&h, &blocks), Err(XmaRefusal::NotEaXma { .. })));
        assert!(matches!(sns_packets(&hdr(3), &blocks), Err(XmaRefusal::UnsupportedChannels(4))));
    }

    #[test]
    fn ast_blocks_allow_terminal_zero_padding_only_and_pick_one_stereo_pair() {
        let mut last = sub(&pkt(3, 300));
        last.extend_from_slice(&[0; 27]);
        let first = sub(&pkt(8, 1800));
        let blocks: Vec<&[u8]> = vec![&first, &last];
        // SNS rule (no padding) refuses; the `.ast` rule accepts zeros in the terminal block only.
        assert!(matches!(block_packets(&blocks, 1, None, false, HeaderRule::Checked), Err(XmaRefusal::TrailingBytes { block: 1, extra: 27 })));
        let p = block_packets(&blocks, 1, None, true, HeaderRule::Checked).unwrap();
        assert_eq!((p.packet_count, p.frame_count_sum), (2, 11));
        let mut nonzero = last.clone();
        *nonzero.last_mut().unwrap() = 1;
        assert!(matches!(block_packets(&[&first, &nonzero], 1, None, true, HeaderRule::Checked), Err(XmaRefusal::TrailingBytes { block: 1, .. })));
        let mut padded_first = first.clone();
        padded_first.extend_from_slice(&[0; 4]);
        assert!(matches!(block_packets(&[&padded_first, &last], 1, None, true, HeaderRule::Checked), Err(XmaRefusal::TrailingBytes { block: 0, extra: 4 })));
        // Two streams per block (channel config 3): stream 1 only, both still length-checked.
        let mut two = sub(&pkt(5, 500));
        two.extend(sub(&pkt(7, 700)));
        let p0 = block_packets(&[&two], 2, Some(0), false, HeaderRule::Checked).unwrap();
        let p1 = block_packets(&[&two], 2, Some(1), false, HeaderRule::Checked).unwrap();
        assert_eq!((p0.packet_count, p0.frame_count_sum, p1.frame_count_sum), (1, 5, 7));
        assert!(matches!(block_packets(&[&two], 2, Some(2), false, HeaderRule::Checked), Err(XmaRefusal::UnsupportedChannels(_))));
        let mut short = two.clone();
        short.truncate(two.len() - 10);
        assert!(matches!(block_packets(&[&short], 2, Some(0), false, HeaderRule::Checked), Err(XmaRefusal::SubBlockLength { stream: 1, .. })));
    }

    #[test]
    fn the_nhl09_header_variant_skips_the_header_check_and_splits_two_packet_sub_blocks() {
        // A constant header word (0x08000000 = "2 frames, offset 0" read literally; 0x20000000 too)
        // and a sub-block holding two packets (R98b).
        let mut pk = vec![0u8; PACKET + 300];
        pk[..4].copy_from_slice(&0x0000_0000u32.to_be_bytes()); // frame count 0: Checked refuses it
        let sub_block = sub(&pk);
        assert!(matches!(block_packets(&[&sub_block], 1, None, false, HeaderRule::Checked), Err(XmaRefusal::Oversize { .. })));
        let p = block_packets(&[&sub_block], 1, None, false, HeaderRule::Nhl09Variant).unwrap();
        assert_eq!((p.packet_count, p.frame_count_sum, p.packets.len()), (2, 0, 2 * PACKET));
        // The length word is still checked under the variant.
        let mut bad = sub_block.clone();
        bad[3] ^= 0x04;
        assert!(matches!(block_packets(&[&bad], 1, None, false, HeaderRule::Nhl09Variant), Err(XmaRefusal::SubBlockLength { .. })));
        let mut seg = vec![0u8; 4];
        seg.extend(vec![0u8; 100]);
        let n = seg.len();
        seg[..4].copy_from_slice(&((n as u32) << 2).to_be_bytes());
        assert!(matches!(raw_segment_packets(&seg), Err(XmaRefusal::PacketHeader { .. })));
        assert_eq!(raw_segment_packets_with(&seg, HeaderRule::Nhl09Variant).unwrap().packet_count, 1);
    }

    #[test]
    fn raw_segments_and_the_riff_header() {
        let mut seg = vec![0u8; 4];
        seg.extend(pkt(16, PACKET));
        seg.extend(pkt(4, 100));
        let n = seg.len();
        seg[..4].copy_from_slice(&((n as u32) << 2 | 2).to_be_bytes());
        let p = raw_segment_packets(&seg).unwrap();
        assert_eq!((p.packet_count, p.frame_count_sum), (2, 20));
        seg[3] ^= 0x04;
        assert!(matches!(raw_segment_packets(&seg), Err(XmaRefusal::SegmentLength { .. })));
        let r = riff_xma2(&p.packets, 5000, 2, 44100);
        assert_eq!(&r[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(r[4..8].try_into().unwrap()) as usize, r.len() - 8);
        assert_eq!(u16::from_le_bytes([r[20], r[21]]), 0x0166);
        assert_eq!(u16::from_le_bytes([r[36], r[37]]), 34, "cbSize");
        assert_eq!(u16::from_le_bytes([r[38], r[39]]), 1, "one stream for stereo");
        assert_eq!(&r[0x48..0x4c], b"data");
        assert_eq!(r.len(), 0x50 + 2 * PACKET);
    }
    #[test]
    fn the_no_frame_marker_is_kept_only_under_its_rule() {
        // A two-packet segment: a normal packet, then (frame count 0, offset 0x7FFF).
        let mut seg = vec![0u8; 4 + 2 * PACKET];
        let len = seg.len() as u32;
        seg[..4].copy_from_slice(&(len << 2).to_be_bytes());
        seg[4..8].copy_from_slice(&((1u32 << 26) | (32 << 11)).to_be_bytes());
        seg[4 + PACKET..8 + PACKET].copy_from_slice(&(NO_FRAME_OFFSET << 11).to_be_bytes());
        assert!(matches!(raw_segment_packets(&seg), Err(XmaRefusal::PacketHeader { frame_count: 0, offset_bits: 0x7FFF, .. })));
        let p = raw_segment_packets_with(&seg, HeaderRule::CheckedNoFrameMarker).unwrap();
        assert_eq!((p.packet_count, p.frame_count_sum), (2, 1));
        // Frame count 0 with any other offset still refuses under the new rule.
        seg[4 + PACKET..8 + PACKET].copy_from_slice(&(0x7FFEu32 << 11).to_be_bytes());
        assert!(raw_segment_packets_with(&seg, HeaderRule::CheckedNoFrameMarker).is_err());
        // NHL 10's (2, 16 384) header (offset = the packet size) still refuses.
        seg[4 + PACKET..8 + PACKET].copy_from_slice(&((2u32 << 26) | (16_384 << 11)).to_be_bytes());
        assert!(raw_segment_packets_with(&seg, HeaderRule::CheckedNoFrameMarker).is_err());
    }

    #[test]
    fn multichannel_segments_split_into_tiling_chunks() {
        let chunk = |n: usize| {
            let mut c = vec![0u8; n];
            c[..4].copy_from_slice(&((n as u32) << 2).to_be_bytes());
            c
        };
        let seg = [chunk(4 + PACKET), chunk(4 + 2 * PACKET)].concat();
        let parts = segment_chunks(&seg).unwrap();
        assert_eq!(parts.iter().map(|p| p.len()).collect::<Vec<_>>(), [4 + PACKET, 4 + 2 * PACKET]);
        // A chunk running past the segment refuses.
        let mut bad = seg.clone();
        bad.truncate(seg.len() - 1);
        assert!(segment_chunks(&bad).is_err());
    }
}
