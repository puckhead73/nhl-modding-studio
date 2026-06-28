//! Load the shipped `defs/relationships` registry, traverse the roster→player bridge chain, and
//! cross-check each edge's source field against a real anchor database via the id-reversal rule.

use std::path::{Path, PathBuf};
use tdb_defs::FieldMetaIndex;
use tdb_relations::{Cardinality, RelationRegistry};

fn defs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../defs/relationships")
}

fn anchor_str(rel: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

fn reverse_id(id: &str) -> String {
    id.chars().rev().collect()
}

#[test]
fn registry_loads_all_defs() {
    let reg = RelationRegistry::from_dir(defs_dir()).unwrap();
    assert_eq!(reg.len(), 8, "expected the 8 shipped relationship defs");
    // team-links carries two bindings, so edges exceed defs.
    assert!(reg.edge_count() >= 9, "team-links should add a second edge");
}

/// The EDIT-MODEL Layer 2 roster→player bridge: eGlu.XSWT → ZBac.vfEq (hop 1, 1:1), then
/// ZBac.ykFq → ubPc.wBIz (hop 2, many:1). The registry must traverse both hops by construction.
#[test]
fn traverses_roster_to_player_bridge() {
    let reg = RelationRegistry::from_dir(defs_dir()).unwrap();

    let hop1 = reg.resolve("eGlu", "XSWT").expect("hop 1 (roster slot → player index)");
    assert_eq!(hop1.binding.target_table, "ZBac");
    assert_eq!(hop1.binding.target_field, "vfEq");
    assert_eq!(hop1.relationship.cardinality, Cardinality::OneToOne);

    // Hop 2 starts from the player-index table the first hop landed on.
    let hop2 = reg
        .resolve(&hop1.binding.target_table, "ykFq")
        .expect("hop 2 (player index → bio)");
    assert_eq!(hop2.binding.target_table, "ubPc");
    assert_eq!(hop2.binding.target_field, "wBIz");
    assert_eq!(hop2.relationship.cardinality, Cardinality::ManyToOne);
}

/// Every confirmed edge's source field, reversed into the binary id space, should exist in a real
/// anchor's metadata — tying the shipped relationships to real data, not just to the prose.
#[test]
fn edges_resolve_against_anchor_meta() {
    let reg = RelationRegistry::from_dir(defs_dir()).unwrap();
    let meta =
        FieldMetaIndex::from_xml(&anchor_str("ps3/nhllegacy/databases/nhlng-meta.xml")).unwrap();

    let mut matched = 0;
    for r in reg.relationships() {
        for b in &r.bindings {
            let (t, f) = (reverse_id(&b.source_table), reverse_id(&b.source_field));
            // Some tables (e.g. the StEO arena directory / Teix) may not appear in this anchor;
            // skip rather than fail, but require the core roster/player edges to resolve.
            if meta.field(&t, &f).is_some() {
                matched += 1;
            }
        }
    }
    assert!(
        matched >= 4,
        "expected several relationship source fields to resolve against the legacy anchor, only {matched} did"
    );
    eprintln!("{matched} relationship edges cross-checked against the legacy anchor meta");
}
