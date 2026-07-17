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

/// Bulk allocation growth: `grow_table_max` adds zeroed slack (population unchanged), preserves
/// every record in this and later tables through the directory shift, keeps CRCs self-consistent,
/// and is idempotent at/below the current allocation.
#[test]
fn grow_table_max_preallocates_slack() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table("cPbu").unwrap();
    let (old_cur, old_max, reclen) = (t.current_records, t.max_records, t.record_len_bytes);
    assert_eq!(old_cur, old_max, "anchor cPbu expected packed → real growth");

    let cf = num_field(&db, "cPbu");
    let kf = num_field(&db, "ttOk");
    let cbefore = db.read_u64(t, 0, &t.fields[cf]).unwrap();
    let clast = db.read_u64(t, old_cur as u32 - 1, &t.fields[cf]).unwrap();
    let kott = db.table("ttOk").unwrap();
    let kbefore = db.read_u64(kott, 0, &kott.fields[kf]).unwrap();

    let target = old_max + 100;
    let out = db.grow_table_max("cPbu", target).unwrap();
    assert_eq!(out.len(), bytes.len() + 100 * reclen as usize, "file grew by the added slack");

    let db2 = TdbFile::parse(&out).unwrap();
    assert!(db2.size_ok(), "db_size updated");
    assert!(db2.header_crc_ok(), "header CRC valid");
    assert_eq!(db2.serialize(), out, "all CRC scopes self-consistent");

    let t2 = db2.table("cPbu").unwrap();
    assert_eq!(t2.max_records, target, "allocation grew to the target");
    assert_eq!(t2.current_records, old_cur, "population untouched");
    assert_eq!(db2.read_u64(t2, 0, &t2.fields[cf]).unwrap(), cbefore, "first record preserved");
    assert_eq!(
        db2.read_u64(t2, old_cur as u32 - 1, &t2.fields[cf]).unwrap(),
        clast,
        "last populated record preserved"
    );

    let kott2 = db2.table("ttOk").unwrap();
    assert_eq!(
        db2.read_u64(kott2, 0, &kott2.fields[kf]).unwrap(),
        kbefore,
        "other table's data preserved through the shift"
    );

    // The slack is consumable: an append now reuses a slot without resizing the file.
    let (out3, idx) = db2.append_record("cPbu").unwrap();
    assert_eq!(out3.len(), out.len(), "append into slack does not resize");
    assert_eq!(idx, old_cur as u32);

    // Idempotence: growing to (or below) the current allocation returns the bytes unchanged.
    assert_eq!(db2.grow_table_max("cPbu", target).unwrap(), out);
    assert_eq!(db2.grow_table_max("cPbu", old_max).unwrap(), out);
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

/// Raw bytes of record `i` in `table` (records are `record_len_bytes` apart from `record_block`).
fn rec<'a>(buf: &'a [u8], db: &TdbFile, table_raw: &str, i: u32) -> &'a [u8] {
    let t = db.table(table_raw).unwrap();
    let rl = t.record_len_bytes as usize;
    let s = t.record_block + i as usize * rl;
    &buf[s..s + rl]
}

/// Inserting into a full table grows the file by one record, opens a zeroed hole **at the requested
/// index**, shifts every following record down by one (order preserved — no far row dropped in),
/// preserves other tables, and leaves CRCs self-consistent.
#[test]
fn insert_grows_and_shifts_tail_down() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table("cPbu").unwrap();
    let (old_cur, old_max, reclen) = (t.current_records, t.max_records, t.record_len_bytes);
    assert_eq!(old_cur, old_max, "anchor cPbu expected full → growth path");
    let last = old_cur as u32 - 1;

    let at = 5u32;
    let before_at = rec(&bytes, &db, "cPbu", at).to_vec();
    let before_last = rec(&bytes, &db, "cPbu", last).to_vec();
    let kbefore = rec(&bytes, &db, "ttOk", 0).to_vec();

    let out = db.insert_record("cPbu", at).unwrap();
    assert_eq!(out.len(), bytes.len() + reclen as usize, "file grew by one record");

    let db2 = TdbFile::parse(&out).unwrap();
    assert_eq!(db2.serialize(), out, "all CRC scopes self-consistent");
    assert!(db2.header_crc_ok() && db2.size_ok());
    let t2 = db2.table("cPbu").unwrap();
    assert_eq!(t2.current_records, old_cur + 1);
    assert_eq!(t2.max_records, old_max + 1);

    assert!(rec(&out, &db2, "cPbu", at).iter().all(|&b| b == 0), "hole zeroed at index");
    assert_eq!(rec(&out, &db2, "cPbu", at + 1), &before_at[..], "old record[at] shifted down one");
    assert_eq!(rec(&out, &db2, "cPbu", last + 1), &before_last[..], "old tail shifted down one");
    assert_eq!(rec(&out, &db2, "ttOk", 0), &kbefore[..], "other table preserved through the shift");
}

/// Inserting at `current_records` is exactly an append (hole at the end, empty shift) — byte-identical
/// to `append_record` on the same table.
#[test]
fn insert_at_end_equals_append() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let cur = db.table("cPbu").unwrap().current_records as u32;
    let (appended, idx) = db.append_record("cPbu").unwrap();
    assert_eq!(idx, cur);
    let inserted = db.insert_record("cPbu", cur).unwrap();
    assert_eq!(inserted, appended, "insert at current == append");
}

/// Insert reuses slack (no growth) when the table isn't full.
#[test]
fn insert_reuses_slack_when_not_full() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let (after_remove, _) = db.remove_record("cPbu", 5).unwrap();
    let db_r = TdbFile::parse(&after_remove).unwrap();
    let (cur, max) = {
        let t = db_r.table("cPbu").unwrap();
        (t.current_records, t.max_records)
    };
    assert!(cur < max, "slack exists after removal");

    let out = db_r.insert_record("cPbu", 3).unwrap();
    assert_eq!(out.len(), after_remove.len(), "insert reuses slack — no growth");
    let db2 = TdbFile::parse(&out).unwrap();
    let t2 = db2.table("cPbu").unwrap();
    assert_eq!(t2.current_records, cur + 1);
    assert_eq!(t2.max_records, max, "max unchanged on slack reuse");
    assert_eq!(db2.serialize(), out, "CRCs self-consistent");
}

/// Ordered removal slides the following records up into the hole (order preserved), zeroes the tail,
/// decrements `current_records`, keeps `max_records` (slack), and does not resize — the opposite of
/// swap-pop's far-row relocation.
#[test]
fn remove_ordered_slides_tail_up() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table("cPbu").unwrap();
    let (cur, max) = (t.current_records as u32, t.max_records);
    let last = cur - 1;

    let at = 5u32;
    let before_after = rec(&bytes, &db, "cPbu", at + 1).to_vec();
    let before_last = rec(&bytes, &db, "cPbu", last).to_vec();

    let out = db.remove_record_ordered("cPbu", at).unwrap();
    assert_eq!(out.len(), bytes.len(), "ordered remove does not resize");

    let db2 = TdbFile::parse(&out).unwrap();
    assert_eq!(db2.serialize(), out, "CRCs self-consistent");
    let t2 = db2.table("cPbu").unwrap();
    assert_eq!(t2.current_records as u32, cur - 1);
    assert_eq!(t2.max_records, max, "max unchanged → slack");
    assert_eq!(rec(&out, &db2, "cPbu", at), &before_after[..], "successor slid up into the hole");
    assert_eq!(rec(&out, &db2, "cPbu", last - 1), &before_last[..], "old tail now one slot lower");
    assert!(rec(&out, &db2, "cPbu", last).iter().all(|&b| b == 0), "vacated tail slot zeroed");
}

/// Insert-at-index then ordered-remove-at-the-same-index round-trips the record contents (the two are
/// inverse shifts) — every surviving record returns to its original bytes.
#[test]
fn insert_then_ordered_remove_is_identity_on_data() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let cur = db.table("cPbu").unwrap().current_records as u32;
    let snapshot: Vec<Vec<u8>> = (0..cur).map(|i| rec(&bytes, &db, "cPbu", i).to_vec()).collect();

    let inserted = db.insert_record("cPbu", 7).unwrap();
    let db_i = TdbFile::parse(&inserted).unwrap();
    let restored = db_i.remove_record_ordered("cPbu", 7).unwrap();
    let db_r = TdbFile::parse(&restored).unwrap();

    assert_eq!(db_r.table("cPbu").unwrap().current_records as u32, cur, "count restored");
    for i in 0..cur {
        assert_eq!(rec(&restored, &db_r, "cPbu", i), &snapshot[i as usize][..], "record {i} restored");
    }
}

/// `copy_record` duplicates a whole record block byte-for-byte, leaves the source intact, keeps CRCs
/// consistent (after a recompute), and rejects out-of-range indices.
#[test]
fn copy_record_duplicates_a_record_block() {
    let bytes = anchor();
    let db = TdbFile::parse(&bytes).unwrap();
    let before0 = rec(&bytes, &db, "cPbu", 0).to_vec();
    let t = db.table("cPbu").unwrap();

    let mut buf = bytes.clone();
    assert!(db.copy_record(&mut buf, t, 0, 3));
    db.recompute_crcs(&mut buf);

    let db2 = TdbFile::parse(&buf).unwrap();
    assert_eq!(db2.serialize(), buf, "CRCs self-consistent");
    assert_eq!(rec(&buf, &db2, "cPbu", 3), &before0[..], "record 3 is now a copy of record 0");
    assert_eq!(rec(&buf, &db2, "cPbu", 0), &before0[..], "source record unchanged");
    assert!(!db.copy_record(&mut buf, t, 9_999_999, 3), "out-of-range source rejected");
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
    // insert accepts index == current (append) but rejects beyond it; ordered-remove rejects OOB.
    let cur = db.table("cPbu").unwrap().current_records as u32;
    assert_eq!(db.insert_record("zzzz", 0).unwrap_err(), AllocError::UnknownTable);
    assert_eq!(
        db.insert_record("cPbu", cur + 1).unwrap_err(),
        AllocError::IndexOutOfRange
    );
    assert!(db.insert_record("cPbu", cur).is_ok(), "index == current is append");
    assert_eq!(
        db.remove_record_ordered("cPbu", cur).unwrap_err(),
        AllocError::IndexOutOfRange
    );
}
