//! Read-only probe: the bytes a PS4 save's decompressed roster record carries after its TDB.
//!
//! `cargo run -p studio-domain --example ps4_trailer -- <DATA>...`

fn main() {
    for path in std::env::args().skip(1) {
        let inner = match tdb_savedata::ps4_extract_roster(&std::fs::read(&path).unwrap()) {
            Ok(inner) => inner,
            Err(e) => {
                println!("{path}: not a roster record ({e})");
                continue;
            }
        };
        let db = match tdb_core::TdbFile::parse(&inner) {
            Ok(db) => db,
            Err(e) => {
                println!("{path}: payload is not a TDB ({e})");
                continue;
            }
        };
        let size = db.db_size as usize;
        let tail = &inner[size.min(inner.len())..];
        let zeros = tail.iter().filter(|b| **b == 0).count();
        let hex: Vec<String> = tail.iter().take(32).map(|b| format!("{b:02x}")).collect();
        let embedded = tail.get(8..16).map(|b| u64::from_le_bytes(b.try_into().unwrap()));
        println!(
            "{path}: inner {} bytes, db_size {size}, trailer {} bytes ({zeros} zero) u64le@8={embedded:?} bytes {}",
            inner.len(),
            tail.len(),
            hex.join(" ")
        );
    }
}
