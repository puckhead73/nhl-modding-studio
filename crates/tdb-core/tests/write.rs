//! Write-path golden tests against the anchors.
//!
//! The headline gate: parsing then re-serializing any vanilla database (no edits) reproduces the
//! original bytes exactly — proving the four CRC scopes and their byte ranges. Then a real edit is
//! applied and re-parsed to confirm the result is valid and the change took effect.

use std::path::{Path, PathBuf};
use tdb_core::TdbFile;

fn anchors_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors")
}

fn read_anchor(rel: &str) -> Vec<u8> {
    std::fs::read(anchors_root().join(rel)).unwrap()
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

/// No-op round-trip: `parse -> serialize` is byte-identical for every vanilla anchor DB (BE and LE).
#[test]
fn round_trip_is_byte_identical() {
    let mut dbs = Vec::new();
    collect_dbs(&anchors_root(), &mut dbs);
    assert!(dbs.len() > 50);

    for path in &dbs {
        let bytes = std::fs::read(path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let out = db.serialize();
        assert!(out == bytes, "round-trip differs for {}", path.display());
    }
    eprintln!(
        "round-trip verified byte-identical for {} databases",
        dbs.len()
    );
}

/// Edit a field, recompute CRCs, re-parse: the value changes, the file stays valid, neighbors intact.
#[test]
fn single_field_edit_is_valid() {
    let bytes = read_anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();

    // ubPc (player bios): edit jersey-style fields are elsewhere, so use position (vjla, 3-bit).
    let t = db.table_canonical("ubPc").unwrap();
    let pos = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "vjla")
        .unwrap();
    // A second field to confirm it is left untouched.
    let other = t
        .fields
        .iter()
        .find(|f| f.id.0 != pos.id.0)
        .unwrap()
        .clone();

    let rec = 0u32;
    let original_pos = db.read_u64(t, rec, pos).unwrap();
    let original_other = db.read_u64(t, rec, &other).unwrap();
    let new_pos = (original_pos + 1) & 0x7; // stays within the 3-bit width

    let mut buf = bytes.clone();
    assert!(db.set_u64(&mut buf, t, rec, pos, new_pos));
    db.recompute_crcs(&mut buf);

    // Re-parse the edited bytes: must be a valid TDB with the new value and CRCs intact.
    let db2 = TdbFile::parse(&buf).unwrap();
    assert!(db2.header_crc_ok(), "edited file header CRC must verify");
    assert!(
        db2.serialize() == buf,
        "edited file must already be self-consistent"
    );

    let t2 = db2.table_canonical("ubPc").unwrap();
    let pos2 = t2
        .fields
        .iter()
        .find(|f| f.id.canonical(db2.endian) == "vjla")
        .unwrap();
    let other2 = t2.fields.iter().find(|f| f.id.0 == other.id.0).unwrap();
    assert_eq!(
        db2.read_u64(t2, rec, pos2).unwrap(),
        new_pos,
        "edit took effect"
    );
    assert_eq!(
        db2.read_u64(t2, rec, other2).unwrap(),
        original_other,
        "neighbor field unchanged"
    );

    // And the edit is a real change (guard against a no-op masking the test).
    assert_ne!(new_pos, original_pos);
}
