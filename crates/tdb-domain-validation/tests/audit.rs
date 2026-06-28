//! Whole-DB integrity audit against a real committed anchor (PS3 NHL Legacy `nhlng.db`).

use std::path::{Path, PathBuf};
use tdb_core::TdbFile;
use tdb_domain_validation::{audit_edge, companion_presence, validate_invariants, Severity};
use tdb_enums::EnumRegistry;
use tdb_relations::RelationRegistry;

fn defs(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../defs")
        .join(sub)
}

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

/// The confirmed foreign keys resolve overwhelmingly against the real roster — the predicate agrees
/// with the relationship evidence (subset ~1.0). We assert a high resolution rate rather than a hard
/// zero, because the committed vanilla anchor is not the operator's `default.db` the defs were
/// promoted on; a few legitimate sentinels/free-agent rows may remain.
#[test]
fn confirmed_fks_resolve_against_anchor() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();

    let mut audited = 0;
    for rel in relations.relationships() {
        for b in &rel.bindings {
            let Some(a) = audit_edge(&db, b) else {
                continue;
            };
            // Only assert on edges with a meaningful population in this anchor.
            if a.source_populated >= 100 {
                audited += 1;
                eprintln!(
                    "{}.{} -> {}.{}: {}/{} resolve ({:.4}), {} dangling",
                    a.source_table,
                    a.source_field,
                    a.target_table,
                    a.target_field,
                    a.resolved,
                    a.source_populated,
                    a.resolution_rate(),
                    a.dangling.len()
                );
                assert!(
                    a.resolution_rate() >= 0.95,
                    "{}.{} -> {}.{} only {:.4} resolved",
                    a.source_table,
                    a.source_field,
                    a.target_table,
                    a.target_field,
                    a.resolution_rate()
                );
            }
        }
    }
    assert!(
        audited >= 2,
        "expected at least the roster/player FKs to be audited, got {audited}"
    );
}

/// The companion-presence predicate (first Error-severity invariant) finds zero broken clusters on
/// the vanilla anchor — every player has the companion pair its kind requires.
#[test]
fn companion_presence_clean_on_anchor() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let findings = companion_presence(&db);
    assert!(
        findings.is_empty(),
        "expected zero companion violations, got {}: {:?}",
        findings.len(),
        findings.iter().take(5).collect::<Vec<_>>()
    );
}

/// The composed save-time audit runs end-to-end and stays **write-clean** (no Error-severity
/// findings) on the vanilla anchor, while correctly *surfacing* the documented edge cases as
/// Warnings — the partial-subset `team-links` orphans (`PuTc.vfEq`) and the `ZBac.ykFq` secondary
/// null sentinel. This proves the engine flags-for-inspection without crying wolf on clean data.
#[test]
fn vanilla_anchor_is_write_clean_and_flags_documented_exceptions() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();
    let enums = EnumRegistry::from_dir(defs("enums")).unwrap();

    let report = validate_invariants(&db, &relations, &enums);
    eprintln!(
        "findings: {} error, {} warning, {} info",
        report.count(Severity::Error),
        report.count(Severity::Warning),
        report.count(Severity::Info),
    );

    // Write-clean: nothing provably illegal on vanilla data.
    assert!(
        report.is_clean(),
        "vanilla anchor must have no Error-severity findings; first few: {:?}",
        report.errors().take(5).collect::<Vec<_>>()
    );

    // But the documented exceptions ARE surfaced as warnings.
    let warns: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .collect();
    assert!(!warns.is_empty(), "expected the documented FK exceptions as warnings");
    assert!(
        warns.iter().any(|f| f.table == "PuTc" && f.field == "vfEq"),
        "team-links orphan-by-design values should warn"
    );
    assert!(
        warns.iter().any(|f| f.table == "ZBac" && f.field == "ykFq"),
        "ZBac.ykFq secondary null sentinel should warn"
    );
}
