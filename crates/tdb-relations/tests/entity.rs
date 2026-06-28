//! Assemble the Layer 2 entity model from a real committed anchor (PS3 NHL Legacy `nhlng.db`) and
//! verify the cluster, the companion invariant, and the roster→player bridge resolve as confirmed.

use std::path::Path;
use tdb_core::TdbFile;
use tdb_relations::{PlayerKind, Position, RosterModel};

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors").join(rel)).unwrap()
}

#[test]
fn assembles_cluster_with_verified_counts() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    // Anchor populations confirmed by the cluster probe.
    assert_eq!(model.teams.len(), 252, "kOtt teams");
    assert_eq!(model.players.len(), 5632, "ubPc players");
    assert_eq!(model.slots.len(), 5532, "eGlu roster slots");
    assert_eq!(model.skater_count(), 5066, "skaters (vjla != 4)");
    assert_eq!(model.goalie_count(), 566, "goalies (vjla == 4)");
}

/// Every player's companion pair matches its kind — the "a Player is the whole cluster" invariant
/// holds with zero violations on vanilla data.
#[test]
fn companion_invariant_holds() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let violations = model.companion_violations();
    assert!(
        violations.is_empty(),
        "{} players have an incomplete companion cluster (first: {:?})",
        violations.len(),
        violations.first()
    );

    // Spot-check the shape: a skater has the skater pair and no goalie pair, and vice versa.
    let skater = model
        .players
        .iter()
        .find(|p| p.kind == PlayerKind::Skater)
        .unwrap();
    assert!(skater.skater_ai.is_some() && skater.skater_equip.is_some());
    assert!(skater.goalie_ai.is_none() && skater.goalie_equip.is_none());

    let goalie = model
        .players
        .iter()
        .find(|p| p.kind == PlayerKind::Goalie)
        .unwrap();
    assert_eq!(goalie.position, Position::Goalie);
    assert!(goalie.goalie_ai.is_some() && goalie.goalie_equip.is_some());
    assert!(goalie.skater_ai.is_none() && goalie.skater_equip.is_none());
}

/// The roster→player two-hop bridge resolves for the overwhelming majority of slots, and each
/// resolved slot also resolves to a real team — the typed graph is connected by construction.
#[test]
fn slots_bridge_to_players_and_teams() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let mut populated = 0;
    let mut player_resolved = 0;
    let mut team_resolved = 0;
    for slot in &model.slots {
        if slot.index_key.is_some() {
            populated += 1;
            if model.slot_player(slot).is_some() {
                player_resolved += 1;
            }
            if model.slot_team(slot).is_some() {
                team_resolved += 1;
            }
        }
    }
    assert!(populated > 5000, "most eGlu slots reference a player index");
    let player_rate = player_resolved as f64 / populated as f64;
    let team_rate = team_resolved as f64 / populated as f64;
    eprintln!(
        "bridge: {player_resolved}/{populated} players ({player_rate:.4}), {team_resolved}/{populated} teams ({team_rate:.4})"
    );
    assert!(player_rate >= 0.99, "player bridge only {player_rate:.4}");
    assert!(team_rate >= 0.99, "team bridge only {team_rate:.4}");

    // A concrete traversal: pick a resolved slot, walk slot → player → companions.
    let slot = model
        .slots
        .iter()
        .find(|s| model.slot_player(s).is_some())
        .unwrap();
    let player = model.slot_player(slot).unwrap();
    assert!(player.companions_complete());
    assert!(model.slot_team(slot).is_some());
}
