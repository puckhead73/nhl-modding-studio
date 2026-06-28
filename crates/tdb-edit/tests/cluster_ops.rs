//! Layer 3 structural operations (create/delete player) verified against the NHL Legacy anchor,
//! audited with the full save-time predicate set (relations + enums loaded from `defs/`).

use std::path::{Path, PathBuf};
use tdb_core::TdbFile;
use tdb_domain_validation::validate_invariants;
use tdb_edit::txn::EditTransaction;
use tdb_edit::EditError;
use tdb_enums::EnumRegistry;
use tdb_relations::{RelationRegistry, RosterModel};

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors").join(rel)).unwrap()
}

fn defs(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../defs").join(sub)
}

const DB: &str = "ps3/nhllegacy/databases/nhlng.db";

/// Run the full save-time audit (all shared predicates) and assert no Error-severity findings.
fn assert_write_clean(db_bytes: &[u8]) {
    let db = TdbFile::parse(db_bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();
    let enums = EnumRegistry::from_dir(defs("enums")).unwrap();
    let report = validate_invariants(&db, &relations, &enums);
    assert!(
        report.is_clean(),
        "audit not clean: {:?}",
        report.errors().take(5).collect::<Vec<_>>()
    );
}

/// Creating a skater builds the full cluster (bio + dSvy + xmja + ZBac + eGlu), wires it onto the
/// team, and leaves the committed db CRC-valid and audit-clean under every shared predicate.
#[test]
fn create_skater_builds_clean_cluster() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    let before = tx.model().unwrap();
    let team = before.teams[0].team_id;
    let (players0, slots0) = (before.players.len(), before.slots.len());
    drop(before);

    // position 2 = RW (skater).
    let (pid, outcome) = tx.create_player(team, 2).unwrap();
    assert_eq!(outcome.edits, 10);

    let model = tx.model().unwrap();
    assert_eq!(model.players.len(), players0 + 1, "one new bio");
    assert_eq!(model.slots.len(), slots0 + 1, "one new roster slot");

    let player = model.player(pid).expect("new player resolvable by id");
    assert!(player.companions_complete(), "skater has dSvy + xmja");
    assert!(player.skater_ai.is_some() && player.skater_equip.is_some());
    assert!(player.goalie_ai.is_none() && player.goalie_equip.is_none());

    let slot = model
        .slots
        .iter()
        .find(|s| s.player_id == Some(pid))
        .expect("new slot bridges to the new player");
    assert_eq!(slot.team_id, team);
    assert!(model.slot_team(slot).is_some());
    drop(model);

    let out = tx.commit().unwrap();
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);
}

/// Creating a goalie builds the goalie companion pair (mHuy + fMVl), not the skater pair.
#[test]
fn create_goalie_builds_goalie_pair() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let team = tx.model().unwrap().teams[3].team_id;

    let (pid, _) = tx.create_player(team, 4).unwrap(); // 4 = G
    let model = tx.model().unwrap();
    let player = model.player(pid).unwrap();
    assert!(player.companions_complete());
    assert!(player.goalie_ai.is_some() && player.goalie_equip.is_some());
    assert!(player.skater_ai.is_none() && player.skater_equip.is_none());
}

/// Deleting a created player removes the whole cluster (counts return to baseline) and leaves the db
/// audit-clean — no orphaned companions, indexes, or slots.
#[test]
fn delete_removes_whole_cluster_clean() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    let base = tx.model().unwrap();
    let team = base.teams[0].team_id;
    let (players0, slots0) = (base.players.len(), base.slots.len());
    drop(base);

    let (pid, _) = tx.create_player(team, 1).unwrap(); // 1 = LW
    assert_eq!(tx.model().unwrap().players.len(), players0 + 1);

    let outcome = tx.delete_player(pid).unwrap();
    // 1 slot + 1 index + 2 companions + 1 bio = 5 rows for a freshly-created, single-slot player.
    assert_eq!(outcome.edits, 5, "removed the five cluster rows");

    let model = tx.model().unwrap();
    assert_eq!(model.players.len(), players0, "bio count back to baseline");
    assert_eq!(model.slots.len(), slots0, "slot count back to baseline");
    assert!(model.player(pid).is_none(), "player no longer resolvable");
    drop(model);

    let out = tx.commit().unwrap();
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);
}

/// Deleting an existing anchor player also leaves the db audit-clean (no dangling references).
#[test]
fn delete_existing_player_is_clean() {
    let bytes = anchor(DB);
    let db0 = TdbFile::parse(&bytes).unwrap();
    let model0 = RosterModel::build(&db0).unwrap();
    // A skater with a roster slot, so the delete exercises every cluster table.
    let pid = model0
        .slots
        .iter()
        .filter_map(|s| s.player_id)
        .find(|&id| {
            model0
                .player(id)
                .is_some_and(|p| p.skater_ai.is_some())
        })
        .unwrap();
    drop(db0);

    let mut tx = EditTransaction::begin(&bytes).unwrap();
    tx.delete_player(pid).unwrap();
    let out = tx.commit().unwrap();
    assert_write_clean(&out);
}

#[test]
fn create_rejects_bad_args() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    assert!(matches!(
        tx.create_player(0, 9).unwrap_err(),
        EditError::InvalidValue { what: "position", .. }
    ));
    assert!(matches!(
        tx.create_player(0xFFFF_FFFF, 0).unwrap_err(),
        EditError::InvalidValue { .. }
    ));
    assert!(matches!(
        tx.delete_player(0xFFFF_FFFF).unwrap_err(),
        EditError::NoSuchPlayer(_)
    ));
}
