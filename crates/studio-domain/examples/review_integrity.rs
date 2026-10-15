//! Read-only diagnosis of bytes outside the overview's recognized record spans.
use tdb_core::TdbFile;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = std::env::args().skip(1).collect();
    if paths.len() != 2 && paths.len() != 4 { return Err("usage: review_integrity <baseline.db> <working.db> [meta.xml defs-directory]".into()); }
    let base = std::fs::read(&paths[0])?;
    let work = std::fs::read(&paths[1])?;
    let b = TdbFile::parse(&base)?;
    let w = TdbFile::parse(&work)?;
    println!("Lengths: {} -> {}", base.len(), work.len());
    for table in &w.tables {
        let id = table.id.canonical(w.endian);
        if let Some(old) = b.table_canonical(&id) {
            if old.max_records != table.max_records || old.current_records != table.current_records {
                println!("{id}: records {} -> {}, capacity {} -> {}", old.current_records, table.current_records, old.max_records, table.max_records);
            }
        }
    }
    let first = w.tables.iter().map(|t| t.block_offset).min().unwrap_or(0);
    let spans: Vec<_> = w.tables.iter().map(|t| (t.block_offset, t.record_block + t.record_len_bytes as usize * t.max_records as usize)).collect();
    let offsets: Vec<_> = (first..base.len().max(work.len())).filter(|&i| !spans.iter().any(|&(s,e)| s <= i && i < e) && base.get(i) != work.get(i)).collect();
    println!("Uncovered differences: {}", offsets.len());
    for &i in offsets.iter().take(32) { println!("0x{i:X}: {:?} -> {:?}", base.get(i), work.get(i)); }
    for (name, bytes, db) in [("baseline", &base, &b), ("working", &work, &w)] {
        let mut repaired = bytes.to_vec(); db.recompute_crcs(&mut repaired);
        println!("{name}: checksum bytes already valid = {}", repaired == *bytes);
    }
    if paths.len() == 4 {
        let project = studio_domain::Project::load(&paths[1], studio_domain::Platform::Xbox,
            studio_domain::ProjectRole::Nhlng, &paths[2], &paths[3])?;
        let review = studio_domain::pending::pending_changes(&base, &work, project.meta(), project.enums(), None, 1000);
        println!("Overview unrecognized bytes: {}", review.integrity.other_bytes);
    }
    Ok(())
}
