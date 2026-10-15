//! LZX decompression — the codec inside EA's `chunklzx` payloads (NHL Legacy Xbox 360 `.big`).
//!
//! This is the Microsoft LZX format (as used in CAB / Xbox `XMemCompress`): LZ77 + canonical
//! Huffman, a sliding window, three block types (verbatim / aligned-offset / uncompressed), delta-
//! encoded Huffman tree lengths via a pretree, position slots with extra bits, and 3 repeated
//! offsets (R0/R1/R2). The bitstream reads 16-bit **little-endian** words and consumes bits MSB-first.
//!
//! Faithful to the well-known public reference (libmspack `lzxd.c`). The leading bit is the
//! Intel-E8 translation flag (0 on these PowerPC payloads → no translation).
//!
//! Verified byte-for-byte against real payloads whose decompressed form ships in the extracted tree
//! (e.g. `gamedata/physics/refcoll.xml`).

use thiserror::Error;

const MIN_MATCH: usize = 2;
const NUM_CHARS: u32 = 256;
const BLOCKTYPE_VERBATIM: u32 = 1;
const BLOCKTYPE_ALIGNED: u32 = 2;
const BLOCKTYPE_UNCOMPRESSED: u32 = 3;
const PRETREE_ELEMENTS: usize = 20;
const ALIGNED_ELEMENTS: usize = 8;
const NUM_PRIMARY_LENGTHS: u32 = 7;
const NUM_SECONDARY_LENGTHS: usize = 249;

#[derive(Debug, Error)]
pub enum LzxError {
    #[error("LZX bitstream ran past the end of the input")]
    UnexpectedEof,
    #[error("invalid LZX block type {0}")]
    BadBlockType(u32),
    #[error("invalid Huffman code in the {tree} tree")]
    BadHuffman { tree: &'static str },
    #[error("match offset {offset} exceeds the {produced} bytes produced so far")]
    BadMatchOffset { offset: usize, produced: usize },
    #[error("decoded output overran the expected size {expected}")]
    Overrun { expected: usize },
    #[error("unsupported window size 2^{0}")]
    BadWindow(u32),
}

/// MSB-first bit reader over LZX's 16-bit little-endian word stream.
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    /// Bits are held left-aligned in the high end of a 32-bit buffer.
    buffer: u32,
    bits_left: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            buffer: 0,
            bits_left: 0,
        }
    }

    #[inline]
    fn ensure(&mut self, n: u32) {
        while self.bits_left < n {
            // One 16-bit little-endian word; zero-fill past EOF (LZX pads with zero bits).
            let lo = self.data.get(self.pos).copied().unwrap_or(0) as u32;
            let hi = self.data.get(self.pos + 1).copied().unwrap_or(0) as u32;
            let word = lo | (hi << 8);
            self.pos += 2;
            self.buffer |= word << (32 - 16 - self.bits_left);
            self.bits_left += 16;
        }
    }

    /// Read `n` bits (0..=17), MSB-first.
    #[inline]
    fn read(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.ensure(n);
        let v = self.buffer >> (32 - n);
        self.buffer <<= n;
        self.bits_left -= n;
        v
    }

    /// Frame realignment (libmspack `lzxd`: after every 32 KiB output frame the partially
    /// consumed 16-bit word is discarded — `REMOVE_BITS(bits_left & 15)`).
    #[inline]
    fn realign16(&mut self) {
        let drop = self.bits_left & 15;
        self.buffer <<= drop;
        self.bits_left -= drop;
    }

    /// Align the read position for an uncompressed block (libmspack `lzxd`: "read 1-16 (not 0-15)
    /// bits to align to bytes" — the partially consumed word is discarded, and when the header
    /// ended exactly on a word boundary one **whole** word is discarded). Any further whole
    /// buffered words are given back to `pos`.
    fn align_to_word(&mut self) {
        if self.bits_left == 0 {
            self.pos += 2;
        } else {
            let whole_words = self.bits_left / 16;
            self.pos -= (whole_words * 2) as usize;
        }
        self.buffer = 0;
        self.bits_left = 0;
    }
}

/// Canonical Huffman decoder driven bit-by-bit (MSB-first), matching LZX's code assignment.
struct Huffman {
    counts: [u16; 17],
    /// Symbols sorted by (code length, symbol).
    symbols: Vec<u16>,
    max_len: u32,
}

impl Huffman {
    fn build(lengths: &[u8]) -> Self {
        let mut counts = [0u16; 17];
        for &l in lengths {
            counts[l as usize] += 1;
        }
        counts[0] = 0; // zero-length symbols are absent
        let mut max_len = 0u32;
        for len in 1..=16u32 {
            if counts[len as usize] > 0 {
                max_len = len;
            }
        }
        // Offsets into the sorted-symbol array per length.
        let mut offsets = [0u16; 17];
        for len in 1..16 {
            offsets[len + 1] = offsets[len] + counts[len];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbols[offsets[l as usize] as usize] = sym as u16;
                offsets[l as usize] += 1;
            }
        }
        Self {
            counts,
            symbols,
            max_len,
        }
    }

    #[inline]
    fn decode(&self, br: &mut BitReader<'_>, tree: &'static str) -> Result<u32, LzxError> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;
        for len in 1..=self.max_len {
            code |= br.read(1) as i32;
            let count = self.counts[len as usize] as i32;
            if code - first < count {
                return Ok(self.symbols[(index + (code - first)) as usize] as u32);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(LzxError::BadHuffman { tree })
    }
}

/// Number of position slots for a given window-bits (libmspack's mapping).
fn position_slots(window_bits: u32) -> Result<usize, LzxError> {
    Ok(match window_bits {
        15..=19 | 22..=25 => (window_bits * 2) as usize,
        20 => 42,
        21 => 50,
        _ => return Err(LzxError::BadWindow(window_bits)),
    })
}

const EXTRA_BITS: [u8; 51] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 14, 14, 15, 15, 16, 16, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
];

fn position_base() -> [u32; 51] {
    let mut base = [0u32; 51];
    let mut j = 0u32;
    for i in 0..51 {
        base[i] = j;
        j += 1u32 << EXTRA_BITS[i];
    }
    base
}

/// Output frame size: after every frame the bitstream realigns to a 16-bit word (libmspack), and
/// an encoder never lets a match cross a frame boundary.
pub const FRAME_SIZE: usize = 32_768;

/// Decode one LZX-compressed chunk to exactly `out_size` bytes. Tree lengths and repeated offsets
/// start fresh (each EA `chunklzx` chunk is an independent LZX stream). Frames of [`FRAME_SIZE`]
/// output bytes realign the bitstream to a 16-bit boundary (research 2026-09-18 handoff 40 §1:
/// "32 KiB frames with 16-bit realignment at each frame end").
pub fn decompress_lzx(
    input: &[u8],
    out_size: usize,
    window_bits: u32,
) -> Result<Vec<u8>, LzxError> {
    let slots = position_slots(window_bits)?;
    let maintree_size = NUM_CHARS as usize + slots * 8;
    let pos_base = position_base();

    let mut br = BitReader::new(input);
    let mut out: Vec<u8> = Vec::with_capacity(out_size);

    // Intel-E8 translation header: 1 flag bit, then (if set) a 32-bit size. These payloads are PPC,
    // so the flag is 0 — but honour the format so the bitstream stays aligned.
    let e8 = br.read(1);
    if std::env::var_os("LZX_DEBUG").is_some() {
        eprintln!("[lzx] window_bits={window_bits} slots={slots} maintree={maintree_size} e8={e8}");
    }
    if e8 == 1 {
        let _e8_size = (br.read(16) << 16) | br.read(16);
        // Translation is not applied (never seen set on these files); if it ever is, decode still
        // proceeds and the caller would need the post-pass. Left intentionally unimplemented.
    }

    // Persisted Huffman lengths (delta-coded across blocks) + repeated offsets.
    let mut main_lens = vec![0u8; maintree_size];
    let mut length_lens = vec![0u8; NUM_SECONDARY_LENGTHS];
    let (mut r0, mut r1, mut r2) = (1u32, 1u32, 1u32);

    while out.len() < out_size {
        // A frame boundary can also be a block boundary. Discard its padding
        // before reading the next block header, not after its Huffman trees.
        if !out.is_empty() && out.len() % FRAME_SIZE == 0 {
            br.realign16();
        }
        let block_type = br.read(3);
        // 24-bit block length: 16 bits then 8.
        let block_len = ((br.read(16) << 8) | br.read(8)) as usize;
        if std::env::var_os("LZX_DEBUG").is_some() {
            eprintln!(
                "[lzx] block_type={block_type} block_len={block_len} out={}",
                out.len()
            );
        }

        match block_type {
            BLOCKTYPE_VERBATIM | BLOCKTYPE_ALIGNED => {
                let mut aligned: Option<Huffman> = None;
                if block_type == BLOCKTYPE_ALIGNED {
                    let mut alens = [0u8; ALIGNED_ELEMENTS];
                    for a in alens.iter_mut() {
                        *a = br.read(3) as u8;
                    }
                    aligned = Some(Huffman::build(&alens));
                }
                read_lengths(&mut br, &mut main_lens, 0, NUM_CHARS as usize)?;
                read_lengths(&mut br, &mut main_lens, NUM_CHARS as usize, maintree_size)?;
                let main = Huffman::build(&main_lens);
                read_lengths(&mut br, &mut length_lens, 0, NUM_SECONDARY_LENGTHS)?;
                let length = Huffman::build(&length_lens);

                let block_start = out.len();
                let block_end = block_start + block_len;
                while out.len() < block_end {
                    if out.len() > block_start && out.len() % FRAME_SIZE == 0 && out.len() < out_size {
                        br.realign16();
                    }
                    let sym = main.decode(&mut br, "main")?;
                    if sym < NUM_CHARS {
                        out.push(sym as u8);
                        continue;
                    }
                    let elem = sym - NUM_CHARS;
                    let mut match_len = (elem & NUM_PRIMARY_LENGTHS) as usize;
                    if match_len as u32 == NUM_PRIMARY_LENGTHS {
                        let footer = length.decode(&mut br, "length")? as usize;
                        match_len += footer;
                    }
                    match_len += MIN_MATCH;

                    let slot = elem >> 3;
                    let offset: u32 = match slot {
                        0 => r0,
                        1 => {
                            // Rotate the repeated-offset slots: the match uses the old r1,
                            // which becomes the new r0, and the old r0 moves into r1.
                            std::mem::swap(&mut r0, &mut r1);
                            r0
                        }
                        2 => {
                            // As above, against slot 2. r1 is untouched.
                            std::mem::swap(&mut r0, &mut r2);
                            r0
                        }
                        _ => {
                            let extra = EXTRA_BITS[slot as usize] as u32;
                            let formatted = if block_type == BLOCKTYPE_ALIGNED && extra >= 3 {
                                let verbatim = br.read(extra - 3) << 3;
                                let a = aligned
                                    .as_ref()
                                    .expect("aligned tree present in aligned block")
                                    .decode(&mut br, "aligned")?;
                                pos_base[slot as usize] + verbatim + a
                            } else {
                                let verbatim = br.read(extra);
                                pos_base[slot as usize] + verbatim
                            };
                            let o = formatted.wrapping_sub(2);
                            r2 = r1;
                            r1 = r0;
                            r0 = o;
                            o
                        }
                    };

                    let offset = offset as usize;
                    if offset == 0 || offset > out.len() {
                        return Err(LzxError::BadMatchOffset {
                            offset,
                            produced: out.len(),
                        });
                    }
                    let start = out.len() - offset;
                    // A match never crosses a frame boundary (libmspack refuses "decode beyond
                    // output frame limits"); refuse rather than realign mid-match.
                    if match_len > 0 {
                        let frame_left = FRAME_SIZE - (out.len() % FRAME_SIZE);
                        if match_len > frame_left && out.len() + match_len < out_size {
                            return Err(LzxError::Overrun { expected: out_size });
                        }
                    }
                    for k in 0..match_len {
                        let b = out[start + k];
                        out.push(b);
                    }
                }
            }
            BLOCKTYPE_UNCOMPRESSED => {
                br.align_to_word();
                // 12 bytes: R0, R1, R2 (little-endian).
                let rd_u32 = |br: &mut BitReader<'_>| -> u32 {
                    let mut v = 0u32;
                    for i in 0..4 {
                        let b = br.data.get(br.pos).copied().unwrap_or(0) as u32;
                        br.pos += 1;
                        v |= b << (8 * i);
                    }
                    v
                };
                r0 = rd_u32(&mut br);
                r1 = rd_u32(&mut br);
                r2 = rd_u32(&mut br);
                let start = br.pos;
                let end = start + block_len;
                if end > br.data.len() {
                    return Err(LzxError::UnexpectedEof);
                }
                out.extend_from_slice(&br.data[start..end]);
                br.pos = end;
                // Re-align: uncompressed blocks are padded to an even byte boundary.
                if block_len & 1 == 1 {
                    br.pos += 1;
                }
                br.buffer = 0;
                br.bits_left = 0;
            }
            other => return Err(LzxError::BadBlockType(other)),
        }
        if out.len() > out_size {
            return Err(LzxError::Overrun { expected: out_size });
        }
    }

    out.truncate(out_size);
    Ok(out)
}

/// Read `first..last` delta-coded code lengths into `lens` using a freshly-read 20-symbol pretree.
fn read_lengths(
    br: &mut BitReader<'_>,
    lens: &mut [u8],
    first: usize,
    last: usize,
) -> Result<(), LzxError> {
    let mut pre = [0u8; PRETREE_ELEMENTS];
    for p in pre.iter_mut() {
        *p = br.read(4) as u8;
    }
    if std::env::var_os("LZX_DEBUG").is_some() {
        eprintln!("[lzx] read_lengths {first}..{last} pretree={pre:?}");
    }
    let pretree = Huffman::build(&pre);

    let mut i = first;
    while i < last {
        let z = pretree.decode(br, "pretree")?;
        match z {
            17 => {
                let run = br.read(4) as usize + 4;
                for _ in 0..run {
                    if i < last {
                        lens[i] = 0;
                        i += 1;
                    }
                }
            }
            18 => {
                let run = br.read(5) as usize + 20;
                for _ in 0..run {
                    if i < last {
                        lens[i] = 0;
                        i += 1;
                    }
                }
            }
            19 => {
                let run = br.read(1) as usize + 4;
                let z2 = pretree.decode(br, "pretree")?;
                let value = ((lens[i] as i32 - z2 as i32 + 17) % 17) as u8;
                for _ in 0..run {
                    if i < last {
                        lens[i] = value;
                        i += 1;
                    }
                }
            }
            _ => {
                lens[i] = ((lens[i] as i32 - z as i32 + 17) % 17) as u8;
                i += 1;
            }
        }
    }
    Ok(())
}
