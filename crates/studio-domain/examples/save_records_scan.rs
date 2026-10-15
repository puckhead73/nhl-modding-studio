//! Read-only probe. PS4 `DATA`: list every record (length, framing guess). Other files: show the
//! bytes that follow the first TDB (the container trailer) and whether they look compressed.
//!
//! `cargo run -p studio-domain --example save_records_scan -- <file>...`

fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        println!("== {path} ({} bytes)", data.len());
        if tdb_savedata::is_ps4_save(&data) {
            let mut pos = 0usize;
            let mut n = 0;
            while pos + 16 <= data.len() {
                let len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
                let payload = &data[pos + 16..(pos + 16 + len).min(data.len())];
                let head: Vec<String> = payload.iter().take(8).map(|b| format!("{b:02x}")).collect();
                let kind = if payload.starts_with(&[0x04, 0x22, 0x4D, 0x18]) || payload.get(1..5) == Some(&[0x04, 0x22, 0x4D, 0x18][..]) {
                    "LZ4"
                } else if payload.starts_with(tdb_archive_codec::MAGIC_CHUNKZIP) {
                    "chunkzip"
                } else if payload.first() == Some(&b'{') {
                    "json"
                } else {
                    "other"
                };
                println!("   record {n}: {len} bytes, {kind}, head {}", head.join(" "));
                pos += 16 + len;
                n += 1;
            }
        } else if let Ok(c) = tdb_savedata::SaveContainer::parse(&data) {
            let db = c.extract_db().unwrap();
            let start = data.windows(4).position(|w| w == &db[..4]).unwrap_or(0);
            let end = start + db.len();
            let tail = &data[end.min(data.len())..];
            let head: Vec<String> = tail.iter().take(24).map(|b| format!("{b:02x}")).collect();
            let zeros = tail.iter().filter(|b| **b == 0).count();
            println!("   TDB {start:#x}..{end:#x}; trailer {} bytes ({zeros} zero), head {}", tail.len(), head.join(" "));
        }
    }
}
