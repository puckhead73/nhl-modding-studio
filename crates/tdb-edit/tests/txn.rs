//! Layer 3 repairing-transaction operations against the committed PS3 NHL Legacy anchor.

use std::path::Path;
use tdb_core::TdbFile;
use tdb_domain_validation::single_captain;
use tdb_edit::txn::{captaincy, EditTransaction};
use tdb_edit::EditError;
use tdb_relations::RosterModel;

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

/// Patch one fixed-bit `eGlu` cell in raw bytes and fix CRCs — a dirty-fixture helper.
fn patch_eglu(bytes: &[u8], record: u32, field: &str, value: u64) -> Vec<u8> {
    let db = TdbFile::parse(bytes).unwrap();
    let eglu = db.table_canonical("eGlu").unwrap();
    let f = eglu
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == field)
        .unwrap();
    let mut buf = bytes.to_vec();
    assert!(db.set_u64(&mut buf, eglu, record, f, value));
    db.recompute_crcs(&mut buf);
    buf
}

/// A team with a captain, plus a second occupied slot on that same team (the natural place to inject
/// or introduce a multi-captain condition). Returns `(team, captain_record, second_record)`.
fn team_captain_and_teammate(bytes: &[u8]) -> (u64, u32, u32) {
    let db = TdbFile::parse(bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let cap = model
        .slots
        .iter()
        .find(|s| s.captaincy == Some(captaincy::CAPTAIN) && s.player_id.is_some())
        .expect("an anchor team with a captain");
    let second = model
        .slots
        .iter()
        .find(|s| s.team_id == cap.team_id && s.player_id.is_some() && s.record != cap.record)
        .expect("a second occupied slot on the captain's team")
        .record;
    (cap.team_id, cap.record, second)
}

/// The delta gate must let a generic edit through even when the DB already carries an unrelated,
/// pre-existing violation (EA vanilla anchors do — this is what blocked all edits on NHL09/10 and
/// Legacy-PS3). Inject a phantom second captain, then make an unrelated bio edit and assert it commits.
#[test]
fn pre_existing_violation_does_not_block_unrelated_edit() {
    let bytes = anchor(DB);
    let (_team, _cap, second) = team_captain_and_teammate(&bytes);

    // Inject a pre-existing multi-captain by stamping the captaincy bit on a second occupied slot.
    let dirty = patch_eglu(&bytes, second, "mCcl", captaincy::CAPTAIN);
    assert!(
        !single_captain(&TdbFile::parse(&dirty).unwrap()).is_empty(),
        "fixture should carry a pre-existing single_captain violation"
    );

    // An unrelated generic edit (ubPc bio position field) must still commit despite the pre-existing
    // roster violation elsewhere in the DB.
    let mut tx = EditTransaction::begin(&dirty).unwrap();
    let db = TdbFile::parse(&dirty).unwrap();
    let ubpc = db.table_canonical("ubPc").unwrap();
    let vjla = ubpc
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "vjla")
        .unwrap();
    let cur = db.read_u64(ubpc, 0, vjla).unwrap();
    let outcome = tx.set_cell_int("ubPc", 0, "vjla", cur);
    assert!(
        outcome.is_ok(),
        "a pre-existing unrelated violation must not roll back an edit, got {outcome:?}"
    );
}

/// The delta gate must still reject an edit that *introduces* a new violation. A generic write of the
/// captaincy bit onto a second occupied slot (bypassing `set_captaincy`'s auto-demote) creates a
/// second captain on a previously-clean team — that must roll back.
#[test]
fn edit_that_introduces_a_violation_is_rejected() {
    let bytes = anchor(DB); // clean anchor
    let (_team, _cap, second) = team_captain_and_teammate(&bytes);

    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let res = tx.set_cell_int("eGlu", second, "mCcl", captaincy::CAPTAIN);
    assert!(
        matches!(
            res,
            Err(EditError::AuditFailed {
                predicate: "single_captain",
                ..
            })
        ),
        "introducing a second captain must be rejected, got {res:?}"
    );
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
        outcome
            .repairs
            .iter()
            .any(|r| r.contains("demoted prior captain")),
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
    assert_ne!(
        old_slot.captaincy,
        Some(captaincy::CAPTAIN),
        "old captain demoted"
    );
    drop(model);

    // Commit: CRC-valid and still audit-clean.
    let out = tx.commit().unwrap();
    let db = TdbFile::parse(&out).unwrap();
    assert!(db.header_crc_ok(), "committed db header CRC must be valid");
    assert!(db.size_ok());
    assert!(
        single_captain(&db).is_empty(),
        "committed db has no multi-captain team"
    );
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

    let outcome = tx
        .set_captaincy(team, player, captaincy::ALTERNATE)
        .unwrap();
    assert_eq!(outcome.edits, 1);
    assert!(outcome.repairs.is_empty());
}
