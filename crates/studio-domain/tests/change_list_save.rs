//! Save Editor over NHL 07 / NHL 08 roster change lists: unedited saves round-trip byte for byte,
//! and the general change-list writer reproduces the game-returned NHL 07 saves exactly for the
//! same edits (ROSTER2 + jersey 38 = ROSTER3, ROSTER3 + speed 49 = ROSTER4).
use std::path::{Path, PathBuf};
use std::sync::Arc;

use studio_domain::change_list_overlay::{compose, pack_container};
use studio_domain::roster_difflist::recognize;
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

fn anchors() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors")
}

fn target(game: &str) -> (Arc<Vec<u8>>, FieldMetaIndex) {
    let (db, meta) = match game {
        "nhl07" => ("xbox360/nhl07/databases/NHL07NG.db", "xbox360/nhl07/databases/NHL07NG-meta.xml"),
        _ => ("xbox360/nhl08/databases/nhlng.db", "xbox360/nhl08/databases/nhlng-meta.xml"),
    };
    let base = std::fs::read(anchors().join(db)).unwrap();
    let xml = std::fs::read_to_string(anchors().join(meta)).unwrap();
    (Arc::new(base), FieldMetaIndex::from_xml(&xml).unwrap())
}

fn inner(container: &[u8]) -> Vec<u8> {
    tdb_savedata::SaveContainer::parse(container)
        .unwrap()
        .extract_db()
        .unwrap()
}

// Game-written NHL 07 saves captured 2026-10-05 (see tests/nhl07_difflist.rs).
fn game_save(slot: u8) -> Vec<u8> {
    let prefix: &[u8] = match slot {
        1 => include_bytes!("fixtures/nhl07/ROSTER1-prefix.bin"),
        2 => include_bytes!("fixtures/nhl07/ROSTER2-prefix.bin"),
        3 => include_bytes!("fixtures/nhl07/ROSTER3-prefix.bin"),
        4 => include_bytes!("fixtures/nhl07/ROSTER4-prefix.bin"),
        _ => unreachable!(),
    };
    let mut bytes = prefix.to_vec();
    bytes.resize(1_598_888, 0);
    bytes
}

/// Set `field` of the row of parent `table` whose change-list key is `key`, in the composite.
fn edit(composite: &mut Vec<u8>, meta: &FieldMetaIndex, base: &[u8], save: &[u8], table: &str, key: u64, field: &str, value: u64) {
    let schema = recognize(base, save, meta).unwrap();
    let key_field = &schema.tables.iter().find(|m| m.parent_table == table).unwrap().key;
    let view = composite.clone();
    let db = TdbFile::parse(&view).unwrap();
    let t = db.table(table).unwrap();
    let k = t.field(key_field).unwrap();
    let f = t.field(field).unwrap();
    let i = (0..t.current_records as u32)
        .find(|&i| db.read_u64(t, i, k) == Some(key))
        .expect("row with that key");
    let start = t.record_block + i as usize * t.record_len_bytes as usize;
    let row = &mut composite[start..start + t.record_len_bytes as usize];
    tdb_core::write_bits(row, f.bit_offset, f.bit_width, value, db.endian);
    db.recompute_crcs(composite);
}

#[test]
fn unedited_change_lists_round_trip_byte_for_byte() {
    let cases = [
        ("nhl07", anchors().join("xbox360/nhl07/savefiles/ROSTER1/ROSTER1")),
        ("nhl08", anchors().join("xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658")),
        ("nhl08", anchors().join("ps3/nhl08/savefiles/BLUS30062-002-000 - ROSTER/SYS-DATA")),
    ];
    for (game, path) in cases {
        let (base, meta) = target(game);
        let container = std::fs::read(&path).unwrap();
        let save = inner(&container);
        let (composite, overlay) = compose(&save, base.clone(), &meta).unwrap();
        // An empty change list shows the game database unchanged.
        assert_eq!(composite, *base, "{}", path.display());
        let list = overlay.strip(&composite, &meta).unwrap();
        assert_eq!(list, save);
        assert_eq!(pack_container(&container, &list).unwrap(), container);
        assert_eq!(overlay.recompose(&list, &meta).unwrap(), composite);
        assert!(overlay.editable_tables() > 10, "{game}: {}", overlay.editable_tables());
    }
    for slot in 1..=4 {
        let (base, meta) = target("nhl07");
        let container = game_save(slot);
        let save = inner(&container);
        let (composite, overlay) = compose(&save, base, &meta).unwrap();
        let list = overlay.strip(&composite, &meta).unwrap();
        assert_eq!(pack_container(&container, &list).unwrap(), container, "ROSTER{slot}");
    }
}

#[test]
fn general_writer_reproduces_the_game_returned_nhl07_saves() {
    let (base, meta) = target("nhl07");
    // ROSTER2 + Demitra (roster row key 587) jersey 38 = ROSTER3, which NHL 07 loaded and resaved.
    // ROSTER3 + skater 613 raw speed 49 = ROSTER4, which NHL 07 displayed and resaved.
    for (from, to, table, key, field, value) in [
        (2, 3, "ulGe", 587, "tRVs", 38),
        (3, 4, "yvSd", 613, "YWPq", 49),
    ] {
        let container = game_save(from);
        let save = inner(&container);
        let (mut composite, overlay) = compose(&save, base.clone(), &meta).unwrap();
        edit(&mut composite, &meta, &base, &save, table, key, field, value);
        let list = overlay.strip(&composite, &meta).unwrap();
        let written = pack_container(&container, &list).unwrap();
        assert_eq!(written, game_save(to), "ROSTER{from} -> ROSTER{to}");
        // The saved file re-reads as the session's exact composite.
        assert_eq!(overlay.recompose(&list, &meta).unwrap(), composite);
    }
}

#[test]
fn reverting_an_edit_restores_the_original_change_list() {
    let (base, meta) = target("nhl07");
    let container = game_save(3);
    let save = inner(&container);
    let (mut composite, overlay) = compose(&save, base.clone(), &meta).unwrap();
    let before = composite.clone();
    edit(&mut composite, &meta, &base, &save, "yvSd", 613, "YWPq", 49);
    assert_ne!(overlay.change_list_for(&composite, &meta).unwrap(), save);
    // Back to the value the file holds: nothing changed, so the file's rows are kept as they are.
    assert_eq!(overlay.change_list_for(&before, &meta).unwrap(), save);
}

#[test]
fn fields_a_change_list_cannot_record_are_frozen_and_refused() {
    let (base, meta) = target("nhl07");
    let container = std::fs::read(anchors().join("xbox360/nhl07/savefiles/ROSTER1/ROSTER1")).unwrap();
    let save = inner(&container);
    let (composite, overlay) = compose(&save, base.clone(), &meta).unwrap();
    // The player bio table records changes only through its update companion.
    let db = TdbFile::parse(&composite).unwrap();
    let bio = db.table("cPbu").unwrap();
    let schema = recognize(&base, &save, &meta).unwrap();
    let update = schema
        .tables
        .iter()
        .find(|m| m.parent_table == "cPbu")
        .unwrap();
    let s = TdbFile::parse(&save).unwrap();
    let carried: Vec<String> = s.table(&update.save_table).unwrap().fields.iter().map(|f| f.id.raw()).collect();
    let fixed = bio
        .fields
        .iter()
        .map(|f| f.id.raw())
        .find(|f| !carried.contains(f) && *f != update.key)
        .expect("a bio field the save cannot record");
    assert!(overlay.frozen().contains(&format!("cPbu.{fixed}")));
    assert!(!overlay.frozen().contains("cPbu"));
    assert!(overlay.frozen().contains("UzqH"), "stock team table is read-only");
    let key = db.read_u64(bio, 0, bio.field(&update.key).unwrap()).unwrap();
    let width = bio.field(&fixed).unwrap().bit_width.min(8);
    let row = &composite[bio.record_block..bio.record_block + bio.record_len_bytes as usize];
    let f = bio.field(&fixed).unwrap();
    let old = tdb_core::read_bits(row, f.bit_offset, width, db.endian);
    let mut edited = composite.clone();
    edit(&mut edited, &meta, &base, &save, "cPbu", key, &fixed, old ^ 1);
    let err = overlay.change_list_for(&edited, &meta).unwrap_err().to_string();
    assert!(err.contains("cannot record a change"), "{err}");
}

#[test]
fn a_full_change_list_is_refused_rather_than_overflowing() {
    // Insert rows past an insert companion's declared capacity: refused, never written.
    let (base, meta) = target("nhl07");
    let container = std::fs::read(anchors().join("xbox360/nhl07/savefiles/ROSTER1/ROSTER1")).unwrap();
    let save = inner(&container);
    let (composite, overlay) = compose(&save, base.clone(), &meta).unwrap();
    let schema = recognize(&base, &save, &meta).unwrap();
    let db = TdbFile::parse(&composite).unwrap();
    let mapping = schema
        .tables
        .iter()
        .filter(|m| m.operation == studio_domain::roster_difflist::Operation::Insert && m.capacity > 0)
        .filter(|m| db.table(&m.parent_table).unwrap().field(&m.key).unwrap().bit_width >= 12)
        .min_by_key(|m| m.capacity)
        .expect("an insert companion with a wide key");
    let parent = mapping.parent_table.clone();
    let t = db.table(&parent).unwrap();
    let key = t.field(&mapping.key).unwrap().clone();
    let top = (0..t.current_records as u32)
        .filter_map(|i| db.read_u64(t, i, &key))
        .max()
        .unwrap_or(0);
    let mut grown = composite.clone();
    for n in 1..=u64::from(mapping.capacity) + 1 {
        let (bytes, i) = TdbFile::parse(&grown).unwrap().append_record(&parent).unwrap();
        grown = bytes;
        let view = grown.clone();
        let db = TdbFile::parse(&view).unwrap();
        let t = db.table(&parent).unwrap();
        let start = t.record_block + i as usize * t.record_len_bytes as usize;
        tdb_core::write_bits(&mut grown[start..start + t.record_len_bytes as usize], key.bit_offset, key.bit_width, top + n, db.endian);
        db.recompute_crcs(&mut grown);
    }
    let err = overlay.change_list_for(&grown, &meta).unwrap_err().to_string();
    assert!(err.contains(&format!("has room for {}", mapping.capacity)), "{err}");
}
