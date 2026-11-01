//! Position-aware PS4 TOC chunk entries and stored-block framing for overlay writes.
//! Game-verified for NHL 24 01.60 under shadPS4 only: unsigned TOC payload edits load and
//! codec-0 blocks are accepted (docs/validation/nhl24-fbw1-stage-a-20261101.md).
use crate::{be, invalid, slice, Result};
use serde::Serialize;

/// The signed wrapper's payload start; every byte before it is preserved verbatim.
pub const WRAPPED_PAYLOAD: usize = 0x22c;
/// Stored block size written by Studio (the size the growth probe loaded with).
pub const BLOCK: usize = 0x10000;

/// One chunk-table entry: the native (EBX/manifest order) GUID and the absolute file
/// offset of its 12-byte `(cas id, offset, size)` data record.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ChunkSlot {
    pub guid: String,
    pub at: usize,
    pub cas: u32,
    pub offset: u32,
    pub size: u32,
}

fn base(toc: &[u8]) -> Result<usize> {
    if toc.starts_with(&[0, 0xd1, 0xce, 1]) {
        slice(toc, WRAPPED_PAYLOAD, 60)?;
        Ok(WRAPPED_PAYLOAD)
    } else if toc.starts_with(&[0, 0xd1, 0xce]) {
        Err(invalid("unsupported D1CE wrapper version"))
    } else {
        Ok(0)
    }
}

/// Every chunk-table entry of a PS4 superbundle TOC, with its byte position.
pub fn chunk_slots(toc: &[u8]) -> Result<Vec<ChunkSlot>> {
    let at = base(toc)?;
    let b = &toc[at..];
    let w = (0..15).map(|i| be(b, i * 4)).collect::<Result<Vec<_>>>()?;
    if w[0] != 0x3c || w[10] as u64 != w[5] as u64 * 3 {
        return Err(invalid("not a PS4 3-word chunk-record TOC"));
    }
    let (cg, cc, cd) = (w[4] as usize, w[5] as usize, w[9] as usize);
    slice(b, cg, cc * 20)?;
    slice(b, cd, cc * 12)?;
    let mut out = Vec::with_capacity(cc);
    for i in 0..cc {
        let r = be(b, cg + i * 20 + 16)?;
        let idx = (r & 0xffffff) as usize;
        if r >> 24 != 1 || !idx.is_multiple_of(3) || idx + 3 > w[10] as usize {
            return Err(invalid("invalid chunk reference"));
        }
        let mut g = slice(b, cg + i * 20, 16)?.to_vec();
        // The TOC stores GUID bytes reversed relative to EBX and collector manifests.
        g.reverse();
        let p = cd + idx * 4;
        out.push(ChunkSlot {
            guid: crate::hex(&g),
            at: at + p,
            cas: be(b, p)?,
            offset: be(b, p + 4)?,
            size: be(b, p + 8)?,
        });
    }
    Ok(out)
}

/// Rewrite the data records of the named chunks. Each GUID must have exactly one entry;
/// the wrapper and signature region, and every other byte, are left unchanged.
pub fn repoint(toc: &[u8], edits: &[(String, u32, u32, u32)]) -> Result<Vec<u8>> {
    let slots = chunk_slots(toc)?;
    let mut out = toc.to_vec();
    for (guid, cas, offset, size) in edits {
        let mut hits = slots.iter().filter(|s| &s.guid == guid);
        let s = hits
            .next()
            .ok_or_else(|| invalid(format!("chunk {guid} is not in this TOC")))?;
        if hits.next().is_some() {
            return Err(invalid(format!("chunk {guid} has more than one TOC entry")));
        }
        out[s.at..s.at + 4].copy_from_slice(&cas.to_be_bytes());
        out[s.at + 4..s.at + 8].copy_from_slice(&offset.to_be_bytes());
        out[s.at + 8..s.at + 12].copy_from_slice(&size.to_be_bytes());
    }
    if out[..base(toc)?] != toc[..base(toc)?] {
        return Err(invalid("TOC header changed"));
    }
    Ok(out)
}

/// Frame `data` as codec-0 blocks of `sizes` decoded bytes each (summing to its length).
pub fn stored_with(data: &[u8], sizes: &[usize]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() + sizes.len() * 8);
    let mut p = 0;
    for &n in sizes {
        if n == 0 || n > 0xfffff {
            return Err(invalid("stored block size out of range"));
        }
        let block = slice(data, p, n)?;
        out.extend_from_slice(&(n as u32).to_be_bytes());
        out.extend_from_slice(&[0, 0x70 | ((n >> 16) & 15) as u8]);
        out.extend_from_slice(&(n as u16).to_be_bytes());
        out.extend_from_slice(block);
        p += n;
    }
    if p != data.len() {
        return Err(invalid("block sizes do not cover the data"));
    }
    Ok(out)
}

/// Uniform [`BLOCK`]-sized framing.
pub fn stored(data: &[u8]) -> Vec<u8> {
    let sizes: Vec<_> = data.chunks(BLOCK).map(<[u8]>::len).collect();
    stored_with(data, &sizes).expect("uniform blocks cover the data")
}

/// The stored-stream range of the uniform blocks that cover `size` bytes at `offset`,
/// as collector manifests record it; `total` is the framed stream length.
pub fn stored_range(offset: usize, size: usize, total: usize) -> (u64, u64) {
    let first = offset / BLOCK;
    let last = (offset + size.max(1) - 1) / BLOCK;
    let lo = first * (BLOCK + 8);
    let hi = ((last + 1) * (BLOCK + 8)).min(total);
    (lo as u64, hi as u64)
}

#[cfg(test)]
pub(crate) mod tests_support {
    use super::WRAPPED_PAYLOAD;

    /// A minimal wrapped PS4 TOC: no bundles, `chunks.len()` chunk entries.
    pub(crate) fn toc(chunks: &[([u8; 16], u32, u32, u32)]) -> Vec<u8> {
        let n = chunks.len() as u32;
        let (cg, cd) = (60u32, 60 + n * 20);
        let mut w = [0u32; 15];
        w[0] = 0x3c;
        w[4] = cg;
        w[5] = n;
        w[9] = cd;
        w[10] = n * 3;
        let mut b: Vec<u8> = w.iter().flat_map(|v| v.to_be_bytes()).collect();
        for (i, (g, ..)) in chunks.iter().enumerate() {
            let mut r = *g;
            r.reverse();
            b.extend_from_slice(&r);
            b.extend_from_slice(&((1 << 24) | (i as u32 * 3)).to_be_bytes());
        }
        for (_, cas, off, size) in chunks {
            for v in [cas, off, size] {
                b.extend_from_slice(&v.to_be_bytes());
            }
        }
        let mut out = vec![0, 0xd1, 0xce, 1];
        out.extend((4..WRAPPED_PAYLOAD).map(|i| i as u8));
        out.extend(b);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::toc;
    use super::*;
    use crate::blocks;

    #[test]
    fn slots_report_native_guids_and_positions() {
        let g = [7u8; 16];
        let mut h = [0u8; 16];
        h[0] = 0xab;
        let t = toc(&[(g, 0x10001, 5, 9), (h, 0x10002, 0, 4)]);
        let s = chunk_slots(&t).unwrap();
        assert_eq!(s[1].guid, format!("ab{}", "00".repeat(15)));
        assert_eq!((s[1].cas, s[1].offset, s[1].size), (0x10002, 0, 4));
        assert_eq!(be(&t, s[0].at + 4).unwrap(), 5);
    }

    #[test]
    fn repoint_changes_only_the_named_record() {
        let g = [1u8; 16];
        let h = [2u8; 16];
        let t = toc(&[(g, 0x10001, 5, 9), (h, 0x10001, 20, 30)]);
        let out = repoint(&t, &[(crate::hex(&h), 0x10002, 0, 99)]).unwrap();
        let s = chunk_slots(&out).unwrap();
        assert_eq!((s[1].cas, s[1].offset, s[1].size), (0x10002, 0, 99));
        assert_eq!(chunk_slots(&t).unwrap()[0], s[0]);
        let changed = t.iter().zip(&out).filter(|(a, b)| a != b).count();
        assert!(changed <= 12);
        assert!(repoint(&t, &[("00".repeat(16), 0, 0, 0)]).is_err());
    }

    #[test]
    fn repoint_refuses_duplicate_entries() {
        let g = [3u8; 16];
        let t = toc(&[(g, 0x10001, 0, 1), (g, 0x10001, 1, 1)]);
        assert!(repoint(&t, &[(crate::hex(&g), 0x10002, 0, 1)]).is_err());
    }

    #[test]
    fn stored_framing_round_trips_and_ranges_cover_files() {
        let data: Vec<u8> = (0..BLOCK * 2 + 77).map(|i| (i * 31) as u8).collect();
        let framed = stored(&data);
        assert_eq!(blocks::decode_stored(&framed, usize::MAX).unwrap(), data);
        let b = blocks::inspect(&framed).unwrap();
        assert!(b.iter().all(|b| b.codec == 0));
        assert_eq!(stored_range(0, 10, framed.len()), (0, (BLOCK + 8) as u64));
        assert_eq!(
            stored_range(BLOCK - 1, 2, framed.len()),
            (0, 2 * (BLOCK + 8) as u64)
        );
        assert_eq!(
            stored_range(2 * BLOCK, 77, framed.len()),
            (2 * (BLOCK + 8) as u64, framed.len() as u64)
        );
    }
}
