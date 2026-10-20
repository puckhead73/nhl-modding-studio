//! Probe: what does a PS4 `DATA` record's LZ4 payload decompress to? (read-only, anchors)
use std::io::Read;
fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        let mut pos = 0usize;
        let mut i = 0;
        println!("{path}");
        while pos + 16 <= data.len() {
            let len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
            let pay = &data[pos + 16..pos + 16 + len];
            let off = if pay.first() == Some(&1) { 1 } else { 0 };
            if pay.get(off..off + 4) == Some(&[0x04, 0x22, 0x4D, 0x18]) {
                let mut dec = lz4_flex::frame::FrameDecoder::new(&pay[off..]);
                let mut out = Vec::new();
                match dec.read_to_end(&mut out) {
                    Ok(_) => {
                        // Nested frame (NHL 17+ mode saves): decode once more.
                        let off2 = if out.first() == Some(&1) { 1 } else { 0 };
                        if out.get(off2..off2 + 4) == Some(&[0x04, 0x22, 0x4D, 0x18]) {
                            let mut dec2 = lz4_flex::frame::FrameDecoder::new(&out[off2..]);
                            let mut out2 = Vec::new();
                            match dec2.read_to_end(&mut out2) {
                                Ok(_) => { println!("  rec {i}: NESTED frame (prefix01={off2}) -> {} bytes", out2.len()); out = out2; }
                                Err(e) => println!("  rec {i}: nested decode error {e}"),
                            }
                        }
                        let db = out.windows(2).position(|w| w == b"DB");
                        println!("  rec {i}: inner {} bytes; head {:02x?}; first 'DB' at {:?}", out.len(), &out[..out.len().min(48)], db);
                        if let Some(d) = db {
                            if d > 0 {
                                println!("     prefix text: {:?}", String::from_utf8_lossy(&out[..d.min(200)]));
                            }
                        }
                    }
                    Err(e) => println!("  rec {i}: decode error {e}"),
                }
            }
            pos += 16 + len;
            i += 1;
        }
    }
}
