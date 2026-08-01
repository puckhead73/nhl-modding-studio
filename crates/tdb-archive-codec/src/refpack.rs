//! EA RefPack ("QFS") compression — decode + encode.
//!
//! RefPack is the LZ77-style codec EA uses to wrap most FE asset
//! payloads in NHL Legacy: front-end team logos, jersey thumbnails,
//! and player-head portraits all ship as RefPack-compressed DDS files
//! inside their `.big` archives. Reference:
//! <https://wiki.niotso.org/RefPack>.
//!
//! ## Header layout
//!
//! ```text
//! byte 0    flag   (bit 7 = 4-byte sizes, bit 6 = has compressed-size field, bits 5..1 fixed = 0b10000)
//! byte 1    0xFB   (signature)
//! [bytes]   optional compressed-size field, 3 or 4 bytes BE
//! [bytes]   uncompressed-size field, 3 or 4 bytes BE
//! body      command stream (described below)
//! ```
//!
//! The NHL13 FE corpus consistently uses the 3-byte-uncompressed-size
//! variant (`0x10 0xFB hi mid lo`). Our encoder always writes that
//! variant — sufficient for payloads up to 16 MiB, well above any
//! portrait/logo/thumbnail we'll ever produce. The PS3 runtime decodes
//! by walking the command stream, so writing 3-byte output is portable
//! regardless of what flag bits the original entry used.
//!
//! ## Command stream
//!
//! Each non-terminator command emits a literal run of `lit_len` bytes
//! from the input followed by a back-reference of `ref_len` bytes
//! starting at `output_pos - ref_offset - 1`. Five command families:
//!
//! | First byte    | Length | Meaning                                |
//! | ------------- | ------ | -------------------------------------- |
//! | `0x00..=0x7F` | 2      | Short — ref_len 3..10, offset 1..1024  |
//! | `0x80..=0xBF` | 3      | Medium — ref_len 4..67, offset 1..16384 |
//! | `0xC0..=0xDF` | 4      | Long — ref_len 5..1028, offset 1..131072 |
//! | `0xE0..=0xFB` | 1      | Literal-only run, length 4..112        |
//! | `0xFC..=0xFF` | 1      | Terminator, trailing literal 0..3      |
//!
//! All literal-run lengths in non-terminator commands are 0..3 bytes;
//! the literal-only family handles longer copies in 4-byte multiples.

#![allow(clippy::needless_range_loop)]

use thiserror::Error;

/// Window size (back-reference reach) — 17-bit offset on the long
/// command form. Matches EA's encoder.
const WINDOW_SIZE: usize = 131_072;
/// Longest back-reference the format can encode (long command form).
const MAX_REF_LEN: usize = 1028;
/// Hash-table size — power-of-two for fast modulo. Matched to a
/// 3-byte rolling hash; collisions are resolved via chained `prev[]`
/// indices.
const HASH_SIZE: usize = 1 << 16;
const HASH_MASK: usize = HASH_SIZE - 1;

#[derive(Debug, Error)]
pub enum RefpackError {
    #[error("RefPack header was truncated")]
    Truncated,
    #[error("RefPack back-reference points before the start of the output")]
    OutOfBounds,
}

/// Parsed RefPack header — uncompressed payload length plus the byte
/// offset where the command stream starts.
#[derive(Debug)]
pub struct RefpackHeader {
    pub uncompressed_len: usize,
    pub body_offset: usize,
}

/// Returns `true` when `bytes` looks like the start of a RefPack
/// payload. Gate: byte 1 == `0xFB` and byte 0's fixed bits match the
/// documented flag mask. Accepts both 3-byte and 4-byte size variants
/// and either compressed-size-present or not.
pub fn is_refpack(bytes: &[u8]) -> bool {
    if bytes.len() < 2 {
        return false;
    }
    bytes[1] == 0xFB && (bytes[0] & 0x3E) == 0x10
}

/// Decode the RefPack header. Returns `None` when the buffer is too
/// short to hold even the minimum header.
pub fn parse_refpack_header(bytes: &[u8]) -> Option<RefpackHeader> {
    if bytes.len() < 5 {
        return None;
    }
    let flag = bytes[0];
    let has_compressed_size = (flag & 0x01) != 0;
    let four_byte_sizes = (flag & 0x80) != 0;
    let mut pos = 2usize;
    let size_bytes = if four_byte_sizes { 4 } else { 3 };
    if has_compressed_size {
        pos += size_bytes;
    }
    if pos + size_bytes > bytes.len() {
        return None;
    }
    let mut uncompressed = 0usize;
    for &b in &bytes[pos..pos + size_bytes] {
        uncompressed = (uncompressed << 8) | b as usize;
    }
    pos += size_bytes;
    Some(RefpackHeader {
        uncompressed_len: uncompressed,
        body_offset: pos,
    })
}

/// Decompress a RefPack-wrapped payload into its original bytes.
///
/// Some FE assets report a header `uncompressed_len` slightly larger
/// than what the command stream actually emits (the field is a hint,
/// not a hard contract). We trust the stream and return what we got.
pub fn refpack_decompress(bytes: &[u8]) -> Result<Vec<u8>, RefpackError> {
    let header = parse_refpack_header(bytes).ok_or(RefpackError::Truncated)?;
    let mut out: Vec<u8> = Vec::with_capacity(header.uncompressed_len);
    let mut ip = header.body_offset;

    while ip < bytes.len() {
        let a = bytes[ip];
        ip += 1;

        if a < 0x80 {
            // Short — 2-byte command.
            if ip >= bytes.len() {
                return Err(RefpackError::Truncated);
            }
            let b = bytes[ip];
            ip += 1;
            let lit_len = (a & 0x03) as usize;
            let ref_len = (((a >> 2) & 0x07) + 3) as usize;
            let ref_offset = (((a & 0x60) as usize) << 3) | b as usize;
            copy_lit(bytes, &mut ip, &mut out, lit_len)?;
            copy_ref(&mut out, ref_offset, ref_len)?;
        } else if a < 0xC0 {
            // Medium — 3-byte command.
            if ip + 1 >= bytes.len() {
                return Err(RefpackError::Truncated);
            }
            let b = bytes[ip];
            let c = bytes[ip + 1];
            ip += 2;
            let lit_len = ((b >> 6) & 0x03) as usize;
            let ref_len = ((a & 0x3F) + 4) as usize;
            let ref_offset = ((b as usize & 0x3F) << 8) | c as usize;
            copy_lit(bytes, &mut ip, &mut out, lit_len)?;
            copy_ref(&mut out, ref_offset, ref_len)?;
        } else if a < 0xE0 {
            // Long — 4-byte command.
            if ip + 2 >= bytes.len() {
                return Err(RefpackError::Truncated);
            }
            let b = bytes[ip];
            let c = bytes[ip + 1];
            let d = bytes[ip + 2];
            ip += 3;
            let lit_len = (a & 0x03) as usize;
            let ref_len = (((a as usize & 0x0C) << 6) | d as usize) + 5;
            let ref_offset = ((a as usize & 0x10) << 12) | ((b as usize) << 8) | c as usize;
            copy_lit(bytes, &mut ip, &mut out, lit_len)?;
            copy_ref(&mut out, ref_offset, ref_len)?;
        } else if a < 0xFC {
            // Literal-only run, length 4..112 (4-byte multiples).
            let lit_len = (((a & 0x1F) + 1) * 4) as usize;
            copy_lit(bytes, &mut ip, &mut out, lit_len)?;
        } else {
            // Terminator (0xFC..=0xFF). Trailing literal then halt.
            let lit_len = (a & 0x03) as usize;
            copy_lit(bytes, &mut ip, &mut out, lit_len)?;
            break;
        }
    }

    Ok(out)
}

fn copy_lit(bytes: &[u8], ip: &mut usize, out: &mut Vec<u8>, n: usize) -> Result<(), RefpackError> {
    if *ip + n > bytes.len() {
        return Err(RefpackError::Truncated);
    }
    out.extend_from_slice(&bytes[*ip..*ip + n]);
    *ip += n;
    Ok(())
}

fn copy_ref(out: &mut Vec<u8>, offset: usize, n: usize) -> Result<(), RefpackError> {
    if offset + 1 > out.len() {
        return Err(RefpackError::OutOfBounds);
    }
    let start = out.len() - offset - 1;
    for i in 0..n {
        let b = out[start + i];
        out.push(b);
    }
    Ok(())
}

// =====================================================================
// Encoder
// =====================================================================

/// Compress `payload` into a RefPack-wrapped byte stream.
///
/// Always writes the **3-byte-uncompressed-size** header variant
/// (`0x10 0xFB hi mid lo`). The encoder uses a chained hash table for
/// longest-match search — a standard LZ77 approach that produces
/// output competitive with EA's reference encoder for typical
/// DDS-shaped payloads (highly repetitive headers + heterogeneous
/// pixel data).
///
/// Output is always valid input to [`refpack_decompress`]; a roundtrip
/// is byte-exact.
///
/// # Panics
///
/// Panics if `payload.len()` exceeds `0xFF_FFFF` (16 MiB - 1) — the
/// 3-byte-size header can't express larger payloads. The caller is
/// expected to gate this for textures (FE portraits are <1 MB).
pub fn refpack_compress(payload: &[u8]) -> Vec<u8> {
    assert!(
        payload.len() <= 0x00FF_FFFF,
        "RefPack 3-byte header cannot encode payloads >= 16 MiB; got {} bytes",
        payload.len()
    );

    // Header: flag=0x10 (3-byte sizes, no compressed-size field),
    // signature=0xFB, then uncompressed length BE 24-bit.
    let mut out = Vec::with_capacity(payload.len() / 2 + 32);
    out.push(0x10);
    out.push(0xFB);
    let n = payload.len();
    out.push(((n >> 16) & 0xFF) as u8);
    out.push(((n >> 8) & 0xFF) as u8);
    out.push((n & 0xFF) as u8);

    if payload.is_empty() {
        out.push(0xFC); // terminator, 0 trailing literals
        return out;
    }

    // Hash table: `head[h]` is the most-recent input position whose
    // 3-byte prefix hashed to `h`; `prev[pos % WINDOW_SIZE]` is the
    // previous position with the same hash. `usize::MAX` means "none".
    let mut head = vec![usize::MAX; HASH_SIZE];
    let mut prev = vec![usize::MAX; WINDOW_SIZE];

    // The next literal byte that hasn't been emitted yet — we batch
    // literals between back-references and flush them either via
    // command-prefix lit_len (0..3 bytes) or via stand-alone
    // literal-only runs (4..112 bytes, multiples of 4).
    let mut literal_start = 0usize;
    let mut ip = 0usize;

    while ip < n {
        // Need at least 3 bytes for a back-reference search; tail
        // bytes go out as trailing literals via the terminator.
        if ip + 3 > n {
            break;
        }

        let h = hash3(&payload[ip..]);
        let (best_off, best_len) = find_match(payload, ip, h, &head, &prev);

        // RefPack short command minimum match is 3 bytes. Anything
        // shorter is cheaper emitted as a literal.
        if best_len >= 3 {
            // Flush pending literals + emit the back-reference in
            // one shot. `emit_match_with_lits` splits the literal
            // buffer into whole-4-byte literal-only runs followed by
            // a 0..3-byte inline prefix on the match command.
            emit_match_with_lits(&mut out, best_off, best_len, &payload[literal_start..ip]);

            // Insert every position the match covers into the hash
            // table so future matches can extend through it.
            for k in ip..(ip + best_len) {
                if k + 3 <= n {
                    let hk = hash3(&payload[k..]);
                    prev[k % WINDOW_SIZE] = head[hk];
                    head[hk] = k;
                }
            }
            ip += best_len;
            literal_start = ip;
        } else {
            // No useful match — advance by one and insert.
            prev[ip % WINDOW_SIZE] = head[h];
            head[h] = ip;
            ip += 1;
        }
    }

    // Tail. RefPack's terminator (0xFC..=0xFF) carries a trailing
    // literal run of 0..3 bytes; longer tails get flushed via
    // literal-only runs first.
    let mut tail_start = literal_start;
    let tail_end = n;
    let tail = &payload[tail_start..tail_end];

    // Flush 4-byte-multiple chunks of literal-only runs (max 112 bytes
    // per command) until <4 bytes remain, then emit terminator.
    let mut tail_remaining = tail.len();
    while tail_remaining >= 4 {
        let take = std::cmp::min(tail_remaining - (tail_remaining % 4), 112);
        let chunk = &payload[tail_start..tail_start + take];
        let units = (take / 4) as u8;
        debug_assert!(units >= 1 && units <= 28);
        out.push(0xE0 | (units - 1));
        out.extend_from_slice(chunk);
        tail_start += take;
        tail_remaining -= take;
    }
    // Terminator: 0xFC + trailing literal length 0..3.
    let trailing = &payload[tail_start..];
    debug_assert!(trailing.len() <= 3);
    out.push(0xFC | trailing.len() as u8);
    out.extend_from_slice(trailing);

    out
}

/// Flush pending literals + emit one back-reference. Splits the
/// literal buffer into whole-4-byte literal-only runs (each 4..112
/// bytes) followed by a 0..3-byte inline prefix that rides on the
/// back-reference command's `lit_len` field. Picks the smallest
/// command family (short / medium / long) that fits the
/// `(offset, ref_len)` combo.
fn emit_match_with_lits(out: &mut Vec<u8>, ref_off: usize, ref_len: usize, lits: &[u8]) {
    // Split: whole 4-byte units go through literal-only runs;
    // remainder 0..3 attaches to the back-reference command.
    let remainder = lits.len() % 4;
    let bulk = lits.len() - remainder;
    let mut start = 0usize;
    while start < bulk {
        let take = std::cmp::min(bulk - start, 112);
        let units = (take / 4) as u8;
        out.push(0xE0 | (units - 1));
        out.extend_from_slice(&lits[start..start + take]);
        start += take;
    }
    let inline_lits = &lits[bulk..];
    let lit_len = inline_lits.len();
    debug_assert!(lit_len <= 3);

    // Pick command family based on offset + ref_len.
    let off_minus_1 = ref_off - 1;
    if off_minus_1 < 1024 && ref_len >= 3 && ref_len <= 10 {
        // Short — 2 bytes.
        let a = (((ref_len - 3) as u8) << 2)
            | ((off_minus_1 >> 8) as u8 & 0x03) << 5
            | (lit_len as u8 & 0x03);
        let b = (off_minus_1 & 0xFF) as u8;
        out.push(a);
        out.push(b);
    } else if off_minus_1 < 16_384 && ref_len >= 4 && ref_len <= 67 {
        // Medium — 3 bytes.
        let a = 0x80 | ((ref_len - 4) as u8 & 0x3F);
        let b = ((lit_len as u8 & 0x03) << 6) | (((off_minus_1 >> 8) as u8) & 0x3F);
        let c = (off_minus_1 & 0xFF) as u8;
        out.push(a);
        out.push(b);
        out.push(c);
    } else if off_minus_1 < 131_072 && ref_len >= 5 && ref_len <= 1028 {
        // Long — 4 bytes.
        let len_minus_5 = ref_len - 5;
        let a = 0xC0
            | ((off_minus_1 >> 16) as u8 & 0x01) << 4
            | (((len_minus_5 >> 8) as u8) & 0x03) << 2
            | (lit_len as u8 & 0x03);
        let b = ((off_minus_1 >> 8) & 0xFF) as u8;
        let c = (off_minus_1 & 0xFF) as u8;
        let d = (len_minus_5 & 0xFF) as u8;
        out.push(a);
        out.push(b);
        out.push(c);
        out.push(d);
    } else {
        // Should never happen: find_match clamps ref_len to MAX_REF_LEN
        // and offsets to WINDOW_SIZE. Emit literals as a fallback so
        // we degrade rather than corrupt.
        for &b in inline_lits {
            out.push(0xE0);
            out.push(b);
        }
        return;
    }
    out.extend_from_slice(inline_lits);
}

/// 3-byte rolling hash — fold three bytes into the hash table index.
/// Simple Bob Jenkins-style mix; good enough for an LZ77 matcher.
fn hash3(bytes: &[u8]) -> usize {
    let a = bytes[0] as usize;
    let b = bytes[1] as usize;
    let c = bytes[2] as usize;
    ((a * 2654435761) ^ (b * 40503) ^ (c * 65599)) & HASH_MASK
}

/// Find the longest matching prefix at `ip` whose hash chains back
/// through `head[h] → prev[…]`. Returns `(offset, length)` where
/// `offset = ip - match_pos - 1` (RefPack's encoding) and `length` is
/// the number of bytes matched (≥ 0).
fn find_match(
    payload: &[u8],
    ip: usize,
    h: usize,
    head: &[usize],
    prev: &[usize],
) -> (usize, usize) {
    let mut best_len = 0usize;
    let mut best_off = 0usize;
    let mut cand = head[h];
    // Cap the chain walk so worst-case input doesn't grind the
    // encoder. 64 walks per position is a standard zlib-style limit.
    let mut walks = 0usize;
    let max_walks = 64;
    let max_avail = std::cmp::min(payload.len() - ip, MAX_REF_LEN);

    while cand != usize::MAX && walks < max_walks {
        if ip - cand > WINDOW_SIZE {
            break;
        }
        // Quick check on the byte at `best_len` — if it doesn't
        // match, this candidate can't beat the best we've got. Skip
        // the quick check when `best_len` already saturates the
        // available window (no room for an improvement, and reading
        // `payload[ip + best_len]` would be out-of-bounds at exactly
        // that saturation point).
        let quick_ok = best_len == 0
            || (ip + best_len < payload.len()
                && payload[cand + best_len] == payload[ip + best_len]);
        if quick_ok {
            let mut k = 0usize;
            while k < max_avail && payload[cand + k] == payload[ip + k] {
                k += 1;
            }
            if k > best_len {
                best_len = k;
                best_off = ip - cand - 1;
                if best_len >= MAX_REF_LEN {
                    break;
                }
            }
        }
        cand = prev[cand % WINDOW_SIZE];
        walks += 1;
    }

    // Only return shapes that some RefPack command family can encode.
    // If find_match returned anything else, emit_match_with_lits's
    // fallback path would drop the back-reference and desync the
    // decoder. Encodable (off_minus_1, ref_len) pairs:
    //   short:  off < 1024   && len in 3..=10
    //   medium: off < 16384  && len in 4..=67
    //   long:   off < 131072 && len in 5..=1028
    if best_len < 3 || best_off >= WINDOW_SIZE {
        return (0, 0);
    }
    if best_off >= 16_384 && best_len < 5 {
        return (0, 0);
    }
    if best_off >= 1024 && best_len < 4 {
        return (0, 0);
    }
    if best_len > MAX_REF_LEN {
        best_len = MAX_REF_LEN;
    }
    (best_off + 1, best_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip_3_byte_size() {
        // 0x10 0xFB followed by a 3-byte BE length.
        let mut h = vec![0x10, 0xFB, 0x12, 0x34, 0x56];
        h.extend_from_slice(&[0xFC]); // terminator
        let parsed = parse_refpack_header(&h).unwrap();
        assert_eq!(parsed.uncompressed_len, 0x123456);
        assert_eq!(parsed.body_offset, 5);
    }

    #[test]
    fn is_refpack_recognises_canonical_flag() {
        assert!(is_refpack(&[0x10, 0xFB]));
        assert!(is_refpack(&[0x11, 0xFB]));
        assert!(is_refpack(&[0x90, 0xFB])); // 4-byte sizes
        assert!(!is_refpack(&[0x10, 0xFC])); // wrong sig
        assert!(!is_refpack(&[0xFF, 0xFB])); // wrong flag mask
        assert!(!is_refpack(&[0x10])); // truncated
    }

    #[test]
    fn round_trip_high_entropy() {
        // Pseudo-random bytes (LCG) — adversarial for LZ77.
        let mut x: u32 = 0xCAFEBABE;
        let mut payload = Vec::with_capacity(8192);
        for _ in 0..8192 {
            x = x.wrapping_mul(1103515245).wrapping_add(12345);
            payload.push((x >> 16) as u8);
        }
        let compressed = refpack_compress(&payload);
        let decompressed = refpack_decompress(&compressed).unwrap();
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn round_trip_low_entropy_repeating() {
        // Highly compressible — should produce many back-references.
        let pattern = b"ABCDEFGH";
        let payload: Vec<u8> = pattern.iter().cycle().take(4096).copied().collect();
        let compressed = refpack_compress(&payload);
        let decompressed = refpack_decompress(&compressed).unwrap();
        assert_eq!(decompressed, payload);
        // Sanity: compression actually compresses.
        assert!(compressed.len() < payload.len() / 4);
    }

    #[test]
    fn round_trip_mixed_runs_and_random() {
        // Realistic mix: 128 byte DDS-style header + repeating chunks
        // + random pixel bytes.
        let mut payload = Vec::new();
        payload.extend_from_slice(b"DDS \x7C\x00\x00\x00");
        payload.extend_from_slice(&[0u8; 120]); // mostly zero header
        for _ in 0..256 {
            payload.extend_from_slice(b"\xff\x00\xff\x00");
        }
        let mut x: u32 = 1;
        for _ in 0..2048 {
            x = x.wrapping_mul(48271).wrapping_rem(0x7fffffff);
            payload.push((x & 0xFF) as u8);
        }
        let compressed = refpack_compress(&payload);
        let decompressed = refpack_decompress(&compressed).unwrap();
        assert_eq!(decompressed, payload);
    }

    #[test]
    fn round_trip_empty() {
        let compressed = refpack_compress(&[]);
        let decompressed = refpack_decompress(&compressed).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn round_trip_short_tail_under_4_bytes() {
        // Exercise the terminator's trailing-literal path for sizes
        // 1..3 — every length needs its own slot.
        for len in 1..=3 {
            let payload: Vec<u8> = (0..len).map(|i| i as u8 + 0x40).collect();
            let compressed = refpack_compress(&payload);
            let decompressed = refpack_decompress(&compressed).unwrap();
            assert_eq!(decompressed, payload, "len={len}");
        }
    }

    #[test]
    fn round_trip_payload_just_over_chunk_boundary() {
        // Sizes around literal-only-run max chunk (112 bytes) and
        // common DDS header sizes.
        for size in [4usize, 8, 64, 112, 113, 128, 256, 257, 1024] {
            let payload: Vec<u8> = (0..size).map(|i| (i * 17) as u8).collect();
            let compressed = refpack_compress(&payload);
            let decompressed = refpack_decompress(&compressed).unwrap();
            assert_eq!(decompressed, payload, "size={size}");
        }
    }

    #[test]
    fn compressed_output_starts_with_canonical_header() {
        let compressed = refpack_compress(b"hello");
        assert_eq!(&compressed[..2], &[0x10, 0xFB]);
        // Uncompressed-size BE 24-bit = 5.
        assert_eq!(&compressed[2..5], &[0x00, 0x00, 0x05]);
        // is_refpack accepts our own output.
        assert!(is_refpack(&compressed));
    }
}
