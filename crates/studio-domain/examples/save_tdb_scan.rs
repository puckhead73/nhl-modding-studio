//! Read-only probe: list every parseable TDB inside a raw (uncompressed) save file — offset, size,
//! table count and whether the core player/team tables are present.
//!
//! `cargo run -p studio-domain --example save_tdb_scan -- <file>...`

use tdb_core::TdbFile;

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).unwrap();
        println!("== {path} ({} bytes)", bytes.len());
        let mut i = 0;
        while i + 24 <= bytes.len() {
            if &bytes[i..i + 2] == b"DB" {
                if let Ok(db) = TdbFile::parse(&bytes[i..]) {
                    let size = db.db_size as usize;
                    let ids: Vec<String> = db.tables.iter().map(|t| t.id.raw()).collect();
                    let rows: u32 = db.tables.iter().map(|t| u32::from(t.current_records)).sum();
                    println!(
                        "   TDB at {i:#x}: {size} bytes, {} tables, {rows} rows, first ids {:?}",
                        db.tables.len(),
                        &ids[..ids.len().min(6)]
                    );
                    i += size.max(1);
                    continue;
                }
            }
            i += 1;
        }
    }
}
