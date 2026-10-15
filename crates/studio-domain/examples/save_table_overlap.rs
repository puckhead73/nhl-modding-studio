//! Read-only probe: for each anchor save, compare its database's tables against the same target's
//! bundled main database — tables only in the save, only in the main database, and shared tables
//! whose descriptor (record length, capacity, field layout) differs.
//!
//! `cargo run -p studio-domain --example save_table_overlap -- <anchor-relative save path>...`

use std::path::Path;

use studio_domain::baselines::{BaselineStore, ROLE_MAIN_DB};
use studio_domain::save_identify::{probe_bytes, SchemaIndex};
use tdb_core::TdbFile;

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let store = BaselineStore::open(&repo.join("baselines/INVENTORY.json"), &repo.join("anchors")).unwrap();
    let index = SchemaIndex::build(&store).unwrap();
    for rel in std::env::args().skip(1) {
        let path = repo.join("anchors").join(&rel);
        let id = match index.identify(&path) {
            Ok(id) => id,
            Err(e) => {
                println!("== {rel}: {e}");
                continue;
            }
        };
        let Some(c) = id.candidates.first() else {
            println!("== {rel}: no candidate");
            continue;
        };
        let target = store.target(&c.game_id, c.platform).unwrap();
        let main_bytes = store.read_verified(target.component(ROLE_MAIN_DB).unwrap()).unwrap();
        let main = TdbFile::parse(&main_bytes).unwrap();
        let (_, inner) = probe_bytes(std::fs::read(&path).unwrap()).unwrap();
        let save = TdbFile::parse(&inner).unwrap();
        println!(
            "== {rel}: {} as {}/{} — save {} tables, main {} tables; entity: {:?}",
            id.probe.kind,
            c.game_id,
            c.platform.as_str(),
            save.tables.len(),
            main.tables.len(),
            id.entity_problem
        );
        let mut only_save = Vec::new();
        let mut differ = Vec::new();
        let mut same = 0;
        for t in &save.tables {
            match main.tables.iter().find(|m| m.id.raw() == t.id.raw()) {
                None => only_save.push(format!("{}({} rows)", t.id.raw(), t.current_records)),
                Some(m) => {
                    let fields = |x: &tdb_core::Table| {
                        let mut v: Vec<_> = x.fields.iter().map(|f| (f.id.raw(), f.bit_offset, f.bit_width, f.kind)).collect();
                        v.sort();
                        v
                    };
                    if m.record_len_bytes != t.record_len_bytes || fields(m) != fields(t) {
                        differ.push(format!("{} layout(len {}→{})", t.id.raw(), m.record_len_bytes, t.record_len_bytes));
                    } else if m.max_records != t.max_records {
                        differ.push(format!("{} cap {}→{} rows {}→{}", t.id.raw(), m.max_records, t.max_records, m.current_records, t.current_records));
                    } else {
                        same += 1;
                    }
                }
            }
        }
        let only_main: Vec<String> = main
            .tables
            .iter()
            .filter(|m| !save.tables.iter().any(|t| t.id.raw() == m.id.raw()))
            .map(|m| format!("{}({})", m.id.raw(), m.current_records))
            .collect();
        println!("   shared identical descriptor: {same}");
        println!("   shared, capacity/layout differs ({}): {}", differ.len(), differ.join(", "));
        println!("   only in save ({}): {}", only_save.len(), only_save.join(", "));
        println!("   only in main ({}): {}", only_main.len(), only_main.join(", "));
    }
}
