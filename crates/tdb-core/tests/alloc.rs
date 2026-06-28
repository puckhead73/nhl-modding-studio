//! Record-allocation primitives verified against the committed PS3 NHL Legacy anchor.

use std::path::Path;
use tdb_core::{AllocError, TdbFile};

fn anchor() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors/ps3/nhllegacy/databases/nhlng.db"),
    )
    .unwrap()
}

/// Index of the first fixed-bit (`<= 64`-wide) field on a table — safe to `read_u64`.
fn num_field(db: &TdbFile, table_raw: &str) -> usize {
    let t = db.table(table_raw).unwrap();
    t.fields
        .iter()
        .position(|f| f.bit_width > 0 && f.bit_width <= 64)
        .unwrap()
}

/// Appending to a full table grows the file by exactly one record, preserves every existing record
/// (in this and other tables, through the directory shift), zero-fills the new slot, and leaves all
/// CRCs self-consistent.
#[test]
fn append_grows_full_table_and_preserves_data() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table("cPbu").unwrap();
    let (old_cur, old_max, reclen) = (t.current_records, t.max_records, t.record_len_bytes);
    assert_eq!(old_cur, old_max, "anchor cPbu expected full → growth path");

    let cf = num_field(&db, "cPbu");
    let kf = num_field(&db, "ttOk");
    let cbefore = db.read_u64(t, 0, &t.fields[cf]).unwrap();
    let kott = db.table("ttOk").unwrap();
    let kbefore = db.read_u64(kott, 0, &kott.fields[kf]).unwrap();

    let (out, idx) = db.append_record("cPbu").unwrap();
    assert_eq!(idx, old_cur as u32);
    assert_eq!(out.len(), bytes.len() + reclen as usize, "file grew by one record");

    let db2 = TdbFile::parse(&out).unwrap();
    assert!(db2.size_ok(), "db_size updated");
    assert!(db2.header_crc_ok(), "header CRC valid");
    assert_eq!(db2.serialize(), out, "all CRC scopes self-consistent");

    let t2 = db2.table("cPbu").unwrap();
    assert_eq!(t2.current_records, old_cur + 1);
    assert_eq!(t2.max_records, old_max + 1);
    assert_eq!(db2.read_u64(t2, 0, &t2.fields[cf]).unwrap(), cbefore, "existing record preserved");
    assert_eq!(db2.read_u64(t2, idx, &t2.fields[cf]).unwrap(), 0, "new record zero-filled");

    let kott2 = db2.table("ttOk").unwrap();
    assert_eq!(
        db2.read_u64(kott2, 0, &kott2.fields[kf]).unwrap(),
        kbefore,
        "other table's data preserved through the shift"
    );
}

/// Swap-pop relocates the tail record into the removed slot, decrements `current_records`, keeps
/// `max_records` (leaving reusable slack), does not resize, and reports the moved index.
#[test]
fn remove_swap_pop_relocates_tail() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table("cPbu").unwrap();
    let cf = num_field(&db, "cPbu");
    let cur = t.current_records as u32;
    let last = cur - 1;
    let tail_val = db.read_u64(t, last, &t.fields[cf]).unwrap();

    let remove_idx = 5u32;
    let (out, moved) = db.remove_record("cPbu", remove_idx).unwrap();
    assert_eq!(moved, Some(last), "non-last removal relocates the tail");
    assert_eq!(out.len(), bytes.len(), "swap-pop does not resize");

    let db2 = TdbFile::parse(&out).unwrap();
    assert_eq!(db2.serialize(), out, "CRCs self-consistent");
    let t2 = db2.table("cPbu").unwrap();
    assert_eq!(t2.current_records as u32, cur - 1);
    assert_eq!(t2.max_records, t.max_records, "max unchanged → slack created");
    assert_eq!(
        db2.read_u64(t2, remove_idx, &t2.fields[cf]).unwrap(),
        tail_val,
        "old tail row now lives at the removed index"
    );
}

/// Removing the last record needs no relocation.
#[test]
fn remove_last_record_does_not_relocate() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let last = db.table("cPbu").unwrap().current_records as u32 - 1;
    let (_out, moved) = db.remove_record("cPbu", last).unwrap();
    assert_eq!(moved, None);
}

/// After a removal frees a slot, the next append reuses it — no growth, no resize.
#[test]
fn append_after_remove_reuses_slack() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let (after_remove, _) = db.remove_record("cPbu", 5).unwrap();
    let removed_len = after_remove.len();

    let db_r = TdbFile::parse(&after_remove).unwrap();
    let t = db_r.table("cPbu").unwrap();
    assert!(t.current_records < t.max_records, "slack exists after removal");
    let (cur, max) = (t.current_records, t.max_records);

    let (after_append, _idx) = db_r.append_record("cPbu").unwrap();
    assert_eq!(after_append.len(), removed_len, "append reuses slack — no growth");
    let db_a = TdbFile::parse(&after_append).unwrap();
    let t_a = db_a.table("cPbu").unwrap();
    assert_eq!(t_a.current_records, cur + 1);
    assert_eq!(t_a.max_records, max, "max unchanged on slack reuse");
}

#[test]
fn allocation_errors() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    assert_eq!(db.append_record("zzzz").unwrap_err(), AllocError::UnknownTable);
    assert_eq!(
        db.remove_record("cPbu", 9_999_999).unwrap_err(),
        AllocError::IndexOutOfRange
    );
}
