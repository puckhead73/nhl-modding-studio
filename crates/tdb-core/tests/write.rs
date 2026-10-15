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

/// Editing an inline string (player firstname) round-trips through `set_inline_str` → re-parse, and
/// the value reads back exactly (NUL-padded remainder), with neighbors untouched.
#[test]
fn inline_string_edit_round_trips() {
    let bytes = read_anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table_canonical("ubPc").unwrap();
    // firstname — on-disk shortname `PedH` (byte-aligned, 200-bit / 25-byte inline string).
    let first = t.fields.iter().find(|f| f.id.raw() == "PedH").unwrap();
    let other = t
        .fields
        .iter()
        .find(|f| f.id.0 != first.id.0)
        .unwrap()
        .clone();

    let rec = 0u32;
    let original_other = db.read_u64(t, rec, &other).unwrap();

    let mut buf = bytes.clone();
    assert!(db.set_inline_str(&mut buf, t, rec, first, "Gretzky"));
    db.recompute_crcs(&mut buf);

    let db2 = TdbFile::parse(&buf).unwrap();
    assert!(db2.header_crc_ok());
    assert!(db2.serialize() == buf, "edited file is self-consistent");

    let t2 = db2.table_canonical("ubPc").unwrap();
    let first2 = t2.fields.iter().find(|f| f.id.raw() == "PedH").unwrap();
    let read_back = db2.read_inline_str(t2, rec, first2).unwrap();
    assert_eq!(
        read_back, b"Gretzky",
        "string edit took effect and reads back exactly"
    );

    let other2 = t2.fields.iter().find(|f| f.id.0 == other.id.0).unwrap();
    assert_eq!(
        db2.read_u64(t2, rec, other2).unwrap(),
        original_other,
        "neighbor unchanged"
    );

    // Too-long strings are refused, not truncated.
    let width_bytes = (first.bit_width / 8) as usize;
    let too_long = "X".repeat(width_bytes + 1);
    assert!(!db.set_inline_str(&mut buf, t, rec, first, &too_long));
}
