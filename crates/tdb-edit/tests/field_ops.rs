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

/// Trading two players between (full) teams exchanges their occupants **in place** — each takes the
/// other's slot, so both per-team blocks stay coherent and no free slot is consumed — clears captaincy
/// on both arrivals, and leaves the db audit-clean. This is the block-full alternative to a move.
#[test]
fn trade_swaps_occupants_in_place() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    let model = tx.model().unwrap();
    let a = model.slots.iter().find(|s| s.player_id.is_some()).unwrap();
    let (a_team, a_player, a_rec) = (a.team_id, a.player_id.unwrap(), a.record);
    let b = model
        .slots
        .iter()
        .find(|s| s.player_id.is_some() && s.team_id != a_team)
        .unwrap();
    let (b_team, b_player, b_rec) = (b.team_id, b.player_id.unwrap(), b.record);
    assert_ne!(a_player, b_player);
    drop(model);

    tx.trade_slots(a_team, a_player, b_team, b_player).unwrap();

    let model = tx.model().unwrap();
    // Each player now occupies the *other's* old slot, on the other team (slot team/key untouched).
    let slot_a = model.slots.iter().find(|s| s.record == a_rec).unwrap();
    let slot_b = model.slots.iter().find(|s| s.record == b_rec).unwrap();
    assert_eq!(slot_a.team_id, a_team, "a's slot keeps its team");
    assert_eq!(slot_a.player_id, Some(b_player), "b_player now sits in a's old slot");
    assert_eq!(slot_b.team_id, b_team, "b's slot keeps its team");
    assert_eq!(slot_b.player_id, Some(a_player), "a_player now sits in b's old slot");
    assert_ne!(slot_a.captaincy, Some(captaincy::CAPTAIN), "captaincy cleared on arrival");
    assert_ne!(slot_b.captaincy, Some(captaincy::CAPTAIN), "captaincy cleared on arrival");
    drop(model);

    let out = tx.commit().unwrap();
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);
    assert_blocks_coherent(&out);
}

/// A traded player whose jersey collides with a *staying* player on the destination team is
/// reassigned the lowest free number on arrival (dup-on-team repair) — the trade still commits
/// audit-clean and the staying player keeps their number.
#[test]
fn trade_reassigns_a_colliding_jersey_on_arrival() {
    use std::collections::HashSet;
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    let model = tx.model().unwrap();
    let a = model.slots.iter().find(|s| s.player_id.is_some()).unwrap();
    let (a_team, a_player) = (a.team_id, a.player_id.unwrap());
    let a_jerseys: HashSet<u64> = model
        .slots
        .iter()
        .filter(|s| s.team_id == a_team)
        .filter_map(|s| s.jersey)
        .collect();
    let b = model
        .slots
        .iter()
        .find(|s| s.player_id.is_some() && s.team_id != a_team)
        .unwrap();
    let (b_team, b_player) = (b.team_id, b.player_id.unwrap());
    // A player staying on b_team whose (non-zero) number a_team does not already use.
    let clash = model
        .slots
        .iter()
        .find(|s| {
            s.team_id == b_team
                && s.player_id.is_some()
                && s.player_id != Some(b_player)
                && s.jersey.is_some_and(|j| j != 0 && !a_jerseys.contains(&j))
        })
        .unwrap()
        .jersey
        .unwrap();
    drop(model);

    // Make a_player wear the clashing number (free on a_team), then trade into b_team.
    tx.set_jersey_number(a_team, a_player, clash).unwrap();
    let outcome = tx.trade_slots(a_team, a_player, b_team, b_player).unwrap();
    assert!(
        outcome.repairs.iter().any(|r| r.contains(&format!("#{clash}"))),
        "expected a jersey-collision repair, got {:?}",
        outcome.repairs
    );

    let model = tx.model().unwrap();
    let arrived = model
        .slots
        .iter()
        .find(|s| s.team_id == b_team && s.player_id == Some(a_player))
        .unwrap();
    let assigned = arrived.jersey.expect("arrived slot has a jersey");
    assert_ne!(assigned, 0, "the colliding jersey was reassigned, not cleared");
    assert_ne!(assigned, clash, "the reassigned number differs from the clashing one");
    // Unique within the destination team (the staying player keeps #clash).
    assert!(
        !model
            .slots
            .iter()
            .any(|s| s.team_id == b_team
                && s.record != arrived.record
                && s.index_key.is_some()
                && s.jersey == Some(assigned)),
        "the reassigned number is unique on the destination team"
    );
    drop(model);
    assert_write_clean(&tx.commit().unwrap());
}

#[test]
fn trade_rejects_bad_args() {
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let model = tx.model().unwrap();
    let s = model.slots.iter().find(|s| s.player_id.is_some()).unwrap();
    let (team, player) = (s.team_id, s.player_id.unwrap());
    let other = model
        .slots
        .iter()
        .find(|s| s.player_id.is_some() && s.team_id != team)
        .unwrap();
    let (other_team, other_player) = (other.team_id, other.player_id.unwrap());
    drop(model);

    // Unknown team.
    assert!(matches!(
        tx.trade_slots(0xFFFF_FFFF, player, other_team, other_player),
        Err(EditError::InvalidValue { .. })
    ));
    // Player not actually on the named team.
    assert!(matches!(
        tx.trade_slots(other_team, player, team, other_player),
        Err(EditError::NoSuchSlot { .. })
    ));
}

/// Releasing a player to free agency then signing them back round-trips them onto a team: release
/// vacates their slot (off-roster = referenced by no slot, keeping their bio/index), and the sign
/// reuses a free slot in the team's block. Both blocks stay coherent and the db stays audit-clean.
#[test]
fn release_then_sign_round_trips_a_player() {
    use std::collections::HashMap;
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    // A player with exactly one membership, so releasing it makes them a true free agent.
    let model = tx.model().unwrap();
    let mut counts: HashMap<u64, usize> = HashMap::new();
    for s in &model.slots {
        if let Some(p) = s.player_id {
            *counts.entry(p).or_default() += 1;
        }
    }
    let s = model.slots.iter().find(|s| s.player_id.is_some_and(|p| counts[&p] == 1)).unwrap();
    let (team, player, rec) = (s.team_id, s.player_id.unwrap(), s.record);
    drop(model);

    tx.release_to_fa(team, player).unwrap();
    let model = tx.model().unwrap();
    assert!(!model.slots.iter().any(|s| s.player_id == Some(player)), "off-roster after release");
    let vacated = model.slots.iter().find(|s| s.record == rec).unwrap();
    assert_eq!(vacated.team_id, team, "vacated slot keeps its team (still in the block)");
    assert!(vacated.player_id.is_none(), "vacated slot is empty");
    assert!(model.player(player).is_some(), "the free agent keeps their bio/identity");
    drop(model);

    tx.sign_from_fa(player, team).unwrap();
    let model = tx.model().unwrap();
    let signed = model.slots.iter().find(|s| s.player_id == Some(player)).unwrap();
    assert_eq!(signed.team_id, team, "player is back on the team after signing");
    drop(model);

    let out = tx.commit().unwrap();
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);
    assert_blocks_coherent(&out);
}

/// Signing a free agent into a full team block (no free slot) errors instead of corrupting the block
/// — block growth is deferred capacity work (Phase 4.4).
#[test]
fn sign_into_full_team_is_rejected() {
    use std::collections::HashSet;
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    let model = tx.model().unwrap();
    let rostered: HashSet<u64> = model.slots.iter().filter_map(|s| s.player_id).collect();
    let fa = model
        .players
        .iter()
        .map(|p| p.player_id)
        .find(|p| !rostered.contains(p))
        .expect("the anchor carries off-roster players (Spike S1)");
    let free_team = model.slots.iter().find(|s| s.player_id.is_none()).unwrap().team_id;
    let full_team = model
        .teams
        .iter()
        .map(|t| t.team_id)
        .find(|&t| {
            t != free_team
                && !model.team_roster(t).is_empty()
                && model.team_roster(t).iter().all(|s| s.player_id.is_some())
        })
        .unwrap();
    drop(model);

    assert!(matches!(
        tx.sign_from_fa(fa, full_team),
        Err(EditError::TeamRosterFull { .. })
    ));
}

#[test]
fn fa_ops_reject_bad_args() {
    use std::collections::HashSet;
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();
    let model = tx.model().unwrap();
    let player = model.slots.iter().find_map(|s| s.player_id).unwrap();
    let player_teams: HashSet<u64> =
        model.slots.iter().filter(|s| s.player_id == Some(player)).map(|s| s.team_id).collect();
    let other_team = model.teams.iter().map(|t| t.team_id).find(|t| !player_teams.contains(t)).unwrap();
    let free_team = model.slots.iter().find(|s| s.player_id.is_none()).unwrap().team_id;
    drop(model);

    // release: unknown team, and a player not on the named team.
    assert!(matches!(
        tx.release_to_fa(0xFFFF_FFFF, player),
        Err(EditError::InvalidValue { .. })
    ));
    assert!(matches!(
        tx.release_to_fa(other_team, player),
        Err(EditError::NoSuchSlot { .. })
    ));
    // sign: an already-rostered player is not a free agent.
    assert!(matches!(
        tx.sign_from_fa(player, free_team),
        Err(EditError::AlreadyRostered { .. })
    ));
}

/// Adding an extra membership installs the player into a free slot on a second team while their
/// original slot (and primary team) stays put; removing it vacates only that new slot. The
/// additive path never touches the canonical membership, and the blocks stay coherent throughout.
#[test]
fn add_then_remove_extra_membership_round_trips() {
    use std::collections::HashMap;
    let bytes = anchor(DB);
    let mut tx = EditTransaction::begin(&bytes).unwrap();

    // A player with exactly one membership, and a different team with a free slot.
    let model = tx.model().unwrap();
    let mut counts: HashMap<u64, usize> = HashMap::new();
    for s in &model.slots {
        if let Some(p) = s.player_id {
            *counts.entry(p).or_default() += 1;
        }
    }
    let s = model.slots.iter().find(|s| s.player_id.is_some_and(|p| counts[&p] == 1)).unwrap();
    let (home_team, player, home_rec) = (s.team_id, s.player_id.unwrap(), s.record);
    let extra_team = model
        .slots
        .iter()
        .find(|s| s.player_id.is_none() && s.team_id != home_team)
        .unwrap()
        .team_id;
    drop(model);

    tx.add_membership(player, extra_team).unwrap();
    let model = tx.model().unwrap();
    let home = model.slots.iter().find(|s| s.record == home_rec).unwrap();
    assert_eq!(home.player_id, Some(player), "original membership untouched by the add");
    let extra = model
        .slots
        .iter()
        .find(|s| s.team_id == extra_team && s.player_id == Some(player))
        .expect("player holds a slot on the extra team");
    assert_eq!(extra.captaincy, Some(captaincy::NONE), "extra membership arrives captainless");
    drop(model);

    // One membership per team: adding the same team again is rejected.
    assert!(matches!(
        tx.add_membership(player, extra_team),
        Err(EditError::InvalidValue { .. })
    ));

    tx.remove_membership(extra_team, player).unwrap();
    let model = tx.model().unwrap();
    assert!(
        !model.slots.iter().any(|s| s.team_id == extra_team && s.player_id == Some(player)),
        "extra membership vacated"
    );
    let home = model.slots.iter().find(|s| s.record == home_rec).unwrap();
    assert_eq!(home.player_id, Some(player), "original membership survives the remove");
    drop(model);
    // Removing a membership the player doesn't hold is a structured error.
    assert!(matches!(
        tx.remove_membership(extra_team, player),
        Err(EditError::NoSuchSlot { .. })
    ));

    let out = tx.commit().unwrap();
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);
    assert_blocks_coherent(&out);
}

/// Total rows in the retired-number table (`wcSL`).
fn retired_count(db_bytes: &[u8]) -> usize {
    TdbFile::parse(db_bytes).unwrap().table_canonical("wcSL").unwrap().current_records as usize
}

/// Whether `(team, number)` is present in the retired-number table.
fn is_retired(db_bytes: &[u8], team: u64, number: u64) -> bool {
    let db = TdbFile::parse(db_bytes).unwrap();
    let t = db.table_canonical("wcSL").unwrap();
    let f = |n: &str| t.fields.iter().find(|f| f.id.canonical(db.endian) == n).unwrap();
    let (tf, nf) = (f("dXSB"), f("JFiY"));
    (0..t.current_records as u32)
        .any(|i| db.read_u64(t, i, tf) == Some(team) && db.read_u64(t, i, nf) == Some(number))
}

/// Retiring a number adds a `(team, number)` row; un-retiring removes it — round-tripping the
/// retired-number table row count, and staying audit-clean.
#[test]
fn retire_then_unretire_round_trips() {
    let base = anchor(DB);
    let team = RosterModel::build(&TdbFile::parse(&base).unwrap()).unwrap().teams[0].team_id;
    let number = (1u64..=99).find(|&n| !is_retired(&base, team, n)).unwrap();
    let base_count = retired_count(&base);

    let mut tx = EditTransaction::begin(&base).unwrap();
    let o = tx.retire_jersey(team, number).unwrap();
    assert_eq!(o.edits, 2, "retire appends one (team, number) row");
    let out = tx.commit().unwrap();
    assert!(is_retired(&out, team, number), "number is now retired");
    assert_eq!(retired_count(&out), base_count + 1);
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
    assert_write_clean(&out);

    let mut tx2 = EditTransaction::begin(&out).unwrap();
    let o2 = tx2.unretire_jersey(team, number).unwrap();
    assert_eq!(o2.edits, 1, "un-retire removes the row");
    let out2 = tx2.commit().unwrap();
    assert!(!is_retired(&out2, team, number), "number is no longer retired");
    assert_eq!(retired_count(&out2), base_count, "row count back to baseline");
    assert_write_clean(&out2);
}

/// Trading a draft pick rewrites the pick's current owner (`qHav.Uwdb`) while leaving the
/// original-owner provenance (`GFcg`) untouched, commits audit-clean, and validates its args.
#[test]
fn trade_draft_pick_moves_owner_and_keeps_provenance() {
    let base = anchor(DB);
    let db = TdbFile::parse(&base).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let (team_a, team_b) = (model.teams[0].team_id, model.teams[1].team_id);

    // A pick currently owned by team A (Uwdb stores the team 1-indexed).
    let qhav = db.table_canonical("qHav").unwrap();
    let f = |id: &str| qhav.fields.iter().find(|f| f.id.canonical(db.endian) == id).unwrap();
    let (owner_f, orig_f) = (f("Uwdb"), f("GFcg"));
    let record = (0..qhav.current_records as u32)
        .find(|&i| db.read_u64(qhav, i, owner_f) == Some(team_a + 1))
        .expect("team A owns a pick in the anchor");
    let orig_before = db.read_u64(qhav, record, orig_f).unwrap();

    let mut tx = EditTransaction::begin(&base).unwrap();
    let o = tx.trade_draft_pick(record, team_b).unwrap();
    assert_eq!(o.edits, 1);
    let out = tx.commit().unwrap();
    let out_db = TdbFile::parse(&out).unwrap();
    let out_t = out_db.table_canonical("qHav").unwrap();
    let of = |id: &str| out_t.fields.iter().find(|f| f.id.canonical(out_db.endian) == id).unwrap();
    assert_eq!(
        out_db.read_u64(out_t, record, of("Uwdb")),
        Some(team_b + 1),
        "current owner moved to team B"
    );
    assert_eq!(
        out_db.read_u64(out_t, record, of("GFcg")),
        Some(orig_before),
        "original-owner provenance untouched"
    );
    assert_write_clean(&out);

    // Bad args: unknown team, out-of-range record.
    let mut tx2 = EditTransaction::begin(&base).unwrap();
    assert!(matches!(
        tx2.trade_draft_pick(record, 0xFFFF_FFFF),
        Err(EditError::InvalidValue { what: "to_team (no such team)", .. })
    ));
    assert!(matches!(
        tx2.trade_draft_pick(0xFFFF_FFFF, team_b),
        Err(EditError::InvalidValue { what: "draft-pick record (out of range)", .. })
    ));
}

/// Retire is idempotent (already-retired → no-op), un-retire is a no-op for a number that wasn't
/// retired, and both validate their arguments.
#[test]
fn retire_is_idempotent_and_validates() {
    let base = anchor(DB);
    let team = RosterModel::build(&TdbFile::parse(&base).unwrap()).unwrap().teams[0].team_id;
    let number = (1u64..=99).find(|&n| !is_retired(&base, team, n)).unwrap();
    let mut tx = EditTransaction::begin(&base).unwrap();

    tx.retire_jersey(team, number).unwrap();
    let again = tx.retire_jersey(team, number).unwrap();
    assert_eq!(again.edits, 0, "already retired → no-op");

    // A number never retired for the team → un-retire is a no-op.
    let free = (1u64..=99).find(|&n| n != number && !is_retired(&base, team, n)).unwrap();
    let noop = tx.unretire_jersey(team, free).unwrap();
    assert_eq!(noop.edits, 0, "un-retiring a never-retired number is a no-op");

    // Bad args: unknown team, out-of-range number (JFiY is 7-bit → max 127).
    assert!(matches!(
        tx.retire_jersey(0xFFFF_FFFF, 10),
        Err(EditError::InvalidValue { what: "team (no such team)", .. })
    ));
    assert!(matches!(
        tx.retire_jersey(team, 9999),
        Err(EditError::InvalidValue { what: "retired jersey number", .. })
    ));
}

/// Reproduce structural block corruption (the old `dXSB`-retag bug's fingerprint), then confirm the
/// edit guard blocks *any* edit by default — and that the acknowledged override lifts the guard so an
/// edit commits on the corrupt DB (the on-load "Override" path).
#[test]
fn edit_override_allows_edits_on_a_structurally_corrupt_db() {
    use std::collections::HashMap;
    let bytes = anchor(DB);
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    // Retag an interior slot of a >=3-slot block to a foreign team → breaks block coherence.
    let mut by_team: HashMap<u64, Vec<u32>> = HashMap::new();
    for s in &model.slots {
        by_team.entry(s.team_id).or_default().push(s.record);
    }
    let (&donor, recs) = by_team.iter().find(|(_, r)| r.len() >= 3).unwrap();
    let mut recs = recs.clone();
    recs.sort_unstable();
    let victim = recs[recs.len() / 2];
    let foreign = *by_team.keys().find(|&&t| t != donor).unwrap();
    // A valid occupied slot on the foreign team to target with an unrelated edit.
    let target = model
        .slots
        .iter()
        .find(|s| s.team_id == foreign && s.player_id.is_some())
        .unwrap();
    let (t_team, t_player) = (target.team_id, target.player_id.unwrap());
    let eglu = db.table_canonical("eGlu").unwrap();
    let dxsb = eglu.fields.iter().find(|f| f.id.canonical(db.endian) == "dXSB").unwrap();
    drop(model);
    let mut buf = bytes.clone();
    assert!(db.set_u64(&mut buf, eglu, victim, dxsb, foreign));
    db.recompute_crcs(&mut buf);
    drop(db);

    // Default: the pre-existing block corruption trips the whole-DB audit, rolling back the edit.
    let mut tx = EditTransaction::begin(&buf).unwrap();
    let blocked = tx.set_captaincy(t_team, t_player, captaincy::CAPTAIN);
    assert!(
        matches!(blocked, Err(EditError::AuditFailed { predicate: "towx_coherence", .. })),
        "expected the structural audit to block the edit, got {blocked:?}"
    );

    // Override: the same edit now commits (unsupported, acknowledged).
    let mut tx = EditTransaction::begin(&buf).unwrap();
    tx.set_allow_pre_existing_violations(true);
    let allowed = tx.set_captaincy(t_team, t_player, captaincy::CAPTAIN);
    assert!(allowed.is_ok(), "override should let the edit through, got {allowed:?}");
    assert!(TdbFile::parse(&tx.commit().unwrap()).is_ok(), "committed bytes still parse");
}

/// The on-load auto-correct reassigns duplicate jersey numbers: force two occupied slots on one team
/// to share a (previously free) number, then confirm the first slot keeps it, the later slot is
/// renumbered to a different unique value, and the DB is left write-clean.
#[test]
fn autocorrect_reassigns_duplicate_jerseys() {
    use std::collections::{HashMap, HashSet};
    let bytes = anchor(DB);
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    // A team with >=2 occupied slots, and a jersey number not currently worn on it.
    let mut by_team: HashMap<u64, Vec<(u32, Option<u64>)>> = HashMap::new();
    for s in &model.slots {
        if s.index_key.is_some() {
            by_team.entry(s.team_id).or_default().push((s.record, s.jersey));
        }
    }
    let (_, slots) = by_team.iter().find(|(_, v)| v.len() >= 2).unwrap();
    let worn: HashSet<u64> = slots.iter().filter_map(|(_, j)| *j).collect();
    let free = (1..=127u64).find(|n| !worn.contains(n)).unwrap();
    let mut recs: Vec<u32> = slots.iter().map(|(r, _)| *r).collect();
    recs.sort_unstable();
    let (keep_rec, fix_rec) = (recs[0], recs[1]);

    let eglu = db.table_canonical("eGlu").unwrap();
    let svrt = eglu.fields.iter().find(|f| f.id.canonical(db.endian) == "sVRt").unwrap();
    drop(model);
    let mut buf = bytes.clone();
    assert!(db.set_u64(&mut buf, eglu, keep_rec, svrt, free));
    assert!(db.set_u64(&mut buf, eglu, fix_rec, svrt, free));
    db.recompute_crcs(&mut buf);
    drop(db);

    let mut tx = EditTransaction::begin(&buf).unwrap();
    let (_, corrections) = tx.autocorrect_duplicate_jerseys().unwrap();
    assert_eq!(corrections.len(), 1, "exactly one duplicate to fix, got {corrections:?}");
    assert_eq!(corrections[0].record, fix_rec, "the later slot is renumbered; the first keeps it");
    assert_eq!(corrections[0].old, free);
    assert_ne!(corrections[0].new, free);

    let out = tx.commit().unwrap();
    assert_write_clean(&out);
    let db2 = TdbFile::parse(&out).unwrap();
    let m2 = RosterModel::build(&db2).unwrap();
    assert_eq!(m2.slots.iter().find(|s| s.record == keep_rec).unwrap().jersey, Some(free));
    assert_ne!(m2.slots.iter().find(|s| s.record == fix_rec).unwrap().jersey, Some(free));
}
