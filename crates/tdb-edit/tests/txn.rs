//! Layer 3 repairing-transaction operations against the committed PS3 NHL Legacy anchor.

use std::path::Path;
use tdb_core::TdbFile;
use tdb_domain_validation::single_captain;
use tdb_edit::txn::{captaincy, EditTransaction};
use tdb_edit::EditError;

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors").join(rel)).unwrap()
}

const DB: &str = "ps3/nhllegacy/databases/nhlng.db";

/// Promoting a new captain auto-demotes the prior one (repair, not refusal); the single-captain
/// invariant holds afterwards; and the committed bytes are CRC-valid and still audit-clean.
#[test]
fn set_captaincy_demotes_prior_and_commits_clean() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    // Find an existing captain and a different team-mate to promote.
    let model = tx.model().unwrap();
    let cap = model
        .slots
        .iter()
        .find(|s| s.captaincy == Some(captaincy::CAPTAIN) && s.player_id.is_some())
        .expect("an anchor team with a captain");
    let team = cap.team_id;
    let old_captain = cap.player_id.unwrap();
    let new_captain = model
        .slots
        .iter()
        .find(|s| {
            s.team_id == team
                && s.player_id.is_some()
                && s.player_id != Some(old_captain)
                && s.captaincy != Some(captaincy::CAPTAIN)
        })
        .expect("a second player on the same team")
        .player_id
        .unwrap();
    drop(model);

    let outcome = tx
        .set_captaincy(team, new_captain, captaincy::CAPTAIN)
        .unwrap();
    assert_eq!(outcome.edits, 2, "1 promotion + 1 auto-demotion");
    assert!(
        outcome.repairs.iter().any(|r| r.contains("demoted prior captain")),
        "expected an auto-demotion repair, got {:?}",
        outcome.repairs
    );

    // Pending state: exactly one captain on the team, and it is the new one.
    let model = tx.model().unwrap();
    let caps: Vec<_> = model
        .slots
        .iter()
        .filter(|s| s.team_id == team && s.captaincy == Some(captaincy::CAPTAIN))
        .collect();
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].player_id, Some(new_captain));
    let old_slot = model
        .slots
        .iter()
        .find(|s| s.team_id == team && s.player_id == Some(old_captain))
        .unwrap();
    assert_ne!(old_slot.captaincy, Some(captaincy::CAPTAIN), "old captain demoted");
    drop(model);

    // Commit: CRC-valid and still audit-clean.
    let out = tx.commit().unwrap();
    let db = TdbFile::parse(&out).unwrap();
    assert!(db.header_crc_ok(), "committed db header CRC must be valid");
    assert!(db.size_ok());
    assert!(single_captain(&db).is_empty(), "committed db has no multi-captain team");
}

/// An operation that cannot resolve its target rolls the working bytes back untouched.
#[test]
fn unknown_slot_rolls_back() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let before = tx.bytes().to_vec();
    let err = tx
        .set_captaincy(0xFFFF_FFFF, 0xFFFF_FFFF, captaincy::CAPTAIN)
        .unwrap_err();
    assert!(matches!(err, EditError::NoSuchSlot { .. }));
    assert_eq!(tx.bytes(), &before[..], "failed op must not mutate state");
}

/// Roles outside the confirmed enum are rejected at entry.
#[test]
fn invalid_role_rejected() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    assert!(matches!(
        tx.set_captaincy(0, 0, 7).unwrap_err(),
        EditError::InvalidValue { .. }
    ));
}

/// Setting a non-captain role makes no auto-demotion edits.
#[test]
fn alternate_role_makes_no_repairs() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let model = tx.model().unwrap();
    let slot = model
        .slots
        .iter()
        .find(|s| s.player_id.is_some() && s.captaincy != Some(captaincy::CAPTAIN))
        .unwrap();
    let (team, player) = (slot.team_id, slot.player_id.unwrap());
    drop(model);

    let outcome = tx.set_captaincy(team, player, captaincy::ALTERNATE).unwrap();
    assert_eq!(outcome.edits, 1);
    assert!(outcome.repairs.is_empty());
}
