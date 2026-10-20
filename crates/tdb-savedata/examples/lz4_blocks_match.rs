//! Probe: compressing each 64 KiB block of the inner payload on a clean state (block API) — does
//! every block of the anchor frame match? (read-only)
use lz4::block::{compress, CompressionMode};
use std::io::Read;
fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        let mut pos = 0usize;
        while pos + 16 <= data.len() {
            let len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
            let pay = &data[pos + 16..pos + 16 + len];
            let off = if pay.first() == Some(&1) { 1 } else { 0 };
            if pay.get(off..off + 4) == Some(&[0x04, 0x22, 0x4D, 0x18]) {
                let frame = &pay[off..];
                let mut dec = lz4_flex::frame::FrameDecoder::new(frame);
                let mut inner = Vec::new();
                dec.read_to_end(&mut inner).unwrap();
                let mut q = 7usize;
                let (mut n, mut ok, mut bad) = (0, 0, Vec::new());
                for chunk in inner.chunks(65536) {
                    let bs = u32::from_le_bytes(frame[q..q + 4].try_into().unwrap()) as usize;
                    let block = &frame[q + 4..q + 4 + bs];
                    let ours = compress(chunk, Some(CompressionMode::DEFAULT), false).unwrap();
                    let port = tdb_savedata::lz4_legacy::compress_block(chunk);
                    if ours == block { ok += 1 } else { bad.push((n, bs, ours.len(), port == block)) }
                    q += 4 + bs;
                    n += 1;
                }
                println!("{path}: {n} blocks, {ok} identical to liblz4; others (idx, anchor len, liblz4 len, port matches?) {:?}", &bad[..bad.len().min(14)]);
                break;
            }
            pos += 16 + len;
        }
    }
}
