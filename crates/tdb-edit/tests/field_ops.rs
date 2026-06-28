//! Layer 3 field operations (move/trade, jersey, roster status) against the NHL Legacy anchor.

use std::path::{Path, PathBuf};
use tdb_core::TdbFile;
use tdb_domain_validation::validate_invariants;
use tdb_edit::txn::{captaincy, EditTransaction};
use tdb_edit::EditError;
use tdb_enums::EnumRegistry;
use tdb_relations::RelationRegistry;

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors").join(rel)).unwrap()
}
fn defs(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../defs").join(sub)
}
const DB: &str = "ps3/nhllegacy/databases/nhlng.db";

fn assert_write_clean(db_bytes: &[u8]) {
    let db = TdbFile::parse(db_bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();
    let enums = EnumRegistry::from_dir(defs("enums")).unwrap();
    let report = validate_invariants(&db, &relations, &enums);
    assert!(report.is_clean(), "audit not clean: {:?}", report.errors().take(5).collect::<Vec<_>>());
}

/// Trading a captain moves their bio + slot to the new team and clears their captaincy (preserving
/// the single-captain invariant), leaving the db audit-clean.
#[test]
fn move_captain_retags_team_and_clears_captaincy() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    let model = tx.model().unwrap();
    let cap = model
        .slots
        .iter()
        .find(|s| s.captaincy == Some(captaincy::CAPTAIN) && s.player_id.is_some())
        .unwrap();
    let (from_team, player) = (cap.team_id, cap.player_id.unwrap());
    let to_team = model.teams.iter().map(|t| t.team_id).find(|&t| t != from_team).unwrap();
    drop(model);

    let outcome = tx.move_player(player, to_team).unwrap();
    assert!(outcome.repairs.iter().any(|r| r.contains("cleared captaincy")));

    let model = tx.model().unwrap();
    let slot = model.slots.iter().find(|s| s.player_id == Some(player)).unwrap();
    assert_eq!(slot.team_id, to_team, "slot retagged to new team");
    assert_ne!(slot.captaincy, Some(captaincy::CAPTAIN), "captaincy cleared on trade");
    assert!(model.team_roster(to_team).iter().any(|s| s.player_id == Some(player)));
    assert!(!model.team_roster(from_team).iter().any(|s| s.player_id == Some(player)));
    drop(model);

    let out = tx.commit().unwrap();
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);
}

/// A jersey number is set on the slot, is unique-checked within the team, and range-checked.
#[test]
fn set_jersey_validates_uniqueness_and_range() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    // A team with at least two rostered players.
    let model = tx.model().unwrap();
    let team = model
        .teams
        .iter()
        .map(|t| t.team_id)
        .find(|&t| model.team_roster(t).iter().filter(|s| s.player_id.is_some()).count() >= 2)
        .unwrap();
    let roster: Vec<u64> = model
        .team_roster(team)
        .iter()
        .filter_map(|s| s.player_id)
        .collect();
    let used: std::collections::HashSet<u64> = model
        .team_roster(team)
        .iter()
        .filter_map(|s| s.jersey)
        .collect();
    let free = (1u64..=99).find(|n| !used.contains(n)).unwrap();
    let (a, b) = (roster[0], roster[1]);
    drop(model);

    // Assign a free number to A.
    tx.set_jersey_number(team, a, free).unwrap();
    let model = tx.model().unwrap();
    let a_slot = model.slots.iter().find(|s| s.team_id == team && s.player_id == Some(a)).unwrap();
    assert_eq!(a_slot.jersey, Some(free));
    drop(model);

    // B cannot take the same number now.
    assert!(matches!(
        tx.set_jersey_number(team, b, free),
        Err(EditError::JerseyTaken { .. })
    ));
    // Out-of-range (sVRt is 7-bit → max 127) is rejected.
    assert!(matches!(
        tx.set_jersey_number(team, b, 9999),
        Err(EditError::InvalidValue { what: "jersey number", .. })
    ));

    let out = tx.commit().unwrap();
    assert_write_clean(&out);
}

/// Roster status writes the bridged ZBac.RREB code and round-trips through the entity model.
#[test]
fn set_roster_status_writes_index_row() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    let model = tx.model().unwrap();
    let slot = model
        .slots
        .iter()
        .find(|s| s.player_id.is_some() && s.index_key.is_some())
        .unwrap();
    let (team, player) = (slot.team_id, slot.player_id.unwrap());
    let new_status = match slot.roster_status {
        Some(0) => 1,
        _ => 0,
    };
    drop(model);

    tx.set_roster_status(team, player, new_status).unwrap();
    let model = tx.model().unwrap();
    let slot = model.slots.iter().find(|s| s.team_id == team && s.player_id == Some(player)).unwrap();
    assert_eq!(slot.roster_status, Some(new_status));
    drop(model);

    let out = tx.commit().unwrap();
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);
}

#[test]
fn move_rejects_bad_args() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let player = tx.model().unwrap().slots.iter().find_map(|s| s.player_id).unwrap();
    assert!(matches!(
        tx.move_player(player, 0xFFFF_FFFF),
        Err(EditError::InvalidValue { .. })
    ));
    assert!(matches!(
        tx.move_player(0xFFFF_FFFF, 0),
        Err(EditError::NoSuchPlayer(_))
    ));
}
