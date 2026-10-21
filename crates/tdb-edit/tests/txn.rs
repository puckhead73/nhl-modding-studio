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

/// `reassign_jersey`: a kit moves between teams by rewriting only its team + per-team index; the
/// asset id stays; indexes are minted from the destination's free set; bad inputs roll back.
#[test]
fn reassign_jersey_moves_a_kit_and_keeps_its_asset_id() {
    use tdb_edit::txn::jerseys;
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table_canonical(jerseys::TABLE).unwrap();
    let f = |id: &str| t.fields.iter().find(|f| f.id.canonical(db.endian) == id).unwrap();
    let (team_f, idx_f, art_f, var_f) = (f(jerseys::TEAM), f(jerseys::INDEX), f(jerseys::ART_ID), f(jerseys::VARIANT));
    let rows = t.current_records as u32;
    let read = |b: &[u8], r: u32, fld| TdbFile::parse(b).unwrap().read_u64(TdbFile::parse(b).unwrap().table_canonical(jerseys::TABLE).unwrap(), r, fld).unwrap();
    // Donor: team 1's last-indexed kit. Destination: team 0.
    let donor = (0..rows).filter(|&r| db.read_u64(t, r, team_f) == Some(1)).max_by_key(|&r| db.read_u64(t, r, idx_f)).unwrap();
    let dest_count = (0..rows).filter(|&r| db.read_u64(t, r, team_f) == Some(0)).count() as u64;
    let (art, var) = (db.read_u64(t, donor, art_f).unwrap(), db.read_u64(t, donor, var_f).unwrap());

    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let out = tx.reassign_jersey(donor, 0, None).unwrap();
    assert_eq!(out.edits, 2);
    let after = tx.bytes().to_vec();
    assert_eq!(read(&after, donor, team_f), 0);
    assert_eq!(read(&after, donor, idx_f), dest_count, "smallest unused index on the destination");
    assert_eq!((read(&after, donor, art_f), read(&after, donor, var_f)), (art, var), "asset id untouched");
    let after_db = TdbFile::parse(&after).unwrap();
    let after_t = after_db.table_canonical(jerseys::TABLE).unwrap();
    assert_eq!(after_t.current_records, t.current_records, "no rows added or removed");
    assert_eq!((0..rows).filter(|&r| after_db.read_u64(after_t, r, team_f) == Some(0)).count() as u64, dest_count + 1);

    // Explicit index, then refusals that leave the bytes untouched.
    tx.reassign_jersey(donor, 1, Some(31)).unwrap();
    assert_eq!(read(tx.bytes(), donor, idx_f), 31);
    let snapshot = tx.bytes().to_vec();
    assert!(matches!(tx.reassign_jersey(donor, 0, Some(32)), Err(EditError::InvalidValue { .. })), "index past the 5-bit width");
    assert!(matches!(tx.reassign_jersey(rows, 0, None), Err(EditError::InvalidValue { .. })), "record out of range");
    assert!(matches!(tx.reassign_jersey(donor, 9_999, None), Err(EditError::InvalidValue { .. })), "unknown team");
    assert_eq!(tx.bytes(), &snapshot[..], "refusals change nothing");
}

/// `clone_jersey` on a packed registry grows the table by one row (block growth, downstream
/// tables shifted, CRCs valid); the per-team ceiling is 32 (`tYnx` 5 bits): a team filled to 32
/// refuses a 33rd by clone or by move, and another team still accepts one.
#[test]
fn clone_jersey_grows_a_packed_registry_and_the_team_ceiling_is_32() {
    use tdb_edit::txn::jerseys;
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let t = db.table_canonical(jerseys::TABLE).unwrap();
    assert_eq!(t.current_records, t.max_records, "the Legacy registry ships packed");
    let (rows, cap) = (t.current_records, t.max_records);
    let team_f = t.fields.iter().find(|f| f.id.canonical(db.endian) == jerseys::TEAM).unwrap();
    let idx_f = t.fields.iter().find(|f| f.id.canonical(db.endian) == jerseys::INDEX).unwrap();
    let count_team = |b: &[u8], team: u64| {
        let d = TdbFile::parse(b).unwrap();
        let tt = d.table_canonical(jerseys::TABLE).unwrap();
        (0..tt.current_records as u32).filter(|&r| d.read_u64(tt, r, team_f) == Some(team)).count() as u64
    };
    let team0_before = count_team(&bytes, 0);
    let source = (0..rows as u32).find(|&r| db.read_u64(t, r, team_f) == Some(0)).unwrap();

    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let (new_rec, out) = tx.clone_jersey(source, 0, 1, 0, 30, None).unwrap();
    assert_eq!(new_rec, rows as u32, "appended at the old row count");
    assert!(out.edits > 0);
    let grown = TdbFile::parse(tx.bytes()).unwrap();
    let gt = grown.table_canonical(jerseys::TABLE).unwrap();
    assert_eq!((gt.current_records, gt.max_records), (rows + 1, cap + 1), "grown by exactly one row");
    assert_eq!(grown.read_u64(gt, new_rec, team_f), Some(0));
    assert_eq!(grown.read_u64(gt, new_rec, idx_f), Some(team0_before), "smallest unused index");
    assert_eq!(count_team(tx.bytes(), 0), team0_before + 1);
    assert_eq!(grown.tables.len(), db.tables.len(), "every other table survived the shift");

    // Fill team 0 to 32 by moving other teams' last kits onto it, then the ceiling holds.
    let mut donors: Vec<u32> = Vec::new();
    for team in 1..=40u64 {
        if count_team(tx.bytes(), 0) >= 32 {
            break;
        }
        let d = TdbFile::parse(tx.bytes()).unwrap();
        let dt = d.table_canonical(jerseys::TABLE).unwrap();
        let Some(rec) = (0..dt.current_records as u32).filter(|&r| d.read_u64(dt, r, team_f) == Some(team)).max_by_key(|&r| d.read_u64(dt, r, idx_f)) else { continue };
        tx.reassign_jersey(rec, 0, None).unwrap();
        donors.push(rec);
    }
    assert_eq!(count_team(tx.bytes(), 0), 32);
    let snapshot = tx.bytes().to_vec();
    assert!(matches!(tx.clone_jersey(source, 0, 1, 0, 31, None), Err(EditError::Alloc(_))), "a 33rd kit by clone is refused");
    assert!(matches!(tx.clone_jersey(source, 0, 1, 0, 31, Some(5)), Err(EditError::Alloc(_))), "even with an explicit index");
    let other = (0..rows as u32).find(|&r| db.read_u64(t, r, team_f) == Some(9)).unwrap();
    assert!(matches!(tx.reassign_jersey(other, 0, None), Err(EditError::Alloc(_))), "a 33rd kit by move is refused");
    assert_eq!(tx.bytes(), &snapshot[..], "refusals change nothing");
    let team9_before = count_team(tx.bytes(), 9);
    tx.clone_jersey(source, 9, 1, 9, 30, None).unwrap();
    assert_eq!(count_team(tx.bytes(), 9), team9_before + 1, "another team still accepts a kit");
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
