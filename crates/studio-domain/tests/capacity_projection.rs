//! Phase 5.4 follow-up — the S4 ceiling gate: `project_capacity` reports per-table headroom against
//! the vanilla ceiling and flags a storage blocker only when the pending creates would cross it.

use std::path::{Path, PathBuf};

use studio_domain::{project_capacity, PendingCreates};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn anchor_bytes() -> Vec<u8> {
    std::fs::read(repo("anchors/ps3/nhl14/databases/nhlng.db")).unwrap()
}

#[test]
fn small_cart_fits_but_a_giant_one_blocks() {
    let bytes = anchor_bytes();
    let db = TdbFile::parse(&bytes).unwrap();

    // A normal team-season worth of creates is comfortably under every ceiling (S4: headroom exists).
    let small = project_capacity(
        &db,
        Some("nhl14"),
        PendingCreates {
            skaters: 20,
            goalies: 2,
        },
    );
    assert!(!small.is_empty(), "roster tables are present on nhl14");
    assert!(
        small.iter().all(|c| c.enforced),
        "nhl14 has a known GEN7 ceiling for every table"
    );
    assert!(
        small.iter().all(|c| !c.over),
        "a 22-player cart fits: {:?}",
        small
            .iter()
            .filter(|c| c.over)
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
    // The bio table's ceiling is the audited nhl14 value.
    let bio = small.iter().find(|c| c.table == "ubPc").unwrap();
    assert_eq!(
        bio.ceiling, 6873,
        "nhl14 ubPc ceiling (docs/audits/roster-capacity.md)"
    );
    assert_eq!(bio.adding, 22, "both skaters and goalies add a bio row");

    // A whole-league flood of skaters overflows the skater-bound tables (bio binds first).
    let huge = project_capacity(
        &db,
        Some("nhl14"),
        PendingCreates {
            skaters: 100_000,
            goalies: 0,
        },
    );
    let bio = huge.iter().find(|c| c.table == "ubPc").unwrap();
    assert!(bio.over, "100k skaters exceed the bio ceiling");
    // Goalie-only tables are untouched by skater creates.
    let goalie_ai = huge.iter().find(|c| c.table == "mHuy").unwrap();
    assert_eq!(goalie_ai.adding, 0);
    assert!(
        !goalie_ai.over,
        "no goalie creates → goalie tables not over"
    );
}

#[test]
fn unknown_title_does_not_falsely_block_a_packed_file() {
    // With no game_id, a packed GEN7 main (max == current) has no reliable ceiling — the projection
    // must report it unenforced rather than block every create.
    let bytes = anchor_bytes();
    let db = TdbFile::parse(&bytes).unwrap();
    let rows = project_capacity(
        &db,
        None,
        PendingCreates {
            skaters: 50,
            goalies: 5,
        },
    );
    // The nhl14 anchor ships packed, so unknown-title rows are unenforced and never `over`.
    assert!(
        rows.iter().all(|c| !c.over),
        "unknown title must not block on an unknowable ceiling"
    );
}
