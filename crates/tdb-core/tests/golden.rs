//! Golden tests against the committed anchors (the verified vanilla corpus).
//!
//! These port the Python audit verifications into the engine: every real database parses, its
//! header CRC and size verify, and known fields decode to sane values on both byte orders.

use std::path::{Path, PathBuf};
use tdb_core::{Endian, TdbFile};

fn anchors_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors")
}

fn read_anchor(rel: &str) -> Vec<u8> {
    let p = anchors_root().join(rel);
    std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn collect_dbs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_dbs(&p, out);
        } else if p.extension().and_then(|e| e.to_str()) == Some("db") {
            out.push(p);
        }
    }
}

/// Every real database under `anchors/` parses, and its header CRC + size verify (BE and LE).
#[test]
fn all_databases_crc_and_size() {
    let mut dbs = Vec::new();
    collect_dbs(&anchors_root(), &mut dbs);
    assert!(
        dbs.len() > 50,
        "expected many anchor DBs, found {}",
        dbs.len()
    );

    let mut checked = 0usize;
    for path in &dbs {
        let bytes = std::fs::read(path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        assert!(db.size_ok(), "db_size mismatch in {}", path.display());
        assert!(
            db.header_crc_ok(),
            "header CRC mismatch in {}",
            path.display()
        );
        checked += 1;
    }
    eprintln!("verified {checked} anchor databases (CRC + size)");
}

#[test]
fn ps3_nhl14_structure() {
    let bytes = read_anchor("ps3/nhl14/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    assert_eq!(db.endian, Endian::Big);
    assert_eq!(db.tables.len(), 134);
    assert!(db.header_crc_ok() && db.size_ok());
    // The big-endian reversal: on-disk raw `cPbu` is canonical `ubPc` (player bios).
    let ubpc = db.table_canonical("ubPc").expect("ubPc table");
    assert_eq!(ubpc.id.raw(), "cPbu");
}

#[test]
fn ps4_nhl19_structure() {
    let bytes = read_anchor("ps4/nhl19/databases/nhl_game.db");
    let db = TdbFile::parse(&bytes).unwrap();
    assert_eq!(db.endian, Endian::Little);
    assert_eq!(db.tables.len(), 111);
    assert!(db.header_crc_ok() && db.size_ok());
}

#[test]
fn xbox360_equals_ps3() {
    // Xbox360 databases are byte-identical to PS3 — same parse result.
    let ps3 = read_anchor("ps3/nhl14/databases/nhlng.db");
    let xbx = read_anchor("xbox360/nhl14/databases/nhlng.db");
    assert_eq!(ps3, xbx, "xbox360 nhl14 nhlng.db should equal ps3");
}

/// Decode a known enum field on both byte orders; values must stay in range.
#[test]
fn position_decodes_in_range() {
    // PS3 (BE): nhllegacy ubPc.vjla (position) — verified distribution was 0..=4.
    let bytes = read_anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table_canonical("ubPc").unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "vjla")
        .expect("position field");
    let mut seen_goalie = false;
    for i in 0..t.current_records as u32 {
        let v = db.read_u64(t, i, f).unwrap();
        assert!(v <= 4, "PS3 position out of range: {v}");
        seen_goalie |= v == 4;
    }
    assert!(seen_goalie, "expected at least one goalie (position 4)");

    // PS4 (LE): nhl19 nhl_game KdqM.vjla — verified values were 0..=4.
    let bytes = read_anchor("ps4/nhl19/databases/nhl_game.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table("KdqM").unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.raw() == "vjla")
        .expect("position field");
    for i in 0..t.current_records as u32 {
        assert!(
            db.read_u64(t, i, f).unwrap() <= 4,
            "PS4 position out of range"
        );
    }
}
