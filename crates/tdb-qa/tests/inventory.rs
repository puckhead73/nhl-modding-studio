//! Phase-0 drift guards. Every committed artifact is regenerated in memory and compared byte-for-byte
//! against what is checked in, so any of these fail loudly:
//!   * an anchor is added/removed/edited (manifest hash/row drift, snapshot count drift);
//!   * a schema changes (snapshot / diff drift);
//!   * a catalog entry loses fixture coverage;
//!   * the QA matrix is hand-edited into an ill-formed state.
//! The fix is always: `cargo run -p tdb-qa --bin qa-inventory` and review the diff.

use std::collections::BTreeSet;

use tdb_qa::catalog::{CATALOG, CATALOG_ENTRIES, FAMILIES, STATUS_VOCAB};
use tdb_qa::{diff, discover_main_anchors, manifest, matrix, repo_root_from_manifest, schema};

const REGEN_HINT: &str = "stale — run `cargo run -p tdb-qa --bin qa-inventory` and commit the diff";

fn read_committed(rel: &str) -> String {
    let path = repo_root_from_manifest().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("missing committed artifact {rel}: {e}"))
}

#[test]
fn anchor_manifest_matches_committed() {
    let repo = repo_root_from_manifest();
    let rows = manifest::build_rows(&repo).unwrap();
    let regen = manifest::to_csv(&rows);
    assert_eq!(regen, read_committed("docs/audits/data/anchors-manifest.csv"), "anchors-manifest.csv {REGEN_HINT}");
}

#[test]
fn schema_snapshots_match_committed() {
    let repo = repo_root_from_manifest();
    let anchors = discover_main_anchors(&repo).unwrap();
    assert_eq!(anchors.len(), 27, "expected 27 main-DB anchors, found {}", anchors.len());

    let mut expected_files = BTreeSet::new();
    for a in &anchors {
        let name = schema::filename(a);
        expected_files.insert(name.clone());
        let snap = schema::snapshot(&repo, a).unwrap();
        let regen = schema::to_json(&snap);
        assert_eq!(regen, read_committed(&format!("docs/audits/data/schema/{name}")), "schema/{name} {REGEN_HINT}");
    }

    // No superseded/extra snapshot files linger in the schema dir.
    let dir = repo.join("docs/audits/data/schema");
    let mut found = BTreeSet::new();
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        assert!(e.path().is_file(), "schema/ must be flat (found dir {:?})", e.path());
        found.insert(e.file_name().to_string_lossy().to_string());
    }
    assert_eq!(found, expected_files, "schema/ has extra/missing files {REGEN_HINT}");
}

#[test]
fn schema_diff_matches_committed() {
    let repo = repo_root_from_manifest();
    let anchors = discover_main_anchors(&repo).unwrap();
    let regen = diff::build_csv(&repo, &anchors).unwrap();
    assert_eq!(regen, read_committed("docs/audits/data/schema-diff.csv"), "schema-diff.csv {REGEN_HINT}");
}

#[test]
fn qa_matrix_matches_committed() {
    let repo = repo_root_from_manifest();
    let regen = matrix::build_csv(&repo);
    assert_eq!(regen, read_committed("docs/audits/data/game-qa-matrix.csv"), "game-qa-matrix.csv {REGEN_HINT}");
}

#[test]
fn qa_matrix_is_well_formed() {
    let csv = read_committed("docs/audits/data/game-qa-matrix.csv");
    let mut lines = csv.lines();
    let header = lines.next().unwrap();
    assert_eq!(
        header,
        "game,platform,generation,fixture,status,automated_test,manual_case,last_verified,evidence,blocker"
    );

    let mut seen_units: BTreeSet<(String, String)> = BTreeSet::new();
    let mut count = 0;
    for line in lines {
        // Columns 1..=5 (game,platform,generation,fixture,status) never contain commas; only the
        // trailing evidence/blocker cells are quoted, so a prefix split is safe for these fields.
        let cols: Vec<&str> = line.splitn(6, ',').collect();
        assert!(cols.len() >= 6, "row has too few columns: {line}");
        let (game, platform, status) = (cols[0], cols[1], cols[4]);
        assert!(!status.is_empty(), "blank status: {line}");
        assert!(STATUS_VOCAB.contains(&status), "status '{status}' not in vocab: {line}");
        assert_ne!(status, "game-verified", "Phase 0 must not assign game-verified: {line}");
        seen_units.insert((game.to_string(), platform.to_string()));
        count += 1;
    }
    assert_eq!(count, CATALOG.len() * FAMILIES.len(), "row count != units × families");

    // Every catalog unit is represented.
    let expected: BTreeSet<(String, String)> =
        CATALOG.iter().map(|u| (u.game.to_string(), u.platform.to_string())).collect();
    assert_eq!(seen_units, expected, "matrix units != catalog units");
}

#[test]
fn every_main_db_anchor_is_present_and_expected() {
    let repo = repo_root_from_manifest();
    let anchors = discover_main_anchors(&repo).unwrap();
    let found: BTreeSet<(String, String)> =
        anchors.iter().map(|a| (a.platform.clone(), a.game.clone())).collect();

    // The exact 27: PS3 ×10 (07–15, legacy), Xbox360 ×10 (07–15, legacy), PS4 ×7 (15–21).
    let mut expected: BTreeSet<(String, String)> = BTreeSet::new();
    for g in ["nhl07", "nhl08", "nhl09", "nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"] {
        expected.insert(("ps3".into(), g.into()));
        expected.insert(("xbox360".into(), g.into()));
    }
    for g in ["nhl15", "nhl16", "nhl17", "nhl18", "nhl19", "nhl20", "nhl21"] {
        expected.insert(("ps4".into(), g.into()));
    }
    assert_eq!(found, expected, "main-DB anchor inventory changed");
    // Every discovered anchor ships its metadata (the catalog's 27 are self-contained).
    for a in &anchors {
        assert!(a.meta_rel.is_some(), "{}/{} has no sibling metadata", a.platform, a.game);
    }
}

#[test]
fn every_catalog_entry_has_documented_coverage() {
    let repo = repo_root_from_manifest();
    // Each of the 17 catalog entries maps to at least one tracked unit.
    for entry in CATALOG_ENTRIES {
        assert!(CATALOG.iter().any(|u| u.game == *entry), "catalog entry {entry} has no tracked unit");
    }
    // Every unit either has its main-DB anchor on disk, or is the recomp entry that documents its
    // shared schema target (no independent anchor by design).
    for u in CATALOG {
        if u.platform == "recomp" {
            assert!(u.schema_target.starts_with("(shared:"), "recomp unit must document a shared schema target");
        } else {
            assert!(repo.join(u.schema_target).is_file(), "unit {}/{} anchor missing: {}", u.platform, u.game, u.schema_target);
        }
    }
}

#[test]
fn ps3_and_xbox360_gen7_schemas_are_identical() {
    // The manifest's core claim, verified structurally: no field differs PS3 vs Xbox 360 per game.
    let diff = read_committed("docs/audits/data/schema-diff.csv");
    let parity_rows = diff.lines().filter(|l| l.starts_with("platform-parity,")).count();
    assert_eq!(parity_rows, 0, "PS3 and Xbox360 gen7 schemas diverge — see schema-diff.csv platform-parity rows");
}
