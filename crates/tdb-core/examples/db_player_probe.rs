//! `db_player_probe` — read-only lookup of one player's record in a TDB `nhl_game.db`,
//! by name, reporting every field so an asset binding can be read off evidence rather
//! than assumed.
//!
//! ```text
//! cargo run -p tdb-core --example db_player_probe -- <nhl_game.db> <name-substring>
//! ```
//!
//! Prints the player-bio (`ubPc`) records whose inline string fields contain the
//! substring (case-insensitive), each field by raw id with its decoded value, then
//! reports how many other records in the table share each of that player's numeric
//! field values — the cheap test for "is this id bespoke to him or pooled?".
//!
//! Diagnostic only: it writes nothing and asserts no field semantics. Raw ids are
//! printed verbatim precisely because this probe is where their meaning gets
//! established, not consumed.

use tdb_core::{read_bits, TdbFile};

/// EA's player-bio table. Its raw (on-disk) id; the canonical form is the reverse.
const PLAYER_TABLE: &str = "ubPc";

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(needle)) = (args.next(), args.next()) else {
        eprintln!("usage: db_player_probe <nhl_game.db> <name-substring>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("read {path}: {e}");
        std::process::exit(1);
    });
    let db = TdbFile::parse(&bytes).unwrap_or_else(|e| {
        eprintln!("parse {path}: {e}");
        std::process::exit(1);
    });
    println!(
        "# {path}: endian={:?} header_crc_ok={} size_ok={}",
        db.endian,
        db.header_crc_ok(),
        db.size_ok()
    );

    let Some(table) = db
        .table(PLAYER_TABLE)
        .or_else(|| db.table_canonical(PLAYER_TABLE))
    else {
        eprintln!("# no {PLAYER_TABLE} table in this database");
        std::process::exit(1);
    };
    println!(
        "# {PLAYER_TABLE}: records={}/{} record_len={}B fields={}",
        table.current_records,
        table.max_records,
        table.record_len_bytes,
        table.fields.len()
    );

    let needle_lc = needle.to_ascii_lowercase();
    let mut matches = Vec::new();
    for i in 0..u32::from(table.current_records) {
        let hit = table.fields.iter().any(|f| {
            db.read_inline_str(table, i, f).is_some_and(|s| {
                String::from_utf8_lossy(s)
                    .to_ascii_lowercase()
                    .contains(&needle_lc)
            })
        });
        if hit {
            matches.push(i);
        }
    }
    println!("# {} record(s) match {needle:?}", matches.len());

    for &i in &matches {
        println!("--- record {i}");
        for f in &table.fields {
            let id = f.id.raw();
            if let Some(s) = db.read_inline_str(table, i, f) {
                if !s.is_empty() && s.iter().all(|&b| (0x20..0x7f).contains(&b)) {
                    println!("  {id}\tstr\t{:?}", String::from_utf8_lossy(s));
                    continue;
                }
            }
            let v = db.read_u64(table, i, f).unwrap_or_default();
            // How many *other* records carry this same value? 1 means unique to him.
            let shared = (0..u32::from(table.current_records))
                .filter(|&j| db.read_u64(table, j, f).unwrap_or_default() == v)
                .count();
            println!("  {id}\tu{}\t{v}\tshared_by={shared}", f.bit_width);
        }
    }

    // Silence the unused-import lint when no numeric field is ever read.
    let _ = read_bits;
}
