//! Read-only probe: the bytes a PS4 save's decompressed roster record carries after its TDB.
//!
//! `cargo run -p studio-domain --example ps4_trailer -- <DATA>...`

fn main() {
    for path in std::env::args().skip(1) {
        let inner = tdb_savedata::ps4_extract_roster(&std::fs::read(&path).unwrap()).unwrap();
        let db = tdb_core::TdbFile::parse(&inner).unwrap();
        let size = db.db_size as usize;
        let tail = &inner[size.min(inner.len())..];
        let zeros = tail.iter().filter(|b| **b == 0).count();
        let head: Vec<String> = tail.iter().take(16).map(|b| format!("{b:02x}")).collect();
        println!("{path}: inner {} bytes, db_size {size}, trailer {} bytes ({zeros} zero) head {}", inner.len(), tail.len(), head.join(" "));
    }
}
