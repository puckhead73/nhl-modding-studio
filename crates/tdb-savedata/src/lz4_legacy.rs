//! The LZ4 block compressor the PS4 games use, reproduced exactly.
//!
//! The NHL 17–21 PS4 saves were written by an LZ4 build from before 1.9.0: liblz4's current fast
//! compressor reproduces most 64 KiB blocks of the anchors but not all — 1.9.0 let a match start
//! one byte closer to the end of a block (`mflimitPlusOne`), so a few blocks come out 1–5 bytes
//! shorter than the games wrote them. This is a straight port of `LZ4_compress_generic` as it
//! stood in 1.7/1.8 (`byU16` table, `noDict`, `notLimited`, acceleration 1), which is the path
//! `LZ4F` takes for an independent block of at most 64 KiB. Every block of every NHL 17/18/19/21
//! anchor frame is reproduced byte for byte (`ps4::tests`), so a no-op save is the file the game
//! wrote. NHL 16 was written by a different build whose blocks differ in both directions (neither
//! this port, the pre-r129 `findMatchAttempts` search, nor liblz4 1.10 reproduces them); its
//! frames are equivalent, not identical.

const MINMATCH: usize = 4;
const WILDCOPYLENGTH: usize = 8;
const LASTLITERALS: usize = 5;
const MFLIMIT: usize = WILDCOPYLENGTH + MINMATCH;
const LZ4_MIN_LENGTH: usize = MFLIMIT + 1;
const HASHLOG: u32 = 12;
const HASH_SIZE: usize = 1 << (HASHLOG + 1); // byU16 doubles the table
const SKIP_TRIGGER: u32 = 6;
const ML_BITS: u32 = 4;
const ML_MASK: usize = (1 << ML_BITS) - 1;
const RUN_MASK: usize = ML_MASK;

#[inline]
fn read32(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

/// `LZ4_hash4` for the `byU16` table (hashLog + 1 bits).
#[inline]
fn hash(seq: u32) -> usize {
    (seq.wrapping_mul(2_654_435_761) >> (MINMATCH as u32 * 8 - (HASHLOG + 1))) as usize
}

/// Matching bytes from `a` and `b` up to `limit` (exclusive, applies to `a`).
#[inline]
fn count(src: &[u8], mut a: usize, mut b: usize, limit: usize) -> usize {
    let start = a;
    while a < limit && src[a] == src[b] {
        a += 1;
        b += 1;
    }
    a - start
}

/// Compress one block (≤ 64 KiB, the game's block size) with the pre-1.9 fast compressor.
pub fn compress_block(src: &[u8]) -> Vec<u8> {
    assert!(src.len() <= 65_536, "the game's blocks are at most 64 KiB");
    let iend = src.len();
    let mut out = Vec::with_capacity(src.len() + src.len() / 255 + 16);
    let mut anchor = 0usize;

    let mut table = vec![0u16; HASH_SIZE];

    if src.len() >= LZ4_MIN_LENGTH {
        let mflimit = iend - MFLIMIT;
        let matchlimit = iend - LASTLITERALS;
        let mut ip = 0usize;

        // First byte.
        table[hash(read32(src, ip))] = ip as u16;
        ip += 1;
        let mut forward_h = hash(read32(src, ip));

        'main: loop {
            // Find a match.
            let mut m;
            {
                let mut forward_ip = ip;
                let mut step = 1usize;
                let mut search_match_nb = 1u32 << SKIP_TRIGGER;
                loop {
                    let h = forward_h;
                    ip = forward_ip;
                    forward_ip += step;
                    step = (search_match_nb >> SKIP_TRIGGER) as usize;
                    search_match_nb += 1;

                    if forward_ip > mflimit {
                        break 'main;
                    }

                    m = table[h] as usize;
                    forward_h = hash(read32(src, forward_ip));
                    table[h] = ip as u16;

                    // byU16: no distance test (every offset fits).
                    if read32(src, m) == read32(src, ip) {
                        break;
                    }
                }
            }

            // Catch up.
            while ip > anchor && m > 0 && src[ip - 1] == src[m - 1] {
                ip -= 1;
                m -= 1;
            }

            // Encode literals.
            let lit_length = ip - anchor;
            let token_at = out.len();
            out.push(0);
            if lit_length >= RUN_MASK {
                let mut len = lit_length - RUN_MASK;
                out[token_at] = (RUN_MASK as u8) << ML_BITS;
                while len >= 255 {
                    out.push(255);
                    len -= 255;
                }
                out.push(len as u8);
            } else {
                out[token_at] = (lit_length as u8) << ML_BITS;
            }
            out.extend_from_slice(&src[anchor..ip]);

            let mut token_at = token_at;
            loop {
                // _next_match: encode offset.
                out.extend_from_slice(&((ip - m) as u16).to_le_bytes());

                // Encode match length.
                let mut match_code = count(src, ip + MINMATCH, m + MINMATCH, matchlimit);
                ip += MINMATCH + match_code;
                if match_code >= ML_MASK {
                    out[token_at] += ML_MASK as u8;
                    match_code -= ML_MASK;
                    while match_code >= 255 {
                        out.push(255);
                        match_code -= 255;
                    }
                    out.push(match_code as u8);
                } else {
                    out[token_at] += match_code as u8;
                }

                anchor = ip;

                // Test end of chunk.
                if ip > mflimit {
                    break 'main;
                }

                // Fill table.
                table[hash(read32(src, ip - 2))] = (ip - 2) as u16;

                // Test next position.
                let h = hash(read32(src, ip));
                m = table[h] as usize;
                table[h] = ip as u16;
                if read32(src, m) == read32(src, ip) {
                    token_at = out.len();
                    out.push(0);
                    continue;
                }

                // Prepare next loop.
                ip += 1;
                forward_h = hash(read32(src, ip));
                break;
            }
        }
    }

    // Last literals.
    let last_run = iend - anchor;
    if last_run >= RUN_MASK {
        let mut acc = last_run - RUN_MASK;
        out.push((RUN_MASK as u8) << ML_BITS);
        while acc >= 255 {
            out.push(255);
            acc -= 255;
        }
        out.push(acc as u8);
    } else {
        out.push((last_run as u8) << ML_BITS);
    }
    out.extend_from_slice(&src[anchor..]);
    out
}

/// Write an LZ4 frame the way the games do: independent 64 KiB blocks, no block checksums, no
/// content size, a content checksum (`FLG 0x64`, `BD 0x40`), every block compressed by
/// [`compress_block`]. A block that does not compress is stored uncompressed, as the format
/// requires.
pub fn compress_frame(inner: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(inner.len() / 2);
    out.extend_from_slice(&[0x04, 0x22, 0x4D, 0x18]);
    let descriptor = [0x64u8, 0x40];
    out.extend_from_slice(&descriptor);
    let hc = (twox_hash::XxHash32::oneshot(0, &descriptor) >> 8) as u8;
    out.push(hc);
    for chunk in inner.chunks(65_536) {
        let block = compress_block(chunk);
        if block.len() < chunk.len() {
            out.extend_from_slice(&(block.len() as u32).to_le_bytes());
            out.extend_from_slice(&block);
        } else {
            out.extend_from_slice(&((chunk.len() as u32) | 0x8000_0000).to_le_bytes());
            out.extend_from_slice(chunk);
        }
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&twox_hash::XxHash32::oneshot(0, inner).to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn blocks_decode_to_their_input() {
        let mut data = Vec::new();
        for i in 0..70_000u32 {
            data.push((i % 7 + (i / 1000) % 3) as u8);
        }
        for src in [&data[..0], &data[..5], &data[..13], &data[..300], &data[..65_536]] {
            let block = compress_block(src);
            let back = lz4_flex::block::decompress(&block, src.len()).unwrap();
            assert_eq!(back, src);
        }
        let frame = compress_frame(&data);
        let mut dec = lz4_flex::frame::FrameDecoder::new(&frame[..]);
        let mut back = Vec::new();
        dec.read_to_end(&mut back).unwrap();
        assert_eq!(back, data);
        assert_eq!(&frame[4..7], &[0x64, 0x40, 0xA7]);
    }
}
