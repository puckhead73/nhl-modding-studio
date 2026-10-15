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
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

fn defs(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../defs")
        .join(sub)
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
        .find(|&id| model0.player(id).is_some_and(|p| p.skater_ai.is_some()))
        .unwrap();
    drop(db0);

    let mut tx = EditTransaction::begin(&bytes).unwrap();
    tx.delete_player(pid).unwrap();
    let out = tx.commit().unwrap();
    assert_write_clean(&out);
}

/// A created player's `eGlu` slot lands **inside its team's contiguous block** (the block grows at its
/// boundary), never appended at the table end — so per-team record blocks stay contiguous and keep
/// tiling. Uses the front team (block at record 0), where a naive append-at-end would clearly break
/// tiling.
#[test]
fn create_places_slot_inside_team_block() {
    let bytes = anchor(DB);
    let db0 = TdbFile::parse(&bytes).unwrap();
    let model0 = RosterModel::build(&db0).unwrap();
    let team = model0.slots.iter().find(|s| s.record == 0).unwrap().team_id;
    let last_eglu = model0.slots.iter().map(|s| s.record).max().unwrap();
    drop(db0);

    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let (pid, _) = tx.create_player(team, 2).unwrap();
    let model = tx.model().unwrap();

    let recs: Vec<u32> = model
        .slots
        .iter()
        .filter(|s| s.team_id == team)
        .map(|s| s.record)
        .collect();
    let (min, max, count) = (
        *recs.iter().min().unwrap(),
        *recs.iter().max().unwrap(),
        recs.len() as u32,
    );
    assert_eq!(
        max - min + 1,
        count,
        "team block stayed contiguous after the insert"
    );
    let new_rec = model
        .slots
        .iter()
        .find(|s| s.player_id == Some(pid))
        .unwrap()
        .record;
    assert!(
        (min..=max).contains(&new_rec),
        "new slot sits inside the team block"
    );
    assert!(new_rec < last_eglu, "not appended at the table tail");
    drop(model);
    assert_write_clean(&tx.commit().unwrap());
}

/// Creating on a team that owns a **vacant in-block slot** reuses it (no `eGlu` growth) rather than
/// inserting — matching the single free slot the vanilla anchor ships and NHLViewNG's free-slot reuse.
#[test]
fn create_reuses_vacant_in_block_slot() {
    let bytes = anchor(DB);
    let db0 = TdbFile::parse(&bytes).unwrap();
    let model0 = RosterModel::build(&db0).unwrap();
    let vacant = model0
        .slots
        .iter()
        .find(|s| s.index_key.is_none())
        .expect("anchor ships exactly one vacant eGlu slot");
    let (team, vac_rec, slots0) = (vacant.team_id, vacant.record, model0.slots.len());
    drop(db0);

    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let (pid, outcome) = tx.create_player(team, 2).unwrap();
    assert_eq!(
        outcome.edits, 10,
        "7 cluster rows + a clean occupant (XSWT + captaincy + jersey)"
    );
    let model = tx.model().unwrap();
    assert_eq!(
        model.slots.len(),
        slots0,
        "no eGlu growth — the vacant slot was reused"
    );
    let slot = model
        .slots
        .iter()
        .find(|s| s.player_id == Some(pid))
        .unwrap();
    assert_eq!(
        slot.record, vac_rec,
        "the vacant slot now carries the new player"
    );
    assert_eq!(
        slot.jersey,
        Some(0),
        "reused slot's stale jersey was cleared to unassigned"
    );
    drop(model);
    assert_write_clean(&tx.commit().unwrap());
}

/// Cloning a player produces a full duplicate cluster with a fresh identity on the target team: one
/// new bio + slot, companion invariant holds, audit-clean — and every fixed-bit bio field is
/// **deep-copied** from the source (except the overridden `wBIz` identity and `dXSB` team).
#[test]
fn clone_deep_copies_bio_and_companions() {
    let bytes = anchor(DB);
    let db0 = TdbFile::parse(&bytes).unwrap();
    let model0 = RosterModel::build(&db0).unwrap();
    let (players0, slots0) = (model0.players.len(), model0.slots.len());
    // A rostered skater as the source; clone onto a different team.
    let src_slot = model0
        .slots
        .iter()
        .find(|s| {
            s.player_id
                .is_some_and(|p| model0.player(p).is_some_and(|pl| pl.skater_ai.is_some()))
        })
        .unwrap();
    let (source, src_team) = (src_slot.player_id.unwrap(), src_slot.team_id);
    let to_team = model0
        .teams
        .iter()
        .map(|t| t.team_id)
        .find(|&t| t != src_team)
        .unwrap();
    drop(db0);

    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let (new_pid, _) = tx.clone_player(source, to_team).unwrap();
    assert_ne!(new_pid, source, "the clone gets a fresh wBIz");
    let out = tx.commit().unwrap();
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);

    let db = TdbFile::parse(&out).unwrap();
    let model = RosterModel::build(&db).unwrap();
    assert_eq!(model.players.len(), players0 + 1, "exactly one new bio");
    assert_eq!(model.slots.len(), slots0 + 1, "exactly one new slot");
    let clone = model.player(new_pid).unwrap();
    let src = model.player(source).unwrap();
    assert_eq!(clone.kind, src.kind, "same kind");
    assert_eq!(clone.position, src.position, "position deep-copied");
    assert!(
        clone.companions_complete(),
        "companion pair present on the clone"
    );
    assert!(
        model
            .slots
            .iter()
            .any(|s| s.player_id == Some(new_pid) && s.team_id == to_team),
        "the clone's slot is on the target team"
    );

    // Deep-copy proof: every fixed-bit bio field matches the source, except identity + team.
    let bio = db.table_canonical("ubPc").unwrap();
    for f in &bio.fields {
        let sid = f.id.canonical(db.endian);
        if sid == "wBIz" || sid == "dXSB" || f.bit_width == 0 || f.bit_width > 64 {
            continue;
        }
        assert_eq!(
            db.read_u64(bio, src.bio_record, f),
            db.read_u64(bio, clone.bio_record, f),
            "bio field {sid} should be deep-copied onto the clone"
        );
    }
}

/// Cloning a goalie duplicates the goalie companion pair (mHuy + fMVl), not the skater pair.
#[test]
fn clone_goalie_copies_the_goalie_pair() {
    let bytes = anchor(DB);
    let db0 = TdbFile::parse(&bytes).unwrap();
    let model0 = RosterModel::build(&db0).unwrap();
    let src_slot = model0
        .slots
        .iter()
        .find(|s| {
            s.player_id
                .is_some_and(|p| model0.player(p).is_some_and(|pl| pl.goalie_ai.is_some()))
        })
        .unwrap();
    let (source, src_team) = (src_slot.player_id.unwrap(), src_slot.team_id);
    let to_team = model0
        .teams
        .iter()
        .map(|t| t.team_id)
        .find(|&t| t != src_team)
        .unwrap();
    drop(db0);

    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let (new_pid, _) = tx.clone_player(source, to_team).unwrap();
    let out = tx.commit().unwrap();
    assert_write_clean(&out);
    let model = RosterModel::build(&TdbFile::parse(&out).unwrap()).unwrap();
    let clone = model.player(new_pid).unwrap();
    assert!(
        clone.goalie_ai.is_some() && clone.goalie_equip.is_some(),
        "goalie companions cloned"
    );
    assert!(
        clone.skater_ai.is_none() && clone.skater_equip.is_none(),
        "no skater companions"
    );
}

#[test]
fn clone_rejects_bad_args() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let (source, team) = {
        let model = tx.model().unwrap();
        let s = model.slots.iter().find(|s| s.player_id.is_some()).unwrap();
        (s.player_id.unwrap(), s.team_id)
    };
    assert!(matches!(
        tx.clone_player(0xFFFF_FFFF, team),
        Err(EditError::NoSuchPlayer(_))
    ));
    assert!(matches!(
        tx.clone_player(source, 0xFFFF_FFFF),
        Err(EditError::InvalidValue { .. })
    ));
}

#[test]
fn create_rejects_bad_args() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    assert!(matches!(
        tx.create_player(0, 9).unwrap_err(),
        EditError::InvalidValue {
            what: "position",
            ..
        }
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
