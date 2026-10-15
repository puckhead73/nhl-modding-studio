//! Read-only real-anchor qualification; synthetic rows stay in memory, never saved.
use studio_domain::roster_difflist::{effective_roster, recognize, Operation};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

fn fixture() -> (Vec<u8>, Vec<u8>, FieldMetaIndex) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors/xbox360/nhl07");
    let base = std::fs::read(root.join("databases/NHL07NG.db")).unwrap();
    let container = std::fs::read(root.join("savefiles/ROSTER1/ROSTER1")).unwrap();
    let save = tdb_savedata::SaveContainer::parse(&container)
        .unwrap()
        .extract_db()
        .unwrap();
    let xml = std::fs::read_to_string(root.join("databases/NHL07NG-meta.xml")).unwrap();
    (base, save, FieldMetaIndex::from_xml(&xml).unwrap())
}

#[test]
fn all_31_companions_are_explicitly_declared_and_empty_overlay_is_identity() {
    let (base, save, meta) = fixture();
    let schema = recognize(&base, &save, &meta).unwrap();
    assert_eq!(schema.tables.len(), 31);
    assert_eq!(
        schema
            .tables
            .iter()
            .filter(|t| t.operation == Operation::Update)
            .count(),
        12
    );
    for (id, parent) in [
        ("DZfz", "dDrd"),
        ("YvhR", "Raym"),
        ("rcpY", "YzeG"),
        ("zrsU", "oDHy"),
        ("RosT", "yvSd"),
        ("uyGF", "yuHm"),
    ] {
        assert_eq!(
            schema
                .tables
                .iter()
                .find(|t| t.save_table == id)
                .unwrap()
                .parent_table,
            parent
        );
    }
    let effective = effective_roster(&base, &save, &meta).unwrap();
    assert_eq!(effective.len(), 16);
    let db = TdbFile::parse(&base).unwrap();
    for (parent, rows) in effective {
        let table = db.table(&parent).unwrap();
        let mapping = schema
            .tables
            .iter()
            .find(|t| t.parent_table == parent)
            .unwrap();
        let key = table.field(&mapping.key).unwrap();
        assert_eq!(rows.len(), table.current_records as usize);
        for i in 0..table.current_records {
            let id = db.read_u64(table, i.into(), key).unwrap();
            let start = table.record_block + usize::from(i) * table.record_len_bytes as usize;
            assert_eq!(
                rows[&id],
                base[start..start + table.record_len_bytes as usize]
            );
        }
    }
    // Capturing the explicit update flag must not change existing editor validation.
    let (_, bio) = meta.save_tables().find(|(id, _)| *id == "cPbu").unwrap();
    assert!(!bio.update_fields.contains(&"zIBw".to_string()));
    assert!(meta.field("cPbu", "zIBw").unwrap().updatable);
}

#[test]
fn malformed_or_unrelated_schema_is_not_recognized() {
    let (base, save, meta) = fixture();
    assert!(recognize(&base, &base, &meta).is_err());
    let mut bad = save.clone();
    bad[24..28].copy_from_slice(b"FAIL");
    assert!(recognize(&base, &bad, &meta).is_err());
    let db = TdbFile::parse(&save).unwrap();
    let table = db.table("CvSu").unwrap();
    let mut bad = save.clone();
    let descriptor_width = table.block_offset + 40 + 12;
    bad[descriptor_width..descriptor_width + 4].copy_from_slice(&1u32.to_be_bytes());
    assert!(recognize(&base, &bad, &meta).is_err());
    assert!(recognize(&base, &save[..save.len() - 1], &meta).is_err());
}

#[test]
fn synthetic_update_uses_key_not_record_position_and_preserves_other_fields() {
    let (base, save, meta) = fixture();
    let original = effective_roster(&base, &save, &meta).unwrap();
    let (mut changed, _) = TdbFile::parse(&save)
        .unwrap()
        .append_record("CvSu")
        .unwrap();
    let db = TdbFile::parse(&base).unwrap();
    let parent = db.table("cPbu").unwrap();
    let parsed = TdbFile::parse(&changed).unwrap();
    let target = parsed.table("CvSu").unwrap().clone();
    let endian = parsed.endian;
    let source_index = 17;
    let key = db
        .read_u64(parent, source_index, parent.field("zIBw").unwrap())
        .unwrap();
    let source = &original["cPbu"][&key];
    for field in &target.fields {
        let f = parent.field(&field.id.raw()).unwrap();
        for offset in (0..field.bit_width).step_by(32) {
            let width = (field.bit_width - offset).min(32);
            let value = tdb_core::read_bits(source, f.bit_offset + offset, width, db.endian);
            tdb_core::write_bits(
                &mut changed
                    [target.record_block..target.record_block + target.record_len_bytes as usize],
                field.bit_offset + offset,
                width,
                value,
                endian,
            );
        }
    }
    let field = target
        .fields
        .iter()
        .find(|f| f.id.raw() != "zIBw" && f.kind == 3 && f.bit_width > 1)
        .unwrap();
    let output =
        &mut changed[target.record_block..target.record_block + target.record_len_bytes as usize];
    let prior = tdb_core::read_bits(output, field.bit_offset, field.bit_width, endian);
    tdb_core::write_bits(output, field.bit_offset, field.bit_width, prior ^ 1, endian);
    let snapshot = changed.clone();
    TdbFile::parse(&snapshot)
        .unwrap()
        .recompute_crcs(&mut changed);
    let mut expected = original;
    let f = parent.field(&field.id.raw()).unwrap();
    tdb_core::write_bits(
        expected.get_mut("cPbu").unwrap().get_mut(&key).unwrap(),
        f.bit_offset,
        f.bit_width,
        prior ^ 1,
        db.endian,
    );
    assert_eq!(effective_roster(&base, &changed, &meta).unwrap(), expected);
}

#[test]
fn synthetic_membership_changes_are_keyed_and_conflicting_operations_are_refused() {
    let (base, save, meta) = fixture();
    let original = effective_roster(&base, &save, &meta).unwrap();
    let db = TdbFile::parse(&base).unwrap();
    let parent = db.table("QEoV").unwrap();
    let key = parent.field("TWSX").unwrap();
    let removed = db.read_u64(parent, 3, key).unwrap();
    let append = |bytes: &[u8], table_id: &str, value: u64| {
        let (mut output, index) = TdbFile::parse(bytes)
            .unwrap()
            .append_record(table_id)
            .unwrap();
        let copy = output.clone();
        let parsed = TdbFile::parse(&copy).unwrap();
        let table = parsed.table(table_id).unwrap();
        let field = table.field("TWSX").unwrap();
        let start = table.record_block + index as usize * table.record_len_bytes as usize;
        tdb_core::write_bits(
            &mut output[start..start + table.record_len_bytes as usize],
            field.bit_offset,
            field.bit_width,
            value,
            parsed.endian,
        );
        parsed.recompute_crcs(&mut output);
        output
    };
    let deleted = append(&save, "mGOv", removed);
    let mut expected = original.clone();
    expected.get_mut("QEoV").unwrap().remove(&removed);
    assert_eq!(effective_roster(&base, &deleted, &meta).unwrap(), expected);
    let added = (0..(1u64 << key.bit_width))
        .find(|id| !original["QEoV"].contains_key(id))
        .unwrap();
    let inserted = append(&save, "sAMA", added);
    let rows = effective_roster(&base, &inserted, &meta).unwrap();
    assert_eq!(rows["QEoV"].len(), original["QEoV"].len() + 1);
    assert!(rows["QEoV"].contains_key(&added));
    assert!(effective_roster(&base, &append(&save, "sAMA", removed), &meta).is_err());
    assert!(effective_roster(&base, &append(&save, "mGOv", added), &meta).is_err());
    assert_eq!(
        effective_roster(&base, &append(&deleted, "sAMA", removed), &meta).unwrap(),
        original
    );
    assert!(effective_roster(&base, &append(&deleted, "mGOv", removed), &meta).is_err());
}

// Game-written saves captured 2026-10-05. Only the header and inner TDB are stored;
// the original allocation's remaining bytes were verified to be all zero.
fn game_save(slot: u8) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let (prefix, hash): (&[u8], &str) = match slot {
        1 => (
            include_bytes!("fixtures/nhl07/ROSTER1-prefix.bin"),
            "a3a4744bea0104dcec7aeb6cf5bfebc52c3b20b9615c8abe7442f5f835db8f22",
        ),
        2 => (
            include_bytes!("fixtures/nhl07/ROSTER2-prefix.bin"),
            "a9bd2aaac12893f1a6521def19582956d620d752758e8ada6cbc20362e2dea05",
        ),
        3 => (
            include_bytes!("fixtures/nhl07/ROSTER3-prefix.bin"),
            "8d95de096f9c86a0d6cda78ec30d65667965837db4ca9c9d2723d00c69612cd1",
        ),
        4 => (
            include_bytes!("fixtures/nhl07/ROSTER4-prefix.bin"),
            "f3352c3198d30c61c4be3960a89f5ad6ccf5b57c508007bb5459f4d4704254fb",
        ),
        _ => unreachable!(),
    };
    let mut bytes = prefix.to_vec();
    bytes.resize(1_598_888, 0);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), hash);
    assert!(tdb_savedata::SaveContainer::parse(&bytes)
        .unwrap()
        .checksums_valid());
    bytes
}

#[test]
fn game_written_replacements_project_demitra_and_reproduce_exact_serialization() {
    let (base, empty, meta) = fixture();
    let original = TdbFile::parse(&base).unwrap();
    let parent = original.table("ulGe").unwrap();
    for (slot, count, team, number) in [(1, 27, 14, 39), (2, 59, 24, 40)] {
        let container = game_save(slot);
        let inner = tdb_savedata::SaveContainer::parse(&container)
            .unwrap()
            .extract_db()
            .unwrap();
        let projection = effective_roster(&base, &inner, &meta).unwrap();
        let demitra = &projection["ulGe"][&587];
        for (id, expected) in [("BSXd", team), ("tRVs", number)] {
            let f = parent.field(id).unwrap();
            assert_eq!(
                tdb_core::read_bits(demitra, f.bit_offset, f.bit_width, original.endian),
                expected
            );
        }
        let actual = TdbFile::parse(&inner).unwrap();
        let mut rebuilt = empty.clone();
        for id in ["AtUj", "ibGM"] {
            let source = actual.table(id).unwrap();
            assert_eq!(source.current_records, count);
            assert_eq!(source.max_records, count);
            assert_eq!(source.data_alloc_type, 6);
            for _ in 0..count {
                rebuilt = TdbFile::parse(&rebuilt)
                    .unwrap()
                    .append_record(id)
                    .unwrap()
                    .0;
            }
            let snapshot = rebuilt.clone();
            let view = TdbFile::parse(&snapshot).unwrap();
            let dest = view.table(id).unwrap();
            let len = count as usize * source.record_len_bytes as usize;
            rebuilt[dest.record_block..dest.record_block + len]
                .copy_from_slice(&inner[source.record_block..source.record_block + len]);
            rebuilt[dest.block_offset + 4..dest.block_offset + 8]
                .copy_from_slice(&6u32.to_be_bytes());
            view.recompute_crcs(&mut rebuilt);
        }
        assert_eq!(
            rebuilt, inner,
            "game-written layout must reproduce byte-for-byte"
        );
    }
}

#[test]
fn experimental_writer_preserves_other_edits_and_fixed_allocation() {
    use studio_domain::roster_difflist::experimental_nhl07_jersey;
    let (base, _, meta) = fixture();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1");
    for container in [std::fs::read(path).unwrap(), game_save(1), game_save(2)] {
        let parsed = tdb_savedata::SaveContainer::parse(&container).unwrap();
        let before = parsed.extract_db().unwrap();
        let mut expected = effective_roster(&base, &before, &meta).unwrap();
        let db = TdbFile::parse(&base).unwrap();
        let jersey = db.table("ulGe").unwrap().field("tRVs").unwrap();
        tdb_core::write_bits(
            expected.get_mut("ulGe").unwrap().get_mut(&587).unwrap(),
            jersey.bit_offset,
            jersey.bit_width,
            37,
            db.endian,
        );
        let written = experimental_nhl07_jersey(&base, &container, &meta, 587, 37).unwrap();
        assert_eq!(written.len(), container.len());
        let parsed = tdb_savedata::SaveContainer::parse(&written).unwrap();
        assert!(parsed.checksums_valid());
        let inner = parsed.extract_db().unwrap();
        let mut checksum_copy = inner.clone();
        TdbFile::parse(&inner)
            .unwrap()
            .recompute_crcs(&mut checksum_copy);
        assert_eq!(checksum_copy, inner);
        assert_eq!(effective_roster(&base, &inner, &meta).unwrap(), expected);
        assert_eq!(
            experimental_nhl07_jersey(&base, &written, &meta, 587, 37).unwrap(),
            written
        );
        assert!(experimental_nhl07_jersey(&base, &container, &meta, 587, 128).is_err());
        assert!(experimental_nhl07_jersey(&base, &container, &meta, 99999, 37).is_err());
        let mut corrupt = container.clone();
        *corrupt.last_mut().unwrap() = 1;
        tdb_savedata::checksum::recompute_a_only(&mut corrupt);
        assert!(experimental_nhl07_jersey(&base, &corrupt, &meta, 587, 37).is_err());
    }
}

#[test]
fn jersey_writer_matches_native_loaded_and_resaved_roster3_exactly() {
    let (base, _, meta) = fixture();
    let generated = studio_domain::roster_difflist::experimental_nhl07_jersey(
        &base,
        &game_save(2),
        &meta,
        587,
        38,
    )
    .unwrap();
    // User loaded our ROSTER2 output in NHL07/Xenia, confirmed STL #38, then saved
    // as ROSTER3. Its complete file is identical, not just the interpreted fields.
    assert_eq!(generated, game_save(3));
}

#[test]
fn speed_update_probe_preserves_roster_and_other_attributes_and_reuses_keyed_row() {
    use studio_domain::roster_difflist::experimental_nhl07_speed_raw;
    let (base, _, meta) = fixture();
    let container = game_save(3);
    let inner = tdb_savedata::SaveContainer::parse(&container)
        .unwrap()
        .extract_db()
        .unwrap();
    let mut expected = effective_roster(&base, &inner, &meta).unwrap();
    let db = TdbFile::parse(&base).unwrap();
    let speed = db.table("yvSd").unwrap().field("YWPq").unwrap();
    let mut current = container.clone();
    for value in [49, 48] {
        let written = experimental_nhl07_speed_raw(&base, &current, &meta, 613, value).unwrap();
        assert_eq!(written.len(), container.len());
        let parsed = tdb_savedata::SaveContainer::parse(&written).unwrap();
        assert!(parsed.checksums_valid());
        let inner = parsed.extract_db().unwrap();
        let tdb = TdbFile::parse(&inner).unwrap();
        let table = tdb.table("RosT").unwrap();
        assert_eq!(
            (
                table.current_records,
                table.max_records,
                table.data_alloc_type
            ),
            (1, 1, 6)
        );
        let mut crcs = inner.clone();
        tdb.recompute_crcs(&mut crcs);
        assert_eq!(crcs, inner);
        tdb_core::write_bits(
            expected.get_mut("yvSd").unwrap().get_mut(&613).unwrap(),
            speed.bit_offset,
            speed.bit_width,
            value,
            db.endian,
        );
        assert_eq!(effective_roster(&base, &inner, &meta).unwrap(), expected);
        assert_eq!(
            experimental_nhl07_speed_raw(&base, &written, &meta, 613, value).unwrap(),
            written
        );
        current = written;
    }
    assert!(experimental_nhl07_speed_raw(&base, &container, &meta, 613, 64).is_err());
    assert!(experimental_nhl07_speed_raw(&base, &container, &meta, 99999, 49).is_err());
}

#[test]
fn speed_writer_matches_native_loaded_and_resaved_roster4_exactly() {
    let (base, _, meta) = fixture();
    let generated = studio_domain::roster_difflist::experimental_nhl07_speed_raw(
        &base,
        &game_save(3),
        &meta,
        613,
        49,
    )
    .unwrap();
    // NHL07 displayed Speed 99, then saved ROSTER4 with the entire file unchanged.
    // This is a game-returned fixture, not output generated inside the test.
    assert_eq!(generated, game_save(4));
}
