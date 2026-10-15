//! NHL07's interleaved keyed roster must not be treated as fixed team blocks.
use std::path::Path;
use tdb_core::TdbFile;
use tdb_edit::txn::EditTransaction;
use tdb_relations::RosterModel;

fn anchor() -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors/xbox360/nhl07/databases/NHL07NG.db")).unwrap()
}
fn value(bytes: &[u8], table: &str, record: u32, field: &str) -> u64 {
    let db = TdbFile::parse(bytes).unwrap();
    let t = db.table_canonical(table).unwrap();
    let f = t.fields.iter().find(|f| f.id.canonical(db.endian) == field).unwrap();
    db.read_u64(t, record, f).unwrap()
}

#[test]
fn transfer_keeps_keys_secondary_membership_and_unrelated_records() {
    let original = anchor();
    let mut tx = EditTransaction::begin(&original).unwrap();
    let before = tx.model().unwrap();
    let source = before.slots.iter().find(|s| s.player_id == Some(613) && s.team_id == 14).unwrap();
    let source_record = source.record;
    let bio = before.player(613).unwrap().bio_record;
    assert!(!before.slots.iter().any(|s| s.team_id == 28 && s.player_id.is_none()));
    tx.move_player(613, 14, 28).unwrap();
    tx.set_jersey_number(28, 613, 90).unwrap();
    let after = tx.model().unwrap();
    assert_eq!(before.slots.len(), after.slots.len());
    for (old, new) in before.slots.iter().zip(&after.slots) {
        assert_eq!(old.record, new.record);
        assert_eq!(old.index_key, new.index_key);
        assert_eq!(old.player_id, new.player_id);
        if old.record == source_record {
            assert_eq!(new.team_id, 28);
            assert_eq!(new.jersey, Some(90));
            assert_eq!(new.captaincy, Some(0));
        } else {
            assert_eq!(old.team_id, new.team_id);
            assert_eq!(old.jersey, new.jersey);
            assert_eq!(old.captaincy, new.captaincy);
        }
    }
    assert_eq!(value(tx.bytes(), "ubPc", bio, "dXSB"), 28);
    assert_eq!(value(tx.bytes(), "ubPc", bio, "sVRt"), 90);
    // Neither the stock/default tables nor any unrelated field may be changed implicitly.
    let db = TdbFile::parse(&original).unwrap();
    let edited = TdbFile::parse(tx.bytes()).unwrap();
    for t in &db.tables {
        let name = t.id.canonical(db.endian);
        let other = edited.table_canonical(&name).unwrap();
        for r in 0..t.current_records as u32 {
            if (name == "ubPc" && r == bio) || (name == "eGlu" && r == source_record) { continue; }
            let start = t.record_block + r as usize * t.record_len_bytes as usize;
            let other_start = other.record_block + r as usize * other.record_len_bytes as usize;
            assert_eq!(&original[start..start + t.record_len_bytes as usize],
                &tx.bytes()[other_start..other_start + other.record_len_bytes as usize], "{name}[{r}]");
        }
    }
    // Moving the same player to a team they already belong to refuses atomically.
    let before_error = tx.bytes().to_vec();
    assert!(tx.move_player(613, 28, 48).is_err());
    assert_eq!(tx.bytes(), before_error);
    // Secondary jersey edits stay secondary; they cannot overwrite the primary bio number.
    let free = (1..=99).find(|n| !after.slots.iter().any(|s| s.team_id == 48 && s.jersey == Some(*n))).unwrap();
    tx.set_jersey_number(48, 613, free).unwrap();
    assert_eq!(value(tx.bytes(), "ubPc", bio, "sVRt"), 90);
    let occupied = after.slots.iter().find(|s| s.team_id == 28 && s.player_id != Some(613) && s.jersey.unwrap_or(0) != 0).unwrap().jersey.unwrap();
    let before_error = tx.bytes().to_vec();
    assert!(tx.set_jersey_number(28, 613, occupied).is_err());
    assert_eq!(tx.bytes(), before_error);
    assert!(RosterModel::build(&TdbFile::parse(tx.bytes()).unwrap()).is_ok());
}
