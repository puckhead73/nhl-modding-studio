//! Layer 3 field operations (move/trade, jersey, roster status) against the NHL Legacy anchor.

use std::path::{Path, PathBuf};
use tdb_core::TdbFile;
use tdb_domain_validation::validate_invariants;
use tdb_edit::txn::{captaincy, EditTransaction};
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

fn assert_write_clean(db_bytes: &[u8]) {
    let db = TdbFile::parse(db_bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();
    let enums = EnumRegistry::from_dir(defs("enums")).unwrap();
    let report = validate_invariants(&db, &relations, &enums);
    assert!(report.is_clean(), "audit not clean: {:?}", report.errors().take(5).collect::<Vec<_>>());
}

/// Roster slots are laid out as contiguous, gap-free, non-overlapping per-team blocks (see
/// `docs/audits/towx-blocks.md`). Assert that invariant still holds — the *old* `move_player`
/// (which retagged a slot's `dXSB`) would break it.
fn assert_blocks_coherent(db_bytes: &[u8]) {
    use std::collections::HashMap;
    let db = TdbFile::parse(db_bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let mut by_team: HashMap<u64, Vec<u32>> = HashMap::new();
    for s in &model.slots {
        by_team.entry(s.team_id).or_default().push(s.record);
    }
    let mut ranges: Vec<(u32, u32, u64)> = Vec::new();
    for (team, mut recs) in by_team {
        recs.sort_unstable();
        let (min, max) = (*recs.first().unwrap(), *recs.last().unwrap());
        assert_eq!(
            (max - min) as usize + 1,
            recs.len(),
            "team {team} block is not contiguous (records {min}..={max}, count {})",
            recs.len()
        );
        ranges.push((min, max, team));
    }
    ranges.sort();
    for w in ranges.windows(2) {
        assert!(
            w[1].0 > w[0].1,
            "team blocks overlap: team {} [{}..={}] vs team {} [{}..={}]",
            w[0].2, w[0].0, w[0].1, w[1].2, w[1].0, w[1].1
        );
    }
}

/// Moving a captain relocates their occupancy into a **free slot in the destination team's block**
/// (without retagging any slot's team), vacates the source slot, clears captaincy on arrival, and
/// leaves the db audit-clean **and block-coherent**.
#[test]
fn move_relocates_into_free_slot_and_preserves_blocks() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    let model = tx.model().unwrap();
    // Destination = a team that actually has a free (unoccupied) slot to relocate into.
    let to_team = model.slots.iter().find(|s| s.player_id.is_none()).unwrap().team_id;
    // Members per player — pick a captain with exactly ONE membership so the move is unambiguous
    // (players can hold several memberships across teams; that case is move-by-from_team's job).
    let mut counts: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for s in &model.slots {
        if let Some(p) = s.player_id {
            *counts.entry(p).or_default() += 1;
        }
    }
    let cap = model
        .slots
        .iter()
        .find(|s| {
            s.captaincy == Some(captaincy::CAPTAIN)
                && s.player_id.is_some_and(|p| counts[&p] == 1)
                && s.team_id != to_team
        })
        .unwrap();
    let (from_team, player) = (cap.team_id, cap.player_id.unwrap());
    let source_record = cap.record;
    drop(model);

    let outcome = tx.move_player(player, from_team, to_team).unwrap();
    assert!(outcome.repairs.iter().any(|r| r.contains("cleared captaincy")));

    let model = tx.model().unwrap();
    let slot = model.slots.iter().find(|s| s.player_id == Some(player)).unwrap();
    assert_eq!(slot.team_id, to_team, "player now on the destination team");
    assert_ne!(slot.record, source_record, "player relocated to a different (destination-block) slot");
    assert_ne!(slot.captaincy, Some(captaincy::CAPTAIN), "captaincy cleared on move");
    assert!(model.team_roster(to_team).iter().any(|s| s.player_id == Some(player)));
    assert!(!model.team_roster(from_team).iter().any(|s| s.player_id == Some(player)));
    // The source slot is now vacant but still belongs to the source team (no team retag).
    let src = model.slots.iter().find(|s| s.record == source_record).unwrap();
    assert_eq!(src.team_id, from_team, "source slot keeps its team");
    assert!(src.player_id.is_none(), "source slot vacated");
    drop(model);

    let out = tx.commit().unwrap();
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);
    assert_blocks_coherent(&out);
}

/// Moving into a full team block (no free slot) errors instead of corrupting the block.
#[test]
fn move_into_full_team_is_rejected() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    let model = tx.model().unwrap();
    let free_team = model.slots.iter().find(|s| s.player_id.is_none()).unwrap().team_id;
    // A full destination team (every slot occupied) that is not the player's own team.
    let full_team = model
        .teams
        .iter()
        .map(|t| t.team_id)
        .find(|&t| t != free_team && model.team_roster(t).iter().all(|s| s.player_id.is_some()))
        .unwrap();
    // A player on some other team.
    let src = model
        .slots
        .iter()
        .find(|s| s.player_id.is_some() && s.team_id != full_team)
        .unwrap();
    let (player, from_team) = (src.player_id.unwrap(), src.team_id);
    drop(model);

    assert!(matches!(
        tx.move_player(player, from_team, full_team),
        Err(EditError::TeamRosterFull { .. })
    ));
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
        tx.move_player(player, 0, 0xFFFF_FFFF),
        Err(EditError::InvalidValue { .. })
    ));
    assert!(matches!(
        tx.move_player(0xFFFF_FFFF, 0, 0),
        Err(EditError::NoSuchPlayer(_))
    ));
}
