//! Edit-session tests against an anchor database.

use std::path::Path;
use tdb_core::TdbFile;
use tdb_edit::EditSession;

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

#[test]
fn empty_session_is_byte_identical() {
    let bytes = anchor("ps3/nhl14/databases/nhlng.db");
    let out = EditSession::new().apply(&bytes).unwrap();
    assert!(out == bytes, "no-op session must reproduce the original");
}

#[test]
fn staged_edit_applies_and_verifies() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();

    // ubPc -> raw cPbu (BE reversal); position field vjla -> raw aljv, 3-bit.
    let t = db.table_canonical("ubPc").unwrap();
    let raw_table = t.id.raw();
    let pos = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "vjla")
        .unwrap();
    let raw_field = pos.id.raw();
    let original = db.read_u64(t, 0, pos).unwrap();
    let new_value = (original + 1) & 0x7;

    let mut session = EditSession::new();
    session.set_field(&raw_table, 0, &raw_field, new_value);
    let out = session.apply(&bytes).unwrap();

    // Edited bytes parse, the CRC verifies, and the value changed.
    let db2 = TdbFile::parse(&out).unwrap();
    assert!(db2.header_crc_ok());
    let t2 = db2.table(&raw_table).unwrap();
    let pos2 = t2.field(&raw_field).unwrap();
    assert_eq!(db2.read_u64(t2, 0, pos2).unwrap(), new_value);
    assert_ne!(new_value, original);
}

#[test]
fn unknown_table_is_an_error() {
    let bytes = anchor("ps3/nhl14/databases/nhlng.db");
    let mut s = EditSession::new();
    s.set_field("zzzz", 0, "yyyy", 1);
    assert!(s.apply(&bytes).is_err());
}
